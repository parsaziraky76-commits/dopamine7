// =============================================================================
//  ۱.۵.۰ — دیالوگِ سراسریِ کدِ ورودِ Zero Trust (پورت از ui/LoginCodeDialog.kt)
//
//  وقتی هسته حینِ اتصال کدِ ایمیلی می‌خواهد، `snapshot.loginCode` پر می‌شود و
//  این دیالوگ روی هر صفحه‌ای که باز است بالا می‌آید. «ارسال کد» کد را به stdinِ
//  هسته می‌دهد؛ «حالا نه» اتصال را همان لحظه متوقف می‌کند. کلیکِ بیرون و Escape
//  هیچ کاری نمی‌کنند — کدِ نیمه‌تایپ‌شده نباید با یک کلیکِ اشتباه از دست برود.
// =============================================================================
import { invoke } from '@tauri-apps/api/core'
import { t } from '../i18n.js'
import { openModal, modalButton, modalNote } from './modal.js'

let current = null // { key, modal, input, send }
// «حالا نه» زده شد؛ تا هسته درخواست را پس بگیرد، همان درخواست دوباره باز نشود.
let dismissedKey = null

function fill(text, ...values) {
  return values.reduce((s, v, i) => s.replace(`{${i}}`, String(v)), text)
}

function close() {
  current?.modal.close()
  current = null
}

/** از هر snapshot صدا زده می‌شود؛ فقط وقتی درخواست عوض شده DOM را می‌سازد. */
export function syncLoginCode(request) {
  if (!request) {
    dismissedKey = null
    close()
    return
  }
  // کلید روی شمارهٔ تلاش: کدِ ردشده در جعبه نمی‌ماند.
  const key = `${request.attempt}/${request.email || ''}`
  if (current?.key === key || dismissedKey === key) return
  close()

  const modal = openModal({ title: t('Enter your login code') })
  const email = (request.email || '').trim()
  modal.body.append(
    modalNote(
      email
        ? fill(t('Your organization e-mailed a one-time code to {0}. Type it here to finish joining.'), email)
        : t('Your organization e-mailed you a one-time code. Type it here to finish joining.'),
    ),
  )
  if (request.attempt > 1) {
    modal.body.append(
      modalNote(
        fill(t('That code was not accepted. Attempt {0} of {1}.'), request.attempt, request.maxAttempts),
        'is-error',
      ),
    )
  }
  const input = document.createElement('input')
  input.className = 'input ltr modal__code'
  input.dir = 'ltr'
  input.type = 'text'
  input.inputMode = 'numeric'
  input.autocomplete = 'one-time-code'
  input.maxLength = 12
  input.placeholder = t('Login code')
  input.setAttribute('aria-label', t('Login code'))
  modal.body.append(input)
  modal.body.append(
    modalNote(fill(t('The connection is waiting for this. It gives up after {0} seconds.'), request.waitSeconds)),
  )

  const send = modalButton(t('Send code'), 'btn--primary', async () => {
    const code = input.value.trim()
    if (!code) return
    send.disabled = true
    try {
      await invoke('zt_login_code_submit', { code })
    } finally {
      send.disabled = false
    }
  })
  send.disabled = true
  input.addEventListener('input', () => {
    send.disabled = !input.value.trim()
  })
  input.addEventListener('keydown', (ev) => {
    if (ev.key === 'Enter' && input.value.trim()) send.click()
  })
  const later = modalButton(t('Not now'), 'btn--ghost', () => {
    dismissedKey = key
    close()
    invoke('zt_login_code_dismiss').catch(() => {})
  })
  modal.actions.append(later, send)
  current = { key, modal, input, send }
  setTimeout(() => input.focus(), 0)
}
