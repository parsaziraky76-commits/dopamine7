//! ۱.۴.۰ (پورت از `core/SmartRoutes.kt` موبایل) — موتور قواعد مسیریابیِ سمتِ
//! برنامه، برای جایی که موتور اِتِر مقصدِ واقعی را **نمی‌بیند**.
//!
//! # چرا لازم است (#66)
//!
//! در `Aether → Psiphon` و `Tor → Psiphon` موتور فقط اتصال‌های خودِ Psiphon به
//! سرورهایش را حمل می‌کند. پس هیچ قاعدهٔ block/direct ای — نه قواعد کاربر و نه
//! فهرست‌های داخلیِ «دور زدن سایت‌های ایرانی» و «مسدودسازی تبلیغات» — هرگز با
//! چیزی تطبیق نمی‌خورد. در موبایل این قواعد به `PsiphonSocksFront` رفتند؛ در
//! ویندوز همتای آن پلِ [`crate::share`] است: تنها جایی که نام مقصد (CONNECT
//! میزبان، یا ATYP=DOMAIN در SOCKS5) دیده می‌شود.
//!
//! # دستور زبان — عیناً `routing.rs` موتور
//!
//! همان قواعد و همان اولویت (block پیش از direct) تا یک قاعده در هر دو جا
//! یک معنا داشته باشد:
//!
//!  * `example.com` / `domain:` / `suffix:` — خودِ نام و هر زیردامنه؛
//!  * `full:` / `exact:` — فقط همان نام؛
//!  * `keyword:` — هر نامی که این رشته را دارد؛
//!  * `1.2.3.0/24`، `ip:`، `cidr:` — شبکهٔ IPv4/IPv6 یا یک آدرس؛
//!  * `port:443` یا `port:8000-9000`؛
//!  * `private` / `geoip:private` — آدرس‌های خصوصی و `localhost`.
//!
//! تنها تفاوت: `regexp:` اینجا پشتیبانی نمی‌شود (برنامه وابستگیِ regex ندارد)
//! و بی‌صدا کنار گذاشته می‌شود؛ موتور همچنان آن را در نشست‌های تک‌هاپی اجرا
//! می‌کند.
//!
//! # نمایه
//!
//! مثل وصلهٔ `smart-routes-index` موتور: پسوندها و نام‌های کامل در `HashSet`،
//! بازه‌های IPv4 مرتب و ادغام‌شده با جست‌وجوی دودویی. با ده‌ها هزار دامنهٔ
//! تبلیغاتی، تصمیم برای هر اتصال O(برچسب‌ها) است، نه O(قواعد).

use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Proxy,
    Direct,
    Block,
}

#[derive(Debug, Clone)]
enum Matcher {
    Keyword(String),
    V6(u128, u8),
    Ports(u16, u16),
    Private,
}

#[derive(Debug, Default, Clone)]
struct Rules {
    suffixes: HashSet<String>,
    fulls: HashSet<String>,
    v4: Vec<(u32, u32)>,
    others: Vec<Matcher>,
    count: usize,
}

