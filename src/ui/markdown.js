// ۱.۴.۰-r4 — پورت از `ai/AiMarkdown.kt` + `ui/ai/AiRichText.kt` موبایل.
//
// پرامپتِ چت حالا (FORMAT در `ai_prompts.rs`) از مدل می‌خواهد مثل اپ Gemini
// Markdown بنویسد: «### » عنوان، **پررنگ**، `کد`، «- » و «1. » فهرست. بدون
// رندرکننده، کاربر ستاره‌ها و # های خام را می‌دید.
//
// # امنیت: هیچ innerHTML
//
// متن از یک API شبکه می‌آید و می‌تواند هر چیزی باشد. این رندرکننده **فقط**
// `createElement` و `textContent` به کار می‌برد؛ هیچ رشته‌ای هرگز به‌عنوان HTML
// تفسیر نمی‌شود، پس `<img onerror>` در پاسخ مدل فقط متن است. لینک هم ساخته
// نمی‌شود: یک لینکِ قابل‌کلیک در پاسخِ یک مدل، در برنامه‌ای که مسیرِ شبکه را
// می‌سازد، چیزی نیست که بخواهیم.
//
// # جهت
//
// هر بلوک `dir="auto"` می‌گیرد تا یک پاراگراف فارسی راست‌به‌چپ و یک فهرستِ
// انگلیسی چپ‌به‌راست، هر کدام درست بنشینند (همان «جهت بر اساسِ اکثریتِ حروف در
// هر بلوک» موبایل).

const INLINE = /(\*\*[^*\n]+\*\*|`[^`\n]+`|\*[^*\n]+\*|_[^_\n]+_)/g

/** متنِ یک خط را با پررنگ/کد/ایتالیک در `parent` می‌نشاند. */
function inline(parent, text) {
  let last = 0
  for (const match of text.matchAll(INLINE)) {
    if (match.index > last) parent.appendChild(document.createTextNode(text.slice(last, match.index)))
    const token = match[0]
    let node
    if (token.startsWith('**')) {
      node = document.createElement('strong')
      node.textContent = token.slice(2, -2)
    } else if (token.startsWith('`')) {
      node = document.createElement('code')
      node.className = 'md__code'
      // مقدارهایی مثل 1280 یا X25519:P-256 همیشه چپ‌به‌راست‌اند.
      node.dir = 'ltr'
      node.textContent = token.slice(1, -1)
    } else {
      node = document.createElement('em')
      node.textContent = token.slice(1, -1)
    }
    parent.appendChild(node)
    last = match.index + token.length
  }
  if (last < text.length) parent.appendChild(document.createTextNode(text.slice(last)))
}

/**
 * Markdownِ پاسخ را در `container` رندر می‌کند. محتوای قبلیِ container پاک می‌شود.
 *
 * @param {HTMLElement} container
 * @param {string} text
 */
export function renderMarkdown(container, text) {
  container.textContent = ''
  const lines = String(text ?? '').replace(/\r\n?/g, '\n').split('\n')
  let list = null
  let listKind = ''
  let para = null
  let fence = null

  const closeList = () => { list = null; listKind = '' }
  const closePara = () => { para = null }

  for (const raw of lines) {
    // بلوکِ کد: خط‌به‌خط و بی‌تفسیر.
    if (/^\s*```/.test(raw)) {
      if (fence) { fence = null; continue }
      closeList(); closePara()
      fence = document.createElement('pre')
      fence.className = 'md__pre'
      fence.dir = 'ltr'
      container.appendChild(fence)
      continue
    }
    if (fence) {
      fence.textContent += (fence.textContent ? '\n' : '') + raw
      continue
    }

    const line = raw.trimEnd()
    if (!line.trim()) { closeList(); closePara(); continue }

    const heading = /^\s{0,3}(#{1,6})\s+(.*)$/.exec(line)
    if (heading) {
      closeList(); closePara()
      const h = document.createElement('div')
      h.className = 'md__h'
      h.dir = 'auto'
      inline(h, heading[2].replace(/\s+#+\s*$/, ''))
      container.appendChild(h)
      continue
    }

    const bullet = /^(\s*)[-*•]\s+(.*)$/.exec(line)
    const numbered = /^(\s*)(\d{1,3})[.)]\s+(.*)$/.exec(line)
    if (bullet || numbered) {
      closePara()
      const kind = bullet ? 'ul' : 'ol'
      const nested = (bullet ? bullet[1] : numbered[1]).length >= 2 && list
      if (!list || (listKind !== kind && !nested)) {
        list = document.createElement(kind)
        list.className = 'md__list'
        list.dir = 'auto'
        if (numbered && numbered[2] !== '1') list.start = Number(numbered[2])
        container.appendChild(list)
        listKind = kind
      }
      const li = document.createElement('li')
      if (nested) li.className = 'md__li--nested'
      inline(li, bullet ? bullet[2] : numbered[3])
      list.appendChild(li)
      continue
    }

    closeList()
    if (!para) {
      para = document.createElement('p')
      para.className = 'md__p'
      para.dir = 'auto'
      container.appendChild(para)
    } else {
      para.appendChild(document.createElement('br'))
    }
    inline(para, line)
  }
}

/**
 * همان متن بدون نشانه‌های Markdown — برای «کپی پاسخ» (موبایل: «Copy gives clean text»).
 *
 * @param {string} text
 * @returns {string}
 */
export function plainText(text) {
  return String(text ?? '')
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .filter((l) => !/^\s*```/.test(l))
    .map((l) => l
      .replace(/^\s{0,3}#{1,6}\s+/, '')
      .replace(/^(\s*)[*•]\s+/, '$1- ')
      .replace(/\*\*([^*\n]+)\*\*/g, '$1')
      .replace(/`([^`\n]+)`/g, '$1')
      .replace(/(^|[^*])\*([^*\n]+)\*/g, '$1$2'))
    .join('\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}
