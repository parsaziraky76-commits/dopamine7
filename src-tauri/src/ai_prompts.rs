//! پورت ۱:۱ از `ai/AiPrompts.kt` — تنها جایی که به مدل گفته می‌شود چه کسی است،
//! به چه زبانی حرف بزند، و اجازه دارد چه چیزی برگرداند.
//!
//! # چرا پرامپت‌ها یک‌جا هستند
//!
//! سه مسیر مصرف‌کنندهٔ مدل وجود دارد (توضیح یک تنظیم، مشاورهٔ ضد‌DPI، چت آزاد) و
//! هر سه به همان قواعد پایه نیاز دارند: زبانِ درست، هرگز ادعای دسترسی به چیزی
//! که ندارد، هرگز پرسیدنِ یک راز، و — برای مشاور — یک قرارداد خروجیِ ماشین‌خوان.
//! جای‌دادنِ این‌ها در نقطهٔ فراخوانی یعنی سه نسخهٔ واگرا، که یکی‌شان فراموش
//! می‌کند بگوید «فارسی» و کاربر بی‌دلیل انگلیسی می‌گیرد.
//!
//! # قرارداد خروجی، و کاری که *نمی‌کند*
//!
//! پرامپتِ مشاور می‌گوید یک شیء JSON برگردان. این یک **درخواست** است و نه یک
//! تضمین، و هیچ چیزی در این فایل مرزِ اعتماد نیست: هر چیزی که برگردد از
//! [`crate::ai_patch`] رد می‌شود، که فهرست مجاز و بازه‌ها را اعمال می‌کند.
//! `responseMimeType: application/json` در [`crate::ai_client`] هم قالب را
//! مجبور می‌کند. [`extract_json_object`] اینجا آخرین شبکهٔ نجات است، برای مدلی
//! که با وجود هر دو، شیء را در یک code fence می‌پیچد.

use crate::ai_patch;

/// زبانی که پاسخ باید در آن نوشته شود.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fa,
    En,
}

impl Lang {
    /// از کد زبانِ رابط کاربری. هر چیز دیگری انگلیسی است.
    pub fn from_code(code: &str) -> Self {
        if code.starts_with("fa") {
            Lang::Fa
        } else {
            Lang::En
        }
    }

    fn instruction(self) -> &'static str {
        match self {
            // «فارسی روان» صریح گفته می‌شود چون بدون آن، مدل‌ها برای یک برنامهٔ
            // فنی به فارسیِ ترجمه‌شده‌ٔ ماشینی می‌افتند.
            Lang::Fa => "Answer in fluent, natural Persian (فارسی). Keep technical terms and setting names in English so the user can find them in the app.",
            Lang::En => "Answer in English.",
        }
    }
}

/// قواعدی که در هر سه مسیر مشترک‌اند.
///
/// به آنچه گفته می‌شود توجه کنید: مدل صریح مطلع می‌شود که **دسترسی** ندارد.
/// بدون این، مدل‌ها با اطمینان می‌گویند «تنظیمات را برایت عوض کردم» یا «الان
/// اتصال را تست می‌کنم» — رفتاری که برای کاربری که دارد یک فیلترشکن را عیب‌یابی
/// می‌کند، از بی‌فایده بدتر است، چون او باور می‌کند کاری انجام شده.
fn base_rules(lang: Lang) -> String {
    format!(
        "You are the built-in assistant of Dopamine, a Windows VPN client that tunnels traffic through \
WireGuard/MASQUE and can chain through Psiphon to get past internet censorship in Iran.\n\
{}\n\
Rules you must follow:\n\
- You cannot see the user's screen, run anything, or change any setting yourself. Never claim you did.\n\
- Never ask for a password, an API key, a service token, or any other credential. You will never need one.\n\
- Be concrete and short. The user is looking at a settings screen, not reading an article.\n\
- If you do not know, say so instead of inventing an option that does not exist in this app.",
        lang.instruction()
    )
}

