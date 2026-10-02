/**
 * Engine test suite. Run with:  bun test
 *
 * The point of these tests is not coverage for its own sake. It is that the application measures a
 * regime, and a measuring instrument has to be checked against data whose regime is known by
 * construction. The synthetic generator provides that ground truth.
 */

import { describe, expect, test } from 'bun:test';

import {
  DEFAULT_CONFIG,
  analyze,
  attribution,
  backtest,
  breaches,
  classifyRegime,
  configFingerprint,
  corr,
  eligibility,
  gaussian,
  halfTurnover,
  mean,
  mulberry32,
  overlayTargetWeights,
  performance,
  rankAutocorrelation,
  ranks,
  returnAutocorrelation,
  runGrid,
  spearman,
  stdev,
  theoreticalPremium,
  varianceRatio,
} from '../src/engine.js';

import { SECTOR_UNIVERSE, syntheticUniverse } from '../src/data.js';

const N = SECTOR_UNIVERSE.length;

function analyzeSynthetic(regime, seed, overrides = {}) {
  const universe = syntheticUniverse({ regime, seed, ...overrides });
  return { universe, ...analyze({ matrix: universe.matrix, sectorNames: universe.sectorNames, config: { ...DEFAULT_CONFIG, seed } }) };
}

describe('elementary statistics', () => {
  test('mean and stdev match hand-computed values', () => {
    expect(mean([1, 2, 3, 4])).toBeCloseTo(2.5, 12);
    expect(stdev([1, 2, 3, 4])).toBeCloseTo(Math.sqrt(5 / 3), 12);
    expect(stdev([5, 5, 5])).toBe(0);
  });

  test('corr is 1 for a perfect linear relationship and -1 for its reverse', () => {
    const x = [1, 2, 3, 4, 5];
    expect(corr(x, x.map((v) => 2 * v + 3))).toBeCloseTo(1, 12);
    expect(corr(x, x.map((v) => -v))).toBeCloseTo(-1, 12);
  });

  test('ranks resolve ties to the mean rank', () => {
    expect(ranks([10, 20, 20, 40])).toEqual([1, 2.5, 2.5, 4]);
  });

  test('spearman is 1 for any strictly monotone transform', () => {
    const x = [1, 2, 3, 4, 5, 6];
    const y = x.map((v) => Math.exp(v));
    expect(spearman(x, y)).toBeCloseTo(1, 10);
  });
});

describe('metric layer', () => {
  test('return autocorrelation detects a constructed AR(1) series', () => {
    // A genuinely stochastic recursion. A deterministic innovation would make the series periodic,
    // and a periodic signal can show negative lag-1 autocorrelation, which is how the first
    // version of this fixture failed.
    const phi = 0.7;
    const rand = mulberry32(20261002);
    const matrix = [];
    const previous = new Array(N).fill(0);
    for (let t = 0; t < 1200; t += 1) {
      const row = [];
      for (let i = 0; i < N; i += 1) {
        previous[i] = phi * previous[i] + gaussian(rand) * 0.01;
        row.push(previous[i]);
      }
      matrix.push(row);
    }
    const result = returnAutocorrelation(matrix, 1);
    expect(result.pooledR).toBeGreaterThan(0.5);
    expect(result.pValue).toBeLessThan(0.001);
  });

  test('variance ratio is about 1 for independent returns', () => {
    let acc = 0;
    let count = 0;
    for (const seed of [1, 2, 3, 4, 5]) {
      const u = syntheticUniverse({ regime: 'memoryless', seed, years: 6 });
      const aggregate = u.matrix.map((row) => mean(row));
      acc += varianceRatio(aggregate, 5).vr;
      count += 1;
    }
    expect(acc / count).toBeGreaterThan(0.8);
    expect(acc / count).toBeLessThan(1.2);
  });

  test('rank autocorrelation separates the three worlds', () => {
    // The primary test from Section 8.2, asserted directly rather than only through the classifier.
    const sample = (regime) => syntheticUniverse({ regime, seed: 3, years: 5 }).matrix;
    const memoryless = rankAutocorrelation(sample('memoryless'));
    const persistent = rankAutocorrelation(sample('persistent'));
    const reverting = rankAutocorrelation(sample('reverting'));
    expect(Math.abs(memoryless.value)).toBeLessThan(0.05);
    expect(memoryless.pValue).toBeGreaterThan(0.05);
    expect(persistent.value).toBeGreaterThan(0.05);
    expect(reverting.value).toBeLessThan(-0.05);
  });

  test('theoretical premium reduces to sigma^2 (1 - rho) / 4 for two assets', () => {
    const vol = 0.2;
    const rho = 0.25;
    const matrix = [
      [1, rho],
      [rho, 1],
    ];
    const premium = theoreticalPremium([vol, vol], matrix, [0.5, 0.5]);
    expect(premium).toBeCloseTo((vol * vol * (1 - rho)) / 4, 12);
  });
});

