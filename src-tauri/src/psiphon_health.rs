//! پورت از `transport/PsiphonHealth.kt`.
//!
//! یک نشست Psiphon را می‌پاید تا سروری را پیدا کند که **بالا، سریع و بی‌فایده**
//! است: تونل برقرار است، دست‌دادن موفق بوده، بایت جابه‌جا می‌شود — و همان سرور
//! باز کردن مقصدهای خاصی را رد می‌کند.
//!
//! ## ریشه‌ای که این ماژول برایش وجود دارد
//!
//! گزارش میدانی موبایل: «روی بعضی سرورها تلگرام همه‌چیز را باز می‌کند ولی گوگل
//! نه.» این شانس نیست و تونل هم خراب نیست؛ لاگ سازوکار را با اسم صدا می‌زند:
//!
//! ```text
//! LocalProxyError: {"message":"... (*Tunnel).dialChannel...:
//!                   ssh: rejected: administratively prohibited"}
//! Info: {"message":"port forward failures for OsBTQokd: 3"}
//! ```
//!
//! `administratively prohibited` یعنی سرور SSH باز کردن port forward را رد
//! می‌کند. تونل سالم است، خروجی سالم است، TCP به مقصدهای دیگر سالم است — همین
//! **یک سرور** این مقصدهای خاص را نمی‌گیرد.
//!
//! ## تفاوت معنادار با نسخهٔ اندروید (و دلیلش)
//!
//! در اندروید شاهدِ «مقصدهای متمایز رد‌شده» از `PsiphonSocksFront` می‌آمد، چون
//! آن لایه تنها جایی بود که می‌دانست *چه چیزی* رد شد. در ویندوز مسیر دادهٔ
//! واقعی پروکسی سیستمی → پل `share.rs` است، و پل همان نقش را دارد: هر CONNECT
//! را خودش به SOCKS5 استیج ۲ می‌دهد و کد پاسخ را می‌بیند.
//!
//! و یک سخت‌گیری بیشتر از اندروید: فقط پاسخ **`REP = 0x02`**
//! (`connection not allowed by ruleset` — ترجمهٔ SOCKS5 همان
//! «administratively prohibited») به‌عنوان شاهد شمرده می‌شود. یک مقصد که خودش
//! خوابیده (`REP=0x04/0x05`) یا تایم‌اوت شبکه، شاهدِ سانسور نیست و اینجا هرگز
//! شمرده نمی‌شود. این دقیقاً همان اشتباهی است که در اندروید یک‌بار رخ داد و
//! باعث شد واچ‌داگ روی **بار** شلیک کند نه روی فیلترینگ.
//!
//! ## حصارهای ایمنی (تا این چیز هرگز حلقهٔ اتصال مجدد نشود)
//!
//! شمارنده‌ها در پنجرهٔ لغزان [WINDOW_MS] زندگی می‌کنند، رد‌شدن‌هایی که در
//! [SETTLE_MS] اولِ یک تونل تازه می‌رسند به تونلِ **ترک‌شده** نسبت داده می‌شوند
//! (نه به جانشینش)، بین دو چرخش [COOLDOWN_MS] فاصله است، و در کل نشست حداکثر
//! [MAX_ROTATIONS] چرخش مجاز است. سروری که [HEALTHY_MS] پاک بماند بودجه را
//! برمی‌گرداند، تا یک نشست هشت‌ساعته به‌خاطر دو دقیقهٔ بد اولش بی‌دفاع نماند.

use crate::log::DiagnosticsLog;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

const TAG: &str = "PsiphonHealth";

/// رد‌شدن‌های داخل این پنجرهٔ لغزان است که «این سرور فیلتر می‌کند» را می‌سازد.
const WINDOW_MS: u64 = 45_000;

/// مقصدهای متمایزِ رد‌شده در یک پنجره که چرخش را کلید می‌زنند.
const DISTINCT_TARGETS_TRIGGER: usize = 6;

/// شمارندهٔ خودِ تونل که **می‌تواند** چرخش بدهد — ولی هرگز تنها، فقط همراه
/// [FAILURE_CORROBORATION].
///
/// این عدد بزرگ است چون شمارندهٔ خام است: پخش‌کنندهٔ ویدیو ده‌ها جریان موازی
/// باز می‌کند و ۲۵ رد‌شدن یک ثانیهٔ عادیِ ویدیو است، نه سانسور.
const FAILURE_DELTA_TRIGGER: u64 = 60;

