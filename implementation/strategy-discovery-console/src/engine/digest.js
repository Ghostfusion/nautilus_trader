/**
 * Digest layer: a synchronous SHA-256, plus the canonical serialisation and the chain rule.
 *
 * WHY THIS IS NOT THE ENGINE'S DIGEST. The artifact of record is the Rust engine, whose canonical
 * digest is blake3 over a canonical serialisation. A browser cannot reproduce that without either a
 * dependency (a pure-JavaScript blake3 exists only as a package) or WebCrypto, and WebCrypto is not
 * reliably available to a page opened from the filesystem, which is how this console is meant to
 * run. So the console reproduces the DISCIPLINE - a canonical serialisation, a digest over it, and a
 * hash chain over the entries - with SHA-256, and labels every digest it prints as "sha256:" so that
 * a digest computed here is never mistaken for the engine's identity.
 *
 * sha256Bytes is checked against the published NIST vectors in tests/digest.test.mjs, including the
 * multi-block and the million-character cases, so it is either SHA-256 or the tests fail.
 */

const K = [
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const INITIAL = [
  0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

const rotr = (x, n) => ((x >>> n) | (x << (32 - n))) >>> 0;

function utf8Bytes(value) {
  return new TextEncoder().encode(value);
}

/** SHA-256 over bytes or a string (encoded as UTF-8). Returns 32 bytes. */
export function sha256Bytes(input) {
  const data = typeof input === 'string' ? utf8Bytes(input) : input;
  const len = data.length;
  const bitLen = len * 8;
  const padded = new Uint8Array(Math.ceil((len + 9) / 64) * 64);
  padded.set(data);
  padded[len] = 0x80;

  const view = new DataView(padded.buffer);
  view.setUint32(padded.length - 8, Math.floor(bitLen / 0x100000000), false);
  view.setUint32(padded.length - 4, bitLen >>> 0, false);

  const h = Uint32Array.from(INITIAL);
  const w = new Uint32Array(64);

  for (let offset = 0; offset < padded.length; offset += 64) {
    for (let i = 0; i < 16; i++) w[i] = view.getUint32(offset + i * 4, false);
    for (let i = 16; i < 64; i++) {
      const x = w[i - 15];
      const y = w[i - 2];
      const s0 = (rotr(x, 7) ^ rotr(x, 18) ^ (x >>> 3)) >>> 0;
      const s1 = (rotr(y, 17) ^ rotr(y, 19) ^ (y >>> 10)) >>> 0;
      w[i] = (w[i - 16] + s0 + w[i - 7] + s1) >>> 0;
    }

    let a = h[0];
    let b = h[1];
    let c = h[2];
    let d = h[3];
    let e = h[4];
    let f = h[5];
    let g = h[6];
    let hh = h[7];

    for (let i = 0; i < 64; i++) {
      const S1 = (rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25)) >>> 0;
      const ch = ((e & f) ^ (~e & g)) >>> 0;
      const t1 = (hh + S1 + ch + K[i] + w[i]) >>> 0;
      const S0 = (rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22)) >>> 0;
      const maj = ((a & b) ^ (a & c) ^ (b & c)) >>> 0;
      const t2 = (S0 + maj) >>> 0;

      hh = g;
      g = f;
      f = e;
      e = (d + t1) >>> 0;
      d = c;
      c = b;
      b = a;
      a = (t1 + t2) >>> 0;
    }

    h[0] = (h[0] + a) >>> 0;
    h[1] = (h[1] + b) >>> 0;
    h[2] = (h[2] + c) >>> 0;
    h[3] = (h[3] + d) >>> 0;
    h[4] = (h[4] + e) >>> 0;
    h[5] = (h[5] + f) >>> 0;
    h[6] = (h[6] + g) >>> 0;
    h[7] = (h[7] + hh) >>> 0;
  }

  const out = new Uint8Array(32);
  const outView = new DataView(out.buffer);
  for (let i = 0; i < 8; i++) outView.setUint32(i * 4, h[i], false);
  return out;
}

const HEX = '0123456789abcdef';

/** SHA-256 as lowercase hex, without any algorithm label. */
export function sha256Hex(input) {
  const bytes = sha256Bytes(input);
  let out = '';
  for (const byte of bytes) out += HEX[byte >> 4] + HEX[byte & 15];
  return out;
}

/**
 * A canonical serialisation: object keys sorted, no insignificant whitespace, arrays in order, and
 * non-finite numbers written as their JSON form. Two objects that differ only in key order or in
 * whitespace produce the same string, which is what makes a digest over it an identity rather than a
 * formatting accident.
 */
export function canonicalJson(value) {
  if (value === null) return 'null';
  const type = typeof value;
  if (type === 'number') return Number.isFinite(value) ? String(value) : 'null';
  if (type === 'boolean') return value ? 'true' : 'false';
  if (type === 'string') return JSON.stringify(value);
  if (type === 'undefined' || type === 'function') return 'null';
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  const keys = Object.keys(value)
    .filter((key) => value[key] !== undefined && typeof value[key] !== 'function')
    .sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(',')}}`;
}

/** A labelled digest over the canonical serialisation of a value. */
export function digestOf(value) {
  return `sha256:${sha256Hex(canonicalJson(value))}`;
}

/** A labelled digest over a string of bytes as given. */
export function digestText(text) {
  return `sha256:${sha256Hex(text)}`;
}

/**
 * The chain rule: an entry's chain value covers the previous chain value and the entry's own
 * digest, so changing any earlier entry changes every later chain value.
 */
export function chainValue(previousChain, entryDigest) {
  return `sha256:${sha256Hex(`${previousChain}\n${entryDigest}`)}`;
}

/** The genesis value a chain starts from, so the first entry is linked like every later one. */
export const GENESIS_CHAIN = digestText('strategy-discovery-console:genesis');

/** First 12 hex characters of a labelled digest, for display. */
export function shortDigest(digest, length = 12) {
  const [label, hex] = String(digest).split(':');
  const head = (hex ?? label).slice(0, length);
  return hex ? `${label}:${head}` : head;
}

/** True when a value looks like a labelled digest this module produced. */
export function isDigest(value) {
  return typeof value === 'string' && /^sha256:[0-9a-f]{64}$/.test(value);
}
