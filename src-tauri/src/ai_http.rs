//! پورت ۱:۱ از `ai/GeminiHttp.kt` — تنها مسیر شبکه‌ای که لایهٔ هوش مصنوعی
//! اجازهٔ استفاده از آن را دارد: HTTPS به گوگل، شماره‌گیری‌شده از پروکسی SOCKS5
//! محلیِ خودِ تونل.
//!
//! # چرا دست‌ساز و نه یک کلاینت HTTP آماده
//!
//! سه دلیل، به ترتیب شدتِ گزندگی:
//!
//!  1. **سوکت‌های خودِ ما از تونل رد نمی‌شوند.** [`crate::ai_gate`] این را کامل
//!     توضیح می‌دهد. یک کلاینت معمولی روی شبکهٔ اپراتور بیرون می‌رفت.
//!  2. **یک پروکسی SOCKS معمولی، DNS را محلی حل می‌کند.** پس دستگاه باید خودش
//!     `generativelanguage.googleapis.com` را روی resolver اپراتور و بیرونِ تونل
//!     حل کند — پرسشی که هم شکست می‌خورد و هم قصد را اعلام می‌کند. یک
//!     `CONNECT` دست‌نویس می‌تواند مقصد را به‌صورت **DOMAIN** (`ATYP=0x03`)
//!     بفرستد و اجازه دهد *نقطهٔ خروج* آن را حل کند — همان کاری که یک تب مرورگر
//!     داخل تونل می‌کند. اینجا از [`crate::probe::socks5_stream_on`] استفاده
//!     می‌شود که در همین مخزن دقیقاً همین کار را می‌کند؛ نوشتن نسخهٔ دومِ همان
//!     دست‌دادن، یعنی دو جای مستقل برای اشتباه‌کردن.
//!  3. **بدون وابستگی تازه.** برنامه هیچ کتابخانهٔ HTTP ندارد و اضافه‌کردن یکی
//!     برای یک نقطهٔ پایانی، حجم نصاب را برای کاری بزرگ می‌کند که ~۲۰۰ خط
//!     انجامش می‌دهد. `native-tls` از قبل در `Cargo.toml` هست (روی ویندوز
//!     SChannel بومی) چون `probe.rs` برای جای‌یابی IP همان را می‌خواست.
//!
//! TLS روی همین دستگاه پایان می‌یابد و `native-tls::TlsConnector::connect(host, …)`
//! نام روی گواهی را هم بررسی می‌کند — همان تفاوتی که موبایل مجبور بود دستی با
//! `HostnameVerifier` جبرانش کند، چون `startHandshake()` زنجیره را بررسی می‌کند
//! ولی نام را نه. اهمیتش اینجا از یک کاوش جای‌یابی بیشتر است: بدون آن، هرکسی
//! که گواهی معتبری برای *هر* دامنه‌ای داشته باشد می‌تواند روی این اتصال بنشیند و
//! کلید API کاربر و هر پرامپتی که تایپ می‌کند را بخواند.

use crate::log::DiagnosticsLog;
use crate::probe;
use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use std::time::Duration;

pub const HOST: &str = "generativelanguage.googleapis.com";
const PORT: u16 = 443;

/// چقدر برای اینکه پروکسی *محلی* اتصال را بپذیرد صبر کنیم.
///
/// کوتاه و جدا از تایم‌اوت درخواست، عمداً: `127.0.0.1` یا فوری جواب می‌دهد یا
/// گوش نمی‌دهد، پس تایم‌اوت بلند اینجا فقط باعث می‌شد گفتنِ «تونل پایین است»
/// یک دقیقه طول بکشد.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// سقفِ بدنهٔ پاسخ. یک پاسخ `models.list` چند ده کیلوبایت است و یک پاسخ چت
/// حداکثر چند کیلوبایت؛ سقف اینجا هست تا یک پاسخ بدقلق نتواند حافظه را ببلعد.
const MAX_BODY: usize = 4 * 1024 * 1024;

/// یک تبادل HTTP کامل. [`Response::code`] وقتی هیچ‌چیز دریافت نشده صفر است.
#[derive(Debug, Clone)]
pub struct Response {
    pub code: u16,
    pub body: String,
    /// `Retry-After` بر حسب ثانیه، وقتی سرور یکی فرستاده باشد.
    ///
    /// اینجا پارس می‌شود و نه در کلاینت، چون یک هدرِ سطحِ ترابرد است و کلاینت
    /// نباید لازم باشد بداند هدرها چطور قالب‌بندی می‌شوند. گوگل روی بعضی ۴۲۹ها
    /// و روی ۵۰۳ها می‌فرستد، و احترام‌گذاشتن به آن، تفاوتِ «یک انتظارِ مؤدب» و
    /// «پنج درخواست در ده ثانیه» است که یک حلقهٔ retry سادهٔ نادان می‌سازد.
    pub retry_after_seconds: Option<f64>,
}

