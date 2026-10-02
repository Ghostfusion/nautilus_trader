/**
 * Sector Regime Engine - interface layer.
 *
 * Rendering and wiring only. Every computation is imported from engine.js so the numbers on screen
 * are the numbers the test suite covers.
 */

import {
  DEFAULT_CONFIG,
  TRADING_DAYS,
  analyze,
  attribution,
  backtest,
  configFingerprint,
  performance,
  runGrid,
} from './engine.js';

import { parseCsv, syntheticUniverse } from './data.js';

const $ = (id) => document.getElementById(id);

const COLORS = {
  basket: '#9aa3b2',
  struct: '#6ea8fe',
  actual: '#c58af9',
  net: '#45b26b',
  dispersion: '#6ea8fe',
  corr: '#d9a441',
};

const state = {
  matrix: null,
  dates: null,
  sectorNames: null,
  groundTruth: null,
  groundTruthDescription: null,
  config: null,
  analysis: null,
  full: null,
  result: null,
  attr: null,
  grid: null,
  split: 0,
  holdoutRevealed: false,
};

/* ------------------------------------------------------------------ *
 * Formatting
 * ------------------------------------------------------------------ */

const num = (x, digits = 4) => (Number.isFinite(x) ? x.toFixed(digits) : 'n/a');
const pct = (x, digits = 2) => (Number.isFinite(x) ? `${(x * 100).toFixed(digits)}%` : 'n/a');
const signedPct = (x, digits = 2) =>
  Number.isFinite(x) ? `${x >= 0 ? '+' : ''}${(x * 100).toFixed(digits)}%` : 'n/a';

function pvalue(p) {
  if (!Number.isFinite(p)) return 'n/a';
  return p < 0.0001 ? '<0.0001' : p.toFixed(4);
}

function esc(s) {
  return String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
}

/* ------------------------------------------------------------------ *
 * Configuration
 * ------------------------------------------------------------------ */

function readNumber(id, fallback) {
  const v = Number($(id).value);
  return Number.isFinite(v) ? v : fallback;
}

function readConfig() {
  return {
    ...DEFAULT_CONFIG,
    alpha: readNumber('cfg-alpha', DEFAULT_CONFIG.alpha),
    bonferroni: $('cfg-bonferroni').checked,
    minObs: readNumber('cfg-minobs', DEFAULT_CONFIG.minObs),
    rollWindow: readNumber('cfg-rollwindow', DEFAULT_CONFIG.rollWindow),
    interval: $('cfg-interval').value,
    band: readNumber('cfg-band', DEFAULT_CONFIG.band),
    costBps: readNumber('cfg-costbps', DEFAULT_CONFIG.costBps),
    expenseBps: readNumber('cfg-expensebps', DEFAULT_CONFIG.expenseBps),
    overlay: $('cfg-overlay').value,
    overlayFrequency: $('cfg-overlayfreq').value,
    overlayLookback: readNumber('cfg-lookback', DEFAULT_CONFIG.overlayLookback),
    overlaySkip: readNumber('cfg-skip', DEFAULT_CONFIG.overlaySkip),
    overlayTop: readNumber('cfg-top', DEFAULT_CONFIG.overlayTop),
    holdoutFraction: readNumber('cfg-holdout', DEFAULT_CONFIG.holdoutFraction),
    seed: readNumber('gen-seed', DEFAULT_CONFIG.seed),
  };
}

/* ------------------------------------------------------------------ *
 * Data loading
 * ------------------------------------------------------------------ */

function loadSynthetic() {
  const universe = syntheticUniverse({
    regime: $('gen-regime').value,
    seed: readNumber('gen-seed', 20261002),
    years: readNumber('gen-years', 8),
    annualVol: readNumber('gen-vol', 0.2),
    correlation: readNumber('gen-corr', 0.6),
    latentShare: readNumber('gen-latent', 0.3),
  });
  state.matrix = universe.matrix;
  state.dates = universe.dates;
  state.sectorNames = universe.sectorNames;
  state.groundTruth = universe.groundTruth;
  state.groundTruthDescription = universe.groundTruthDescription;
}

