//! پورت ۱:۱ از `ai/AiRedaction.kt` — پاک‌سازی یک برش از لاگ پیش از رفتن به گوگل.
//!
//! # چرا یک لاگ را نمی‌توان همین‌طور فرستاد
//!
//! قابلیت بهینه‌سازی DPI با نشان‌دادن آنچه لاگ اتصال می‌گوید به مدل کار می‌کند.
//! آن لاگ را یک ابزار دور‌زدنِ فیلترینگ برای کاربرانی زیر نظارت می‌نویسد و در
//! سطوح مختلف پرگویی حاوی این‌هاست: نشانی اندپوینتی که اسکن رویش نشسته، آی‌پی
//! خودِ اپراتور کاربر که کاوش جای‌یابی گزارش کرده، شناسه‌های ثبت‌نام Zero Trust،
//! و — به‌محض اینکه کسی یکی را در فیلدی بچسباند — یک اطلاعات محرمانه.
//! دادنِ آن به‌صورت کلمه‌به‌کلمه به یک API شخص ثالث، داده‌ای را که فقط برای
//! کمک به عیب‌یابی تونل وجود دارد می‌گیرد و هویت شبکه‌ای کاربر را با آن منتشر
//! می‌کند.
//!
//! پس خلاصه‌ای که دستگاه را ترک می‌کند **فیلتر** می‌شود، نه صرفاً کوتاه:
//!
//!  - هر چیزی که شکل یک اطلاعات محرمانه دارد یک‌کاسه جایگزین می‌شود، از جمله
//!    کلید Gemini خودِ کاربر (که وگرنه از دل درخواستی که **با** همان کلید
//!    احراز شده بازگو می‌شد)؛
//!  - نشانی‌های عمومی IPv4 به /16 و IPv6 به /32 ماسک می‌شوند، که برای استدلال
//!    مدل درباره‌ی «همان لبه مدام شکست می‌خورد» کافی است و برای شناسایی یک
//!    مشترک کافی نیست. بازه‌های خصوصی، لوپ‌بک و لینک‌لوکال در هر دو خانواده
//!    دست‌نخورده می‌مانند، چون `127.0.0.1:1819` تشخیصی‌ترین رشتهٔ کل لاگ است؛
//!  - شناسه‌های دامنهٔ نصب — دستهٔ ثبت‌نام `device=` در WARP و هر UUID خالی —
//!    حذف می‌شوند، چون این نصب را برای همیشه نام‌گذاری می‌کنند و از هر تغییر
//!    نشانی جان سالم می‌برند؛
//!  - خلاصه سقف دارد، تا یک نشستِ پرگو نتواند بی‌صدا یک مگابایت تاریخ را از
//!    دستگاه بیرون بفرستد.
//!
//! # چرا بدون `regex`
//!
//! مخزن هیچ وابستگی regex ندارد و کل هدفِ اعلام‌شدهٔ لایهٔ هوش مصنوعی این است که
//! **هیچ** وابستگی تازه‌ای به نصاب اضافه نکند. مهم‌تر: قواعد این فایل در سمت
//! اندروید همان چیزی بودند که به دو باگ *امنیتی* و یک *کرش* منجر شدند — یک
//! الگوی شُل که `10:23:41` (مُهر زمان هر خط از این لاگ) را IPv6 می‌دید، و یک
//! `groupValues[1]` روی الگویی که هیچ گروه ثبت‌کننده‌ای نداشت. اسکنِ صریحِ
//! نویسه‌به‌نویسه هر دو دسته اشتباه را حذف می‌کند: هر تصمیم اینجا با نام قابل
//! خواندن است و با آزمون بسته شده.

/// سقف سختِ نویسه‌های لاگی که مایلیم بفرستیم.
pub const MAX_DIGEST_CHARS: usize = 12_000;

/// تعداد خطوطِ انتهاییِ لاگ که خلاصه از آن ساخته می‌شود.
pub const DEFAULT_MAX_LINES: usize = 220;

