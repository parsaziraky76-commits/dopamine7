// tests/chat-smoke.mjs — موارد ۳ و ۴ گزارش کاربر، روی *رفتار* و نه ظاهر.
//
// چیزی که اینجا ثابت می‌شود:
//   ۳. چت یک صفحهٔ مستقل است (نه بخشی از تنظیماتِ دستیار)، و همان امکاناتِ
//      موبایل را دارد: پرسش‌های آمادهٔ صفحهٔ خالی، کپی، ویرایش، حذف، حذفِ
//      گروهی، و «تلاش مجدد» روی پیامی که فرستاده نشده.
//   ۴. پایکِ «متوجه نشدید؟ از دستیار بپرسید» زیر پاسخِ حبابِ ✨ می‌آید و پرسش را
//      به همان صفحهٔ چت منتقل می‌کند.
//   ۵. هیچ حذفی بی‌تأیید انجام نمی‌شود — سطلِ یک حباب، حذفِ گروهی و پاک‌کردنِ کلِ
//      گفت‌وگو، هر سه از یک دیالوگ رد می‌شوند و «انصراف» واقعاً هیچ نمی‌فرستد.
//   ۶. پیشنهادِ تنظیمات یک کارتِ `قدیم → جدید` می‌شود، «اعمال» فرمانِ
//      `ai_apply_changes` را با شناسهٔ همان حباب می‌فرستد، و بعدش پنجرهٔ «قطع و
//      وصل کنید» می‌آید. حبابی که علامتِ اعمال دارد دیگر دکمه ندارد.
//
// معیارِ درستی: **فرمانی که به Rust می‌رود**. یک دکمه که وجود دارد ولی
// `ai_retry` را با شناسهٔ درست صدا نمی‌زند، از نبودنش بدتر است.

import { JSDOM } from 'jsdom'

const dom = new JSDOM('<!doctype html><html><body><div id="view"></div></body></html>', {
  url: 'http://localhost/',
  pretendToBeVisual: true,
})
globalThis.window = dom.window
globalThis.document = dom.window.document
globalThis.HTMLElement = dom.window.HTMLElement
globalThis.localStorage = dom.window.localStorage
globalThis.requestAnimationFrame = (fn) => setTimeout(() => fn(0), 0)
globalThis.cancelAnimationFrame = clearTimeout
// `navigator` در Node یک getter است و بازنویسی نمی‌شود؛ همان navigatorِ jsdom با
// یک کلیپ‌بوردِ ساختگی به globalThis وصل می‌شود تا «کپی» قابلِ سنجش باشد.
const clipboard = []
Object.defineProperty(dom.window.navigator, 'clipboard', {
  value: { writeText: (text) => { clipboard.push(text); return Promise.resolve() } },
  configurable: true,
})
Object.defineProperty(globalThis, 'navigator', {
  value: dom.window.navigator,
  configurable: true,
})

const invoked = []
window.__TAURI_INTERNALS__ = {
  invoke: (cmd, args) => {
    invoked.push([cmd, args])
    if (cmd === 'plugin:event|listen') return Promise.resolve(1)
    if (cmd === 'ai_snapshot') return Promise.resolve(AI)
    if (cmd === 'ai_explain') return Promise.resolve('MTU is the largest packet the tunnel will send without fragmenting it.')
    return Promise.resolve(null)
  },
  transformCallback: (cb) => cb,
  metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
}

const AI = {
  gate: 'READY', gateCode: 'READY', hasKey: true, keyHint: '4271',
  models: [{ id: 'gemini-2.5-flash', displayName: 'Gemini 2.5 Flash', description: '', inputTokenLimit: 0, outputTokenLimit: 0, chatCapable: true }],
  selectedModel: 'gemini-2.5-flash', modelNumber: 1,
  busy: false, error: null, errorKind: null, messages: [], advisor: null,
  probe: { state: 'OK', modelCount: 1, via: 'Dopamine → Psiphon', message: null },
}