impl Rules {
    fn from_entries<'a>(entries: impl IntoIterator<Item = &'a str>) -> Self {
        let mut rules = Rules::default();
        let mut v4: Vec<(u32, u32)> = Vec::new();
        for raw in entries {
            let entry = raw.trim();
            if entry.is_empty() || entry.starts_with('#') {
                continue;
            }
            if rules.push(entry, &mut v4) {
                rules.count += 1;
            }
        }
        v4.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(v4.len());
        for (start, end) in v4 {
            if let Some(last) = merged.last_mut() {
                if (start as u64) <= (last.1 as u64) + 1 {
                    if end > last.1 {
                        last.1 = end;
                    }
                    continue;
                }
            }
            merged.push((start, end));
        }
        rules.v4 = merged;
        rules
    }

    /// یک ورودی را می‌افزاید؛ `false` یعنی ورودی بدشکل بود و نادیده گرفته شد.
    fn push(&mut self, entry: &str, v4: &mut Vec<(u32, u32)>) -> bool {
        // ریشهٔ خطای تستِ `ipv6_networks`: یک آدرس/شبکهٔ IPv6 خودش «:» دارد.
        // بدون این بررسی، `2001:db8::/32` به kind=`2001` و value=`db8::/32`
        // شکسته می‌شد، به شاخهٔ `_ => false` می‌افتاد و قاعده بی‌صدا دور
        // ریخته می‌شد؛ پس `[2001:db8::1]` به جای Block به Proxy می‌رسید.
        // ورودیِ بی‌پیشوند اگر کامل یک IP/CIDR باشد، پیش از جدا کردنِ
        // `kind:` همان‌جا ثبت می‌شود. هیچ‌کدام از نوع‌های معتبر (domain,
        // full, ip, port, ...) یک IPv6 معتبر نمی‌سازند، پس تداخلی نیست.
        if parse_net(entry).is_some() {
            return self.push_net(entry, v4);
        }
        let (kind, value) = match entry.split_once(':') {
            Some((kind, value)) if !kind.contains('.') && !kind.contains('/') => {
                (kind.trim().to_ascii_lowercase(), value.trim())
            }
            _ => (String::new(), entry),
        };
        match kind.as_str() {
            "domain" | "suffix" => match normalize_domain(value) {
                Some(d) => {
                    self.suffixes.insert(d);
                    true
                }
                None => false,
            },
            "full" | "exact" => match normalize_domain(value) {
                Some(d) => {
                    self.fulls.insert(d);
                    true
                }
                None => false,
            },
            "keyword" => {
                let needle = value.trim().to_ascii_lowercase();
                if needle.is_empty() {
                    return false;
                }
                self.others.push(Matcher::Keyword(needle));
                true
            }
            "ip" | "cidr" => self.push_net(value, v4),
            "port" => match parse_ports(value) {
                Some((lo, hi)) => {
                    self.others.push(Matcher::Ports(lo, hi));
                    true
                }
                None => false,
            },
            "geoip" | "geosite" => {
                if value.eq_ignore_ascii_case("private") {
                    self.others.push(Matcher::Private);
                    true
                } else {
                    false
                }
            }
            // `regexp:` و هر نوعِ ناشناخته‌ای: کنار گذاشته می‌شود.
            "" => {
                if value.eq_ignore_ascii_case("private") {
                    self.others.push(Matcher::Private);
                    return true;
                }
                if self.push_net(value, v4) {
                    return true;
                }
                match normalize_domain(value) {
                    Some(d) => {
                        self.suffixes.insert(d);
                        true
                    }
                    None => false,
                }
            }
            _ => false,
        }
    }

    fn push_net(&mut self, value: &str, v4: &mut Vec<(u32, u32)>) -> bool {
        match parse_net(value) {
            Some(Net::V4(start, end)) => {
                v4.push((start, end));
                true
            }
            Some(Net::V6(base, bits)) => {
                self.others.push(Matcher::V6(base, bits));
                true
            }
            None => false,
        }
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    // >>> AETHER-APP-PATCH smart-routes-by-address (mobile 1.4.0 `Rules.hasDomainRules`)
    /// قاعده‌ای که فقط با «نام» تطبیق می‌خورد (پسوند، نام کامل، کلیدواژه).
    fn has_domain_rules(&self) -> bool {
        !self.suffixes.is_empty()
            || !self.fulls.is_empty()
            || self.others.iter().any(|m| matches!(m, Matcher::Keyword(_)))
    }

    /// قاعده‌ای که فقط با «آدرس» تطبیق می‌خورد (CIDR IPv4/IPv6، `private`).
    fn has_ip_rules(&self) -> bool {
        !self.v4.is_empty()
            || self
                .others
                .iter()
                .any(|m| matches!(m, Matcher::V6(..) | Matcher::Private))
    }

    fn has_v6_rules(&self) -> bool {
        self.others.iter().any(|m| matches!(m, Matcher::V6(..)))
    }
    // <<< AETHER-APP-PATCH smart-routes-by-address

    fn matches(&self, host: &str, ip: Option<IpAddr>, port: u16) -> bool {
        match ip {
            None => {
                if !self.suffixes.is_empty() || !self.fulls.is_empty() {
                    let lowered = host.trim_end_matches('.').to_ascii_lowercase();
                    if self.fulls.contains(&lowered) {
                        return true;
                    }
                    if !self.suffixes.is_empty() {
                        let mut rest = lowered.as_str();
                        loop {
                            if self.suffixes.contains(rest) {
                                return true;
                            }
                            match rest.find('.') {
                                Some(dot) => rest = &rest[dot + 1..],
                                None => break,
                            }
                        }
                    }
                }
            }
            Some(IpAddr::V4(v4)) => {
                let x = u32::from(v4);
                let idx = self.v4.partition_point(|&(start, _)| start <= x);
                if idx > 0 && self.v4[idx - 1].1 >= x {
                    return true;
                }
            }
            Some(IpAddr::V6(_)) => {}
        }
        self.others.iter().any(|m| match m {
            Matcher::Ports(lo, hi) => port >= *lo && port <= *hi,
            Matcher::Private => match ip {
                Some(ip) => is_private(ip),
                None => host.eq_ignore_ascii_case("localhost"),
            },
            Matcher::Keyword(needle) => {
                ip.is_none() && host.to_ascii_lowercase().contains(needle.as_str())
            }
            Matcher::V6(base, bits) => match ip {
                Some(IpAddr::V6(v6)) => v6_contains(*base, *bits, v6),
                _ => false,
            },
        })
    }
}

