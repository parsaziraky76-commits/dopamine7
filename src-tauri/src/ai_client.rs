//! پورت ۱:۱ از `ai/GeminiClient.kt` — کلاینت نازک و بی‌وابستگیِ REST API جمینای.
//!
//! JSON با `serde_json` ساخته و پارس می‌شود، که از قبل در این مخزن هست (پروفایل
//! و snapshot با همان سریالایز می‌شوند)، پس قابلیت‌های هوش مصنوعی **صفر**
//! وابستگی تازه به نصابی اضافه می‌کنند که کاربرانش هر دلیلی دارند نگران محتوایش
//! باشند. در سمت اندروید همین نقش را `org.json` پلتفرم بازی می‌کرد.
//!
//! هر فراخوانی بلوکه‌کننده است و **باید** بیرون از رشتهٔ رابط کاربری اجرا شود؛
//! [`crate::ai_session`] همه را روی رشتهٔ کارگر خودش می‌برد.

use crate::ai_http;
use crate::ai_model_policy as policy;
use crate::log::DiagnosticsLog;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

/// یک مدل جمینای، همان‌طور که کلید خودِ کاربر مجاز به دیدنش است.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiModel {
    /// شناسهٔ خالی، مثل `gemini-3.8-flash` (پیشوند `models/` کنده شده).
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub input_token_limit: u32,
    pub output_token_limit: u32,
    /// وقتی true است این مدل می‌تواند به `generateContent` پاسخ دهد.
    pub chat_capable: bool,
}

/// یک نوبت از یک گفت‌وگو.
#[derive(Debug, Clone)]
pub struct GeminiTurn {
    pub from_user: bool,
    pub text: String,
}

/// چه چیزی غلط شد، در همان درشت‌دانگی که رابط کاربری واقعاً به آن واکنش می‌دهد.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AiErrorKind {
    /// کلید رد شد (۴۰۱/۴۰۳، یا «API key not valid» خودِ گوگل).
    BadKey,
    /// سهمیه یا محدودیت نرخ (۴۲۹).
    RateLimit,
    /// شناسهٔ مدل برای این کلید ناشناخته است (۴۰۴).
    NoSuchModel,
    /// درخواست هرگز به گوگل نرسید: پروکسی رد کرد، TLS شکست، تایم‌اوت.
    Transport,
    /// به گوگل رسیدیم و او سمت خودش شکست (۵xx).
    ///
    /// از [`AiErrorKind::Transport`] جدا شده چون این دو به کلمات مخالف نیاز
    /// دارند. یک ۵۰۰ به کاربر به‌عنوان «نتوانستیم از تونل به گوگل برسیم» گزارش
    /// می‌شد، که مردم را می‌فرستاد تونلی را دوباره بررسی کنند که بی‌عیب کار
    /// می‌کرد — درخواست تا خودِ گوگل رفته و برگشته بود. این هم تنها ردهٔ شکستی
    /// است که یک retry ساده درستش می‌کند، و همین است که «دوباره تلاش کن» همیشه
    /// کار می‌کرد و هیچ چیز دیگری نه.
    ServerError,
    /// گوگل جواب داد، با چیزی که این کلاینت نمی‌توانست استفاده کند.
    Protocol,
    /// پاسخ وسط تولید بریده شد (`finishReason: MAX_TOKENS`).
    ///
    /// ردهٔ خودش را دارد چون شکستِ پشتِ یک پاسخ چتِ نصفه و یک پاسخ مشاور که
    /// به‌عنوان JSON پارس نمی‌شود، همین است: متن خراب نیست، **ناقص** است، و
    /// راه‌حلش بودجهٔ خروجی بزرگ‌تر است نه مدل دیگر یا کلید دیگر.
    Truncated,
    /// گوگل بنا به دلایل ایمنی از پاسخ‌دادن خودداری کرد.
    Blocked,
}

impl AiErrorKind {
    /// کدام شکست‌ها را یک تلاش دوم می‌تواند به‌طور موجه درست کند.
    ///
    /// کلیدِ ردشده، مدل ناشناخته و ردِ ایمنی جبری‌اند: تکرارشان وقت و سهمیهٔ
    /// کاربر را هدر می‌دهد تا به همان پاسخ برسد. یک ۵xx، یک محدودیت نرخ و یک
    /// سوکتِ افتاده نه.
    fn worth_retrying(self) -> bool {
        matches!(
            self,
            AiErrorKind::ServerError | AiErrorKind::RateLimit | AiErrorKind::Transport
        )
    }
}

