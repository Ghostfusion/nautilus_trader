/**
 * The acceptance record, transcribed.
 *
 * The twenty-two criteria rows themselves live in `./layers.js`, because they are the shared record
 * every layer reads; this module holds what the console must not restate by hand: the count the bundle
 * reports, the rule that makes an entry an entry, and the four failures with the bundle's own
 * `observed` and `defect` text beside the missing piece each failure is carried by.
 *
 * The console never presents a failure as a pass and never fills a missing value with a default, so
 * the four failures are here as data: criterion 2 and criterion 10 wait on the same absent join,
 * criterion 4 on the settlement bridge, and criterion 6 on the absent dividend cash path.
 */

import { fact } from './source.js';

/** The artifact of record for every criterion in this view. */
export const BUNDLE = 'python/tests/acceptance/discovery_acceptance_bundle.json';

const DESIGN = 'docs/design/strategy_discovery_design.md';
const README = 'tutorials/strategy-discovery/the-boundary/README.md';

/** The three statuses a criterion may carry. `blocked` is a legitimate answer, not a failure. */
export const STATUSES = fact(
  ['passed', 'failed', 'blocked'],
  DESIGN,
  2067,
  '| Status                | Pass, fail, or blocked with the dependency that blocks it, and for a failure the open defect    |',
);

/** The tally the bundle reports, in the bundle's own sentence and as the four numbers. */
export const COUNTS = {
  sentence: fact(
    'The bundle as a whole holds twenty-two entries: eighteen pass with identities whose digests ' +
      'recompute, four are failed with their defects named, and none is blocked.',
    README,
    182,
    'The bundle as a whole holds twenty-two entries: eighteen pass with identities whose digests recompute,\n' +
      'four are failed with their defects named, and none is blocked. The four that fail are not about this',
  ),
  total: fact(
    22,
    README,
    182,
    'The bundle as a whole holds twenty-two entries: eighteen pass with identities whose digests recompute,',
  ),
  passed: fact(
    18,
    README,
    182,
    'The bundle as a whole holds twenty-two entries: eighteen pass with identities whose digests recompute,',
  ),
  failed: fact(4, README, 183, 'four are failed with their defects named, and none is blocked.'),
  blocked: fact(0, README, 183, 'four are failed with their defects named, and none is blocked.'),
  bundleObserved: fact(
    'twenty-two entries validate; eighteen pass with identities whose digests recompute from their ' +
      "components; four are failed with their defects named, being criteria 2 and 10 against decision " +
      "M1.3's unreachable run identity, criterion 4 against the settlement bridge, and criterion 6 " +
      'against the absent dividend cash path; none is blocked, because no criterion is now without the ' +
      'evidence to evaluate it',
    BUNDLE,
    529,
    '"observed": "twenty-two entries validate; eighteen pass with identities whose digests recompute from their components; four are failed with their defects named, being criteria 2 and 10 against decision M1.3\'s unreachable run identity, criterion 4 against the settlement bridge, and criterion 6 against the absent dividend cash path; none is blocked, because no criterion is now without the evidence to evaluate it.',
  ),
};

/** The two rules that make this view a record rather than a claim. */
export const RULES = {
  reporting: fact(
    "A criterion is not satisfied by the existence of a design section, and this document's own " +
      'passing checks are evidence about documentation rather than about behaviour',
    DESIGN,
    1904,
    "One rule about reporting governs all of them. A criterion is not satisfied by the existence of a\n" +
      "design section, and this document's own passing checks are evidence about documentation rather than\n" +
      'about behaviour: linting, the documentation conventions hook, table normalisation, the character\n' +
      'checks, offline link resolution and cross-reference numbering establish hygiene, not engine timing,\n' +
      'cash accounting, statistical validity or runtime determinism. Each criterion therefore needs an entry\n' +
      'in the acceptance-evidence bundle of Section 15.1, produced by running its test, and a criterion whose\n' +
      'fixture does not exist yet is reported as blocked rather than passed.',
  ),
  entry: fact(
    'An entry is produced by running the test, never by reading this document, and a self-assessment is ' +
      'not an entry.',
    DESIGN,
    2069,
    'Three rules govern the bundle. An entry is produced by running the test, never by reading this\n' +
      "document, and a self-assessment is not an entry. A criterion whose fixture does not yet exist is\n" +
      'blocked, which is a legitimate status and the honest one.',
  ),
};