enum Net {
    V4(u32, u32),
    V6(u128, u8),
}

fn normalize_domain(value: &str) -> Option<String> {
    let cleaned = value
        .trim()
        .trim_start_matches('*')
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if cleaned.is_empty() || cleaned.contains(char::is_whitespace) {
        None
    } else {
        Some(cleaned)
    }
}

fn parse_net(value: &str) -> Option<Net> {
    let value = value.trim();
    let (addr, bits) = match value.split_once('/') {
        Some((a, b)) => (a.trim(), Some(b.trim().parse::<u8>().ok()?)),
        None => (value, None),
    };
    // `[2001:db8::]/32` و `[::1]` هم پذیرفته می‌شوند.
    let addr = addr.trim_start_matches('[').trim_end_matches(']');
    if let Ok(v4) = addr.parse::<Ipv4Addr>() {
        let bits = bits.unwrap_or(32);
        if bits > 32 {
            return None;
        }
        let x = u32::from(v4);
        let mask: u32 = if bits == 0 {
            0
        } else {
            u32::MAX << (32 - bits as u32)
        };
        let start = x & mask;
        let end = start | !mask;
        return Some(Net::V4(start, end));
    }
    if let Ok(v6) = addr.parse::<Ipv6Addr>() {
        let bits = bits.unwrap_or(128);
        if bits > 128 {
            return None;
        }
        // Base address must be masked to match the bits specification, or
        // v6_contains will never match hosts in the CIDR block unless they
        // exactly match the unmasked base. For "2001:db8::/32", mask the base
        // to ensure "2001:db8::1" falls within it.
        let mask: u128 = if bits == 0 {
            0
        } else {
            u128::MAX << (128 - bits as u32)
        };
        let masked_base = u128::from(v6) & mask;
        return Some(Net::V6(masked_base, bits));
    }
    None
}

fn v6_contains(base: u128, bits: u8, ip: Ipv6Addr) -> bool {
    if bits == 0 {
        return true;
    }
    let mask: u128 = u128::MAX << (128 - bits as u32);
    (u128::from(ip) & mask) == (base & mask)
}

fn parse_ports(value: &str) -> Option<(u16, u16)> {
    let value = value.trim();
    match value.split_once('-') {
        Some((lo, hi)) => {
            let lo = lo.trim().parse::<u16>().ok()?;
            let hi = hi.trim().parse::<u16>().ok()?;
            Some(if hi < lo { (hi, lo) } else { (lo, hi) })
        }
        None => {
            let single = value.parse::<u16>().ok()?;
            Some((single, single))
        }
    }
}