/// خطایی که همیشه یک جملهٔ قابل‌نمایش دارد.
///
/// یک نتیجهٔ ساختاریافته و نه صرفاً `anyhow`: هر شکستی اینجا باید در نهایت به
/// یک جمله روی صفحه تبدیل شود. پرت‌کردنِ خطا، این ترجمه را به شش نقطهٔ فراخوانی
/// مختلف هل می‌داد و تضمین می‌کرد یکی‌شان یک `SSLPeerUnverifiedException` خام را
/// به کسی نشان دهد که فقط می‌خواست بداند MTU چیست.
#[derive(Debug, Clone)]
pub struct AiError {
    pub message: String,
    pub kind: AiErrorKind,
    /// انتظارِ پیشنهادیِ سرور، وقتی شکست یکی همراه داشت. backoff را می‌راند.
    pub retry_after_seconds: Option<f64>,
}

impl AiError {
    fn new(message: impl Into<String>, kind: AiErrorKind) -> Self {
        Self {
            message: message.into(),
            kind,
            retry_after_seconds: None,
        }
    }
}

pub type AiResult<T> = Result<T, AiError>;

const API: &str = "/v1beta";
const MAX_ATTEMPTS: u32 = 3;
const BASE_BACKOFF_MS: u64 = 700;
const MAX_BACKOFF_MS: u64 = 6_000;
const MAX_MODEL_PAGES: u32 = 5;
/// یک نشستِ HTTP کامل. طولانی چون درخواست از **دو** تونل رد می‌شود.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// مدل‌هایی که *این* کلید می‌تواند استفاده کند را فهرست می‌کند.
///
/// کل نکتهٔ این قابلیت: یک کلید رایگان، یک کلید با صورت‌حساب فعال و یک کلید از
/// منطقه‌ای که مدلی در آن عرضه نشده، سه فهرست متفاوت می‌بینند؛ پس یک انتخابگرِ
/// ثابت‌نوشته مدل‌هایی را پیشنهاد می‌کرد که برای نیمی از کاربران ۴۰۴ می‌دهند. ما
/// از خودِ کلید می‌پرسیم چه می‌تواند بکند.
pub fn list_models(api_key: &str, socks_port: u16) -> AiResult<Vec<GeminiModel>> {
    let mut collected: Vec<GeminiModel> = Vec::new();
    let mut page_token: Option<String> = None;
    let mut page = 0u32;
    loop {
        let suffix = match &page_token {
            Some(t) if !t.is_empty() => format!("&pageToken={t}"),
            _ => String::new(),
        };
        let body = call(
            "GET",
            &format!("{API}/models?pageSize=200{suffix}"),
            socks_port,
            api_key,
            None,
        )?;
        let json: Value = serde_json::from_str(&body).map_err(|_| {
            AiError::new("Google's reply was not valid JSON.", AiErrorKind::Protocol)
        })?;
        if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
            for item in models {
                collected.push(to_model(item));
            }
        }
        page_token = json
            .get("nextPageToken")
            .and_then(|t| t.as_str())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string());
        page += 1;
        // پیمایش را محدود کن: یک nextPageToken فراری نباید «بازکردن انتخابگر
        // مدل» را به یک حلقهٔ بی‌کران روی یک اتصال حجمی تبدیل کند.
        if page_token.is_none() || page >= MAX_MODEL_PAGES {
            break;
        }
    }

    // فهرست مجاز **اینجا** اعمال می‌شود و نه در انتخابگر: شناسه‌ای که هرگز وارد
    // برنامه نشود، نمی‌تواند انتخاب، کَش یا فرستاده شود.
    let chat_capable: Vec<GeminiModel> = collected
        .iter()
        .filter(|m| m.chat_capable)
        .cloned()
        .collect();
    let offered = policy::filter(&chat_capable);
    DiagnosticsLog::i(
        "ai",
        &format!(
            "models: {} returned by the key, {} offered after the allow-list",
            collected.len(),
            offered.len()
        ),
    );
    Ok(offered)
}

