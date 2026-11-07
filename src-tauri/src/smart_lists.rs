//! ۱.۴.۰ (پورت از `core/SmartLists.kt` موبایل) — فهرست‌های داخلیِ «دور زدنِ
//! سایت‌های ایرانی» و «مسدودسازیِ تبلیغات و ردیاب‌ها».
//!
//! # دو مصرف‌کننده، یک منبع
//!
//!  * **موتور** (نشست‌های تک‌هاپیِ `Aether` و `Tor → Aether`): موتور مقصد را
//!    می‌بیند، پس فهرست‌ها به‌صورت یک فایل مسیرها (`[block]` / `[direct]`) با
//!    `AETHER_ROUTES_FILE` به او داده می‌شوند — [`engine_routes_file`].
//!  * **پلِ محلی** (`Aether → Psiphon`، `Tor → Psiphon`): موتور فقط اتصال‌های
//!    Psiphon را می‌بیند، پس همان قواعد به‌همراه قواعدِ دستیِ کاربر به
//!    [`crate::smart_routes::set_front`] می‌روند — [`front_rules`].
//!
//! # منبعِ فهرست‌ها
//!
//! نسخهٔ seed از `assets/routing/` داخلِ باینری جاسازی می‌شود (`include_str!`)،
//! پس حتی نصبِ تازه و آفلاین هم کار می‌کند. پس از هر اتصالِ موفق، اگر موعد
//! رسیده باشد (هفتگی؛ پس از شکست ۶ ساعت، بارِ اول ۳۰ دقیقه) نسخهٔ کامل
//! **از داخل تونل** دریافت می‌شود و فقط وقتی جایگزین می‌شود که از حداقلِ
//! ورودی‌ها کمتر نباشد (فهرستِ نیمه‌کاره هرگز جای فهرستِ سالم را نمی‌گیرد).
//!
//! # بازنویسی در این مرحله
//!
//! نسخهٔ قبلی به `HttpClient` (که در `mini_https` وجود ندارد) و tokio تکیه داشت
//! و فهرست را از `http://127.0.0.1:1825/...` (نشانی‌ای که هیچ‌کس رویش گوش
//! نمی‌دهد) می‌خواست؛ کامپایل نمی‌شد. این نسخه همگام است و روی یک ترد
//! پس‌زمینه کار می‌کند.
//!
//! # اشتراکِ LAN
//!
//! قواعد مستقیم فقط در پل loopback ویندوز فعال‌اند؛ پل مهمان و موتورِ
//! بالادستِ مشترک هرگز دستور Direct دریافت نمی‌کنند.

use crate::log::DiagnosticsLog;
use crate::mini_https::{self, Dial};
use crate::profile::ConnectionProfile;
use crate::smart_routes::RouteRules;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TAG: &str = "SmartLists";
const DIR: &str = "routing";
pub const IRAN_CIDR: &str = "iran_cidr.txt";
pub const IRAN_DOMAINS: &str = "iran_domains.txt";
pub const ADS: &str = "ads_domains.txt";
const ROUTES_FILE: &str = "smart_routes.txt";
const META_FILE: &str = "lists.json";

pub const IRAN_CIDR_URL: &str =
    "https://raw.githubusercontent.com/aminvakil/ir-ip-list/main/ipv4.txt";
pub const ADS_URL: &str =
    "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/light.txt";

const REFRESH_EVERY_SECS: u64 = 7 * 24 * 60 * 60;
const RETRY_AFTER_FAILURE_SECS: u64 = 6 * 60 * 60;
const FIRST_RETRY_SECS: u64 = 30 * 60;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_DOWNLOAD_BYTES: usize = 8 * 1024 * 1024;
const MIN_IRAN_CIDRS: usize = 800;
const MIN_AD_DOMAINS: usize = 5_000;

const SEED_IRAN_CIDR: &str = include_str!("../../assets/routing/iran_cidr.txt");
const SEED_IRAN_DOMAINS: &str = include_str!("../../assets/routing/iran_domains.txt");
const SEED_ADS: &str = include_str!("../../assets/routing/ads_domains.txt");

static REFRESHING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Cidr,
    Domain,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Meta {
    cidr_ok: u64,
    cidr_try: u64,
    ads_ok: u64,
    ads_try: u64,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn dir(data_dir: &Path) -> PathBuf {
    data_dir.join(DIR)
}

pub fn wants_lists(p: &ConnectionProfile) -> bool {
    p.block_ads || p.bypass_iran
}

/// قواعد مستقیم میزبان با روشن شدن اشتراک LAN خاموش نمی‌شوند.
pub fn smart_direct_active(p: &ConnectionProfile) -> bool {
    p.bypass_iran
}

// ---- پاک‌سازی ---------------------------------------------------------------

fn valid_domain(d: &str) -> bool {
    if d.is_empty() || d.len() > 253 {
        return false;
    }
    d.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
    })
}

