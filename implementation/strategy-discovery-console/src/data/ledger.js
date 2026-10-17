/**
 * Transcription table for the record and audit slices.
 *
 * It carries the ledger's format identities, the closed vocabulary of inventory events, the fields
 * each event kind must record, the fingerprint rules and the chain rule, the audit's candidate-set
 * and outcome rules, the assertions the audit refuses to make, and the two limits the tutorial
 * states. Every value is a quotation from `python/nautilus_trader/discovery/ledger.py`, the digest
 * recipe in `python/nautilus_trader/optimization/space.py`, or one of the two tutorial pages this
 * slice models; `tools/check-facts.mjs` asserts that each supporting text is still present in the
 * file it names.
 *
 * Nothing here computes and nothing here is derived: the engine modules fold these values, and a
 * value that is not written down with its source is not in this file.
 */

import { fact } from './source.js';

const LEDGER = 'python/nautilus_trader/discovery/ledger.py';
const BUNDLE = 'python/tests/acceptance/discovery_acceptance_bundle.json';
const SPACE = 'python/nautilus_trader/optimization/space.py';
const WHAT_RAN = 'tutorials/strategy-discovery/what-ran-versus-what-was-written/README.md';
const AUDITING = 'tutorials/strategy-discovery/auditing-a-campaign/README.md';

/**
 * The identity strings of the formats this slice reads and writes. A reader uses them to know how to
 * read an entry, a line of the inventory, the counts mapping, an envelope and the acceptance bundle.
 */
export const FORMATS = [
  fact('discovery.ledger/v2', LEDGER, 75, 'LEDGER_SCHEMA = "discovery.ledger/v2"'),
  fact('discovery.inventory/v1', LEDGER, 78, 'INVENTORY_SCHEMA = "discovery.inventory/v1"'),
  fact('discovery.counts/v1', LEDGER, 82, 'COUNTS_MAPPING = "discovery.counts/v1"'),
  fact('discovery.envelope/v1', LEDGER, 85, 'ENVELOPE_SCHEMA = "discovery.envelope/v1"'),
  fact(
    'discovery.acceptance-bundle/v2',
    LEDGER,
    88,
    'BUNDLE_SCHEMA = "discovery.acceptance-bundle/v2"',
  ),
  fact('entries', LEDGER, 90, '_ENTRIES_DIR = "entries"'),
  fact('inventory.jsonl', LEDGER, 91, '_EVENTS_FILE = "inventory.jsonl"'),
  fact('inventory.writer', LEDGER, 92, '_WRITER_FILE = "inventory.writer"'),
];

/**
 * The closed set of inventory events, in declaration order. An event outside this set cannot be
 * written, so a caller cannot invent an event that a later reader would have to interpret; the order
 * is the order the module declares, and the view offers the kinds in it.
 */
export const ENTRY_KIND_FACTS = [
  fact('proposal_recorded', LEDGER, 456, 'PROPOSAL_RECORDED = "proposal_recorded"'),
  fact('specification_refused', LEDGER, 457, 'SPECIFICATION_REFUSED = "specification_refused"'),
  fact('compilation_recorded', LEDGER, 458, 'COMPILATION_RECORDED = "compilation_recorded"'),
  fact('equivalence_merge', LEDGER, 459, 'EQUIVALENCE_MERGE = "equivalence_merge"'),
  fact('job_submitted', LEDGER, 460, 'JOB_SUBMITTED = "job_submitted"'),
  fact(
    'execution_attempt_closed',
    LEDGER,
    461,
    'EXECUTION_ATTEMPT_CLOSED = "execution_attempt_closed"',
  ),
  fact('registration_verified', LEDGER, 462, 'REGISTRATION_VERIFIED = "registration_verified"'),
  fact('trial_admitted', LEDGER, 463, 'TRIAL_ADMITTED = "trial_admitted"'),
  fact(
    'registration_incomplete',
    LEDGER,
    464,
    'REGISTRATION_INCOMPLETE = "registration_incomplete"',
  ),
  fact('duplicate_registration', LEDGER, 465, 'DUPLICATE_REGISTRATION = "duplicate_registration"'),
  fact('gate_verdict', LEDGER, 466, 'GATE_VERDICT = "gate_verdict"'),
  fact('custody_event', LEDGER, 467, 'CUSTODY_EVENT = "custody_event"'),
  fact('search_policy_recorded', LEDGER, 468, 'SEARCH_POLICY_RECORDED = "search_policy_recorded"'),
];

