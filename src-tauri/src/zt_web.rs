//! ۱.۴.۰ (پورت ۱:۱ از `core/ZeroTrustWeb.kt` و `core/AccessToken.kt` موبایل) —
//! تکه‌های خالصِ ورودِ پیش از اتصالِ Zero Trust: نام تیم، نشانی‌ها، بیرون کشیدنِ
//! JWT از HTML/کوکی، فرمِ کدِ ایمیلی، و رمزگشاییِ کوئری.
//!
//! # بازنویسی در این مرحله
//!
//! نسخهٔ قبلیِ این فایل به `regex` تکیه داشت که در `Cargo.toml` نیست؛ یعنی کل
//! برنامه کامپایل نمی‌شد. این نسخه هیچ وابستگیِ تازه‌ای ندارد و رفتارش عیناً
//! همان `ZeroTrustWeb.kt` است (همان آزمون‌ها پایین آمده‌اند).

pub const TEAM_SUFFIX: &str = "cloudflareaccess.com";
const ENROLL_PATH: &str = "/warp";
const MIN_JWT_LENGTH: usize = 32;

/// `acme`، `acme.cloudflareaccess.com`، `https://acme.cloudflareaccess.com/warp`
/// → `acme`. `None` اگر چیزی جز حروف/رقم/`-`/`_` بماند.
pub fn normalize_team(raw: &str) -> Option<String> {
    let mut value = raw.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    for prefix in ["https://", "http://"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            value = rest.to_string();
        }
    }
    let value = value.trim_end_matches('/');
    let mut value = value.split('/').next().unwrap_or("").to_string();
    if let Some(stripped) = value.strip_suffix(TEAM_SUFFIX) {
        value = stripped.trim_end_matches('.').to_string();
    }
    if value.is_empty() {
        return None;
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(value)
}

pub fn team_host(team: &str) -> String {
    format!("{team}.{TEAM_SUFFIX}")
}

pub fn team_domain(team: &str) -> String {
    format!("https://{}", team_host(team))
}

pub fn login_url(team: &str) -> String {
    format!("{}{ENROLL_PATH}", team_domain(team))
}

pub fn callback_url(team: &str) -> String {
    format!("{}/cdn-cgi/access/callback", team_domain(team))
}

/// صفحهٔ پایانیِ ثبت‌نام به `com.cloudflare.warp://…?token=<JWT>` ریدایرکت
/// می‌کند (یا آن را در HTML می‌گذارد). مقدارِ پس از `token=` تا اولین جداکننده.
pub fn extract_jwt_from_html(html: &str) -> Option<String> {
    let marker = "token=";
    let at = html.find(marker)?;
    let rest = &html[at + marker.len()..];
    let end = rest
        .find(|c: char| c == '"' || c == '\'' || c == '&' || c == '<' || c.is_whitespace())
        .unwrap_or(rest.len());
    let candidate = rest[..end].trim();
    looks_like_jwt(candidate).then(|| candidate.to_string())
}

/// یک سرآیندِ `Set-Cookie` → مقدارِ `CF_Authorization` اگر JWT باشد.
pub fn extract_jwt_from_cookie(header: &str) -> Option<String> {
    for part in header.split(';') {
        let entry = part.trim();
        let value = if let Some(v) = entry.strip_prefix("CF_Authorization=") {
            v
        } else if let Some(v) = entry.strip_prefix("cf_authorization=") {
            v
        } else {
            return None;
        };
        if looks_like_jwt(value) {
            return Some(value.to_string());
        }
    }
    None
}

