//! پیشرفت bootstrap تور، خوانده‌شده از خروجی استاندارد خودِ موتور.
//!
//! # چرا این ماژول وجود دارد
//!
//! پورت ۱:۱ از `core/TorBootstrap.kt` نسخهٔ موبایل (۱.۳.۰)، برای همان
//! باگی که در لاگ میدانی حالت‌های `Tor`، `Tor → Psiphon` و `Tor → Aether`
//! دیده شد و کاربر آن را «وصل نمی‌شود» گزارش کرد:
//!
//! موتور لیسنر SOCKS5 خود را **همان لحظهٔ شروع** باز می‌کند، پیش از آنکه تور
//! به شبکه رسیده باشد. برنامه آن پورتِ باز را دلیل آماده‌بودن استیج ۱
//! می‌گرفت و بی‌درنگ یک دست‌دادن SOCKS5 با مهلت ۴ ثانیه می‌زد. در لاگ
//! میدانی پورت ۳۰۰ms پس از اجرا باز بود، دست‌دادن در ثانیهٔ ۰٫۴ زده شد و
//! تلاش در ثانیهٔ ۴٫۴ مرده اعلام شد — در حالی که همان لاگ نشان می‌داد تور
//! روی ۳۰٪ است و خوش‌وخرم گواهی‌های دایرکتوری را می‌گیرد. بودجهٔ ۳۰۰
//! ثانیه‌ای که برنامه برای این تلاش حساب کرده بود هرگز خرج نشد، چون هیچ‌چیز
//! منتظرِ خودِ bootstrap نبود.
//!
//! پس برنامه باید بداند تور واقعاً تا کجا رسیده، برای سه دلیل جدا:
//!
//! * تا **تا هر وقت که bootstrap در حرکت است** روی دست‌دادن صبر کند، نه
//!   برای یک پنجرهٔ ثابت چهار ثانیه‌ای؛
//! * تا **زود** تسلیم شود، با پیامی که علت واقعی را نام می‌برد، وقتی درصد از
//!   حرکت ایستاد — همین، و نه یک دست‌دادنِ سریع، شکلِ واقعیِ شبکه‌ای است که
//!   تور را فیلتر می‌کند؛
//! * تا درصد را در رابط کاربری نشان دهد، تا پنج دقیقه «ساخت مدارهای تور…»
//!   شبیه یک برنامهٔ هنگ‌کرده نباشد.
//!
//! درصد از همان سطری خوانده می‌شود که موتور از قبل چاپ می‌کند:
//!
//! ```text
//! tor reaching the network: 30%: connecting to the internet; directory is …
//! ```
//!
//! تلاش مجدد با پل، bootstrap تور را از درصدی پایین از سر می‌گیرد، پس درصدی
//! که **پایین** می‌رود هم پیشرفت است: هر تغییری حرکت به حساب می‌آید.
//!
//! # تفاوت‌ها با نسخهٔ موبایل، و دلیلشان
//!
//! * جای `MutableStateFlow` یک [`Mutex`] از `parking_lot` نشسته: تنها
//!   نویسنده ترد `aether-log` است و خواننده‌ها رشتهٔ فرمان‌های Tauri‌اند.
//! * جای regex یک تجزیه‌کنندهٔ دستی است. کل برنامه هیچ وابستگی regex ندارد و
//!   افزودن یک crate تازه به مسیر اعتماد برای یک الگو، بهایی است که این یک
//!   سطر نمی‌ارزد.
//! * ساعت [`Instant`] است: یکنوا، و مثل نسخهٔ موبایل قابل تست.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use parking_lot::Mutex;

/// وضعیت لحظه‌ایِ bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// آخرین درصدی که موتور گزارش کرده، یا `None` وقتی هنوز چیزی نیامده.
    pub percent: Option<u8>,
    /// حرفِ خودِ موتور دربارهٔ کاری که در آن درصد می‌کند.
    pub detail: String,
    /// درست، به‌محض آنکه bootstrap صد درصد را گزارش کند.
    pub done: bool,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            percent: None,
            detail: String::new(),
            done: false,
        }
    }
}

struct Inner {
    snap: Snapshot,
    /// آخرین لحظه‌ای که درصد **تغییر** کرد.
    advanced_at: Option<Instant>,
    /// لحظهٔ [`reset`] — مبنای سنجش وقتی موتور هنوز هیچ درصدی نگفته.
    watching_since: Option<Instant>,
}

