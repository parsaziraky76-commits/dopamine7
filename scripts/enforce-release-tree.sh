#!/usr/bin/env bash
# =============================================================================
#  enforce-release-tree.sh — پاک‌سازیِ هوشمندِ درختِ بیلد پیش از کامپایل
# -----------------------------------------------------------------------------
#  اجرا: داخلِ خودِ Job «build» (همان رانری که cargo و tauri را صدا می‌زند)،
#  بلافاصله بعد از بازگردانیِ کشِ Rust و پیش از هر دستورِ cargo/npm.
#
#  چرا اینجا و نه در یک Job جدا (ریشهٔ خطای اجرای 1.3.0):
#    * Jobهای GitHub Actions هر کدام روی یک ماشینِ تازه اجرا می‌شوند و دیسکِ
#      مشترک ندارند؛ پاک‌سازی در Job جدا هیچ اثری روی Job بیلد نداشت.
#    * آن Job جدا `timeout-minutes` نداشت و scripts/check-workflow-order.py
#      (نگهبانِ سهمیهٔ دقیقه‌های ویندوز) به‌درستی کلِ Preflight را شکست.
#
#  چه چیزی پاک می‌شود (فقط خروجی‌های بیلد که در git نیستند — .gitignore):
#    1) dist/            نصب‌کننده‌ها، ZIPهای پرتابل و payloadِ نسخه‌های قبلی
#    2) dist-engine/     موتور/Wintun/Psiphon/Tor ساخته‌شده با هستهٔ قبلی
#    3) src-tauri/target/**/release/{bundle,aether-desktop.exe}
#                        باینری و باندلِ اپ از نسخهٔ قبلی (کشِ وابستگی‌ها می‌ماند)
#    4) native/aether/**/target/**/release/aether.exe
#                        موتورِ قدیمی، تا گیتِ «مهرِ پچ» هرگز یک باینریِ کهنه را
#                        به‌جای باینریِ تازه پیدا نکند
#
#  چه چیزی عمداً دست نمی‌خورد:
#    * Cargo.lock — نسخهٔ یک وابستگی (مثلاً 1.2.5) ربطی به نسخهٔ اپ ندارد؛ حذفِ
#      قفل، `--locked` را بی‌اثر و بیلد را غیرقابل‌تکرار می‌کرد.
#    * کشِ deps/ و .fingerprint/ — cargo خودش ناسازگاری را تشخیص می‌دهد.
#    * native/.core-prev — اسنپ‌شاتِ بازگشتِ هسته (core-rollback.ps1).
#    * هر فایلی که در git است.
#
#  این اسکریپت هرگز بیلد را نمی‌شکند، مگر پوشهٔ ریشهٔ مخزن پیدا نشود.
# =============================================================================
set -uo pipefail

TAG="enforce-release-tree"
cd "$(dirname "$0")/.." || { echo "::error::[$TAG] repository root not found"; exit 1; }

# نسخهٔ جاری از tauri.conf.json — بدون python (روی Git-Bash ویندوز ممکن است
# python3 نباشد).
VERSION=$(grep -m1 -oE '"version"[[:space:]]*:[[:space:]]*"[^"]+"' src-tauri/tauri.conf.json \
          | sed -E 's/.*"([^"]+)"$/\1/')
VERSION=${VERSION:-unknown}
echo "[$TAG] release tree for Aether ${VERSION}: removing build outputs of older releases"

removed=0
drop() {
  # $1 = مسیر؛ فقط اگر وجود داشته باشد و در git ردیابی نشود.
  local p="$1"
  [ -e "$p" ] || return 0
  if command -v git >/dev/null 2>&1 && git ls-files --error-unmatch "$p" >/dev/null 2>&1; then
    echo "[$TAG]   keep (tracked by git): $p"
    return 0
  fi
  rm -rf -- "$p" && { echo "[$TAG]   removed: $p"; removed=$((removed + 1)); }
  return 0
}

# 1) و 2) خروجی‌های کاملِ بسته‌بندی و موتور
drop dist
drop dist-engine

# 3) باینری و باندلِ اپ (هر target triple)
shopt -s nullglob
for rel in src-tauri/target/release src-tauri/target/*/release; do
  drop "$rel/bundle"
  drop "$rel/aether-desktop.exe"
  drop "$rel/aether-desktop.pdb"
done

# 4) موتورِ ساخته‌شدهٔ قبلی
for rel in native/aether/target/release native/aether/target/*/release \
           native/aether/aether/target/release native/aether/aether/target/*/release; do
  drop "$rel/aether.exe"
  drop "$rel/aether.pdb"
done
shopt -u nullglob

echo "[$TAG] done: ${removed} stale path(s) removed, Cargo.lock and dependency caches kept"
exit 0
