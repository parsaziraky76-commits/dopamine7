//! ۱.۵.۰ — پورت از `core/ShareLeakGuard.kt` و `core/LanGuard.kt` موبایل:
//! قاعده‌های «صفرِ نشت» و احرازِ اختیاریِ اشتراکِ LAN.
//!
//! همه تابع‌های خالص‌اند (بی‌سوکت، بی‌سیستم‌عامل) تا جدا تست شوند. فقط
//! بخش‌هایی آمده‌اند که ویندوز لازم دارد: ردهٔ رابط‌های هات‌اسپات/USB/بلوتوثِ
//! اندروید این‌جا معنایی ندارد، چون ویندوز فقط حالتِ «شبکهٔ خانگی» را دارد.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// نامِ جادوییِ بررسی: از هر مسیرِ اشتراک (پروکسی HTTP، CONNECT، SOCKS5) پاسخش
/// را خودِ پل می‌دهد، پس باز شدنش یعنی «واقعاً از اشتراک رد می‌شوی».
pub const CHECK_HOST: &str = crate::share_pages::CHECK_HOST;
/// آدرسی که `dopamine.check` در DNSِ UDPِ اشتراک به آن حل می‌شود (TEST-NET-2).
pub const CHECK_SENTINEL_V4: Ipv4Addr = Ipv4Addr::new(198, 51, 100, 254);

pub const DEFAULT_USER: &str = "aether";
pub const MIN_PASSWORD: usize = 8;
pub const MAX_PASSWORD: usize = 128;
pub const MAX_USER: usize = 64;

fn unwrap_mapped(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        v4 => v4,
    }
}

/// مقصدی که یک مهمانِ LAN حق دارد از راهِ تونل به آن برسد: فقط تک‌پخشیِ
/// عمومی. هرگز loopback، شبکهٔ خصوصی، link-local، CGNAT، multicast یا رزرو —
/// وگرنه یک مهمان می‌توانست از راهِ پل به خودِ این کامپیوتر یا روترِ خانه برسد.
pub fn is_public_unicast(ip: IpAddr) -> bool {
    match unwrap_mapped(ip) {
        IpAddr::V4(a) => {
            let o = a.octets();
            !(a.is_unspecified()
                || a.is_loopback()
                || a.is_link_local()
                || a.is_private()
                || a.is_multicast()
                || o[0] == 0
                || (o[0] == 100 && (64..=127).contains(&o[1]))
                || (o[0] == 192 && o[1] == 0 && o[2] == 0)
                || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
                || o[0] >= 240)
        }
        IpAddr::V6(a) => {
            let s = a.segments();
            let first = (s[0] >> 8) as u8;
            !(a.is_unspecified()
                || a.is_loopback()
                || a.is_multicast()
                || (s[0] & 0xffc0) == 0xfe80 // link-local
                || (s[0] & 0xffc0) == 0xfec0 // site-local (قدیمی)
                || (first & 0xfe) == 0xfc // ULA
                || (s[0] == 0x2001 && s[1] == 0x0db8) // مستندات
                || first == 0x00)
        }
    }
}

/// فقط لیترالِ IP (بی DNS). `[v6]` هم پذیرفته می‌شود.
pub fn parse_ip_literal(host: &str) -> Option<IpAddr> {
    let h = host.trim();
    let h = h.strip_prefix('[').unwrap_or(h);
    let h = h.strip_suffix(']').unwrap_or(h);
    let parts: Vec<&str> = h.split('.').collect();
    if parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 3 && p.bytes().all(|b| b.is_ascii_digit()))
    {
        let nums: Vec<u16> = parts
            .iter()
            .map(|p| p.parse::<u16>().unwrap_or(999))
            .collect();
        if nums.iter().any(|n| *n > 255) {
            return None;
        }
        return Some(IpAddr::V4(Ipv4Addr::new(
            nums[0] as u8,
            nums[1] as u8,
            nums[2] as u8,
            nums[3] as u8,
        )));
    }
    if h.contains(':')
        && h.bytes()
            .all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.')
    {
        return h.parse::<Ipv6Addr>().ok().map(IpAddr::V6);
    }
    None
}