/// ۱.۴.۰-r4 (پورت از `AiPrompts.GROUNDING`): مدل پیش از هر توصیه چطور فکر کند.
///
/// شکایتِ میدانی‌ای که این را لازم کرد: دستیار تنظیمی را «پیشنهاد» می‌داد که
/// کاربر از قبل داشت — «MTU را ۱۲۸۰ کن» به کسی که ۱۲۸۰ داشت. بلوکِ تنظیمات همیشه
/// در پرامپت بود؛ هیچ چیز به مدل نمی‌گفت با آن **مقایسه کند**. پس مقایسه حالا یک
/// رویهٔ صریح و مرتب است (بی‌صدا انجام می‌شود و فقط نتیجه‌اش گفته می‌شود)، و
/// برنامه نتیجه را دوباره در کد می‌سنجد ([`crate::ai_patch::verification_feedback`]
/// + یک دور خوداصلاحی در [`crate::ai_session`]). پرامپت خطِ اولِ دفاع است، نه تنها خط.
pub const GROUNDING: &str =
    "HOW TO THINK BEFORE YOU ANSWER (do this silently; show only the result):
1. Read THE USER'S CURRENT SETTINGS below. They are the ground truth about this device right now. \
Each line says whether the value is the app's default or was changed by the user.
2. Work out the most likely cause from what the user said (and the log, if one is given). Rank \
candidate fixes by how likely they are to help.
3. For EVERY candidate change, compare it with the current value. If the user already has that \
value, it is NOT a fix - drop it. If it was your best idea, say plainly that it is already set \
(e.g. \"your MTU is already 1280, so that is not the cause\") and move on to the next candidate.
4. Check the candidate actually applies to the current mode (backend and protocol): do not \
recommend a knob that has no effect in that mode. If you are not sure whether it applies, say so \
instead of guessing.
5. Prefer ONE or TWO changes at a time, most likely first, and say how the user will know whether \
it helped. Changing five things at once makes it impossible to learn what worked.
6. Never invent settings, values, menu names, features, numbers or facts. If the information \
needed is not here, say what you would need to know, or ask ONE short clarifying question.
7. If everything relevant already looks right, say so honestly and suggest the next diagnostic \
step instead of inventing a change.";

/// ۱.۴.۰-r4 (پورت از `AiPrompts.FORMAT`): قالبِ پاسخ، به سبکِ خودِ اپ Gemini.
///
/// رابط کاربری این Markdown را رندر می‌کند (`src/ui/markdown.js`)، پس ساختار
/// خوش‌آمد است — ولی فقط زیرمجموعه‌ای که در یک حبابِ چت خوب درمی‌آید.
pub const FORMAT: &str = "FORMATTING (the app renders Markdown like the Gemini app does):
- Open with a direct one- or two-sentence answer. No preamble, no \"Great question\", no restating \
the question.
- For anything longer, use short sections with a \"### \" title, or a bold mini-title on its own \
line (**Like this**), then one to three sentences.
- Use \"- \" bullets for options or facts and \"1. \" numbered steps for things to do in order. One \
level of nesting at most.
- Put setting names and key values in **bold**; put exact values the user types (1280, \
X25519:P-256) in `backticks`.
- Short paragraphs (one to three sentences). A blank line between blocks.
- No tables unless you are comparing three or more options side by side.
- Never wrap the whole answer in a code block; no emoji clutter; no closing sign-off.";

/// دستور سیستمی برای «این تنظیم چه کار می‌کند؟».
pub fn explain_system(lang: Lang) -> String {
    format!(
        "{}\n\nYou are explaining ONE setting. Structure: what it does (1-2 sentences), when to turn it on, \
when to leave it alone. At most 90 words. No headings, no bullet lists, no markdown.",
        base_rules(lang)
    )
}

/// پرسشِ کاربر برای «این تنظیم چه کار می‌کند؟».
pub fn explain_user(title: &str, subtitle: &str, current_value: &str) -> String {
    let mut prompt = format!("Setting: {title}");
    if !subtitle.trim().is_empty() {
        prompt.push_str(&format!("\nDescription shown in the app: {subtitle}"));
    }
    if !current_value.trim().is_empty() {
        prompt.push_str(&format!("\nIts value right now: {current_value}"));
    }
    prompt.push_str("\n\nExplain it.");
    prompt
}

/// نامِ code fence که مدل تغییراتِ پیشنهادی‌اش را در آن می‌گذارد.
///
/// پورت از `AiPrompts.APPLY_FENCE`. یک نامِ اختصاصی و نه ` ```json `: پاسخِ چت
/// می‌تواند به‌کلِ دلایلِ دیگری هم JSON داشته باشد (کاربر پرسیده «فایل پیکربندی
/// WireGuard چه شکلی است؟») و برداشتنِ آن به‌عنوان پیشنهادِ تنظیمات، یک دکمهٔ
/// «اعمال» روی چیزی می‌ساخت که هرگز پیشنهاد نبود.
pub const APPLY_FENCE: &str = "aether-apply";

/// یک تغییرِ پیشنهادیِ مدل، همان‌طور که از پاسخ بیرون آمده — و نه بیشتر.
///
/// هنوز اعتبارسنجی **نشده**: `key` می‌تواند هر چیزی باشد و `value` هر متنی.
/// داوری‌اش کارِ [`crate::ai_patch::apply_chat`] است.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedChange {
    pub key: String,
    pub value: String,
    pub why: String,
}

