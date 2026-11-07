#!/usr/bin/env bash
# =============================================================================
#  انتشارِ ریلیز از کامپیوترِ خودت — بدونِ یک دقیقه از سهمیهٔ GitHub Actions
# -----------------------------------------------------------------------------
#  چرا این فایل وجود دارد:
#  در اجرای ۹۵۱۲۸۶۰۶۸۲۰ همهٔ بیلدها سبز شدند و فقط جابِ آخر (انتشار) استارت
#  نخورد، چون سهمیهٔ دقیقهٔ حساب در میانهٔ همان اجرا تمام شد (۲۰۰۰/۲۰۰۰).
#  فایل‌های نصب‌کننده اما ساخته و آپلود شده‌اند. این اسکریپت همان کاری را
#  می‌کند که جابِ `release` می‌کرد — چک‌سام + انتشارِ idempotent با gh —
#  ولی روی ماشینِ خودت، پس هیچ دقیقه‌ای مصرف نمی‌شود.
#
#  استفاده:
#    ۱) در صفحهٔ اجرا، بخش Artifacts، هر دو `aether-x64` و `aether-x86` را
#       بگیر و کنارِ هم در یک پوشه باز کن (مثلاً ~/Downloads/aether-dist).
#    ۲) gh auth login   (یک‌بار برای همیشه)
#    ۳) bash scripts/publish-release-local.sh ~/Downloads/aether-dist
#
#  گزینه‌ها:
#    --tag vX.Y.Z   تگِ ریلیز (پیش‌فرض: از src-tauri/tauri.conf.json)
#    --repo O/R     مخزنِ مقصد (پیش‌فرض: از remote origin)
#    --dry-run      فقط نشان بده چه فرمانی اجرا می‌شد
# =============================================================================
set -euo pipefail

cd "$(dirname "$0")/.."

DIST=""; TAG=""; REPO=""; DRY=0
while [ $# -gt 0 ]; do
  case "$1" in
    --tag)     TAG="$2"; shift 2 ;;
    --repo)    REPO="$2"; shift 2 ;;
    --dry-run) DRY=1; shift ;;
    -h|--help) sed -n '2,28p' "$0"; exit 0 ;;
    *)         DIST="$1"; shift ;;
  esac
done

[ -n "$DIST" ] || { echo "✗ مسیرِ پوشهٔ فایل‌های بیلد را بده. --help را ببین."; exit 2; }
[ -d "$DIST" ] || { echo "✗ پوشه پیدا نشد: $DIST"; exit 2; }

command -v gh >/dev/null || { echo "✗ gh نصب نیست: https://cli.github.com"; exit 2; }
[ "$DRY" -eq 1 ] || gh auth status >/dev/null 2>&1 || { echo "✗ اول: gh auth login"; exit 2; }

if [ -z "$TAG" ]; then
  V=$(python3 -c "import json;print(json.load(open('src-tauri/tauri.conf.json'))['version'])")
  TAG="v$V"
fi
if [ -z "$REPO" ]; then
  url=$(git config --get remote.origin.url 2>/dev/null || true)
  [ -n "$url" ] || { echo "✗ مخزن را پیدا نکردم (remote origin نیست). با --repo مالک/نام بده."; exit 2; }
  REPO=$(printf '%s' "$url" | sed -E 's#^.*github\.com[:/]##; s#\.git$##')
fi
# اگر بیرونِ کلونِ گیت اجرا شود، ریلیز به شاخهٔ پیش‌فرضِ مخزن بسته می‌شود.
SHA=$(git rev-parse HEAD 2>/dev/null || true)

# همان دروازهٔ جابِ CI، کلمه‌به‌کلمه: یک ریلیزِ نیم‌بند بدتر از نبودنِ آن است.
missing=0
for pat in "Dopamine-Setup-*-x64.exe" "Dopamine-Setup-*-x86.exe" \
           "Dopamine-Portable-*-x64.zip" "Dopamine-Portable-*-x86.zip"; do
  # shellcheck disable=SC2086
  if ! compgen -G "$DIST/$pat" > /dev/null; then
    echo "✗ خروجی بیلد پیدا نشد: $pat"
    missing=1
  fi
done
[ "$missing" -eq 0 ] || { echo "  (هر دو آرتیفکتِ aether-x64 و aether-x86 را در همان پوشه باز کن)"; exit 1; }

echo "── چک‌سام‌ها"
( cd "$DIST" && sha256sum Dopamine-* | tee SHA256SUMS.txt )

mapfile -t assets < <(ls "$DIST"/Dopamine-Setup-*.exe "$DIST"/Dopamine-Portable-*.zip "$DIST/SHA256SUMS.txt")
TITLE="Dopamine ${TAG#v}"
NOTES=".github/release-notes.md"
[ -f "$NOTES" ] || { echo "✗ یادداشتِ ریلیز نیست: $NOTES"; exit 1; }

run() {
  if [ "$DRY" -eq 1 ]; then printf '  [dry-run] %s\n' "$*"; else "$@"; fi
}

target=()
if [ -n "$SHA" ]; then
  target=(--target "$SHA")
  echo "── انتشار در $REPO با تگ $TAG (commit ${SHA:0:8})"
else
  echo "── انتشار در $REPO با تگ $TAG (بیرونِ کلونِ گیت: بدونِ --target)"
fi

if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
  run gh release upload "$TAG" "${assets[@]}" --repo "$REPO" --clobber
  run gh release edit "$TAG" --repo "$REPO" --title "$TITLE" \
      --notes-file "$NOTES" "${target[@]}" --latest --draft=false --prerelease=false
  echo "✓ ریلیزِ موجود به‌روز شد ($TAG)"
else
  run gh release create "$TAG" "${assets[@]}" --repo "$REPO" --title "$TITLE" \
      --notes-file "$NOTES" "${target[@]}" --latest
  echo "✓ ریلیز ساخته شد ($TAG)"
fi