async function loadCsv() {
  const file = $('csv-file').files?.[0];
  if (!file) throw new Error('choose a CSV file first');
  const text = await file.text();
  const parsed = parseCsv(text, { mode: $('csv-mode').value });
  state.matrix = parsed.matrix;
  state.dates = parsed.dates;
  state.sectorNames = parsed.sectorNames;
  state.groundTruth = null;
  state.groundTruthDescription = null;
}

/* ------------------------------------------------------------------ *
 * Run
 * ------------------------------------------------------------------ */

function run() {
  if (!state.matrix) {
    $('status').textContent = 'no data loaded';
    return;
  }
  const config = readConfig();
  $('fingerprint').textContent = configFingerprint(config).hash;

  const total = state.matrix.length;
  const split = Math.max(
    Math.min(config.minObs, Math.floor(total * 0.5)),
    Math.floor(total * (1 - config.holdoutFraction)),
  );
  state.split = split;

  // The regime is decided on the observation window only, then frozen for everything downstream.
  const observation = state.matrix.slice(0, split);
  state.analysis = analyze({ matrix: observation, sectorNames: state.sectorNames, config });
  state.full = analyze({ matrix: state.matrix, sectorNames: state.sectorNames, config });
  state.config = config;

  const regimeLabel = state.analysis.regime.label;
  state.result = backtest({ matrix: state.matrix, config, regimeLabel });
  state.attr = attribution({
    matrix: state.matrix,
    result: state.result,
    sectorNames: state.sectorNames,
    config,
  });
  state.grid = runGrid({ matrix: state.matrix, config, regimeLabel, sectorNames: state.sectorNames });
  state.holdoutRevealed = false;

  renderAll();
  $('status').textContent = `analysed ${total} observations across ${state.sectorNames.length} sectors`;
}

function renderAll() {
  renderGroundTruth();
  renderVerdict();
  renderAxes();
  renderEligibility();
  renderMetrics();
  renderEquity();
  renderAttribution();
  renderStructure();
  renderGrid();
  renderHoldout();
}

/* ------------------------------------------------------------------ *
 * Rendering
 * ------------------------------------------------------------------ */

function renderGroundTruth() {
  const el = $('ground-truth');
  if (!state.groundTruth) {
    el.innerHTML = 'Loaded from CSV. No ground truth is known for this data, so only the measurement is reported.';
    return;
  }
  const measured = state.analysis.regime.label;
  const match = measured === state.groundTruth;
  el.innerHTML = `Synthetic data with known regime <strong>${esc(state.groundTruth)}</strong>.
    The classifier measured <strong>${esc(measured)}</strong>.
    ${match ? 'Agreement.' : 'Disagreement - expected at the classifier error rate, which the validation sweep quantifies.'}
    <br /><span class="status">${esc(state.groundTruthDescription ?? '')}</span>`;
}

function renderVerdict() {
  const { regime, metrics } = state.analysis;
  const cls =
    regime.label === 'MEMORYLESS'
      ? 'tag-memoryless'
      : regime.label === 'UNCERTAIN'
        ? 'tag-uncertain'
        : regime.label === 'POSITIVE_DEPENDENCE'
          ? 'tag-positive'
          : 'tag-negative';
  const reasons = regime.reasons.map((r) => `<li>${esc(r)}</li>`).join('');
  $('verdict').innerHTML = `
    <div class="verdict">
      <span class="label ${cls}">${esc(regime.label)}</span>
      <span class="conf">confidence ${esc(regime.confidence)}</span>
      <span class="conf">alpha used ${num(regime.alphaUsed, 4)}</span>
      <span class="conf">${metrics.observations} observations</span>
    </div>
    <ul class="reasons">${reasons}</ul>
    <p class="sub" style="margin-top:10px">
      Decided on the observation window only (${state.split} of ${state.matrix.length} observations),
      then frozen. The overlay, if any, is gated on this label.
    </p>`;
}

