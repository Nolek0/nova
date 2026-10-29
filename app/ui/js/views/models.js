// Page Modèles : moteur llama.cpp, catalogue de modèles gratuits, Ollama et modèle Apple.
import { invoke, esc, el, toast, errorText, formatBytes, confirmDialog } from '../api.js';
import { icon } from '../icons.js';

const progress = {}; // tâche → dernier état reçu
const VIDEO_ID = 'video:wan-1.3b';
const VIDEO_TASK = `model:${VIDEO_ID}`;
const HD_ID = 'video:wan-hd';
const HD_TASK = `model:${HD_ID}`;
/** Texte de progression : octets pour un fichier, étapes (en %) pour l'installation de Nova Vidéo. */
const progressText = (p) => (p.task === VIDEO_TASK || p.task === HD_TASK ? `${esc(p.label || 'Préparation…')} · ${Math.round(p.total ? (100 * p.done) / p.total : 0)} %` : p.total ? `${formatBytes(p.done)} sur ${formatBytes(p.total)}` : 'Préparation…');
let rerender = null;

export function onProgress(p) {
  progress[p.task] = p;
  if (p.finished) {
    if (p.error && !/annulé/.test(p.error)) toast(p.error, 'error');
    delete progress[p.task];
    rerender?.(true);
  } else {
    const bar = document.querySelector(`[data-task="${CSS.escape(p.task)}"]`);
    if (bar) {
      bar.querySelector('.progress div').style.width = (p.total ? (100 * p.done) / p.total : 0) + '%';
      bar.querySelector('.ptext').innerHTML = progressText(p);
    } else rerender?.(false);
  }
}

export async function render(v, ctx) {
  const page = el(`<div class="page"><div class="row" style="align-items:flex-start"><div class="grow"><h1>Modèles</h1><p class="sub">L’IA de NOVA fonctionne sur ton ordinateur, gratuitement. Rien n’est téléchargé sans ton clic.</p></div><button class="btn" id="refresh">${icon('refresh')} Rafraîchir</button></div><div class="stack" id="content"><div class="faint">Vérification des moteurs…</div></div></div>`);
  v.appendChild(page);
  page.querySelector('#refresh').onclick = () => draw(true);
  rerender = (refresh) => { if (document.body.contains(page)) draw(refresh); };
  async function draw(refresh = true) {
    const o = refresh ? await ctx.refreshModels() : ctx.state.overview;
    if (!o) return;
    const content = page.querySelector('#content');
    content.innerHTML = '';
    content.appendChild(novaCard(o, ctx));
    if (o.video?.supported) content.appendChild(videoCard(o, ctx));
    content.appendChild(engineCard(o, ctx));
    if (o.engine.installed) {
      content.appendChild(el(`<h2 style="margin-top:8px">${icon('sparkles')} Modèles gratuits <span class="faint" style="font-weight:400">— mémoire détectée : ${o.ram_gb} Go</span></h2>`));
      const grid = el('<div class="grid-cards"></div>');
      o.catalog.forEach((m) => grid.appendChild(modelCard(m, o, ctx)));
      content.appendChild(grid);
    }
    content.appendChild(otherEngines(o, ctx));
  }
  draw(true);
}

/** Nova 1 : modèle maison, intégré à l'application (rien à télécharger). */
function novaCard(o, ctx) {
  const info = o.available.find((m) => m.provider === 'nova1');
  const inUse = ctx.state.settings.model === 'nova1';
  const card = el(`<div class="card stack"><div class="row"><span class="mark"></span><b style="font-size:15px">Nova 1</b><span class="faint">créé par NOVA, entraîné de zéro</span><span class="grow"></span><span class="badge ok">${icon('check')} Intégré · ${formatBytes(info?.size_bytes || 0)}</span></div>
    <p class="muted" style="margin:0;font-size:12.5px">Le modèle maison de NOVA, ultra rapide (moins d'une milliseconde) et sans téléchargement. Il répond seul aux demandes simples, quel que soit le modèle choisi, et peut être utilisé à part entière.</p>
    <div class="kv"><div><small>Texte</small><span>salutations, heure, date, dé, blagues…</span></div><div><small>Dessin</small><span>« dessine un canard sur un lac »</span></div><div><small>Résumé</small><span>PDF et textes joints</span></div><div><small>Vision</small><span>${info?.vision ? 'reconnaît 106 catégories, images et vidéos' : 'non installée dans cette version'}</span></div></div>
    <div class="faint">Pour une conversation libre, des explications ou du code, utilise un modèle de conversation ci-dessous (Gemma 4…).</div>
    <div class="row act"></div></div>`);
  const act = card.querySelector('.act');
  if (inUse) act.appendChild(el(`<span class="badge ok">${icon('check')} Utilisé</span>`));
  else {
    const use = el('<button class="btn primary small">Utiliser Nova 1</button>');
    use.onclick = async () => { await ctx.setSetting('model', 'nova1'); toast('Nova 1 sera utilisé pour les prochaines réponses.'); ctx.setView('models'); };
    act.appendChild(use);
  }
  return card;
}

