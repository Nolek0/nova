// Page Modèles : moteur llama.cpp, catalogue de modèles gratuits, Ollama et modèle Apple.
import { invoke, esc, el, toast, errorText, formatBytes, confirmDialog } from '../api.js';
import { icon } from '../icons.js';

const progress = {}; // tâche → dernier état reçu
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
      bar.querySelector('.ptext').textContent = `${formatBytes(p.done)} sur ${formatBytes(p.total)}`;
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
  const box = el(`<div class="grow" data-task="${esc(p.task)}" style="display:grid;gap:6px"><div class="progress"><div style="width:${pct}%"></div></div><div class="row"><span class="faint ptext">${p.total ? `${formatBytes(p.done)} sur ${formatBytes(p.total)}` : 'Préparation…'}</span><span class="grow"></span><button class="btn small">Annuler</button></div></div>`);
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
