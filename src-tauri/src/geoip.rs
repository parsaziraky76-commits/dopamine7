// >>> AETHER-APP-PATCH the-flag-is-already-on-disk
//! کدِ کشورِ یک نشانی، از فایل‌های `geoip`/`geoip6`ِ خودِ Tor — بی‌هیچ درخواستِ
//! شبکه.
//!
//! # چرا این ماجول هست
//!
//! نشانِ «IP + پرچم» تا امروز از دو راه پر می‌شد، و هر دو در نشستِ توری خراب
//! بودند:
//!
//! ۱. `/cdn-cgi/trace`ِ کلودفلر که برای هر خروجیِ تور `loc=T1` می‌دهد — یک
//!    کدِ شبهٔ‌کشور، نه کشور. ([`crate::probe`] از ۱.۲.۵ آن را رد می‌کند، پس
//!    نتیجه «ناشناخته» می‌شد.)
//! ۲. اصلاحِ `ip-api.com` که برای پر کردنِ همان جای خالی، یک درخواستِ HTTPِ
//!    بی‌رمزِ دیگر — این بار از دلِ تور — می‌فرستد: چند ثانیه وقت روی مداری که
//!    تازه ساخته شده، برای دو حرف.
//!
//! و همان دو حرف از قبل روی دیسکِ کاربر است: هر نصبِ Tor فایلِ `geoip` و
//! `geoip6` را همراه دارد، چون خودِ tor برای `ExitNodes {de}` و
//! `EntryNodes`ِ کشوری به آن نیاز دارد. پس پاسخ محلی است و ما آن را از شبکه
//! می‌پرسیدیم.
//!
//! جست‌وجو دقیقاً همان چیزی است که tor می‌کند: بازه‌های مرتب، و یک جست‌وجوی
//! دودویی. فایل یک بار خوانده و در حافظه نگه داشته می‌شود.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use crate::log::DiagnosticsLog;

const TAG: &str = "geoip";

/// پوشه‌ای که `geoip`/`geoip6` در آن است — همان پوشهٔ پشتیبانِ tor.
fn dir() -> &'static RwLock<Option<PathBuf>> {
    static DIR: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
    DIR.get_or_init(|| RwLock::new(None))
}

/// جدول‌های خوانده‌شده، هر کدام یک بار برای تمام عمرِ فرآیند.
fn table_v4() -> &'static OnceLock<Vec<(u32, u32, [u8; 2])>> {
    static T: OnceLock<Vec<(u32, u32, [u8; 2])>> = OnceLock::new();
    &T
}

fn table_v6() -> &'static OnceLock<Vec<(u128, u128, [u8; 2])>> {
    static T: OnceLock<Vec<(u128, u128, [u8; 2])>> = OnceLock::new();
    &T
}

/// می‌گوید فایل‌های کشور کجایند. یک بار در راه‌اندازیِ برنامه صدا زده می‌شود.
///
/// اگر صدا زده نشود — یا نصب فایل را نداشته باشد — [`lookup`] فقط `None`
/// می‌دهد و رفتارِ قبلی (اصلاحِ شبکه‌ای) سرِ جایش می‌ماند. یعنی این ماجول
/// هیچ‌جا مسیرِ کار را نمی‌بندد.
pub fn use_dir(path: &Path) {
    if let Ok(mut slot) = dir().write() {
        *slot = Some(path.to_path_buf());
    }
}

/// کدِ کشورِ این نشانی، یا `None` اگر فایل نبود/بازه پیدا نشد.
pub fn lookup(ip: IpAddr) -> Option<String> {
    let base = dir().read().ok()?.clone()?;
    match ip {
        IpAddr::V4(v4) => {
            let table = table_v4().get_or_init(|| load_v4(&base.join("geoip")));
            find(table, u32::from(v4))
        }
        IpAddr::V6(v6) => {
            let table = table_v6().get_or_init(|| load_v6(&base.join("geoip6")));
            find(table, u128::from(v6))
        }
    }
}

