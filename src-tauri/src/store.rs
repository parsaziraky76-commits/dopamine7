//! پورت از `data/ProfileStore.kt`.
//!
//! در اندروید DataStore بود؛ در ویندوز یک فایل JSON در %LOCALAPPDATA%\Aether.
//! رفتار یکسان است: خواندن هرگز خطا نمی‌دهد — فایل خراب یعنی پیش‌فرض‌ها.

use crate::log::DiagnosticsLog;
use crate::profile::{ConnectionProfile, SETTINGS_REV};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct ProfileStore {
    path: PathBuf,
}

impl ProfileStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("profile.json"),
        }
    }

    pub fn load(&self) -> ConnectionProfile {
        let mut profile = match std::fs::read_to_string(&self.path) {
            Ok(raw) => match serde_json::from_str::<ConnectionProfile>(&raw) {
                Ok(p) => p,
                Err(e) => {
                    DiagnosticsLog::w(
                        "store",
                        &format!("Profile unreadable ({e}); using defaults."),
                    );
                    ConnectionProfile::default()
                }
            },
            Err(_) => ConnectionProfile::default(),
        };
        // A missing profile already defaults to Aether. Never reset a saved choice:
        // even profiles from an older settings revision belong to the user.
        let changed = migrate(&mut profile);
        if changed {
            let _ = self.save(&profile);
        }
        profile
    }

    /// نوشتن اتمیک: اول فایل موقت، بعد rename. قطع برق وسط ذخیره
    /// نباید تنظیمات کاربر را نابود کند.
    pub fn save(&self, profile: &ConnectionProfile) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(profile)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// Brings a profile written by an older build up to the current defaults.
///
/// Returns `true` when something changed, so the caller can persist it once.
///
/// ## rev 2 - the MASQUE carrier toggle
///
/// `MASQUE over HTTP/2` is a fallback for networks that block HTTP/3, and it is
/// the one data plane whose flow control is a single HTTP/2 window shared by the
/// whole machine. Two things could leave it stuck on: the user ticking it once
/// while chasing a connection problem, and - until 1.2.3-p2 - Smart Auto's
/// hardened pass, which used to turn it on unconditionally and then wrote the
/// profile back. Either way the setting long outlived the network condition that
/// justified it, and every later session paid for it in download speed.
///
/// Turning it back off is safe: the pre-connect probe now measures UDP for real,
/// so a network that genuinely needs the TCP carrier gets it anyway, on evidence
/// rather than on a stale checkbox.
///
/// The backend is *not* migrated: only a missing profile or missing backend field
/// uses Aether. Previous releases reset the user's saved choice at startup.
fn migrate(profile: &mut ConnectionProfile) -> bool {
    if profile.settings_rev >= SETTINGS_REV {
        return false;
    }

    if profile.masque_http2 {
        profile.masque_http2 = false;
        DiagnosticsLog::i(
            "store",
            "Settings migration: 'MASQUE over HTTP/2' was left on from an earlier session and has been turned off. \
             The HTTP/3 carrier is faster, and the network probe now selects HTTP/2 by itself when UDP is blocked.",
        );
    }

    profile.settings_rev = SETTINGS_REV;
    true
}

// ===========================================================================
//  v12 — انبار ترجیحاتِ کوچک (فقط لایهٔ هوش مصنوعی)
// ===========================================================================

/// یک انبار کلید/مقدارِ متنی برای ترجیحاتی که نه پروفایل اتصال‌اند و نه راز.
///
/// # چرا یک انبار سومِ کوچک
///
/// لایهٔ هوش مصنوعی دو چیز را باید به یاد بسپارد: مدلِ انتخاب‌شده و فهرست
/// مدل‌هایی که کلیدِ کاربر آخرین بار می‌دید. هیچ‌کدام جایی برای زندگی نداشتند و
/// هر سه گزینهٔ موجود غلط بودند:
///
/// * **`profile.json`** — پروفایل *پیکربندی تونل* است و با «بازنشانی همهٔ
///   تنظیمات» پاک می‌شود. انتخاب مدل به تونل بی‌ربط است و نباید با ریست‌کردن
///   MTU از بین برود.
/// * **`secrets.bin`** — با DPAPI مهر می‌شود. شناسهٔ یک مدل راز نیست، و
///   گذاشتنش آنجا هزینهٔ رمزنگاری را به داده‌ای عمومی می‌داد و مرزِ «هر چیزی که
///   در این فایل است محرمانه است» را گل‌آلود می‌کرد.
/// * **`localStorage`** — جایی است که ترجیحات رابط کاربری (زبان) می‌مانند، ولی
///   *سمت جاوااسکریپت* است، و این مقادیر را **Rust** موقع ساخت نشست می‌خواند.
///   خواندنشان از فرانت‌اند یعنی هر فراخوان هوش مصنوعی باید مدل را به‌عنوان
///   پارامتر پاس بدهد و فهرست مجاز روی مرزی اعمال شود که کاربر کنترلش می‌کند.
///
/// همان قواعد `ProfileStore`: خواندن هرگز خطا نمی‌دهد، نوشتن اتمیک است.
pub struct PrefsStore {
    path: PathBuf,
}

impl PrefsStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("prefs.json"),
        }
    }

    fn read_all(&self) -> std::collections::BTreeMap<String, String> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.read_all().get(key).cloned().filter(|v| !v.is_empty())
    }

    pub fn set_string(&self, key: &str, value: &str) -> Result<()> {
        // >>> AETHER-APP-PATCH ui-lang-on-disk (1.3.0)
        // چند نویسنده (هندسهٔ پنجره روی رشتهٔ رویداد، زبان و مدل روی رشتهٔ IPC)
        // همین فایل را خواندن-تغییر-نوشتن می‌کنند؛ بی قفل، یکی تغییر دیگری را
        // بی‌صدا پاک می‌کرد.
        static WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // <<< AETHER-APP-PATCH ui-lang-on-disk
        let mut all = self.read_all();
        if value.is_empty() {
            all.remove(key);
        } else {
            all.insert(key.to_string(), value.to_string());
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&all)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{ConnectionProfile, TransportBackend};

    #[test]
    fn first_install_defaults_to_aether() {
        let dir = std::env::temp_dir().join(format!("aether-fresh-{}", std::process::id()));
        let store = ProfileStore::new(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(store.load().backend, TransportBackend::Aether);
    }

    #[test]
    fn migration_preserves_legacy_backend() {
        let mut old = ConnectionProfile {
            backend: TransportBackend::Tor,
            settings_rev: 2,
            ..ConnectionProfile::default()
        };
        assert!(migrate(&mut old));
        assert_eq!(old.backend, TransportBackend::Tor);
        assert_eq!(old.settings_rev, SETTINGS_REV);
    }

    #[test]
    fn load_preserves_saved_backend_across_launches() {
        let dir = std::env::temp_dir().join(format!("aether-saved-{}", std::process::id()));
        let store = ProfileStore::new(&dir);
        let saved = ConnectionProfile {
            backend: TransportBackend::AetherTor,
            settings_rev: SETTINGS_REV,
            ..ConnectionProfile::default()
        };
        store.save(&saved).unwrap();
        assert_eq!(store.load().backend, TransportBackend::AetherTor);
        assert_eq!(store.load().backend, TransportBackend::AetherTor);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
