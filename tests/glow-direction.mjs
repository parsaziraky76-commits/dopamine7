// tests/glow-direction.mjs — جهتِ چرخشِ نور دور کارت اتصال، اندازه‌گیری‌شده.
//
// # چرا این تست وجود دارد
//
// مورد ۲ از گزارش کاربر: «چرخش نور دور بلوک باید مثل نسخه موبایل جهتش عوض شود».
// یک عکسِ ثابت این را ثابت نمی‌کند و «به نظرم درست شد» هم یک اندازه‌گیری نیست.
// جهتِ دیده‌شده از دو چیز می‌آید و هر دو باید سنجیده شوند:
//
//   ۱. جهتِ پیمایشِ مسیر (`roundRectPath`) — آیا ترتیبِ رأس‌ها روی صفحه
//      پادساعتگرد است؟ با مساحتِ علامت‌دار (شوی‌لِیس) سنجیده می‌شود. در دستگاهِ
//      مختصاتِ canvas که y به پایین است، مجموعِ منفی یعنی پادساعتگردِ *دیده‌شده*.
//   ۲. جهتِ حرکتِ باند روی آن مسیر — `lineDashOffset` در canvas dash را به عقب
//      می‌لغزاند، پس اگر کد `-start` نگذارد، نور در جهتِ **مخالفِ** مسیر می‌رود و
//      درست‌کردنِ مسیر تنهایی هیچ چیزی را عوض نمی‌کند. این همان تله‌ای است که
//      یک «اصلاحِ جهت» را بی‌اثر می‌کند.
//
// معیارِ مقایسه: `ConnectionCard.kt` مسیر را با `Path().addRoundRect(...)`
// می‌سازد و `GlowCycle.kt` باند را با `start = ((phase + offset) % 1) * perimeter`
// جلو می‌برد — یعنی نور در جهتِ پیمایشِ مسیر حرکت می‌کند. پس تطبیق با موبایل
// یعنی: مسیرِ پادساعتگرد + حرکتِ هم‌جهت با مسیر.

import { roundRectPath, drawGlowCycle, GLOW_CYCLE_COLORS } from '../src/ui/glowcycle.js'

let failed = 0
const ok = (cond, msg) => {
  console.log(`  ${cond ? 'ok  ' : 'FAIL'} ${msg}`)
  if (!cond) failed++
}

// ---- جاسوسِ Path2D: هر دستور را با نقطهٔ پایانی‌اش ثبت می‌کند --------------
//
// نقطهٔ پایانیِ arcTo دقیقاً محاسبه نمی‌شود: برای علامتِ مساحت، رأسِ گوشه (همان
// نقطهٔ کنترل) کافی است و خطا در هر چهار گوشه یکسان و متقارن است.
const calls = []
class FakePath2D {
  moveTo(x, y) { calls.push(['moveTo', x, y]) }
  lineTo(x, y) { calls.push(['lineTo', x, y]) }
  arcTo(x1, y1, x2, y2, r) { calls.push(['arcTo', x1, y1, x2, y2, r]) }
  closePath() { calls.push(['closePath']) }
}
globalThis.Path2D = FakePath2D

const W = 430
const H = 300
const { perimeter } = roundRectPath(W, H, 26, 0.5)

// رأس‌ها: نقطهٔ هر دستور، به ترتیبِ پیمایش.
const pts = []
for (const c of calls) {
  if (c[0] === 'moveTo' || c[0] === 'lineTo') pts.push([c[1], c[2]])
  else if (c[0] === 'arcTo') pts.push([c[1], c[2]])
}
ok(pts.length >= 8, `مسیر ${pts.length} رأس دارد`)

// مساحتِ علامت‌دار در مختصاتِ y-به-پایین.
let area = 0
for (let i = 0; i < pts.length; i++) {
  const [x1, y1] = pts[i]
  const [x2, y2] = pts[(i + 1) % pts.length]
  area += x1 * y2 - x2 * y1
}
ok(area < 0, `مسیر روی صفحه پادساعتگرد است (مساحتِ علامت‌دار ${area.toFixed(0)} < 0)`)

// و همان سنجه روی یک مستطیلِ ساعتگردِ ساخته‌شده به دست، تا مطمئن شویم علامت را
// برعکس نخوانده‌ایم — یک تستِ جهت که خودش جهت را اشتباه بفهمد، بی‌ارزش است.
const cw = [[0, 0], [10, 0], [10, 10], [0, 10]] // راست → پایین → چپ → بالا = ساعتگرد
let cwArea = 0
for (let i = 0; i < cw.length; i++) {
  const [x1, y1] = cw[i]
  const [x2, y2] = cw[(i + 1) % cw.length]
  cwArea += x1 * y2 - x2 * y1
}
ok(cwArea > 0, `کنترلِ سنجه: مستطیلِ ساعتگرد مساحتِ مثبت می‌دهد (${cwArea})`)

// ---- حرکتِ باند: باید در جهتِ پیمایشِ مسیر جلو برود ----------------------
const stub = {
  lineDashOffset: 0,
  setLineDash() {},
  save() {}, restore() {}, stroke() {},
  set strokeStyle(_v) {}, set lineWidth(_v) {}, set lineCap(_v) {}, set lineJoin(_v) {},
  set globalCompositeOperation(_v) {},
}
const offsetsAt = (phase) => {
  const seen = []
  const probe = new Proxy(stub, {
    set(target, key, value) {
      if (key === 'lineDashOffset') seen.push(value)
      target[key] = value
      return true
    },
  })
  drawGlowCycle(probe, new FakePath2D(), perimeter, { phase, breath: 1, colour: GLOW_CYCLE_COLORS[0] }, 1.5)
  return seen
}

const first = offsetsAt(0.10)
const later = offsetsAt(0.14)
ok(first.length === 7, `هفت باند رسم شد (${first.length})`)
// باندِ اول: offset = -start و start با phase بالا می‌رود، پس offset باید *کم* شود.
ok(later[0] < first[0], `باند با گذشتِ زمان در جهتِ مسیر جلو می‌رود (offset ${first[0].toFixed(1)} → ${later[0].toFixed(1)})`)
// و علامتش منفی است: یک offsetِ مثبت یعنی dash در جهتِ مخالفِ مسیر می‌لغزد.
ok(first.every((v) => v <= 0), 'همهٔ offsetها ≤ 0 هستند — dash هم‌جهتِ مسیر است')

console.log(failed === 0 ? '\nGLOW DIRECTION OK' : `\n${failed} ایراد`)
process.exit(failed === 0 ? 0 : 1)
