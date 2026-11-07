#!/usr/bin/env node
// شمارشِ کارِ هر فریم برای **هر سه** حلقه، از طریق خودِ کارتِ اتصال.
//
// jsdom استفاده می‌شود تا `createConnectionCard()` واقعی ساخته شود؛ `getContext`
// با یک context شمارشگر جایگزین می‌شود و `requestAnimationFrame` دستی رانده
// می‌شود، پس عددها از خودِ کدِ محصول می‌آیند و نه از یک بازسازیِ تقریبی.
//
// اجرا: node bench/count-card-ops.mjs <ریشهٔ-پروژه> [حالت]
//   حالت: connected (پیش‌فرض) | disconnected

import { JSDOM } from 'jsdom'

const ROOT = process.argv[2]
const MODE = process.argv[3] ?? 'connected'
if (!ROOT) {
  console.error('usage: node count-card-ops.mjs <project-root> [connected|disconnected]')
  process.exit(2)
}

const counts = () => ({ calls: 0, byName: {}, paths: 0, gradients: 0, strokes: 0, fills: 0, rects: 0 })
let C = counts()
let COUNTING = false

const bump = (name) => {
  if (!COUNTING) return
  C.calls++
  C.byName[name] = (C.byName[name] ?? 0) + 1
}

const dom = new JSDOM('<!doctype html><html><body><div id="view"></div></body></html>', {
  url: 'http://localhost/',
  pretendToBeVisual: true,
})

class CountingPath2D {
  constructor() { if (COUNTING) C.paths++ }
  moveTo() {} lineTo() {} arcTo() {} closePath() {} rect() {} arc() {}
}
class CountingGradient {
  constructor() { if (COUNTING) C.gradients++ }
  addColorStop() { bump('addColorStop') }
}

function countingCtx() {
  const noop = (name) => () => { bump(name) }
  return {
    save: noop('save'), restore: noop('restore'),
    clearRect: noop('clearRect'), setTransform: noop('setTransform'),
    setLineDash: noop('setLineDash'),
    beginPath: noop('beginPath'), closePath: noop('closePath'),
    moveTo: noop('moveTo'), lineTo: noop('lineTo'), arcTo: noop('arcTo'), arc: noop('arc'),
    rect: noop('rect'), clip: noop('clip'),
    translate: noop('translate'), rotate: noop('rotate'), scale: noop('scale'),
    createRadialGradient: () => { bump('createRadialGradient'); return new CountingGradient() },
    createLinearGradient: () => { bump('createLinearGradient'); return new CountingGradient() },
    stroke: () => { bump('stroke'); if (COUNTING) C.strokes++ },
    fill: () => { bump('fill'); if (COUNTING) C.fills++ },
    fillRect: () => { bump('fillRect'); if (COUNTING) { C.fills++; C.rects++ } },
  }
}

const CTX = countingCtx()
dom.window.HTMLCanvasElement.prototype.getContext = function () { return CTX }
dom.window.Path2D = CountingPath2D

// اندازهٔ واقعیِ کارت و متر پینگ — jsdom چیدمان نمی‌سنجد، پس دستی داده می‌شود.
const SIZES = { ccard: [430, 470], 'ccard__ping-wave': [398, 34], 'connect__mark': [132, 132] }
dom.window.Element.prototype.getBoundingClientRect = function () {
  bump('getBoundingClientRect')
  for (const [cls, [w, h]] of Object.entries(SIZES)) {
    if (this.classList.contains(cls)) return { width: w, height: h, top: 0, left: 0, right: w, bottom: h, x: 0, y: 0 }
  }
  return { width: 430, height: 40, top: 0, left: 0, right: 430, bottom: 40, x: 0, y: 0 }
}