const { applyAiSnapshot, aiHintButton } = await import('../src/ai.js')
const { renderChat } = await import('../src/views/chat.js')
const { setTabRouter, resetNav } = await import('../src/ui/nav.js')

let failed = 0
const ok = (cond, msg) => {
  console.log(`  ${cond ? 'ok  ' : 'FAIL'} ${msg}`)
  if (!cond) failed++
}
const last = (cmd) => [...invoked].reverse().find((c) => c[0] === cmd)
const tick = () => new Promise((r) => setTimeout(r, 0))
/** دیالوگِ بازِ روی صفحه، یا null. */
const dialog = () => view.querySelector('.chat__modal')
/** دکمهٔ تأیید/انصرافِ دیالوگِ باز. */
const dialogButton = (kind) => {
  const box = dialog()
  if (!box) return null
  const buttons = [...box.querySelectorAll('button')]
  return kind === 'cancel'
    ? buttons.find((b) => b.classList.contains('btn--ghost'))
    : buttons.find((b) => b.classList.contains('btn--danger') || b.classList.contains('btn--primary'))
}

// ---- صفحهٔ خالی: پرسش‌های آماده ------------------------------------------
console.log('\n── صفحهٔ خالی')
const view = renderChat()
document.getElementById('view').appendChild(view)
applyAiSnapshot(AI)

const chips = [...view.querySelectorAll('.chat__chip')]
ok(chips.length === 4, `چهار پرسشِ آماده روی صفحهٔ خالی (${chips.length})`)
ok(view.querySelector('.chat__composer') !== null, 'جعبهٔ نوشتن وجود دارد')
chips[0].click()
await tick()
const sent = last('ai_send_chat')
ok(sent != null && sent[1].text === chips[0].textContent, `کلیکِ پرسشِ آماده همان متن را می‌فرستد («${sent?.[1]?.text?.slice(0, 32)}…»)`)

// ---- گفت‌وگوی پُر: کپی، ویرایش، حذف --------------------------------------
console.log('\n── حباب‌ها')
const MESSAGES = [
  { id: 11, fromUser: true, text: 'Why is my connection slow?', failed: false, edited: false },
  { id: 12, fromUser: false, text: 'Because the exit is chained through Psiphon.', failed: false, edited: false },
  { id: 13, fromUser: true, text: 'Make the tunnel harder to detect', failed: false, edited: false },
  { id: 14, fromUser: false, text: 'API key not valid. Pass a valid API key.', failed: true, errorKind: 'BAD_KEY', sourcePrompt: 'Make the tunnel harder to detect', edited: false },
]
applyAiSnapshot({ ...AI, messages: MESSAGES })

const rows = [...view.querySelectorAll('.chat__row')]
ok(rows.length === 4, `چهار حباب رندر شد (${rows.length})`)
ok(rows[0].classList.contains('is-user'), 'حبابِ کاربر نشانِ is-user دارد')
ok(rows[3].classList.contains('is-failed'), 'حبابِ شکست نشانِ is-failed دارد')

// جملهٔ شکست باید ترجمه‌شده باشد و متنِ خامِ گوگل هم به‌عنوان جزئیات بماند.
const failHead = rows[3].querySelector('.chat__failhead')?.textContent ?? ''
const failDetail = rows[3].querySelector('.chat__faildetail')?.textContent ?? ''
ok(failHead.includes('Google rejected this key'), `شکست با جملهٔ خوانا گفته می‌شود («${failHead.slice(0, 40)}…»)`)
ok(failDetail.includes('API key not valid'), 'متنِ خامِ گوگل به‌عنوان جزئیات می‌ماند')

// کپی: فقط روی پاسخِ مدل.
const copyBtn = rows[1].querySelector('.chat__act')
copyBtn.click()
await tick()
ok(clipboard[0] === MESSAGES[1].text, 'کپیِ پاسخ متنِ همان حباب را در کلیپ‌بورد می‌گذارد')