/// یک گفت‌وگو را می‌فرستد و پاسخ مدل را به‌صورت متن ساده برمی‌گرداند.
///
/// * `system` — دستور سیستمی؛ شخصیت برنامه، زبانی که باید در آن پاسخ دهد و
///   قرارداد ماشین‌خوانِ تغییر تنظیمات را حمل می‌کند ([`crate::ai_prompts`]).
/// * `history` — نوبت‌های پیشین، قدیمی‌ترین اول. کامل فرستاده می‌شود چون REST
///   API بی‌حالت است؛ هیچ گفت‌وگوی سمت‌سروری وجود ندارد که به آن اضافه شود.
/// * `json_output` — درخواست `application/json` به‌جای نثر.
///
///   مشاور از این استفاده می‌کند، که پاسخش پارس و به پیکربندی تبدیل می‌شود. با
///   این تنظیم، مدل نمی‌تواند شیء را در یک code fence بپیچد یا با یک جمله
///   معرفی‌اش کند — که بیشترِ چیزی است که استخراج‌کنندهٔ سهل‌گیرِ
///   [`crate::ai_prompts`] برای جان‌سالم‌بردن از آن نوشته شده بود.
#[allow(clippy::too_many_arguments)]
pub fn generate(
    api_key: &str,
    socks_port: u16,
    model: &str,
    system: &str,
    history: &[GeminiTurn],
    temperature: f64,
    max_output_tokens: u32,
    json_output: bool,
) -> AiResult<String> {
    if model.trim().is_empty() {
        return Err(AiError::new(
            "No Gemini model selected.",
            AiErrorKind::NoSuchModel,
        ));
    }
    if !policy::is_allowed(model) {
        // کمربند و بند شلوار روی فهرست مجاز: یک شناسهٔ مدل می‌تواند از فایل
        // تنظیماتی که بیلد قدیمی‌تری نوشته هم برسد، و آن مسیر از کشف رد نمی‌شود.
        return Err(AiError::new(
            format!("Model \"{model}\" is not one of the models this app supports."),
            AiErrorKind::NoSuchModel,
        ));
    }

    let contents: Vec<Value> = history
        .iter()
        .map(|turn| {
            json!({
                "role": if turn.from_user { "user" } else { "model" },
                "parts": [{ "text": turn.text }],
            })
        })
        .collect();

    let mut generation_config = json!({
        "temperature": temperature,
        "maxOutputTokens": max_output_tokens,
        // ------------------------------------------------------------------
        // ریشهٔ پاسخ‌های بریده / پارس‌نشدنی.
        //
        // maxOutputTokens بودجه‌ای برای **هر چیزی** است که مدل تولید می‌کند، و
        // روی مدلی که توان استدلال دارد، توکن‌های استدلال **اول** از همان بودجه
        // خرج می‌شوند. مشاور ۱۴۰۰ توکن می‌خواست با یک خلاصهٔ لاگ در پرامپت، مدل
        // همه را صرف فکرکردن می‌کرد، و پاسخ دیدنی خالی یا وسط شیء بریده
        // برمی‌گشت — دقیقاً همان جفتِ «۲۰۰ OK» و «پاسخ مشاور به‌عنوان JSON پارس
        // نشد» در لاگ میدانی.
        //
        // thinkingBudget = 0 استدلال را خاموش می‌کند تا تمام بودجه به پاسخ
        // برسد. برای این برنامه معاملهٔ درستی است: هر پرامپت اینجا افق‌کوتاه است
        // (یک تنظیم را توضیح بده، یک لاگ را روی فهرستی ثابت از پیچ‌ها بنگار) و
        // هیچ‌کدام از یک پاس استدلالی که به قیمت پاسخ تمام شود سود نمی‌برند. یک
        // منبع بی‌کران و بی‌حساب از تأخیر را هم از درخواستی که از دو تونل رد
        // می‌شود برمی‌دارد.
        //
        // بی‌قید‌و‌شرط فرستاده می‌شود: مدل‌هایی که thinking را پشتیبانی نمی‌کنند،
        // یک عضو ناشناخته در generationConfig را نادیده می‌گیرند و درخواست را رد
        // نمی‌کنند.
        "thinkingConfig": { "thinkingBudget": 0 },
    });
    if json_output {
        generation_config["responseMimeType"] = json!("application/json");
    }

    let payload = json!({
        "systemInstruction": { "parts": [{ "text": system }] },
        "contents": contents,
        "generationConfig": generation_config,
    });

    let path = format!("{API}/models/{}:generateContent", policy::normalise(model));
    let body = call(
        "POST",
        &path,
        socks_port,
        api_key,
        Some(&payload.to_string()),
    )?;
    let json: Value = serde_json::from_str(&body)
        .map_err(|_| AiError::new("Google's reply was not valid JSON.", AiErrorKind::Protocol))?;

    // پیش از تولید رد شد: هیچ کاندیدی برای خواندن نیست و دلیلش جای کاملاً
    // دیگری زندگی می‌کند.
    if let Some(reason) = json
        .get("promptFeedback")
        .and_then(|f| f.get("blockReason"))
        .and_then(|r| r.as_str())
        .filter(|r| !r.is_empty() && *r != "BLOCK_REASON_UNSPECIFIED")
    {
        return Err(AiError::new(reason, AiErrorKind::Blocked));
    }

    let candidates = json.get("candidates").and_then(|c| c.as_array());
    let Some(candidate) = candidates.and_then(|c| c.first()) else {
        return Err(AiError::new(
            "The model returned no answer.",
            AiErrorKind::Blocked,
        ));
    };

    let mut text = String::new();
    if let Some(parts) = candidate
        .get("content")
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
    {
        for part in parts {
            // بخش‌های استدلال با `thought: true` علامت‌گذاری می‌شوند و پاسخ
            // **نیستند**. به‌هم‌چسباندنشان پاسخ‌هایی می‌ساخت که با حرف‌زدنِ مدل با
            // خودش شروع می‌شدند، و JSONی که نثر جلویش بود. thinkingBudget=0
            // باید یعنی هیچ‌کدام وجود ندارند؛ این گاردِ مدل‌هایی است که نادیده‌اش
            // می‌گیرند.
            if part
                .get("thought")
                .and_then(|t| t.as_bool())
                .unwrap_or(false)
            {
                continue;
            }
            if let Some(chunk) = part.get("text").and_then(|t| t.as_str()) {
                text.push_str(chunk);
            }
        }
    }

    let finish = candidate
        .get("finishReason")
        .and_then(|f| f.as_str())
        .unwrap_or("");
    if text.trim().is_empty() {
        let kind = match finish {
            "SAFETY" | "PROHIBITED_CONTENT" => AiErrorKind::Blocked,
            "MAX_TOKENS" => AiErrorKind::Truncated,
            _ => AiErrorKind::Protocol,
        };
        let message = if finish.is_empty() {
            "The model returned an empty answer."
        } else {
            finish
        };
        return Err(AiError::new(message, kind));
    }
    // پاسخ داد، ولی بریده شد. به‌عنوان خطا گزارش می‌شود و نه موفقیت: نیم‌جمله در
    // یک حبابِ چت و نیم‌شیء در مشاور، هر دو شکست‌اند، و فراخوان می‌تواند با
    // بودجهٔ بزرگ‌تر تلاش کند چون حالا می‌داند این کدام شکست است. متن نیمه هم
    // سوار پیام می‌شود تا اگر خواست همان را نشان دهد.
    if finish == "MAX_TOKENS" {
        DiagnosticsLog::w(
            "ai",
            &format!("answer hit MAX_TOKENS after {} chars", text.len()),
        );
        return Err(AiError::new(text, AiErrorKind::Truncated));
    }
    Ok(text)
}

