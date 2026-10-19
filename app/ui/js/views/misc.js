// Bibliothèque de fichiers, galerie des fonctionnalités, journal d'activité.
import { invoke, fileSrc, esc, el, formatDate, toast, errorText } from '../api.js';
import { icon } from '../icons.js';

export async function renderLibrary(v, ctx) {
  const page = el(`<div class="page" style="max-width:1100px"><h1>Bibliothèque</h1><p class="sub">Fichiers et images joints à tes conversations, conservés sur cet ordinateur.</p><div class="tabs"></div><div class="tiles"></div></div>`);
  v.appendChild(page);
  const items = await invoke('library');
  let filter = null;
  const tabs = page.querySelector('.tabs');
  const draw = () => {
    tabs.innerHTML = '';
    [[null, 'Tout'], ['image', 'Images'], ['pdf', 'PDF'], ['text', 'Textes']].forEach(([k, l]) => {
      const b = el(`<button class="tab ${filter === k ? 'active' : ''}">${l}</button>`);
      b.onclick = () => { filter = k; draw(); };
      tabs.appendChild(b);
    });
    const grid = page.querySelector('.tiles');
    grid.innerHTML = '';
    const shown = items.filter((i) => !filter || i.attachment.kind === filter);
    if (!shown.length) grid.appendChild(el('<div class="faint">Aucun fichier pour l’instant. Glisse un fichier dans une conversation pour l’ajouter.</div>'));
    shown.forEach(({ attachment: a, conversation_id, date }) => {
      const t = el(`<button class="tile"><div class="pv">${a.kind === 'image' ? `<img src="${fileSrc(a.path)}" alt="">` : icon(a.kind === 'pdf' ? 'filetext' : 'file')}</div><b>${esc(a.filename)}</b><small>${formatDate(date)}</small></button>`);
      t.onclick = () => ctx.openConversation(conversation_id);
      t.ondblclick = () => invoke('open_path', { path: a.path, reveal: false }).catch((e) => toast(errorText(e), 'error'));
      t.title = 'Clic : ouvrir la conversation · double-clic : ouvrir le fichier';
      grid.appendChild(t);
    });
  };
  draw();
}

