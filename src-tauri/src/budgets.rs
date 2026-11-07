//! بودجه‌های زمانیِ یک تلاشِ اتصال — یک‌جا، چون به هم بند‌ند.
//!
//! # چرا این فایل هست
//!
//! تا ۱.۲.۵ این عددها در دو فایل زندگی می‌کردند: `FIRST_PASS_MAX_MS` در
//! `smart_auto.rs` و `ENGINE_SETUP_RESERVE_MS`/`MIN_SCAN_BUDGET_MS` در
//! `engine.rs`. ولی این‌ها **یک** حساب‌اند: بودجهٔ پله می‌رود به موتور، موتور
//! اولش هویت و گِیت‌وی و تونل را می‌سازد و بقیه‌اش را صرفِ اسکنِ نقطهٔ پایانی
//! می‌کند. جدا‌بودنشان یعنی می‌شود یکی را عوض کرد و رابطه بی‌صدا بشکند — و
//! همین هم شد (پایین‌تر).
//!
//! حالا هر دو سر از همین‌جا می‌خوانند و رابطه‌شان با یک `assert` در زمانِ
//! **کامپایل** و با تست قفل است.

/// هر چیزی که موتور باید پیش از شروعِ اسکن انجام دهد: بارکردن یا ساختنِ هویت،
/// جست‌وجوی اختیاریِ ECHConfigList، بررسیِ گِیت‌ویِ کش‌شده — و بعدش ساختنِ تونل،
/// اعتبارسنجیِ مسیرِ داده و بازکردنِ SOCKS5. از لاگِ میدانی اندازه‌گیری و با
/// دستِ باز رُند شده.
pub const ENGINE_SETUP_RESERVE_MS: u64 = 14_000;

/// هرگز پنجرهٔ اسکنی به موتور نده که حتی برای امتحانِ seedهای مستندِ گِیت‌وی
/// کوتاه است.
pub const MIN_SCAN_BUDGET_MS: u64 = 8_000;

/// بودجهٔ اسکنِ خودِ موتور در حالتِ turbo. عددِ خودِ هسته است، نه انتخابِ ما:
/// اگر پنجره‌ای کوتاه‌تر از این بدهیم، اسکن در سه‌چهارمِ راه کشته می‌شود و هر
/// چه یاد گرفته دور می‌ریزد.
pub const ENGINE_TURBO_SCAN_MS: u64 = 45_000;

/// سقفِ پاسِ اول — عیناً `FIRST_PASS_MAX_MS` در `AetherVpnService.kt`.
///
/// # ۱.۲.۵: این عدد ۳۵ ثانیه بود و با موتور نمی‌خواند
///
/// اندروید ۷۵ ثانیه می‌دهد. دسکتاپ ۳۵ ثانیه می‌داد، و ۳۵ منهای ۱۴ ثانیه
/// آماده‌سازی یعنی موتور برای اسکن **۲۱ ثانیه** داشت — کمتر از نصفِ پنجرهٔ
/// اسکنِ turbo خودش ([`ENGINE_TURBO_SCAN_MS`]). یعنی پاسِ اول روی هر شبکه‌ای
/// که واقعاً به اسکن نیاز داشت، ریاضی‌وار محکوم بود: کشته می‌شد با اسکنی که
/// تقریباً تمام شده بود، نتیجه‌اش دور ریخته می‌شد، و پلهٔ بعد همان اسکن را از
/// صفر شروع می‌کرد. کامنتِ خودِ `engine.rs` همین را از لاگِ میدانی نقل می‌کند:
/// «دو تلاش، ۹۷ ثانیه، `scan deadline reached with no gateway`، هیچ‌چیز
/// یاد‌گرفته‌نشده».
///
/// با ۷۵ ثانیه، پلهٔ اولِ نردبانِ Smart Auto — که روی turbo است و بودجهٔ
/// کاملش ۶۰ ثانیه است — همان ۶۰ ثانیه را می‌گیرد (`min(60, 75)`)، پس موتور
/// ۴۶ ثانیه برای اسکن دارد و پنجرهٔ turboاش جا می‌شود. برای پروتکلِ دستی
/// (Balanced، ۱۵۰ ثانیه) پاسِ اول ۷۵ ثانیه می‌شود، عیناً مثل اندروید.
pub const FIRST_PASS_MAX_MS: u64 = 75_000;

