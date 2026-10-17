/**
 * The record: one append-only chain of ledger events.
 *
 * Models `python/nautilus_trader/discovery/ledger.py`. `LedgerEvent` supplies the six fields an
 * entry carries and the digest it computes from them; `CampaignInventory._append` supplies the next
 * sequence number, the digest of the record before it, and the fields each kind must carry;
 * `CampaignInventory.verify_chain` supplies the sequence check, the link check and the checkpoint
 * check, whose findings are reported by name rather than raised. `DiscoveryLedger.append` supplies
 * the append-only rule: the ledger files one entry per candidate identity and refuses a second one
 * rather than writing over it.
 *
 * THE CONSOLE ADDS ONE LINK. Every entry also carries `chain`, the value from ./digest.js that covers
 * the previous chain value and this entry's digest, and `verifyChain` recomputes it. The module's own
 * check compares each record's quoted digest with the digest recomputed from the record before it, so
 * a damaged final record is caught by the checkpoint alone; the console's chain value fails at the
 * damaged record itself and at every record after it. Both are reported: `findings` carries the
 * module's messages and `brokenAt` carries the first position whose chain value does not recompute.
 *
 * One mapping is the console's rather than the module's, and it is named here. The chain has no event
 * that files an evidence package, so the one-entry-per-candidate rule is applied to
 * `compilation_recorded`, the event that names a candidate's compilation.
 *
 * Nothing here touches the DOM, the network, a clock or a source of randomness, and every value it
 * returns is plain data.
 */

import { GENESIS_CHAIN, chainValue, digestOf } from './digest.js';
import { valueOf } from '../data/source.js';
import {
  CHAIN_RULES,
  ENTRY_KIND_FACTS,
  LIMIT_FACTS,
  REFUSAL_MESSAGES,
  REQUIRED_FIELDS,
} from '../data/ledger.js';

/** The closed vocabulary, in the order the module declares it. */
const KINDS = Object.freeze(ENTRY_KIND_FACTS.map(valueOf));

/** What each kind must record. The module checks presence only, never type, and so does this. */
const REQUIRED = Object.freeze(
  Object.fromEntries(Object.entries(REQUIRED_FIELDS).map(([kind, names]) => [kind, valueOf(names)])),
);

/** The kinds the ledger files once per candidate identity. */
const ONE_ENTRY_PER_CANDIDATE = Object.freeze(['compilation_recorded']);

/** The module's own refusal texts, transcribed in ../data/ledger.js. */
const MESSAGES = Object.freeze({
  REWRITE_REFUSED: valueOf(REFUSAL_MESSAGES.REWRITE_REFUSED),
  MISSING_REQUIRED_FIELD: valueOf(REFUSAL_MESSAGES.MISSING_REQUIRED_FIELD),
  KIND_NOT_IN_VOCABULARY: valueOf(REFUSAL_MESSAGES.KIND_NOT_IN_VOCABULARY),
});

const SEQUENCE_FINDING = valueOf(CHAIN_RULES.sequenceFinding);
const LINK_FINDING = valueOf(CHAIN_RULES.linkFinding);
const CHECKPOINT_FINDING = valueOf(CHAIN_RULES.checkpointFinding);

/** Python's repr of a string, for the placeholders the module writes with `!r`. */
function pyRepr(value) {
  if (typeof value === 'string') return `'${value}'`;
  if (Array.isArray(value)) return `[${value.map(pyRepr).join(', ')}]`;
  if (value === null || value === undefined) return 'None';
  return String(value);
}

/**
 * Render one of the module's f-string messages with the values it names, so a refusal reads exactly
 * as the module's would. `{name!r}` and `{sorted(...)}` are rendered as Python's repr; a name the
 * caller did not supply is left as written rather than silently blanked.
 */
function renderMessage(template, values) {
  return String(template).replace(/\{([^{}]+)\}/g, (whole, key) => {
    const repr = key.endsWith('!r');
    const name = repr ? key.slice(0, -2) : key;
    if (!(name in values)) return whole;
    return repr || name.startsWith('sorted(') ? pyRepr(values[name]) : String(values[name]);
  });
}

/** The refusal value the write path returns instead of raising. */
function refuse(code, reason) {
  return { ok: false, code, reason };
}

/**
 * The closed event vocabulary, in declaration order. A view offers exactly these kinds, so an event
 * outside the set cannot be written and a later reader never has to interpret one.
 */
export function ENTRY_KINDS() {
  return [...KINDS];
}

/** The fields one kind must record, so a view can ask for exactly what the module requires. */
export function requiredFields(kind) {
  if (!KINDS.includes(kind)) {
    throw new Error(`the entry kind ${JSON.stringify(kind)} is not in the vocabulary`);
  }
  return [...REQUIRED[kind]];
}

/**
 * Build one entry, linked to the entry before it.
 *
 * `payload` carries the fields the kind requires, plus two the chain lifts onto the entry itself:
 * `candidateId`, the candidate the event is about, and `key`, the idempotency key of the job it is
 * about. Both default to the empty string, as the module's `LedgerEvent` does. The digest covers
 * `{ sequence, kind, key, candidateId, payload, previous }` - the module spells `candidateId`
 * `candidate_id` - and `chain` covers every entry before this one.
 */
