// NOVA — interface principale : navigation, conversations, barre de saisie.
import { invoke, Channel, listen, fileSrc, dialog, currentWebview, esc, el, toast, errorText, showMenu, confirmDialog, formatDate } from './api.js';
import { icon } from './icons.js';
import { renderMarkdown, wireMarkdown } from './markdown.js';
import * as Tools from './views/tools.js';
import * as Models from './views/models.js';
import * as Settings from './views/settings.js';
import * as Weather from './views/weather.js';
import * as Files from './views/files.js';
import * as Misc from './views/misc.js';
import * as Code from './views/code.js';
import * as Updater from './updater.js';
import * as I18n from './i18n.js';

export const REPO_URL = 'https://github.com/Nolek0/nova';

export function checkForUpdates(manual = false) {
  return Updater.checkForUpdates(manual, { modal });
}

export const state = {
  settings: {},
  platform: 'macos',
  ramGb: 0,
  conversations: [],
  convId: null,
  messages: [],
  view: 'home',
  draft: '',
  attachments: [],
  processing: 0,
  models: [],
  web: false,
  streamingId: null,
  searchQuery: '',
  searchHits: [],
  dictating: false,
  speakingId: null,
};

const $ = (s) => document.querySelector(s);

// ------------------------------------------------------------------ Démarrage

async function init() {
  const boot = await invoke('bootstrap');
  state.settings = boot.settings;
  state.conversations = boot.conversations;
  state.platform = boot.platform;
  state.ramGb = boot.ram_gb;
  state.memories = boot.memories;
  state.notesDir = boot.notes_dir;
  // Langue de l'interface : réglage, sinon langue du système.
  const code = I18n.resolve(state.settings.language);
  await I18n.setLanguage(code);
  const langName = I18n.UI_LANGUAGES.find((l) => l.code === code)?.name || 'English';
  if (state.settings.uiLanguageName !== langName) setSetting('uiLanguageName', langName);
  document.documentElement.dataset.platform = boot.platform;
  applyTheme();
  $('#toggle-sidebar').innerHTML = icon('panel');
  $('#toggle-sidebar').onclick = () => $('#app').classList.toggle('sidebar-collapsed');
  $('#new-chat').innerHTML = `${icon('pen')}<span>Nouvelle conversation</span><kbd>${isMac() ? '⌘N' : 'Ctrl+N'}</kbd>`;
  $('#new-chat').onclick = () => (state.view === 'code' ? Code.newSession(ctxFor()) : newConversation());
  const search = $('#search input');
  $('#search').insertAdjacentHTML('afterbegin', icon('search'));
  search.oninput = async () => {
    state.searchQuery = search.value;
    state.searchHits = search.value.trim().length >= 2 ? await invoke('search', { query: search.value }) : [];
    renderSidebar();
  };
  $('#status').onclick = () => setView('models');
  document.addEventListener('keydown', onKey);
  setupDragDrop();
  listen('progress', (e) => { Models.onProgress(e.payload); trackDownload(e.payload); });
  // Mini-chat flottant (barre des menus) : garde la liste à jour et permet d'y rouvrir une conversation.
  listen('conversations-changed', () => {
    reloadConversations();
    if (state.convId && state.view === 'chat' && !state.streamingId) openConversation(state.convId);
  });
  listen('open-conversation', (e) => openConversation(e.payload));
  // Île de l'encoche (macOS) : « Ouvrir » ou demande tapée dans l'île.
  listen('island-ask', (e) => (state.settings.novaCode === '1' ? Code.askFromIsland(ctxFor(), e.payload) : null));
  // Menu de l'application (barre des menus).
  listen('menu', (e) => {
    const actions = {
      'settings': () => setView('settings'),
      'models': () => setView('models'),
      'new-chat': () => newConversation(),
      'attach': () => pickFiles(),
      'export': () => setView('settings'),
      'search': () => { $('#app').classList.remove('sidebar-collapsed'); $('#search input').focus(); },
      'toggle-sidebar': () => $('#app').classList.toggle('sidebar-collapsed'),
      'tools': () => setView('tools'),
      'weather': () => setView('weather'),
      'files': () => setView('files'),
      'nova-code': () => openNovaCode(),
      'stop': () => stopGeneration(),
      'toggle-think': () => $('#composer [data-act=think]')?.click(),
      'toggle-web': () => $('#composer [data-act=web]')?.click(),
      'check-updates': () => checkForUpdates(true),
      'help-site': () => invoke('open_url', { url: REPO_URL }),
      'help-issue': () => invoke('open_url', { url: `${REPO_URL}/issues/new` }),
      'help-activity': () => setView('activity'),
    };
    actions[e.payload]?.();
  });
  // Question envoyée depuis la barre rapide (icône NOVA de la barre des menus).
  listen('quick-ask', async (e) => {
    if (state.streamingId) await invoke('cancel_generation', { messageId: state.streamingId });
    newConversation();
    state.draft = e.payload;
    send();
  });
  renderSidebar();
  setView('home');
  updateOnline();
  const overview = await refreshModels();
  warmUp();
  Updater.scheduleAutoCheck(state, modal);
  if (state.settings.onboarded !== '1' && overview) maybeOnboard(overview);
  addEventListener('online', updateOnline);
  addEventListener('offline', updateOnline);
}

export const isMac = () => state.platform === 'macos';

export function applyTheme() {
  document.documentElement.dataset.theme = state.settings.appearance || 'dark';
}

export async function setSetting(key, value) {
  state.settings[key] = value;
  await invoke('set_setting', { key, value: String(value) });
}

/** Charge le modèle choisi en mémoire à l'avance (première réponse immédiate). */
export function warmUp() {
  const m = selectedModel();
  if (m?.provider === 'llama') invoke('warm_up', { modelId: m.id }).catch(() => {});
}