// ---- لوله‌کشی --------------------------------------------------------------

/// یک تبادل HTTP، با هر شکستی از قبل ترجمه‌شده.
///
/// به آنچه لاگ **نمی‌شود** توجه کنید: کلید، رشتهٔ پرسمانِ مسیر و بدنهٔ درخواست.
/// فقط متد، بخشِ مدل‌دارِ مسیر و کد وضعیت به لاگ تشخیصی می‌روند، چون آن لاگ
/// صادرشدنی است و کاربران آن را در گزارش‌های عمومی می‌چسبانند.
fn call(
    method: &str,
    path: &str,
    socks_port: u16,
    api_key: &str,
    json_body: Option<&str>,
) -> AiResult<String> {
    if api_key.trim().is_empty() {
        return Err(AiError::new("No API key stored.", AiErrorKind::BadKey));
    }
    let mut attempt = 0u32;
    let mut last = AiError::new("No attempt was made.", AiErrorKind::Transport);
    while attempt < MAX_ATTEMPTS {
        attempt += 1;
        match attempt_call(method, path, socks_port, api_key, json_body) {
            Ok(body) => return Ok(body),
            Err(error) => {
                let retryable = error.kind.worth_retrying();
                let wait = backoff_millis(attempt, error.retry_after_seconds);
                last = error;
                if attempt >= MAX_ATTEMPTS || !retryable {
                    break;
                }
                DiagnosticsLog::i(
                    "ai",
                    &format!(
                        "{:?} on attempt {attempt}/{MAX_ATTEMPTS}; retrying in {wait}ms",
                        last.kind
                    ),
                );
                // خوابِ بلوکه‌کننده اینجا درست است: هر فراخوانِ این تابع روی
                // رشتهٔ کارگرِ ai_session است و خواندن‌های سوکت در دو طرفش خیلی
                // بیشتر بلوکه می‌شوند. خوابیدن، کل سیاست retry را در یک جای
                // خواندنی نگه می‌دارد.
                std::thread::sleep(Duration::from_millis(wait));
            }
        }
    }
    Err(last)
}