/**
 * What every event kind must record, keyed by the kind's value. The module refuses a payload that is
 * missing one of these, so a caller cannot write a bare record; it checks presence only, never type.
 */
export const REQUIRED_FIELDS = {
  proposal_recorded: fact([], LEDGER, 1400, 'EventKind.PROPOSAL_RECORDED: (),'),
  specification_refused: fact(['code'], LEDGER, 1401, 'EventKind.SPECIFICATION_REFUSED: ("code",),'),
  compilation_recorded: fact(
    ['specification_digest', 'new_configuration'],
    LEDGER,
    1402,
    'EventKind.COMPILATION_RECORDED: ("specification_digest", "new_configuration"),',
  ),
  equivalence_merge: fact(
    ['merged_into', 'rule'],
    LEDGER,
    1403,
    'EventKind.EQUIVALENCE_MERGE: ("merged_into", "rule"),',
  ),
  job_submitted: fact([], LEDGER, 1404, 'EventKind.JOB_SUBMITTED: (),'),
  execution_attempt_closed: fact(
    ['outcome'],
    LEDGER,
    1405,
    'EventKind.EXECUTION_ATTEMPT_CLOSED: ("outcome",)',
  ),
  registration_verified: fact(
    ['receipt', 'receipt_digest'],
    LEDGER,
    1406,
    'EventKind.REGISTRATION_VERIFIED: ("receipt", "receipt_digest"),',
  ),
  trial_admitted: fact(
    ['receipt_digest'],
    LEDGER,
    1407,
    'EventKind.TRIAL_ADMITTED: ("receipt_digest",)',
  ),
  registration_incomplete: fact(
    ['reason'],
    LEDGER,
    1408,
    'EventKind.REGISTRATION_INCOMPLETE: ("reason",)',
  ),
  duplicate_registration: fact(
    ['reason'],
    LEDGER,
    1409,
    'EventKind.DUPLICATE_REGISTRATION: ("reason",)',
  ),
  gate_verdict: fact(
    ['gate', 'status', 'evidence_digest'],
    LEDGER,
    1410,
    'EventKind.GATE_VERDICT: ("gate", "status", "evidence_digest"),',
  ),
  custody_event: fact(
    ['transition', 'detail'],
    LEDGER,
    1411,
    'EventKind.CUSTODY_EVENT: ("transition", "detail"),',
  ),
  search_policy_recorded: fact(
    ['search_policy_digest'],
    LEDGER,
    1412,
    'EventKind.SEARCH_POLICY_RECORDED: ("search_policy_digest",)',
  ),
};

/**
 * The fingerprint kinds, as the tutorial states them. Every one except the engine's is the same
 * recipe, so a change to any input changes the value that covers it and nothing else.
 */
