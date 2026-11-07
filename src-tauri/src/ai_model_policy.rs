//! پورت ۱:۱ از `ai/AiModelPolicy.kt` — فهرست مجاز مدل‌ها، و تنها جایی که
//! ترتیب مدل‌ها تصمیم گرفته می‌شود.
//!
//! # چرا فهرست مجاز و نه یک فیلتر در رابط کاربری
//!
//! `models.list` روی یک کلید Gemini هر چیزی را که کلید *مستحقِ* دیدنش است
//! برمی‌گرداند، که در عمل بیش از سی مورد است: مدل‌های تصویر، ویدیو، گفتار و
//! TTS، مدل‌های embedding، و یک دنبالهٔ بلند از گونه‌های `-pro` و `-thinking` که
//! روی تیر رایگان — همان کلیدی که تقریباً هر کاربر این برنامه دارد — به اولین
//! پرسش واقعی با ۴۲۹ جواب می‌دهند. یک انتخابگر ساخته‌شده از آن فهرست، فهرستی از
//! راه‌های شکست‌خوردنِ این قابلیت است و کاربر باید با سوزاندن سهمیه کشف کند
//! کدام کدام است.
//!
//! پس مجموعه همین‌جا، در لایهٔ داده، ثابت است و روی **هر** مسیری که می‌تواند یک
//! شناسهٔ مدل را جلوی کاربر یا داخل یک درخواست بگذارد اعمال می‌شود:
//!
//!  1. پاسخ تازهٔ `models.list` ([`filter`])،
//!  2. فهرست کَش‌شده که از انبار تنظیمات در استارتاپ پخش می‌شود ([`filter_ids`])،
//!  3. انتخاب مدل پیش‌فرض ([`pick_default`])،
//!  4. شناسه‌ای که واقعاً به `generateContent` فرستاده می‌شود ([`is_allowed`]).
//!
//! فیلترکردن فقط در گام ۱ همان باگی است که این فایل برای جلوگیری از آن وجود
//! دارد: نوسازی فهرست، یا حتی راه‌اندازی مجدد با یک فهرستِ کَش‌شده از بیلدی
//! قدیمی‌تر، مدل‌های فیلترشده را مستقیم به انتخابگر برمی‌گرداند.
//!
//! # ترتیب
//!
//! [`ALLOWED`] از تازه‌ترین به قدیمی‌ترین نوشته شده و همان ترتیب، ترتیبِ نمایش
//! است — یک جست‌وجوی رتبه، نه مرتب‌سازی رشته‌ای. مرتب‌کردن شناسه‌ها به‌صورت متن،
//! `gemini-3.1` را بعد از `gemini-3.5` و `gemini-flash-lite-latest` را پیش از
//! هر انتشار شماره‌داری می‌گذارد، چون از نظر لغوی `1` < `5` و `f` > `3` است.
//! پارس‌کردنِ نسخه‌آگاهِ نام‌گذاری یک فروشنده حدسی است که می‌پوسد؛ یک فهرست صریح
//! تصمیمی است که نمی‌پوسد.

use crate::ai_client::GeminiModel;

/// تنها مدل‌هایی که برنامه پیشنهاد می‌کند، تازه‌ترین اول.
///
/// هر یک از این‌ها یک مدل ردهٔ Flash است: سریع، ارزان و پاسخ‌دادنی روی یک کلید
/// رایگان.
pub const ALLOWED: [&str; 6] = [
    "gemini-3.8-flash",
    // 1.4.0 (موبایل 1.3.1): 3.7-flash در ۱۳ اوت ۲۰۲۶ منتشر شد و در این فهرست نبود؛
    // کلیدی که آن را می‌دید، پایین‌تر از مدل‌های قدیمی‌تر رتبه می‌گرفت.
    "gemini-3.7-flash",
    "gemini-3.5-flash-lite",
    "gemini-3.1-flash-lite",
    "gemini-3.1-flash-lite-preview",
    "gemini-flash-lite-latest",
];

/// پیشوند `models/` که REST API استفاده می‌کند را می‌کند و فاصله‌ها را می‌برد.
pub fn normalise(raw_id: &str) -> String {
    raw_id
        .trim()
        .trim_start_matches("models/")
        .trim()
        .to_string()
}

fn rank(raw_id: &str) -> Option<usize> {
    let id = normalise(raw_id);
    ALLOWED.iter().position(|allowed| *allowed == id)
}

