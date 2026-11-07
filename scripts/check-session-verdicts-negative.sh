#!/usr/bin/env bash
# کنترل‌های منفیِ نگهبانِ منطقِ نشست.
set -u
cd "$(dirname "$0")/.."
G=scripts/check-session-verdicts.py
files=(src-tauri/src/tor_native.rs src-tauri/src/state.rs src-tauri/src/leakguard.rs
        src-tauri/src/diagnostics.rs src-tauri/src/tun.rs src-tauri/src/profile.rs src-tauri/src/store.rs)
for f in "${files[@]}"; do cp "$f" "/tmp/sv.$(basename "$f")"; done
restore() { for f in "${files[@]}"; do cp "/tmp/sv.$(basename "$f")" "$f"; done; }
trap restore EXIT

fails=0
expect_red() {
  if python3 "$G" >/tmp/sv.out 2>&1; then
    echo "  ✗ $1 — نگهبان سبز ماند"; fails=$((fails + 1))
  else
    echo "  ✓ $1 → $(grep -m1 '^   - ' /tmp/sv.out | sed 's/^   - //' | cut -c1-95)"
  fi
  restore
}

echo "کنترل‌های منفی:"

py() { python3 -c "
import sys, pathlib
p = pathlib.Path(sys.argv[1]); s = p.read_text(encoding='utf-8')
alt, neu = sys.argv[2], sys.argv[3]
assert s.count(alt) >= 1, ('یافت نشد', alt[:40])
p.write_text(s.replace(alt, neu, 1), encoding='utf-8')" "$@"; }

# ۱) فازِ انلاق دوباره «مرده» حساب شود
py src-tauri/src/tor_native.rs 'PHASE_STARTING => true,' 'PHASE_STARTING => false,'
expect_red "انلاق دوباره مرده حساب شد"

# ۲) mark_starting بعد از ساختنِ ترد صدا زده شود
py src-tauri/src/state.rs 'tor.mark_starting();' '/* moved */'
expect_red "علامتِ انلاق حذف شد"

# ۳) داوریِ نشتی دامنه را نخواند
py src-tauri/src/diagnostics.rs 'let leaking = !via_tunnel && !browser_policy_only && !udp_open_by_design;' 'let leaking = !via_tunnel && !browser_policy_only;'
expect_red "داوری دامنه را نادیده گرفت"

# ۴) تخفیفِ نشتی بی‌شرطِ سیاستِ مرورگر (نشتِ واقعی را می‌بخشد)
py src-tauri/src/diagnostics.rs 'guard.udp_browser_scoped && guard.browser_policies > 0' 'guard.udp_browser_scoped'
expect_red "تخفیف بی‌قید شد (fail-open)"

# ۵) در هر اجرا انتخابِ کاربر دوباره به Aether برگردد
py src-tauri/src/store.rs 'let changed = migrate(&mut profile);' 'let changed = migrate(&mut profile); profile.backend = TransportBackend::Aether;'
expect_red "انتخاب بک‌اند کاربر هنگام بارگذاری پاک شد"

# ۶) بازگردانیِ رجیستری دوباره سریالی شود
py src-tauri/src/leakguard.rs 'fn restore_policies(' 'fn unused_restore_policies('
expect_red "بازگردانی دوباره سریالی شد"

# ۷) لاگِ تکراریِ TUN برگردد
py src-tauri/src/tun.rs 'if had_session {' 'if true {'
expect_red "لاگِ ابزار شبکه دوباره بی‌قید شد"

# ۸) اثرانگشت دوباره در «تور تنها» گرفته شود
py src-tauri/src/state.rs 'let fingerprint = kind == Prep::Plan && !tor_only;' 'let fingerprint = kind == Prep::Plan;'
expect_red "اثرانگشت دوباره برای تور هم گرفته شد"

echo
if python3 "$G" >/dev/null 2>&1; then
  echo "بازگردانی ✓ — نگهبان روی درختِ دست‌نخورده سبز است"
else
  echo "✗ بازگردانی ناقص ماند"; fails=$((fails + 1))
fi
[ "$fails" -eq 0 ] || { echo "✗ $fails کنترل شکست خورد"; exit 1; }
echo "✓ هر ۸ کنترلِ منفی گرفته شد"