export async function refreshModels() {
  try {
    const o = await invoke('models_overview');
    state.overview = o;
    state.models = o.available;
    renderStatus();
    renderComposerModel();
    return o;
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

function updateOnline() {
  state.online = navigator.onLine;
  renderStatus();
}

function onKey(e) {
  const mod = isMac() ? e.metaKey : e.ctrlKey;
  if (mod && e.key.toLowerCase() === 'n') { e.preventDefault(); newConversation(); }
  if (mod && e.key === ',') { e.preventDefault(); setView('settings'); }
  if (mod && e.key === '.') { e.preventDefault(); stopGeneration(); }
  if (mod && e.key.toLowerCase() === 'o' && e.shiftKey) { e.preventDefault(); pickFiles(); }
  if (e.key === 'Escape') document.querySelector('.modal-backdrop')?.remove();
}

// ------------------------------------------------------------------ Barre latérale

const NAV = [
  ['tools', 'Outils', 'grid'],
  ['weather', 'Météo', 'weather'],
  ['files', 'Fichiers', 'folder'],
  ['library', 'Bibliothèque', 'image'],
  ['gallery', 'Fonctionnalités', 'sparkles'],
  ['models', 'Modèles', 'cpu'],
  ['activity', 'Activité', 'list'],
  ['settings', 'Paramètres', 'settings'],
];

export function renderSidebar() {
  const box = $('#side-scroll');
  box.innerHTML = '';
  if (state.searchQuery.trim()) {
    if (!state.searchHits.length) box.appendChild(el(`<div class="side-empty">${state.searchQuery.trim().length < 2 ? 'Tape au moins 2 caractères.' : 'Aucun résultat.'}</div>`));
    for (const h of state.searchHits) {
      const b = el(`<button class="hit"><b>${esc(h.conversation_title)}</b><small>${esc(h.snippet)}</small></button>`);
      b.onclick = () => openConversation(h.conversation_id);
      box.appendChild(b);
    }
    return;
  }
  if (state.settings.novaCode === '1') {
    const code = state.view === 'code';
    const tabs = el(`<div class="mode-tabs"><button class="${code ? '' : 'on'}">${icon('pen')}<span>Chat</span></button><button class="${code ? 'on' : ''}">${icon('code')}<span>Code</span></button></div>`);
    const [bChat, bCode] = tabs.querySelectorAll('button');
    bChat.onclick = () => { if (code) setView(state.convId ? 'chat' : 'home'); };
    bCode.onclick = () => { if (!code) setView('code'); };
    box.appendChild(tabs);
    if (code) {
      const sessions = state.conversations.filter((c) => c.kind === 'code');
      box.appendChild(el('<div class="side-title">Sessions Nova Code</div>'));
      for (const c of sessions) {
        const proj = (c.project || '').split(/[\\/]/).pop();
        const b = el(`<button class="conv ${Code.currentConversation() === c.id ? 'active' : ''}"><span>${esc(c.title)}</span><small class="faint" translate="no">${esc(proj)}</small></button>`);
        b.onclick = () => Code.openSession(ctxFor(), c);
        b.oncontextmenu = (e) => { e.preventDefault(); showMenu(b, [{ label: 'Renommer…', icon: icon('pencil'), action: () => renameConversation(c) }, 'sep', { label: 'Supprimer…', icon: icon('trash'), danger: true, action: () => deleteConversation(c) }]); };
        box.appendChild(b);
      }
      if (!sessions.length) box.appendChild(el('<div class="side-empty">Tes sessions de code apparaîtront ici.</div>'));
      return;
    }
  }
  for (const [id, label, ic] of NAV) {
    const b = el(`<button class="nav-item ${state.view === id ? 'active' : ''}">${icon(ic)}<span>${label}</span></button>`);
    b.onclick = () => setView(id);
    box.appendChild(b);
  }
  const chats = state.conversations.filter((c) => c.kind !== 'code');
  const pinned = chats.filter((c) => c.pinned);
  const others = chats.filter((c) => !c.pinned);
  if (pinned.length) {
    box.appendChild(el('<div class="side-title">Épinglées</div>'));
    pinned.forEach((c) => box.appendChild(convRow(c)));
  }
  const now = Date.now() / 1000;
  const groups = [['Aujourd’hui', []], ['7 derniers jours', []], ['30 derniers jours', []], ['Plus ancien', []]];
  const today = new Date(); today.setHours(0, 0, 0, 0);
  for (const c of others) {
    const age = now - c.updated_at;
    if (c.updated_at * 1000 >= today.getTime()) groups[0][1].push(c);
    else if (age < 7 * 86400) groups[1][1].push(c);
    else if (age < 30 * 86400) groups[2][1].push(c);
    else groups[3][1].push(c);
  }
  for (const [title, list] of groups) {
    if (!list.length) continue;
    box.appendChild(el(`<div class="side-title">${title}</div>`));
    list.forEach((c) => box.appendChild(convRow(c)));
  }
  if (!chats.length) box.appendChild(el('<div class="side-empty" style="margin-top:10px">Tes conversations apparaîtront ici. Elles restent enregistrées sur cet ordinateur.</div>'));
}

function convRow(c) {
  const b = el(`<button class="conv ${state.convId === c.id && state.view === 'chat' ? 'active' : ''}">${c.pinned ? icon('pin', 'pin') : ''}<span>${esc(c.title)}</span></button>`);
  b.onclick = () => openConversation(c.id);
  b.oncontextmenu = (e) => {
    e.preventDefault();
    showMenu(b, [
      { label: c.pinned ? 'Désépingler' : 'Épingler', icon: icon('pin'), action: async () => { await invoke('toggle_pin', { id: c.id }); reloadConversations(); } },
      { label: 'Renommer…', icon: icon('pencil'), action: () => renameConversation(c) },
      'sep',
      { label: 'Supprimer…', icon: icon('trash'), danger: true, action: () => deleteConversation(c) },
    ]);
  };
  return b;
}

function renameConversation(c) {
  modal(`<h3>Renommer la conversation</h3><input class="field" id="rename" value="${esc(c.title)}"><div class="row" style="justify-content:flex-end"><button class="btn" data-close>Annuler</button><button class="btn primary" id="ok">Renommer</button></div>`, (m) => {
    const input = m.querySelector('#rename');
    input.select();
    const ok = async () => { await invoke('rename_conversation', { id: c.id, title: input.value }); m.remove(); reloadConversations(); };
    m.querySelector('#ok').onclick = ok;
    input.onkeydown = (e) => e.key === 'Enter' && ok();
  });
}

async function deleteConversation(c) {
  if (!(await confirmDialog(`Supprimer « ${c.title} » ? La conversation et ses pièces jointes seront effacées de cet ordinateur.`))) return;
  await invoke('delete_conversation', { id: c.id });
  if (state.convId === c.id) newConversation();
  reloadConversations();
}

export async function reloadConversations() {
  state.conversations = await invoke('list_conversations');
  renderSidebar();
}

function renderStatus() {
  const o = state.overview;
  const box = $('#status');
  const llama = o?.available?.some((m) => m.provider === 'llama');
  const rows = [
    ['IA locale', llama ? 'ok' : o?.engine?.installed ? 'warn' : '', llama ? 'prête' : o?.engine?.installed ? 'aucun modèle' : 'à installer'],
  ];
  if (isMac()) rows.push(['Modèle Apple', o?.apple_ready ? 'ok' : '', o ? (o.apple_ready ? 'prêt' : 'indisponible') : '…']);
  if (o?.available?.some((m) => m.provider === 'ollama')) rows.push(['Ollama', 'ok', 'connecté']);
  rows.push(['Internet', state.online ? 'ok' : '', state.online ? 'connecté' : 'hors ligne']);
  box.innerHTML = rows.map(([l, c, t]) => `<div class="status-row"><span class="dot ${c}"></span>${l}<em>${t}</em></div>`).join('');
}

// ------------------------------------------------------------------ Téléchargements en cours

const downloads = {};

/** Suit chaque téléchargement (moteur ou modèle) et l'affiche en bas de la barre latérale. */
function trackDownload(p) {
  if (p.finished) {
    delete downloads[p.task];
    if (!p.error) {
      toast(`${p.task === 'engine' ? 'Moteur llama.cpp' : p.label} : téléchargement terminé.`);
      refreshModels();
    }
  } else {
    const prev = downloads[p.task];
    const now = performance.now();
    let speed = prev?.speed || 0;
    if (prev && now - prev.t > 900) {
      const inst = ((p.done - prev.done) / (now - prev.t)) * 1000;
      speed = prev.speed ? prev.speed * 0.7 + inst * 0.3 : inst;
      downloads[p.task] = { ...p, t: now, speed };
    } else if (!prev) {
      downloads[p.task] = { ...p, t: now, speed: 0 };
    } else {
      downloads[p.task] = { ...prev, done: p.done, total: p.total, label: p.label };
    }
  }
  renderDownloads();
}

function renderDownloads() {
  const box = $('#downloads');
  const list = Object.values(downloads);
  box.innerHTML = '';
  box.className = list.length ? 'downloads' : '';
  for (const d of list) {
    const pct = d.total ? Math.min(100, (100 * d.done) / d.total) : 0;
    const left = d.speed > 0 && d.total ? (d.total - d.done) / d.speed : null;
    const eta = left === null ? '' : left > 90 ? ` · ~${Math.round(left / 60)} min` : ` · ~${Math.max(1, Math.round(left))} s`;
    const name = d.task === 'engine' ? 'Moteur llama.cpp' : (d.label || '').replace(/ \(.*\)$/, '');
    const row = el(`<div class="dl"><div class="dl-head">${icon('download')}<b>${esc(name)}</b><span class="grow"></span><span class="dl-pct">${pct.toFixed(0)} %</span><button class="icon-btn" title="Annuler le téléchargement">${icon('x')}</button></div>
      <div class="progress"><div style="width:${pct}%"></div></div>
      <div class="faint">${(d.done / 1e9).toFixed(2)} Go sur ${d.total ? (d.total / 1e9).toFixed(2) : '?'} Go${d.speed ? ` · ${(d.speed / 1e6).toFixed(1)} Mo/s` : ''}${eta}</div></div>`);
    row.querySelector('button').onclick = () => invoke('cancel_download', { task: d.task });
    box.appendChild(row);
  }
}

// ------------------------------------------------------------------ Navigation

function ctxFor() {
  return { state, setView, checkForUpdates, currentVersion: Updater.currentVersion, refreshModels, setSetting, applyTheme, quickStart, pickFiles, openConversation, reloadConversations, renderSidebar, isMac, modal, addAttachmentFromPath, selectedModel };
}

function openNovaCode() {
  if (state.settings.novaCode === '1') return setView('code');
  setView('settings');
  toast('Active Nova Code dans les Paramètres (section « Nova Code »).');
}

export function setView(view, opts = {}) {
  state.view = view;
  const v = $('#view');
  v.innerHTML = '';
  v.style.overflow = '';
  v.scrollTop = 0;
  const ctx = ctxFor();
  switch (view) {
    case 'code': Code.render(v, ctx); break;
    case 'home': renderHome(v); break;
    case 'chat': renderChat(v); break;
    case 'tools': Tools.render(v, ctx, opts); break;
    case 'models': Models.render(v, ctx); break;
    case 'settings': Settings.render(v, ctx); break;
    case 'weather': Weather.render(v, ctx, opts); break;
    case 'files': Files.render(v, ctx); break;
    case 'library': Misc.renderLibrary(v, ctx); break;
    case 'gallery': Misc.renderGallery(v, ctx); break;
    case 'activity': Misc.renderActivity(v, ctx); break;
  }
  renderSidebar();
}

export function newConversation() {
  if (state.streamingId) stopGeneration();
  state.convId = null;
  state.messages = [];
  state.draft = '';
  state.attachments = [];
  setView('home');
}

export async function openConversation(id) {
  state.convId = id;
  state.messages = await invoke('get_messages', { conversationId: id });
  setView('chat');
}

// ------------------------------------------------------------------ Accueil

function renderHome(v) {
  const name = state.settings.userName?.trim();
  const greet = (state.settings.greeting || 'Que puis-je faire pour toi ?').trim();
  const line = name ? `${greet.replace(/[ ?!.]+$/, '')}, ${esc(name)} ?` : esc(greet);
  const shortcuts = [
    ['Résoudre une équation', 'Étapes et vérification', 'sigma', 'var(--accent)', () => setView('tools', { tab: 'equations' })],
    ['Résumer un PDF', 'Texte extrait localement', 'filetext', 'var(--violet)', () => { pickFiles(); quickStart('Résume ce document en 5 points clés.'); }],
    ['Consulter la météo', 'Open-Meteo, source affichée', 'weather', 'var(--cyan)', () => setView('weather')],
    ['Rechercher un fichier', 'Dans tes dossiers autorisés', 'search', 'var(--accent)', () => setView('files')],
    ['Écrire du code', 'Fonctions, explications', 'code', 'var(--violet)', () => quickStart('Écris une fonction Python qui ')],
    isMac()
      ? ['Photo avec l’iPhone', 'Caméra de continuité', 'phone', 'var(--cyan)', () => appleCamera()]
      : ['Prendre une photo', 'Avec la webcam', 'camera', 'var(--cyan)', () => webcamCapture()],
    ['Examiner une image', 'Modèle de vision ou OCR', 'scan', 'var(--accent)', () => { pickFiles(true); quickStart('Décris cette image et lis le texte qu’elle contient.'); }],
    isMac()
      ? ['Dicter un message', 'Reconnaissance sur le Mac', 'mic', 'var(--violet)', () => toggleDictation()]
      : ['Convertir des unités', 'Longueurs, masses, °C…', 'arrows', 'var(--violet)', () => setView('tools', { tab: 'units' })],
  ];
  const home = el(`<div class="home"><div class="fade-in" style="display:grid;justify-items:center"><div class="mark big"></div><h1>${line}</h1></div><div id="composer-slot" class="composer-host"></div><div class="shortcuts fade-in"></div><div id="home-banner"></div></div>`);
  v.appendChild(home);
  home.querySelector('#composer-slot').replaceWith(buildComposer('Pose une question, colle une équation, joins un fichier…'));
  const sc = home.querySelector('.shortcuts');
  for (const [t, s, ic, color, fn] of shortcuts) {
    const b = el(`<button class="shortcut"><span class="ic" style="color:${color}">${icon(ic)}</span><span><b>${t}</b><small>${s}</small></span></button>`);
    b.onclick = fn;
    sc.appendChild(b);
  }
  if (state.overview && !state.models.length) {
    home.querySelector('#home-banner').appendChild(el(`<div class="card row" style="max-width:680px"><span>${icon('info')}</span><span class="grow">Aucun modèle d’IA installé pour l’instant. Les outils (calculs, météo, fichiers) fonctionnent déjà.</span><button class="btn primary small">Installer une IA locale gratuite</button></div>`));
    home.querySelector('#home-banner button').onclick = () => setView('models');
  }
  focusComposer();
}

// ------------------------------------------------------------------ Barre de saisie

function buildComposer(placeholder = 'Écris à NOVA…') {
  const c = el(`<div class="composer" id="composer">
      <div class="attachments hidden"></div>
      <textarea rows="1" placeholder="${esc(placeholder)}" aria-label="Message pour NOVA"></textarea>
      <div class="dictating hidden"><i></i><span>Micro actif — dictée locale. Clique sur le micro pour arrêter.</span></div>
      <div class="composer-bar">
        <button class="icon-btn" data-act="attach" title="Joindre des fichiers">${icon('plus')}</button>
        <button class="icon-btn" data-act="camera" title="${isMac() ? 'Photo avec l’iPhone ou la webcam' : 'Prendre une photo avec la webcam'}">${icon('camera')}</button>
        ${isMac() ? `<button class="icon-btn" data-act="mic" title="Dicter (reconnaissance locale)">${icon('mic')}</button>` : ''}
        <button class="icon-btn" data-act="tools" title="Outils">${icon('fx')}</button>
        <button class="web-toggle" data-act="web" title="Ajouter des sources Wikipédia à la réponse (quitte l’ordinateur)">${icon('globe')}<span>Internet</span></button>
        <button class="web-toggle think-toggle" data-act="think" title="Réflexion approfondie : plus lent, pour les questions difficiles. Désactivée = réponse rapide.">${icon('brain')}<span>Réflexion</span></button>
        <span class="spacer"></span>
        <button class="model-btn" data-act="model"></button>
        <button class="send" data-act="send" title="Envoyer (Entrée)">${icon('up')}</button>
      </div></div>`);
  const ta = c.querySelector('textarea');
  ta.value = state.draft;
  ta.oninput = () => { state.draft = ta.value; autosize(ta); renderSend(); };
  ta.onkeydown = (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.altKey && !e.isComposing) { e.preventDefault(); send(); }
  };
  ta.onpaste = async (e) => {
    const items = [...(e.clipboardData?.items || [])].filter((i) => i.type.startsWith('image/'));
    if (!items.length) return;
    e.preventDefault();
    for (const it of items) {
      const file = it.getAsFile();
      const data = await new Promise((r) => { const fr = new FileReader(); fr.onload = () => r(fr.result); fr.readAsDataURL(file); });
      addAttachmentBytes(data, `presse-papiers-${Date.now()}.png`);
    }
  };
  c.querySelector('[data-act=attach]').onclick = (e) => showMenu(e.currentTarget, [
    { label: 'Joindre des fichiers…', icon: icon('file'), action: () => pickFiles() },
    { label: 'Joindre des images…', icon: icon('image'), action: () => pickFiles(true) },
  ], { above: true });
  c.querySelector('[data-act=camera]').onclick = (e) => {
    const items = [];
    if (isMac()) items.push({ label: 'Photo avec l’iPhone (Caméra de continuité)', icon: icon('phone'), action: appleCamera });
    items.push({ label: 'Photo avec la webcam', icon: icon('video'), action: webcamCapture });
    items.push('sep', { label: 'Choisir une image existante…', icon: icon('image'), action: () => pickFiles(true) });
    showMenu(e.currentTarget, items, { above: true });
  };
  c.querySelector('[data-act=mic]')?.addEventListener('click', toggleDictation);
  c.querySelector('[data-act=tools]').onclick = (e) => showMenu(e.currentTarget, [
    { header: 'Insérer un exemple' },
    { label: 'Résoudre une équation', action: () => quickStart('résous x^2 - 5x + 6 = 0') },
    { label: 'Système d’équations', action: () => quickStart('2x + y = 5; x - y = 1') },
    { label: 'Dérivée', action: () => quickStart('dérivée de x^2 sin(x)') },
    { label: 'Conversion', action: () => quickStart('100 °F en °C') },
    { label: 'Météo', action: () => quickStart('météo à Paris') },
    { label: 'Chercher un fichier', action: () => quickStart('cherche le fichier ') },
    { label: 'Ouvrir une application', action: () => quickStart('ouvre ') },
    { label: 'Créer une note', action: () => quickStart('crée une note : ') },
    'sep',
    { label: 'Ouvrir la boîte à outils', icon: icon('grid'), action: () => setView('tools') },
  ], { above: true });
  const web = c.querySelector('[data-act=web]');
  web.classList.toggle('on', state.web);
  web.onclick = () => { state.web = !state.web; web.classList.toggle('on', state.web); };
  const think = c.querySelector('[data-act=think]');
  think.classList.toggle('on', state.settings.thinking === '1');
  think.onclick = () => {
    const on = state.settings.thinking !== '1';
    setSetting('thinking', on ? '1' : '0');
    think.classList.toggle('on', on);
    toast(on ? 'Réflexion approfondie activée : réponses plus réfléchies mais plus lentes.' : 'Réponse rapide activée.');
  };
  c.querySelector('[data-act=model]').onclick = (e) => modelMenu(e.currentTarget);
  c.querySelector('[data-act=send]').onclick = () => (state.streamingId ? stopGeneration() : send());
  setTimeout(() => { autosize(ta); renderAttachments(); renderSend(); renderComposerModel(); renderDictation(); }, 0);
  return c;
}

function autosize(ta) {
  ta.style.height = 'auto';
  ta.style.height = Math.min(220, ta.scrollHeight) + 'px';
}

function focusComposer() {
  setTimeout(() => $('#composer textarea')?.focus(), 30);
}

export function quickStart(text) {
  state.draft = text;
  const ta = $('#composer textarea');
  if (ta) { ta.value = text; autosize(ta); ta.focus(); ta.setSelectionRange(text.length, text.length); renderSend(); }
  else if (state.view !== 'home' && state.view !== 'chat') { setView(state.convId ? 'chat' : 'home'); }
}

function canSend() {
  return (state.draft.trim() || state.attachments.length) && !state.processing && !state.streamingId;
}

function renderSend() {
  const b = $('#composer [data-act=send]');
  if (!b) return;
  if (state.streamingId) { b.className = 'send stop'; b.innerHTML = icon('stop'); b.disabled = false; b.title = 'Arrêter la génération'; }
  else { b.className = 'send'; b.innerHTML = icon('up'); b.disabled = !canSend(); b.title = 'Envoyer (Entrée)'; }
}

function selectedModel() {
  return state.models.find((m) => m.id === state.settings.model) || state.models[0];
}

function renderComposerModel() {
  const b = $('#composer [data-act=model]');
  if (!b) return;
  const m = selectedModel();
  b.innerHTML = m ? `${icon(m.local ? 'shield' : 'cloud', 'lock')}<span>${esc(m.name)}</span>${icon('down')}` : `<span>Aucun modèle</span>${icon('down')}`;
  b.title = m ? (m.local ? 'Traitement local : rien ne quitte ton ordinateur' : 'Modèle distant') : 'Installe un modèle dans la page Modèles';
}

function modelMenu(anchor) {
  const cur = selectedModel();
  const items = state.models.length ? [{ header: 'Modèles disponibles' }] : [{ header: 'Aucun modèle installé' }];
  for (const m of state.models) {
    items.push({ label: `${m.name}${m.vision ? ' · vision' : ''}`, check: cur?.id === m.id, action: async () => { await setSetting('model', m.id); renderComposerModel(); warmUp(); } });
  }
  items.push('sep', { label: 'Gérer les modèles…', icon: icon('cpu'), action: () => setView('models') }, { label: 'Rafraîchir', icon: icon('refresh'), action: refreshModels });
  showMenu(anchor, items, { align: 'right', above: true });
}

// ------------------------------------------------------------------ Pièces jointes

export async function pickFiles(imagesOnly = false) {
  const res = await dialog.open({
    multiple: true,
    directory: false,
    title: imagesOnly ? 'Choisis une ou plusieurs images' : 'Choisis les fichiers à joindre',
    filters: imagesOnly ? [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'heic', 'bmp', 'tiff'] }] : undefined,
  });
  if (!res) return;
  for (const p of [].concat(res)) addAttachmentFromPath(typeof p === 'string' ? p : p.path);
}