static STATE: Mutex<Inner> = Mutex::new(Inner {
    snap: Snapshot {
        percent: None,
        detail: String::new(),
        done: false,
    },
    advanced_at: None,
    watching_since: None,
});

/// شمارندهٔ نسل، تا `state.rs` بفهمد آیا bootstrapِ در جریان همان چیزی است که
/// خودش شروع کرده. یک تلاشِ لغوشده که دیرتر خطش را چاپ می‌کند نباید نسل بعدی
/// را «در حرکت» نشان دهد.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// وقتی موتوری (دوباره) اجرا می‌شود صدا زده می‌شود: پیشرفتِ اجرای قبلی
/// بی‌معناست.
///
/// نسل تازه را برمی‌گرداند تا فراخواننده بتواند بعداً بپرسد آیا وضعیتی که
/// می‌خواند هنوز مالِ همان تلاش است — [`generation`].
pub fn reset() -> u64 {
    let mut inner = STATE.lock();
    inner.snap = Snapshot::default();
    inner.advanced_at = None;
    inner.watching_since = Some(Instant::now());
    GENERATION.fetch_add(1, Ordering::SeqCst) + 1
}

// >>> AETHER-APP-PATCH tor-reverse-gate (mobile 1.4.0 `AetherVpnService`: torFronted gate)
/// bootstrap را «تمام» ثبت می‌کند، بی آنکه سطری از موتور آمده باشد.
///
/// # چرا لازم است (`loge3`، حالتِ `Tor → Aether`)
///
/// در این حالت تورِ جلوی موتور `tor.exe` است، نه موتور: tor.exe به ۱۰۰٪ رسید،
/// بعد `aether.exe --masque` اجرا شد و [`crate::engine`] هنگامِ اجرای موتور
/// [`reset`] را صدا زد. موتورِ MASQUE هرگز سطرِ «tor reaching the network»
/// چاپ نمی‌کند، پس `done()` دیگر هیچ‌وقت درست نمی‌شد: پورتِ ۱۸۱۹ باز بود،
/// خروجیِ WARP (`masque exit: 104.28.225.221`) هم آمده بود، و دروازهٔ تور باز
/// هم «Reaching the Tor network…» و ۴۵ ثانیه بعد «Tor is not getting through
/// directly — trying bridges…» نشان می‌داد تا کاربر دستی قطع کرد.
///
/// موبایل همین مرحله را با دست‌دادنِ واقعی روی پراکسی می‌سنجد، نه با درصدی که
/// موتور نمی‌گوید. اینجا هم پس از آمادگیِ تور، معیار به خودآزمای مسیرِ داده
/// سپرده می‌شود.
pub fn mark_done(detail: &str) {
    let mut inner = STATE.lock();
    let now = Instant::now();
    inner.snap.percent = Some(100);
    inner.snap.done = true;
    if !detail.is_empty() {
        inner.snap.detail = detail.to_string();
    }
    inner.advanced_at = Some(now);
    if inner.watching_since.is_none() {
        inner.watching_since = Some(now);
    }
}
// <<< AETHER-APP-PATCH tor-reverse-gate

// >>> AETHER-APP-PATCH tor-cold-cache-waves
/// پیشرفتی که درصد را عوض نکرده — دیسکریپتورِ پلِ تازه، بایت‌های تازه، یا
/// نوبت/موجِ تازه — ساعتِ گیرکردن را صفر می‌کند (همان `as_frac` موبایل).
pub fn touch() {
    let mut inner = STATE.lock();
    if inner.snap.done {
        return;
    }
    let now = Instant::now();
    inner.advanced_at = Some(now);
    if inner.watching_since.is_none() {
        inner.watching_since = Some(now);
    }
}

/// آیا این سطرِ `tor_native` خودش پیشرفت است؟
pub fn is_native_headway(line: &str) -> bool {
    line.contains("new bridge descriptor")
        || line.contains("Learned fingerprint")
        || line.starts_with("[*] tor is still loading the directory")
        || line.starts_with("[*] tor bridge wave")
        // AETHER-APP-FIX tor-proves-its-exit: اثباتِ خروجی تا ۴۵ ثانیه طول می‌کشد.
        || line.starts_with("[*] tor is proving its way out")
        || (line.starts_with("[*] ") && line.contains(": go "))
}
// <<< AETHER-APP-PATCH tor-cold-cache-waves

/// نسل فعلی — [`reset`].
pub fn generation() -> u64 {
    GENERATION.load(Ordering::SeqCst)
}

