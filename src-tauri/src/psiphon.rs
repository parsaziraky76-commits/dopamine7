//! پورت از `transport/PsiphonTransport.kt` — استیج ۲ زنجیرهٔ `Aether → Psiphon`.
//!
//! ```text
//!   stage 1   موتور اِتِر        → SOCKS5 127.0.0.1:1819      (هنوز بدون مسیر داده)
//!   stage 2   Psiphon           → SOCKS5 127.0.0.1:1825      از راه 1819 بیرون می‌رود
//!   سپس      پل + پروکسی سیستمی → 127.0.0.1:1825             خروجی = Psiphon
//! ```
//!
//! # چرا اصلاً زنجیره؟
//!
//! خروجی‌های اِتِر/WARP لبه‌های anycast کلادفلر هستند. مجموعهٔ بزرگی از مقصدها
//! یا آن‌ها را کامل بلاک می‌کنند یا نمای دیگری از اینترنت را از آن‌ها سرو
//! می‌کنند — یعنی تونل بالاست و سایت باز نمی‌شود. زنجیره **خروجی** را با یک
//! آدرس Psiphon عوض می‌کند و در همان حال ترافیک مبهم‌سازی‌شدهٔ اِتِر را روی
//! **هاپ اول** نگه می‌دارد؛ همان هاپی که باید از شبکهٔ محلی جان سالم ببرد.
//!
//! # تفاوت ساختاری با اندروید — و اینکه چرا اینجا کد **کمتری** لازم است
//!
//! در اندروید یک لایهٔ سوم هم وجود دارد: `PsiphonSocksFront` روی پورت ۱۸۲۵،
//! و Psiphon خودش روی ۱۸۲۷ می‌نشیند. آن لایه فقط و فقط برای این ساخته شد که
//! `hev-socks5-tunnel` (tun2socks اندروید) تمام UDP و در نتیجه تمام DNS را با
//! `UDP ASSOCIATE` حمل می‌کند، و پروکسی محلی psiphon-tunnel-core فقط CONNECT
//! می‌فهمد:
//!
//! ```text
//!   socks5ReadCommand: SOCKS message field command was 0x03, not 0x01
//! ```
//!
//! ۶۳۶ بار در ۵۰ ثانیه. آن front بود که udpgw، لِینِ اختصاصی DNS، حمل QUIC و
//! سرکوب AAAA را آورد.
//!
//! **در ویندوز هیچ‌کدامش لازم نیست، و این خبر خوبی است.** مسیر دادهٔ این نسخه
//! پروکسی سیستمی WinINET → پل `share.rs` است؛ یعنی مسیر داده از بنیاد
//! **TCP-only و IPv4-only** است و تنها چیزی که از استیج ۲ می‌خواهد
//! `CONNECT` است — همان چیزی که Psiphon می‌دهد. پس Psiphon مستقیم روی
//! [crate::engine::CHAIN_SOCKS_PORT] می‌نشیند و همان پورت، خروجیِ کل خط لوله
//! است. یک لایهٔ کمتر = یک نقطهٔ شکست کمتر.
//!
//! و این تصادفی نیست که همان شکل «درست» است: در مستند
//! `PSIPHON_APP_CONNECTIVITY.md` نسخهٔ موبایل، سرنخِ قطعی این بود که وقتی گوشی
//! به لپ‌تاپ USB-tether می‌شد، `gemini.google.com` و `aistudio.google.com`
//! **بلافاصله** در مرورگر لپ‌تاپ باز می‌شدند — چون آن مسیر پروکسیِ TCP-only و
//! IPv4-only بود و همان دو چیزی را حذف می‌کرد که روی گوشی خراب بود (بلک‌هول
//! UDP/443 و آدرس IPv6 روی TUN). مسیر دادهٔ ویندوز **همان مسیر لپ‌تاپ** است:
//!
//!   * **QUIC**: مرورگر پشت پروکسی HTTP/SOCKS برای مقصدها HTTP/3 نمی‌زند و
//!     تمیز به TCP برمی‌گردد؛ و [crate::leakguard] هم UDP خروجی مرورگر را
//!     می‌بندد. پس هیچ دیتاگرام UDP/443 ای در سیاه‌چاله گم نمی‌شود که برنامه
//!     آن را «اینترنت نداری» بخواند.
//!   * **AAAA / IPv6**: مسیر داده لوپ‌بکِ IPv4 است و هیچ آدرس IPv6 ای به
//!     رزولور ویندوز اعلام نمی‌شود، پس Happy Eyeballs هیچ‌وقت AAAA ای را که
//!     خروجی Psiphon نمی‌تواند بگیرد ترجیح نمی‌دهد — همان چیزی که روی اندروید
//!     ۶۵ جریان رد‌شده و خطای «تحریم/بلاک کشور» می‌ساخت.
//!   * **DNS**: مرورگرهای کرومیوم ورودی `socks=` را برمی‌دارند و نام مقصد را
//!     با `ATYP=DOMAIN` داخل تونل حل می‌کنند، و پل هم برای HTTP همین کار را
//!     می‌کند. یعنی نام‌ها آن‌طرفِ Psiphon حل می‌شوند، نه اینجا.
//!
//! پس «باز نشدن سایت‌های هوش مصنوعی» روی این مسیر ساختاراً همان شکلی است که
//! در گزارش میدانی **کار می‌کرد**.
//!
//! # سازوکارهایی که عیناً از اندروید آمده‌اند
//!
//! ۱. **کشور خروج یک بن‌بست بود.** `EgressRegion` در psiphon-tunnel-core یک
//!    فیلتر **سخت** است: اگر همان لحظه هیچ سروری در آن کشور در دسترس نباشد،
//!    کنترلر تا ابد دنبال خروجی می‌گردد که پیدا نمی‌شود. [PsiphonTransport::start]
//!    مثل موبایل تلاش دوم را با خروجی خودکار و datastore تازه می‌زند، چون یک
//!    تونل کارکنده در کشور اشتباه از هیچ تونلی بهتر است.
//! ۲. **پورت را دیکته نمی‌کنیم.** پورتی که Psiphon واقعاً گرفته از notice
//!    `ListeningSocksProxyPort` خوانده می‌شود و کل خط لوله همان را دنبال
//!    می‌کند. در اندروید `check(port == 1819)` نشست را می‌کشت.
//! ۳. **datastore گیرکرده قابل بازیابی است.** datastore جای خودش را دارد
//!    ([DATA_DIR_NAME]) و کانفیگ عمداً **بیرون** آن نوشته می‌شود، تا پاک‌کردن
//!    datastore بین دو تلاش کانفیگ را با خودش نبرد.
//! ۴. **پروکسی بالادست، نه مسیریابی دستی.** Psiphon `UpstreamProxyUrl` می‌گیرد و
//!    **هر** اتصالی که می‌سازد — از جمله واکشی فهرست سرور و دایرکتوری — از آن
//!    می‌رود، پس زنجیره نمی‌تواند یک dial مستقیم لو بدهد.
//! ۵. **چرخش سرور، نشست را نمی‌کشد.** [crate::psiphon_health] سروری را که
//!    port forward ها را رد می‌کند محکوم می‌کند و [PsiphonTransport::rotate]
//!    فقط استیج ۲ را عوض می‌کند؛ پنجرهٔ مهلت [ROTATION_GRACE_MS] نمی‌گذارد
//!    سوپروایزر این جابه‌جایی عمدی را «مرگ» بخواند و کل نشست را پایین بیاورد.