export async function addAttachmentFromPath(path) {
  if (state.view !== 'home' && state.view !== 'chat') setView(state.convId ? 'chat' : 'home');
  const placeholder = { id: 'tmp-' + Math.random(), kind: 'other', filename: path.split(/[\\/]/).pop(), path: '', processing: true };
  state.attachments.push(placeholder);
  state.processing++;
  renderAttachments(); renderSend();
  try {
    const att = await invoke('import_attachment', { path });
    Object.assign(placeholder, att, { processing: false });
  } catch (e) {
    state.attachments = state.attachments.filter((a) => a !== placeholder);
    toast(errorText(e), 'error');
  }
  state.processing--;
  renderAttachments(); renderSend();
}

async function addAttachmentBytes(dataUrl, name) {
  if (state.view !== 'home' && state.view !== 'chat') setView(state.convId ? 'chat' : 'home');
  state.processing++;
  renderSend();
  try {
    state.attachments.push(await invoke('import_bytes', { dataBase64: dataUrl, name }));
  } catch (e) { toast(errorText(e), 'error'); }
  state.processing--;
  renderAttachments(); renderSend();
}

function attachmentChip(a, removable) {
  const thumb = a.kind === 'image' && a.path ? `<img class="thumb" src="${fileSrc(a.path)}" alt="">` : `<span class="thumb">${icon(a.kind === 'pdf' ? 'filetext' : 'file')}</span>`;
  const detail = a.processing ? 'Lecture locale…' : a.kind === 'image' ? (a.extracted_text ? 'Image · texte détecté' : 'Image') : a.kind === 'pdf' ? (a.extracted_text ? 'PDF · texte extrait' : 'PDF · aucun texte') : a.kind === 'text' ? 'Fichier texte' : 'Fichier';
  const chip = el(`<div class="att" title="${esc(a.filename)}">${thumb}<span><b>${esc(a.filename)}</b><small>${detail}</small></span>${removable ? `<button class="x" title="Retirer">${icon('x')}</button>` : ''}</div>`);
  chip.querySelector('.x')?.addEventListener('click', () => { state.attachments = state.attachments.filter((x) => x !== a); renderAttachments(); renderSend(); });
  if (!removable && a.path) chip.ondblclick = () => invoke('open_path', { path: a.path, reveal: false });
  return chip;
}