let CLOCK = 10_000
const FRAME_CBS = []
globalThis.window = dom.window
globalThis.document = dom.window.document
globalThis.HTMLElement = dom.window.HTMLElement
globalThis.localStorage = dom.window.localStorage
globalThis.Path2D = CountingPath2D
globalThis.performance = { now: () => CLOCK }
globalThis.requestAnimationFrame = (cb) => { FRAME_CBS.push(cb); return FRAME_CBS.length }
globalThis.cancelAnimationFrame = () => {}
dom.window.performance.now = () => CLOCK
dom.window.requestAnimationFrame = globalThis.requestAnimationFrame
dom.window.cancelAnimationFrame = globalThis.cancelAnimationFrame
dom.window.devicePixelRatio = 1.5

const { createConnectionCard } = await import(`${ROOT}/src/views/connectioncard.js`)

const SNAP_CONNECTED = {
  state: 'CONNECTED', detail: '', error: null,
  endpoint: '188.114.99.205:3581', protocol: 'WIREGUARD \u2192 PSIPHON',
  latencyMs: 199, uptimeSecs: 42, rxBytes: 980_000, txBytes: 828_000,
  shareSocks: null, shareHttp: null, ipLoading: false,
  ipInfo: { ip: '139.162.179.163', countryCode: 'DE', viaTunnel: true },
  webrtcLeak: false, leakGuard: true, torPercent: null,
}
const SNAP_OFF = { ...SNAP_CONNECTED, state: 'DISCONNECTED', latencyMs: null, uptimeSecs: 0, rxBytes: 0, txBytes: 0, ipInfo: null }

const card = createConnectionCard()
document.getElementById('view').appendChild(card.node)
card.paint(MODE === 'connected' ? SNAP_CONNECTED : SNAP_OFF, {
  busy: false, title: 'Connected', caption: 'Tap to disconnect',
})
if (card.start) card.start()

// موجِ ظهور و چند فریمِ گرم‌شدن، بیرون از شمارش.
CLOCK += 2_000
for (let i = 0; i < 3; i++) {
  const batch = FRAME_CBS.splice(0)
  CLOCK += 16.7
  for (const cb of batch) cb(CLOCK)
}

const FRAMES = 30
COUNTING = true
C = counts()
let driven = 0
for (let i = 0; i < FRAMES; i++) {
  const batch = FRAME_CBS.splice(0)
  if (batch.length === 0) break
  driven = batch.length
  CLOCK += 16.7
  for (const cb of batch) cb(CLOCK)
}
COUNTING = false
card.stop?.()

const per = (n) => (n / FRAMES).toFixed(1)
console.log(`\n=== کارتِ اتصال — حالت ${MODE} — ${FRAMES} فریم، ${driven} حلقهٔ فعال ===`)
console.log(`  فراخوان canvas در هر فریم : ${per(C.calls)}`)
console.log(`  stroke در هر فریم         : ${per(C.strokes)}`)
console.log(`  fill / fillRect           : ${per(C.fills)}`)
console.log(`  Path2D تازه در هر فریم    : ${per(C.paths)}`)
console.log(`  گرادیان تازه در هر فریم   : ${per(C.gradients)}`)
console.log(`  getBoundingClientRect     : ${per(C.byName.getBoundingClientRect ?? 0)}`)
const top = Object.entries(C.byName).sort((a, b) => b[1] - a[1]).slice(0, 6)
console.log(`  پرتکرارترین               : ${top.map(([k, v]) => `${k}\u00d7${(v / FRAMES).toFixed(0)}`).join('\u060c ')}`)
console.log(`\n  \u2192 \u062f\u0631 \u06f6\u06f0fps: ${Math.round((C.calls / FRAMES) * 60).toLocaleString('en-US')} \u0641\u0631\u0627\u062e\u0648\u0627\u0646 canvas \u062f\u0631 \u062b\u0627\u0646\u06cc\u0647`)
console.log(`  \u2192 \u062a\u062e\u0635\u06cc\u0635\u0650 \u062a\u0627\u0632\u0647: ${Math.round(((C.paths + C.gradients) / FRAMES) * 60).toLocaleString('en-US')} \u0634\u06cc\u0621 \u062f\u0631 \u062b\u0627\u0646\u06cc\u0647`)
