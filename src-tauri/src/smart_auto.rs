//! پورت از `core/SmartAuto.kt` + منطق نردبان `AetherVpnService.directPlan/buildPlan`.
//!
//! ریشهٔ باگ قبلی دسکتاپ: فقط «یک» پروتکل انتخاب می‌شد و هیچ نردبان
//! تلاشِ چندمرحله‌ای وجود نداشت؛ در اندروید هر اتصال یک «برنامه» چند
//! کاندیدایی است که یکی‌یکی امتحان می‌شوند تا اولینِ قبول‌شده در خودآزما
//! برنده شود. همان منطق این‌جا پیاده شده:
//!
//!  * پروتکل دستی  ← دو پاس (معادل directPlan): اول همان تنظیمات کاربر
//!    (سقف ۷۵ ثانیه)، بعد پاس ضد-DPI سخت‌شده — پروتکل هرگز عوض نمی‌شود.
//!  * Smart Auto ← نردبان MASQUE → MASQUE سخت‌شده → GOOL → WireGuard
//!    (همان ترتیب ترجیح SmartAuto.kt).

use crate::log::DiagnosticsLog;
use crate::profile::{ConnectionProfile, GoolMode, IpVersion, Noize, Protocol, ScanMode, TorMode};

const TAG: &str = "auto";

/// سقف پاس اول — همان `FIRST_PASS_MAX_MS` اندروید. تعریفش در `budgets.rs`
/// است، کنارِ رزروِ راه‌اندازیِ موتور که با آن یک حساب می‌سازد.
use crate::budgets::FIRST_PASS_MAX_MS;

/// ترتیب ترجیح — همان ترتیبی که SmartAuto.kt دارد.
/// ترتیبِ پیش‌فرض وقتی UDP از کار افتاده — MASQUE روی TCP جلو می‌افتد و
/// WireGuard که بی UDP اصلاً کار نمی‌کند، آخر می‌ماند.
const PREFERENCE: [Protocol; 3] = [Protocol::Masque, Protocol::Gool, Protocol::Wireguard];

/// شکلِ فیلترینگِ این شبکه — همان `DpiClass` در `SmartAuto.kt`.
///
/// دسکتاپ تا پیش از این چنین چیزی نداشت و ترتیبِ نردبانش **ثابت** بود:
/// MASQUE، GOOL، WireGuard. یعنی روی شبکه‌ای هم که UDP در آن سالم است و
/// اندروید با WireGuard در ۵ ثانیه وصل می‌شود، دسکتاپ اول ۳۵ ثانیه MASQUE
/// «همان‌طور که تنظیم شده»، بعد ۱۲۰ ثانیه MASQUEِ سخت‌شده، بعد پلهٔ HTTP/2 و
/// بعد GOOL را می‌رفت — و WireGuard عملاً پس از چند دقیقه به نوبت می‌رسید.
/// WireGuard «ناموفق» شمرده نمی‌شد؛ هیچ‌وقت امتحان نمی‌شد.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpiClass {
    /// UDP جواب می‌دهد و خروجِ مستقیم باز است — مسیرِ تقریباً پاک.
    Open,
    /// UDP سالم است ولی خروجِ مستقیم بسته — فیلترینگ روی TCP/SNI.
    SniFiltering,
    /// خروجِ مستقیم باز است ولی UDP جواب نمی‌دهد — QUIC و WireGuard گرسنه
    /// می‌مانند و ترابردِ TCP-شکل راهِ ورود است.
    UdpThrottled,
    /// هر دو خراب.
    Hostile,
}

impl NetFingerprint {
    /// نگاشتِ دو سنجهٔ دسکتاپ به همان چهار کلاسِ موبایل.
    pub fn dpi_class(self) -> DpiClass {
        match (self.udp_ok, self.filtered) {
            (true, false) => DpiClass::Open,
            (true, true) => DpiClass::SniFiltering,
            (false, false) => DpiClass::UdpThrottled,
            (false, true) => DpiClass::Hostile,
        }
    }
}

/// ترتیبِ پروتکل‌ها برای این شبکه — عیناً نردبانِ `SmartAuto.kt`.
fn preference_for(fp: NetFingerprint) -> [Protocol; 3] {
    match fp.dpi_class() {
        // روی شبکهٔ باز، WireGuard سریع‌ترین است و اندروید هم با همین شروع
        // می‌کند. این تنها تغییری است که «چند دقیقه» را به «چند ثانیه»
        // برمی‌گردانَد.
        DpiClass::Open => [Protocol::Wireguard, Protocol::Masque, Protocol::Gool],
        // UDP سالم است، پس WireGuard هنوز بهترین شانس است؛ MASQUE که روی
        // TLS/SNI حساس‌تر است، پس از GOOL می‌آید.
        DpiClass::SniFiltering => [Protocol::Wireguard, Protocol::Gool, Protocol::Masque],
        // بی UDP، WireGuard آخر است — همان ترتیبی که دسکتاپ همیشه داشت.
        DpiClass::UdpThrottled | DpiClass::Hostile => PREFERENCE,
    }
}

/// What the pre-connect probes learned about this network.
///
/// Until 1.2.3-p2 this was a single `hostile: bool` derived from one TCP:80
/// dial, so the planner had no idea whether UDP worked - and the MASQUE carrier
/// choice is entirely a question about UDP. See [`harden`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetFingerprint {
    /// Direct TCP egress looks blocked: lead with the hardened anti-DPI pass.
    pub filtered: bool,
    /// A real UDP round trip completed. This is the signal for WireGuard.
    /// It must not be replaced by the QUIC result: UDP:53 can work while
    /// Cloudflare QUIC on UDP:443 is filtered (as in the field log).
    pub udp_ok: bool,
    /// A Cloudflare QUIC version-negotiation response was received. This only
    /// selects MASQUE's carrier; it does not decide whether WireGuard may run.
    pub quic_ok: bool,
}

impl Default for NetFingerprint {
    fn default() -> Self {
        // Absent evidence, assume UDP works: HTTP/3 is the fast carrier and the
        // ladder still falls through to HTTP/2 if the attempt fails.
        Self {
            filtered: false,
            udp_ok: true,
            quic_ok: true,
        }
    }
}

/// معادل `AutoCandidate` اندروید — یک استراتژی آمادهٔ اجرا.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub profile: ConnectionProfile,
    pub timeout_ms: u64,
    pub label: String,
}