describe('regime classifier against known ground truth', () => {
  const seeds = [11, 22, 33, 44, 55, 66, 77, 88, 99, 110];
  const accuracy = (regime, expected) => {
    let correct = 0;
    for (const seed of seeds) {
      const { regime: classified } = analyzeSynthetic(regime, seed, { years: 6 });
      if (classified.label === expected) correct += 1;
    }
    return correct / seeds.length;
  };

  test('memoryless universes classify as MEMORYLESS at least 8 times in 10', () => {
    // A statistical classifier has a false positive rate; the claim is accuracy, not perfection.
    expect(accuracy('memoryless', 'MEMORYLESS')).toBeGreaterThanOrEqual(0.8);
  });

  test('persistent universes classify as POSITIVE_DEPENDENCE at least 8 times in 10', () => {
    expect(accuracy('persistent', 'POSITIVE_DEPENDENCE')).toBeGreaterThanOrEqual(0.8);
  });

  test('reverting universes classify as NEGATIVE_DEPENDENCE at least 8 times in 10', () => {
    expect(accuracy('reverting', 'NEGATIVE_DEPENDENCE')).toBeGreaterThanOrEqual(0.8);
  });

  test('a sample below the minimum observation count is UNCERTAIN, never MEMORYLESS', () => {
    for (const seed of seeds) {
      const { regime } = analyzeSynthetic('uncertain', seed);
      expect(regime.label).toBe('UNCERTAIN');
      expect(regime.confidence).toBe('low');
    }
  });

  test('Bonferroni correction tightens the threshold the decision uses', () => {
    const { metrics } = analyzeSynthetic('memoryless', 7, { years: 4 });
    const corrected = classifyRegime(metrics, { ...DEFAULT_CONFIG, bonferroni: true });
    const raw = classifyRegime(metrics, { ...DEFAULT_CONFIG, bonferroni: false });
    expect(corrected.alphaUsed).toBeCloseTo(raw.alphaUsed / 3, 12);
  });

  test('conflicting primary directions are reported as UNCERTAIN', () => {
    const metrics = {
      observations: 2000,
      rankAutocorr: { statistic: 3.5, pValue: 0.0005, value: 0.08, n: 1999 },
      returnAutocorr: { statistic: -3.5, pValue: 0.0005, pooledR: -0.08, n: 20000 },
      varianceRatio: { q: 5, statistic: 0.1, pValue: 0.9, vr: 1.01, n: 2000 },
      persistence: { value: 0.5, baseRate: 0.5, statistic: 0, pValue: 1, n: 1000 },
      structure: { dispersion: [0.01], avgCorr: [0.5], medianCorr: [0.5], window: 63 },
      rankTurnover: 3,
      dispersion: 0.01,
      avgCorrelation: 0.5,
      medianCorrelation: 0.5,
      hurst: { h: 0.5, points: [] },
    };
    const regime = classifyRegime(metrics, DEFAULT_CONFIG);
    expect(regime.label).toBe('UNCERTAIN');
  });

  test('an effect outside the deadband without significance is UNCERTAIN, not MEMORYLESS', () => {
    const metrics = {
      observations: 2000,
      rankAutocorr: { statistic: 1.2, pValue: 0.23, value: 0.09, n: 1999 },
      returnAutocorr: { statistic: 0.4, pValue: 0.69, pooledR: 0.01, n: 20000 },
      varianceRatio: { q: 5, statistic: 0.2, pValue: 0.84, vr: 1.02, n: 2000 },
      persistence: { value: 0.5, baseRate: 0.5, statistic: 0, pValue: 1, n: 1000 },
      structure: { dispersion: [0.01], avgCorr: [0.5], medianCorr: [0.5], window: 63 },
      rankTurnover: 3,
      dispersion: 0.01,
      avgCorrelation: 0.5,
      medianCorrelation: 0.5,
      hurst: { h: 0.5, points: [] },
    };
    expect(classifyRegime(metrics, DEFAULT_CONFIG).label).toBe('UNCERTAIN');
  });
});

