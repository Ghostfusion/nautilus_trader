/**
 * Data layer: universe definition, synthetic universes with a known ground-truth regime, CSV import.
 *
 * The synthetic generator exists for one reason: the document says the regime decision is a
 * measurement, so the measuring instrument has to be validated against data whose regime is known by
 * construction. See tests/engine.test.mjs.
 */

import { gaussian, mean, mulberry32, stdev, TRADING_DAYS } from './engine.js';

/**
 * The eleven Select Sector SPDR funds used as the reference universe in the document (Section 7).
 *
 * The expense ratio field is a PLACEHOLDER of 10 basis points for every fund. It is an input to the
 * cost model and must be replaced with the current prospectus figure before any conclusion is drawn
 * from a net-of-cost number.
 */
export const SECTOR_UNIVERSE = [
  { symbol: 'XLC', name: 'Communication Services' },
  { symbol: 'XLY', name: 'Consumer Discretionary' },
  { symbol: 'XLP', name: 'Consumer Staples' },
  { symbol: 'XLE', name: 'Energy' },
  { symbol: 'XLF', name: 'Financials' },
  { symbol: 'XLV', name: 'Health Care' },
  { symbol: 'XLI', name: 'Industrials' },
  { symbol: 'XLB', name: 'Materials' },
  { symbol: 'XLRE', name: 'Real Estate' },
  { symbol: 'XLK', name: 'Technology' },
  { symbol: 'XLU', name: 'Utilities' },
];

export const PLACEHOLDER_EXPENSE_BPS = 10;

export const REGIME_GROUND_TRUTH = {
  memoryless: {
    label: 'MEMORYLESS',
    persistence: 0,
    description: 'Independent returns. No temporal dependence by construction.',
  },
  persistent: {
    label: 'POSITIVE_DEPENDENCE',
    persistence: 0.8,
    description: 'Latent sector drift with positive autocorrelation: leaders tend to stay leaders.',
  },
  reverting: {
    label: 'NEGATIVE_DEPENDENCE',
    persistence: -0.6,
    description: 'Latent sector drift with negative autocorrelation: leaders tend to reverse.',
  },
  uncertain: {
    label: 'UNCERTAIN',
    persistence: 0,
    description: 'Short sample, below the minimum observation count. Cannot be classified.',
  },
};

/** Lower-triangular Cholesky factor of a symmetric positive-definite matrix. */
export function cholesky(matrix) {
  const n = matrix.length;
  const L = Array.from({ length: n }, () => new Array(n).fill(0));
  for (let i = 0; i < n; i += 1) {
    for (let j = 0; j <= i; j += 1) {
      let sum = matrix[i][j];
      for (let k = 0; k < j; k += 1) sum -= L[i][k] * L[j][k];
      if (i === j) {
        L[i][j] = Math.sqrt(Math.max(sum, 1e-12));
      } else {
        L[i][j] = sum / L[j][j];
      }
    }
  }
  return L;
}

/** Equal-correlation matrix with unit diagonal. */
export function equalCorrelation(n, rho) {
  const m = [];
  for (let i = 0; i < n; i += 1) {
    m.push([]);
    for (let j = 0; j < n; j += 1) m[i].push(i === j ? 1 : rho);
  }
  return m;
}

/**
 * Build a synthetic universe.
 *
 * Returns daily simple returns plus the ground-truth regime label, so a test can assert what the
 * classifier should say.
 */
