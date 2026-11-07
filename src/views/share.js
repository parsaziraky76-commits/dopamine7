// =============================================================================
//  اشتراک VPN — پورت از ui/SharePanel.kt (موبایل ۱.۵.۰)
//
//  همان ساختار و همان متن‌های موبایل، با یک تفاوت عمدی: حالت «هات‌اسپات
//  موبایل» در ویندوز نیست؛ فقط حالت «وای‌فای خانگی» (هر شبکهٔ Wi‑Fi یا کابلیِ
//  همین رایانه). آدرس‌ها، وضعیت و تعداد اتصال‌ها از snapshot می‌آیند؛ رمزِ
//  اختیاریِ اشتراک از `share_info` (مهرشده با DPAPI) و هرگز در صفحه‌های اشتراک
//  نشان داده نمی‌شود.
// =============================================================================
import { invoke } from '@tauri-apps/api/core'
import { app, onChange, saveProfile } from '../main.js'
import { t } from '../i18n.js'
import { toast } from '../ui/toast.js'
import { openModal, modalButton, modalNote } from '../ui/modal.js'

const HTTP_PORT = 10811
const SOCKS_PORT = 10810
const CHECK_URL = 'http://dopamine.check/'
const MIN_PASSWORD = 8

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c])
const fill = (text, ...values) => values.reduce((s, v, i) => s.replace(`{${i}}`, String(v)), text)

let creds = { authRequired: false, user: 'dopamine', password: '' }

function copy(text, message) {
  if (!text) return
  try {
    navigator.clipboard?.writeText(text)
  } catch {
    // کلیپ‌بورد در دسترس نیست — بی‌صدا.
  }
  toast(message || t('Address copied'))
}

function valueRow(label, value, { secret = false, id = '' } = {}) {
  const shown = secret ? '\u2022'.repeat(Math.min(Math.max(value.length, 8), 16)) : value
  return `
    <div class="share__row">
      <div class="share__rowtext">
        <span class="share__label">${esc(label)}</span>
        <span class="share__value ltr" dir="ltr" ${id ? `id="${id}"` : ''} data-secret="${secret ? '1' : ''}">${esc(shown)}</span>
      </div>
      <div class="share__rowbtns">
        ${secret ? `<button type="button" class="btn btn--small btn--ghost" data-reveal="${id}">${t('Show')}</button>` : ''}
        <button type="button" class="btn btn--small btn--ghost" data-copy-value="${esc(value)}">${t('Copy')}</button>
      </div>
    </div>`
}

function notice(text, kind = '') {
  return `<p class="share__notice ${kind}">${esc(text)}</p>`
}

function readyBlock(primary, others) {
  const ip = primary.address
  const http = `${ip}:${HTTP_PORT}`
  const socks = `${ip}:${SOCKS_PORT}`
  const pac = `http://${ip}:${HTTP_PORT}/proxy.pac`
  const setup = `http://${ip}:${HTTP_PORT}/`
  const ifaceLabel = (kind) => (kind === 'ethernet' ? t('Ethernet address') : t('Wi\u2011Fi address'))

  let html = `
    <p class="share__pill"><span aria-hidden="true">\u25CF</span> ${esc(t('Zero\u2011leak guard active'))}</p>
    <p class="field__hint">${esc(t('Everything the other device sends through the share leaves via the VPN tunnel only: DNS is resolved inside the tunnel, UDP is relayed through it, local/private targets are blocked, and if the tunnel drops, connections fail instead of going direct.'))}</p>
    <h3 class="view__subtitle">${esc(t('Enter on the other device (same Wi\u2011Fi)'))}</h3>
    ${valueRow(t('PC IP address'), ip)}
    ${valueRow(t('HTTP proxy — enter in system proxy settings'), http)}
    ${valueRow(t('SOCKS5 proxy — for apps that support SOCKS'), socks)}
    ${valueRow(t('Automatic proxy (PAC) URL — easiest, fail\u2011closed'), pac)}`
  if (others.length) {
    html += `<h3 class="view__subtitle">${esc(t('Also reachable on these links (use the one the device is connected by)'))}</h3>`
    html += others.map((o) => valueRow(ifaceLabel(o.kind), o.address)).join('')
  }
  html += `<button type="button" class="btn share__copyall" id="share-copy-all">${esc(t('Copy all details'))}</button>`

  html += `
    <h3 class="view__subtitle">${esc(t('WebRTC (real\u2011IP leak in browsers)'))}</h3>
    ${notice(t('A browser set to use a proxy still sends WebRTC on its own connection, which can reveal the device\u2019s real IP. Open the setup page below on the other device: it tests WebRTC live and gives a one\u2011click lock for Chrome, Edge, Brave and Firefox. Phones: use v2rayNG / NekoBox in VPN mode with the SOCKS5 address.'), 'is-warn')}
    ${valueRow(t('Setup page + WebRTC test (open on the other device)'), setup)}
    <h3 class="view__subtitle">${esc(t('How to connect'))}</h3>
    <ol class="share__steps">
      <li>${esc(t('Make sure the other device is on the same Wi\u2011Fi (or wired network) as this PC.'))}</li>
      <li>${esc(t('In its Wi\u2011Fi / network settings choose Proxy → Automatic and paste the PAC URL (or Manual with the HTTP proxy). For apps and TUN clients (v2rayN, NekoBox, sing\u2011box) use the SOCKS5 address with remote DNS.'))}</li>
      <li>${esc(t('Open the check page below in a browser: it only loads when the device is protected.'))}</li>
    </ol>
    ${valueRow(t('Protection check page'), CHECK_URL)}
    <p class="share__warn">${esc(t('Only traffic sent to the share is protected. A device or app that is not set to use it goes out on its own connection — a shared proxy cannot force other devices through the VPN.'))}</p>`

  const all = [
    'Dopamine VPN share',
    `IP: ${ip}`,
    `HTTP proxy: ${http}`,
    `SOCKS5 proxy: ${socks}`,
    `PAC URL: ${pac}`,
    ...others.map((o) => `Also: ${o.address} (${o.kind.toUpperCase()})`),
    `Setup + WebRTC lock: ${setup}`,
    ...(creds.authRequired ? [`Username: ${creds.user}`, `Password: ${creds.password}`] : []),
    `Check: ${CHECK_URL}`,
  ].join('\n')
  return { html, all }
}

