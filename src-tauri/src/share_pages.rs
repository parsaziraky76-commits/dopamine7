//! ۱.۵.۰ — صفحه‌ها و فایل‌هایی که اشتراکِ VPN به دستگاه‌های دیگر می‌دهد.
//! پورت از `core/SharePages.kt` موبایل (طراحیِ r10).
//!
//! فقط رشته‌اند، بی‌هیچ وابستگی، تا جدا تست و بازبینی شوند. **هیچ‌جای این
//! فایل رمزِ اشتراک را در بر ندارد:** این صفحه‌ها به هر دستگاهی در زیرشبکهٔ
//! اشتراک — احرازشده یا نه — داده می‌شوند.
//!
//! تفاوت‌های نسخهٔ ویندوز با موبایل، و فقط همین‌ها:
//!  * «گوشی» ← «این کامپیوتر»: اشتراک از ویندوز است.
//!  * راهنمای حالت فقط «وای‌فای/شبکهٔ خانگی» را دارد و همان «حالت فعلی» است؛
//!    حالتِ هات‌اسپاتِ موبایل در ویندوز نیست.
//!  * «کارت اشتراک» ← «زبانهٔ اشتراک» در برنامه.
//!
//! بقیه عیناً همان است: تیرهٔ سرمه‌ای، دوزبانه (پیش‌فرض فارسی، راست‌به‌چپ،
//! سوییچ EN)، فقط فونت‌های محلی (`local()` برای Vazirmatn/Vazir)، هر واژهٔ
//! لاتین داخل جملهٔ فارسی یک تکهٔ LTRِ جدا، دکمه‌های کپی، راهنمای هر دستگاه،
//! تستِ زندهٔ WebRTC و قفل‌های یک‌کلیکی، CSP (بی‌هیچ بارگذاری از بیرون)،
//! `no-referrer`، و `esc()` برای هر مقدارِ پویا.

pub const CHECK_HOST: &str = "dopamine.check";

pub const PATH_WINDOWS: &str = "/webrtc/aether-webrtc-lock.reg";
pub const PATH_FIREFOX: &str = "/webrtc/policies.json";
pub const PATH_MACOS: &str = "/webrtc/aether-webrtc-lock-macos.sh";
pub const PATH_LINUX: &str = "/webrtc/aether-webrtc-lock-linux.sh";

/// بدنه‌ای که پل عیناً می‌نویسد. `filename` آن را دانلود می‌کند.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub status: &'static str,
    pub content_type: &'static str,
    pub body: String,
    pub filename: Option<&'static str>,
}

/// مسیرهای دانلود؛ `None` برای هر چیزِ دیگر.
pub fn download(path: &str) -> Option<Page> {
    let p = path.to_ascii_lowercase();
    let page = |content_type, body, filename| Page {
        status: "200 OK",
        content_type,
        body,
        filename: Some(filename),
    };
    match p.as_str() {
        PATH_WINDOWS => Some(page(
            "application/octet-stream",
            windows_reg(),
            "aether-webrtc-lock.reg",
        )),
        PATH_FIREFOX => Some(page(
            "application/json; charset=utf-8",
            firefox_policies(),
            "policies.json",
        )),
        PATH_MACOS => Some(page(
            "text/x-shellscript; charset=utf-8",
            mac_script(),
            "aether-webrtc-lock-macos.sh",
        )),
        PATH_LINUX => Some(page(
            "text/x-shellscript; charset=utf-8",
            linux_script(),
            "aether-webrtc-lock-linux.sh",
        )),
        _ => None,
    }
}

// ------------------------------------------------------------ فایل‌های قفل

const POLICY_VALUE: &str = "disable_non_proxied_udp";

/// ویندوز: Chrome و Chromium `WebRtcIPHandling` را می‌خوانند، Edge
/// `WebRtcLocalhostIpHandling` را؛ Brave نام‌های Chromium را. هر دو نام در هر
/// کلید نوشته می‌شوند (سیاستِ ناشناخته نادیده گرفته می‌شود). HKLM و HKCU، با CRLF.
pub fn windows_reg() -> String {
    let keys = [
        "SOFTWARE\\Policies\\Google\\Chrome",
        "SOFTWARE\\Policies\\Microsoft\\Edge",
        "SOFTWARE\\Policies\\BraveSoftware\\Brave",
        "SOFTWARE\\Policies\\Chromium",
    ];
    let mut sb = String::new();
    sb.push_str("Windows Registry Editor Version 5.00\r\n\r\n");
    sb.push_str("; Dopamine VPN share - WebRTC lock.\r\n");
    sb.push_str("; Makes Chrome / Edge / Brave send WebRTC only through the configured proxy,\r\n");
    sb.push_str(
        "; so it cannot reveal this computer's real IP. Restart the browser afterwards.\r\n",
    );
    sb.push_str("; Undo: delete the WebRtcIPHandling / WebRtcLocalhostIpHandling values.\r\n\r\n");
    for hive in ["HKEY_LOCAL_MACHINE", "HKEY_CURRENT_USER"] {
        for key in keys {
            sb.push_str(&format!("[{hive}\\{key}]\r\n"));
            sb.push_str(&format!("\"WebRtcIPHandling\"=\"{POLICY_VALUE}\"\r\n"));
            sb.push_str(&format!(
                "\"WebRtcLocalhostIpHandling\"=\"{POLICY_VALUE}\"\r\n\r\n"
            ));
        }
    }
    sb
}

/// Firefox `distribution/policies.json`: WebRTC فقط از راهِ پروکسی.
pub fn firefox_policies() -> String {
    concat!(
        "{\n",
        "  \"policies\": {\n",
        "    \"Preferences\": {\n",
        "      \"media.peerconnection.ice.proxy_only_if_behind_proxy\": { \"Value\": true, \"Status\": \"locked\" },\n",
        "      \"media.peerconnection.ice.default_address_only\": { \"Value\": true, \"Status\": \"locked\" },\n",
        "      \"media.peerconnection.ice.no_host\": { \"Value\": true, \"Status\": \"locked\" }\n",
        "    }\n",
        "  }\n",
        "}\n",
    )
    .to_string()
}

pub fn mac_script() -> String {
    format!(
        "#!/bin/sh\n\
         # Dopamine VPN share - WebRTC lock for macOS (Chrome, Edge, Brave).\n\
         # Run:  sh aether-webrtc-lock-macos.sh   then quit and reopen the browser.\n\
         defaults write com.google.Chrome WebRtcIPHandling -string {POLICY_VALUE}\n\
         defaults write com.microsoft.Edge WebRtcLocalhostIpHandling -string {POLICY_VALUE}\n\
         defaults write com.brave.Browser WebRtcIPHandling -string {POLICY_VALUE}\n\
         echo \"WebRTC lock applied. Restart the browser, then open http://dopamine.check/ to verify.\"\n"
    )
}

pub fn linux_script() -> String {
    format!(
        "#!/bin/sh\n\
         # Dopamine VPN share - WebRTC lock for Linux (Chrome, Chromium, Brave, Edge).\n\
         # Run:  sudo sh aether-webrtc-lock-linux.sh   then restart the browser.\n\
         set -e\n\
         for d in /etc/opt/chrome/policies/managed /etc/chromium/policies/managed \\\n\
         \x20        /etc/brave/policies/managed /etc/opt/edge/policies/managed; do\n\
         \x20 mkdir -p \"$d\"\n\
         \x20 printf '{{ \"WebRtcIPHandling\": \"{POLICY_VALUE}\", \"WebRtcLocalhostIpHandling\": \"{POLICY_VALUE}\" }}\\n' \\\n\
         \x20   > \"$d/aether-webrtc-lock.json\"\n\
         done\n\
         echo \"WebRTC lock applied. Restart the browser, then open http://dopamine.check/ to verify.\"\n"
    )
}