export const IDENTITY_KINDS = [
  fact(
    'digest = "sha256:" + SHA-256( canonical_json( mapping ) )',
    WHAT_RAN,
    93,
    'digest = "sha256:" + SHA-256( canonical_json( mapping ) )',
  ),
  fact(
    'specification = digest of { schema, the twelve blocks }',
    WHAT_RAN,
    107,
    'specification = digest of { schema, the twelve blocks }',
  ),
  fact(
    'behavioural = digest of { schema, specification, policy, registry, features, assumptions, engine }',
    WHAT_RAN,
    108,
    'behavioural   = digest of { schema, specification, policy, registry, features, assumptions, engine }',
  ),
  fact(
    "experiment = digest of { the run's whole parameter set, including the behavioural value }",
    WHAT_RAN,
    109,
    "experiment    = digest of { the run's whole parameter set, including the behavioural value }",
  ),
  fact(
    'canonical_bytes_sha256 = digest of { "bytes": canonical_bytes.hex() }',
    WHAT_RAN,
    123,
    'canonical_bytes_sha256 = digest of { "bytes": canonical_bytes.hex() }',
  ),
  fact(
    'canonical_digest = "blake3:" + the engine\'s own hash of the document it holds',
    WHAT_RAN,
    124,
    'canonical_digest       = "blake3:" + the engine\'s own hash of the document it holds',
  ),
  fact(
    'record.digest = digest of { sequence, kind, key, candidate_id, payload, previous }',
    WHAT_RAN,
    135,
    'record.digest = digest of { sequence, kind, key, candidate_id, payload, previous }',
  ),
  fact(
    'previous = the digest of the record before it, or empty for the first record',
    WHAT_RAN,
    136,
    'previous      = the digest of the record before it, or empty for the first record',
  ),
];

/**
 * The serialisation a digest is taken over: keys sorted, punctuation compact, non-finite numbers
 * rejected, ASCII escapes. The compact form is the pre-image; the stored line or file text is not.
 */
export const SERIALISATION = {
  flags: fact(
    ['sort_keys=True', 'separators=(",", ":")', 'ensure_ascii=True', 'allow_nan=False'],
    SPACE,
    68,
    'sort_keys=True,\n        separators=(",", ":"),\n        ensure_ascii=True,\n        allow_nan=False,',
  ),
  sha256Prefix: fact(
    'sha256:',
    SPACE,
    94,
    'return f"sha256:{hashlib.sha256(encoded).hexdigest()}"',
  ),
  enginePrefix: fact(
    'blake3:',
    LEDGER,
    3374,
    '"the engine\'s canonical blake3 digest, which the extension computes from the "',
  ),
};

/**
 * The chain rule and the findings `verify_chain` reports. A finding is reported by name rather than
 * raised, so a reader sees every problem the chain has rather than the first one.
 */
export const CHAIN_RULES = {
  recordDigest: fact(
    'digest_of(canonical_mapping(self.to_dict()))',
    LEDGER,
    894,
    'return digest_of(canonical_mapping(self.to_dict()))',
  ),
  recordFields: fact(
    ['sequence', 'kind', 'key', 'candidate_id', 'payload', 'previous'],
    LEDGER,
    835,
    '"sequence": self.sequence,\n            "kind": self.kind.value,\n            "key": self.key,\n            "candidate_id": self.candidate_id,\n            "payload": dict(self.payload),\n            "previous": self.previous,',
  ),
  keyFile: fact(
    'digest.replace(":", "_")',
    LEDGER,
    444,
    'return digest.replace(":", "_")',
  ),
  appendOnly: fact(
    'A second append for the same key is refused, because an append-only ledger does not rewrite what it already learned.',
    WHAT_RAN,
    74,
    "which it treats as the candidate's key. A second append for the same key is refused, because an\n   append-only ledger does not rewrite what it already learned.",
  ),
  sequenceFinding: fact(
    'event {position} records sequence {event.sequence}: a record was removed or inserted',
    LEDGER,
    2306,
    'f"event {position} records sequence {event.sequence}: a record was removed "\n                    f"or inserted",',
  ),
  linkFinding: fact(
    'event {position} chains to {event.previous!r} instead of {previous!r}',
    LEDGER,
    2311,
    'f"event {position} chains to {event.previous!r} instead of {previous!r}",',
  ),
  checkpointFinding: fact(
    'the inventory ends at {head!r} and the checkpoint holds {checkpoint!r}',
    LEDGER,
    2318,
    'f"the inventory ends at {head!r} and the checkpoint holds {checkpoint!r}",',
  ),
  emptyHead: fact('', LEDGER, 2249, 'return "" if not events else events[-1].digest()'),
};

