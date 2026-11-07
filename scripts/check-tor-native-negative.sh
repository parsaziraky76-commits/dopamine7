#!/usr/bin/env bash
# کنترل‌های منفیِ نگهبانِ تورِ بومی.
#
# یک نگهبانِ سبز تا وقتی ثابت نکند که قرمز هم می‌شود، هیچ چیزی را ثابت نکرده
# است. هر بند یکی از راه‌هایی است که این سیم‌کشی می‌تواند در بازنویسیِ بعدی
# ساکت از بین برود؛ نگهبان باید همه را بگیرد.
#
# ۱.۲.۵ — و یک درسِ گران: جهشی که *نگیرد* هم کنترل را سبز نشان می‌داد. وقتی
# `state.rs` بازسازی شد، دو جهش به متنی می‌زدند که دیگر وجود نداشت؛ `str.replace`
# بی‌صدا هیچ نکرد و کنترل «✓» گفت، چون نگهبان به دلیلِ دیگری قرمز بود. حالا هر
# جهش خودش را تأیید می‌کند: نگرفتن، شکست است.
set -u
cd "$(dirname "$0")/.."

GUARD=scripts/check-tor-native.py
S=src-tauri/src/state.rs
P=src-tauri/src/pt.rs
E=src-tauri/src/engine.rs
N=src-tauri/src/tor_native.rs
D=src-tauri/src/diagnostics.rs
B=src-tauri/src/probe.rs
cp "$S" /tmp/g.state; cp "$P" /tmp/g.pt; cp "$E" /tmp/g.engine
cp "$N" /tmp/g.native; cp "$D" /tmp/g.diag; cp "$B" /tmp/g.probe
restore() {
  cp /tmp/g.state "$S"; cp /tmp/g.pt "$P"; cp /tmp/g.engine "$E"
  cp /tmp/g.native "$N"; cp /tmp/g.diag "$D"; cp /tmp/g.probe "$B"
}
trap restore EXIT

fails=0

# جهش را اعمال می‌کند و مطمئن می‌شود که *واقعاً* اعمال شد.
mutate() {
  if ! python3 -; then
    echo "  ✗ جهش اعمال نشد — متنِ هدف در کد عوض شده، این کنترل چیزی نمی‌سنجد"
    fails=$((fails + 1))
    return 1
  fi
}

expect_red() { # نام، انتظارِ قرمزی
  local name="$1"
  if python3 "$GUARD" >/tmp/g.out 2>&1; then
    echo "  ✗ $name — نگهبان سبز ماند (باید می‌گرفت)"
    fails=$((fails + 1))
  else
    echo "  ✓ $name → $(grep -m1 '^   - ' /tmp/g.out | sed 's/^   - //')"
  fi
  restore
}

echo "کنترل‌های منفی:"

# ۱) ترتیب: اگر بعد از engine.start پرسیده شود، arti و tor هر دو ۱۸۱۹ را می‌خواهند
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("        if self.start_native_tor(&cand)? {\n            return Ok(());\n        }", "", 1)
s = s.replace(
    "        self.engine.start(&engine_profile, Some(cand.timeout_ms))?;",
    "        self.engine.start(&engine_profile, Some(cand.timeout_ms))?;\n        if self.start_native_tor(&cand)? { return Ok(()); }",
    1,
)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "پرسشِ بومی بعد از اجرای موتور"

