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
    showUpdate(update, modal);
  } catch (e) {
    if (manual) toast(`Impossible de vérifier les mises à jour : ${errorText(e)}`, 'error');
  } finally {
    checking = false;
  }
}

function showUpdate(update, modal) {
  const notes = (update.body || '').trim();
  modal(`<div class="row"><span class="mark big"></span><div><h3 style="font-size:17px">Nouvelle version de NOVA</h3>
      <div class="muted" style="font-size:13px">Version <b>${esc(update.version)}</b> disponible (tu as la ${esc(update.currentVersion)}).</div></div></div>
    ${notes ? `<div class="card" style="max-height:220px;overflow:auto;font-size:12.5px;white-space:pre-wrap">${esc(notes)}</div>` : ''}
    <div class="faint">Tes conversations, réglages et modèles sont conservés.</div>
    <div id="up-progress" class="hidden" style="display:grid;gap:6px"><div class="faint" id="up-label">Téléchargement…</div><div class="progress"><div></div></div></div>
    <div class="row" style="justify-content:flex-end" id="up-actions"><button class="btn" data-close>Plus tard</button><button class="btn primary" id="up-install">${icon('download')} Installer et redémarrer</button></div>`,
  (m) => {
    m.querySelector('#up-install').onclick = async () => {
      m.querySelector('#up-progress').classList.remove('hidden');
      m.querySelector('#up-actions').innerHTML = '';
      let total = 0, done = 0;
      try {
        await update.downloadAndInstall((ev) => {
          if (ev.event === 'Started') total = ev.data.contentLength || 0;
          if (ev.event === 'Progress') {
            done += ev.data.chunkLength;
            m.querySelector('.progress div').style.width = (total ? (100 * done) / total : 0) + '%';
            m.querySelector('#up-label').textContent = total ? `${(done / 1e6).toFixed(1)} Mo sur ${(total / 1e6).toFixed(1)} Mo` : `${(done / 1e6).toFixed(1)} Mo`;
          }
          if (ev.event === 'Finished') m.querySelector('#up-label').textContent = 'Installation…';
        });
        m.querySelector('#up-label').textContent = 'Redémarrage de NOVA…';
        await T.process.relaunch();
      } catch (e) {
        m.querySelector('#up-label').textContent = `Échec : ${errorText(e)}`;
        m.querySelector('#up-actions').innerHTML = '<button class="btn" data-close>Fermer</button>';
        m.querySelector('[data-close]').onclick = () => m.remove();
      }
    };
  });
}

/** Vérification automatique : au démarrage puis toutes les 6 heures (désactivable). */
export function scheduleAutoCheck(state, modal) {
  const run = () => { if (state.settings.autoUpdate !== '0') checkForUpdates(false, { modal }); };
  setTimeout(run, 8000);
  setInterval(run, 6 * 3600 * 1000);
}

export function updateBadge() {
  return pending ? el(`<span class="badge accent">${icon('download')} ${esc(pending.version)}</span>`) : null;
}