/// همان `routing::is_private` موتور.
pub fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// یک مجموعهٔ کامل قواعد: block و direct.
#[derive(Debug, Default, Clone)]
pub struct RouteRules {
    block: Rules,
    direct: Rules,
}

impl RouteRules {
    pub fn parse<'a, B, D>(block: B, direct: D) -> Self
    where
        B: IntoIterator<Item = &'a str>,
        D: IntoIterator<Item = &'a str>,
    {
        Self {
            block: Rules::from_entries(block),
            direct: Rules::from_entries(direct),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.block.is_empty() && self.direct.is_empty()
    }

    pub fn block_count(&self) -> usize {
        self.block.count
    }

    pub fn direct_count(&self) -> usize {
        self.direct.count
    }

    // >>> AETHER-APP-PATCH smart-routes-by-address (mobile 1.4.0 `RouteRules.decide`)
    pub fn has_domain_rules(&self) -> bool {
        self.block.has_domain_rules() || self.direct.has_domain_rules()
    }

    /// آیا قاعده‌ای هست که فقط با آدرسِ IP تطبیق بخورد؟ (مثلاً ۱۳۳۵ بازهٔ ایران)
    pub fn has_ip_rules(&self) -> bool {
        self.block.has_ip_rules() || self.direct.has_ip_rules()
    }

    pub fn has_v6_rules(&self) -> bool {
        self.block.has_v6_rules() || self.direct.has_v6_rules()
    }

    fn decide_one(&self, host: &str, ip: Option<IpAddr>, port: u16) -> Action {
        if self.block.matches(host, ip, port) {
            return Action::Block;
        }
        if self.direct.matches(host, ip, port) {
            return Action::Direct;
        }
        Action::Proxy
    }

    /// عیناً `RouteRules.decide(name, ip, port)` موبایل: اول «نام» (اگر هست)،
    /// و فقط اگر نام چیزی نگفت (Proxy) «آدرس». این همان ترتیبِ
    /// `handle_connect` موتور است (SNI/Host اول، آدرس دوم).
    pub fn decide_name_ip(&self, name: Option<&str>, ip: Option<IpAddr>, port: u16) -> Action {
        if let Some(name) = name {
            let name = name.trim();
            if !name.is_empty() {
                let by_name = self.decide_one(name, None, port);
                if by_name != Action::Proxy || ip.is_none() {
                    return by_name;
                }
            }
        }
        match ip {
            Some(ip) => self.decide_one("", Some(ip), port),
            None => Action::Proxy,
        }
    }

    /// فقط قواعدِ BLOCK برای یک نامِ به‌یادمانده (`recall`)؛ هرگز Direct.
    pub fn blocks_name(&self, name: &str, port: u16) -> bool {
        self.block.matches(name, None, port)
    }
    // <<< AETHER-APP-PATCH smart-routes-by-address

    /// تصمیم برای یک مقصد. block همیشه پیش از direct — همان اولویتِ موتور.
    pub fn decide(&self, host: &str, port: u16) -> Action {
        let bare = host.trim().trim_start_matches('[').trim_end_matches(']');
        let ip = bare.parse::<IpAddr>().ok();
        if self.block.matches(bare, ip, port) {
            return Action::Block;
        }
        if self.direct.matches(bare, ip, port) {
            return Action::Direct;
        }
        Action::Proxy
    }
}

// ---------------------------------------------------------------------------
//  قواعدِ فعالِ پل (front)
// ---------------------------------------------------------------------------

static FRONT: OnceLock<RwLock<Option<Arc<RouteRules>>>> = OnceLock::new();
static GUEST_FRONT: OnceLock<RwLock<Option<Arc<RouteRules>>>> = OnceLock::new();

fn front_slot() -> &'static RwLock<Option<Arc<RouteRules>>> {
    FRONT.get_or_init(|| RwLock::new(None))
}