impl Response {
    pub fn ok(&self) -> bool {
        (200..=299).contains(&self.code)
    }
}

/// یک درخواست را انجام می‌دهد و کل پاسخ را برمی‌گرداند.
///
/// * `socks_port` — پورت SOCKS5 محلی که باید از آن شماره‌گیری شود؛
///   [`crate::ai_gate::socks_port`]. هرگز یک سوکت مستقیم.
/// * `api_key` — به‌صورت هدر `x-goog-api-key` می‌رود، هرگز به‌صورت پارامتر
///   `?key=`: یک خطِ درخواست در جاهای خیلی بیشتری از یک هدر لاگ می‌شود.
pub fn request(
    method: &str,
    path: &str,
    socks_port: u16,
    api_key: &str,
    json_body: Option<&str>,
    timeout: Duration,
) -> Result<Response> {
    let stream =
        probe::socks5_stream_on(socks_port, HOST, PORT, CONNECT_TIMEOUT).ok_or_else(|| {
            anyhow!(
                "the local SOCKS5 proxy on 127.0.0.1:{socks_port} did not open a path to {HOST}"
            )
        })?;
    // Nagle خاموش: درخواست یک نوشتن کوچک است و بعدش یک خواندن، پس صبرکردن
    // برای ادغام‌کردنش فقط به هر پاسخ تأخیر اضافه می‌کند.
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let connector = native_tls::TlsConnector::new()
        .map_err(|e| anyhow!("TLS could not be initialised: {e}"))?;
    // نام میزبان همین‌جا بررسی می‌شود؛ خطای برگشتی از یک عدم‌تطابق گواهی، همان
    // چیزی است که در موبایل به‌صورت `SSLPeerUnverifiedException` بالا می‌آمد.
    let mut tls = connector.connect(HOST, stream).map_err(|e| {
        DiagnosticsLog::e(
            "ai",
            &format!("TLS to {HOST} failed (certificate or handshake): {e}"),
        );
        anyhow!("TLS handshake with {HOST} failed: {e}")
    })?;

    let payload = json_body.map(|b| b.as_bytes());
    let mut head = String::with_capacity(320);
    head.push_str(method);
    head.push(' ');
    head.push_str(path);
    head.push_str(" HTTP/1.1\r\n");
    head.push_str("Host: ");
    head.push_str(HOST);
    head.push_str("\r\n");
    head.push_str("x-goog-api-key: ");
    head.push_str(api_key);
    head.push_str("\r\n");
    head.push_str(concat!(
        "User-Agent: Dopamine-Windows/",
        env!("CARGO_PKG_VERSION"),
        "\r\n"
    ));
    head.push_str("Accept: application/json\r\n");
    // identity: هرگز بدنهٔ فشرده نمی‌خواهیم، چون این کلاینت gzip پیاده نکرده و
    // یک پاسخِ بی‌صدا gzip‌شده شبیه خطای پارس به نظر می‌رسید.
    head.push_str("Accept-Encoding: identity\r\n");
    if let Some(bytes) = payload {
        head.push_str("Content-Type: application/json; charset=utf-8\r\n");
        // طول بر حسب **بایت** و نه نویسه: یک پرامپت فارسی بایت بیشتری از نویسه
        // دارد و Content-Length شمرده‌شده روی نویسه، درخواست را به یک هنگ
        // نصفه‌کاره تبدیل می‌کند.
        head.push_str(&format!("Content-Length: {}\r\n", bytes.len()));
    }
    // پاسخ تا EOF خوانده می‌شود، پس هیچ قالب‌بندی keep-alive بین درخواست‌ها
    // ردیابی نمی‌شود.
    head.push_str("Connection: close\r\n\r\n");

    tls.write_all(head.as_bytes())?;
    if let Some(bytes) = payload {
        tls.write_all(bytes)?;
    }
    tls.flush()?;

    let mut raw: Vec<u8> = Vec::with_capacity(16 * 1024);
    let mut chunk = [0u8; 16 * 1024];
    loop {
        match tls.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&chunk[..n]);
                if raw.len() > MAX_BODY {
                    break;
                }
            }
            // `close_notify` نبودن در پایان، در عمل شایع است و یک بدنهٔ کامل را
            // نباید دور بیندازیم؛ اگر چیزی خوانده‌ایم، همان را پارس می‌کنیم.
            Err(_) if !raw.is_empty() => break,
            Err(e) => return Err(anyhow!("reading the reply failed: {e}")),
        }
    }
    Ok(parse(&raw))
}

