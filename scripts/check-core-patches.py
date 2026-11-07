#!/usr/bin/env python3
"""هر پچی که در هسته گذاشته‌ایم باید از یک ارتقای هسته جان سالم ببرد.

# چرا این گارد وجود دارد

`scripts/sync-core.sh` هستهٔ Aether را ارتقا می‌دهد و در پایان درختِ قدیم را
حذف و درختِ آپ‌استریم را جای آن می‌گذارد. پچ‌های ما فقط برای فایل‌هایی که در
`PATCHED_FILES` هستند و در `.upstream-baseline` مبنا دارند سه‌طرفه merge
می‌شوند. هر فایلِ دیگری بی‌صدا با نسخهٔ آپ‌استریم عوض می‌شود — بی خطا، بی
هشدار، بی اینکه بیلد بشکند.

در ۱۷ سپتامبر ۲۰۲۶ همین اتفاق نزدیک بود بیفتد: کارِ توری که دو روز طول کشید
در `tor.rs` (۲۲۹ سطر)، `bridges.rs` (۲۴۲ سطر) و `lastconn.rs` (۱۹ سطر)
نشسته بود و هیچ‌کدام در فهرست نبودند؛ `build.rs` هم که مهرِ
`AETHER-BUILD-STAMP:` را می‌زند اصلاً در آپ‌استریم وجود ندارد و با آن
`cp -a` پاک می‌شد. اولین ارتقای موفقِ هسته، هر سه را برمی‌گرداند به حالتِ
پیش از رفعِ اشکال و لاگِ بعدی دقیقاً همان خطاهای قبلی را نشان می‌داد.

از این پس فهرست دستی نگه داشته نمی‌شود؛ منبعِ حقیقت خودِ نشانهٔ
`AETHER-APP-PATCH` در فایل‌های هسته است و این گارد اختلافِ آن با فهرست را
می‌شکند.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORE = ROOT / "native" / "aether"
BASELINE = CORE / ".upstream-baseline"
SYNC = ROOT / "scripts" / "sync-core.sh"
MARKER = "AETHER-APP-PATCH"

problems: list[str] = []


def note(text: str) -> None:
    print(f"    {text}")


def bash_array(body: str, name: str) -> list[str]:
    """محتوای یک آرایهٔ bash را می‌خواند، بی‌آنکه کامنت‌ها را به‌حساب بیاورد."""
    match = re.search(rf"^{name}=\((.*?)^\)", body, re.S | re.M)
    if not match:
        return []
    items: list[str] = []
    for line in match.group(1).splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            items.append(line)
    return items


# -----------------------------------------------------------------------------
# ۱) فایل‌های هسته که نشانِ پچِ ما را دارند
# -----------------------------------------------------------------------------
def patched_in_tree() -> list[str]:
    found: list[str] = []
    for path in sorted(CORE.rglob("*")):
        if not path.is_file() or path.suffix not in {".rs", ".toml"}:
            continue
        relative = path.relative_to(CORE)
        parts = relative.parts
        if ".upstream-baseline" in parts or "target" in parts:
            continue
        try:
            body = path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        if MARKER in body:
            found.append(relative.as_posix())
    return found


if not SYNC.is_file():
    print("✗ scripts/sync-core.sh پیدا نشد")
    sys.exit(1)

sync_body = SYNC.read_text(encoding="utf-8")
protected = bash_array(sync_body, "PATCHED_FILES")
app_owned = bash_array(sync_body, "APP_OWNED_FILES")
tracked = set(protected) | set(app_owned)

print("── ۱) هر فایلِ پچ‌خوردهٔ هسته در sync-core.sh شناخته شده است")
if not protected:
    problems.append("PATCHED_FILES در sync-core.sh خوانده نشد")
in_tree = patched_in_tree()
if not in_tree:
    problems.append(f"هیچ فایلی با نشانِ {MARKER} در هسته نیست — گارد بی‌معنی می‌شود")
for relative in in_tree:
    if relative in tracked:
        note(f"✓ {relative}")
    else:
        problems.append(
            f"{relative} پچِ ما را دارد ولی نه در PATCHED_FILES است و نه در "
            f"APP_OWNED_FILES — اولین ارتقای هسته آن را بی‌صدا پاک می‌کند"
        )

# -----------------------------------------------------------------------------
# ۲) هر فایلِ محافظت‌شده مبنایی برای merge سه‌طرفه دارد
# -----------------------------------------------------------------------------
print("── ۲) هر فایلِ PATCHED_FILES مبنای آپ‌استریم دارد")
for relative in protected:
    ours = CORE / relative
    base = BASELINE / relative
    if not ours.is_file():
        problems.append(f"{relative} در فهرست است ولی در هسته وجود ندارد")
        continue
    if not base.is_file():
        problems.append(
            f"{relative} مبنایی در .upstream-baseline ندارد — merge سه‌طرفه "
            f"ممکن نیست و sync ارتقا را رد می‌کند"
        )
        continue
    note(f"✓ {relative}")

# -----------------------------------------------------------------------------
# ۳) فایل‌های مالِ اپ: بی‌مبنا، ولی باید پس از جابه‌جاییِ درخت برگردند
# -----------------------------------------------------------------------------
print("── ۳) فایل‌های مالِ اپ پس از تعویضِ درخت بازگردانده می‌شوند")
if not app_owned:
    problems.append("APP_OWNED_FILES در sync-core.sh نیست")
restores = re.search(
    r'for rel in "\$\{APP_OWNED_FILES\[@\]\}".*?\bcp "\$keep" "\$CORE_DIR/\$rel"',
    sync_body,
    re.S,
)
if restores is None:
    problems.append(
        "حلقهٔ بازگرداندنِ APP_OWNED_FILES از snapshot در sync-core.sh نیست — "
        "فهرست بی‌اثر است"
    )
else:
    note("✓ حلقهٔ بازگردانی از $PREV_DIR سرِ جایش است")
for relative in app_owned:
    if not (CORE / relative).is_file():
        problems.append(f"{relative} مالِ اپ اعلام شده ولی در درخت نیست")
        continue
    if relative in protected:
        problems.append(f"{relative} هم در PATCHED_FILES است و هم APP_OWNED_FILES")
        continue
    note(f"✓ {relative}")

# -----------------------------------------------------------------------------
# ۴) نبودِ مبنا دیگر بی‌صدا نیست
# -----------------------------------------------------------------------------
print("── ۴) sync-core.sh نبودِ مبنا/فایل را بی‌صدا رد نمی‌کند")
if re.search(r'\[\[ -f "\$ours" && -f "\$base" && -f "\$theirs" \]\] \|\| continue', sync_body):
    problems.append(
        "همان `|| continue` قدیمی برگشته است: فایلِ بی‌مبنا بی هیچ هشداری با "
        "نسخهٔ آپ‌استریم عوض می‌شود"
    )
else:
    for needed in ('No upstream baseline for', 'no longer ships'):
        if needed not in sync_body:
            problems.append(f"هشدارِ «{needed}» در sync-core.sh نیست")
    note("✓ هر سه حالتِ نبودن، ارتقا را متوقف می‌کند")

# -----------------------------------------------------------------------------
if problems:
    print()
    for problem in problems:
        print(f"✗ {problem}")
    sys.exit(1)

print()
print(f"CORE PATCHES OK — {len(in_tree)} فایلِ پچ‌خورده، همه در امان")
