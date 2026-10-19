// Boîte à outils : chaque outil fonctionne seul, localement, sans modèle d'IA.
import { invoke, esc, el, errorText } from '../api.js';
import { icon } from '../icons.js';
import { renderMarkdown, wireMarkdown } from '../markdown.js';

const TABS = [
  ['calculator', 'Calculatrice', 'calc'],
  ['equations', 'Équations', 'sigma'],
  ['derivative', 'Dérivées', 'fx'],
  ['graph', 'Graphique', 'chart'],
  ['units', 'Conversions', 'arrows'],
  ['stats', 'Statistiques', 'bars'],
  ['matrices', 'Matrices', 'matrix'],
];

let current = 'calculator';

export function render(v, ctx, opts = {}) {
  if (opts.tab) current = opts.tab;
  const page = el(`<div class="page"><h1>Outils</h1><p class="sub">Tous ces outils fonctionnent sur ton ordinateur, sans Internet ni modèle d'IA.</p><div class="tabs"></div><div id="tool"></div></div>`);
  v.appendChild(page);
  const tabs = page.querySelector('.tabs');
  for (const [id, label, ic] of TABS) {
    const b = el(`<button class="tab ${id === current ? 'active' : ''}">${icon(ic)}${label}</button>`);
    b.onclick = () => { current = id; v.innerHTML = ''; render(v, ctx); };
    tabs.appendChild(b);
  }
  const box = page.querySelector('#tool');
  ({ calculator, equations, derivative, graph, units, stats, matrices })[current](box, ctx);
}

const degrees = (ctx) => ctx.state.settings.angleMode === 'degrees';

function resultCard(md) {
  const c = el('<div class="card md"></div>');
  c.innerHTML = renderMarkdown(md);
  wireMarkdown(c);
  return c;
}

function errorLine(msg) {
  return el(`<div class="warn-line" style="color:var(--danger)">${icon('alert')} ${esc(msg)}</div>`);
}

function examples(list, pick) {
  const c = el('<div class="chips">Exemples :</div>');
  list.forEach((e) => { const b = el(`<button class="chip">${esc(e)}</button>`); b.onclick = () => pick(e); c.appendChild(b); });
  return c;
}

// ---------------------------------------------------------------- Calculatrice

function calculator(box, ctx) {
  const keys = ['sin(', 'cos(', 'tan(', 'ln(', 'log(', '√(', '7', '8', '9', '÷', '(', ')', '4', '5', '6', '×', '^', '!', '1', '2', '3', '−', 'π', 'e', '0', '.', 'ans', '+', 'C', '='];
  const history = [];
  const ui = el(`<div class="stack"><div class="row"><input class="field big-field grow" placeholder="ex. 2^10 / (3 + sin(pi/6))"><select class="field" id="angle"><option value="radians">Radians</option><option value="degrees">Degrés</option></select></div><div id="out"></div><div class="keypad"></div><div id="hist"></div></div>`);
  box.appendChild(ui);
  const input = ui.querySelector('input'), out = ui.querySelector('#out');
  const angle = ui.querySelector('#angle');
  angle.value = ctx.state.settings.angleMode || 'radians';
  angle.onchange = () => ctx.setSetting('angleMode', angle.value);
  const compute = async () => {
    out.innerHTML = '';
    if (!input.value.trim()) return;
    try {
      const r = await invoke('math_eval', { expr: input.value, degrees: degrees(ctx) });
      history.unshift([input.value, r.value, r.fraction]);
      input.value = r.value;
      renderHist();
    } catch (e) { out.appendChild(errorLine(errorText(e))); }
  };
  const renderHist = () => {
    const h = ui.querySelector('#hist');
    h.innerHTML = history.length ? `<div class="card" style="max-width:520px;display:grid;gap:6px"><b style="font-size:12px" class="muted">Historique</b>${history.slice(0, 12).map(([e, v, f]) => `<div class="row" style="justify-content:space-between"><span class="mono muted">${esc(e)}</span><b class="mono">= ${esc(v)}${f ? ` <span class="faint">(${esc(f)})</span>` : ''}</b></div>`).join('')}</div>` : '';
  };
  input.onkeydown = (e) => e.key === 'Enter' && compute();
  const pad = ui.querySelector('.keypad');
  for (const k of keys) {
    const b = el(`<button class="${/^\d$|^\.$/.test(k) ? 'num' : ''} ${k === '=' ? 'eq' : ''}">${k === '√(' ? '√' : esc(k)}</button>`);
    b.onclick = () => {
      if (k === 'C') { input.value = ''; out.innerHTML = ''; }
      else if (k === '=') compute();
      else if (k === 'ans') input.value += history[0]?.[1] ?? '';
      else input.value += k;
      input.focus();
    };
    pad.appendChild(b);
  }
  input.focus();
}

