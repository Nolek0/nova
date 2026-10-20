// Nova Control : l'agent qui accomplit une tâche sur le Mac (voir src-tauri/src/control.rs).
// Les actions sensibles s'affichent ici et attendent « Autoriser » ; Échap ou ■ arrête tout.
import { invoke, Channel, esc, el, toast, errorText } from '../api.js';
import { icon } from '../icons.js';

const S = { running: false, steps: [], status: '', goal: '' };
/** Modes d'autorisation (comme Claude) : Auto (actions sensibles seulement), Manuel, Tout accepter. */
export const MODES = [
  ['auto', 'Auto : demander pour les actions sensibles'],
  ['ask_all', 'Manuel : demander avant chaque action'],
  ['accept_all', 'Tout accepter : ne jamais demander'],
];
const EXAMPLES = [
  'Ouvre Notes et crée une note « Courses » avec : pain, lait, œufs',
  'Ouvre Photoshop et enregistre l’image ouverte en PNG sur le Bureau',
  'Écris un e-mail à paul@exemple.fr pour lui dire que je serai en retard demain',
  'Ouvre Safari et va sur wikipedia.org',
];

export async function render(v, ctx) {
  const page = el(`<div class="page control"><h1>${icon('power')} Nova Control</h1>
    <p class="sub">Décris une tâche : Nova Control la réalise sur ton Mac, étape par étape, avec le curseur bleu. Tout reste sur ton ordinateur. Les actions sensibles (envoyer, supprimer, acheter, publier…) attendent ton accord. <b>Échap</b> arrête tout à tout moment.</p>
    <div id="perm"></div>
    <div class="card stack" id="task">
      <textarea class="field" id="goal" rows="3" placeholder="Ex. : Ouvre Photoshop et enregistre l’image ouverte en PNG sur le Bureau"></textarea>
      <div class="row ex" id="ex"></div>
      <div class="row"><span class="faint">Cerveau</span><select class="field" id="model" style="max-width:240px" translate="no"></select>
        <span class="faint">Mode</span><select class="field" id="mode" style="max-width:240px" title="Quand Nova Control doit demander ton accord">${MODES.map(([v, l]) => `<option value="${v}">${l}</option>`).join('')}</select><span class="grow"></span>
        <button class="btn primary" id="go">${icon('power')} Lancer</button><button class="btn danger hidden" id="stop">■ Arrêter</button></div>
      <div class="faint"><b>Nova Control</b> (intégré) sait : ouvrir/fermer des apps et des sites, chercher sur le web, créer des notes et des dossiers, envoyer des e-mails, enregistrer/exporter (nom, format, dossier), choisir dans les menus, cliquer sur un bouton, écrire, les raccourcis, le volume et la musique — et enchaîner plusieurs étapes (« ouvre Photoshop puis enregistre l'image en PNG sur le Bureau »). Pour une tâche libre, le mode avancé utilise un modèle de vision installé (Gemma 4). Mac uniquement.</div>
    </div>
    <div class="stack" id="steps"></div></div>`);
  v.appendChild(page);
  const goal = page.querySelector('#goal');
  goal.value = S.goal;
  goal.oninput = () => (S.goal = goal.value);
  EXAMPLES.forEach((x) => {
    const b = el(`<button class="chip">${esc(x)}</button>`);
    b.onclick = () => { goal.value = S.goal = x; goal.focus(); };
    page.querySelector('#ex').appendChild(b);
  });
  // Modèles : ceux qui voient les images d'abord.
  try {
    const o = await invoke('models_overview');
    const list = o.available.filter((m) => m.provider !== 'nova1' && m.vision);
    const sel = page.querySelector('#model');
    // Cerveau intégré par défaut (aucun téléchargement) ; les modèles de vision installés en mode avancé.
    sel.appendChild(el('<option value="nova-control">Nova Control · intégré, ultra rapide</option>'));
    list.forEach((m) => sel.appendChild(el(`<option value="${esc(m.id)}">${esc(m.name)} · mode avancé (plus lent)</option>`)));
    const pref = ctx.state.settings.controlModel;
    sel.value = pref && list.some((m) => m.id === pref) ? pref : 'nova-control';
    sel.onchange = () => ctx.setSetting('controlModel', sel.value);
  } catch {}
  const modeSel = page.querySelector('#mode');
  modeSel.value = ctx.state.settings.controlMode || 'auto';
  modeSel.onchange = () => {
    ctx.setSetting('controlMode', modeSel.value);
    if (modeSel.value === 'accept_all') toast('Tout accepter : Nova Control agira sans te demander. Échap arrête tout ; il n’écrit jamais dans un champ de mot de passe.');
  };
  await checkPermissions(page);
  page.querySelector('#go').onclick = () => start(page, ctx);
  page.querySelector('#stop').onclick = () => invoke('control_stop');
  goal.onkeydown = (e) => { if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) start(page, ctx); };
  paint(page);
}