// ----------------------------------------------------------------- صفحه‌ها

/// صفحهٔ بررسی. `via` مسیری است که درخواست از آن آمد (`None` = مرورگر این
/// کامپیوتر را مستقیم باز کرد، که دسترسی را ثابت می‌کند نه محافظت را).
/// `exit_ip` خروجیِ تأییدشدهٔ تونل است برای تستِ زندهٔ WebRTC.
pub fn check_page(via: Option<&str>, exit_ip: Option<&str>, on_device: bool) -> String {
    let protected = via.is_some();
    let chip = if protected {
        state_chip("ok", "محافظت‌شده", "Protected")
    } else {
        state_chip("warn", "فقط در دسترس", "Reachable only")
    };
    let heading = if protected {
        h1(
            &format!("این مرورگر از {A} رد می‌شه"),
            "This browser goes through Dopamine",
        )
    } else {
        h1(
            "این کامپیوتر در دسترسه، ولی مرورگرت هنوز ازش رد نمی‌شه",
            "The share is reachable, but this browser isn't using it yet",
        )
    };
    let check = code("http://dopamine.check/");
    let lead = if !protected {
        para(
            &format!(
                "این صفحه رو مستقیم از کامپیوترِ اشتراک‌گذار باز کردی. یعنی دستگاهت به اشتراک می‌رسه، نه اینکه اینترنتش از \
                 اشتراک رد می‌شه. پروکسی رو تنظیم کن و بعد {check} رو باز کن."
            ),
            &format!(
                "You opened the sharing PC directly. That proves this device can reach the share, not that this \
                 browser uses it. Set the proxy, then open {check}."
            ),
            "lead",
        )
    } else if on_device {
        para(
            &format!("این مرورگر روی خودِ همین کامپیوتر داره از پروکسی {A} استفاده می‌کنه."),
            "This browser is using the Dopamine proxy on this PC itself.",
            "lead",
        )
    } else {
        let v = esc(via.unwrap_or_default());
        para(
            &format!(
                "صفحه‌های این مرورگر از اشتراک {A} (از راه {}) رد می‌شن و از تونل {} کامپیوتر می‌رن بیرون، نه از اینترنت خودت.",
                ltr(&v),
                ltr("VPN")
            ),
            &format!(
                "This browser's pages go through the Dopamine share (via {v}) and leave through the PC's VPN tunnel, not your own connection."
            ),
            "lead",
        )
    };
    let setup_link = if protected {
        String::new()
    } else {
        format!(
            r#"<p class="actions"><a class="btn" href="/">{}</a></p>"#,
            bi("راهنمای راه‌اندازی", "Setup guide")
        )
    };
    let body = format!(
        r#"<section class="hero">
  {chip}
  {heading}
  {lead}
  {exit}
  {setup_link}
</section>
{rtc}
{mode}"#,
        exit = exit_row(exit_ip),
        rtc = web_rtc_section(exit_ip),
        mode = mode_section(),
    );
    page("بررسی اشتراک Dopamine", "Dopamine share check", &body)
}

/// صفحهٔ راه‌اندازی در `http://<این کامپیوتر>:10811/`. `auth_required` فقط
/// می‌گوید ورود لازم است یا نه — خودِ رمز هرگز بخشی از هیچ صفحه‌ای نیست.
pub fn setup_page(
    host: &str,
    http_port: u16,
    socks_port: u16,
    auth_required: Option<bool>,
) -> String {
    let h = esc(host);
    let pac = format!("http://{h}:{http_port}/proxy.pac");
    let mode_chip = state_chip("info", "حالت: وای‌فای خونه", "Mode: home Wi-Fi");
    let auth_note = match auth_required {
        Some(true) => note(
            "ok",
            "رمز اشتراک روشنه. نام کاربری و رمز رو از زبانهٔ «اشتراک» تو برنامه بردار؛ این صفحه هیچ‌وقت رمز رو نشون نمی‌ده.",
            "Share authentication is on. Take the username and password from the Share tab in the app; this page never shows the password.",
        ),
        Some(false) => note(
            "warn",
            &format!(
                "رمز اشتراک خاموشه، یعنی هر دستگاهی که تو همین شبکه‌ست می‌تونه از {} تو استفاده کنه. \
                 تو شبکهٔ خونه مشکلی نیست، ولی تو وای‌فای عمومی از زبانهٔ «اشتراک» روشنش کن.",
                ltr("VPN")
            ),
            "Share authentication is off: any device on this network can use your VPN. Fine at home; on public Wi-Fi turn it on from the Share tab.",
        ),
        None => String::new(),
    };
    let check = code("http://dopamine.check/");
    let body = format!(
        r#"<section class="hero">
  {on} {mode_chip}
  {title}
  {lead}
</section>

<section aria-labelledby="s-addr">
  {h_addr}
  {p_addr}
  <div class="rows">
    {r_pac}
    {r_http}
    {r_socks}
  </div>
  {auth_note}
</section>

{devices}
{mode}

<section aria-labelledby="s-test">
  {h_test}
  {p_test}
  <p class="actions">
    <a class="btn" href="http://{CHECK_HOST}/">{open_check}</a>
    <a class="btn ghost" href="/check">{test_here}</a>
  </p>
</section>

{rtc}"#,
        on = state_chip("ok", "اشتراک روشنه", "Sharing is on"),
        title = h1(
            &format!("اینترنت {} این کامپیوتر، روی بقیهٔ دستگاه‌ها", ltr("VPN")),
            "Share this PC's VPN with your other devices"
        ),
        lead = para(
            &format!(
                "این صفحه از خود کامپیوترِ اشتراک‌گذار باز شده، پس این دستگاه بهش می‌رسه. حالا فقط پروکسی رو تنظیم کن تا \
                 اینترنتش از تونل {} رد بشه.",
                ltr("VPN")
            ),
            "This page came straight from the sharing PC, so this device can reach it. Set the proxy below and its traffic will leave through the PC's VPN tunnel.",
            "lead",
        ),
        h_addr = h2("s-addr", "آدرس‌ها", "Addresses"),
        p_addr = para(
            &format!(
                "ساده‌ترین راه آدرس {} ـه: یه بار واردش کن، بقیه‌ش خودکاره. اگه {} کامپیوتر \
                 قطع بشه، این دستگاه هم آفلاین می‌شه و آی‌پی واقعیت لو نمی‌ره.",
                ltr("PAC"),
                ltr("VPN")
            ),
            "PAC is the easiest: enter it once and the rest is automatic. If the PC's VPN drops, this device goes offline instead of leaking your real IP.",
            "sub",
        ),
        r_pac = addr_row(&bi(&format!("پروکسی خودکار ({})", ltr("PAC")), "Automatic proxy (PAC)"), &pac, true, ""),
        r_http = addr_row(&bi(&format!("پروکسی {}", ltr("HTTP")), "HTTP proxy"), &format!("{h}:{http_port}"), false, ""),
        r_socks = addr_row(
            &bi(&format!("پروکسی {}", ltr("SOCKS5")), "SOCKS5 proxy"),
            &format!("{h}:{socks_port}"),
            false,
            &bi(&format!("گزینهٔ {} رو روشن کن", ltr("Remote DNS")), "Turn Remote DNS on"),
        ),
        devices = device_section(),
        mode = mode_section(),
        h_test = h2("s-test", "تست نهایی", "Final check"),
        p_test = para(
            &format!(
                "بعد از تنظیم پروکسی، {check} رو باز کن. این آدرس فقط وقتی باز می‌شه که \
                 واقعاً از اشتراک رد بشی؛ اگه باز نشد، پروکسی هنوز درست تنظیم نشده."
            ),
            &format!(
                "After setting the proxy, open {check}. It only loads when you really go through the share; if it does not open, the proxy is not set yet."
            ),
            "sub",
        ),
        open_check = bi(&format!("باز کردن {}", ltr("dopamine.check")), "Open dopamine.check"),
        test_here = bi("تست دسترسی از همین‌جا", "Test reachability from here"),
        rtc = web_rtc_section(None),
    );
    page("اشتراک VPN در Dopamine", "Dopamine VPN share", &body)
}

