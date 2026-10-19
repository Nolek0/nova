// Île (macOS) : Nova Chat ou Nova Code depuis l'encoche (ou la pastille des écrans externes).
// Le cœur Rust déplie/replie l'île selon la position de la souris (événement « island »).
// La réponse s'affiche dans l'île ; « Ouvrir » reprend la conversation dans NOVA.
import { invoke, listen, Channel, esc, el, errorText } from './api.js';
import { renderMarkdown, wireMarkdown } from './markdown.js';
import { icon } from './icons.js';
import * as I18n from './i18n.js';

const T = window.__TAURI__;
const island = document.getElementById('island');
const q = document.getElementById('q');
const thread = document.getElementById('thread');
const modelSel = document.getElementById('model');
let settings = {};
let mode = 'chat';
let conversationId = null;
let streaming = false;

async function load() {
  try {
    const b = await invoke('bootstrap');
    settings = b.settings;
    if (I18n.currentLanguage() !== I18n.resolve(settings.language)) await I18n.setLanguage(I18n.resolve(settings.language));
    const codeOn = settings.novaCode === '1';
    document.getElementById('tabs').classList.toggle('single', !codeOn);
    if (!codeOn && mode === 'code') setMode('chat');
    const proj = settings.codeProject;
    document.getElementById('proj').textContent = mode === 'code' && proj ? proj.split(/[\\/]/).pop() : '';
    const o = await invoke('models_overview');
    const wanted = settings.islandModel || settings.model;
    modelSel.innerHTML = '';
    for (const m of o.available) modelSel.appendChild(el(`<option value="${esc(m.id)}">${esc(m.name)}</option>`));
    if (!o.available.length) modelSel.appendChild(el('<option value="">Aucun modèle installé</option>'));
    modelSel.value = o.available.some((m) => m.id === wanted) ? wanted : (o.available[0]?.id ?? '');
  } catch {}
}

async function init() {
  const g = await invoke('island_geometry').catch(() => null);
  if (g) {
    island.style.setProperty('--nw', g.notch_w + 'px');
    island.style.setProperty('--nh', g.notch_h + 'px');
    island.classList.toggle('no-notch', !g.has_notch);
  }
  await load();
}

function setMode(m) {
  if (mode === m) return;
  mode = m;
  document.querySelectorAll('#tabs button').forEach((b) => b.classList.toggle('on', b.dataset.mode === m));
  q.placeholder = I18n.t(m === 'code' ? 'Demander à Nova Code…' : 'Demande à NOVA…');
  const proj = settings.codeProject;
  document.getElementById('proj').textContent = m === 'code' && proj ? proj.split(/[\\/]/).pop() : '';
  reset();
}

function reset() {
  if (streaming) return;
  conversationId = null;
  thread.innerHTML = '';
  island.classList.remove('has-thread');
  hold();
}

function paint(node, text, status, done, error) {
  node.innerHTML = '';
  if (error) { node.appendChild(el(`<div class="error-box">${icon('alert')}<span>${esc(error)}</span></div>`)); return; }
  if (text) {
    const md = el('<div class="md"></div>');
    md.innerHTML = renderMarkdown(text);
    wireMarkdown(md);
    node.appendChild(md);
  }
  if (!done && !text) node.appendChild(el(`<div class="status-line"><span class="typing"><i></i><i></i><i></i></span>${esc(status || '')}</div>`));
}

async function ask(text) {
  streaming = true;
  hold();
  island.classList.add('has-thread', 'busy');
  thread.appendChild(Object.assign(el('<div class="q-user"></div>'), { textContent: text }));
  const answer = el('<div class="q-ai"></div>');
  thread.appendChild(answer);
  paint(answer, '', I18n.t('Réflexion…'), false);
  thread.scrollTop = thread.scrollHeight;
  document.getElementById('mark').classList.add('spin');
  let reply = '', status = '', frame = 0, finished = false;
  const redraw = () => { frame = 0; if (!finished) { paint(answer, reply, status, false); thread.scrollTop = thread.scrollHeight; } };
  const end = () => { streaming = false; island.classList.remove('busy'); document.getElementById('mark').classList.remove('spin'); hold(); };
  const ch = new Channel();
  ch.onmessage = (ev) => {
    if (ev.type === 'started') conversationId = ev.conversation_id;
    else if (ev.type === 'status') status = ev.text;
    else if (ev.type === 'delta') reply += ev.text;
    else if (ev.type === 'replace') reply = ev.text;
    else if (ev.type === 'meta' || ev.type === 'reasoning') return;
    else if (ev.type === 'done') {
      finished = true;
      if (frame) cancelAnimationFrame(frame);
      const m = ev.message;
      paint(answer, m.content, '', true, m.status === 'failed' ? m.meta?.error || 'La génération a échoué.' : null);
      if (m.meta?.action) answer.appendChild(el(`<div class="faint">Action proposée : ${esc(m.meta.action.title)}. Clique « Ouvrir » pour l'autoriser dans NOVA.</div>`));
      if (mode === 'code' && settings.codeProject && /```/.test(m.content || '')) answer.appendChild(el(`<div class="faint">Clique « Ouvrir » pour relire et appliquer le code dans Nova Code.</div>`));
      thread.scrollTop = thread.scrollHeight;
      T.event.emit('conversations-changed');
      end();
      return;
    }
    if (!frame) frame = requestAnimationFrame(redraw);
  };
  try {
    const options = { model_id: modelSel.value || null, web: false, think: false };
    if (mode === 'code') Object.assign(options, { mode: 'code', project: settings.codeProject || null, open_file: null });
    await invoke('send_message', { conversationId, text, attachments: [], options, onEvent: ch });
  } catch (e) {
    paint(answer, '', '', true, errorText(e));
    end();
  }
}

// L'île reste ouverte pendant la saisie et pendant une réponse.
function hold() {
  const typing = document.activeElement === q && q.value.trim() !== '';
  invoke('island_hold', { hold: typing || streaming }).catch(() => {});
}

listen('island', (e) => {
  island.classList.toggle('open', !!e.payload);
  if (e.payload) load();
  else q.blur();
});

document.querySelectorAll('#tabs button').forEach((b) => (b.onclick = () => setMode(b.dataset.mode)));
modelSel.onchange = () => { settings.islandModel = modelSel.value; invoke('set_setting', { key: 'islandModel', value: modelSel.value }).catch(() => {}); };
document.getElementById('ask').onsubmit = (e) => {
  e.preventDefault();
  const text = q.value.trim();
  if (!text || streaming) return;
  q.value = '';
  ask(text);
};
document.getElementById('new').onclick = () => { reset(); q.focus(); };
document.getElementById('open').onclick = () => invoke('island_open', { conversationId, mode });
// Un clic dans l'île lui donne le clavier (sinon la frappe irait à l'app au premier plan).
document.querySelector('.isl-panel').addEventListener('mousedown', (e) => {
  const field = e.target.closest('input') ? q : null;
  invoke('island_focus').catch(() => {}).then(() => {
    if (!field) return;
    // macOS ne donne pas toujours le clavier à la page tout de suite : on redemande.
    setTimeout(() => field.focus(), 30);
    setTimeout(() => field.focus(), 150);
  });
});
window.addEventListener('focus', () => { if (island.classList.contains('open')) q.focus(); });
q.addEventListener('input', hold);
q.addEventListener('blur', hold);
q.addEventListener('keydown', (e) => { if (e.key === 'Escape') { q.value = ''; q.blur(); hold(); } });

init();
