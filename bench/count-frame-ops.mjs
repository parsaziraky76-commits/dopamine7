#!/usr/bin/env node
// شمارشِ کارِ هر فریم — نه حدس، نه میلی‌ثانیه.
//
// این اسکریپت خودِ توابعِ ترسیمِ برنامه را با یک context شمارشگر اجرا می‌کند و
// می‌شمارد که یک فریم چند فراخوانِ canvas و چند تخصیصِ تازه (Path2D / گرادیان)
// می‌سازد. عدد میلی‌ثانیه نمی‌دهد چون هیچ موتور رندری این‌جا نیست؛ چیزی که
// می‌دهد قابلِ تکرار است و مستقل از ماشین.
//
// اجرا: node bench/count-frame-ops.mjs <ریشهٔ-پروژه>

const ROOT = process.argv[2]
if (!ROOT) {
  console.error('usage: node count-frame-ops.mjs <project-root>')
  process.exit(2)
}

const counts = () => ({ calls: 0, byName: {}, paths: 0, gradients: 0, strokes: 0, fills: 0 })
let C = counts()

const bump = (name) => {
  C.calls++
  C.byName[name] = (C.byName[name] ?? 0) + 1
}

class FakePath2D {
  constructor() { C.paths++ }
  moveTo() {} lineTo() {} arcTo() {} closePath() {} rect() {} arc() {}
}
class FakeGradient {
  constructor() { C.gradients++ }
  addColorStop() { bump('addColorStop') }
}

function makeCtx() {
  const noop = (name) => (...a) => { bump(name) }
  const ctx = {
    canvas: null,
    save: noop('save'), restore: noop('restore'),
    clearRect: noop('clearRect'),
    setTransform: noop('setTransform'),
    setLineDash: noop('setLineDash'),
    beginPath: noop('beginPath'), closePath: noop('closePath'),
    moveTo: noop('moveTo'), lineTo: noop('lineTo'), arcTo: noop('arcTo'), arc: noop('arc'),
    rect: noop('rect'), clip: noop('clip'),
    translate: noop('translate'), rotate: noop('rotate'), scale: noop('scale'),
    createRadialGradient: (...a) => { bump('createRadialGradient'); return new FakeGradient() },
    createLinearGradient: (...a) => { bump('createLinearGradient'); return new FakeGradient() },
    stroke: (...a) => { bump('stroke'); C.strokes++ },
    fill: (...a) => { bump('fill'); C.fills++ },
    fillRect: (...a) => { bump('fillRect'); C.fills++ },
    measureText: () => ({ width: 10 }),
    createPattern: () => { bump('createPattern'); return { __pattern: true } },
  }
  return ctx
}

// ---- محیطِ مرورگرِ حداقلی ---------------------------------------------------
const W = 430
const H = 470
const MARK = 132

globalThis.Path2D = FakePath2D
// ساعتِ کنترل‌شده: نشانِ A در فریمِ اول عمداً چیزی نمی‌کشد (موجِ ظهور از صفر
// شروع می‌شود)، پس بدون جلوبردنِ ساعت، سنجه صفر می‌خواند.
let CLOCK = 10_000
globalThis.performance = { now: () => CLOCK }

let RAF_CB = null
globalThis.requestAnimationFrame = (cb) => { RAF_CB = cb; return 1 }
globalThis.cancelAnimationFrame = () => { RAF_CB = null }
globalThis.window = { devicePixelRatio: 1.5, matchMedia: () => ({ matches: false, addEventListener() {} }) }
globalThis.document = {
  hidden: false,
  visibilityState: 'visible',
  addEventListener() {},
  removeEventListener() {},
  // ۱.۲.۵-perf: مسیرِ الگوی خطوط مویی به یک canvas کوچک نیاز دارد.
  createElement: () => fakeCanvas(1, 5),
}