/// Whether a protocol's data plane is the MASQUE carrier, i.e. whether the
/// HTTP/3-vs-HTTP/2 choice applies to it at all.
///
/// ## 1.2.6: `MASQUE×2` belongs here too
///
/// The three helpers below used to test `== Protocol::Masque`, which was right
/// while MASQUE was the only protocol on that carrier. `Protocol::Mim`
/// (`--mim`, ported from mobile 1.3.0) is two MASQUE hops, and the core runs
/// **both** of them on whatever carrier `AETHER_MASQUE_HTTP2` names -- its
/// `Protocol::MasqueInMasque` arm calls `select_masque_transport()` exactly like
/// the single-hop arm, and the desktop always sends that variable. So on the
/// network in the field log -- where the probe proved
///
/// ```text
///   netprobe: UDP works but no Cloudflare edge answered QUIC on UDP:443
/// ```
///
/// -- a hand-picked `MASQUE×2` would have dialled QUIC twice on a network where
/// QUIC answers once: the outer hop can never come up, so the inner hop is never
/// reached. Same measurement, same conclusion, so the same switch has to apply.
fn rides_masque_carrier(protocol: Protocol) -> bool {
    matches!(protocol, Protocol::Masque | Protocol::Mim)
}

/// ساخت نسخهٔ سخت‌شدهٔ ضد-DPI — معادل پاس دوم `directPlan` اندروید.
///
/// ## 1.2.3-p2: hardening no longer means "make the download slow"
///
/// This used to set `masque_http2 = true` unconditionally. That one line is how
/// almost every desktop session ended up on the HTTP/2 carrier: MASQUE is the
/// first rung of the Smart Auto ladder, so a single flaky first attempt promoted
/// the hardened pass, the hardened pass demoted the carrier from HTTP/3 to
/// HTTP/2, and the session then stayed on the one data plane with a 64 KB
/// flow-control window - for the rest of its life, on a network where UDP was
/// perfectly healthy.
///
/// The mobile build never did this: `SmartAuto.kt` only passes `h2 = true` in
/// the `UDP_THROTTLED` and `HOSTILE` branches, i.e. only after a real UDP probe
/// has failed. Same rule here now. A user who ticked the toggle themselves is
/// still honoured.
fn harden(p: &ConnectionProfile, fp: NetFingerprint) -> ConnectionProfile {
    let mut h = p.clone();
    if h.noize == Noize::Off {
        h.noize = Noize::Firewall;
    }
    if rides_masque_carrier(h.protocol) {
        h.masque_http2 = p.masque_http2 || !fp.quic_ok;
        h.fragment = true;
        // >>> AETHER-APP-FIX ech-only-where-sni-is-filtered
        // Mobile parity (`SmartAuto.kt`): ECH is part of the MASQUE rung only in
        // SNI_FILTERING / UDP_THROTTLED / HOSTILE — never on an OPEN network.
        // Field log 2026-10-08 (OPEN): the hardened MASQUE rung was started with
        // `--ech auto`, the engine could not fetch an ECH key ("udp://1.1.1.1:53
        // did not answer") and exited after 12 s — a rung that could not work.
        h.ech = p.ech || fp.dpi_class() != DpiClass::Open;
        // <<< AETHER-APP-FIX ech-only-where-sni-is-filtered
    }
    h
}

/// The profile with the MASQUE carrier forced onto HTTP/2 when the pre-connect
/// probe proved QUIC cannot work on this network.
///
/// ## 1.2.3-p3: the FIRST rung must not ride a carrier already known to be dead
///
/// [`harden`] has done this since p2, but only for the hardened pass. The plain
/// "as configured" rung - the first thing the ladder tries, and the one that owns
/// the first-pass window - kept riding HTTP/3 even after the probe had proved
/// QUIC was filtered. On the network in the field log that guaranteed the first
/// 35 seconds of every connect went to a rung that could not possibly succeed,
/// and because a filtered network leads with the hardened pass, the second rung
/// then burnt another 60 seconds exactly the same way. Three quarters of the
/// connect time the user complained about was spent on carriers the app had
/// already measured as unusable.
///
/// This only ever turns the HTTP/2 carrier ON. A user who ticked the toggle
/// themselves is untouched, and on a healthy-UDP network nothing changes at all.
fn carrier_for(p: &ConnectionProfile, fp: NetFingerprint) -> ConnectionProfile {
    let mut out = p.clone();
    if rides_masque_carrier(out.protocol) && !fp.quic_ok {
        out.masque_http2 = true;
    }
    out
}

/// Carrier suffix for a label, so the log says which data plane is being tried.
fn carrier(p: &ConnectionProfile) -> &'static str {
    if rides_masque_carrier(p.protocol) && p.masque_http2 {
        " · h2"
    } else if rides_masque_carrier(p.protocol) {
        " · h3"
    } else {
        ""
    }
}

// >>> AETHER-APP-FIX plain-pass-goes-last-on-a-filtered-network
/// بودجهٔ یک پلهٔ «آخرین‌شانس» — تلاشی که دلیل داریم شکست می‌خورد ولی حذفش
/// هم درست نیست، چون انگشت‌نگاری شبکه قطعی نیست.
///
/// ۲۰ ثانیه یعنی: اگر انگشت‌نگاری اشتباه کرده بود و مسیر ساده کار می‌کند،
/// در همین چند ثانیهٔ اول دست می‌دهد؛ و اگر نه، کاربر ۷۵ ثانیه پای دکمه
/// نمی‌نشیند.
const LAST_RESORT_MS: u64 = 20_000;

fn last_resort(mut c: Candidate) -> Candidate {
    c.timeout_ms = c.timeout_ms.min(LAST_RESORT_MS);
    c
}
// <<< AETHER-APP-FIX plain-pass-goes-last-on-a-filtered-network

