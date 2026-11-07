//! تور به‌عنوان یک فرزندِ نظارت‌شده: `tor.exe` رسمی، torrc، و پورتِ کنترل.
//!
//! # چرا این ماژول وجود دارد
//!
//! تا ۱.۲.۵ مسیرِ تورِ برنامه از `aether.exe --tor-only` می‌گذشت، و آن مسیر
//! روی arti (کتابخانهٔ تورِ Rust) سوار است. لاگِ میدانیِ ۱۶ سپتامبر ۲۰۲۶
//! (`loge1.txt`) نشان می‌دهد این مسیر در ایران به مقصد نمی‌رسد و روی ۱۵٪
//! می‌ماند، و سه محدودیتِ arti در همان لاگ دیده می‌شود:
//!
//! * arti برای هر پل «هویت» می‌خواهد. سطرِ `meek_lite` که خودِ Tor منتشر
//!   می‌کند فینگرپرینت ندارد، پس در arti دو میلی‌ثانیه‌ای می‌سوزد:
//!   «none of the bridges could be read by tor».
//! * فهرستِ پل‌های داخلی را ما دستی نگه می‌داشتیم. `pt_config.json`ی که در
//!   بستهٔ رسمیِ Tor است `fronts=app.datapacket.com,www.datapacket.com` دارد
//!   — جمع، همان چیزی که lyrebird 0.6.1 می‌فهمد — و ما مجبور بودیم آن را
//!   برای arti تغییر دهیم.
//! * هر سپرهٔ ناکام، یک کلاینتِ کاملِ arti بود که نمی‌مُرد.
//!
//! پس روشِ تور را عوض می‌کنیم، نه تنظیماتش را: همان چیزی که مرورگر تور
//! اجرا می‌کند — `tor.exe` رسمی — به‌عنوان یک پروسهٔ فرزند، با یک torrc، و
//! پیشرفتش از پورتِ کنترلِ خودش خوانده می‌شود.
//!
//! روش، خط‌به‌خط از پروژهٔ WhiteAesther 1.9.4 (`src-tauri/src/tor.rs`) گرفته
//! شده؛ پروژه‌ای که هستهٔ Aether 2.0.0 را دارد و کانکشنِ تورش روی ویندوز کار
//! می‌کند. نکته‌هایی که آن‌ها با اندازه‌گیری به‌دست آورده‌اند و اینجا عیناً
//! حفظ شده‌اند، چون هرکدام یک باگِ ساعت‌خور است:
//!
//! * **مسیرِ lyrebird در `ClientTransportPlugin` نقل‌قول نمی‌شود.** تور برای
//!   گزینه‌های خودش نقل‌قول را برمی‌دارد ولی آرگومانِ `exec` را همان‌طور به
//!   CreateProcess می‌دهد؛ نقل‌قول‌شده، نتیجه‌اش
//!   `CreateProcessA() failed: The system cannot find the file specified`
//!   است و بعد هر پل با «there is no configured transport called obfs4»
//!   می‌افتد. اندازه‌گیری‌شده که بی‌نقل‌قول، حتی با فاصله در مسیر
//!   (`C:\Program Files`)، کار می‌کند.
//! * **بقیهٔ مسیرها نقل‌قول می‌شوند و بک‌اسلش‌هایشان دوبرابر.** تور در یک
//!   مقدارِ نقل‌قول‌شده بک‌اسلش را escape می‌خواند، پس `C:\Users\…` به مسیری
//!   با کاراکترِ کنترلی تبدیل می‌شود و تور فایل را رد می‌کند.
//! * **`SocksPort auto` و `ControlPort auto`.** پورتِ ثابت روی ماشینی که ما
//!   کنترلش نمی‌کنیم می‌تواند گرفته باشد، و آن خطا شبیهِ «تور بالا نمی‌آید»
//!   دیده می‌شود. پورتِ واقعی از خودِ تور پرسیده می‌شود.
//! * **`__OwningControllerProcess`.** بی‌آن، تور پس از مرگِ برنامه زنده
//!   می‌ماند و یک تونلِ بی‌مصرف باقی می‌گذارد.
//! * **گیتِ «وصل شد» فقط `PROGRESS=100` است.** تور پورتِ SOCKS را همان لحظهٔ
//!   شروع باز می‌کند، مدت‌ها پیش از داشتنِ مدار؛ پورتِ بازِ بی‌مدار اتصال را
//!   می‌پذیرد و بعد رویش می‌نشیند. این دقیقاً همان اشتباهی است که
//!   `tor_bootstrap.rs` برای مسیرِ arti مستند کرده است.
//! * **هر دو جریانِ خروجی خوانده می‌شوند**، وگرنه تور با پرشدنِ لوله روی
//!   لاگِ خودش قفل می‌کند — و همان لاگ تنها روایتِ خرابی‌هایی است که پیش از
//!   بالاآمدنِ پورتِ کنترل رخ می‌دهند.
//!
//! # آنچه عوض شده، و دلیلش
//!
//! * جای `AppHandle` تائوری، مسیرها مستقیم گرفته می‌شوند. این ماژول هیچ
//!   وابستگیِ تائوری ندارد تا همین کدی که روی ویندوز می‌رود، در آزمونِ
//!   میزبان هم ساخته و اجرا شود.
//! * سطرهای لاگ به یک کال‌بک می‌روند تا `log.rs` خودمان همان مسیرِ همیشگی را
//!   داشته باشد.
//! * فهرستِ پل‌های داخلی از `pt_config.json` خوانده می‌شود، نه از فهرستی که
//!   ما نگه می‌داریم. دلیلش را WhiteAesther نوشته و لاگِ خودمان تأییدش کرد:
//!   فهرستِ دستی می‌پوسد.

use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU16, AtomicU64, AtomicU8, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// مهلتِ ساختِ مدار.
///
/// دست‌ودل‌بازانه و به‌عمد: پلی که باید از راهِ یک ترابرِ افزودنی گرفته شود
/// معمولاً چند ده ثانیه می‌برد. کوتاه‌کردنش یعنی اعلامِ شکست روی شبکه‌ای که
/// تور در آن موفق می‌شد.
const BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(180);

/// فاصلهٔ پرسیدنِ «تا کجا رسیدی؟».
const BOOTSTRAP_POLL: Duration = Duration::from_millis(500);

// >>> AETHER-APP-FIX tor-proves-its-exit (mobile 1.5.0 `tor.rs`: `prove` + `PROOF_LIMIT`)
/// چقدر به یک تورِ ۱۰۰٪ فرصت داده می‌شود تا *یک درخواستِ واقعی* را از یک
/// خروجی رد کند — همان `PROOF_LIMIT = 45s` موبایل.
///
/// # چرا ۱۰۰٪ کافی نیست (لاگ‌های ۸ اکتبر، هر سه حالتِ تور)
///
/// `Bootstrapped 100% (done)` فقط می‌گوید تور *یک* مدار ساخته است. روی شبکه‌ای
/// که obfs4 را پس از دست‌دادن خفه می‌کند، همان پل دیسکریپتورها را (با کشِ گرم،
/// چند کیلوبایت) می‌رساند و بعد هیچ استریمی از آن رد نمی‌شود:
///
/// * «تور تنها»: ۱۰۰٪ در ۱۸:۲۹:۰۶، بعد ۱۰۰ ثانیه همهٔ سنجه‌ها شکست، و
///   `Failed to find node for hop #1` — پلِ تنها مرده بود.
/// * «تور → اتر»: ۱۰۰٪ در ۱۸:۳۵:۵۹، بعد ثبت‌نامِ WARP از راهِ تور چهار بار
///   ۲۰ ثانیه بی‌جواب ماند.
/// * «تور → سایفون»: سایفون از راهِ تور بالا آمد، یک پاسخ با ۹۱۴۳ms گرفت و
///   SSH keepaliveاش ۲۰ ثانیه بعد مرد.
///
/// موبایل پیش از اعلامِ «آماده» `client.connect(check.torproject.org:443)`
/// می‌زند و اگر جواب نیاید، همان موج را شکست‌خورده می‌گیرد و سراغِ ترابرِ بعدی
/// می‌رود. نسخهٔ ویندوز این گام را نداشت: ۱۰۰٪ را آمادگی می‌گرفت، مسیرِ داده،
/// پراکسیِ سیستم و TUN را بالا می‌آورد و تنها پس از شکستِ خودآزما کلِ تلاش را
/// پایین می‌کشید — بی‌آنکه هرگز snowflake یا meek را امتحان کند.
const PROOF_LIMIT: Duration = Duration::from_secs(45);

// >>> AETHER-APP-FIX tor-proof-in-parallel
/// سقفِ هر تلاش در اثباتِ موازی. هر دور سه درخواستِ هم‌زمان می‌فرستد، هر کدام
/// با نامِ کاربریِ SOCKSِ یکتا — تور (`IsolateSOCKSAuth`، پیش‌فرض) برای هر کدام
/// مدارِ جدا می‌سازد. لاگِ ۸ اکتبر (`loge2`): یک خروجیِ خفه ۴۰+۵ ثانیه اثبات را
/// نگه داشت، تور کشته و از نو اجرا شد، و توری تازه همان کار را ۶ ثانیه‌ای کرد.
/// با سه مدارِ موازی، یک خروجیِ بد دیگر کلِ اثبات را نمی‌سوزاند.
const PROOF_PARALLEL_ATTEMPT: Duration = Duration::from_secs(20);
// <<< AETHER-APP-FIX tor-proof-in-parallel

// >>> AETHER-APP-FIX tor-auto-like-mobile
/// مهلتِ پرسیدنِ دسترس‌پذیریِ هر پل (موبایل `bridges.rs`: `keep_reachable`، ۶s).
const BRIDGE_REACH_TIMEOUT: Duration = Duration::from_secs(6);
/// سقفِ پل‌های هر ترابر پس از غربال (موبایل ۶؛ اینجا ۸ تا هیچ پلِ زندهٔ
/// فهرستِ خودِ Tor — امروز ۷ تا — بیرون نماند).
const BRIDGE_KEEP: usize = 8;
/// مهلتِ جابه‌جاکردنِ لیسنرِ SOCKSِ برنده روی پورتِ ثابتِ برنامه.
const SOCKS_HANDOVER: Duration = Duration::from_secs(15);
// <<< AETHER-APP-FIX tor-auto-like-mobile

/// مقصدهای اثبات: نام، نه IP (ATYP=DOMAIN، resolve در خروجی)، و روی ۸۰ تا
/// پاسخِ `HTTP/` بی‌نیاز از TLS خوانده شود. اولی همان هدفِ پیش‌فرضِ موبایل است؛
/// دو تای بعدی برای خروجی‌ای که یکی را رد کند. هر پاسخِ HTTP — حتی 301 یا 403 —
/// یعنی بایت‌ها در هر دو جهت از مدار رد شده‌اند، که همان چیزی است که ۱۰۰٪ نمی‌گوید.
const PROOF_TARGETS: [(&str, u16); 3] = [
    ("check.torproject.org", 80),
    ("www.cloudflare.com", 80),
    ("example.com", 80),
];
// <<< AETHER-APP-FIX tor-proves-its-exit

/// مهلتِ نوشتنِ فایلِ پورتِ کنترل و کوکی توسطِ تور.
const CONTROL_FILE_TIMEOUT: Duration = Duration::from_secs(20);

/// مهلتِ برقراریِ خودِ اتصالِ TCP به پورتِ کنترل.
const CONTROL_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// یک برشِ خواندن. با پرشدنش شکستی اعلام نمی‌شود؛ فقط دوباره تلاش می‌شود.
///
/// # چرا این عدد کوچک است و مهلتِ واقعی جای دیگری است
///
/// لاگِ ۱۷ سپتامبر (`loge2.txt`) سه اجرای پشت‌سرهم را نشان می‌دهد که همه‌شان
/// دقیقاً ۱۰.۳ ثانیه پس از شروع مردند، با یک خطای واحد:
///
/// ```text
///   08:15:30.803 [notice] Opened Control listener connection (ready) on 127.0.0.1:11533
///   08:15:33.000 [notice] Bootstrapped 0% (starting): Starting
///   ← هفت ثانیه سکوتِ کاملِ تور →
///   08:15:40.000 [notice] Starting with guard context "bridges"
///   E/tor: tor did not come up: خواندنِ پورتِ کنترلِ تور نشد: … (os error 10060)
/// ```
///
/// پورتِ کنترل باز شده بود و اتصال هم برقرار شد (وگرنه خطا «در دسترس نبود»
/// می‌شد، نه os error 10060 که مهلتِ *خواندن* است). چیزی که رخ داد این است:
/// تور در ثانیه‌های اولِ راه‌اندازی حلقهٔ رویدادش را چند ثانیه رها نمی‌کند —
/// همان هفت ثانیه سکوت در لاگِ خودش — و در آن فاصله به فرمانِ ما جواب نمی‌دهد.
/// مهلتِ ۱۰ ثانیه‌ایِ قبلی درست وسطِ آن سکوت می‌افتاد و کلِ تلاش را می‌کشت،
/// روی توری که هنوز داشت بالا می‌آمد. پس مهلتِ خواندن دیگر ابزارِ تصمیم نیست:
/// هر برش کوتاه است، و تصمیم را [`CONTROL_HANDSHAKE_TIMEOUT`] و بودجهٔ خودِ
/// نشست می‌گیرند.
const CONTROL_READ_SLICE: Duration = Duration::from_secs(2);

/// فاصلهٔ دو تلاشِ گرفتنِ نشستِ کنترل.
const CONTROL_RETRY: Duration = Duration::from_millis(500);

/// چقدر برای *برقرارشدنِ* نشستِ کنترل صبر می‌شود.
///
/// از سکوتِ اندازه‌گیری‌شدهٔ بالا (~۷ ثانیه روی این ماشین) دست‌ودل‌بازانه
/// بزرگ‌تر گرفته شده. نرسیدن به آن دیگر شکست نیست: مسیرِ دومِ
/// [`Inner::log_progress`] از لاگِ خودِ تور خوانده می‌شود.
const CONTROL_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(90);

/// مهلتِ یک پرسشِ ساده از پورتِ کنترل، وقتی نشست قبلاً برقرار شده.
const CONTROL_REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// یک موجِ ترابر چقدر وقت می‌گیرد، وقتی موجِ بعدی هم در صف است.
///
/// >>> AETHER-APP-PATCH tor-cold-cache-waves (mobile 1.4.0 `tor.rs`: `BRIDGE_PROBE`)
/// ۱۵۰ ثانیه برای bootstrapِ اولِ یک ماشین کم بود: لاگِ `loge1` (۴ اکتبر) نشان
/// می‌دهد obfs4 در ۲۵ ثانیه به ۵۰٪ (loading_descriptors) رسید و تور هنوز داشت
/// میکرودیسکریپتورها را از پل می‌گرفت. موبایل برای یک موجِ پل ۳۶۰ ثانیه می‌دهد؛
/// اینجا هم همان. موجِ مرده زودتر با [`WAVE_STALL`] کنار می‌رود، پس این عدد
/// فقط سقفِ موجی است که واقعاً دارد جلو می‌رود.
const WAVE_BUDGET: Duration = Duration::from_secs(360);

/// هر موج چند «نوبت» دارد (موبایل: `WAVE_TRIES`).
///
/// نوبتِ دوم فقط وقتی گرفته می‌شود که نوبتِ اول دست‌کم به یک رله رسیده باشد
/// ([`RETRY_FLOOR`]) — یعنی ترابر روی این شبکه کار می‌کند و آنچه کم بود فقط
/// کشِ سردِ دایرکتوری بود. tor هر دیسکریپتوری را که گرفته در `DataDirectory`
/// می‌نویسد، پس نوبتِ دوم از همان‌جا ادامه می‌دهد: لاگِ `loge3` همین را نشان
/// می‌دهد — با کشِ گرم، همان obfs4 در ۱۵ ثانیه از ۱۵٪ به ۷۵٪ پرید.
const WAVE_TRIES: usize = 2;

/// حداقلِ درصدی که نوبتِ دوم را توجیه می‌کند: `10% (conn_done)` = به یک رله رسید.
const RETRY_FLOOR: u8 = 10;

/// چند بایت خواندنِ تازه از شبکه «پیشرفت» حساب می‌شود.
///
/// موبایل (arti) پیشرفت را با کسرِ دقیق (`as_frac`) می‌سنجد، پس هر
/// دیسکریپتوری که می‌رسد ساعتِ گیرکردن را صفر می‌کند. `tor.exe` درصد را فقط
/// عدد صحیح می‌دهد و در فاز ۵۰٪ هر یک درصد یعنی ۴٪ کلِ دیسکریپتورها — روی پلِ
/// کُند ده‌ها ثانیه. معادلِ آن کسر در tor C، `GETINFO traffic/read` است:
/// توری که دارد دانلود می‌کند بایت می‌خواند؛ توری که شبکه خفه‌اش کرده نه.
const TRAFFIC_HEADWAY: u64 = 4 * 1024; // Reduced from 16KB for faster progress detection

/// نامِ فایلی که ترابرِ موفقِ آخر را نگه می‌دارد (موبایل: `bridges::WORKED_FILE`).
const WORKED_FILE: &str = "transport-that-worked";
// <<< AETHER-APP-PATCH tor-cold-cache-waves

