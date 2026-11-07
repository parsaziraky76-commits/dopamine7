// =============================================================================
//  پورت ۱:۱ از `ui/components/GlowCycle.kt` — نور متحرک لبهٔ کارت اتصال.
// -----------------------------------------------------------------------------
//  چرا canvas و نه CSS:
//
//  کاری که این فایل انجام می‌دهد در CSS شدنی نیست — نه با `border-image` و نه
//  با یک gradient چرخان. نور واقعی روی *محیط یک مستطیل گردگوشه* حرکت می‌کند:
//  هفت باند با طول متغیر، هر باند پنج لایهٔ بلوم با blend جمعی (`lighter`)، و
//  یک قطعه که وقتی از گوشه رد می‌شود باید دور بزند و از ابتدای مسیر ادامه
//  پیدا کند. همان چیزی که Compose با `PathMeasure.getSegment` می‌کرد و اینجا
//  با `setLineDash` + `lineDashOffset` روی یک `Path2D` گردگوشه بازسازی شده
//  است؛ dash یعنی همان «قطعهٔ محیط» و offset یعنی همان `phase`.
//
//  یک اصل دیگر هم عیناً از موبایل می‌آید: هیچ‌چیز وقتی متصل نیستیم اجرا
//  نمی‌شود. سازندهٔ چرخه فقط در حالت CONNECTED ساخته می‌شود و `stop()` حلقهٔ
//  فریم را واقعاً لغو می‌کند — نه اینکه یک انیمیشن نامرئی را زنده نگه دارد.
// =============================================================================

/** پالت: فقط رنگ‌های اصلی. یک رنگ برای هر دور کامل محیط. */
export const GLOW_CYCLE_COLORS = [
  [0xff, 0x1e, 0x1e], // قرمز
  [0x00, 0xe2, 0x3c], // سبز — لومینانس کم‌شده تا بلوم خودش به سفید نزند
  [0x2a, 0x6b, 0xff], // آبی — از #0000FF بالا آمده که روی navy سیاه دیده نشود
  [0xff, 0xd4, 0x00], // زرد
]

/** میلی‌ثانیه برای یک دور کامل محیط (یک رنگ). */
export const GLOW_LAP_MS = 5200

const BREATH_MS = 1700

/**
 * یک باند اکولایزری: جای شروع روی محیط، طول، و هارمونیکی که شدتش را می‌راند.
 * هارمونیک‌ها عدد صحیح‌اند تا وقتی phase از ۱ به ۰ می‌پرد لبه پرش نکند.
 */
const GLOW_BANDS = [
  { offset: 0.00, span: 0.15, harmonic: 2, skew: 0.00, tint: 0.00 },
  { offset: 0.13, span: 0.08, harmonic: 3, skew: 0.34, tint: 0.45 },
  { offset: 0.28, span: 0.13, harmonic: 5, skew: 0.11, tint: 0.20 },
  { offset: 0.43, span: 0.06, harmonic: 7, skew: 0.61, tint: 0.85 },
  { offset: 0.56, span: 0.14, harmonic: 3, skew: 0.79, tint: 0.35 },
  { offset: 0.70, span: 0.09, harmonic: 5, skew: 0.24, tint: 0.65 },
  { offset: 0.85, span: 0.12, harmonic: 2, skew: 0.50, tint: 1.00 },
]

/** بلوم، از بیرون به داخل: ضریب عرض و ضریب آلفا در هر لایه. */
const GLOW_LAYERS = [
  [3.2, 0.045],
  [2.4, 0.085],
  [1.7, 0.16],
  [1.25, 0.34],
  [1.0, 0.82],
]

export const TWO_PI = 6.2831855

function lerpChannel(a, b, k) {
  return Math.round(a + (b - a) * k)
}

/** میان‌یابی خطی به سمت سفید — همان `lerp(base, Color.White, k)`. */
function towardWhite(rgb, k) {
  return [
    lerpChannel(rgb[0], 255, k),
    lerpChannel(rgb[1], 255, k),
    lerpChannel(rgb[2], 255, k),
  ]
}

function rgba(rgb, alpha) {
  const a = Math.min(1, Math.max(0, alpha))
  return `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${a})`
}

