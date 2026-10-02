/**
 * Sector Regime Engine - core computation.
 *
 * Pure functions only: no DOM, no I/O, no globals. Every number the application shows is produced
 * here, so the whole engine can be tested headlessly.
 *
 * Implements the architecture of strategies/sector_rotation_strategies.md:
 *   - Section 2  : two independent axes, four regimes including UNCERTAIN
 *   - Section 4.1: comparison types A/B/C kept separate
 *   - Section 8  : metrics with an evidence hierarchy (primary / secondary / diagnostic)
 *   - Section 9  : eligibility gating rather than ranking
 *   - Section 10 : two-layer portfolio, drift band, split cost model, attribution
 */

export const TRADING_DAYS = 252;

/* ------------------------------------------------------------------ *
 * Deterministic RNG
 * ------------------------------------------------------------------ */

export function mulberry32(seed) {
  let a = seed >>> 0;
  return function next() {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Box-Muller, drawing from a supplied uniform generator. */
export function gaussian(rand) {
  let u = 0;
  let v = 0;
  while (u === 0) u = rand();
  while (v === 0) v = rand();
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
}

/* ------------------------------------------------------------------ *
 * Elementary statistics
 * ------------------------------------------------------------------ */

export function mean(xs) {
  if (xs.length === 0) return 0;
  let s = 0;
  for (const x of xs) s += x;
  return s / xs.length;
}

export function variance(xs, ddof = 1) {
  const n = xs.length;
  if (n <= ddof) return 0;
  const m = mean(xs);
  let s = 0;
  for (const x of xs) s += (x - m) * (x - m);
  return s / (n - ddof);
}

export function stdev(xs, ddof = 1) {
  return Math.sqrt(variance(xs, ddof));
}

export function covariance(xs, ys, ddof = 1) {
  const n = Math.min(xs.length, ys.length);
  if (n <= ddof) return 0;
  const mx = mean(xs.slice(0, n));
  const my = mean(ys.slice(0, n));
  let s = 0;
  for (let i = 0; i < n; i += 1) s += (xs[i] - mx) * (ys[i] - my);
  return s / (n - ddof);
}

export function corr(xs, ys) {
  const n = Math.min(xs.length, ys.length);
  if (n < 3) return 0;
  const sx = stdev(xs.slice(0, n));
  const sy = stdev(ys.slice(0, n));
  if (sx === 0 || sy === 0) return 0;
  return covariance(xs, ys) / (sx * sy);
}

/** Average ranks with ties resolved to the mean rank, one-based. */
export function ranks(xs) {
  const idx = xs.map((x, i) => [x, i]).sort((a, b) => a[0] - b[0]);
  const out = new Array(xs.length).fill(0);
  let i = 0;
  while (i < idx.length) {
    let j = i;
    while (j + 1 < idx.length && idx[j + 1][0] === idx[i][0]) j += 1;
    const avgRank = (i + j) / 2 + 1;
    for (let k = i; k <= j; k += 1) out[idx[k][1]] = avgRank;
    i = j + 1;
  }
  return out;
}

export function spearman(xs, ys) {
  return corr(ranks(xs), ranks(ys));
}

/** Abramowitz and Stegun 7.1.26 error function approximation. */
export function erf(x) {
  const sign = x < 0 ? -1 : 1;
  const ax = Math.abs(x);
  const t = 1 / (1 + 0.3275911 * ax);
  const y =
    1 -
    ((((1.061405429 * t - 1.453152027) * t + 1.421413741) * t - 0.284496736) * t + 0.254829592) *
      t *
      Math.exp(-ax * ax);
  return sign * y;
}

export function normalCdf(z) {
  return 0.5 * (1 + erf(z / Math.SQRT2));
}

export function twoSidedP(z) {
  return Math.min(1, Math.max(0, 2 * (1 - normalCdf(Math.abs(z)))));
}

export function fisherZ(r) {
  const clamped = Math.max(-0.999999, Math.min(0.999999, r));
  return 0.5 * Math.log((1 + clamped) / (1 - clamped));
}

export function quantile(xs, q) {
  if (xs.length === 0) return 0;
  const sorted = [...xs].sort((a, b) => a - b);
  const pos = (sorted.length - 1) * q;
  const lo = Math.floor(pos);
  const hi = Math.ceil(pos);
  if (lo === hi) return sorted[lo];
  return sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo);
}

/* ------------------------------------------------------------------ *
 * Metric layer - Section 8.2
 * ------------------------------------------------------------------ */

/**
 * Lag-1 autocorrelation of each sector's own return series, pooled with Fisher z weights.
 * Time-series question: does a sector's own return reverse or persist?
 */
export function returnAutocorrelation(matrix, lag = 1) {
  const nSectors = matrix[0]?.length ?? 0;
  const perSector = [];
  let weightSum = 0;
  let zWeighted = 0;
  for (let i = 0; i < nSectors; i += 1) {
    const series = matrix.map((row) => row[i]);
    const a = series.slice(0, series.length - lag);
    const b = series.slice(lag);
    const n = a.length;
    const r = corr(a, b);
    perSector.push({ sector: i, r, n });
    if (n > 3) {
      const w = n - 3;
      zWeighted += w * fisherZ(r);
      weightSum += w;
    }
  }
  const pooledZ = weightSum > 0 ? zWeighted / weightSum : 0;
  const statistic = weightSum > 0 ? pooledZ * Math.sqrt(weightSum) : 0;
  return {
    pooledR: Math.tanh(pooledZ),
    statistic,
    pValue: twoSidedP(statistic),
    n: perSector.reduce((acc, p) => acc + p.n, 0),
    perSector,
  };
}

/**
 * Cross-sectional rank autocorrelation: corr(rank_t, rank_t+1) per adjacent day pair, then a
 * one-sample t test on the information coefficient series. This is the question a ranking strategy
 * actually depends on, and it is a primary test in Section 8.1.
 */
export function rankAutocorrelation(matrix) {
  const ics = [];
  for (let t = 0; t + 1 < matrix.length; t += 1) {
    const ic = spearman(matrix[t], matrix[t + 1]);
    if (Number.isFinite(ic)) ics.push(ic);
  }
  const n = ics.length;
  const m = mean(ics);
  const sd = stdev(ics);
  const statistic = sd > 0 && n > 2 ? m / (sd / Math.sqrt(n)) : 0;
  return { ic: ics, value: m, statistic, pValue: twoSidedP(statistic), n };
}

/** Lo-MacKinlay variance ratio with the homoskedastic asymptotic standard error. */
export function varianceRatio(series, q = 5) {
  const n = series.length;
  if (n < q * 4) return { q, vr: 1, statistic: 0, pValue: 1, n };
  const m = mean(series);
  const dev = series.map((x) => x - m);
  const varOne = variance(series);
  if (varOne === 0) return { q, vr: 1, statistic: 0, pValue: 1, n };
  let sum = 0;
  let count = 0;
  for (let t = q - 1; t < n; t += 1) {
    let acc = 0;
    for (let k = 0; k < q; k += 1) acc += dev[t - k];
    sum += acc * acc;
    count += 1;
  }
  const varQ = count > 0 ? sum / (count * q) : varOne;
  const vr = varQ / varOne;
  const varVr = (2 * (2 * q - 1) * (q - 1)) / (3 * q * n);
  const statistic = varVr > 0 ? (vr - 1) / Math.sqrt(varVr) : 0;
  return { q, vr, statistic, pValue: twoSidedP(statistic), n };
}

/** P(r_t+1 > 0 | r_t > 0) pooled across sectors, against the unconditional base rate. */
export function conditionalPersistence(matrix) {
  let joint = 0;
  let conditioned = 0;
  let positive = 0;
  let total = 0;
  const nSectors = matrix[0]?.length ?? 0;
  for (let i = 0; i < nSectors; i += 1) {
    for (let t = 0; t + 1 < matrix.length; t += 1) {
      const now = matrix[t][i];
      const next = matrix[t + 1][i];
      if (now > 0) {
        conditioned += 1;
        if (next > 0) joint += 1;
      }
      total += 1;
      if (next > 0) positive += 1;
    }
  }
  const p = conditioned > 0 ? joint / conditioned : 0;
  const base = total > 0 ? positive / total : 0;
  const se = conditioned > 0 ? Math.sqrt((p * (1 - p)) / conditioned) : 0;
  const statistic = se > 0 ? (p - base) / se : 0;
  return { value: p, baseRate: base, statistic, pValue: twoSidedP(statistic), n: conditioned };
}

/** Structural state by period: dispersion and pairwise correlation of the cross-section. */
export function rollingStructure(matrix, window = 63) {
  const nSectors = matrix[0]?.length ?? 0;
  const dispersion = [];
  const avgCorr = [];
  const medianCorr = [];
  for (let t = window; t < matrix.length; t += 1) {
    const block = matrix.slice(t - window, t);
    const perSectorSeries = [];
    for (let i = 0; i < nSectors; i += 1) perSectorSeries.push(block.map((row) => row[i]));
    dispersion.push(mean(block.map((row) => stdev(row))));
    const pairs = [];
    for (let i = 0; i < nSectors; i += 1) {
      for (let j = i + 1; j < nSectors; j += 1) pairs.push(corr(perSectorSeries[i], perSectorSeries[j]));
    }
    avgCorr.push(mean(pairs));
    medianCorr.push(quantile(pairs, 0.5));
  }
  return { dispersion, avgCorr, medianCorr, window };
}

/** Sharpe-style ratio and drawdown from an equity path. */
export function performance(equity, periodsPerYear = TRADING_DAYS) {
  const n = equity.length;
  if (n < 2) return { cagr: 0, vol: 0, sharpe: 0, maxDrawdown: 0 };
  const rets = [];
  for (let i = 1; i < n; i += 1) rets.push(equity[i] / equity[i - 1] - 1);
  // n equity points span n - 1 return periods. Getting this wrong biases every annualised figure.
  const years = (n - 1) / periodsPerYear;
  const cagr = equity[n - 1] > 0 ? Math.pow(equity[n - 1] / equity[0], 1 / years) - 1 : -1;
  const vol = stdev(rets) * Math.sqrt(periodsPerYear);
  const sharpe = vol > 0 ? (mean(rets) * periodsPerYear) / vol : 0;
  let peak = equity[0];
  let maxDd = 0;
  for (const value of equity) {
    if (value > peak) peak = value;
    const dd = peak > 0 ? value / peak - 1 : 0;
    if (dd < maxDd) maxDd = dd;
  }
  return { cagr, vol, sharpe, maxDrawdown: maxDd };
}

/** Rescaled-range Hurst exponent. Diagnostic only per Section 8.1. */
export function hurst(series, sizes = [8, 16, 32, 64, 128]) {
  const usable = sizes.filter((s) => s * 2 <= series.length);
  const xs = [];
  const ys = [];
  for (const size of usable) {
    const rsValues = [];
    const blocks = Math.floor(series.length / size);
    for (let b = 0; b < blocks; b += 1) {
      const block = series.slice(b * size, (b + 1) * size);
      const m = mean(block);
      let cumulative = 0;
      let min = Infinity;
      let max = -Infinity;
      for (const x of block) {
        cumulative += x - m;
        if (cumulative < min) min = cumulative;
        if (cumulative > max) max = cumulative;
      }
      const range = max - min;
      const sd = stdev(block, 0);
      if (sd > 0 && range > 0) rsValues.push(range / sd);
    }
    if (rsValues.length > 0) {
      xs.push(Math.log(size));
      ys.push(Math.log(mean(rsValues)));
    }
  }
  if (xs.length < 3) return { h: NaN, points: [] };
  const mx = mean(xs);
  const my = mean(ys);
  let num = 0;
  let den = 0;
  for (let i = 0; i < xs.length; i += 1) {
    num += (xs[i] - mx) * (ys[i] - my);
    den += (xs[i] - mx) * (xs[i] - mx);
  }
  return { h: den > 0 ? num / den : NaN, points: xs.map((x, i) => [x, ys[i]]) };
}

/* ------------------------------------------------------------------ *
 * Regime classification - Section 2 with the Section 8.1 hierarchy
 * ------------------------------------------------------------------ */

export const DEFAULT_CONFIG = {
  alpha: 0.05,
  bonferroni: true,
  deadband: { rankAutocorr: 0.05, returnAutocorr: 0.05, varianceRatio: 0.2 },
  minObs: 252,
  varianceRatioQ: 5,
  rollWindow: 63,
  interval: 'daily-band',
  band: 0.05,
  costBps: 5,
  // Placeholder expense ratio in basis points per fund. Replace with prospectus figures.
  expenseBps: 10,
  overlay: 'none',
  overlayLookback: 126,
  overlaySkip: 21,
  overlayTop: 3,
  overlayFrequency: 'monthly',
  holdoutFraction: 0.3,
  seed: 20261002,
};

export const TIER = { PRIMARY: 'primary', SECONDARY: 'secondary', DIAGNOSTIC: 'diagnostic' };

/** Build the test table. Tier drives whether a test may decide the regime. */
export function buildTests(metrics) {
  return [
    {
      key: 'rankAutocorr',
      name: 'Cross-sectional rank autocorrelation',
      tier: TIER.PRIMARY,
      statistic: metrics.rankAutocorr.statistic,
      pValue: metrics.rankAutocorr.pValue,
      value: metrics.rankAutocorr.value,
      direction: Math.sign(metrics.rankAutocorr.value),
      n: metrics.rankAutocorr.n,
    },
    {
      key: 'returnAutocorr',
      name: 'Return autocorrelation (pooled)',
      tier: TIER.PRIMARY,
      statistic: metrics.returnAutocorr.statistic,
      pValue: metrics.returnAutocorr.pValue,
      value: metrics.returnAutocorr.pooledR,
      direction: Math.sign(metrics.returnAutocorr.pooledR),
      n: metrics.returnAutocorr.n,
    },
    {
      key: 'varianceRatio',
      name: `Variance ratio (q=${metrics.varianceRatio.q})`,
      tier: TIER.PRIMARY,
      statistic: metrics.varianceRatio.statistic,
      pValue: metrics.varianceRatio.pValue,
      value: metrics.varianceRatio.vr,
      direction: Math.sign(metrics.varianceRatio.vr - 1),
      n: metrics.varianceRatio.n,
    },
    {
      key: 'conditionalPersistence',
      name: 'Conditional persistence',
      tier: TIER.SECONDARY,
      statistic: metrics.persistence.statistic,
      pValue: metrics.persistence.pValue,
      value: metrics.persistence.value,
      direction: Math.sign(metrics.persistence.value - metrics.persistence.baseRate),
      n: metrics.persistence.n,
    },
    {
      key: 'rankTurnover',
      name: 'Rank turnover (lower is more persistent)',
      tier: TIER.SECONDARY,
      statistic: NaN,
      pValue: NaN,
      value: metrics.rankTurnover,
      direction: 0,
      n: metrics.rankAutocorr.n,
    },
    {
      key: 'dispersion',
      name: 'Cross-sectional dispersion (daily, mean)',
      tier: TIER.SECONDARY,
      statistic: NaN,
      pValue: NaN,
      value: metrics.dispersion,
      direction: 0,
      n: metrics.structure.dispersion.length,
    },
    {
      key: 'avgCorrelation',
      name: 'Average pairwise correlation',
      tier: TIER.SECONDARY,
      statistic: NaN,
      pValue: NaN,
      value: metrics.avgCorrelation,
      direction: 0,
      n: metrics.structure.avgCorr.length,
    },
    {
      key: 'hurst',
      name: 'Hurst exponent (R/S, diagnostic only)',
      tier: TIER.DIAGNOSTIC,
      statistic: NaN,
      pValue: NaN,
      value: metrics.hurst.h,
      direction: 0,
      n: metrics.structure.dispersion.length,
    },
  ];
}

/**
 * Pre-registered decision rule, frozen before looking at data:
 *   - alpha is Bonferroni-corrected across the three primary tests
 *   - a significant positive primary test admits positive dependence, negative admits contrarian
 *   - conflicting directions, or an effect outside the deadband without significance, is UNCERTAIN
 *   - no significance and every effect inside the deadband is MEMORYLESS
 *   - too few observations is UNCERTAIN, never MEMORYLESS
 */
export function classifyRegime(metrics, config = DEFAULT_CONFIG) {
  const tests = buildTests(metrics);
  const primary = tests.filter((t) => t.tier === TIER.PRIMARY);
  const alpha = config.bonferroni ? config.alpha / primary.length : config.alpha;
  const reasons = [];

  if (metrics.observations < config.minObs) {
    reasons.push(
      `${metrics.observations} observations is below the minimum of ${config.minObs}; no regime can be claimed`,
    );
    return { label: 'UNCERTAIN', confidence: 'low', reasons, tests, alphaUsed: alpha };
  }

  /** Is the effect large enough to be worth acting on, not merely detectable? */
  const material = (t) => {
    const band = config.deadband[t.key];
    if (band === undefined) return true;
    if (t.key === 'varianceRatio') return Math.abs(t.value - 1) > band;
    return Math.abs(t.value) > band;
  };

  const rejecting = primary.filter((t) => t.pValue < alpha);
  // Significance alone is not enough. With thousands of daily observations an economically trivial
  // effect is easy to detect, and admitting an overlay on it would trade a rounding error against a
  // spread. An overlay requires significance AND an effect outside the deadband.
  const admitted = rejecting.filter(material);
  const trivial = rejecting.filter((t) => !material(t));
  const positive = admitted.filter((t) => t.direction > 0);
  const negative = admitted.filter((t) => t.direction < 0);

  for (const t of admitted) {
    reasons.push(
      `${t.name} = ${fmt(t.value)} rejects at alpha ${alpha.toFixed(4)} (p=${t.pValue.toFixed(4)}) and clears the deadband`,
    );
  }
  for (const t of trivial) {
    reasons.push(
      `${t.name} = ${fmt(t.value)} is statistically detectable (p=${t.pValue.toFixed(4)}) but sits inside the deadband, so it is not treated as signal`,
    );
  }

  if (positive.length > 0 && negative.length > 0) {
    reasons.push('Primary tests disagree in direction, which is treated as no usable signal');
    return { label: 'UNCERTAIN', confidence: 'low', reasons, tests, alphaUsed: alpha };
  }

  // Outside the deadband but not significant: not signal, and not ignorable either.
  const outsideDeadband = primary.filter((t) => material(t) && t.pValue >= alpha);

  if (positive.length > 0) {
    const strongest = positive.reduce((a, b) => (a.pValue < b.pValue ? a : b));
    const confidence = strongest.pValue < alpha / 10 && strongest.n >= config.minObs ? 'high' : 'medium';
    reasons.push('Positive dependence is admissible; a momentum overlay is permitted');
    return { label: 'POSITIVE_DEPENDENCE', confidence, reasons, tests, alphaUsed: alpha };
  }
  if (negative.length > 0) {
    const strongest = negative.reduce((a, b) => (a.pValue < b.pValue ? a : b));
    const confidence = strongest.pValue < alpha / 10 && strongest.n >= config.minObs ? 'high' : 'medium';
    reasons.push('Negative dependence is admissible; a contrarian overlay is permitted');
    return { label: 'NEGATIVE_DEPENDENCE', confidence, reasons, tests, alphaUsed: alpha };
  }

  if (outsideDeadband.length > 0) {
    for (const t of outsideDeadband) {
      reasons.push(`${t.name} = ${fmt(t.value)} is outside the deadband but not significant`);
    }
    reasons.push('Point estimates are not signal; the regime is uncertain and no overlay is permitted');
    return { label: 'UNCERTAIN', confidence: 'low', reasons, tests, alphaUsed: alpha };
  }

  reasons.push('No primary test rejects and every effect is inside its deadband');
  reasons.push('Absence of evidence is not evidence of absence: the label is memoryless, not proven iid');
  return {
    label: 'MEMORYLESS',
    confidence: metrics.observations >= config.minObs * 4 ? 'medium' : 'low',
    reasons,
    tests,
    alphaUsed: alpha,
  };
}

/** Section 9: eligibility gating, not ranking. */
export function eligibility(regime) {
  const allowsMomentum = regime === 'POSITIVE_DEPENDENCE';
  const allowsContrarian = regime === 'NEGATIVE_DEPENDENCE';
  return [
    {
      strategy: 'Diversified rebalancing',
      condition: 'Dispersion with imperfect correlation',
      enabled: true,
      reason: 'Structural layer; requires no forecast',
    },
    {
      strategy: 'Buy and hold',
      condition: 'No predictive assumption',
      enabled: true,
      reason: 'Baseline for every comparison',
    },
    {
      strategy: 'Momentum overlay',
      condition: 'Positive dependence, significant',
      enabled: allowsMomentum,
      reason: allowsMomentum ? 'Regime admits it' : `Blocked: regime is ${regime}`,
    },
    {
      strategy: 'Contrarian overlay',
      condition: 'Negative dependence, significant',
      enabled: allowsContrarian,
      reason: allowsContrarian ? 'Regime admits it' : `Blocked: regime is ${regime}`,
    },
    {
      strategy: 'Dispersion / pairs',
      condition: 'Stable spread relationship',
      enabled: false,
      reason: 'Not implemented; evidence thin and specification dependent',
    },
    {
      strategy: 'Business-cycle rotation',
      condition: 'Reliable cycle to sector relationship',
      enabled: false,
      reason: 'Excluded by the document: no reliable relationship established',
    },
  ];
}

/* ------------------------------------------------------------------ *
 * Analysis entry point
 * ------------------------------------------------------------------ */

export function rankTurnoverSeries(matrix) {
  const turnover = [];
  for (let t = 0; t + 1 < matrix.length; t += 1) {
    const a = ranks(matrix[t]);
    const b = ranks(matrix[t + 1]);
    const n = a.length;
    let acc = 0;
    for (let i = 0; i < n; i += 1) acc += Math.abs(a[i] - b[i]);
    turnover.push(acc / n);
  }
  return turnover;
}

export function analyze({ matrix, sectorNames, config = DEFAULT_CONFIG }) {
  const returnAutocorr = returnAutocorrelation(matrix, 1);
  const rankAutocorr = rankAutocorrelation(matrix);
  const varianceRatioResult = varianceRatio(portfolioAggregate(matrix), config.varianceRatioQ);
  const persistence = conditionalPersistence(matrix);
  const structure = rollingStructure(matrix, config.rollWindow);
  const turnoverSeries = rankTurnoverSeries(matrix);
  const hurstResult = hurst(
    structure.dispersion.length > 0 ? structure.dispersion : matrix.map((r) => r[0]),
  );

  const nSectors = sectorNames.length;
  const columns = [];
  for (let i = 0; i < nSectors; i += 1) columns.push(matrix.map((row) => row[i]));
  const pairs = [];
  const correlationMatrix = [];
  for (let i = 0; i < nSectors; i += 1) {
    correlationMatrix.push([]);
    for (let j = 0; j < nSectors; j += 1) {
      if (i === j) {
        correlationMatrix[i].push(1);
      } else if (j < i) {
        correlationMatrix[i].push(correlationMatrix[j][i]);
      } else {
        const c = corr(columns[i], columns[j]);
        correlationMatrix[i].push(c);
        pairs.push(c);
      }
    }
  }

  const metrics = {
    observations: matrix.length,
    sectors: nSectors,
    returnAutocorr,
    rankAutocorr,
    varianceRatio: varianceRatioResult,
    persistence,
    structure,
    correlationMatrix,
    pairs,
    rankTurnover: mean(turnoverSeries),
    turnoverSeries,
    dispersion: mean(structure.dispersion),
    avgCorrelation: mean(pairs),
    medianCorrelation: quantile(pairs, 0.5),
    hurst: hurstResult,
  };
  const regime = classifyRegime(metrics, config);

  // Implied rebalancing premium for this universe: Section 4.3 of the document.
  const vols = columns.map((c) => stdev(c) * Math.sqrt(TRADING_DAYS));
  const weights = new Array(nSectors).fill(1 / nSectors);
  const impliedPremium = theoreticalPremium(vols, correlationMatrix, weights);

  return { metrics, regime, eligibility: eligibility(regime.label), impliedPremium, vols };
}

/** Equal-weight portfolio return per day, used for the variance ratio. */
export function portfolioAggregate(matrix) {
  return matrix.map((row) => mean(row));
}

/**
 * Section 4.2: premium = (1/2) * [ sum(w_i * sigma_i^2) - portfolio_variance ], annualised.
 */
export function theoreticalPremium(annualVols, correlationMatrix, weights) {
  const n = weights.length;
  let weightedVar = 0;
  for (let i = 0; i < n; i += 1) weightedVar += weights[i] * annualVols[i] * annualVols[i];
  let portfolioVar = 0;
  for (let i = 0; i < n; i += 1) {
    for (let j = 0; j < n; j += 1) {
      portfolioVar += weights[i] * weights[j] * annualVols[i] * annualVols[j] * correlationMatrix[i][j];
    }
  }
  return 0.5 * (weightedVar - portfolioVar);
}

/* ------------------------------------------------------------------ *
 * Backtest - Section 10
 * ------------------------------------------------------------------ */

/**
 * Simulate three parallel books plus a separate cost ledger:
 *
 *   basket  - target weights, allowed to drift, gross of costs (comparison type B baseline)
 *   struct  - drift-band rebalanced to equal weight, gross of costs (Layer A, comparison type A/B)
 *   actual  - the book actually run: Layer A plus the overlay target when the regime admits one
 *   net     - the actual book with trading cost and holding cost applied
 *
 * Gross-vs-net is deliberate. Trading and holding costs are never folded into the mechanism series,
 * because Section 10.6 requires them as separate attribution lines. The net path is built
 * multiplicatively so that log returns are exactly additive across the decomposition.
 */
export function backtest({ matrix, config, regimeLabel }) {
  const nSectors = matrix[0].length;
  const target = new Array(nSectors).fill(1 / nSectors);
  const expenses =
    (config.expenses ?? []).length === nSectors
      ? config.expenses
      : new Array(nSectors).fill(config.expenseBps ?? 0);
  const holdingDaily = expenses.map((bps) => bps / 10000 / TRADING_DAYS);
  const costRate = config.costBps / 10000;

  const basket = [1];
  const struct = [1];
  const actual = [1];
  const net = [1];

  let basketWeights = [...target];
  let structWeights = [...target];
  let actualWeights = [...target];
  let wanted = [...target];

  const allowOverlay =
    regimeLabel === 'POSITIVE_DEPENDENCE' || regimeLabel === 'NEGATIVE_DEPENDENCE';
  const overlayActive = config.overlay !== 'none' && allowOverlay;

  const ledger = { trading: 0, holding: 0, rebalances: 0, structTurnover: 0, actualTurnover: 0 };
  const rebalanceFlags = [];
  let overlayRebalances = 0;

  for (let t = 1; t < matrix.length; t += 1) {
    const row = matrix[t];

    // Mark to market, then renormalise so weights stay fractions of portfolio value.
    const basketStep = grow(basketWeights, row);
    const structStep = grow(structWeights, row);
    const actualStep = grow(actualWeights, row);
    basketWeights = basketStep.weights;
    structWeights = structStep.weights;
    actualWeights = actualStep.weights;

    if (overlayActive && overlayDue(t, config)) {
      wanted = overlayTargetWeights(matrix, t, config, target);
      overlayRebalances += 1;
    }

    const calendarDue = calendarTrigger(t, config.interval);
    const actualBreach = breaches(actualWeights, wanted, config.band);
    const structBreach = breaches(structWeights, target, config.band);

    let tradingDrag = 0;
    if (actualBreach || calendarDue) {
      const turnover = halfTurnover(actualWeights, wanted);
      const drag = turnover * costRate;
      tradingDrag += drag;
      ledger.trading += Math.log(1 - drag);
      ledger.actualTurnover += turnover;
      ledger.rebalances += 1;
      actualWeights = [...wanted];
    }
    if (structBreach || calendarDue) {
      ledger.structTurnover += halfTurnover(structWeights, target);
      structWeights = [...target];
    }
    rebalanceFlags.push(actualBreach || calendarDue);

    // Holding cost is a continuous drag on each holding, kept separate from trading cost.
    let holdingDrag = 0;
    for (let i = 0; i < nSectors; i += 1) holdingDrag += actualWeights[i] * holdingDaily[i];
    ledger.holding += Math.log(1 - holdingDrag);

    basket.push(basket[basket.length - 1] * (1 + basketStep.ret));
    struct.push(struct[struct.length - 1] * (1 + structStep.ret));
    actual.push(actual[actual.length - 1] * (1 + actualStep.ret));
    const netReturn = (1 + actualStep.ret) * (1 - tradingDrag) * (1 - holdingDrag) - 1;
    net.push(net[net.length - 1] * (1 + netReturn));
  }

  return {
    basket,
    struct,
    actual,
    net,
    ledger,
    rebalanceFlags,
    overlayActive,
    overlayRebalances,
    expenses,
  };
}

/** Mark to market: returns the drifted weights, the gross return, and the normalised weights. */
function grow(weights, row) {
  const value = weights.map((w, i) => w * (1 + row[i]));
  let gross = 0;
  for (const v of value) gross += v;
  return { ret: gross - 1, weights: value.map((v) => v / gross) };
}

/**
 * Drift band as a fraction of the target weight, which is the standard definition and the only one
 * that scales across a portfolio: with eleven sectors an absolute 5 percentage point deviation from
 * a 9.09 percent target is a 55 percent relative move, so higher bands would never trigger and the
 * frequency-and-band grid would be meaningless.
 */
export function breaches(weights, target, band) {
  for (let i = 0; i < weights.length; i += 1) {
    if (target[i] > 0 && Math.abs(weights[i] / target[i] - 1) > band) return true;
  }
  return false;
}

/** One-way turnover between two weight vectors. */
export function halfTurnover(from, to) {
  let acc = 0;
  for (let i = 0; i < from.length; i += 1) acc += Math.abs(to[i] - from[i]);
  return acc / 2;
}

function calendarTrigger(t, interval) {
  if (interval === 'weekly') return t % 5 === 0;
  if (interval === 'monthly') return t % 21 === 0;
  if (interval === 'quarterly') return t % 63 === 0;
  // 'daily-band' checks the band every day and has no calendar component.
  return false;
}

function overlayDue(t, config) {
  if (config.overlayFrequency === 'weekly') return t % 5 === 0;
  if (config.overlayFrequency === 'quarterly') return t % 63 === 0;
  return t % 21 === 0;
}

/** Momentum uses top-N by lookback return skipping the most recent month; contrarian uses bottom-N. */
export function overlayTargetWeights(matrix, t, config, target) {
  const nSectors = target.length;
  const start = Math.max(0, t - config.overlaySkip - config.overlayLookback);
  const end = Math.max(0, t - config.overlaySkip);
  if (end <= start) return [...target];
  const scores = [];
  for (let i = 0; i < nSectors; i += 1) {
    let acc = 1;
    for (let k = start; k < end; k += 1) acc *= 1 + matrix[k][i];
    scores.push({ i, score: acc - 1 });
  }
  scores.sort((a, b) => b.score - a.score);
  const top = Math.max(1, Math.min(config.overlayTop, nSectors));
  const selected =
    config.overlay === 'contrarian' ? scores.slice(scores.length - top) : scores.slice(0, top);
  const weights = new Array(nSectors).fill(0);
  for (const { i } of selected) weights[i] = 1 / selected.length;
  return weights;
}

/**
 * Sections 4.1 and 10.6: attribute the result across comparison types, keeping them apart.
 * All lines are sums of log returns, so they are exactly additive and the identity is tested.
 */
export function attribution({ matrix, result, sectorNames, config }) {
  const nSectors = sectorNames.length;
  const target = new Array(nSectors).fill(1 / nSectors);
  // The backtest consumes rows 1..T-1, so there are T-1 return periods, not T.
  const years = (matrix.length - 1) / TRADING_DAYS;

  const componentLog = [];
  for (let i = 0; i < nSectors; i += 1) {
    let acc = 1;
    for (const row of matrix) acc *= 1 + row[i];
    componentLog.push(Math.log(acc));
  }
  const weightedComponentLog = componentLog.reduce((acc, l, i) => acc + target[i] * l, 0);
  const basketLog = Math.log(result.basket[result.basket.length - 1]);
  const structLog = Math.log(result.struct[result.struct.length - 1]);
  const actualLog = Math.log(result.actual[result.actual.length - 1]);
  const netLog = Math.log(result.net[result.net.length - 1]);
  const trading = result.ledger.trading;
  const holding = result.ledger.holding;

  const lines = [
    { key: 'basket', label: 'Underlying sector exposure (drifting basket)', value: basketLog },
    { key: 'rebalanceB', label: 'Rebalancing contribution (type B, vs drifting)', value: structLog - basketLog },
    { key: 'overlay', label: 'Directional overlay contribution (exposure differs)', value: actualLog - structLog },
    { key: 'trading', label: 'Trading cost', value: trading },
    { key: 'holding', label: 'Holding cost (expense ratios)', value: holding },
  ];
  const sum = lines.reduce((acc, l) => acc + l.value, 0);

  return {
    years,
    lines,
    sum,
    total: netLog,
    residual: sum - netLog,
    compounded: Math.exp(netLog) - 1,
    // Type A: growth rate against the weighted average of component growth rates. This is the
    // comparison the Section 4.2 formula describes, and it is not the same as type B.
    typeA: {
      label: 'Rebalancing contribution (type A, growth vs component average)',
      value: structLog - weightedComponentLog,
      annual: (structLog - weightedComponentLog) / years,
    },
    typeB: {
      label: 'Rebalancing contribution (type B, vs drifting buy and hold)',
      value: structLog - basketLog,
      annual: (structLog - basketLog) / years,
    },
    overlay: {
      label: 'Directional overlay contribution (not pure alpha)',
      value: actualLog - structLog,
      annual: (actualLog - structLog) / years,
    },
    tradingCost: { label: 'Trading cost', value: trading, annual: trading / years },
    holdingCost: { label: 'Holding cost', value: holding, annual: holding / years },
  };
}

/** Section 10.4: fixed grid over rebalance frequency and drift band. */
export function runGrid({ matrix, config, regimeLabel, sectorNames }) {
  const frequencies = ['daily-band', 'weekly', 'monthly', 'quarterly'];
  const bands = [0.05, 0.1, 0.15, 0.2];
  const rows = [];
  for (const interval of frequencies) {
    for (const band of bands) {
      const cfg = { ...config, interval, band };
      const result = backtest({ matrix, config: cfg, regimeLabel });
      const perf = performance(result.net);
      const attr = attribution({ matrix, result, sectorNames, config: cfg });
      rows.push({
        interval,
        band,
        cagr: perf.cagr,
        vol: perf.vol,
        sharpe: perf.sharpe,
        maxDrawdown: perf.maxDrawdown,
        rebalances: result.ledger.rebalances,
        turnover: result.ledger.structTurnover,
        rebalancingContribution: attr.typeB.annual,
        tradingCost: attr.tradingCost.annual,
      });
    }
  }
  return rows;
}

/* ------------------------------------------------------------------ *
 * Pre-registration helper
 * ------------------------------------------------------------------ */

export function configFingerprint(config) {
  const ordered = {};
  for (const key of Object.keys(config).sort()) {
    const value = config[key];
    if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
      const nested = {};
      for (const k of Object.keys(value).sort()) nested[k] = value[k];
      ordered[key] = nested;
    } else {
      ordered[key] = value;
    }
  }
  const text = JSON.stringify(ordered);
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i += 1) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return { hash: h.toString(16).padStart(8, '0'), text };
}

export function fmt(x, digits = 4) {
  if (!Number.isFinite(x)) return 'n/a';
  return x.toFixed(digits);
}