function authBlock() {
  let html = `
    <section class="field field--row">
      <div>
        <span class="field__label">${esc(t('Require username & password'))}</span>
        <span class="field__hint">${esc(t('Optional — recommended on shared or public Wi\u2011Fi. Devices must sign in before using the share.'))}</span>
      </div>
      <button type="button" class="switch ${creds.authRequired ? 'is-on' : ''}" id="share-auth-toggle" role="switch" aria-checked="${creds.authRequired}"><span class="switch__knob"></span></button>
    </section>`
  if (creds.authRequired) {
    html += `
      <p class="field__hint">${esc(t('The other device must sign in to the proxy with these:'))}</p>
      ${valueRow(t('Proxy username'), creds.user)}
      ${valueRow(t('Proxy password'), creds.password, { secret: true, id: 'share-pass' })}
      <div class="row">
        <button type="button" class="btn btn--small" id="share-auth-edit">${esc(t('Edit'))}</button>
        <button type="button" class="btn btn--small" id="share-auth-rotate">${esc(t('Generate a new password'))}</button>
      </div>`
  }
  return html
}

function editDialog(onSaved) {
  const modal = openModal({ title: t('Share username & password'), onEscape: () => modal.close() })
  const mk = (label, value, type) => {
    const wrap = document.createElement('label')
    wrap.className = 'field'
    const span = document.createElement('span')
    span.className = 'field__label'
    span.textContent = label
    const input = document.createElement('input')
    input.className = 'input ltr'
    input.dir = 'ltr'
    input.type = type
    input.autocomplete = 'off'
    input.spellcheck = false
    input.value = value
    wrap.append(span, input)
    modal.body.append(wrap)
    return input
  }
  const user = mk(t('Proxy username'), creds.user, 'text')
  const pass = mk(t('Proxy password'), creds.password, 'password')
  const error = modalNote('', 'is-error')
  error.hidden = true
  modal.body.append(error)
  const save = modalButton(t('Save'), 'btn--primary', async () => {
    error.hidden = true
    save.disabled = true
    try {
      creds = await invoke('share_set_credentials', { user: user.value, password: pass.value }) || creds
      modal.close()
      toast(t('Credentials saved'))
      onSaved()
    } catch (e) {
      error.textContent = String(e) === 'badUser'
        ? t('Username: 1–64 Latin characters, no spaces or ":".')
        : String(e) === 'shortPass'
          ? fill(t('Password must be at least {0} Latin characters.'), MIN_PASSWORD)
          : t(String(e))
      error.hidden = false
    } finally {
      save.disabled = false
    }
  })
  modal.actions.append(modalButton(t('Cancel'), 'btn--ghost', () => modal.close()), save)
  setTimeout(() => user.focus(), 0)
}