// ویرایش: دیالوگ باز می‌شود و `ai_edit_message` با شناسهٔ همان پیام می‌رود.
rows[2].querySelector('.chat__act').click()
const editor = view.querySelector('.chat__modal')
ok(editor !== null, 'دیالوگِ ویرایش باز می‌شود')
const area = editor.querySelector('textarea')
ok(area.value === MESSAGES[2].text, 'ویرایشگر با متنِ فعلیِ پیام پر می‌شود')
area.value = 'Make it much harder to detect'
;[...editor.querySelectorAll('button')].find((b) => b.classList.contains('btn--primary')).click()
await tick()
const edited = last('ai_edit_message')
ok(edited?.[1]?.id === 13 && edited?.[1]?.text === 'Make it much harder to detect', `ویرایش با شناسهٔ درست می‌رود (id=${edited?.[1]?.id})`)
ok(view.querySelector('.chat__modal') === null, 'دیالوگ پس از فرستادن بسته می‌شود')

// تلاش مجدد: فقط روی حبابِ شکست، با شناسهٔ همان حباب.
const retryBtn = rows[3].querySelector('.chat__act--accent')
ok(retryBtn !== null, 'حبابِ شکست دکمهٔ تلاش مجدد دارد')
retryBtn.click()
await tick()
ok(last('ai_retry')?.[1]?.id === 14, `تلاش مجدد با شناسهٔ حبابِ شکست می‌رود (id=${last('ai_retry')?.[1]?.id})`)

// ---- مورد ۲: حذف بی‌تأیید نیست --------------------------------------------
//
// این‌جا جایی است که اشکال زندگی می‌کرد: تأیید فقط روی حذفِ گروهی بود و آیکونِ
// سطلِ روی هر حباب یک کلیکِ بی‌بازگشت. پس اول ثابت می‌شود که «انصراف» هیچ چیزی
// نمی‌فرستد، و تنها بعدش تأیید سنجیده می‌شود.
console.log('\n── تأییدِ حذف')
invoked.length = 0
const trash = [...rows[0].querySelectorAll('.chat__act')].at(-1)
trash.click()
await tick()
ok(dialog() !== null, 'سطلِ روی حباب اول دیالوگِ تأیید باز می‌کند')
ok(dialog().textContent.includes('1'), `تعداد در خودِ پرسش است («${dialog().querySelector('.chat__modaltitle')?.textContent}»)`)
ok(last('ai_delete_messages') == null, 'و تا تأیید نشود هیچ فرمانی نمی‌رود')
dialogButton('cancel').click()
await tick()
ok(dialog() === null, 'انصراف دیالوگ را می‌بندد')
ok(last('ai_delete_messages') == null, 'و انصراف هیچ چیزی حذف نمی‌کند')

trash.click()
await tick()
dialogButton('confirm').click()
await tick()
ok(JSON.stringify(last('ai_delete_messages')?.[1]?.ids) === '[11]', `تأیید همان یک پیام را حذف می‌کند (${JSON.stringify(last('ai_delete_messages')?.[1]?.ids)})`)
ok(dialog() === null, 'دیالوگ پس از تأیید بسته می‌شود')

// و پاک‌کردنِ کلِ گفت‌وگو هم همان مسیر را دارد.
invoked.length = 0
const wipeBtn = [...view.querySelectorAll('.chat__headacts button')][0]
wipeBtn.click()
await tick()
ok(dialog() !== null, 'پاک‌کردنِ گفت‌وگو هم دیالوگ می‌خواهد')
dialogButton('cancel').click()
await tick()
ok(last('ai_clear_chat') == null, 'انصراف گفت‌وگو را پاک نمی‌کند')
wipeBtn.click()
await tick()
dialogButton('confirm').click()
await tick()
ok(last('ai_clear_chat') != null, 'تأیید فرمانِ ai_clear_chat را می‌فرستد')