/// برچسب‌هایی که مقدارِ بعدشان یک اطلاعات محرمانه است.
///
/// شکل `"key":"value"` هم پوشش داده می‌شود و این تزئینی نیست: نیمی از لاگ این
/// برنامه، نوتیس‌های JSON خودِ Psiphon است، پس قاعده‌ای که فقط برچسب‌های خالی را
/// بفهمد، `{"sessionId":"5a4c…"}` را نثر عادی می‌خواند و کلمه‌به‌کلمه فوروارد
/// می‌کند.
const CREDENTIAL_LABELS: [&str; 10] = [
    "authorization",
    "password",
    "passwd",
    "client_id",
    "client-id",
    "clientid",
    "secret",
    "bearer",
    "token",
    "key",
];

/// شناسه‌های دامنهٔ نصب.
///
/// نیمهٔ مهم، `device` است: `CREDENTIAL_LABELS` آن را نمی‌گرفت چون کلمه «device»
/// است و نه «key» یا «token» — و آن خط را موتور روی **هر** اتصال در پرگوییِ
/// debug می‌نویسد:
///
/// ```text
/// [+] identity ready: device=c79b496b-… ipv4=172.16.0.2 ipv6=2606:4700:110:…
/// ```
const DEVICE_LABELS: [&str; 8] = [
    "installation_id",
    "installation-id",
    "installationid",
    "session_id",
    "session-id",
    "sessionid",
    "identity",
    "device",
];

/// خلاصه را می‌سازد: تازه‌ترین [`DEFAULT_MAX_LINES`] خط، پاک‌شده و سقف‌خورده.
///
/// **دنباله** و نه سر: وقتی اتصال بدرفتاری می‌کند، شاهدِ مفید آن است که آخر چه
/// شد، و سرِ یک لاگِ بازگردانده‌شده، نشستِ قبلی است.
pub fn digest(full_log: &str, own_api_key: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = full_log.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    let cleaned: Vec<String> = lines[start..].iter().map(|l| safe_redact_line(l)).collect();
    let mut joined = cleaned.join("\n");
    if !own_api_key.trim().is_empty() {
        joined = joined.replace(own_api_key.trim(), "[REDACTED]");
    }
    if joined.chars().count() <= MAX_DIGEST_CHARS {
        return joined;
    }
    // از **انتها** بریده می‌شود، به همان دلیل که دنبالهٔ خطوط گرفته می‌شود.
    let skip = joined.chars().count() - MAX_DIGEST_CHARS;
    joined.chars().skip(skip).collect()
}

/// **هر** خطِ [`text`] را پاک می‌کند، بی‌سقفِ خط و بی‌سقفِ طول.
///
/// [`digest`] برای ساختنِ یک بارِ کوچک و کران‌دار برای یک مدل است. این یکی برای
/// جهت دیگر است: دکمهٔ «کپی لاگ» خودِ کاربر، آنجا که کلِ لاگ خواسته می‌شود ولی
/// هویتِ داخلش نه. همان قواعد، همان محدودسازیِ خطایِ خط‌به‌خط — خطی که پرت کند
/// حذف می‌شود، هرگز خام منتشر نمی‌شود.
pub fn redact_all(text: &str, own_api_key: &str) -> String {
    let cleaned: Vec<String> = text.lines().map(safe_redact_line).collect();
    let joined = cleaned.join("\n");
    if own_api_key.trim().is_empty() {
        joined
    } else {
        joined.replace(own_api_key.trim(), "[REDACTED]")
    }
}

