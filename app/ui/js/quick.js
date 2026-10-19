// Mini-chat flottant ouvert depuis l'icône NOVA de la barre des menus (macOS).
// Les questions sont traitées par le même cœur que la fenêtre principale ; la conversation
// est enregistrée et peut être rouverte dans NOVA.
import { invoke, listen, Channel, esc, el, errorText } from './api.js';
import { renderMarkdown, wireMarkdown } from './markdown.js';
import { icon } from './icons.js';

const T = window.__TAURI__;
const input = document.getElementById('q');
const thread = document.getElementById('thread');
const foot = document.getElementById('foot');
const shell = document.getElementById('shell');
let settings = {};
let conversationId = null;
let streamingId = null;

async function loadSettings() {
  try {
    const b = await invoke('bootstrap');
    settings = b.settings;
    document.documentElement.dataset.theme = settings.appearance || 'dark';
    const o = await invoke('models_overview');
    const m = o.available.find((x) => x.id === settings.model) || o.available[0];
    document.getElementById('model').textContent = m ? `${m.name} · sur ton ordinateur` : 'Aucun modèle installé — les outils restent disponibles';
    settings.model = m?.id ?? null;
  } catch {}
}

// La fenêtre suit la hauteur du contenu.
new ResizeObserver(() => invoke('quick_resize', { height: Math.ceil(shell.getBoundingClientRect().height) + 2 })).observe(shell);

listen('quick-shown', () => {
  loadSettings();
  T.window.getCurrentWindow().setFocus().catch(() => {});
  setTimeout(() => input.focus(), 20);
  setTimeout(() => input.focus(), 120);
});
window.addEventListener('focus', () => input.focus());

function reset() {
  conversationId = null;
  thread.innerHTML = '';
  thread.classList.add('hidden');
  foot.classList.add('hidden');
  input.value = '';
  input.focus();
}

function scrollDown() {
  thread.scrollTop = thread.scrollHeight;
}

function paint(node, text, status, done, error) {
  node.innerHTML = '';
  if (error) {
    node.appendChild(el(`<div class="error-box">${icon('alert')}<span>${esc(error)}</span></div>`));
    return;
  }
  if (text) {
    const md = el('<div class="md"></div>');
    md.innerHTML = renderMarkdown(text);
    wireMarkdown(md);
    node.appendChild(md);
  }
  if (!done && !text) node.appendChild(el(`<div class="status-line"><span class="typing"><i></i><i></i><i></i></span>${esc(status || '')}</div>`));
}

async function ask(text) {
  thread.classList.remove('hidden');
  foot.classList.remove('hidden');
  thread.appendChild(Object.assign(el('<div class="q-user"></div>'), { textContent: text }));
  const answer = el('<div class="q-ai"></div>');
  thread.appendChild(answer);
  paint(answer, '', 'Réflexion…', false);
  scrollDown();
  document.getElementById('mark').classList.add('spin');

  let reply = '', status = '', frame = 0, finished = false;
  const redraw = () => { frame = 0; if (!finished) { paint(answer, reply, status, false); scrollDown(); } };
  const ch = new Channel();
  ch.onmessage = (ev) => {
    if (ev.type === 'started') { conversationId = ev.conversation_id; streamingId = ev.assistant.id; }
    else if (ev.type === 'status') status = ev.text;
    else if (ev.type === 'delta') reply += ev.text;
    else if (ev.type === 'replace') reply = ev.text;
    else if (ev.type === 'meta') return;
    else if (ev.type === 'done') {
      finished = true;
      if (frame) cancelAnimationFrame(frame);
      streamingId = null;
      const m = ev.message;
      paint(answer, m.content, '', true, m.status === 'failed' ? m.meta?.error || 'La génération a échoué.' : null);
      if (m.meta?.action) answer.appendChild(el(`<div class="faint">Action proposée : ${esc(m.meta.action.title)}. Ouvre la conversation dans NOVA pour l'autoriser.</div>`));
      document.getElementById('mark').classList.remove('spin');
      scrollDown();
      T.event.emit('conversations-changed');
      return;
    }
    if (!frame) frame = requestAnimationFrame(redraw);
  };
  try {
    await invoke('send_message', { conversationId, text, attachments: [], options: { model_id: settings.model ?? null, web: false, think: false }, onEvent: ch });
  } catch (e) {
    paint(answer, '', '', true, errorText(e));
    document.getElementById('mark').classList.remove('spin');
  }
}

document.getElementById('quick').addEventListener('submit', (e) => {
  e.preventDefault();
  const text = input.value.trim();
  if (!text || streamingId) return;
  input.value = '';
  ask(text);
});

document.getElementById('new').onclick = reset;
document.getElementById('open').onclick = () => { if (conversationId) invoke('quick_open_in_main', { conversationId }); };

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') invoke('quick_hide');
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'n') { e.preventDefault(); reset(); }
});

loadSettings();
input.focus();