use crate::engine;
use crate::exit_regions;
use crate::log::DiagnosticsLog;
use crate::probe;
use crate::psiphon_health::{self, Rotation, Target};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use serde_json::json;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const TAG: &str = "psiphon";

/// پیشوندِ «استیج ۲ راه‌اندازی خودش را رد کرد».
///
/// یک خطای پیکربندی/آرگومان با تلاش مجدد درست نمی‌شود: هر پلهٔ نردبان دقیقاً
/// همان‌جا و با همان پیام می‌افتد (همان چیزی که در لاگ کاربر ۴ پله را سوزاند).
/// پس این پیشوند یک **کلاسِ خطا** روی همان کانال رشته‌ای می‌سازد که
/// [crate::state] از قبل دارد، و آن‌طرف می‌تواند بلافاصله و با یک پیام دقیق
/// شکست بخورد به‌جای اینکه بی‌فایده نردبان را ادامه بدهد.
pub const CONFIG_FAULT_PREFIX: &str = "Psiphon rejected its own launch settings — ";

/// آیا این پیام شکست، یک خطای پیکربندیِ استیج ۲ است (و نه شبکه)؟
pub fn is_config_fault(message: &str) -> bool {
    message.contains(CONFIG_FAULT_PREFIX)
}

/// نام فایل اجرایی استیج ۲ در پوشهٔ `engine`.
pub const PSIPHON_EXE: &str = "psiphon-tunnel-core.exe";
/// فهرست سرور تعبیه‌شده — همان فایلی که نسخهٔ موبایل در assets دارد.
pub const SERVER_LIST_FILE: &str = "server_entries.txt";
/// پوشهٔ datastore، جدا از هر چیز دیگری تا مستقل پاک شود.
const DATA_DIR_NAME: &str = "psiphon";
/// کانفیگ **بیرون** از datastore می‌نشیند (بند ۳ مستند بالا).
const CONFIG_FILE: &str = "psiphon.config";

/// بودجهٔ برقراری. سمت Go در [ESTABLISH_TIMEOUT_SECONDS] تسلیم می‌شود، پس این
/// فقط پشتیبانِ فرآیندی است که هیچ‌وقت هیچ چیزی نمی‌گوید.
const ESTABLISH_TIMEOUT_SECONDS: u64 = 180;
const ESTABLISH_TIMEOUT_MS: u64 = 200_000;
const PORT_RELEASE_WAIT_MS: u64 = 3_000;

/// چقدر یک چرخشِ در جریان اجازه دارد «زنده» گزارش شود.
///
/// یک راه‌اندازی مجدد سالم حدود دو ثانیه‌ای برمی‌گردد، پس این عدد دو مرتبه
/// بزرگ‌تر از لازم است — و باز هم بسیار کمتر از بودجهٔ برقراری، چون چرخشی که
/// یک دقیقه و نیم طول بکشد یک نشست خراب است و مال سوپروایزر است، نه واچ‌داگ.
const ROTATION_GRACE_MS: u64 = 90_000;

const REMOTE_SERVER_LIST_PUBLIC_KEY: &str = "MIICIDANBgkqhkiG9w0BAQEFAAOCAg0AMIICCAKCAgEAt7Ls+/39r+T6zNW7GiVpJfzq/xvL9SBH5rIFnk0RXYEYavax3WS6HOD35eTAqn8AniOwiH+DOkvgSKF2caqk/y1dfq47Pdymtwzp9ikpB1C5OfAysXzBiwVJlCdajBKvBZDerV1cMvRzCKvKwRmvDmHgphQQ7WfXIGbRbmmk6opMBh3roE42KcotLFtqp0RRwLtcBRNtCdsrVsjiI1Lqz/lH+T61sGjSjQ3CHMuZYSQJZo/KrvzgQXpkaCTdbObxHqb6/+i1qaVOfEsvjoiyzTxJADvSytVtcTjijhPEV6XskJVHE1Zgl+7rATr/pDQkw6DPCNBS1+Y6fy7GstZALQXwEDN/qhQI9kWkHijT8ns+i1vGg00Mk/6J75arLhqcodWsdeG/M/moWgqQAnlZAGVtJI1OgeF5fsPpXu4kctOfuZlGjVZXQNW34aOzm8r8S0eVZitPlbhcPiR4gT/aSMz/wd8lZlzZYsje/Jr8u/YtlwjjreZrGRmG8KMOzukV3lLmMppXFMvl4bxv6YFEmIuTsOhbLTwFgh7KYNjodLj/LsqRVfwz31PgWQFTEPICV7GCvgVlPRxnofqKSjgTWI4mxDhBpVcATvaoBl1L/6WLbFvBsoAUBItWwctO2xalKxF5szhGm8lccoc5MZr8kfE0uxMgsxz4er68iCID+rsCAQM=";
const SERVER_ENTRY_SIGNATURE_PUBLIC_KEY: &str = "sHuUVTWaRyh5pZwy4UguSgkwmBe0EHtJJkoF5WrxmvA=";
const EXCHANGE_OBFUSCATION_KEY: &str = "DpXzloJk1Hw6aSzmKKky0xcahsEHubch81Mi6K0XMlU=";

/// ساعت یکنواخت — مثل `psiphon_health`، پرش ساعت سیستم نباید مهلت‌ها را بشکند.
fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// وضعیتی که ترد خوانندهٔ notice می‌نویسد و بقیه می‌خوانند.
struct Shared {
    /// پورتی که Psiphon **واقعاً** گرفته (از `ListeningSocksProxyPort`).
    socks_port: AtomicU16,
    /// `Tunnels` با count ≥ ۱ دیده شده است؟
    connected: AtomicBool,
    /// `Exiting` دیده شده — فرآیند دارد تمام می‌شود.
    exiting: AtomicBool,
    /// مهلتی که تا آن یک چرخشِ در جریان «زنده» به حساب می‌آید. ۰ = بی‌چرخش.
    rotation_deadline: AtomicU64,
    /// مناطقی که کتابخانه گفته واقعاً می‌تواند به آن‌ها برسد (خوراک هدایت).
    available_regions: Mutex<Vec<String>>,
    /// منطقهٔ خروج تونل فعال — برای نشان و لاگ.
    connected_region: Mutex<String>,
    /// notice های روتینِ حذف‌شده از آخرین سطری که به لاگ رسید.
    noise_notices: AtomicU64,
    /// نسل نشست: خروجی فرآیندهای قدیمی باید بی‌صدا دور ریخته شود.
    generation: AtomicU64,
    /// خطای پیکربندی/آرگومانی که فرآیند را همان اول کشت. خالی = چنین چیزی نبود.
    config_fault: Mutex<String>,
}

impl Shared {
    fn new() -> Self {
        Self {
            socks_port: AtomicU16::new(0),
            connected: AtomicBool::new(false),
            exiting: AtomicBool::new(false),
            rotation_deadline: AtomicU64::new(0),
            available_regions: Mutex::new(Vec::new()),
            connected_region: Mutex::new(String::new()),
            noise_notices: AtomicU64::new(0),
            generation: AtomicU64::new(0),
            config_fault: Mutex::new(String::new()),
        }
    }

