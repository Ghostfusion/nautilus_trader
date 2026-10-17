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
  fact('discovery.ledger/v2', LEDGER, 76, 'LEDGER_SCHEMA = "discovery.ledger/v2"'),
  fact('discovery.inventory/v1', LEDGER, 79, 'INVENTORY_SCHEMA = "discovery.inventory/v1"'),
  fact('discovery.counts/v1', LEDGER, 83, 'COUNTS_MAPPING = "discovery.counts/v1"'),
  fact('discovery.envelope/v1', LEDGER, 86, 'ENVELOPE_SCHEMA = "discovery.envelope/v1"'),
  fact(
    'discovery.acceptance-bundle/v2',
    LEDGER,
    89,
    'BUNDLE_SCHEMA = "discovery.acceptance-bundle/v2"',
  ),
  fact('entries', LEDGER, 91, '_ENTRIES_DIR = "entries"'),
  fact('inventory.jsonl', LEDGER, 92, '_EVENTS_FILE = "inventory.jsonl"'),
  fact('inventory.writer', LEDGER, 93, '_WRITER_FILE = "inventory.writer"'),
];

/**
 * The closed set of inventory events, in declaration order. An event outside this set cannot be
 * written, so a caller cannot invent an event that a later reader would have to interpret; the order
 * is the order the module declares, and the view offers the kinds in it.
 */
export const ENTRY_KIND_FACTS = [
  fact('proposal_recorded', LEDGER, 457, 'PROPOSAL_RECORDED = "proposal_recorded"'),
  fact('specification_refused', LEDGER, 458, 'SPECIFICATION_REFUSED = "specification_refused"'),
  fact('compilation_recorded', LEDGER, 459, 'COMPILATION_RECORDED = "compilation_recorded"'),
  fact('equivalence_merge', LEDGER, 460, 'EQUIVALENCE_MERGE = "equivalence_merge"'),
  fact('job_submitted', LEDGER, 461, 'JOB_SUBMITTED = "job_submitted"'),
  fact(
    'execution_attempt_closed',
    LEDGER,
    462,
    'EXECUTION_ATTEMPT_CLOSED = "execution_attempt_closed"',
  ),
  fact('registration_verified', LEDGER, 463, 'REGISTRATION_VERIFIED = "registration_verified"'),
  fact('trial_admitted', LEDGER, 464, 'TRIAL_ADMITTED = "trial_admitted"'),
  fact(
    'registration_incomplete',
    LEDGER,
    465,
    'REGISTRATION_INCOMPLETE = "registration_incomplete"',
  ),
  fact('duplicate_registration', LEDGER, 466, 'DUPLICATE_REGISTRATION = "duplicate_registration"'),
  fact('gate_verdict', LEDGER, 467, 'GATE_VERDICT = "gate_verdict"'),
  fact('custody_event', LEDGER, 468, 'CUSTODY_EVENT = "custody_event"'),
  fact('search_policy_recorded', LEDGER, 469, 'SEARCH_POLICY_RECORDED = "search_policy_recorded"'),
];

/**
 * What every event kind must record, keyed by the kind's value. The module refuses a payload that is
 * missing one of these, so a caller cannot write a bare record; it checks presence only, never type.
 */
