// Nova Code : espace de programmation (projet, arborescence, fichier ouvert, assistant).
// L'IA propose des fichiers complets ; rien n'est écrit sans un clic sur « Appliquer » après avoir vu les différences.
import { invoke, Channel, dialog, esc, el, toast, errorText } from '../api.js';
import { icon } from '../icons.js';
import { renderMarkdown, wireMarkdown } from '../markdown.js';

let pendingAsk = null;

/** Envoie l'état à l'île de l'encoche (macOS), si elle est ouverte. */
function island(status) {
  const name = S.project ? S.project.split(/[\\/]/).pop() : '';
  window.__TAURI__.event.emitTo('island', 'island-status', { project: name, ...status }).catch(() => {});
}

/** Demande reçue depuis l'île : ouvre Nova Code et l'envoie. */
export function askFromIsland(ctx, text) {
  pendingAsk = text || null;
  ctx.setView('code');
}

const S = { project: null, tree: [], open: null, openText: '', convId: null, messages: [], streaming: null, collapsed: new Set() };

export function currentConversation() {
  return S.convId;
}

/** Ouvre une session Nova Code existante (depuis la barre latérale). */
export async function openSession(ctx, conv) {
  S.convId = conv.id;
  if (conv.project && conv.project !== S.project) await loadProject(ctx, conv.project, false);
  S.messages = await invoke('get_messages', { conversationId: conv.id });
  ctx.setView('code');
}

export function newSession(ctx) {
  S.convId = null;
  S.messages = [];
  ctx.setView('code');
}

async function loadProject(ctx, path, reset = true) {
  S.tree = await invoke('code_open_project', { path });
  S.project = path;
  S.open = null;
  S.openText = '';
  if (reset) { S.convId = null; S.messages = []; }
  S.collapsed = new Set(S.tree.filter((p) => p.endsWith('/') && p.split('/').length > 2));
  await ctx.setSetting('codeProject', path);
  island({ busy: false });
}

export async function render(v, ctx) {
  v.style.overflow = 'hidden';
  if (!S.project && ctx.state.settings.codeProject) {
    try { await loadProject(ctx, ctx.state.settings.codeProject, false); } catch { S.project = null; }
  }
  const root = el(`<div class="code-view">
    <div class="code-top"><span class="code-badge">${icon('code')} Nova Code</span><b class="code-proj" translate="no"></b><span class="grow"></span>
      <button class="btn small" id="c-new">${icon('pen')} Nouvelle session</button><button class="btn small" id="c-open">${icon('folder')} Ouvrir un projet…</button></div>
    <div class="code-body">
      <div class="code-tree" id="c-tree"></div>
      <div class="code-file" id="c-file"></div>
      <div class="code-chat"><div class="code-msgs" id="c-msgs"></div>
        <form class="code-input" id="c-form"><textarea rows="3" placeholder="Demande une modification, une explication… (@fichier pour l'inclure)"></textarea>
          <div class="row"><span class="faint" id="c-hint"></span><span class="grow"></span><button class="btn primary small" id="c-send">${icon('up')} Envoyer</button></div></form></div>
    </div></div>`);
  v.appendChild(root);
  root.querySelector('#c-open').onclick = async () => {
    const path = await dialog.open({ directory: true, title: 'Choisir le dossier du projet' });
    if (!path) return;
    try { await loadProject(ctx, path); ctx.setView('code'); ctx.renderSidebar(); } catch (e) { toast(errorText(e), 'error'); }
  };
  root.querySelector('#c-new').onclick = () => newSession(ctx);
  root.querySelector('.code-proj').textContent = S.project ? S.project.split(/[\\/]/).pop() : '';
  if (!S.project) {
    root.querySelector('.code-body').innerHTML = `<div class="code-empty"><div class="mark big"></div><h2>Nova Code</h2>
      <p class="muted">Ouvre le dossier d'un projet : Nova Code lit ses fichiers, explique le code et propose des modifications. Rien n'est modifié sans ton accord, et une copie de chaque fichier est gardée avant d'écrire.</p>
      <button class="btn primary" id="c-open2">${icon('folder')} Ouvrir un projet…</button></div>`;
    root.querySelector('#c-open2').onclick = () => root.querySelector('#c-open').click();
    return;
  }
  renderTree(root, ctx);
  renderFile(root);
  renderMessages(root);
  const ta = root.querySelector('textarea');
  const form = root.querySelector('#c-form');
  ta.onkeydown = (e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); form.requestSubmit(); } };
  form.onsubmit = (e) => { e.preventDefault(); send(root, ctx, ta); };
  updateHint(root);
  ta.focus();
  if (pendingAsk) { ta.value = pendingAsk; pendingAsk = null; form.requestSubmit(); }
}