// ---- پارس پاسخ -------------------------------------------------------------

fn parse(raw: &[u8]) -> Response {
    let Some(split) = index_of_header_end(raw) else {
        return Response {
            code: 0,
            body: String::new(),
            retry_after_seconds: None,
        };
    };
    // بایت‌های هدر بنا به تعریف latin-1 هستند.
    let head_text: String = raw[..split].iter().map(|&b| b as char).collect();
    let body_bytes = &raw[split + 4..];

    let code = head_text
        .split("\r\n")
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    let chunked = head_text.lines().any(|l| {
        let lower = l.to_ascii_lowercase();
        lower.starts_with("transfer-encoding") && lower.contains("chunked")
    });
    let body = if chunked {
        dechunk(body_bytes)
    } else {
        body_bytes.to_vec()
    };

    let retry_after = head_text
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("retry-after"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim())
        // فقط قالب delta-seconds. قالب HTTP-date قانونی است و گوگل اینجا از آن
        // استفاده نمی‌کند؛ بد پارس‌کردن یک تاریخ، از برگشتن به backoff خودمان
        // بدتر بود.
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| *v >= 0.0);

    Response {
        code,
        body: String::from_utf8_lossy(&body).into_owned(),
        retry_after_seconds: retry_after,
    }
}

/// جای CRLFCRLF که بلوک هدر را تمام می‌کند.
fn index_of_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n")
}

/// `Transfer-Encoding: chunked` را رمزگشایی می‌کند.
///
/// روی **بایت‌ها** و نه روی یک رشتهٔ رمزگشایی‌شده، و همین کل نکته است: مرز یک
/// chunk می‌تواند وسط یک توالی چندبایتیِ UTF-8 بیفتد، پس اول بدنه را به متن
/// تبدیل‌کردن و بعد هدرهای chunk را کندن، هر پاسخ فارسی‌ای را که از یک chunk
/// بلندتر باشد خراب می‌کند. اول قالب‌بندی، یک بار رمزگشایی، در پایان.
fn dechunk(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len());
    let mut i = 0usize;
    while i < src.len() {
        let mut end = i;
        while end + 1 < src.len() && !(src[end] == b'\r' && src[end + 1] == b'\n') {
            end += 1;
        }
        if end + 1 >= src.len() {
            break;
        }
        let size_text: String = src[i..end].iter().map(|&b| b as char).collect();
        let size_text = size_text.trim().split(';').next().unwrap_or("").to_string();
        let Ok(size) = usize::from_str_radix(&size_text, 16) else {
            break;
        };
        i = end + 2;
        if size == 0 {
            break;
        }
        if i + size > src.len() {
            out.extend_from_slice(&src[i..]);
            break;
        }
        out.extend_from_slice(&src[i..i + size]);
        // از CRLF خودِ chunk رد شو.
        i += size + 2;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_body_and_retry_after() {
        let raw = b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 2.5\r\nContent-Type: application/json\r\n\r\n{\"error\":{}}";
        let r = parse(raw);
        assert_eq!(r.code, 429);
        assert_eq!(r.retry_after_seconds, Some(2.5));
        assert_eq!(r.body, "{\"error\":{}}");
        assert!(!r.ok());
    }

    #[test]
    fn dechunks_across_a_multibyte_boundary() {
        // «سلام» در UTF-8 هشت بایت است. اینجا مرز chunk وسط یک نویسه می‌افتد:
        // اگر ابتدا به متن تبدیل می‌کردیم، این آزمون نویسهٔ خراب می‌داد.
        let word = "سلام".as_bytes();
        let (a, b) = word.split_at(3);
        let mut raw = Vec::new();
        raw.extend_from_slice(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n");
        raw.extend_from_slice(format!("{:x}\r\n", a.len()).as_bytes());
        raw.extend_from_slice(a);
        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(format!("{:x}\r\n", b.len()).as_bytes());
        raw.extend_from_slice(b);
        raw.extend_from_slice(b"\r\n0\r\n\r\n");
        let r = parse(&raw);
        assert_eq!(r.code, 200);
        assert_eq!(r.body, "سلام");
    }

    #[test]
    fn a_reply_with_no_header_block_is_code_zero() {
        let r = parse(b"garbage");
        assert_eq!(r.code, 0);
        assert!(r.body.is_empty());
    }
}