/// یک سطر خروجی موتور را برای پیام پیشرفت می‌کاود.
///
/// روی ترد درنگِ لاگ اجرا می‌شود و برای هر سطری که موتور چاپ می‌کند صدا زده
/// می‌شود، پس عمداً ارزان است: اگر پیشوند در سطر نباشد، پیش از هر تخصیص
/// حافظه‌ای برمی‌گردد.
pub fn ingest(line: &str) {
    let Some((percent, detail)) = parse(line) else {
        return;
    };
    let mut inner = STATE.lock();
    let now = Instant::now();
    // هر تغییری حرکت است — از جمله پایین‌رفتن، که شکلِ یک تلاش تازه از راه
    // پل است.
    if inner.snap.percent != Some(percent) {
        inner.advanced_at = Some(now);
    }
    if inner.watching_since.is_none() {
        inner.watching_since = Some(now);
    }
    if !detail.is_empty() {
        inner.snap.detail = detail;
    }
    inner.snap.percent = Some(percent);
    inner.snap.done = percent >= 100;
}

/// وضعیت فعلی.
pub fn snapshot() -> Snapshot {
    STATE.lock().snap.clone()
}

/// آیا موتور اصلاً هیچ پیشرفتی گزارش کرده؟
pub fn seen() -> bool {
    STATE.lock().snap.percent.is_some()
}

/// آیا bootstrap تمام شده؟
pub fn done() -> bool {
    STATE.lock().snap.done
}

/// چند میلی‌ثانیه از آخرین **تغییر** درصد گذشته — یا از [`reset`] وقتی موتور
/// حتی یک سطر پیشرفت چاپ نکرده، چون موتوری که هیچ نمی‌گوید نسخهٔ بدترِ همان
/// گیرکردن است.
pub fn idle_ms() -> u64 {
    let inner = STATE.lock();
    let since = inner.advanced_at.or(inner.watching_since);
    match since {
        None => 0,
        Some(t) => t.elapsed().as_millis() as u64,
    }
}

/// آیا bootstrap ناتمام است و `after_ms` است که تکان نخورده؟
pub fn stalled(after_ms: u64) -> bool {
    !done() && idle_ms() >= after_ms
}

/// خلاصهٔ یک‌سطریِ انسانی برای لاگ و پیام خطا.
pub fn describe() -> String {
    let snap = snapshot();
    match snap.percent {
        None => "no bootstrap progress reported".to_string(),
        Some(_) if snap.done => "bootstrap complete".to_string(),
        Some(p) if snap.detail.is_empty() => format!("stuck at {p}%"),
        Some(p) => format!("stuck at {p}% ({})", snap.detail),
    }
}

