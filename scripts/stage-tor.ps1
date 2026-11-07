<#
.SYNOPSIS
  تدارکِ تورِ رسمی: dist-engine/tor/{tor.exe, lyrebird.exe, pt_config.json, geoip, geoip6}

.DESCRIPTION
  همان بسته‌ای که مرورگر تور از آن ساخته می‌شود — «tor expert bundle» — دانلود
  و با دایجستِ منتشرشدهٔ خودِ Tor سنجیده می‌شود.

  چرا این اسکریپت اضافه شد: مسیرِ تورِ برنامه تا ۱.۲.۵ روی arti بود و لاگِ
  میدانیِ ۱۶ سپتامبر ۲۰۲۶ نشان داد در ایران روی ۱۵٪ می‌ماند. سه محدودیتِ
  arti در همان لاگ دیده می‌شود، و هیچ‌کدام تنظیماتی نیست:

    * arti برای هر پل هویت می‌خواهد؛ سطرِ meek_liteی که خودِ Tor منتشر می‌کند
      فینگرپرینت ندارد و در arti با «none of the bridges could be read by tor»
      می‌سوزد.
    * فهرستِ پل‌های داخلی را ما دستی نگه می‌داشتیم و می‌پوسید.
    * هر سپرهٔ ناکام یک کلاینتِ کاملِ arti بود که نمی‌مُرد.

  روش از پروژهٔ WhiteAesther 1.9.4 گرفته شده (`scripts/stage-tor.mjs`)، که
  هستهٔ Aether 2.0.0 را دارد و تورش روی ویندوز کار می‌کند.

  دایجستِ زیر دستی از `sha256sums-signed-build.txt` برداشته نشده و حدس هم
  نیست: بستهٔ ۱۵.۰.۲۳ در همین مخزن دانلود و sha256 آن محاسبه شد و با عددی که
  Tor منتشر کرده یکی بود.

  خروجی: dist-engine/tor/  و  dist-engine/TOR_VERSION
#>