/** Nova Vidéo : Wan 2.1 1.3B via le portage Mac de WanGP (Python + modèles, ≈ 13 Go). */
function videoCard(o, ctx) {
  const v = o.video;
  const task = progress[VIDEO_TASK];
  const inUse = ctx.state.settings.model === VIDEO_ID;
  const card = el(`<div class="card stack"><div class="row"><b style="font-size:15px">${icon('video')} Nova Vidéo</b><span class="faint">Wan 2.1 · 1,3 milliard de paramètres</span><span class="grow"></span>${v.installed ? `<span class="badge ok">${icon('check')} Installé</span>` : '<span class="badge">Non installé</span>'}</div>
    <p class="muted" style="margin:0;font-size:12.5px">Crée des vidéos de 5 secondes à partir d'une phrase, entièrement sur ton Mac. Écris simplement « fais une vidéo d'un chat qui saute » dans le chat, ou choisis Nova Vidéo dans la liste des modèles.</p>
    <div class="kv"><div><small>Taille</small><span>≈ ${v.size_gb} Go (programme + modèles)</span></div><div><small>Durée</small><span>5 s en 720×416</span></div><div><small>Temps</small><span>≈ 7 min sur un Mac de 16 Go</span></div><div><small>Mémoire utilisée</small><span>≈ 4 Go au plus</span></div><div><small>Mémoire conseillée</small><span>${v.min_ram_gb} Go</span></div></div>
    ${v.ram_gb < v.min_ram_gb ? `<div class="warn-line">${icon('alert')} ${v.ram_gb} Go de mémoire détectés : la génération risque d'échouer ou d'être très lente.</div>` : ''}
    <div class="faint">Basé sur Wan 2.1 (Alibaba, Apache 2.0) et WanGP de DeepBeepMeep (WanGP Community License), porté sur Mac par NOVA : github.com/Nolek0/wan2gp-mac</div>
    <div class="row act"></div></div>`);
  const act = card.querySelector('.act');
  if (task) act.appendChild(progressBar(task));
  else if (v.installed) {
    if (inUse) act.appendChild(el(`<span class="badge ok">${icon('check')} Utilisé</span>`));
    else {
      const use = el('<button class="btn primary small">Utiliser pour les prochains messages</button>');
      use.onclick = async () => { await ctx.setSetting('model', VIDEO_ID); toast('Tes prochains messages créeront des vidéos.'); ctx.setView('models'); };
      act.appendChild(use);
    }
    const del = el(`<button class="btn small danger">${icon('trash')} Supprimer</button>`);
    del.onclick = async () => {
      if (!(await confirmDialog(`Supprimer Nova Vidéo (≈ ${v.size_gb} Go) de cet ordinateur ? Les vidéos déjà créées sont gardées.`))) return;
      await invoke('delete_model', { id: VIDEO_ID });
      if (inUse) await ctx.setSetting('model', '');
      ctx.setView('models');
    };
    act.appendChild(del);
    card.appendChild(hdBlock(v, ctx));
  } else {
    const dl = el(`<button class="btn small">${icon('download')} Télécharger (≈ ${v.size_gb} Go)</button>`);
    dl.onclick = async () => {
      if (!(await confirmDialog(`Télécharger Nova Vidéo ? (≈ ${v.size_gb} Go, gratuit) : le programme depuis GitHub, Python et PyTorch, puis les modèles Wan 2.1 depuis Hugging Face. Compte 15 à 30 minutes. Tu peux annuler à tout moment.`))) return;
      progress[VIDEO_TASK] = { task: VIDEO_TASK, label: 'Préparation…', done: 0, total: 100 };
      ctx.setView('models');
      invoke('download_model', { id: VIDEO_ID })
        .then(() => toast('Nova Vidéo est prêt. Écris « fais une vidéo de… » dans le chat.'))
        .catch((err) => toast(errorText(err), 'error'));
    };
    act.appendChild(dl);
  }
  return card;
}