/**
 * وضعیت انیمیشن‌شدهٔ نمایش نور: کجای دور فعلی هستیم، این دور با چه رنگی
 * می‌دود، و نفس کشیدن کلی.
 *
 * همان قرارداد Compose: یک ساعت پیوسته `0 → N` که `floor` آن شمارهٔ دور
 * (یعنی رنگ) است و بخش کسری، جای قطعه روی محیط. برگشت به صفر بعد از آخرین
 * دور، همان چیزی است که توالی را بی‌درز می‌کند.
 */
export class GlowCycle {
  constructor(lapMillis = GLOW_LAP_MS) {
    this.lapMillis = lapMillis
    this.startedAt = performance.now()
  }

  /** ۰..۱ داخل دور فعلی. */
  get phase() {
    const laps = (performance.now() - this.startedAt) / this.lapMillis
    return laps - Math.floor(laps)
  }

  /** شدت کلی، تا لبه نفس بکشد و فقط چشمک نزند. */
  get breath() {
    // معادل `RepeatMode.Reverse` با easing نرم: مثلث + هموارسازی smoothstep.
    const t = ((performance.now() - this.startedAt) % (2 * BREATH_MS)) / BREATH_MS
    const tri = t <= 1 ? t : 2 - t
    const eased = tri * tri * (3 - 2 * tri)
    return 0.74 + 0.26 * eased
  }

  /** رنگ دور *فعلی*. */
  get colour() {
    const laps = (performance.now() - this.startedAt) / this.lapMillis
    const step = Math.floor(laps) % GLOW_CYCLE_COLORS.length
    return GLOW_CYCLE_COLORS[(step + GLOW_CYCLE_COLORS.length) % GLOW_CYCLE_COLORS.length]
  }
}

/**
 * مسیر یک مستطیل گردگوشه + محیط تقریبیِ آن.
 *
 * محیط لازم است چون طول dash بر حسب پیکسل است، ولی `span` باندها کسری از
 * محیط‌اند — دقیقاً مثل `PathMeasure.length` در سمت اندروید.
 */
export function roundRectPath(width, height, radius, inset) {
  const left = inset
  const top = inset
  const right = width - inset
  const bottom = height - inset
  const r = Math.max(0, Math.min(radius, (right - left) / 2, (bottom - top) / 2))
  const path = new Path2D()
  // ۱.۲.۴-p4 — مسیر **پادساعتگرد** ساخته می‌شود: بالا-چپ ← پایین ← راست ← بالا.
  //
  // جهتِ چرخشِ نور همان جهتِ پیمایشِ مسیر است، چون `lineDashOffset` منفی
  // (`-start` در `drawGlowCycle`) قطعه را در همان راستا جلو می‌برد. تا اینجا این
  // ترتیب ساعتگرد بود و کاربر گزارش داد نورِ دسکتاپ برعکسِ موبایل می‌چرخد، پس
  // ترتیب برگشت. سمت اندروید مسیر با `Path().addRoundRect(...)` در
  // `ConnectionCard.kt` ساخته می‌شود و باند با همان `start` جلو می‌رود.
  //
  // این ترتیب حدس نیست: `tests/glow-direction.mjs` با مساحتِ علامت‌دار می‌سنجد
  // که پیمایش روی صفحه پادساعتگرد است، و با دو phase می‌سنجد که باند در جهتِ
  // مسیر جلو می‌رود — بدون شرطِ دوم، عوض‌کردنِ مسیر تنها هیچ اثری روی جهتِ
  // دیده‌شده ندارد.
  path.moveTo(left, top + r)
  path.lineTo(left, bottom - r)
  path.arcTo(left, bottom, left + r, bottom, r)
  path.lineTo(right - r, bottom)
  path.arcTo(right, bottom, right, bottom - r, r)
  path.lineTo(right, top + r)
  path.arcTo(right, top, right - r, top, r)
  path.lineTo(left + r, top)
  path.arcTo(left, top, left, top + r, r)
  path.closePath()
  const straight = 2 * (right - left - 2 * r) + 2 * (bottom - top - 2 * r)
  const perimeter = straight + TWO_PI * r
  return { path, perimeter }
}