function renderAttachments() {
  const box = $('#composer .attachments');
  if (!box) return;
  box.innerHTML = '';
  box.classList.toggle('hidden', !state.attachments.length);
  state.attachments.forEach((a) => box.appendChild(attachmentChip(a, true)));
}

function setupDragDrop() {
  let overlay;
  currentWebview().onDragDropEvent((e) => {
    const t = e.payload.type;
    if (t === 'enter' || t === 'over') {
      if (!overlay) { overlay = el('<div class="drop-overlay">Dépose tes fichiers ici</div>'); $('#main').appendChild(overlay); }
    } else {
      overlay?.remove(); overlay = null;
      if (t === 'drop') (e.payload.paths || []).forEach(addAttachmentFromPath);
    }
  });
}

async function appleCamera() {
  toast('Fenêtre NOVA ouverte : choisis « Prendre une photo » sous le nom de ton iPhone.');
  try {
    const att = await invoke('apple_camera');
    if (att) {
      if (state.view !== 'home' && state.view !== 'chat') setView(state.convId ? 'chat' : 'home');
      state.attachments.push(att);
      renderAttachments(); renderSend();
      if (!state.draft) quickStart('Que vois-tu sur cette photo ?');
    }
  } catch (e) { toast(errorText(e), 'error'); }
}