// ---- حالتِ انتخاب و حذفِ گروهی -------------------------------------------
console.log('\n── انتخابِ گروهی')
applyAiSnapshot({ ...AI, messages: MESSAGES })
const fresh = [...view.querySelectorAll('.chat__row')]
fresh[0].dispatchEvent(new dom.window.MouseEvent('contextmenu', { bubbles: true, cancelable: true }))
ok(!view.querySelector('.chat__pickbar').hidden, 'راست‌کلیک حالتِ انتخاب را باز می‌کند')
// کلیکِ ساده در حالتِ انتخاب، تیک می‌زند (و بیرونِ آن حالت هیچ کاری نمی‌کند).
;[...view.querySelectorAll('.chat__row')][1].click()
ok(view.querySelector('.chat__pickcount').textContent.includes('2'), `شمارندهٔ انتخاب درست است («${view.querySelector('.chat__pickcount').textContent}»)`)
// `window.confirm` عمداً به یک تله بسته می‌شود: اگر روزی کسی دیالوگِ درون‌برنامه‌ای
// را با آن عوض کند، این تست می‌شکند و نه اینکه بی‌صدا رد شود. پنجرهٔ سیستمیِ
// WebView2 در رابط فارسی «OK / Cancel» انگلیسی نشان می‌داد.
invoked.length = 0
let nativeConfirmUsed = false
window.confirm = () => { nativeConfirmUsed = true; return true }
;[...view.querySelectorAll('.chat__pickbar button')].find((b) => b.classList.contains('btn--danger')).click()
await tick()
ok(dialog() !== null, 'حذفِ گروهی هم از دیالوگِ خودِ برنامه رد می‌شود')
ok(!nativeConfirmUsed, 'و از window.confirmِ سیستمی استفاده نمی‌شود')
dialogButton('confirm').click()
await tick()
const bulk = last('ai_delete_messages')?.[1]?.ids ?? []
ok(bulk.length === 2 && bulk.includes(11) && bulk.includes(12), `حذفِ گروهی در یک فرمان می‌رود (${JSON.stringify(bulk)})`)

// ---- مورد ۱: کارتِ تنظیماتِ پیشنهادی و دکمهٔ اعمال ------------------------
//
// مقادیر از سمت Rust می‌آیند و از قبل اعتبارسنجی شده‌اند (`vet_changes`)، پس اینجا
// فقط این سنجیده می‌شود که کارت همان را نشان دهد و دکمه فرمانِ درست را بفرستد.
console.log('\n── تنظیماتِ پیشنهادی')
const PROPOSED = [
  { id: 21, fromUser: true, text: 'Make the tunnel harder to detect', failed: false, edited: false, changes: [], applied: false },
  {
    id: 22, fromUser: false, failed: false, edited: false, applied: false,
    text: 'Lowering the MTU and turning fragmentation on should help.',
    changes: [
      { key: 'mtu', before: '1280', value: '1380', why: 'handshakes are timing out' },
      { key: 'fragment', before: 'off', value: 'on', why: 'splits the TLS ClientHello' },
    ],
  },
]
applyAiSnapshot({ ...AI, busy: false, messages: PROPOSED })

const card = view.querySelector('.chat__changes')
ok(card !== null, 'کارتِ پیشنهاد زیرِ پاسخ رندر می‌شود')
ok(view.querySelectorAll('.chat__row')[0].querySelector('.chat__changes') === null, 'و حبابِ کاربر کارت نمی‌گیرد')
ok(card.querySelector('.chat__changestitle').textContent.includes('2'), `عنوان تعداد را می‌گوید («${card.querySelector('.chat__changestitle').textContent}»)`)
const lines = [...card.querySelectorAll('.chat__changekey')].map((l) => l.textContent)
ok(lines[0] === 'mtu: 1280 → 1380', `تغییر به شکلِ قدیم → جدید نوشته می‌شود («${lines[0]}»)`)
ok(lines[1] === 'fragment: off → on', `و مقدارِ نرمال‌شده نشان داده می‌شود («${lines[1]}»)`)
ok([...card.querySelectorAll('.chat__changewhy')].some((w) => w.textContent.includes('timing out')), 'دلیلِ خودِ مدل زیر هر تغییر می‌آید')
ok(card.querySelector('.chat__changenote').textContent.includes('next connect'), 'و کارت می‌گوید در اتصال بعدی اثر می‌کند')
// متنِ خامِ بلوک هرگز نباید در حباب دیده شود؛ Rust آن را بریده، این فقط نگهبانِ دوم است.
ok(!view.querySelector('.chat__text').textContent.includes('changes'), 'JSONِ خام در حباب نیست')