/** Version HD (facultative) : Wan 2.2 5B + FastWan, 960×544 à 24 images/s, visages et détails bien plus nets. */
function hdBlock(v, ctx) {
  const task = progress[HD_TASK];
  const box = el(`<div class="stack" style="border-top:1px solid var(--border);padding-top:10px;gap:8px"><div class="row"><b>Version HD</b><span class="faint">Wan 2.2 · 5 milliards de paramètres</span><span class="grow"></span>${v.hd_installed ? `<span class="badge ok">${icon('check')} Utilisée pour tes vidéos</span>` : '<span class="badge">Facultative</span>'}</div>
    <p class="muted" style="margin:0;font-size:12.5px">Des vidéos bien plus réalistes : visages, yeux et vêtements nets, 960×544 à 24 images par seconde. Environ 9 min par vidéo de 5 s et ≈ 7 Go de mémoire au plus. Une fois installée, elle remplace la version standard.</p>
    <div class="row act"></div></div>`);
  const act = box.querySelector('.act');
  if (task) act.appendChild(progressBar(task));
  else if (v.hd_installed) {
    const del = el(`<button class="btn small danger">${icon('trash')} Supprimer la HD</button>`);
    del.onclick = async () => {
      if (!(await confirmDialog(`Supprimer la version HD (≈ ${v.hd_size_gb} Go) ? Nova Vidéo repassera en version standard.`))) return;
      await invoke('delete_model', { id: HD_ID });
      ctx.setView('models');
    };
    act.appendChild(del);
  } else {
    const dl = el(`<button class="btn primary small">${icon('download')} Passer en HD (≈ ${v.hd_size_gb} Go)</button>`);
    dl.onclick = async () => {
      if (!(await confirmDialog(`Télécharger Nova Vidéo HD ? (≈ ${v.hd_size_gb} Go depuis Hugging Face : Wan 2.2 5B, son décodeur et l'accélération FastWan, licence Apache 2.0). Tu peux annuler à tout moment.`))) return;
      progress[HD_TASK] = { task: HD_TASK, label: 'Préparation…', done: 0, total: 100 };
      ctx.setView('models');
      invoke('download_model', { id: HD_ID }).then(() => toast('Nova Vidéo HD est prête : tes prochaines vidéos seront en HD.')).catch((err) => toast(errorText(err), 'error'));
    };
    act.appendChild(dl);
  }
  return box;
}

function engineCard(o, ctx) {
  const task = progress.engine;
  const e = o.engine;
  const card = el(`<div class="card stack"><div class="row"><b style="font-size:15px">Moteur d’IA local (llama.cpp)</b><span class="grow"></span>${e.installed ? `<span class="badge ok">${icon('check')} Installé · ${esc(e.version || '')}</span>` : '<span class="badge warn">Non installé</span>'}</div>
    <p class="muted" style="margin:0;font-size:12.5px">Moteur libre et gratuit qui exécute les modèles directement sur ta carte graphique (${o.engine.platform.startsWith('macos') ? 'Metal sur Mac' : 'Vulkan sur Windows : NVIDIA, AMD et Intel'}). Fonctionne sans Internet une fois installé.</p>
    ${e.running_model ? `<div class="faint">En mémoire : ${esc(e.running_model)}</div>` : ''}
    <div class="row act"></div></div>`);
  const act = card.querySelector('.act');
  if (task) act.appendChild(progressBar(task));
  else if (!e.installed) {
    const b = el(`<button class="btn primary">${icon('download')} Installer le moteur (≈ ${o.engine.platform.startsWith('macos') ? '12' : '32'} Mo)</button>`);
    b.onclick = () => {
      progress.engine = { task: 'engine', done: 0, total: 0 };
      ctx.setView('models');
      invoke('install_engine').then(() => toast('Moteur installé. Choisis maintenant un modèle.')).catch((err) => toast(errorText(err), 'error'));
    };
    act.appendChild(b);
    act.appendChild(el(`<span class="faint">Source officielle : github.com/ggml-org/llama.cpp</span>`));
  } else {
    const b = el(`<button class="btn small">${icon('refresh')} Mettre à jour</button>`);
    b.onclick = () => { progress.engine = { task: 'engine', done: 0, total: 0 }; ctx.setView('models'); invoke('install_engine').catch((err) => toast(errorText(err), 'error')); };
    act.appendChild(b);
  }
  return card;
}

function progressBar(p) {
  const pct = p.total ? (100 * p.done) / p.total : 0;
  const box = el(`<div class="grow" data-task="${esc(p.task)}" style="display:grid;gap:6px"><div class="progress"><div style="width:${pct}%"></div></div><div class="row"><span class="faint ptext">${progressText(p)}</span><span class="grow"></span><button class="btn small">Annuler</button></div></div>`);
  box.querySelector('button').onclick = () => invoke('cancel_download', { task: p.task });
  return box;
}

