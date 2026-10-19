// Météo : Open-Meteo (gratuit, sans clé). Hors ligne, la dernière donnée connue est signalée comme ancienne.
import { invoke, esc, el, errorText, formatDate } from '../api.js';
import { icon } from '../icons.js';

const WMO = {
  0: ['☀️', 'Ciel dégagé'], 1: ['🌤️', 'Plutôt dégagé'], 2: ['⛅', 'Partiellement nuageux'], 3: ['☁️', 'Couvert'],
  45: ['🌫️', 'Brouillard'], 48: ['🌫️', 'Brouillard givrant'], 51: ['🌦️', 'Bruine légère'], 53: ['🌦️', 'Bruine'], 55: ['🌦️', 'Bruine dense'],
  56: ['🌧️', 'Bruine verglaçante'], 57: ['🌧️', 'Bruine verglaçante'], 61: ['🌦️', 'Pluie faible'], 63: ['🌧️', 'Pluie'], 65: ['🌧️', 'Forte pluie'],
  66: ['🌧️', 'Pluie verglaçante'], 67: ['🌧️', 'Pluie verglaçante'], 71: ['🌨️', 'Neige faible'], 73: ['🌨️', 'Neige'], 75: ['❄️', 'Forte neige'],
  77: ['🌨️', 'Grains de neige'], 80: ['🌦️', 'Averses'], 81: ['🌦️', 'Averses'], 82: ['⛈️', 'Fortes averses'], 85: ['🌨️', 'Averses de neige'],
  86: ['🌨️', 'Averses de neige'], 95: ['⛈️', 'Orage'], 96: ['⛈️', 'Orage avec grêle'], 99: ['⛈️', 'Orage avec grêle'],
};
const wmo = (c, day = true) => (c === 0 && !day ? ['🌙', 'Ciel dégagé'] : WMO[c] || ['🌡️', 'Inconnu']);
const r0 = (x) => (x === null || x === undefined ? '–' : Math.round(x));

