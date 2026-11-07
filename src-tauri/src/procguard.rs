//! # procguard — مالکیتِ فرآیندها و پورت‌های محلی
//!
//! ## ریشهٔ خرابیِ «هیچ‌کدام از ۶ حالت وصل نمی‌شود» (لاگ‌های ۲۰۲۶-۱۰-۰۸)
//!
//! هر شش لاگ یک امضا دارند:
//!
//! ```text
//! W/state:  Local port 1819 is still busy after 3s — starting anyway.
//! D/engine: Error: Other("the socks5 listener cannot use 127.0.0.1:1819: … (os error 10048) …")
//! ```
//!
//! و در حالت «اتر تنها» حتی بدتر: `SOCKS5 handshake: OK` در حالی که موتورِ
//! همین تلاش ۲۰ms بعد از اجرا مرده بود — یعنی **یک فرآیندِ دیگر** روی ۱۸۱۹
//! نشسته و به دست‌دادنِ SOCKS5 جواب می‌دهد ولی هیچ upstreamی ندارد
//! (`TCP via proxy failed — the engine accepted the connection but no upstream
//! is flowing`). آن فرآیند، یتیمِ یک نشستِ قبلی است:
//!
//! * `AetherProcess::stop` و `TorNative::stop` فقط `TerminateProcess` روی
//!   فرزندِ مستقیم صدا می‌زدند. ویندوز درختِ فرآیند را با والد نمی‌کُشد؛ پس
//!   `psiphon-tunnel-core.exe` (که هسته در حالتِ `--psiphon-only` با
//!   `LocalSocksProxyPort=1819` اجرا می‌کند و `kill_on_drop` آن هرگز اجرا
//!   نمی‌شود چون هسته کشته می‌شود نه بسته) و `lyrebird.exe` زیرِ tor.exe زنده
//!   می‌ماندند.
//! * بسته‌شدنِ ناگهانیِ برنامه (کرش، End task، نصبِ نسخهٔ جدید روی نسخهٔ در
//!   حالِ اجرا، خاموش‌کردنِ ویندوز) هیچ `Drop`ی اجرا نمی‌کند، پس خودِ
//!   `aether.exe`/`tor.exe` هم یتیم می‌ماندند.
//!
//! و برنامه با «starting anyway» هر بار چهار تلاشِ محکوم‌به‌شکست می‌ساخت.
//!
//! ## رفعِ ریشه‌ای (سه لایه)
//!
//! 1. **پیشگیری** — هر فرزند (aether.exe، tor.exe) بلافاصله پس از ساخت در یک
//!    Job Object با `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` قرار می‌گیرد. نوه‌ها
//!    (psiphon-tunnel-core، lyrebird) عضویت را ارث می‌برند. توقف یعنی
//!    `TerminateJobObject` = کلِ درخت؛ و اگر برنامه به هر شکلی بمیرد، سیستم‌عامل
//!    دستهٔ Job را می‌بندد و کلِ درخت را خودش می‌کُشد. یتیم دیگر ساخته نمی‌شود.
//! 2. **بازپس‌گیری** — پیش از هر تلاش، صاحبِ واقعیِ پورت با
//!    `GetExtendedTcpTable` (PID) و `QueryFullProcessImageNameW` (مسیر) پیدا
//!    می‌شود. اگر یکی از باینری‌های خودِ اتر از پوشه‌های خودِ اتر باشد (یتیمِ
//!    نسخه‌های قبلی)، کشته و پورت پس گرفته می‌شود.
//! 3. **شکستِ سریع و صادق** — اگر صاحبِ پورت برنامهٔ دیگری باشد، اتصال با نام
//!    و PID همان برنامه متوقف می‌شود، نه با چهار تلاشِ بی‌حاصل؛ و «پورت باز
//!    است» فقط وقتی پذیرفته می‌شود که شنونده واقعاً فرزندِ همین نشست باشد.

use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::log::DiagnosticsLog;

const TAG: &str = "portguard";

/// باینری‌هایی که خودِ اتر اجرا می‌کند (مستقیم یا به‌عنوان نوه).
const OWN_IMAGES: &[&str] = &[
    "aether.exe",
    "psiphon-tunnel-core.exe",
    "tor.exe",
    "lyrebird.exe",
    "snowflake-client.exe",
    "webtunnel-client.exe",
    "conjure-client.exe",
];