invoked.length = 0
const applyBtn = card.querySelector('.chat__apply')
ok(applyBtn !== null && applyBtn.textContent.includes('Apply'), 'دکمهٔ اعمال روی کارت هست')
applyBtn.click()
await tick()
await tick()
ok(last('ai_apply_changes')?.[1]?.id === 22, `اعمال با شناسهٔ همان حباب می‌رود (id=${last('ai_apply_changes')?.[1]?.id})`)
const popup = dialog()
ok(popup !== null, 'پس از اعمال پنجرهٔ «برای اتصال بعدی ذخیره شد» باز می‌شود')
ok(popup.textContent.includes('disconnect and connect again'), 'و می‌گوید باید قطع و وصل شود')
dialogButton('confirm').click()
await tick()
ok(dialog() === null, 'پنجره با «متوجه شدم» بسته می‌شود')

// حبابی که اعمال شده، دکمه ندارد و تیک دارد.
applyAiSnapshot({ ...AI, busy: false, messages: [PROPOSED[0], { ...PROPOSED[1], applied: true }] })
ok(view.querySelector('.chat__apply') === null, 'پس از اعمال، دکمه جای خودش را می‌دهد')
ok(view.querySelector('.chat__applied')?.textContent.includes('Applied'), 'و کارت «اعمال شد» نشان می‌دهد')

// یک پاسخِ معمولی هیچ کارتی نمی‌گیرد.
applyAiSnapshot({ ...AI, busy: false, messages: [{ id: 31, fromUser: false, text: 'MTU is the largest packet size.', failed: false, edited: false, changes: [], applied: false }] })
ok(view.querySelector('.chat__changes') === null, 'پاسخِ بدونِ پیشنهاد کارت نمی‌سازد')

// ---- مورد ۳: پرسش پیش از پاسخ روی صفحه است -------------------------------
//
// تضمینِ ترتیب در Rust است (`append_user_message` پیش از رفتن به شبکه، با تستِ
// خودش در `ai_session.rs`). آنچه اینجا سنجیده می‌شود نیمهٔ رابط کاربری است:
// جعبه فوراً خالی می‌شود، و snapshotی که فقط حبابِ کاربر و حالتِ انتظار دارد —
// همان چیزی که Rust پیش از پاسخ می‌فرستد — درست رندر می‌شود.
console.log('\n── پرسش پیش از پاسخ')
applyAiSnapshot({ ...AI, busy: false, messages: [] })
invoked.length = 0
const box = view.querySelector('.chat__input')
box.value = 'why is my connection slow?'
view.querySelector('.chat__composer').dispatchEvent(new dom.window.Event('submit', { bubbles: true, cancelable: true }))
await tick()
ok(box.value === '', 'جعبهٔ ورودی همان لحظهٔ فرستادن خالی می‌شود')
ok(last('ai_send_chat')?.[1]?.text === 'why is my connection slow?', 'و متن دست‌نخورده به Rust می‌رود')

applyAiSnapshot({
  ...AI,
  busy: true,
  messages: [{ id: 41, fromUser: true, text: 'why is my connection slow?', failed: false, edited: false, changes: [], applied: false }],
})
const early = [...view.querySelectorAll('.chat__row')]
ok(early.length === 2, `پرسش و نشانگرِ انتظار با هم دیده می‌شوند (${early.length} ردیف)`)
ok(early[0].classList.contains('is-user') && early[0].textContent.includes('why is my connection slow?'), 'حبابِ پرسش پیش از هر پاسخی روی صفحه است')
ok(view.querySelector('.chat__bubble.is-pending') !== null, 'و زیرش «در حال فکر کردن» است')

