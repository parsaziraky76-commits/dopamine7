<#
.SYNOPSIS  جمع‌آوری خروجی نهایی در dist/payload-<arch> (ورودی مشترک Setup و Portable).
#>
param(
  [Parameter(Mandatory=$true)][string]$Target,
  [Parameter(Mandatory=$true)][string]$Arch
)
$ErrorActionPreference = 'Stop'
$root    = Split-Path -Parent $PSScriptRoot
$relDir  = Join-Path $root "src-tauri/target/$Target/release"
$out     = Join-Path $root "dist/payload-$Arch"

# پیدا کردن اجرایی برنامه.
#
# مهم: وقتی Tauri را با `--bundles none` می‌سازیم، نام خروجی همان نام
# کریت در Cargo.toml است (aether-desktop.exe) نه productName (Dopamine.exe).
# قبلاً مستقیم Dopamine.exe را می‌خواستیم — یعنی یک خطای حتمی در مراحل بعد.
# حالا هر دو نام را می‌پذیریم و خروجی را به Dopamine.exe تغییر نام می‌دهیم.
$bin = $null
foreach ($name in @('Dopamine.exe', 'Aether.exe', 'aether-desktop.exe', 'aether_desktop.exe')) {
  $candidate = Join-Path $relDir $name
  if (Test-Path $candidate) { $bin = $candidate; break }
}
if (-not $bin) {
  # آخرین تلاش: هر exe سطح بالا که ابزار کمکی build/deps نباشد.
  $bin = Get-ChildItem (Join-Path $relDir '*.exe') -ErrorAction SilentlyContinue |
         Where-Object { $_.Name -notmatch '^(build|deps)' } |
         Sort-Object Length -Descending |
         Select-Object -First 1 -ExpandProperty FullName
}
if (-not $bin) {
  Write-Host "Contents of $relDir :"
  Get-ChildItem $relDir -ErrorAction SilentlyContinue | Select-Object Name, Length | Format-Table -AutoSize
  throw "App binary not found in $relDir"
}
Write-Host "==> App binary: $bin"

Remove-Item $out -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path (Join-Path $out 'engine') | Out-Null

# همیشه به نام Dopamine.exe کپی می‌شود — نصب‌کننده، میانبرها و
# قاعدهٔ فایروال همه به همین نام تکیه دارند.
Copy-Item $bin (Join-Path $out 'Dopamine.exe') -Force
Copy-Item (Join-Path $root 'dist-engine/aether.exe')  (Join-Path $out 'engine') -Force
Copy-Item (Join-Path $root 'dist-engine/wintun.dll')  (Join-Path $out 'engine') -Force
# استیج ۲ زنجیره + فهرست سرور تعبیه‌شده. کنار aether.exe می‌نشینند تا
# `prepare_runtime_engine` هر دو را با بقیهٔ موتور به پوشهٔ دادهٔ قابل‌نوشتن
# کاربر منتقل کند (همان رفع ریشه‌ای خطای Access is denied).
Copy-Item (Join-Path $root 'dist-engine/psiphon-tunnel-core.exe') (Join-Path $out 'engine') -Force
Copy-Item (Join-Path $root 'dist-engine/server_entries.txt')      (Join-Path $out 'engine') -Force
$psiVer = Join-Path $root 'dist-engine/PSIPHON_VERSION'
if (Test-Path $psiVer) { Copy-Item $psiVer (Join-Path $out 'engine') -Force }
# ترابرِ افزودنیِ تور (اگر ساخته شده باشد) در زیرپوشهٔ `pt` کنار موتور.
# هستهٔ ۲.۰.۰ همان `<پوشهٔ موتور>/pt` را می‌گردد، و `prepare_runtime_engine`
# این زیرپوشه را با بقیهٔ موتور به پوشهٔ دادهٔ قابل‌نوشتن منتقل می‌کند.
#
# نبودنش خطا نیست و نباید انتشار را نگه دارد: پل‌ها یک افزودنی‌اند، تور بدون
# آن‌ها هم وصل می‌شود. فقط با صدای بلند گفته می‌شود تا کسی نپرسد چرا نصبِ
# منتشرشده پل ندارد.
# تورِ رسمی (اگر تدارک شده باشد) در زیرپوشهٔ `tor` کنارِ موتور: tor.exe،
# lyrebird.exe، pt_config.json و دو فایلِ geoip. مسیرِ تورِ برنامه از همین
# باینری می‌گذرد، نه از arti داخلِ موتور؛ دلیلش در سرِ
# `src-tauri/src/tor_native.rs` است.
$torSrc = Join-Path $root 'dist-engine/tor'
if (Test-Path $torSrc) {
  $torOut = Join-Path $out 'engine/tor'
  New-Item -ItemType Directory -Force -Path $torOut | Out-Null
  Copy-Item (Join-Path $torSrc '*') $torOut -Force
  Write-Host "==> official tor bundled: engine/tor/tor.exe"
} else {
  Write-Host "==> no dist-engine/tor; this payload carries no tor binary"
}