/// شناسهٔ فرآیندهای زندهٔ همین نشست — و Job هر کدام.
///
/// `job` صفر یعنی ساختِ Job شکست خورده (مثلاً ویندوزی که Job تودرتو را
/// نمی‌پذیرد)؛ آن‌وقت فقط PID مستقیم شناخته می‌شود.
struct LiveEntry {
    id: u64,
    pid: u32,
    job: isize,
}

static LIVE: Mutex<Vec<LiveEntry>> = Mutex::new(Vec::new());
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// پوشه‌هایی که باینری‌های خودِ اتر از آن‌ها اجرا می‌شوند — یک‌بار در شروع
/// برنامه ست می‌شود (پوشهٔ نصب و پوشهٔ دادهٔ کاربر).
static OWN_ROOTS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

pub fn set_own_roots(roots: &[PathBuf]) {
    if let Ok(mut g) = OWN_ROOTS.lock() {
        *g = roots.to_vec();
    }
}

/// درختِ فرآیندِ یک فرزند. با `terminate` یا `Drop` کلِ درخت می‌میرد.
pub struct KillTree {
    id: u64,
    #[cfg(windows)]
    job: isize,
}

impl KillTree {
    /// فرزندِ تازه‌ساخته را (همراهِ هر نوه‌ای که بعداً بسازد) به یک Job
    /// «با مرگِ دسته، بمیر» می‌سپارد. هرگز خطا برنمی‌گرداند: اگر Job ساخته
    /// نشود، دست‌کم PID ثبت می‌شود تا نگهبانِ پورت فرزندِ خودی را غریبه نبیند.
    pub fn adopt(child: &Child, label: &str) -> KillTree {
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = child.id();
        #[cfg(windows)]
        let job = match win::adopt(child) {
            Ok(job) => job,
            Err(e) => {
                DiagnosticsLog::w(
                    TAG,
                    &format!("{label} (pid {pid}) could not be placed in a kill-on-close job ({e}); falling back to a direct kill."),
                );
                0
            }
        };
        #[cfg(not(windows))]
        let job = {
            let _ = label;
            0
        };
        if let Ok(mut g) = LIVE.lock() {
            g.push(LiveEntry { id, pid, job });
        }
        KillTree {
            id,
            #[cfg(windows)]
            job,
        }
    }

    /// کلِ درخت را همین حالا می‌کُشد (فرزند + نوه‌ها).
    pub fn terminate(&self) {
        #[cfg(windows)]
        if self.job != 0 {
            win::terminate_job(self.job);
        }
    }
}

impl Drop for KillTree {
    fn drop(&mut self) {
        self.terminate();
        if let Ok(mut g) = LIVE.lock() {
            g.retain(|e| e.id != self.id);
        }
        #[cfg(windows)]
        if self.job != 0 {
            win::close(self.job);
        }
    }
}

/// صاحبِ یک پورتِ محلی.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Holder {
    /// کسی روی پورت گوش نمی‌دهد.
    Free,
    /// فرزندِ زندهٔ همین نشست.
    Live { pid: u32 },
    /// یتیمِ خودِ اتر بود و کشته شد.
    Reclaimed { pid: u32, image: String },
    /// برنامهٔ دیگری — دست نمی‌خورد.
    Foreign { pid: u32, image: String },
    /// روی این سیستم‌عامل/این دسترسی قابلِ تشخیص نیست.
    #[allow(dead_code)] // only built off Windows
    Unknown,
}

impl Holder {
    pub fn describe_foreign(&self, port: u16) -> Option<String> {
        match self {
            Holder::Foreign { pid, image } => Some(format!(
                "Local port 127.0.0.1:{port} is held by another program: {image} (PID {pid}). \
                 Dopamine needs this port for its local SOCKS5 proxy. Close that program \
                 (or end PID {pid} in Task Manager) and connect again."
            )),
            _ => None,
        }
    }
}

fn file_name_lower(path: &Path) -> String {
    // Split on both separators by hand so the rule is the same on every host
    // (Windows paths are also exercised by the Linux preflight unit tests).
    let s = path.to_string_lossy().to_ascii_lowercase();
    s.rsplit(['\\', '/']).next().unwrap_or_default().to_string()
}