/// این‌که پاسِ اول باید انقدر بزرگ باشد که پنجرهٔ اسکنِ turbo داخلش جا شود،
/// خواسته‌ای سلیقه‌ای نیست: نقضش همان باگی است که بالا توضیح داده شد. پس در
/// زمانِ **کامپایل** بررسی می‌شود، نه در یک تستی که ممکن است اجرا نشود.
const _: () = assert!(FIRST_PASS_MAX_MS >= ENGINE_SETUP_RESERVE_MS + ENGINE_TURBO_SCAN_MS);

/// بودجهٔ دیوارِ ساعتِ یک پله را به پنجرهٔ اسکنی که موتور می‌فهمد ترجمه می‌کند.
pub fn scan_budget_ms(rung_budget_ms: u64) -> u64 {
    rung_budget_ms
        .saturating_sub(ENGINE_SETUP_RESERVE_MS)
        .max(MIN_SCAN_BUDGET_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// عددِ اندروید. اگر کسی این را پایین بیاورد، باید همین‌جا توضیح بدهد چرا.
    #[test]
    fn the_first_pass_window_matches_android() {
        assert_eq!(FIRST_PASS_MAX_MS, 75_000);
    }

    /// همان چیزی که در ۱.۲.۵ شکسته بود: پاسِ اول باید پنجرهٔ اسکنِ turboِ موتور
    /// را کامل در خود جا بدهد، نه سه‌چهارمش را.
    #[test]
    fn the_first_pass_leaves_room_for_a_whole_turbo_scan() {
        let scan = scan_budget_ms(FIRST_PASS_MAX_MS);
        assert!(
            scan >= ENGINE_TURBO_SCAN_MS,
            "the first pass leaves the engine {scan}ms but its turbo scan needs {ENGINE_TURBO_SCAN_MS}ms"
        );
    }

    /// و همان محاسبه برای پلهٔ واقعیِ نردبان: turbo، ۶۰ ثانیه.
    ///
    /// این عدد جایی است که ایرادِ قدیمی دیده می‌شود: با سقفِ ۳۵ ثانیه، این پله
    /// به موتور ۲۱ ثانیه می‌داد. حالا ۴۶ ثانیه می‌دهد.
    #[test]
    fn a_turbo_rung_gives_the_engine_more_than_its_scan_needs() {
        let rung = 60_000u64.min(FIRST_PASS_MAX_MS);
        assert_eq!(rung, 60_000, "a turbo rung must keep its whole 60s budget");
        assert_eq!(scan_budget_ms(rung), 46_000);
        assert!(scan_budget_ms(rung) >= ENGINE_TURBO_SCAN_MS);
    }

    /// نشانِ عددیِ باگِ قدیمی — تا اگر کسی روزی سقف را به ۳۵ ثانیه برگرداند،
    /// این تست بگوید نتیجه‌اش چه بود.
    #[test]
    fn the_old_thirty_five_second_cap_starved_the_scan() {
        assert!(scan_budget_ms(35_000) < ENGINE_TURBO_SCAN_MS);
        assert_eq!(scan_budget_ms(35_000), 21_000);
    }

    /// کفِ اسکن هرگز نباید زیر پا برود، حتی با بودجهٔ مسخره.
    #[test]
    fn the_scan_floor_holds_for_absurd_budgets() {
        assert_eq!(scan_budget_ms(0), MIN_SCAN_BUDGET_MS);
        assert_eq!(scan_budget_ms(1), MIN_SCAN_BUDGET_MS);
        assert_eq!(scan_budget_ms(ENGINE_SETUP_RESERVE_MS), MIN_SCAN_BUDGET_MS);
        // و هیچ‌وقت سرریز نمی‌کند.
        assert_eq!(scan_budget_ms(u64::MAX), u64::MAX - ENGINE_SETUP_RESERVE_MS);
    }

    /// بودجهٔ بزرگ‌تر باید همیشه پنجرهٔ اسکنِ بزرگ‌تر یا مساوی بدهد — یعنی
    /// ترجمه یک‌نواخت است و جایی معکوس نمی‌شود.
    #[test]
    fn a_bigger_rung_never_gets_a_smaller_scan() {
        let mut previous = 0;
        for total in (0..=300_000).step_by(5_000) {
            let scan = scan_budget_ms(total);
            assert!(scan >= previous, "scan window shrank at {total}ms");
            previous = scan;
        }
    }
}