/// دستور سیستمی برای چت آزاد.
///
/// ۱.۴.۰-r4: `GROUNDING` و `FORMAT` اضافه شدند، و دو زمینهٔ تازه:
///  - `model` — نامِ مدلی که دارد جواب می‌دهد، تا «تو کدام مدلی؟» جوابِ درست بگیرد؛
///  - `connection_status` — وضعیتِ **زندهٔ** اتصال («connected»،
///    «reconnecting» …). بدون آن، مدل به کاربری که همین حالا قطع است می‌گفت
///    «اتصالت را قطع و وصل کن» و نمی‌دانست تنظیمِ اعمال‌شده کِی اثر می‌کند.
///
/// بلوکِ پیشنهاد هم دیگر فقط «وقتی کاربر خواست» نیست (همان موبایل): هر بار که
/// مدل یک تغییرِ مشخص را **توصیه** می‌کند، بلوک می‌آید — کاربر هنوز باید دکمه
/// را بزند و برنامه هر مورد را با تنظیماتِ فعلی می‌سنجد.
pub fn chat_system(
    lang: Lang,
    profile_snapshot: &str,
    model: &str,
    connection_status: &str,
) -> String {
    let model_line = if model.trim().is_empty() {
        String::new()
    } else {
        format!(" You are {}.", model.trim())
    };
    let status_line = if connection_status.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\nConnection status right now: {}.",
            connection_status.trim()
        )
    };
    format!(
        "{}\n\n{}\n\n{}\n\nYou are in a chat with the user about this app, their connection, and censorship \
circumvention in general.{} Be useful, precise and brief: keep answers under 180 words unless the \
user asks for detail. Stay consistent with what you said earlier in this conversation; if you got \
something wrong before, correct it explicitly.{}\n\n\
SETTING CHANGES. Whenever you recommend a concrete change to one of the settings listed below - \
whether or not the user asked for it - append at the very END of your reply one fenced block with \
exactly those changes:\n\n\
```{}\n\
{{\"changes\":[{{\"key\":\"mtu\",\"value\":\"1380\",\"why\":\"short reason\"}}]}}\n\
```\n\n\
- Put your normal answer above the block. The app strips the block out and turns it into a button \
the user has to press, and checks each change against the current settings. You are PROPOSING; \
never say you already changed anything.\n\
- Only these keys and values are possible:\n{}\n\
- Anything else - the network backend, the upstream proxy, routing rules, split tunnelling, a manual \
endpoint, LAN sharing, the kill switch, organization credentials - you CANNOT change, by design, \
because those decide which traffic is protected and where it goes. If the user asks for one of \
those, explain where to change it by hand instead: Settings, then the relevant page.\n\
- Tunnel settings are handed to the engine when it starts, so an applied change takes effect on the \
NEXT connect. Say so when it matters.\n\
- Never put a change in the block whose value equals the current one. Do not add the block at all \
when you are only explaining something.\n\n\
THE USER'S CURRENT SETTINGS:\n{}",
        base_rules(lang),
        GROUNDING,
        FORMAT,
        model_line,
        status_line,
        APPLY_FENCE,
        ai_patch::chat_keys_for_prompt(),
        profile_snapshot,
    )
}

