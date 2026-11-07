#!/usr/bin/env bash
# کنترل‌های منفیِ نگهبانِ نحو — و یک کنترلِ مثبت.
#
# یک سنجهٔ تعادلِ آکولاد دو گونه می‌تواند بی‌فایده باشد: چیزی را نگیرد، یا سرِ
# کدِ سالم بیخود فریاد بزند. هر دو اینجا آزموده می‌شود؛ بندِ آخر کدِ درستی است
# که آکولاد را داخلِ رشتهٔ خام و کامنت پنهان کرده و **نباید** قرمز کند.
set -u
cd "$(dirname "$0")/.."

GUARD=scripts/check-rust-parses.py
T=src-tauri/src/tor_native.rs
P=src-tauri/src/probe.rs
cp "$T" /tmp/p.tor; cp "$P" /tmp/p.probe
restore() { cp /tmp/p.tor "$T"; cp /tmp/p.probe "$P"; }
trap restore EXIT

fails=0
expect_red() {
  local name="$1"
  if python3 "$GUARD" >/tmp/p.out 2>&1; then
    echo "  ✗ $name — نگهبان سبز ماند (باید می‌گرفت)"
    fails=$((fails + 1))
  else
    echo "  ✓ $name → $(grep -m1 '^   - ' /tmp/p.out | sed 's/.*: //' | cut -c1-80)"
  fi
  restore
  if ! python3 "$GUARD" >/dev/null 2>&1; then
    echo "  ✗ بازگردانی ناقص ماند"
    fails=$((fails + 1))
  fi
}

echo "کنترل‌های نگهبانِ نحو"

# ۱) همان آسیبی که در ۱۷ سپتامبر هر دو جابِ بیلد را انداخت
python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/tor_native.rs"); t = p.read_text(encoding="utf-8")
t = t.replace("""    /// پورتی که تور واقعاً گرفته، پرسیده‌شده نه فرض‌شده.
    fn socks_port""",
              """    /// پورتی که تور واقعاً گرفته، پرسیده‌شده نه فرض‌شده.    fn socks_port""", 1)
p.write_text(t, encoding="utf-8")
PY
expect_red "کامنت و امضای تابع روی یک سطر (خطای واقعیِ CI)"

# ۲) یک آکولادِ بسته که جفت ندارد
printf '\n}\n' >> "$T"
expect_red "آکولادِ بستهٔ اضافی در انتهای فایل"

# ۳) یک آکولادِ باز که بسته نمی‌شود
printf '\nfn nie_geschlossen() {\n' >> "$P"
expect_red "تابعِ بی‌آکولادِ پایانی"

# ۴) پرانتزی که با آکولاد بسته می‌شود
python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/probe.rs"); t = p.read_text(encoding="utf-8")
t += "\nfn falsches_paar( } \n"
p.write_text(t, encoding="utf-8")
PY
expect_red "بستنِ پرانتز با آکولاد"

# ۴ب) یک سطرِ شِل/env که به انتهای فایل چسبیده (خطای واقعیِ CI، ۲۰۲۶-۱۰-۰۵:
#     engine.rs:918 «expected one of `!` or `::`, found `=`»). تعادلِ آکولاد
#     سالم می‌ماند، پس فقط بررسیِ سطحِ بالا آن را می‌گیرد.
printf '\nAETHER_QUICK_RECONNECT_MAX_HANDSHAKE_MS=0 # Disable handshake timeout for Tor\n' >> "$T"
expect_red "سطرِ env/شِل روی سطحِ بالای فایلِ Rust (خطای واقعیِ CI ۵ اکتبر)"

# ۵) کنترلِ مثبت: آکولاد داخلِ رشتهٔ خام و کامنت — نباید قرمز شود
cat >> "$P" <<'RUST'

/// یک کامنت با آکولادِ تنها: {
/// و یکی دیگر: }
fn kommentar_und_rohstring_sind_kein_code() -> &'static str {
    let _a = r#"{{{ نه کد است و نه آکولادِ واقعی "# ;
    let _b = "} } }";
    let _c = '}';
    // }
    "{"
}
RUST
if python3 "$GUARD" >/dev/null 2>&1; then
  echo "  ✓ آکولادِ پنهان در رشتهٔ خام/کامنت/کاراکتر → هشدارِ بی‌جا نداد"
else
  echo "  ✗ هشدارِ بی‌جا روی کدِ سالم:"
  python3 "$GUARD" | head -3
  fails=$((fails + 1))
fi
restore

echo
if [ "$fails" -eq 0 ]; then
  echo "✓ پنج آسیب گرفته شد، و کدِ سالم قرمز نشد"
  exit 0
fi
echo "✗ $fails بند نگرفت"
exit 1
