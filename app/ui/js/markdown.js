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
  root.querySelectorAll('a[href]').forEach((a) => {
    a.onclick = (e) => {
      e.preventDefault();
      if (/^https?:/.test(a.href)) invoke('open_url', { url: a.href });
    };
  });
}