fn valid_cidr(c: &str) -> bool {
    let (ip, bits) = match c.split_once('/') {
        Some((ip, bits)) => (ip, Some(bits)),
        None => (c, None),
    };
    let Ok(_) = ip.parse::<std::net::Ipv4Addr>() else {
        return false;
    };
    match bits {
        None => true,
        Some(b) => b.parse::<u8>().map(|n| n <= 32).unwrap_or(false),
    }
}

/// متنِ خام → ورودی‌های پاک. کامنت، `0.0.0.0 name` (قالبِ hosts) و `||name^`
/// (قالبِ adblock) را هم می‌فهمد.
fn sanitize(text: &str, kind: Kind) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('!') {
            continue;
        }
        let mut token = line
            .split_whitespace()
            .last()
            .unwrap_or("")
            .to_ascii_lowercase();
        if kind == Kind::Domain {
            token = token
                .trim_start_matches("||")
                .trim_end_matches('^')
                .trim_start_matches("*.")
                .trim_start_matches('.')
                .trim_end_matches('.')
                .to_string();
            if valid_domain(&token) {
                out.push(token);
            }
        } else if valid_cidr(&token) {
            out.push(token);
        }
    }
    out
}

fn entries(data_dir: &Path, name: &str, kind: Kind) -> Vec<String> {
    let downloaded = dir(data_dir).join(name);
    if let Ok(text) = std::fs::read_to_string(&downloaded) {
        let list = sanitize(&text, kind);
        if !list.is_empty() {
            return list;
        }
    }
    let seed = match name {
        IRAN_CIDR => SEED_IRAN_CIDR,
        IRAN_DOMAINS => SEED_IRAN_DOMAINS,
        _ => SEED_ADS,
    };
    sanitize(seed, kind)
}

/// (block, direct) از فهرست‌های داخلی، برای این پروفایل.
fn built_in(data_dir: &Path, p: &ConnectionProfile) -> (Vec<String>, Vec<String>) {
    let block = if p.block_ads {
        entries(data_dir, ADS, Kind::Domain)
    } else {
        Vec::new()
    };
    let direct = if smart_direct_active(p) {
        let mut d = entries(data_dir, IRAN_DOMAINS, Kind::Domain);
        d.extend(entries(data_dir, IRAN_CIDR, Kind::Cidr));
        d
    } else {
        Vec::new()
    };
    (block, direct)
}

/// فایلِ مسیرها برای موتور، یا `None` اگر چیزی برای نوشتن نیست.
///
/// فقط فهرست‌های داخلی؛ قواعدِ دستیِ کاربر همچنان با `--route-block` /
/// `--route-direct` می‌روند و موتور هر دو را ادغام می‌کند.
pub fn engine_routes_file(data_dir: &Path, p: &ConnectionProfile) -> Option<PathBuf> {
    if !wants_lists(p) {
        return None;
    }
    // The engine is a shared upstream; its routes must contain blocks only.
    // Direct routes are applied at the localhost bridge, never at the engine.
    let (block, _) = built_in(data_dir, p);
    let direct: Vec<String> = Vec::new();
    if block.is_empty() && direct.is_empty() {
        return None;
    }
    let target_dir = dir(data_dir);
    std::fs::create_dir_all(&target_dir).ok()?;
    let target = target_dir.join(ROUTES_FILE);
    let tmp = target_dir.join(format!("{ROUTES_FILE}.tmp"));
    let mut text = String::with_capacity((block.len() + direct.len()) * 16);
    text.push_str("# Generated by Dopamine Desktop from the built-in lists. Rewritten on every connect.\n[block]\n");
    for e in &block {
        text.push_str(e);
        text.push('\n');
    }
    text.push_str("[direct]\n");
    for e in &direct {
        text.push_str(e);
        text.push('\n');
    }
    if std::fs::write(&tmp, text).is_err() || std::fs::rename(&tmp, &target).is_err() {
        DiagnosticsLog::w(
            TAG,
            "Could not write the routes file; the built-in lists are off for this session.",
        );
        return None;
    }
    DiagnosticsLog::i(
        TAG,
        &format!(
            "Routes file for the engine: {} block, {} direct (built-in lists).",
            block.len(),
            direct.len()
        ),
    );
    Some(target)
}