export async function render(v, ctx) {
  const city = ctx.state.settings.defaultCity || '';
  const page = el(`<div class="page"><h1>Météo</h1><p class="sub">Données Open-Meteo (services météo nationaux), gratuites. Cette page interroge Internet.</p>
    <div class="row" style="position:relative"><span class="muted">${icon('pinmap')}</span><input class="field grow" id="city" placeholder="Rechercher une ville…" value="${esc(city)}"><button class="btn primary" id="go">Afficher</button></div>
    <div id="suggest" class="card hidden" style="padding:4px;margin-top:6px"></div><div id="out" class="stack" style="margin-top:16px"></div></div>`);
  v.appendChild(page);
  const input = page.querySelector('#city'), out = page.querySelector('#out'), sug = page.querySelector('#suggest');
  let timer;
  input.oninput = () => {
    clearTimeout(timer);
    if (input.value.trim().length < 2) { sug.classList.add('hidden'); return; }
    timer = setTimeout(async () => {
      try {
        const places = await invoke('search_places', { query: input.value });
        sug.innerHTML = '';
        places.forEach((p) => {
          const b = el(`<button class="file-row"><span class="name">${esc(p.name)}</span><span class="where">${esc([p.admin1, p.country].filter(Boolean).join(', '))}</span></button>`);
          b.onclick = () => { input.value = p.name; sug.classList.add('hidden'); show(p.name); };
          sug.appendChild(b);
        });
        sug.classList.toggle('hidden', !places.length);
      } catch { sug.classList.add('hidden'); }
    }, 300);
  };
  input.onkeydown = (e) => { if (e.key === 'Enter') { sug.classList.add('hidden'); show(input.value); } };
  page.querySelector('#go').onclick = () => show(input.value);

  async function show(name) {
    if (!name.trim()) return;
    out.innerHTML = '<div class="faint">Chargement…</div>';
    try {
      const r = await invoke('weather', { city: name });
      out.innerHTML = '';
      const c = r.data.current, d = r.data.daily, h = r.data.hourly;
      if (r.stale) out.appendChild(el(`<div class="stale">⚠️ <b>Pas de connexion Internet.</b> Voici la dernière météo enregistrée, du ${esc(formatDate(r.fetched_at))}. Ce n’est <b>pas</b> une donnée actuelle.</div>`));
      const [ic, label] = wmo(c.weather_code, c.is_day === 1);
      const place = [r.place.name, r.place.admin1, r.place.country].filter(Boolean).join(', ');
      out.appendChild(el(`<div class="card stack"><div class="row"><b>${esc(place)}</b><span class="grow"></span><button class="btn small" id="def">${icon('pinmap')} Ville par défaut</button></div>
        <div class="weather-now"><span class="icon">${ic}</span><span class="temp">${r0(c.temperature_2m)}°C</span><div><div style="font-weight:500">${label}</div><div class="muted">Ressenti ${r0(c.apparent_temperature)}°C</div></div></div>
        <div class="kv"><div><small>Humidité</small><span>${r0(c.relative_humidity_2m)} %</span></div><div><small>Vent</small><span>${r0(c.wind_speed_10m)} km/h</span></div><div><small>Rafales</small><span>${r0(c.wind_gusts_10m)} km/h</span></div><div><small>Précipitations</small><span>${c.precipitation ?? '–'} mm</span></div><div><small>Lever / coucher</small><span>${(d.sunrise?.[0] || '').slice(11)} / ${(d.sunset?.[0] || '').slice(11)}</span></div></div></div>`));
      out.querySelector('#def').onclick = async () => { await ctx.setSetting('defaultCity', r.place.name); out.querySelector('#def').textContent = '✓ Ville par défaut'; };
      const hours = el('<div class="card"><h2>Prochaines heures</h2><div class="hours"></div></div>');
      (h.time || []).slice(0, 24).forEach((t, i) => hours.querySelector('.hours').appendChild(el(`<div class="hour"><div class="faint">${t.slice(11, 13)} h</div><div class="e">${wmo(h.weather_code[i])[0]}</div><b>${r0(h.temperature_2m[i])}°</b><div class="faint">${h.precipitation_probability?.[i] ?? '–'} %</div></div>`)));
      out.appendChild(hours);
      const days = el('<div class="card"><h2>7 jours</h2><div class="days"></div></div>');
      const lo = Math.min(...d.temperature_2m_min), hi = Math.max(...d.temperature_2m_max);
      d.time.forEach((t, i) => {
        const date = new Date(t + 'T12:00');
        const name = i === 0 ? 'Aujourd’hui' : i === 1 ? 'Demain' : date.toLocaleDateString('fr-FR', { weekday: 'long', day: 'numeric' });
        const left = ((d.temperature_2m_min[i] - lo) / (hi - lo || 1)) * 100, width = ((d.temperature_2m_max[i] - d.temperature_2m_min[i]) / (hi - lo || 1)) * 100;
        days.querySelector('.days').appendChild(el(`<div class="day"><span style="text-transform:capitalize">${name}</span><span style="font-size:20px">${wmo(d.weather_code[i])[0]}</span><div style="position:relative;height:5px;background:var(--hover);border-radius:99px"><div class="bar" style="position:absolute;left:${left}%;width:${Math.max(4, width)}%"></div></div><span class="mono">${r0(d.temperature_2m_min[i])}° / <b>${r0(d.temperature_2m_max[i])}°</b></span><span class="faint">${d.precipitation_probability_max?.[i] ?? '–'} %</span></div>`));
      });
      out.appendChild(days);
      out.appendChild(el(`<div class="faint">Source : ${esc(r.source)} · mise à jour : ${esc(formatDate(r.fetched_at))}</div>`));
    } catch (e) {
      out.innerHTML = '';
      out.appendChild(el(`<div class="error-box">${icon('alert')}<span>${esc(errorText(e))}</span></div>`));
    }
  }
  if (city) show(city); else input.focus();
}