fn under_own_roots(path: &Path) -> bool {
    let p = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('/', "\\");
    let roots = OWN_ROOTS.lock().map(|g| g.clone()).unwrap_or_default();
    roots.iter().any(|r| {
        let mut r = r.to_string_lossy().to_ascii_lowercase().replace('/', "\\");
        if r.is_empty() {
            return false;
        }
        if !r.ends_with('\\') {
            r.push('\\');
        }
        p.starts_with(&r)
    })
}

/// آیا این تصویرِ فرآیند، باینریِ خودِ اتر از پوشه‌های خودِ اتر است؟
fn is_own_image(path: &Path) -> bool {
    let name = file_name_lower(path);
    if !OWN_IMAGES.contains(&name.as_str()) {
        return false;
    }
    if under_own_roots(path) {
        return true;
    }
    // Another copy of Aether Desktop (an older portable folder, a previous
    // install location): its engine always lives in `…\engine\`. Only the
    // engine and its Psiphon child qualify — a `tor.exe` outside our folders
    // may be Tor Browser and is never touched.
    let lower = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('/', "\\");
    (name == "aether.exe" || name == "psiphon-tunnel-core.exe")
        && lower.ends_with(&format!("\\engine\\{name}"))
}

fn pid_is_live(pid: u32) -> bool {
    let entries: Vec<(u32, isize)> = LIVE
        .lock()
        .map(|g| g.iter().map(|e| (e.pid, e.job)).collect())
        .unwrap_or_default();
    if entries.iter().any(|(p, _)| *p == pid) {
        return true;
    }
    #[cfg(windows)]
    {
        let jobs: Vec<isize> = entries
            .iter()
            .map(|(_, j)| *j)
            .filter(|j| *j != 0)
            .collect();
        if !jobs.is_empty() {
            return win::pid_in_any_job(pid, &jobs);
        }
    }
    false
}

/// PID شنوندهٔ TCP روی `127.0.0.1:port` یا `0.0.0.0:port`.
pub fn listener_pid(port: u16) -> Option<u32> {
    #[cfg(windows)]
    {
        win::listener_pid(port)
    }
    #[cfg(not(windows))]
    {
        let _ = port;
        None
    }
}

/// مسیرِ کاملِ فایلِ اجراییِ یک PID.
pub fn image_of(pid: u32) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        win::image_path(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        None
    }
}

/// صاحبِ پورت را پیدا می‌کند و اگر یتیمِ خودِ اتر باشد، آن را می‌کُشد.
///
/// فرزندانِ زندهٔ همین نشست هرگز کشته نمی‌شوند (در زنجیرهٔ Tor → Psiphon،
/// tor.exe روی ۱۸۲۰ زنده است وقتی موتور برای ۱۸۱۹ آماده می‌شود).
pub fn reclaim(port: u16) -> Holder {
    #[cfg(not(windows))]
    {
        let _ = port;
        Holder::Unknown
    }
    #[cfg(windows)]
    {
        let Some(pid) = listener_pid(port) else {
            return Holder::Free;
        };
        if pid == std::process::id() {
            // خودِ برنامه (مثلاً پل محلی) — هرگز نباید روی این پورت‌ها باشد،
            // ولی کشتنِ خود هم هرگز درست نیست.
            return Holder::Foreign {
                pid,
                image: "Dopamine (this app)".into(),
            };
        }
        if pid_is_live(pid) {
            return Holder::Live { pid };
        }
        let path = image_of(pid);
        let image = path
            .as_deref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| format!("PID {pid} (image not readable)"));
        match path.as_deref() {
            Some(p) if is_own_image(p) => {
                if win::kill_pid(pid, Duration::from_millis(2_500)) {
                    DiagnosticsLog::w(
                        TAG,
                        &format!(
                            "Reclaimed 127.0.0.1:{port} from an orphaned {} (pid {pid}) left behind by an earlier session.",
                            file_name_lower(p)
                        ),
                    );
                    Holder::Reclaimed { pid, image }
                } else {
                    DiagnosticsLog::e(
                        TAG,
                        &format!("An orphaned {image} (pid {pid}) holds 127.0.0.1:{port} and could not be terminated."),
                    );
                    Holder::Foreign { pid, image }
                }
            }
            _ => Holder::Foreign { pid, image },
        }
    }
}