export function entry(kind, payload, previous) {
  if (!KINDS.includes(kind)) {
    throw new Error(
      `the entry kind ${JSON.stringify(kind)} is not in the vocabulary: an event outside the closed set cannot be written`,
    );
  }
  if (payload === null || typeof payload !== 'object' || Array.isArray(payload)) {
    throw new Error('an entry payload must be a mapping of the fields its kind carries');
  }

  const { key = '', candidateId = '', ...fields } = payload;
  const sequence = previous ? previous.sequence + 1 : 0;
  const previousDigest = previous ? previous.digest : '';
  const digest = digestOf({
    sequence,
    kind,
    key,
    candidateId,
    payload: fields,
    previous: previousDigest,
  });
  const chain = chainValue(previous ? previous.chain : GENESIS_CHAIN, digest);

  return Object.freeze({
    sequence,
    kind,
    key: String(key),
    candidateId: String(candidateId),
    payload: Object.freeze(fields),
    previous: previousDigest,
    digest,
    chain,
  });
}

/**
 * Append one event to the chain, or refuse it.
 *
 * The append is append-only in two senses and both are enforced: the entries already in the chain are
 * returned untouched and frozen rather than rewritten, and a kind the ledger files once per candidate
 * identity refuses a second entry for the same candidate. The refusal is a value, so the interface
 * shows the rule instead of catching an exception.
 */
export function append(entries, kind, payload) {
  if (!Array.isArray(entries)) {
    throw new Error('an append needs the chain it appends to');
  }
  if (!KINDS.includes(kind)) {
    return refuse('KIND_NOT_IN_VOCABULARY', MESSAGES.KIND_NOT_IN_VOCABULARY);
  }

  const fields = payload ?? {};
  const missing = REQUIRED[kind].filter((name) => !(name in fields));
  if (missing.length > 0) {
    return refuse(
      'MISSING_REQUIRED_FIELD',
      renderMessage(MESSAGES.MISSING_REQUIRED_FIELD, {
        'kind.value': kind,
        'sorted(missing)': missing,
      }),
    );
  }

  const candidateId = String(fields.candidateId ?? '');
  const held = entries.some((item) => item.kind === kind && item.candidateId === candidateId);
  if (ONE_ENTRY_PER_CANDIDATE.includes(kind) && held) {
    return refuse(
      'REWRITE_REFUSED',
      renderMessage(MESSAGES.REWRITE_REFUSED, {
        'package.candidate_id': candidateId,
        path: `entries/${candidateId.replace(':', '_')}.json`,
      }),
    );
  }

  const appended = entry(kind, fields, entries.length > 0 ? entries[entries.length - 1] : null);
  return { ok: true, entry: appended, entries: [...entries, appended] };
}

/**
 * Verify the chain: every sequence number against its position, every link against the digest
 * recomputed from the record before it, every chain value against the one recomputed from the whole
 * chain so far, and the chain's end against a checkpoint held outside the writer.
 *
 * A finding is a string in the module's own words; `brokenAt` is the first position whose chain value
 * does not recompute, which is where a tamper shows and from which every later link differs. Nothing
 * is raised: a reader sees every problem the chain has rather than the first one.
 */
export function verifyChain(entries, checkpoint) {
  if (!Array.isArray(entries)) {
    throw new Error('verifyChain needs the chain it verifies');
  }

  const findings = [];
  const links = [];
  let previousDigest = '';
  let previousChain = GENESIS_CHAIN;
  let brokenAt = null;

  entries.forEach((item, position) => {
    const digest = digestOf({
      sequence: item.sequence,
      kind: item.kind,
      key: item.key,
      candidateId: item.candidateId,
      payload: item.payload,
      previous: item.previous,
    });
    const chain = chainValue(previousChain, digest);
    const ok = item.digest === digest && item.chain === chain;
    if (!ok && brokenAt === null) brokenAt = position;

    if (item.sequence !== position) {
      findings.push(
        renderMessage(SEQUENCE_FINDING, { position, 'event.sequence': item.sequence }),
      );
    }
    if (item.previous !== previousDigest) {
      findings.push(
        renderMessage(LINK_FINDING, {
          position,
          'event.previous': item.previous,
          previous: previousDigest,
        }),
      );
    }

    links.push({ position, kind: item.kind, ok, digest, chain });
    previousDigest = digest;
    previousChain = chain;
  });

  const head = previousChain;
  if (checkpoint !== undefined && checkpoint !== null && checkpoint !== head) {
    findings.push(renderMessage(CHECKPOINT_FINDING, { head, checkpoint }));
  }

  return {
    ok: findings.length === 0 && brokenAt === null,
    events: entries.length,
    head,
    findings,
    brokenAt,
    links,
  };
}

/**
 * The write path's refusals. `message` is the module's own text and `condition` says what produces it,
 * so the interface can show the rule rather than describe it.
 */
export const REFUSAL_CODES = Object.freeze({
  REWRITE_REFUSED: Object.freeze({
    code: 'REWRITE_REFUSED',
    message: MESSAGES.REWRITE_REFUSED,
    condition: 'a second append for a candidate the ledger already holds an entry for',
  }),
  MISSING_REQUIRED_FIELD: Object.freeze({
    code: 'MISSING_REQUIRED_FIELD',
    message: MESSAGES.MISSING_REQUIRED_FIELD,
    condition: 'a payload that does not carry a field its kind requires',
  }),
  KIND_NOT_IN_VOCABULARY: Object.freeze({
    code: 'KIND_NOT_IN_VOCABULARY',
    message: MESSAGES.KIND_NOT_IN_VOCABULARY,
    condition: 'an event kind outside the closed vocabulary',
  }),
});

/**
 * The two limits this layer carried, as documented values rather than computed ones. `resolved` is the
 * module's own sentence and `page` is the tutorial's own paragraph, because the console cannot compute
 * either: the engine's digest is recomputed from retained bytes, and the manifest a receipt is checked
 * against is the one the registration retained - and the page now says so too.
 */
export const LIMITS = Object.freeze(
  LIMIT_FACTS.map((limit) =>
    Object.freeze({
      id: limit.id,
      resolved: valueOf(limit.resolved),
      page: valueOf(limit.page),
    }),
  ),
);