async function webcamCapture() {
  let stream;
  try {
    stream = await navigator.mediaDevices.getUserMedia({ video: { width: 1920, height: 1080 } });
  } catch {
    toast('Caméra inaccessible. Autorise NOVA dans les réglages de confidentialité de ton système.', 'error');
    return;
  }
  modal(`<h3>Prendre une photo</h3><video autoplay playsinline></video><img class="preview hidden" alt=""><div class="row" style="justify-content:flex-end"><button class="btn" data-close>Annuler</button><button class="btn hidden" id="retake">Reprendre</button><button class="btn primary" id="shoot">${icon('camera')} Prendre la photo</button><button class="btn primary hidden" id="use">Utiliser la photo</button></div>`, (m) => {
    const video = m.querySelector('video'), img = m.querySelector('img');
    video.srcObject = stream;
    let data;
    m.querySelector('#shoot').onclick = () => {
      const canvas = document.createElement('canvas');
      canvas.width = video.videoWidth; canvas.height = video.videoHeight;
      canvas.getContext('2d').drawImage(video, 0, 0);
      data = canvas.toDataURL('image/jpeg', 0.9);
      img.src = data;
      video.classList.add('hidden'); img.classList.remove('hidden');
      ['#shoot'].forEach((s) => m.querySelector(s).classList.add('hidden'));
      ['#retake', '#use'].forEach((s) => m.querySelector(s).classList.remove('hidden'));
    };
    m.querySelector('#retake').onclick = () => {
      video.classList.remove('hidden'); img.classList.add('hidden');
      m.querySelector('#shoot').classList.remove('hidden');
      ['#retake', '#use'].forEach((s) => m.querySelector(s).classList.add('hidden'));
    };
    m.querySelector('#use').onclick = () => { m.remove(); addAttachmentBytes(data, `photo-${Date.now()}.jpg`); };
  }, () => stream.getTracks().forEach((t) => t.stop()));
}

