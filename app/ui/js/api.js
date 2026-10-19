// Accès au cœur Rust (Tauri). Toute la logique métier est côté Rust ; l'interface ne fait qu'afficher.
const T = window.__TAURI__;

export const invoke = (cmd, args = {}) => T.core.invoke(cmd, args);
export const Channel = T.core.Channel;
export const listen = (name, cb) => T.event.listen(name, cb);
export const fileSrc = (path) => T.core.convertFileSrc(path);
export const dialog = T.dialog;
export const currentWebview = () => T.webview.getCurrentWebview();

// ---- Petites aides d'interface partagées

export function esc(s) {
  return String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
}

export function el(html) {
  const t = document.createElement('template');
  t.innerHTML = html.trim();
  return t.content.firstElementChild;
}

let toastTimer;
export function toast(message, kind = 'info') {
  document.querySelector('.toast')?.remove();
  const node = el(`<div class="toast ${kind}" role="status">${kind === 'error' ? '⚠️' : ''} <span>${esc(message)}</span></div>`);
  document.body.appendChild(node);
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => node.remove(), kind === 'error' ? 6000 : 2800);
}

export function errorText(e) {
  return typeof e === 'string' ? e : e?.message || JSON.stringify(e);
}

/** Menu contextuel positionné sous `anchor`. items: [{label, icon, action, check, danger}] | 'sep' | {header} */
export function showMenu(anchor, items, { align = 'left', above = false } = {}) {
  closeMenu();
  const menu = el('<div class="menu" role="menu"></div>');
  for (const it of items) {
    if (it === 'sep') { menu.appendChild(el('<div class="sep"></div>')); continue; }
    if (it.header) { menu.appendChild(el(`<div class="label">${esc(it.header)}</div>`)); continue; }
    const b = el(`<button role="menuitem" ${it.disabled ? 'disabled' : ''}>${it.icon ? `<span>${it.icon}</span>` : ''}<span>${esc(it.label)}</span>${it.check ? '<span class="check">✓</span>' : ''}</button>`);
    if (it.danger) b.style.color = 'var(--danger)';
    if (it.disabled) b.style.opacity = '0.5';
    b.onclick = () => { closeMenu(); it.action?.(); };
    menu.appendChild(b);
  }
  document.body.appendChild(menu);
  const r = anchor.getBoundingClientRect();
  const mw = menu.offsetWidth, mh = menu.offsetHeight;
  let x = align === 'right' ? r.right - mw : r.left;
  let y = above ? r.top - mh - 6 : r.bottom + 6;
  if (y + mh > innerHeight - 8) y = r.top - mh - 6;
  if (y < 8) y = 8;
  menu.style.left = Math.max(8, Math.min(x, innerWidth - mw - 8)) + 'px';
  menu.style.top = y + 'px';
  setTimeout(() => document.addEventListener('mousedown', outside), 0);
  function outside(ev) { if (!menu.contains(ev.target)) closeMenu(); }
  menu._cleanup = () => document.removeEventListener('mousedown', outside);
}

export function closeMenu() {
  document.querySelectorAll('.menu').forEach((m) => { m._cleanup?.(); m.remove(); });
}

export async function confirmDialog(message, title = 'NOVA') {
  return dialog.confirm(message, { title, kind: 'warning', okLabel: 'Confirmer', cancelLabel: 'Annuler' });
}

export function formatBytes(n) {
  if (!n) return '';
  const u = ['o', 'Ko', 'Mo', 'Go', 'To'];
  let i = 0;
  while (n >= 1000 && i < u.length - 1) { n /= 1000; i++; }
  return `${n.toFixed(n < 10 && i > 0 ? 1 : 0)} ${u[i]}`;
}

export function formatDate(sec) {
  return new Date(sec * 1000).toLocaleString('fr-FR', { dateStyle: 'medium', timeStyle: 'short' });
}