/// یک تبادل HTTP، با هر شکستی از قبل طبقه‌بندی‌شده.
fn attempt_call(
    method: &str,
    path: &str,
    socks_port: u16,
    api_key: &str,
    json_body: Option<&str>,
) -> AiResult<String> {
    let response = match ai_http::request(
        method,
        path,
        socks_port,
        api_key,
        json_body,
        REQUEST_TIMEOUT,
    ) {
        Ok(r) => r,
        Err(failure) => {
            DiagnosticsLog::w(
                "ai",
                &format!("request failed via 127.0.0.1:{socks_port}: {failure}"),
            );
            return Err(AiError::new(failure.to_string(), AiErrorKind::Transport));
        }
    };
    let logged_path = path.split('?').next().unwrap_or(path);
    DiagnosticsLog::i(
        "ai",
        &format!("{method} {logged_path} -> {}", response.code),
    );
    if response.ok() {
        return Ok(response.body);
    }
    if response.code == 0 {
        return Err(AiError::new(
            "No response through the tunnel.",
            AiErrorKind::Transport,
        ));
    }

    let parsed: Option<Value> = serde_json::from_str(&response.body).ok();
    let error_obj = parsed.as_ref().and_then(|v| v.get("error"));
    let api_message = error_obj
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .filter(|m| !m.is_empty())
        .map(|m| m.to_string());

    let mentions_key = api_message
        .as_deref()
        .map(|m| m.to_lowercase().contains("api key"))
        .unwrap_or(false);
    let kind = if response.code == 400 && mentions_key {
        AiErrorKind::BadKey
    } else if response.code == 401 || response.code == 403 {
        AiErrorKind::BadKey
    } else if response.code == 429 {
        AiErrorKind::RateLimit
    } else if response.code == 404 {
        AiErrorKind::NoSuchModel
    } else if response.code >= 500 {
        // ۵xx یعنی گوگل شکست خورده، **نه** تونل. رجوع به AiErrorKind::ServerError.
        AiErrorKind::ServerError
    } else {
        AiErrorKind::Protocol
    };

    Err(AiError {
        message: api_message.unwrap_or_else(|| format!("HTTP {}", response.code)),
        kind,
        retry_after_seconds: response
            .retry_after_seconds
            .or_else(|| retry_delay_from_error(error_obj)),
    })
}

/// `RetryInfo.retryDelay` خودِ گوگل را از بدنهٔ خطا می‌خواند.
///
/// بدنهٔ یک ۴۲۹ حاوی `details[].retryDelay: "2.379075806s"` است، یعنی سرور
/// دقیقاً می‌گوید چقدر صبر کنیم. لاگ میدانی پنج تا ۴۲۹ در ده ثانیه نشان می‌داد
/// چون هیچ‌کس آن را نخوانده بود. احترام‌گذاشتن به آن هم از یک backoff ثابت
/// سریع‌تر است و هم تفاوتِ «یک بار صبرکردن» و «سخت‌تر محدود‌شدن به‌خاطر کوبیدن».
fn retry_delay_from_error(error: Option<&Value>) -> Option<f64> {
    let details = error?.get("details")?.as_array()?;
    for item in details {
        let raw = item
            .get("retryDelay")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .trim();
        if raw.is_empty() {
            continue;
        }
        if let Ok(seconds) = raw.trim_end_matches('s').parse::<f64>() {
            if seconds >= 0.0 {
                return Some(seconds);
            }
        }
    }
    None
}