export function renderShare() {
  const root = document.createElement('div')
  root.className = 'view view--share'
  root.innerHTML = `
    <h2 class="view__title">${t('Share VPN')}</h2>
    <p class="view__lead" id="share-status"></p>

    <section class="field field--row">
      <div>
        <span class="field__label">${t('Share on this network')}</span>
        <span class="field__hint">${t('Opens a local proxy that devices on the same Wi\u2011Fi or wired network can use')}</span>
      </div>
      <button type="button" class="switch" id="share-toggle" role="switch"><span class="switch__knob"></span></button>
    </section>

    <div class="share__body" id="share-body"></div>
    <hr class="share__divider">
    <div class="share__auth" id="share-auth"></div>
  `

  const sw = root.querySelector('#share-toggle')
  sw.addEventListener('click', async () => {
    const on = !sw.classList.contains('is-on')
    sw.classList.toggle('is-on', on)
    sw.setAttribute('aria-checked', String(on))
    await saveProfile({ lanShare: on })
  })

  // یک شنونده برای همهٔ دکمه‌ها — بدنه با هر تغییرِ وضعیت دوباره ساخته می‌شود.
  root.addEventListener('click', async (ev) => {
    const target = ev.target instanceof Element ? ev.target.closest('button') : null
    if (!target) return
    if (target.dataset.copyValue !== undefined) {
      copy(target.dataset.copyValue)
      return
    }
    if (target.dataset.reveal) {
      const el = root.querySelector(`#${target.dataset.reveal}`)
      if (!el) return
      const hidden = el.textContent.startsWith('\u2022')
      el.textContent = hidden ? creds.password : '\u2022'.repeat(Math.min(Math.max(creds.password.length, 8), 16))
      target.textContent = hidden ? t('Hide') : t('Show')
      return
    }
    if (target.id === 'share-copy-all') {
      copy(root._shareAll || '', t('All details copied'))
      return
    }
    if (target.id === 'share-auth-toggle') {
      target.disabled = true
      try {
        creds = await invoke('share_set_auth', { required: !creds.authRequired }) || creds
      } catch (e) {
        toast(t(String(e)))
      }
      paintAuth()
      lastSig = ''
      paint(app)
      return
    }
    if (target.id === 'share-auth-edit') {
      editDialog(() => { paintAuth(); lastSig = ''; paint(app) })
      return
    }
    if (target.id === 'share-auth-rotate') {
      try {
        creds = await invoke('share_rotate_password') || creds
        toast(t('New proxy password generated'))
      } catch (e) {
        toast(t(String(e)))
      }
      paintAuth()
      lastSig = ''
      paint(app)
    }
  })

  const paintAuth = () => {
    root.querySelector('#share-auth').innerHTML = authBlock()
  }

  let lastSig = ''
  const paint = ({ snapshot, profile }) => {
    const s = snapshot || {}
    const on = !!profile?.lanShare
    sw.classList.toggle('is-on', on)
    sw.setAttribute('aria-checked', String(on))

    const connected = s.state === 'CONNECTED'
    const status = s.shareStatus || 'off'
    const targets = Array.isArray(s.shareTargets) ? s.shareTargets : []
    const clients = s.shareClients || 0
    const live = connected && on && status === 'ready'
    const statusEl = root.querySelector('#share-status')
    statusEl.textContent = live
      ? fill(t('Sharing live · {0} connection(s)'), clients)
      : t('Use this PC as a proxy for your laptop, phone or another computer')
    statusEl.classList.toggle('is-live', live)

    const sig = JSON.stringify([connected, on, status, targets, creds.authRequired, creds.user, creds.password])
    if (sig === lastSig) return
    lastSig = sig

    const body = root.querySelector('#share-body')
    root._shareAll = ''
    if (!connected) {
      body.innerHTML = notice(t('Connect the VPN first — sharing routes other devices through this PC’s tunnel.'))
    } else if (!on) {
      body.innerHTML = notice(t('Turn on sharing, and the addresses to enter on the other device appear here.'))
    } else if (status === 'portBusy') {
      body.innerHTML = notice(t('Ports 10810/10811 are used by another app on this PC. Close it and toggle sharing again.'), 'is-error')
    } else if (status === 'waiting' || (status === 'ready' && !targets.length)) {
      body.innerHTML = notice(t('Connect this PC to your home Wi\u2011Fi or wired network. The addresses appear here automatically.'))
    } else if (status === 'ready') {
      const { html, all } = readyBlock(targets[0], targets.slice(1))
      body.innerHTML = html
      root._shareAll = all
    } else {
      body.innerHTML = notice(t('Starting sharing… if this takes more than a few seconds, toggle the switch off and on again.'))
    }
  }

  paintAuth()
  paint(app)
  onChange(paint, root)
  invoke('share_info')
    .then((c) => {
      if (c && typeof c === 'object') creds = c
      paintAuth()
      lastSig = ''
      paint(app)
    })
    .catch(() => {})
  return root
}
