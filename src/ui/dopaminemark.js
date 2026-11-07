// Dopamine connected-state mark.
// The square background remains dim and fixed; only the transparent D/circuit
// foreground is revealed. The light sweep and sparkles are masked to that same
// foreground artwork so they never illuminate the icon background.
import baseLogoUrl from '../assets/dopamine-logo-base.png'
import foregroundUrl from '../assets/dopamine-logo-foreground.png'

const ENERGY_SVG = `
<svg class="dp-mark__energy" viewBox="0 0 432 432" aria-hidden="true" preserveAspectRatio="none">
  <defs>
    <clipPath id="dp-letter-d" clipPathUnits="userSpaceOnUse">
      <path d="M164 168 H226 C269 168 293 185 293 216 C293 247 269 264 226 264 H164 V241 H225 C253 241 270 231 270 216 C270 201 253 191 225 191 H187 V241 H164 Z" fill-rule="evenodd"/>
    </clipPath>
    <linearGradient id="dp-beam-gradient" x1="0" x2="1" y1="0" y2="0">
      <stop offset="0" stop-color="#20D9D0" stop-opacity="0"/>
      <stop offset=".25" stop-color="#238BFF" stop-opacity=".22"/>
      <stop offset=".48" stop-color="#BDFBFF" stop-opacity=".98"/>
      <stop offset=".61" stop-color="#48E8FF" stop-opacity=".9"/>
      <stop offset="1" stop-color="#238BFF" stop-opacity="0"/>
    </linearGradient>
    <filter id="dp-glow" x="-100%" y="-100%" width="300%" height="300%">
      <feGaussianBlur stdDeviation="13"/>
    </filter>
  </defs>
  <g clip-path="url(#dp-letter-d)">
    <rect class="dp-mark__softbeam" fill="#238BFF" filter="url(#dp-glow)" height="432" opacity=".72" width="250"/>
    <rect class="dp-mark__beam" fill="url(#dp-beam-gradient)" height="432" width="190"/>
    <path class="dp-mark__ring-glint dp-mark__ring-glint--halo" d="M170 175 H224 C261 175 283 190 283 216 C283 242 261 257 224 257 H170" fill="none" filter="url(#dp-glow)" stroke="#45CFFF" stroke-dasharray="34 332" stroke-linecap="round" stroke-width="13"/>
    <path class="dp-mark__ring-glint dp-mark__ring-glint--core" d="M170 175 H224 C261 175 283 190 283 216 C283 242 261 257 224 257 H170" fill="none" stroke="#E8FCFF" stroke-dasharray="17 349" stroke-linecap="round" stroke-width="4"/>
    <g transform="translate(210 177)"><path class="dp-mark__star dp-mark__star--a" d="M0 -8 L2 -2 L8 0 L2 2 L0 8 L-2 2 L-8 0 L-2 -2 Z"/></g>
    <g transform="translate(278 219)"><path class="dp-mark__star dp-mark__star--b" d="M0 -7 L2 -2 L7 0 L2 2 L0 7 L-2 2 L-7 0 L-2 -2 Z"/></g>
    <g transform="translate(207 255)"><path class="dp-mark__star dp-mark__star--c" d="M0 -7 L2 -2 L7 0 L2 2 L0 7 L-2 2 L-7 0 L-2 -2 Z"/></g>
  </g>
</svg>`

export function startDopamineMark(host, { animated = true } = {}) {
  if (!host) return () => {}

  host.innerHTML = `
    <span class="dp-mark__card ${animated ? 'is-entering' : 'is-static'}" aria-hidden="true">
      <img class="dp-mark__base" src="${baseLogoUrl}" alt="" draggable="false"/>
      <img class="dp-mark__reveal" src="${foregroundUrl}" alt="" draggable="false"/>
      <span class="dp-mark__shine"></span>
      <span class="dp-mark__sparkles">
        <i class="dp-mark__spark dp-mark__spark--1"></i>
        <i class="dp-mark__spark dp-mark__spark--2"></i>
        <i class="dp-mark__spark dp-mark__spark--3"></i>
        <i class="dp-mark__spark dp-mark__spark--4"></i>
        <i class="dp-mark__spark dp-mark__spark--5"></i>
      </span>
      ${ENERGY_SVG}
    </span>`

  host.classList.add('connect__mark--dopamine')
  return () => {
    host.classList.remove('connect__mark--dopamine')
    host.innerHTML = ''
  }
}
