//! رها کردنِ فایلِ کانفیگ روی پنجره (Drag & Drop) — ۱.۳.۰.
//!
//! # چرا رویدادِ drag-dropِ خودِ تائوری کافی نبود
//!
//! مانیفستِ برنامه `requireAdministrator` است (Wintun و kill-switch). Explorer با
//! سطحِ یکپارچگیِ «متوسط» اجرا می‌شود و برنامه با سطحِ «بالا». ویندوز (UIPI)
//! کشیدن‌ورها‌کردنِ OLE را از پروسهٔ پایین‌تر به پروسهٔ بالاتر کاملاً می‌بندد:
//! `IDropTarget`ی که wry روی پنجره‌های WebView2 ثبت می‌کند هرگز صدا زده نمی‌شود،
//! نشانگر «ممنوع» می‌شود و هیچ رویدادی (`enter`/`over`/`drop`) به صفحه نمی‌رسد.
//! یعنی ایراد در جاوااسکریپت نبود؛ رویداد اصلاً به پروسه نمی‌آمد.
//!
//! # راهِ ریشه‌ای
//!
//! تنها مسیری که ویندوز برای رها کردن از Explorer روی پنجرهٔ ارتقایافته باز
//! می‌گذارد، سازوکارِ قدیمیِ `WM_DROPFILES` است، به این شرط که:
//!
//! 1. پنجره هیچ `IDropTarget`ِ ثبت‌شده‌ای نداشته باشد (`RevokeDragDrop`)، وگرنه
//!    ole32 همان مسیرِ بسته‌شدهٔ OLE را امتحان می‌کند؛
//! 2. پنجره `WS_EX_ACCEPTFILES` داشته باشد (`DragAcceptFiles`)؛
//! 3. پیام‌های `WM_DROPFILES`، `WM_COPYDATA` و `WM_COPYGLOBALDATA` از فیلترِ UIPI
//!    رد شوند (`ChangeWindowMessageFilterEx`).
//!
//! این سه روی پنجرهٔ اصلی و همهٔ پنجره‌های فرزندِ همین رشته (میزبان‌های WebView2)
//! اعمال می‌شوند، و یک subclass پیامِ `WM_DROPFILES` را می‌گیرد، مسیرها را با
//! `DragQueryFileW` می‌خواند و با رویدادِ [`EVENT`] به صفحه می‌دهد؛ صفحه همان
//! `cc_import_path` را صدا می‌زند که مسیرِ عادیِ تائوری صدا می‌زد.
//!
//! وقتی برنامه بی‌ارتقا اجرا شود (`tauri dev`)، هیچ‌کدام از این‌ها لازم نیست و
//! رویدادِ بومیِ تائوری (`tauri://drag-drop`) دست‌نخورده کار می‌کند.

/// رویدادی که مسیرهای رهاشده را به صفحه می‌برد (`string[]`).
pub const EVENT: &str = "aether://file-drop";

/// یک دستهٔ رهاشده را تمیز می‌کند: فاصلهٔ دو سر، مسیرِ خالی، و تکرار (بی‌توجه به
/// بزرگی/کوچکیِ حروف، مثلِ خودِ سیستمِ فایلِ ویندوز) حذف می‌شوند؛ ترتیب می‌ماند.
pub fn normalize_paths<I, S>(raw: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for p in raw {
        let p: String = p.into();
        let p = p.trim().trim_end_matches('\0').to_string();
        if p.is_empty() {
            continue;
        }
        if seen.insert(p.to_lowercase()) {
            out.push(p);
        }
    }
    out
}