/// قواعدی که پل باید اعمال کند، یا `None` = مسیر قدیمی، دست‌نخورده.
///
/// فقط در نشست‌های Psiphon پر می‌شود؛ در نشست‌های تک‌موتوری خودِ موتور
/// مقصد را می‌بیند و قواعد آن‌جا اعمال می‌شوند، و اعمالِ دوباره اینجا فقط
/// یک جای دوم برای اشتباه بود.
pub fn set_front(rules: Option<RouteRules>) {
    let rules = rules.filter(|r| !r.is_empty()).map(Arc::new);
    *front_slot().write() = rules;
}

pub fn front() -> Option<Arc<RouteRules>> {
    front_slot().read().clone()
}

pub fn set_guest_front(rules: Option<RouteRules>) {
    let rules = rules.filter(|r| !r.is_empty()).map(Arc::new);
    *GUEST_FRONT.get_or_init(|| RwLock::new(None)).write() = rules;
}

pub fn front_for_listener(loopback: bool) -> Option<Arc<RouteRules>> {
    if loopback {
        front()
    } else {
        GUEST_FRONT.get_or_init(|| RwLock::new(None)).read().clone()
    }
}

// >>> AETHER-APP-PATCH smart-routes-by-address
// ---------------------------------------------------------------------------
//  پل در همهٔ حالت‌ها — چرا (ریشهٔ «سایت‌های ایرانی باز نمی‌شوند»)
// ---------------------------------------------------------------------------
//
// موبایل یک VPN واقعی است: برنامه‌ها خودشان DNS را (از داخل تونل) حل می‌کنند و
// tun2socks مقصد را به‌صورت **آدرس IP** به موتور/پل می‌دهد، به‌همراه SNI. پس
// ۱۳۳۵ بازهٔ IPv4 ایران روی همان آدرس تطبیق می‌خورد.
//
// ویندوز پروکسی سیستمی است: مرورگر `CONNECT digikala.com:443` می‌فرستد و هرگز
// آدرسی در کار نیست. قواعدِ CIDR — بدنهٔ اصلیِ فهرستِ ایران — هیچ‌وقت با هیچ
// اتصالی تطبیق نمی‌خوردند؛ فقط ۳۴ نامِ seed (و `.ir`) مستقیم می‌رفتند و هر
// سایت/CDN ایرانیِ دیگری از خروجیِ خارجی می‌رفت و (به‌خاطر ژئوبلاک) باز نمی‌شد.
//
// درمان، هم‌ارزِ موبایل: پل نام را — **از داخل تونل**، مثل DNS داخل‌تونلِ
// موبایل — به آدرس تبدیل می‌کند (`tunnel_dns`) و همان `decide(name, ip)` را
// اجرا می‌کند. این کار در **همهٔ** حالت‌ها در پل انجام می‌شود، چون در ویندوز
// موتور هم در حالت‌های تک‌هاپی فقط نام را می‌بیند.

static FRONT_UDP_PASSTHROUGH: AtomicBool = AtomicBool::new(false);

/// در حالت‌های غیرِ Psiphon، پل باید UDP ASSOCIATE / BIND را بی‌تغییر به موتور
/// بدهد (موتور UDP دارد). در Psiphon همچنان رد می‌شود (همان رفتارِ موبایل).
pub fn set_front_udp_passthrough(on: bool) {
    FRONT_UDP_PASSTHROUGH.store(on, Ordering::Relaxed);
}