/// قواعدِ پل برای حالت‌های Psiphon: فهرست‌های داخلی + قواعدِ دستیِ کاربر.
/// خالی = مسیرِ قدیمیِ پل، دست‌نخورده.
pub fn front_rules(data_dir: &Path, p: &ConnectionProfile) -> RouteRules {
    let (mut block, mut direct) = built_in(data_dir, p);
    block.extend(
        p.route_block
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    );
    // This is the host policy only. LAN guests use guest_rules() instead.
    direct.extend(
        p.route_direct
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    );
    let rules = RouteRules::parse(
        block.iter().map(String::as_str),
        direct.iter().map(String::as_str),
    );
    if !rules.is_empty() {
        DiagnosticsLog::i(
            TAG,
            &format!(
                "Bridge routing rules: {} block, {} direct.",
                rules.block_count(),
                rules.direct_count()
            ),
        );
    }
    rules
}

/// The guest-facing listener never receives direct rules (built-in or manual).
/// Blocking remains enabled, including on chained Psiphon sessions.
pub fn guest_rules(data_dir: &Path, p: &ConnectionProfile) -> RouteRules {
    let (mut block, _) = built_in(data_dir, p);
    block.extend(
        p.route_block
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    );
    RouteRules::parse(block.iter().map(String::as_str), std::iter::empty::<&str>())
}

// ---- به‌روزرسانیِ هفتگی از داخلِ تونل -------------------------------------

