//! پورت ۱:۱ از `transport/ExitRegions.kt`.
//!
//! کشورهای خروج برای بک‌اند زنجیره‌ای `Aether → Psiphon`.
//!
//! فهرست همان egress region های منتشرشدهٔ Psiphon است و کدها عیناً با نسخهٔ
//! موبایل یکی‌اند، پس یک پروفایل ذخیره‌شده روی هر دو سکو یک معنی دارد.
//!
//! ⚠ Psiphon این انتخاب را یک **فیلتر سخت** می‌داند: اگر در آن کشور هیچ
//! سروری در دسترس نباشد، اتصال آن انتخاب ممکن نیست. برنامه پس از
//! پایان بودجهٔ اتصال شکست را نشان می‌دهد؛ کاربر می‌تواند «خودکار» را برگزیند.
//!
//! پرچم‌ها اینجا نگه‌داری نمی‌شوند: ویندوز فونت اموجی پرچم ندارد و رابط
//! کاربری از `src/flags.js` (پرچم‌های SVG درون‌خطی) استفاده می‌کند. این ماژول
//! فقط منبع حقیقتِ «کدها و نام‌ها» برای سمت Rust است.

/// `""` = خودکار. اول فهرست می‌ماند تا سطر پیش‌فرض دراپ‌داون باشد.
pub const VALUES: [&str; 56] = [
    "", "AE", "AR", "AT", "AU", "BE", "BG", "BR", "CA", "CH", "CL", "CO", "CY", "CZ", "DE", "DK",
    "EE", "ES", "FI", "FR", "GB", "GR", "HK", "HR", "HU", "IE", "IL", "IN", "IS", "IT", "JP", "KR",
    "LT", "LU", "LV", "MD", "MX", "MY", "NL", "NO", "NZ", "PH", "PL", "PT", "RO", "RS", "SE", "SG",
    "SK", "TH", "TR", "TW", "UA", "US", "VN", "ZA",
];

const NAMES: [(&str, &str); 56] = [
    ("", "Automatic"),
    ("AE", "United Arab Emirates"),
    ("AR", "Argentina"),
    ("AT", "Austria"),
    ("AU", "Australia"),
    ("BE", "Belgium"),
    ("BG", "Bulgaria"),
    ("BR", "Brazil"),
    ("CA", "Canada"),
    ("CH", "Switzerland"),
    ("CL", "Chile"),
    ("CO", "Colombia"),
    ("CY", "Cyprus"),
    ("CZ", "Czechia"),
    ("DE", "Germany"),
    ("DK", "Denmark"),
    ("EE", "Estonia"),
    ("ES", "Spain"),
    ("FI", "Finland"),
    ("FR", "France"),
    ("GB", "United Kingdom"),
    ("GR", "Greece"),
    ("HK", "Hong Kong"),
    ("HR", "Croatia"),
    ("HU", "Hungary"),
    ("IE", "Ireland"),
    ("IL", "Israel"),
    ("IN", "India"),
    ("IS", "Iceland"),
    ("IT", "Italy"),
    ("JP", "Japan"),
    ("KR", "South Korea"),
    ("LT", "Lithuania"),
    ("LU", "Luxembourg"),
    ("LV", "Latvia"),
    ("MD", "Moldova"),
    ("MX", "Mexico"),
    ("MY", "Malaysia"),
    ("NL", "Netherlands"),
    ("NO", "Norway"),
    ("NZ", "New Zealand"),
    ("PH", "Philippines"),
    ("PL", "Poland"),
    ("PT", "Portugal"),
    ("RO", "Romania"),
    ("RS", "Serbia"),
    ("SE", "Sweden"),
    ("SG", "Singapore"),
    ("SK", "Slovakia"),
    ("TH", "Thailand"),
    ("TR", "Turkey"),
    ("TW", "Taiwan"),
    ("UA", "Ukraine"),
    ("US", "United States"),
    ("VN", "Vietnam"),
    ("ZA", "South Africa"),
];

/// کد را به شکل استاندارد درمی‌آورد (trim + بزرگ). ورودی نامعتبر → `""`.
///
/// این تنها دروازهٔ اعتبارسنجی است: هر کدی که در [VALUES] نباشد به «خودکار»
/// تبدیل می‌شود. دلیلش امنیتی است، نه سلیقه‌ای — مقدار مستقیم داخل JSON کانفیگ
/// Psiphon می‌نشیند و نباید هر رشتهٔ دلخواهی از رابط کاربری به آن راه پیدا کند.
pub fn normalize(code: &str) -> String {
    let cc = code.trim().to_ascii_uppercase();
    if VALUES.contains(&cc.as_str()) {
        cc
    } else {
        String::new()
    }
}

/// نام کشور بدون پرچم — برای لاگ، که اموجی در آن فقط نوفه است.
pub fn name(code: &str) -> String {
    let cc = code.trim().to_ascii_uppercase();
    NAMES
        .iter()
        .find(|(k, _)| *k == cc)
        .map(|(_, v)| (*v).to_string())
        .unwrap_or_else(|| {
            if cc.is_empty() {
                "Automatic".to_string()
            } else {
                cc
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// هر کد باید یک نام داشته باشد، وگرنه رابط کاربری کد خام نشان می‌دهد.
    #[test]
    fn every_value_has_a_name() {
        for cc in VALUES {
            assert!(
                NAMES.iter().any(|(k, _)| *k == cc),
                "region {cc} has no display name"
            );
        }
        assert_eq!(VALUES.len(), NAMES.len());
    }

    /// «خودکار» باید سطر اول بماند (پیش‌فرض دراپ‌داون).
    #[test]
    fn automatic_is_first() {
        assert_eq!(VALUES[0], "");
        assert_eq!(name(""), "Automatic");
    }

    /// ورودی زبالهٔ رابط کاربری هرگز نباید به کانفیگ Psiphon برسد.
    #[test]
    fn normalize_rejects_unknown_codes() {
        assert_eq!(normalize("de"), "DE");
        assert_eq!(normalize("  us  "), "US");
        assert_eq!(normalize("XX"), "");
        assert_eq!(normalize("'; DROP"), "");
        assert_eq!(normalize(""), "");
    }
}