// ---- توقف ---------------------------------------------------------------
console.log('\n── توقف و انتظار')
applyAiSnapshot({ ...AI, busy: true, messages: MESSAGES })
const stop = view.querySelector('.chat__stop')
ok(!stop.hidden && view.querySelector('.chat__send').hidden, 'در حالِ انتظار، «توقف» جای «بفرست» را می‌گیرد')
ok(view.querySelector('.chat__bubble.is-pending') !== null, 'نشانگرِ «در حال فکر کردن» دیده می‌شود')
stop.click()
await tick()
ok(last('ai_stop') != null, 'دکمهٔ توقف فرمانِ ai_stop را می‌فرستد')

// ---- مورد ۴: انتقالِ پرسش از حبابِ ✨ به چت -------------------------------
console.log('\n── «متوجه نشدید؟ از دستیار بپرسید»')
applyAiSnapshot({ ...AI, busy: false, messages: [] })
let routedTo = null
setTabRouter((tab) => { routedTo = tab })

const hint = aiHintButton({ title: 'MTU', subtitle: 'Largest packet size', value: () => '1380' })
document.body.appendChild(hint)
hint.click()
await tick()
await tick()
const bubbleEl = document.querySelector('.aibubble')
ok(bubbleEl !== null, 'حبابِ توضیح باز می‌شود')
const ask = bubbleEl.querySelector('.aibubble__ask')
ok(ask !== null, 'پایکِ «از دستیار بپرسید» زیر پاسخ می‌آید')
ok(ask.textContent.includes('Ask the assistant'), `متنِ پایک درست است («${ask.textContent}»)`)
ask.click()
await tick()
ok(routedTo === 'chat', `کلیک روی پایک به تبِ چت می‌رود (${routedTo})`)
ok(document.querySelector('.aibubble') === null, 'حباب پس از انتقال بسته می‌شود')

// و پرسش با نمایشِ صفحهٔ چت فرستاده می‌شود، با عنوانِ تنظیم و متنِ توضیح در آن.
invoked.length = 0
view.__onShow()
await tick()
const handoff = last('ai_send_chat')
ok(handoff != null, 'پرسشِ منتقل‌شده با باز‌شدنِ صفحهٔ چت فرستاده می‌شود')
ok(handoff?.[1]?.text?.includes('MTU'), 'پرسش عنوانِ تنظیم را دارد')
ok(handoff?.[1]?.text?.includes('largest packet'), 'پرسش خودِ توضیح را هم می‌برد تا دستیار همان را دوباره تولید نکند')

// یک بار مصرف: بازدیدِ دوم نباید همان پرسش را دوباره بفرستد.
invoked.length = 0
view.__onShow()
await tick()
ok(last('ai_send_chat') == null, 'بازدیدِ دومِ صفحه پرسش را دوباره نمی‌فرستد')

// ---- دروازهٔ بسته --------------------------------------------------------
console.log('\n── دروازهٔ بسته')
resetNav()
applyAiSnapshot({ ...AI, gateCode: 'DISCONNECTED', gate: 'DISCONNECTED', messages: [] })
ok(view.querySelector('.chat__input').disabled, 'با تونلِ پایین جعبهٔ ورودی غیرفعال است')
ok(!view.querySelector('.ai__gate').hidden, 'و دلیلش روی صفحه نوشته می‌شود')
ok([...view.querySelectorAll('.chat__chip')].every((c) => c.disabled), 'پرسش‌های آماده هم غیرفعال می‌شوند')

console.log(failed === 0 ? '\nCHAT OK' : `\n${failed} ایراد`)
process.exit(failed === 0 ? 0 : 1)
