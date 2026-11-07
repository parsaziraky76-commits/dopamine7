<#
.SYNOPSIS  تست دودی خودکار: نصب بی‌صدا → راستی‌آزمایی فایل‌ها → حذف بی‌صدا.
.DESCRIPTION
  هیچ نصب‌کننده‌ای نباید منتشر شود که خود CI آن را نصب و حذف نکرده باشد.
  معادل مرحلهٔ «Verify release signature / verify APK» در پایپ‌لاین اندروید.
#>
param(
  [Parameter(Mandatory=$true)][string]$Arch,
  [Parameter(Mandatory=$true)][string]$Version,
  # فقط بیلدی که منتشر می‌شود باید تورِ رسمی را در درختِ نصب‌شده داشته باشد؛
  # در بیلدِ PR گامِ تدارکِ تور دروازه ندارد و ممکن است چیزی نگذاشته باشد.
  [switch]$RequireTor
)
$ErrorActionPreference = 'Stop'
$root  = Split-Path -Parent $PSScriptRoot
$setup = Join-Path $root "dist/Dopamine-Setup-$Version-$Arch.exe"
if (-not (Test-Path $setup)) { throw "Installer not found: $setup" }

$target = Join-Path $env:TEMP "DopamineSmoke-$Arch"
Remove-Item $target -Recurse -Force -ErrorAction SilentlyContinue

# هیچ مرحله‌ای حق ندارد بی‌نهایت منتظر بماند. اگر فرآیندی از سقف
# زمانی بگذرد (مثلاً چون پنجره‌ای نامرئی منتظر کلیک مانده)،
# همان‌جا کشته و با پیام روشن شکست اعلام می‌شود.
# هشدار مهم برای آینده: اسم پارامتر هرگز نباید `$Args` باشد!
# `$Args` در PowerShell یک متغیر خودکار رزروشده است و مقدارش داخل
# تابع خالی می‌شود. نتیجهٔ اجرای قبلی: نصب‌کننده بدون سوییچ‌های
# بی‌صدا اجرا شد، ویزارد گرافیکی روی دسکتاپ نامرئی CI باز ماند
# و تا سقف زمانی منتظر کلیک ماند.
function Invoke-WithTimeout {
  param(
    [string]$Exe,
    [string[]]$ArgumentList,
    [int]$TimeoutSec,
    [string]$What,
    [string]$LogFile = ''
  )
  Write-Host "==> $What : `"$Exe`" $($ArgumentList -join ' ')"
  if (-not $ArgumentList -or $ArgumentList.Count -eq 0) {
    throw "$What was about to run WITHOUT arguments - refusing (this would open an interactive wizard on CI)."
  }
  $p = Start-Process -FilePath $Exe -ArgumentList $ArgumentList -PassThru
  if (-not $p.WaitForExit($TimeoutSec * 1000)) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    if ($LogFile -and (Test-Path $LogFile)) {
      Write-Host "---- last lines of $LogFile ----"
      Get-Content $LogFile -Tail 60 | Write-Host
    }
    throw "$What did not finish within $TimeoutSec seconds - it was probably waiting on a hidden dialog. Killed."
  }
  return $p.ExitCode
}

Write-Host '==> Silent install'
$installLog = Join-Path $env:TEMP 'aether-install.log'
$code = Invoke-WithTimeout -Exe $setup -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/ALLUSERS',"/DIR=`"$target`"","/LOG=`"$installLog`"") -TimeoutSec 300 -What 'Installer' -LogFile $installLog
if ($code -ne 0) {
  # هر شکستِ نصب، نه فقط تایم‌اوت، باید لاگش را نشان بدهد.
  if (Test-Path $installLog) {
    Write-Host "---- last 120 lines of $installLog ----"
    Get-Content $installLog -Tail 120 | Write-Host
  } else {
    Write-Host "::warning::نصب‌کننده هیچ لاگی ننوشت ($installLog) - يعنی پیش از شروعِ نصب افتاد."
  }
  throw "Installer exited with $code"
}

# تا امروز این فهرست پوشهٔ tor را نمی‌دید: نصب‌کننده‌ای که «تور تنها» در آن
# اجراشدنی نیست، سبز از CI بیرون می‌آمد و فقط روی کامپیوترِ کاربر لو می‌رفت.
$needed = @('Dopamine.exe','engine\aether.exe','engine\wintun.dll',
            'engine\psiphon-tunnel-core.exe','engine\server_entries.txt','unins000.exe')
if ($RequireTor) {
  $needed += @('engine\tor\tor.exe','engine\tor\lyrebird.exe','engine\tor\pt_config.json',
               'engine\tor\geoip','engine\tor\geoip6')
  # کانفیگ‌های سفارشی بی xray.exe فقط خطای «هسته پیدا نشد» می‌دهند.
  $needed += @('engine\xray\xray.exe')
} else {
  Write-Host '==> (بیلدِ بدونِ انتشار: پوشهٔ tor اجباری نیست)'
}
$missing = @()
foreach ($f in $needed) { if (-not (Test-Path (Join-Path $target $f))) { $missing += $f } }
if ($missing.Count -gt 0) {
  # همهٔ کمبودها یک‌جا، نه اولی: وگرنه هر بار ۴۰ دقیقه برای یک نام صرف می‌شود.
  Write-Host '---- what the installer actually put there ----'
  Get-ChildItem $target -Recurse -File | ForEach-Object {
    Write-Host ('   ' + $_.FullName.Substring($target.Length + 1))
  }
  throw ('Installed tree is missing ' + $missing.Count + ' file(s): ' + ($missing -join ', '))
}
Write-Host ('==> Installed tree OK (' + $needed.Count + ' files checked, Uninstaller present)')

# مطمئن می‌شویم موتور واقعاً اجراشدنی است (معماری درست، CRT استاتیک).
# هر هسته‌ای پرچم --version را پشتیبانی نمی‌کند، پس این بررسی فقط
# گزارشی است. دروازهٔ واقعی، وجود فایل‌های بالاست.
$engine = Join-Path $target 'engine\aether.exe'
try {
  & $engine --version 2>&1 | Write-Host
  Write-Host "==> engine exit code: $LASTEXITCODE"
} catch {
  Write-Host "::warning::Could not run engine --version: $_"
}

Write-Host '==> Silent uninstall'
$ucode = Invoke-WithTimeout -Exe (Join-Path $target 'unins000.exe') -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -TimeoutSec 180 -What 'Uninstaller'
if ($ucode -ne 0) { throw "Uninstaller exited with $ucode" }
Start-Sleep -Seconds 3
if (Test-Path (Join-Path $target 'Dopamine.exe')) { throw 'Uninstall left the application behind' }

Write-Host '==> Smoke test passed'