pub fn is_allowed(raw_id: &str) -> bool {
    rank(raw_id).is_some()
}

/// جای نمایش، از ۱ — شمارهٔ ردیفی که در انتخابگر دیده می‌شود.
///
/// از [`ALLOWED`] مشتق می‌شود و نه از جای مدل در فهرستی که رندر می‌شود، تا مدل
/// شمارهٔ ۳ همان شمارهٔ ۳ بماند، چه کلید کاربر شمارهٔ ۲ را ببیند چه نبیند. عددی
/// که با ناموجودشدنِ یک مدلِ بی‌ربط جابه‌جا شود، از نبودِ عدد بدتر است.
pub fn display_number(raw_id: &str) -> usize {
    rank(raw_id).map(|r| r + 1).unwrap_or(0)
}

/// شناسه‌های خالی را فیلتر و مرتب می‌کند — مسیرِ فهرستِ کَش‌شده.
pub fn filter_ids(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        let id = normalise(id);
        if is_allowed(&id) && !out.contains(&id) {
            out.push(id);
        }
    }
    out.sort_by_key(|id| rank(id).unwrap_or(usize::MAX));
    out
}

/// مدل‌های کشف‌شده را فیلتر و مرتب می‌کند — مسیرِ کشفِ تازه.
pub fn filter(models: &[GeminiModel]) -> Vec<GeminiModel> {
    let mut out: Vec<GeminiModel> = Vec::new();
    for model in models {
        let id = normalise(&model.id);
        if !is_allowed(&id) || out.iter().any(|m| m.id == id) {
            continue;
        }
        let mut copy = model.clone();
        copy.id = id;
        out.push(copy);
    }
    out.sort_by_key(|m| rank(&m.id).unwrap_or(usize::MAX));
    out
}

/// مدلی که وقتی کاربر انتخابی نکرده — یا مدلی انتخاب کرده که دیگر پیشنهاد
/// نمی‌شود — استفاده شود: صرفاً تازه‌ترین مدل مجازی که کلید می‌بیند.
///
/// هیچ اکتشافِ زیررشته‌ای. نردبان قدیمیِ `contains("flash")` برای سروکله‌زدن با
/// یک فهرست بی‌کران وجود داشت؛ با فهرستی ثابت از پنج مورد، «اولی» هم درست است و
/// هم قابل توضیح.
/// ۱.۴.۰-r5 (پورت از `AiModelPolicy.DEFAULT_PREFERENCE`): پیش‌فرض دیگر «تازه‌ترین
/// مدل» نیست.
///
/// تازه‌ترین مدل‌های Flash سریع‌ترند، ولی روی کلیدِ رایگان کمترین سهمیهٔ روزانه را
/// دارند و شکایتِ میدانی این بود که چت وسطِ روز با ۴۲۹ از کار می‌افتاد. Gemini 3.1
/// Flash-Lite بالاترین سقفِ روزانهٔ رایگان را میان مدل‌های مجاز دارد، پس کلیدِ
/// تازه با آن شروع می‌کند. مدل‌های تازه‌تر یک کلیک دورتر در انتخابگر می‌مانند.
///
/// ترتیبِ جایگزین: اگر کلید 3.1 Flash-Lite را نبیند، نسخهٔ preview و نامِ مستعارِ
/// `flash-lite-latest` بعدی‌اند و فقط پس از آن‌ها ترتیبِ نمایش.
pub const DEFAULT_PREFERENCE: [&str; 3] = [
    "gemini-3.1-flash-lite",
    "gemini-3.1-flash-lite-preview",
    "gemini-flash-lite-latest",
];

/// مدلی که کلیدِ تازه، اگر ببیندش، با آن شروع می‌کند.
pub const DEFAULT_MODEL: &str = "gemini-3.1-flash-lite";

/// آیا این همان مدلِ پیش‌فرضِ توصیه‌شده است؟ (برای برچسبِ انتخابگر)
pub fn is_default(raw_id: &str) -> bool {
    normalise(raw_id) == DEFAULT_MODEL
}

/// انتخابِ پیش‌فرض روی شناسه‌های خام — مسیرِ فهرستِ کَش‌شده در استارتاپ.
pub fn pick_default_id(ids: &[String]) -> Option<String> {
    let usable = filter_ids(ids);
    DEFAULT_PREFERENCE
        .iter()
        .find(|preferred| usable.iter().any(|id| id == *preferred))
        .map(|id| id.to_string())
        .or_else(|| usable.first().cloned())
}