// ------------------------------------------------------------ تکه‌ها

/// «Aether» به‌صورت یک تکهٔ LTRِ جدا داخل متنِ فارسی.
const A: &str = r#"<span class="ltr" dir="ltr">Dopamine</span>"#;

fn ltr(text: &str) -> String {
    format!(r#"<span class="ltr" dir="ltr">{text}</span>"#)
}

fn code(text: &str) -> String {
    format!(r#"<code dir="ltr">{text}</code>"#)
}

/// هر دو زبان کنار هم؛ CSS زبانِ فعال را نشان می‌دهد.
fn bi(fa: &str, en: &str) -> String {
    format!(r#"<span class="fa">{fa}</span><span class="en" dir="ltr">{en}</span>"#)
}

fn para(fa: &str, en: &str, cls: &str) -> String {
    format!(r#"<p class="fa {cls}">{fa}</p><p class="en {cls}" dir="ltr">{en}</p>"#)
}

fn h1(fa: &str, en: &str) -> String {
    format!(r#"<h1><span class="fa">{fa}</span><span class="en" dir="ltr">{en}</span></h1>"#)
}

fn h2(id: &str, fa: &str, en: &str) -> String {
    format!(
        r#"<h2 id="{id}"><span class="fa">{fa}</span><span class="en" dir="ltr">{en}</span></h2>"#
    )
}

fn state_chip(kind: &str, fa: &str, en: &str) -> String {
    format!(
        r#"<span class="chip {kind}"><i aria-hidden="true"></i>{}</span>"#,
        bi(fa, en)
    )
}

fn note(kind: &str, fa: &str, en: &str) -> String {
    format!(r#"<div class="note {kind}">{}</div>"#, para(fa, en, ""))
}

fn steps(fa: &[String], en: &[String]) -> String {
    let li = |items: &[String]| {
        items
            .iter()
            .map(|i| format!("<li>{i}</li>"))
            .collect::<String>()
    };
    format!(
        r#"<ol class="steps fa">{}</ol><ol class="steps en" dir="ltr">{}</ol>"#,
        li(fa),
        li(en)
    )
}

fn addr_row(label: &str, value: &str, recommended: bool, hint: &str) -> String {
    let badge = if recommended {
        format!(
            r#"<span class="badge">{}</span>"#,
            bi("پیشنهادی", "Recommended")
        )
    } else {
        String::new()
    };
    let hint_line = if hint.is_empty() {
        String::new()
    } else {
        format!(r#"<span class="hint">{hint}</span>"#)
    };
    format!(
        r#"
<div class="row">
  <div class="k">{label} {badge}</div>
  <div class="v"><code dir="ltr">{value}</code>{hint_line}</div>
  <button type="button" class="copy" data-copy="{value}">{copy}</button>
</div>"#,
        copy = bi("کپی", "Copy")
    )
}

fn exit_row(exit_ip: Option<&str>) -> String {
    let ip = exit_ip.map(sanitize_ip).unwrap_or_default();
    if ip.is_empty() {
        return String::new();
    }
    format!(
        r#"
<div class="exit">
  <span class="k">{}</span>
  <code dir="ltr">{ip}</code>
</div>"#,
        bi("آی‌پی خروجی تونل", "Tunnel exit IP")
    )
}

fn s(items: &[&str]) -> Vec<String> {
    items.iter().map(|i| i.to_string()).collect()
}

/// قدم‌های پروکسی برای هر دستگاه، یک ردیفِ بازشونده برای هر سکو.
fn device_section() -> String {
    let pac_fa = format!("آدرس {}", ltr("PAC"));
    let l = ltr;
    let devices = [
        device(
            "Windows",
            &[
                format!("{} ← {} ← {} رو باز کن.", l("Settings"), l("Network &amp; Internet"), l("Proxy")),
                format!("زیر {}، گزینهٔ {} رو روشن کن.", l("Automatic proxy setup"), l("Use setup script")),
                format!("{pac_fa} رو تو {} بچسبون و {} رو بزن.", l("Script address"), l("Save")),
            ],
            &s(&[
                "Open Settings → Network &amp; Internet → Proxy.",
                "Under Automatic proxy setup, turn on Use setup script.",
                "Paste the PAC address into Script address and press Save.",
            ]),
            None,
        ),
        device(
            "macOS",
            &[
                format!("{} ← {} ← {} ← {}.", l("System Settings"), l("Wi-Fi"), l("Details"), l("Proxies")),
                format!("{} رو روشن کن و {pac_fa} رو بچسبون.", l("Automatic proxy configuration")),
                format!("{} رو بزن و مرورگر رو یه بار ببند و باز کن.", l("OK")),
            ],
            &s(&[
                "System Settings → Wi-Fi → Details → Proxies.",
                "Turn on Automatic proxy configuration and paste the PAC address.",
                "Press OK and restart the browser once.",
            ]),
            None,
        ),
        device(
            "Android",
            &[
                format!("تو تنظیمات {} روی شبکه‌ای که بهش وصلی بزن (یا آیکون چرخ‌دنده) و برو تو بخش پروکسی.", l("Wi-Fi")),
                format!("{} رو انتخاب کن و {pac_fa} رو بچسبون.", l("Proxy Auto-Config")),
                format!(
                    "این فقط مرورگر و بعضی برنامه‌ها رو رد می‌کنه. برای همهٔ برنامه‌ها، {} یا {} رو با آدرس {} تو حالت {} بزن.",
                    l("v2rayNG"),
                    l("NekoBox"),
                    l("SOCKS5"),
                    l("VPN")
                ),
            ],
            &s(&[
                "In Wi-Fi settings, tap the network you are on (or its gear icon) and open Proxy.",
                "Choose Proxy Auto-Config and paste the PAC address.",
                "That covers the browser and some apps. For every app, run v2rayNG or NekoBox with the SOCKS5 address in VPN mode.",
            ]),
            None,
        ),
        device(
            "iPhone / iPad",
            &[
                format!("{} ← {} ← دکمهٔ {} کنار شبکه.", l("Settings"), l("Wi-Fi"), l("(i)")),
                format!("پایین صفحه {} ← {}.", l("Configure Proxy"), l("Automatic")),
                format!("{pac_fa} رو تو {} بچسبون و {} رو بزن.", l("URL"), l("Save")),
            ],
            &s(&[
                "Settings → Wi-Fi → the (i) button next to the network.",
                "At the bottom: Configure Proxy → Automatic.",
                "Paste the PAC address into URL and tap Save.",
            ]),
            None,
        ),
        device(
            "Firefox",
            &[
                format!("{} ← پایین صفحه {} ← {}.", l("Settings"), l("Network Settings"), l("Settings…")),
                format!("{} رو بزن و {pac_fa} رو بچسبون.", l("Automatic proxy configuration URL")),
                format!("اگه {} می‌زنی، تیک {} رو هم بزن.", l("SOCKS5"), l("Proxy DNS when using SOCKS v5")),
            ],
            &s(&[
                "Settings → Network Settings (at the bottom) → Settings….",
                "Pick Automatic proxy configuration URL and paste the PAC address.",
                "If you use SOCKS5 instead, also tick Proxy DNS when using SOCKS v5.",
            ]),
            None,
        ),
        device(
            "TV / کل دستگاه",
            &[
                format!("رو تلویزیون، گوشی یا لپ‌تاپ مهمان {}، {} یا {} رو نصب کن.", l("v2rayNG"), l("NekoBox"), l("sing-box")),
                format!(
                    "یه کانفیگ {} با آدرس بالا بساز و {} رو روی {} بذار (بدون استثنا).",
                    l("SOCKS5"),
                    l("Routing"),
                    l("Global")
                ),
                format!("تو حالت {} روشنش کن. این‌جوری همه‌چی، حتی {} و {}، از تونل رد می‌شه.", l("VPN"), l("WebRTC"), l("UDP")),
            ],
            &s(&[
                "On the TV, guest phone or laptop install v2rayNG, NekoBox or sing-box.",
                "Create a SOCKS5 profile with the address above and set routing to Global (no bypass).",
                "Start it in VPN mode. Then everything, WebRTC and UDP included, goes through the tunnel.",
            ]),
            Some("TV / whole device"),
        ),
    ];
    format!(
        r#"
<section aria-labelledby="s-dev">
  {}
  {}
  <div class="rows acc">
    {}
  </div>
</section>"#,
        h2("s-dev", "تنظیم روی هر دستگاه", "Set it up on each device"),
        para(
            &format!("دستگاهت رو باز کن و قدم‌ها رو برو. هر جا {pac_fa} خواست، همون آدرس بالا رو بچسبون."),
            "Open your device and follow the steps. Wherever it asks for a PAC or script address, paste the one above.",
            "sub",
        ),
        devices.join("\n    ")
    )
}

fn device(title: &str, fa: &[String], en: &[String], en_title: Option<&str>) -> String {
    let t = match en_title {
        None => ltr(&esc(title)),
        Some(en) => bi(&esc(title), &esc(en)),
    };
    format!(
        r#"
<details class="item">
  <summary><span class="name">{t}</span><i class="chev" aria-hidden="true"></i></summary>
  <div class="body">{}</div>
</details>"#,
        steps(fa, en)
    )
}

/// راهنمای حالت. ویندوز فقط «شبکهٔ خانگی» دارد، پس همان باز و «حالت فعلی» است.
fn mode_section() -> String {
    let current = format!(
        r#"<span class="badge cur">{}</span>"#,
        bi("حالت فعلی", "Current")
    );
    format!(
        r#"
<section aria-labelledby="s-mode">
  {h}
  {sub}
  <div class="rows acc">
    <details class="item" open>
      <summary><span class="name">{name}</span>{current}<i class="chev" aria-hidden="true"></i></summary>
      <div class="body">
        {p}
        {st}
        {n}
      </div>
    </details>
  </div>
</section>"#,
        h = h2("s-mode", "وای‌فای خونه", "Home Wi-Fi"),
        sub = para(
            &format!(
                "اشتراک در {A} ویندوز روی شبکهٔ خونه کار می‌کنه: {} یا کابل {}.",
                ltr("Wi-Fi"),
                ltr("Ethernet")
            ),
            "Sharing in Dopamine for Windows works over your home network: Wi-Fi or an Ethernet cable.",
            "sub",
        ),
        name = bi("وای‌فای خونه", "Home Wi-Fi"),
        p = para(
            &format!("کامپیوتر و این دستگاه هر دو به یه مودم وصلن و کامپیوتر {} رو با بقیه شریک می‌شه.", ltr("VPN")),
            "The PC and this device share one router, and the PC shares its VPN with the rest.",
            "sub",
        ),
        st = steps(
            &[
                format!("تو {A}، زبانهٔ «اشتراک» رو باز کن و «اشتراک در شبکهٔ محلی» رو روشن کن."),
                format!(
                    "این دستگاه رو به همون شبکه‌ای وصل کن که کامپیوتر بهش وصله. شبکهٔ مهمان ({}) نباشه، چون معمولاً اونجا دستگاه‌ها همدیگه رو نمی‌بینن.",
                    ltr("Guest")
                ),
                format!(
                    "آدرس {} رو از زبانهٔ «اشتراک» (یا صفحهٔ {}) تو تنظیمات پروکسی بزن.",
                    ltr("PAC"),
                    code("http://&lt;PC&gt;:10811/")
                ),
                "آدرس کامپیوتر ممکنه با ری‌استارت مودم عوض بشه؛ اگه یهو قطع شد، آدرس تازه رو از زبانهٔ «اشتراک» بردار.".to_string(),
            ],
            &[
                "In Dopamine, open the Share tab and turn on Share over LAN.".to_string(),
                "Join this device to the same network as the PC. Not a Guest network: devices there usually cannot see each other.".to_string(),
                format!(
                    "Enter the PAC address from the Share tab (or from {}) in the proxy settings.",
                    code("http://&lt;PC&gt;:10811/")
                ),
                "The PC's address can change when the router restarts; if it suddenly stops, take the new address from the Share tab.".to_string(),
            ],
        ),
        n = note(
            "warn",
            &format!(
                "تو وای‌فای عمومی (کافه، دانشگاه، هتل) حتماً «رمز اشتراک» رو روشن کن. تو این حالت {} \
                 مستقیم از مودم می‌ره بیرون و اصلاً به کامپیوتر نمی‌رسه؛ برای همین قفل {} لازمه.",
                ltr("WebRTC"),
                ltr("WebRTC")
            ),
            "On public Wi-Fi (cafés, campus, hotels) turn share authentication on. In this mode WebRTC goes straight out of the router and never reaches the PC, so the WebRTC lock is needed.",
        ),
    )
}

fn web_rtc_section(exit_ip: Option<&str>) -> String {
    format!(
        r#"
<section aria-labelledby="s-rtc">
  {}
  <div id="rtc" class="box wait" role="status" aria-live="polite">
    <i aria-hidden="true"></i>
    <div class="msg">{}</div>
  </div>
  {}
</section>
{}"#,
        h2("s-rtc", &format!("نشت {}", ltr("WebRTC")), "WebRTC leak"),
        bi(
            &format!("داریم {} رو تست می‌کنیم…", ltr("WebRTC")),
            "Testing WebRTC…"
        ),
        lock_section(),
        web_rtc_script(exit_ip)
    )
}

fn lock_section() -> String {
    format!(
        r#"
{}
<div class="rows">
  {}
  {}
  {}
  {}
</div>
{}"#,
        para(
            &format!(
                "مرورگرها {} رو (برای تماس تصویری و این‌جور چیزها) بیرون از پروکسی می‌فرستن، مگه اینکه قفلش \
                 کنی. قفل مناسب رو یه بار نصب کن، مرورگر رو کامل ببند و باز کن، بعد این صفحه رو رفرش کن.",
                ltr("WebRTC")
            ),
            "Browsers send WebRTC outside a proxy unless told not to. Install the lock once on this computer, fully restart the browser, and reload this page.",
            "sub",
        ),
        lock_row(
            "Windows · Chrome, Edge, Brave",
            PATH_WINDOWS,
            "aether-webrtc-lock.reg",
            &format!("دوبار روش کلیک کن و {} رو بزن.", ltr("Yes")),
            "Double-click it and accept.",
        ),
        lock_row(
            "Firefox",
            PATH_FIREFOX,
            "policies.json",
            &format!(
                "بذارش تو یه پوشه به اسم {} کنار {} (تو مک: {}).",
                code("distribution"),
                code("firefox.exe"),
                code("Firefox.app/Contents/Resources/distribution")
            ),
            &format!(
                "Put it in a folder named {} next to {} (macOS: {}).",
                code("distribution"),
                code("firefox.exe"),
                code("Firefox.app/Contents/Resources/distribution")
            ),
        ),
        lock_row(
            "macOS · Chrome, Edge, Brave",
            PATH_MACOS,
            "aether-webrtc-lock-macos.sh",
            &format!("تو ترمینال بزن: {}", code("sh aether-webrtc-lock-macos.sh")),
            &format!("In Terminal: {}", code("sh aether-webrtc-lock-macos.sh")),
        ),
        lock_row(
            "Linux · Chrome, Chromium, Brave, Edge",
            PATH_LINUX,
            "aether-webrtc-lock-linux.sh",
            &format!("تو ترمینال بزن: {}", code("sudo sh aether-webrtc-lock-linux.sh")),
            &format!("In a terminal: {}", code("sudo sh aether-webrtc-lock-linux.sh")),
        ),
        para(
            &format!(
                "نمی‌خوای چیزی نصب کنی؟ افزونهٔ رسمی {} رو بریز و گزینهٔ {} رو بزن. رو گوشی‌ها و کل دستگاه هم {} یا {} \
                 تو حالت {} با آدرس {} و مسیریابی {} همه‌چی رو از تونل رد می‌کنه.",
                ltr("WebRTC Network Limiter"),
                ltr("Use my proxy server"),
                ltr("v2rayNG"),
                ltr("NekoBox"),
                ltr("VPN"),
                ltr("SOCKS5"),
                ltr("Global")
            ),
            "No install: the official WebRTC Network Limiter extension, option \"Use my proxy server\". On phones or a whole device, v2rayNG or NekoBox in VPN mode with the SOCKS5 address and Global routing sends everything, WebRTC included, through the tunnel.",
            "sub tail",
        )
    )
}

fn lock_row(title: &str, path: &str, file: &str, fa: &str, en: &str) -> String {
    format!(
        r#"
<div class="row">
  <div class="k">{}</div>
  <div class="v">{}</div>
  <a class="dl" href="{path}" download="{file}">{}</a>
</div>"#,
        ltr(&esc(title)),
        bi(fa, en),
        bi("دانلود", "Download")
    )
}

fn web_rtc_script(exit_ip: Option<&str>) -> String {
    WEBRTC_JS.replace("@@EXIT@@", &exit_ip.map(sanitize_ip).unwrap_or_default())
}

/// فقط لیترالِ IPِ ابتدای مقدار — «1.2.3.4<script>» یعنی «1.2.3.4».
pub fn sanitize_ip(value: &str) -> String {
    value
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_hexdigit() || *c == '.' || *c == ':')
        .take(45)
        .collect()
}

/// هر مقداری که از این فایل نیامده HTML-escape می‌شود.
pub fn esc(value: &str) -> String {
    let mut sb = String::with_capacity(value.len() + 8);
    for c in value.chars() {
        match c {
            '&' => sb.push_str("&amp;"),
            '<' => sb.push_str("&lt;"),
            '>' => sb.push_str("&gt;"),
            '"' => sb.push_str("&quot;"),
            '\'' => sb.push_str("&#39;"),
            _ => sb.push(c),
        }
    }
    sb
}

fn page(title_fa: &str, title_en: &str, body: &str) -> String {
    let foot = para(
        &format!(
            "این صفحه هیچ‌وقت رمز اشتراک رو نشون نمی‌ده و از اینترنت چیزی بارگذاری نمی‌کنه. تست {} \
             فقط با دو سرور {} عمومی (گوگل و کلادفلر) حرف می‌زنه.",
            ltr("WebRTC"),
            ltr("STUN")
        ),
        "This page never shows the share password and loads nothing from the internet. The WebRTC test only talks to two public STUN servers (Google and Cloudflare).",
        "",
    );
    let mut out = String::with_capacity(body.len() + CSS.len() + UI_JS.len() + 4096);
    out.push_str(&format!(
        r##"<!doctype html>
<html lang="fa" dir="rtl" data-lang="fa" data-title-fa="{title_fa}" data-title-en="{title_en}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none'">
<meta name="referrer" content="no-referrer">
<meta name="color-scheme" content="dark">
<meta name="theme-color" content="#0a1026">
<title>{title_fa}</title>
"##
    ));
    out.push_str(LANG_BOOT);
    out.push_str("\n<style>");
    out.push_str(CSS);
    out.push_str("</style>\n</head>\n<body>\n<div class=\"wrap\">\n");
    out.push_str(&format!(
        r#"  <header class="top">
    <div class="brand">
      <span class="mark" aria-hidden="true"></span>
      <span class="bn" dir="ltr">Dopamine</span>
      <span class="bs">{}</span>
    </div>
    <div class="lang" role="group" aria-label="Language / زبان">
      <button type="button" data-set-lang="fa" aria-pressed="true" lang="fa">فارسی</button>
      <button type="button" data-set-lang="en" aria-pressed="false" lang="en">English</button>
    </div>
  </header>
  <main>
"#,
        bi("اشتراک VPN", "VPN share")
    ));
    out.push_str(body);
    out.push_str("\n  </main>\n  <footer class=\"foot\">\n    ");
    out.push_str(&foot);
    out.push_str("\n  </footer>\n</div>\n");
    out.push_str(UI_JS);
    out.push_str("\n</body></html>");
    out
}

/// پیش از اولین نقاشی اجرا می‌شود تا خوانندهٔ انگلیسی چشمکِ فارسی نبیند.
const LANG_BOOT: &str = r#"<script>
(function () {
  try {
    if (localStorage.getItem('aether-lang') === 'en') {
      var r = document.documentElement;
      r.setAttribute('data-lang', 'en'); r.lang = 'en'; r.dir = 'ltr';
    }
  } catch (e) {}
})();
</script>"#;

const CSS: &str = r#"
@font-face{font-family:DopamineFa;font-weight:400;font-display:swap;
  src:local("Vazirmatn"),local("Vazirmatn Regular"),local("Vazirmatn-Regular"),local("Vazir"),local("Vazir Regular"),local("Vazir-Regular"),local("Vazir FD"),local("Vazirmatn FD")}
@font-face{font-family:DopamineFa;font-weight:700;font-display:swap;
  src:local("Vazirmatn Bold"),local("Vazirmatn-Bold"),local("Vazir Bold"),local("Vazir-Bold"),local("Vazirmatn")}
:root{--bg:#0a1026;--bg2:#0d1531;--s1:#111b3b;--s2:#18244d;--line:#1f2d5a;--line2:#2b3c75;
  --tx:#e7ecf8;--mut:#a0acc9;--dim:#7482a8;--ac:#8fa8ff;--ac2:#c3d0ff;
  --ok:#46d3a0;--okbg:#0e2a2c;--okl:#1f5b50;--warn:#f1c35e;--warnbg:#241f14;--warnl:#5b4a22;
  --bad:#ff7d8e;--badbg:#2b1224;--badl:#6b2a40;--info:#8fa8ff;--infobg:#141f45;--infol:#2e4088}
*{box-sizing:border-box}
html{background:var(--bg);color-scheme:dark;-webkit-text-size-adjust:100%}
body{margin:0;color:var(--tx);line-height:1.8;font-size:16px;
  font-family:DopamineFa,Vazirmatn,Vazir,"Segoe UI",Tahoma,"SF Arabic","Geeza Pro","Noto Sans Arabic UI","Noto Sans Arabic","Noto Naskh Arabic",system-ui,sans-serif;
  background:radial-gradient(1200px 520px at 50% -180px,#15225280,transparent 70%),var(--bg);min-height:100vh}
html[data-lang=en] body{line-height:1.65}
html[data-lang=fa] .en,html[data-lang=en] .fa{display:none!important}
.ltr{direction:ltr;unicode-bidi:isolate}
code{direction:ltr;unicode-bidi:isolate;display:inline-block;font-family:ui-monospace,"SF Mono",Menlo,Consolas,"Roboto Mono",monospace;
  font-size:.9em;background:var(--s1);border:1px solid var(--line);border-radius:6px;padding:0 6px;line-height:1.7;word-break:break-all;color:var(--ac2)}
a{color:var(--ac);text-decoration-thickness:1px;text-underline-offset:3px}
a:hover{color:var(--ac2)}
:focus-visible{outline:2px solid var(--ac);outline-offset:2px;border-radius:6px}
.wrap{max-width:760px;margin:0 auto;padding:16px 20px 48px}
.top{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:6px 0 18px;border-bottom:1px solid var(--line)}
.brand{display:flex;align-items:center;gap:10px;min-width:0}
.mark{width:30px;height:30px;border-radius:9px;flex:none;background:var(--s2);border:1px solid var(--line2);position:relative}
.mark::after{content:"";position:absolute;inset:8px;border-radius:50%;border:2px solid var(--ac);border-top-color:transparent;transform:rotate(-40deg)}
.bn{font-weight:700;font-size:17px;letter-spacing:.3px}
.bs{color:var(--dim);font-size:14px;white-space:nowrap}
.lang{display:inline-flex;flex:none;border:1px solid var(--line2);border-radius:999px;padding:3px;background:var(--bg2)}
.lang button{font:inherit;font-size:14px;line-height:1;border:0;background:transparent;color:var(--mut);padding:8px 14px;border-radius:999px;cursor:pointer;transition:background .25s cubic-bezier(.22,1,.36,1),color .25s}
.lang button[lang=en]{font-family:system-ui,"Segoe UI",Roboto,sans-serif}
.lang button[aria-pressed=true]{background:var(--s2);color:var(--tx);box-shadow:inset 0 0 0 1px var(--line2)}
.lang button:hover{color:var(--tx)}
.hero{padding:34px 0 30px}
.chip{display:inline-flex;align-items:center;gap:8px;font-size:13.5px;line-height:1;padding:7px 12px;border-radius:999px;border:1px solid;margin:0 0 0 6px;vertical-align:middle}
html[dir=ltr] .chip{margin:0 6px 0 0}
.chip i{width:8px;height:8px;border-radius:50%;background:currentColor}
.chip.ok{color:var(--ok);background:var(--okbg);border-color:var(--okl)}
.chip.warn{color:var(--warn);background:var(--warnbg);border-color:var(--warnl)}
.chip.info{color:var(--info);background:var(--infobg);border-color:var(--infol)}
.chip.ok i{box-shadow:0 0 0 4px #46d3a022}
h1{font-size:clamp(25px,5.2vw,34px);line-height:1.45;margin:18px 0 10px;font-weight:800;letter-spacing:-.2px}
html[data-lang=en] h1{line-height:1.25;letter-spacing:-.5px}
.lead{color:var(--mut);font-size:17px;max-width:62ch;margin:0}
.exit{display:flex;flex-wrap:wrap;align-items:center;gap:10px;margin:20px 0 0}
.exit .k{color:var(--dim);font-size:14px}
.exit code{font-size:15px;padding:2px 10px}
section{padding:28px 0 26px;border-top:1px solid var(--line)}
.hero{border-top:0}
h2{font-size:20px;line-height:1.5;margin:0 0 4px;font-weight:700}
.sub{color:var(--mut);margin:0 0 16px;font-size:15.5px;max-width:68ch}
.sub.tail{margin:16px 0 0;font-size:14.5px;color:var(--dim)}
.rows{border:1px solid var(--line);border-radius:14px;background:var(--bg2);overflow:hidden}
.row{display:grid;grid-template-columns:minmax(0,1fr) auto;grid-template-areas:"k b" "v b";gap:4px 14px;padding:14px 16px;align-items:center}
.row+.row{border-top:1px solid var(--line)}
.row .k{grid-area:k;font-size:14px;color:var(--mut);display:flex;align-items:center;flex-wrap:wrap;gap:8px}
.row .v{grid-area:v;min-width:0;font-size:14.5px}
.row .v code{font-size:15px;padding:2px 8px}
.row .hint{display:block;color:var(--dim);font-size:13px;margin-top:4px}
.row .copy,.row .dl{grid-area:b}
.badge{font-size:12px;line-height:1;padding:5px 9px;border-radius:999px;background:var(--infobg);color:var(--ac2);border:1px solid var(--infol);white-space:nowrap}
.badge.cur{background:var(--okbg);color:var(--ok);border-color:var(--okl)}
.copy,.dl,.btn{font:inherit;font-size:14px;line-height:1;display:inline-flex;align-items:center;justify-content:center;gap:6px;
  padding:10px 14px;border-radius:10px;border:1px solid var(--line2);background:var(--s1);color:var(--tx);cursor:pointer;text-decoration:none;white-space:nowrap;
  transition:background .25s cubic-bezier(.22,1,.36,1),border-color .25s,color .25s}
.copy:hover,.dl:hover,.btn:hover{background:var(--s2);border-color:#3a4f94;color:var(--tx)}
.copy:active,.dl:active,.btn:active{transform:translateY(1px)}
.copy.done{color:var(--ok);border-color:var(--okl);background:var(--okbg)}
.btn{background:var(--ac);color:#0a1026;border-color:var(--ac);font-weight:700;padding:12px 18px}
.btn:hover{background:var(--ac2);border-color:var(--ac2);color:#0a1026}
.btn.ghost{background:transparent;color:var(--tx);border-color:var(--line2);font-weight:500}
.btn.ghost:hover{background:var(--s1);color:var(--tx)}
.actions{display:flex;flex-wrap:wrap;gap:10px;margin:20px 0 0}
.note{border:1px solid;border-radius:12px;padding:12px 14px;margin:14px 0 0;font-size:14.5px}
.note p{margin:0}
.note.ok{background:var(--okbg);border-color:var(--okl)}
.note.warn{background:var(--warnbg);border-color:var(--warnl)}
.acc .item+.item{border-top:1px solid var(--line)}
.acc summary{list-style:none;cursor:pointer;display:flex;align-items:center;gap:10px;padding:15px 16px;font-weight:600;user-select:none}
.acc summary::-webkit-details-marker{display:none}
.acc summary:hover{background:#ffffff05}
.acc .name{flex:1;min-width:0}
.chev{width:9px;height:9px;border-inline-end:2px solid var(--dim);border-bottom:2px solid var(--dim);transform:rotate(45deg);transition:transform .3s cubic-bezier(.22,1,.36,1);margin:0 4px 4px}
.acc details[open]>summary .chev{transform:rotate(225deg);margin-bottom:-4px}
.acc details[open]>summary{border-bottom:1px solid var(--line)}
.acc .body{padding:16px 16px 18px}
.acc .body .note{margin-top:4px}
.steps{list-style:none;counter-reset:s;padding:0;margin:0 0 12px}
.steps>li{counter-increment:s;position:relative;padding-inline-start:40px;margin:0 0 12px;font-size:15px}
.steps>li::before{content:counter(s);position:absolute;inset-inline-start:0;top:2px;width:26px;height:26px;border-radius:50%;
  background:var(--s2);border:1px solid var(--line2);color:var(--ac2);display:flex;align-items:center;justify-content:center;font-size:13px;font-weight:700;line-height:1}
html[data-lang=fa] .steps>li::before{content:counter(s,persian)}
.box{display:flex;gap:12px;align-items:flex-start;border:1px solid;border-radius:12px;padding:14px 16px;margin:6px 0 18px;font-size:15px}
.box .msg{min-width:0;overflow-wrap:anywhere}
.box>i{flex:none;width:24px;height:24px;border-radius:50%;margin-top:2px;display:flex;align-items:center;justify-content:center;font-style:normal;font-weight:800;font-size:14px;line-height:1}
.box.wait{background:var(--infobg);border-color:var(--infol)}
.box.wait>i{border:2px solid var(--line2);border-top-color:var(--ac);animation:spin .9s linear infinite}
.box.ok{background:var(--okbg);border-color:var(--okl)}
.box.ok>i{background:var(--ok);color:#0a1026}
.box.ok>i::before{content:"\2713"}
.box.warn{background:var(--warnbg);border-color:var(--warnl)}
.box.warn>i{background:var(--warn);color:#0a1026}
.box.warn>i::before{content:"!"}
.box.bad{background:var(--badbg);border-color:var(--badl)}
.box.bad>i{background:var(--bad);color:#0a1026}
.box.bad>i::before{content:"\2715"}
.box b{color:var(--tx)}
@keyframes spin{to{transform:rotate(360deg)}}
.foot{border-top:1px solid var(--line);margin-top:8px;padding-top:18px;color:var(--dim);font-size:13.5px}
.foot p{margin:0;max-width:70ch}
@media (max-width:560px){
  .wrap{padding:12px 16px 40px}
  .bs{display:none}
  .hero{padding:26px 0 24px}
  .row{grid-template-columns:minmax(0,1fr);grid-template-areas:"k" "v" "b";gap:8px}
  .row .copy,.row .dl{justify-self:start;padding:9px 16px}
  .actions .btn{flex:1 1 100%}
}
@media (prefers-reduced-motion:reduce){*{transition:none!important;animation-duration:2s!important}}
"#;

/// سوییچِ زبان و دکمه‌های کپی. کپی روی http سادهٔ (بی secure context) هم کار می‌کند.
const UI_JS: &str = r#"<script>
(function () {
  var root = document.documentElement;
  var buttons = document.querySelectorAll('[data-set-lang]');
  function setLang(l, save) {
    root.setAttribute('data-lang', l);
    root.lang = l;
    root.dir = l === 'fa' ? 'rtl' : 'ltr';
    document.title = root.getAttribute('data-title-' + l) || document.title;
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute('aria-pressed', buttons[i].getAttribute('data-set-lang') === l ? 'true' : 'false');
    }
    if (save) { try { localStorage.setItem('aether-lang', l); } catch (e) {} }
  }
  setLang(root.getAttribute('data-lang') === 'en' ? 'en' : 'fa', false);
  for (var i = 0; i < buttons.length; i++) {
    buttons[i].addEventListener('click', function () { setLang(this.getAttribute('data-set-lang'), true); });
  }
  function fallbackCopy(text) {
    var t = document.createElement('textarea');
    t.value = text; t.setAttribute('readonly', ''); t.style.position = 'fixed'; t.style.opacity = '0';
    document.body.appendChild(t); t.select();
    var ok = false;
    try { ok = document.execCommand('copy'); } catch (e) {}
    document.body.removeChild(t);
    return ok;
  }
  var copies = document.querySelectorAll('[data-copy]');
  for (var j = 0; j < copies.length; j++) {
    copies[j].addEventListener('click', function () {
      var b = this, text = b.getAttribute('data-copy');
      var saved = b.innerHTML;
      function flag(ok) {
        b.classList.add('done');
        b.innerHTML = ok ? '<span class="fa">کپی شد</span><span class="en">Copied</span>'
                         : '<span class="fa">دستی کپی کن</span><span class="en">Copy by hand</span>';
        setTimeout(function () { b.classList.remove('done'); b.innerHTML = saved; }, 1600);
      }
      if (navigator.clipboard && window.isSecureContext) {
        navigator.clipboard.writeText(text).then(function () { flag(true); }, function () { flag(fallbackCopy(text)); });
      } else {
        flag(fallbackCopy(text));
      }
    });
  }
})();
</script>"#;

/// تستِ زندهٔ WebRTC: کاندیداهای ICE در برابرِ دو سرورِ STUNِ عمومی، و هر
/// آدرسِ عمومی که WebRTC یاد می‌گیرد با خروجیِ تونل مقایسه می‌شود.
const WEBRTC_JS: &str = r#"<script>
(function () {
  var exitIp = "@@EXIT@@";
  var box = document.getElementById('rtc');
  if (!box) return;
  function esc(s) {
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
  }
  function ips(list) {
    return list.map(function (ip) { return '<b dir="ltr" class="ltr">' + esc(ip) + '</b>'; }).join(', ');
  }
  function show(cls, en, fa) {
    box.className = 'box ' + cls;
    box.innerHTML = '<i aria-hidden="true"></i><div class="msg"><span class="fa">' + fa +
      '</span><span class="en" dir="ltr">' + en + '</span></div>';
  }
  var W = '<span class="ltr" dir="ltr">WebRTC</span>';
  if (!window.RTCPeerConnection) {
    show('ok', 'WebRTC is not available in this browser, so it cannot reveal your IP.',
      W + ' تو این مرورگر نیست، پس نمی‌تونه آی‌پیت رو لو بده.');
    return;
  }
  function isV6(ip) { return ip.indexOf(':') >= 0; }
  function isPublic(ip) {
    if (isV6(ip)) {
      var l = ip.toLowerCase();
      return !(l.indexOf('fe80') === 0 || l.indexOf('fc') === 0 || l.indexOf('fd') === 0 || l === '::1');
    }
    var p = ip.split('.').map(Number);
    if (p.length !== 4) return false;
    if (p[0] === 10 || p[0] === 127 || p[0] === 0) return false;
    if (p[0] === 172 && p[1] >= 16 && p[1] <= 31) return false;
    if (p[0] === 192 && p[1] === 168) return false;
    if (p[0] === 169 && p[1] === 254) return false;
    if (p[0] === 100 && p[1] >= 64 && p[1] <= 127) return false;
    return true;
  }
  var found = {}, finished = false, pc;
  try {
    pc = new RTCPeerConnection({ iceServers: [{ urls: ['stun:stun.l.google.com:19302', 'stun:stun.cloudflare.com:3478'] }] });
  } catch (e) {
    show('ok', 'WebRTC is blocked in this browser, so it cannot reveal your IP.',
      W + ' تو این مرورگر بسته‌ست، پس نمی‌تونه آی‌پیت رو لو بده.');
    return;
  }
  function done() {
    if (finished) return;
    finished = true;
    try { pc.close(); } catch (e) {}
    var all = Object.keys(found);
    var sameFamily = all.filter(function (ip) { return !exitIp || isV6(ip) === isV6(exitIp); });
    var leaks = sameFamily.filter(function (ip) { return ip !== exitIp; });
    var exitTag = '<b dir="ltr" class="ltr">' + esc(exitIp) + '</b>';
    if (all.length === 0) {
      show('ok', 'No WebRTC leak: WebRTC cannot reveal any public IP from this browser.',
        'نشتی نیست: ' + W + ' هیچ آی‌پی عمومی‌ای از این مرورگر لو نمی‌ده.');
    } else if (exitIp && leaks.length > 0) {
      show('bad', 'WebRTC LEAK: this browser reveals ' + ips(leaks) + ' (not the tunnel exit ' + exitTag +
        '). Install the lock below and restart the browser.',
        'نشت ' + W + '! این مرورگر آی‌پی ' + ips(leaks) + ' رو لو می‌ده (نه آی‌پی خروجی تونل). ' +
        'قفل پایین رو نصب کن و مرورگر رو کامل ببند و باز کن.');
    } else if (exitIp && sameFamily.length > 0) {
      show('ok', 'WebRTC goes through the tunnel: it only sees the exit ' + exitTag + '.',
        W + ' هم از تونل رد می‌شه و فقط آی‌پی خروجی ' + exitTag + ' رو می‌بینه.');
    } else {
      show('warn', 'WebRTC reveals ' + ips(all) + '. If that is not the exit IP shown in Dopamine, it is a ' +
        'leak: install the lock below.',
        W + ' این آدرس رو نشون می‌ده: ' + ips(all) + '. اگه با آی‌پی خروجی توی برنامهٔ ' +
        '<span class="ltr" dir="ltr">Dopamine</span> یکی نیست، یعنی نشت داره؛ قفل پایین رو نصب کن.');
    }
  }
  pc.onicecandidate = function (e) {
    if (!e.candidate) { done(); return; }
    var parts = (e.candidate.candidate || '').split(' ');
    var ip = parts[4];
    if (ip && ip.indexOf('.local') < 0 && isPublic(ip)) found[ip] = true;
  };
  try { pc.createDataChannel('aether'); } catch (e) {}
  pc.createOffer().then(function (o) { return pc.setLocalDescription(o); }).catch(function () { done(); });
  setTimeout(done, 7000);
})();
</script>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloads_are_routed_case_insensitively_and_nothing_else() {
        assert_eq!(
            download("/WEBRTC/aether-webrtc-lock.reg").unwrap().filename,
            Some("aether-webrtc-lock.reg")
        );
        assert!(download(PATH_FIREFOX)
            .unwrap()
            .body
            .contains("proxy_only_if_behind_proxy"));
        assert!(download(PATH_MACOS)
            .unwrap()
            .body
            .starts_with("#!/bin/sh\n"));
        assert!(download(PATH_LINUX)
            .unwrap()
            .body
            .contains("\"$d/aether-webrtc-lock.json\""));
        assert!(download("/webrtc/other").is_none());
        assert!(download("/").is_none());
    }

    #[test]
    fn the_reg_file_is_crlf_and_covers_every_browser_in_both_hives() {
        let reg = windows_reg();
        assert!(reg.starts_with("Windows Registry Editor Version 5.00\r\n"));
        assert!(!reg.replace("\r\n", "").contains('\n'));
        assert_eq!(
            reg.matches("\"WebRtcIPHandling\"=\"disable_non_proxied_udp\"")
                .count(),
            8
        );
        assert!(reg.contains("[HKEY_CURRENT_USER\\SOFTWARE\\Policies\\Microsoft\\Edge]"));
    }

    #[test]
    fn the_linux_script_keeps_its_shell_variable_and_json() {
        let sh = linux_script();
        assert!(sh.contains("mkdir -p \"$d\""));
        assert!(sh.contains("printf '{ \"WebRtcIPHandling\": \"disable_non_proxied_udp\""));
    }

    #[test]
    fn pages_are_hardened_bilingual_and_never_carry_a_password() {
        let setup = setup_page("192.168.1.20", 10811, 10810, Some(true));
        for page in [
            setup.clone(),
            check_page(Some("SOCKS5"), Some("203.0.113.9"), false),
            check_page(None, None, false),
        ] {
            assert!(page.contains("Content-Security-Policy"));
            assert!(page.contains("default-src 'none'"));
            assert!(page.contains(r#"<meta name="referrer" content="no-referrer">"#));
            assert!(page.contains(r#"<html lang="fa" dir="rtl" data-lang="fa""#));
            assert!(page.contains(r#"data-set-lang="en""#));
            assert!(!page.contains("http://fonts") && !page.contains("https://fonts"));
            assert!(!page.contains("src=\"http"));
            assert!(!page.to_lowercase().contains("password:"));
        }
        assert!(setup.contains(r#"data-copy="http://192.168.1.20:10811/proxy.pac""#));
        assert!(setup.contains(r#"data-copy="192.168.1.20:10811""#));
        assert!(setup.contains(r#"data-copy="192.168.1.20:10810""#));
        // فقط حالتِ شبکهٔ خانگی، و همان «حالت فعلی».
        assert!(setup.contains("badge cur"));
        assert!(!setup.contains("هات‌اسپات گوشی"));
        assert!(!setup.contains("Phone hotspot"));
    }

    #[test]
    fn dynamic_values_are_escaped() {
        let page = setup_page("<x>\"", 10811, 10810, None);
        assert!(!page.contains("<x>\""));
        assert!(page.contains("&lt;x&gt;&quot;"));
        let check = check_page(
            Some("<script>"),
            Some("1.2.3.4<script>alert(1)</script>"),
            false,
        );
        assert!(!check.contains("<script>alert"));
        assert!(check.contains("var exitIp = \"1.2.3.4\";"));
        assert!(check.contains("&lt;script&gt;"));
    }

    #[test]
    fn sanitize_keeps_only_the_leading_ip_literal() {
        assert_eq!(sanitize_ip(" 1.2.3.4<script>"), "1.2.3.4");
        assert_eq!(sanitize_ip("2001:db8::1 x"), "2001:db8::1");
        assert_eq!(sanitize_ip("\"; alert(1)"), "");
    }
}