describe('eligibility gating', () => {
  test('both overlays are blocked unless the matching regime is measured', () => {
    for (const regime of ['MEMORYLESS', 'UNCERTAIN']) {
      const rows = eligibility(regime);
      const momentum = rows.find((r) => r.strategy === 'Momentum overlay');
      const contrarian = rows.find((r) => r.strategy === 'Contrarian overlay');
      expect(momentum.enabled).toBe(false);
      expect(contrarian.enabled).toBe(false);
      expect(momentum.reason).toContain('Blocked');
    }
  });

  test('the structural layer is always enabled and business-cycle rotation never is', () => {
    for (const regime of ['MEMORYLESS', 'UNCERTAIN', 'POSITIVE_DEPENDENCE', 'NEGATIVE_DEPENDENCE']) {
      const rows = eligibility(regime);
      expect(rows.find((r) => r.strategy === 'Diversified rebalancing').enabled).toBe(true);
      expect(rows.find((r) => r.strategy === 'Business-cycle rotation').enabled).toBe(false);
    }
    expect(eligibility('POSITIVE_DEPENDENCE').find((r) => r.strategy === 'Momentum overlay').enabled).toBe(true);
    expect(eligibility('NEGATIVE_DEPENDENCE').find((r) => r.strategy === 'Contrarian overlay').enabled).toBe(true);
  });
});