/// ۱.۴.۰-r4 (پورت از `AiPrompts.chatRevision`): نوبتِ خوداصلاحی.
///
/// وقتی پاسخِ تازه از بررسیِ خودِ برنامه رد شد (مقداری که کاربر از قبل دارد،
/// مقداری که گزینه نمی‌پذیرد، کلیدی که مدل اجازه ندارد) — **نادیدنی برای کاربر** —
/// فرستاده می‌شود. مدل کلِ پاسخ را با واقعیت‌های سنجیده‌شده جلوی چشمش بازنویسی
/// می‌کند تا متنی که کاربر می‌خواند هرگز چیزی را توصیه نکند که دکمهٔ زیرش رد
/// می‌کند. فقط یک دور: پاسخِ دوم هر چه بگوید نشان داده می‌شود، با همان فیلترِ کد.
pub fn chat_revision(feedback: &str) -> String {
    format!(
        "AUTOMATIC CHECK BY THE APP (not written by the user): your previous reply failed verification \
against the user's real settings:\n{feedback}\n\n\
Rewrite your whole reply for the user now. Keep the same language and format. Do not mention this \
check or apologise for it. Do not recommend anything the user already has - where it matters, state \
that it is already set - and offer the next most likely fix instead, or say honestly that the \
settings already look right. Only include the fenced `{APPLY_FENCE}` block if a valid, non-redundant \
change remains."
    )
}

/// ۱.۴.۰-r4 (پورت از `AiPrompts.advisorRevision`): همان خوداصلاحی برای پاسخِ JSONِ مشاور.
pub fn advisor_revision(feedback: &str) -> String {
    format!(
        "AUTOMATIC CHECK BY THE APP: some keys in your \"patch\" failed verification against the \
user's real settings:\n{feedback}\n\n\
Answer again with the complete JSON object ({{\"reason\": ..., \"patch\": {{...}}}}). Remove or \
replace those keys; never propose a value the user already has. If the obvious fix is already in \
place, say so in \"reason\". JSON only."
    )
}

/// پاسخِ چت را به «متنِ دیدنی» و «تغییراتِ پیشنهادی» تفکیک می‌کند.
///
/// پورت از `AiPrompts.splitChatReply`. اگر بلوکی نبود، کلِ متن دیدنی است و
/// فهرست خالی — که حالتِ عادیِ تقریباً هر نوبتِ گفت‌وگو است.
///
/// بلوک از متنِ دیدنی **بریده می‌شود**: گذاشتنش در حباب یعنی کاربر یک JSON خام
/// زیر پاسخش ببیند و همان تغییرات دوباره در کارتِ پیشنهاد تکرار شوند.
pub fn split_chat_reply(raw: &str) -> (String, Vec<ProposedChange>) {
    let opener = format!("```{APPLY_FENCE}");
    let Some(fence_start) = raw.find(&opener) else {
        return (raw.trim().to_string(), Vec::new());
    };
    // ابتدای بدنه: اولین خطِ تازه پس از خودِ نشانه. بدون این، نامِ fence هم جزء
    // JSON شمرده می‌شد.
    let Some(body_offset) = raw[fence_start..].find('\n') else {
        return (raw.trim().to_string(), Vec::new());
    };
    let body_start = fence_start + body_offset;
    let fence_end = raw[body_start..].find("```").map(|i| body_start + i);
    let block = match fence_end {
        Some(end) => &raw[body_start..end],
        // بلوکِ بسته‌نشده هم خوانده می‌شود: یک پاسخِ بریده که وسطِ fence تمام شده
        // باز هم می‌تواند شیءِ کاملی داشته باشد، و دورانداختنش یعنی سهمیه سوخته و
        // هیچ چیزی روی صفحه.
        None => &raw[body_start..],
    };
    let mut visible = raw[..fence_start].to_string();
    if let Some(end) = fence_end {
        visible.push_str(&raw[end + 3..]);
    }
    let visible = visible.trim().to_string();

    let Some(object) = extract_json_object(block) else {
        return (visible, Vec::new());
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&object) else {
        return (visible, Vec::new());
    };
    let Some(items) = parsed.get("changes").and_then(|c| c.as_array()) else {
        return (visible, Vec::new());
    };

    let mut changes = Vec::new();
    for item in items {
        let key = text_of(item.get("key"));
        if key.is_empty() {
            continue;
        }
        changes.push(ProposedChange {
            key,
            // عدد و بولین نصفِ وقت‌ها بی‌کوتیشن می‌آیند، پس کلِ زنجیره متن حرف
            // می‌زند و `ai_patch::coerce` تبدیل را انجام می‌دهد.
            value: text_of(item.get("value")),
            why: text_of(item.get("why")),
        });
    }
    (visible, changes)
}