/**
 * باندهای متحرک را روی [path] می‌کشد، با رنگ همین دور.
 *
 * @param ctx زمینهٔ ۲بعدیِ canvas، از قبل با devicePixelRatio مقیاس‌شده.
 * @param stroke عرض پایهٔ قلم بر حسب پیکسل CSS.
 */
export function drawGlowCycle(ctx, path, perimeter, cycle, stroke) {
  if (perimeter <= 0) return
  const phase = cycle.phase
  const breath = cycle.breath
  const base = cycle.colour
  // تنوع باند‌به‌باند *درون* رنگ همین دور: نسخهٔ روشن‌ترِ همان رنگ، هرگز
  // رنگی دیگر — وگرنه «یک رنگ در هر دور» بی‌معنا می‌شود.
  const highlight = towardWhite(base, 0.30)

  ctx.save()
  ctx.globalCompositeOperation = 'lighter'
  ctx.lineCap = 'round'
  // round و نه miter: اتصال تیز روی شعاع ۲۶ پیکسلیِ گوشه، سوزن می‌سازد.
  ctx.lineJoin = 'round'

  for (const spec of GLOW_BANDS) {
    const raw = 0.5 + 0.5 * Math.sin(TWO_PI * (spec.harmonic * phase + spec.skew))
    // smoothstep: دو سرِ تورّم هر باند نرم می‌شود تا لبه‌اش پله نخورد.
    const amp = raw * raw * (3 - 2 * raw)
    const length = perimeter * spec.span * (0.30 + 0.95 * amp)
    const start = ((phase + spec.offset) % 1) * perimeter
    const mix = Math.min(1, Math.max(0, spec.tint * 0.6 + amp * 0.4))
    const colour = [
      lerpChannel(base[0], highlight[0], mix),
      lerpChannel(base[1], highlight[1], mix),
      lerpChannel(base[2], highlight[2], mix),
    ]
    const width = stroke * (1.05 + 1.75 * amp)
    const alpha = (0.18 + 0.82 * amp) * breath

    // یک قطعه از محیط: dash روشن به طول باند، سپس فاصلهٔ تمام‌محیط.
    // offset منفی است چون dash در canvas به عقب می‌لغزد.
    ctx.setLineDash([length, perimeter])
    ctx.lineDashOffset = -start
    for (const [widthScale, alphaScale] of GLOW_LAYERS) {
      ctx.strokeStyle = rgba(colour, alpha * alphaScale)
      ctx.lineWidth = width * widthScale
      ctx.stroke(path)
    }
  }
  ctx.restore()
}

/**
 * canvas را با `devicePixelRatio` هم‌اندازهٔ جعبهٔ CSS خودش می‌کند.
 *
 * بدون این، لبهٔ مویی روی نمایشگر ۱۵۰٪ ویندوز — که پیش‌فرض بیشتر لپ‌تاپ‌هاست
 * — تار و دوپیکسلی می‌شد؛ همان مشکلی که در موبایل با ۱.۵dp حل شده بود.
 * `true` برمی‌گرداند اگر اندازه واقعاً عوض شده باشد.
 */
export function fitCanvas(canvas, cssWidth, cssHeight) {
  const dpr = Math.max(1, Math.min(3, window.devicePixelRatio || 1))
  const w = Math.max(1, Math.round(cssWidth * dpr))
  const h = Math.max(1, Math.round(cssHeight * dpr))
  if (canvas.width === w && canvas.height === h) return false
  canvas.width = w
  canvas.height = h
  const ctx = canvas.getContext('2d')
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  return true
}