/// بی‌پیشرفتیِ چنددقیقه‌ای یعنی این ترابر روی این شبکه راه نمی‌دهد.
///
/// معیار «بی‌پیشرفت بودن» است نه سپری‌شدنِ زمان: درصدی که بالا می‌رود ساعتِ
/// این مهلت را از صفر می‌کند. و فقط وقتی به‌کار می‌رود که موجِ بعدی وجود
/// داشته باشد — روی آخرین موج، بودجهٔ نشست تنها حرفِ آخر است.
///
/// >>> AETHER-APP-PATCH tor-cold-cache-waves (mobile 1.4.0 `tor.rs`: `STALL_LIMIT = 75s`)
/// ۴۵ ثانیه درست همان چیزی بود که `loge1` را کشت: obfs4 روی ۵۰٪ «بی‌پیشرفت»
/// اعلام شد درحالی‌که در `loge2` همان فاز ۳۷ ثانیه طول کشید تا فقط به ۵۱٪
/// برسد. حالا هم عدد همانِ موبایل است و هم «پیشرفت» ریزدانه‌تر سنجیده می‌شود
/// (دیسکریپتورِ پلِ تازه، بایت‌های خوانده‌شده) — رجوع به [`TRAFFIC_HEADWAY`].
const WAVE_STALL: Duration = Duration::from_secs(75);
/// <<< AETHER-APP-PATCH tor-cold-cache-waves

#[cfg(windows)]
pub const TOR_FILENAME: &str = "tor.exe";
#[cfg(not(windows))]
pub const TOR_FILENAME: &str = "tor";

#[cfg(windows)]
pub const LYREBIRD_FILENAME: &str = "lyrebird.exe";
#[cfg(not(windows))]
pub const LYREBIRD_FILENAME: &str = "lyrebird";

#[cfg(windows)]
pub const SNOWFLAKE_FILENAME: &str = "snowflake-client.exe";
#[cfg(not(windows))]
pub const SNOWFLAKE_FILENAME: &str = "snowflake-client";

#[cfg(windows)]
pub const CONJURE_FILENAME: &str = "conjure-client.exe";
#[cfg(not(windows))]
pub const CONJURE_FILENAME: &str = "conjure-client";

/// چند سطر از خروجیِ خودِ تور برای گزارشِ عیب‌یابی نگه داشته شود.
const MAX_LINES: usize = 400;

/// کدام پل‌ها، اگر اصلاً.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BridgeMode {
    /// مستقیم به شبکهٔ تور. جایی که تور بسته نیست، سریع‌ترین راه است.
    #[default]
    None,
    /// همان فهرستی که Tor در `pt_config.json` کنارِ باینری می‌فرستد.
    BuiltIn,
    /// سطرهایی که کاربر خودش چسبانده است.
    Custom,
    // >>> AETHER-APP-FIX tor-auto-like-mobile
    /// «خودکار»، همان `TorBridges::Auto` موبایل (`tor.rs`: `establish`): هم
    /// مستقیم و هم پل‌های داخلی. موبایل این دو را پشتِ سرِ هم می‌رود (اول ۷۵
    /// ثانیه مستقیم، بعد پل‌ها)؛ اینجا هم‌زمان اجرا می‌شوند و هر کدام زودتر
    /// خروجی‌اش را ثابت کند نشست را می‌برد — [`TorNative::race`].
    Auto,
    // <<< AETHER-APP-FIX tor-auto-like-mobile
}

/// آنچه برای یک اجرا لازم است.
#[derive(Debug, Clone)]
pub struct TorLaunch {
    /// مسیرِ `tor.exe`.
    pub binary: PathBuf,
    /// پوشه‌ای که `lyrebird.exe`، `geoip`، `geoip6` و `pt_config.json` در آن است.
    pub support: PathBuf,
    /// `DataDirectory` تور.
    pub home: PathBuf,
    pub bridges: BridgeMode,
    /// از فهرستِ داخلی کدام ترابر: `obfs4`، `snowflake` یا `meek`.
    pub transport: String,
    /// سطرهای دستیِ کاربر، هر سطر یکی.
    pub custom_bridges: String,
    /// هُپِ قبلی، اگر تور دومین حلقهٔ زنجیر است.
    pub upstream: Option<SocketAddr>,
    /// پورتِ SOCKS. `None` یعنی خودِ تور یکی بردارد.
    ///
    /// WhiteAesther اینجا به‌عمد `auto` می‌گذارد، چون پورتِ ثابت می‌تواند روی
    /// ماشینِ کاربر گرفته باشد. برنامهٔ ما اما قراردادِ دیگری دارد: کاربر
    /// مرورگرش را به `127.0.0.1:1819` وصل می‌کند و همهٔ مسیرهای دیگر —
    /// پراکسیِ سیستم، TUN، اشتراکِ شبکه — همان عدد را می‌شناسند. پس برای
    /// حالتِ «تور تنها» پورت را ثابت می‌دهیم و اگر گرفته بود، خطا را
    /// سربسته نمی‌گذاریم.
    pub socks: Option<u16>,
    /// کلِ وقتی که این نشست برای رسیدن به مدار دارد.
    ///
    /// `None` یعنی [`BOOTSTRAP_TIMEOUT`]. برنامه اما بودجهٔ خودش را دارد و در
    /// لاگ هم اعلامش می‌کند (`timeout=600s`)؛ پیش از این آن عدد اعلام می‌شد و
    /// اینجا ۱۸۰ ثانیه اعمال می‌شد — یعنی لاگ چیزی می‌گفت که کد نمی‌کرد.
    pub budget: Option<Duration>,
}

impl TorLaunch {
    pub fn validate(&self) -> Result<(), String> {
        if !self.binary.is_file() {
            return Err("این نصب فایلِ tor را ندارد".into());
        }
        match self.bridges {
            BridgeMode::None | BridgeMode::BuiltIn | BridgeMode::Auto => Ok(()),
            BridgeMode::Custom => {
                if self.bridge_lines().is_empty() {
                    Err("حداقل یک سطرِ پل بچسبان، یا پل‌های داخلی را انتخاب کن".into())
                } else {
                    Ok(())
                }
            }
        }
    }