/// مدلی که وقتی کاربر چیزی انتخاب نکرده (یا انتخابش دیگر پیشنهاد نمی‌شود)
/// به کار می‌رود: اول [`DEFAULT_PREFERENCE`]، بعد اولین مدلِ مجاز.
pub fn pick_default(models: &[GeminiModel]) -> Option<String> {
    let ids: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
    pick_default_id(&ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str) -> GeminiModel {
        GeminiModel {
            id: id.to_string(),
            display_name: id.to_string(),
            description: String::new(),
            input_token_limit: 0,
            output_token_limit: 0,
            chat_capable: true,
        }
    }

    #[test]
    fn strips_the_rest_api_prefix() {
        assert_eq!(normalise("  models/gemini-3.8-flash "), "gemini-3.8-flash");
        assert!(is_allowed("models/gemini-3.8-flash"));
    }

    #[test]
    fn order_is_the_declared_one_not_alphabetical() {
        // مرتب‌سازی متنی این را برعکس می‌کرد: `3.1` پیش از `3.5` و
        // `gemini-flash-lite-latest` پیش از همهٔ شماره‌دارها.
        let models = [
            model("gemini-flash-lite-latest"),
            model("gemini-3.1-flash-lite"),
            model("gemini-3.8-flash"),
        ];
        let ids: Vec<String> = filter(&models).into_iter().map(|m| m.id).collect();
        assert_eq!(
            ids,
            [
                "gemini-3.8-flash",
                "gemini-3.1-flash-lite",
                "gemini-flash-lite-latest"
            ]
        );
        // ۱.۴.۰-r5: ترتیبِ نمایش همان است، ولی پیش‌فرض Flash-Lite است.
        assert_eq!(pick_default(&models).unwrap(), "gemini-3.1-flash-lite");
    }

    #[test]
    fn the_default_falls_back_in_preference_order_then_display_order() {
        let only_alias = [model("gemini-3.8-flash"), model("gemini-flash-lite-latest")];
        assert_eq!(
            pick_default(&only_alias).unwrap(),
            "gemini-flash-lite-latest"
        );
        let no_lite = [model("gemini-3.7-flash"), model("gemini-3.8-flash")];
        assert_eq!(pick_default(&no_lite).unwrap(), "gemini-3.8-flash");
        assert!(pick_default(&[]).is_none());
        let cached = vec![
            "models/gemini-3.1-flash-lite".to_string(),
            "gemini-3.8-flash".to_string(),
        ];
        assert_eq!(pick_default_id(&cached).unwrap(), "gemini-3.1-flash-lite");
        assert!(is_default("models/gemini-3.1-flash-lite"));
        assert!(!is_default("gemini-3.8-flash"));
    }

    #[test]
    fn a_cached_list_from_an_older_build_is_filtered_too() {
        // همان باگی که این فایل برای جلوگیری از آن وجود دارد.
        let cached = vec![
            "gemini-2.5-pro".to_string(),
            "imagen-4.0".to_string(),
            "models/gemini-3.8-flash".to_string(),
            "gemini-3.8-flash".to_string(),
        ];
        assert_eq!(filter_ids(&cached), ["gemini-3.8-flash"]);
    }

    #[test]
    fn display_number_is_stable_when_a_model_is_unavailable() {
        assert_eq!(display_number("gemini-3.1-flash-lite"), 4);
        assert_eq!(display_number("gemini-3.7-flash"), 2);
        assert_eq!(display_number("something-else"), 0);
    }
}

#[derive(Debug, Clone)]
pub struct ModelTier {
    pub tier: &'static str,
    pub tokens: &'static str,
    pub description: &'static str,
}

pub fn get_model_tier(model_name: &str) -> Option<ModelTier> {
    match model_name {
        "gemini-2.0-flash" => Some(ModelTier {
            tier: "RECOMMENDED_DEFAULT",
            tokens: "1M tokens",
            description: "Fast, default model with 1M token context",
        }),
        "gemini-1.5-pro" => Some(ModelTier {
            tier: "HIGH_QUOTA_ALT",
            tokens: "2M tokens",
            description: "High capacity with 2M token context for complex tasks",
        }),
        "gemini-1.5-flash" => Some(ModelTier {
            tier: "FAST_LOW_QUOTA",
            tokens: "1M tokens",
            description: "Lightweight model with 1M token context",
        }),
        _ => None,
    }
}