/// مقصدهای متمایزی که باید در همان پنجره ثبت شده باشند تا
/// [FAILURE_DELTA_TRIGGER] اجازهٔ چرخش داشته باشد.
///
/// عمداً کمتر از [DISTINCT_TARGETS_TRIGGER]: کار شمارنده این است که یک حکم را
/// **سریع‌تر** کند، نه اینکه بتواند تنهایی به آن برسد.
const FAILURE_CORROBORATION: usize = 3;

/// کمینهٔ فاصلهٔ دو چرخش.
const COOLDOWN_MS: u64 = 60_000;

/// فاصلهٔ کوتاه‌تر وقتی چرخش اثبات‌شده جابه‌جا نکرده: تونل تازه روی سروری است
/// که همین حالا در فهرست سیاه است. کاربر به یک اتصال خراب زل زده و انتظار یک
/// دقیقه هیچ چیزی یاد نمی‌دهد.
const REPEAT_COOLDOWN_MS: u64 = 12_000;

/// مهلت آرام‌گرفتن در شروع هر تونل تازه.
const SETTLE_MS: u64 = 6_000;

/// یک دورهٔ پاک به این اندازه، بودجهٔ چرخش را برمی‌گرداند.
const HEALTHY_MS: u64 = 180_000;

/// سقف قطعی در هر نشست، پیش از هر بازپرداختِ [HEALTHY_MS].
const MAX_ROTATIONS: u32 = 4;

/// هر چند رگبار، یک سطر لاگ. رگبار رد‌شدن نباید رینگ لاگ را بخورد.
const BURST_LOG_EVERY: u64 = 20;

/// راهبرد یک چرخش.
///
/// در اندروید [Rotation::Reconnect] یعنی `reconnectPsiphon()` (ارزان، درجا،
/// بی‌قابلیت هدایت) و [Rotation::Restart] یعنی `restartPsiphon()` که تنها مسیری
/// است که کانفیگ را دوباره می‌خواند و می‌تواند فهرست سیاه را به فیلتر واقعی
/// تبدیل کند.
///
/// در ویندوز Psiphon یک **فرآیند** است، نه کتابخانه، پس هر دو حالت یک راه‌اندازی
/// مجدد فرآیند هستند؛ تفاوت همان تفاوت معنادار اندروید است و باقی می‌ماند:
/// `Reconnect` با همان کانفیگ برمی‌گردد، `Restart` با کانفیگ هدایت‌شده.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Reconnect,
    Restart,
}

impl Rotation {
    fn label(self) -> &'static str {
        match self {
            Self::Reconnect => "reconnect",
            Self::Restart => "restart",
        }
    }
}

/// سروری که ترک می‌شود، تا `psiphon.rs` بتواند دورش بزند.
#[derive(Debug, Clone, Default)]
pub struct Target {
    pub server_id: String,
    pub region: String,
}

/// `Arc` و نه `Box`: [dispatch] باید بتواند اشاره‌گر را از زیر قفل بیرون بکشد
/// و **بیرون** از قفل صدایش بزند. با `Box` تنها راه، نگه‌داشتن قفل در طول
/// اجرای چرخش بود — و چرخش، فرآیند Psiphon را می‌کشد و بالا می‌آورد، پس همان
/// قفل را از مسیر notice دوباره می‌خواهد: بن‌بست قطعی.
type RotateFn = Arc<dyn Fn(Rotation, Target) + Send + Sync>;

struct Health {
    /// تابعی که چرخش را اجرا می‌کند. `None` = هیچ نشستی متصل نیست.
    action: Option<RotateFn>,
    active_server: String,
    active_region: String,
    /// سرورهای محکوم‌شدهٔ این نشست، و منطقهٔ خروجشان.
    filtering: HashSet<String>,
    filtering_regions: HashSet<String>,
    /// یک‌بار اعلام، نه در هر شاهد تازه.
    announced: HashSet<String>,
    /// نگاشت شناسهٔ سرور به منطقه‌اش (از `ConnectedServer`).
    server_regions: HashMap<String, String>,
    /// مقصدهای رد‌شده در پنجرهٔ جاری: `host:port` → زمان.
    refused_targets: HashMap<String, u64>,
    window_started_at: u64,
    /// مبنای شمارندهٔ port-forward برای پنجرهٔ جاری. `None` = هنوز تنظیم نشده.
    window_base_failures: Option<u64>,
    settle_until: u64,
    tunnel_since: u64,
    rotations: u32,
    last_rotation_at: u64,
    loud_failure_bursts: u64,
}

