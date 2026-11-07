<#
.SYNOPSIS
  تدارکِ هستهٔ Xray برای «کانفیگ‌های سفارشی»: dist-engine/xray/{xray.exe, LICENSE}

.DESCRIPTION
  کانفیگ‌های سفارشی (vless/vmess/trojan/ss/socks/hysteria2/wireguard و فایلِ
  npvt) را یک پروسهٔ جداگانهٔ xray.exe اجرا می‌کند، همان‌طور که اندرویدِ
  Aether 1.5.0 همین هسته را به کار می‌گیرد. توضیحِ کامل در
  `src-tauri/src/custom/process.rs`.

  چرا v25.12.8 و نه v26: نسخه‌های ۲۶ کلیدِ `allowInsecure` و `seed`ِ mKCP را
  رد می‌کنند و بخشِ بزرگی از کانفیگ‌هایی که کاربران دارند (و فایل‌های npvtِ
  قدیمی) از کار می‌افتند. اندروید هم روی همین خط مانده است.

  هر دو دایجستِ zip از فایلِ `.dgst`ِ منتشرشده کنارِ همان ریلیز در
  github.com/XTLS/Xray-core برداشته شده و با دانلودِ مستقل هم سنجیده شده‌اند.
  دایجستِ xray.exeِ داخلِ بسته هم جداگانه پین است.

  geoip.dat / geosite.dat عمداً کپی نمی‌شوند: کانفیگی که برنامه می‌سازد هیچ
  قاعدهٔ geoip:/geosite: ندارد و ۳۰ مگابایت به نصب‌کننده اضافه می‌کردند.

  خروجی: dist-engine/xray/  و  dist-engine/XRAY_VERSION
#>

param(
  [string]$Version = '25.12.8',
  [string]$Arch    = 'x64',
  [hashtable]$ZipSha256 = @{
    'x64' = '5419794670658e14519409320752329375e0fd8a57089f373b3febe9bf3ec5d5'
    'x86' = 'b270190001fd9e566b8c1dc5f4bd8dac62d261d26fe23b7e42953e11adcde8f9'
  },
  [hashtable]$ExeSha256 = @{
    'x64' = '525a4b7d396769dc02beaedce2ee378968b510676b7a46e2b1c6348165b8a0d4'
    'x86' = '9979efec7a45b4923b81f5ea7423159fc2ddc758d527753e7d2a2d7e75b8ce7a'
  }
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Skip([string]$why) {
  # مثلِ stage-tor: دانلودِ ناموفق یک PR را زمین نمی‌زند؛ گیتِ انتشار در
  # ورک‌فلو جلوی منتشرشدنِ بیلدِ بی‌Xray را می‌گیرد.
  Write-Host "::warning::[xray] $why"
  Write-Host "==> [xray] SKIPPED"
  exit 0
}

$root = Split-Path -Parent $PSScriptRoot
$out  = Join-Path $root 'dist-engine/xray'

# جدولِ صریح تا معماریِ ناشناخته بی‌صدا به ۶۴ بیتی نیفتد.
$slug = @{ 'x64' = '64'; 'x86' = '32' }[$Arch]
if (-not $slug) { Skip "unknown architecture: $Arch" }
if (-not $ZipSha256.ContainsKey($Arch) -or -not $ExeSha256.ContainsKey($Arch)) {
  Skip "no pinned digest for $Arch"
}

$asset   = "Xray-windows-$slug.zip"
$url     = "https://github.com/XTLS/Xray-core/releases/download/v$Version/$asset"
$tmp     = Join-Path ([System.IO.Path]::GetTempPath()) "aether-xray-$Version-$Arch"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
$archive = Join-Path $tmp $asset

Write-Host "==> [xray] downloading $url"
$fetched = $false
$failures = @()
foreach ($attempt in 1..3) {
  try {
    Remove-Item $archive -Force -ErrorAction SilentlyContinue
    Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing -TimeoutSec 600
    $fetched = $true
    break
  } catch {
    $msg = $_.Exception.Message
    $failures += "try ${attempt}: $msg"
    Write-Host "    [xray] try $attempt failed: $msg"
    if ($msg -match '404') { break }
    Start-Sleep -Seconds (5 * $attempt)
  }
}
if (-not $fetched) { Skip ("download of $asset failed: " + ($failures -join ' | ')) }

$got = (Get-FileHash -Path $archive -Algorithm SHA256).Hash.ToLower()
if ($got -ne $ZipSha256[$Arch].ToLower()) {
  # Skip نیست: دایجستِ نخوانده یعنی پین دارد کار می‌کند.
  throw "[xray] digest mismatch for ${asset}: expected $($ZipSha256[$Arch]), got $got"
}
Write-Host "    zip digest ok: $got"

$ex = Join-Path $tmp 'x'
Remove-Item $ex -Recurse -Force -ErrorAction SilentlyContinue
Expand-Archive -Path $archive -DestinationPath $ex -Force
$exe = Join-Path $ex 'xray.exe'
if (-not (Test-Path $exe)) { Skip "archive has no xray.exe; upstream layout changed" }
$gotExe = (Get-FileHash -Path $exe -Algorithm SHA256).Hash.ToLower()
if ($gotExe -ne $ExeSha256[$Arch].ToLower()) {
  throw "[xray] xray.exe digest mismatch: expected $($ExeSha256[$Arch]), got $gotExe"
}
Write-Host "    exe digest ok: $gotExe"

New-Item -ItemType Directory -Force -Path $out | Out-Null
Copy-Item $exe $out -Force
# Xray-core زیرِ MPL-2.0 است؛ متنِ مجوز همراهِ باینری می‌رود.
$lic = Join-Path $ex 'LICENSE'
if (Test-Path $lic) { Copy-Item $lic (Join-Path $out 'LICENSE') -Force }

"$Version" | Out-File -FilePath (Join-Path $root 'dist-engine/XRAY_VERSION') -Encoding ascii -NoNewline

$size = [math]::Round((Get-Item (Join-Path $out 'xray.exe')).Length / 1MB, 1)
Write-Host "==> [xray] ready: dist-engine/xray/xray.exe ($size MB, v$Version, $Arch)"