// ---------------------------------------------------------------- Équations et dérivées

function equations(box, ctx) {
  const ui = el(`<div class="stack"><label class="faint">Équation ou système (sépare les équations par « ; »)</label><input class="field big-field" placeholder="ex. x^2 - 5x + 6 = 0"><div class="ex"></div><div><button class="btn primary">Résoudre</button></div><div id="out"></div></div>`);
  box.appendChild(ui);
  const input = ui.querySelector('input'), out = ui.querySelector('#out');
  const run = async () => {
    out.innerHTML = '';
    try { out.appendChild(resultCard(await invoke('math_run', { kind: 'equation', input: input.value, variable: null, degrees: degrees(ctx) }))); }
    catch (e) { out.appendChild(errorLine(errorText(e))); }
  };
  ui.querySelector('.ex').appendChild(examples(['2x + 3 = 11', 'x^3 - 6x^2 + 11x - 6 = 0', 'cos(x) = x', '2x + y = 5; x - y = 1'], (e) => { input.value = e; run(); }));
  ui.querySelector('.btn').onclick = run;
  input.onkeydown = (e) => e.key === 'Enter' && run();
  input.focus();
}

function derivative(box, ctx) {
  const ui = el(`<div class="stack"><div class="row"><input class="field big-field grow" placeholder="ex. x^2 · sin(x)"><input class="field big-field" style="width:80px" value="x" title="Variable"></div><div class="ex"></div><div><button class="btn primary">Dériver</button></div><div id="out"></div><p class="faint">Intégrales, limites et factorisation symbolique arriveront avec un moteur de calcul formel dans une prochaine version.</p></div>`);
  box.appendChild(ui);
  const [input, variable] = ui.querySelectorAll('input');
  const out = ui.querySelector('#out');
  const run = async () => {
    out.innerHTML = '';
    try { out.appendChild(resultCard(await invoke('math_run', { kind: 'derivative', input: input.value, variable: variable.value || 'x', degrees: degrees(ctx) }))); }
    catch (e) { out.appendChild(errorLine(errorText(e))); }
  };
  ui.querySelector('.ex').appendChild(examples(['x^3 + 2x', 'sin(x)·cos(x)', 'e^(2x)', 'ln(x^2 + 1)'], (e) => { input.value = e; run(); }));
  ui.querySelector('.btn').onclick = run;
  input.onkeydown = (e) => e.key === 'Enter' && run();
  input.focus();
}

// ---------------------------------------------------------------- Graphique (canvas)

