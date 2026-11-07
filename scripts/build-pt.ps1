<#
.SYNOPSIS
  ساخت ترابرِ افزودنیِ تور: lyrebird.exe (obfs4 / meek_lite / webtunnel).
.DESCRIPTION
  معادل ویندوزیِ `scripts/build-natives.sh pt` در مخزن اندروید، که همان
  باینری را به‌شکل `libpt-lyrebird.so` در jniLibs می‌گذارد.

  # این مرحله عمداً «حق شکستن بیلد» را ندارد

  پل‌های تور یک قابلیتِ افزودنی‌اند: تور بدون هیچ ترابری هم وصل می‌شود —
  مستقیم، و در حالتِ `Aether → Tor` از داخل تونل. اگر یک تغییرِ بالادست در Go
  ساختِ lyrebird را بشکند، نباید انتشارِ نصب‌کننده‌ای را نگه دارد که هر چیز
  دیگرش سالم است. پس نبودنِ Go یا شکستِ کلون، با کدِ خروجِ صفر و یک هشدارِ
  صریح رد می‌شود، و `engine.rs` در زمان اتصال غیبتش را در لاگ می‌نویسد.

  # چرا از سورس، و چرا همان تگِ موبایل

  همان دلیلِ `build-psiphon.ps1`: نسخه به نسخهٔ موبایل پین می‌شود
  (`lyrebird-0.6.1`) تا هر دو سکو یک ترابر را اجرا کنند و باگی که در یکی
  دیده می‌شود در دیگری غیرقابل‌بازتولید نباشد؛ و زنجیرهٔ تأمین از سورسِ
  تگ‌خوردهٔ خودِ Tor Project می‌آید، نه از آرتیفکتِ شخص سوم.

  # چرا CGO خاموش است

  نسخهٔ اندروید `CGO_ENABLED=1` دارد چون با کامپایلرِ NDK ساخته می‌شود.
  lyrebird روی ویندوز به CGO نیازی ندارد و روشن‌بودنش فقط بیلد را شکننده
  می‌کند — همان تصمیمی که در `build-psiphon.ps1` گرفته شده.

  خروجی: dist-engine/pt/lyrebird.exe  و  dist-engine/pt/LYREBIRD_VERSION
#>
param(
  [Parameter(Mandatory = $true)][string]$Arch,
  [string]$Version = 'lyrebird-0.6.1',
  [string]$Repo = 'https://gitlab.torproject.org/tpo/anti-censorship/pluggable-transports/lyrebird.git'
)

# نه `Stop`: هیچ خطایی در این فایل نباید کلِ بیلد را بکشد. هر مسیرِ شکست
# خودش پیام می‌دهد و با صفر برمی‌گردد.
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
$out  = Join-Path $root 'dist-engine/pt'

function Skip([string]$why) {
  Write-Host "==> [pt] $why"
  Write-Host "    Tor still works: directly, and through the tunnel in the Aether -> Tor mode."
  Write-Host "    Tor BRIDGES will not: the app logs the missing transport at connect time."
  exit 0
}

if (-not (Get-Command go -ErrorAction SilentlyContinue)) {
  Skip 'No Go toolchain on this runner - SKIPPING lyrebird.'
}

# پوشهٔ موقت: روی راناکِ گیت‌هاب `RUNNER_TEMP` هست، ولی وقتی کسی این اسکریپت
# را دستی روی ماشین خودش اجرا می‌کند نیست — و `Join-Path $null` آن‌وقت یک
# صفحهٔ خطای سرخ می‌دهد و بعدش `git clone` با مقصدِ خالی صدا می‌شود.
$tmpBase = @($env:RUNNER_TEMP, $env:TEMP, $env:TMPDIR) |
           Where-Object { $_ -and $_.Trim() } |
           Select-Object -First 1
if (-not $tmpBase) { $tmpBase = [System.IO.Path]::GetTempPath() }
$src = Join-Path $tmpBase 'lyrebird'
if (-not (Test-Path $src)) {
  Write-Host "==> [pt] cloning lyrebird $Version"
  git clone --quiet --depth 1 --branch $Version $Repo $src
  if ($LASTEXITCODE -ne 0) { Skip "Could not fetch lyrebird $Version." }
}
$rev = (git -C $src rev-parse --short HEAD) 2>$null
Write-Host "    lyrebird revision: $rev"

# GOARCH از معماری بیلد — ویندوز ۳۲بیتی همان 386 است.
$goarch = if ($Arch -eq 'x86') { '386' } else { 'amd64' }

New-Item -ItemType Directory -Force -Path $out | Out-Null
$target = Join-Path $out 'lyrebird.exe'

Push-Location $src
try {
  $env:GOOS = 'windows'
  $env:GOARCH = $goarch
  $env:CGO_ENABLED = '0'
  Write-Host "==> [pt] building lyrebird (GOOS=windows GOARCH=$goarch)"
  # -s -w: بدون سمبل و DWARF، مثل بقیهٔ باینری‌های منتشرشده.
  go build -trimpath -ldflags '-s -w' -o $target ./cmd/lyrebird
  if ($LASTEXITCODE -ne 0) { Skip 'lyrebird build failed (upstream Go change?).' }
}
finally { Pop-Location }

if (-not (Test-Path $target)) { Skip 'lyrebird build produced no binary.' }

# نامِ فایل باید دقیقاً `lyrebird.exe` باشد: هستهٔ ۲.۰.۰ در هر پوشهٔ جست‌وجو
# اول `<name>.exe` و بعد `<name>` را می‌بیند (bridges.rs:241) و همین یک
# باینری obfs4 و meek_lite و webtunnel را سرویس می‌دهد.
"$Version ($rev)" | Out-File -FilePath (Join-Path $out 'LYREBIRD_VERSION') -Encoding ascii -NoNewline
$size = [math]::Round((Get-Item $target).Length / 1MB, 1)
Write-Host "==> [pt] ready: dist-engine/pt/lyrebird.exe ($size MB, $Version)"
