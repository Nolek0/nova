// Langues de l'interface.
// L'interface est écrite en français ; chaque texte affiché (et les attributs title, placeholder,
// aria-label) est traduit à la volée grâce aux dictionnaires de js/locales/<code>.js.
// Les contenus (réponses de l'IA, messages, Markdown, champs de saisie) ne sont jamais traduits.

const T = window.__TAURI__;

/** Langues de l'interface : code, nom natif, traduction à faire relire. */
export const UI_LANGUAGES = [
  { code: 'en', name: 'English' },
  { code: 'fr', name: 'Français' },
  { code: 'es', name: 'Español' },
  { code: 'pt', name: 'Português' },
  { code: 'de', name: 'Deutsch' },
  { code: 'it', name: 'Italiano' },
  { code: 'nl', name: 'Nederlands' },
  { code: 'sw', name: 'Kiswahili', review: true },
  { code: 'ln', name: 'Lingála', review: true },
  { code: 'ht', name: 'Kreyòl ayisyen', review: true },
];

/** Langues proposées pour les réponses de l'IA (toute langue peut aussi être saisie librement). */
export const REPLY_LANGUAGES = [
  'Afrikaans', 'Albanais', 'Allemand', 'Amharique', 'Anglais', 'Arabe', 'Arabe marocain (darija)', 'Arménien', 'Azéri', 'Bambara', 'Basque',
  'Bengali', 'Biélorusse', 'Birman', 'Bosnien', 'Breton', 'Bulgare', 'Catalan', 'Cebuano', 'Chinois (mandarin)', 'Chinois (cantonais)', 'Coréen',
  'Corse', 'Créole antillais (Martinique, Guadeloupe)', 'Créole guyanais', 'Créole haïtien', 'Créole mauricien', 'Créole réunionnais', 'Créole cap-verdien',
  'Croate', 'Danois', 'Dioula', 'Espagnol', 'Espéranto', 'Estonien', 'Éwé', 'Féroïen', 'Fidjien', 'Finnois', 'Flamand', 'Fon', 'Français', 'Frison',
  'Gaélique écossais', 'Galicien', 'Gallois', 'Géorgien', 'Grec', 'Guarani', 'Gujarati', 'Haoussa', 'Hawaïen', 'Hébreu', 'Hindi', 'Hongrois', 'Igbo',
  'Indonésien', 'Irlandais', 'Islandais', 'Italien', 'Japonais', 'Javanais', 'Kabyle', 'Kannada', 'Kazakh', 'Khmer', 'Kikongo', 'Kinyarwanda',
  'Kirghize', 'Kirundi', 'Kurde', 'Lao', 'Latin', 'Letton', 'Lingala', 'Lituanien', 'Luxembourgeois', 'Macédonien', 'Malais', 'Malayalam', 'Malgache',
  'Maltais', 'Maori', 'Marathi', 'Mongol', 'Moore', 'Napolitain', 'Néerlandais', 'Népalais', 'Norvégien', 'Occitan', 'Oromo', 'Ouïghour', 'Ourdou',
  'Ouzbek', 'Pachto', 'Pendjabi', 'Persan', 'Peul (fula)', 'Polonais', 'Portugais', 'Portugais (Brésil)', 'Quechua', 'Roumain', 'Russe', 'Samoan',
  'Sango', 'Sanskrit', 'Serbe', 'Sesotho', 'Shona', 'Sicilien', 'Sindhi', 'Singhalais', 'Slovaque', 'Slovène', 'Somali', 'Soundanais', 'Suédois',
  'Swahili', 'Tadjik', 'Tagalog', 'Tahitien', 'Tamoul', 'Tamazight', 'Tatar', 'Tchèque', 'Telugu', 'Thaï', 'Tibétain', 'Tigrigna', 'Tshiluba',
  'Tswana', 'Turc', 'Turkmène', 'Twi', 'Ukrainien', 'Vietnamien', 'Wallon', 'Wolof', 'Xhosa', 'Yiddish', 'Yoruba', 'Zoulou',
];

let dict = null;
let lang = 'fr';
let observer = null;
// Contenus jamais traduits (réponses, messages, code, valeurs saisies).
const SKIP = '.md, .bubble, .reasoning-text, pre, code, [translate="no"], .q-user, .q-ai, .hit small, .snip, .file-row .where';

export function currentLanguage() {
  return lang;
}

/** Langue effective : réglage, sinon langue du système, sinon anglais. */
export function resolve(setting) {
  const wanted = setting && setting !== 'system' ? setting : (navigator.languages?.[0] || navigator.language || 'en');
  const base = wanted.toLowerCase().split(/[-_]/)[0];
  return UI_LANGUAGES.some((l) => l.code === base) ? base : 'en';
}

/** Traduit une chaîne française (utilisable directement dans le code). */
export function t(fr) {
  if (!dict || typeof fr !== 'string') return fr;
  return dict[fr] ?? dict[fr.trim()] ?? fr;
}

function translateText(node) {
  if (node.parentElement?.closest('textarea, input, select option[translate="no"]')) return;
  const raw = node.nodeValue;
  const s = raw.trim();
  if (!s || !dict) return;
  const tr = dict[s];
  if (tr !== undefined && tr !== s) node.nodeValue = raw.replace(s, tr);
}

function translateElement(el) {
  for (const attr of ['title', 'placeholder', 'aria-label']) {
    const v = el.getAttribute?.(attr);
    if (v && dict[v] !== undefined) el.setAttribute(attr, dict[v]);
  }
}

function walk(root) {
  if (!dict || !root) return;
  if (root.nodeType === Node.TEXT_NODE) {
    if (!root.parentElement?.closest(SKIP)) translateText(root);
    return;
  }
  if (root.nodeType !== Node.ELEMENT_NODE || root.closest?.(SKIP)) return;
  translateElement(root);
  const it = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, {
    acceptNode: (n) => ((n.nodeType === Node.ELEMENT_NODE ? n : n.parentElement)?.closest(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
  });
  let n;
  while ((n = it.nextNode())) {
    if (n.nodeType === Node.TEXT_NODE) translateText(n);
    else translateElement(n);
  }
}

/** Charge une langue et traduit toute l'interface, y compris ce qui s'affichera ensuite. */
export async function setLanguage(code) {
  lang = code;
  document.documentElement.lang = code;
  if (code === 'fr') {
    dict = null;
  } else {
    try {
      dict = (await import(`./locales/${code}.js`)).default;
    } catch {
      dict = (await import('./locales/en.js')).default;
    }
  }
  observer?.disconnect();
  if (dict) {
    walk(document.body);
    observer = new MutationObserver((muts) => {
      for (const m of muts) {
        if (m.type === 'characterData') walk(m.target);
        else if (m.type === 'attributes') translateElement(m.target);
        else m.addedNodes.forEach(walk);
      }
    });
    observer.observe(document.body, { childList: true, subtree: true, characterData: true, attributes: true, attributeFilter: ['title', 'placeholder', 'aria-label'] });
  }
  // Menu de l'application dans la même langue.
  const labels = {};
  for (const [k, v] of Object.entries(dict || {})) if (k.startsWith('menu.') || k.startsWith('about.')) labels[k] = v;
  try { await T.core.invoke('set_menu_labels', { labels }); } catch {}
}