fn numeric_like(h: &str) -> bool {
    // `^(0x[0-9a-f]+|[0-9]+)(\.(0x[0-9a-f]+|[0-9]+))*$` — شکل‌های عددیِ عجیبِ IP
    // (0x7f.1، 2130706433) که بعضی resolverها به loopback ترجمه می‌کنند.
    h.split('.').all(|part| {
        if let Some(hex) = part.strip_prefix("0x") {
            !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit())
        } else {
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())
        }
    })
}

const LOCAL_SUFFIXES: [&str; 5] = [".local", ".lan", ".home.arpa", ".internal", ".intranet"];

/// آیا یک مهمانِ LAN می‌تواند این نام/آدرس را از راهِ تونل بخواهد؟
pub fn destination_allowed(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if h.is_empty() || h.len() > 253 {
        return false;
    }
    if let Some(ip) = parse_ip_literal(&h) {
        return is_public_unicast(ip);
    }
    if h.starts_with('[') || h.contains(':') {
        return false; // IPv6ِ بدشکل
    }
    if numeric_like(&h) {
        return false;
    }
    if h == "localhost"
        || h.ends_with(".localhost")
        || h == "localhost.localdomain"
        || h == "ip6-localhost"
        || h == "ip6-loopback"
    {
        return false;
    }
    if LOCAL_SUFFIXES.iter().any(|s| h == s[1..] || h.ends_with(s)) {
        return false;
    }
    true
}

/// دیتاگرامِ UDPِ SOCKS5 (RFC 1928 §7) از یک مهمان: فقط بی‌قطعه، و فقط به مقصدِ مجاز.
pub fn udp_datagram_allowed(data: &[u8]) -> bool {
    if data.len() < 4 + 1 + 2 {
        return false;
    }
    if data[0] != 0 || data[1] != 0 || data[2] != 0 {
        return false;
    }
    match data[3] {
        0x01 => {
            data.len() >= 4 + 4 + 2
                && is_public_unicast(IpAddr::V4(Ipv4Addr::new(
                    data[4], data[5], data[6], data[7],
                )))
        }
        0x04 => {
            if data.len() < 4 + 16 + 2 {
                return false;
            }
            let mut a = [0u8; 16];
            a.copy_from_slice(&data[4..20]);
            is_public_unicast(IpAddr::V6(Ipv6Addr::from(a)))
        }
        0x03 => {
            let len = data[4] as usize;
            len > 0
                && data.len() >= 4 + 1 + len + 2
                && destination_allowed(&String::from_utf8_lossy(&data[5..5 + len]))
        }
        _ => false,
    }
}

const IDENTITY_HEADERS: [&str; 15] = [
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
    "x-forwarded-server",
    "x-real-ip",
    "x-client-ip",
    "client-ip",
    "true-client-ip",
    "cf-connecting-ip",
    "x-originating-ip",
    "x-remote-ip",
    "x-remote-addr",
    "x-cluster-client-ip",
    "forwarded",
    "via",
];

/// سرآیندی که می‌تواند آدرسِ واقعیِ مهمان را به سایت بگوید.
pub fn is_identity_header(line: &str) -> bool {
    let name = line
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    IDENTITY_HEADERS.contains(&name.as_str())
}

/// آیا `a` و `b` در یک زیرشبکهٔ `prefix` بیتی‌اند؟
pub fn same_subnet(a: IpAddr, b: IpAddr, prefix: u8) -> bool {
    let bytes = |ip: IpAddr| -> Vec<u8> {
        match unwrap_mapped(ip) {
            IpAddr::V4(v) => v.octets().to_vec(),
            IpAddr::V6(v) => v.octets().to_vec(),
        }
    };
    let (x, y) = (bytes(a), bytes(b));
    if x.len() != y.len() {
        return false;
    }
    let bits = (prefix as usize).min(x.len() * 8);
    let full = bits / 8;
    if x[..full] != y[..full] {
        return false;
    }
    let rest = bits % 8;
    if rest == 0 {
        return true;
    }
    let mask = (0xFFu16 << (8 - rest)) as u8;
    (x[full] & mask) == (y[full] & mask)
}

/// پیشوندِ نامعقول (۰، ۳۱، ۳۲) یعنی ۲۴ — یک /32 هیچ همسایه‌ای را نمی‌پذیرفت.
pub fn sane_prefix(prefix: u8) -> u8 {
    if (8..=30).contains(&prefix) {
        prefix
    } else {
        24
    }
}