/** The messages the ledger raises when a caller asks for something the write path refuses. */
export const REFUSAL_MESSAGES = {
  REWRITE_REFUSED: fact(
    'an entry for candidate {package.candidate_id} already exists at {path}',
    LEDGER,
    383,
    'f"an entry for candidate {package.candidate_id} already exists at {path}",',
  ),
  MISSING_REQUIRED_FIELD: fact(
    'a {kind.value} event is missing {sorted(missing)}',
    LEDGER,
    2416,
    'raise ValueError(f"a {kind.value} event is missing {sorted(missing)}")',
  ),
  KIND_NOT_IN_VOCABULARY: fact(
    'kind must be an EventKind',
    LEDGER,
    823,
    'raise TypeError("kind must be an EventKind")',
  ),
};

/**
 * The audit's candidate set: the compiled lineage first, then the candidates a refusal, a verdict or
 * an attempt names, in first-seen order. The set is wider than the lineage on purpose, so a refused
 * candidate that never compiled still has a row.
 */
export const CANDIDATE_SET = {
  order: fact(
    ['lineage', 'refusals', 'verdicts', 'attempts'],
    AUDITING,
    78,
    'candidates = lineage keys, then refusal keys, then verdict keys, then attempt keys',
  ),
  firstSeen: fact(
    'The order is therefore first-seen order, and a refused candidate that never compiled appears last.',
    AUDITING,
    84,
    '- The order is therefore first-seen order, and a refused candidate that never compiled appears last.',
  ),
};

/**
 * The outcome rule, in the order it is applied, and the four words it can return. A verdict outranks
 * any number of attempts; one admitted attempt is enough for "admitted".
 */
export const OUTCOME_RULE = {
  verdictFirst: fact(
    "the last verdict's status",
    AUDITING,
    89,
    "outcome = the last verdict's status   if any gate judged the candidate",
  ),
  words: {
    admitted: fact(
      'admitted',
      AUDITING,
      90,
      '        = "admitted"                  otherwise, if it ran and was admitted',
    ),
    ran: fact('ran', AUDITING, 91, '        = "ran"                       otherwise, if it ran and was not admitted'),
    refused: fact(
      'refused',
      AUDITING,
      92,
      '        = "refused"                   otherwise, if it was refused',
    ),
    proposed: fact('proposed', AUDITING, 93, '        = "proposed"                  otherwise'),
  },
  codeLines: fact(
    ['return verdicts[-1].status', 'return "admitted" if admitted else "ran"', 'return "refused" if refusals else "proposed"'],
    LEDGER,
    2641,
    'return verdicts[-1].status\n        if attempts:\n            return "admitted" if admitted else "ran"\n\n        return "refused" if refusals else "proposed"',
  ),
};

/**
 * The policy comparison, which is a tri-state. Nothing declared is `null`, which is neither a pass
 * nor a failure; a report that printed it as `false` would be wrong.
 */
export const POLICY_COMPARISON = {
  nothingDeclared: fact('', LEDGER, 2778, 'if not self.policy_digest:\n            return None'),
  match: fact(
    'self.policy_digest == self.recorded_policy_digest',
    LEDGER,
    2781,
    'return self.policy_digest == self.recorded_policy_digest',
  ),
  tutorial: fact(
    'policy_matches = whether the declared digest equals the recorded one   when a digest was declared',
    AUDITING,
    107,
    'policy_matches = whether the declared digest equals the recorded one   when a digest was declared',
  ),
  nothingDeclaredTutorial: fact(
    '                = nothing at all                                        when none was declared',
    AUDITING,
    108,
    '               = nothing at all                                        when none was declared',
  ),
};