/// معادل `directPlan` — پروتکل دستی، دو پاس، بدون تعویض پروتکل.
fn direct_plan(user: &ConnectionProfile, fp: NetFingerprint) -> Vec<Candidate> {
    let full = user.connect_timeout_ms();
    let hostile = fp.filtered;
    // 1.2.3-p3: the plain pass rides the carrier the probe says works, not the
    // one the panel happens to default to. See [`carrier_for`].
    let plain = carrier_for(user, fp);
    let hardened = harden(&plain, fp);
    let name = format!("{:?}", user.protocol).to_uppercase();
    if hardened == plain {
        return vec![Candidate {
            timeout_ms: full,
            label: format!("{name} · as configured{}", carrier(&plain)),
            profile: plain,
        }];
    }
    let as_configured = Candidate {
        timeout_ms: full.min(FIRST_PASS_MAX_MS),
        label: format!("{name} · as configured{}", carrier(&plain)),
        profile: plain,
    };
    let anti_dpi = Candidate {
        label: format!("{name} · hardened anti-DPI{}", carrier(&hardened)),
        profile: hardened,
        timeout_ms: full,
    };
    if hostile {
        // >>> AETHER-APP-FIX plain-pass-goes-last-on-a-filtered-network
        // پیش‌تر اینجا `vec![anti_dpi, as_configured]` بود: پاسِ ساده دوم می‌ماند
        // ولی بودجه‌اش همان `FIRST_PASS_MAX_MS` یعنی **۷۵ ثانیه** بود. روی شبکه‌ای
        // که انگشت‌نگاری می‌گوید فیلتر است، این ۷۵ ثانیه خرجِ تلاشی می‌شود که
        // دلیل داریم شکست می‌خورد — همان معطلی‌ای که کاربر از اختلافِ
        // ویندوز و موبایل گزارش کرد.
        //
        // پاسِ ساده حذف نمی‌شود — انگشت‌نگاری می‌تواند منفیِ کاذب بدهد — ولی
        // می‌رود به انتهای نردبان با بودجهٔ کوتاه: اگر درست باشد، زود جواب
        // می‌دهد؛ اگر نه، ۷۵ ثانیه از کاربر نمی‌گیرد.
        vec![anti_dpi, last_resort(as_configured)]
        // <<< AETHER-APP-FIX plain-pass-goes-last-on-a-filtered-network
    } else {
        vec![as_configured, anti_dpi]
    }
}

/// نردبان Smart Auto — معادل `SmartAuto.buildPlan`.
fn auto_plan(user: &ConnectionProfile, fp: NetFingerprint) -> Vec<Candidate> {
    // TURBO per attempt, exactly as SmartAuto.kt does it: the ladder's speed
    // comes from trying the NEXT strategy quickly, not from one long exhaustive
    // scan. The budget is then read off the rung's OWN profile, so the number
    // and the scan it pays for can never drift apart.
    let mut rung_base = user.clone();
    rung_base.scan_mode = ScanMode::Turbo;
    let user = &rung_base;
    let full = user.connect_timeout_ms();
    let hostile = fp.filtered;
    let mut plan = Vec::new();

    // فقط IPv6: WireGuard پایدارتر است — همان قاعدهٔ اندروید.
    let order: Vec<Protocol> = if user.ip_version == IpVersion::V6 {
        vec![Protocol::Wireguard, Protocol::Masque, Protocol::Gool]
    } else {
        preference_for(fp).to_vec()
    };
    DiagnosticsLog::i(
        TAG,
        &format!(
            "Ladder order for {:?}: {}",
            fp.dpi_class(),
            order
                .iter()
                .map(|p| format!("{p:?}").to_uppercase())
                .collect::<Vec<_>>()
                .join(" -> ")
        ),
    );

    for (i, proto) in order.iter().enumerate() {
        let mut base = user.clone();
        base.protocol = *proto;
        // Same rule as `direct_plan`: never spawn a rung on a carrier the
        // pre-connect probe already proved cannot carry traffic here.
        let base = carrier_for(&base, fp);
        let name = format!("{proto:?}").to_uppercase();
        if i == 0 {
            let as_configured = Candidate {
                label: format!("{name} · as configured{}", carrier(&base)),
                profile: base.clone(),
                timeout_ms: full.min(FIRST_PASS_MAX_MS),
            };
            let hardened = harden(&base, fp);
            let anti_dpi = Candidate {
                label: format!("{name} · hardened anti-DPI{}", carrier(&hardened)),
                profile: hardened,
                timeout_ms: full.min(120_000),
            };
            if hostile {
                plan.push(anti_dpi);
                // >>> AETHER-APP-FIX plain-pass-goes-last-on-a-filtered-network
                // معطلیِ لاگ از **بودجه** می‌آمد، نه از جایگاه: این پله با
                // `FIRST_PASS_MAX_MS` یعنی **۷۵ ثانیه** می‌گرفت، روی شبکه‌ای که
                // انگشت‌نگاری می‌گوید فیلتر است. حالا همان جا می‌ماند ولی
                // `LAST_RESORT_MS` می‌گیرد.
                //
                // در نسخهٔ قبلیِ این اصلاح، این پله را به تهِ نردبان موکول
                // کردم و قراردادِ مستندِ نردبان را شکستم: در SNI_FILTERING
                // باید آخرین پله MASQUE باشد — تست
                // `sni_filtering_keeps_wireguard_first_and_pushes_masque_last`
                // در CI همین را گرفت (left: Wireguard, right: Masque). ترتیبِ
                // پروتکل‌ها تصمیمِ دیگری است و این اصلاح حق ندارد به آن
                // دست بزند.
                plan.push(last_resort(as_configured));
                // <<< AETHER-APP-FIX plain-pass-goes-last-on-a-filtered-network
            } else {
                plan.push(as_configured);
                plan.push(anti_dpi);
            }
            // 1.2.3-p2: on a healthy-UDP network the two MASQUE rungs above both
            // ride HTTP/3, so the ladder would never reach the TCP carrier at
            // all if QUIC turned out to be blocked mid-scan rather than at probe
            // time. One explicit HTTP/2 rung keeps that escape hatch, at the
            // END, where it belongs - instead of being the second thing tried.
            if *proto == Protocol::Masque && fp.udp_ok && !user.masque_http2 {
                let mut h2 = harden(&base, fp);
                h2.masque_http2 = true;
                plan.push(Candidate {
                    label: format!("{name} · hardened anti-DPI{}", carrier(&h2)),
                    profile: h2,
                    timeout_ms: full.min(120_000),
                });
            }
        } else {
            let hardened = harden(&base, fp);
            plan.push(Candidate {
                label: format!("{name} · hardened anti-DPI{}", carrier(&hardened)),
                profile: hardened,
                timeout_ms: if i + 1 == order.len() {
                    full
                } else {
                    full.min(120_000)
                },
            });
        }
    }
    plan
}