/// `action`ِ فرمی که `id="totp-form"` دارد — همان فرمی که ایمیل را می‌گیرد.
pub fn extract_totp_form_action(html: &str) -> Option<String> {
    let anchor = html.find("id=\"totp-form\"")?;
    let form_start = html[..anchor].rfind("<form")?;
    let tag_end = form_start + html[form_start..].find('>')?;
    let tag = &html[form_start..tag_end];
    let key_at = tag.find("action=")?;
    let rest = tag[key_at + "action=".len()..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let body = &rest[1..];
    let end = body.find(quote)?;
    let action = decode_entities(&body[..end]);
    action.starts_with("https://").then_some(action)
}

pub fn decode_entities(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let semi = rest.find(';');
        let Some(semi) = semi.filter(|s| *s <= 8) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded: Option<String> = match entity {
            "amp" => Some("&".into()),
            "lt" => Some("<".into()),
            "gt" => Some(">".into()),
            "quot" => Some("\"".into()),
            "apos" | "#39" => Some("'".into()),
            _ => numeric_entity(entity),
        };
        match decoded {
            Some(d) => {
                out.push_str(&d);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn numeric_entity(entity: &str) -> Option<String> {
    let code = if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        u32::from_str_radix(hex, 16).ok()?
    } else if let Some(dec) = entity.strip_prefix('#') {
        dec.parse::<u32>().ok()?
    } else {
        return None;
    };
    char::from_u32(code).map(|c| c.to_string())
}

pub fn query_value(url: &str, key: &str) -> Option<String> {
    let q = url.find('?')?;
    let query = url[q + 1..].split('#').next().unwrap_or("");
    for pair in query.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        if k == key {
            return Some(percent_decode(v));
        }
    }
    None
}

pub fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 2 < bytes.len() {
            match (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                (Some(hi), Some(lo)) => {
                    out.push((hi << 4) | lo);
                    i += 3;
                }
                _ => {
                    out.push(b);
                    i += 1;
                }
            }
        } else if b == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub fn plausible_email(raw: &str) -> bool {
    let email = raw.trim();
    let len = email.chars().count();
    if !(3..=254).contains(&len) {
        return false;
    }
    if email
        .chars()
        .any(|c| c.is_whitespace() || (c as u32) < 0x20)
    {
        return false;
    }
    match (email.find('@'), email.rfind('@')) {
        (Some(at), Some(last)) => at > 0 && at == last && at < email.len() - 1,
        _ => false,
    }
}

// ---- AccessToken.kt ---------------------------------------------------------

/// base64url بدون padding (و با padding) → بایت‌ها.
fn b64url_decode(segment: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(segment.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for c in segment.trim_end_matches('=').bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    // یک نویسهٔ تنهای باقی‌مانده base64 معتبر نیست.
    if bits >= 6 {
        return None;
    }
    Some(out)
}

pub fn looks_like_jwt(token: &str) -> bool {
    let t = token.trim();
    if t.len() < MIN_JWT_LENGTH {
        return false;
    }
    let segments: Vec<&str> = t.split('.').collect();
    if segments.len() != 3 || segments.iter().any(|s| s.is_empty()) {
        return false;
    }
    if !t
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return false;
    }
    b64url_decode(segments[0]).is_some()
}

/// ادعای `exp` (ثانیهٔ یونیکس) اگر باشد.
pub fn expiry_seconds(token: &str) -> Option<i64> {
    let segments: Vec<&str> = token.trim().split('.').collect();
    if segments.len() != 3 {
        return None;
    }
    let payload = String::from_utf8(b64url_decode(segments[1])?).ok()?;
    let at = payload.find("\"exp\"")?;
    let rest = payload[at + 5..]
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    let digits: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .take(19)
        .collect();
    digits.parse::<i64>().ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Empty,
    Malformed,
    Expired,
    /// روزهای باقی‌مانده، اگر توکن `exp` داشته باشد.
    Usable(Option<i64>),
}

pub fn inspect(raw: &str, now_secs: i64) -> Verdict {
    let token = raw.trim();
    if token.is_empty() {
        return Verdict::Empty;
    }
    if !looks_like_jwt(token) {
        return Verdict::Malformed;
    }
    match expiry_seconds(token) {
        None => Verdict::Usable(None),
        Some(exp) if exp <= now_secs => Verdict::Expired,
        Some(exp) => Verdict::Usable(Some((exp - now_secs) / 86_400)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // {"alg":"none"} . {"exp":4102444800} . sig
    const JWT: &str = "eyJhbGciOiJub25lIn0.eyJleHAiOjQxMDI0NDQ4MDB9.c2lnbmF0dXJlLXNpZ25hdHVyZQ";

    #[test]
    fn team_names_are_normalised() {
        assert_eq!(normalize_team("acme").as_deref(), Some("acme"));
        assert_eq!(
            normalize_team(" ACME.cloudflareaccess.com ").as_deref(),
            Some("acme")
        );
        assert_eq!(
            normalize_team("https://acme.cloudflareaccess.com/warp").as_deref(),
            Some("acme")
        );
        assert_eq!(normalize_team(""), None);
        assert_eq!(normalize_team("a b"), None);
        assert_eq!(normalize_team("evil.com"), None);
    }

    #[test]
    fn urls_are_built_on_the_team_host() {
        assert_eq!(login_url("acme"), "https://acme.cloudflareaccess.com/warp");
        assert_eq!(
            callback_url("acme"),
            "https://acme.cloudflareaccess.com/cdn-cgi/access/callback"
        );
    }

    #[test]
    fn jwt_is_found_in_html_and_cookies() {
        let html = format!("<a href=\"com.cloudflare.warp://acme/auth?token={JWT}\">");
        assert_eq!(extract_jwt_from_html(&html).as_deref(), Some(JWT));
        assert_eq!(extract_jwt_from_html("token=short"), None);
        let cookie = format!("CF_Authorization={JWT}; Path=/; Secure");
        assert_eq!(extract_jwt_from_cookie(&cookie).as_deref(), Some(JWT));
        assert_eq!(extract_jwt_from_cookie("other=1; CF_Authorization=x"), None);
    }

    #[test]
    fn totp_form_action_is_read_and_unescaped() {
        let html = r#"<form method="post" action="https://acme.cloudflareaccess.com/cdn-cgi/access/login?a=1&amp;b=2" id="totp-form">"#;
        assert_eq!(
            extract_totp_form_action(html).as_deref(),
            Some("https://acme.cloudflareaccess.com/cdn-cgi/access/login?a=1&b=2")
        );
        assert_eq!(
            extract_totp_form_action(r#"<form action="/x" id="totp-form">"#),
            None
        );
    }

    #[test]
    fn queries_and_emails() {
        assert_eq!(
            query_value("https://h/x?nonce=a%2Bb+c&z=1", "nonce").as_deref(),
            Some("a+b c")
        );
        assert!(plausible_email("me@example.com"));
        assert!(!plausible_email("me@@example.com"));
        assert!(!plausible_email("@x"));
        assert!(!plausible_email("a b@x.io"));
    }

    #[test]
    fn tokens_are_inspected() {
        assert!(looks_like_jwt(JWT));
        assert_eq!(expiry_seconds(JWT), Some(4_102_444_800));
        assert!(matches!(inspect(JWT, 0), Verdict::Usable(Some(_))));
        assert_eq!(inspect(JWT, 4_102_444_800), Verdict::Expired);
        assert_eq!(inspect("", 0), Verdict::Empty);
        assert_eq!(inspect("a.b.c", 0), Verdict::Malformed);
    }
}