/// [`redact_line`] با تضمین سختِ اینکه نمی‌تواند فرآیند را از پا بیندازد.
///
/// پاک‌کننده روی متنی صدا زده می‌شود که مهاجم بر آن اثر دارد (نام سرورها،
/// مقادیر SNI، کانفیگ چسبانده‌شده). باگِ اندرویدی هزینه را ثابت کرد: یک فرضِ
/// بی‌تطابق داخل یک `Regex.replace` یک کرشِ قابل‌تکرار در زمان اتصال بود.
///
/// اگر روزی خطی پرت کند، *آن خط* حذف می‌شود — با یک نشانگر جایگزین می‌شود و
/// هرگز با محتوای خامش، چون برگشتن به متن پاک‌نشده یک کرش را به یک نشتِ داده
/// تبدیل می‌کرد — و بقیهٔ خلاصه بیرون می‌رود.
///
/// # هشدارِ صادقانه: این تور در بیلد انتشار پهن نیست
///
/// `Cargo.toml` این کریت `panic = "abort"` دارد، پس در بیلد release هیچ
/// unwindی وجود ندارد که گرفته شود و این `catch_unwind` بی‌اثر است. عمداً نگه
/// داشته می‌شود چون در بیلد debug و در `cargo test` **کار می‌کند** و همان‌جاست
/// که یک باگ باید پیدا شود. نتیجهٔ عملی این است که کدِ زیر باید *بنا به ساخت*
/// بی‌پنیک باشد و نه به این گارد تکیه کند: هر دسترسی به `chars` در همین فایل
/// کران‌بررسی‌شده است و هیچ `unwrap`ی روی داده‌ی ورودی وجود ندارد.
fn safe_redact_line(line: &str) -> String {
    std::panic::catch_unwind(|| redact_line(line))
        .unwrap_or_else(|_| "[REDACTION-FAILED-LINE-DROPPED]".to_string())
}

/// یک خط را پاک می‌کند: اطلاعات محرمانه، بعد شناسه‌ها، بعد نشانی‌ها.
///
/// ترتیب اهمیت دارد. اطلاعات محرمانه اول می‌آید چون رازی که تصادفاً یک زنجیرهٔ
/// دونقطه داشته باشد وگرنه نیمش را قاعدهٔ IPv6 می‌خورد و نیمش جان سالم می‌برد.
/// IPv6 پیش از IPv4 می‌آید چون یک نشانی v6 نگاشتهٔ v4 (`::ffff:1.2.3.4`) باید
/// به‌عنوان **یک** نشانی رفتار شود، نه یک زنجیرهٔ دونقطه با یک چهارگانِ نقطه‌دار
/// جامانده پشتش.
pub fn redact_line(line: &str) -> String {
    let mut out = redact_google_keys(line);
    out = redact_labelled(&out, &CREDENTIAL_LABELS);
    out = redact_labelled(&out, &DEVICE_LABELS);
    out = redact_uuids(&out);
    out = mask_ipv6_literals(&out);
    mask_ipv4_literals(&out)
}

// --------------------------------------------------------- کلیدهای API گوگل