function renderAxes() {
  const m = state.analysis.metrics;
  const premium = state.analysis.impliedPremium;
  const temporal = `
    <div class="card">
      <h3>Axis 1 - temporal dependence</h3>
      <table>
        <tbody>
          <tr><td>Rank autocorrelation</td><td class="num">${num(m.rankAutocorr.value, 4)}</td><td class="num">p=${pvalue(m.rankAutocorr.pValue)}</td></tr>
          <tr><td>Return autocorrelation</td><td class="num">${num(m.returnAutocorr.pooledR, 4)}</td><td class="num">p=${pvalue(m.returnAutocorr.pValue)}</td></tr>
          <tr><td>Variance ratio (q=${m.varianceRatio.q})</td><td class="num">${num(m.varianceRatio.vr, 3)}</td><td class="num">p=${pvalue(m.varianceRatio.pValue)}</td></tr>
          <tr><td>Conditional persistence</td><td class="num">${num(m.persistence.value, 4)}</td><td class="num">base ${num(m.persistence.baseRate, 4)}</td></tr>
        </tbody>
      </table>
      <p class="sub" style="margin-top:10px">Decides whether a directional overlay is admissible at all.</p>
    </div>`;
  const structural = `
    <div class="card">
      <h3>Axis 2 - cross-sectional structure</h3>
      <table>
        <tbody>
          <tr><td>Cross-sectional dispersion (daily)</td><td class="num">${num(m.dispersion, 4)}</td></tr>
          <tr><td>Average pairwise correlation</td><td class="num">${num(m.avgCorrelation, 4)}</td></tr>
          <tr><td>Median pairwise correlation</td><td class="num">${num(m.medianCorrelation, 4)}</td></tr>
          <tr><td>Implied rebalancing premium</td><td class="num">${signedPct(premium, 2)} per year</td></tr>
        </tbody>
      </table>
      <p class="sub" style="margin-top:10px">
        Implied premium is <code>(1/2)(sum w_i sigma_i^2 - portfolio variance)</code> computed from the
        measured vols and correlations, before costs. It is a formula output, not a realised result,
        and it falls as correlation rises.
      </p>
    </div>`;
  $('axes').innerHTML = temporal + structural;
}

function renderEligibility() {
  const rows = state.analysis.eligibility
    .map(
      (r) => `<tr class="${r.enabled ? 'enabled' : 'disabled'}">
        <td>${esc(r.strategy)}</td>
        <td>${esc(r.condition)}</td>
        <td>${r.enabled ? 'eligible' : 'blocked'}</td>
        <td>${esc(r.reason)}</td>
      </tr>`,
    )
    .join('');
  $('eligibility').innerHTML = `<table>
    <thead><tr><th>Strategy</th><th>Required condition</th><th>Status</th><th>Why</th></tr></thead>
    <tbody>${rows}</tbody></table>`;
}

function renderMetrics() {
  const rows = state.analysis.regime.tests
    .map((t) => {
      const value = t.key === 'varianceRatio' || t.key === 'rankTurnover' || t.key === 'dispersion' || t.key === 'avgCorrelation' || t.key === 'hurst'
        ? num(t.value, 4)
        : num(t.value, 4);
      return `<tr>
        <td><span class="badge ${t.tier}">${t.tier}</span></td>
        <td>${esc(t.name)}</td>
        <td class="num">${value}</td>
        <td class="num">${num(t.statistic, 3)}</td>
        <td class="num">${pvalue(t.pValue)}</td>
        <td class="num">${Number.isFinite(t.n) ? t.n : 'n/a'}</td>
      </tr>`;
    })
    .join('');
  $('metrics').innerHTML = `<table>
    <thead><tr><th>Tier</th><th>Metric</th><th class="num">Value</th><th class="num">Statistic</th><th class="num">p</th><th class="num">n</th></tr></thead>
    <tbody>${rows}</tbody></table>
    <p class="sub" style="margin-top:10px">
      Only the three primary rows can decide the regime. Secondary and diagnostic rows are reported
      because they describe the universe, not because they vote.
    </p>`;
}

