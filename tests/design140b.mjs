// جستجوی کشور + مدال حمایت (jsdom)
import { JSDOM } from 'jsdom'
import assert from 'node:assert/strict'
const dom = new JSDOM('<!doctype html><html><body></body></html>', { url: 'http://localhost/' })
globalThis.window = dom.window; globalThis.document = dom.window.document
globalThis.HTMLElement = dom.window.HTMLElement; globalThis.localStorage = dom.window.localStorage
window.__TAURI_INTERNALS__ = { invoke: () => Promise.resolve(null), transformCallback: (c) => c }
const { openSupportDialog, supportSuppressed } = await import('../src/ui/support.js')
const h = openSupportDialog({ showDontShowAgain: true })
assert.ok(document.querySelector('.news--support'))
assert.ok(document.body.textContent.includes('t.me/dopamin_love_p'))
document.querySelector('.news__checkbox').checked = true
h.close()
assert.equal(supportSuppressed(), true)
assert.equal(document.querySelector('.news--support'), null)
const { renderAnnouncements } = await import('../src/views/announcements.js')
const panel = renderAnnouncements()
assert.ok(panel.textContent.includes('پشتیبانی Dopamine'))
console.log('design140b ok')