export const REQUIRED_FIELDS = {
  proposal_recorded: fact([], LEDGER, 1401, 'EventKind.PROPOSAL_RECORDED: (),'),
  specification_refused: fact(['code'], LEDGER, 1402, 'EventKind.SPECIFICATION_REFUSED: ("code",),'),
  compilation_recorded: fact(
    ['specification_digest', 'new_configuration'],
    LEDGER,
    1403,
    'EventKind.COMPILATION_RECORDED: ("specification_digest", "new_configuration"),',
  ),
  equivalence_merge: fact(
    ['merged_into', 'rule'],
    LEDGER,
    1404,
    'EventKind.EQUIVALENCE_MERGE: ("merged_into", "rule"),',
  ),
  job_submitted: fact([], LEDGER, 1405, 'EventKind.JOB_SUBMITTED: (),'),
  execution_attempt_closed: fact(
    ['outcome'],
    LEDGER,
    1406,
    'EventKind.EXECUTION_ATTEMPT_CLOSED: ("outcome",)',
  ),
  registration_verified: fact(
    ['receipt', 'receipt_digest', 'manifest'],
    LEDGER,
    1407,
    'EventKind.REGISTRATION_VERIFIED: ("receipt", "receipt_digest", "manifest"),',
  ),
  trial_admitted: fact(
    ['receipt_digest'],
    LEDGER,
    1408,
    'EventKind.TRIAL_ADMITTED: ("receipt_digest",)',
  ),
  registration_incomplete: fact(
    ['reason'],
    LEDGER,
    1409,
    'EventKind.REGISTRATION_INCOMPLETE: ("reason",)',
  ),
  duplicate_registration: fact(
    ['reason'],
    LEDGER,
    1410,
    'EventKind.DUPLICATE_REGISTRATION: ("reason",)',
  ),
  gate_verdict: fact(
    ['gate', 'status', 'evidence_digest'],
    LEDGER,
    1411,
    'EventKind.GATE_VERDICT: ("gate", "status", "evidence_digest"),',
  ),
  custody_event: fact(
    ['transition', 'detail'],
    LEDGER,
    1412,
    'EventKind.CUSTODY_EVENT: ("transition", "detail"),',
  ),
  search_policy_recorded: fact(
    ['search_policy_digest'],
    LEDGER,
    1413,
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
    BUNDLE,
    47,
    '"artifact_digest": "blake3:577e55e9b89511a9c7b4cdeb270d0172cdf5b4ab419d5a3ff6b741ac9d4fa8d8",',
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
    895,
    'return digest_of(canonical_mapping(self.to_dict()))',
  ),
  recordFields: fact(
    ['sequence', 'kind', 'key', 'candidate_id', 'payload', 'previous'],
    LEDGER,
    836,
    '"sequence": self.sequence,\n            "kind": self.kind.value,\n            "key": self.key,\n            "candidate_id": self.candidate_id,\n            "payload": dict(self.payload),\n            "previous": self.previous,',
  ),
  keyFile: fact(
    'digest.replace(":", "_")',
    LEDGER,
    445,
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
    2334,
    'f"event {position} records sequence {event.sequence}: a record was removed "\n                    f"or inserted",',
  ),
  linkFinding: fact(
    'event {position} chains to {event.previous!r} instead of {previous!r}',
    LEDGER,
    2339,
    'f"event {position} chains to {event.previous!r} instead of {previous!r}",',
  ),
  checkpointFinding: fact(
    'the inventory ends at {head!r} and the checkpoint holds {checkpoint!r}',
    LEDGER,
    2346,
    'f"the inventory ends at {head!r} and the checkpoint holds {checkpoint!r}",',
  ),
  emptyHead: fact('', LEDGER, 2277, 'return "" if not events else events[-1].digest()'),
};

/** The messages the ledger raises when a caller asks for something the write path refuses. */
export const REFUSAL_MESSAGES = {
  REWRITE_REFUSED: fact(
    'an entry for candidate {package.candidate_id} already exists at {path}',
    LEDGER,
    384,
    'f"an entry for candidate {package.candidate_id} already exists at {path}",',
  ),
  MISSING_REQUIRED_FIELD: fact(
    'a {kind.value} event is missing {sorted(missing)}',
    LEDGER,
    2444,
    'raise ValueError(f"a {kind.value} event is missing {sorted(missing)}")',
  ),
  KIND_NOT_IN_VOCABULARY: fact(
    'kind must be an EventKind',
    LEDGER,
    824,
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
    2669,
    'return verdicts[-1].status\n        if attempts:\n            return "admitted" if admitted else "ran"\n\n        return "refused" if refusals else "proposed"',
  ),
};

/**
 * The policy comparison, which is a tri-state. Nothing declared is `null`, which is neither a pass
 * nor a failure; a report that printed it as `false` would be wrong.
 */