function renderEquity() {
  const r = state.result;
  const series = [
    { name: 'drifting basket (gross)', color: COLORS.basket, values: r.basket },
    { name: 'drift-band rebalanced (gross)', color: COLORS.struct, values: r.struct },
    { name: r.overlayActive ? 'with overlay (gross)' : 'actual book (gross)', color: COLORS.actual, values: r.actual },
    { name: 'net of all costs', color: COLORS.net, values: r.net },
  ];
  lineChart($('chart-equity'), series);
  $('legend').innerHTML = series
    .map((s) => `<span style="color:${s.color}">${esc(s.name)}</span>`)
    .join('');
  const rebalances = r.ledger.rebalances;
  $('equity-caption').textContent =
    `${rebalances} rebalance events, ${num(r.ledger.actualTurnover, 2)} cumulative one-way turnover. ` +
    (r.overlayActive
      ? 'The overlay is active because the regime admits it.'
      : 'No overlay: the regime does not admit one, or none was selected.');
}

function renderAttribution() {
  const a = state.attr;
  const rows = a.lines
    .map(
      (l) => `<tr>
        <td>${esc(l.label)}</td>
        <td class="num">${num(l.value, 4)}</td>
        <td class="num">${signedPct(l.value / a.years, 2)}</td>
      </tr>`,
    )
    .join('');
  $('attribution').innerHTML = `
    <table>
      <thead><tr><th>Line</th><th class="num">Log return</th><th class="num">Annualised</th></tr></thead>
      <tbody>
        ${rows}
        <tr><td><strong>Total (net, log)</strong></td><td class="num"><strong>${num(a.total, 4)}</strong></td><td class="num"><strong>${signedPct(a.total / a.years, 2)}</strong></td></tr>
      </tbody>
    </table>
    <table style="margin-top:12px">
      <tbody>
        <tr><td>${esc(a.typeA.label)}</td><td class="num">${signedPct(a.typeA.annual, 2)} per year</td></tr>
        <tr><td>${esc(a.typeB.label)}</td><td class="num">${signedPct(a.typeB.annual, 2)} per year</td></tr>
        <tr><td>${esc(a.overlay.label)}</td><td class="num">${signedPct(a.overlay.annual, 2)} per year</td></tr>
        <tr><td>Trading cost</td><td class="num">${signedPct(a.tradingCost.annual, 2)} per year</td></tr>
        <tr><td>Holding cost</td><td class="num">${signedPct(a.holdingCost.annual, 2)} per year</td></tr>
        <tr><td>Additivity residual (sum of lines minus total)</td><td class="num">${a.residual.toExponential(2)}</td></tr>
      </tbody>
    </table>
    <p class="sub" style="margin-top:10px">
      Type A measures compound growth against the weighted average of the components' own growth
      rates. Type B measures the rebalanced book against the drifting basket. They answer different
      questions and are not interchangeable. Over ${num(a.years, 2)} years the compounded net result is
      ${signedPct(a.compounded, 2)}.
    </p>`;
}

function renderStructure() {
  const s = state.full.metrics.structure;
  lineChart($('chart-dispersion'), [
    { name: 'dispersion', color: COLORS.dispersion, values: s.dispersion },
  ]);
  lineChart($('chart-corr'), [{ name: 'average correlation', color: COLORS.corr, values: s.avgCorr }]);

  const names = state.sectorNames;
  const matrix = state.full.metrics.correlationMatrix;
  const n = names.length;
  let html = '<div class="heat" style="grid-template-columns: 64px repeat(' + n + ', 1fr)">';
  html += '<div class="axis"></div>';
  for (const name of names) html += `<div class="axis">${esc(name)}</div>`;
  for (let i = 0; i < n; i += 1) {
    html += `<div class="axis" style="text-align:right">${esc(names[i])}</div>`;
    for (let j = 0; j < n; j += 1) {
      const c = matrix[i][j];
      const t = Math.max(0, Math.min(1, (c + 1) / 2));
      const r = Math.round(40 + t * 150);
      const g = Math.round(60 + (1 - Math.abs(c)) * 120);
      const b = Math.round(140 + (1 - t) * 90);
      html += `<div class="cell" style="background:rgb(${r},${g},${b})" title="${esc(names[i])} vs ${esc(names[j])}: ${num(c, 3)}">${num(c, 2)}</div>`;
    }
  }
  html += '</div>';
  $('heatmap').innerHTML = html;
}

