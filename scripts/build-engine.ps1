<#
.SYNOPSIS
  کامپایل هستهٔ aether برای ویندوز (دقیقاً همان سورس مخزن اندروید).
.DESCRIPTION
  معادل scripts/build-natives.sh در AetherMobile.
  خروجی: dist-engine/aether.exe

  سخت‌سازی‌های اضافه‌شده تا بیلد بی‌دلیل نشکند:
    - مسیر کریت هسته دیگر ثابت فرض نمی‌شود؛ خودش پیدایش می‌کند.
    - اگر Cargo.lock نباشد، بدون --locked دوباره تلاش می‌کند.
#>
param(
  [Parameter(Mandatory = $true)][string]$Target
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$out  = Join-Path $root 'dist-engine'
New-Item -ItemType Directory -Force -Path $out | Out-Null

# ---- پیدا کردن کریت هسته -------------------------------------------
# چیدمان مخزن بالادست ممکن است تغییر کند؛ مسیر ثابت یک بمب ساعتی است.
$candidates = @(
  (Join-Path $root 'native/aether/aether'),
  (Join-Path $root 'native/aether')
)
$core = $null
foreach ($c in $candidates) {
  if (Test-Path (Join-Path $c 'Cargo.toml')) { $core = $c; break }
}
if (-not $core) {
  # آخرین تلاش: دنبال هر Cargo.toml که یک باینری به نام aether می‌سازد.
  $found = Get-ChildItem (Join-Path $root 'native') -Recurse -Filter 'Cargo.toml' -ErrorAction SilentlyContinue |
           Where-Object { (Get-Content $_.FullName -Raw) -match 'name\s*=\s*"aether"' } |
           Select-Object -First 1
  if ($found) { $core = Split-Path -Parent $found.FullName }
}
if (-not $core) {
  throw "Could not locate the aether core crate under $root\native (did sync-core run?)"
}
Write-Host "==> Core crate: $core"
Write-Host "==> Building aether core for $Target"

# The Tor-enabled binary exceeds Rust's default query depth (128) while
# computing the async main future on both Windows targets. Apply this after
# core sync and before every build, including the rollback build: sync-core
# can replace the vendored main.rs with an upstream copy.
$mainPath = Join-Path $core 'src/main.rs'
if (-not (Test-Path $mainPath)) { throw "Core binary entry point is missing: $mainPath" }
$mainSource = [System.IO.File]::ReadAllText($mainPath)
$limitPattern = '(?m)^#!\[recursion_limit\s*=\s*"(\d+)"\]'
$limitMatch = [regex]::Match($mainSource, $limitPattern)
if (-not $limitMatch.Success) {
  $mainSource = "#![recursion_limit = `"512`"]`n" + $mainSource
  [System.IO.File]::WriteAllText($mainPath, $mainSource, [System.Text.UTF8Encoding]::new($false))
} elseif ([int]$limitMatch.Groups[1].Value -lt 512) {
  $mainSource = [regex]::Replace($mainSource, $limitPattern, '#![recursion_limit = "512"]')
  [System.IO.File]::WriteAllText($mainPath, $mainSource, [System.Text.UTF8Encoding]::new($false))
}

# static-CRT: خروجی نباید به Visual C++ Redistributable وابسته باشد، وگرنه
# نسخهٔ پرتابل روی بعضی سیستم‌ها اجرا نمی‌شود.
$env:RUSTFLAGS = '-C target-feature=+crt-static'

# ---- فیچرها -------------------------------------------------------
# ۱.۲.۵: حالت‌های تور (`--tor` / `--tor-only` / `--tor-reverse`) فقط در باینری‌ای
# وجود دارند که با فیچر `tor` کامپایل شده باشد؛ هستهٔ ۲.۰.۰ خودِ arti را پشت
# همین فیچر دارد. بدون این سوییچ، رابط کاربری چهار حالت تور را نشان
# می‌داد و موتور هیچ‌کدام را نمی‌شناخت.
#
# ولی فیچر **کورکورانه** پاس داده نمی‌شود. اگر core-rollback.ps1 به هستهٔ
# قدیمی‌تری برگشته باشد، آن Cargo.toml فیچر `tor` را ندارد و
# `cargo build --features tor` کل بیلد را می‌کشت — درست همان چیزی که مسیر
# rollback برای جلوگیری ازش هست. قاعدهٔ همیشگی مخزن: ارتقا/برگشتِ خودکار
# هسته هرگز نباید یک انتشار را بشکند.
$featureArgs = @()
$coreToml = Get-Content (Join-Path $core 'Cargo.toml') -Raw
if ($coreToml -match '(?m)^\s*tor\s*=\s*\[') {
  $featureArgs = @('--features', 'tor')
  Write-Host '==> Engine features: tor (arti inside the engine)'
} else {
  Write-Host '::warning::This core has no `tor` feature - building without it. The Tor transport modes will not exist in this engine.'
}

Push-Location $core
try {
  $useLocked = Test-Path (Join-Path $core 'Cargo.lock')
  if ($useLocked) {
    cargo build --release --locked --target $Target @featureArgs
    if ($LASTEXITCODE -ne 0) {
      Write-Host '::warning::--locked build failed; retrying without it.'
      cargo build --release --target $Target @featureArgs
    }
  } else {
    cargo build --release --target $Target @featureArgs
  }
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

  $bin = Join-Path $core "target/$Target/release/aether.exe"
  if (-not (Test-Path $bin)) {
    # برخی نسخه‌های هسته باینری را با نام دیگری تولید می‌کنند.
    $bin = Get-ChildItem "target/$Target/release/*.exe" -ErrorAction SilentlyContinue |
           Where-Object { $_.Name -notmatch 'build|deps' } |
           Select-Object -First 1 -ExpandProperty FullName
  }
  if (-not $bin) { throw 'aether.exe was not produced' }

  Copy-Item $bin (Join-Path $out 'aether.exe') -Force
  Write-Host "==> engine -> dist-engine/aether.exe"

  # همان کاری که مخزن اندروید می‌کند: نسخهٔ هسته در خروجی ثبت شود
  # تا در پنل About نمایش داده شود.
  # نسخهٔ واقعی هسته را ثبت کن، نه برچسب ثابت مخزن.
  # sync-core نسخهٔ واقعی (مثلاً 1.4.0) را در native/aether/CORE_VERSION می‌نویسد؛
  # فایل ریشهٔ مخزن فقط یک برچسب baseline است و پنل About را گمراه می‌کرد.
  $realVer = Join-Path $root 'native/aether/CORE_VERSION'
  if (Test-Path $realVer) {
    Copy-Item $realVer (Join-Path $out 'CORE_VERSION') -Force
  } else {
    Copy-Item (Join-Path $root 'CORE_VERSION') (Join-Path $out 'CORE_VERSION') -Force -ErrorAction SilentlyContinue
  }
}
finally { Pop-Location }