/// اسکریپتِ PAC: نام‌های محلی و شبکه‌های خصوصی مستقیم، بقیه از پروکسیِ HTTP.
pub fn pac_script(proxy_host: &str, http_port: u16) -> String {
    format!(
        "function FindProxyForURL(url, host) {{\n\
         \x20 host = host.toLowerCase();\n\
         \x20 if (isPlainHostName(host) || host == \"localhost\" ||\n\
         \x20     shExpMatch(host, \"*.local\") ||\n\
         \x20     shExpMatch(host, \"10.*\") || shExpMatch(host, \"127.*\") ||\n\
         \x20     shExpMatch(host, \"192.168.*\") || shExpMatch(host, \"169.254.*\") ||\n\
         \x20     /^172\\.(1[6-9]|2[0-9]|3[01])\\./.test(host)) {{\n\
         \x20   return \"DIRECT\";\n\
         \x20 }}\n\
         \x20 return \"PROXY {proxy_host}:{http_port}\";\n\
         }}\n"
    )
}

pub fn is_check_host(host: &str) -> bool {
    host.trim()
        .trim_end_matches('.')
        .eq_ignore_ascii_case(CHECK_HOST)
}

/// `dopamine.check` یا آدرسِ نگهبانش.
pub fn is_check_target(host: &str) -> bool {
    is_check_host(host) || parse_ip_literal(host) == Some(IpAddr::V4(CHECK_SENTINEL_V4))
}

/// طولِ سرآیندِ UDPِ SOCKS5، یا `None` اگر بدشکل است.
pub fn socks_udp_header_length(data: &[u8]) -> Option<usize> {
    if data.len() < 4 {
        return None;
    }
    let header = match data[3] {
        0x01 => 4 + 4 + 2,
        0x04 => 4 + 16 + 2,
        0x03 => {
            if data.len() < 5 {
                return None;
            }
            4 + 1 + data[4] as usize + 2
        }
        _ => return None,
    };
    (header <= data.len()).then_some(header)
}

/// یک پرس‌وجوی DNS (پورت ۵۳) برای `dopamine.check` در یک دیتاگرامِ UDPِ SOCKS5 —
/// پاسخش همین‌جا ساخته می‌شود (A → نگهبان)، با همان سرآیندِ SOCKS.
pub fn check_host_dns_reply(data: &[u8]) -> Option<Vec<u8>> {
    let header = socks_udp_header_length(data)?;
    if data[2] != 0 {
        return None;
    }
    let port = u16::from_be_bytes([data[header - 2], data[header - 1]]);
    if port != 53 {
        return None;
    }
    let answer = check_host_dns_answer(&data[header..])?;
    let mut out = data[..header].to_vec();
    out.extend_from_slice(&answer);
    Some(out)
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_be_bytes([b[i], b[i + 1]])
}

/// پاسخِ DNS برای پرس‌وجوی `dopamine.check`؛ `None` برای هر نامِ دیگر.
pub fn check_host_dns_answer(q: &[u8]) -> Option<Vec<u8>> {
    if q.len() < 12 + 1 + 4 {
        return None;
    }
    let flags = u16_at(q, 2);
    if flags & 0x8000 != 0 || (flags >> 11) & 0x0F != 0 || u16_at(q, 4) != 1 {
        return None;
    }
    let mut p = 12;
    let mut name = String::new();
    loop {
        let label = *q.get(p)? as usize;
        if label == 0 {
            p += 1;
            break;
        }
        if label & 0xC0 != 0 || p + 1 + label > q.len() {
            return None;
        }
        if !name.is_empty() {
            name.push('.');
        }
        name.push_str(&String::from_utf8_lossy(&q[p + 1..p + 1 + label]));
        if name.len() > 253 {
            return None;
        }
        p += 1 + label;
    }
    if p + 4 > q.len() || !is_check_host(&name) {
        return None;
    }
    let (qtype, qclass) = (u16_at(q, p), u16_at(q, p + 2));
    let question_end = p + 4;
    let with_address = qtype == 1 && qclass == 1;
    let mut out = Vec::with_capacity(question_end + 16);
    out.extend_from_slice(&q[0..2]);
    out.extend_from_slice(&(0x8000 | 0x0400 | (flags & 0x0100) | 0x0080).to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&(if with_address { 1u16 } else { 0 }).to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&q[12..question_end]);
    if with_address {
        out.extend_from_slice(&0xC00Cu16.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&[0, 0, 0, 60]);
        out.extend_from_slice(&4u16.to_be_bytes());
        out.extend_from_slice(&CHECK_SENTINEL_V4.octets());
    }
    Some(out)
}

