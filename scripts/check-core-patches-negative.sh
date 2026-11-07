#!/usr/bin/env bash
# کنترل‌های منفیِ نگهبانِ پچ‌های هسته.
#
# این نگهبان از جنسِ خطرناک‌ترها است: چیزی را می‌پاید که شکستنش هیچ خطایی
# تولید نمی‌کند — پچ‌ها بی‌صدا ناپدید می‌شوند و بیلد سبز می‌ماند. پس تا ثابت
# نشود که قرمز می‌شود، هیچ نگفته است.
#
# هر جهش خودش را تأیید می‌کند: جهشی که نگیرد، شکستِ کنترل است، نه موفقیتش.
set -u
cd "$(dirname "$0")/.."

GUARD=scripts/check-core-patches.py
SY=scripts/sync-core.sh
BASE=native/aether/.upstream-baseline/aether/src/tor.rs
OWNED=native/aether/aether/build.rs

cp "$SY" /tmp/cp.sync
cp "$BASE" /tmp/cp.base
cp "$OWNED" /tmp/cp.owned
restore() {
  cp /tmp/cp.sync "$SY"
  cp /tmp/cp.base "$BASE"
  cp /tmp/cp.owned "$OWNED"
}
trap restore EXIT

fails=0

mutate() { # جهش را اعمال می‌کند و مطمئن می‌شود که واقعاً اعمال شد
  if ! python3 -; then
    echo "  ✗ جهش اعمال نشد — متنِ هدف عوض شده، این کنترل چیزی نمی‌سنجد"
    fails=$((fails + 1))
    return 1
  fi
}

expect_red() {
  local name="$1"
  if python3 "$GUARD" >/tmp/cp.out 2>&1; then
    echo "  ✗ $name — نگهبان سبز ماند (باید می‌گرفت)"
    fails=$((fails + 1))
  else
    echo "  ✓ $name → $(grep -m1 '^✗ ' /tmp/cp.out | head -c 400)"
  fi
  restore
}

echo "کنترل‌های منفی:"

# ۱) همان اتفاقی که ۱۷ سپتامبر رخ داده بود: کارِ توری که در فهرست نیست.
mutate <<'EOF'
from pathlib import Path
p = Path("scripts/sync-core.sh"); s = b = p.read_text(encoding="utf-8")
s = s.replace("  aether/src/tor.rs\n", "", 1)
assert s != b, "سطرِ tor.rs در PATCHED_FILES پیدا نشد"
p.write_text(s, encoding="utf-8")
EOF
[ $? -eq 0 ] && expect_red "فایلِ پچ‌خورده از PATCHED_FILES افتاده"

# ۲) در فهرست هست، ولی مبنا نیست: merge سه‌طرفه ممکن نیست.
if rm -f "$BASE"; then
  expect_red "مبنای آپ‌استریم برای یک فایلِ محافظت‌شده نیست"
else
  echo "  ✗ جهش اعمال نشد — مبنا حذف نشد"
  fails=$((fails + 1))
fi

# ۳) بازگشتِ همان `|| continue` که نبودِ مبنا را بی‌صدا رد می‌کرد.
mutate <<'EOF'
import re
from pathlib import Path
p = Path("scripts/sync-core.sh"); s = b = p.read_text(encoding="utf-8")
s = re.sub(
    r'  # >>> AETHER-APP-PATCH core-sync-never-drops-a-patch-silently.*?'
    r'  # <<< AETHER-APP-PATCH core-sync-never-drops-a-patch-silently\n',
    '  [[ -f "$ours" && -f "$base" && -f "$theirs" ]] || continue\n',
    s,
    flags=re.S,
)
assert s != b, "بلوکِ سه‌گانهٔ بررسی پیدا نشد"
p.write_text(s, encoding="utf-8")
EOF
[ $? -eq 0 ] && expect_red "سکوتِ قدیمی به‌جای توقفِ ارتقا برگشته"

# ۴) فهرستِ فایل‌های مالِ اپ هست، ولی حلقهٔ بازگردانی حذف شده — فهرست بی‌اثر.
mutate <<'EOF'
import re
from pathlib import Path
p = Path("scripts/sync-core.sh"); s = b = p.read_text(encoding="utf-8")
s = re.sub(
    r'# >>> AETHER-APP-PATCH core-sync-keeps-app-owned-files.*?'
    r'# <<< AETHER-APP-PATCH core-sync-keeps-app-owned-files\n',
    '',
    s,
    flags=re.S,
)
assert s != b, "حلقهٔ بازگردانیِ فایل‌های مالِ اپ پیدا نشد"
p.write_text(s, encoding="utf-8")
EOF
[ $? -eq 0 ] && expect_red "حلقهٔ بازگردانیِ فایل‌های مالِ اپ برداشته شده"

# ۵) فایلِ مالِ اپ اعلام شده ولی در درخت نیست (همان چیزی که cp -a می‌کرد).
if rm -f "$OWNED"; then
  expect_red "فایلِ مالِ اپ از درخت غایب است"
else
  echo "  ✗ جهش اعمال نشد — build.rs حذف نشد"
  fails=$((fails + 1))
fi

# ۶) و یک بندِ سلامتِ خودِ کنترل: بی هیچ جهشی نگهبان باید سبز باشد.
if python3 "$GUARD" >/tmp/cp.out 2>&1; then
  echo "  ✓ بی‌جهش، نگهبان سبز است"
else
  echo "  ✗ بی‌جهش، نگهبان قرمز است — بازگردانی درست کار نکرده"
  fails=$((fails + 1))
fi

echo
if [ "$fails" -ne 0 ]; then
  echo "✗ $fails کنترلِ منفی شکست"
  exit 1
fi
echo "CORE PATCHES NEGATIVE OK"