export function renderGallery(v, ctx) {
  const mac = ctx.isMac();
  const hasModel = ctx.state.models.length > 0;
  const sections = [
    ['Disponibles', [
      ['Conversation avec une IA locale', hasModel ? 'Réponses en direct, historique enregistré sur l’ordinateur.' : 'Installe un modèle gratuit dans la page Modèles.', 'sparkles', hasModel ? 'ready' : 'setup', () => (hasModel ? ctx.setView('home') : ctx.setView('models'))],
      ['Équations et systèmes', 'Étapes détaillées et vérification de chaque solution.', 'sigma', 'ready', () => ctx.setView('tools', { tab: 'equations' })],
      ['Dérivées symboliques', 'Avec vérification numérique.', 'fx', 'ready', () => ctx.setView('tools', { tab: 'derivative' })],
      ['Graphiques de fonctions', 'Courbe, dérivée et zéros.', 'chart', 'ready', () => ctx.setView('tools', { tab: 'graph' })],
      ['Statistiques et probabilités', 'Moyenne, quartiles, écart-type, loi binomiale.', 'bars', 'ready', () => ctx.setView('tools', { tab: 'stats' })],
      ['Matrices', 'Déterminant, inverse, produit, transposée.', 'matrix', 'ready', () => ctx.setView('tools', { tab: 'matrices' })],
      ['Conversions d’unités', '12 grandeurs, plus de 80 unités.', 'arrows', 'ready', () => ctx.setView('tools', { tab: 'units' })],
      ['Météo', 'Actuelle, heure par heure et 7 jours, source affichée.', 'weather', 'ready', () => ctx.setView('weather')],
      ['Recherche de fichiers', 'Dans les dossiers que tu autorises.', 'search', 'ready', () => ctx.setView('files')],
      ['Actions sur l’ordinateur', 'Ouvrir une app, un site, un fichier, créer une note — avec ton accord.', 'shield', 'ready', () => ctx.quickStart('ouvre ')],
      ['Mode Internet', 'Réponses appuyées sur Wikipédia, avec sources citées.', 'globe', 'ready', () => ctx.setView('home')],
      ['Lecture de documents', 'PDF, textes et code, extraits localement.', 'filetext', 'ready', () => ctx.pickFiles()],
      ['Images', 'Analysées par un modèle de vision local' + (mac ? ' ou lues par OCR' : '') + '.', 'image', 'ready', () => ctx.pickFiles(true)],
      mac ? ['Photo avec l’iPhone', 'Caméra de continuité, depuis le bouton caméra.', 'phone', 'ready', () => ctx.setView('home')] : ['Photo avec la webcam', 'Depuis le bouton caméra.', 'camera', 'ready', () => ctx.setView('home')],
      ['Lecture vocale', 'Les réponses lues avec les voix du système.', 'volume', 'ready', null],
      mac ? ['Dictée', 'Reconnaissance vocale locale du Mac.', 'mic', 'ready', null] : null,
      ['Mémoire volontaire', 'Tu choisis ce que NOVA retient.', 'brain', 'ready', () => ctx.setView('settings')],
      ['Recherche dans l’historique', 'Plein texte, insensible aux accents.', 'search', 'ready', null],
    ].filter(Boolean)],
    ['Prochaines versions', [
      ['Calcul formel avancé', 'Intégrales, limites, factorisation.', 'sum', 'planned', null],
      ['Dictée sur Windows', 'Reconnaissance vocale locale (Whisper).', 'mic', 'planned', null],
      ['Horloge mondiale, minuteur, devises', 'Fuseaux horaires et taux de la BCE.', 'info', 'planned', null],
      ['Notes, tâches et rappels', 'Stockés localement.', 'list', 'planned', null],
      ['Exécution de code contrôlée', 'Toujours affichée et validée avant exécution.', 'code', 'planned', null],
    ]],
  ];
  const page = el(`<div class="page" style="max-width:1100px"><h1>Fonctionnalités</h1><p class="sub">Ce que NOVA sait faire aujourd’hui, et ce qui arrive ensuite.</p></div>`);
  v.appendChild(page);
  for (const [title, list] of sections) {
    page.appendChild(el(`<h2 style="margin-top:14px">${title}</h2>`));
    const grid = el('<div class="grid-cards"></div>');
    for (const [t, d, ic, status, fn] of list) {
      const color = { ready: 'var(--success)', setup: 'var(--warning)', planned: 'var(--text-3)' }[status];
      const b = el(`<button class="feature" ${fn ? '' : 'disabled'}><span class="ic">${icon(ic)}</span><span><b>${esc(t)} <span class="dot" style="background:${color}"></span></b><small>${esc(d)}</small></span></button>`);
      if (fn) b.onclick = fn;
      grid.appendChild(b);
    }
    page.appendChild(grid);
  }
}

export async function renderActivity(v) {
  const page = el(`<div class="page" style="max-width:1100px"><div class="row" style="align-items:flex-start"><div class="grow"><h1>Activité</h1><p class="sub">Journal des outils exécutés, des actions et des opérations sur tes données.</p></div><button class="btn" id="clear">Effacer le journal</button></div><div id="list"></div></div>`);
  v.appendChild(page);
  const draw = async () => {
    const entries = await invoke('activity_log');
    const box = page.querySelector('#list');
    if (!entries.length) { box.innerHTML = '<div class="faint">Aucune activité enregistrée pour l’instant.</div>'; return; }
    const label = { free: 'Local', network: 'Internet', confirm: 'Confirmation', high: 'Sensible' };
    box.innerHTML = `<div class="card" style="padding:0;overflow:auto"><table class="data"><tr><th>Date</th><th>Outil</th><th>Détail</th><th>Niveau</th><th>Résultat</th></tr>${entries.map((e) => `<tr><td>${esc(formatDate(e.date))}</td><td>${esc(e.tool)}</td><td>${esc(e.summary)}</td><td>${esc(label[e.risk] || e.risk)}</td><td>${esc(e.outcome)}</td></tr>`).join('')}</table></div>`;
  };
  page.querySelector('#clear').onclick = async () => { await invoke('clear_activity'); draw(); };
  draw();
}