/**
 * سقفِ نرخِ فریم برای هر نمایش — یک‌جا، تا قابلِ تنظیم و قابلِ بازبینی باشد.
 *
 * # چرا سقف، و چرا این عددها
 *
 * هیچ‌یک از این سه نمایش حرکتی سریع‌تر از چند ثانیه‌بر‌دور ندارد: دورِ نورِ لبه
 * ۵۲۰۰ms است، موجِ پینگ ۱۵۰۰ms، و کندترین ساعتِ نشان ۳۴۰۰ms. برای حرکتی با این
 * سرعت، ۶۰ فریم بر ثانیه چیزی به آنچه چشم می‌بیند اضافه نمی‌کند ولی هزینه را
 * دقیقاً به همان نسبت بالا می‌برد — و هزینهٔ این سه، رَستری‌کردنِ ده‌ها قلمِ پهن
 * با blendِ جمعی است، نه اجرای جاوااسکریپت.
 *
 * عددها جدا هستند چون کارِ هر فریم جداست: موجِ پینگ در هر فریم ۲۶ مستطیل
 * گردگوشه می‌کشد و گران‌ترینِ سه‌تاست، پس بیشترین کاهش را می‌گیرد.
 */
export const FRAME_CAPS = {
  /** نورِ متحرکِ لبهٔ کارت — ۳۵ قلمِ جمعی در هر فریم. */
  edge: 45,
  /** متر قدرت پینگ — ۲۶ مستطیل گردگوشه در هر فریم. */
  wave: 30,
  /** نشانِ A — گرادیان‌ها، کلیپ و لبهٔ نئون. */
  mark: 40,
}

// =============================================================================
//  زمان‌بندِ مشترکِ فریم
// -----------------------------------------------------------------------------
//  تا ۱.۲.۵ هر نمایش `requestAnimationFrame` خودش را داشت. سه نمایشِ هم‌زمان
//  یعنی سه زنجیرهٔ مستقلِ rAF که هیچ‌کدام از دیگری خبر ندارد، و — مهم‌تر —
//  هیچ‌کدام نمی‌داند که پنجره مینیمایز شده است: `__onHide` فقط با *تعویض تب*
//  صدا زده می‌شود، پس یک پنجرهٔ مینیمایزِ متصل همان ۶۰ فریم را تا پایان عمر
//  برنامه ادامه می‌داد.
//
//  حالا یک rAF همه را می‌راند، هر مشترک سقفِ نرخِ خودش را دارد، و کلِ زمان‌بند
//  با `visibilitychange` می‌خوابد و بیدار می‌شود. WebView2 مینیمایز و پوشیده‌شدن
//  را از همین راه گزارش می‌کند.
// =============================================================================

/** مشترک‌های فعال: `{ draw, minGap, lastAt }`. */
const subscribers = new Set()
let rafId = 0
let scheduler_paused = false

/** ساعتِ زمان‌بند. آرگومانِ rAF مرجح است؛ هارنس‌های بی‌مرورگر آن را نمی‌دهند. */
function stamp(ts) {
  if (typeof ts === 'number') return ts
  return typeof performance !== 'undefined' ? performance.now() : Date.now()
}

function pump(ts) {
  rafId = 0
  const now = stamp(ts)
  // روی یک کپی پیمایش می‌شود: یک `draw` مجاز است خودش را از حلقه بردارد
  // (متر پینگ وقتی به سکون می‌رسد همین کار را می‌کند).
  for (const sub of [...subscribers]) {
    if (!subscribers.has(sub)) continue
    if (now - sub.lastAt < sub.minGap) continue
    sub.lastAt = now
    sub.draw(now)
  }
  schedule()
}

function schedule() {
  if (rafId || scheduler_paused || subscribers.size === 0) return
  rafId = requestAnimationFrame(pump)
}

/** خوابیدن/بیدارشدنِ کلِ زمان‌بند — پنجرهٔ پنهان هیچ فریمی نمی‌سازد. */
function setPaused(paused) {
  if (scheduler_paused === paused) return
  scheduler_paused = paused
  if (paused) {
    if (rafId) cancelAnimationFrame(rafId)
    rafId = 0
    return
  }
  // بیدار که شد، هیچ مشترکی نباید به‌خاطرِ سقفِ نرخ یک فریم عقب بیفتد.
  for (const sub of subscribers) sub.lastAt = -Infinity
  schedule()
}

// `typeof` و نه دسترسیِ مستقیم: هارنس‌های بی‌مرورگرِ `tests/` هیچ `document`
// نمی‌سازند و این ماژول باید همان‌جا هم بارگذاری شود.
if (typeof document !== 'undefined' && typeof document.addEventListener === 'function') {
  document.addEventListener('visibilitychange', () => {
    setPaused(document.visibilityState === 'hidden' || document.hidden === true)
  })
}