/// چقدر پیش از تلاش `attempt + 1` صبر شود.
///
/// عددِ خودِ سرور وقتی فرستاده باشد برنده است؛ وگرنه نمایی با یک سقف. سقف از
/// شکل منحنی مهم‌تر است: این کد وقتی اجرا می‌شود که کاربر به یک اسپینر نگاه
/// می‌کند، پس سیاستی که اجازه دارد یک دقیقه صبر کند، سیاستی است که شبیه هنگ است.
fn backoff_millis(attempt: u32, retry_after_seconds: Option<f64>) -> u64 {
    let suggested = retry_after_seconds
        .map(|s| (s * 1000.0) as u64)
        .unwrap_or(0);
    let exponential = BASE_BACKOFF_MS << (attempt - 1);
    suggested
        .max(exponential)
        .clamp(BASE_BACKOFF_MS, MAX_BACKOFF_MS)
}

fn to_model(item: &Value) -> GeminiModel {
    let raw = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let id = policy::normalise(raw);
    let supports_generate = item
        .get("supportedGenerationMethods")
        .and_then(|m| m.as_array())
        .map(|arr| arr.iter().any(|m| m.as_str() == Some("generateContent")))
        .unwrap_or(false);
    let display = item
        .get("displayName")
        .and_then(|d| d.as_str())
        .filter(|d| !d.is_empty())
        .unwrap_or(&id)
        .to_string();
    GeminiModel {
        id,
        display_name: display,
        description: item
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string(),
        input_token_limit: item
            .get("inputTokenLimit")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        output_token_limit: item
            .get("outputTokenLimit")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        chat_capable: supports_generate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_servers_own_delay_beats_our_curve_but_not_the_ceiling() {
        // بدون پیشنهاد سرور: نمایی.
        assert_eq!(backoff_millis(1, None), 700);
        assert_eq!(backoff_millis(2, None), 1_400);
        // پیشنهاد سرور بزرگ‌تر است، پس برنده می‌شود.
        assert_eq!(backoff_millis(1, Some(2.379_075_806)), 2_379);
        // ولی سقف، انتظارِ شبیه‌هنگ را می‌بُرد.
        assert_eq!(backoff_millis(1, Some(120.0)), MAX_BACKOFF_MS);
    }

    #[test]
    fn reads_google_retry_info_out_of_an_error_body() {
        let body: Value = serde_json::from_str(
            r#"{"error":{"code":429,"details":[{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"2.5s"}]}}"#,
        )
        .unwrap();
        assert_eq!(retry_delay_from_error(body.get("error")), Some(2.5));
    }

    #[test]
    fn only_transient_failures_are_retried() {
        assert!(AiErrorKind::ServerError.worth_retrying());
        assert!(AiErrorKind::RateLimit.worth_retrying());
        assert!(AiErrorKind::Transport.worth_retrying());
        // این‌ها جبری‌اند: تکرارشان فقط سهمیه می‌سوزاند.
        assert!(!AiErrorKind::BadKey.worth_retrying());
        assert!(!AiErrorKind::NoSuchModel.worth_retrying());
        assert!(!AiErrorKind::Blocked.worth_retrying());
        assert!(!AiErrorKind::Truncated.worth_retrying());
    }

    #[test]
    fn a_model_row_needs_generate_content_to_be_chat_capable() {
        let item: Value = serde_json::from_str(
            r#"{"name":"models/gemini-3.8-flash","displayName":"Flash","supportedGenerationMethods":["generateContent","countTokens"]}"#,
        )
        .unwrap();
        let model = to_model(&item);
        assert_eq!(model.id, "gemini-3.8-flash");
        assert!(model.chat_capable);

        let embed: Value = serde_json::from_str(
            r#"{"name":"models/text-embedding-004","supportedGenerationMethods":["embedContent"]}"#,
        )
        .unwrap();
        assert!(!to_model(&embed).chat_capable);
    }
}
