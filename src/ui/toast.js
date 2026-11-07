// توستِ مشترک — یک تعریف، برای همهٔ نماها.
//
// پیش از این تنها `diagnostics.js` توست داشت و آن هم درونِ خودش. صفحهٔ دستیار
// بازخوردی نداشت، و «کلید ذخیره شد» از یک کلیکِ بی‌اثر قابل تشخیص نبود. استایل
// `.toast` از قبل در `app.css` وجود داشت؛ فقط جای درستی برای صدا زدنش نبود.

/** مدتِ نمایش. کوتاه‌تر از این خوانده نمی‌شود، بلندتر از این مزاحم است. */
const LIFETIME_MS = 2200
/** برابرِ `transition` در `.toast`؛ حذفِ زودتر، محوشدن را می‌بُرد. */
const FADE_MS = 400

export function toast(message) {
  if (!message) return null
  // یک توست در هر زمان: دو پیام روی هم، هر دو را ناخوانا می‌کند.
  document.querySelector('.toast')?.remove()

  const el = document.createElement('div')
  el.className = 'toast'
  el.setAttribute('role', 'status')
  el.setAttribute('aria-live', 'polite')
  // پیام می‌تواند متنِ خطای موتور باشد (لاتین) در رابطِ فارسی.
  el.dir = 'auto'
  el.textContent = message
  document.body.appendChild(el)

  // `.toast` با `opacity: 0` شروع می‌شود و کلاسِ `is-shown` آن را می‌آورد؛ یک
  // فریم فاصله لازم است وگرنه مرورگر گذارِ ورود را رد می‌کند.
  requestAnimationFrame(() => el.classList.add('is-shown'))
  setTimeout(() => {
    el.classList.remove('is-shown')
    setTimeout(() => el.remove(), FADE_MS)
  }, LIFETIME_MS)
  return el
}