function updateHint(root) {
  const h = root.querySelector('#c-hint');
  if (h) h.innerHTML = S.open ? `<span>Fichier ouvert transmis :</span> <span translate="no">${esc(S.open)}</span>` : '';
  const b = root.querySelector('#c-send');
  if (b) { b.disabled = !!S.streaming; }
}

// ------------------------------------------------------------------ Arborescence

function renderTree(root, ctx) {
  const box = root.querySelector('#c-tree');
  box.innerHTML = '';
  const hidden = (p) => [...S.collapsed].some((d) => p !== d && p.startsWith(d));
  for (const p of S.tree) {
    if (hidden(p)) continue;
    const isDir = p.endsWith('/');
    const parts = p.replace(/\/$/, '').split('/');
    const name = parts[parts.length - 1];
    const row = el(`<button class="tree-row ${S.open === p ? 'active' : ''}" style="padding-left:${8 + (parts.length - 1) * 12}px" title="${esc(p)}">${isDir ? `<span class="caret">${S.collapsed.has(p) ? '▸' : '▾'}</span>` : icon('file')}<span translate="no">${esc(name)}</span></button>`);
    row.onclick = async (e) => {
      if (isDir) {
        S.collapsed.has(p) ? S.collapsed.delete(p) : S.collapsed.add(p);
        renderTree(root, ctx);
      } else if (e.altKey) {
        const ta = root.querySelector('textarea');
        ta.value += (ta.value && !ta.value.endsWith(' ') ? ' ' : '') + '@' + p + ' ';
        ta.focus();
      } else {
        await openFile(root, p);
        renderTree(root, ctx);
      }
    };
    box.appendChild(row);
  }
  const refresh = el(`<button class="tree-row faint" style="margin-top:6px">${icon('refresh')}<span>Actualiser</span></button>`);
  refresh.onclick = async () => { S.tree = await invoke('code_tree', { project: S.project }); renderTree(root, ctx); };
  box.appendChild(refresh);
}

async function openFile(root, p) {
  try {
    S.openText = await invoke('code_read', { project: S.project, path: p });
    S.open = p;
  } catch (e) {
    toast(errorText(e), 'error');
    return;
  }
  renderFile(root);
  updateHint(root);
}

function langOf(p) {
  const ext = (p.split('.').pop() || '').toLowerCase();
  const map = { rs: 'rust', js: 'javascript', mjs: 'javascript', ts: 'typescript', tsx: 'typescript', jsx: 'javascript', py: 'python', swift: 'swift', md: 'markdown', json: 'json', toml: 'ini', yml: 'yaml', yaml: 'yaml', html: 'xml', css: 'css', sh: 'bash', zsh: 'bash', c: 'c', h: 'c', cpp: 'cpp', java: 'java', kt: 'kotlin', go: 'go', rb: 'ruby', php: 'php', sql: 'sql', cs: 'csharp' };
  return map[ext] || 'plaintext';
}

function renderFile(root) {
  const box = root.querySelector('#c-file');
  if (!S.open) {
    box.innerHTML = `<div class="code-empty small"><p class="muted">Choisis un fichier à gauche pour l'afficher. ⌥-clic (Alt-clic) pour le mentionner dans ta demande.</p></div>`;
    return;
  }
  const lang = langOf(S.open);
  let html;
  try { html = window.hljs.highlight(S.openText, { language: lang, ignoreIllegals: true }).value; } catch { html = esc(S.openText); }
  const lines = S.openText.split('\n').length;
  box.innerHTML = `<div class="file-head"><b translate="no">${esc(S.open)}</b><span class="faint">${lines} <span>lignes</span></span></div>
    <div class="file-body"><pre class="gutter">${Array.from({ length: lines }, (_, i) => i + 1).join('\n')}</pre><pre class="src"><code class="hljs">${html}</code></pre></div>`;
}

// ------------------------------------------------------------------ Assistant

/** Fichiers proposés par l'IA : blocs « ```lang path=chemin », ou chemin en première ligne du bloc.
 *  Les petits modèles respectent mal le format : un bloc unique sans chemin vise le fichier ouvert. */