function graph(box) {
  const ui = el(`<div class="stack"><div class="row"><span class="mono muted">f(x) =</span><input class="field big-field grow" value="sin(x) * x"></div>
    <div class="row" style="font-size:12.5px"><span class="muted">x de</span><input class="field" style="width:80px" value="-10"><span class="muted">à</span><input class="field" style="width:80px" value="10"><label class="row" style="gap:6px"><input type="checkbox" id="deriv"> afficher f′(x)</label><span class="grow"></span><button class="btn primary">Tracer</button></div>
    <div id="err"></div><div class="card" style="padding:8px"><canvas class="plot"></canvas></div><div id="info" class="mono muted" style="font-size:12.5px"></div></div>`);
  box.appendChild(ui);
  const [fx, xmin, xmax] = ui.querySelectorAll('input.field');
  const canvas = ui.querySelector('canvas');
  let data = null;
  const draw = () => {
    const dpr = devicePixelRatio || 1;
    const w = canvas.clientWidth, h = canvas.clientHeight;
    canvas.width = w * dpr; canvas.height = h * dpr;
    const g = canvas.getContext('2d');
    g.scale(dpr, dpr);
    g.clearRect(0, 0, w, h);
    if (!data) return;
    const css = getComputedStyle(document.documentElement);
    const series = [data.points, ...(ui.querySelector('#deriv').checked ? [data.derivative] : [])];
    const ys = series.flat().map((p) => p[1]).filter((y) => y !== null);
    if (!ys.length) return;
    let lo = Math.min(...ys, 0), hi = Math.max(...ys, 0);
    if (hi - lo < 1e-9) { lo -= 1; hi += 1; }
    const pad = (hi - lo) * 0.08; lo -= pad; hi += pad;
    const x0 = Number(xmin.value), x1 = Number(xmax.value);
    const X = (x) => 34 + ((x - x0) / (x1 - x0)) * (w - 44);
    const Y = (y) => 10 + (1 - (y - lo) / (hi - lo)) * (h - 30);
    // Grille et axes
    g.strokeStyle = css.getPropertyValue('--border'); g.lineWidth = 1;
    g.font = '11px ' + css.getPropertyValue('--mono'); g.fillStyle = css.getPropertyValue('--text-3');
    for (let i = 0; i <= 4; i++) {
      const yv = lo + ((hi - lo) * i) / 4, yy = Y(yv);
      g.beginPath(); g.moveTo(34, yy); g.lineTo(w - 10, yy); g.stroke();
      g.fillText(+yv.toPrecision(3) + '', 2, yy + 4);
    }
    for (let i = 0; i <= 6; i++) {
      const xv = x0 + ((x1 - x0) * i) / 6;
      g.fillText(+xv.toPrecision(3) + '', X(xv) - 8, h - 4);
    }
    g.strokeStyle = css.getPropertyValue('--text-3');
    g.beginPath(); g.moveTo(34, Y(0)); g.lineTo(w - 10, Y(0)); g.stroke();
    if (x0 < 0 && x1 > 0) { g.beginPath(); g.moveTo(X(0), 10); g.lineTo(X(0), h - 20); g.stroke(); }
    const colors = [css.getPropertyValue('--accent'), css.getPropertyValue('--violet')];
    series.forEach((pts, si) => {
      g.strokeStyle = colors[si]; g.lineWidth = 2; g.setLineDash(si ? [5, 4] : []);
      g.beginPath();
      let pen = false;
      for (const [x, y] of pts) {
        if (y === null) { pen = false; continue; }
        if (!pen) { g.moveTo(X(x), Y(y)); pen = true; } else g.lineTo(X(x), Y(y));
      }
      g.stroke();
    });
    g.setLineDash([]);
    g.fillStyle = css.getPropertyValue('--cyan');
    data.roots.forEach((r) => { g.beginPath(); g.arc(X(r), Y(0), 4, 0, 7); g.fill(); });
  };
  const run = async () => {
    ui.querySelector('#err').innerHTML = '';
    try {
      data = await invoke('plot', { expr: fx.value, xmin: Number(xmin.value), xmax: Number(xmax.value) });
      ui.querySelector('#info').textContent = [
        data.derivative_expr ? `f′(x) = ${data.derivative_expr}` : '',
        data.roots.length ? `Zéros : ${data.roots.slice(0, 12).map((r) => 'x ≈ ' + r).join(' · ')}` : '',
      ].filter(Boolean).join('    ');
      draw();
    } catch (e) { data = null; draw(); ui.querySelector('#err').appendChild(errorLine(errorText(e))); }
  };
  ui.querySelector('.btn').onclick = run;
  ui.querySelector('#deriv').onchange = draw;
  fx.onkeydown = (e) => e.key === 'Enter' && run();
  new ResizeObserver(draw).observe(canvas);
  run();
}

// ---------------------------------------------------------------- Conversions

async function units(box) {
  const cats = await invoke('units_catalog');
  const ui = el(`<div class="stack"><select class="field" id="cat" style="max-width:320px"></select>
    <div class="row"><input class="field big-field" id="val" style="width:180px" value="1"><select class="field" id="from"></select><button class="icon-btn" id="swap" title="Inverser">${icon('arrows')}</button><select class="field" id="to"></select></div>
    <div class="card" id="res"></div><p class="faint">Astuce : tu peux aussi écrire « 25 mph en km/h » directement dans la conversation.</p></div>`);
  box.appendChild(ui);
  const cat = ui.querySelector('#cat'), from = ui.querySelector('#from'), to = ui.querySelector('#to'), val = ui.querySelector('#val'), res = ui.querySelector('#res');
  cats.forEach((c, i) => cat.appendChild(el(`<option value="${i}">${esc(c.name)}</option>`)));
  const fill = () => {
    const units = cats[cat.value].units;
    from.innerHTML = to.innerHTML = '';
    units.forEach((u) => { from.appendChild(el(`<option value="${esc(u.symbol)}">${esc(u.name)} (${esc(u.symbol)})</option>`)); to.appendChild(el(`<option value="${esc(u.symbol)}">${esc(u.name)} (${esc(u.symbol)})</option>`)); });
    to.selectedIndex = Math.min(1, units.length - 1);
    convert();
  };
  const convert = async () => {
    const v = Number(val.value.replace(',', '.'));
    if (!Number.isFinite(v)) { res.innerHTML = '<span class="faint">Saisis une valeur numérique.</span>'; return; }
    const r = await invoke('units_convert', { value: v, category: Number(cat.value), from: from.value, to: to.value });
    res.innerHTML = `<div class="row" style="align-items:baseline"><span class="muted">${esc(val.value)} ${esc(from.value)} =</span><b style="font-size:24px">${esc(+r.toPrecision(10))} ${esc(to.value)}</b></div>`;
  };
  cat.onchange = fill;
  [from, to].forEach((s) => (s.onchange = convert));
  val.oninput = convert;
  ui.querySelector('#swap').onclick = () => { const f = from.value; from.value = to.value; to.value = f; convert(); };
  fill();
}