// ------------------------------------------------------------------ Dictée (Mac)

async function toggleDictation() {
  if (state.dictating) { await invoke('dictation_stop'); return; }
  if (state.view !== 'home' && state.view !== 'chat') setView(state.convId ? 'chat' : 'home');
  const base = state.draft ? state.draft + ' ' : '';
  const ch = new Channel();
  ch.onmessage = (ev) => {
    if (ev.listening) { state.dictating = true; renderDictation(); }
    if (ev.partial !== undefined || ev.final !== undefined) {
      state.draft = base + (ev.final ?? ev.partial);
      const ta = $('#composer textarea');
      if (ta) { ta.value = state.draft; autosize(ta); renderSend(); }
    }
    if (ev.error) toast(ev.error, 'error');
    if (ev.ended) { state.dictating = false; renderDictation(); }
  };
  invoke('dictation_start', { onEvent: ch }).catch((e) => { toast(errorText(e), 'error'); state.dictating = false; renderDictation(); });
}

function renderDictation() {
  $('#composer .dictating')?.classList.toggle('hidden', !state.dictating);
  const b = $('#composer [data-act=mic]');
  if (b) { b.classList.toggle('rec', state.dictating); b.title = state.dictating ? 'Arrêter la dictée' : 'Dicter (reconnaissance locale)'; }
}

// ------------------------------------------------------------------ Conversation

function renderChat(v) {
  const chat = el(`<div class="chat"><div class="messages"><div class="messages-inner"></div></div><div id="composer-slot"></div><div class="chat-footer"></div></div>`);
  v.appendChild(chat);
  v.style.overflow = 'hidden';
  chat.querySelector('#composer-slot').replaceWith(buildComposer());
  const inner = chat.querySelector('.messages-inner');
  state.messages.forEach((m) => inner.appendChild(messageNode(m)));
  markLast();
  updateFooter();
  scrollBottom(true);
  focusComposer();
}

function updateFooter() {
  const f = $('.chat-footer');
  if (!f) return;
  const m = selectedModel();
  f.textContent = m
    ? `${m.name} fonctionne sur ton ordinateur : rien ne quitte la machine${state.web ? ', sauf la recherche Wikipédia (mode Internet)' : ''}. NOVA peut se tromper ; vérifie les informations importantes.`
    : 'Aucun modèle installé : les outils locaux restent utilisables.';
}

function scrollBottom(force = false) {
  const box = $('.messages');
  if (!box) return;
  const near = box.scrollHeight - box.scrollTop - box.clientHeight < 160;
  if (force || near) box.scrollTop = box.scrollHeight;
}

function markLast() {
  document.querySelectorAll('.msg-ai').forEach((n) => n.classList.remove('last'));
  const last = [...document.querySelectorAll('.msg-ai')].pop();
  last?.classList.add('last');
}

function messageNode(m) {
  if (m.role === 'user') {
    const n = el(`<div class="msg-user" data-id="${m.id}"><div class="row atts"></div><div class="bubble"></div><div class="msg-actions"></div></div>`);
    n.querySelector('.bubble').textContent = m.content;
    m.attachments.forEach((a) => n.querySelector('.atts').appendChild(attachmentChip(a, false)));
    const actions = n.querySelector('.msg-actions');
    actions.append(actionBtn('Copier', 'copy', () => copyText(m.content)), actionBtn('Modifier et renvoyer', 'pencil', () => editMessage(n, m)));
    return n;
  }
  const n = el(`<div class="msg-ai" data-id="${m.id}"><div class="mark"></div><div class="body" style="display:grid;gap:10px;min-width:0"></div></div>`);
  fillAssistant(n, m);
  return n;
}

function actionBtn(title, ic, fn) {
  const b = el(`<button class="icon-btn" title="${esc(title)}" aria-label="${esc(title)}">${icon(ic)}</button>`);
  b.onclick = fn;
  return b;
}

async function copyText(t) {
  await navigator.clipboard.writeText(t);
  toast('Copié dans le presse-papiers.');
}

function fillAssistant(n, m, live = {}) {
  const body = n.querySelector('.body');
  const streaming = state.streamingId === m.id;
  n.querySelector('.mark').classList.toggle('spin', streaming);
  body.innerHTML = '';
  const reasoning = live.reasoning ?? m.meta?.reasoning;
  if (reasoning) {
    // La réflexion du modèle reste repliée : seule une ligne discrète est visible.
    const thinking = streaming && !m.content;
    const box = el(`<div class="reasoning"><button class="reasoning-toggle">${thinking ? '<span class="typing"><i></i><i></i><i></i></span> Réflexion en cours…' : `${icon('brain')} Voir la réflexion`}</button><div class="reasoning-text hidden"></div></div>`);
    box.querySelector('.reasoning-text').textContent = reasoning;
    if (!thinking) {
      box.querySelector('button').onclick = () => {
        const t = box.querySelector('.reasoning-text');
        t.classList.toggle('hidden');
        box.querySelector('button').innerHTML = `${icon('brain')} ${t.classList.contains('hidden') ? 'Voir la réflexion' : 'Masquer la réflexion'}`;
      };
    }
    body.appendChild(box);
  }
  if (live.status) body.appendChild(el(`<div class="status-line"><span class="typing"><i></i><i></i><i></i></span>${esc(live.status)}</div>`));
  if (m.content) {
    const md = el('<div class="md"></div>');
    md.innerHTML = renderMarkdown(m.content);
    wireMarkdown(md);
    body.appendChild(md);
  } else if (streaming && !live.status && !reasoning) {
    body.appendChild(el('<div class="typing"><i></i><i></i><i></i></div>'));
  }
  if (m.meta?.files?.length) body.appendChild(fileList(m.meta.files));
  if (m.meta?.action) body.appendChild(actionCard(m));
  if (m.meta?.sources?.length) {
    const s = el('<div class="sources"></div>');
    m.meta.sources.forEach((src, i) => {
      const a = el(`<a href="${esc(src.url)}">${icon('external')} [${i + 1}] ${esc(src.title)}</a>`);
      a.onclick = (e) => { e.preventDefault(); invoke('open_url', { url: src.url }); };
      s.appendChild(a);
    });
    body.appendChild(s);
  }
  if (m.status === 'failed') {
    const box = el(`<div class="error-box">${icon('alert')}<div style="display:grid;gap:8px"><span></span><div class="row"><button class="btn primary small">Réessayer</button></div></div></div>`);
    box.querySelector('span').textContent = m.meta?.error || 'La génération a échoué.';
    box.querySelector('button').onclick = () => regenerate(m);
    if (/pas téléchargé|pas installé|Aucun modèle/.test(m.meta?.error || '')) {
      const go = el('<button class="btn small">Ouvrir la page Modèles</button>');
      go.onclick = () => setView('models');
      box.querySelector('.row').appendChild(go);
    }
    body.appendChild(box);
  } else if (m.status === 'interrupted') {
    body.appendChild(el(`<div class="warn-line">${icon('alert')} Réponse interrompue</div>`));
  }
  if (!streaming) {
    const actions = el('<div class="msg-actions"></div>');
    if (m.content) {
      actions.append(actionBtn('Copier la réponse', 'copy', () => copyText(m.content)));
      actions.append(actionBtn(state.speakingId === m.id ? 'Arrêter la lecture' : 'Lire à voix haute', state.speakingId === m.id ? 'stop' : 'volume', () => speak(m)));
    }
    actions.append(actionBtn('Régénérer', 'refresh', () => regenerate(m)));
    if (m.meta?.tool) actions.append(el(`<span class="badge cyan">${icon('fx')} ${esc(m.meta.tool)}</span>`));
    else if (m.meta?.model) actions.append(el(`<span class="badge">${icon('shield')} ${esc(m.meta.model)}</span>`));
    if (m.meta?.left_device) actions.append(el(`<span class="badge warn" title="Une requête a été envoyée sur Internet">${icon('globe')} Internet</span>`));
    body.appendChild(actions);
  }
}