/// دیتاگرامِ STUN (RFC 5389، magic cookie) — فقط برای یک خطِ لاگ.
pub fn is_stun_datagram(data: &[u8]) -> bool {
    let Some(header) = socks_udp_header_length(data) else {
        return false;
    };
    let p = &data[header..];
    if p.len() < 20 || p[0] & 0xC0 != 0 || p[4..8] != [0x21, 0x12, 0xA4, 0x42] {
        return false;
    }
    let declared = u16_at(p, 2) as usize;
    declared % 4 == 0 && 20 + declared <= p.len()
}

// ---- LanGuard.kt -------------------------------------------------------------

fn b64_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// base64ِ استاندارد (با یا بی `=`)؛ `None` برای هر نویسهٔ نامعتبر.
pub fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let s = input.trim().trim_end_matches('=');
    if s.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits = 0;
    for c in s.bytes() {
        acc = (acc << 6) | b64_value(c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn base64_encode(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// مقدارِ `Proxy-Authorization: Basic …` → (کاربر، رمز).
pub fn parse_basic_credential(header_value: &str) -> Option<(String, String)> {
    let t = header_value.trim();
    if t.len() < 5 || !t[..5].eq_ignore_ascii_case("basic") {
        return None;
    }
    let encoded = t[5..].trim();
    if encoded.is_empty() {
        return None;
    }
    let decoded = String::from_utf8(base64_decode(encoded)?).ok()?;
    let (user, pass) = decoded.split_once(':')?;
    Some((user.to_string(), pass.to_string()))
}

/// مقایسهٔ زمان‌ثابت. موبایل هر دو را SHA-256 می‌کند تا طول هم لو نرود؛ این‌جا
/// بی‌هیچ وابستگیِ رمزنگاری، هر بایتِ هر دو طرف تا طولِ بلندتر پیموده می‌شود.
pub fn secret_equals(expected: &str, offered: &str) -> bool {
    if expected.is_empty() {
        return false;
    }
    let (a, b) = (expected.as_bytes(), offered.as_bytes());
    let n = a.len().max(b.len());
    let mut diff = (a.len() ^ b.len()) as u32;
    for i in 0..n {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        diff |= (x ^ y) as u32;
    }
    std::hint::black_box(diff) == 0
}

const PASSWORD_ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[cfg(windows)]
fn fill_random(buf: &mut [u8]) -> bool {
    use windows::Win32::Security::Cryptography::{
        BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    };
    unsafe { BCryptGenRandom(None, buf, BCRYPT_USE_SYSTEM_PREFERRED_RNG).is_ok() }
}

#[cfg(not(windows))]
fn fill_random(buf: &mut [u8]) -> bool {
    use std::io::Read;
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(buf))
        .is_ok()
}

/// رمزِ تصادفیِ خوانا (بی 0/O/1/l/I). از مولدِ رمزنگاریِ سیستم.
pub fn random_password(length: usize) -> String {
    let mut out = String::with_capacity(length);
    let n = PASSWORD_ALPHABET.len();
    // نمونه‌گیریِ ردکننده: بایت‌های بالای بزرگ‌ترین مضربِ n کنار گذاشته می‌شوند.
    let limit = 256 - (256 % n);
    let mut buf = [0u8; 64];
    while out.len() < length {
        if !fill_random(&mut buf) {
            // مولدِ سیستم در دسترس نیست: رمز ساخته نمی‌شود، بهتر از رمزِ قابل‌حدس.
            return String::new();
        }
        for b in buf {
            if (b as usize) < limit && out.len() < length {
                out.push(PASSWORD_ALPHABET[b as usize % n] as char);
            }
        }
    }
    out
}

/// نام کاربری: ۱ تا ۶۴ نویسهٔ ASCIIِ چاپی و بی‌فاصله، بی `:` (Basic آن را
/// جداکننده می‌گیرد) — همان قاعدهٔ `ShareCredentials.setCustom` موبایل.
pub fn valid_user(user: &str) -> bool {
    let u = user.trim();
    !u.is_empty()
        && u.len() <= MAX_USER
        && u.bytes().all(|b| (0x21..=0x7E).contains(&b) && b != b':')
}

/// رمز: ۸ تا ۱۲۸ نویسهٔ ASCIIِ چاپی (فاصله مجاز است).
pub fn valid_password(password: &str) -> bool {
    (MIN_PASSWORD..=MAX_PASSWORD).contains(&password.len())
        && password.bytes().all(|b| (0x20..=0x7E).contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn only_public_unicast_is_public() {
        for p in ["8.8.8.8", "1.1.1.1", "2606:4700::1111", "203.0.113.9"] {
            assert!(is_public_unicast(ip(p)), "{p}");
        }
        for p in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.1.1",
            "172.16.0.1",
            "169.254.1.1",
            "100.64.0.1",
            "0.1.2.3",
            "224.0.0.1",
            "255.255.255.255",
            "198.18.0.1",
            "192.0.0.8",
            "::1",
            "fe80::1",
            "fd00::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
            "::ffff:192.168.1.1",
        ] {
            assert!(!is_public_unicast(ip(p)), "{p}");
        }
    }

    #[test]
    fn lan_guests_cannot_reach_the_lan_or_this_pc() {
        for h in [
            "example.com",
            "8.8.8.8",
            "[2606:4700::1111]",
            "example.com.",
        ] {
            assert!(destination_allowed(h), "{h}");
        }
        for h in [
            "localhost",
            "a.localhost",
            "printer.local",
            "nas.lan",
            "x.home.arpa",
            "svc.internal",
            "corp.intranet",
            "127.0.0.1",
            "192.168.1.1",
            "[::1]",
            "0x7f.1",
            "2130706433",
            "",
            "::1",
            "[bad",
        ] {
            assert!(!destination_allowed(h), "{h}");
        }
    }

    #[test]
    fn udp_datagrams_are_checked_like_tcp() {
        let mut pkt = vec![0, 0, 0, 1, 8, 8, 8, 8, 0, 53];
        assert!(udp_datagram_allowed(&pkt));
        pkt[4] = 192;
        pkt[5] = 168;
        assert!(!udp_datagram_allowed(&pkt));
        let frag = [0, 0, 1, 1, 8, 8, 8, 8, 0, 53];
        assert!(!udp_datagram_allowed(&frag));
        let mut named = vec![0, 0, 0, 3, 11];
        named.extend_from_slice(b"example.com");
        named.extend_from_slice(&[0, 53]);
        assert!(udp_datagram_allowed(&named));
        let mut local = vec![0, 0, 0, 3, 9];
        local.extend_from_slice(b"localhost");
        local.extend_from_slice(&[0, 53]);
        assert!(!udp_datagram_allowed(&local));
    }

    #[test]
    fn identity_headers_are_recognised() {
        assert!(is_identity_header("X-Forwarded-For: 1.2.3.4"));
        assert!(is_identity_header("via: 1.1 proxy"));
        assert!(!is_identity_header("Host: example.com"));
    }

    #[test]
    fn subnets_compare_on_the_prefix() {
        assert!(same_subnet(ip("192.168.1.20"), ip("192.168.1.1"), 24));
        assert!(!same_subnet(ip("192.168.2.20"), ip("192.168.1.1"), 24));
        assert!(same_subnet(ip("10.0.5.1"), ip("10.0.200.1"), 16));
        assert!(same_subnet(
            ip("::ffff:192.168.1.20"),
            ip("192.168.1.1"),
            24
        ));
        assert_eq!(sane_prefix(32), 24);
        assert_eq!(sane_prefix(16), 16);
    }

    #[test]
    fn the_pac_names_the_proxy_and_keeps_local_names_direct() {
        let pac = pac_script("192.168.1.20", 10811);
        assert!(pac.starts_with("function FindProxyForURL(url, host) {\n"));
        assert!(pac.contains("return \"PROXY 192.168.1.20:10811\";"));
        assert!(pac.contains("return \"DIRECT\";"));
        assert!(pac.contains("/^172\\.(1[6-9]|2[0-9]|3[01])\\./.test(host)"));
    }

    #[test]
    fn the_check_host_and_its_sentinel() {
        assert!(is_check_target("dopamine.check"));
        assert!(is_check_target("DOPAMINE.CHECK."));
        assert!(is_check_target("198.51.100.254"));
        assert!(!is_check_target("dopamine.check.example"));
    }

    fn dns_query(name: &str, qtype: u16) -> Vec<u8> {
        let mut q = vec![0x12, 0x34, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0];
        for label in name.split('.') {
            q.push(label.len() as u8);
            q.extend_from_slice(label.as_bytes());
        }
        q.push(0);
        q.extend_from_slice(&qtype.to_be_bytes());
        q.extend_from_slice(&1u16.to_be_bytes());
        q
    }

    #[test]
    fn dns_for_the_check_host_is_answered_locally() {
        let mut pkt = vec![0, 0, 0, 1, 1, 1, 1, 1, 0, 53];
        pkt.extend_from_slice(&dns_query("dopamine.check", 1));
        let reply = check_host_dns_reply(&pkt).expect("answered");
        assert_eq!(&reply[..10], &pkt[..10]);
        let dns = &reply[10..];
        assert_eq!(&dns[..2], &[0x12, 0x34]);
        assert_eq!(u16_at(dns, 6), 1);
        assert_eq!(&dns[dns.len() - 4..], &[198, 51, 100, 254]);
        // AAAA: بی‌پاسخ ولی NOERROR.
        let mut aaaa = vec![0, 0, 0, 1, 1, 1, 1, 1, 0, 53];
        aaaa.extend_from_slice(&dns_query("dopamine.check", 28));
        assert_eq!(u16_at(&check_host_dns_reply(&aaaa).unwrap()[10..], 6), 0);
        let mut other = vec![0, 0, 0, 1, 1, 1, 1, 1, 0, 53];
        other.extend_from_slice(&dns_query("example.com", 1));
        assert!(check_host_dns_reply(&other).is_none());
        let mut not53 = vec![0, 0, 0, 1, 1, 1, 1, 1, 0, 54];
        not53.extend_from_slice(&dns_query("dopamine.check", 1));
        assert!(check_host_dns_reply(&not53).is_none());
    }

    #[test]
    fn stun_is_recognised() {
        let mut pkt = vec![0, 0, 0, 1, 74, 125, 1, 1, 0x4a, 0x69];
        pkt.extend_from_slice(&[0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xA4, 0x42]);
        pkt.extend_from_slice(&[7u8; 12]);
        assert!(is_stun_datagram(&pkt));
        pkt[14] = 0;
        assert!(!is_stun_datagram(&pkt));
    }

    #[test]
    fn basic_credentials_round_trip() {
        let value = format!("Basic {}", base64_encode(b"aether:s3cr:et"));
        assert_eq!(
            parse_basic_credential(&value),
            Some(("aether".into(), "s3cr:et".into()))
        );
        assert_eq!(
            parse_basic_credential("basic YTpi"),
            Some(("a".into(), "b".into()))
        );
        assert!(parse_basic_credential("Bearer x").is_none());
        assert!(parse_basic_credential("Basic !!!").is_none());
        assert!(parse_basic_credential("Basic bm9jb2xvbg==").is_none());
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
    }

    #[test]
    fn secrets_compare_exactly() {
        assert!(secret_equals("abc", "abc"));
        assert!(!secret_equals("abc", "abd"));
        assert!(!secret_equals("abc", "abcd"));
        assert!(!secret_equals("", ""));
    }

    #[test]
    fn generated_passwords_are_long_and_readable() {
        let p = random_password(16);
        assert_eq!(p.len(), 16);
        assert!(p.bytes().all(|b| PASSWORD_ALPHABET.contains(&b)));
        assert_ne!(p, random_password(16));
        assert!(valid_password(&p));
    }

    #[test]
    fn credential_rules() {
        assert!(valid_user("aether"));
        assert!(!valid_user(""));
        assert!(!valid_user("a:b"));
        assert!(!valid_user(&"x".repeat(65)));
        assert!(!valid_password("short"));
        assert!(!valid_password(&"x".repeat(129)));
        assert!(!valid_password("tab\there!"));
        assert!(!valid_password("رمزِفارسیِطولانی"));
        assert!(valid_password("with space ok"));
        assert!(!valid_user("has space"));
    }
}