    /// آیا چرخشی در جریان است و هنوز داخل بودجه‌اش؟
    ///
    /// خودمنقضی‌شونده: به‌محض گذشتن مهلت، پرچم پاک می‌شود و [PsiphonTransport::is_alive]
    /// بعدی راست می‌گوید، پس راه‌اندازی مجددی که هرگز برنمی‌گردد به سوپروایزر
    /// تحویل داده می‌شود، نه اینکه ابدی نشانِ «متصل» را نگه دارد.
    fn rotating(&self) -> bool {
        let until = self.rotation_deadline.load(Ordering::Relaxed);
        if until == 0 {
            return false;
        }
        if now_ms() < until {
            return true;
        }
        self.rotation_deadline.store(0, Ordering::Relaxed);
        DiagnosticsLog::w(
            TAG,
            &format!(
                "Psiphon rotation did not come back within {}s — handing the session back to the supervisor.",
                ROTATION_GRACE_MS / 1000
            ),
        );
        false
    }
}

/// همه‌چیزِ قابل‌تغییر که فقط یک نگه‌دارنده باید هم‌زمان داشته باشد.
///
/// ⚠ قاعدهٔ بن‌بست: این قفل **هرگز** در طول انتظار (خواب، await پورت، …) نگه
/// داشته نمی‌شود. انتظار روی [Shared] انجام می‌شود که اتمیک است.
struct Inner {
    exe: PathBuf,
    server_list: PathBuf,
    data_dir: PathBuf,
    config_path: PathBuf,
    child: Option<Child>,
    /// کشوری که **کاربر** انتخاب کرده. هرگز بی‌خبرِ او عوض نمی‌شود.
    region: String,
    /// `socks5://127.0.0.1:1819` — استیج ۱.
    upstream: String,
    /// پورتی که از Psiphon می‌خواهیم بگیرد.
    local_socks_port: u16,
    /// کانفیگ فعلی روی دیسک (ممکن است هدایت‌شده باشد).
    config: String,
    /// پس از نخستین محکومیت روشن می‌شود: replay و server affinity خاموش.
    avoid_replay: bool,
}

pub struct PsiphonTransport {
    inner: Arc<Mutex<Inner>>,
    shared: Arc<Shared>,
}

impl PsiphonTransport {
    /// استیج ۲ را کنار موتور آماده می‌کند.
    ///
    /// درست مثل `engine::prepare_runtime_engine`، فایل اجرایی و فهرست سرور از
    /// پوشهٔ نصب به پوشهٔ دادهٔ **قابل‌نوشتن** کاربر منتقل شده‌اند (همان کاری که
    /// `AetherProcess::new` می‌کند و ریشهٔ خطای `Access is denied` را بست)، پس
    /// اینجا فقط به همان پوشه اشاره می‌کنیم.
    pub fn new(install_dir: &Path, working_dir: &Path) -> Self {
        let runtime_dir = working_dir.join("engine");
        let bundled_dir = install_dir.join("engine");
        let pick = |name: &str| -> PathBuf {
            let staged = runtime_dir.join(name);
            if staged.exists() {
                staged
            } else {
                bundled_dir.join(name)
            }
        };
        Self {
            inner: Arc::new(Mutex::new(Inner {
                exe: pick(PSIPHON_EXE),
                server_list: pick(SERVER_LIST_FILE),
                data_dir: working_dir.join(DATA_DIR_NAME),
                config_path: working_dir.join(CONFIG_FILE),
                child: None,
                region: String::new(),
                upstream: String::new(),
                local_socks_port: engine::CHAIN_SOCKS_PORT,
                config: String::new(),
                avoid_replay: false,
            })),
            shared: Arc::new(Shared::new()),
        }
    }

    /// آیا استیج ۲ اصلاً روی این نصب موجود است؟
    ///
    /// جدا از [PsiphonTransport::start] است تا رابط کاربری بتواند بک‌اند
    /// زنجیره‌ای را به‌جای «اتصال ناموفق» با یک پیام روشن رد کند.
    pub fn is_available(&self) -> bool {
        let g = self.inner.lock();
        g.exe.exists() && g.server_list.exists()
    }

    pub fn missing_parts(&self) -> String {
        let g = self.inner.lock();
        let mut missing = Vec::new();
        if !g.exe.exists() {
            missing.push(PSIPHON_EXE);
        }
        if !g.server_list.exists() {
            missing.push(SERVER_LIST_FILE);
        }
        missing.join(", ")
    }

    /// خروجیِ خط لوله: پورتی که مسیر داده باید به آن وصل شود.
    pub fn socks_port(&self) -> u16 {
        let p = self.shared.socks_port.load(Ordering::Relaxed);
        if p != 0 {
            p
        } else {
            self.inner.lock().local_socks_port
        }
    }

    /// منطقهٔ خروجِ تونل فعال، اگر Psiphon گفته باشد.
    pub fn exit_region(&self) -> Option<String> {
        let r = self.shared.connected_region.lock().clone();
        if r.is_empty() {
            None
        } else {
            Some(r)
        }
    }

    /// **مسدودکننده.** استیج ۲ را بالا می‌آورد و پورت نهایی را برمی‌گرداند.
    ///
    /// از یک ترد پس‌زمینه صدا زده می‌شود (`state.rs`), هرگز از حلقهٔ tick:
    /// برقراری Psiphon تا سه دقیقه طول می‌کشد و رابط کاربری نباید فریز شود.
    ///
    /// پاس اول کشور کاربر را محترم می‌شمارد، پاس دوم فیلتر را برمی‌دارد و
    /// datastore را ریست می‌کند. وقتی کاربر از اول «خودکار» خواسته، پاس دومی
    /// وجود ندارد: فیلتری نمانده که شل شود.
    pub fn start(&self, region: &str, upstream: &str) -> Result<u16> {
        let wanted = exit_regions::normalize(region);
        {
            let mut g = self.inner.lock();
            g.region = wanted.clone();
            g.upstream = upstream.to_string();
            g.avoid_replay = false;
            g.local_socks_port = engine::CHAIN_SOCKS_PORT;
        }
        // حالت هدایتِ این نشست از صفر شروع می‌شود.
        *self.shared.available_regions.lock() = Vec::new();
        psiphon_health::reset();

        if !self.is_available() {
            return Err(anyhow!(
                "The Psiphon stage is missing from this installation ({}). Reinstall Dopamine or switch the transport back to Dopamine.",
                self.missing_parts()
            ));
        }

        let attempts: Vec<String> = if wanted.is_empty() {
            vec![String::new()]
        } else {
            vec![wanted.clone(), String::new()]
        };
        let mut last_error: Option<String> = None;

        for (index, egress) in attempts.iter().enumerate() {
            if index > 0 {
                DiagnosticsLog::w(
                    TAG,
                    &format!(
                        "Psiphon could not establish in {} ({}) — retrying with an automatic exit and a fresh datastore.",
                        exit_regions::name(&wanted),
                        last_error.as_deref().unwrap_or("timed out")
                    ),
                );
                self.stop_quietly();
                self.reset_data_store();
                let port = self.inner.lock().local_socks_port;
                engine::wait_for_port_release(port, Duration::from_millis(PORT_RELEASE_WAIT_MS));
            }
            match self.establish(egress) {
                Ok(port) => return Ok(port),
                Err(e) => {
                    let message = e.to_string();
                    self.stop_quietly();
                    // پاس دوم فقط فیلتر کشور را شل می‌کند و datastore را تازه
                    // می‌کند؛ هیچ‌کدام یک آرگومان یا کانفیگ نامعتبر را درست
                    // نمی‌کند. پس بلافاصله با همان پیام دقیق بیرون می‌رویم.
                    if is_config_fault(&message) {
                        DiagnosticsLog::e(TAG, &message);
                        return Err(e);
                    }
                    last_error = Some(message);
                }
            }
        }

        Err(anyhow!(
            "{}",
            last_error.unwrap_or_else(|| "Psiphon did not establish a tunnel".to_string())
        ))
    }