/** The reading is given one code over its own plain data, so two readings can be compared. */
export const AUDIT_DIGEST = {
  rule: fact(
    'a sha256 code over the audit\'s own plain data',
    AUDITING,
    120,
    "audit digest = a sha256 code over the audit's own plain data",
  ),
  recipe: fact(
    'digest_of(canonical_mapping(self.to_dict()))',
    LEDGER,
    2824,
    'return digest_of(canonical_mapping(self.to_dict()))',
  ),
};

/**
 * What the audit refuses to claim. Each entry is a sentence from the tutorial's "Where it goes
 * wrong", and each is a statement the reading deliberately does not make rather than a defect.
 */
export const REFUSED_ASSERTION_FACTS = [
  {
    id: 'names-only-what-was-retained',
    claim: fact(
      'The audit can only report what the ledger retained. It adds no record and can invent none, so an event that was never written down leaves no trace, and a refusal that never reached the ledger is a candidate the audit cannot name.',
      AUDITING,
      225,
      '- The audit can only report what the ledger retained. It adds no record and can invent none, so an\n  event that was never written down leaves no trace, and a refusal that never reached the ledger is a\n  candidate the audit cannot name.',
    ),
  },
  {
    id: 'does-not-verify-the-chain',
    claim: fact(
      'It does not verify the chain. The fixture verifies the chain separately, against a checkpoint held outside it; the audit simply reads the events, so a reader who wants a verified reading has to verify as well.',
      AUDITING,
      228,
      '- It does not verify the chain. The fixture verifies the chain separately, against a checkpoint held\n  outside it; the audit simply reads the events, so a reader who wants a verified reading has to verify\n  as well.',
    ),
  },
  {
    id: 'does-not-join-the-build-identities',
    claim: fact(
      "It does not compare the engine version with the receipts' build identities. It reports the identities as recorded and the version as declared, side by side, and leaves the comparison to the reader, because the join that would make the comparison meaningful is the open item named above.",
      AUDITING,
      231,
      "- It does not compare the engine version with the receipts' build identities. It reports the identities\n  as recorded and the version as declared, side by side, and leaves the comparison to the reader,\n  because the join that would make the comparison meaningful is the open item named above.",
    ),
  },
  {
    id: 'nothing-declared-is-not-a-failure',
    claim: fact(
      'A policy answer of "nothing declared" is honest and easy to misread. The comparison returns nothing at all when the reader declared no digest, which is neither a pass nor a failure; a report that printed it as false would be wrong.',
      AUDITING,
      234,
      '- A policy answer of "nothing declared" is honest and easy to misread. The comparison returns nothing\n  at all when the reader declared no digest, which is neither a pass nor a failure; a report that\n  printed it as false would be wrong.',
    ),
  },
  {
    id: 'deterministic-only-over-declared-inputs',
    claim: fact(
      'Determinism is only over the same inputs. A different declared policy digest produces a different reading and a different code even over an identical chain, which is deliberate, because the declared policy is part of the statement the reading makes.',
      AUDITING,
      237,
      '- Determinism is only over the same inputs. A different declared policy digest produces a different\n  reading and a different code even over an identical chain, which is deliberate, because the declared\n  policy is part of the statement the reading makes.',
    ),
  },
  {
    id: 'explains-nothing',
    claim: fact(
      'It summarises nothing away but it also explains nothing. The outcome word is a one-line answer derived from the fields; every fact behind it stays in the record, so a reader who wants to know why a candidate was refused must read the reasons, not the outcome word.',
      AUDITING,
      240,
      '- It summarises nothing away but it also explains nothing. The outcome word is a one-line answer derived\n  from the fields; every fact behind it stays in the record, so a reader who wants to know why a\n  candidate was refused must read the reasons, not the outcome word.',
    ),
  },
];