fn load_meta(data_dir: &Path) -> Meta {
    std::fs::read_to_string(dir(data_dir).join(META_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_meta(data_dir: &Path, meta: &Meta) {
    if let Ok(json) = serde_json::to_string(meta) {
        let _ = std::fs::create_dir_all(dir(data_dir));
        let _ = std::fs::write(dir(data_dir).join(META_FILE), json);
    }
}

fn due(last_ok: u64, last_try: u64, now: u64) -> bool {
    if last_ok != 0 && now.saturating_sub(last_ok) < REFRESH_EVERY_SECS {
        return false;
    }
    let backoff = if last_ok == 0 {
        FIRST_RETRY_SECS
    } else {
        RETRY_AFTER_FAILURE_SECS
    };
    last_try == 0 || now.saturating_sub(last_try) >= backoff
}

fn download(
    data_dir: &Path,
    socks_port: u16,
    name: &str,
    url: &str,
    kind: Kind,
) -> Result<usize, String> {
    let r = mini_https::exchange(
        Dial::Socks(socks_port),
        "GET",
        url,
        &[("User-Agent", "Dopamine-Desktop".to_string())],
        None,
        DOWNLOAD_TIMEOUT,
        MAX_DOWNLOAD_BYTES,
    )
    .map_err(|e| e.to_string())?;
    if r.code != 200 {
        return Err(format!("HTTP {}", r.code));
    }
    let list = sanitize(&r.text(), kind);
    let minimum = if kind == Kind::Cidr {
        MIN_IRAN_CIDRS
    } else {
        MIN_AD_DOMAINS
    };
    if list.len() < minimum {
        return Err(format!("only {} entries (need {minimum})", list.len()));
    }
    let d = dir(data_dir);
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let tmp = d.join(format!("{name}.download"));
    std::fs::write(&tmp, list.join("\n")).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, d.join(name)).map_err(|e| e.to_string())?;
    Ok(list.len())
}

/// پس از اتصالِ موفق صدا زده می‌شود؛ هرگز مسدود نمی‌کند.
///
/// `iran_domains.txt` دانلود نمی‌شود (منبعِ عمومیِ قابلِ اعتمادی ندارد؛ همان
/// seed دستیِ موبایل است). دو فهرستِ دیگر فقط اگر کاربر لازمشان دارد.
pub fn refresh_if_due(data_dir: &Path, p: &ConnectionProfile, socks_port: u16) {
    if !wants_lists(p) {
        return;
    }
    if REFRESHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let data_dir = data_dir.to_path_buf();
    let want_cidr = p.bypass_iran;
    let want_ads = p.block_ads;
    let spawned = std::thread::Builder::new()
        .name("aether-smart-lists".into())
        .spawn(move || {
            let mut meta = load_meta(&data_dir);
            let t = now();
            if want_cidr && due(meta.cidr_ok, meta.cidr_try, t) {
                meta.cidr_try = t;
                match download(&data_dir, socks_port, IRAN_CIDR, IRAN_CIDR_URL, Kind::Cidr) {
                    Ok(n) => {
                        meta.cidr_ok = t;
                        DiagnosticsLog::i(
                            TAG,
                            &format!("Iranian address list refreshed: {n} blocks."),
                        );
                    }
                    Err(e) => {
                        DiagnosticsLog::w(TAG, &format!("Iranian address list not refreshed: {e}"))
                    }
                }
                save_meta(&data_dir, &meta);
            }
            if want_ads && due(meta.ads_ok, meta.ads_try, t) {
                meta.ads_try = t;
                match download(&data_dir, socks_port, ADS, ADS_URL, Kind::Domain) {
                    Ok(n) => {
                        meta.ads_ok = t;
                        DiagnosticsLog::i(
                            TAG,
                            &format!("Ad and tracker list refreshed: {n} domains."),
                        );
                    }
                    Err(e) => {
                        DiagnosticsLog::w(TAG, &format!("Ad and tracker list not refreshed: {e}"))
                    }
                }
                save_meta(&data_dir, &meta);
            }
            REFRESHING.store(false, Ordering::SeqCst);
        });
    if spawned.is_err() {
        REFRESHING.store(false, Ordering::SeqCst);
    }
}

/// شمارِ ورودی‌های فعلی، برای نمایش در تنظیمات.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCounts {
    pub iran_cidrs: usize,
    pub iran_domains: usize,
    pub ad_domains: usize,
    pub iran_refreshed_at: u64,
    pub ads_refreshed_at: u64,
}

pub fn counts(data_dir: &Path) -> ListCounts {
    let meta = load_meta(data_dir);
    ListCounts {
        iran_cidrs: entries(data_dir, IRAN_CIDR, Kind::Cidr).len(),
        iran_domains: entries(data_dir, IRAN_DOMAINS, Kind::Domain).len(),
        ad_domains: entries(data_dir, ADS, Kind::Domain).len(),
        iran_refreshed_at: meta.cidr_ok,
        ads_refreshed_at: meta.ads_ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_and_legacy_profiles_do_not_enable_lan() {
        assert!(!ConnectionProfile::default().lan_share);
        let missing: ConnectionProfile = serde_json::from_str("{}").unwrap();
        assert!(!missing.lan_share);
    }

    #[test]
    fn seeds_are_embedded_and_parse() {
        assert!(sanitize(SEED_IRAN_CIDR, Kind::Cidr).len() >= MIN_IRAN_CIDRS);
        assert!(sanitize(SEED_IRAN_DOMAINS, Kind::Domain).contains(&"ir".to_string()));
        assert!(!sanitize(SEED_ADS, Kind::Domain).is_empty());
    }

    #[test]
    fn sanitize_understands_hosts_and_adblock_lines() {
        let text = "# c\n0.0.0.0 Ads.Example.com\n||track.example^\n*.x.example\nbad name!\n";
        assert_eq!(
            sanitize(text, Kind::Domain),
            vec!["ads.example.com", "track.example", "x.example"]
        );
        assert_eq!(
            sanitize("1.2.3.0/24\n1.2.3.4\n1.2.3.0/40\nx\n", Kind::Cidr),
            vec!["1.2.3.0/24", "1.2.3.4"]
        );
    }

    #[test]
    fn shared_engine_never_receives_direct_routes() {
        let p = ConnectionProfile {
            bypass_iran: true,
            block_ads: true,
            lan_share: true,
            route_direct: vec!["bank.ir".to_string()],
            ..ConnectionProfile::default()
        };
        let data = std::env::temp_dir().join(format!(
            "aether-route-test-{}-{}",
            std::process::id(),
            now()
        ));
        let path = engine_routes_file(&data, &p).expect("shared engine block list");
        let text = std::fs::read_to_string(path).unwrap();
        let direct = text.split("[direct]\n").nth(1).unwrap();
        assert!(direct.trim().is_empty(), "engine leaked host direct routes");
        assert!(text.contains("[block]\n"));
        let _ = std::fs::remove_dir_all(data);
    }

    #[test]
    fn refresh_schedule() {
        assert!(due(0, 0, 10));
        assert!(!due(0, 10, 10 + FIRST_RETRY_SECS - 1));
        assert!(due(0, 10, 10 + FIRST_RETRY_SECS));
        assert!(!due(100, 100, 100 + REFRESH_EVERY_SECS - 1));
        assert!(due(100, 100, 100 + REFRESH_EVERY_SECS));
    }

    #[test]
    fn lan_sharing_turns_off_direct_rules() {
        let mut p = ConnectionProfile {
            bypass_iran: true,
            ..ConnectionProfile::default()
        };
        assert!(smart_direct_active(&p));
        p.lan_share = true;
        assert!(smart_direct_active(&p));
        assert!(wants_lists(&p));
        let host = front_rules(Path::new("."), &p);
        let guest = guest_rules(Path::new("."), &p);
        assert!(host.direct_count() > 0);
        assert_eq!(guest.direct_count(), 0);
    }
}
