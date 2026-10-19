// Paramètres : chaque changement est enregistré immédiatement (SQLite local).
import { invoke, dialog, esc, el, toast, errorText, confirmDialog } from '../api.js';
import { icon } from '../icons.js';
import { folders, addFolder, removeFolder } from './files.js';
import { UI_LANGUAGES, REPLY_LANGUAGES } from '../i18n.js';

export function render(v, ctx) {
  const s = ctx.state.settings;
  const page = el(`<div class="page"><h1>Paramètres</h1><p class="sub">Chaque modification est enregistrée immédiatement sur cet ordinateur.</p><div class="stack" id="groups"></div></div>`);
  v.appendChild(page);
  const groups = page.querySelector('#groups');
  const group = (title, ic, html) => { const g = el(`<div class="card stack"><h2>${icon(ic)} ${title}</h2>${html}</div>`); groups.appendChild(g); return g; };

  // Langue
  const lg = group('Langue', 'globe', `<div class="row"><span style="width:180px">Langue de l'interface</span><select class="field" id="uilang"></select></div>
    <div class="faint hidden" id="review">Traduction à faire relire par un locuteur natif.</div>
    <div class="row"><span style="width:180px">Langue des réponses de l'IA</span><select class="field" id="replylang" style="max-width:300px"></select><input class="field hidden" id="replyother" placeholder="Écris le nom de la langue" style="width:200px"></div>`);
  const uiSel = lg.querySelector('#uilang');
  uiSel.appendChild(el(`<option value="system">Système</option>`));
  UI_LANGUAGES.forEach((l) => uiSel.appendChild(el(`<option value="${l.code}" translate="no">${esc(l.name)}${l.review ? ' *' : ''}</option>`)));
  uiSel.value = s.language || 'system';
  const showReview = () => lg.querySelector('#review').classList.toggle('hidden', !UI_LANGUAGES.find((l) => l.code === uiSel.value)?.review);
  showReview();
  uiSel.onchange = async () => { await ctx.setSetting('language', uiSel.value); location.reload(); };
  const rSel = lg.querySelector('#replylang'), rOther = lg.querySelector('#replyother');
  rSel.appendChild(el(`<option value="auto">Automatique (langue de l'interface)</option>`));
  REPLY_LANGUAGES.forEach((l) => rSel.appendChild(el(`<option value="${esc(l)}" translate="no">${esc(l)}</option>`)));
  rSel.appendChild(el(`<option value="__other">Autre langue…</option>`));
  const cur = s.replyLanguage || 'auto';
  if (cur === 'auto' || REPLY_LANGUAGES.includes(cur)) rSel.value = cur;
  else { rSel.value = '__other'; rOther.value = cur; rOther.classList.remove('hidden'); }
  rSel.onchange = () => {
    rOther.classList.toggle('hidden', rSel.value !== '__other');
    if (rSel.value !== '__other') ctx.setSetting('replyLanguage', rSel.value);
    else rOther.focus();
  };
  let tOther;
  rOther.oninput = () => { clearTimeout(tOther); tOther = setTimeout(() => rOther.value.trim() && ctx.setSetting('replyLanguage', rOther.value.trim()), 400); };

  // Apparence
  const ap = group('Apparence', 'monitor', `<div class="row"><span style="width:150px">Thème</span><select class="field" id="theme"><option value="dark">Sombre</option><option value="light">Clair</option><option value="system">Système</option></select></div><div class="faint">Les préférences d’accessibilité du système (réduction des animations, contraste) sont respectées.</div>`);
  const theme = ap.querySelector('#theme');
  theme.value = s.appearance || 'dark';
  theme.onchange = async () => { await ctx.setSetting('appearance', theme.value); ctx.applyTheme(); };

  // Nova Code
  const nc = group('Nova Code', 'code', `<label class="row"><input type="checkbox" id="nc"> Activer Nova Code (onglet « Code » pour travailler sur tes projets)</label>
    <div class="faint">Nova Code lit uniquement le dossier de projet que tu choisis. Il propose des modifications que tu relis (différences) avant de les appliquer ; l'ancienne version de chaque fichier est sauvegardée.</div>`);
  const ncBox = nc.querySelector('#nc');
  ncBox.checked = s.novaCode === '1';
  ncBox.onchange = async () => {
    await ctx.setSetting('novaCode', ncBox.checked ? '1' : '0');
    ctx.renderSidebar();
  };

  // Bulle de l'encoche (Mac)
  const isl = ctx.isMac() ? group("Bulle de l'encoche", 'sparkles', `<label class="row"><input type="checkbox" id="isl"> Afficher NOVA dans l'encoche du Mac et en haut de chaque écran</label>
    <div class="faint">Passe la souris sur l'encoche (ou sur la petite pastille en haut des écrans externes) : choisis Nova Chat ou Nova Code et le modèle, puis pose ta question sans quitter ce que tu fais.</div>`) : null;
  const islBox = isl?.querySelector('#isl');
  if (islBox) islBox.checked = s.notchIsland === '1';
  if (islBox) islBox.onchange = async () => {
    await ctx.setSetting('notchIsland', islBox.checked ? '1' : '0');
    try { await invoke('set_notch_island', { enabled: islBox.checked }); } catch (e) { toast(errorText(e), 'error'); }
  };

  // Personnalisation
  const pe = group('Personnalisation', 'pencil', `<div class="row"><span style="width:150px">Ton prénom</span><input class="field" id="name" placeholder="Facultatif" style="width:240px"></div>
    <div class="row"><span style="width:150px">Phrase d’accueil</span><input class="field grow" id="greet" placeholder="Que puis-je faire pour toi ?"></div>
    <div><div style="margin-bottom:6px">Instructions personnalisées</div><textarea class="field" id="ci" rows="3" style="width:100%" placeholder="Ex. : Je suis en terminale, explique les maths pas à pas. Réponds de façon concise."></textarea></div>`);
  bindText(pe.querySelector('#name'), 'userName');
  bindText(pe.querySelector('#greet'), 'greeting');
  bindText(pe.querySelector('#ci'), 'customInstructions');

  // Mémoire
  const me = group('Mémoire', 'brain', `<label class="row"><input type="checkbox" id="usemem"> Transmettre la mémoire aux modèles</label><div class="faint">NOVA ne mémorise rien automatiquement : seules les informations ajoutées ici sont retenues d’une conversation à l’autre.</div><div class="row"><input class="field grow" id="mem" placeholder="Ajouter une information à retenir…"><button class="btn" id="addmem">Ajouter</button></div><div id="mems" class="stack" style="gap:4px"></div>`);
  const use = me.querySelector('#usemem');
  use.checked = s.useMemories !== '0';
  use.onchange = () => ctx.setSetting('useMemories', use.checked ? '1' : '0');
  const drawMems = (list) => {
    ctx.state.memories = list;
    const box = me.querySelector('#mems');
    box.innerHTML = list.length ? '' : '<div class="faint">Aucune information enregistrée.</div>';
    list.forEach((m) => {
      const row = el(`<div class="row"><span class="grow">${esc(m.content)}</span><button class="icon-btn" title="Oublier">${icon('trash')}</button></div>`);
      row.querySelector('button').onclick = async () => drawMems(await invoke('delete_memory', { id: m.id }));
      box.appendChild(row);
    });
  };
  const addMem = async () => { const inp = me.querySelector('#mem'); if (!inp.value.trim()) return; drawMems(await invoke('add_memory', { content: inp.value })); inp.value = ''; };
  me.querySelector('#addmem').onclick = addMem;
  me.querySelector('#mem').onkeydown = (e) => e.key === 'Enter' && addMem();
  drawMems(ctx.state.memories || []);

  // Fichiers autorisés
  const fi = group('Fichiers autorisés', 'folder', `<div class="faint">Dossiers dans lesquels NOVA peut chercher et ouvrir des fichiers. Rien d’autre n’est accessible.</div><div id="folders" class="stack" style="gap:4px"></div><div><button class="btn small" id="addf">${icon('plus')} Ajouter un dossier</button></div>`);
  const drawFolders = () => {
    const box = fi.querySelector('#folders');
    const list = folders(ctx);
    box.innerHTML = list.length ? '' : '<div class="faint">Aucun dossier autorisé.</div>';
    list.forEach((f) => {
      const row = el(`<div class="row">${icon('folder')}<span class="grow mono" style="font-size:12.5px">${esc(f)}</span><button class="btn small">Retirer</button></div>`);
      row.querySelector('button').onclick = async () => { await removeFolder(ctx, f); drawFolders(); };
      box.appendChild(row);
    });
  };
  fi.querySelector('#addf').onclick = async () => { if (await addFolder(ctx)) drawFolders(); };
  drawFolders();

  // Outils et calcul
  const to = group('Outils et calcul', 'fx', `<label class="row"><input type="checkbox" id="tools"> Utiliser automatiquement les outils (calculs, conversions, météo, fichiers, actions)</label><div class="row"><span style="width:150px">Unité d’angle</span><select class="field" id="angle"><option value="radians">Radians</option><option value="degrees">Degrés</option></select></div><div class="row"><span style="width:150px">Ville par défaut</span><input class="field" id="city" placeholder="Pour « météo » sans ville" style="width:240px"></div>`);
  const tools = to.querySelector('#tools');
  tools.checked = s.toolsEnabled !== '0';
  tools.onchange = () => ctx.setSetting('toolsEnabled', tools.checked ? '1' : '0');
  const angle = to.querySelector('#angle');
  angle.value = s.angleMode || 'radians';
  angle.onchange = () => ctx.setSetting('angleMode', angle.value);
  bindText(to.querySelector('#city'), 'defaultCity');

  // Voix
  const vo = group('Voix', 'volume', `<div class="row"><span style="width:150px">Voix de lecture</span><select class="field grow" id="voice"></select></div><div class="row"><span style="width:150px">Débit</span><input type="range" id="rate" min="0.6" max="1.6" step="0.1" style="width:220px"><button class="btn small" id="test">Tester</button></div><div class="faint">Voix installées sur ton système, utilisées localement. ${ctx.isMac() ? 'La dictée utilise la reconnaissance vocale du Mac ; le micro ne s’active que quand tu cliques sur le bouton micro, et rien n’est enregistré.' : 'La dictée sur Windows arrivera dans une prochaine version.'}</div>`);
  const voiceSel = vo.querySelector('#voice');
  const fillVoices = () => {
    const voices = speechSynthesis.getVoices().filter((x) => x.lang.startsWith('fr'));
    voiceSel.innerHTML = voices.map((x) => `<option ${x.name === s.voice ? 'selected' : ''}>${esc(x.name)}</option>`).join('') || '<option>Voix par défaut</option>';
  };
  fillVoices();
  speechSynthesis.onvoiceschanged = fillVoices;
  voiceSel.onchange = () => ctx.setSetting('voice', voiceSel.value);
  const rate = vo.querySelector('#rate');
  rate.value = s.voiceRate || '1';
  rate.onchange = () => ctx.setSetting('voiceRate', rate.value);
  vo.querySelector('#test').onclick = () => {
    speechSynthesis.cancel();
    const u = new SpeechSynthesisUtterance('Bonjour, je suis NOVA, ton assistant personnel.');
    u.lang = 'fr-FR'; u.rate = Number(rate.value);
    u.voice = speechSynthesis.getVoices().find((x) => x.name === voiceSel.value) || null;
    speechSynthesis.speak(u);
  };

  // Confidentialité
  const pr = group('Confidentialité et réseau', 'shield', `<div class="stack" style="gap:6px;font-size:13px">
    <div>✅ Conversations, réglages et mémoire : stockés sur cet ordinateur (SQLite).</div>
    <div>✅ IA : exécutée localement (llama.cpp${ctx.isMac() ? ', modèle Apple' : ''}${ctx.state.models.some((m) => m.provider === 'ollama') ? ', Ollama' : ''}).</div>
    <div>✅ Texte des PDF et images : extrait localement.</div>
    <div>🌐 Envoyé sur Internet uniquement quand tu le demandes : météo (Open-Meteo), mode Internet et questions d'actualité (Wikipédia), téléchargement de modèles (GitHub, Hugging Face). Chaque réponse concernée porte le badge « Internet ».</div>
    <div>🛡️ Actions sur l’ordinateur : toujours affichées et soumises à ton accord avant exécution.</div></div>`);
  pr.appendChild(el(`<label class="row"><input type="checkbox" id="autoweb"> Vérifier automatiquement sur Wikipédia les questions d'actualité (dirigeants, « actuel »…)</label>`));
  pr.appendChild(el(`<div class="faint">Seule la question est envoyée à Wikipédia, et la source est affichée sous la réponse. Désactivé : NOVA répond avec les connaissances du modèle, qui peuvent dater.</div>`));
  const aw = pr.querySelector('#autoweb');
  aw.checked = s.autoWeb !== '0';
  aw.onchange = () => ctx.setSetting('autoWeb', aw.checked ? '1' : '0');

  // Données
  const da = group('Données', 'download', `<div class="row"><button class="btn" id="export">Exporter toutes les données (JSON)…</button><button class="btn" id="activity">Journal d’activité</button><button class="btn danger" id="wipe">Tout supprimer…</button></div><div class="faint">Notes créées par NOVA : ${esc(ctx.state.notesDir || '')}</div>`);
  da.querySelector('#export').onclick = async () => {
    const path = await dialog.save({ defaultPath: `NOVA-export-${new Date().toISOString().slice(0, 10)}.json`, filters: [{ name: 'JSON', extensions: ['json'] }] });
    if (!path) return;
    try { await invoke('export_data', { path }); toast('Export terminé.'); } catch (e) { toast(errorText(e), 'error'); }
  };
  da.querySelector('#activity').onclick = () => ctx.setView('activity');
  da.querySelector('#wipe').onclick = async () => {
    if (!(await confirmDialog('Supprimer toutes les conversations et pièces jointes ? La mémoire et les réglages sont conservés. Cette action est irréversible.'))) return;
    await invoke('delete_all_conversations');
    ctx.state.convId = null; ctx.state.messages = [];
    await ctx.reloadConversations();
    toast('Conversations supprimées.');
  };

  // Mises à jour
  const up = group('Mises à jour', 'download', `<div class="row"><span style="width:150px">Version installée</span><b id="ver">…</b></div><label class="row"><input type="checkbox" id="auto"> Rechercher automatiquement les mises à jour</label><div class="row"><button class="btn" id="checknow">${icon('refresh')} Rechercher maintenant</button><button class="btn" id="releases">${icon('external')} Notes de version (GitHub)</button></div><div class="faint">Les nouvelles versions sont publiées sur GitHub et signées : NOVA vérifie la signature avant d'installer. Tes données sont conservées.</div>`);
  ctx.currentVersion().then((v) => (up.querySelector('#ver').textContent = v));
  const auto = up.querySelector('#auto');
  auto.checked = s.autoUpdate !== '0';
  auto.onchange = () => ctx.setSetting('autoUpdate', auto.checked ? '1' : '0');
  up.querySelector('#checknow').onclick = () => ctx.checkForUpdates(true);
  up.querySelector('#releases').onclick = () => invoke('open_url', { url: 'https://github.com/Nolek0/nova/releases' });

  // Raccourcis et à propos
  const mod = ctx.isMac() ? '⌘' : 'Ctrl+';
  group('Raccourcis clavier', 'code', `<table class="data"><tr><td class="mono">${mod}N</td><td>Nouvelle conversation</td></tr><tr><td class="mono">Entrée</td><td>Envoyer</td></tr><tr><td class="mono">Maj+Entrée</td><td>Nouvelle ligne</td></tr><tr><td class="mono">${mod}.</td><td>Arrêter la génération</td></tr><tr><td class="mono">${mod}${ctx.isMac() ? '⇧' : 'Maj+'}O</td><td>Joindre un fichier</td></tr><tr><td class="mono">${mod},</td><td>Paramètres</td></tr></table>`);
  const about = group('À propos', 'info', `<div>NOVA · macOS et Windows · <a href="#" id="gh">github.com/Nolek0/nova</a></div><div class="faint">Cœur en Rust (Tauri), IA locale via llama.cpp, modèles libres (Google Gemma, Alibaba Qwen, OpenAI gpt-oss). Gratuit et sans compte.</div>`);
  about.querySelector('#gh').onclick = (e) => { e.preventDefault(); invoke('open_url', { url: 'https://github.com/Nolek0/nova' }); };

  function bindText(input, key) {
    input.value = s[key] || '';
    let t;
    input.oninput = () => { clearTimeout(t); t = setTimeout(() => ctx.setSetting(key, input.value), 300); };
  }
}