impl Health {
    fn new() -> Self {
        Self {
            action: None,
            active_server: String::new(),
            active_region: String::new(),
            filtering: HashSet::new(),
            filtering_regions: HashSet::new(),
            announced: HashSet::new(),
            server_regions: HashMap::new(),
            refused_targets: HashMap::new(),
            window_started_at: 0,
            window_base_failures: None,
            settle_until: 0,
            tunnel_since: 0,
            rotations: 0,
            last_rotation_at: 0,
            loud_failure_bursts: 0,
        }
    }

    fn server_label(&self) -> String {
        if self.active_server.is_empty() {
            "(unknown)".to_string()
        } else {
            self.active_server.clone()
        }
    }

    /// شمارنده‌ها را به [server] می‌چسباند.
    fn adopt_server(&mut self, server: &str) {
        if self.active_server != server {
            self.active_server = server.to_string();
            self.refused_targets.clear();
            self.window_started_at = 0;
            self.window_base_failures = None;
        }
        self.active_region = self.server_regions.get(server).cloned().unwrap_or_default();
    }

    /// پنجره را باز می‌کند، یا اگر منقضی شده رول می‌کند.
    ///
    /// وقتی پنجره باز است هم ورودی‌های کهنه حذف می‌شوند، تا یک چکهٔ آرام در ده
    /// دقیقه هرگز جمع نشود و به آستانه نرسد.
    fn open_or_roll_window(&mut self, now: u64) {
        if self.window_started_at == 0 || now.saturating_sub(self.window_started_at) > WINDOW_MS {
            self.window_started_at = now;
            self.window_base_failures = None;
            self.refused_targets.clear();
        } else {
            self.refused_targets
                .retain(|_, at| now.saturating_sub(*at) <= WINDOW_MS);
        }
    }

    /// حکم را یک‌بار به نام سرور فعال ثبت می‌کند.
    fn convict(&mut self, why: &str) {
        if self.active_server.is_empty() {
            return;
        }
        let server = self.active_server.clone();
        self.filtering.insert(server.clone());
        if let Some(region) = self.server_regions.get(&server) {
            if !region.is_empty() {
                self.filtering_regions.insert(region.clone());
            }
        }
        if !self.announced.insert(server) {
            return;
        }
        DiagnosticsLog::w(
            TAG,
            &format!(
                "Server {} has {} in {}s (ssh 'administratively prohibited'); \
                 this server filters rather than fails — excluded for the rest of this session.",
                self.server_label(),
                why,
                WINDOW_MS / 1000
            ),
        );
    }

    /// بودجهٔ چرخش را پس می‌دهد اگر نشست به‌قدر کافی آرام و سالم بوده.
    fn refund_if_healthy(&mut self, now: u64) {
        if self.rotations == 0 {
            return;
        }
        if self.tunnel_since == 0 || now.saturating_sub(self.tunnel_since) < HEALTHY_MS {
            return;
        }
        if self.filtering.contains(&self.active_server) {
            return;
        }
        self.rotations = 0;
        DiagnosticsLog::i(
            TAG,
            &format!(
                "Server {} has been clean for {} min — rotation budget restored.",
                self.server_label(),
                HEALTHY_MS / 60_000
            ),
        );
    }