function fakeCanvas(w, h) {
  const ctx = makeCtx()
  const cv = {
    width: 0, height: 0,
    clientWidth: w, clientHeight: h,
    getContext: () => ctx,
    getBoundingClientRect: () => { bump('getBoundingClientRect'); return { width: w, height: h, top: 0, left: 0, right: w, bottom: h } },
    style: {},
  }
  ctx.canvas = cv
  return cv
}

// ---- سنجه ۱: نورِ لبهٔ کارت -------------------------------------------------
const glow = await import(`${ROOT}/src/ui/glowcycle.js`)

function measureGlow(frames = 1) {
  const canvas = fakeCanvas(W, H)
  const ctx = canvas.getContext('2d')
  const cycle = new glow.GlowCycle()
  CLOCK += 500
  C = counts()
  for (let i = 0; i < frames; i++) {
    // همان کاری که drawEdgeFrame در هر فریم می‌کند
    CLOCK += 16.7
    glow.fitCanvas(canvas, W, H)
    ctx.clearRect(0, 0, W, H)
    const { path, perimeter } = glow.roundRectPath(W, H, 26, 0.5)
    const grad = ctx.createRadialGradient(W / 2, 0, 0, W / 2, 0, W * 0.95)
    grad.addColorStop(0, 'rgba(0,0,0,0)')
    grad.addColorStop(1, 'rgba(0,0,0,0)')
    ctx.fill(path)
    ctx.stroke(path)
    glow.drawGlowCycle(ctx, path, perimeter, cycle, 1.5)
  }
  return { ...C, frames }
}

// ---- سنجه ۲: نشانِ A --------------------------------------------------------
const mark = await import(`${ROOT}/src/ui/aethermark.js`)

function measureMark(frames = 1) {
  const canvas = fakeCanvas(MARK, MARK)
  C = counts()
  const stop = mark.startAetherMark(canvas)
  // موجِ ظهور (REVEAL_MS = 760ms) تمام شده باشد تا فریمِ «پایدار» سنجیده شود.
  CLOCK += 2_000
  C = counts()
  for (let i = 0; i < frames; i++) {
    CLOCK += 16.7
    const cb = RAF_CB
    RAF_CB = null
    if (!cb) break
    cb(performance.now())
  }
  stop()
  return { ...C, frames }
}

// ---- گزارش ------------------------------------------------------------------
const fmt = (label, r) => {
  const perFrame = (n) => (n / r.frames).toFixed(1)
  console.log(`\n${label}  (${r.frames} فریم)`)
  console.log(`  فراخوان canvas در هر فریم : ${perFrame(r.calls)}`)
  console.log(`  stroke در هر فریم         : ${perFrame(r.strokes)}`)
  console.log(`  fill در هر فریم           : ${perFrame(r.fills)}`)
  console.log(`  Path2D تازه در هر فریم    : ${perFrame(r.paths)}`)
  console.log(`  گرادیان تازه در هر فریم   : ${perFrame(r.gradients)}`)
  const rect = r.byName.getBoundingClientRect ?? 0
  console.log(`  getBoundingClientRect     : ${perFrame(rect)}`)
  const top = Object.entries(r.byName).sort((a, b) => b[1] - a[1]).slice(0, 5)
  console.log(`  پرتکرارترین               : ${top.map(([k, v]) => `${k}×${(v / r.frames).toFixed(0)}`).join('، ')}`)
}

const N = 30
const g = measureGlow(N)
const m = measureMark(N)
fmt('نورِ لبهٔ کارت (glowcycle)', g)
fmt('نشانِ A (aethermark)', m)

const perFrameTotal = (g.calls + m.calls) / N
console.log(`\nجمعِ دو حلقه: ${perFrameTotal.toFixed(1)} فراخوان canvas در هر فریم`)
console.log(`در ۶۰ فریم بر ثانیه: ${Math.round(perFrameTotal * 60).toLocaleString('en-US')} فراخوان در ثانیه`)
console.log(`تخصیصِ تازه در ثانیه: ${Math.round(((g.paths + m.paths + g.gradients + m.gradients) / N) * 60).toLocaleString('en-US')} شیء (Path2D + گرادیان)`)