/**
 * The four failing criteria, each with the bundle's own `observed` and `defect` text and the missing
 * piece the page names for it. `missing` is the page's cell, which is the failure's cause rather than
 * its symptom, so the view can explain a failure without inventing a reason for it.
 */
export const FAILURES = [
  {
    id: fact(2, BUNDLE, 19, '"criterion": 2,'),
    requirement: fact(
      'A rerun is identical, and what is not deterministic is named.',
      BUNDLE,
      20,
      '"requirement": "A rerun is identical, and what is not deterministic is named.",',
    ),
    milestone: fact('M4', BUNDLE, 23, '"milestone": "M4",'),
    status: fact('failed', BUNDLE, 24, '"status": "failed",'),
    observed: fact(
      "The recomputation agrees across the two stores: the four counts, each entry's identity and the " +
        'declared non-deterministic components recompute from the ledger and the inventory alone, and the ' +
        "inventory's chain verifies against a checkpoint held outside it. What cannot be recomputed is " +
        "named rather than passed over: the engine's canonical blake3 digest, the seed, build and " +
        'configuration join of decision M1.3, and the retained artifact, which needs the store that ' +
        'holds it',
      BUNDLE,
      37,
      '"observed": "The recomputation agrees across the two stores: the four counts, each entry\'s identity and the declared non-deterministic components recompute from the ledger and the inventory alone, and the inventory\'s chain verifies against a checkpoint held outside it. What cannot be recomputed is named rather than passed over: the engine\'s canonical blake3 digest, the seed, build and configuration join of decision M1.3, and the retained artifact, which needs the store that holds it",',
    ),
    defect: fact(
      'the canonical blake3 digest and the seed, build and configuration join cannot be recomputed in ' +
        'Python: the engine computes the digest from the document it holds, and the manifest of decision ' +
        'M1.3 has no binding',
      BUNDLE,
      45,
      '"defect": "the canonical blake3 digest and the seed, build and configuration join cannot be recomputed in Python: the engine computes the digest from the document it holds, and the manifest of decision M1.3 has no binding"',
    ),
    missing: fact(
      'the canonical blake3 digest and the seed, build and configuration join cannot be recomputed in ' +
        'Python, because the engine computes the digest from the document it holds and the manifest of ' +
        'decision M1.3 has no binding',
      README,
      189,
      '| 2         | A rerun is identical, and what is not deterministic is named                                    | the canonical blake3 digest and the seed, build and configuration join cannot be recomputed in Python, because the engine computes the digest from the document it holds and the manifest of decision M1.3 has no binding |',
    ),
  },
  {
    id: fact(4, BUNDLE, 62, '"criterion": 4,'),
    requirement: fact(
      'A signal cannot fill before its declared eligibility time, and the test that proves it can fail.',
      BUNDLE,
      63,
      '"requirement": "A signal cannot fill before its declared eligibility time, and the test that proves it can fail.",',
    ),
    milestone: fact('M2', BUNDLE, 66, '"milestone": "M2",'),
    status: fact('failed', BUNDLE, 67, '"status": "failed",'),
    observed: fact(
      'The pre-enforcement half fails as expected: with no deferral the fill carries the decision bar\'s ' +
        "own timestamp and that bar's own close, and the account is debited at that instant. The enforced " +
        'half holds for the latency model, whose fill is never earlier than the arrival and lands on the ' +
        'first bar at or after it. Two cases disagree with the table and are named here. The bridge ' +
        "mechanism resolves an artifact's eligibility to the next session's open and stamps the signal " +
        'with it, but its submission path holds no clock, so the order set is submitted at delivery and ' +
        'the fill precedes the declared eligibility by the whole interval to the next session. And a ' +
        'deferred intent with no subsequent market data is filled at shutdown against the last known ' +
        'close, without notifying the strategy, where the table requires that no fill be fabricated. With ' +
        'bar-only data the deferral prices at a bar close rather than at an opening price, so the ' +
        'declared eligibility can be honoured in time but not in price unless the snapshot carries market ' +
        'data at that instant',
      BUNDLE,
      72,
      '"observed": "The pre-enforcement half fails as expected: with no deferral the fill carries the decision bar\'s own timestamp and that bar\'s own close, and the account is debited at that instant. The enforced half holds for the latency model, whose fill is never earlier than the arrival and lands on the first bar at or after it. Two cases disagree with the table and are named here. The bridge mechanism resolves an artifact\'s eligibility to the next session\'s open and stamps the signal with it, but its submission path holds no clock, so the order set is submitted at delivery and the fill precedes the declared eligibility by the whole interval to the next session. And a deferred intent with no subsequent market data is filled at shutdown against the last known close, without notifying the strategy, where the table requires that no fill be fabricated. With bar-only data the deferral prices at a bar close rather than at an opening price, so the declared eligibility can be honoured in time but not in price unless the snapshot carries market data at that instant",',
    ),
    defect: fact(
      'the decision-bridge projection resolves eligibility and defers nothing, and the shutdown drain ' +
        'settles a deferred intent against the last known close without notifying the strategy; the ' +
        'latency model enforces the contract, so the accepted mechanism is named and the fixture\'s ' +
        'pre-enforcement half is retained as evidence that it discriminates',
      BUNDLE,
      74,
      '"defect": "the decision-bridge projection resolves eligibility and defers nothing, and the shutdown drain settles a deferred intent against the last known close without notifying the strategy; the latency model enforces the contract, so the accepted mechanism is named and the fixture\'s pre-enforcement half is retained as evidence that it discriminates"',
    ),
    missing: fact(
      'the decision-bridge projection resolves eligibility and defers nothing, and the shutdown drain ' +
        'settles a deferred intent against the last known close without notifying the strategy',
      README,
      190,
      '| 4         | A signal cannot fill before its declared eligibility time, and the test that proves it can fail | the decision-bridge projection resolves eligibility and defers nothing, and the shutdown drain settles a deferred intent against the last known close without notifying the strategy                                      |',
    ),
  },
  {
    id: fact(6, BUNDLE, 105, '"criterion": 6,'),
    requirement: fact(
      'Corporate-action accounting reconciles in value and in cash.',
      BUNDLE,
      106,
      '"requirement": "Corporate-action accounting reconciles in value and in cash.",',
    ),
    milestone: fact('M2', BUNDLE, 109, '"milestone": "M2",'),
    status: fact('failed', BUNDLE, 110, '"status": "failed",'),
    observed: fact(
      'The value leg reconciles and the cash leg does not. The adjustment series carries both actions: a ' +
        "split factor of one quarter and 2.50 per share, adjusting the raw 400.00 to 97.50. The held " +
        "position's whole cash movement is its 40,000 purchase cost, so the 250 it was entitled to at the " +
        'ex-date is credited to nothing. The split leaves the quantity at 100 rather than 400, and a round ' +
        'trip opened at 400.00 and closed at 200.00 realises 20,000 where a total-return account of the ' +
        'same position would carry 19,750. Both action records are delivered to the module, and the same ' +
        'run without them produces identical numbers, so the omission is in the accounting rather than in ' +
        'the data. The defect is the absent upstream dividend cash-flow path, which is the ' +
        'equity-readiness prerequisite of decision SD12',
      BUNDLE,
      115,
      '"observed": "The value leg reconciles and the cash leg does not. The adjustment series carries both actions: a split factor of one quarter and 2.50 per share, adjusting the raw 400.00 to 97.50. The held position\'s whole cash movement is its 40,000 purchase cost, so the 250 it was entitled to at the ex-date is credited to nothing. The split leaves the quantity at 100 rather than 400, and a round trip opened at 400.00 and closed at 200.00 realises 20,000 where a total-return account of the same position would carry 19,750. Both action records are delivered to the module, and the same run without them produces identical numbers, so the omission is in the accounting rather than in the data. The defect is the absent upstream dividend cash-flow path, which is the equity-readiness prerequisite of decision SD12",',
    ),
    defect: fact(
      'the corporate-action stage credits no entitlement: a held position moves only by its purchase cost ' +
        'and the distribution it was entitled to is credited to nothing, so the cash leg cannot reconcile ' +
        'and the equity-readiness statement stays withheld',
      BUNDLE,
      117,
      '"defect": "the corporate-action stage credits no entitlement: a held position moves only by its purchase cost and the distribution it was entitled to is credited to nothing, so the cash leg cannot reconcile and the equity-readiness statement stays withheld"',
    ),
    missing: fact(
      'the corporate-action stage credits no entitlement: a held position moves only by its purchase cost ' +
        'and the distribution it was entitled to is credited to nothing, so the cash leg cannot reconcile',
      README,
      191,
      '| 6         | Corporate-action accounting reconciles in value and in cash                                     | the corporate-action stage credits no entitlement: a held position moves only by its purchase cost and the distribution it was entitled to is credited to nothing, so the cash leg cannot reconcile                       |',
    ),
  },
  {
    id: fact(10, BUNDLE, 204, '"criterion": 10,'),
    requirement: fact('Every number recomputes.', BUNDLE, 205, '"requirement": "Every number recomputes.",'),
    milestone: fact('M4', BUNDLE, 208, '"milestone": "M4",'),
    status: fact('failed', BUNDLE, 209, '"status": "failed",'),
    observed: fact(
      'The envelope declares every required kind and resolves each reference from the content retained ' +
        'beside its digest; a reference whose content was not retained leaves the package unresolved and ' +
        'the recomputation incomplete rather than passing; a receipt resolves against the artifact the ' +
        'ledger holds; and an entry no admitted trial explains is reported as a disagreement. Two inputs ' +
        'cannot be recomputed here and each names the dependency that blocks it: the engine\'s canonical ' +
        'blake3 digest and the engine-side manifest of decision M1.3',
      BUNDLE,
      222,
      '"observed": "The envelope declares every required kind and resolves each reference from the content retained beside its digest; a reference whose content was not retained leaves the package unresolved and the recomputation incomplete rather than passing; a receipt resolves against the artifact the ledger holds; and an entry no admitted trial explains is reported as a disagreement. Two inputs cannot be recomputed here and each names the dependency that blocks it: the engine\'s canonical blake3 digest and the engine-side manifest of decision M1.3",',
    ),
    defect: fact(
      "the engine's canonical digest cannot be recomputed from retained bytes, and the engine-side run " +
        'manifest of decision M1.3 is unreachable from Python, so the envelope is not complete over its ' +
        'whole span',
      BUNDLE,
      230,
      '"defect": "the engine\'s canonical digest cannot be recomputed from retained bytes, and the engine-side run manifest of decision M1.3 is unreachable from Python, so the envelope is not complete over its whole span"',
    ),
    missing: fact(
      "the engine's canonical digest cannot be recomputed from retained bytes, and the engine-side run " +
        'manifest of decision M1.3 is unreachable from Python, so the envelope is not complete over its ' +
        'whole span',
      README,
      192,
      '| 10        | Every number recomputes                                                                         | the engine\'s canonical digest cannot be recomputed from retained bytes, and the engine-side run manifest of decision M1.3 is unreachable from Python, so the envelope is not complete over its whole span                 |',
    ),
  },
];

/** The two criteria that wait on the same absent join, in the page's own words. */
export const SHARED_CAUSE = fact(
  'Two of those four, criteria 2 and 10, wait on the same absent join, the run identity that would tie ' +
    "a seed, a build and a configuration together at the engine's own boundary.",
  README,
  194,
  'Two of those four, criteria 2 and 10, wait on the same absent join, the run identity that would tie a\n' +
    "seed, a build and a configuration together at the engine's own boundary. What the passing tests do not\n" +
    'show is what the design says itself: they are necessary and not sufficient, and an import-graph check\n' +
    'is evidence rather than proof.',
);