/// نقطهٔ ورود: برنامهٔ کامل اتصال برای پروفایل کاربر.
pub fn build_plan(user: &ConnectionProfile, fp: NetFingerprint) -> Vec<Candidate> {
    // پیش از هر چیز: حالت‌هایی که ترافیک دستگاه از تور بیرون می‌رود، نردبان
    // ندارند. اثرانگشتِ شبکه هم برایشان بی‌معنی است — چیزی که آن‌جا شکست
    // می‌خورد یا موفق می‌شود، bootstrapِ تور است نه یک نقطهٔ پایانی WARP.
    if let Some(plan) = tor_plan(user) {
        let summary: Vec<String> = plan.iter().map(|c| c.label.clone()).collect();
        DiagnosticsLog::i(
            TAG,
            &format!("Plan ready (1 attempt): {}", summary.join(" \u{2192} ")),
        );
        return plan;
    }
    if fp.filtered {
        DiagnosticsLog::w(
            TAG,
            "Network fingerprint: this network looks filtered - anti-DPI attempts run first.",
        );
    }
    // Say it out loud: this single bit decides HTTP/3 (QUIC) versus HTTP/2 (TCP)
    // for the MASQUE carrier, and the two have very different throughput.
    DiagnosticsLog::i(
        TAG,
        &format!(
            "Network fingerprint: udp={} → MASQUE carrier prefers {}",
            if fp.udp_ok { "ok" } else { "blocked/throttled" },
            if fp.quic_ok {
                "HTTP/3 (QUIC)"
            } else {
                "HTTP/2 (TCP)"
            },
        ),
    );
    let plan = if user.protocol == Protocol::Smart {
        DiagnosticsLog::i(TAG, "Smart Auto: building the strategy ladder…");
        auto_plan(user, fp)
    } else {
        direct_plan(user, fp)
    };
    let summary: Vec<String> = plan.iter().map(|c| c.label.clone()).collect();
    DiagnosticsLog::i(
        TAG,
        &format!(
            "Plan ready ({} attempt(s)): {}",
            plan.len(),
            summary.join(" → ")
        ),
    );
    plan
}

/// بودجهٔ یک تلاشِ تور-جلو — معادل `torBudget` اندروید.
///
/// وقتی پل مجاز است، بودجه باید جای fallbackِ خودِ هسته روی پل‌ها را هم داشته
/// باشد؛ وگرنه بودجهٔ سادهٔ bootstrap. آنچه این بودجهٔ بلند را به یک سکوتِ بلند
/// تبدیل نمی‌کند، شناساگرِ گیرکردن در `state.rs` است.
fn tor_budget(p: &ConnectionProfile) -> u64 {
    if p.tor_bridges != crate::profile::TorBridges::Off {
        crate::diagnostics::TOR_BRIDGE_BUDGET_MS
    } else {
        crate::diagnostics::TOR_BOOTSTRAP_BUDGET_MS
    }
}

/// نردبانِ حالت‌های تور، یا `None` وقتی این نشست تور-جلو نیست.
///
/// # چرا نردبانِ معمول اینجا **زیان‌آور** است
///
/// نردبان از یک فرض ساخته شده: هر پله یک ترابردِ دیگر برای رسیدن به یک نقطهٔ
/// پایانی است، پس پلهٔ شکست‌خورده چیزی دربارهٔ شبکه می‌گوید. در حالت‌های تور آن
/// فرض می‌شکند، و هر بار به شکل خودش:
///
/// * `--tor-only` هیچ تونلی بالا نمی‌آورد؛ نه نقطهٔ پایانی‌ای برای اسکن هست، نه
///   ترابردی برای مبهم‌سازی. هر سه پله **همان** فراخوانیِ هسته می‌شدند و تنها
///   کاری که می‌کردند این بود که bootstrapِ نیمه‌تمامِ تور را بکشند و از صفر
///   شروع کنند — دقیقاً همان‌جایی که تور به وقت نیاز دارد.
/// * زنجیرهٔ برعکس (`Tor → Aether`) تنها یک ترابرد دارد: هسته در این حالت
///   `--wg` و `--gool` را رد می‌کند، پس پله‌های WireGuard و Gool در یک چشم‌به‌هم‌زدن
///   شکست می‌خوردند و بودجه‌ای را می‌سوزاندند که مالِ bootstrap بود. MASQUE هم
///   اجباراً روی HTTP/2 است، چون UDP از تور رد نمی‌شود.
///
/// زنجیرهٔ عادی (`Aether → Tor`) عمداً اینجا **نیست**: آنجا استیج ۱ یک تونل
/// کاملِ WARP است و تور بعد از آن و از داخلش راه می‌افتد، پس همان اثرانگشت‌زنی
/// و همان سخت‌سازیِ ضد-DPI که برای هر نشست دیگری درست است، برای آن هم درست است.
/// بودجهٔ تورِ آن نشست جای دیگری است: دروازهٔ `tor_gate` در `state.rs`.
fn tor_plan(user: &ConnectionProfile) -> Option<Vec<Candidate>> {
    match user.backend.tor_mode() {
        Some(TorMode::Only) => {
            DiagnosticsLog::i(
                TAG,
                "Tor only: no tunnel to scan — one attempt, on a bootstrap-sized budget.",
            );
            Some(vec![Candidate {
                profile: user.clone(),
                timeout_ms: tor_budget(user),
                label: "Tor \u{b7} direct or via bridges".to_string(),
            }])
        }
        Some(TorMode::Reverse) => {
            DiagnosticsLog::i(
                TAG,
                "Tor \u{2192} Dopamine: MASQUE over HTTP/2 is the only transport Tor can carry \
                 — one attempt, on a bootstrap-sized budget.",
            );
            // >>> AETHER-APP-PATCH tor-reverse-short-scan (port of mobile 1.4.0 `reversePlan`)
            // تا اینجا فقط ترابرد قفل می‌شد. موبایل ۱.۴.۰ نشان داد این کافی نیست:
            // لاگِ میدانی `scan mode=turbo candidates=896 … budget=45s` از داخلِ
            // یک مدارِ تور بود، با بازه‌هایی که اثرانگشتِ DPI روی مسیرِ *مستقیم*
            // ساخته بود. آنچه یک خروجیِ تور می‌تواند برسد هیچ ربطی به آن بازه‌ها
            // ندارد. پس سه چیز این‌جا پین می‌شود، دقیقاً مثل موبایل:
            //   * Turbo — کوتاه‌ترین اسکنِ موتور: اولین پاسخ برنده است و
            //     گیت‌وی‌های شناخته‌شدهٔ کلادفلر (MASQUE_SEEDS) اول پروب می‌شوند؛
            //   * endpoint_mode = Auto و manual_range خالی — بازه‌های مسیرِ مستقیم
            //     به اسکنِ داخلِ تور نمی‌رسند؛
            //   * بودجهٔ هم‌اندازهٔ bootstrapِ تور (پایین‌تر، `tor_budget`).
            // یک peerِ دستی هم عمداً کنار می‌رود: همان دلیل، یک نشانیِ اندازه‌گیری‌شده
            // روی مسیرِ مستقیم.
            let mut p = user.clone();
            p.protocol = Protocol::Masque;
            p.masque_http2 = true;
            p.scan_mode = ScanMode::Turbo;
            p.endpoint_mode = crate::profile::EndpointMode::Auto;
            p.manual_range = String::new();
            p.manual_peer = String::new();
            // <<< AETHER-APP-PATCH tor-reverse-short-scan
            Some(vec![Candidate {
                profile: p,
                timeout_ms: tor_budget(user),
                label: "MASQUE \u{b7} h2 \u{b7} through Tor".to_string(),
            }])
        }
        _ => None,
    }
}