function renderGrid() {
  const rows = state.grid
    .map(
      (r) => `<tr>
        <td>${esc(r.interval)}</td>
        <td class="num">${pct(r.band, 0)}</td>
        <td class="num">${signedPct(r.cagr, 2)}</td>
        <td class="num">${pct(r.vol, 2)}</td>
        <td class="num">${num(r.sharpe, 2)}</td>
        <td class="num">${pct(r.maxDrawdown, 2)}</td>
        <td class="num">${r.rebalances}</td>
        <td class="num">${num(r.turnover, 2)}</td>
        <td class="num">${signedPct(r.rebalancingContribution, 2)}</td>
        <td class="num">${signedPct(r.tradingCost, 2)}</td>
      </tr>`,
    )
    .join('');
  $('grid').innerHTML = `<table>
    <thead><tr>
      <th>Check</th><th class="num">Band (rel.)</th><th class="num">CAGR</th><th class="num">Vol</th>
      <th class="num">Sharpe</th><th class="num">Max DD</th><th class="num">Rebalances</th>
      <th class="num">Turnover</th><th class="num">Rebal contrib</th><th class="num">Trading cost</th>
    </tr></thead>
    <tbody>${rows}</tbody></table>
    <p class="sub" style="margin-top:10px">All figures net of trading and holding cost.</p>`;
}

function renderHoldout() {
  const el = $('holdout');
  const split = state.split;
  const total = state.matrix.length;
  const header = `<p class="sub">Observation window: rows 0 to ${split} (${split} observations).
    Held-out window: rows ${split} to ${total} (${total - split} observations).</p>`;
  if (!state.holdoutRevealed) {
    $('reveal-holdout').classList.remove('hidden');
    el.innerHTML = `${header}<p class="note">Held-out results hidden. The regime was frozen from the observation window before any held-out number was computed.</p>`;
    return;
  }
  const rowsFor = (label, equity) => {
    const inSample = performance(equity.slice(0, split + 1));
    const outSample = performance(equity.slice(split));
    return `<tr>
      <td>${esc(label)}</td>
      <td class="num">${signedPct(inSample.cagr, 2)}</td>
      <td class="num">${num(inSample.sharpe, 2)}</td>
      <td class="num">${pct(inSample.maxDrawdown, 2)}</td>
      <td class="num">${signedPct(outSample.cagr, 2)}</td>
      <td class="num">${num(outSample.sharpe, 2)}</td>
      <td class="num">${pct(outSample.maxDrawdown, 2)}</td>
    </tr>`;
  };
  el.innerHTML = `${header}
    <table>
      <thead><tr>
        <th rowspan="2">Book</th><th colspan="3" class="num">Observation window</th>
        <th colspan="3" class="num">Held-out window</th>
      </tr><tr>
        <th class="num">CAGR</th><th class="num">Sharpe</th><th class="num">Max DD</th>
        <th class="num">CAGR</th><th class="num">Sharpe</th><th class="num">Max DD</th>
      </tr></thead>
      <tbody>
        ${rowsFor('drifting basket (gross)', state.result.basket)}
        ${rowsFor('drift-band rebalanced (gross)', state.result.struct)}
        ${rowsFor('actual book (gross)', state.result.actual)}
        ${rowsFor('net of all costs', state.result.net)}
      </tbody>
    </table>
    <p class="sub" style="margin-top:10px">
      The held-out window is short by construction. Treat these figures as a consistency check on the
      frozen rule, not as an estimate of future performance.
    </p>`;
}

/* ------------------------------------------------------------------ *
 * Validation sweep
 * ------------------------------------------------------------------ */