function fileList(files) {
  const box = el('<div class="file-list card" style="padding:6px"></div>');
  for (const f of files.slice(0, 30)) {
    const row = el(`<button class="file-row">${icon(f.is_dir ? 'folder' : 'file')}<span style="min-width:0;flex:1"><div class="name">${esc(f.name)}</div><div class="where">${esc(f.folder)}</div>${f.snippet ? `<div class="snip">${esc(f.snippet)}</div>` : ''}</span><span class="icon-btn" title="Afficher dans le dossier">${icon('folder')}</span></button>`);
    row.onclick = (e) => {
      const reveal = e.target.closest('.icon-btn') !== null;
      invoke('open_path', { path: f.path, reveal }).catch((err) => toast(errorText(err), 'error'));
    };
    box.appendChild(row);
  }
  return box;
}

function actionCard(m) {
  const a = m.meta.action;
  const statusText = { done: '✓ Fait', refused: 'Refusée', failed: 'Échec' }[a.status];
  const card = el(`<div class="action-card"><div class="row"><span class="badge warn">${icon('shield')} Confirmation requise</span><b>${esc(a.title)}</b></div><pre></pre><div class="row act"></div></div>`);
  card.querySelector('pre').textContent = a.detail;
  const row = card.querySelector('.act');
  if (a.status === 'pending') {
    const yes = el('<button class="btn primary small">Autoriser</button>');
    const no = el('<button class="btn small">Refuser</button>');
    const decide = async (approve) => {
      try {
        const updated = await invoke('decide_action', { messageId: m.id, approve });
        const i = state.messages.findIndex((x) => x.id === m.id);
        state.messages[i] = updated;
        const node = document.querySelector(`.msg-ai[data-id="${m.id}"]`);
        if (node) fillAssistant(node, updated);
      } catch (e) { toast(errorText(e), 'error'); }
    };
    yes.onclick = () => decide(true);
    no.onclick = () => decide(false);
    row.append(yes, no);
  } else {
    row.appendChild(el(`<span class="${a.status === 'done' ? 'muted' : 'warn-line'}">${statusText}${a.result ? ' — ' + esc(a.result) : ''}</span>`));
  }
  return card;
}