#[cfg(windows)]
pub mod win {
    use std::sync::Arc;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, TRUE, WPARAM};
    use windows::Win32::System::Ole::RevokeDragDrop;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Shell::{
        DefSubclassProc, DragAcceptFiles, DragFinish, DragQueryFileW, RemoveWindowSubclass,
        SetWindowSubclass, HDROP,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        ChangeWindowMessageFilterEx, EnumChildWindows, GetWindowThreadProcessId, MSGFLT_ALLOW,
        WM_COPYDATA, WM_DROPFILES, WM_NCDESTROY,
    };

    /// پیامی که ole32 دادهٔ رهاشده را با آن منتقل می‌کند؛ در SDK نامی ندارد.
    pub const WM_COPYGLOBALDATA: u32 = 0x0049;
    const SUBCLASS_ID: usize = 0xAE70_D20F;

    /// مقصدِ مسیرهای رهاشده.
    pub type Sink = Arc<dyn Fn(Vec<String>) + Send + Sync>;

    /// همهٔ مسیرهای یک `HDROP`. حافظه را آزاد نمی‌کند (`DragFinish` با فراخوان است).
    ///
    /// # Safety
    /// `hdrop` باید یک `HDROP`ِ معتبر باشد.
    pub unsafe fn read_hdrop(hdrop: HDROP) -> Vec<String> {
        let count = DragQueryFileW(hdrop, u32::MAX, None);
        let mut out = Vec::with_capacity(count as usize);
        for i in 0..count {
            let len = DragQueryFileW(hdrop, i, None);
            if len == 0 {
                continue;
            }
            let mut buf = vec![0u16; len as usize + 1];
            let got = DragQueryFileW(hdrop, i, Some(&mut buf)) as usize;
            out.push(String::from_utf16_lossy(&buf[..got.min(buf.len())]));
        }
        out
    }

    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        refdata: usize,
    ) -> LRESULT {
        if msg == WM_DROPFILES {
            let hdrop = HDROP(wparam.0 as *mut core::ffi::c_void);
            let paths = super::normalize_paths(read_hdrop(hdrop));
            DragFinish(hdrop);
            if refdata != 0 && !paths.is_empty() {
                let sink = &*(refdata as *const Sink);
                sink(paths);
            }
            return LRESULT(0);
        }
        if msg == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID);
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    /// یک پنجره را برای `WM_DROPFILES` آماده می‌کند. تکرارش بی‌ضرر است.
    ///
    /// # Safety
    /// باید روی همان رشته‌ای صدا زده شود که پنجره را ساخته.
    pub unsafe fn attach(hwnd: HWND, sink: &'static Sink) -> bool {
        // هدفِ OLE از پروسهٔ پایین‌تر هرگز در دسترس نیست و جلوی مسیرِ قدیمی را می‌گیرد.
        let _ = RevokeDragDrop(hwnd);
        for m in [WM_DROPFILES, WM_COPYDATA, WM_COPYGLOBALDATA] {
            let _ = ChangeWindowMessageFilterEx(hwnd, m, MSGFLT_ALLOW, None);
        }
        DragAcceptFiles(hwnd, TRUE);
        SetWindowSubclass(
            hwnd,
            Some(subclass_proc),
            SUBCLASS_ID,
            sink as *const Sink as usize,
        )
        .as_bool()
    }

    unsafe extern "system" fn collect(
        hwnd: HWND,
        lparam: LPARAM,
    ) -> windows::Win32::Foundation::BOOL {
        let list = &mut *(lparam.0 as *mut Vec<HWND>);
        list.push(hwnd);
        TRUE
    }

    /// پنجرهٔ اصلی و همهٔ نوادگانِ ساخته‌شده روی همین رشته؛ تعدادِ آماده‌شده‌ها.
    ///
    /// # Safety
    /// روی رشتهٔ UI صدا زده شود.
    pub unsafe fn attach_tree(root: HWND, sink: &'static Sink) -> usize {
        let mut list: Vec<HWND> = vec![root];
        let _ = EnumChildWindows(
            root,
            Some(collect),
            LPARAM(&mut list as *mut Vec<HWND> as isize),
        );
        let me = GetCurrentThreadId();
        list.into_iter()
            .filter(|h| GetWindowThreadProcessId(*h, None) == me)
            .filter(|h| attach(*h, sink))
            .count()
    }

    pub fn is_elevated() -> bool {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
        unsafe {
            let mut token = HANDLE::default();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
                return false;
            }
            let mut elevation = TOKEN_ELEVATION::default();
            let mut size = 0u32;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            )
            .is_ok();
            let _ = CloseHandle(token);
            ok && elevation.TokenIsElevated != 0
        }
    }
}

/// پنجرهٔ اصلی را برای رها کردنِ فایل از Explorer آماده می‌کند.
///
/// فقط وقتی برنامه ارتقایافته است کاری می‌کند. پنجره‌های WebView2 گاهی پس از
/// `setup` ساخته یا بازسازی می‌شوند، پس اعمال چند بار با فاصله و با هر بار
/// فوکوس‌گرفتنِ پنجره تکرار می‌شود (هر بار بی‌ضرر و ارزان).
#[cfg(windows)]
pub fn install<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) {
    use std::sync::{Arc, OnceLock};
    use tauri::{Emitter, Manager};

    if !win::is_elevated() {
        return;
    }
    static SINK: OnceLock<usize> = OnceLock::new();
    let app = window.app_handle().clone();
    let sink_addr = *SINK.get_or_init(move || {
        let sink: win::Sink = Arc::new(move |paths: Vec<String>| {
            crate::log::DiagnosticsLog::i(
                "custom",
                &format!("{} file(s) dropped on the window.", paths.len()),
            );
            let _ = app.emit(EVENT, paths);
        });
        Box::leak(Box::new(sink)) as *const win::Sink as usize
    });

    fn apply<R: tauri::Runtime>(w: &tauri::WebviewWindow<R>, sink_addr: usize) {
        let w2 = w.clone();
        let _ = w.run_on_main_thread(move || {
            if let Ok(h) = w2.hwnd() {
                let hwnd = windows::Win32::Foundation::HWND(h.0 as *mut core::ffi::c_void);
                // SAFETY: روی رشتهٔ اصلی؛ `sink_addr` یک `Box`ِ نشت‌داده‌شدهٔ همیشگی است.
                unsafe {
                    let sink = &*(sink_addr as *const win::Sink);
                    win::attach_tree(hwnd, sink);
                }
            }
        });
    }

    apply(window, sink_addr);
    let later = window.clone();
    std::thread::spawn(move || {
        for ms in [250u64, 1000, 3000, 8000] {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            apply(&later, sink_addr);
        }
    });
    let watched = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(true) = event {
            apply(&watched, sink_addr);
        }
    });
}