async function validate() {
  const button = $('validate');
  button.disabled = true;
  const el = $('validation');
  const seeds = [101, 202, 303, 404];
  const regimes = ['memoryless', 'persistent', 'reverting'];
  const labels = { memoryless: 'MEMORYLESS', persistent: 'POSITIVE_DEPENDENCE', reverting: 'NEGATIVE_DEPENDENCE' };
  const measured = ['MEMORYLESS', 'UNCERTAIN', 'POSITIVE_DEPENDENCE', 'NEGATIVE_DEPENDENCE'];
  const table = {};
  for (const g of regimes) {
    table[g] = {};
    for (const m of measured) table[g][m] = 0;
  }

  const config = readConfig();
  let done = 0;
  const total = seeds.length * regimes.length;
  for (const regime of regimes) {
    for (const seed of seeds) {
      el.innerHTML = `<p class="note">running ${done} of ${total}...</p>`;
      await new Promise((resolve) => setTimeout(resolve, 0));
      const universe = syntheticUniverse({ regime, seed, years: 6 });
      const result = analyze({ matrix: universe.matrix, sectorNames: universe.sectorNames, config });
      table[regime][result.regime.label] += 1;
      done += 1;
    }
  }

  const correct = regimes.reduce((acc, g) => acc + table[g][labels[g]], 0);
  const header = measured.map((m) => `<th class="num">${esc(m.replace('_DEPENDENCE', ''))}</th>`).join('');
  const rows = regimes
    .map((g) => {
      const cells = measured.map((m) => `<td class="num">${table[g][m] || ''}</td>`).join('');
      return `<tr><td>${esc(g)}</td>${cells}</tr>`;
    })
    .join('');
  el.innerHTML = `
    <table>
      <thead><tr><th>Ground truth</th>${header}</tr></thead>
      <tbody>${rows}</tbody>
    </table>
    <p class="sub" style="margin-top:10px">
      ${correct} of ${total} classified correctly (${pct(correct / total, 0)}). Each cell is a count of runs.
      The validation uses the configuration currently frozen in section 2, so changing alpha or the
      deadband changes the instrument.
    </p>`;
  button.disabled = false;
}

/* ------------------------------------------------------------------ *
 * Charts
 * ------------------------------------------------------------------ */

function fitCanvas(canvas, fallbackHeight) {
  const dpr = window.devicePixelRatio || 1;
  const width = canvas.clientWidth || canvas.parentElement?.clientWidth || 800;
  const height = Number(canvas.getAttribute('height')) || fallbackHeight;
  canvas.width = Math.max(1, Math.floor(width * dpr));
  canvas.height = Math.max(1, Math.floor(height * dpr));
  const ctx = canvas.getContext('2d');
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, width, height);
  return { ctx, width, height };
}

function lineChart(canvas, series) {
  const { ctx, width, height } = fitCanvas(canvas, 240);
  const pad = { left: 52, right: 12, top: 12, bottom: 22 };
  const plotW = width - pad.left - pad.right;
  const plotH = height - pad.top - pad.bottom;

  let min = Infinity;
  let max = -Infinity;
  for (const s of series) {
    for (const v of s.values) {
      if (Number.isFinite(v)) {
        if (v < min) min = v;
        if (v > max) max = v;
      }
    }
  }
  if (!Number.isFinite(min) || !Number.isFinite(max)) {
    ctx.fillStyle = '#9aa3b2';
    ctx.fillText('no data', pad.left, height / 2);
    return;
  }
  if (max - min < 1e-9) {
    max += 0.01;
    min -= 0.01;
  }
  const x = (i, n) => pad.left + (n <= 1 ? 0 : (i * plotW) / (n - 1));
  const y = (v) => pad.top + plotH - ((v - min) / (max - min)) * plotH;

  // Grid and axis labels.
  ctx.strokeStyle = '#2a2f3a';
  ctx.fillStyle = '#9aa3b2';
  ctx.font = '11px ui-monospace, monospace';
  ctx.lineWidth = 1;
  const ticks = 4;
  for (let k = 0; k <= ticks; k += 1) {
    const value = min + ((max - min) * k) / ticks;
    const yy = Math.round(y(value)) + 0.5;
    ctx.beginPath();
    ctx.moveTo(pad.left, yy);
    ctx.lineTo(pad.left + plotW, yy);
    ctx.stroke();
    ctx.fillText(value.toFixed(value < 10 ? 3 : 1), 6, yy + 3);
  }

  for (const s of series) {
    ctx.strokeStyle = s.color;
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    let started = false;
    for (let i = 0; i < s.values.length; i += 1) {
      const v = s.values[i];
      if (!Number.isFinite(v)) continue;
      const xx = x(i, s.values.length);
      const yy = y(v);
      if (!started) {
        ctx.moveTo(xx, yy);
        started = true;
      } else {
        ctx.lineTo(xx, yy);
      }
    }
    ctx.stroke();
  }

  ctx.fillStyle = '#9aa3b2';
  ctx.fillText('0', pad.left, height - 6);
  ctx.fillText(String(Math.max(...series.map((s) => s.values.length)) - 1), pad.left + plotW - 24, height - 6);
}

