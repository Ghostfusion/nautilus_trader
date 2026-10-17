/**
 * Random layer: the seeded generator every stochastic part of the console uses.
 *
 * The discovery layer's rule is that a search is reproducible from its seed: the same seed and the
 * same inputs produce the same candidates in the same order, so a run can be replayed from a record
 * rather than trusted. Nothing here may use Math.random, because a part of the app that cannot be
 * replayed cannot be checked.
 */

/** FNV-1a over a string, so a seed may be a name or a run identity rather than a number. */
export function hashSeed(seed) {
  let h = 0x811c9dc5;
  const text = String(seed);
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h >>> 0;
}

/**
 * mulberry32: 32 bits of state, fast, and adequate for a demonstration whose claims are about
 * reproducibility rather than about cryptographic quality. Uniform on [0, 1).
 */
export function mulberry32(seed) {
  let a = (typeof seed === 'number' ? seed : hashSeed(seed)) >>> 0;
  return function next() {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** A generator with the conveniences the search and the feed generators need. */
export function makeRng(seed) {
  const next = mulberry32(seed);
  return {
    seed: typeof seed === 'number' ? seed : hashSeed(seed),
    next,
    /** An integer in [min, max]. */
    int(min, max) {
      return min + Math.floor(next() * (max - min + 1));
    },
    /** A real number in [min, max). */
    real(min, max) {
      return min + next() * (max - min);
    },
    /** True with probability p. */
    chance(p) {
      return next() < p;
    },
    /** A uniformly chosen element. Throws on an empty list rather than returning undefined. */
    pick(list) {
      if (list.length === 0) throw new Error('pick from an empty list');
      return list[Math.floor(next() * list.length)];
    },
    /** A shuffled copy; the argument is never mutated. */
    shuffle(list) {
      const out = list.slice();
      for (let i = out.length - 1; i > 0; i--) {
        const j = Math.floor(next() * (i + 1));
        [out[i], out[j]] = [out[j], out[i]];
      }
      return out;
    },
    /** A standard normal, by the Box-Muller transform. */
    normal() {
      let u = 0;
      while (u === 0) u = next();
      const v = next();
      return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
    },
  };
}

/** The same generator state, restarted: used by the replay views to show that a seed fixes a run. */
export function replay(seed, fn) {
  return fn(makeRng(seed));
}

/** Deterministic sample of n items from a list, without replacement. */
export function sample(seed, list, n) {
  return makeRng(seed).shuffle(list).slice(0, Math.min(n, list.length));
}

/** Mean of a list of numbers; the empty list is not a sample and has no mean. */
export function mean(values) {
  if (values.length === 0) throw new Error('mean of an empty list');
  return values.reduce((a, b) => a + b, 0) / values.length;
}

/** Sample standard deviation (n - 1). A single observation has no standard deviation. */
export function stdev(values) {
  if (values.length < 2) throw new Error('stdev needs at least two observations');
  const m = mean(values);
  const variance = values.reduce((acc, value) => acc + (value - m) ** 2, 0) / (values.length - 1);
  return Math.sqrt(variance);
}

/** Population skewness, used by the gate arithmetic where the sample is the whole record. */
export function skewness(values) {
  if (values.length < 3) throw new Error('skewness needs at least three observations');
  const m = mean(values);
  const s = Math.sqrt(values.reduce((acc, value) => acc + (value - m) ** 2, 0) / values.length);
  if (s === 0) return 0;
  return values.reduce((acc, value) => acc + ((value - m) / s) ** 3, 0) / values.length;
}

/** Population excess kurtosis (0 for a normal sample). */
export function kurtosis(values) {
  if (values.length < 4) throw new Error('kurtosis needs at least four observations');
  const m = mean(values);
  const s = Math.sqrt(values.reduce((acc, value) => acc + (value - m) ** 2, 0) / values.length);
  if (s === 0) return 0;
  return values.reduce((acc, value) => acc + ((value - m) / s) ** 4, 0) / values.length - 3;
}

/** Quantile of a sorted copy, by linear interpolation between order statistics. */
export function quantile(values, p) {
  if (values.length === 0) throw new Error('quantile of an empty list');
  const sorted = values.slice().sort((a, b) => a - b);
  const position = (sorted.length - 1) * Math.min(Math.max(p, 0), 1);
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  if (lower === upper) return sorted[lower];
  return sorted[lower] + (sorted[upper] - sorted[lower]) * (position - lower);
}
