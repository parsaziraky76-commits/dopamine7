//! ترابرهای افزونهٔ تور (pluggable transports): آیا یکی نصب است؟
//!
//! # چرا این پرسش یک ماژول جدا دارد
//!
//! تا ۱.۲.۵ پاسخ از «آیا پوشهٔ `pt` وجود دارد» گرفته می‌شد. لاگ میدانیِ
//! ۱۵ سپتامبر نشان داد این پاسخ غلط است: در آن اجرا
//! `AETHER_TOR_PT_DIR=…\engine\pt` فرستاده شده بود — یعنی پوشه بود — ولی هیچ
//! ترابری داخلش نبود. برنامه بر همان مبنا فازِ پل را «ممکن» فرض کرد و صبرِ
//! ۴۲۰ ثانیه‌ای پل را روی توری گذاشت که هیچ obfs4ای برای اجرا نداشت.
//!
//! پس شرط باید وجودِ **فایلِ اجرایی** باشد، و فهرست نام‌ها باید همان فهرستی
//! باشد که موتور می‌شناسد؛ نه بیشتر. اگر برنامه فایلی را ترابر بشمارد که موتور
//! نمی‌شناسد، به کاربر می‌گوید پل ممکن است و بعد چند دقیقه انتظارِ بی‌حاصل
//! تحویلش می‌دهد.
//!
//! ماژول عمداً به `std` بسته است و هیچ API ویندوزی ندارد، تا همین کدی که
//! می‌رود روی میزبان هم آزمون شود.

use std::path::{Path, PathBuf};

/// نام‌های اجراییِ ترابر که هستهٔ ۲.۰.۰ می‌شناسد — `bridges.rs::PT_BINARIES`.
const PT_BINARIES: &[&str] = &[
    "lyrebird",
    "obfs4proxy",
    "snowflake-client",
    "webtunnel-client",
    "conjure-client",
    "meek-client",
];

/// پوشه‌هایی که ترابر می‌تواند در آن باشد، به ترتیبِ اولویت.
///
/// اولی جای ترابرِ همراهِ نصب است (`engine/pt`، همان‌جا که
/// `prepare_runtime_engine` می‌گذارد)، دومی جای ترابری که کاربر خودش کنار
/// دادهٔ برنامه می‌گذارد (مثلاً `snowflake-client.exe` از Tor Browser).
pub fn dirs(working_dir: &Path) -> Vec<PathBuf> {
    vec![
        working_dir.join("engine").join("pt"),
        // >>> AETHER-APP-PATCH tor-native-carrier
        // بستهٔ رسمیِ تور، lyrebird را کنارِ خودِ tor.exe می‌گذارد. بی‌این سطر،
        // نصبی که تورِ رسمی دارد ولی پوشهٔ `pt` ندارد «هیچ ترابری نیست»
        // می‌گرفت و فازِ پل را کوتاه می‌کرد — درحالی‌که ترابر همان‌جا بود.
        working_dir.join("engine").join("tor"),
        // <<< AETHER-APP-PATCH tor-native-carrier
        working_dir.join("pt"),
    ]
}

/// نخستین ترابری که در این پوشه‌ها پیدا می‌شود، اگر باشد.
///
/// خالص و آزمون‌پذیر: تنها ورودی‌اش مسیرهاست.
pub fn find(dirs: &[PathBuf]) -> Option<PathBuf> {
    for dir in dirs {
        for name in PT_BINARIES {
            // ویندوز `.exe` دارد و کسی که ترابر را از یک بستهٔ POSIX کنار هم
            // گذاشته، ندارد. هر دو پرسیده می‌شود، چون هستهٔ ۲.۰.۰ هم هر دو را
            // می‌پرسد (`bridges.rs::locate`).
            for filename in [format!("{name}.exe"), (*name).to_string()] {
                let candidate = dir.join(filename);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// آیا زیر این پوشهٔ کاری ترابری نصب است؟
pub fn installed(working_dir: &Path) -> bool {
    find(&dirs(working_dir)).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "aether-pt-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn empty_folder_is_not_a_transport() {
        let base = temp();
        std::fs::create_dir_all(base.join("engine").join("pt")).unwrap();
        // همان وضعیتی که در لاگ میدانی بود: پوشه هست، ترابر نیست.
        assert!(!installed(&base));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn missing_folder_is_not_a_transport() {
        let base = temp();
        assert!(!installed(&base));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn lyrebird_in_engine_pt_counts() {
        let base = temp();
        let dir = base.join("engine").join("pt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("lyrebird.exe"), b"binary").unwrap();
        assert!(installed(&base));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn user_placed_transport_in_pt_counts() {
        let base = temp();
        let dir = base.join("pt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("snowflake-client"), b"binary").unwrap();
        assert!(installed(&base));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_binary_the_engine_does_not_know_is_not_a_transport() {
        let base = temp();
        let dir = base.join("engine").join("pt");
        std::fs::create_dir_all(&dir).unwrap();
        // نه ترابر است و نه موتور دنبالش می‌گردد.
        std::fs::write(dir.join("tor.exe"), b"binary").unwrap();
        std::fs::write(dir.join("lyrebird.txt"), b"notes").unwrap();
        assert!(!installed(&base));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_directory_named_like_a_transport_does_not_count() {
        let base = temp();
        // `is_file` لازم است: پوشه‌ای به نام lyrebird.exe اجرا نمی‌شود.
        std::fs::create_dir_all(base.join("engine").join("pt").join("lyrebird.exe")).unwrap();
        assert!(!installed(&base));
        std::fs::remove_dir_all(&base).ok();
    }
}