/// معادل `SmartAuto.choose()` — برای سازگاری با کد/تست‌های قبلی حفظ شده.
pub fn pick(profile: &ConnectionProfile) -> Protocol {
    if profile.protocol != Protocol::Smart {
        return profile.protocol;
    }
    if profile.has_manual_peer() {
        return Protocol::Masque;
    }
    if profile.ip_version == IpVersion::V6 {
        return Protocol::Wireguard;
    }
    PREFERENCE[0]
}

/// ترتیب تلاش مجدد پس از شکست — معادل `nextCandidate()`.
pub fn next_after(failed: Protocol) -> Option<Protocol> {
    let idx = PREFERENCE.iter().position(|p| *p == failed)?;
    PREFERENCE.get(idx + 1).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::TransportBackend;

    /// نردبان باید Turbo بدهد در حالی که پروفایلِ ذخیره‌شده Balanced می‌ماند —
    /// همان چیدمانِ موبایل ۱.۳.۰ (`SmartAuto.kt`: «TURBO per attempt»).
    ///
    /// دو سنجه دارد و هر دو لازم است: اگر فقط حالتِ اسکن سنجیده شود، کسی می‌تواند
    /// آن را بگذارد و بودجه را از پروفایلِ کاربر بخواند؛ و اگر فقط بودجه سنجیده
    /// شود، عددِ درست می‌ماند در حالی که موتور اسکنِ دیگری اجرا می‌کند.
    #[test]
    fn every_ladder_rung_scans_in_turbo_while_the_profile_stays_balanced() {
        let user = ConnectionProfile::default();
        assert_eq!(
            user.scan_mode,
            ScanMode::Balanced,
            "پیش‌فرضِ ذخیره‌شده باید مثل موبایل باشد"
        );
        let plan = build_plan(&user, NetFingerprint::default());
        assert!(plan.len() > 1, "نردبان باید چند پله داشته باشد، نه یکی");
        for c in &plan {
            assert_eq!(c.profile.scan_mode, ScanMode::Turbo, "{}", c.label);
            assert!(
                c.timeout_ms <= 60_000,
                "{} بودجهٔ {} گرفت — یعنی از پروفایلِ کاربر خوانده شده نه از پلهٔ خودش",
                c.label,
                c.timeout_ms
            );
        }
    }

    /// پروتکلِ دستی = بی نردبان. آن‌جا حالتِ اسکنِ خودِ کاربر می‌ماند و بودجهٔ
    /// کاملِ ۱۵۰ ثانیه می‌آید — همان کاری که `directPlan` موبایل می‌کند. پیش از
    /// ۱.۲.۵ این مسیر در دسکتاپ فقط ۶۰ ثانیه می‌گرفت، چون پیش‌فرض Turbo بود.
    #[test]
    fn a_manual_protocol_keeps_the_users_own_scan_mode() {
        let user = ConnectionProfile {
            protocol: Protocol::Wireguard,
            gool_mode: GoolMode::Modern,
            ..Default::default()
        };
        let plan = build_plan(&user, NetFingerprint::default());
        assert!(!plan.is_empty());
        for c in &plan {
            assert_eq!(c.profile.scan_mode, ScanMode::Balanced, "{}", c.label);
        }
        assert_eq!(
            plan.last().unwrap().timeout_ms,
            150_000,
            "پاسِ آخرِ مسیرِ دستی باید بودجهٔ کاملِ حالتِ اسکنِ کاربر را بگیرد"
        );
    }

    #[test]
    fn auto_never_reaches_the_engine() {
        let p = ConnectionProfile::default();
        assert_ne!(pick(&p), Protocol::Smart);
        for c in build_plan(&p, NetFingerprint::default()) {
            assert_ne!(c.profile.protocol, Protocol::Smart);
        }
    }

    #[test]
    fn explicit_protocol_is_respected() {
        let p = ConnectionProfile {
            protocol: Protocol::Wireguard,
            gool_mode: GoolMode::Modern,
            ..Default::default()
        };
        assert_eq!(pick(&p), Protocol::Wireguard);
        for c in build_plan(&p, NetFingerprint::default()) {
            assert_eq!(c.profile.protocol, Protocol::Wireguard);
        }
    }

    /// `--tor-only` gets exactly one attempt. Three identical engine invocations
    /// would only kill a half-finished bootstrap twice and start over.
    #[test]
    fn tor_only_gets_one_attempt_on_a_bootstrap_budget() {
        let p = ConnectionProfile {
            backend: TransportBackend::Tor,
            protocol: Protocol::Smart,
            gool_mode: GoolMode::Modern,
            tor_bridges: crate::profile::TorBridges::Off,
            ..Default::default()
        };
        let plan = build_plan(&p, NetFingerprint::default());
        assert_eq!(plan.len(), 1, "tor-only must not walk a ladder");
        assert_eq!(
            plan[0].timeout_ms,
            crate::diagnostics::TOR_BOOTSTRAP_BUDGET_MS
        );
    }

    /// Bridges make the engine try several transports of its own, so the budget
    /// has to cover that — same rule as mobile's `torBudget`.
    #[test]
    fn permitted_bridges_widen_the_budget() {
        let p = ConnectionProfile {
            backend: TransportBackend::Tor,
            tor_bridges: crate::profile::TorBridges::Always,
            ..Default::default()
        };
        assert_eq!(
            build_plan(&p, NetFingerprint::default())[0].timeout_ms,
            crate::diagnostics::TOR_BRIDGE_BUDGET_MS,
        );
    }

    /// The reverse chain has ONE usable transport. A WireGuard or Gool rung here
    /// is refused by the engine instantly and burns Tor's budget, so the plan
    /// must contain neither — and its MASQUE must be on HTTP/2, because Tor
    /// carries no UDP.
    #[test]
    fn the_reverse_chain_never_plans_a_transport_tor_cannot_carry() {
        for protocol in [
            Protocol::Smart,
            Protocol::Wireguard,
            Protocol::Gool,
            Protocol::Masque,
        ] {
            let p = ConnectionProfile {
                backend: TransportBackend::TorAether,
                protocol,
                ..Default::default()
            };
            let plan = build_plan(
                &p,
                NetFingerprint {
                    filtered: true,
                    udp_ok: true,
                    quic_ok: true,
                },
            );
            assert_eq!(
                plan.len(),
                1,
                "reverse chain must not walk a ladder ({protocol:?})"
            );
            assert_eq!(plan[0].profile.protocol, Protocol::Masque);
            assert!(plan[0].profile.masque_http2, "UDP does not survive Tor");
        }
    }

    /// Mobile 1.4.0 `reversePlan`: through Tor the engine runs its SHORTEST scan,
    /// and nothing measured on the direct path (a pinned range or peer) may narrow
    /// it. The field log that motivated this scanned 896 candidates on two /24s
    /// from a direct-path DPI fingerprint, through one Tor circuit, in 45 s.
    #[test]
    fn the_reverse_chain_scans_turbo_and_drops_direct_path_ranges() {
        let p = ConnectionProfile {
            backend: TransportBackend::TorAether,
            scan_mode: ScanMode::Ironclad,
            endpoint_mode: crate::profile::EndpointMode::ManualRange,
            manual_range: "188.114.97.0/24".into(),
            manual_peer: "162.159.198.1:443".into(),
            ..Default::default()
        };
        let plan = build_plan(&p, NetFingerprint::default());
        assert_eq!(plan.len(), 1);
        let rung = &plan[0].profile;
        assert_eq!(rung.scan_mode, ScanMode::Turbo);
        assert_eq!(rung.endpoint_mode, crate::profile::EndpointMode::Auto);
        assert!(rung.manual_range.is_empty());
        assert!(!rung.has_manual_peer());
        assert!(!rung.to_env().contains_key("AETHER_SCAN_CIDRS"));
        assert!(rung.to_args().iter().any(|a| a == "--turbo"));
    }

    /// The ordinary chain is deliberately NOT gated: stage 1 there is a full WARP
    /// tunnel with Tor started afterwards and through it, so it wants the same
    /// fingerprinting and anti-DPI hardening as any other session. Its Tor budget
    /// lives in `state.rs`, not here.
    #[test]
    fn the_ordinary_tor_chain_keeps_the_full_ladder() {
        let plain = build_plan(
            &ConnectionProfile {
                protocol: Protocol::Smart,
                gool_mode: GoolMode::Modern,
                ..Default::default()
            },
            NetFingerprint::default(),
        );
        let chained = build_plan(
            &ConnectionProfile {
                backend: TransportBackend::AetherTor,
                protocol: Protocol::Smart,
                gool_mode: GoolMode::Modern,
                ..Default::default()
            },
            NetFingerprint::default(),
        );
        assert!(chained.len() > 1, "Dopamine → Tor lost its ladder");
        assert_eq!(
            plain.iter().map(|c| c.label.clone()).collect::<Vec<_>>(),
            chained.iter().map(|c| c.label.clone()).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn fallback_order_matches_android() {
        assert_eq!(next_after(Protocol::Masque), Some(Protocol::Gool));
        assert_eq!(next_after(Protocol::Gool), Some(Protocol::Wireguard));
        assert_eq!(next_after(Protocol::Wireguard), None);
    }

    /// 1.2.3-p3 regression guard. On a network where the QUIC probe failed,
    /// EVERY MASQUE rung has to be HTTP/2 - including the plain first pass,
    /// which is the one that owns the first-pass window.
    #[test]
    fn no_masque_rung_rides_quic_when_quic_is_dead() {
        let p = ConnectionProfile::default();
        let fp = NetFingerprint {
            filtered: false,
            udp_ok: false,
            quic_ok: false,
        };
        let plan = build_plan(&p, fp);
        let masque: Vec<&Candidate> = plan
            .iter()
            .filter(|c| c.profile.protocol == Protocol::Masque)
            .collect();
        assert!(!masque.is_empty());
        for c in &masque {
            assert!(
                c.profile.masque_http2,
                "rung `{}` still rides QUIC",
                c.label
            );
            assert!(
                c.label.ends_with(" · h2"),
                "label lies about the carrier: {}",
                c.label
            );
        }
    }

    /// The mirror image: on a healthy network every MASQUE rung rides HTTP/3,
    /// the fast carrier.
    ///
    /// تا ۱.۲.۵ این تست چیزِ دیگری می‌گفت — «پلهٔ اول MASQUE با h3 است» — و
    /// همان ایرادِ گزارش‌شده را به‌عنوان رفتارِ درست قفل کرده بود. حالا پلهٔ اولِ
    /// شبکهٔ باز WireGuard است (پایین‌تر سنجیده می‌شود) و ادعای واقعیِ این تست
    /// روی MASQUE باقی می‌ماند، هر جای نردبان که باشد.
    /// اثرانگشتِ دقیقِ لاگ میدانی: UDP کار می‌کند، QUIC فیلتر است.
    ///
    /// ```text
    ///   netprobe: UDP works but no Cloudflare edge answered QUIC on UDP:443
    ///   auto:     udp=blocked/throttled → Ladder for UdpThrottled: MASQUE -> GOOL -> WIREGUARD
    /// ```
    ///
    /// سطر دوم غلط بود و از همان یک بیت می‌آمد: نتیجهٔ QUIC به‌جای نتیجهٔ UDP
    /// نوشته می‌شد. در همان دستگاه، WireGuard دستی در ۳۳ ثانیه وصل شد
    /// (`wg endpoint 162.159.195.33:908`)، پس شبکه اصلاً UDP-blocked نبود.
    /// حالا دو سنجه جدا هستند: WireGuard اول می‌آید و MASQUE حاملِ HTTP/2
    /// می‌گیرد، نه اینکه کلِ شبکه UDP-مرده فرض شود.
    #[test]
    fn udp_alive_but_quic_filtered_still_leads_with_wireguard() {
        let fp = NetFingerprint {
            filtered: false,
            udp_ok: true,
            quic_ok: false,
        };
        assert_eq!(
            fp.dpi_class(),
            DpiClass::Open,
            "UDP سالم است، پس شبکه باز است"
        );

        let plan = build_plan(&ConnectionProfile::default(), fp);
        assert_eq!(
            plan[0].profile.protocol,
            Protocol::Wireguard,
            "پلهٔ اول باید WireGuard باشد، نه MASQUE — این همان باگ لاگ است",
        );

        // و هر پلهٔ MASQUE روی HTTP/2 می‌رود، چون QUIC اینجا واقعاً فیلتر است.
        let masque: Vec<&Candidate> = plan
            .iter()
            .filter(|c| c.profile.protocol == Protocol::Masque)
            .collect();
        assert!(!masque.is_empty(), "MASQUE باید در نردبان بماند");
        for rung in masque {
            assert!(
                rung.profile.masque_http2,
                "پلهٔ `{}` هنوز روی QUIC است",
                rung.label
            );
        }
    }

    /// `MASQUE×2` دستی هم باید حاملی بگیرد که پروب ثابت کرده کار می‌کند.
    ///
    /// روی شبکهٔ لاگ میدانی QUIC فیلتر است، پس دو هاپِ MASQUE روی QUIC یعنی
    /// هاپِ بیرونی هرگز بالا نمی‌آید و هاپِ درونی هرگز نوبت نمی‌گیرد. هستهٔ
    /// ۲.۰.۰ هر دو هاپ را روی همان `AETHER_MASQUE_HTTP2` می‌برد، پس همین سوییچ
    /// اینجا هم معنا دارد.
    #[test]
    fn a_hand_picked_two_hop_masque_also_avoids_a_dead_carrier() {
        let quic_dead = NetFingerprint {
            filtered: false,
            udp_ok: true,
            quic_ok: false,
        };
        let user = ConnectionProfile {
            protocol: Protocol::Mim,
            gool_mode: GoolMode::Modern,
            ..Default::default()
        };
        assert!(!user.masque_http2, "کاربر هیچ تیکی نزده است");

        let plan = build_plan(&user, quic_dead);
        assert!(!plan.is_empty());
        for rung in &plan {
            assert_eq!(
                rung.profile.protocol,
                Protocol::Mim,
                "پروتکل دستی عوض نمی‌شود"
            );
            assert!(
                rung.profile.masque_http2,
                "پلهٔ `{}` هنوز روی QUIC است",
                rung.label
            );
            assert!(
                rung.label.contains("· h2"),
                "برچسب حامل را نمی‌گوید: {}",
                rung.label
            );
        }

        // و روی شبکهٔ سالم هیچ‌چیز عوض نمی‌شود: QUIC حاملِ سریع‌تر است.
        let healthy = NetFingerprint {
            filtered: false,
            udp_ok: true,
            quic_ok: true,
        };
        for rung in build_plan(&user, healthy) {
            assert!(!rung.profile.masque_http2, "شبکهٔ سالم نباید به HTTP/2 برود");
            assert!(rung.label.contains("· h3"), "برچسب: {}", rung.label);
        }
    }

    #[test]
    fn healthy_udp_still_leads_with_quic() {
        let p = ConnectionProfile::default();
        let plan = build_plan(&p, NetFingerprint::default());
        let masque: Vec<_> = plan
            .iter()
            .filter(|c| c.profile.protocol == Protocol::Masque)
            .collect();
        assert!(!masque.is_empty(), "no MASQUE rung at all");
        for c in masque {
            assert!(
                !c.profile.masque_http2,
                "rung `{}` fell back to h2",
                c.label
            );
            assert!(
                c.label.ends_with(" · h3"),
                "label lies about the carrier: {}",
                c.label
            );
        }
    }

    /// ۱.۲.۵: ترتیبِ نردبان از شکلِ فیلترینگ می‌آید، نه از یک آرایهٔ ثابت.
    ///
    /// این همان چیزی است که کاربر گزارش کرد: روی شبکه‌ای که اندروید با
    /// WireGuard در ۵ ثانیه وصل می‌شود، دسکتاپ WireGuard را «نادیده می‌گیرد».
    /// دلیلش این بود که WireGuard در `PREFERENCE` همیشه آخر بود، پس پیش از
    /// رسیدن به آن باید ۳۵ + ۱۲۰ + ۱۲۰ + ۱۲۰ ثانیه پله‌های MASQUE و GOOL
    /// می‌سوخت.
    #[test]
    fn a_healthy_network_leads_with_wireguard_like_android() {
        let p = ConnectionProfile::default();
        let fp = NetFingerprint {
            filtered: false,
            udp_ok: true,
            quic_ok: true,
        };
        assert_eq!(fp.dpi_class(), DpiClass::Open);
        let plan = build_plan(&p, fp);
        assert_eq!(
            plan[0].profile.protocol,
            Protocol::Wireguard,
            "first rung is `{}`",
            plan[0].label
        );
    }

    /// و روی شبکه‌ای که UDP در آن مرده، همان ترتیبِ قبلی درست است: MASQUE اول،
    /// WireGuard آخر — چون WireGuard بی UDP هیچ شانسی ندارد.
    #[test]
    fn without_udp_wireguard_stays_last() {
        let p = ConnectionProfile::default();
        let fp = NetFingerprint {
            filtered: false,
            udp_ok: false,
            quic_ok: false,
        };
        assert_eq!(fp.dpi_class(), DpiClass::UdpThrottled);
        let plan = build_plan(&p, fp);
        assert_eq!(plan[0].profile.protocol, Protocol::Masque);
        assert_eq!(plan.last().unwrap().profile.protocol, Protocol::Wireguard);
    }

    /// فیلترینگِ TCP/SNI با UDPِ سالم: WireGuard اول می‌ماند (UDP دست‌نخورده
    /// است) و MASQUE که به TLS/SNI حساس‌تر است به آخر می‌رود — نردبانِ
    /// `SmartAuto.kt` برای `SNI_FILTERING`.
    #[test]
    fn sni_filtering_keeps_wireguard_first_and_pushes_masque_last() {
        let p = ConnectionProfile::default();
        let fp = NetFingerprint {
            filtered: true,
            udp_ok: true,
            quic_ok: true,
        };
        assert_eq!(fp.dpi_class(), DpiClass::SniFiltering);
        let plan = build_plan(&p, fp);
        assert_eq!(plan[0].profile.protocol, Protocol::Wireguard);
        assert_eq!(plan.last().unwrap().profile.protocol, Protocol::Masque);
    }

    /// هر چهار کلاس باید هر سه پروتکل را در نردبان داشته باشند: ترتیب عوض
    /// می‌شود، ولی هیچ پروتکلی حذف نمی‌شود — وگرنه یک شبکه‌ای پیدا می‌شود که
    /// تنها راهِ کارآمدش هرگز امتحان نمی‌شود.
    #[test]
    fn no_dpi_class_ever_drops_a_protocol() {
        for (filtered, udp_ok) in [(false, true), (true, true), (false, false), (true, false)] {
            let fp = NetFingerprint {
                filtered,
                udp_ok,
                quic_ok: udp_ok,
            };
            let plan = build_plan(&ConnectionProfile::default(), fp);
            for proto in [Protocol::Wireguard, Protocol::Masque, Protocol::Gool] {
                assert!(
                    plan.iter().any(|c| c.profile.protocol == proto),
                    "{:?}: {proto:?} missing from the ladder",
                    fp.dpi_class()
                );
            }
        }
    }

    /// و آن‌چه بی این سنجه بی‌صدا برمی‌گردد: کاربر با شبکهٔ سالم باید در
    /// **پاسِ اول** به WireGuard برسد، نه پس از سوختنِ بودجهٔ چند پله.
    #[test]
    fn on_a_healthy_network_wireguard_gets_the_first_pass_window() {
        let plan = build_plan(&ConnectionProfile::default(), NetFingerprint::default());
        assert_eq!(plan[0].profile.protocol, Protocol::Wireguard);
        assert!(
            plan[0].timeout_ms <= FIRST_PASS_MAX_MS,
            "first rung waits {}ms",
            plan[0].timeout_ms
        );
    }

    /// A user who chose HTTP/2 in the panel keeps it on a healthy network too:
    /// `carrier_for` only ever turns the carrier on.
    #[test]
    fn an_explicit_http2_choice_is_never_undone() {
        let p = ConnectionProfile {
            masque_http2: true,
            ..Default::default()
        };
        for c in build_plan(&p, NetFingerprint::default()) {
            if c.profile.protocol == Protocol::Masque {
                assert!(c.profile.masque_http2);
            }
        }
    }

    #[test]
    fn direct_plan_has_a_hardened_second_pass() {
        let p = ConnectionProfile {
            protocol: Protocol::Gool,
            gool_mode: GoolMode::Modern,
            ..Default::default()
        };
        let plan = build_plan(&p, NetFingerprint::default());
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].profile.noize, Noize::Off);
        assert_eq!(plan[1].profile.noize, Noize::Firewall);
        assert_eq!(plan[0].profile.protocol, plan[1].profile.protocol);
    }

    // >>> AETHER-APP-FIX ech-only-where-sni-is-filtered
    /// OPEN network: no rung asks for ECH (mobile `SmartAuto.kt` OPEN ladder);
    /// a filtered one still gets it on its hardened MASQUE rungs.
    #[test]
    fn ech_is_only_used_where_the_network_filters_sni() {
        let p = ConnectionProfile::default();
        let open = NetFingerprint {
            filtered: false,
            udp_ok: true,
            quic_ok: true,
        };
        assert!(build_plan(&p, open).iter().all(|c| !c.profile.ech));
        let filtered = NetFingerprint {
            filtered: true,
            udp_ok: true,
            quic_ok: true,
        };
        assert!(build_plan(&p, filtered)
            .iter()
            .any(|c| c.profile.ech && c.profile.protocol == Protocol::Masque));
    }
    // <<< AETHER-APP-FIX ech-only-where-sni-is-filtered

    // >>> AETHER-APP-FIX plain-pass-goes-last-on-a-filtered-network
    /// این تست همان ۷۵ ثانیه‌ای را می‌بندد که کاربر پای دکمهٔ Connect نشست.
    ///
    /// نکتهٔ درسی: نسخهٔ اول این تست ادعا می‌کرد پلهٔ ساده باید **آخرین** پله
    /// باشد. آن ادعا غلط بود و قراردادِ ترتیبِ پروتکل‌ها را می‌شکست — CI با
    /// `sni_filtering_keeps_wireguard_first_and_pushes_masque_last` گرفتش
    /// (left: Wireguard, right: Masque). چیزی که باید کوتاه شود بودجه است، نه
    /// جایگاه.
    #[test]
    fn on_a_filtered_network_the_plain_pass_only_gets_a_short_budget() {
        let p = ConnectionProfile::default();
        let fp = NetFingerprint {
            filtered: true,
            udp_ok: true,
            quic_ok: true,
        };
        let plan = build_plan(&p, fp);

        let plain: Vec<usize> = plan
            .iter()
            .enumerate()
            .filter(|(_, c)| c.profile.noize == Noize::Off)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            plain.len(),
            1,
            "exactly one plain rung: {:?}",
            plan.iter().map(|c| c.label.clone()).collect::<Vec<_>>()
        );

        let at = plain[0];
        assert_ne!(at, 0, "on a filtered network the hardened pass leads");
        assert!(
            plan[at].timeout_ms <= LAST_RESORT_MS,
            "the plain rung got {}ms; it used to get FIRST_PASS_MAX_MS ({}ms), which is the stall in the log",
            plan[at].timeout_ms,
            FIRST_PASS_MAX_MS
        );

        // و قراردادی که این اصلاح یک بار شکستش و دیگر نباید بشکند: ترتیبِ
        // پروتکل‌ها دستِ این اصلاح نیست.
        assert_eq!(plan[0].profile.protocol, Protocol::Wireguard);
        assert_eq!(plan.last().unwrap().profile.protocol, Protocol::Masque);
    }

    /// روی شبکهٔ سالم هیچ‌چیز عوض نشده: پاسِ ساده اول است و بودجهٔ کاملِ
    /// پاسِ اول را دارد.
    #[test]
    fn on_a_healthy_network_the_plain_pass_still_leads() {
        let p = ConnectionProfile::default();
        let fp = NetFingerprint {
            filtered: false,
            udp_ok: true,
            quic_ok: true,
        };
        let plan = build_plan(&p, fp);
        assert_eq!(plan[0].profile.noize, Noize::Off);
        assert!(plan[0].timeout_ms > LAST_RESORT_MS);
    }
    // <<< AETHER-APP-FIX plain-pass-goes-last-on-a-filtered-network
}
