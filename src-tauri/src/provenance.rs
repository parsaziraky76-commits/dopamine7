//! هویتِ ساخت — برنامه و هستهٔ اصلیِ (دست‌نخوردهٔ) اِتِر.
//!
//! از ۱.۳.۰ (بازنویسیِ هسته از روی Aether-main 2.3.0) هستهٔ `native/aether`
//! دقیقاً همان سورسِ بالادست است و هیچ پچی از سمتِ برنامه در آن نیست؛ پس
//! موتور دیگر مهرِ `AETHER-BUILD-STAMP` چاپ نمی‌کند. هویتِ موتور همان بنرِ
//! خودِ هسته است:
//!
//! ```text
//! [.. INFO  aether] Aether v2.3.0
//! ```
//!
//! و برنامه آن را با فایلِ `CORE_VERSION` کنارِ `aether.exe` می‌سنجد. اگر
//! موتورِ دیگری (کهنه یا از نسخهٔ دیگر) کنار برنامه افتاده باشد، همین‌جا با یک
//! سطرِ `E/build` دیده می‌شود.

use crate::log::DiagnosticsLog;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const TAG: &str = "build";

/// سطحِ پچِ برنامه — از `PATCHLEVEL` ریشهٔ مخزن (`src-tauri/build.rs`).
pub const APP_PATCH_LEVEL: &str = env!("AETHER_APP_PATCHLEVEL");

/// مقداری که یک ساختِ بی‌هویت می‌گیرد — نه یک نسخهٔ ساختگی.
pub const UNSTAMPED: &str = "unstamped";

static REPORTED: AtomicBool = AtomicBool::new(false);
static ENGINE_VERSION: Mutex<Option<String>> = Mutex::new(None);
static EXPECTED_CORE: Mutex<Option<String>> = Mutex::new(None);

/// حکمِ مقایسهٔ نسخهٔ هستهٔ گزارش‌شده با نسخهٔ همراهِ برنامه.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// موتور همان هسته‌ای است که برنامه با آن بسته‌بندی شده.
    Consistent,
    /// برنامه نمی‌داند کدام هسته را انتظار دارد (`CORE_VERSION` نیست).
    Unknown,
    /// دو نسخهٔ متفاوت — موتورِ اشتباه کنار برنامه است.
    Mismatch,
}

/// نسخهٔ هسته را از بنرِ موتور بیرون می‌کشد: `Aether v2.3.0` → `2.3.0`.
///
/// ارزان است: روی هزاران سطرِ دیگر با یک `find` برمی‌گردد.
pub fn banner_version(line: &str) -> Option<&str> {
    const MARK: &str = "Aether v";
    let start = line.find(MARK)? + MARK.len();
    // بنر باید آخرِ سطر باشد؛ «Aether v2 something» بنر نیست.
    let rest = line[start..].trim_end();
    if rest.is_empty()
        || !rest.chars().next().is_some_and(|c| c.is_ascii_digit())
        || !rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return None;
    }
    Some(rest)
}

/// تابعِ خالصِ حکم.
pub fn verdict(expected: &str, engine: &str) -> Verdict {
    let expected = expected.trim().trim_start_matches('v');
    if expected.is_empty() || expected == "unknown" {
        Verdict::Unknown
    } else if expected == engine.trim().trim_start_matches('v') {
        Verdict::Consistent
    } else {
        Verdict::Mismatch
    }
}

/// در هر اتصال، پیش از راه‌افتادنِ موتور صدا زده می‌شود.
pub fn log_app_identity(core_version: &str) {
    REPORTED.store(false, Ordering::Relaxed);
    if let Ok(mut slot) = ENGINE_VERSION.lock() {
        *slot = None;
    }
    if let Ok(mut slot) = EXPECTED_CORE.lock() {
        *slot = Some(core_version.to_string());
    }
    DiagnosticsLog::i(
        TAG,
        &format!(
            "App patch level {} (version {}, bundled Aether core {} — upstream, unmodified). \
             Quote this line in any bug report - it is what identifies the build.",
            APP_PATCH_LEVEL,
            env!("CARGO_PKG_VERSION"),
            core_version,
        ),
    );
    if APP_PATCH_LEVEL == UNSTAMPED {
        DiagnosticsLog::w(
            TAG,
            "This build has no PATCHLEVEL stamp, so it cannot be identified. Build it \
             from the repository root so PATCHLEVEL is picked up.",
        );
    }
}

/// خروجیِ موتور را می‌پاید و بنرِ نسخه را با `CORE_VERSION` می‌سنجد.
pub fn ingest(line: &str) {
    if REPORTED.load(Ordering::Relaxed) || !line.contains("Aether v") {
        return;
    }
    let Some(found) = banner_version(line) else {
        return;
    };
    REPORTED.store(true, Ordering::Relaxed);
    if let Ok(mut slot) = ENGINE_VERSION.lock() {
        *slot = Some(found.to_string());
    }
    let expected = expected_core();
    match verdict(&expected, found) {
        Verdict::Consistent => DiagnosticsLog::i(
            TAG,
            &format!("Engine reports Aether core {found}, the version bundled with this app."),
        ),
        Verdict::Unknown => DiagnosticsLog::w(
            TAG,
            &format!(
                "Engine reports Aether core {found}, but this installation has no CORE_VERSION \
                 file to compare it with."
            ),
        ),
        Verdict::Mismatch => DiagnosticsLog::e(
            TAG,
            &format!(
                "ENGINE/CORE VERSION MISMATCH: this app ships core {expected} but aether.exe \
                 reports {found}. Rebuild the engine (scripts/build-engine.ps1) and reinstall."
            ),
        ),
    }
}

fn expected_core() -> String {
    EXPECTED_CORE
        .lock()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

/// نسخه‌ای که موتور گزارش کرد — برای تشخیص‌ها.
pub fn engine_patch_level() -> Option<String> {
    ENGINE_VERSION.lock().ok().and_then(|s| s.clone())
}

/// نسخهٔ هسته‌ای که برنامه انتظار دارد (از `CORE_VERSION`).
pub fn expected_core_version() -> String {
    expected_core()
}

/// آیا موتور خودش را معرفی کرده و با هستهٔ همراهِ برنامه می‌خواند؟
pub fn consistent() -> bool {
    engine_patch_level()
        .map(|e| verdict(&expected_core(), &e) == Verdict::Consistent)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_LINE: &str = "[2026-10-08T04:30:01.208Z INFO  aether] Aether v2.3.0";

    #[test]
    fn the_version_is_read_out_of_the_engine_banner() {
        assert_eq!(banner_version(REAL_LINE), Some("2.3.0"));
        assert_eq!(banner_version("Aether v2.3.0\r"), Some("2.3.0"));
    }

    #[test]
    fn ordinary_lines_carry_no_version() {
        for line in [
            "[*] performance profile: High (cpus=8)",
            "Aether v",
            "",
            "Aether vNext is coming",
            "[+] Aether v2 tunnel up and running",
        ] {
            assert_eq!(
                banner_version(line),
                None,
                "line `{line}` produced a version"
            );
        }
    }

    #[test]
    fn every_verdict_is_reachable() {
        assert_eq!(verdict("2.3.0", "2.3.0"), Verdict::Consistent);
        assert_eq!(verdict("v2.3.0", "2.3.0"), Verdict::Consistent);
        assert_eq!(verdict("2.3.0", "2.2.0"), Verdict::Mismatch);
        assert_eq!(verdict("unknown", "2.3.0"), Verdict::Unknown);
        assert_eq!(verdict("", "2.3.0"), Verdict::Unknown);
    }
}