    /// یک تلاش برقراری: کانفیگ بنویس، فرآیند را اجرا کن، منتظر تونل بمان.
    fn establish(&self, egress: &str) -> Result<u16> {
        let config = self.build_config(egress);
        // مسیر فایل اجرایی اینجا لازم نیست: `spawn_child` خودش آن را از
        // همین `inner` می‌خواند، پس نگه‌داشتن یک کپی فقط نوفه بود.
        let (local_port, upstream) = {
            let mut g = self.inner.lock();
            g.config = config.clone();
            (g.local_socks_port, g.upstream.clone())
        };
        DiagnosticsLog::i(
            TAG,
            &format!(
                "Starting Psiphon (exit={}, socks={}, upstream={})",
                exit_regions::name(egress),
                local_port,
                if upstream.is_empty() {
                    "direct".to_string()
                } else {
                    redact_upstream(&upstream)
                },
            ),
        );
        // واچ‌داگ به این کنترلر وصل می‌شود: بودجهٔ چرخش برای هر نشست است و
        // کتابخانه با re-establish خودش نمی‌تواند آن را ریست کند.
        self.bind_health();
        self.spawn_child(&config)?;

        let deadline = Instant::now() + Duration::from_millis(ESTABLISH_TIMEOUT_MS);
        loop {
            if self.shared.connected.load(Ordering::Relaxed) {
                break;
            }
            if self.shared.exiting.load(Ordering::Relaxed) {
                return Err(self.fault_error("Psiphon stopped before it established a tunnel"));
            }
            if !self.child_alive() {
                return Err(
                    self.fault_error("The Psiphon stage exited before it established a tunnel")
                );
            }
            if Instant::now() >= deadline {
                return Err(anyhow!(
                    "Psiphon found no usable server within {}s",
                    ESTABLISH_TIMEOUT_MS / 1000
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // پورتی که واقعاً گرفته شده، نه پورتی که خواسته بودیم.
        let bound = self.socks_port();
        if bound != local_port {
            DiagnosticsLog::w(
                TAG,
                &format!("Psiphon bound SOCKS5 on {bound} instead of {local_port} — the pipeline follows it."),
            );
            self.inner.lock().local_socks_port = bound;
        }
        // حقیقتِ زمینی: لیسنر باید واقعاً قابل اتصال باشد. یک تونل «متصل» که
        // پورتش باز نیست، خط لولهٔ مرده است.
        if !probe::socks_ready(bound) {
            return Err(anyhow!(
                "Psiphon reported a tunnel but nothing is listening on 127.0.0.1:{bound}"
            ));
        }
        DiagnosticsLog::i(
            TAG,
            &format!(
                "Psiphon stage ready on 127.0.0.1:{bound} (exit region {})",
                self.exit_region()
                    .unwrap_or_else(|| "automatic".to_string())
            ),
        );
        Ok(bound)
    }

    /// خطای این تلاش: اگر ترد notice یک خطای پیکربندی دیده، **همان** گزارش
    /// می‌شود (با پیشوند [CONFIG_FAULT_PREFIX])، وگرنه جملهٔ عمومی.
    ///
    /// چرا مهم است: «The Psiphon stage exited before it established a tunnel»
    /// دقیقاً همان پیامی است که کاربر در لاگ دید و هیچ چیزی درباره‌اش نمی‌گفت.
    fn fault_error(&self, fallback: &str) -> anyhow::Error {
        let fault = self.shared.config_fault.lock().clone();
        if fault.is_empty() {
            anyhow!("{}", fallback)
        } else {
            anyhow!("{}{}", CONFIG_FAULT_PREFIX, fault)
        }
    }

    /// کانفیگ JSON این نشست.
    fn build_config(&self, egress: &str) -> String {
        build_config(&self.inner, egress)
    }

    /// فرآیند ConsoleClient را اجرا می‌کند.
    fn spawn_child(&self, config: &str) -> Result<()> {
        spawn_child(&self.inner, &self.shared, config)
    }

    /// چرخش را به واچ‌داگ وصل می‌کند.
    fn bind_health(&self) {
        let inner = self.inner.clone();
        let shared = self.shared.clone();
        psiphon_health::bind(move |strategy, target| {
            rotate(&inner, &shared, strategy, target);
        });
    }

    fn child_alive(&self) -> bool {
        let mut g = self.inner.lock();
        match g.child.as_mut() {
            Some(c) => matches!(c.try_wait(), Ok(None)),
            None => false,
        }
    }

    /// «زنده» یعنی **قابل‌استفاده**، نه اینکه فرآیند وجود دارد.
    pub fn is_alive(&self) -> bool {
        // یک چرخشِ عمدی، مرگ نیست (نگاه کنید به [Shared::rotating]).
        if self.shared.rotating() {
            return true;
        }
        self.shared.connected.load(Ordering::Relaxed) && self.child_alive()
    }

    /// خاتمهٔ بی‌صدا بین دو تلاش — بدون پاک‌کردن حالت واچ‌داگ.
    fn stop_quietly(&self) {
        self.shared.connected.store(false, Ordering::Relaxed);
        // نسل را جلو می‌بریم تا خطوط در راهِ فرآیند مرده بی‌اثر شوند.
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
        let child = self.inner.lock().child.take();
        if let Some(mut c) = child {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    /// datastore را پاک می‌کند — کلاسیک‌ترین علتِ Psiphon ای که هرگز برقرار
    /// نمی‌شود و نمی‌تواند بگوید چرا.
    fn reset_data_store(&self) {
        let dir = self.inner.lock().data_dir.clone();
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        DiagnosticsLog::i(TAG, "Psiphon datastore reset.");
    }

    /// پایان نشست. شمارنده‌ها و بودجهٔ چرخش هرگز نباید به نشست بعدی سر بزنند.
    pub fn stop(&self) {
        self.shared.rotation_deadline.store(0, Ordering::Relaxed);
        psiphon_health::reset();
        self.stop_quietly();
        *self.shared.connected_region.lock() = String::new();
        self.shared.socks_port.store(0, Ordering::Relaxed);
        DiagnosticsLog::w(TAG, "Psiphon stage stopped and reaped.");
    }
}

impl Drop for PsiphonTransport {
    fn drop(&mut self) {
        self.stop_quietly();
    }
}

/// نشست را روی سرور دیگری می‌برد **بدون** پایین‌آوردن خط لولهٔ برنامه.
///
/// در اندروید این کار با `reconnectPsiphon()` / `restartPsiphon()` انجام می‌شد.
/// اینجا Psiphon یک فرآیند است، پس هر دو حالت یک راه‌اندازی مجدد فرآیندند و
/// تفاوت معنادار همان می‌ماند: `Restart` کانفیگ هدایت‌شده را می‌نویسد.
///
/// هیچ چیز دیگری تکان نمی‌خورد: پل، پروکسی سیستمی، گارد نشتی و Wintun همه سر
/// جای خودشان می‌مانند و کاربر یک وقفهٔ کوتاه می‌بیند، نه یک قطعی.
fn rotate(inner: &Arc<Mutex<Inner>>, shared: &Arc<Shared>, strategy: Rotation, target: Target) {
    // پنجرهٔ مهلت **پیش** از هر تخریبی گرفته می‌شود، تا سوپروایزر هرگز نتواند
    // فاصلهٔ بین «کشتن» و «بالا آمدن» را ببیند.
    shared
        .rotation_deadline
        .store(now_ms() + ROTATION_GRACE_MS, Ordering::Relaxed);

    // The warm latency session was dialled through the exit we are leaving. If it
    // survived the rotation the badge would report the abandoned server's timeout
    // as the user's ping — the mobile edition drops it at exactly this point too.
    crate::ping::reset();

    if strategy == Rotation::Restart {
        prepare_steered_config(inner, shared, &target);
    }

    let config = inner.lock().config.clone();
    // فرآیند قدیمی می‌رود…
    {
        shared.connected.store(false, Ordering::Relaxed);
        shared.generation.fetch_add(1, Ordering::SeqCst);
        let child = inner.lock().child.take();
        if let Some(mut c) = child {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
    let port = inner.lock().local_socks_port;
    engine::wait_for_port_release(port, Duration::from_millis(PORT_RELEASE_WAIT_MS));

    // …و جانشین با همان پورت بالا می‌آید، پس پل و پروکسی سیستمی هیچ‌وقت
    // نمی‌فهمند چیزی عوض شده.
    match spawn_child(inner, shared, &config) {
        Ok(()) => DiagnosticsLog::i(
            TAG,
            &format!(
                "Psiphon restarted for rotation ({}) — the pipeline stayed up on 127.0.0.1:{port}.",
                strategy_label(strategy)
            ),
        ),
        Err(e) => {
            DiagnosticsLog::e(TAG, &format!("Psiphon server rotation failed: {e}"));
            // چرخشی که حتی صادر نشد نباید نشست را «زنده» پین کند.
            shared.rotation_deadline.store(0, Ordering::Relaxed);
        }
    }
}

fn strategy_label(s: Rotation) -> &'static str {
    match s {
        Rotation::Reconnect => "same config",
        Rotation::Restart => "steered config",
    }
}

/// کانفیگ را بازنویسی می‌کند تا کنترلرِ راه‌اندازی‌شده نتواند دوباره روی همان
/// سروری بنشیند که همین حالا ترکش داده.
///
/// psiphon-tunnel-core هیچ کلید «این سرورها را نگیر» ندارد، و از دید خودش آن
/// سرور فیلترکننده عالی است: تونل برقرار شد، دست‌دادن گذشت، بایت جابه‌جا شد. پس
/// استثنا از سه اهرمی ساخته می‌شود که **وجود دارند**: `DisableReplay`،
/// `EstablishTunnelServerAffinityGracePeriodMilliseconds` و `EgressRegion`.
fn prepare_steered_config(inner: &Arc<Mutex<Inner>>, shared: &Arc<Shared>, target: &Target) {
    let user_egress = {
        let mut g = inner.lock();
        g.avoid_replay = true;
        g.region.clone()
    };
    let egress = if !user_egress.is_empty() {
        DiagnosticsLog::i(
            TAG,
            &format!(
                "Psiphon staying in {} because it was chosen by hand — replay and server affinity \
                 are disabled instead so establishment cannot settle on {} again.",
                exit_regions::name(&user_egress),
                if target.server_id.is_empty() {
                    "(unknown)"
                } else {
                    &target.server_id
                }
            ),
        );
        user_egress
    } else {
        next_egress_away(shared, &target.region)
    };
    let config = build_config(inner, &egress);
    inner.lock().config = config;
}

/// منطقهٔ خروج بعدیِ در‌دسترس که خانهٔ سرور محکوم‌شده نباشد.
///
/// نوبتی و نه تصادفی، تا دو چرخش پشت‌سرهم نشست را به جایی که از آن آمده
/// برنگردانند؛ و اگر چیز بهتری نماند `""` برمی‌گردد — برقراری بدون فیلتر همیشه
/// از برقراری پین‌شده به منطقه‌ای که سرور ندارد بهتر است.
fn next_egress_away(shared: &Arc<Shared>, bad_region: &str) -> String {
    let mut bad = psiphon_health::filtering_regions();
    if !bad_region.is_empty() {
        bad.insert(bad_region.to_ascii_uppercase());
    }
    let available = shared.available_regions.lock().clone();
    let pool: Vec<String> = available.into_iter().filter(|r| !bad.contains(r)).collect();
    if pool.is_empty() {
        DiagnosticsLog::w(
            TAG,
            "Psiphon has no clean exit region left to steer to — restarting with replay and \
             server affinity disabled and no region filter.",
        );
        return String::new();
    }
    static CURSOR: AtomicU64 = AtomicU64::new(0);
    let idx = (CURSOR.fetch_add(1, Ordering::Relaxed) as usize) % pool.len();
    let pick = pool[idx].clone();
    DiagnosticsLog::i(
        TAG,
        &format!(
            "Psiphon steering the exit away from {} to {} — {} filtering, {} clean regions to choose from.",
            exit_regions::name(bad_region),
            exit_regions::name(&pick),
            if bad.len() == 1 {
                "1 region is".to_string()
            } else {
                format!("{} regions are", bad.len())
            },
            pool.len(),
        ),
    );
    pick
}

/// یک سطر notice: اول واچ‌داگ، بعد فیلتر نوفه، بعد لاگ.
///
/// ترتیب امنیتی است نه سلیقه‌ای: فیلترکردن یک نگرانیِ **لاگ** است و هرگز نباید
/// چیزی را که واچ‌داگ اجازه دارد ببیند عوض کند.
fn handle_notice(shared: &Arc<Shared>, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    psiphon_health::on_notice(line);

    let kind = psiphon_health::notice_type(line).unwrap_or_default();
    let message = psiphon_health::json_str(line, "message").unwrap_or_default();

    match kind.as_str() {
        // پورتی که واقعاً گرفته شد. دنبال‌کردن این عدد (به‌جای دیکته‌کردنش)
        // همان چیزی است که یک نشست زنجیره‌ای را ممکن می‌کند.
        "ListeningSocksProxyPort" => {
            if let Some(port) = psiphon_health::json_u64(line, "port") {
                shared.socks_port.store(port as u16, Ordering::Relaxed);
            }
        }
        // معادل `onConnected` / `onExiting` اندروید.
        "Tunnels" => {
            let count = psiphon_health::json_u64(line, "count").unwrap_or(0);
            if count >= 1 {
                shared.connected.store(true, Ordering::Relaxed);
                // چرخش تمام شد: تظاهر بس است، ترابرد واقعاً برگشته.
                if shared.rotation_deadline.swap(0, Ordering::Relaxed) != 0 {
                    DiagnosticsLog::i(
                        TAG,
                        "Psiphon rotation complete — session continues on a new server.",
                    );
                }
            } else {
                // عمداً `connected` را پاک **نمی‌کنیم** اگر چرخشی در جریان است:
                // این همان لحظه‌ای است که سوپروایزر اندروید نشست سالم را
                // پایین می‌آورد.
                if !shared.rotating() {
                    shared.connected.store(false, Ordering::Relaxed);
                }
            }
        }
        "Exiting" => {
            shared.exiting.store(true, Ordering::Relaxed);
            shared.connected.store(false, Ordering::Relaxed);
        }
        // فقط منطقه‌ای که کتابخانه گفته می‌تواند به آن برسد مجاز به هدایت است:
        // `EgressRegion` فیلتر سخت است و هدایت به منطقهٔ خالی، برقراری را
        // معلق می‌کند نه اینکه چیزی را درست کند.
        "AvailableEgressRegions" => {
            let list = json_string_array(line, "regions");
            if !list.is_empty() {
                *shared.available_regions.lock() = list.clone();
                DiagnosticsLog::i(
                    TAG,
                    &format!("Psiphon egress regions available: {}", list.join(", ")),
                );
            }
        }
        // نام دقیق فیلد بین نسخه‌های psiphon-tunnel-core یکسان نبوده، پس هر سه
        // شکل شناخته‌شده پذیرفته می‌شود: نشانِ کشور نباید به یک تغییر نام
        // بی‌ضرر در بالادست حساس باشد.
        "ConnectedServerRegion" => {
            if let Some(region) = psiphon_health::json_str(line, "serverRegion")
                .or_else(|| psiphon_health::json_str(line, "regionCode"))
                .or_else(|| psiphon_health::json_str(line, "region"))
            {
                *shared.connected_region.lock() = region.to_ascii_uppercase();
            }
        }
        _ => {}
    }

    // خطای پیکربندی/آرگومان: فرآیند همین حالا تمام می‌شود و هیچ تلاش مجددی
    // نجاتش نمی‌دهد. زودتر و **دقیق** شکست خوردن، تنها رفتار درست است.
    if let Some(hint) = config_fault_hint(&kind, &message) {
        {
            let mut slot = shared.config_fault.lock();
            if slot.is_empty() {
                *slot = if message.is_empty() {
                    hint.to_string()
                } else {
                    format!("{hint} Psiphon said: {message}")
                };
            }
        }
        shared.exiting.store(true, Ordering::Relaxed);
        shared.connected.store(false, Ordering::Relaxed);
        DiagnosticsLog::e(TAG, &format!("{CONFIG_FAULT_PREFIX}{hint}"));
    }

    if is_noise_notice(&kind, &message) {
        shared.noise_notices.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let suppressed = shared.noise_notices.swap(0, Ordering::Relaxed);
    if suppressed > 0 {
        DiagnosticsLog::d(
            TAG,
            &format!("(+{suppressed} routine server-list notices suppressed)"),
        );
    }
    let body = if message.is_empty() { line } else { &message };
    match kind.as_str() {
        "Error" | "Alert" | "LocalProxyError" | "Exiting" => {
            DiagnosticsLog::w(TAG, &format!("{kind}: {body}"))
        }
        _ => DiagnosticsLog::d(TAG, &format!("{kind}: {body}")),
    }
}

/// آیا این notice یعنی «استیج ۲ راه‌اندازی خودش را رد کرد»؟
///
/// فقط شکل‌هایی که **قطعاً** پیکربندی‌اند اینجا هستند. هر چیزی که ممکن است
/// شبکه باشد (سرور پیدا نشد، پروکسی بالادست جواب نداد، datastore قفل بود)
/// عمداً بیرون مانده تا نردبان و پاس دومِ [PsiphonTransport::start] کار خودشان
/// را بکنند — یک شکست شبکه‌ای با تلاش دوباره واقعاً درست می‌شود.
fn config_fault_hint(kind: &str, message: &str) -> Option<&'static str> {
    if kind != "Error" && kind != "Alert" {
        return None;
    }
    let m = message.to_ascii_lowercase();
    if m.contains("error getting listener ip") {
        // همان باگی که `child_args` مستندش می‌کند: نام اینترفیس، نه آدرس IP.
        return Some(
            "it was pointed at a network interface that does not exist on this PC, so it exited              before it could open its SOCKS5 port. This is a launch-argument fault, not a network              problem — please report it with the diagnostics log.",
        );
    }
    if m.contains("error loading configuration file")
        || m.contains("error processing configuration file")
        || m.contains("invalid config")
    {
        return Some(
            "it refused its own configuration file and exited. This is a packaging fault, not a              network problem — reinstall Dopamine and report it with the diagnostics log.",
        );
    }
    None
}

/// notice هایی که خالص دفترداری‌اند و هرگز نباید به لاگ برسند.
///
/// ریشه‌ای که این رفع می‌کند: لاگ تشخیصی یک رینگ کراندارِ ۸۰۰ سطری است. در
/// نمونهٔ میدانی موبایل **۳۹۳ از ۴۷۸ سطر** notice های Psiphon بودند و اکثریت
/// قاطعشان `updated server <8 chars>` — یک سطر به‌ازای هر ورودی سرور در یک
/// به‌روزرسانی روتین، صدها سطر در چند ثانیه. سطرهای مسیر دادهٔ خود موتور ۱۸ سطر
/// از ۴۷۸ بودند. یعنی در پنجره‌ای که مشکل رخ می‌دهد **هیچ تله‌متری‌ای وجود
/// نداشت**. حذف این نوفه همان چیزی است که رینگ را برای یک نشست کامل نگه می‌دارد.
///
/// فقط شکل‌هایی که بی‌قید‌و‌شرط بی‌فایده‌اند اینجا هستند. هر چیزی که تشخیصی
/// حمل می‌کند — خطا، هشدار، رد‌شدن، چرخش، شکست port forward — دست‌نخورده است، و
/// [crate::psiphon_health] در هر حال **همهٔ** notice ها را می‌بیند چون پیش از
/// این فیلتر تغذیه می‌شود.
fn is_noise_notice(kind: &str, message: &str) -> bool {
    if kind == "BytesTransferred" || (kind == "Info" && message.is_empty()) {
        return true;
    }
    kind == "Info"
        && (message.starts_with("updated server ")
            || message.starts_with("discarding server ")
            || message.starts_with("ServerEntryIterator.reset")
            || message.starts_with("Set dial parameters for "))
}

/// آرایهٔ رشته‌ایِ یک کلید JSON — برای `AvailableEgressRegions`.
fn json_string_array(line: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let Some(at) = line.find(&needle) else {
        return Vec::new();
    };
    let rest = &line[at + needle.len()..];
    let Some(open) = rest.find('[') else {
        return Vec::new();
    };
    let Some(close) = rest[open..].find(']') else {
        return Vec::new();
    };
    rest[open + 1..open + close]
        .split(',')
        .filter_map(|piece| {
            let cc = piece.trim().trim_matches('"').trim().to_ascii_uppercase();
            if cc.len() == 2 && cc.chars().all(|c| c.is_ascii_alphabetic()) {
                Some(cc)
            } else {
                None
            }
        })
        .collect()
}

/// آدرس پروکسی بالادست برای لاگ — اگر روزی احراز هویت داشت، لو نرود.
/// همان قاعدهٔ `redact_args` در `engine.rs`.
fn redact_upstream(raw: &str) -> String {
    match raw.split_once("://") {
        Some((scheme, rest)) => match rest.rsplit_once('@') {
            Some((_, endpoint)) => format!("{scheme}://••••••@{endpoint}"),
            None => raw.to_string(),
        },
        None => raw.to_string(),
    }
}

/// کانفیگ JSON — هر کلید همان کلیدی است که نسخهٔ موبایل می‌فرستد.
///
/// کلیدهای ناشناخته را سمت Go بی‌صدا نادیده می‌گیرد، ولی هیچ کلید «الکی»
/// اینجا نیست.
///
/// تابع آزاد است و نه متد، چون [rotate] هم باید بتواند صدایش بزند و آن مسیر
/// فقط `Arc` ها را در دست دارد، نه یک [PsiphonTransport].
fn build_config(inner: &Mutex<Inner>, egress: &str) -> String {
    let g = inner.lock();
    let mut cfg = json!({
        "PropagationChannelId": "FFFFFFFFFFFFFFFF",
        "SponsorId": "1111111111111111",
        "EstablishTunnelTimeoutSeconds": ESTABLISH_TIMEOUT_SECONDS,
        "DataRootDirectory": g.data_dir.to_string_lossy(),
        "ClientVersion": "127",
        "LocalSocksProxyPort": g.local_socks_port,
        // پروکسی HTTP محلی اینجا وزن مرده است: پل، پروکسی سیستمی و خودآزما همه
        // SOCKS5 حرف می‌زنند، و یک لیسنر کمتر یعنی یک تداخل پورت کمتر با موتور
        // و با پل اشتراک.
        "DisableLocalHTTPProxy": true,
        "RemoteServerListSignaturePublicKey": REMOTE_SERVER_LIST_PUBLIC_KEY,
        "ServerEntrySignaturePublicKey": SERVER_ENTRY_SIGNATURE_PUBLIC_KEY,
        "ExchangeObfuscationKey": EXCHANGE_OBFUSCATION_KEY,
        // برخلاف موبایل عمداً خاموش: هیچ چیزی در ویندوز این شمارنده را مصرف
        // نمی‌کند (rx/tx از پل و Wintun می‌آید) و یک notice در ثانیه همان رینگ
        // ۸۰۰ سطری لاگ را می‌خورد که مستند موبایل نشان داد ۳۹۳ از ۴۷۸ سطرش
        // نوفهٔ Psiphon شده بود.
        "EmitBytesTransferred": false,
        "EmitDiagnosticNotices": true,
        "DeviceRegion": "IR",
        "ConnectionWorkerPoolSize": 12,
        "DNSResolverPreferredAlternateServers": ["1.1.1.1:53", "8.8.8.8:53", "9.9.9.9:53"],
        "DNSResolverPreferAlternateServerProbability": 1.0,
    });
    let map = cfg.as_object_mut().expect("config root is an object");
    // `EgressRegion` وقتی کاربر «خودکار» خواسته **حذف** می‌شود، نه اینکه `""`
    // فرستاده شود — همان راه مستندشدهٔ «بدون فیلتر منطقه».
    if !egress.is_empty() {
        map.insert("EgressRegion".into(), json!(egress));
    }
    if g.avoid_replay {
        // replay آخرین سرور را با پارامترهای dial به‌خاطر‌سپرده دوباره می‌زند و
        // آن را candidate 0 می‌کند (`isReplay: true`). این نیمی از دلیلی بود که
        // چرخش مدام به همان سروری برمی‌گشت که ترکش کرده بود.
        map.insert("DisableReplay".into(), json!(true));
        // نیمهٔ دیگر server affinity است: سرور قبلی یک آغازِ انحصاری می‌گیرد
        // پیش از آنکه بقیهٔ کارگرها اجازهٔ مسابقه داشته باشند.
        map.insert(
            "EstablishTunnelServerAffinityGracePeriodMilliseconds".into(),
            json!(0),
        );
    }
    // امنیت/مسیریابی: با پروکسی بالادست، psiphon-tunnel-core **هر** اتصالی را
    // که می‌سازد از آن می‌برد، از جمله واکشی فهرست سرور، پس زنجیره نمی‌تواند
    // یک dial مستقیم لو بدهد.
    if !g.upstream.is_empty() {
        map.insert("UpstreamProxyUrl".into(), json!(g.upstream.clone()));
    }
    cfg.to_string()
}

/// آرگومان‌های خط فرمان استیج ۲ — تابعِ خالص، تا یک تست بتواند قفلش کند.
///
/// # ⚠ چرا `-listenInterface` اینجا نیست، و هرگز نباید برگردد
///
/// **این ریشهٔ باگِ «حالت ترکیبی اِتِر + سایفون در ویندوز کانکت نمی‌شود» بود.**
/// قبلاً `-listenInterface 127.0.0.1` فرستاده می‌شد، با این نیت که «استیج ۲ فقط
/// روی لوپ‌بک بشنود». ولی `ListenInterface` در psiphon-tunnel-core یک **نام
/// اینترفیس** می‌گیرد، نه یک آدرس IP. کنترلر این‌طور تفسیرش می‌کند:
///
/// ```text
///   ListenInterface == ""     → listenIP = 127.0.0.1      ← همان چیزی که می‌خواستیم
///   ListenInterface == "any"  → listenIP = 0.0.0.0
///   هر چیز دیگر               → net.InterfaceByName(<آن رشته>)
/// ```
///
/// روی ویندوز هیچ اینترفیسی به نام «127.0.0.1» وجود ندارد، پس `Controller.Run`
/// در همان اولین ثانیه با
///
/// ```text
///   Error: error getting listener IP: psiphon.(*Controller).Run#424:
///          common.GetInterfaceIPAddresses#38: route ip+net: no such network interface
/// ```
///
/// برمی‌گشت — **پیش از** اینکه هیچ لیسنری ساخته شود. استیج ۱ (MASQUE) سالم بالا
/// می‌آمد، `Stage 1 is a working SOCKS5 proxy` هم لاگ می‌شد، و بعد فرآیند
/// Psiphon بی‌صدا تمام می‌شد؛ نردبان Smart Auto هم چهار پله را با همان خطای
/// تکراری می‌سوزاند و کاربر فقط «کانکت نشد» می‌دید. نسخهٔ موبایل هرگز این را
/// نداشت چون `PsiphonTransport.kt` کتابخانه را بدون `ListenInterface` صدا
/// می‌زند و کتابخانه خودش روی ۱۲۷.۰.۰.۱ می‌نشیند.
///
/// پس رفعِ ریشه‌ای **حذف** آن آرگومان است، نه عوض‌کردن مقدارش: مقدار پیش‌فرض
/// (خالی) عیناً همان لوپ‌بک-only بودن را می‌دهد که هدف اصلی بود. اگر روزی
/// واقعاً لازم شد، تنها مقادیر مجاز نام اینترفیس یا `any` است — و `any` برای
/// استیج ۲ ممنوع است، چون اشتراک LAN کارِ [crate::share] با سیاست خودش است.
fn child_args(config_path: &Path, server_list: &Path, data_dir: &Path) -> Vec<std::ffi::OsString> {
    vec![
        "-config".into(),
        config_path.as_os_str().to_os_string(),
        "-serverList".into(),
        server_list.as_os_str().to_os_string(),
        "-dataRootDirectory".into(),
        data_dir.as_os_str().to_os_string(),
    ]
}

/// فرآیند ConsoleClient را اجرا و تردهای خوانندهٔ notice را بالا می‌آورد.
fn spawn_child(inner: &Arc<Mutex<Inner>>, shared: &Arc<Shared>, config: &str) -> Result<()> {
    let (exe, server_list, data_dir, config_path) = {
        let g = inner.lock();
        (
            g.exe.clone(),
            g.server_list.clone(),
            g.data_dir.clone(),
            g.config_path.clone(),
        )
    };
    std::fs::create_dir_all(&data_dir)?;
    std::fs::write(&config_path, config)?;

    // نسل تازه: هر خطی که از فرآیند قبلی برسد از این لحظه دور ریخته می‌شود.
    let generation = shared.generation.fetch_add(1, Ordering::SeqCst) + 1;
    shared.connected.store(false, Ordering::Relaxed);
    shared.exiting.store(false, Ordering::Relaxed);
    shared.socks_port.store(0, Ordering::Relaxed);
    // تشخیص خطای پیکربندی مال **این** فرآیند است، نه ارثِ فرآیند قبلی.
    shared.config_fault.lock().clear();

    let mut cmd = Command::new(&exe);
    cmd.args(child_args(&config_path, &server_list, &data_dir))
        .current_dir(&data_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // مثل موتور: پنجرهٔ کنسول نباید جلوی کاربر بالا بیاید.
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow!("Could not start {}: {e}", exe.display()))?;

    // notice ها روی stderr می‌آیند؛ stdout هم خوانده می‌شود تا لوله هرگز پر
    // نشود و فرآیند روی یک write بلاک نکند (همان درسِ spawn_drain موتور).
    if let Some(err) = child.stderr.take() {
        spawn_notice_reader(shared, err, generation);
    }
    if let Some(out) = child.stdout.take() {
        spawn_notice_reader(shared, out, generation);
    }
    inner.lock().child = Some(child);
    Ok(())
}

fn spawn_notice_reader<R>(shared: &Arc<Shared>, reader: R, generation: u64)
where
    R: std::io::Read + Send + 'static,
{
    let shared = shared.clone();
    std::thread::Builder::new()
        .name("psiphon-notice".into())
        .spawn(move || {
            for line in BufReader::new(reader).lines().map_while(Result::ok) {
                // خروجی یک فرآیند بازنشسته نباید وضعیت جانشینش را دست بزند.
                if shared.generation.load(Ordering::Relaxed) != generation {
                    return;
                }
                handle_notice(&shared, &line);
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared() -> Arc<Shared> {
        Arc::new(Shared::new())
    }

    /// پورتِ گزارش‌شده باید دنبال شود، نه دیکته.
    #[test]
    fn follows_the_reported_socks_port() {
        let s = shared();
        handle_notice(
            &s,
            r#"{"data":{"port":1825},"noticeType":"ListeningSocksProxyPort"}"#,
        );
        assert_eq!(s.socks_port.load(Ordering::Relaxed), 1825);
    }

    #[test]
    fn tunnels_notice_drives_connected() {
        let s = shared();
        assert!(!s.connected.load(Ordering::Relaxed));
        handle_notice(&s, r#"{"data":{"count":1},"noticeType":"Tunnels"}"#);
        assert!(s.connected.load(Ordering::Relaxed));
        handle_notice(&s, r#"{"data":{"count":0},"noticeType":"Tunnels"}"#);
        assert!(!s.connected.load(Ordering::Relaxed));
    }

    /// ریشهٔ «خودش یک‌دفعه قطع می‌شود»: در طول چرخش، `Tunnels: 0` نباید
    /// نشست را مرده اعلام کند.
    #[test]
    fn rotation_grace_survives_a_tunnels_zero() {
        let s = shared();
        handle_notice(&s, r#"{"data":{"count":1},"noticeType":"Tunnels"}"#);
        s.rotation_deadline
            .store(now_ms() + ROTATION_GRACE_MS, Ordering::Relaxed);
        handle_notice(&s, r#"{"data":{"count":0},"noticeType":"Tunnels"}"#);
        assert!(
            s.connected.load(Ordering::Relaxed),
            "an in-flight rotation must not read as death"
        );
    }

    #[test]
    fn parses_available_egress_regions() {
        let s = shared();
        handle_notice(
            &s,
            r#"{"data":{"regions":["AT","BE","DE"]},"noticeType":"AvailableEgressRegions"}"#,
        );
        assert_eq!(
            *s.available_regions.lock(),
            vec!["AT".to_string(), "BE".to_string(), "DE".to_string()]
        );
    }

    #[test]
    fn routine_server_list_notices_are_noise() {
        assert!(is_noise_notice("Info", "updated server abcdefgh"));
        assert!(is_noise_notice("Info", "discarding server abcdefgh"));
        assert!(is_noise_notice("BytesTransferred", ""));
        // هر چیزی که تشخیصی حمل کند باید بماند.
        assert!(!is_noise_notice(
            "LocalProxyError",
            "ssh: rejected: administratively prohibited"
        ));
        assert!(!is_noise_notice(
            "Info",
            "port forward failures for abc: 12"
        ));
    }

    /// ریشهٔ باگ «اِتِر + سایفون کانکت نمی‌شود»: هرگز نباید به استیج ۲ گفته
    /// شود روی یک «اینترفیس» به‌نام یک آدرس IP بنشیند.
    #[test]
    fn stage_two_is_never_told_which_interface_to_bind() {
        let args = child_args(
            Path::new("C:\\data\\psiphon.config"),
            Path::new("C:\\data\\engine\\server_entries.txt"),
            Path::new("C:\\data\\psiphon"),
        );
        let flat: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(
            !flat.iter().any(|a| a == "-listenInterface"),
            "ListenInterface takes an interface name, not an IP; the default already binds \
             127.0.0.1 — passing anything here killed the Psiphon stage on startup"
        );
        assert!(flat.iter().any(|a| a == "-config"));
        assert!(flat.iter().any(|a| a == "-serverList"));
        assert!(flat.iter().any(|a| a == "-dataRootDirectory"));
        assert_eq!(flat.len(), 6, "exactly three flags with one value each");
    }

    /// همان سطری که در لاگ کاربر بود: باید یک خطای پیکربندی شناخته شود، فوراً
    /// نشست را تمام‌شده اعلام کند، و پیامش قابل‌عمل باشد.
    #[test]
    fn the_listener_ip_error_is_a_config_fault() {
        let s = shared();
        handle_notice(
            &s,
            r#"{"data":{"message":"error getting listener IP: psiphon.(*Controller).Run#424: common.GetInterfaceIPAddresses#38: route ip+net: no such network interface"},"noticeType":"Error","timestamp":"2026-09-04T05:54:19.510Z"}"#,
        );
        let fault = s.config_fault.lock().clone();
        assert!(!fault.is_empty(), "the fault must be recorded");
        assert!(s.exiting.load(Ordering::Relaxed), "no retry can fix this");
        assert!(is_config_fault(&format!("{CONFIG_FAULT_PREFIX}{fault}")));
    }

    /// و برعکس: یک شکست شبکه‌ای هرگز نباید نردبان را کوتاه کند.
    #[test]
    fn a_network_error_is_not_a_config_fault() {
        let s = shared();
        handle_notice(
            &s,
            r#"{"data":{"message":"ssh: rejected: administratively prohibited"},"noticeType":"LocalProxyError"}"#,
        );
        handle_notice(
            &s,
            r#"{"data":{"message":"upstreamproxy error: dial tcp 127.0.0.1:1819: connect: connection refused"},"noticeType":"Error"}"#,
        );
        assert!(s.config_fault.lock().is_empty());
        assert!(!s.exiting.load(Ordering::Relaxed));
        assert!(!is_config_fault(
            "Psiphon found no usable server within 200s"
        ));
    }

    /// راز پروکسی بالادست هرگز نباید در لاگ ماندگار بنشیند.
    #[test]
    fn upstream_credentials_are_redacted() {
        assert_eq!(
            redact_upstream("socks5://user:pass@127.0.0.1:1819"),
            "socks5://••••••@127.0.0.1:1819"
        );
        assert_eq!(
            redact_upstream("socks5://127.0.0.1:1819"),
            "socks5://127.0.0.1:1819"
        );
    }
}
