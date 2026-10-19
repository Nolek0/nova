// Fichiers : recherche limitée aux dossiers que l'utilisateur autorise ici.
import { invoke, dialog, esc, el, toast, errorText, formatBytes, formatDate } from '../api.js';
import { icon } from '../icons.js';

export function folders(ctx) {
  try { return JSON.parse(ctx.state.settings.allowedFolders || '[]'); } catch { return []; }
}

export async function addFolder(ctx) {
  const dir = await dialog.open({ directory: true, multiple: false, title: 'Choisis un dossier que NOVA pourra consulter' });
  if (!dir) return false;
  const list = await invoke('set_allowed_folders', { folders: [...folders(ctx), dir] });
  ctx.state.settings.allowedFolders = JSON.stringify(list);
  return true;
}

export async function removeFolder(ctx, f) {
  const list = await invoke('set_allowed_folders', { folders: folders(ctx).filter((x) => x !== f) });
  ctx.state.settings.allowedFolders = JSON.stringify(list);
}

export function render(v, ctx) {
  const page = el(`<div class="page"><h1>Fichiers</h1><p class="sub">NOVA ne cherche que dans les dossiers que tu ajoutes ici. Tout reste sur ton ordinateur.</p>
    <div class="card stack"><h2>${icon('shield')} Dossiers autorisés</h2><div id="folders" class="stack" style="gap:6px"></div><div><button class="btn small" id="add">${icon('plus')} Ajouter un dossier</button></div></div>
    <div class="row" style="margin-top:18px"><input class="field grow" id="q" placeholder="Nom de fichier ou mots contenus dans un document…"><label class="row faint" style="gap:6px"><input type="checkbox" id="content" checked> chercher aussi dans le contenu</label><button class="btn primary" id="go">${icon('search')} Rechercher</button></div>
    <div id="results" style="margin-top:14px"></div></div>`);
  v.appendChild(page);
  const drawFolders = () => {
    const box = page.querySelector('#folders');
    const list = folders(ctx);
    box.innerHTML = list.length ? '' : '<div class="faint">Aucun dossier pour l’instant. Ajoute par exemple Documents ou Téléchargements.</div>';
    list.forEach((f) => {
      const row = el(`<div class="row">${icon('folder')}<span class="grow mono" style="font-size:12.5px">${esc(f)}</span><button class="btn small">Retirer</button></div>`);
      row.querySelector('button').onclick = async () => { await removeFolder(ctx, f); drawFolders(); };
      box.appendChild(row);
    });
  };
  page.querySelector('#add').onclick = async () => { if (await addFolder(ctx)) drawFolders(); };
  const q = page.querySelector('#q');
  const run = async () => {
    const res = page.querySelector('#results');
    if (!folders(ctx).length) { toast('Ajoute d’abord un dossier autorisé.'); return; }
    if (!q.value.trim()) return;
    res.innerHTML = '<div class="faint">Recherche…</div>';
    try {
      const hits = await invoke('search_files', { query: q.value, content: page.querySelector('#content').checked });
      res.innerHTML = hits.length ? `<div class="faint" style="margin-bottom:6px">${hits.length} résultat(s)</div>` : '<div class="faint">Aucun résultat.</div>';
      const list = el('<div class="file-list card" style="padding:6px"></div>');
      hits.forEach((f) => {
        const row = el(`<button class="file-row">${icon(f.is_dir ? 'folder' : 'file')}<span style="min-width:0;flex:1"><div class="name">${esc(f.name)}</div><div class="where">${esc(f.folder)} · ${formatBytes(f.size)} · ${formatDate(f.modified)}</div>${f.snippet ? `<div class="snip">${esc(f.snippet)}</div>` : ''}</span><span class="icon-btn" title="Afficher dans le dossier">${icon('folder')}</span><span class="icon-btn" title="Joindre à une conversation" data-attach>${icon('clip')}</span></button>`);
        row.onclick = (e) => {
          if (e.target.closest('[data-attach]')) { ctx.addAttachmentFromPath(f.path); return; }
          invoke('open_path', { path: f.path, reveal: !!e.target.closest('.icon-btn') }).catch((err) => toast(errorText(err), 'error'));
        };
        list.appendChild(row);
      });
      if (hits.length) res.appendChild(list);
    } catch (e) { res.innerHTML = ''; toast(errorText(e), 'error'); }
  };
  q.onkeydown = (e) => e.key === 'Enter' && run();
  page.querySelector('#go').onclick = run;
  drawFolders();
  q.focus();
}
