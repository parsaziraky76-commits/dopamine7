// هارنس بی‌مرورگر: فقط برای اینکه ثابت شود منطق ترسیم بدون خطا اجرا می‌شود
// و رشته‌های رنگ معتبرند. یک canvas قلابی که هر فراخوانی را ثبت می‌کند.
const calls = []
const badColours = []
const COLOUR_RE = /^(rgba\(\d+,\d+,\d+,[\d.]+\)|#[0-9A-Fa-f]{6}|rgba\(255,255,255,[\d.]+\))$/

class FakeGradient {
  addColorStop(_o, c) { if (typeof c === 'string' && !COLOUR_RE.test(c)) badColours.push(c) }
}

class FakeCtx {
  constructor() {
    this._fill = ''
    this._stroke = ''
  }
  set fillStyle(v) { if (typeof v === 'string' && !COLOUR_RE.test(v)) badColours.push(v); this._fill = v }
  get fillStyle() { return this._fill }
  set strokeStyle(v) { if (typeof v === 'string' && !COLOUR_RE.test(v)) badColours.push(v); this._stroke = v }
  get strokeStyle() { return this._stroke }
  setTransform() {} save() {} restore() {} translate() {} rotate() {} clip() {}
  beginPath() {} closePath() {} moveTo() {} lineTo() {} arcTo() {} arc() {} rect() {}
  fill() { calls.push('fill') } stroke() { calls.push('stroke') }
  fillRect() { calls.push('fillRect') } clearRect() {} setLineDash() {}
  createRadialGradient() { return new FakeGradient() }
  createLinearGradient() { return new FakeGradient() }
}

class FakeCanvas {
  constructor(w, h) { this.width = w; this.height = h; this.clientWidth = w; this.clientHeight = h; this._ctx = new FakeCtx() }
  getContext() { return this._ctx }
  getBoundingClientRect() { return { width: this.clientWidth, height: this.clientHeight } }
}

globalThis.window = { devicePixelRatio: 1.5 }
globalThis.performance = { now: () => nowValue }
globalThis.Path2D = class { moveTo() {} lineTo() {} arcTo() {} closePath() {} }
let nowValue = 0
let rafQueue = []
globalThis.requestAnimationFrame = (fn) => { rafQueue.push(fn); return rafQueue.length }
globalThis.cancelAnimationFrame = () => {}

const { GlowCycle, drawGlowCycle, roundRectPath, fitCanvas } = await import('../src/ui/glowcycle.js')

// ---- لبهٔ کارت، در چند لحظهٔ مختلف از چرخه ----
const edge = new FakeCanvas(470, 520)
const ctx = edge.getContext('2d')
const cycle = new GlowCycle()
const { path, perimeter } = roundRectPath(470, 520, 26, 0.5)
console.log('perimeter =', perimeter.toFixed(1))
for (const t of [0, 900, 5199, 5201, 12000, 20800, 20801]) {
  nowValue = t
  drawGlowCycle(ctx, path, perimeter, cycle, 1.5)
}
const strokesPerFrame = 7 * 5 // ۷ باند × ۵ لایهٔ بلوم
console.log('strokes drawn =', calls.filter((c) => c === 'stroke').length, 'expected', strokesPerFrame * 7)

// رنگِ هر دور باید در هر ۵۲۰۰ms یکی جلو برود و بعد از چهارمی برگردد.
const laps = [0, 5201, 10401, 15601, 20801].map((t) => { nowValue = t; return cycle.colour.join(',') })
console.log('lap colours =', laps)

// ---- نشان D دوپامین ----
const markHost = {
  innerHTML: '',
  classList: { add() {}, remove() {} },
}
const { startDopamineMark } = await import('../src/ui/dopaminemark.js')
const stop = startDopamineMark(markHost)
if (!markHost.innerHTML.includes('Dopamine') && !markHost.innerHTML.includes('dp-mark__reveal')) {
  throw new Error('Dopamine connected mark did not render')
}
stop()
console.log('Dopamine mark rendered and cleaned up')

// ---- fitCanvas و devicePixelRatio ----
const c = new FakeCanvas(0, 0)
fitCanvas(c, 470, 22)
console.log('fitCanvas 470x22 @1.5 ->', c.width, 'x', c.height, '(expect 705 x 33)')

console.log(badColours.length === 0 ? 'OK: هیچ رشتهٔ رنگ نامعتبری تولید نشد' : `FAIL colours: ${badColours.slice(0, 5)}`)