function modelCard(m, o, ctx) {
  const task = progress[`model:${m.id}`];
  const recommended = m.id === o.recommended;
  const tooBig = m.min_ram_gb > o.ram_gb;
  const inUse = ctx.state.settings.model === `llama:${m.id}` || (!ctx.state.settings.model && o.available[0]?.id === `llama:${m.id}`);
  const card = el(`<div class="card model-card"><div class="top"><b>${esc(m.name)}</b><span class="faint">${esc(m.publisher)}</span><span class="grow"></span>${recommended ? '<span class="badge accent">Recommandé</span>' : ''}${m.mmproj ? `<span class="badge cyan">${icon('eye')} vision</span>` : ''}</div>
    <div class="muted" style="font-size:12.5px">${esc(m.description)}</div>
    <div class="kv"><div><small>Taille</small><span>${m.size_gb} Go</span></div><div><small>Mémoire conseillée</small><span>${m.min_ram_gb} Go</span></div><div><small>Points forts</small><span>${esc(m.strengths)}</span></div></div>
    ${tooBig ? `<div class="warn-line">${icon('alert')} Plus de mémoire que les ${o.ram_gb} Go détectés : risque de lenteur.</div>` : ''}
    <div class="row act"></div></div>`);
  const act = card.querySelector('.act');
  if (task) act.appendChild(progressBar(task));
  else if (m.installed) {
    if (inUse) act.appendChild(el(`<span class="badge ok">${icon('check')} Utilisé</span>`));
    else {
      const use = el('<button class="btn primary small">Utiliser</button>');
      use.onclick = async () => { await ctx.setSetting('model', `llama:${m.id}`); invoke('warm_up', { modelId: `llama:${m.id}` }).catch(() => {}); toast(`${m.name} sera utilisé pour les prochaines réponses.`); ctx.setView('models'); };
      act.appendChild(use);
    }
    const del = el(`<button class="btn small danger">${icon('trash')} Supprimer</button>`);
    del.onclick = async () => {
      if (!(await confirmDialog(`Supprimer ${m.name} (${m.size_gb} Go) de cet ordinateur ?`))) return;
      await invoke('delete_model', { id: m.id });
      ctx.setView('models');
    };
    act.appendChild(del);
  } else {
    const dl = el(`<button class="btn ${recommended ? 'primary' : ''} small">${icon('download')} Télécharger (${m.size_gb} Go)</button>`);
    dl.onclick = async () => {
      if (!(await confirmDialog(`Télécharger ${m.name} depuis Hugging Face ? (${m.size_gb} Go, gratuit). Tu peux annuler à tout moment.`))) return;
      progress[`model:${m.id}`] = { task: `model:${m.id}`, done: 0, total: 0 };
      ctx.setView('models');
      invoke('download_model', { id: m.id })
        .then(async () => { if (!ctx.state.settings.model?.startsWith('llama:')) await ctx.setSetting('model', `llama:${m.id}`); toast(`${m.name} est prêt.`); })
        .catch((err) => toast(errorText(err), 'error'));
    };
    act.appendChild(dl);
  }
  return card;
}

function otherEngines(o, ctx) {
  const box = el('<div class="stack" style="margin-top:6px"></div>');
  if (ctx.isMac()) {
    box.appendChild(el(`<div class="card stack"><div class="row"><b>Modèle Apple (sur l’appareil)</b><span class="grow"></span>${o.apple_ready ? `<span class="badge ok">${icon('check')} Prêt</span>` : '<span class="badge warn">Indisponible</span>'}</div><p class="muted" style="margin:0;font-size:12.5px">Fourni par macOS avec Apple Intelligence, sans téléchargement. Petit modèle, rapide, mais plus limité et soumis à des garde-fous stricts. ${o.apple_ready ? '' : esc(o.apple_message)}</p></div>`));
  }
  const ollama = o.available.filter((m) => m.provider === 'ollama');
  box.appendChild(el(`<div class="card stack"><div class="row"><b>Ollama (facultatif)</b><span class="grow"></span>${ollama.length ? `<span class="badge ok">${ollama.length} modèle(s)</span>` : '<span class="badge">Non détecté</span>'}</div><p class="muted" style="margin:0;font-size:12.5px">Si tu utilises déjà Ollama, NOVA détecte automatiquement ses modèles. Inutile sinon : le moteur intégré ci-dessus suffit.</p>${ollama.map((m) => `<div class="row" style="font-size:13px">${icon('cpu')} ${esc(m.name)} ${m.vision ? '<span class="badge cyan">vision</span>' : ''}</div>`).join('')}</div>`));
  return box;
}
