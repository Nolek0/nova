// Mises à jour automatiques depuis les releases GitHub (paquets signés : seules les versions
// publiées avec la clé de NOVA peuvent s'installer).
import { esc, el, toast, errorText } from './api.js';
import { icon } from './icons.js';

const T = window.__TAURI__;
let checking = false;
let pending = null;

export async function currentVersion() {
  try { return await T.app.getVersion(); } catch { return ''; }
}

/** manual = demandé par l'utilisateur (affiche aussi « à jour » et les erreurs). */
export async function checkForUpdates(manual = false, { modal } = {}) {
  if (checking) return;
  checking = true;
  try {
    const update = await T.updater.check();
    if (!update) {
      if (manual) toast(`NOVA est à jour (version ${await currentVersion()}).`);
      return;
    }
    pending = update;
    showUpdate(update);
  } catch (e) {
    if (manual) toast(`Impossible de vérifier les mises à jour : ${errorText(e)}`, 'error');
  } finally {
    checking = false;
  }
}

/** Petite carte en bas à droite, qui glisse doucement : « Nouvelle version disponible ». */
function showUpdate(update) {
  document.querySelector('.update-pop')?.remove();
  const pop = el(`<div class="update-pop" role="status">
    <div class="row"><span class="mark"></span><div class="grow"><b>Nouvelle version de NOVA</b><div class="faint">Version ${esc(update.version)} disponible · tu as la ${esc(update.currentVersion)}</div></div>
      <button class="icon-btn" data-x title="Plus tard">${icon('x')}</button></div>
    <div class="up-progress hidden"><div class="faint up-label">Téléchargement…</div><div class="progress"><div></div></div></div>
    <div class="row up-actions"><button class="btn small" data-later>Plus tard</button><span class="grow"></span><button class="btn small primary" data-install>${icon('download')} Mettre à jour</button></div>
  </div>`);
  document.body.appendChild(pop);
  requestAnimationFrame(() => requestAnimationFrame(() => pop.classList.add('in')));
  const close = () => { pop.classList.remove('in'); setTimeout(() => pop.remove(), 400); };
  pop.querySelector('[data-x]').onclick = close;
  pop.querySelector('[data-later]').onclick = close;
  pop.querySelector('[data-install]').onclick = async () => {
    const label = pop.querySelector('.up-label');
    const bar = pop.querySelector('.progress div');
    pop.querySelector('.up-progress').classList.remove('hidden');
    pop.querySelector('.up-actions').classList.add('hidden');
    pop.querySelector('[data-x]').classList.add('hidden');
    let total = 0, done = 0;
    try {
      await update.downloadAndInstall((ev) => {
        if (ev.event === 'Started') total = ev.data.contentLength || 0;
        if (ev.event === 'Progress') {
          done += ev.data.chunkLength;
          bar.style.width = (total ? (100 * done) / total : 0) + '%';
          label.textContent = total ? `${(done / 1e6).toFixed(1)} Mo sur ${(total / 1e6).toFixed(1)} Mo` : `${(done / 1e6).toFixed(1)} Mo`;
        }
        if (ev.event === 'Finished') { bar.style.width = '100%'; label.textContent = 'Installation…'; }
      });
      label.textContent = 'Redémarrage de NOVA…';
      await T.process.relaunch();
    } catch (e) {
      label.textContent = `Échec : ${errorText(e)}`;
      pop.querySelector('[data-x]').classList.remove('hidden');
    }
  };
}

/** Vérification automatique : une seule fois, au lancement de NOVA (désactivable dans les Paramètres). */
export function scheduleAutoCheck(state, modal) {
  setTimeout(() => { if (state.settings.autoUpdate !== '0') checkForUpdates(false, { modal }); }, 3000);
}

export function updateBadge() {
  return pending ? el(`<span class="badge accent">${icon('download')} ${esc(pending.version)}</span>`) : null;
}