# >>> AETHER-APP-PATCH custom-configs-xray
# هستهٔ Xray برای «کانفیگ‌های سفارشی» (scripts/stage-xray.ps1). نبودنش فقط
# همان قابلیت را از کار می‌اندازد — شش حالتِ اصلی به آن دست نمی‌زنند — پس
# این‌جا خطا نیست؛ گیتِ انتشار در ورک‌فلو جلوی بیلدِ منتشرشدهٔ بی‌Xray را می‌گیرد.
$xraySrc = Join-Path $root 'dist-engine/xray'
if (Test-Path (Join-Path $xraySrc 'xray.exe')) {
  $xrayOut = Join-Path $out 'engine/xray'
  New-Item -ItemType Directory -Force -Path $xrayOut | Out-Null
  Copy-Item (Join-Path $xraySrc '*') $xrayOut -Force
  Write-Host "==> xray core bundled: engine/xray/xray.exe"
} else {
  Write-Host "==> NOTE: no dist-engine/xray - custom configs will report a missing core."
}
# <<< AETHER-APP-PATCH custom-configs-xray

$ptSrc = Join-Path $root 'dist-engine/pt'
if (Test-Path (Join-Path $ptSrc 'lyrebird.exe')) {
  $ptOut = Join-Path $out 'engine/pt'
  New-Item -ItemType Directory -Force -Path $ptOut | Out-Null
  Copy-Item (Join-Path $ptSrc '*') $ptOut -Force
  Write-Host "==> pluggable transport bundled: engine/pt/lyrebird.exe"
} else {
  Write-Host "==> NOTE: no pluggable transport in this build - Tor works, Tor BRIDGES do not."
}

# نسخهٔ واقعی هستهٔ سینک‌شده را در payload بگذار (نه برچسب ثابت ریشهٔ مخزن).
$realVer = Join-Path $root 'native/aether/CORE_VERSION'
if (Test-Path $realVer) { Copy-Item $realVer (Join-Path $out 'engine') -Force }
else { Copy-Item (Join-Path $root 'CORE_VERSION') (Join-Path $out 'engine') -Force }
Copy-Item (Join-Path $root 'LICENSE')                 $out -Force -ErrorAction SilentlyContinue

# دروازهٔ سلامت: هیچ پیلودی بدون موتور و درایور نباید منتشر شود.
foreach ($f in @('Dopamine.exe','engine/aether.exe','engine/wintun.dll','engine/psiphon-tunnel-core.exe','engine/server_entries.txt')) {
  if (-not (Test-Path (Join-Path $out $f))) { throw "Payload incomplete: $f is missing" }
}
Get-ChildItem $out -Recurse | Select-Object FullName, Length | Format-Table -AutoSize
Write-Host "==> payload ready: $out"