export function proposedFiles(text, openFile = null, tree = []) {
  const out = [];
  const blocks = [...(text || '').matchAll(/```([^\n`]*)\n([\s\S]*?)\n?```/g)];
  for (const [, info, body] of blocks) {
    let path = (info.match(/path[=:]\s*([^\s`]+)/) || [])[1];
    let content = body;
    const first = body.split('\n')[0];
    const inline = first.match(/^\s*(?:\/\/|#|--|<!--)?\s*(?:path|file|fichier)\s*[=:]\s*([^\s>]+)/i);
    if (!path && inline) { path = inline[1]; content = body.split('\n').slice(1).join('\n'); }
    if (!path) continue;
    path = path.replace(/^\.\//, '');
    // Chemin approximatif (ex. « src/demo/math.js ») : on retient le fichier du projet qui se termine pareil.
    if (tree.length && !tree.includes(path)) {
      const base = path.split('/').pop();
      const match = openFile && openFile.split('/').pop() === base ? openFile : tree.filter((t) => t.split('/').pop() === base);
      if (typeof match === 'string') path = match;
      else if (match.length === 1) path = match[0];
    }
    out.push({ path, content: content.replace(/\n*$/, '\n') });
  }
  if (!out.length && openFile && blocks.length === 1) out.push({ path: openFile, content: blocks[0][2].replace(/\n*$/, '\n') });
  return out;
}

function renderMessages(root) {
  const box = root.querySelector('#c-msgs');
  box.innerHTML = '';
  if (!S.messages.length) {
    box.appendChild(el(`<div class="code-empty small"><p class="muted">Exemples : « Explique ce fichier », « Ajoute une fonction de tri dans @src/utils.js », « Trouve le bug ».</p></div>`));
  }
  for (const m of S.messages) box.appendChild(messageNode(root, m));
  box.scrollTop = box.scrollHeight;
}

function messageNode(root, m) {
  if (m.role === 'user') return el(`<div class="c-msg user"><div class="bubble">${esc(m.content)}</div></div>`);
  const node = el(`<div class="c-msg ai" data-id="${m.id}"><div class="md"></div><div class="c-apply"></div></div>`);
  fillAi(root, node, m);
  return node;
}

function fillAi(root, node, m, status) {
  const md = node.querySelector('.md');
  md.innerHTML = m.content ? renderMarkdown(m.content) : `<span class="faint">${esc(status || 'Réflexion…')}</span>`;
  wireMarkdown(md);
  const ap = node.querySelector('.c-apply');
  ap.innerHTML = '';
  if (S.streaming === m.id) return;
  for (const f of proposedFiles(m.content, S.open, S.tree)) {
    const b = el(`<div class="apply-row">${icon('file')}<b translate="no">${esc(f.path)}</b><span class="grow"></span><button class="btn small primary">Voir et appliquer…</button></div>`);
    b.querySelector('button').onclick = () => reviewChange(root, f);
    ap.appendChild(b);
  }
}

async function send(root, ctx, ta) {
  const text = ta.value.trim();
  if (!text || S.streaming) return;
  ta.value = '';
  const ch = new Channel();
  let reply = null;
  let live = '';
  let status = '';
  let frame = 0;
  const box = root.querySelector('#c-msgs');
  const paint = () => {
    frame = 0;
    if (!reply) return;
    reply.content = live;
    const node = box.querySelector(`[data-id="${reply.id}"]`);
    if (node) fillAi(root, node, reply, status);
    box.scrollTop = box.scrollHeight;
  };
  const schedule = () => { if (!frame) frame = requestAnimationFrame(paint); };
  ch.onmessage = (ev) => {
    switch (ev.type) {
      case 'started':
        S.convId = ev.conversation_id;
        S.messages.push(ev.user);
        reply = ev.assistant;
        S.messages.push(reply);
        S.streaming = reply.id;
        if (S.messages.length === 2) box.innerHTML = '';
        box.appendChild(messageNode(root, ev.user));
        box.appendChild(messageNode(root, reply));
        updateHint(root);
        island({ busy: true, preview: text.slice(0, 160) });
        ctx.reloadConversations();
        break;
      case 'status': status = ev.text; schedule(); break;
      case 'delta': live += ev.text; status = ''; schedule(); break;
      case 'replace': live = ev.text; schedule(); break;
      case 'done': {
        S.streaming = null;
        const i = S.messages.findIndex((x) => x.id === ev.message.id);
        if (i >= 0) S.messages[i] = ev.message;
        cancelAnimationFrame(frame);
        frame = 0;
        const node = box.querySelector(`[data-id="${ev.message.id}"]`);
        if (node) fillAi(root, node, ev.message);
        box.scrollTop = box.scrollHeight;
        updateHint(root);
        island({ busy: false, preview: (ev.message.content || '').replace(/```[\s\S]*?```/g, '[code]').replace(/[#*_`>]/g, '').trim().slice(0, 200) });
        ctx.reloadConversations();
        break;
      }
    }
  };
  try {
    await invoke('send_message', {
      conversationId: S.convId, text, attachments: [],
      options: { model_id: ctx.selectedModel()?.id ?? null, web: false, think: ctx.state.settings.thinking === '1', mode: 'code', project: S.project, open_file: S.open },
      onEvent: ch,
    });
  } catch (e) {
    S.streaming = null;
    updateHint(root);
    island({ busy: false });
    toast(errorText(e), 'error');
  }
}

// ------------------------------------------------------------------ Différences et application

/** Différence ligne à ligne (plus longue sous-séquence commune), suffisante pour relire un changement. */
export function lineDiff(a, b) {
  const A = a.split('\n'), B = b.split('\n');
  if (A.length * B.length > 4_000_000) return [{ t: '-', s: `(${A.length} lignes)` }, { t: '+', s: `(${B.length} lignes)` }];
  const n = A.length, m = B.length;
  const L = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1));
  for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) L[i][j] = A[i] === B[j] ? L[i + 1][j + 1] + 1 : Math.max(L[i + 1][j], L[i][j + 1]);
  const out = [];
  let i = 0, j = 0;
  while (i < n && j < m) {
    if (A[i] === B[j]) { out.push({ t: ' ', s: A[i] }); i++; j++; }
    else if (L[i + 1][j] >= L[i][j + 1]) out.push({ t: '-', s: A[i++] });
    else out.push({ t: '+', s: B[j++] });
  }
  while (i < n) out.push({ t: '-', s: A[i++] });
  while (j < m) out.push({ t: '+', s: B[j++] });
  return out;
}

async function reviewChange(root, f) {
  let before = '';
  let exists = true;
  try { before = await invoke('code_read', { project: S.project, path: f.path }); } catch { exists = false; }
  const diff = lineDiff(before, f.content);
  const added = diff.filter((d) => d.t === '+').length, removed = diff.filter((d) => d.t === '-').length;
  // N'affiche que les changements et 3 lignes de contexte autour.
  const keep = diff.map((d, i) => d.t !== ' ' || diff.slice(Math.max(0, i - 3), i + 4).some((x) => x.t !== ' '));
  let rows = '', gap = false;
  diff.forEach((d, i) => {
    if (!keep[i]) { if (!gap) rows += '<div class="d-gap">⋯</div>'; gap = true; return; }
    gap = false;
    rows += `<div class="d-${d.t === '+' ? 'add' : d.t === '-' ? 'del' : 'ctx'}">${d.t} ${esc(d.s)}</div>`;
  });
  const { modal } = await import('../app.js');
  modal(`<h3>${exists ? 'Modifier' : 'Créer'} <span translate="no">${esc(f.path)}</span></h3>
    <div class="faint"><span style="color:var(--success)">+${added}</span> · <span style="color:var(--danger)">−${removed}</span>${exists ? ' · une copie de l’ancienne version sera gardée' : ' · nouveau fichier'}</div>
    <div class="diff" translate="no">${rows || '<div class="d-ctx">Aucun changement.</div>'}</div>
    <div class="row" style="justify-content:flex-end"><button class="btn" data-close>Annuler</button><button class="btn primary" id="apply">${icon('check')} Appliquer</button></div>`, (m) => {
    m.classList.add('wide');
    m.querySelector('#apply').onclick = async () => {
      try {
        await invoke('code_write', { project: S.project, path: f.path, content: f.content });
        m.remove();
        toast(`${f.path} : enregistré.`);
        S.tree = await invoke('code_tree', { project: S.project });
        if (S.open === f.path || !S.open) await openFile(root, f.path);
        const t = root.querySelector('#c-tree');
        if (t) renderTree(root, { reloadConversations() {} });
      } catch (e) {
        toast(errorText(e), 'error');
      }
    };
  });
}