# ۲) یکی از چهار بررسیِ زنده‌بودن به موتور برگردد
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("                } else if !self.carrier_alive() {", "                } else if !self.engine.is_alive() {", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "بررسیِ زنده‌بودنِ خودآزما به موتور برگشت"

# ۳) پورتِ «تور تنها» عوض شود (چیزی که پراکسیِ سیستم و TUN نمی‌بینند)
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace(
    "            Some(TorMode::Only) => {",
    "            Some(TorMode::Only) => {\n                #[allow(unreachable_code)]\n                return Ok(false);",
    1,
)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "«تور تنها» دیگر پورتِ ۱۸۱۹ را نمی‌گیرد"

# ۴) توقفِ تور در جمع‌کردنِ نشست حذف شود
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("        if let Some(tor) = self.native_tor.take() {\n            tor.stop();\n        }", "", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "جمع‌کردنِ نشست تور را نمی‌بندد"

# ۵) مسیرِ ترابرِ کنارِ tor.exe از pt::dirs برود
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/pt.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace('        working_dir.join("engine").join("tor"),\n', "", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "engine/tor از مسیرهای ترابر افتاد"

# ۶) engine.rs مسیرها را ندهد
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/engine.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("pub fn native_tor(", "fn unused_native_tor(", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "engine.rs مسیرِ tor.exe را نمی‌دهد"

# ۷) مهلتِ خواندن باز هم کشنده شود — دقیقاً باگِ لاگِ ۱۷ سپتامبر
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/tor_native.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("fn is_timeout(", "fn unused_is_timeout(", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "مهلتِ خواندن باز معنای شکست گرفت"

# ۸) مسیرِ دوم (لاگِ خودِ تور) حذف شود
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/tor_native.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("following its own log", "giving up on this attempt")
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "از دست‌رفتنِ پورتِ کنترل باز کشنده شد"

# ۹) بودجه دوباره از ثابتِ ماجول خوانده شود
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/tor_native.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("launch.budget.unwrap_or(BOOTSTRAP_TIMEOUT)", "BOOTSTRAP_TIMEOUT", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "بودجهٔ برنامه نادیده گرفته شد"

# ۱۰) همان وعدهٔ دروغِ قبلی: lyrebird مدعیِ snowflake
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/tor_native.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace('(LYREBIRD_FILENAME, "meek_lite,obfs4,webtunnel")',
              '(LYREBIRD_FILENAME, "meek_lite,obfs4,webtunnel,snowflake")', 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "lyrebird دوباره مدعیِ snowflake شد"

# ۱۱) ترتیبِ موج‌ها برعکس شود: گران‌ترین اول
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/tor_native.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace('["obfs4", "snowflake", "meek", "webtunnel"]',
              '["snowflake", "obfs4", "meek", "webtunnel"]', 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "snowflake پیش از obfs4"

# ── از این‌جا: سیم‌کشیِ زنجیره‌ها و پرچم و مدار (لاگِ ۱۷ سپتامبر) ─────────────

# ۱۲) poll_chain دوباره موتور را دربارهٔ استیج ۱ بپرسد — همان تخریبِ نابه‌جا
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("            if !self.carrier_alive() {", "            if !self.engine.is_alive() {", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "poll_chain دوباره موتور را می‌پرسد"

# ۱۳) «هرگز اجرا نشده» باز با «مرده» یکی شود
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/engine.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("pub fn was_started(", "fn unused_was_started(", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "carrier «هرگز اجرا نشده» را از «مرده» جدا نمی‌کند"

# ۱۴) `Tor → Aether` دوباره به تورِ داخلیِ موتور (arti) برگردد
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("            Some(TorMode::Reverse) => self.tor_listener_port(),\n", "", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "Tor → Aether دوباره از arti می‌رود"

# ۱۵) موتورِ پشتِ تور بی‌upstream اجرا شود — یعنی مستقیم بیرون برود
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("        self.engine.set_upstream_socks(Some(tor_port));", "", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "موتورِ پشتِ تور از تور رد نمی‌شود"

# ۱۶) و اگر بک‌اندِ توری‌اش را نگه دارد، دو تور روی یک زنجیر بالا می‌آید
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
at = s.find("fn start_engine_behind_tor(")
assert at > 0, "تابع پیدا نشد"
head, tail = s[:at], s[at:]
tail = tail.replace("        stage.backend = crate::profile::TransportBackend::Aether;\n", "", 1)
s = head + tail
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "موتورِ پشتِ تور بک‌اندِ توری‌اش را نگه داشت"

# ۱۷) تورِ پشتِ تونل پل بگیرد — نشانیِ پل لختِ روی شبکهٔ سانسورشده
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
at = s.find("fn start_tor_behind_engine(")
assert at > 0, "تابع پیدا نشد"
head, tail = s[:at], s[at:]
tail = tail.replace("crate::tor_native::BridgeMode::None,", "crate::tor_native::BridgeMode::BuiltIn,", 1)
s = head + tail
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "تورِ پشتِ تونل باز پل می‌گیرد"

# ۱۸) سنجهٔ استیج ۱ دوباره آی‌پیِ خام بزند
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/diagnostics.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace(
    "            if probe::tcp_via_proxy_on(port, host, *dest) {",
    '            if probe::tcp_via_proxy_on(port, "1.1.1.1", 80) {',
    1,
)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "سنجهٔ استیج ۱ باز آی‌پیِ خام می‌زند"

# ۱۹) پرچم دوباره فقط از شبکه بیاید
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/probe.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("        if let Some(code) = crate::geoip::lookup(addr) {", "        if let Some(code) = None::<String> {", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "پرچم باز از شبکه پرسیده می‌شود"

# ۲۰) بعد از NEWNYM نشستِ گرمِ پینگ نگه داشته شود — همان مدارِ قدیم سنجیده می‌شود
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
at = s.find("fn consider_new_circuit(")
assert at > 0, "تابع پیدا نشد"
head, tail = s[:at], s[at:]
tail = tail.replace("                ping::reset();\n", "", 1)
s = head + tail
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "پینگ بعد از مدارِ تازه ریست نمی‌شود"

# ۲۱) چرخشِ مدار بی‌سقف شود
mutate <<'EOF'
from pathlib import Path
p = Path("src-tauri/src/state.rs"); s = b = p.read_text(encoding="utf-8")
s = s.replace("        if self.circuit_tries >= MAX_ROTATIONS {", "        if false {", 1)
s = s.replace("        const MAX_ROTATIONS: u8 = 3;\n", "", 1)
assert s != b, "جهش نگرفت"
p.write_text(s, encoding="utf-8")
EOF
expect_red "چرخشِ مدار سقف ندارد"

echo
if python3 "$GUARD" >/dev/null 2>&1; then
  echo "بازگردانی ✓ — نگهبان روی درختِ دست‌نخورده سبز است"
else
  echo "✗ بازگردانی ناقص ماند"
  fails=$((fails + 1))
fi

if [ "$fails" -ne 0 ]; then
  echo "✗ $fails کنترلِ منفی شکست خورد: نگهبان به‌اندازهٔ لازم سخت‌گیر نیست"
  exit 1
fi

echo "✓ هر ۲۱ کنترلِ منفی گرفته شد"