// ---------------------------------------------------------------- Statistiques

function stats(box) {
  const ui = el(`<div class="stack"><label class="faint">Série de valeurs (séparées par des espaces, virgules ou points-virgules)</label><input class="field big-field" value="12, 15, 11, 18, 15, 20, 14"><div id="out"></div>
    <h2 style="margin-top:8px">Probabilités</h2><div class="row"><label>n <input class="field" id="n" style="width:80px" value="10"></label><label>k <input class="field" id="k" style="width:80px" value="3"></label><label>p <input class="field" id="p" style="width:80px" value="0.5"></label></div><div id="prob"></div></div>`);
  box.appendChild(ui);
  const input = ui.querySelector('input');
  const f = (x) => (x === null || x === undefined ? '–' : +Number(x).toPrecision(8));
  const run = async () => {
    const s = await invoke('stats_describe', { text: input.value });
    const out = ui.querySelector('#out');
    if (!s) { out.innerHTML = '<span class="faint">Saisis au moins une valeur.</span>'; return; }
    out.innerHTML = `<div class="card"><table class="data"><tr><th>Effectif</th><td>${s.count}</td><th>Somme</th><td>${f(s.sum)}</td></tr><tr><th>Moyenne</th><td>${f(s.mean)}</td><th>Médiane</th><td>${f(s.median)}</td></tr><tr><th>Minimum</th><td>${f(s.min)}</td><th>Maximum</th><td>${f(s.max)}</td></tr><tr><th>Q1</th><td>${f(s.q1)}</td><th>Q3</th><td>${f(s.q3)}</td></tr><tr><th>Variance (pop.)</th><td>${f(s.variance)}</td><th>Écart-type (pop.)</th><td>${f(s.std_dev)}</td></tr><tr><th>Variance (éch.)</th><td>${f(s.sample_variance)}</td><th>Écart-type (éch.)</th><td>${f(s.sample_std_dev)}</td></tr></table></div>`;
  };
  const runProb = async () => {
    const n = parseInt(ui.querySelector('#n').value), k = parseInt(ui.querySelector('#k').value), p = Number(ui.querySelector('#p').value.replace(',', '.'));
    const box2 = ui.querySelector('#prob');
    if (!(n >= 0 && k >= 0 && p >= 0 && p <= 1)) { box2.innerHTML = '<span class="faint">n et k entiers positifs, p entre 0 et 1.</span>'; return; }
    const r = await invoke('probabilities', { n, k, p });
    box2.innerHTML = `<div class="card"><table class="data"><tr><th>Combinaisons C(n,k)</th><td>${f(r.combinations)}</td><th>Arrangements A(n,k)</th><td>${f(r.permutations)}</td></tr><tr><th>Binomiale P(X = k)</th><td>${f(r.binomial)}</td><th>P(X ≤ k)</th><td>${f(r.cdf)}</td></tr></table></div>`;
  };
  input.oninput = run;
  ui.querySelectorAll('#n,#k,#p').forEach((i) => (i.oninput = runProb));
  run(); runProb();
}

// ---------------------------------------------------------------- Matrices

function matrices(box) {
  const ui = el(`<div class="stack"><div class="row" style="align-items:flex-start"><div><div class="faint">Matrice A</div><textarea class="field mono" rows="4" style="width:220px">2 1\n1 3</textarea></div><div><div class="faint">Matrice B</div><textarea class="field mono" rows="4" style="width:220px">1 0\n0 1</textarea></div></div>
    <div class="row"><button class="btn" data-op="det">det(A)</button><button class="btn" data-op="inv">A⁻¹</button><button class="btn" data-op="transpose">Aᵀ</button><button class="btn" data-op="mul">A × B</button><button class="btn" data-op="add">A + B</button></div><div id="out"></div><p class="faint">Une ligne par rangée, valeurs séparées par des espaces.</p></div>`);
  box.appendChild(ui);
  const [a, b] = ui.querySelectorAll('textarea');
  ui.querySelectorAll('[data-op]').forEach((btn) => (btn.onclick = async () => {
    const out = ui.querySelector('#out');
    out.innerHTML = '';
    try {
      const r = await invoke('matrix_op', { op: btn.dataset.op, a: a.value, b: b.value });
      const text = Array.isArray(r) ? r.map((row) => row.map((x) => +Number(x).toPrecision(8)).join('   ')).join('\n') : r;
      out.appendChild(el(`<div class="card"><pre class="mono" style="margin:0;font-size:14px">${esc(btn.textContent)} =\n${esc(text)}</pre></div>`));
    } catch (e) { out.appendChild(errorLine(errorText(e))); }
  }));
}