describe('backtest and attribution', () => {
  const universe = syntheticUniverse({ regime: 'memoryless', seed: 4242, years: 6 });
  const config = { ...DEFAULT_CONFIG, costBps: 8, expenseBps: 12 };

  test('the attribution lines sum to the net result exactly', () => {
    const result = backtest({ matrix: universe.matrix, config, regimeLabel: 'MEMORYLESS' });
    const attr = attribution({ matrix: universe.matrix, result, sectorNames: universe.sectorNames, config });
    expect(Math.abs(attr.residual)).toBeLessThan(1e-12);
    expect(attr.lines.reduce((a, l) => a + l.value, 0)).toBeCloseTo(attr.total, 12);
  });

  test('costs reduce the result and are monotone in the cost assumption', () => {
    const cheap = backtest({ matrix: universe.matrix, config: { ...config, costBps: 1 }, regimeLabel: 'MEMORYLESS' });
    const dear = backtest({ matrix: universe.matrix, config: { ...config, costBps: 40 }, regimeLabel: 'MEMORYLESS' });
    expect(cheap.net[cheap.net.length - 1]).toBeGreaterThan(dear.net[dear.net.length - 1]);
    // Gross books must be identical: the cost assumption cannot leak into the mechanism series.
    expect(cheap.struct[cheap.struct.length - 1]).toBeCloseTo(dear.struct[dear.struct.length - 1], 12);
  });

  test('a zero band rebalances every day, a very wide band almost never', () => {
    const tight = backtest({ matrix: universe.matrix, config: { ...config, band: 0.0001 }, regimeLabel: 'MEMORYLESS' });
    const wide = backtest({ matrix: universe.matrix, config: { ...config, band: 5 }, regimeLabel: 'MEMORYLESS' });
    expect(tight.ledger.rebalances).toBeGreaterThan(universe.matrix.length * 0.9);
    expect(wide.ledger.rebalances).toBe(0);
  });

  test('holding cost is charged continuously and appears as its own line', () => {
    const free = backtest({ matrix: universe.matrix, config: { ...config, expenseBps: 0 }, regimeLabel: 'MEMORYLESS' });
    const charged = backtest({ matrix: universe.matrix, config: { ...config, expenseBps: 50 }, regimeLabel: 'MEMORYLESS' });
    expect(free.ledger.holding).toBe(0);
    expect(charged.ledger.holding).toBeLessThan(0);
    expect(charged.net[charged.net.length - 1]).toBeLessThan(free.net[free.net.length - 1]);
  });

  test('the overlay is inert when the regime does not admit it', () => {
    const gated = backtest({
      matrix: universe.matrix,
      config: { ...config, overlay: 'momentum' },
      regimeLabel: 'MEMORYLESS',
    });
    expect(gated.overlayActive).toBe(false);
    expect(gated.overlayRebalances).toBe(0);
    for (let t = 0; t < gated.actual.length; t += 1) {
      expect(gated.actual[t]).toBeCloseTo(gated.struct[t], 12);
    }
  });

  test('the overlay activates only in the regime that admits it', () => {
    const active = backtest({
      matrix: universe.matrix,
      config: { ...config, overlay: 'momentum' },
      regimeLabel: 'POSITIVE_DEPENDENCE',
    });
    expect(active.overlayActive).toBe(true);
    expect(active.overlayRebalances).toBeGreaterThan(0);
    expect(active.actual[active.actual.length - 1]).not.toBeCloseTo(active.struct[active.struct.length - 1], 6);
  });

  test('momentum selects the strongest sector and contrarian the weakest', () => {
    // A strictly monotone cross-section, so there are no ties and the expectation is exact:
    // sector 0 compounds fastest, sector N-1 slowest.
    const matrix = [];
    for (let t = 0; t < 300; t += 1) {
      const row = [];
      for (let i = 0; i < N; i += 1) row.push(0.0006 - i * 0.0001);
      matrix.push(row);
    }
    const target = new Array(N).fill(1 / N);
    const base = { ...config, overlayLookback: 100, overlaySkip: 0, overlayTop: 1 };

    const momentumWeights = overlayTargetWeights(matrix, 250, { ...base, overlay: 'momentum' }, target);
    expect(momentumWeights[0]).toBe(1);

    const contrarianWeights = overlayTargetWeights(matrix, 250, { ...base, overlay: 'contrarian' }, target);
    expect(contrarianWeights[N - 1]).toBe(1);
    expect(contrarianWeights.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 12);
  });

  test('the band is relative to the target weight, not absolute', () => {
    // With eleven sectors the difference is large: 5 percentage points of absolute deviation on a
    // 9.09 percent target is a 55 percent relative move, so the definition changes the behaviour
    // completely. Pinned here because the whole frequency-and-band grid depends on it.
    const target = [0.5, 0.5];
    // 9.8 percent relative deviation stays inside a 10 percent band.
    expect(breaches([0.549, 0.451], target, 0.1)).toBe(false);
    // 10.2 percent breaches it. The boundary itself is deliberately not asserted, because an
    // exact-equality assertion would be testing float representation rather than the rule.
    expect(breaches([0.551, 0.449], target, 0.1)).toBe(true);
    // Absolute semantics would treat a 0.01 deviation as a breach of a 0.1 band. Relative does not.
    expect(breaches([0.51, 0.49], target, 0.1)).toBe(false);
    // A zero band triggers on any drift, and nothing at the target itself.
    expect(breaches([0.5, 0.5], target, 0)).toBe(false);
    expect(breaches([0.5002, 0.4998], target, 0.0001)).toBe(true);
    expect(breaches([0.50001, 0.49999], target, 0.0001)).toBe(false);
  });

  test('half turnover is symmetric and bounded', () => {
    const a = [0.5, 0.5, 0, 0];
    const b = [0, 0, 0.5, 0.5];
    expect(halfTurnover(a, b)).toBeCloseTo(1, 12);
    expect(halfTurnover(a, a)).toBe(0);
  });

  test('performance recovers a known CAGR and drawdown', () => {
    const equity = [];
    for (let i = 0; i < 253; i += 1) equity.push(Math.pow(2, i / 252));
    const perf = performance(equity);
    expect(perf.cagr).toBeCloseTo(1, 2);
    expect(perf.maxDrawdown).toBeCloseTo(0, 12);

    const withDip = [1, 1.2, 0.6, 1.3];
    expect(performance(withDip).maxDrawdown).toBeCloseTo(0.6 / 1.2 - 1, 12);
  });

  test('the grid covers every frequency and band combination without error', () => {
    const small = syntheticUniverse({ regime: 'memoryless', seed: 5, years: 2 });
    const rows = runGrid({
      matrix: small.matrix,
      config: { ...DEFAULT_CONFIG, rollWindow: 63 },
      regimeLabel: 'MEMORYLESS',
      sectorNames: small.sectorNames,
    });
    expect(rows.length).toBe(16);
    for (const row of rows) {
      expect(Number.isFinite(row.cagr)).toBe(true);
      expect(Number.isFinite(row.sharpe)).toBe(true);
    }
  });

  test('results are deterministic for a fixed seed', () => {
    const a = syntheticUniverse({ regime: 'persistent', seed: 999 });
    const b = syntheticUniverse({ regime: 'persistent', seed: 999 });
    expect(a.matrix[10][3]).toBe(b.matrix[10][3]);
    const c = syntheticUniverse({ regime: 'persistent', seed: 1000 });
    expect(a.matrix[10][3]).not.toBe(c.matrix[10][3]);
  });
});