/* ------------------------------------------------------------------ *
 * Wiring
 * ------------------------------------------------------------------ */

function syncSource() {
  const synthetic = $('source-synthetic').checked;
  $('synthetic-controls').classList.toggle('hidden', !synthetic);
  $('csv-controls').classList.toggle('hidden', synthetic);
}

async function runFromUi() {
  $('status').textContent = 'loading...';
  try {
    if ($('source-synthetic').checked) {
      loadSynthetic();
    } else {
      await loadCsv();
    }
    run();
  } catch (error) {
    $('status').textContent = `error: ${error.message}`;
  }
}

function exportJson() {
  if (!state.analysis) return;
  const payload = {
    generatedAt: new Date().toISOString(),
    fingerprint: configFingerprint(state.config).hash,
    config: state.config,
    regime: {
      label: state.analysis.regime.label,
      confidence: state.analysis.regime.confidence,
      alphaUsed: state.analysis.regime.alphaUsed,
      reasons: state.analysis.regime.reasons,
    },
    metrics: {
      observations: state.analysis.metrics.observations,
      rankAutocorrelation: state.analysis.metrics.rankAutocorr.value,
      rankAutocorrelationP: state.analysis.metrics.rankAutocorr.pValue,
      returnAutocorrelation: state.analysis.metrics.returnAutocorr.pooledR,
      returnAutocorrelationP: state.analysis.metrics.returnAutocorr.pValue,
      varianceRatio: state.analysis.metrics.varianceRatio.vr,
      varianceRatioP: state.analysis.metrics.varianceRatio.pValue,
      dispersion: state.analysis.metrics.dispersion,
      averageCorrelation: state.analysis.metrics.avgCorrelation,
      medianCorrelation: state.analysis.metrics.medianCorrelation,
      hurst: state.analysis.metrics.hurst.h,
      impliedPremium: state.analysis.impliedPremium,
    },
    eligibility: state.analysis.eligibility,
    attribution: {
      lines: state.attr.lines,
      total: state.attr.total,
      residual: state.attr.residual,
      typeA: state.attr.typeA,
      typeB: state.attr.typeB,
      overlay: state.attr.overlay,
      tradingCost: state.attr.tradingCost,
      holdingCost: state.attr.holdingCost,
    },
    grid: state.grid,
  };
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `sector-regime-${payload.fingerprint}.json`;
  link.click();
  URL.revokeObjectURL(url);
}

function init() {
  $('source-synthetic').addEventListener('change', syncSource);
  $('source-csv').addEventListener('change', syncSource);
  $('run').addEventListener('click', runFromUi);
  $('validate').addEventListener('click', validate);
  $('export-json').addEventListener('click', exportJson);
  $('reveal-holdout').addEventListener('click', () => {
    state.holdoutRevealed = true;
    renderHoldout();
  });
  syncSource();
  runFromUi();
}

if (typeof document !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
}

// No exports: the browser bundle is built as an IIFE so index.html can be opened directly from disk
// without a server. The engine is the exported, tested surface; this file is the interface.
