//! معادل ویندوزیِ `data/SecretStore.kt` — انبار رازِ مهرشده.
//!
//! # چرا نه فقط یک فایل JSON کنار پروفایل
//!
//! کلید API کاربر یک اطلاعات محرمانه است و باید مثل یکی با آن رفتار شود. در
//! اندروید با یک کلید AES-GCM سخت‌افزاری از Android Keystore مهر می‌شود. معادلِ
//! *واقعیِ* آن روی ویندوز — و نه یک هم‌سنگ سلیقه‌ای — همان چیزی است که خود
//! ویندوز برای همین کار دارد: **DPAPI** (`CryptProtectData`). کلیدِ رمزنگاری از
//! نشستِ ورودِ همین کاربر مشتق می‌شود، هرگز در دسترس فرآیند ما نیست، و متن
//! رمزشده روی هیچ حساب کاربری یا دستگاه دیگری باز نمی‌شود. یعنی فایل
//! `secrets.bin` که از پروفایل کاربر بیرون کشیده شود، به‌تنهایی بی‌ارزش است.
//!
//! `CRYPTPROTECT_UI_FORBIDDEN` هم صریح پاس داده می‌شود: این کد ممکن است از
//! رشتهٔ پس‌زمینه صدا زده شود و DPAPI هرگز نباید اجازه داشته باشد یک پنجرهٔ
//! گفت‌وگو بالا بیاورد که هیچ‌کس منتظرش نیست.
//!
//! # قاعده‌های دیگری که از موبایل می‌آیند
//!
//! * انبارِ راز از `profile.json` جدا است. پروفایل با «بازنشانی همهٔ تنظیمات»
//!   پاک می‌شود؛ کلید API نه. ازدست‌دادن کلید به‌خاطر اینکه کسی MTU‌اش را ریست
//!   کرده، آزاردهنده است — همان استدلال `GeminiStore` که DataStore جداگانه دارد.
//! * کلید پیش از ذخیره `trim` می‌شود. کلیدی که از یک صفحهٔ وب کپی می‌شود تقریباً
//!   همیشه با یک newline یا فاصلهٔ اضافی می‌آید و کلیدِ دارای فاصله با یک ۴۰۰
//!   شکست می‌خورد که هیچ چیز مفیدی درباره‌ی علتش نمی‌گوید.
//! * نوشتن اتمیک است (فایل موقت + rename)، مثل `ProfileStore`.

use crate::log::DiagnosticsLog;
use anyhow::{anyhow, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// نامِ کلیدِ رازِ Gemini در انبار.
pub const GEMINI_KEY: &str = "geminiApiKey";

pub struct SecretStore {
    path: PathBuf,
}

impl SecretStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("secrets.bin"),
        }
    }

    /// راز را می‌خواند، یا رشتهٔ خالی اگر نبود/باز نشد.
    ///
    /// خواندن هرگز خطا نمی‌دهد — دقیقاً مثل `ProfileStore::load`. فایلِ
    /// باز‌نشدنی (پروفایل ویندوزِ عوض‌شده، دیسکِ کپی‌شده به ماشین دیگر) یعنی
    /// «کلیدی نداریم»، و رابط کاربری همان مسیر «کلید وارد کنید» را نشان می‌دهد
    /// که یک نصبِ تازه می‌بیند.
    pub fn read(&self, name: &str) -> String {
        self.read_all().get(name).cloned().unwrap_or_default()
    }

    /// راز را می‌نویسد. مقدار خالی یعنی فراموش‌کردنِ آن.
    pub fn write(&self, name: &str, value: &str) -> Result<()> {
        let mut all = self.read_all();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            all.remove(name);
        } else {
            all.insert(name.to_string(), trimmed.to_string());
        }
        self.write_all(&all)
    }

    fn read_all(&self) -> BTreeMap<String, String> {
        let sealed = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(_) => return BTreeMap::new(),
        };
        match unprotect(&sealed) {
            Ok(plain) => serde_json::from_slice(&plain).unwrap_or_default(),
            Err(e) => {
                // با *محتوا* لاگ نمی‌شود، فقط با علت. این لاگ صادراتی است.
                DiagnosticsLog::w(
                    "ai",
                    &format!(
                        "The sealed secret store could not be opened ({e}); treating it as empty."
                    ),
                );
                BTreeMap::new()
            }
        }
    }

    fn write_all(&self, all: &BTreeMap<String, String>) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if all.is_empty() {
            let _ = std::fs::remove_file(&self.path);
            return Ok(());
        }
        let plain = serde_json::to_vec(all)?;
        let sealed = protect(&plain)?;
        let tmp = self.path.with_extension("bin.tmp");
        std::fs::write(&tmp, &sealed)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

