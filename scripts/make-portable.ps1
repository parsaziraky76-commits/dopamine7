<#
.SYNOPSIS
  ساخت نسخهٔ پرتابل.
.DESCRIPTION
  همان پیلود نصبی، بعلاوهٔ یک فایل نشانه (portable.flag).
  وجود این فایل باعث می‌شود برنامه تنظیمات و لاگ‌ها را کنار خود اجرایی
  ذخیره کند (نه در %LOCALAPPDATA%) — معنای واقعی پرتابل بودن.
#>
param(
  [Parameter(Mandatory = $true)][string]$Arch,
  [Parameter(Mandatory = $true)][string]$Version
)

$ErrorActionPreference = 'Stop'
$root    = Split-Path -Parent $PSScriptRoot
$payload = Join-Path $root "dist/payload-$Arch"
$stage   = Join-Path $root "dist/portable-$Arch/Dopamine"
$zip     = Join-Path $root "dist/Dopamine-Portable-$Version-$Arch.zip"

if (-not (Test-Path $payload)) { throw "Payload missing: $payload" }

Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item "$payload/*" $stage -Recurse -Force

# نشانهٔ حالت پرتابل
Set-Content -Path (Join-Path $stage 'portable.flag') -Value '1' -NoNewline -Encoding ascii

@"
Dopamine $Version - Portable ($Arch)
=================================

اجرا: روی Dopamine.exe کلیک کنید.

توجه: برقراری تونل نیازمند دسترسی مدیر (Administrator) است، چون آداپتور
      Wintun باید ساخته شود — دقیقاً معادل دیالوگ مجوز VPN در اندروید.
      برنامه در صورت نیاز خودش درخواست ارتقای دسترسی می‌دهد.

تمام تنظیمات و لاگ‌ها در کنار همین پوشه ذخیره می‌شوند.
بروزرسانی فقط از طریق کانال رسمی تلگرام Dopamine انجام می‌شود.
"@ | Set-Content -Path (Join-Path $stage 'READ-ME-FIRST.txt') -Encoding utf8

Remove-Item $zip -Force -ErrorAction SilentlyContinue
Compress-Archive -Path $stage -DestinationPath $zip -CompressionLevel Optimal
Write-Host "==> $zip"