/// جست‌وجوی دودویی در بازه‌های مرتب و ناهم‌پوشان.
fn find<T: Ord + Copy>(table: &[(T, T, [u8; 2])], needle: T) -> Option<String> {
    let hit = table
        .binary_search_by(|(low, high, _)| {
            if needle < *low {
                std::cmp::Ordering::Greater
            } else if needle > *high {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .ok()?;
    let code = table[hit].2;
    Some(String::from_utf8_lossy(&code).to_ascii_uppercase())
}

fn load_v4(path: &Path) -> Vec<(u32, u32, [u8; 2])> {
    load(path, |token| {
        // فایلِ tor مرزها را **عددِ دهدهی** می‌نویسد (`16777216,16777471,AU`).
        // شکلِ نقطه‌دار هم پذیرفته می‌شود، چون نسخه‌های ساخته‌شده با ابزارهای
        // دیگر آن را می‌نویسند و ردکردنشان یعنی جدولی خالی و پرچمی خاموش.
        token
            .parse::<u32>()
            .ok()
            .or_else(|| token.parse::<Ipv4Addr>().ok().map(u32::from))
    })
}

fn load_v6(path: &Path) -> Vec<(u128, u128, [u8; 2])> {
    load(path, |token| {
        token
            .parse::<u128>()
            .ok()
            .or_else(|| token.parse::<Ipv6Addr>().ok().map(u128::from))
    })
}

/// خواندنِ یک فایلِ کشور. خطِ ناخوانا نادیده گرفته می‌شود، نه اینکه فایل را
/// بیندازد: این داده از بیرون می‌آید و یک خطِ خراب نباید پرچم را خاموش کند.
fn load<T: Ord + Copy>(path: &Path, parse: impl Fn(&str) -> Option<T>) -> Vec<(T, T, [u8; 2])> {
    let body = match std::fs::read_to_string(path) {
        Ok(body) => body,
        Err(error) => {
            DiagnosticsLog::d(TAG, &format!("{} not readable: {error}", path.display()));
            return Vec::new();
        }
    };
    let mut rows: Vec<(T, T, [u8; 2])> = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split(',');
        let (Some(low), Some(high), Some(code)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let code = code.trim().as_bytes();
        // `??` و هر چیزِ غیرِ دوحرفیِ الفبایی: همان «ناشناخته»، و ناشناخته را
        // نباید به‌جای کشور نشان داد.
        if code.len() != 2 || !code.iter().all(|b| b.is_ascii_alphabetic()) {
            continue;
        }
        let (Some(low), Some(high)) = (parse(low.trim()), parse(high.trim())) else {
            continue;
        };
        if low > high {
            continue;
        }
        rows.push((low, high, [code[0], code[1]]));
    }
    // فایلِ tor مرتب است، ولی جست‌وجوی دودویی روی دادهٔ نامرتب خطای خاموش
    // می‌دهد — پس مرتب‌سازی، نه فرضِ مرتب‌بودن.
    rows.sort_unstable_by_key(|(low, _, _)| *low);
    if rows.is_empty() {
        DiagnosticsLog::d(TAG, &format!("{} had no usable ranges", path.display()));
    } else {
        DiagnosticsLog::d(
            TAG,
            &format!("{}: {} ranges loaded", path.display(), rows.len()),
        );
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_decimal_range_resolves() {
        let file = std::env::temp_dir().join("aether-geoip-test-v4");
        std::fs::write(
            &file,
            "# comment\n16777216,16777471,AU\n3232235520,3232301055,DE\n",
        )
        .unwrap();
        let table = load_v4(&file);
        assert_eq!(table.len(), 2);
        assert_eq!(
            find(
                &table,
                u32::from("192.168.1.1".parse::<Ipv4Addr>().unwrap())
            )
            .as_deref(),
            Some("DE")
        );
        assert_eq!(
            find(&table, u32::from("1.0.0.5".parse::<Ipv4Addr>().unwrap())).as_deref(),
            Some("AU")
        );
        // بیرونِ هر بازه: پرچمی نیست، و حدس هم زده نمی‌شود.
        assert_eq!(
            find(&table, u32::from("8.8.8.8".parse::<Ipv4Addr>().unwrap())),
            None
        );
    }

    #[test]
    fn dotted_and_v6_forms_are_accepted() {
        let v4 = std::env::temp_dir().join("aether-geoip-test-dotted");
        std::fs::write(&v4, "1.0.0.0,1.0.0.255,NL\n").unwrap();
        let table = load_v4(&v4);
        assert_eq!(
            find(&table, u32::from("1.0.0.7".parse::<Ipv4Addr>().unwrap())).as_deref(),
            Some("NL")
        );

        let v6 = std::env::temp_dir().join("aether-geoip-test-v6");
        std::fs::write(
            &v6,
            "2001:200::,2001:200:ffff:ffff:ffff:ffff:ffff:ffff,JP\n",
        )
        .unwrap();
        let table = load_v6(&v6);
        assert_eq!(
            find(
                &table,
                u128::from("2001:200::42".parse::<Ipv6Addr>().unwrap())
            )
            .as_deref(),
            Some("JP")
        );
    }

    #[test]
    fn unknown_and_broken_rows_are_dropped() {
        let file = std::env::temp_dir().join("aether-geoip-test-junk");
        std::fs::write(
            &file,
            "16777216,16777471,??\nnonsense\n1,2\n3232235520,3232301055,DE\n5,4,FR\n",
        )
        .unwrap();
        let table = load_v4(&file);
        // تنها سطرِ سالم می‌ماند: `??` کشور نیست، بازهٔ برعکس بازه نیست.
        assert_eq!(table.len(), 1);
        assert_eq!(table[0].2, [b'D', b'E']);
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        let table = load_v4(Path::new("/nonexistent/geoip"));
        assert!(table.is_empty());
        assert_eq!(find(&table, 42u32), None);
    }
}
// <<< AETHER-APP-PATCH the-flag-is-already-on-disk
