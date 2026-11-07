// هر دارایی تصویری به یک ماژول با export پیش‌فرضِ رشته‌ای تبدیل می‌شود.
export async function load(url, context, nextLoad) {
  if (/\.(png|jpe?g|svg|webp)$/.test(new URL(url).pathname)) {
    return { format: 'module', shortCircuit: true, source: `export default "${url}"` }
  }
  return nextLoad(url, context)
}