#[cfg(not(windows))]
pub fn install<R: tauri::Runtime>(_window: &tauri::WebviewWindow<R>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_dedupes_and_keeps_order() {
        let got = normalize_paths([
            "  C:\\cfg\\a.npvt ",
            "",
            "C:\\cfg\\B.json\0",
            "c:\\CFG\\A.NPVT",
            "C:\\cfg\\c.conf",
        ]);
        assert_eq!(
            got,
            vec!["C:\\cfg\\a.npvt", "C:\\cfg\\B.json", "C:\\cfg\\c.conf"]
        );
    }

    /// مسیرِ کامل: فایلی که روی پنجره رها شده، با همان تابعی خوانده می‌شود که
    /// `cc_import_path` صدا می‌زند و به یک کانفیگ تبدیل می‌شود.
    #[test]
    fn a_dropped_file_path_imports_its_config() {
        let dir = std::env::temp_dir().join(format!("aether-drop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("dropped.txt");
        std::fs::write(
            &file,
            "vless://user@vision.example.net:443?security=tls&alpn=h2&encryption=none&insecure=0&headerType=none&fp=chrome&type=tcp&allowInsecure=0&flow=xtls-rprx-vision&sni=vision.example.net#%40tls-vision\n",
        )
        .unwrap();
        let paths = normalize_paths([file.to_string_lossy().to_string()]);
        let configs =
            crate::custom::import::from_file(std::path::Path::new(&paths[0]), None).unwrap();
        assert_eq!(configs.len(), 1);
        let bad = dir.join("dropped.exe");
        std::fs::write(&bad, "x").unwrap();
        assert!(crate::custom::import::from_file(&bad, None).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// شبیه‌سازیِ واقعیِ رها کردن: یک `HDROP` همان‌طور که Explorer می‌سازد
    /// (DROPFILES + مسیرهای UTF-16) به پنجره‌ای فرستاده می‌شود که `attach` شده،
    /// و مسیرها باید به مقصد برسند.
    #[cfg(windows)]
    #[test]
    fn wm_dropfiles_reaches_the_sink() {
        use std::sync::{Arc, Mutex};
        use windows::core::w;
        use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
        use windows::Win32::System::Memory::{
            GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
        };
        use windows::Win32::UI::Shell::DROPFILES;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, SendMessageW, HMENU, WINDOW_EX_STYLE, WM_DROPFILES,
            WS_OVERLAPPED,
        };

        let files = [
            r"C:\Users\me\Downloads\free.npvt",
            r"C:\Users\me\سرور.json",
            r"c:\users\me\downloads\FREE.npvt",
        ];
        let got: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let g = got.clone();
        let sink: &'static win::Sink =
            Box::leak(Box::new(
                Arc::new(move |p: Vec<String>| g.lock().unwrap().extend(p)) as win::Sink,
            ));

        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("aether-drop-test"),
                WS_OVERLAPPED,
                0,
                0,
                40,
                40,
                HWND::default(),
                HMENU::default(),
                HINSTANCE::default(),
                None,
            )
            .expect("window");
            assert!(win::attach(hwnd, sink), "subclass installed");

            let mut wide: Vec<u16> = Vec::new();
            for f in files {
                wide.extend(f.encode_utf16());
                wide.push(0);
            }
            wide.push(0);
            let head = std::mem::size_of::<DROPFILES>();
            let size = head + wide.len() * 2;
            let mem = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size).expect("alloc");
            let p = GlobalLock(mem) as *mut u8;
            let df = DROPFILES {
                pFiles: head as u32,
                fWide: true.into(),
                ..Default::default()
            };
            std::ptr::write_unaligned(p as *mut DROPFILES, df);
            std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, p.add(head), wide.len() * 2);
            let _ = GlobalUnlock(mem);

            // `DragFinish` در subclass حافظه را آزاد می‌کند.
            SendMessageW(hwnd, WM_DROPFILES, WPARAM(mem.0 as usize), LPARAM(0));
            let _ = DestroyWindow(hwnd);
        }
        assert_eq!(
            *got.lock().unwrap(),
            vec![
                r"C:\Users\me\Downloads\free.npvt".to_string(),
                r"C:\Users\me\سرور.json".to_string()
            ]
        );
    }
}
