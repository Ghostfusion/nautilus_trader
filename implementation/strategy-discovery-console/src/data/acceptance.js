/**
 * The acceptance record, transcribed.
 *
 * The twenty-two criteria rows themselves live in `./layers.js`, because they are the shared record
 * every layer reads; this module holds what the console must not restate by hand: the count the bundle
 * reports, the rule that makes an entry an entry, and the four criteria that used to fail, with the
 * bundle's own text recording what closed each.
 *
 * The console never presents a failure as a pass and never fills a missing value with a default. The
 * record now holds no failure at all, so what is kept here as data is the closure rather than the
 * absence: criterion 2 and criterion 10 against the run identity decision M1.3 had left unbound,
 * criterion 4 against the settlement bridge, and criterion 6 against the absent dividend cash path.
 * Each carries the bundle's own `defect`, which now reads as what closed it.
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
  2071,
  '| Status                | Pass, fail, or blocked with the dependency that blocks it, and for a failure the open defect    |',
);

/** The tally the bundle reports, in the bundle's own sentence and as the four numbers. */
export const COUNTS = {
  sentence: fact(
    'The bundle as a whole holds twenty-two entries: all twenty-two pass with identities whose digests ' +
      'recompute, none is failed and none is blocked.',
    README,
    182,
    'The bundle as a whole holds twenty-two entries: all twenty-two pass with identities whose digests\n' +
      'recompute, none is failed and none is blocked. Four of them were failed until this change, and they',
  ),
  total: fact(
    22,
    README,
    182,
    'The bundle as a whole holds twenty-two entries: all twenty-two pass with identities whose digests',
  ),
  passed: fact(
    22,
    README,
    182,
    'The bundle as a whole holds twenty-two entries: all twenty-two pass with identities whose digests',
  ),
  failed: fact(0, README, 183, 'recompute, none is failed and none is blocked.'),
  blocked: fact(0, README, 183, 'recompute, none is failed and none is blocked.'),
  bundleObserved: fact(
    'twenty-two entries validate; all twenty-two pass with identities whose digests recompute from ' +
      'their components; none is failed and none is blocked, because every criterion\'s fixture now ' +
      "exists and meets its pass condition, and the defects criteria 2, 4, 6 and 10 carried are " +
      'recorded in their own entries rather than here. The bundle resolves its receipts against the ' +
      'content the caller retains, and a tampered identity or a missing dependency is a finding',
    BUNDLE,
    550,
    '"observed": "twenty-two entries validate; all twenty-two pass with identities whose digests recompute from their components; none is failed and none is blocked, because every criterion\'s fixture now exists and meets its pass condition, and the defects criteria 2, 4, 6 and 10 carried are recorded in their own entries rather than here. The bundle resolves its receipts against the content the caller retains, and a tampered identity or a missing dependency is a finding",',
  ),
};

/** The two rules that make this view a record rather than a claim. */
export const RULES = {
  reporting: fact(
    "A criterion is not satisfied by the existence of a design section, and this document's own " +
      'passing checks are evidence about documentation rather than about behaviour',
    DESIGN,
    1908,
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
    2073,
    'Three rules govern the bundle. An entry is produced by running the test, never by reading this\n' +
      "document, and a self-assessment is not an entry. A criterion whose fixture does not yet exist is\n" +
      'blocked, which is a legitimate status and the honest one.',
  ),
};

/**
 * The four criteria that used to fail, each with the bundle's own `observed` line and the `defect` it
 * now records against the criterion. That defect reads as what closed the failure rather than as what
 * is absent, so the view can show the closure in the bundle's words rather than in its own. The list
 * of failed criteria is empty, which is why this is a record of a closure and not a failure table.
 */