async function checkPermissions(page) {
  const box = page.querySelector('#perm');
  let p = {};
  try { p = await invoke('control_permissions'); } catch (e) { box.innerHTML = `<div class="error-box">${esc(errorText(e))}</div>`; return false; }
  if (p.accessibility && p.screen) { box.innerHTML = ''; return true; }
  box.innerHTML = `<div class="card stack warn-card"><b>${icon('shield')} Deux autorisations sont nécessaires (une seule fois)</b>
    <div class="muted" style="font-size:13px">macOS demande ton accord pour que NOVA puisse voir l’écran et cliquer à ta place. Ouvre chaque réglage, active <b>NOVA</b>, puis reviens ici.</div>
    <div class="row"><span class="badge ${p.accessibility ? 'ok' : 'warn'}">${p.accessibility ? '✓' : '✗'} Accessibilité</span><button class="btn small" id="p-ax">Ouvrir le réglage</button></div>
    <div class="row"><span class="badge ${p.screen ? 'ok' : 'warn'}">${p.screen ? '✓' : '✗'} Enregistrement de l’écran</span><button class="btn small" id="p-sc">Ouvrir le réglage</button></div>
    <div class="row"><button class="btn primary small" id="p-check">${icon('refresh')} Vérifier à nouveau</button><span class="faint">Après avoir autorisé l’enregistrement de l’écran, macOS peut demander de relancer NOVA.</span></div></div>`;
  box.querySelector('#p-ax').onclick = async () => { await invoke('control_request_permissions').catch(() => {}); invoke('open_url', { url: 'x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility' }); };
  box.querySelector('#p-sc').onclick = async () => { await invoke('control_request_permissions').catch(() => {}); invoke('open_url', { url: 'x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture' }); };
  box.querySelector('#p-check').onclick = () => checkPermissions(page);
  return false;
}

function paint(page) {
  page.querySelector('#go').classList.toggle('hidden', S.running);
  page.querySelector('#stop').classList.toggle('hidden', !S.running);
  renderSteps(S, page.querySelector('#steps'), () => paint(page));
}

/** Affiche le déroulé d'une tâche (étapes, demandes d'accord, résultat) dans `box`. */
function renderSteps(S, box, rerender) {
  box.innerHTML = '';
  if (S.status) box.appendChild(el(`<div class="status-line"><span class="typing"><i></i><i></i><i></i></span>${esc(S.status)}</div>`));
  for (const s of [...S.steps].reverse()) {
    if (s.type === 'approval') {
      const card = el(`<div class="card stack approval"><b>${icon('shield')} Autoriser : ${esc(s.title)} ?</b><pre class="detail">${esc(s.detail)}</pre>
        <div class="row" style="justify-content:flex-end"><button class="btn" data-no>Refuser</button><button class="btn primary" data-yes>Autoriser</button></div></div>`);
      if (s.decided) { card.querySelector('.row').innerHTML = `<span class="faint">${s.decided === 'yes' ? 'Autorisé' : 'Refusé'}</span>`; }
      else {
        const decide = (allow) => { s.decided = allow ? 'yes' : 'no'; invoke('control_decide', { id: s.id, allow }); rerender(); };
        // « Autoriser » ne réagit qu'à un vrai clic de souris : une touche (Entrée, Espace), même tapée
        // par erreur ou par une application, ne peut rien autoriser. « Refuser » accepte tout.
        card.querySelector('[data-yes]').onclick = (e) => { if (e.detail > 0 && e.isTrusted) decide(true); };
        card.querySelector('[data-no]').onclick = () => decide(false);
        card.querySelector('[data-yes]').tabIndex = -1;
      }
      box.appendChild(card);
    } else if (s.type === 'step') {
      box.appendChild(el(`<div class="c-step ${s.ok ? '' : 'bad'}"><span class="n">${s.n}</span><div><b>${esc(s.action)}</b>${s.thought ? `<div class="faint">${esc(s.thought)}</div>` : ''}${s.ok ? '' : `<div class="err">${esc(s.result)}</div>`}</div></div>`));
    } else if (s.type === 'done') {
      box.appendChild(el(`<div class="card done">${icon('check')} <b>${esc(s.summary)}</b>${s.steps ? ` <span class="faint">· ${s.steps} étape${s.steps > 1 ? 's' : ''}</span>` : ''}</div>`));
    } else if (s.type === 'question') {
      // Question : on répond ici, la tâche reprend avec la réponse.
      const card = el(`<div class="card stack approval"><b>${icon('info')} Nova Control demande : ${esc(s.text)}</b>
        <div class="row"><input class="field grow" placeholder="Ta réponse…"><button class="btn primary">Répondre</button></div></div>`);
      const input = card.querySelector('input');
      if (s.answered !== undefined) card.querySelector('.row').innerHTML = `<span class="faint">Ta réponse : ${esc(s.answered)}</span>`;
      else {
        const send = () => { const t = input.value.trim(); if (!t) return; s.answered = t; invoke('control_answer', { id: s.id, text: t }); rerender(); };
        card.querySelector('button').onclick = send;
        input.onkeydown = (e) => { if (e.key === 'Enter') send(); };
        setTimeout(() => input.focus(), 30);
      }
      box.appendChild(card);
    } else if (s.type === 'error') {
      box.appendChild(el(`<div class="error-box">${icon('alert')}<span>${esc(s.message)}</span></div>`));
    }
  }
}