/// یک مقدارِ JSON را به متن تبدیل می‌کند، چه رشته باشد چه عدد چه بولین.
fn text_of(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(s)) => s.trim().to_string(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// دستور سیستمی برای مشاور ضد‌DPI — تنها مسیری که JSON برمی‌گرداند.
///
/// فهرست کلیدها از [`crate::ai_patch::writable_keys_for_prompt`] تولید می‌شود و
/// دستی تکرار نشده: دو فهرستِ دستی یعنی روزی که یکی عوض شود، مدل کلیدهایی
/// پیشنهاد کند که نگهبان رد می‌کند و کاربر یک «هیچ تغییری اعمال نشد» بی‌توضیح
/// ببیند.
pub fn advisor_system(lang: Lang) -> String {
    format!(
        "{}\n\n{}\n\nYou are tuning this app's transport settings to get through DPI-based censorship. \
You will be shown the current settings and a redacted tail of the connection log.\n\
The log has been stripped of anything that identifies the user: public IP addresses are masked to \
their first two groups, install identifiers and credentials are replaced. Do not ask for the \
unmasked values; reason with what you are given.\n\n\
Reply with a single JSON object and nothing else. Shape:\n\
{{\"reason\": \"one or two sentences on what the log shows\", \"patch\": {{ ...settings... }}}}\n\
Only these keys may appear inside \"patch\": {}\n\
Rules for the patch:\n\
- Include ONLY keys you are actually changing. An empty patch is a valid answer and the right one \
when the log shows a healthy connection.\n\
- Never propose a value the user already has: compare every key with THE USER'S CURRENT SETTINGS \
(each line says whether it is the default or was changed by the user). A key whose value equals the \
current one is an error, and the app will reject it. If the obvious fix is already in place, say so \
in \"reason\" and propose the next most likely one instead (or none).\n\
- Every change must be tied to something specific in the log.\n\
- Change as few things as possible. One or two settings, not a rewrite.\n\
- The baseline MTU is {}. Lower it only if the log shows fragmentation or handshake timeouts.\n\
- \"reason\" is shown to the user. Write it in the same language as the rest of your answer.",
        base_rules(lang),
        GROUNDING,
        ai_patch::writable_keys_for_prompt(),
        ai_patch::BASELINE_MTU,
    )
}

/// پرسشِ مشاور: تنظیمات فعلی + خلاصهٔ پاک‌شدهٔ لاگ.
///
/// ۱.۴.۰-r4: snapshotِ حاشیه‌دار (`[default]` / `[changed by user; default: X]`) پیش
/// از JSONِ کامل می‌آید، تا گام‌های `GROUNDING` چیزی برای مقایسه داشته باشند.
pub fn advisor_user(annotated_snapshot: &str, settings_json: &str, redacted_log: &str) -> String {
    format!(
        "THE USER'S CURRENT SETTINGS:\n{annotated_snapshot}\n\nFull settings (JSON):\n{settings_json}\n\nRecent connection log (redacted, oldest line first):\n{redacted_log}"
    )
}

/// اولین شیء JSONِ متوازن را از یک پاسخ بیرون می‌کشد.
///
/// آخرین شبکهٔ نجات، و نه مسیر اصلی: `responseMimeType: application/json` باید
/// یعنی پاسخ از قبل خالص است. این تابع برای مدلی است که با وجود آن، شیء را در
/// ```` ```json ```` می‌پیچد یا یک جمله جلویش می‌گذارد.
///
/// شمارشِ آکولاد و **آگاه به رشته**، نه `find('{')` و `rfind('}')`: مقدارِ
/// `reason` می‌تواند آکولاد داشته باشد و یک برشِ ساده روی متنی که با یک
/// `}` داخل نقل‌قول تمام می‌شود، نصفِ شیء را برمی‌گرداند.
pub fn extract_json_object(reply: &str) -> Option<String> {
    let chars: Vec<char> = reply.chars().collect();
    let start = chars.iter().position(|c| *c == '{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in chars.iter().enumerate().skip(start) {
        if in_string {
            if escaped {
                escaped = false;
            } else if *c == '\\' {
                escaped = true;
            } else if *c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(chars[start..=i].iter().collect());
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persian_is_requested_explicitly_but_terms_stay_english() {
        let system = explain_system(Lang::from_code("fa-IR"));
        assert!(system.contains("Persian"));
        assert!(system.contains("technical terms"));
        assert!(matches!(Lang::from_code("en-US"), Lang::En));
    }

    #[test]
    fn the_model_is_told_it_cannot_change_anything() {
        // مستقیماً جلوی «تنظیماتت را عوض کردم» را می‌گیرد.
        let chat = chat_system(
            Lang::En,
            &crate::ai_patch::chat_snapshot(&Default::default()),
            "gemini-3.1-flash-lite",
            "connected",
        );
        for system in [
            explain_system(Lang::En),
            chat.clone(),
            advisor_system(Lang::En),
        ] {
            assert!(system.contains("Never claim you did"), "{system}");
            assert!(system.contains("Never ask for a password"), "{system}");
        }
        // و در چت، که *می‌تواند* پیشنهاد بدهد، همین قاعده یک بار دیگر و صریح‌تر
        // گفته می‌شود: پیشنهاد دادن با انجام دادن یکی نیست.
        assert!(
            chat.contains("You are PROPOSING; never say you already changed anything"),
            "{chat}"
        );
    }

    #[test]
    fn the_advisor_prompt_lists_the_real_desktop_keys_and_no_secrets() {
        let system = advisor_system(Lang::En);
        assert!(system.contains("reconnectAttempts"));
        assert!(system.contains("dns (list of text)"));
        // نام‌های اندرویدی نباید آنجا باشند.
        assert!(!system.contains("reconnectRetryLimit"));
        assert!(!system.contains("accessSecret"));
    }

    #[test]
    fn grounding_format_model_and_status_reach_the_chat_prompt() {
        let system = chat_system(
            Lang::En,
            &crate::ai_patch::chat_snapshot(&Default::default()),
            "gemini-3.1-flash-lite",
            "reconnecting (attempt 2)",
        );
        assert!(
            system.contains("HOW TO THINK BEFORE YOU ANSWER"),
            "{system}"
        );
        assert!(system.contains("FORMATTING"), "{system}");
        assert!(
            system.contains("You are gemini-3.1-flash-lite."),
            "{system}"
        );
        assert!(
            system.contains("Connection status right now: reconnecting (attempt 2)."),
            "{system}"
        );
        assert!(
            system.contains("[default]"),
            "the annotated snapshot must be in the prompt"
        );
        assert!(
            system.contains("Never put a change in the block whose value equals the current one")
        );
        // بدونِ وضعیت، خطِ وضعیت هم نیست — نه یک «Connection status right now: .» خالی.
        let quiet = chat_system(Lang::En, "", "", "");
        assert!(!quiet.contains("Connection status right now"));
        assert!(!quiet.contains("You are ."));
    }

    #[test]
    fn the_advisor_is_grounded_too() {
        let system = advisor_system(Lang::En);
        assert!(system.contains("HOW TO THINK BEFORE YOU ANSWER"));
        assert!(system.contains("Never propose a value the user already has"));
        let user = advisor_user("  mtu = 1280   [default]", "{}", "log line");
        assert!(user.starts_with("THE USER'S CURRENT SETTINGS:"));
        assert!(user.contains("[default]") && user.contains("log line"));
    }

    #[test]
    fn revision_turns_carry_the_feedback_and_the_fence_rule() {
        let chat = chat_revision("- mtu = \"1280\": REDUNDANT");
        assert!(chat.contains("REDUNDANT") && chat.contains(APPLY_FENCE));
        assert!(chat.contains("Do not mention this check"));
        let advisor = advisor_revision("- mtu = \"1280\": REDUNDANT");
        assert!(advisor.contains("REDUNDANT") && advisor.contains("JSON only"));
    }

    #[test]
    fn a_fenced_object_is_recovered() {
        let reply = "Sure! Here you go:\n```json\n{\"reason\":\"x\",\"patch\":{\"mtu\":1380}}\n```";
        let json = extract_json_object(reply).unwrap();
        assert_eq!(json, "{\"reason\":\"x\",\"patch\":{\"mtu\":1380}}");
    }

    #[test]
    fn a_brace_inside_a_string_does_not_end_the_object() {
        // یک برشِ ساده اینجا شیء را وسط `reason` می‌بُرید.
        let reply = r#"{"reason":"the log shows {weird} output","patch":{}}"#;
        assert_eq!(extract_json_object(reply).unwrap(), reply);
    }

    #[test]
    fn an_answer_with_no_object_is_none_not_a_panic() {
        assert!(extract_json_object("I cannot help with that.").is_none());
        assert!(extract_json_object("{\"unterminated\":").is_none());
    }

    // ---- تفکیکِ پاسخِ چت --------------------------------------------------

    #[test]
    fn a_plain_chat_answer_has_no_changes() {
        let (visible, changes) = split_chat_reply("  MTU is the largest packet size.  ");
        assert_eq!(visible, "MTU is the largest packet size.");
        assert!(changes.is_empty());
    }

    #[test]
    fn the_apply_block_is_parsed_and_removed_from_the_visible_text() {
        let reply = format!(
            "I will lower the MTU.\n\n```{APPLY_FENCE}\n\
             {{\"changes\":[{{\"key\":\"mtu\",\"value\":1380,\"why\":\"handshakes time out\"}},\
             {{\"key\":\"fragment\",\"value\":true,\"why\":\"splits the ClientHello\"}}]}}\n```"
        );
        let (visible, changes) = split_chat_reply(&reply);
        assert_eq!(visible, "I will lower the MTU.");
        assert!(
            !visible.contains("changes"),
            "the raw JSON must never reach the bubble"
        );
        assert_eq!(changes.len(), 2);
        // عدد و بولینِ بی‌کوتیشن باید به متن تبدیل شده باشند.
        assert_eq!(
            changes[0],
            ProposedChange {
                key: "mtu".into(),
                value: "1380".into(),
                why: "handshakes time out".into(),
            }
        );
        assert_eq!(changes[1].value, "true");
    }

    #[test]
    fn text_after_the_block_stays_visible() {
        let reply = format!(
            "Before.\n```{APPLY_FENCE}\n{{\"changes\":[{{\"key\":\"ech\",\"value\":\"true\"}}]}}\n```\nAfter."
        );
        let (visible, changes) = split_chat_reply(&reply);
        assert_eq!(visible, "Before.\n\nAfter.");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].why, "", "a missing reason is empty, not a panic");
    }

    #[test]
    fn a_json_fence_that_is_not_ours_is_left_alone() {
        // کاربر پرسیده «یک فایل پیکربندی WireGuard چه شکلی است؟». این JSON یک
        // پیشنهادِ تنظیمات نیست و نباید دکمهٔ «اعمال» بسازد.
        let reply =
            "Like this:\n```json\n{\"changes\":[{\"key\":\"mtu\",\"value\":\"9000\"}]}\n```";
        let (visible, changes) = split_chat_reply(reply);
        assert!(changes.is_empty(), "only our own fence may propose changes");
        assert_eq!(visible, reply.trim());
    }

    #[test]
    fn a_truncated_block_still_yields_its_changes() {
        let reply =
            format!("Lowering it.\n```{APPLY_FENCE}\n{{\"changes\":[{{\"key\":\"mtu\",\"value\":\"1380\"}}]}}");
        let (visible, changes) = split_chat_reply(&reply);
        assert_eq!(visible, "Lowering it.");
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn a_malformed_block_loses_the_changes_but_keeps_the_answer() {
        let reply = format!("Here.\n```{APPLY_FENCE}\nnot json at all\n```");
        let (visible, changes) = split_chat_reply(&reply);
        assert_eq!(visible, "Here.");
        assert!(changes.is_empty());
    }

    #[test]
    fn an_entry_without_a_key_is_dropped() {
        let reply = format!(
            "Ok.\n```{APPLY_FENCE}\n{{\"changes\":[{{\"value\":\"1380\"}},{{\"key\":\"ech\",\"value\":\"true\"}}]}}\n```"
        );
        let (_, changes) = split_chat_reply(&reply);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].key, "ech");
    }

    #[test]
    fn the_chat_prompt_carries_the_allow_list_and_the_current_values() {
        let system = chat_system(
            Lang::Fa,
            &crate::ai_patch::chat_snapshot(&Default::default()),
            "",
            "",
        );
        assert!(system.contains(APPLY_FENCE));
        assert!(
            system.contains("mtu: a number from 1280 to 9000"),
            "{system}"
        );
        assert!(
            system.contains("mtu = 1280"),
            "the snapshot must be in the prompt"
        );
        // و هیچ‌کدام از کلیدهای خطرناک به مدل پیشنهاد نمی‌شوند.
        assert!(!system.contains("upstream:"));
        assert!(!system.contains("routeDirect"));
    }
}
