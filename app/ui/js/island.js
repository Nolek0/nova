// Île de l'encoche (macOS) : affiche l'état de Nova Code et permet de lui écrire.
// Le cœur Rust déplie/replie l'île selon la position de la souris (événement « island »).
import { invoke, listen } from './api.js';
import * as I18n from './i18n.js';

const T = window.__TAURI__;
const island = document.getElementById('island');
const q = document.getElementById('q');

async function init() {
  try {
    const b = await invoke('bootstrap');
    await I18n.setLanguage(I18n.resolve(b.settings.language));
    const proj = b.settings.codeProject;
    if (proj) document.getElementById('proj').textContent = proj.split(/[\\/]/).pop();
  } catch {}
  const g = await invoke('island_geometry').catch(() => null);
  if (g) {
    island.style.setProperty('--nw', g.notch_w + 'px');
    island.style.setProperty('--nh', g.notch_h + 'px');
    island.classList.toggle('no-notch', !g.has_notch);
  }
}

listen('island', (e) => {
  island.classList.toggle('open', !!e.payload);
  if (!e.payload) q.blur();
});

// État envoyé par la fenêtre principale (vue Nova Code).
listen('island-status', (e) => {
  const s = e.payload || {};
  island.classList.toggle('busy', !!s.busy);
  document.getElementById('state').textContent = I18n.t(s.busy ? 'Réfléchit…' : 'Prêt');
  if (s.project) document.getElementById('proj').textContent = s.project;
  if (s.preview) document.getElementById('preview').textContent = s.preview;
});

document.getElementById('ask').onsubmit = (e) => {
  e.preventDefault();
  const text = q.value.trim();
  q.value = '';
  invoke('island_open', { text: text || null });
};
document.getElementById('open').onclick = () => invoke('island_open', { text: null });
const hold = () => invoke('island_hold', { hold: document.activeElement === q && q.value.trim() !== '' }).catch(() => {});
q.addEventListener('input', hold);
q.addEventListener('blur', hold);
q.addEventListener('keydown', (e) => { if (e.key === 'Escape') { q.value = ''; q.blur(); hold(); } });

init();