/// معادل دستیِ `Regex("tor reaching the network:\s*(\d{1,3})\s*%\s*:?\s*(.*)")`.
///
/// همان چیزی را می‌پذیرد که آن الگو می‌پذیرفت و همان‌جاها رد می‌کند: درصد
/// بیرون از ۰..=۱۰۰ رد می‌شود (وگرنه یک سطر لاگِ نامربوط با عددی بزرگ،
/// bootstrap را «تمام» نشان می‌داد).
fn parse(line: &str) -> Option<(u8, String)> {
    const MARKER: &str = "tor reaching the network:";
    let rest = line.split_once(MARKER)?.1.trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 3 {
        return None;
    }
    let percent: u16 = digits.parse().ok()?;
    if percent > 100 {
        return None;
    }
    let after = rest[digits.len()..].trim_start();
    let after = after.strip_prefix('%')?.trim_start();
    let detail = after.strip_prefix(':').unwrap_or(after).trim();
    Some((percent as u8, detail.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// تست‌ها یک وضعیت جهانی مشترک دارند، پس هر کدام با `reset` شروع می‌شود
    /// و این قفل جلوی درهم‌رفتنشان را می‌گیرد.
    static SERIAL: Mutex<()> = Mutex::new(());

    // >>> AETHER-APP-PATCH tor-reverse-gate
    #[test]
    fn a_tor_in_front_survives_the_engine_reset() {
        let _guard = SERIAL.lock();
        reset();
        ingest("[*] tor reaching the network: 100%: Done");
        assert!(done());
        // موتورِ پشتِ تور اجرا می‌شود و درصد را صفر می‌کند…
        reset();
        assert!(!done());
        // …و دروازه با علامتِ صریح دوباره «تمام» می‌بیند.
        mark_done("tor.exe in front of the engine");
        assert!(done());
        assert!(!stalled(0));
    }
    // <<< AETHER-APP-PATCH tor-reverse-gate

    // >>> AETHER-APP-PATCH tor-cold-cache-waves
    #[test]
    fn a_descriptor_download_is_not_a_stall() {
        assert!(is_native_headway(
            "[tor] Oct 04 15:53:28.000 [notice] new bridge descriptor 'torfnase' (fresh)"
        ));
        assert!(is_native_headway(
            "[*] tor is still loading the directory at 50% (812 KiB read so far)"
        ));
        assert!(is_native_headway(
            "[*] obfs4: go 2 of 2 — the first go reached 50%"
        ));
        assert!(!is_native_headway(
            "[tor] Oct 04 15:46:23.000 [notice] Ignoring directory request, since no bridge nodes are available yet."
        ));
    }
    // <<< AETHER-APP-PATCH tor-cold-cache-waves

    #[test]
    fn parses_the_engine_line() {
        let _g = SERIAL.lock();
        reset();
        ingest(
            "2026-01-01 12:00:00 [+] tor reaching the network: 30%: connecting to the \
             internet; directory is fetching certificates",
        );
        let snap = snapshot();
        assert_eq!(snap.percent, Some(30));
        assert!(snap.detail.starts_with("connecting to the internet"));
        assert!(!snap.done);
        assert!(seen());
    }

    #[test]
    fn hundred_percent_is_done() {
        let _g = SERIAL.lock();
        reset();
        ingest("tor reaching the network: 100%: done");
        assert!(done());
        assert!(!stalled(0), "یک bootstrap تمام‌شده هرگز گیر نکرده است");
    }

    #[test]
    fn a_percentage_going_down_still_counts_as_movement() {
        let _g = SERIAL.lock();
        reset();
        ingest("tor reaching the network: 45%: fetching");
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(idle_ms() >= 25);
        // تلاش تازه با پل: درصد پایین می‌رود، ولی این پیشرفت است.
        ingest("tor reaching the network: 5%: connecting to bridge");
        assert!(idle_ms() < 25, "پایین‌رفتن درصد باید ساعت را صفر کند");
        assert_eq!(snapshot().percent, Some(5));
    }

    #[test]
    fn the_same_percentage_twice_is_not_movement() {
        let _g = SERIAL.lock();
        reset();
        ingest("tor reaching the network: 10%: handshaking");
        std::thread::sleep(std::time::Duration::from_millis(30));
        ingest("tor reaching the network: 10%: handshaking");
        assert!(idle_ms() >= 25, "تکرارِ همان درصد گیرکردن را پنهان می‌کرد");
        assert!(stalled(20));
        assert!(!stalled(5_000));
    }

    #[test]
    fn silence_is_measured_from_the_reset() {
        let _g = SERIAL.lock();
        reset();
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(!seen());
        // موتوری که هیچ نمی‌گوید نسخهٔ بدترِ همان گیرکردن است، پس گیرکرده
        // حساب می‌شود.
        assert!(stalled(20));
        assert_eq!(describe(), "no bootstrap progress reported");
    }

    #[test]
    fn unrelated_lines_are_ignored() {
        let _g = SERIAL.lock();
        reset();
        for line in [
            "[+] scanning endpoints: 30%",
            "tor reaching the network: 300%: nonsense",
            "tor reaching the network: %",
            "tor reaching the network: abc%",
            "tor is reaching the network: 30%",
        ] {
            ingest(line);
            assert!(!seen(), "این سطر پذیرفته شد: {line}");
        }
    }

    #[test]
    fn detail_survives_a_line_without_one() {
        let _g = SERIAL.lock();
        reset();
        ingest("tor reaching the network: 25%: loading relay descriptors");
        ingest("tor reaching the network: 50%");
        let snap = snapshot();
        assert_eq!(snap.percent, Some(50));
        assert_eq!(
            snap.detail, "loading relay descriptors",
            "توضیحِ قبلی بهتر از خالی است"
        );
        assert!(describe().contains("50%"));
    }

    #[test]
    fn reset_bumps_the_generation() {
        let _g = SERIAL.lock();
        let first = reset();
        ingest("tor reaching the network: 40%: x");
        let second = reset();
        assert!(second > first);
        assert_eq!(generation(), second);
        assert!(!seen(), "پیشرفتِ اجرای قبلی باید پاک شود");
    }
}