/// پیش از هر تلاش: پورت‌ها را آزاد می‌کند یا دلیلِ دقیقِ آزادنشدن را برمی‌گرداند.
///
/// `required` پورت‌هایی است که اگر برنامهٔ غریبه‌ای آن‌ها را گرفته باشد، اتصال
/// بی‌معناست (۱۸۱۹ همیشه). بقیه فقط از یتیم پاک می‌شوند.
pub fn prepare_ports(required: &[u16], optional: &[u16], wait: Duration) -> Result<(), String> {
    for &port in required.iter().chain(optional.iter()) {
        let _ = reclaim(port);
    }
    let deadline = Instant::now() + wait;
    loop {
        let mut blocker: Option<String> = None;
        let mut busy = false;
        for &port in required {
            match reclaim(port) {
                Holder::Free | Holder::Reclaimed { .. } => {}
                // A child of this session that is still on its way out.
                Holder::Live { .. } => busy = true,
                Holder::Unknown => {
                    if std::net::TcpStream::connect_timeout(
                        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                        Duration::from_millis(200),
                    )
                    .is_ok()
                    {
                        busy = true;
                    }
                }
                h @ Holder::Foreign { .. } => {
                    busy = true;
                    blocker = h.describe_foreign(port);
                }
            }
        }
        if !busy {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return match blocker {
                Some(msg) => Err(msg),
                // روی سیستمی که صاحب قابل تشخیص نیست، رفتارِ قبلی حفظ می‌شود.
                None => Ok(()),
            };
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// پس از «پورت باز است»: آیا شنونده واقعاً فرزندِ همین نشست است؟
///
/// `Some(پیام)` یعنی برنامهٔ دیگری جواب می‌دهد و بالا آوردنِ مسیر داده روی آن
/// همان «SOCKS5 handshake: OK ولی هیچ upstreamی نیست» لاگ کاربر است.
pub fn stranger_on(port: u16) -> Option<String> {
    let pid = listener_pid(port)?;
    if pid_is_live(pid) {
        return None;
    }
    let image = image_of(pid)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| format!("PID {pid}"));
    Some(format!(
        "127.0.0.1:{port} is answered by {image} (PID {pid}), not by the engine this session started."
    ))
}

/// پاک‌سازیِ آغازِ برنامه: یتیم‌های نشست‌های قبلی روی پورت‌های ثابت.
pub fn sweep_orphans(ports: &[u16]) {
    for &port in ports {
        if let Holder::Foreign { pid, image } = reclaim(port) {
            DiagnosticsLog::w(
                TAG,
                &format!("127.0.0.1:{port} is in use by {image} (PID {pid}) — Dopamine will report it if a connection needs this port."),
            );
        }
    }
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    use std::path::PathBuf;
    use std::process::Child;
    use std::time::Duration;

    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    };
    use windows::Win32::Networking::WinSock::AF_INET;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob,
        JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        PROCESS_TERMINATE,
    };

    fn h(raw: isize) -> HANDLE {
        HANDLE(raw as *mut c_void)
    }

    pub fn adopt(child: &Child) -> Result<isize, String> {
        unsafe {
            let job = CreateJobObjectW(None, PCWSTR::null()).map_err(|e| e.to_string())?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
            if let Err(e) = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) {
                let _ = CloseHandle(job);
                return Err(e.to_string());
            }
            let process = HANDLE(child.as_raw_handle() as *mut c_void);
            if let Err(e) = AssignProcessToJobObject(job, process) {
                let _ = CloseHandle(job);
                return Err(e.to_string());
            }
            Ok(job.0 as isize)
        }
    }

    pub fn terminate_job(job: isize) {
        unsafe {
            let _ = TerminateJobObject(h(job), 1);
        }
    }

    pub fn close(job: isize) {
        unsafe {
            let _ = CloseHandle(h(job));
        }
    }

    pub fn pid_in_any_job(pid: u32, jobs: &[isize]) -> bool {
        unsafe {
            let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return false;
            };
            let mut hit = false;
            for &job in jobs {
                let mut result = BOOL(0);
                if IsProcessInJob(process, h(job), &mut result).is_ok() && result.as_bool() {
                    hit = true;
                    break;
                }
            }
            let _ = CloseHandle(process);
            hit
        }
    }

    pub fn listener_pid(port: u16) -> Option<u32> {
        unsafe {
            let mut size: u32 = 0;
            let _ = GetExtendedTcpTable(
                None,
                &mut size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            );
            for _ in 0..4 {
                if size == 0 {
                    return None;
                }
                // u32-aligned buffer (the table is an array of u32 fields).
                let mut buf: Vec<u32> = vec![0; (size as usize + 3) / 4 + 1];
                let mut len = (buf.len() * 4) as u32;
                let rc = GetExtendedTcpTable(
                    Some(buf.as_mut_ptr() as *mut c_void),
                    &mut len,
                    false,
                    AF_INET.0 as u32,
                    TCP_TABLE_OWNER_PID_LISTENER,
                    0,
                );
                if rc == 122 {
                    // ERROR_INSUFFICIENT_BUFFER — the table grew; retry.
                    size = len;
                    continue;
                }
                if rc != 0 {
                    return None;
                }
                let count = buf[0] as usize;
                let rows = buf.as_ptr().add(1) as *const MIB_TCPROW_OWNER_PID;
                let max_rows = (buf.len() - 1) * 4 / std::mem::size_of::<MIB_TCPROW_OWNER_PID>();
                for i in 0..count.min(max_rows) {
                    let row = &*rows.add(i);
                    let local_port = u16::from_be((row.dwLocalPort & 0xFFFF) as u16);
                    if local_port != port {
                        continue;
                    }
                    // dwLocalAddr is in network order: 127.0.0.1 → bytes 7f 00 00 01.
                    let addr = row.dwLocalAddr.to_ne_bytes();
                    if addr == [0, 0, 0, 0] || addr[0] == 127 {
                        return Some(row.dwOwningPid);
                    }
                }
                return None;
            }
            None
        }
    }

    pub fn image_path(pid: u32) -> Option<PathBuf> {
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = vec![0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok();
            let _ = CloseHandle(process);
            if !ok {
                return None;
            }
            Some(PathBuf::from(String::from_utf16_lossy(
                &buf[..len as usize],
            )))
        }
    }

    pub fn kill_pid(pid: u32, wait: Duration) -> bool {
        unsafe {
            let Ok(process) = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, pid)
            else {
                return false;
            };
            let killed = TerminateProcess(process, 1).is_ok();
            let gone = WaitForSingleObject(process, wait.as_millis() as u32) == WAIT_OBJECT_0;
            let _ = CloseHandle(process);
            killed && gone
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_our_binaries_from_our_folders_count_as_ours() {
        set_own_roots(&[PathBuf::from(
            r"C:\Users\U\AppData\Local\com.dopamine.desktop",
        )]);
        assert!(is_own_image(Path::new(
            r"C:\Users\U\AppData\Local\com.dopamine.desktop\engine\psiphon-tunnel-core.exe"
        )));
        assert!(is_own_image(Path::new(
            r"c:\users\u\appdata\local\COM.DOPAMINE.DESKTOP\engine\aether.exe"
        )));
        // Tor Browser's tor.exe, or any other program, is never touched.
        assert!(!is_own_image(Path::new(
            r"C:\Tor Browser\Browser\TorBrowser\Tor\tor.exe"
        )));
        assert!(!is_own_image(Path::new(
            r"C:\Users\U\AppData\Local\com.dopamine.desktop\engine\other.exe"
        )));
        // A sibling folder that merely shares the prefix is not ours.
        assert!(!is_own_image(Path::new(
            r"C:\Users\U\AppData\Local\com.dopamine.desktop-evil\aether.exe"
        )));
        // An older portable copy of Dopamine Desktop: its engine folder counts.
        assert!(is_own_image(Path::new(
            r"D:\Downloads\Dopamine-Portable-1.2.5\engine\psiphon-tunnel-core.exe"
        )));
        assert!(!is_own_image(Path::new(r"D:\Tools\engine\tor.exe")));
    }

    #[test]
    fn a_foreign_holder_is_reported_with_its_name_and_pid() {
        let h = Holder::Foreign {
            pid: 4242,
            image: r"C:\x\proxy.exe".into(),
        };
        let msg = h.describe_foreign(1819).unwrap();
        assert!(msg.contains("1819") && msg.contains("proxy.exe") && msg.contains("4242"));
        assert!(Holder::Free.describe_foreign(1819).is_none());
    }
}
