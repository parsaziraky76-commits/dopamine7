<#
.SYNOPSIS
  خواندنِ محتوای یک فایلِ باینری (مثل aether.exe) به‌صورت متن، برای جست‌وجوی
  رشته‌های ASCII تعبیه‌شده در آن (مثل مهرِ AETHER-BUILD-STAMP:).

.DESCRIPTION
  ریشهٔ خطا: در Windows PowerShell 5.1، `Select-String -Encoding Byte` هر بایت
  را مستقیم به یک کاراکتر نگاشت می‌کرد - دقیقاً چیزی که برای جست‌وجوی متنِ خامِ
  داخلِ یک باینری لازم است. رانرهای گیت‌هاب اکشنز اکنون از PowerShell 7 (pwsh
  Core) استفاده می‌کنند و در آن `Byte` دیگر یک نامِ Encoding معتبر نیست:
    Select-String: 'Byte' is not a supported encoding name.
  این خطا هیچ ربطی به کدِ برنامه یا بیلد ندارد - فقط رانتایمِ CI عوض شده.

  راه‌حلِ ریشه‌ای: به‌جای تکیه بر نامِ Encoding مبهمِ `Select-String`، فایل را
  مستقیماً به بایت می‌خوانیم و با Latin-1 (ISO-8859-1, codepage 28591) به متن
  تبدیل می‌کنیم. Latin-1 دقیقاً همان کاری را می‌کند که `Byte` قدیمی می‌کرد: هر
  بایت 0-255 را بدون خطا، بدون fallback و بدون از دست دادن هیچ بایتی به یک
  کاراکتر نگاشت می‌کند - برخلاف UTF-8 که روی بایت‌های نامعتبرِ باینری می‌تواند
  کاراکترهای جایگزین (U+FFFD) بگذارد و مرزبندیِ بایت‌ها را به‌هم بزند.
  این تابع در هر نسخهٔ PowerShell (5.1 یا 7+) یکسان کار می‌کند، پس دیگر به
  رفتار Encoding-ایِ Select-String در نسخه‌های مختلف وابسته نیستیم.
#>

function Get-BinaryAsLatin1Text {
  param([Parameter(Mandatory = $true)][string]$Path)
  $bytes = [System.IO.File]::ReadAllBytes($Path)
  return [System.Text.Encoding]::GetEncoding(28591).GetString($bytes)
}

# جست‌وجوی رشتهٔ ثابت (نه regex) - معادل Select-String -SimpleMatch -Quiet.
function Test-BinaryContainsLiteral {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Literal
  )
  return (Get-BinaryAsLatin1Text -Path $Path).Contains($Literal)
}

# همهٔ رشته‌های یکتایی که با یک الگوی regex مطابقت دارند - برای پیامِ خطای
# تشخیصی («مهرهایی که واقعاً حضور دارند»).
function Find-BinaryMatches {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Pattern
  )
  $text = Get-BinaryAsLatin1Text -Path $Path
  return ([regex]::Matches($text, $Pattern) | ForEach-Object { $_.Value } | Sort-Object -Unique)
}