    /// تصمیم چرخش. خودِ فراخوانی **بیرون** از قفل انجام می‌شود (مقدار برگشتی).
    fn decide_rotation(&mut self, now: u64, why: &str, repeat: bool) -> Option<(Rotation, Target)> {
        if self.action.is_none() {
            return None;
        }
        self.refund_if_healthy(now);

        let done = self.rotations;
        if done >= MAX_ROTATIONS {
            // بن‌بست عمدی: کاربری که به تونلِ کارکنده‌ولی‌فیلترشده نگاه می‌کند،
            // باز هم وضعش از کسی بهتر است که تونلش هر دقیقه پایین می‌آید.
            return None;
        }
        let cooldown = if repeat {
            REPEAT_COOLDOWN_MS
        } else {
            COOLDOWN_MS
        };
        if self.last_rotation_at != 0 && now.saturating_sub(self.last_rotation_at) < cooldown {
            return None;
        }

        let attempt = done + 1;
        let strategy = if attempt == 1 && !repeat {
            Rotation::Reconnect
        } else {
            Rotation::Restart
        };
        let target = Target {
            server_id: self.active_server.clone(),
            region: self.active_region.clone(),
        };
        self.rotations = attempt;
        self.last_rotation_at = now;
        // پنجرهٔ تازه: شمارنده‌ها سرورِ ترک‌شده را توصیف می‌کنند، و مهلت
        // آرام‌گرفتن نمی‌گذارد جریان‌های در حال مرگش جانشین را متهم کنند.
        self.window_started_at = 0;
        self.window_base_failures = None;
        self.refused_targets.clear();
        self.settle_until = now + SETTLE_MS;

        DiagnosticsLog::w(
            TAG,
            &format!(
                "Rotating off server {} ({why}) — {} {attempt}/{MAX_ROTATIONS}. \
                 The pipeline stays up; only the exit server changes.",
                self.server_label(),
                strategy.label(),
            ),
        );
        Some((strategy, target))
    }
}

/// Destinations that belong to the app's OWN health probes.
///
/// # Root cause this closes (ported from mobile `PsiphonHealth.selfProbeTargets`)
///
/// `on_destination_refused` treats a refused CONNECT as evidence that the exit
/// filters. The app's own probes go through the same SOCKS5 path, so every probe
/// a server declined was counted against that server. Nothing was wedged: the app
/// convicted its exit, dropped the session and rebuilt it, on a schedule, forever
/// — which is the "ping spikes, it disconnects, it comes back, over and over"
/// report. Probes now use port 443 (see [`crate::ping`]) and they register here so
/// they can never be counted as evidence about a server no matter which port they
/// end up on.
fn self_probes() -> &'static Mutex<HashSet<String>> {
    static CELL: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Declares `host:port` as one of the app's own health-probe destinations.
/// Idempotent, and safe to call before a session exists.
pub fn register_self_probe(host: &str, port: u16) {
    self_probes().lock().insert(format!("{host}:{port}"));
}

/// True when `host:port` is one of the app's own probes.
pub fn is_self_probe(host: &str, port: u16) -> bool {
    self_probes().lock().contains(&format!("{host}:{port}"))
}

fn cell() -> &'static Mutex<Health> {
    static CELL: OnceLock<Mutex<Health>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Health::new()))
}

/// ساعت یکنواخت نشست. `SystemTime` عمداً استفاده نمی‌شود: پرش ساعت سیستم
/// نباید پنجره‌ها و کول‌داون‌ها را به هم بزند.
fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// چرخش را به این نشست وصل می‌کند. با هر کنترلر تازه دوباره صدا زده می‌شود.
pub fn bind<F>(rotate: F)
where
    F: Fn(Rotation, Target) + Send + Sync + 'static,
{
    cell().lock().action = Some(Arc::new(rotate));
}

/// همه‌چیز را پاک می‌کند — یک اتصال تازه از صفر شروع می‌شود.
/// سرورهای بدِ دیروز مسئلهٔ نشست امروز نیستند.
pub fn reset() {
    *cell().lock() = Health::new();
}

/// آیا این سرور در این نشست محکوم شده است؟
pub fn is_filtering(server_id: &str) -> bool {
    cell().lock().filtering.contains(server_id)
}

/// مناطقی که سرور فیلترکننده در آن‌ها دیده شده — خوراک هدایت در `psiphon.rs`.
pub fn filtering_regions() -> HashSet<String> {
    cell().lock().filtering_regions.clone()
}

/// پل `share.rs` یک CONNECT را با `REP=0x02` رد‌شده دیده است.
///
/// **فقط** همین یک کد پاسخ اینجا می‌آید (نگاه کنید به مستند بالای فایل): مقصدی
/// که خودش خوابیده یا تایم‌اوت شبکه، شاهد سانسور نیست.
pub fn on_destination_refused(host: &str, port: u16) {
    // A refusal of one of our OWN probes says nothing about the server, and
    // counting it is how a healthy exit used to get convicted. See [`self_probes`].
    if is_self_probe(host, port) {
        return;
    }
    let now = now_ms();
    let decision = {
        let mut h = cell().lock();
        if h.active_server.is_empty() {
            return;
        }
        // رد‌شدن‌های چند صد میلی‌ثانیهٔ اولِ یک تونل تازه، دنبالهٔ تونل ترک‌شده
        // است، نه شاهدی علیه جانشین.
        if now < h.settle_until {
            return;
        }
        h.open_or_roll_window(now);
        h.refused_targets.insert(format!("{host}:{port}"), now);
        let distinct = h.refused_targets.len();
        if distinct < DISTINCT_TARGETS_TRIGGER {
            None
        } else {
            let why = format!("refused {distinct} different destinations");
            h.convict(&why);
            h.decide_rotation(now, &why, false)
        }
    };
    dispatch(decision);
}

