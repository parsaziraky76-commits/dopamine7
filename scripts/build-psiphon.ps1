<#
.SYNOPSIS
  ساخت استیج ۲ زنجیره: psiphon-tunnel-core.exe (ConsoleClient).
.DESCRIPTION
  معادل ویندوزیِ psiphontunnel-<ver>.aar در مخزن اندروید.

  چرا از سورس ساخته می‌شود و باینری آماده دانلود نمی‌شود:
    * نسخه به نسخهٔ موبایل پین می‌شود (PSIPHON_VERSION) تا هر دو سکو یک
      کتابخانه را اجرا کنند و یک باگ در یکی، در دیگری غیرقابل‌بازتولید نباشد.
    * زنجیرهٔ تأمین: چیزی که منتشر می‌کنیم از سورسِ تگ‌خوردهٔ بالادست آمده،
      نه از یک آرتیفکت شخص سوم.

  خروجی: dist-engine/psiphon-tunnel-core.exe  و  dist-engine/server_entries.txt
#>
param(
  [Parameter(Mandatory = $true)][string]$Arch,
  [string]$Version = '83aa73b9b982e7421e00117f5b0c5aceb5dda452'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$out  = Join-Path $root 'dist-engine'
New-Item -ItemType Directory -Force -Path $out | Out-Null

# GOARCH از معماری بیلد. ویندوز ۳۲بیتی همان 386 است.
$goarch = if ($Arch -eq 'x86') { '386' } else { 'amd64' }

$src = Join-Path $env:RUNNER_TEMP ('psiphon-tunnel-core-' + $Version)
$repo = 'https://github.com/CluvexStudio/psiphon-tunnel-core.git'
if (-not (Test-Path $src)) {
  Write-Host "==> Cloning psiphon-tunnel-core v$Version"
  git clone --filter=blob:none --no-checkout $repo $src
  if ($LASTEXITCODE -ne 0) { throw "Could not fetch psiphon-tunnel-core $Version" }
}

Push-Location $src
try {
  git fetch --depth 1 origin $Version
  if ($LASTEXITCODE -ne 0) { throw "Could not fetch pinned Psiphon commit $Version" }
  git checkout --detach $Version
  if ($LASTEXITCODE -ne 0) { throw "Could not checkout pinned Psiphon commit $Version" }
  $actual = (git rev-parse HEAD).Trim()
  if ($LASTEXITCODE -ne 0 -or $actual -ne $Version) { throw "Psiphon commit mismatch: $actual" }

  $env:GOOS = 'windows'
  $env:GOARCH = $goarch
  # CGO لازم نیست و روی ویندوز فقط بیلد را شکننده می‌کند.
  $env:CGO_ENABLED = '0'
  $target = Join-Path $out 'psiphon-tunnel-core.exe'
  Write-Host "==> Building ConsoleClient (GOOS=windows GOARCH=$goarch)"
  # -s -w: بدون سمبل و DWARF — همان روحیهٔ strip در پروفایل release هستهٔ Rust.
  $env:GOFLAGS = "-mod=vendor"
  go build -trimpath -ldflags "-s -w" -o $target ./ConsoleClient
  if ($LASTEXITCODE -ne 0) { throw 'psiphon-tunnel-core build failed' }
  Write-Host "==> stage 2 -> dist-engine/psiphon-tunnel-core.exe"
}
finally { Pop-Location }

# فهرست سرور تعبیه‌شده — همان فایلی که نسخهٔ موبایل در assets دارد.
$entries = Join-Path $root 'assets/psiphon/server_entries.txt'
if (-not (Test-Path $entries)) { throw "Embedded server list is missing: $entries" }
Copy-Item $entries (Join-Path $out 'server_entries.txt') -Force

"$Version" | Out-File -FilePath (Join-Path $out 'PSIPHON_VERSION') -Encoding ascii -NoNewline
Write-Host "==> psiphon stage ready (v$Version, $goarch)"