export const POLICY_COMPARISON = {
  nothingDeclared: fact('', LEDGER, 2806, 'if not self.policy_digest:\n            return None'),
  match: fact(
    'self.policy_digest == self.recorded_policy_digest',
    LEDGER,
    2809,
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
    2852,
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
 * What the acceptance bundle records against the two acceptance criteria this slice carries. Both now
 * pass, and each entry's `defect` reads as what closed the failure rather than as what is absent, so
 * the console names the change in the bundle's own words rather than describing it.
 */
export const CRITERION_DEFECTS = {
  2: fact(
    'closed by this change. The recomputation previously named three inputs unavailable: the ' +
    'engine\'s canonical blake3 digest, the seed, build and configuration join of decision M1.3, ' +
    'and the retained artifact, which no store was supplied to resolve. The digest is now ' +
    'recomputed from the retained bytes with CanonicalBacktestResult.digest_of_bytes, the ' +
    'manifest is produced by the engine itself through BacktestNode.get_run_manifest and retained ' +
    'beside the receipt at registration, and the fixture supplies a store, so nothing is ' +
    'unavailable and every quantity agrees',
    BUNDLE,
    56,
    '"defect": "closed by this change. The recomputation previously named three inputs ' +
    'unavailable: the engine\'s canonical blake3 digest, the seed, build and configuration join of ' +
    'decision M1.3, and the retained artifact, which no store was supplied to resolve. The digest ' +
    'is now recomputed from the retained bytes with CanonicalBacktestResult.digest_of_bytes, the ' +
    'manifest is produced by the engine itself through BacktestNode.get_run_manifest and retained ' +
    'beside the receipt at registration, and the fixture supplies a store, so nothing is ' +
    'unavailable and every quantity agrees"',
  ),
  10: fact(
    'closed by this change. Previously the engine\'s canonical digest could not be recomputed from ' +
    'retained bytes and the engine-side run manifest of decision M1.3 was unreachable from ' +
    'Python, so the envelope was not complete over its whole span and the recomputation named ' +
    'both; a third input, the retained artifact, was named whenever no store was supplied. The ' +
    'digest now recomputes through CanonicalBacktestResult.digest_of_bytes, the engine produces ' +
    'the manifest through BacktestNode.get_run_manifest and the registration retains it, and the ' +
    'fixture supplies the store, so the recomputation agrees with nothing unavailable',
    BUNDLE,
    251,
    '"defect": "closed by this change. Previously the engine\'s canonical digest could not be ' +
    'recomputed from retained bytes and the engine-side run manifest of decision M1.3 was ' +
    'unreachable from Python, so the envelope was not complete over its whole span and the ' +
    'recomputation named both; a third input, the retained artifact, was named whenever no store ' +
    'was supplied. The digest now recomputes through CanonicalBacktestResult.digest_of_bytes, the ' +
    'engine produces the manifest through BacktestNode.get_run_manifest and the registration ' +
    'retains it, and the fixture supplies the store, so the recomputation agrees with nothing ' +
    'unavailable"',
  ),
};

/**
 * The two limits the tutorial once stated, and what closed each. `resolved` is the module's own
 * sentence and `page` is the tutorial's own paragraph, which now records that the limit is closed
 * rather than standing in for it, so the console shows both because the page and the code agree.
 */
export const LIMIT_FACTS = [
  {
    id: 'receipt-join',
    resolved: fact(
      'The manifest is retained rather than re-read, because the check a recomputation makes is ' +
      'against what the run recorded, and the run is not repeated to make it.',
      LEDGER,
      1711,
      'The manifest is retained rather than re-read, because the check a recomputation makes is\n' +
      '        against what the run recorded, and the run is not repeated to make it.',
    ),
    page: fact(
      'The receipt join was missing, and now exists. The runner emits a receipt that joins the ' +
      'seed, the build and the configuration to the engine\'s own run record: the backtest node is ' +
      'asked for its manifest, BacktestNode.get_run_manifest, and the registration retains it, so ' +
      'the reconciliation compares against the engine\'s own declaration rather than the one the ' +
      'caller declares. The check and its tampered-receipt fixture are unchanged; what the page ' +
      'records is that the binding the fixture leaned on is now the engine\'s own.',
      WHAT_RAN,
      218,
      '- The receipt join was missing, and now exists. The runner emits a receipt that joins the ' +
      'seed, the\n  build and the configuration to the engine\'s own run record: the backtest node is ' +
      'asked for its\n  manifest, `BacktestNode.get_run_manifest`, and the registration retains it, ' +
      'so the reconciliation\n  compares against the engine\'s own declaration rather than the one ' +
      'the caller declares. The check\n  and its tampered-receipt fixture are unchanged; what the ' +
      'page records is that the binding the\n  fixture leaned on is now the engine\'s own.',
    ),
  },
  {
    id: 'engine-digest',
    resolved: fact(
      'The engine\'s canonical digest is recomputed from the retained bytes with the extension\'s own ' +
      'hashing step, so a document that was retained is checked against the digest it was recorded ' +
      'under rather than against its SHA-256 alone.',
      LEDGER,
      3303,
      'The engine\'s canonical digest is recomputed from the retained bytes with the extension\'s own\n' +
      '    hashing step, so a document that was retained is checked against the digest it was recorded\n' +
      '    under rather than against its SHA-256 alone.',
    ),
    page: fact(
      'The engine\'s own fingerprint could not be recomputed, and now can. The extension exposes ' +
      'CanonicalBacktestResult.digest_of_bytes, which recomputes the canonical digest from the ' +
      'bytes a store retains, so the ledger records the digest and the document\'s SHA-256 and the ' +
      'recomputation verifies both. A rerun is shown identical from the record alone, which is what ' +
      'criterion 10 asks for.',
      WHAT_RAN,
      224,
      '- The engine\'s own fingerprint could not be recomputed, and now can. The extension exposes\n  ' +
      '`CanonicalBacktestResult.digest_of_bytes`, which recomputes the canonical digest from the ' +
      'bytes a\n  store retains, so the ledger records the digest and the document\'s SHA-256 and the ' +
      'recomputation\n  verifies both. A rerun is shown identical from the record alone, which is ' +
      'what criterion 10 asks\n  for.',
    ),
  },
];