/// هر سطر notice که از فرآیند Psiphon می‌آید — پیش از هر فیلتر لاگ.
pub fn on_notice(line: &str) {
    let kind = notice_type(line);
    match kind.as_deref() {
        // سرور فعلی و منطقه‌اش. باید پیش از ActiveTunnel ثبت شود تا هدایت
        // بداند از کدام کشور دارد فرار می‌کند.
        Some("ConnectedServer") => {
            if let Some(id) = json_str(line, "diagnosticID") {
                let region = json_str(line, "region").unwrap_or_default();
                let mut h = cell().lock();
                if !region.is_empty() {
                    h.server_regions.insert(id.clone(), region);
                }
            }
        }
        // یک تونل بالا آمد — تنها لحظه‌ای که می‌توان فهمید چرخش کاری کرد یا نه.
        Some("ActiveTunnel") => {
            if let Some(id) = json_str(line, "diagnosticID") {
                on_tunnel_established(&id);
            }
        }
        // شمارندهٔ خودِ تونل: `port forward failures for <id>: <n>`.
        Some("Info") | Some("Alert") | Some("Warning") => {
            if let Some((server, count)) = parse_port_forward_failures(line) {
                on_port_forward_failures(&server, count);
            }
        }
        _ => {}
    }
}

/// `ActiveTunnel` — عمداً روی **هر** تونل برقرارشده، نه فقط روی تغییر شناسه:
/// تمام باگ همین بود که شناسه عوض **نمی‌شد** و کدِ قدیمی که ریست را به
/// «شناسه تغییر کرد» گره زده بود، دقیقاً در همان حالتِ مهم هیچ کاری نمی‌کرد.
fn on_tunnel_established(server_id: &str) {
    let now = now_ms();
    let decision = {
        let mut h = cell().lock();
        let known = h.filtering.contains(server_id);
        h.adopt_server(server_id);
        h.settle_until = now + SETTLE_MS;
        h.tunnel_since = now;
        if !known {
            None
        } else {
            // چرخش ما را جابه‌جا نکرد. همان‌جا با راهبرد قوی‌تر دوباره صادرش
            // می‌کنیم، نه اینکه منتظر بمانیم شاهدی را که داریم دوباره بسازیم.
            DiagnosticsLog::w(
                TAG,
                &format!(
                    "The new tunnel is carried by {} again, which already proved it filters — \
                     rotating again with that server excluded.",
                    h.server_label()
                ),
            );
            let why = format!("landed back on the filtering server {}", h.server_label());
            h.decide_rotation(now, &why, true)
        }
    };
    dispatch(decision);
}

fn on_port_forward_failures(server: &str, count: u64) {
    let now = now_ms();
    let decision = {
        let mut h = cell().lock();
        if h.active_server.is_empty() {
            h.adopt_server(server);
        }
        // شمارنده‌ای که برای سروری گزارش شده که دیگر نشست را حمل نمی‌کند،
        // دنبالهٔ یک تونل رهاشده است، نه شاهد.
        if server != h.active_server {
            return;
        }
        if now < h.settle_until {
            return;
        }
        h.open_or_roll_window(now);
        let base = *h.window_base_failures.get_or_insert(count);
        // شمارنده در هر سرور یکنواخت است و با تغییر سرور صفر می‌شود، پس دلتای
        // منفی یعنی سرور تازه از عددی کوچک‌تر شروع کرده: مبنا را جابه‌جا کن،
        // این را «پیشرفت» نخوان.
        if count < base {
            h.window_base_failures = Some(count);
            return;
        }
        let delta = count - base;
        if delta < FAILURE_DELTA_TRIGGER {
            return;
        }
        // دروازهٔ هم‌شهادتی. شمارندهٔ خام می‌گوید نشست چقدر **شلوغ** است، نه
        // اینکه سرور فیلتر می‌کند. تنها شاهدِ مقصدهای متمایز می‌تواند این دو را
        // از هم جدا کند، پس شمارنده می‌تواند حکم را تند کند، ولی هرگز تنهایی
        // به آن نمی‌رسد.
        let distinct = h.refused_targets.len();
        if distinct < FAILURE_CORROBORATION {
            h.loud_failure_bursts += 1;
            if h.loud_failure_bursts % BURST_LOG_EVERY == 1 {
                DiagnosticsLog::i(
                    TAG,
                    &format!(
                        "Server {} declined {delta} port forwards in {}s but only {distinct} \
                         distinct destination(s) — that is load, not filtering (a media player \
                         opens dozens of flows at once), so the session is left alone.",
                        h.server_label(),
                        WINDOW_MS / 1000,
                    ),
                );
            }
            return;
        }
        let why = format!("{delta} refused port forwards across {distinct} distinct destinations");
        h.convict(&why);
        h.decide_rotation(now, &why, false)
    };
    dispatch(decision);
}