/**
 * The defects the acceptance bundle records against the two acceptance criteria this slice cannot
 * pass. They are the missing pieces in the bundle's own words, so the console names the failure with
 * the evidence rather than describing it.
 */
export const CRITERION_DEFECTS = {
  2: fact(
    'the canonical blake3 digest and the seed, build and configuration join cannot be recomputed in Python: the engine computes the digest from the document it holds, and the manifest of decision M1.3 has no binding',
    BUNDLE,
    45,
    '"defect": "the canonical blake3 digest and the seed, build and configuration join cannot be recomputed in Python: the engine computes the digest from the document it holds, and the manifest of decision M1.3 has no binding"',
  ),
  10: fact(
    "the engine's canonical digest cannot be recomputed from retained bytes, and the engine-side run manifest of decision M1.3 is unreachable from Python, so the envelope is not complete over its whole span",
    BUNDLE,
    230,
    '"defect": "the engine\'s canonical digest cannot be recomputed from retained bytes, and the engine-side run manifest of decision M1.3 is unreachable from Python, so the envelope is not complete over its whole span"',
  ),
};

/**
 * The two limits the tutorial states. `unavailable` is the name the recomputation gives the missing
 * input and `caveat` is the tutorial's own paragraph, so the console can show what is missing
 * without computing it: the engine's canonical digest cannot be recomputed from retained bytes, and
 * the manifest a receipt is checked against has no engine-side binding.
 */
export const LIMIT_FACTS = [
  {
    id: 'receipt-join',
    unavailable: fact(
      'the engine-side run manifest a receipt is checked against, which has no Python binding until decision M1.3 is implemented',
      LEDGER,
      3378,
      '"the engine-side run manifest a receipt is checked against, which has no Python "\n                "binding until decision M1.3 is implemented"',
    ),
    caveat: fact(
      "The receipt join is missing. The runner cannot yet emit a receipt that joins the seed, the build and the configuration to the engine's own run record, so until decision M1.3's narrow engine-side step exists, the manifest reconciliation compares against is the one the caller declares. The check is real and its tampered-receipt fixture is discriminating; what it cannot yet claim is that the manifest is the engine's own, and the missing piece is that engine-side run manifest binding.",
      WHAT_RAN,
      218,
      "- The receipt join is missing. The runner cannot yet emit a receipt that joins the seed, the build\n  and the configuration to the engine's own run record, so until decision M1.3's narrow engine-side\n  step exists, the manifest reconciliation compares against is the one the caller declares. The check\n  is real and its tampered-receipt fixture is discriminating; what it cannot yet claim is that the\n  manifest is the engine's own, and the missing piece is that engine-side run manifest binding.",
    ),
  },
  {
    id: 'engine-digest',
    unavailable: fact(
      "the engine's canonical blake3 digest, which the extension computes from the document it holds and no Python surface recomputes from retained bytes",
      LEDGER,
      3374,
      '"the engine\'s canonical blake3 digest, which the extension computes from the "\n                "document it holds and no Python surface recomputes from retained bytes"',
    ),
    caveat: fact(
      "The engine's own fingerprint cannot be recomputed. The extension computes its canonical digest from the document it holds and exposes no surface that recomputes it from retained bytes, so the ledger records the digest and the document's SHA-256 and the recomputation verifies only the latter. The missing piece is a narrow engine-side hashing step over the retained bytes; until it exists, criterion 10 cannot pass and a rerun cannot be shown identical from the record alone.",
      WHAT_RAN,
      223,
      "- The engine's own fingerprint cannot be recomputed. The extension computes its canonical digest\n  from the document it holds and exposes no surface that recomputes it from retained bytes, so the\n  ledger records the digest and the document's SHA-256 and the recomputation verifies only the\n  latter. The missing piece is a narrow engine-side hashing step over the retained bytes; until it\n  exists, criterion 10 cannot pass and a rerun cannot be shown identical from the record alone.",
    ),
  },
];