export function syntheticUniverse({
  regime = 'memoryless',
  years = 8,
  nSectors = SECTOR_UNIVERSE.length,
  annualVol = 0.2,
  dispersion = 0.06,
  correlation = 0.6,
  latentShare = 0.3,
  seed = 20261002,
  startDate = '2017-01-03',
} = {}) {
  const spec = REGIME_GROUND_TRUTH[regime] ?? REGIME_GROUND_TRUTH.memoryless;
  const days = regime === 'uncertain' ? 200 : Math.round(years * TRADING_DAYS);
  const rand = mulberry32(seed);
  const corrMatrix = equalCorrelation(nSectors, correlation);
  const L = cholesky(corrMatrix);
  const dailyVol = annualVol / Math.sqrt(TRADING_DAYS);
  const dispersionDaily = dispersion / Math.sqrt(TRADING_DAYS);

  // Per-sector base volatility, spread around the target so the cross-section is not degenerate.
  const sectorVols = [];
  for (let i = 0; i < nSectors; i += 1) {
    const tilt = (i - (nSectors - 1) / 2) / Math.max(1, nSectors - 1);
    sectorVols.push(dailyVol * (1 + dispersionDaily * 4 * tilt));
  }

  const latent = new Array(nSectors).fill(0);
  const targetLatentVol = dailyVol * Math.sqrt(latentShare);
  const noiseVol = dailyVol * Math.sqrt(1 - latentShare);
  const phi = spec.persistence;

  const matrix = [];
  const dates = [];
  let cursor = new Date(`${startDate}T00:00:00Z`);
  for (let t = 0; t < days; t += 1) {
    // Latent persistent component, one per sector.
    for (let i = 0; i < nSectors; i += 1) {
      latent[i] = phi * latent[i] + gaussian(rand) * targetLatentVol;
    }
    // Correlated noise, drawn through the Cholesky factor.
    const z = [];
    for (let i = 0; i < nSectors; i += 1) z.push(gaussian(rand) * noiseVol);
    const correlated = [];
    for (let i = 0; i < nSectors; i += 1) {
      let acc = 0;
      for (let j = 0; j <= i; j += 1) acc += L[i][j] * z[j];
      correlated.push(acc * sectorVols[i] / dailyVol);
    }
    const row = [];
    for (let i = 0; i < nSectors; i += 1) row.push(latent[i] + correlated[i]);
    matrix.push(row);

    // Advance the calendar, skipping weekends.
    do {
      cursor = new Date(cursor.getTime() + 24 * 3600 * 1000);
    } while (cursor.getUTCDay() === 0 || cursor.getUTCDay() === 6);
    dates.push(cursor.toISOString().slice(0, 10));
  }

  return {
    matrix,
    dates,
    sectorNames: SECTOR_UNIVERSE.slice(0, nSectors).map((s) => s.symbol),
    groundTruth: spec.label,
    groundTruthDescription: spec.description,
    spec: { regime, years, nSectors, annualVol, correlation, seed, latentShare, persistence: phi },
  };
}

/**
 * Compounded price paths from a return matrix, for display and for CSV round-tripping.
 */
export function pricesFromReturns(matrix, start = 100) {
  const nSectors = matrix[0].length;
  const prices = [new Array(nSectors).fill(start)];
  for (const row of matrix) {
    const prev = prices[prices.length - 1];
    prices.push(row.map((r, i) => prev[i] * (1 + r)));
  }
  return prices;
}

export function returnsFromPrices(prices) {
  const out = [];
  for (let t = 1; t < prices.length; t += 1) {
    out.push(prices[t].map((p, i) => (prices[t - 1][i] > 0 ? p / prices[t - 1][i] - 1 : 0)));
  }
  return out;
}

/**
 * Parse a CSV with a header row of sector names and a first column of dates.
 * mode 'prices' (default) compounds to simple returns; mode 'returns' uses the values directly.
 */
export function parseCsv(text, { mode = 'prices' } = {}) {
  const lines = text.split(/\r?\n/).filter((l) => l.trim().length > 0);
  if (lines.length < 3) throw new Error('CSV needs a header row and at least two data rows');
  const header = lines[0].split(',').map((h) => h.trim().replace(/^"|"$/g, ''));
  const sectorNames = header.slice(1);
  const dates = [];
  const rows = [];
  for (let i = 1; i < lines.length; i += 1) {
    const cells = lines[i].split(',').map((c) => c.trim().replace(/^"|"$/g, ''));
    const values = cells.slice(1).map(Number);
    if (values.some((v) => !Number.isFinite(v))) continue;
    if (values.length !== sectorNames.length) continue;
    dates.push(cells[0]);
    rows.push(values);
  }
  if (rows.length < 3) throw new Error('CSV produced fewer than three usable rows');
  const matrix = mode === 'prices' ? returnsFromPrices(rows) : rows;
  return { matrix, dates: dates.slice(1), sectorNames, mode, rowCount: matrix.length };
}

/** Serialise a return matrix back to CSV, which is what the export button writes. */
export function toCsv({ matrix, dates, sectorNames }) {
  const header = ['date', ...sectorNames].join(',');
  const lines = [header];
  for (let t = 0; t < matrix.length; t += 1) {
    const date = dates?.[t + 1] ?? String(t + 1);
    lines.push([date, ...matrix[t].map((v) => v.toFixed(8))].join(','));
  }
  return lines.join('\n');
}

/** Summary used by both the UI and the tests. */
export function describeUniverse(matrix, sectorNames) {
  const nSectors = sectorNames.length;
  const vols = [];
  for (let i = 0; i < nSectors; i += 1) {
    vols.push(stdev(matrix.map((row) => row[i])) * Math.sqrt(TRADING_DAYS));
  }
  return {
    observations: matrix.length,
    sectors: nSectors,
    years: matrix.length / TRADING_DAYS,
    annualVol: mean(vols),
    firstDate: null,
    lastDate: null,
  };
}
