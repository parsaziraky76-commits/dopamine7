// ویت تصویرها را به یک URL تبدیل می‌کند؛ Node نمی‌تواند `.png` را import کند.
// این قلاب همان کار ویت را در تست انجام می‌دهد تا هارنس مجبور نشود ماژول‌هایی
// که آیکن import می‌کنند را دور بزند.
//
// مسیر قلاب نسبت به همین فایل حل می‌شود، نه نسبت به پوشهٔ جاری؛ پس
// `node --import ./tests/png-loader.mjs` از هر مسیری کار می‌کند.
import { register } from 'node:module'

register('./png-hook.mjs', import.meta.url)