param(
  [string]$Version = '15.0.23',
  [string]$Arch    = 'x64',
  # دایجست به تفکیکِ معماری. هر دو از فایلِ امضاشدهٔ خودِ Tor
  # (`sha256sums-signed-build.txt` در همان پوشهٔ انتشار) گرفته شده‌اند و با
  # دانلودِ مستقل هم سنجیده شده‌اند.
  [hashtable]$Sha256 = @{
    'x64' = '231dad6b9cb401a54c260db7046965ef04e4f72ff071b140d423fb5da281ab1e'
    'x86' = '1e4de9a4f1d99b8f40b5e0c75f3dcc3ea51b0aeab040d48fd23881e9fa94979a'
  }
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Skip([string]$why) {
  # مثلِ گامِ lyrebird: با کدِ صفر و هشدارِ صریح بیرون می‌آییم. گیتِ انتشار در
  # ورک‌فلو جلوی منتشرشدنِ بیلدِ بی‌تور را می‌گیرد؛ این اسکریپت یک PR را زمین
  # نمی‌زند.
  Write-Host "::warning::[tor] $why"
  Write-Host "==> [tor] SKIPPED"
  exit 0
}

$root = Split-Path -Parent $PSScriptRoot
$out  = Join-Path $root 'dist-engine/tor'

# نامِ معماری در فایلِ Tor با نامِ ما یکی نیست: ما x64/x86 می‌گوییم، Tor
# x86_64/i686. جدول صریح است تا یک معماریِ ناشناخته بی‌صدا به x64 نیفتد و
# باینریِ ۶۴ بیتی در بستهٔ ۳۲ بیتی جا نگیرد.
$slug = @{ 'x64' = 'x86_64'; 'x86' = 'i686' }[$Arch]
if (-not $slug) { Skip "معماریِ ناشناخته: $Arch" }
if (-not $Sha256.ContainsKey($Arch)) { Skip "برای $Arch دایجستِ پین‌شده‌ای ندارم." }
$expected = $Sha256[$Arch]

$asset = "tor-expert-bundle-windows-$slug-$Version.tar.gz"
$tmp   = Join-Path ([System.IO.Path]::GetTempPath()) "aether-tor-$Version"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
$archive = Join-Path $tmp $asset

# رفعِ ریشه‌ایِ شکستِ ۴ اکتبر ۲۰۲۶ (404 روی هر دو معماری):
# dist.torproject.org فقط نسخه‌های جاری را نگه می‌دارد. با انتشارِ Tor Browser
# 15.0.24 (۲۹ سپتامبر) پوشهٔ 15.0.23 از آن‌جا برداشته شد، در حالی که بیلدِ ما
# عمداً روی 15.0.23 و دایجستش پین است. archive.torproject.org همهٔ نسخه‌ها را
# برای همیشه نگه می‌دارد و فایلِ آن بایت‌به‌بایت همان است (دایجستِ پین‌شده در
# sha256sums-signed-build.txt همان آرشیو هم هست). پس اول dist (سریع‌تر)، بعد
# آرشیو؛ دایجست در هر حال پایینِ همین بلوک سنجیده می‌شود، پس آینهٔ دوم هیچ
# اعتمادِ تازه‌ای اضافه نمی‌کند.
$mirrors = @(
  "https://dist.torproject.org/torbrowser/$Version/$asset",
  "https://archive.torproject.org/tor-package-archive/torbrowser/$Version/$asset"
)

Write-Host "==> [tor] downloading $asset"
$fetched  = $false
$failures = @()
foreach ($url in $mirrors) {
  foreach ($attempt in 1..3) {
    try {
      Remove-Item $archive -Force -ErrorAction SilentlyContinue
      Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing -TimeoutSec 600
      Write-Host "    from: $url"
      $fetched = $true
      break
    } catch {
      $msg = $_.Exception.Message
      $failures += "$url (try $attempt): $msg"
      Write-Host "    [tor] $url try $attempt failed: $msg"
      # 404 یعنی این آینه اصلاً این نسخه را ندارد؛ تکرار بی‌فایده است.
      if ($msg -match '404') { break }
      Start-Sleep -Seconds (5 * $attempt)
    }
  }
  if ($fetched) { break }
}
if (-not $fetched) {
  Skip ("دانلودِ $asset از هیچ آینه‌ای نشد: " + ($failures -join ' | '))
}

$got = (Get-FileHash -Path $archive -Algorithm SHA256).Hash.ToLower()
if ($got -ne $expected.ToLower()) {
  # این یکی Skip نیست. فایلی که دایجستش نمی‌خواند یعنی پین دارد کار می‌کند، و
  # هیچ بیلدی نباید با باینریِ تأییدنشدهٔ تور جلو برود.
  throw "[tor] digest mismatch for ${asset}: expected $expected, got $got"
}
Write-Host "    digest ok: $got"

# بسته tar.gz است و tar در ویندوزِ ۱۰ به بعد داخلی است.
Write-Host "==> [tor] extracting"
& tar -xzf $archive -C $tmp
if ($LASTEXITCODE -ne 0) { Skip 'بازکردنِ بسته نشد (tar).' }

$torExe   = Join-Path $tmp 'tor/tor.exe'
$lyrebird = Join-Path $tmp 'tor/pluggable_transports/lyrebird.exe'
$ptConfig = Join-Path $tmp 'tor/pluggable_transports/pt_config.json'
$geoip    = Join-Path $tmp 'data/geoip'
$geoip6   = Join-Path $tmp 'data/geoip6'

foreach ($needed in @($torExe, $lyrebird, $ptConfig, $geoip, $geoip6)) {
  if (-not (Test-Path $needed)) { Skip "بسته $needed را ندارد؛ چیدمانِ بالادست عوض شده." }
}

New-Item -ItemType Directory -Force -Path $out | Out-Null
Copy-Item $torExe   $out -Force
Copy-Item $lyrebird $out -Force
Copy-Item $ptConfig $out -Force
Copy-Item $geoip    $out -Force
Copy-Item $geoip6   $out -Force

# ترابرهای اختیاری، اگر بستهٔ بالادست داشته باشد.
#
# چرا اختیاری و نه اجباری: چیدمانِ `pluggable_transports` بین نسخه‌ها عوض
# می‌شود و نبودنِ اسنوفلیک نباید انتشارِ یک نصب‌کننده را که obfs4 در آن کار
# می‌کند نگه دارد. سمتِ برنامه هم همین قرارداد را دارد: `available_plugins`
# فقط ترابری را به torrc می‌نویسد که فایلش روی دیسک باشد، پس نبودنِ این فایل
# یعنی «موجِ اسنوفلیک وجود ندارد»، نه یک وعدهٔ شکسته روی ماشینِ کاربر.
$optional = @('snowflake-client.exe', 'conjure-client.exe')
$staged   = @()
foreach ($name in $optional) {
  $candidate = Join-Path $tmp "tor/pluggable_transports/$name"
  if (Test-Path $candidate) {
    Copy-Item $candidate $out -Force
    $staged += $name
  }
}
if ($staged.Count -gt 0) {
  Write-Host "==> [tor] optional transports bundled: $($staged -join ', ')"
} else {
  Write-Host "==> [tor] WARNING: bundle carries no snowflake-client.exe - the snowflake wave will be skipped at runtime"
}

# فهرستِ پل‌های داخلی از همین فایل خوانده می‌شود، پس اگر خالی باشد بهتر است
# همین‌جا معلوم شود، نه روی ماشینِ کاربر.
$bridges = (Get-Content $ptConfig -Raw | ConvertFrom-Json).bridges
$counts  = foreach ($name in $bridges.PSObject.Properties.Name) {
  "$name=$($bridges.$name.Count)"
}
if (-not $counts) { throw "[tor] pt_config.json هیچ پلِ داخلی ندارد." }

"$Version" | Out-File -FilePath (Join-Path $root 'dist-engine/TOR_VERSION') -Encoding ascii -NoNewline

$size = [math]::Round((Get-Item (Join-Path $out 'tor.exe')).Length / 1MB, 1)
Write-Host "==> [tor] ready: dist-engine/tor/tor.exe ($size MB, $Version); built-in bridges: $($counts -join ', ')"