fn is_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// کلیدهای API گوگل شکل ثابت و قابل‌تشخیصی دارند؛ هر جا بودند بکششان.
fn redact_google_keys(line: &str) -> String {
    let bytes: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == 'A' && bytes[i..].starts_with(&['A', 'I', 'z', 'a']) {
            let mut end = i + 4;
            while end < bytes.len() && is_key_char(bytes[end]) {
                end += 1;
            }
            if end - (i + 4) >= 10 {
                out.push_str("[REDACTED]");
                i = end;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

// ------------------------------------------------- فیلدهای برچسب‌دار (key=value)

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// `label: value` / `label=value` / `"label":"value"` را به `label=[REDACTED]`
/// تبدیل می‌کند.
///
/// تطبیقِ برچسب بی‌توجه به بزرگی حروف است و باید یک **کلمهٔ کامل** باشد: بدون
/// این شرط، `monkey=` هم به‌خاطر «key» می‌افتاد و — بدتر — `keepalive` هم.
fn redact_labelled(line: &str, labels: &[&str]) -> String {
    let chars: Vec<char> = line.chars().collect();
    let lower: Vec<char> = line.to_lowercase().chars().collect();
    // to_lowercase می‌تواند طول را عوض کند (نه در ASCII، ولی خط می‌تواند فارسی
    // باشد)؛ اگر همتراز نبود، امن‌ترین کار این است که برچسب‌ها را نادیده بگیریم
    // و بگذاریم بقیهٔ قواعد کارشان را بکنند.
    if lower.len() != chars.len() {
        return line.to_string();
    }

    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    'outer: while i < chars.len() {
        // مرزِ کلمه: نویسهٔ قبلی نباید بخشی از یک شناسه باشد.
        let at_boundary = i == 0 || !is_ident_char(chars[i - 1]);
        if at_boundary {
            for label in labels {
                let label_chars: Vec<char> = label.chars().collect();
                let end = i + label_chars.len();
                if end <= lower.len() && lower[i..end] == label_chars[..] {
                    // بعد از برچسب: نقل‌قول اختیاری، فاصله، `:` یا `=`، فاصله،
                    // نقل‌قول اختیاری، و بعد یک اجرای بدون‌فاصله به‌عنوان مقدار.
                    let mut j = end;
                    if j < chars.len() && chars[j] == '"' {
                        j += 1;
                    }
                    while j < chars.len() && chars[j] == ' ' {
                        j += 1;
                    }
                    if j < chars.len() && (chars[j] == ':' || chars[j] == '=') {
                        j += 1;
                        while j < chars.len() && chars[j] == ' ' {
                            j += 1;
                        }
                        if j < chars.len() && chars[j] == '"' {
                            j += 1;
                        }
                        let value_start = j;
                        while j < chars.len() && !chars[j].is_whitespace() {
                            j += 1;
                        }
                        if j > value_start {
                            out.push_str(label);
                            out.push_str("=[REDACTED]");
                            i = j;
                            continue 'outer;
                        }
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

// -------------------------------------------------------------------- UUIDها

/// UUIDهای خالی، هر جا که باشند.
///
/// این قاعده نیمهٔ مهمِ کار است: یک شناسهٔ نشست، یک شناسهٔ تشخیصی یا یک دستهٔ
/// ثبت‌نام که بی‌برچسبِ قابل‌تشخیص چاپ شده، دقیقاً همان رشته‌ای است که یک قاعدهٔ
/// فیلدِ برچسب‌دار از دست می‌دهد، و هر یک نامی پایدار برای این نصب است.
fn redact_uuids(line: &str) -> String {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    while i < chars.len() {
        let boundary = i == 0 || !chars[i - 1].is_ascii_hexdigit();
        if boundary {
            let mut j = i;
            let mut ok = true;
            for (g, len) in GROUPS.iter().enumerate() {
                if g > 0 {
                    if j >= chars.len() || chars[j] != '-' {
                        ok = false;
                        break;
                    }
                    j += 1;
                }
                let start = j;
                while j < chars.len() && chars[j].is_ascii_hexdigit() && j - start < *len {
                    j += 1;
                }
                if j - start != *len {
                    ok = false;
                    break;
                }
            }
            // نباید ادامه داشته باشد، وگرنه یک رشتهٔ هگزِ بلندتر است.
            if ok && (j >= chars.len() || !chars[j].is_ascii_hexdigit()) {
                out.push_str("[REDACTED-ID]");
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

// -------------------------------------------------------------------- IPv6

fn is_v6_char(c: char) -> bool {
    c.is_ascii_hexdigit() || c == ':' || c == '.'
}

/// نشانی‌های IPv6 را پیدا و ماسک می‌کند.
///
/// # تنها تصمیمِ سختِ این فایل
///
/// یک الگوی شُل‌تر اینجا **پذیرفتنی نیست**، هرچند این فایل معمولاً پاک‌کردنِ
/// بیش‌ازحد را ترجیح می‌دهد. قاعده‌ای مثل «دو یا چند گروه هگز جدا‌شده با
/// دونقطه» با `10:23:41` هم تطبیق می‌کند، که مُهر زمانِ ابتدای **هر** خط از لاگ
/// این برنامه است. پاک‌کردنِ آن‌ها ترتیب و زمان‌بندی را از خلاصه می‌کَند — همان
/// دو چیزی که مشاور بیش از همه رویشان استدلال می‌کند — و این کار را بی‌صدا
/// می‌کرد و از لاگی که هنوز سالم به نظر می‌رسید مشاورهٔ بدتری می‌ساخت.
///
/// پس: یا هر هشت گروه نوشته شده باشند، یا اجرایی که واقعاً `::` را داشته باشد.
/// هیچ شکل متنیِ معتبری از IPv6 وجود ندارد که هیچ‌کدام نباشد، و هیچ‌یک از دو
/// حالت نمی‌تواند با `HH:MM:SS` تطبیق کند.
fn mask_ipv6_literals(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    while i < chars.len() {
        if is_v6_char(chars[i]) && (i == 0 || !is_v6_char(chars[i - 1])) {
            let mut end = i;
            while end < chars.len() && is_v6_char(chars[end]) {
                end += 1;
            }
            // دونقطهٔ انتهاییِ آویزان (`ipv6:` یا انتهای یک برچسب) بخشی از
            // نشانی نیست.
            let mut candidate_end = end;
            while candidate_end > i && chars[candidate_end - 1] == ':' {
                // ولی `::` انتهایی هست: `2606:4700::` معتبر است.
                if candidate_end >= 2 && chars[candidate_end - 2] == ':' {
                    break;
                }
                candidate_end -= 1;
            }
            let token: String = chars[i..candidate_end].iter().collect();
            if looks_like_ipv6(&token) {
                out.push_str(&mask_ipv6(&token));
                i = candidate_end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// دو شکل پذیرفته‌شده — و فقط همان دو.
fn looks_like_ipv6(token: &str) -> bool {
    if token.len() < 3 || !token.contains(':') {
        return false;
    }
    // شکل فشرده: باید `::` داشته باشد.
    if token.contains("::") {
        // نباید دو بار elision داشته باشد.
        return token.matches("::").count() == 1;
    }
    // شکل کامل: دقیقاً هشت هگزتت، یا شش هگزتت و یک چهارگانِ v4.
    let parts: Vec<&str> = token.split(':').collect();
    if parts.iter().any(|p| p.is_empty() || p.len() > 4) {
        return false;
    }
    if parts.len() == 8 {
        return parts
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_hexdigit()));
    }
    if parts.len() == 7 {
        let (hex, last) = parts.split_at(6);
        return hex.iter().all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
            && looks_like_ipv4(last[0]);
    }
    false
}

/// دو هگزتت اول یک IPv6 عمومی را نگه می‌دارد و بقیه را می‌ریزد.
///
/// دو هگزتت همان /32ی است که به یک ارائه‌دهنده تخصیص می‌یابد، پس `2606:4700:…`
/// هنوز «کلادفلر» خوانده می‌شود و دو اندپوینتِ شکست‌خورده در یک پیشوند هنوز
/// پیدا‌کردنی مرتبط‌اند — که کل ارزش تشخیصیِ یک نشانی در این لاگ است. هر چیزی
/// پس از آن، از جمله شناسهٔ رابط که یکتای این نصب است، می‌رود.
fn mask_ipv6(ip: &str) -> String {
    let lower = ip.to_lowercase();
    // لوپ‌بک و لینک‌لوکال به همان دلیلی که `127.0.0.1` می‌ماند، کامل می‌مانند:
    // لوله‌کشی‌اند، نه هویت.
    if lower == "::" || lower == "::1" || lower.starts_with("fe80:") {
        return ip.to_string();
    }
    // یک نشانی نگاشتهٔ v4، *همان* یک نشانی IPv4 با کلاه v6 است، پس سیاست v4 را
    // می‌گیرد: پیشوند می‌ماند، بخش میزبان می‌رود.
    if lower.contains('.') {
        if let Some(cut) = lower.rfind(':') {
            return format!("{}{}", &lower[..cut + 1], mask_ipv4(&lower[cut + 1..]));
        }
    }
    // Unique-local (`fc00::/7`) فضای نشانی خصوصی است، مثل `10.x`.
    if lower.starts_with("fc") || lower.starts_with("fd") {
        return ip.to_string();
    }
    let parts: Vec<&str> = lower.split(':').collect();
    // یک /32 به دو هگزتتِ پیشروِ واقعی نیاز دارد. نشانی‌ای که از جلو elide
    // می‌کند (`::1234`) پیشوندی برای نگه‌داشتن ندارد، پس کامل ماسک می‌شود و
    // نه نصفه‌توصیف — یک نشانیِ نیمه‌ماسک هنوز یک نشانی است.
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return "[REDACTED-IPV6]".to_string();
    }
    format!("{}:{}:x:x", parts[0], parts[1])
}

// -------------------------------------------------------------------- IPv4

fn looks_like_ipv4(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()))
}

fn mask_ipv4_literals(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    while i < chars.len() {
        let boundary = i == 0 || !(chars[i - 1].is_ascii_digit() || chars[i - 1] == '.');
        if boundary && chars[i].is_ascii_digit() {
            let mut end = i;
            while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.') {
                end += 1;
            }
            // نقطهٔ انتهاییِ آویزان (پایان جمله) بخشی از نشانی نیست.
            let mut candidate_end = end;
            while candidate_end > i && chars[candidate_end - 1] == '.' {
                candidate_end -= 1;
            }
            let token: String = chars[i..candidate_end].iter().collect();
            if looks_like_ipv4(&token) {
                out.push_str(&mask_ipv4(&token));
                i = candidate_end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// بخش میزبانِ یک IPv4 عمومی را ماسک می‌کند، خصوصی/لوپ‌بک را کامل نگه می‌دارد.
///
/// `127.0.0.1`، `10.x`، `192.168.x` و `172.16-31.x` لوله‌کشی‌اند، نه هویت، و
/// همان چیزی هستند که لاگ را خواندنی می‌کنند. هر چیز دیگری `a.b.x.x` می‌شود.
fn mask_ipv4(ip: &str) -> String {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return ip.to_string();
    }
    let mut octets = [0u32; 4];
    for (i, part) in parts.iter().enumerate() {
        match part.parse::<u32>() {
            Ok(v) if v <= 255 => octets[i] = v,
            _ => return ip.to_string(),
        }
    }
    let private = octets[0] == 127
        || octets[0] == 10
        || (octets[0] == 192 && octets[1] == 168)
        || (octets[0] == 172 && (16..=31).contains(&octets[1]))
        || (octets[0] == 169 && octets[1] == 254)
        || octets[0] == 0;
    if private {
        ip.to_string()
    } else {
        format!("{}.{}.x.x", octets[0], octets[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// خطی که سمت اندروید هم کرش داد و هم نشت: هر سه جزء باید بروند و
    /// `172.16.0.2` باید بماند.
    #[test]
    fn the_identity_line_loses_everything_that_names_the_install() {
        let line = "10:23:41 D/core [+] identity ready: device=c79b496b-d123-4e32-851e-7fddc4be55a2 ipv4=172.16.0.2 ipv6=2606:4700:110:8cf5:d172:c495:8919:3df5";
        let out = redact_line(line);
        assert!(out.contains("device=[REDACTED]"), "{out}");
        assert!(!out.contains("c79b496b"), "{out}");
        // RFC1918 لوله‌کشی است و می‌ماند.
        assert!(out.contains("172.16.0.2"), "{out}");
        // پیشوند /32 می‌ماند، شناسهٔ رابط می‌رود.
        assert!(out.contains("2606:4700:x:x"), "{out}");
        assert!(!out.contains("8919"), "{out}");
        // و مُهر زمان — دلیلِ وجودِ سخت‌گیریِ قاعدهٔ IPv6 — دست‌نخورده است.
        assert!(out.starts_with("10:23:41 "), "{out}");
    }

    #[test]
    fn a_timestamp_is_never_mistaken_for_an_address() {
        for line in [
            "10:23:41",
            "00:00:08 connected",
            "23:59:59.123 tick",
            "version 1.2.9",
        ] {
            assert_eq!(redact_line(line), line, "line: {line}");
        }
    }

    #[test]
    fn loopback_and_the_local_socks_ports_survive_intact() {
        let line = "stage 1 engine -> SOCKS5 127.0.0.1:1819 ; stage 2 -> 127.0.0.1:1825";
        assert_eq!(redact_line(line), line);
    }

    #[test]
    fn a_public_v4_keeps_only_its_16() {
        assert_eq!(
            redact_line("endpoint 188.114.99.205:3581"),
            "endpoint 188.114.x.x:3581"
        );
        assert_eq!(redact_line("server 139.162.179.163"), "server 139.162.x.x");
    }

    #[test]
    fn psiphon_json_notices_are_understood() {
        let line = r#"{"noticeType":"Info","sessionId":"5a4c9f1e77","data":{"token":"abc.def"}}"#;
        let out = redact_line(line);
        assert!(!out.contains("5a4c9f1e77"), "{out}");
        assert!(!out.contains("abc.def"), "{out}");
    }

    #[test]
    fn google_keys_die_anywhere_including_the_users_own() {
        let out = redact_line("GET ?key=AIzaSyA1b2C3d4E5f6G7h8I9 failed");
        assert!(!out.contains("AIza"), "{out}");
        // و کلید خودِ کاربر، حتی اگر شکل دیگری داشته باشد، از digest هم می‌رود.
        let d = digest("line with MYSECRETKEY in it", "MYSECRETKEY", 10);
        assert!(!d.contains("MYSECRETKEY"), "{d}");
    }

    #[test]
    fn a_label_is_a_whole_word_not_a_substring() {
        // `keepalive` و `monkey` نباید به‌خاطر «key» بیفتند.
        let line = "keepalive 25 monkey business";
        assert_eq!(redact_line(line), line);
    }

    #[test]
    fn an_ipv4_mapped_v6_address_is_handled_as_one_address() {
        // اگر IPv4 اول اجرا می‌شد، `.0.113.9` جامی‌ماند: سه اوکتت از یک نشانی
        // عمومی، منتشرشده توسط قاعده‌ای که برای ماسک‌کردنش وجود دارد.
        let out = redact_line("peer ::ffff:203.0.113.9 down");
        assert!(out.contains("::ffff:203.0.x.x"), "{out}");
        assert!(!out.contains("113.9"), "{out}");
    }

    #[test]
    fn an_address_that_elides_from_the_front_is_masked_whole() {
        let out = redact_line("addr 2606::1234 seen");
        assert!(
            out.contains("2606:") || out.contains("[REDACTED-IPV6]"),
            "{out}"
        );
        let out2 = redact_line("addr ::1234:5678 seen");
        assert!(out2.contains("[REDACTED-IPV6]"), "{out2}");
    }

    #[test]
    fn the_digest_takes_the_tail_and_respects_the_cap() {
        let log: String = (0..500).map(|i| format!("line {i}\n")).collect();
        let d = digest(&log, "", 5);
        assert!(d.contains("line 499"));
        assert!(!d.contains("line 100"));

        let huge: String = (0..20_000).map(|_| "0123456789").collect::<String>();
        let capped = digest(&huge, "", 10);
        assert_eq!(capped.chars().count(), MAX_DIGEST_CHARS);
    }
}