async function start(page, ctx) {
  const goal = page.querySelector('#goal').value.trim();
  if (!goal || S.running) return;
  if (!(await checkPermissions(page))) { toast('Autorise d’abord NOVA dans les Réglages Système.', 'error'); return; }
  S.running = true;
  S.steps = [];
  S.status = 'Préparation…';
  paint(page);
  const ch = new Channel();
  ch.onmessage = (ev) => {
    if (ev.type === 'decided') { const ap = S.steps.find((x) => x.type === 'approval' && x.id === ev.id); if (ap) ap.decided = ev.allow ? 'yes' : 'no'; if (document.body.contains(page)) paint(page); return; }
    if (ev.type === 'status') S.status = ev.text;
    else if (ev.type === 'done' || ev.type === 'error') {
      S.status = '';
      S.running = false;
      if (ev.type === 'error' && ev.message === 'permissions') { checkPermissions(page); ev = { type: 'error', message: 'Autorisations manquantes : active NOVA dans les Réglages Système (ci-dessus).' }; }
      S.steps.push(ev);
    } else S.steps.push(ev);
    if (document.body.contains(page)) paint(page);
  };
  try {
    await invoke('control_start', { goal, modelId: page.querySelector('#model').value || null, mode: ctx.state.settings.controlMode || 'auto', onEvent: ch });
  } catch (e) {
    S.running = false;
    S.status = '';
    S.steps.push({ type: 'error', message: errorText(e) });
    paint(page);
  }
}

/** Nova Control depuis une conversation : la tâche et son déroulé s'affichent dans `mount`. */
export async function runInline(goal, mount, ctx) {
  const st = { running: true, steps: [], status: 'Préparation…' };
  const mode = MODES.find(([v]) => v === (ctx.state.settings.controlMode || 'auto'))?.[1] || '';
  const card = el(`<div class="card stack control-inline"><div class="row"><b>${icon('power')} Nova Control</b><span class="faint">${esc(mode)}</span><span class="grow"></span><button class="btn small danger">■ Arrêter</button></div><div class="stack steps"></div></div>`);
  mount.appendChild(card);
  const stop = card.querySelector('.btn');
  stop.onclick = () => invoke('control_stop');
  const rerender = () => { stop.classList.toggle('hidden', !st.running); renderSteps(st, card.querySelector('.steps'), rerender); };
  const fail = (message) => { st.running = false; st.status = ''; st.steps.push({ type: 'error', message }); rerender(); };
  rerender();
  try {
    const p = await invoke('control_permissions');
    if (!p.accessibility || !p.screen) return fail('Autorise d’abord NOVA (Accessibilité et Enregistrement de l’écran) : page Nova Control.');
  } catch (e) { return fail(errorText(e)); }
  const ch = new Channel();
  ch.onmessage = (ev) => {
    if (ev.type === 'decided') { const ap = st.steps.find((x) => x.type === 'approval' && x.id === ev.id); if (ap) ap.decided = ev.allow ? 'yes' : 'no'; rerender(); return; }
    if (ev.type === 'status') st.status = ev.text;
    else if (ev.type === 'done' || ev.type === 'error') {
      st.status = '';
      st.running = false;
      st.steps.push(ev.type === 'error' && ev.message === 'permissions' ? { type: 'error', message: 'Autorisations manquantes : page Nova Control.' } : ev);
      saveToHistory(goal, st, ctx);
    } else st.steps.push(ev);
    rerender();
  };
  try {
    await invoke('control_start', { goal, modelId: ctx.state.settings.controlModel || null, mode: ctx.state.settings.controlMode || 'auto', onEvent: ch });
  } catch (e) { fail(errorText(e)); }
}

/** Compte rendu Markdown d'une tâche, enregistré dans l'historique du chat. */
async function saveToHistory(goal, st, ctx) {
  const lines = [];
  const end = st.steps.find((s) => s.type === 'done' || s.type === 'error');
  lines.push(`**Nova Control** — ${end?.type === 'done' ? end.summary : `⚠️ ${end?.message || 'interrompu'}`}`, '');
  for (const s of st.steps) {
    if (s.type === 'step') lines.push(`${s.ok ? '✓' : '✗'} ${s.action}${s.ok ? '' : ` — ${s.result}`}`);
    else if (s.type === 'approval') lines.push(`🛡 ${s.title} : ${s.decided === 'yes' ? 'autorisé' : s.decided === 'no' ? 'refusé' : 'sans réponse'}`);
    else if (s.type === 'question') lines.push(`❓ ${s.text}${s.answered !== undefined ? ` → ${s.answered}` : ''}`);
  }
  try {
    const id = await invoke('control_save', { conversationId: ctx.state.convId || null, goal, report: lines.join('\n') });
    ctx.state.convId = id;
    ctx.reloadConversations();
  } catch {}
}
