// Rendu Markdown sécurisé : marked (analyse), DOMPurify (nettoyage), highlight.js (code).
import { esc, invoke } from './api.js';
import { icon } from './icons.js';

const renderer = {
  code({ text, lang }) {
    const language = (lang || '').split(/\s/)[0].toLowerCase();
    let html;
    try {
      html = language && window.hljs.getLanguage(language)
        ? window.hljs.highlight(text, { language }).value
        : window.hljs.highlightAuto(text).value;
    } catch { html = esc(text); }
    return `<div class="code-block"><div class="code-head"><span>${esc(language || 'code')}</span><button data-copy>${icon('copy')}<span>Copier</span></button></div><pre><code class="hljs">${html}</code></pre></div>`;
  },
};
window.marked.use({ renderer, gfm: true, breaks: false });

export function renderMarkdown(text) {
  const raw = window.marked.parse(text || '');
  return window.DOMPurify.sanitize(raw, { ADD_ATTR: ['data-copy'], FORBID_TAGS: ['style', 'form', 'input', 'iframe'] });
}

/** Active les boutons « Copier » des blocs de code et ouvre les liens dans le navigateur. */
export function wireMarkdown(root) {
  root.querySelectorAll('[data-copy]').forEach((btn) => {
    btn.onclick = async () => {
      const code = btn.closest('.code-block').querySelector('code').innerText;
      await navigator.clipboard.writeText(code);
      btn.innerHTML = `${icon('check')}<span>Copié</span>`;
      setTimeout(() => (btn.innerHTML = `${icon('copy')}<span>Copier</span>`), 1500);
    };
  });
  // Dessins de Nova 1 : bouton pour les enregistrer en PNG.
  root.querySelectorAll('img[src^="data:image/svg+xml"]').forEach((img) => {
    if (img.parentElement?.classList.contains('drawing')) return;
    const wrap = document.createElement('div');
    wrap.className = 'drawing';
    img.replaceWith(wrap);
    wrap.appendChild(img);
    const btn = document.createElement('button');
    btn.className = 'btn small';
    btn.innerHTML = `${icon('download')}<span>Enregistrer en PNG</span>`;
    btn.onclick = async () => {
      const path = await window.__TAURI__.dialog.save({ defaultPath: 'dessin-nova.png', filters: [{ name: 'Image PNG', extensions: ['png'] }] });
      if (!path) return;
      const im = new Image();
      im.src = img.src;
      await im.decode();
      const c = document.createElement('canvas');
      c.width = (im.naturalWidth || 640) * 2;
      c.height = (im.naturalHeight || 440) * 2;
      c.getContext('2d').drawImage(im, 0, 0, c.width, c.height);
      const b64 = c.toDataURL('image/png').split(',')[1];
      try { await invoke('save_base64_file', { path, data: b64 }); btn.innerHTML = `${icon('check')}<span>Enregistré</span>`; } catch (e) { btn.textContent = String(e); }
    };
    wrap.appendChild(btn);
  });
  root.querySelectorAll('a[href]').forEach((a) => {
    a.onclick = (e) => {
      e.preventDefault();
      if (/^https?:/.test(a.href)) invoke('open_url', { url: a.href });
    };
  });
}