// >>> AETHER-APP-PATCH custom-configs-sealed (1.3.0)
/// همان مهرِ DPAPI برای انبارِ کانفیگ‌های سفارشی (`custom/store.rs`):
/// اطلاعاتِ ورودِ سرورها هرگز به‌صورتِ متنِ ساده روی دیسک نمی‌نشینند.
pub(crate) fn seal(plain: &[u8]) -> Result<Vec<u8>> {
    protect(plain)
}

/// بازکردنِ مهرِ [`seal`].
pub(crate) fn unseal(sealed: &[u8]) -> Result<Vec<u8>> {
    unprotect(sealed)
}
// <<< AETHER-APP-PATCH custom-configs-sealed

// ---------------------------------------------------------------- ویندوز: DPAPI

#[cfg(windows)]
fn protect(plain: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: plain.len() as u32,
            pbData: plain.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptProtectData(
            &mut input,
            windows::core::w!("Dopamine secrets"),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| anyhow!("CryptProtectData failed: {e}"))?;
        let sealed = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(windows::Win32::Foundation::HLOCAL(output.pbData as *mut _));
        Ok(sealed)
    }
}

#[cfg(windows)]
fn unprotect(sealed: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: sealed.len() as u32,
            pbData: sealed.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptUnprotectData(
            &mut input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| anyhow!("CryptUnprotectData failed: {e}"))?;
        let plain = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(windows::Win32::Foundation::HLOCAL(output.pbData as *mut _));
        Ok(plain)
    }
}

// ------------------------------------------------------- غیرویندوز: فقط تست
//
// بیلد انتشار همیشه ویندوز است؛ این شاخه فقط برای این وجود دارد که
// `cargo test` روی یک ماشین توسعهٔ لینوکسی هم اجرا شود. عمداً *مهر نمی‌کند* و
// همین را در لاگ می‌گوید، تا کسی به اشتباه فکر نکند یک بیلد غیرویندوزی هم
// حفاظت DPAPI را دارد.

#[cfg(not(windows))]
fn protect(plain: &[u8]) -> Result<Vec<u8>> {
    DiagnosticsLog::w(
        "ai",
        "This is not a Windows build: the secret store is NOT sealed.",
    );
    Ok(plain.to_vec())
}

#[cfg(not(windows))]
fn unprotect(sealed: &[u8]) -> Result<Vec<u8>> {
    Ok(sealed.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_trims() {
        let dir = std::env::temp_dir().join(format!("aether-secret-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = SecretStore::new(&dir);

        assert_eq!(store.read(GEMINI_KEY), "");
        // فاصله و newtline‌ای که از کپی‌کردن از یک صفحهٔ وب می‌آید باید برود.
        store.write(GEMINI_KEY, "  AIzaExampleKey \n").unwrap();
        assert_eq!(store.read(GEMINI_KEY), "AIzaExampleKey");

        // مقدار خالی یعنی فراموش کن.
        store.write(GEMINI_KEY, "").unwrap();
        assert_eq!(store.read(GEMINI_KEY), "");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