function speak(m) {
  if (state.speakingId === m.id) { speechSynthesis.cancel(); state.speakingId = null; refreshMessage(m); return; }
  speechSynthesis.cancel();
  const text = m.content.replace(/```[\s\S]*?```/g, ' (bloc de code) ').replace(/<[^>]+>/g, '').replace(/[*_`#>|]/g, '');
  const u = new SpeechSynthesisUtterance(text);
  u.lang = 'fr-FR';
  const voices = speechSynthesis.getVoices().filter((v) => v.lang.startsWith('fr'));
  u.voice = voices.find((v) => v.name === state.settings.voice) || voices[0] || null;
  u.rate = Number(state.settings.voiceRate || 1);
  u.onend = u.onerror = () => { state.speakingId = null; refreshMessage(m); };
  state.speakingId = m.id;
  speechSynthesis.speak(u);
  refreshMessage(m);
}

function refreshMessage(m) {
  const node = document.querySelector(`.msg-ai[data-id="${m.id}"]`);
  const fresh = state.messages.find((x) => x.id === m.id) || m;
  if (node) fillAssistant(node, fresh);
}

function editMessage(node, m) {
  if (state.streamingId) return;
  const bubble = node.querySelector('.bubble');
  const box = el(`<div style="display:grid;gap:8px;width:min(560px,100%)"><textarea class="field" rows="3"></textarea><div class="row" style="justify-content:flex-end"><button class="btn small">Annuler</button><button class="btn primary small">Renvoyer</button></div></div>`);
  const ta = box.querySelector('textarea');
  ta.value = m.content;
  bubble.replaceWith(box);
  ta.focus();
  box.querySelector('.btn').onclick = () => openConversation(state.convId);
  box.querySelector('.btn.primary').onclick = () => runGeneration('regenerate', { messageId: m.id, newText: ta.value }, m.id);
}

// ------------------------------------------------------------------ Envoi et flux

async function send() {
  if (!canSend()) return;
  const text = state.draft;
  const attachments = state.attachments.map(({ processing, ...a }) => a);
  state.draft = '';
  state.attachments = [];
  const ta = $('#composer textarea');
  if (ta) { ta.value = ''; autosize(ta); }
  renderAttachments();
  await runGeneration('send_message', { conversationId: state.convId, text, attachments });
}

function regenerate(m) {
  if (state.streamingId) return;
  runGeneration('regenerate', { messageId: m.id, newText: null }, m.id);
}

/** Lance une génération et met l'interface à jour au fil des événements reçus du cœur Rust. */
async function runGeneration(command, args, replaceFromId = null) {
  const ch = new Channel();
  let reply = null;
  const live = { text: '', reasoning: '', status: '' };
  let frame = 0;
  const paint = () => {
    frame = 0;
    if (!reply) return;
    reply.content = live.text;
    const node = document.querySelector(`.msg-ai[data-id="${reply.id}"]`);
    if (node) fillAssistant(node, reply, { reasoning: live.reasoning || undefined, status: live.status || undefined });
    scrollBottom();
  };
  const schedule = () => { if (!frame) frame = requestAnimationFrame(paint); };

  const handle = async (ev) => {
    switch (ev.type) {
      case 'started': {
        const firstMessage = !state.convId;
        state.convId = ev.conversation_id;
        if (replaceFromId) {
          // Régénération ou modification : le cœur a déjà réécrit l'historique, on le relit.
          state.messages = (await invoke('get_messages', { conversationId: ev.conversation_id })).filter((x) => x.id !== ev.assistant.id);
        } else {
          state.messages.push(ev.user);
        }
        reply = ev.assistant;
        state.messages.push(reply);
        state.streamingId = reply.id;
        if (state.view !== 'chat' || firstMessage || replaceFromId) setView('chat');
        else {
          const inner = $('.messages-inner');
          inner.appendChild(messageNode(ev.user));
          inner.appendChild(messageNode(reply));
          markLast();
          scrollBottom(true);
        }
        renderSend();
        reloadConversations();
        break;
      }
      case 'status': live.status = ev.text; schedule(); break;
      case 'delta': live.text += ev.text; live.status = ''; schedule(); break;
      case 'reasoning': live.reasoning += ev.text; schedule(); break;
      case 'replace': live.text = ev.text; schedule(); break;
      case 'meta': if (reply) reply.meta = ev.meta; break;
      case 'done': {
        state.streamingId = null;
        const i = state.messages.findIndex((x) => x.id === ev.message.id);
        if (i >= 0) state.messages[i] = ev.message;
        const node = document.querySelector(`.msg-ai[data-id="${ev.message.id}"]`);
        if (node) fillAssistant(node, ev.message);
        markLast();
        renderSend();
        scrollBottom();
        reloadConversations();
        break;
      }
    }
  };
  // Les événements sont traités dans l'ordre, même quand l'un d'eux attend une lecture.
  let chain = Promise.resolve();
  ch.onmessage = (ev) => { chain = chain.then(() => handle(ev)).catch((e) => console.error(e)); };
  try {
    await invoke(command, { ...args, options: { model_id: selectedModel()?.id ?? null, web: state.web, think: state.settings.thinking === '1' }, onEvent: ch });
  } catch (e) {
    state.streamingId = null;
    renderSend();
    toast(errorText(e), 'error');
  }
}

function stopGeneration() {
  if (state.streamingId) invoke('cancel_generation', { messageId: state.streamingId });
}

// ------------------------------------------------------------------ Assistant de premier lancement

/** Propose, au premier lancement, le meilleur modèle gratuit pour la mémoire de la machine. */
function maybeOnboard(o) {
  if (o.available.some((m) => m.provider === 'llama')) { setSetting('onboarded', '1'); return; }
  const rec = o.catalog.find((m) => m.id === o.recommended);
  if (!rec) return;
  const total = (rec.size_gb + (o.engine.installed ? 0 : 0.03)).toFixed(1);
  let unlisten = null;
  modal(`<div class="row"><span class="mark big"></span><div><h3 style="font-size:18px">Bienvenue dans NOVA</h3><div class="muted" style="font-size:13px">Ton assistant personnel, gratuit, qui fonctionne sur ton ordinateur.</div></div></div>
    <div class="muted" style="font-size:13px">Ton ordinateur a <b>${o.ram_gb} Go</b> de mémoire. Pour cette machine, je te conseille :</div>
    <div class="card model-card"><div class="top"><b>${esc(rec.name)}</b><span class="faint">${esc(rec.publisher)}</span><span class="grow"></span>${rec.mmproj ? `<span class="badge cyan">${icon('eye')} vision</span>` : ''}</div>
      <div class="muted" style="font-size:12.5px">${esc(rec.description)}</div>
      <div class="kv"><div><small>Téléchargement</small><span>${total} Go</span></div><div><small>Licence</small><span>Apache 2.0</span></div><div><small>Points forts</small><span>${esc(rec.strengths)}</span></div></div></div>
    <div class="faint">Le moteur llama.cpp${o.engine.installed ? ' (déjà installé)' : ' (≈ 12 Mo)'} et le modèle viennent de leurs sources officielles (GitHub, Hugging Face). Ensuite, NOVA fonctionne sans Internet. Tu pourras changer de modèle dans la page Modèles.</div>
    <div id="ob-progress" class="hidden" style="display:grid;gap:6px"><div class="faint" id="ob-label">Préparation…</div><div class="progress"><div></div></div></div>
    <div class="row" style="justify-content:flex-end" id="ob-actions"><button class="btn" data-close>Plus tard</button><button class="btn primary" id="ob-install">${icon('download')} Installer (${total} Go)</button></div>`,
  (m) => {
    m.querySelector('#ob-install').onclick = async () => {
      setSetting('onboarded', '1');
      const box = m.querySelector('#ob-progress');
      box.classList.remove('hidden');
      m.querySelector('#ob-actions').innerHTML = '<button class="btn" data-close>Continuer en arrière-plan</button>';
      m.querySelector('[data-close]').onclick = () => m.remove();
      unlisten = await listen('progress', (e) => {
        const p = e.payload;
        if (p.finished) return;
        m.querySelector('#ob-label').textContent = `${p.task === 'engine' ? 'Moteur llama.cpp' : rec.name} — ${formatBytesShort(p.done)} sur ${formatBytesShort(p.total)}`;
        m.querySelector('.progress div').style.width = (p.total ? (100 * p.done) / p.total : 0) + '%';
      });
      try {
        if (!o.engine.installed) await invoke('install_engine');
        await invoke('download_model', { id: rec.id });
        await setSetting('model', `llama:${rec.id}`);
        await refreshModels();
        m.querySelector('#ob-label').textContent = `${rec.name} est prêt. Tout fonctionne maintenant sur ton ordinateur.`;
        m.querySelector('.progress div').style.width = '100%';
        m.querySelector('#ob-actions').innerHTML = '<button class="btn primary" data-close>Commencer</button>';
        m.querySelector('[data-close]').onclick = () => m.remove();
        if (state.view === 'home') setView('home');
      } catch (e) {
        m.querySelector('#ob-label').textContent = 'Échec : ' + errorText(e) + ' — tu peux réessayer depuis la page Modèles.';
      }
    };
    m.querySelector('[data-close]').addEventListener('click', () => setSetting('onboarded', '1'));
  },
  () => unlisten?.());
}

function formatBytesShort(n) {
  if (!n) return '…';
  return n >= 1e9 ? (n / 1e9).toFixed(2) + ' Go' : Math.round(n / 1e6) + ' Mo';
}

// ------------------------------------------------------------------ Fenêtre modale

export function modal(html, setup, onClose) {
  const back = el(`<div class="modal-backdrop"><div class="modal" role="dialog">${html}</div></div>`);
  const close = () => back.remove();
  back.addEventListener('mousedown', (e) => { if (e.target === back) close(); });
  back.querySelectorAll('[data-close]').forEach((b) => (b.onclick = close));
  const observer = new MutationObserver(() => { if (!document.body.contains(back)) { observer.disconnect(); onClose?.(); onClose = null; } });
  document.body.appendChild(back);
  observer.observe(document.body, { childList: true });
  const m = back.querySelector('.modal');
  m.remove = () => back.remove();
  setup?.(m);
  return m;
}

init().catch((e) => {
  document.body.innerHTML = `<div style="padding:40px;font-family:system-ui;color:#ecedf0">Erreur au démarrage de NOVA : ${esc(errorText(e))}</div>`;
});