describe('pre-registration', () => {
  test('the configuration fingerprint ignores key order and tracks values', () => {
    const a = configFingerprint({ alpha: 0.05, band: 0.1 });
    const b = configFingerprint({ band: 0.1, alpha: 0.05 });
    const c = configFingerprint({ alpha: 0.05, band: 0.2 });
    expect(a.hash).toBe(b.hash);
    expect(a.hash).not.toBe(c.hash);
    expect(a.hash).toMatch(/^[0-9a-f]{8}$/);
  });

  test('the engine reports an implied premium consistent with the measured universe', () => {
    const { impliedPremium, metrics } = analyzeSynthetic('memoryless', 31337, { years: 6 });
    expect(Number.isFinite(impliedPremium)).toBe(true);
    // The premium is a variance difference and cannot be negative for a valid correlation matrix.
    expect(impliedPremium).toBeGreaterThan(-1e-9);
    expect(metrics.avgCorrelation).toBeGreaterThan(0);
    expect(metrics.avgCorrelation).toBeLessThan(1);
  });
});

describe('csv round trip', () => {
  test('returns survive a write and re-read', async () => {
    const { parseCsv, toCsv, pricesFromReturns, returnsFromPrices } = await import('../src/data.js');
    const universe = syntheticUniverse({ regime: 'memoryless', seed: 77, years: 1 });
    const prices = pricesFromReturns(universe.matrix);
    const csv = toCsv({
      matrix: universe.matrix,
      dates: universe.dates,
      sectorNames: universe.sectorNames,
    });
    const reparsed = parseCsv(csv, { mode: 'returns' });
    expect(reparsed.sectorNames.length).toBe(N);
    expect(reparsed.matrix.length).toBe(universe.matrix.length);
    const fromPrices = returnsFromPrices(prices);
    expect(fromPrices.length).toBe(universe.matrix.length);
    expect(Math.abs(fromPrices[5][2] - universe.matrix[5][2])).toBeLessThan(1e-9);
  });
});