/// چرخش را **بیرون از قفل** و روی ترد خودش اجرا می‌کند.
///
/// این ترتیب اختیاری نیست: تصمیم از داخل مسیر خواندن notice می‌آید و اجرای
/// چرخش، فرآیند Psiphon را می‌کشد و بالا می‌آورد — یعنی دقیقاً همان تردی را
/// می‌بندد که notice ها از آن می‌آیند. اندروید همین کار را با یک ترد جدا
/// می‌کرد، به همان دلیل.
fn dispatch(decision: Option<(Rotation, Target)>) {
    let Some((strategy, target)) = decision else {
        return;
    };
    // اشاره‌گر را زیر یک قفلِ کوتاه برمی‌داریم و قفل را همان‌جا رها می‌کنیم.
    let Some(action) = cell().lock().action.clone() else {
        return;
    };
    std::thread::Builder::new()
        .name("psiphon-rotate".into())
        .spawn(move || action(strategy, target))
        .ok();
}

// ---------------------------------------------------------------------------
//  تجزیهٔ notice — بدون کریت regex.
//
//  فرمت ConsoleClient یک JSON در هر سطر است:
//    {"data":{"port":1825},"noticeType":"ListeningSocksProxyPort","timestamp":"…"}
//  در اندروید همین‌ها به شکل `ActiveTunnel: {…}` می‌رسیدند. هر دو شکل اینجا
//  خوانده می‌شوند تا اگر روزی خروجی عوض شد، واچ‌داگ کور نشود.
// ---------------------------------------------------------------------------

/// مقدار `noticeType` یک سطر، یا شکل اندرویدیِ `<Type>: {…}`.
pub fn notice_type(line: &str) -> Option<String> {
    if let Some(v) = json_str(line, "noticeType") {
        return Some(v);
    }
    let head = line.split_once(':')?.0.trim();
    if head.is_empty() || head.len() > 48 || !head.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(head.to_string())
}

/// مقدار رشته‌ای یک کلید JSON، هر جای سطر که باشد (تجزیهٔ سطحی و بی‌تخصیص).
///
/// عمداً ساده است: این یک تجزیه‌گر کامل JSON نیست و لازم هم نیست باشد. کلیدهای
/// مورد نیاز همه در سطح اول `data` می‌نشینند و مقدارشان رشته یا عدد است.
pub fn json_str(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut rest = line;
    loop {
        let at = rest.find(&needle)?;
        let after = &rest[at + needle.len()..];
        let after = after.trim_start();
        let Some(after) = after.strip_prefix(':') else {
            rest = &rest[at + needle.len()..];
            continue;
        };
        let after = after.trim_start();
        if let Some(body) = after.strip_prefix('"') {
            // رشته — با احترام به بک‌اسلش، تا یک مقدارِ escape شده سطر را نبُرد.
            let mut out = String::new();
            let mut chars = body.chars();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        if let Some(next) = chars.next() {
                            out.push(next);
                        }
                    }
                    '"' => return Some(out),
                    _ => out.push(c),
                }
            }
            return None;
        }
        // عدد یا bool — تا نخستین جداکننده.
        let end = after
            .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
            .unwrap_or(after.len());
        let value = after[..end].trim();
        if value.is_empty() {
            return None;
        }
        return Some(value.to_string());
    }
}

/// مقدار عددی یک کلید JSON.
pub fn json_u64(line: &str, key: &str) -> Option<u64> {
    json_str(line, key)?.parse().ok()
}