/**
 * حلقهٔ فریم مشترک — با دو قاعده: هر بار که صفحه دیده نمی‌شود متوقف شود، و
 * هرگز تندتر از سقفی که فراخوان اعلام کرده نکشد.
 *
 * ریشهٔ مشکلی که در نسخهٔ دسکتاپ ۱.۲.۳ داشتیم: انیمیشن‌های CSS وقتی ویندوز
 * `prefers-reduced-motion` گزارش می‌کرد (ماشین مجازی، RDP، «افکت‌های
 * انیمیشن» خاموش) سراسری بی‌اثر می‌شدند و کمان دکمه یخ می‌زد.
 * `requestAnimationFrame` روی کامپوزیتور اجرا می‌شود و این اتفاق برایش
 * نمی‌افتد؛ به همین دلیل تمام حرکتِ این کارت از همین‌جا می‌آید.
 *
 * @param draw فراخوانِ ترسیم. ساعتِ فریم را به‌عنوان آرگومان می‌گیرد، تا لازم
 *   نباشد خودش `performance.now()` را دوباره بخواند.
 * @param fps سقفِ نرخِ فریم. `0` یعنی بی‌سقف (همان رفتارِ پیشین).
 * @returns تابعی که این حلقه را واقعاً از زمان‌بند برمی‌دارد.
 */
export function frameLoop(draw, fps = 0) {
  const sub = {
    draw,
    // ۱ms تخفیف: با سقفِ ۳۰ و فاصلهٔ واقعیِ ۳۳.۳ms، مقایسهٔ دقیقِ ≥۳۳.۳ گاهی
    // یک فریم را بی‌دلیل می‌انداخت و نرخ روی ۲۰ می‌نشست.
    minGap: fps > 0 ? 1000 / fps - 1 : 0,
    lastAt: -Infinity,
  }
  subscribers.add(sub)
  schedule()
  return () => {
    subscribers.delete(sub)
    if (subscribers.size === 0 && rafId) {
      cancelAnimationFrame(rafId)
      rafId = 0
    }
  }
}

/**
 * اندازهٔ یک المان را دنبال می‌کند، بی‌آنکه در هر فریم چیدمان را اجبار کند.
 *
 * # چرا این لازم شد
 *
 * هر سه حلقهٔ ترسیم در **هر فریم** `getBoundingClientRect()` می‌گرفتند. این
 * فراخوان مرورگر را وادار می‌کند چیدمان را همان‌جا و همان لحظه حساب کند، و
 * چیزی که با آن سنجیده می‌شد — عرض و ارتفاعِ کارت — بین دو فریم عوض نمی‌شود
 * مگر پنجره تغییرِ اندازه بدهد.
 *
 * `ResizeObserver` همان خبر را می‌دهد بی‌آنکه کسی چیدمان را اجبار کند. جایی که
 * نیست (هارنس‌های jsdom در `tests/`)، به خواندنِ کم‌تکرار برمی‌گردیم: نیم‌ثانیه
 * یک‌بار، نه شصت‌بار در ثانیه.
 */
export function trackSize(el, onResize) {
  let width = 0
  let height = 0
  let polledAt = -Infinity

  const read = () => {
    const box = el.getBoundingClientRect()
    const w = box.width
    const h = box.height
    if (w === width && h === height) return
    width = w
    height = h
    onResize?.(w, h)
  }

  let observer = null
  if (typeof ResizeObserver === 'function') {
    // `ResizeObserver` بی‌درنگ یک بار برای اندازهٔ فعلی هم آتش می‌کند.
    observer = new ResizeObserver(read)
    observer.observe(el)
  } else {
    read()
  }

  return {
    get width() { return width },
    get height() { return height },
    /** فقط در نبودِ `ResizeObserver` کاری می‌کند. */
    poll(now) {
      if (observer) return
      if (now - polledAt < 500) return
      polledAt = now
      read()
    },
    read,
    stop() { observer?.disconnect() },
  }
}