export const FORMER_FAILURES = [
  {
    id: fact(2, BUNDLE, 19, '"criterion": 2,'),
    requirement: fact(
      'A rerun is identical, and what is not deterministic is named.',
      BUNDLE,
      20,
      '"requirement": "A rerun is identical, and what is not deterministic is named.",',
    ),
    milestone: fact('M4', BUNDLE, 23, '"milestone": "M4",'),
    status: fact('passed', BUNDLE, 24, '"status": "passed",'),
    observed: fact(
      'Every number recomputes and nothing is unavailable. The four counts, each entry\'s identity, ' +
      'the declared non-deterministic components, the inventory\'s chain against a checkpoint held ' +
      'outside it, the envelope\'s references against the content retained beside their digests, and ' +
      'each receipt against the run manifest its registration retained all reconcile from the ' +
      'ledger and the inventory alone, and a reference whose content was not retained still leaves ' +
      'the package unresolved rather than passing. The engine\'s canonical digest is recomputed from ' +
      'the retained bytes with the extension\'s own hashing step, ' +
      'CanonicalBacktestResult.digest_of_bytes, which is the same step the engine takes over the ' +
      'document it holds. The manifest is the engine\'s own record and is reachable from Python for ' +
      'the first time: BacktestNode.get_run_manifest returns run_id, build_identity, ' +
      'configuration_digest, seed and manifest_digest, its keys are the field names a receipt ' +
      'carries, and manifest_digest is the BLAKE3 digest of the other four. The identical-rerun ' +
      'half of the requirement is evidenced by the composition fixtures, which run one experiment ' +
      'twice and observe the same canonical digest and the same canonical document bytes ' +
      '(tests/integration/test_discovery_engine_composition.py, ' +
      'test_one_backtest_per_experiment_with_identity)',
      BUNDLE,
      38,
      '"observed": "Every number recomputes and nothing is unavailable. The four counts, each ' +
      'entry\'s identity, the declared non-deterministic components, the inventory\'s chain against a ' +
      'checkpoint held outside it, the envelope\'s references against the content retained beside ' +
      'their digests, and each receipt against the run manifest its registration retained all ' +
      'reconcile from the ledger and the inventory alone, and a reference whose content was not ' +
      'retained still leaves the package unresolved rather than passing. The engine\'s canonical ' +
      'digest is recomputed from the retained bytes with the extension\'s own hashing step, ' +
      'CanonicalBacktestResult.digest_of_bytes, which is the same step the engine takes over the ' +
      'document it holds. The manifest is the engine\'s own record and is reachable from Python for ' +
      'the first time: BacktestNode.get_run_manifest returns run_id, build_identity, ' +
      'configuration_digest, seed and manifest_digest, its keys are the field names a receipt ' +
      'carries, and manifest_digest is the BLAKE3 digest of the other four. The identical-rerun ' +
      'half of the requirement is evidenced by the composition fixtures, which run one experiment ' +
      'twice and observe the same canonical digest and the same canonical document bytes ' +
      '(tests/integration/test_discovery_engine_composition.py, ' +
      'test_one_backtest_per_experiment_with_identity)",',
    ),
    closed: fact(
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
  },
  {
    id: fact(4, BUNDLE, 73, '"criterion": 4,'),
    requirement: fact(
      'A signal cannot fill before its declared eligibility time, and the test that proves it can fail.',
      BUNDLE,
      74,
      '"requirement": "A signal cannot fill before its declared eligibility time, and the test that proves it can fail.",',
    ),
    milestone: fact('M2', BUNDLE, 77, '"milestone": "M2",'),
    status: fact('passed', BUNDLE, 78, '"status": "passed",'),
    observed: fact(
      'The pre-enforcement half fails as expected: with no deferral the fill carries the decision ' +
      'bar\'s own timestamp and that bar\'s own close, and the account is debited at that instant. ' +
      'The enforced half holds for the latency model, whose fill is never earlier than the arrival ' +
      'and lands on the first bar at or after it. The shutdown drain no longer fabricates a fill: a ' +
      'settlement that reaches an arrival past the run\'s last observation settles every other ' +
      'command but leaves a single market order that increases the position unexecuted, so a ' +
      'deferred intent with no subsequent market data is reported as it stands, priced by nothing, ' +
      'the account is not debited, and the strategy is not told of a fill it never saw. The crate\'s ' +
      'own settlement tests hold under the same change, so a cancel-all and a position close ' +
      'emitted in on_stop are still processed through their own latency, which is what keeps the ' +
      'change a narrow one rather than a withdrawal of the settlement point. One case still ' +
      'disagrees with the table and is named here: the bridge mechanism resolves an artifact\'s ' +
      'eligibility to the next session\'s open and stamps the signal with it, but its submission ' +
      'path holds no clock, so the order set is submitted at delivery and the fill precedes the ' +
      'declared eligibility by the whole interval to the next session. With bar-only data the ' +
      'deferral prices at a bar close rather than at an opening price, so the declared eligibility ' +
      'can be honoured in time but not in price unless the snapshot carries market data at that ' +
      'instant',
      BUNDLE,
      83,
      '"observed": "The pre-enforcement half fails as expected: with no deferral the fill carries ' +
      'the decision bar\'s own timestamp and that bar\'s own close, and the account is debited at ' +
      'that instant. The enforced half holds for the latency model, whose fill is never earlier ' +
      'than the arrival and lands on the first bar at or after it. The shutdown drain no longer ' +
      'fabricates a fill: a settlement that reaches an arrival past the run\'s last observation ' +
      'settles every other command but leaves a single market order that increases the position ' +
      'unexecuted, so a deferred intent with no subsequent market data is reported as it stands, ' +
      'priced by nothing, the account is not debited, and the strategy is not told of a fill it ' +
      'never saw. The crate\'s own settlement tests hold under the same change, so a cancel-all and ' +
      'a position close emitted in on_stop are still processed through their own latency, which is ' +
      'what keeps the change a narrow one rather than a withdrawal of the settlement point. One ' +
      'case still disagrees with the table and is named here: the bridge mechanism resolves an ' +
      'artifact\'s eligibility to the next session\'s open and stamps the signal with it, but its ' +
      'submission path holds no clock, so the order set is submitted at delivery and the fill ' +
      'precedes the declared eligibility by the whole interval to the next session. With bar-only ' +
      'data the deferral prices at a bar close rather than at an opening price, so the declared ' +
      'eligibility can be honoured in time but not in price unless the snapshot carries market data ' +
      'at that instant",',
    ),
    closed: fact(
      'closed by this change, except for the bridge case it names. The shutdown drained and settled ' +
      'a deferred intent at a clock advanced past the last observation, which filled it at the ' +
      'run\'s last close without notifying the strategy; the drain now reaches each deferred arrival ' +
      'and settles everything there except a single market order that would increase the position, ' +
      'which stays unexecuted, so the fabricated fill is gone while the cancel and close paths keep ' +
      'the behaviour upstream\'s own tests assert',
      BUNDLE,
      85,
      '"defect": "closed by this change, except for the bridge case it names. The shutdown drained ' +
      'and settled a deferred intent at a clock advanced past the last observation, which filled it ' +
      'at the run\'s last close without notifying the strategy; the drain now reaches each deferred ' +
      'arrival and settles everything there except a single market order that would increase the ' +
      'position, which stays unexecuted, so the fabricated fill is gone while the cancel and close ' +
      'paths keep the behaviour upstream\'s own tests assert"',
    ),
  },
  {
    id: fact(6, BUNDLE, 116, '"criterion": 6,'),
    requirement: fact(
      'Corporate-action accounting reconciles in value and in cash.',
      BUNDLE,
      117,
      '"requirement": "Corporate-action accounting reconciles in value and in cash.",',
    ),
    milestone: fact('M2', BUNDLE, 120, '"milestone": "M2",'),
    status: fact('passed', BUNDLE, 121, '"status": "passed",'),
    observed: fact(
      'The value leg and the cash leg both reconcile. The adjustment series carries both actions: a ' +
      'split factor of one quarter and 2.50 per share, adjusting the raw 400.00 to 97.50. The ' +
      'corporate-action stage now takes effect at each action\'s own effective instant rather than ' +
      'at the position its announcement holds in the data stream, which is what made the earlier ' +
      'reading empty: the exchange queues an action it is given in advance and settles it when the ' +
      'clock reaches that instant, before the data at that instant, so an entitlement is measured ' +
      'on the positions that are open then. The held dividend credits the 250 the 100 shares were ' +
      'entitled to at the ex-date, in the position\'s settlement currency. The split restates the ' +
      'position to 400 shares and divides its price basis by four, so the raw series\' own fall from ' +
      '400.00 to 100.00 is not a loss. The round trip opened at 400.00 on 100 shares and closed at ' +
      '100.00 on the 400 the split left realises nothing, and the distribution is the whole of the ' +
      'cash it carries, so the same run without the action records realises -30,000 where the one ' +
      'with them carries 250. Two limits are named rather than left implicit: a dividend credits ' +
      'cash and recognises no profit and loss, because the raw series the run replays already ' +
      'carries the ex-date drop and recognising it a second time would count the distribution ' +
      'twice, and the split publishes no position event, so an event-store replay that rebuilds ' +
      'positions from recorded events rebuilds the quantity as opened',
      BUNDLE,
      126,
      '"observed": "The value leg and the cash leg both reconcile. The adjustment series carries ' +
      'both actions: a split factor of one quarter and 2.50 per share, adjusting the raw 400.00 to ' +
      '97.50. The corporate-action stage now takes effect at each action\'s own effective instant ' +
      'rather than at the position its announcement holds in the data stream, which is what made ' +
      'the earlier reading empty: the exchange queues an action it is given in advance and settles ' +
      'it when the clock reaches that instant, before the data at that instant, so an entitlement ' +
      'is measured on the positions that are open then. The held dividend credits the 250 the 100 ' +
      'shares were entitled to at the ex-date, in the position\'s settlement currency. The split ' +
      'restates the position to 400 shares and divides its price basis by four, so the raw series\' ' +
      'own fall from 400.00 to 100.00 is not a loss. The round trip opened at 400.00 on 100 shares ' +
      'and closed at 100.00 on the 400 the split left realises nothing, and the distribution is the ' +
      'whole of the cash it carries, so the same run without the action records realises -30,000 ' +
      'where the one with them carries 250. Two limits are named rather than left implicit: a ' +
      'dividend credits cash and recognises no profit and loss, because the raw series the run ' +
      'replays already carries the ex-date drop and recognising it a second time would count the ' +
      'distribution twice, and the split publishes no position event, so an event-store replay that ' +
      'rebuilds positions from recorded events rebuilds the quantity as opened",',
    ),
    closed: fact(
      'closed by this change. The corporate-action stage previously credited no entitlement and ' +
      'restated no quantity: a held position moved only by its purchase cost, the distribution it ' +
      'was entitled to was credited to nothing, and a round trip across the boundary realised a ' +
      'price fall where a total-return account of the same position carries the distribution. The ' +
      'stage now settles each action at its own effective instant, credits a dividend\'s entitlement ' +
      'to the positions that hold the instrument in their settlement currency, and restates a ' +
      'split\'s quantity and price basis, so the cash leg reconciles and the equity-readiness ' +
      'statement is no longer withheld',
      BUNDLE,
      128,
      '"defect": "closed by this change. The corporate-action stage previously credited no ' +
      'entitlement and restated no quantity: a held position moved only by its purchase cost, the ' +
      'distribution it was entitled to was credited to nothing, and a round trip across the ' +
      'boundary realised a price fall where a total-return account of the same position carries the ' +
      'distribution. The stage now settles each action at its own effective instant, credits a ' +
      'dividend\'s entitlement to the positions that hold the instrument in their settlement ' +
      'currency, and restates a split\'s quantity and price basis, so the cash leg reconciles and ' +
      'the equity-readiness statement is no longer withheld"',
    ),
  },
  {
    id: fact(10, BUNDLE, 215, '"criterion": 10,'),
    requirement: fact('Every number recomputes.', BUNDLE, 216, '"requirement": "Every number recomputes.",'),
    milestone: fact('M4', BUNDLE, 219, '"milestone": "M4",'),
    status: fact('passed', BUNDLE, 220, '"status": "passed",'),
    observed: fact(
      'Every number recomputes and no input is unavailable. The envelope declares every required ' +
      'kind and resolves each reference from the content retained beside its digest; a reference ' +
      'whose content was not retained still leaves the package unresolved and the recomputation ' +
      'incomplete rather than passing; a receipt resolves against the artifact the ledger holds; ' +
      'and an entry no admitted trial explains is still reported as a disagreement rather than ' +
      'resolved in one store\'s favour. The two inputs that were previously named are both resolved. ' +
      'The engine\'s canonical digest is recomputed from the bytes the store retains, with the ' +
      'extension\'s own hashing step, so the digest is checked and not only the document\'s SHA-256, ' +
      'and the record of that SHA-256 was corrected in the same change: the ledger recorded the ' +
      'digest of a wrapper document while the recomputation hashes the bytes themselves. The run ' +
      'manifest a receipt is checked against is the one the registration retained, produced by the ' +
      'engine through BacktestNode.get_run_manifest, so a recomputation checks a receipt against ' +
      'what the run recorded rather than against a document its caller supplied, and a receipt ' +
      'whose manifest is absent or disagrees is reported rather than passed',
      BUNDLE,
      234,
      '"observed": "Every number recomputes and no input is unavailable. The envelope declares ' +
      'every required kind and resolves each reference from the content retained beside its digest; ' +
      'a reference whose content was not retained still leaves the package unresolved and the ' +
      'recomputation incomplete rather than passing; a receipt resolves against the artifact the ' +
      'ledger holds; and an entry no admitted trial explains is still reported as a disagreement ' +
      'rather than resolved in one store\'s favour. The two inputs that were previously named are ' +
      'both resolved. The engine\'s canonical digest is recomputed from the bytes the store retains, ' +
      'with the extension\'s own hashing step, so the digest is checked and not only the document\'s ' +
      'SHA-256, and the record of that SHA-256 was corrected in the same change: the ledger ' +
      'recorded the digest of a wrapper document while the recomputation hashes the bytes ' +
      'themselves. The run manifest a receipt is checked against is the one the registration ' +
      'retained, produced by the engine through BacktestNode.get_run_manifest, so a recomputation ' +
      'checks a receipt against what the run recorded rather than against a document its caller ' +
      'supplied, and a receipt whose manifest is absent or disagrees is reported rather than ' +
      'passed",',
    ),
    closed: fact(
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
  },
];

/** The two criteria that shared the absent join, in the page's own words, and that join exists now. */
export const SHARED_CAUSE = fact(
  'Criteria 2 and 10 shared the absent join, the run identity that would tie a seed, a build and a ' +
    "configuration together at the engine's own boundary, and that join now exists.",
  README,
  194,
  'Criteria 2 and 10 shared the absent join, the run identity that would tie a seed, a build and a\n' +
    "configuration together at the engine's own boundary, and that join now exists. What the passing tests\n" +
    'do not show is what the design says itself: they are necessary and not sufficient, and an import-graph\n' +
    'check is evidence rather than proof.',
);