pub fn front_udp_passthrough() -> bool {
    FRONT_UDP_PASSTHROUGH.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
//  حافظهٔ آدرس → نام (عیناً `NameMemory` موبایل؛ فقط برای BLOCK)
// ---------------------------------------------------------------------------

const NAME_MEMORY_MAX: usize = 8192;

static NAMES: OnceLock<Mutex<HashMap<IpAddr, (String, Instant)>>> = OnceLock::new();

fn names() -> &'static Mutex<HashMap<IpAddr, (String, Instant)>> {
    NAMES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// آدرس‌هایی که `name` (از داخل تونل) به آن‌ها حل شد. یک «اشاره» است، نه
/// مرجع: نشانی‌های CDN مشترک‌اند، پس فقط برای BLOCK به کار می‌رود.
pub fn learn_names(name: &str, ips: &[IpAddr], ttl: Duration) {
    if ips.is_empty() {
        return;
    }
    let expires = Instant::now() + ttl;
    let mut map = names().lock();
    if map.len() + ips.len() > NAME_MEMORY_MAX {
        let now = Instant::now();
        map.retain(|_, (_, exp)| *exp > now);
        if map.len() + ips.len() > NAME_MEMORY_MAX {
            map.clear();
        }
    }
    for ip in ips {
        map.insert(*ip, (name.to_string(), expires));
    }
}

pub fn recall_name(ip: IpAddr) -> Option<String> {
    let mut map = names().lock();
    match map.get(&ip) {
        Some((name, exp)) if *exp > Instant::now() => Some(name.clone()),
        Some(_) => {
            map.remove(&ip);
            None
        }
        None => None,
    }
}

pub fn forget_names() {
    names().lock().clear();
}

// ---------------------------------------------------------------------------
//  TLS SNI / HTTP Host از بایت‌های اولِ جریان (عیناً `Sniff` موبایل)
// ---------------------------------------------------------------------------

const SNIFF_METHODS: [&str; 9] = [
    "GET ", "POST ", "HEAD ", "PUT ", "DELETE ", "OPTIONS ", "PATCH ", "CONNECT ", "TRACE ",
];

pub fn sniff_host(head: &[u8]) -> Option<String> {
    let first = *head.first()?;
    let raw = if first == 0x16 {
        sni(head)
    } else {
        http_host(head)
    }?;
    let name = raw.trim().trim_end_matches('.').to_ascii_lowercase();
    let plausible = !name.is_empty()
        && name.len() <= 253
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_');
    if plausible {
        Some(name)
    } else {
        None
    }
}

fn be16(d: &[u8], at: usize) -> Option<usize> {
    Some(((*d.get(at)? as usize) << 8) | (*d.get(at + 1)? as usize))
}

fn sni(d: &[u8]) -> Option<String> {
    // record header (5) + handshake type (1) + length (3)
    if d.len() < 9 || d[5] != 0x01 {
        return None;
    }
    let mut p: usize = 9 + 2 + 32; // client version + random
    p += 1 + *d.get(p)? as usize; // session id
    p += 2 + be16(d, p)?; // cipher suites
    p += 1 + *d.get(p)? as usize; // compression methods
    let end = d.len().min(p + 2 + be16(d, p)?);
    p += 2;
    while p + 4 <= end {
        let kind = be16(d, p)?;
        let len = be16(d, p + 2)?;
        p += 4;
        if kind == 0 {
            if p + 5 > end {
                return None;
            }
            let name_type = d[p + 2];
            let name_len = be16(d, p + 3)?;
            if name_type != 0 || p + 5 + name_len > d.len() {
                return None;
            }
            return std::str::from_utf8(&d[p + 5..p + 5 + name_len])
                .ok()
                .map(str::to_string);
        }
        p += len;
    }
    None
}

fn http_host(d: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(&d[..d.len().min(4096)]);
    if !SNIFF_METHODS.iter().any(|m| text.starts_with(m)) {
        return None;
    }
    for line in text.split("\r\n").skip(1) {
        if line.is_empty() {
            break;
        }
        let is_host = line
            .get(..5)
            .map(|k| k.eq_ignore_ascii_case("host:"))
            .unwrap_or(false);
        if is_host {
            let value = line[5..].trim();
            return Some(if let Some(rest) = value.strip_prefix('[') {
                rest.split(']').next().unwrap_or("").to_string()
            } else {
                value.split(':').next().unwrap_or("").to_string()
            });
        }
    }
    None
}
// <<< AETHER-APP-PATCH smart-routes-by-address

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(block: &[&str], direct: &[&str]) -> RouteRules {
        RouteRules::parse(block.iter().copied(), direct.iter().copied())
    }

    #[test]
    fn suffix_rules_cover_subdomains_and_not_lookalikes() {
        let r = rules(&["ads.example"], &["ir"]);
        assert_eq!(r.decide("ads.example", 443), Action::Block);
        assert_eq!(r.decide("x.ads.example", 443), Action::Block);
        assert_eq!(r.decide("badads.example", 443), Action::Proxy);
        assert_eq!(r.decide("digikala.ir", 443), Action::Direct);
        assert_eq!(r.decide("DIGIKALA.IR.", 443), Action::Direct);
        assert_eq!(r.decide("example.com", 443), Action::Proxy);
    }

    #[test]
    fn block_wins_over_direct() {
        let r = rules(&["tracker.ir"], &["ir"]);
        assert_eq!(r.decide("tracker.ir", 443), Action::Block);
        assert_eq!(r.decide("bank.ir", 443), Action::Direct);
    }

    #[test]
    fn cidr_and_ip_rules() {
        let r = rules(&[], &["2.144.0.0/14", "5.160.0.0/16", "192.168.0.0/16"]);
        assert_eq!(r.decide("2.145.1.1", 443), Action::Direct);
        assert_eq!(r.decide("5.160.255.255", 443), Action::Direct);
        assert_eq!(r.decide("5.161.0.1", 443), Action::Proxy);
        assert_eq!(r.decide("8.8.8.8", 443), Action::Proxy);
    }

    #[test]
    fn full_keyword_port_private() {
        let r = rules(
            &["full:a.example", "keyword:doubleclick", "port:25"],
            &["private"],
        );
        assert_eq!(r.decide("a.example", 443), Action::Block);
        assert_eq!(r.decide("b.a.example", 443), Action::Proxy);
        assert_eq!(r.decide("ad.doubleclick.net", 443), Action::Block);
        assert_eq!(r.decide("mail.example", 25), Action::Block);
        assert_eq!(r.decide("10.0.0.5", 80), Action::Direct);
        assert_eq!(r.decide("localhost", 80), Action::Direct);
    }

    #[test]
    fn ipv6_networks() {
        let r = rules(&["2001:db8::/32"], &[]);
        assert_eq!(r.decide("[2001:db8::1]", 443), Action::Block);
        assert_eq!(r.decide("2001:db9::1", 443), Action::Proxy);
        assert_eq!(r.block_count(), 1);
    }

    #[test]
    fn ipv6_with_prefix_brackets_and_single_address() {
        let r = rules(
            &["ip:2001:db8::/32", "[2a00::]/16", "::1"],
            &["cidr:fd00::/8"],
        );
        assert_eq!(r.block_count(), 3);
        assert_eq!(r.direct_count(), 1);
        assert_eq!(r.decide("2001:db8:ffff::5", 443), Action::Block);
        assert_eq!(r.decide("[2a00:1450::1]", 443), Action::Block);
        assert_eq!(r.decide("[::1]", 443), Action::Block);
        assert_eq!(r.decide("fd12::7", 443), Action::Direct);
        assert_eq!(r.decide("2606:4700::1", 443), Action::Proxy);
        // نوع‌های معمولی نباید با IPv6 اشتباه گرفته شوند.
        let d = rules(&["domain:ads.example", "port:25"], &[]);
        assert_eq!(d.block_count(), 2);
        assert_eq!(d.decide("x.ads.example", 443), Action::Block);
    }

    #[test]
    fn malformed_and_regex_entries_are_ignored() {
        let r = rules(&["regexp:.*", "", "# comment", "port:abc"], &[]);
        assert!(r.is_empty());
        assert_eq!(r.decide("anything.example", 443), Action::Proxy);
    }

    // >>> AETHER-APP-PATCH smart-routes-by-address
    #[test]
    fn a_name_decides_first_and_the_address_second() {
        let r = rules(&["ads.example"], &["ir", "2.144.0.0/14"]);
        let iran: IpAddr = "2.145.1.1".parse().unwrap();
        let abroad: IpAddr = "8.8.8.8".parse().unwrap();
        // نامِ بی‌قاعده + آدرسِ ایرانی => مستقیم (همان چیزی که قبلاً هرگز رخ نمی‌داد)
        assert_eq!(
            r.decide_name_ip(Some("digikala-cdn.com"), Some(iran), 443),
            Action::Direct
        );
        assert_eq!(
            r.decide_name_ip(Some("example.com"), Some(abroad), 443),
            Action::Proxy
        );
        // نام برنده است، حتی روی آدرسِ ایرانی
        assert_eq!(
            r.decide_name_ip(Some("x.ads.example"), Some(iran), 443),
            Action::Block
        );
        assert_eq!(
            r.decide_name_ip(Some("bank.ir"), Some(abroad), 443),
            Action::Direct
        );
        assert_eq!(r.decide_name_ip(None, Some(iran), 443), Action::Direct);
        assert_eq!(r.decide_name_ip(None, None, 443), Action::Proxy);
        assert!(r.has_ip_rules());
        assert!(r.has_domain_rules());
        assert!(!r.has_v6_rules());
        assert!(r.blocks_name("ads.example", 443));
        assert!(!r.blocks_name("bank.ir", 443));
        let only_names = rules(&["ads.example"], &["ir"]);
        assert!(!only_names.has_ip_rules());
    }

    #[test]
    fn sni_and_http_host_are_sniffed() {
        let http = b"GET /x HTTP/1.1\r\nHost: WWW.Example.IR:8080\r\nAccept: */*\r\n\r\n";
        assert_eq!(sniff_host(http).as_deref(), Some("www.example.ir"));
        assert_eq!(sniff_host(b"SSH-2.0-OpenSSH\r\n"), None);
        assert_eq!(sniff_host(b""), None);
        // ClientHello با تنها افزونهٔ server_name = "a.ir"
        let name = b"a.ir";
        let mut sni_ext = vec![0x00, 0x00];
        let list_len = 3 + name.len();
        sni_ext.extend_from_slice(&((list_len + 2) as u16).to_be_bytes());
        sni_ext.extend_from_slice(&(list_len as u16).to_be_bytes());
        sni_ext.push(0x00);
        sni_ext.extend_from_slice(&(name.len() as u16).to_be_bytes());
        sni_ext.extend_from_slice(name);
        let mut body = vec![0x03, 0x03];
        body.extend_from_slice(&[0u8; 32]);
        body.push(0); // session id
        body.extend_from_slice(&[0x00, 0x02, 0x13, 0x01]); // one cipher suite
        body.extend_from_slice(&[0x01, 0x00]); // compression
        body.extend_from_slice(&(sni_ext.len() as u16).to_be_bytes());
        body.extend_from_slice(&sni_ext);
        let mut hello = vec![0x16, 0x03, 0x01];
        hello.extend_from_slice(&((body.len() + 4) as u16).to_be_bytes());
        hello.push(0x01);
        hello.extend_from_slice(&[0x00]);
        hello.extend_from_slice(&(body.len() as u16).to_be_bytes());
        hello.extend_from_slice(&body);
        assert_eq!(sniff_host(&hello).as_deref(), Some("a.ir"));
    }

    #[test]
    fn remembered_names_expire() {
        let ip: IpAddr = "203.0.113.9".parse().unwrap();
        learn_names("ads.example", &[ip], Duration::from_secs(60));
        assert_eq!(recall_name(ip).as_deref(), Some("ads.example"));
        learn_names("ads.example", &[ip], Duration::from_secs(0));
        assert_eq!(recall_name(ip), None);
    }
    // <<< AETHER-APP-PATCH smart-routes-by-address

    #[test]
    fn empty_front_means_the_old_path() {
        set_front(Some(RouteRules::default()));
        assert!(front().is_none());
        set_front(Some(rules(&["ads.example"], &[])));
        assert!(front().is_some());
        set_front(None);
        assert!(front().is_none());
    }
}
