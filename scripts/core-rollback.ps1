<#
.SYNOPSIS  بازگرداندن هسته به نسخهٔ قبلی.
.DESCRIPTION
  معادل scripts/core-rollback.sh مخزن اندروید.
  قاعده: ارتقای خودکار هسته هرگز نباید بتواند یک انتشار را بشکند.
#>
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
# sync-core.sh اسنپ‌شات را با `cp -a native/aether/. native/.core-prev/` می‌گیرد،
# یعنی خودِ native/.core-prev همان پوشهٔ کاملِ هسته است (CORE_VERSION، aether/،
# quiche/ ...). مسیرِ قبلی (native/.core-prev/aether) فقط کریت را برمی‌داشت و
# آن را جای کلِ native/aether می‌گذاشت — چیدمانی که build-engine.ps1 نمی‌شناسد.
# مسیرِ قدیمی فقط وقتی پذیرفته می‌شود که واقعاً یک هستهٔ کامل باشد.
$snap = Join-Path $root 'native/.core-prev'
if (-not (Test-Path (Join-Path $snap 'CORE_VERSION'))) {
  $legacy = Join-Path $root 'native/.core-prev/aether'
  if (Test-Path (Join-Path $legacy 'CORE_VERSION')) { $snap = $legacy }
}
$core = Join-Path $root 'native/aether'

if (-not (Test-Path (Join-Path $snap 'CORE_VERSION'))) {
  throw 'No core snapshot to roll back to - this is a real build error, not an upgrade failure.'
}

Write-Host '::warning::Rolling the engine back to the previously shipped core.'
Remove-Item $core -Recurse -Force
New-Item -ItemType Directory -Force -Path $core | Out-Null
Copy-Item (Join-Path $snap '*') $core -Recurse -Force

# CORE_VERSION هم باید برگردد — مخزن نباید ارتقایی را ثبت کند که کامپایل نمی‌شود.
$prevVersion = Join-Path $snap 'CORE_VERSION'
if (Test-Path $prevVersion) { Copy-Item $prevVersion (Join-Path $root 'CORE_VERSION') -Force }

Write-Host "==> Rolled back to core $(Get-Content (Join-Path $root 'CORE_VERSION') -Raw)"