    /// سطرهای چسبانده‌شده، بی خطِ خالی و بی توضیح.
    ///
    /// پیشوندِ `Bridge ` برداشته می‌شود: صفحه‌ای که مردم از آن کپی می‌کنند
    /// همین کلمه را هم می‌نویسد و torrc دقیقاً یک‌بار می‌خواهدش؛ وگرنه
    /// `Bridge Bridge obfs4 …` می‌شود و تور کلِ فایل را با خطای تجزیه رد
    /// می‌کند — روی سطری که کاربر ننوشته است.
    fn bridge_lines(&self) -> Vec<String> {
        self.custom_bridges
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                line.strip_prefix("Bridge ")
                    .or_else(|| line.strip_prefix("bridge "))
                    .unwrap_or(line)
                    .trim()
                    .to_string()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorSnapshot {
    /// `idle`، `connecting`، `connected`، `error`.
    pub state: String,
    pub pid: Option<u32>,
    pub socks_port: Option<u16>,
    /// پیشرفتِ bootstrap، ۰ تا ۱۰۰.
    pub bootstrap: u8,
    /// تور با زبانِ خودش می‌گوید الان چه می‌کند.
    pub bootstrap_summary: Option<String>,
    /// `ip:port`ِ اولین هاپ (پل)، وقتی پورتِ کنترل آن را داده باشد.
    pub first_hop: Option<String>,
    pub last_error: Option<String>,
}

impl Default for TorSnapshot {
    fn default() -> Self {
        Self {
            state: "idle".into(),
            pid: None,
            socks_port: None,
            bootstrap: 0,
            bootstrap_summary: None,
            first_hop: None,
            last_error: None,
        }
    }
}

type LineSink = Arc<dyn Fn(&str) + Send + Sync>;

/// فازِ حامل. عددی است چون در یک `AtomicU8` می‌نشیند و از چند ترد خوانده
/// می‌شود؛ سه حالت بیشتر ندارد و enum برایش هزینهٔ اضافه بود.
const PHASE_IDLE: u8 = 0;
const PHASE_STARTING: u8 = 1;
const PHASE_RUNNING: u8 = 2;

struct Inner {
    /// فازِ راه‌اندازی؛ ببینید `is_alive`.
    phase: AtomicU8,
    child: Mutex<Option<Child>>,
    /// 1.3.0 port-guard: tor.exe and the pluggable transports it spawns
    /// (lyrebird.exe …) live in one kill-on-close job; see `procguard.rs`.
    tree: Mutex<Option<crate::procguard::KillTree>>,
    snapshot: Mutex<TorSnapshot>,
    lines: Mutex<VecDeque<String>>,
    generation: AtomicU64,
    /// درصدی که از *لاگِ خودِ تور* درآمده، مستقل از پورتِ کنترل.
    ///
    /// مسیرِ دوم، و دلیلِ وجودش لاگِ ۱۷ سپتامبر است: وقتی پورتِ کنترل چند
    /// ثانیه جواب نمی‌دهد، همان تور در stdout خودش `Bootstrapped N%` را
    /// می‌نویسد. یک نشست نباید به یک کاناله بند باشد وقتی کانالِ دوم مجانی
    /// جلوی چشم است.
    log_progress: AtomicU8,
    /// پورتی که تور در لاگش گفت لیسنرِ SOCKS را روی آن باز کرده. صفر یعنی
    /// هنوز نگفته.
    log_socks: AtomicU16,
    // >>> AETHER-APP-PATCH tor-cold-cache-waves
    /// شمارِ سطرهای لاگِ تور که خودشان پیشرفت‌اند بی‌آنکه درصد را عوض کنند
    /// (`new bridge descriptor`، `Learned fingerprint`، …).
    log_activity: AtomicU64,
    // <<< AETHER-APP-PATCH tor-cold-cache-waves
    // >>> AETHER-APP-PATCH a-bad-circuit-is-not-a-bad-network
    /// مسیرِ فایلِ پورتِ کنترل و کوکیِ این اجرا — تا پس از bootstrap هم
    /// بتوان فرمانی به تور داد. پیش‌تر هر دو فقط متغیرِ محلیِ `start`
    /// بودند و با پایانِ راه‌اندازی از دست می‌رفتند.
    control: Mutex<Option<(PathBuf, PathBuf)>>,
    // <<< AETHER-APP-PATCH a-bad-circuit-is-not-a-bad-network
    // >>> AETHER-APP-FIX tor-auto-like-mobile
    /// دو مسابقه‌دهندهٔ حالتِ خودکار (مستقیم و پل)، تا `stop` هر دو را بگیرد.
    racers: Mutex<Vec<TorNative>>,
    /// برندهٔ مسابقه پس از تحویل: فرزندش به همین حامل منتقل شده، ولی خواننده‌های
    /// لاگش هنوز به `Inner`ِ خودش بسته‌اند و باید تا پایانِ نشست زنده بمانند —
    /// وگرنه لولهٔ stdoutِ تور پر می‌شود و تور قفل می‌کند.
    adopted: Mutex<Option<TorNative>>,
    // <<< AETHER-APP-FIX tor-auto-like-mobile
    sink: LineSink,
}

#[derive(Clone)]
pub struct TorNative {
    inner: Arc<Inner>,
}

impl TorNative {
    pub fn new(sink: LineSink) -> Self {
        Self {
            inner: Arc::new(Inner {
                phase: AtomicU8::new(PHASE_IDLE),
                child: Mutex::new(None),
                tree: Mutex::new(None),
                snapshot: Mutex::new(TorSnapshot::default()),
                lines: Mutex::new(VecDeque::with_capacity(MAX_LINES)),
                generation: AtomicU64::new(0),
                log_progress: AtomicU8::new(0),
                log_socks: AtomicU16::new(0),
                log_activity: AtomicU64::new(0),
                // >>> AETHER-APP-PATCH a-bad-circuit-is-not-a-bad-network
                control: Mutex::new(None),
                // <<< AETHER-APP-PATCH a-bad-circuit-is-not-a-bad-network
                racers: Mutex::new(Vec::new()),
                adopted: Mutex::new(None),
                sink,
            }),
        }
    }

    pub fn snapshot(&self) -> TorSnapshot {
        self.inner.snapshot.lock().clone()
    }

    pub fn lines(&self) -> Vec<String> {
        // AETHER-APP-FIX tor-auto-like-mobile: سطرهای برندهٔ مسابقه در `Inner`ِ خودش‌اند.
        if let Some(winner) = self.inner.adopted.lock().as_ref() {
            return winner.lines();
        }
        self.inner.lines.lock().iter().cloned().collect()
    }

    /// آیا فرزند هنوز زنده است. یک پروسهٔ مرده، یک snapshotِ سرحال جا
    /// می‌گذارد؛ پس این پرسش جدا از snapshot پاسخ داده می‌شود.
    /// آیا این حامل زنده است؟
    ///
    /// «هنوز شروع نشده» با «مرده» یکی نیست. راه‌اندازی روی یک تردِ دیگر است و
    /// tor.exe چند ده میلی‌ثانیه بعد از ساخته‌شدنِ آن ترد اجرا می‌شود؛ در آن
    /// فاصله ماشینِ حالت `false` می‌گرفت و تلاش را در همان تیکِ اول جمع می‌کرد
    /// (لاگِ ۱۶ سپتامبر: ۲۰۰ms پس از شروع، «Engine exited before it opened the
    /// SOCKS5 port»). پس فاز صریح است، و Starting زنده است.
    pub fn is_alive(&self) -> bool {
        match self.inner.phase.load(Ordering::SeqCst) {
            PHASE_STARTING => true,
            PHASE_RUNNING => {
                let mut guard = self.inner.child.lock();
                match guard.as_mut() {
                    Some(child) => !matches!(child.try_wait(), Ok(Some(_))),
                    // در حالِ اجرا و بی‌فرزند نمی‌شود؛ اگر شد، مرده است.
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// «راه‌اندازی شکست خورد» — همهٔ مسیرهای خطای `start` از یک جا اعلام
    /// می‌شوند: تردِ فراخوان، وقتی `start` خطا برگرداند. اینکه هر `?` خودش
    /// فاز را بگذارد، همان چیزی است که یکی‌اش فراموش می‌شود.
    pub fn mark_dead(&self) {
        self.inner.phase.store(PHASE_IDLE, Ordering::SeqCst);
    }

    /// «از این لحظه در حالِ راه‌اندازی‌ام» — پیش از سپردنِ `start` به یک ترد
    /// صدا زده می‌شود، وگرنه همان مسابقهٔ بالا برمی‌گردد.
    pub fn mark_starting(&self) {
        self.inner.phase.store(PHASE_STARTING, Ordering::SeqCst);
    }

    /// تور را راه می‌اندازد و منتظرِ مدار می‌ماند.
    ///
    /// # موج‌های ترابر
    ///
    /// با پل‌های داخلی، یک نوبتِ obfs4 تمامِ شانسِ کاربر نیست. مسیرِ arti همین نردبان را
    /// داشت (`obfs4 → snowflake → meek`)؛ با کوچ به `tor.exe` گم شد و لاگِ ۱۷ سپتامبر
    /// دقیقاً همین را نشان می‌دهد: `Plan ready (1 attempt)`. در شبکه‌ای که obfs4ِ عمومیِ
    /// توکار را شمارده و بسته، دو ترابرِ domain-fronted تنها شانسِ باقی‌مانده‌اند و باید
    /// نوبت بگیرند.
    pub fn start(&self, launch: &TorLaunch) -> Result<SocketAddr, String> {
        // `stop` فاز را به Idle می‌برد، پس Starting بعد از آن دوباره گذاشته
        // می‌شود — وگرنه پنجرهٔ مسابقه فقط جابه‌جا می‌شود.
        launch.validate()?;
        self.stop();
        self.inner.phase.store(PHASE_STARTING, Ordering::SeqCst);
        // AETHER-APP-FIX tor-fresh-circuit-for-the-engine: درخواستِ نشستِ قبلی کهنه است.
        FRESH_CIRCUIT_WANTED.store(false, Ordering::SeqCst);

        // >>> AETHER-APP-FIX tor-auto-like-mobile
        if launch.bridges == BridgeMode::Auto {
            if launch.upstream.is_none() {
                return self.race(launch);
            }
            // پشتِ یک حاملِ دیگر پل معنا ندارد (رجوع به `render_torrc`): مستقیم.
            let mut direct = launch.clone();
            direct.bridges = BridgeMode::None;
            return self.run_waves(&direct);
        }
        self.run_waves(launch)
    }

    /// موج‌های ترابر روی همین حامل — مسیرِ همیشگیِ `None`/`BuiltIn`/`Custom`.
    fn run_waves(&self, launch: &TorLaunch) -> Result<SocketAddr, String> {
        // <<< AETHER-APP-FIX tor-auto-like-mobile
        let overall = Instant::now() + session_budget(launch);
        let waves = plan_waves(launch);
        let last = waves.len().saturating_sub(1);
        let mut failures: Vec<String> = Vec::new();
        // >>> AETHER-APP-PATCH tor-cold-cache-waves
        if waves.len() > 1 {
            let names: Vec<String> = waves.iter().map(|wave| wave_label(wave)).collect();
            (self.inner.sink)(&format!(
                "[*] tor will try {} in turn (each one dropped after {}s without headway)",
                names.join(", then "),
                WAVE_STALL.as_secs()
            ));
        }
        // <<< AETHER-APP-PATCH tor-cold-cache-waves

        'waves: for (index, transport) in waves.iter().enumerate() {
            let left = overall.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            let slice = if index == last {
                left
            } else {
                left.min(WAVE_BUDGET)
            };
            // موجِ آخر با کلِ بودجهٔ نشست کار می‌کند و معیارِ گیرکردن ندارد: جایی
            // برای رفتن نیست، پس کوتاه‌کردنش فقط شانس را می‌سوزاند.
            let stall = if index == last {
                None
            } else {
                Some(WAVE_STALL)
            };
            let label = wave_label(transport);
            if waves.len() > 1 {
                (self.inner.sink)(&format!(
                    "[*] tor bridge wave {}/{}: {label} ({}s of the {}s left)",
                    index + 1,
                    waves.len(),
                    slice.as_secs(),
                    left.as_secs()
                ));
            }
            // >>> AETHER-APP-FIX tor-auto-like-mobile
            // موبایل (`bridges.rs`: `keep_reachable`) پیش از دادنِ پل‌ها به تور، هر
            // کدام را با یک TCPِ شش‌ثانیه‌ای می‌سنجد و مرده‌ها را کنار می‌گذارد.
            // لاگِ ۸ اکتبر: دو پلِ مرده (`146.57.248.225:22`، `51.222.13.177:80`)
            // تمامِ نشست `general SOCKS server failure` دادند. موجی که هیچ پلش
            // جواب نداد، اگر موجِ دیگری مانده، اصلاً اجرا نمی‌شود.
            let screened: Option<TorLaunch> =
                if launch.bridges == BridgeMode::BuiltIn && launch.upstream.is_none() {
                    match screen_built_in(launch, transport, &self.inner.sink) {
                        Screen::Use(lines) => Some(TorLaunch {
                            bridges: BridgeMode::Custom,
                            custom_bridges: lines.join("\n"),
                            ..launch.clone()
                        }),
                        Screen::Skip if index != last => {
                            failures.push(format!("{label}: no bridge answered"));
                            (self.inner.sink)(&format!(
                                "[-] {label}: none of the built-in bridges answered a TCP probe \
                                 in {}s; skipping straight to the next transport",
                                BRIDGE_REACH_TIMEOUT.as_secs()
                            ));
                            continue 'waves;
                        }
                        Screen::Skip | Screen::Keep => None,
                    }
                } else {
                    None
                };
            let wave_launch = screened.as_ref().unwrap_or(launch);
            // <<< AETHER-APP-FIX tor-auto-like-mobile
            // >>> AETHER-APP-PATCH tor-cold-cache-waves
            let wave_end = Instant::now() + slice;
            let mut round = 1usize;
            loop {
                let go = wave_end
                    .min(overall)
                    .saturating_duration_since(Instant::now());
                if go.is_zero() {
                    break;
                }
                match self.run_once(wave_launch, transport, go, stall) {
                    Ok(address) => {
                        if waves.len() > 1 && !transport.trim().is_empty() {
                            remember_transport(&launch.home, transport);
                        }
                        (self.inner.sink)(&format!(
                            "[+] tor has a circuit via {label}; SOCKS listener on {address}"
                        ));
                        return Ok(address);
                    }
                    Err(error) => {
                        let reached = self.inner.snapshot.lock().bootstrap;
                        // `stop` (کاربر قطع کرد) فاز را Idle می‌کند. بعد از آن نه
                        // نوبتِ دوم معنا دارد نه موجِ بعدی — و `kill_child_keep_phase`
                        // نباید حاملِ ایستاده را دوباره «در حالِ راه‌اندازی» کند.
                        if self.inner.phase.load(Ordering::SeqCst) == PHASE_IDLE {
                            failures.push(format!("{label}: {error}"));
                            break 'waves;
                        }
                        self.kill_child_keep_phase();
                        let time_left = !wave_end
                            .min(overall)
                            .saturating_duration_since(Instant::now())
                            .is_zero();
                        if stall.is_some()
                            && round < WAVE_TRIES
                            && reached >= RETRY_FLOOR
                            && time_left
                        {
                            round += 1;
                            // >>> AETHER-APP-FIX tor-proves-its-exit
                            if reached >= 100 {
                                (self.inner.sink)(&format!(
                                    "[*] {label}: go {round} of {WAVE_TRIES} — the first go reached \
                                     100% but its exit carried nothing ({error}); a fresh tor \
                                     builds its circuits anew before the next transport is tried"
                                ));
                                continue;
                            }
                            // <<< AETHER-APP-FIX tor-proves-its-exit
                            (self.inner.sink)(&format!(
                                "[*] {label}: go {round} of {WAVE_TRIES} — the first go reached \
                                 {reached}% ({error}), so the transport works here; tor now \
                                 resumes from the directory it already cached"
                            ));
                            continue;
                        }
                        failures.push(format!("{label}: {error}"));
                        if index != last {
                            (self.inner.sink)(&format!(
                                "[-] {label} did not get there ({error}); trying the next transport"
                            ));
                        }
                        break;
                    }
                }
            }
            // <<< AETHER-APP-PATCH tor-cold-cache-waves
        }

        self.stop();
        let error = if failures.is_empty() {
            "بودجهٔ این نشست پیش از اولین تلاش تمام شد".to_string()
        } else {
            failures.join(" | ")
        };
        let mut snapshot = self.inner.snapshot.lock();
        snapshot.state = "error".into();
        snapshot.last_error = Some(error.clone());
        Err(error)
    }

    // >>> AETHER-APP-FIX tor-auto-like-mobile
    /// حالتِ خودکار: مستقیم و پل‌های داخلی، هم‌زمان.
    ///
    /// # چرا (موبایل 1.5.0، `tor.rs`: `establish`)
    ///
    /// موبایل با `TorBridges::Auto` اول تورِ *مستقیم* را می‌رود (۷۵ ثانیه، با
    /// پوشهٔ کشِ جدا) و فقط اگر نشد سراغِ پل‌ها می‌رود. ویندوز Auto را از روزِ
    /// اول به «پل‌های داخلی» ترجمه می‌کرد و هرگز مستقیم را امتحان نمی‌کرد؛ روی
    /// همان وای‌فای‌ای که موبایل زیرِ ۱۵ ثانیه وصل می‌شد، ویندوز از راهِ یک پلِ
    /// obfs4ِ `iat-mode=1` (حدودِ ۴۰KB/s) ۱۴.۶ مگابایت دایرکتوری می‌کشید — لاگِ
    /// ۸ اکتبر، ۳۷۵ ثانیه.
    ///
    /// اینجا دو `tor.exe` جدا با دو `DataDirectory` جدا بالا می‌آیند (کشِ هر
    /// کدام گرم می‌ماند)؛ هر کدام زودتر *خروجی‌اش را ثابت کند* برنده است، دیگری
    /// کشته می‌شود، و لیسنرِ SOCKSِ برنده با `SETCONF` روی پورتِ ثابتِ برنامه
    /// می‌نشیند. روی شبکه‌ای که تورِ مستقیم را بسته، تلاشِ مستقیم چیزی جز چند
    /// SYNِ بی‌جواب خرج نمی‌کند و پل‌ها بی‌هیچ تأخیری کارِ خودشان را می‌کنند —
    /// برخلافِ موبایل که آن ۷۵ ثانیه را پیش از اولین پل می‌سوزاند.
    fn race(&self, launch: &TorLaunch) -> Result<SocketAddr, String> {
        let sink = self.inner.sink.clone();
        let started = Instant::now();
        let overall = started + session_budget(launch);
        let best = Arc::new(AtomicU8::new(0));
        let direct_hop = Arc::new(Mutex::new(None));
        let bridge_hop = Arc::new(Mutex::new(None));
        let direct = TorNative::new(racer_sink(
            sink.clone(),
            "direct",
            best.clone(),
            direct_hop.clone(),
        ));
        let bridged = TorNative::new(racer_sink(
            sink.clone(),
            "bridges",
            best,
            bridge_hop.clone(),
        ));
        let direct_launch = TorLaunch {
            home: launch.home.join("direct"),
            bridges: BridgeMode::None,
            transport: String::new(),
            custom_bridges: String::new(),
            socks: None,
            ..launch.clone()
        };
        let bridge_launch = TorLaunch {
            bridges: BridgeMode::BuiltIn,
            socks: None,
            ..launch.clone()
        };
        direct.mark_starting();
        bridged.mark_starting();
        *self.inner.racers.lock() = vec![direct.clone(), bridged.clone()];
        *self.inner.snapshot.lock() = TorSnapshot {
            state: "connecting".into(),
            bootstrap_summary: Some("Starting Tor".into()),
            ..TorSnapshot::default()
        };
        sink(
            "[*] tor bridges=auto, as on mobile: the Tor network directly and the built-in \
             bridges are tried at the same time (mobile tries them one after the other); \
             whichever proves an exit first carries the session and the other is stopped",
        );

        let (tx, rx) = mpsc::channel::<(&'static str, Result<SocketAddr, String>)>();
        for (label, racer, plan) in [
            ("direct", direct.clone(), direct_launch),
            ("bridges", bridged.clone(), bridge_launch),
        ] {
            let sender = tx.clone();
            let spawned = thread::Builder::new()
                .name(format!("tor-race-{label}"))
                .spawn(move || {
                    let result = racer.start(&plan);
                    if result.is_err() {
                        racer.mark_dead();
                    }
                    let _ = sender.send((label, result));
                });
            if let Err(error) = spawned {
                let _ = tx.send((label, Err(format!("could not start a thread: {error}"))));
            }
        }
        drop(tx);

        let mut failures: Vec<String> = Vec::new();
        let mut pending = 2usize;
        let winner = loop {
            if self.inner.phase.load(Ordering::SeqCst) == PHASE_IDLE || pending == 0 {
                break None;
            }
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok((label, Ok(_))) => break Some(label),
                Ok((label, Err(error))) => {
                    pending -= 1;
                    if pending > 0 {
                        sink(&format!(
                            "[-] tor via {label} did not get there ({error}); still waiting for \
                             the other attempt"
                        ));
                    }
                    failures.push(format!("{label}: {error}"));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // هر مسابقه‌دهنده بودجهٔ خودش را دارد؛ این فقط کفِ اطمینان است.
                    if Instant::now() > overall + PROOF_LIMIT + Duration::from_secs(15) {
                        break None;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break None,
            }
        };

        let Some(label) = winner else {
            let stopped = self.inner.phase.load(Ordering::SeqCst) == PHASE_IDLE;
            self.stop();
            let error = if stopped {
                "تور در میانهٔ راه‌اندازی متوقف شد".to_string()
            } else if failures.is_empty() {
                "بودجهٔ این نشست پیش از اولین تلاش تمام شد".to_string()
            } else {
                failures.join(" | ")
            };
            let mut snapshot = self.inner.snapshot.lock();
            snapshot.state = "error".into();
            snapshot.last_error = Some(error.clone());
            return Err(error);
        };

        let (win, lose, hop_line) = if label == "direct" {
            (direct, bridged, direct_hop)
        } else {
            (bridged, direct, bridge_hop)
        };
        lose.stop();
        self.inner
            .racers
            .lock()
            .retain(|racer| !Arc::ptr_eq(&racer.inner, &lose.inner));

        let port = match launch.socks {
            Some(fixed) => match win.rebind_socks(fixed) {
                Ok(port) => port,
                Err(error) => {
                    win.stop();
                    self.stop();
                    let error =
                        format!("تور وصل شد ولی لیسنرش روی 127.0.0.1:{fixed} ننشست ({error})");
                    let mut snapshot = self.inner.snapshot.lock();
                    snapshot.state = "error".into();
                    snapshot.last_error = Some(error.clone());
                    return Err(error);
                }
            },
            None => win.snapshot().socks_port.unwrap_or_default(),
        };
        if !self.adopt(win, port) {
            return Err("تور در میانهٔ راه‌اندازی متوقف شد".into());
        }

        if let Some(line) = hop_line.lock().take() {
            sink(&line);
        }
        sink(&format!(
            "[+] tor has a circuit via {label} (bridges=auto) after {}s; SOCKS listener on \
             127.0.0.1:{port}",
            started.elapsed().as_secs()
        ));
        sink("[*] tor reaching the network: 100%: Done");
        Ok(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
    }

    /// فرزندِ برنده را به همین حامل منتقل می‌کند. `false` یعنی در همین فاصله
    /// `stop` صدا زده شده و برنده هم متوقف شد.
    fn adopt(&self, winner: TorNative, port: u16) -> bool {
        let inner = &self.inner;
        let mut child = inner.child.lock();
        if inner.phase.load(Ordering::SeqCst) != PHASE_STARTING {
            drop(child);
            winner.stop();
            return false;
        }
        *child = winner.inner.child.lock().take();
        *inner.tree.lock() = winner.inner.tree.lock().take();
        *inner.control.lock() = winner.inner.control.lock().clone();
        let mut snapshot = winner.snapshot();
        snapshot.state = "connected".into();
        snapshot.bootstrap = 100;
        snapshot.socks_port = Some(port);
        *inner.snapshot.lock() = snapshot;
        inner.racers.lock().clear();
        *inner.adopted.lock() = Some(winner);
        inner.phase.store(PHASE_RUNNING, Ordering::SeqCst);
        true
    }

    /// لیسنرِ SOCKSِ این تور را روی `port` می‌برد (`SETCONF SocksPort`) و
    /// می‌سنجد که واقعاً همان‌جا گوش می‌دهد.
    fn rebind_socks(&self, port: u16) -> Result<u16, String> {
        let (control_file, cookie_file) = self
            .inner
            .control
            .lock()
            .clone()
            .ok_or_else(|| "این نشستِ تور پورتِ کنترل ندارد".to_string())?;
        let generation = self.inner.generation.load(Ordering::SeqCst);
        let deadline = Instant::now() + SOCKS_HANDOVER;
        let mut session = self
            .open_control(&control_file, &cookie_file, generation, deadline)
            .ok_or_else(|| "پورتِ کنترلِ تور جواب نداد".to_string())?;
        let reply =
            session.command(&format!("SETCONF SocksPort=\"127.0.0.1:{port}\""), deadline)?;
        if !reply.starts_with("250") {
            return Err(format!("تور SETCONF SocksPort را رد کرد: {reply}"));
        }
        let listening = session.socks_port(CONTROL_REPLY_TIMEOUT)?;
        if listening != port {
            return Err(format!("تور حالا روی {listening} گوش می‌دهد، نه {port}"));
        }
        TcpStream::connect_timeout(
            &SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
            Duration::from_secs(3),
        )
        .map_err(|error| format!("لیسنرِ تازه جواب نداد: {error}"))?;
        self.inner.snapshot.lock().socks_port = Some(port);
        Ok(port)
    }
    // <<< AETHER-APP-FIX tor-auto-like-mobile

    /// یک نوبت: یک `tor.exe` با یک ترابر، تا مدار یا تا پایانِ برشِ زمانی‌اش.
    fn run_once(
        &self,
        launch: &TorLaunch,
        transport: &str,
        budget: Duration,
        stall: Option<Duration>,
    ) -> Result<SocketAddr, String> {
        let inner = &self.inner;
        let generation = inner.generation.fetch_add(1, Ordering::SeqCst) + 1;

        // تور اگر پوشهٔ دادهٔ خود را برای گروه یا همه خواندنی ببیند بالا
        // نمی‌آید؛ این بررسیِ خودِ تور است و دورزدنش کارِ ما نیست.
        std::fs::create_dir_all(&launch.home)
            .map_err(|error| format!("پوشهٔ تور آماده نشد: {error}"))?;

        let control_port_file = launch.home.join("control-port");
        let cookie_file = launch.home.join("control-auth-cookie");
        // فایلِ ماندهٔ اجرای قبلی، پورتی را نشان می‌دهد که دیگر توری پشتش
        // نیست، و به‌عنوانِ فایلِ همین اجرا خوانده می‌شود.
        let _ = std::fs::remove_file(&control_port_file);
        let _ = std::fs::remove_file(&cookie_file);

        let torrc = launch.home.join("torrc");
        // >>> AETHER-APP-PATCH a-bad-circuit-is-not-a-bad-network
        // همین دو مسیر پس از اتصال هم لازم‌اند — رجوع به
        // [`TorNative::new_circuit`].
        *inner.control.lock() = Some((control_port_file.clone(), cookie_file.clone()));
        // <<< AETHER-APP-PATCH a-bad-circuit-is-not-a-bad-network
        let rendered = render_torrc(launch, transport, &control_port_file, &cookie_file)?;
        std::fs::write(&torrc, &rendered)
            .map_err(|error| format!("نوشتنِ تنظیماتِ تور نشد: {error}"))?;

        {
            let mut snapshot = inner.snapshot.lock();
            *snapshot = TorSnapshot {
                state: "connecting".into(),
                bootstrap_summary: Some("Starting Tor".into()),
                ..TorSnapshot::default()
            };
        }
        // درصدِ موجِ قبلی نباید موجِ تازه را «در حالِ پیشرفت» نشان دهد.
        inner.log_progress.store(0, Ordering::SeqCst);
        inner.log_socks.store(0, Ordering::SeqCst);
        inner.log_activity.store(0, Ordering::SeqCst);

        let mut command = Command::new(&launch.binary);
        command
            .arg("-f")
            .arg(&torrc)
            .current_dir(&launch.home)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut command);

        let mut child = command
            .spawn()
            .map_err(|error| format!("اجرای تور نشد: {error}"))?;
        let tree = crate::procguard::KillTree::adopt(&child, "tor.exe");
        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        {
            let mut guard = inner.child.lock();
            if inner.generation.load(Ordering::SeqCst) != generation {
                tree.terminate();
                let _ = child.kill();
                let _ = child.wait();
                return Err("تور در میانهٔ راه‌اندازی متوقف شد".into());
            }
            *guard = Some(child);
            *inner.tree.lock() = Some(tree);
        }
        // از این‌جا به بعد زنده‌بودن را خودِ فرآیند جواب می‌دهد.
        inner.phase.store(PHASE_RUNNING, Ordering::SeqCst);
        inner.snapshot.lock().pid = Some(pid);

        // هر دو جریان خوانده می‌شوند، وگرنه تور با پرشدنِ لوله روی لاگِ خودش
        // قفل می‌کند — و همان لاگ تنها روایتِ خرابی‌های پیش از بالاآمدنِ
        // پورتِ کنترل است.
        for reader in [
            stdout.map(|out| Box::new(out) as Box<dyn Read + Send>),
            stderr.map(|err| Box::new(err) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            spawn_log_reader(inner.clone(), reader, generation);
        }

        // سطرهای پلِ همین موج، برای ترجمهٔ اثرِ انگشتِ اولین هاپ به `ip:port`.
        let offered = bridge_lines(launch, transport).unwrap_or_default();

        self.wait_for_circuit(
            &control_port_file,
            &cookie_file,
            generation,
            Instant::now() + budget,
            stall,
            launch.socks,
            &offered,
            transport,
        )
    }

    /// منتزرِ `PROGRESS=100` می‌ماند و در راه می‌گوید تا کجا رسیده.
    ///
    /// # چرا دو منبع دارد
    ///
    /// پورتِ کنترل دقیق است (درصد و خلاصهٔ متنی)، ولی تنها منبع نیست: تور همان درصد را
    /// در stdout خودش هم می‌نویسد. لاگِ ۱۷ سپتامبر نشان داد وابستگیِ تک‌کاناله چه
    /// هزینه‌ای دارد — یک مهلتِ خواندنِ ۱۰ ثانیه‌ای روی پورتِ کنترل، در همان هفت ثانیه‌ای
    /// که تور به هیچ فرمانی جواب نمی‌دهد، کلِ نشست را می‌کشت. حالا از دست رفتنِ پورتِ
    /// کنترل فقط یک سطرِ هشدار است و کار با لاگ ادامه می‌یابد.
    fn wait_for_circuit(
        &self,
        control_port_file: &Path,
        cookie_file: &Path,
        generation: u64,
        deadline: Instant,
        stall: Option<Duration>,
        fallback_socks: Option<u16>,
        offered: &[String],
        transport: &str,
    ) -> Result<SocketAddr, String> {
        let inner = &self.inner;
        let mut session = self.open_control(control_port_file, cookie_file, generation, deadline);

        let mut told = 0u8;
        let mut best = 0u8;
        let mut headway = Instant::now();
        // >>> AETHER-APP-PATCH tor-cold-cache-waves
        let mut activity_seen = inner.log_activity.load(Ordering::SeqCst);
        let mut traffic_mark: Option<u64> = None;
        let mut noted = Instant::now();
        // <<< AETHER-APP-PATCH tor-cold-cache-waves

        while Instant::now() < deadline {
            if inner.generation.load(Ordering::SeqCst) != generation {
                return Err("تور در میانهٔ راه‌اندازی متوقف شد".into());
            }
            // توری که پیش از bootstrap مرده، سوکتِ کنترل را بسته می‌گذارد؛
            // دیدنش اینجا یک هنگ را به یک پیام تبدیل می‌کند.
            if let Some(child) = inner.child.lock().as_mut() {
                if matches!(child.try_wait(), Ok(Some(_))) {
                    return Err("تور پیش از ساختِ مدار متوقف شد".into());
                }
            } else {
                return Err("تور در میانهٔ راه‌اندازی متوقف شد".into());
            }

            // درصد: هر چه لاگ گفته، و اگر پورتِ کنترل جواب می‌دهد هر چه آن می‌گوید.
            let mut progress = inner.log_progress.load(Ordering::SeqCst);
            let mut summary = None;
            // پرچم، نه انتساب در همان بازو: نشستی که هم‌زمان قرض داده شده و کنار
            // گذاشته می‌شود، همان چیزی است که قرض‌گیر را ناخوش می‌کند.
            let mut lost_control = false;
            let mut traffic: Option<u64> = None;
            if let Some(open) = session.as_mut() {
                match open.get_info("status/bootstrap-phase", CONTROL_REPLY_TIMEOUT) {
                    Ok(phase) => {
                        progress = progress.max(parse_progress(&phase).unwrap_or(0));
                        summary = parse_summary(&phase);
                        // >>> AETHER-APP-PATCH tor-cold-cache-waves
                        // فقط پیش از ۱۰۰٪ و فقط وقتی معیارِ گیرکردن در کار است.
                        if stall.is_some() && progress < 100 {
                            match open.get_info("traffic/read", CONTROL_REPLY_TIMEOUT) {
                                Ok(reply) => traffic = parse_traffic(&reply),
                                Err(error) => {
                                    (inner.sink)(&format!(
                                        "[-] tor's control port stopped answering ({error}); \
                                         following its own log instead"
                                    ));
                                    lost_control = true;
                                }
                            }
                        }
                        // <<< AETHER-APP-PATCH tor-cold-cache-waves
                    }
                    Err(error) => {
                        (inner.sink)(&format!(
                            "[-] tor's control port stopped answering ({error}); \
                             following its own log instead"
                        ));
                        lost_control = true;
                    }
                }
            }
            if lost_control {
                session = None;
            }
            {
                let mut snapshot = inner.snapshot.lock();
                snapshot.bootstrap = progress;
                if summary.is_some() {
                    snapshot.bootstrap_summary = summary.clone();
                }
            }
            // >>> AETHER-APP-FIX tor-proves-its-exit
            // ۱۰۰٪ اینجا اعلام نمی‌شود: `tor_bootstrap::done()` همین سطر را
            // می‌خواند و دروازهٔ تور با آن باز می‌شود. ۱۰۰٪ فقط پس از اثباتِ خروجی
            // (پایین‌تر) گفته می‌شود.
            if progress != told && progress < 100 {
                // <<< AETHER-APP-FIX tor-proves-its-exit
                told = progress;
                (inner.sink)(&format!(
                    "[*] tor reaching the network: {progress}%{}",
                    summary
                        .as_deref()
                        .map(|text| format!(": {text}"))
                        .unwrap_or_default()
                ));
            }
            // >>> AETHER-APP-PATCH tor-cold-cache-waves
            // «پیشرفت» = هر یک از سه نشانه، مثلِ کسرِ ریزِ arti در موبایل:
            // درصدِ بالاتر، سطری از لاگ که خودش پیشرفت است، یا بایت‌های تازه.
            let mut moved = false;
            let mut climbed = false;
            if progress > best {
                best = progress;
                moved = true;
                climbed = true;
            }
            let activity = inner.log_activity.load(Ordering::SeqCst);
            if activity != activity_seen {
                activity_seen = activity;
                moved = true;
            }
            if let Some(read) = traffic {
                match traffic_mark {
                    None => traffic_mark = Some(read),
                    Some(mark) if read >= mark.saturating_add(TRAFFIC_HEADWAY) => {
                        traffic_mark = Some(read);
                        moved = true;
                    }
                    _ => {}
                }
            }
            if moved {
                headway = Instant::now();
                // پیشرفتی که درصد را عوض نکرده برای لایهٔ بالا (`tor_bootstrap` و
                // پیامِ روی صفحه) نامرئی است؛ هر ۱۵ ثانیه یک سطر آن را می‌گوید،
                // تا «Tor is not getting through» روی توری که دارد دانلود می‌کند
                // نیاید.
                if !climbed && noted.elapsed() >= Duration::from_secs(15) {
                    noted = Instant::now();
                    let kib = traffic.map(|read| read / 1024).unwrap_or(0);
                    (inner.sink)(&format!(
                        "[*] tor is still loading the directory at {best}% ({kib} KiB read so far)"
                    ));
                }
            }
            // <<< AETHER-APP-PATCH tor-cold-cache-waves

            if progress >= 100 {
                // پورت: اول از خودِ تور پرسیده می‌شود، و اگر پورتِ کنترل نیست از
                // لاگِ خودش و در نهایت از قراردادِ خودِ برنامه.
                let port = session
                    .as_mut()
                    .and_then(|open| open.socks_port(CONTROL_REPLY_TIMEOUT).ok())
                    .or_else(|| match inner.log_socks.load(Ordering::SeqCst) {
                        0 => None,
                        port => Some(port),
                    })
                    .or(fallback_socks)
                    .ok_or("تور مدار ساخت ولی هیچ لیسنرِ SOCKSی اعلام نکرد")?;
                // اندپوینتِ واقعی: اولین هاپِ مدار، از پورتِ کنترل پرسیده و با
                // سطرهای پلِ همین موج به `ip:port` ترجمه‌شده. کارتِ اتصال تا
                // امروز اینجا IPِ *خروجی* و نامِ کشور را نشان می‌داد — یعنی
                // همان چیزی که ردیفِ بالایش هم می‌گفت.
                let hop = session
                    .as_mut()
                    .and_then(|open| open.first_hop(CONTROL_REPLY_TIMEOUT))
                    .and_then(|fingerprint| bridge_endpoint_of(offered, &fingerprint));
                if let Some(addr) = &hop {
                    let via = if transport.is_empty() {
                        "direct"
                    } else {
                        transport
                    };
                    (inner.sink)(&format!("[+] tor first hop: {addr} via {via}"));
                }

                // >>> AETHER-APP-FIX tor-proves-its-exit
                (inner.sink)(&format!(
                    "[*] tor is proving its way out: bootstrap is at 100%, now one real request \
                     through an exit (up to {}s) before the data path is built on it",
                    PROOF_LIMIT.as_secs()
                ));
                let proof_end = deadline.min(Instant::now() + PROOF_LIMIT);
                match self.prove_exit(port, generation, proof_end) {
                    Ok(answered) => (inner.sink)(&format!(
                        "[+] tor's way out is open: {answered} answered through an exit"
                    )),
                    Err(error) => {
                        (inner.sink)(&format!(
                            "[-] tor reached 100% but no exit stream got through ({error}); \
                             this transport does not carry traffic on this network"
                        ));
                        return Err(format!(
                            "تور به ۱۰۰٪ رسید ولی هیچ استریمی از خروجی رد نشد ({error})"
                        ));
                    }
                }
                (inner.sink)(&format!(
                    "[*] tor reaching the network: 100%: {}",
                    summary.as_deref().unwrap_or("Done")
                ));
                // <<< AETHER-APP-FIX tor-proves-its-exit

                let mut snapshot = inner.snapshot.lock();
                snapshot.socks_port = Some(port);
                snapshot.state = "connected".into();
                snapshot.bootstrap = 100;
                snapshot.first_hop = hop;
                return Ok(SocketAddr::from((Ipv4Addr::LOCALHOST, port)));
            }

            if let Some(limit) = stall {
                if headway.elapsed() >= limit {
                    return Err(format!(
                        "تا {best}% رسید و {}s بی‌پیشرفت ماند",
                        limit.as_secs()
                    ));
                }
            }
            thread::sleep(BOOTSTRAP_POLL);
        }

        Err(format!("در مهلتِ این نوبت مدار نساخت؛ تا {best}% رسید"))
    }

    // >>> AETHER-APP-FIX tor-proves-its-exit
    /// یک درخواستِ واقعی از راهِ تور — معادلِ `prove` موبایل.
    ///
    /// SOCKS5 با ATYP=DOMAIN به یکی از [`PROOF_TARGETS`]، بعد `HEAD /` و انتظار
    /// برای اولین بایت‌های `HTTP/`. پاسخِ CONNECT به‌تنهایی کافی نیست: روی پلی
    /// که خفه شده، `RELAY_CONNECTED` گاهی می‌رسد و داده نه.
    fn prove_exit(&self, port: u16, generation: u64, deadline: Instant) -> Result<String, String> {
        let inner = &self.inner;
        let alive = || {
            inner.generation.load(Ordering::SeqCst) == generation
                && match inner.child.lock().as_mut() {
                    Some(child) => !matches!(child.try_wait(), Ok(Some(_))),
                    None => false,
                }
        };
        // >>> AETHER-APP-FIX tor-proof-in-parallel
        // هر دور: سه مقصد هم‌زمان، هر کدام روی مدارِ جدای خودش (نامِ کاربریِ SOCKSِ
        // یکتا ← `IsolateSOCKSAuth`). اولین پاسخِ `HTTP/` کافی است.
        let mut last = String::from("no attempt fitted in the budget");
        let mut round = 0u32;
        while Instant::now() < deadline {
            if !alive() {
                return Err("tor stopped while its exit was being proved".into());
            }
            let left = deadline.saturating_duration_since(Instant::now());
            let budget = left.min(PROOF_PARALLEL_ATTEMPT);
            if budget < Duration::from_secs(2) {
                break;
            }
            round += 1;
            let (tx, rx) = mpsc::channel::<(String, Result<(), String>)>();
            for (index, (host, dest)) in PROOF_TARGETS.iter().enumerate() {
                let sender = tx.clone();
                let user = format!("aether-proof-{generation}-{round}-{index}");
                let (host, dest) = (host.to_string(), *dest);
                let _ = thread::Builder::new()
                    .name("tor-proof".into())
                    .spawn(move || {
                        let result = http_head_via_socks(port, &host, dest, budget, Some(&user));
                        let _ = sender.send((format!("{host}:{dest}"), result));
                    });
            }
            drop(tx);
            let round_end = Instant::now() + budget + Duration::from_secs(1);
            let mut errors: Vec<String> = Vec::new();
            loop {
                let wait = round_end.saturating_duration_since(Instant::now());
                if wait.is_zero() {
                    break;
                }
                match rx.recv_timeout(wait.min(Duration::from_millis(500))) {
                    Ok((target, Ok(()))) => return Ok(target),
                    Ok((target, Err(error))) => errors.push(format!("{target}: {error}")),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if !alive() {
                            return Err("tor stopped while its exit was being proved".into());
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            last = if errors.is_empty() {
                format!("no answer within {}s", budget.as_secs())
            } else {
                errors.join("; ")
            };
            // سطری که خودش پیشرفت است، تا دروازهٔ گیرکردن (`tor_bootstrap`)
            // اثبات را «بی‌حرکتی» نخواند.
            (inner.sink)(&format!(
                "[*] tor is proving its way out: round {round} on three isolated circuits got \
                 nothing ({last}); trying again on fresh ones"
            ));
            thread::sleep(Duration::from_millis(300));
        }
        // <<< AETHER-APP-FIX tor-proof-in-parallel
        Err(last)
    }
    // <<< AETHER-APP-FIX tor-proves-its-exit

    /// نشستِ کنترل، اگر به دست بیاید. نیامدنش شکست نیست.
    // >>> AETHER-APP-PATCH a-bad-circuit-is-not-a-bad-network
    /// از تور یک مدارِ تازه می‌خواهد — `SIGNAL NEWNYM`.
    ///
    /// # چرا این کار لازم است
    ///
    /// در لاگِ میدانی، نشستِ توری که کامل bootstrap شده بود و ترافیک را
    /// می‌برد، ۳۹۷ms و در نشستی دیگر ۲۱۱۸ms تأخیر نشان می‌داد. هیچ‌کدام
    /// خرابیِ اندازه‌گیری نبود ([`crate::ping`] از ۱.۲.۵ روی یک نشستِ گرم و با
    /// یک round trip می‌سنجد): مدارِ تور از سه رله‌ای ساخته می‌شود که تصادفی
    /// انتخاب شده‌اند، و یکی از آن سه می‌تواند آن سرِ دنیا و اشباع باشد. عددِ
    /// بد، *خبرِ همان مدار* است، نه خبرِ شبکه — و تور خودش راهِ عوض‌کردنش را
    /// دارد.
    ///
    /// `NEWNYM` مدارهای موجود را نمی‌بندد؛ فقط استریم‌های تازه را روی مدارِ
    /// تازه می‌فرستد. پس صداکننده باید نشستِ گرمِ پینگ را هم بیندازد
    /// (`ping::reset`)، وگرنه همان مدارِ قدیم را اندازه می‌گیرد و «تغییری
    /// نکرد» نتیجه می‌دهد.
    ///
    /// «بهترینِ سه مدار» شدنی نیست و ادعا هم نمی‌شود: تور راهی برای برگشتن به
    /// مدارِ قبلی نمی‌دهد. کاری که می‌شود کرد این است که از مدارِ *بد* عبور
    /// کنیم و روی اولین مدارِ قابل‌قبول بایستیم.
    pub fn new_circuit(&self, budget: Duration) -> Result<(), String> {
        let (control_file, cookie_file) = self
            .inner
            .control
            .lock()
            .clone()
            .ok_or_else(|| "این نشستِ تور پورتِ کنترل ندارد".to_string())?;
        if !self.is_alive() {
            return Err("تور زنده نیست".into());
        }
        let generation = self.inner.generation.load(Ordering::SeqCst);
        let deadline = Instant::now() + budget;
        let mut session = self
            .open_control(&control_file, &cookie_file, generation, deadline)
            .ok_or_else(|| "پورتِ کنترلِ تور جواب نداد".to_string())?;
        let reply = session.command("SIGNAL NEWNYM", deadline)?;
        if !reply.starts_with("250") {
            return Err(format!("تور NEWNYM را رد کرد: {reply}"));
        }
        Ok(())
    }
    // <<< AETHER-APP-PATCH a-bad-circuit-is-not-a-bad-network

    fn open_control(
        &self,
        control_port_file: &Path,
        cookie_file: &Path,
        generation: u64,
        deadline: Instant,
    ) -> Option<ControlSession> {
        let inner = &self.inner;
        let handshake = deadline.min(Instant::now() + CONTROL_HANDSHAKE_TIMEOUT);
        let control = match read_control_port(control_port_file, CONTROL_FILE_TIMEOUT) {
            Ok(address) => address,
            Err(error) => {
                (inner.sink)(&format!(
                    "[-] {error}; following tor's own log for progress instead"
                ));
                return None;
            }
        };
        let cookie = match read_cookie(cookie_file, CONTROL_FILE_TIMEOUT) {
            Ok(cookie) => cookie,
            Err(error) => {
                (inner.sink)(&format!(
                    "[-] {error}; following tor's own log for progress instead"
                ));
                return None;
            }
        };
        let alive = || {
            inner.generation.load(Ordering::SeqCst) == generation
                && match inner.child.lock().as_mut() {
                    Some(child) => !matches!(child.try_wait(), Ok(Some(_))),
                    None => false,
                }
        };
        match ControlSession::open(control, &cookie, handshake, &alive) {
            Ok(session) => Some(session),
            Err(error) => {
                (inner.sink)(&format!(
                    "[-] tor's control port did not answer in time ({error}); \
                     following its own log for progress instead"
                ));
                None
            }
        }
    }

    /// فرزند را می‌کشد ولی حامل را زنده نگه می‌دارد — برای فاصلهٔ دو موج.
    ///
    /// ترتیب مهم است: اول فاز به Starting می‌رود، بعد فرزند کشته می‌شود. عکسش یک
    /// پنجرهٔ چندمیلی‌ثانیه‌ای می‌سازد که در آن حامل «مرده» دیده می‌شود و ماشینِ حالت
    /// کلِ تلاش را جمع می‌کند.
    fn kill_child_keep_phase(&self) {
        let inner = &self.inner;
        inner.phase.store(PHASE_STARTING, Ordering::SeqCst);
        inner.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(tree) = inner.tree.lock().take() {
            // tor.exe + lyrebird.exe together; dropping the tree closes the job.
            tree.terminate();
        }
        if let Some(mut child) = inner.child.lock().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn stop(&self) {
        let inner = &self.inner;
        inner.phase.store(PHASE_IDLE, Ordering::SeqCst);
        inner.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(tree) = inner.tree.lock().take() {
            // tor.exe + lyrebird.exe together; dropping the tree closes the job.
            tree.terminate();
        }
        if let Some(mut child) = inner.child.lock().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        // >>> AETHER-APP-FIX tor-auto-like-mobile
        // اول فرزند (بالا)، بعد خواننده‌های لاگِ برنده: برعکسش لوله را پیش از
        // مرگِ تور می‌بندد.
        let racers: Vec<TorNative> = std::mem::take(&mut *inner.racers.lock());
        for racer in racers {
            racer.stop();
        }
        let adopted = inner.adopted.lock().take();
        if let Some(winner) = adopted {
            winner.stop();
        }
        // <<< AETHER-APP-FIX tor-auto-like-mobile
        *inner.snapshot.lock() = TorSnapshot::default();
    }
}

impl Drop for TorNative {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            self.stop();
        }
    }
}

fn spawn_log_reader(inner: Arc<Inner>, reader: Box<dyn Read + Send>, generation: u64) {
    thread::spawn(move || {
        let mut lines = BufReader::new(reader).lines();
        while let Some(Ok(line)) = lines.next() {
            if inner.generation.load(Ordering::SeqCst) != generation {
                return;
            }
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            // مسیرِ دوم: هر چه تور دربارهٔ پیشرفت و لیسنرش می‌گوید، پیش از آنکه
            // پورتِ کنترل حرفی زده باشد.
            if let Some(progress) = parse_log_progress(&line) {
                inner.log_progress.fetch_max(progress, Ordering::SeqCst);
            }
            if let Some(port) = parse_log_socks(&line) {
                inner.log_socks.store(port, Ordering::SeqCst);
            }
            if is_headway_line(&line) {
                inner.log_activity.fetch_add(1, Ordering::SeqCst);
            }
            {
                let mut kept = inner.lines.lock();
                if kept.len() == MAX_LINES {
                    kept.pop_front();
                }
                kept.push_back(line.clone());
            }
            (inner.sink)(&format!("[tor] {line}"));
        }
    });
}

// >>> AETHER-APP-FIX tor-proves-its-exit
/// `CONNECT host:port` روی SOCKS5ِ تور، بعد `HEAD /` و خواندنِ `HTTP/`.
fn http_head_via_socks(
    port: u16,
    host: &str,
    dest: u16,
    budget: Duration,
    isolate: Option<&str>,
) -> Result<(), String> {
    let deadline = Instant::now() + budget;
    let proxy = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let mut stream = TcpStream::connect_timeout(&proxy, Duration::from_secs(3))
        .map_err(|error| format!("socks port: {error}"))?;
    let _ = stream.set_nodelay(true);
    let slice = |stream: &TcpStream| -> Result<(), String> {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err("timed out".into());
        }
        stream
            .set_read_timeout(Some(left))
            .and_then(|_| stream.set_write_timeout(Some(left)))
            .map_err(|error| error.to_string())
    };
    let io = |error: std::io::Error| -> String {
        if is_timeout(&error) {
            "timed out".to_string()
        } else {
            error.to_string()
        }
    };

    slice(&stream)?;
    // >>> AETHER-APP-FIX tor-proof-in-parallel
    // با نامِ کاربری (RFC 1929): تور هر نامِ تازه را مدارِ تازه می‌دهد و رمز را
    // نمی‌سنجد.
    match isolate {
        Some(user) => {
            let user = user.as_bytes();
            if user.is_empty() || user.len() > 255 {
                return Err("bad isolation name".into());
            }
            stream.write_all(&[0x05, 0x01, 0x02]).map_err(io)?;
            let mut method = [0u8; 2];
            stream.read_exact(&mut method).map_err(io)?;
            if method != [0x05, 0x02] {
                return Err("tor refused the SOCKS5 greeting".into());
            }
            let mut auth = vec![0x01, user.len() as u8];
            auth.extend_from_slice(user);
            auth.extend_from_slice(&[0x01, b'x']);
            stream.write_all(&auth).map_err(io)?;
            let mut verdict = [0u8; 2];
            stream.read_exact(&mut verdict).map_err(io)?;
            if verdict[1] != 0x00 {
                return Err("tor refused the SOCKS5 isolation name".into());
            }
        }
        None => {
            stream.write_all(&[0x05, 0x01, 0x00]).map_err(io)?;
            let mut method = [0u8; 2];
            stream.read_exact(&mut method).map_err(io)?;
            if method != [0x05, 0x00] {
                return Err("tor refused the SOCKS5 greeting".into());
            }
        }
    }
    // <<< AETHER-APP-FIX tor-proof-in-parallel

    let name = host.as_bytes();
    if name.is_empty() || name.len() > 255 {
        return Err("bad proof host".into());
    }
    let mut request = vec![0x05, 0x01, 0x00, 0x03, name.len() as u8];
    request.extend_from_slice(name);
    request.extend_from_slice(&dest.to_be_bytes());
    slice(&stream)?;
    stream.write_all(&request).map_err(io)?;
    let mut head = [0u8; 4];
    stream.read_exact(&mut head).map_err(io)?;
    if head[0] != 0x05 {
        return Err("not a SOCKS5 reply".into());
    }
    if head[1] != 0x00 {
        return Err(format!(
            "exit refused the stream ({})",
            socks_reply_name(head[1])
        ));
    }
    let rest = match head[3] {
        0x01 => 4 + 2,
        0x04 => 16 + 2,
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).map_err(io)?;
            len[0] as usize + 2
        }
        _ => return Err("malformed SOCKS5 reply".into()),
    };
    let mut skip = vec![0u8; rest];
    stream.read_exact(&mut skip).map_err(io)?;

    let ask = format!(
        "HEAD / HTTP/1.1\r\nHost: {host}\r\nUser-Agent: Mozilla/5.0\r\nAccept: */*\r\nConnection: close\r\n\r\n"
    );
    slice(&stream)?;
    stream.write_all(ask.as_bytes()).map_err(io)?;
    let mut first = [0u8; 5];
    slice(&stream)?;
    stream
        .read_exact(&mut first)
        .map_err(|error| format!("the stream opened but no data came back ({})", io(error)))?;
    if &first == b"HTTP/" {
        Ok(())
    } else {
        Err("the exit answered with something that is not HTTP".into())
    }
}

/// نامِ کدِ پاسخِ SOCKS5، برای لاگ.
fn socks_reply_name(code: u8) -> &'static str {
    match code {
        0x01 => "general failure",
        0x02 => "not allowed by the exit policy",
        0x03 => "network unreachable",
        0x04 => "host unreachable",
        0x05 => "connection refused",
        0x06 => "TTL expired / timed out",
        0x07 => "command not supported",
        0x08 => "address type not supported",
        _ => "unknown reply",
    }
}
// <<< AETHER-APP-FIX tor-proves-its-exit

/// `… [notice] Bootstrapped 45% (requesting_descriptors): …` → `Some(45)`
fn parse_log_progress(line: &str) -> Option<u8> {
    let after = line.split("Bootstrapped ").nth(1)?;
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// `… [notice] Opened Socks listener connection (ready) on 127.0.0.1:1819` → `Some(1819)`
///
/// فقط loopback پذیرفته می‌شود: هر نشانیِ دیگری در این سطر یعنی چیزی را می‌خوانیم
/// که قراردادِ ما نیست، و برداشتنِ عددِ آن یک وعدهٔ دروغ می‌سازد.
fn parse_log_socks(line: &str) -> Option<u16> {
    if !line.contains("Socks listener") {
        return None;
    }
    let address = line.rsplit(" on ").next()?.trim();
    let (host, port) = address.rsplit_once(':')?;
    if host != "127.0.0.1" {
        return None;
    }
    port.trim().parse().ok()
}

// >>> AETHER-APP-PATCH tor-cold-cache-waves
/// سطرهایی از لاگِ tor که بی‌آنکه درصد را عوض کنند، نشانهٔ جلو رفتن‌اند.
///
/// در `loge2` (Tor → Psiphon) موجِ obfs4 روی ۲۵٪ «بی‌پیشرفت» کشته شد درست
/// وقتی که سه دیسکریپتورِ پلِ تازه (`torfnase`، `PraxedisGuerrero`، `freenator`)
/// پشتِ سرِ هم رسیده بودند — یعنی تور داشت دقیقاً کاری را می‌کرد که باید.
fn is_headway_line(line: &str) -> bool {
    line.contains("new bridge descriptor")
        || line.contains("Learned fingerprint")
        || line.contains("Bootstrapped ")
}

/// `250-traffic/read=123456` → `Some(123456)`
fn parse_traffic(reply: &str) -> Option<u64> {
    let after = reply.split("traffic/read=").nth(1)?;
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// ترابرِ موفقِ آخر، اگر ثبت شده (موبایل: `bridges::recall`).
fn recall_transport(home: &Path) -> Option<String> {
    std::fs::read_to_string(home.join(WORKED_FILE))
        .ok()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

/// ترابرِ موفق را برای نشستِ بعد نگه می‌دارد (موبایل: `bridges::remember`).
fn remember_transport(home: &Path, transport: &str) {
    let _ = std::fs::write(home.join(WORKED_FILE), transport.trim());
}

/// ترابرِ موفقِ قبلی را جلوی صف می‌گذارد؛ بقیه به همان ترتیبِ ترجیح.
///
/// موبایل همین را با `waves(plan, first)` می‌کند. بی‌آن، کاربری که روی این
/// شبکه فقط با meek وصل می‌شود هر بار ابتدا یک موجِ کاملِ obfs4 را می‌سوزاند.
fn put_first(mut waves: Vec<String>, worked: Option<&str>) -> Vec<String> {
    if let Some(worked) = worked {
        if let Some(at) = waves.iter().position(|wave| wave == worked) {
            let chosen = waves.remove(at);
            waves.insert(0, chosen);
        }
    }
    waves
}
// <<< AETHER-APP-PATCH tor-cold-cache-waves

/// برچسبِ یک موج، برای لاگ.
fn wave_label(transport: &str) -> String {
    match transport.trim() {
        "" => "direct or via bridges".to_string(),
        other => other.to_string(),
    }
}

/// ترتیبِ ترابرهایی که این نشست می‌آزماید.
///
/// فقط برای پلِ داخلی و وقتی تور خودش روبروی شبکه است. پشتِ یک حاملِ دیگر
/// (`upstream`) پل از اول معنا ندارد، و وقتی کاربر خودش ترابر یا سطرِ پل داده،
/// انتخابِ او جای حدسِ ما را می‌گیرد.
fn plan_waves(launch: &TorLaunch) -> Vec<String> {
    if launch.bridges != BridgeMode::BuiltIn
        || launch.upstream.is_some()
        || !launch.transport.trim().is_empty()
    {
        return vec![launch.transport.trim().to_string()];
    }
    match built_in_waves(&launch.support) {
        Ok(waves) if !waves.is_empty() => {
            put_first(waves, recall_transport(&launch.home).as_deref())
        }
        // فهرست خوانده نشد: همان مسیرِ قدیم، یک نوبت و پیش‌فرضِ obfs4.
        _ => vec![String::new()],
    }
}

/// ترابرهایی که همِ پل دارند و هم بایناریِ اجراکننده‌اش در این نصب هست.
///
/// داده‌محور است، نه فهرستِ دستی: نامِ کلیدها در `pt_config.json` از نسخه‌ای به
/// نسخه‌ای عوض می‌شود (`meek`، `meek-azure`، …) و روشِ واقعی همان اولین واژهٔ
/// خودِ سطرِ پل است. ترتیبِ ترجیح از لاگِ میدانی می‌آید: obfs4 ارزان‌ترین است و
/// اگر باز باشد زود جواب می‌دهد؛ دو تای بعدی domain-fronted اند و جایی به کار
/// می‌آیند که obfs4ِ توکار شمارده و بسته باشد.
fn built_in_waves(support: &Path) -> Result<Vec<String>, String> {
    const PREFERENCE: [&str; 4] = ["obfs4", "snowflake", "meek", "webtunnel"];
    let body = std::fs::read_to_string(support.join("pt_config.json"))
        .map_err(|error| format!("فهرستِ پل‌های داخلی خوانده نشد: {error}"))?;
    let config: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| format!("فهرستِ پل‌های داخلی ناخوانا است: {error}"))?;
    let groups = config
        .get("bridges")
        .and_then(|bridges| bridges.as_object())
        .ok_or("فهرستِ پل‌های داخلی بخشِ bridges ندارد")?;
    let plugins = available_plugins(support);

    let mut usable: Vec<String> = Vec::new();
    for (key, value) in groups {
        let Some(first) = value
            .as_array()
            .and_then(|list| list.iter().find_map(|line| line.as_str()))
        else {
            continue;
        };
        let Some(method) = first.split_whitespace().next() else {
            continue;
        };
        if plugin_for(&plugins, method).is_some() {
            usable.push(key.clone());
        }
    }

    usable.sort_by_key(|key| {
        PREFERENCE
            .iter()
            .position(|wanted| key.contains(wanted))
            .unwrap_or(PREFERENCE.len())
    });
    Ok(usable)
}

/// هر بایناریِ ترابر، و روش‌هایی که واقعاً اجرا می‌کند.
///
/// # چرا این فهرست وجود دارد
///
/// پیش از این، یک سطرِ ثابت می‌گفت lyrebird همِ `meek_lite,obfs4,webtunnel,snowflake` را
/// اجرا می‌کند. lyrebird سه تای اول را می‌دهد و snowflake را نه — و ما هم در
/// `stage-tor.ps1` تنها همین یک بایناری را می‌فرستیم. نتیجهٔ آن وعده این بود که
/// انتخابِ snowflake حتماً به `No running bridges` می‌رسید، بی آنکه کسی بداند چرا.
/// اینجا فقط روشی اعلام می‌شود که بایناری‌اش روی دیسک هست.
///
/// از 1.3.0 (این دور) مرجع `pt_config.json`ِ خودِ بسته است و این فهرست فقط
/// پشتیبانِ نصبی است که آن فایل را ندارد — رجوع به [`declared_plugins`].
fn available_plugins(support: &Path) -> Vec<(String, PathBuf)> {
    // >>> AETHER-APP-FIX snowflake-runs-in-lyrebird
    // اول همان قراردادی که خودِ Tor کنارِ باینری می‌فرستد. بستهٔ 15.0.23
    // (`tor-expert-bundle-windows-*-15.0.23`، دایجستِ پین‌شدهٔ همین مخزن) دیگر
    // `snowflake-client.exe` ندارد؛ `pt_config.json`ِ همان بسته می‌گوید:
    //
    //     "snowflake": "ClientTransportPlugin snowflake exec ${pt_path}lyrebird.exe"
    //
    // فهرستِ ثابتِ پایین این را نمی‌دانست، پس موجِ snowflake روی هر نصب بی‌صدا
    // حذف می‌شد — لاگِ ۸ اکتبر: «tor will try obfs4, then meek» — درست همان
    // ترابری که موبایل (`bridges.rs`: `PT_BINARIES`) روی شبکه‌ای که obfs4ِ
    // توکار را بسته به آن تکیه می‌کند. حالا روش‌ها از خودِ فایل خوانده می‌شوند و
    // فقط وقتی پذیرفته می‌شوند که بایناریِ گفته‌شده واقعاً کنارِ tor باشد.
    let declared = declared_plugins(support);
    if !declared.is_empty() {
        return declared;
    }
    // <<< AETHER-APP-FIX snowflake-runs-in-lyrebird
    const PROVIDERS: [(&str, &str); 3] = [
        (LYREBIRD_FILENAME, "meek_lite,obfs4,webtunnel"),
        (SNOWFLAKE_FILENAME, "snowflake"),
        (CONJURE_FILENAME, "conjure"),
    ];
    PROVIDERS
        .iter()
        .filter_map(|(file, methods)| {
            let path = support.join(file);
            path.is_file().then(|| (methods.to_string(), path))
        })
        .collect()
}

// >>> AETHER-APP-FIX snowflake-runs-in-lyrebird
/// `pluggableTransports`ِ `pt_config.json`: هر مقدار یک سطرِ
/// `ClientTransportPlugin <روش‌ها> exec ${pt_path}<فایل> [آرگومان‌ها]` است.
///
/// فقط ترابرهایی برگردانده می‌شوند که فایلشان کنارِ tor هست. آرگومان‌ها (مثلِ
/// `-registerURL`ِ conjure) کنار گذاشته می‌شوند: torrcِ ما فقط مسیر را می‌نویسد
/// و هیچ موجی بدونِ پلِ داخلیِ همان روش ساخته نمی‌شود.
fn declared_plugins(support: &Path) -> Vec<(String, PathBuf)> {
    let Ok(body) = std::fs::read_to_string(support.join("pt_config.json")) else {
        return Vec::new();
    };
    parse_declared_plugins(&body)
        .into_iter()
        .filter_map(|(methods, file)| {
            let path = support.join(&file);
            path.is_file().then_some((methods, path))
        })
        .collect()
}

/// بخشِ خالصِ [`declared_plugins`]: `(روش‌ها، نامِ فایل)` بدونِ نگاه به دیسک.
fn parse_declared_plugins(body: &str) -> Vec<(String, String)> {
    let Ok(config) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(table) = config
        .get("pluggableTransports")
        .and_then(|value| value.as_object())
    else {
        return Vec::new();
    };
    let mut found: Vec<(String, String)> = Vec::new();
    for line in table.values().filter_map(|value| value.as_str()) {
        let mut words = line.split_whitespace();
        if words.next() != Some("ClientTransportPlugin") {
            continue;
        }
        let Some(methods) = words.next() else {
            continue;
        };
        if words.next() != Some("exec") {
            continue;
        }
        let Some(program) = words.next() else {
            continue;
        };
        let program = program.replace("${pt_path}", "");
        // فقط نامِ فایل: مسیرِ داخلِ بسته (`TorBrowser\Tor\…`) مالِ ما نیست.
        let file = program
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        let methods: Vec<&str> = methods
            .split(',')
            .map(str::trim)
            .filter(|method| !method.is_empty())
            .collect();
        if file.is_empty() || methods.is_empty() {
            continue;
        }
        found.push((methods.join(","), file));
    }
    found
}
// <<< AETHER-APP-FIX snowflake-runs-in-lyrebird

/// کدام ترابر این روش را اجرا می‌کند؟
fn plugin_for<'a>(plugins: &'a [(String, PathBuf)], method: &str) -> Option<&'a (String, PathBuf)> {
    plugins
        .iter()
        .find(|(methods, _)| methods.split(',').any(|known| known == method))
}

#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    /// CREATE_NO_WINDOW: وگرنه هر اجرا یک پنجرهٔ کنسول باز می‌کند.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_command: &mut Command) {}

/// تنظیماتی که تور با آن شروع می‌شود.
fn render_torrc(
    launch: &TorLaunch,
    transport: &str,
    control_port_file: &Path,
    cookie_file: &Path,
) -> Result<String, String> {
    let support = launch.support.as_path();
    let mut config = String::new();
    // هر دو auto: پورتِ ثابت یک چیزِ دیگر است که می‌تواند روی ماشینِ کاربر
    // گرفته باشد، و آن خطا شبیهِ «تور بالا نمی‌آید» دیده می‌شود.
    match launch.socks {
        Some(port) => config.push_str(&format!("SocksPort 127.0.0.1:{port}\n")),
        None => config.push_str("SocksPort auto\n"),
    }
    config.push_str("ControlPort auto\n");
    config.push_str(&format!(
        "ControlPortWriteToFile {}\n",
        quote(control_port_file)
    ));
    config.push_str("CookieAuthentication 1\n");
    config.push_str(&format!("CookieAuthFile {}\n", quote(cookie_file)));
    config.push_str(&format!("DataDirectory {}\n", quote(&launch.home)));
    // >>> AETHER-APP-FIX tor-chain-skips-geoip
    // پشتِ هاپِ جلویی (کانفیگِ سفارشی یا موتور) هیچ گزینهٔ کشوری‌ای در torrc
    // نیست و تور فایلِ GeoIP را برای هیچ تصمیمی نمی‌خواند؛ ولی پارسِ آن روی
    // ماشینِ کاربر ۷ ثانیه پیش از «Bootstrapped 0%» می‌گرفت (لاگِ Config → Tor:
    // 05:38:12.6 → 05:38:19.7). پرچم‌ها از ماژولِ `geoip`ِ خودِ برنامه می‌آیند.
    if launch.upstream.is_some() {
        config.push_str("GeoIPFile \"\"\n");
        config.push_str("GeoIPv6File \"\"\n");
    } else {
        config.push_str(&format!("GeoIPFile {}\n", quote(&support.join("geoip"))));
        config.push_str(&format!("GeoIPv6File {}\n", quote(&support.join("geoip6"))));
    }
    // <<< AETHER-APP-FIX tor-chain-skips-geoip
    // همه‌چیز روی loopback.
    config.push_str("SocksPolicy accept 127.0.0.0/8\n");
    config.push_str("SocksPolicy reject *\n");
    // >>> AETHER-APP-FIX tor-slow-bridge-tuning
    // پلِ کُند و خفه‌شده، نه شبکهٔ باز، حالتِ عادیِ این برنامه است. هر پنج گزینه
    // در `man tor` برای 0.4.9 مستند است:
    //
    // * `ConfluxEnabled 0` — conflux برای هر استریم دو «پا» می‌سازد. پشتِ پل
    //   هر دو پا از همان یک پلِ زنده می‌گذرند، پس فقط مدارِ اضافه روی همان لولهٔ
    //   تنگ است؛ لاگِ ۸ اکتبر: `died due to an invalid selected path, purpose
    //   Unlinked conflux circuit`. arti (موبایل) اصلاً conflux ندارد.
    // * `LearnCircuitBuildTimeout 0` + `CircuitBuildTimeout 60` — مهلتِ یادگرفته
    //   روی پلِ کند چند ثانیه می‌شود و مدارهای سالم را وسطِ ساخت می‌کشد.
    //
    // >>> AETHER-APP-FIX tor-stream-timeout-like-arti
    // `CircuitStreamTimeout 30` برداشته شد. لاگِ «تور + اتر» (۸ اکتبر): یک خروجیِ
    // بد (`Digitalcourage4ip3b`، بعد `Quintex335`) هر استریمِ تازه را ۳۰ ثانیه
    // نگه می‌داشت — «We tried for 30 seconds to connect … Retrying on a new
    // circuit» — درحالی‌که هر سنجهٔ اسکنِ MASQUE فقط ۶ ثانیه دارد؛ سه اسکنِ ۴۵
    // ثانیه‌ای پشتِ هم بی‌نتیجه ماندند. پیش‌فرضِ خودِ تور (۱۰ ثانیه، بعد ۱۵) و
    // مهلتِ اتصالِ arti در موبایل (۱۰ ثانیه) مدارِ بد را زود کنار می‌گذارند.
    //
    // مهلتِ ثابتِ ساختِ مدار فقط برای پل و هُپِ جلویی است؛ تورِ مستقیم با
    // `CircuitBuildTimeout`ِ یادگرفتهٔ خودش سریع‌تر است.
    config.push_str("ConfluxEnabled 0\n");
    if launch.bridges != BridgeMode::None || launch.upstream.is_some() {
        config.push_str("LearnCircuitBuildTimeout 0\n");
        config.push_str("CircuitBuildTimeout 60\n");
    }
    // تورِ خوابیده (Dormant) از کشِ قبلی بیدار نشود و دقیقه‌ای منتظر نماند.
    config.push_str("DormantCanceledByStartup 1\n");
    // <<< AETHER-APP-FIX tor-stream-timeout-like-arti
    // <<< AETHER-APP-FIX tor-slow-bridge-tuning
    // بی‌این، تور پس از مرگِ پروسه‌ای که اجرایش کرده زنده می‌ماند و روی یک
    // کرش، تونلی جا می‌گذارد که هیچ ترافیکی به آن نمی‌رود.
    config.push_str("__OwningControllerProcess ");
    config.push_str(&std::process::id().to_string());
    config.push('\n');

    // هُپِ جلویی، وقتی هست. نه نقل‌قول‌شده و نه URL: تور این یکی را
    // `host:port` خالی می‌خواهد.
    if let Some(address) = launch.upstream {
        config.push_str(&format!("Socks5Proxy {address}\n"));
    }

    // پل برای رسیدن به توری است که خودش بسته است. پشتِ یک حاملِ دیگر، آن
    // پرسش قبلاً پاسخ گرفته — هُپِ جلویی همان چیزی است که ما را بیرون برده —
    // پس تورِ زنجیره‌ای رله‌های مستقیم را می‌گیرد و ترابرها را کنار
    // می‌گذارد. WhiteAesther این ترکیب را هم به‌عمد نمی‌فرستد: تور
    // `Socks5Proxy` و `ClientTransportPlugin` را با هم می‌پذیرد، ولی هرگز
    // دیده نشده که lyrebird پلش را *از راهِ آن پراکسی* بگیرد، و ترابری که
    // آن را نادیده بگیرد سراغِ پل می‌رود؛ همان یک نشانی که روی شبکهٔ
    // سانسورشده نباید لخت گرفته شود.
    let bridges_wanted = launch.bridges != BridgeMode::None && launch.upstream.is_none();
    if bridges_wanted {
        let plugins = available_plugins(support);
        if plugins.is_empty() {
            return Err("ترابرهای افزودنی در این نصب نیستند".into());
        }
        let lines = bridge_lines(launch, transport)?;
        // روشِ هر سطر باید مجریِ زنده‌ای داشته باشد. سطری که مجری ندارد
        // در تور به `No running bridges` ترجمه می‌شود — خطایی که شبکه را متهم
        // می‌کند و تقصیرِ بسته‌بندیِ خودِ ما است. پس صریح گفته می‌شود.
        for line in &lines {
            let method = line.split_whitespace().next().unwrap_or_default();
            if plugin_for(&plugins, method).is_none() {
                return Err(format!(
                    "ترابرِ {method} در این نصب نیست؛ پلی از نوعِ دیگر انتخاب کن"
                ));
            }
        }
        // این مسیرها نقل‌قول نمی‌شوند. دلیلش در سرِ همین فایل است.
        for (methods, path) in &plugins {
            config.push_str(&format!(
                "ClientTransportPlugin {methods} exec {}\n",
                path.to_string_lossy()
            ));
        }
        config.push_str("UseBridges 1\n");
        for line in lines {
            config.push_str(&format!("Bridge {line}\n"));
        }
    }
    Ok(config)
}

/// پل‌هایی که در torrc نوشته می‌شوند.
fn bridge_lines(launch: &TorLaunch, transport: &str) -> Result<Vec<String>, String> {
    match launch.bridges {
        BridgeMode::None => Ok(Vec::new()),
        BridgeMode::Custom => Ok(launch.bridge_lines()),
        BridgeMode::BuiltIn | BridgeMode::Auto => {
            let transport = match transport.trim() {
                "" => "obfs4",
                other => other,
            };
            let lines = built_in_bridges(&launch.support.join("pt_config.json"), transport)?;
            if lines.is_empty() {
                return Err(format!(
                    "این نسخه پلِ داخلیِ {transport} ندارد؛ یکی را دستی بچسبان"
                ));
            }
            Ok(lines)
        }
    }
}

// >>> AETHER-APP-FIX tor-fresh-circuit-for-the-engine
/// موتورِ پشتِ تور گفته درخواستی بی‌جواب ماند؛ `state.rs` در تیکِ بعد NEWNYM می‌زند.
static FRESH_CIRCUIT_WANTED: AtomicBool = AtomicBool::new(false);
/// آخرین بار که این درخواست برآورده شد — تور خودش هم NEWNYM را به هر ۱۰ ثانیه یک‌بار
/// محدود می‌کند.
static FRESH_CIRCUIT_LAST: Mutex<Option<Instant>> = parking_lot::const_mutex(None);
const FRESH_CIRCUIT_GAP: Duration = Duration::from_secs(10);

/// سطرِ موتور: آیا یک درخواستِ موتور از راهِ تور بی‌جواب ماند؟ ارزان: دو `contains`.
pub fn ingest_engine_line(line: &str) {
    if line.contains("did not answer within")
        || line.contains("scan deadline reached with no gateway")
    {
        FRESH_CIRCUIT_WANTED.store(true, Ordering::SeqCst);
    }
}

/// یک بار `true` برمی‌گرداند به‌ازای هر درخواست، و نه زودتر از [`FRESH_CIRCUIT_GAP`].
pub fn take_fresh_circuit_request() -> bool {
    let mut last = FRESH_CIRCUIT_LAST.lock();
    if last.is_some_and(|at| at.elapsed() < FRESH_CIRCUIT_GAP) {
        return false;
    }
    if !FRESH_CIRCUIT_WANTED.swap(false, Ordering::SeqCst) {
        return false;
    }
    *last = Some(Instant::now());
    true
}
// <<< AETHER-APP-FIX tor-fresh-circuit-for-the-engine

/// کلِ وقتِ این نشست: بودجهٔ برنامه، یا [`BOOTSTRAP_TIMEOUT`] اگر نداد.
fn session_budget(launch: &TorLaunch) -> Duration {
    launch.budget.unwrap_or(BOOTSTRAP_TIMEOUT)
}

// >>> AETHER-APP-FIX tor-auto-like-mobile
/// سینکِ یک مسابقه‌دهندهٔ حالتِ خودکار.
///
/// دو تور هم‌زمان حرف می‌زنند، ولی لایهٔ بالا (`tor_bootstrap`، `firsthop`)
/// فقط یک روایت می‌شناسد. پس: درصد فقط وقتی به همان شکلِ همیشگی می‌رود که از
/// بهترینِ هر دو بالاتر باشد (هرگز ۱۰۰ — آن را خودِ حامل پس از تحویل می‌گوید)،
/// بقیهٔ درصدها سطرِ «پیشرفت» می‌شوند، و «first hop» تا معلوم‌شدنِ برنده نگه
/// داشته می‌شود تا کارتِ اتصال نشانیِ بازنده را نشان ندهد.
fn racer_sink(
    outer: LineSink,
    label: &'static str,
    best: Arc<AtomicU8>,
    hop: Arc<Mutex<Option<String>>>,
) -> LineSink {
    Arc::new(move |line: &str| {
        const MARK: &str = "tor reaching the network:";
        if let Some((_, rest)) = line.split_once(MARK) {
            let rest = rest.trim();
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(progress) = digits.parse::<u8>() {
                if progress < 100 && best.fetch_max(progress, Ordering::SeqCst) < progress {
                    outer(&format!("[*] tor reaching the network: {rest} [{label}]"));
                    return;
                }
            }
            outer(&format!(
                "[*] tor is still loading the directory [{label}]: {rest}"
            ));
            return;
        }
        if line.contains("tor first hop: ") {
            *hop.lock() = Some(line.to_string());
            outer(&line.replace(
                "tor first hop: ",
                &format!("tor first hop candidate [{label}]: "),
            ));
            return;
        }
        if let Some(rest) = line.strip_prefix("[tor] ") {
            outer(&format!("[tor:{label}] {rest}"));
            return;
        }
        outer(&format!("{line} [{label}]"));
    })
}

/// نتیجهٔ غربالِ پل‌های داخلیِ یک موج.
#[derive(Debug, PartialEq)]
enum Screen {
    /// همین سطرها به تور داده شوند.
    Use(Vec<String>),
    /// هیچ پلی جواب نداد.
    Skip,
    /// غربال‌پذیر نبود (snowflake/meek/webtunnel، یا فهرست خوانده نشد): دست‌نخورده.
    Keep,
}

/// `ip:port`ِ یک سطرِ پل، اگر واقعاً با TCP سنجیدنی باشد.
///
/// فقط obfs4 و پلِ ساده. snowflake و meek از یک CDN می‌گذرند (موبایل هم
/// نمی‌سنجدشان) و نشانیِ سطرِ webtunnel یک نشانیِ مستند (`[2001:db8::…]`) است
/// که مقصدِ واقعی نیست.
fn bridge_addr(line: &str) -> Option<SocketAddr> {
    let mut words = line.split_whitespace();
    let first = words.next()?;
    if let Ok(address) = first.parse::<SocketAddr>() {
        return Some(address);
    }
    if first != "obfs4" {
        return None;
    }
    let address = words.next()?.parse::<SocketAddr>().ok()?;
    (!is_documentation_addr(&address)).then_some(address)
}

/// از پل‌هایی که جواب دادند، کم‌تأخیرترها اول، حداکثر [`BRIDGE_KEEP`] تا.
///
/// عمداً بر پایهٔ `iat-mode` چیزی کنار گذاشته نمی‌شود: TCP فقط می‌گوید درِ پل
/// باز است، نه اینکه دست‌دادنِ obfs4 از آن رد می‌شود — و در لاگِ ۸ اکتبر همان
/// پلی که واقعاً کار کرد (`212.83.43.95`) `iat-mode=1` بود.
fn choose_bridges(mut reached: Vec<(String, Duration)>) -> Vec<String> {
    reached.sort_by_key(|(_, rtt)| *rtt);
    reached.truncate(BRIDGE_KEEP);
    reached.into_iter().map(|(line, _)| line).collect()
}

/// همهٔ پل‌های سنجیدنی را هم‌زمان با یک TCPِ کوتاه می‌سنجد.
fn reach_bridges(lines: &[String]) -> Vec<(String, Duration)> {
    let (tx, rx) = mpsc::channel::<(String, Duration)>();
    let mut asked = 0usize;
    for line in lines {
        let Some(address) = bridge_addr(line) else {
            continue;
        };
        asked += 1;
        let sender = tx.clone();
        let line = line.clone();
        let _ = thread::Builder::new()
            .name("tor-bridge-reach".into())
            .spawn(move || {
                let began = Instant::now();
                if TcpStream::connect_timeout(&address, BRIDGE_REACH_TIMEOUT).is_ok() {
                    let _ = sender.send((line, began.elapsed()));
                }
            });
    }
    drop(tx);
    let deadline = Instant::now() + BRIDGE_REACH_TIMEOUT + Duration::from_secs(1);
    let mut reached = Vec::new();
    while reached.len() < asked {
        let wait = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(wait) {
            Ok(hit) => reached.push(hit),
            Err(_) => break,
        }
    }
    reached
}

/// غربالِ پل‌های داخلیِ یک موج، به روشِ موبایل.
fn screen_built_in(launch: &TorLaunch, transport: &str, sink: &LineSink) -> Screen {
    let Ok(lines) = bridge_lines(launch, transport) else {
        return Screen::Keep;
    };
    let (probed, rest): (Vec<String>, Vec<String>) = lines
        .into_iter()
        .partition(|line| bridge_addr(line).is_some());
    if probed.is_empty() {
        return Screen::Keep;
    }
    let reached = reach_bridges(&probed);
    let label = wave_label(transport);
    if reached.is_empty() {
        return if rest.is_empty() {
            Screen::Skip
        } else {
            Screen::Use(rest)
        };
    }
    let answered = reached.len();
    let mut kept = choose_bridges(reached);
    sink(&format!(
        "[*] {label}: {answered} of {} built-in bridges answered a TCP probe within {}s; tor \
         gets those {} (dead bridges only cost time)",
        probed.len(),
        BRIDGE_REACH_TIMEOUT.as_secs(),
        kept.len()
    ));
    kept.extend(rest);
    Screen::Use(kept)
}
// <<< AETHER-APP-FIX tor-auto-like-mobile

/// پل‌های داخلیِ خودِ Tor، از فایلی که کنارِ باینری می‌آید.
///
/// به‌عمد فهرستِ ما نیست. فهرستِ دستی می‌پوسد: لاگِ ۱۶ سپتامبر نشان داد یکی
/// از پل‌های فهرستِ ما دیگر جواب نمی‌داد و سطرِ `meek_lite` بی‌فینگرپرینت را
/// arti نمی‌خواند. این یکی را Tor نگه می‌دارد، با همان باینری سفر می‌کند، و
/// زیرِ همان دایجستِ امضاشده‌ای است که اسکریپتِ تدارک بررسی می‌کند.
fn built_in_bridges(path: &Path, transport: &str) -> Result<Vec<String>, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|error| format!("فهرستِ پل‌های داخلی خوانده نشد: {error}"))?;
    parse_built_in(&body, transport)
}

fn parse_built_in(body: &str, transport: &str) -> Result<Vec<String>, String> {
    let config: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("فهرستِ پل‌های داخلی ناخوانا است: {error}"))?;
    Ok(config
        .get("bridges")
        .and_then(|bridges| bridges.get(transport))
        .and_then(|list| list.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|value| value.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

/// مسیر، همان‌طور که torrc می‌خواهد.
///
/// نقل‌قول‌شده و با بک‌اسلشِ دوبرابر: مسیرِ ویندوز پر از بک‌اسلش است و تور در
/// مقدارِ نقل‌قول‌شده آن را escape می‌خواند، پس `C:\Users\…` به مسیری با
/// کاراکترِ کنترلی تبدیل می‌شود و تور فایل را رد می‌کند.
fn quote(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy().replace('\\', "\\\\"))
}

fn read_control_port(path: &Path, timeout: Duration) -> Result<SocketAddr, String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(body) = std::fs::read_to_string(path) {
            // PORT=127.0.0.1:51234
            if let Some(address) = body.trim().strip_prefix("PORT=") {
                if let Ok(parsed) = address.trim().parse() {
                    return Ok(parsed);
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err("تور هیچ پورتِ کنترلی اعلام نکرد".into())
}

fn read_cookie(path: &Path, timeout: Duration) -> Result<Vec<u8>, String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(bytes) = std::fs::read(path) {
            if !bytes.is_empty() {
                return Ok(bytes);
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err("تور کوکیِ کنترلش را ننوشت".into())
}

/// `NOTICE BOOTSTRAP PROGRESS=25 TAG=… SUMMARY="Loading…"`
fn parse_progress(phase: &str) -> Option<u8> {
    phase
        .split("PROGRESS=")
        .nth(1)?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn parse_summary(phase: &str) -> Option<String> {
    let after = phase.split("SUMMARY=\"").nth(1)?;
    let text = after.split('"').next()?.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// اثرِ انگشتِ اولین هاپ از پاسخِ `GETINFO circuit-status`.
///
/// قالبِ پاسخِ تور:
///
/// ```text
/// 250+circuit-status=
/// 5 BUILT $AAAA…~guard,$BBBB…~middle,$CCCC…~exit BUILD_FLAGS=… PURPOSE=GENERAL
/// .
/// 250 OK
/// ```
///
/// فقط مدارِ ساخته‌شده به کار می‌آید؛ مدارِ نیمه‌کاره مسیرِ نهایی‌اش را ندارد.
fn first_hop_of(reply: &str) -> Option<String> {
    for line in reply.lines() {
        let mut parts = line.split_whitespace();
        let _id = parts.next()?;
        if parts.next() != Some("BUILT") {
            continue;
        }
        let path = parts.next()?;
        let hop = path.split(',').next()?;
        let fingerprint = hop.trim_start_matches('$').split(['~', '=']).next()?.trim();
        if fingerprint.len() == 40 && fingerprint.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(fingerprint.to_ascii_uppercase());
        }
    }
    None
}

/// نشانیِ `ip:port`ِ پلی که این اثرِ انگشت مالِ آن است.
///
/// # چرا هر نشانی‌ای پذیرفته نمی‌شود
///
/// سطرهای اسنوفلیک نشانیِ ساختگی دارند — `192.0.2.3:80` از بلوکِ مستنداتِ
/// RFC 5737. آن عدد هیچ‌جا نیست و نشان‌دادنش به کاربر دروغ است: اسنوفلیک از
/// راهِ کارگزارِ WebRTC می‌رود، نه از آن نشانی. پس بلوکِ مستندات رد می‌شود و
/// در آن حالت اندپوینتی گزارش نمی‌شود.
fn bridge_endpoint_of(bridges: &[String], fingerprint: &str) -> Option<String> {
    for line in bridges {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let has = tokens
            .iter()
            .any(|token| token.eq_ignore_ascii_case(fingerprint));
        if !has {
            continue;
        }
        for token in &tokens {
            if let Ok(addr) = token.parse::<SocketAddr>() {
                if is_documentation_addr(&addr) {
                    return None;
                }
                return Some(addr.to_string());
            }
        }
    }
    None
}

/// بلوکِ `192.0.2.0/24` (RFC 5737) — نشانیِ نمونه، نه نشانیِ واقعی.
fn is_documentation_addr(addr: &SocketAddr) -> bool {
    match addr.ip() {
        std::net::IpAddr::V4(v4) => v4.octets()[..3] == [192, 0, 2],
        std::net::IpAddr::V6(_) => false,
    }
}

/// یک گفت‌وگوی احرازشده با پورتِ کنترلِ تور.
struct ControlSession {
    stream: TcpStream,
    /// بایت‌هایی که خوانده شده‌اند ولی هنوز سطرِ کاملی نساخته‌اند.
    ///
    /// # چرا دستی و نه `BufReader`
    ///
    /// خواندن با مهلتِ کوتاه، هر [`CONTROL_READ_SLICE`] یک `TimedOut` می‌دهد و ما
    /// می‌خواهیم صبر کنیم نه شکست بخوریم. اگر آن خطا از میانِ یک سطرِ نیمه‌خوانده بگذرد،
    /// بایت‌های خوانده‌شده نباید گم شوند — پس بافر مالِ خودِ نشست است و بین دو تلاش
    /// باقی می‌ماند.
    pending: Vec<u8>,
}

impl ControlSession {
    /// نشست را می‌گیرد و تا `deadline` دست برنمی‌دارد.
    ///
    /// `alive` هر دور پرسیده می‌شود: توری که مرده، صبرکردن برایش فقط تلف‌کردنِ بودجهٔ
    /// موجِ بعدی است.
    fn open(
        control: SocketAddr,
        cookie: &[u8],
        deadline: Instant,
        alive: &dyn Fn() -> bool,
    ) -> Result<Self, String> {
        let hex: String = cookie.iter().map(|byte| format!("{byte:02x}")).collect();
        loop {
            if !alive() {
                return Err("تور پیش از پاسخ‌دادن به پورتِ کنترل متوقف شد".into());
            }
            match Self::try_open(control, &hex, deadline) {
                Ok(session) => return Ok(session),
                // آخرین علتِ واقعی برگردانده می‌شود، نه یک متنِ آماده: پیشتر یک
                // مقدارِ اولیه اینجا بود که هرگز خوانده نمی‌شد.
                Err(error) => {
                    if Instant::now() >= deadline {
                        return Err(error);
                    }
                }
            }
            thread::sleep(CONTROL_RETRY);
        }
    }

    fn try_open(control: SocketAddr, hex: &str, deadline: Instant) -> Result<Self, String> {
        let stream = TcpStream::connect_timeout(&control, CONTROL_CONNECT_TIMEOUT)
            .map_err(|error| format!("پورتِ کنترلِ تور در دسترس نبود: {error}"))?;
        stream
            .set_read_timeout(Some(CONTROL_READ_SLICE))
            .map_err(|error| format!("تنظیمِ پورتِ کنترلِ تور نشد: {error}"))?;
        let mut session = Self {
            stream,
            pending: Vec::new(),
        };

        // احراز با کوکی، نه رمز: تور فایلی می‌نویسد که فقط همین کاربر
        // می‌خواندش، و ما ثابت می‌کنیم خوانده‌ایمش.
        let reply = session.command(&format!("AUTHENTICATE {hex}"), deadline)?;
        if !reply.starts_with("250") {
            return Err(format!("پورتِ کنترلِ تور احراز را رد کرد: {reply}"));
        }
        Ok(session)
    }

    fn command(&mut self, line: &str, deadline: Instant) -> Result<String, String> {
        self.stream
            .write_all(format!("{line}\r\n").as_bytes())
            .map_err(|error| format!("نوشتن روی پورتِ کنترلِ تور نشد: {error}"))?;
        self.read_reply(deadline)
    }

    /// یک پاسخ را می‌خواند، که می‌تواند چند سطر باشد.
    ///
    /// پروتکل ادامه را با `-` یا `+` پس از کد نشان می‌دهد و سطرِ آخر را با
    /// فاصله؛ پس خواندنِ یک سطر، سطرِ اولِ پاسخِ چندسطری را برمی‌دارد و بقیه
    /// را برای فرمانِ بعدی جا می‌گذارد تا غلط خوانده شود.
    fn read_reply(&mut self, deadline: Instant) -> Result<String, String> {
        let mut reply = String::new();
        loop {
            let line = self.read_line(deadline)?;
            if !reply.is_empty() {
                reply.push('\n');
            }
            reply.push_str(&line);
            // «250 OK» تمامش می‌کند؛ «250-…» و «250+…» نه.
            if line.len() < 4 || line.as_bytes()[3] == b' ' {
                return Ok(reply);
            }
        }
    }

    /// یک سطر، با صبر تا `deadline`.
    ///
    /// مهلتِ سوکت کوچک است و پرشدنش تصمیم نیست: تا زمانی که مهلتِ فرمان نگذشته،
    /// دوباره می‌خوانیم. این همان جایی است که لاگِ ۱۷ سپتامبر در آن سه بار مرد —
    /// os error 10060 روی سوکتی که تور چند ثانیه بعد به آن جواب می‌داد.
    fn read_line(&mut self, deadline: Instant) -> Result<String, String> {
        loop {
            if let Some(at) = self.pending.iter().position(|byte| *byte == b'\n') {
                let line: Vec<u8> = self.pending.drain(..=at).collect();
                return Ok(String::from_utf8_lossy(&line).trim_end().to_string());
            }
            let mut chunk = [0u8; 512];
            match self.stream.read(&mut chunk) {
                Ok(0) => return Err("پورتِ کنترلِ تور بسته شد".into()),
                Ok(read) => self.pending.extend_from_slice(&chunk[..read]),
                Err(error) if is_timeout(&error) => {
                    if Instant::now() >= deadline {
                        return Err(format!("تور در مهلت به پورتِ کنترل جواب نداد ({error})"));
                    }
                }
                Err(error) => return Err(format!("خواندنِ پورتِ کنترلِ تور نشد: {error}")),
            }
        }
    }

    fn get_info(&mut self, key: &str, budget: Duration) -> Result<String, String> {
        let reply = self.command(&format!("GETINFO {key}"), Instant::now() + budget)?;
        if !reply.starts_with("250") {
            return Err(format!("تور GETINFO {key} را رد کرد: {reply}"));
        }
        Ok(reply)
    }

    /// اثرِ انگشتِ اولین هاپِ مدارِ ساخته‌شده — «اندپوینت»ی که واقعاً به آن وصل شده‌ایم.
    fn first_hop(&mut self, budget: Duration) -> Option<String> {
        let reply = self.get_info("circuit-status", budget).ok()?;
        first_hop_of(&reply)
    }

    /// پورتی که تور واقعاً گرفته، پرسیده‌شده نه فرض‌شده.
    fn socks_port(&mut self, budget: Duration) -> Result<u16, String> {
        let reply = self.get_info("net/listeners/socks", budget)?;
        // 250-net/listeners/socks="127.0.0.1:9150"
        let quoted = reply
            .split('"')
            .nth(1)
            .ok_or("تور هیچ لیسنرِ SOCKS اعلام نکرد")?;
        quoted
            .rsplit(':')
            .next()
            .and_then(|port| port.parse().ok())
            .ok_or_else(|| format!("لیسنرِ SOCKSی که تور گفت خوانده نشد: {quoted}"))
    }
}

/// مهلتِ خواندن پر شد — که در ویندوز `TimedOut` است و در برخی مسیرها `WouldBlock`.
fn is_timeout(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch() -> TorLaunch {
        TorLaunch {
            binary: PathBuf::from("C:\\Program Files\\Aether\\tor\\tor.exe"),
            support: PathBuf::from("C:\\Program Files\\Aether\\tor"),
            home: PathBuf::from("C:\\Users\\Sadeghi\\AppData\\Local\\aether\\tor"),
            bridges: BridgeMode::None,
            transport: String::new(),
            custom_bridges: String::new(),
            upstream: None,
            socks: None,
            budget: None,
        }
    }

    #[test]
    fn paths_are_quoted_with_doubled_backslashes() {
        let config = render_torrc(
            &launch(),
            "",
            Path::new("C:\\Users\\Sadeghi\\control-port"),
            Path::new("C:\\Users\\Sadeghi\\cookie"),
        )
        .expect("renders");
        assert!(config.contains(
            "DataDirectory \"C:\\\\Users\\\\Sadeghi\\\\AppData\\\\Local\\\\aether\\\\tor\""
        ));
        assert!(config.contains("SocksPort auto"));
        assert!(config.contains("ControlPort auto"));
        assert!(config.contains("CookieAuthentication 1"));
        assert!(config.contains("__OwningControllerProcess"));
    }

    #[test]
    fn a_chained_tor_takes_direct_relays_and_an_upstream() {
        let mut launch = launch();
        launch.upstream = Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 1819)));
        launch.bridges = BridgeMode::BuiltIn;
        let config = render_torrc(&launch, "", Path::new("c"), Path::new("k")).expect("renders");
        assert!(config.contains("Socks5Proxy 127.0.0.1:1819"));
        assert!(
            config.contains("GeoIPFile \"\"\n"),
            "a chained tor must not parse GeoIP"
        );
        // پشتِ حاملِ دیگر، ترابر کاری ندارد — و lyrebird هم لازم نمی‌شود،
        // پس رندر بی‌آن‌که فایل موجود باشد باید موفق شود.
        assert!(!config.contains("UseBridges"));
        assert!(!config.contains("ClientTransportPlugin"));
    }

    #[test]
    fn pasted_lines_lose_the_bridge_keyword() {
        let mut launch = launch();
        launch.bridges = BridgeMode::Custom;
        launch.custom_bridges =
            "Bridge obfs4 1.2.3.4:80 ABCD cert=x iat-mode=0\n\n# a comment\nobfs4 5.6.7.8:443 EF01 cert=y iat-mode=0\n"
                .into();
        let lines = bridge_lines(&launch, "").expect("lines");
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("obfs4 1.2.3.4:80"));
        assert!(lines[1].starts_with("obfs4 5.6.7.8:443"));
    }

    #[test]
    fn custom_mode_needs_at_least_one_line() {
        let mut launch = launch();
        launch.bridges = BridgeMode::Custom;
        launch.custom_bridges = "   \n# only a comment\n".into();
        assert!(launch.bridge_lines().is_empty());
    }

    #[test]
    fn built_in_bridges_come_from_tors_own_file() {
        // شکلِ واقعیِ pt_config.json در بستهٔ رسمیِ 15.0.23.
        let body = r#"{
            "recommendedDefault": "obfs4",
            "bridges": {
                "meek": ["meek_lite 192.0.2.20:80 url=https://example.invalid front=www.phpmyadmin.net utls=HelloRandomizedALPN"],
                "obfs4": ["obfs4 37.218.245.14:38224 D9A8 cert=bjRa iat-mode=0"],
                "snowflake": ["snowflake 192.0.2.3:80 2B28 fingerprint=2B28 url=https://example.invalid/ fronts=app.datapacket.com,www.datapacket.com ice=stun:stun.epygi.com:3478"]
            }
        }"#;
        let obfs4 = parse_built_in(body, "obfs4").expect("parses");
        assert_eq!(obfs4.len(), 1);
        // سطرِ meek_lite فینگرپرینت ندارد و C-tor آن را می‌پذیرد؛ arti نه.
        // این تفاوت، دلیلِ وجودِ همین ماژول است.
        let meek = parse_built_in(body, "meek").expect("parses");
        assert!(meek[0].starts_with("meek_lite "));
        // `fronts=` جمع است، همان‌طور که Tor می‌فرستد. دست نمی‌زنیم.
        let snowflake = parse_built_in(body, "snowflake").expect("parses");
        assert!(snowflake[0].contains("fronts=app.datapacket.com,www.datapacket.com"));
        assert!(parse_built_in(body, "webtunnel")
            .expect("parses")
            .is_empty());
    }

    #[test]
    fn progress_and_summary_come_out_of_one_line() {
        let phase = "250-status/bootstrap-phase=NOTICE BOOTSTRAP PROGRESS=15 TAG=conn_done \
                     SUMMARY=\"Connecting to a relay\"\n250 OK";
        assert_eq!(parse_progress(phase), Some(15));
        assert_eq!(
            parse_summary(phase).as_deref(),
            Some("Connecting to a relay")
        );
        assert_eq!(parse_progress("250 OK"), None);
    }

    #[test]
    fn a_hundred_percent_is_what_counts() {
        let done = "250-status/bootstrap-phase=NOTICE BOOTSTRAP PROGRESS=100 TAG=done \
                    SUMMARY=\"Done\"\n250 OK";
        assert_eq!(parse_progress(done), Some(100));
    }

    #[test]
    fn a_fixed_socks_port_is_written_as_a_loopback_address() {
        let mut fixed = launch();
        fixed.socks = Some(1819);
        let config = render_torrc(&fixed, "", Path::new("c"), Path::new("k")).expect("renders");
        assert!(config.contains("SocksPort 127.0.0.1:1819"));
        assert!(!config.contains("SocksPort auto"));
        // پورتِ کنترل همچنان auto می‌ماند: دو پورتِ ثابت، دو برخوردِ ممکن.
        assert!(config.contains("ControlPort auto"));
    }

    #[test]
    fn a_missing_binary_is_refused_before_anything_is_written() {
        let mut broken = launch();
        broken.binary = PathBuf::from("/definitely/not/here/tor");
        assert!(broken.validate().is_err());
    }

    /// لاگِ ۱۷ سپتامبر: `Bootstrapped 0% (starting): Starting` تنها روایتِ پیشرفت است
    /// وقتی پورتِ کنترل ساکت مانده.
    #[test]
    fn progress_also_comes_out_of_tors_own_log() {
        assert_eq!(
            parse_log_progress("Sep 17 08:15:33.000 [notice] Bootstrapped 0% (starting): Starting"),
            Some(0)
        );
        assert_eq!(
            parse_log_progress("[notice] Bootstrapped 100% (done): Done"),
            Some(100)
        );
        assert_eq!(
            parse_log_progress("[notice] Starting with guard context"),
            None
        );
    }

    #[test]
    fn the_socks_port_can_be_read_off_the_log_too() {
        assert_eq!(
            parse_log_socks(
                "Sep 17 08:15:30.803 [notice] Opened Socks listener connection (ready) on 127.0.0.1:1819"
            ),
            Some(1819)
        );
        // یک نشانیِ غیرِ loopback قراردادِ ما نیست و نباید به‌عنوانِ پورتِ ما جا بزند.
        assert_eq!(
            parse_log_socks("[notice] Opened Socks listener connection (ready) on 10.0.0.5:9050"),
            None
        );
        assert_eq!(
            parse_log_socks("[notice] Opening Control listener on 127.0.0.1:0"),
            None
        );
    }

    /// یک ترابر، یک باینری. سطرِ ثابتِ قبلی مدعیِ snowflakeی بود که نمی‌فرستادیم.
    #[test]
    fn a_transport_is_only_offered_when_its_binary_is_here() {
        let dir = std::env::temp_dir().join(format!("aether-pt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let lyrebird = dir.join(LYREBIRD_FILENAME);
        std::fs::write(&lyrebird, b"binary").expect("write");

        let plugins = available_plugins(&dir);
        assert!(plugin_for(&plugins, "obfs4").is_some());
        assert!(plugin_for(&plugins, "meek_lite").is_some());
        // snowflake را lyrebird اجرا نمی‌کند.
        assert!(plugin_for(&plugins, "snowflake").is_none());

        std::fs::write(dir.join(SNOWFLAKE_FILENAME), b"binary").expect("write");
        let plugins = available_plugins(&dir);
        assert!(plugin_for(&plugins, "snowflake").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // >>> AETHER-APP-FIX snowflake-runs-in-lyrebird
    /// همان `pluggableTransports`ِ بستهٔ 15.0.23: snowflake را lyrebird اجرا می‌کند.
    const BUNDLE_PT: &str = r#"{
      "pluggableTransports": {
        "lyrebird": "ClientTransportPlugin meek_lite,obfs2,obfs3,obfs4,scramblesuit,webtunnel exec ${pt_path}lyrebird.exe",
        "snowflake": "ClientTransportPlugin snowflake exec ${pt_path}lyrebird.exe",
        "conjure": "ClientTransportPlugin conjure exec ${pt_path}conjure-client.exe -registerURL https://registration.refraction.network/api"
      },
      "bridges": {}
    }"#;

    #[test]
    fn the_bundle_declares_which_binary_runs_each_transport() {
        let declared = parse_declared_plugins(BUNDLE_PT);
        assert_eq!(declared.len(), 3);
        assert!(declared
            .iter()
            .any(|(methods, file)| methods == "snowflake" && file == "lyrebird.exe"));
        assert!(declared
            .iter()
            .any(|(methods, file)| methods.contains("obfs4") && file == "lyrebird.exe"));
        assert!(declared
            .iter()
            .any(|(methods, file)| methods == "conjure" && file == "conjure-client.exe"));
        assert!(parse_declared_plugins("not json").is_empty());
        assert!(parse_declared_plugins("{}").is_empty());
    }

    #[test]
    fn snowflake_is_offered_when_the_bundle_runs_it_in_lyrebird() {
        let dir = std::env::temp_dir().join(format!("aether-pt-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("pt_config.json"), BUNDLE_PT).expect("write");
        std::fs::write(dir.join("lyrebird.exe"), b"binary").expect("write");

        let plugins = available_plugins(&dir);
        assert!(plugin_for(&plugins, "obfs4").is_some());
        assert!(plugin_for(&plugins, "snowflake").is_some());
        // conjure-client.exe روی دیسک نیست، پس اعلام هم نمی‌شود.
        assert!(plugin_for(&plugins, "conjure").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
    // <<< AETHER-APP-FIX snowflake-runs-in-lyrebird

    // >>> AETHER-APP-FIX tor-slow-bridge-tuning
    #[test]
    fn the_torrc_is_tuned_for_slow_bridges() {
        // پشتِ هُپِ جلویی (یا پل): مهلتِ ثابتِ ساختِ مدار.
        let mut chained = launch();
        chained.upstream = Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 1819)));
        let config = render_torrc(&chained, "", Path::new("c"), Path::new("k")).expect("renders");
        assert!(config.contains("ConfluxEnabled 0\n"));
        assert!(config.contains("LearnCircuitBuildTimeout 0\n"));
        assert!(config.contains("CircuitBuildTimeout 60\n"));
        assert!(config.contains("DormantCanceledByStartup 1\n"));
        // AETHER-APP-FIX tor-stream-timeout-like-arti: پیش‌فرضِ ۱۰/۱۵ ثانیهٔ خودِ تور.
        assert!(!config.contains("CircuitStreamTimeout"));
        // تورِ مستقیم مهلتِ ساخت را خودش یاد می‌گیرد.
        let direct = render_torrc(&launch(), "", Path::new("c"), Path::new("k")).expect("renders");
        assert!(direct.contains("ConfluxEnabled 0\n"));
        assert!(!direct.contains("LearnCircuitBuildTimeout"));
        assert!(!direct.contains("CircuitStreamTimeout"));
    }

    // >>> AETHER-APP-FIX tor-auto-like-mobile
    #[test]
    fn only_obfs4_and_plain_bridges_are_probed() {
        let obfs4 = "obfs4 198.51.100.7:443 AAAA cert=x iat-mode=0";
        assert_eq!(
            bridge_addr(obfs4),
            Some("198.51.100.7:443".parse().expect("addr"))
        );
        assert_eq!(
            bridge_addr("198.51.100.8:9001 BBBB"),
            Some("198.51.100.8:9001".parse().expect("addr"))
        );
        assert_eq!(
            bridge_addr("snowflake 192.0.2.3:80 CCCC fingerprint=x url=y"),
            None
        );
        assert_eq!(
            bridge_addr("meek_lite 192.0.2.20:80 DDDD url=x front=y"),
            None
        );
        assert_eq!(
            bridge_addr("webtunnel [2001:db8::1]:443 EEEE url=https://x"),
            None
        );
    }

    #[test]
    fn answered_bridges_are_kept_fastest_first() {
        let a = "obfs4 198.51.100.1:443 A cert=x iat-mode=1".to_string();
        let b = "obfs4 198.51.100.2:443 B cert=x iat-mode=0".to_string();
        let kept = choose_bridges(vec![
            (b.clone(), Duration::from_millis(300)),
            (a.clone(), Duration::from_millis(10)),
        ]);
        // پلِ iat-mode=1 هم می‌ماند: تنها پلِ کارگرِ لاگِ ۸ اکتبر همین بود.
        assert_eq!(kept, vec![a, b]);
    }

    #[test]
    fn racers_share_one_story() {
        let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let record = seen.clone();
        let outer: LineSink = Arc::new(move |line: &str| record.lock().push(line.to_string()));
        let best = Arc::new(AtomicU8::new(0));
        let hop_a = Arc::new(Mutex::new(None));
        let hop_b = Arc::new(Mutex::new(None));
        let a = racer_sink(outer.clone(), "direct", best.clone(), hop_a.clone());
        let b = racer_sink(outer, "bridges", best, hop_b.clone());
        a("[*] tor reaching the network: 10%: Connecting");
        b("[*] tor reaching the network: 5%: Connecting");
        b("[*] tor reaching the network: 100%: Done");
        b("[+] tor first hop: 198.51.100.2:443 via obfs4");
        a("[tor] Bootstrapped 10%");
        let lines = seen.lock().clone();
        assert_eq!(
            lines[0],
            "[*] tor reaching the network: 10%: Connecting [direct]"
        );
        // درصدِ پایین‌تر و ۱۰۰ هیچ‌کدام به‌شکلِ درصد نمی‌روند، ولی پیشرفت‌اند.
        assert!(lines[1].starts_with("[*] tor is still loading the directory"));
        assert!(!lines[2].contains("tor reaching the network:"));
        assert!(!lines[3].contains("tor first hop: "));
        assert_eq!(
            hop_b.lock().as_deref(),
            Some("[+] tor first hop: 198.51.100.2:443 via obfs4")
        );
        assert!(hop_a.lock().is_none());
        assert_eq!(lines[4], "[tor:direct] Bootstrapped 10%");
    }

    #[test]
    fn an_unanswered_engine_request_asks_for_one_fresh_circuit() {
        *FRESH_CIRCUIT_LAST.lock() = None;
        FRESH_CIRCUIT_WANTED.store(false, Ordering::SeqCst);
        ingest_engine_line("[+] selected MASQUE gateway 162.159.198.2:443");
        assert!(!take_fresh_circuit_request());
        ingest_engine_line(
            "WARN aether::account] [!] registration retry 1/4 in 0.5s: api: registration: api: \
             api.cloudflareclient.com did not answer within 20s",
        );
        assert!(take_fresh_circuit_request());
        assert!(!take_fresh_circuit_request());
        // تور NEWNYM را زودتر از ۱۰ ثانیه نمی‌پذیرد؛ درخواستِ تازه منتظر می‌ماند.
        ingest_engine_line("WARN aether::prober] [-] scan deadline reached with no gateway");
        assert!(!take_fresh_circuit_request());
        assert!(FRESH_CIRCUIT_WANTED.load(Ordering::SeqCst));
    }

    #[test]
    fn auto_validates_and_never_runs_waves_by_itself() {
        let mut auto = launch();
        auto.bridges = BridgeMode::Auto;
        assert_eq!(plan_waves(&auto), vec![String::new()]);
        assert_eq!(
            serde_json::to_string(&BridgeMode::Auto).expect("json"),
            "\"auto\""
        );
    }
    // <<< AETHER-APP-FIX tor-auto-like-mobile
    // <<< AETHER-APP-FIX tor-slow-bridge-tuning

    // >>> AETHER-APP-FIX tor-proves-its-exit
    #[test]
    fn the_exit_proof_matches_mobile() {
        assert_eq!(PROOF_LIMIT.as_secs(), 45);
        assert!(PROOF_PARALLEL_ATTEMPT <= PROOF_LIMIT);
        assert!(PROOF_TARGETS
            .iter()
            .all(|(host, _)| host.parse::<Ipv4Addr>().is_err()));
        assert_eq!(socks_reply_name(0x06), "TTL expired / timed out");
    }

    #[test]
    fn a_dead_socks_port_does_not_prove_anything() {
        // پورتی که کسی در آن گوش نمی‌دهد: باید خطا بدهد، نه هنگ کند.
        let free = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = free.local_addr().expect("addr").port();
        drop(free);
        let started = Instant::now();
        assert!(
            http_head_via_socks(port, "example.com", 80, Duration::from_secs(5), None).is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(5));
    }
    // <<< AETHER-APP-FIX tor-proves-its-exit

    /// بودجه از برنامه می‌آید، وگرنه لاگ `timeout=600s` می‌گوید و کد ۱۸۰ ثانیه اعمال
    /// می‌کند — همان ناسازگاریِ لاگِ ۱۷ سپتامبر.
    #[test]
    fn the_session_budget_is_the_apps_not_a_constant() {
        let mut wide = launch();
        wide.budget = Some(Duration::from_secs(600));
        assert_eq!(wide.budget.unwrap_or(BOOTSTRAP_TIMEOUT).as_secs(), 600);
        assert_eq!(
            launch().budget.unwrap_or(BOOTSTRAP_TIMEOUT),
            BOOTSTRAP_TIMEOUT
        );
    }

    // >>> AETHER-APP-PATCH tor-cold-cache-waves
    #[test]
    fn the_transport_that_worked_goes_first() {
        let waves = vec![
            "obfs4".to_string(),
            "snowflake".to_string(),
            "meek".to_string(),
        ];
        assert_eq!(
            put_first(waves.clone(), Some("meek")),
            vec![
                "meek".to_string(),
                "obfs4".to_string(),
                "snowflake".to_string()
            ]
        );
        assert_eq!(put_first(waves.clone(), None), waves);
        assert_eq!(put_first(waves.clone(), Some("webtunnel")), waves);
    }

    #[test]
    fn a_fresh_bridge_descriptor_is_headway() {
        assert!(is_headway_line(
            "[notice] new bridge descriptor 'torfnase' (fresh): $3956~torfnase at 212.83.43.74"
        ));
        assert!(is_headway_line(
            "[notice] Learned fingerprint 255E for bridge 192.0.2.20:80 (with transport 'meek_lite')."
        ));
        assert!(!is_headway_line(
            "[notice] Ignoring directory request, since no bridge nodes are available yet."
        ));
        assert!(!is_headway_line(
            "[warn] Proxy Client: unable to connect OR connection (handshaking (proxy)) with 146.57.248.225:22"
        ));
    }

    #[test]
    fn traffic_is_read_off_the_control_reply() {
        assert_eq!(
            parse_traffic("250-traffic/read=1048576\n250 OK"),
            Some(1_048_576)
        );
        assert_eq!(parse_traffic("552 Unrecognized key"), None);
    }

    #[test]
    fn the_cold_cache_numbers_match_mobile() {
        assert_eq!(WAVE_STALL.as_secs(), 75);
        assert_eq!(WAVE_BUDGET.as_secs(), 360);
        assert_eq!(WAVE_TRIES, 2);
    }
    // <<< AETHER-APP-PATCH tor-cold-cache-waves

    /// بی پلِ داخلی، موجی هم نیست: یک نوبت، همان‌طور که کاربر خواسته.
    #[test]
    fn waves_are_only_for_the_built_in_list() {
        let direct = launch();
        assert_eq!(plan_waves(&direct), vec![String::new()]);

        let mut chosen = launch();
        chosen.bridges = BridgeMode::BuiltIn;
        chosen.transport = "snowflake".into();
        assert_eq!(plan_waves(&chosen), vec!["snowflake".to_string()]);

        // پشتِ حاملِ دیگر، پل معنا ندارد و نردبان هم نه.
        let mut chained = launch();
        chained.bridges = BridgeMode::BuiltIn;
        chained.upstream = Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 1819)));
        assert_eq!(plan_waves(&chained), vec![String::new()]);
    }
}
