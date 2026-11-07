//! پورت ۱:۹ از `ai/AiTopic.kt` — فهرست رسمیِ «موضوع»ها: هر تنظیمی که کاربر
//! می‌تواند دربارهٔ آن از دستیار بپرسد.
//!
//! # چرا یک ثبتِ سمتِ Rust، وقتی صفحات تنظیمات جاوااسکریپت‌اند
//!
//! وسوسه‌اش این بود که عنوان و زیرنویس را همان‌طور که روی صفحه است از فرانت‌اند
//! پاس بدهیم و تمام. دو دلیل که نمی‌شود:
//!
//!  1. **چیزی که به مدل می‌رود باید محدود باشد.** یک عنوانِ آزاد از فرانت‌اند
//!     یعنی هر متنی که به `ai_explain` برسد مستقیم داخل پرامپت می‌نشیند. برای
//!     تنها صفحه‌ای که خودش UI را می‌سازد این خطر کوچک است، ولی این همان مرزی
//!     است که [`crate::ai_patch`] در جهت مخالف نگه می‌دارد؛ نگهبانی که یک‌طرفه
//!     باشد فقط نصفِ یک مرز است.
//!  2. **کلیدِ واقعیِ تنظیم را مدل باید بداند.** کاربر «Endpoint» می‌بیند؛ آنچه
//!     به توضیح معنا می‌دهد `endpointMode` و مقدارِ فعلی‌اش است. نگاشتِ
//!     عنوان → کلید باید یک جا باشد، و آن جا همان‌جایی است که فهرست کلیدهای
//!     نوشتنی هم زندگی می‌کند.
//!
//! دستاورد سومی هم دارد که برای پورتِ UI تنظیمات (تصویر a2) مستقیم لازم است:
//! **ترتیب و گروه‌بندیِ صفحات تنظیمات** همین‌جا اعلام می‌شود، پس رابط کاربری و
//! دستیار نمی‌توانند دو روایت متفاوت از ساختار تنظیمات داشته باشند.

use serde::Serialize;

/// یک گروه در صفحهٔ اصلی تنظیمات — همان کارت‌های تصویر a2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Group {
    /// «Tunnel» — اتصال، ترابرد و ضد‌DPI، DNS و مسیریابی، پروکسی بالادست.
    Tunnel,
    /// «Security» — سوییچ قطع، گارد نشتی، حفاظت IPv6، Zero Trust.
    Security,
    /// «Application» — زبان، شروع خودکار، اشتراک شبکه.
    Application,
    /// «Diagnostics» — لاگ، خودآزمایی، دربارهٔ برنامه.
    Diagnostics,
}

/// یک موضوعِ قابل‌پرسش.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    /// شناسهٔ پایدار — چیزی که فرانت‌اند می‌فرستد.
    pub id: &'static str,
    /// کلیدِ پروفایل که این موضوع درباره‌اش است، یا `""` برای موضوع‌های بی‌فیلد.
    pub key: &'static str,
    /// عنوان انگلیسی، همان‌طور که در رابط کاربری دیده می‌شود.
    pub title: &'static str,
    /// یک‌خط توضیح، همان زیرنویس زیر عنوان در a2.
    pub subtitle: &'static str,
    pub group: Group,
}