/// `port forward failures for 99Fh7IiB: 115` — هر جای پیام که باشد.
fn parse_port_forward_failures(line: &str) -> Option<(String, u64)> {
    const MARK: &str = "port forward failures for ";
    let at = line.find(MARK)?;
    let rest = &line[at + MARK.len()..];
    let (server, tail) = rest.split_once(':')?;
    let server = server.trim();
    if server.is_empty() {
        return None;
    }
    let digits: String = tail
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    Some((server.to_string(), digits.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards the mobile root cause: the app's own probes can never be read as
    /// evidence that an exit filters, so they can never trigger a rotation.
    #[test]
    fn self_probes_are_never_evidence_against_a_server() {
        for (host, port) in crate::ping::probe_targets() {
            register_self_probe(host, port);
        }
        for (host, port) in crate::ping::probe_targets() {
            assert!(is_self_probe(host, port), "{host}:{port} must be exempt");
        }
        // A destination we never registered stays scoreable.
        assert!(!is_self_probe("example.invalid", 443));
        // And a probe target on a DIFFERENT port is a different destination.
        assert!(!is_self_probe("1.1.1.1", 53));
    }

    #[test]
    fn reads_notice_type_from_console_json() {
        let line =
            r#"{"data":{"port":1825},"noticeType":"ListeningSocksProxyPort","timestamp":"x"}"#;
        assert_eq!(
            notice_type(line).as_deref(),
            Some("ListeningSocksProxyPort")
        );
        assert_eq!(json_u64(line, "port"), Some(1825));
    }

    /// شکل اندرویدی هم باید خوانده شود (تضمین در برابر تغییر خروجی).
    #[test]
    fn reads_android_style_notice() {
        let line = r#"ActiveTunnel: {"diagnosticID":"99Fh7IiB","protocol":"TLS-OSSH"}"#;
        assert_eq!(notice_type(line).as_deref(), Some("ActiveTunnel"));
        assert_eq!(json_str(line, "diagnosticID").as_deref(), Some("99Fh7IiB"));
    }

    #[test]
    fn reads_connected_server_region() {
        let line =
            r#"{"data":{"diagnosticID":"OsBTQokd","region":"DE"},"noticeType":"ConnectedServer"}"#;
        assert_eq!(json_str(line, "region").as_deref(), Some("DE"));
    }

    #[test]
    fn parses_port_forward_failures() {
        let line =
            r#"{"data":{"message":"port forward failures for 99Fh7IiB: 115"},"noticeType":"Info"}"#;
        assert_eq!(
            parse_port_forward_failures(line),
            Some(("99Fh7IiB".to_string(), 115))
        );
    }

    /// شناسهٔ سرور Psiphon base64 است و می‌تواند `+` و `/` داشته باشد.
    #[test]
    fn parses_base64_server_ids() {
        let line = r#"Info: {"message":"port forward failures for vpDb1v+6: 26"}"#;
        assert_eq!(
            parse_port_forward_failures(line),
            Some(("vpDb1v+6".to_string(), 26))
        );
    }

    #[test]
    fn ignores_unrelated_lines() {
        assert_eq!(parse_port_forward_failures("nothing to see"), None);
        assert_eq!(json_str("{}", "diagnosticID"), None);
    }

    /// یک مقدار escape شده نباید تجزیه را از ریل خارج کند.
    #[test]
    fn handles_escaped_strings() {
        let line = r#"{"data":{"message":"he said \"no\" loudly"},"noticeType":"Alert"}"#;
        assert_eq!(notice_type(line).as_deref(), Some("Alert"));
        assert_eq!(
            json_str(line, "message").as_deref(),
            Some(r#"he said "no" loudly"#)
        );
    }

    /// آستانه‌ها همان عددهای اندروید بمانند — این تست عمداً شکننده است.
    #[test]
    fn thresholds_match_the_android_watchdog() {
        assert_eq!(WINDOW_MS, 45_000);
        assert_eq!(DISTINCT_TARGETS_TRIGGER, 6);
        assert_eq!(FAILURE_DELTA_TRIGGER, 60);
        assert_eq!(FAILURE_CORROBORATION, 3);
        assert_eq!(MAX_ROTATIONS, 4);
        assert_eq!(SETTLE_MS, 6_000);
        assert_eq!(HEALTHY_MS, 180_000);
    }
}
