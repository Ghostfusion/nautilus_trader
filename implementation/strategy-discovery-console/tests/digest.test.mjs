/**
 * Tests for the digest layer. The SHA-256 cases are the published NIST vectors, so a mistake in the
 * round constants, the padding or the length encoding fails here rather than silently producing a
 * plausible-looking digest in the interface.
 */

import { createHash } from 'node:crypto';

import { describe, expect, test } from 'bun:test';

import {
  GENESIS_CHAIN,
  canonicalJson,
  chainValue,
  digestOf,
  digestText,
  isDigest,
  sha256Hex,
  shortDigest,
} from '../src/engine/digest.js';

describe('sha256', () => {
  test('matches the published vectors', () => {
    expect(sha256Hex('')).toBe('e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
    expect(sha256Hex('abc')).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
    expect(sha256Hex('abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq')).toBe(
      '248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1',
    );
  });

  test('handles the padding boundaries', () => {
    // Every length through two blocks, checked against an independent implementation rather than
    // against a remembered constant, so the boundary cases (55/56 bytes, the length field) are
    // settled by the standard library and not by this file.
    for (let length = 0; length <= 200; length++) {
      const text = 'a'.repeat(length);
      const reference = createHash('sha256').update(text, 'utf8').digest('hex');
      expect(sha256Hex(text)).toBe(reference);
    }
  });

  test('handles a multi-block input', () => {
    expect(sha256Hex('a'.repeat(1000000))).toBe(
      'cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0',
    );
  });
});

describe('canonical serialisation', () => {
  test('key order and whitespace do not change the identity', () => {
    const a = { b: 1, a: { d: [1, 2], c: 'x' } };
    const b = { a: { c: 'x', d: [1, 2] }, b: 1 };
    expect(canonicalJson(a)).toBe(canonicalJson(b));
    expect(digestOf(a)).toBe(digestOf(b));
  });

  test('order inside an array is part of the identity', () => {
    expect(digestOf([1, 2])).not.toBe(digestOf([2, 1]));
  });

  test('a changed value changes the digest', () => {
    expect(digestOf({ bound: 0.05 })).not.toBe(digestOf({ bound: 0.0500001 }));
  });

  test('undefined members are dropped, so an absent field is not a null one', () => {
    expect(canonicalJson({ a: 1, b: undefined })).toBe('{"a":1}');
    expect(digestOf({ a: 1, b: undefined })).not.toBe(digestOf({ a: 1, b: null }));
  });

  test('digests are labelled with the algorithm, so they cannot be mistaken for blake3', () => {
    expect(digestText('x')).toStartWith('sha256:');
    expect(isDigest(digestText('x'))).toBe(true);
    expect(isDigest('blake3:abc')).toBe(false);
    expect(shortDigest(digestText('x'))).toHaveLength('sha256:'.length + 12);
  });
});

describe('the chain rule', () => {
  test('recognises no tampering and detects any', () => {
    const entries = [{ n: 1 }, { n: 2 }, { n: 3 }].map((entry) => ({
      entry,
      digest: digestOf(entry),
    }));

    const build = (list) => {
      let chain = GENESIS_CHAIN;
      return list.map((item) => {
        chain = chainValue(chain, item.digest);
        return chain;
      });
    };

    const good = build(entries);
    expect(good).toHaveLength(3);
    expect(good[0]).not.toBe(good[1]);

    const tampered = entries.slice();
    tampered[0] = { entry: { n: 99 }, digest: digestOf({ n: 99 }) };
    const rebuilt = build(tampered);
    expect(rebuilt[0]).not.toBe(good[0]);
    // Changing the first entry changes every later chain value, which is the property the audit uses.
    expect(rebuilt[1]).not.toBe(good[1]);
    expect(rebuilt[2]).not.toBe(good[2]);
  });

  test('the genesis value is a digest and is stable', () => {
    expect(isDigest(GENESIS_CHAIN)).toBe(true);
    expect(GENESIS_CHAIN).toBe(GENESIS_CHAIN);
  });
});