/// همهٔ موضوع‌ها، به ترتیبی که در رابط کاربری ظاهر می‌شوند.
pub const TOPICS: &[Topic] = &[
    // --- Tunnel
    Topic {
        id: "backend",
        key: "backend",
        title: "Connection",
        subtitle: "Network backend, exit country, protocol and scanning",
        group: Group::Tunnel,
    },
    Topic {
        id: "exitRegion",
        key: "exitRegion",
        title: "Exit country",
        subtitle: "Which country the Psiphon stage leaves from",
        group: Group::Tunnel,
    },
    Topic {
        id: "protocol",
        key: "protocol",
        title: "Protocol",
        subtitle: "WireGuard or MASQUE, or let the app choose",
        group: Group::Tunnel,
    },
    Topic {
        id: "scanMode",
        key: "scanMode",
        title: "Endpoint scanning",
        subtitle: "How hard the app looks for a reachable endpoint",
        group: Group::Tunnel,
    },
    Topic {
        id: "transport",
        key: "noize",
        title: "Transport & anti-DPI",
        subtitle: "Obfuscation, endpoint, MTU and anti-DPI",
        group: Group::Tunnel,
    },
    Topic {
        id: "endpointMode",
        key: "endpointMode",
        title: "Endpoint",
        subtitle: "Automatic, scanned, or an address you type yourself",
        group: Group::Tunnel,
    },
    Topic {
        id: "mtu",
        key: "mtu",
        title: "MTU",
        subtitle: "Largest packet the tunnel will send",
        group: Group::Tunnel,
    },
    Topic {
        id: "keepalive",
        key: "keepalive",
        title: "Keepalive",
        subtitle: "How often a heartbeat keeps the path open",
        group: Group::Tunnel,
    },
    Topic {
        id: "fragment",
        key: "fragment",
        title: "Fragmentation",
        subtitle: "Splits the first packets so DPI cannot read them whole",
        group: Group::Tunnel,
    },
    Topic {
        id: "ech",
        key: "ech",
        title: "Encrypted Client Hello",
        subtitle: "Hides the server name during the TLS handshake",
        group: Group::Tunnel,
    },
    Topic {
        id: "dns",
        key: "dns",
        title: "DNS & routing rules",
        subtitle: "Resolvers inside the tunnel, block and bypass lists",
        group: Group::Tunnel,
    },
    Topic {
        id: "upstream",
        key: "upstream",
        title: "Upstream proxy (chaining)",
        subtitle: "Dial out through a proxy already running on this PC",
        group: Group::Tunnel,
    },
    // --- Security
    Topic {
        id: "killSwitch",
        key: "killSwitch",
        title: "Kill switch",
        subtitle: "Cuts direct traffic the moment the tunnel drops",
        group: Group::Security,
    },
    Topic {
        id: "leakGuard",
        key: "leakGuard",
        title: "WebRTC leak guard",
        subtitle: "Stops raw UDP from revealing your real address",
        group: Group::Security,
    },
    Topic {
        id: "ipv6Protection",
        key: "ipv6Protection",
        title: "IPv6 protection",
        subtitle: "Keeps public IPv6 off the unprotected path",
        group: Group::Security,
    },
    Topic {
        id: "zeroTrust",
        key: "team",
        title: "Zero Trust",
        subtitle: "Join a Cloudflare organisation instead of plain WARP",
        group: Group::Security,
    },
    Topic {
        id: "reconnectAttempts",
        key: "reconnectAttempts",
        title: "Reconnect attempts",
        subtitle: "How many times the app retries by itself",
        group: Group::Security,
    },
    // --- Application
    Topic {
        id: "lanShare",
        key: "lanShare",
        title: "Share on the local network",
        subtitle: "Lets other devices on this network use the tunnel",
        group: Group::Application,
    },
    Topic {
        id: "splitMode",
        key: "splitMode",
        title: "Split tunnelling",
        subtitle: "Choose which programs use the tunnel",
        group: Group::Application,
    },
    Topic {
        id: "language",
        key: "",
        title: "Language",
        subtitle: "Interface language",
        group: Group::Application,
    },
    // --- Diagnostics
    Topic {
        id: "logs",
        key: "",
        title: "Connection log",
        subtitle: "What the engine reported, newest last",
        group: Group::Diagnostics,
    },
    Topic {
        id: "selfTest",
        key: "",
        title: "Self-test",
        subtitle: "Checks the parts the tunnel needs before it starts",
        group: Group::Diagnostics,
    },
];

/// یک موضوع را با شناسه‌اش پیدا می‌کند.
pub fn find(id: &str) -> Option<&'static Topic> {
    TOPICS.iter().find(|t| t.id == id)
}

/// موضوع‌های یک گروه، به ترتیبِ اعلام‌شده.
pub fn in_group(group: Group) -> impl Iterator<Item = &'static Topic> {
    TOPICS.iter().filter(move |t| t.group == group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut seen: Vec<&str> = Vec::new();
        for topic in TOPICS {
            assert!(
                !seen.contains(&topic.id),
                "duplicate topic id: {}",
                topic.id
            );
            seen.push(topic.id);
        }
    }

    #[test]
    fn every_field_backed_topic_names_a_real_desktop_profile_key() {
        // این تست همان لغزشی را می‌گیرد که در `ai_patch` هم گرفته شد: یک نامِ
        // فیلدِ اندرویدی که اینجا کپی شده باشد، ساکت می‌ماند تا اولین کاربری که
        // روی آیکن دستیار بزند و توضیحی دربارهٔ فیلدی بگیرد که وجود ندارد.
        let profile = serde_json::to_value(crate::profile::ConnectionProfile::default()).unwrap();
        let object = profile.as_object().unwrap();
        for topic in TOPICS {
            if topic.key.is_empty() {
                continue;
            }
            assert!(
                object.contains_key(topic.key),
                "unknown profile key \"{}\" on topic \"{}\"",
                topic.key,
                topic.id
            );
        }
    }

    #[test]
    fn the_groups_are_all_populated() {
        for group in [
            Group::Tunnel,
            Group::Security,
            Group::Application,
            Group::Diagnostics,
        ] {
            assert!(in_group(group).count() > 0, "empty group: {group:?}");
        }
    }
}
