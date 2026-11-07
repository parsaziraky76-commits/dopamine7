import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'
import iconUrl from '../icon.png'
import { t } from '../i18n.js'

const TELEGRAM_URL = 'https://t.me/dopamin_love_p'
const TELEGRAM_ICON = '<svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true" fill="currentColor"><path d="M21.94 4.35c.25-1.02-.38-1.47-1.09-1.2L2.7 10.14c-1.19.46-1.18 1.13-.21 1.42l4.63 1.44 10.75-6.77c.51-.33.97-.15.59.18l-8.7 7.85-.33 4.78c.48 0 .69-.21.94-.46l2.26-2.18 4.7 3.46c.87.48 1.49.23 1.71-.8l3.9-14.71Z"/></svg>'

export function renderAbout() {
  const root = document.createElement('div')
  root.className = 'view view--about'
  root.innerHTML = `
    <div class="about__mark"><img src="${iconUrl}" alt="Dopamine" width="88" height="88"></div>
    <h2 class="view__title">Dopamine</h2>
    <p class="view__lead">${t('Freedom, in one tap')}</p>
    <dl class="kv">
      <dt>${t('App version')}</dt><dd class="ltr" dir="ltr" id="a-app">—</dd>
      <dt>${t('Core version')}</dt><dd class="ltr" dir="ltr" id="a-core">—</dd>
      <dt>${t('Architecture')}</dt><dd class="ltr" dir="ltr" id="a-arch">—</dd>
    </dl>
    <section class="about-card" id="about-card">
      <button type="button" class="about-card__head" id="about-toggle" aria-expanded="false" aria-controls="about-body">
        <span class="about-card__info"><svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="currentColor"><path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm0 5a1.25 1.25 0 1 1 0 2.5A1.25 1.25 0 0 1 12 7Zm1 10h-2v-6h2v6Z"/></svg></span>
        <span class="about-card__titles"><span class="about-card__title">${t('About')}</span><span class="about-card__sub">${t('Dopamine features, support and notices')}</span></span>
        <span class="about-card__chevron"><svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true" fill="currentColor"><path d="M7.41 8.59 12 13.17l4.59-4.58L18 10l-6 6-6-6 1.41-1.41Z"/></svg></span>
      </button>
      <div class="about-card__body" id="about-body"><div class="about-card__inner">
        <hr class="about-card__sep"><p class="about-card__version">${t('Version')} <span class="ltr" dir="ltr" id="a-version">—</span></p>
        <h3 class="about-sec__title">${t('Dopamine Desktop')}</h3>
        <p class="about-sec__note">${t('A Windows desktop client with one-tap tunnelling, live connection details, custom configurations, diagnostics and bilingual controls.')}</p>
        <ul class="features">
          <li><span class="features__dot">•</span><span>${t('System-wide tunnel using the Windows TUN driver')}</span></li>
          <li><span class="features__dot">•</span><span>${t('Automatic reconnect and connection diagnostics')}</span></li>
          <li><span class="features__dot">•</span><span>${t('Custom server configurations and protocol chaining')}</span></li>
          <li><span class="features__dot">•</span><span>${t('Live latency, traffic and session information')}</span></li>
          <li><span class="features__dot">•</span><span>${t('English and Persian interface')}</span></li>
        </ul>
        <hr class="about-card__sep">
        <h3 class="about-sec__title">${t('Dopamine Telegram')}</h3>
        <p class="about-sec__note">${t('Follow updates and contact the Dopamine team.')}</p>
        <button type="button" class="linkrow" id="dopamine-telegram" data-url="${TELEGRAM_URL}">
          <span class="linkrow__icon">${TELEGRAM_ICON}</span><span class="linkrow__label ltr" dir="ltr">t.me/dopamin_love_p</span>
        </button>
        <hr class="about-card__sep">
        <p class="about-sec__note">${t('This build includes third-party components. Their original copyright and license notices are preserved in LICENSE and THIRD_PARTY_NOTICES.')}</p>
      </div></div>
    </section>`

  invoke('about_info').then((info) => {
    root.querySelector('#a-app').textContent = info.appVersion
    root.querySelector('#a-core').textContent = info.coreVersion
    root.querySelector('#a-arch').textContent = info.arch
    root.querySelector('#a-version').textContent = info.appVersion
  }).catch(() => {})

  const card = root.querySelector('#about-card')
  const toggle = root.querySelector('#about-toggle')
  toggle.addEventListener('click', () => {
    const openNow = card.classList.toggle('is-open')
    toggle.setAttribute('aria-expanded', String(openNow))
  })
  root.querySelector('#dopamine-telegram').addEventListener('click', () => open(TELEGRAM_URL).catch(() => {}))
  return root
}
