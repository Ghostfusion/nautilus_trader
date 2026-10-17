/**
 * The shadow period's behaviour: the shared contract, the record, the counts and the refusals.
 *
 * If one of these broke, a run declaring a different contract would read as shadowing the backtest,
 * a divergence would be summarised away instead of reported, or a fill that precedes its own signal
 * would be counted rather than refused.
 */

import { describe, expect, test } from 'bun:test';

import {
  LABEL,
  REFUSAL_CODES,
  TEMPORAL_CONTRACT,
  contractDigest,
  declaredFields,
  filled,
  observe,
  parityCheck,
  shadowReport,
  watch,
} from '../src/engine/shadow.js';
import { workedExample, workedTotals } from '../src/data/shadow.js';

const NS = 1e9;

/** The page's worked example, as the feed would deliver it: instants in whole nanoseconds. */
function workedObservations() {
  return workedExample().map((row) => ({
    signal_at: row.signalSeconds * NS,
    intended_entry: row.intendedEntry,
    eligible_fill_at: row.eligibleFillSeconds * NS,
    fill_price: row.fillPrice,
    modelled_cost: row.modelledCost,
    observed_cost: row.observedCost,
    non_fill_reason: row.nonFillReason,
  }));
}

/** The plan the page's example runs under, with the declared count it states. */
function workedPlan(minimum = null) {
  return {
    candidate_id: 'frozen-candidate',
    specification_digest: 'sha256:specification',
    period: '2024-01-01 to 2024-02-01',
    minimum_observations: minimum ?? workedTotals().minimumObservations,
  };
}

/** The instants the replay of the worked example was expected to decide at. */
function expectedInstants() {
  return workedExample().map((row) => row.signalSeconds * NS);
}

describe('the temporal contract', () => {
  test('the declared field set is the four declarations of the data block, in their own order', () => {
    // The contract is what the run promised about when it decides and when it may execute, so the
    // order and the four names are the contract's identity.
    expect(declaredFields(TEMPORAL_CONTRACT).map(([name]) => name)).toEqual([
      'signal_frequency',
      'decision_time',
      'execution_eligibility',
      'corporate_action_treatment',
    ]);
    for (const [, value] of declaredFields(TEMPORAL_CONTRACT)) {
      expect(typeof value).toBe('string');
      expect(value.length).toBeGreaterThan(0);
    }
  });

  test('a changed declaration is a different contract, whatever it is called', () => {
    // Two runs that disagree about a declaration are not comparable, so the digest has to move.
    const moved = { ...TEMPORAL_CONTRACT, execution_eligibility: 'next_session_open' };
    expect(contractDigest(moved)).not.toBe(contractDigest(TEMPORAL_CONTRACT));
    expect(contractDigest({ ...TEMPORAL_CONTRACT })).toBe(contractDigest(TEMPORAL_CONTRACT));
  });

  test('parity holds only when the declarations and the pair of instants match', () => {
    // The two halves fail differently: a differing declaration means the run is not shadowing the
    // backtest, a differing instant means the feed was late or early.
    const reading = { contract: TEMPORAL_CONTRACT, decision_at: 0, eligible_fill_at: 86400 * NS };
    expect(parityCheck(reading, reading)).toEqual({
      same_fields: true,
      same_pair: true,
      holds: true,
      differing_fields: [],
      detail: 'replay and shadow share one temporal contract',
    });
    const shifted = { ...reading, eligible_fill_at: 172800 * NS };
    const shiftedCheck = parityCheck(reading, shifted);
    expect(shiftedCheck.same_fields).toBe(true);
    expect(shiftedCheck.same_pair).toBe(false);
    expect(shiftedCheck.holds).toBe(false);
    expect(shiftedCheck.detail).toBe(`the timestamps differ: (0, ${86400 * NS}) against (0, ${172800 * NS})`);
  });

  test('parity names the declaration that differs while the instants agree', () => {
    // Only the differing-field list says a run declared something else; the timestamps cannot.
    const reading = { contract: TEMPORAL_CONTRACT, decision_at: 0, eligible_fill_at: 86400 * NS };
    const other = {
      ...reading,
      contract: { ...TEMPORAL_CONTRACT, execution_eligibility: 'next_session_open' },
    };
    const check = parityCheck(reading, other);
    expect(check.same_pair).toBe(true);
    expect(check.holds).toBe(false);
    expect(check.differing_fields).toEqual(['execution_eligibility']);
    expect(check.detail).toBe('the declared fields differ: execution_eligibility');
  });
});

describe('a shadow period', () => {
  test('the report states every count the page computes from its five rows', () => {
    // Each of these is a count the report states rather than a summary of one, and the page's own
    // arithmetic is the check on them.
    const totals = workedTotals();
    const report = shadowReport({
      plan: workedPlan(),
      contract: TEMPORAL_CONTRACT,
      expected_signal_times: expectedInstants(),
      observations: workedObservations(),
    });
    expect(report.ok).toBe(true);
    expect(report.eligible_decisions).toBe(totals.eligibleDecisions);
    expect(report.observed_decisions).toBe(totals.observedDecisions);
    expect(report.reached).toBe(totals.reached);
    expect(report.missing_observations).toBe(totals.missingObservations);
    expect(report.signal_discrepancies).toHaveLength(totals.signalDiscrepancies);
    expect(report.non_fills).toHaveLength(totals.nonFills);
    expect(report.modelled_cost).toBe(totals.modelledCost);
    expect(report.observed_cost).toBe(totals.observedCost);
    expect(report.cost_difference).toBe(totals.costDifference);
    expect(report.diverged).toBe(totals.diverged);
    expect(report.label).toBe(LABEL);
    expect(report.schema).toBe('discovery.shadow/v1');
  });

  test('an observation carries all seven fields and says which of them filled', () => {
    // The page lists six fields and the design lists six; the record carries seven, and the reason
    // is what separates a security that did not trade from one the feed never delivered.
    const observations = workedObservations();
    const report = shadowReport({
      plan: workedPlan(),
      contract: TEMPORAL_CONTRACT,
      expected_signal_times: expectedInstants(),
      observations,
    });
    for (const observation of report.observations) {
      expect(Object.keys(observation).sort()).toEqual(
        [
          'eligible_fill_at',
          'fill_price',
          'filled',
          'intended_entry',
          'modelled_cost',
          'non_fill_reason',
          'observed_cost',
          'signal_at',
        ].sort(),
      );
    }
    expect(report.observations.map((row) => row.filled)).toEqual([true, true, false, false, true]);
    expect(filled(observations[0])).toBe(true);
    expect(filled(observations[2])).toBe(false);
  });

  test('a non-fill is reported with its reason, and a quiet row is neither a fill nor a non-fill', () => {
    // A non-fill is an intended entry that did not fill; a row that intended nothing is not
    // divergence, which is what keeps a quiet period from being reported as one.
    const report = shadowReport({
      plan: workedPlan(),
      contract: TEMPORAL_CONTRACT,
      expected_signal_times: expectedInstants(),
      observations: workedObservations(),
    });
    expect(report.non_fills).toEqual([
      {
        signal_at: 1209600 * NS,
        intended_entry: true,
        reason: 'the feed delivered no quote at the session open',
      },
    ]);
    expect(report.findings).toContain(
      `the signal at ${1209600 * NS} did not fill: the feed delivered no quote at the session open`,
    );
  });

  test('a period that reached its declared count is readable, and one short of it says why', () => {
    // Readable is a precondition for reading the report and evidence of nothing, so it is reported
    // beside the count it reached rather than instead of it.
    const readable = shadowReport({
      plan: workedPlan(4),
      contract: TEMPORAL_CONTRACT,
      expected_signal_times: expectedInstants(),
      observations: workedObservations(),
    });
    expect(readable.readable).toBe(true);
    expect(readable.unreadable_reason).toBe('');
    const short = shadowReport({
      plan: workedPlan(6),
      contract: TEMPORAL_CONTRACT,
      expected_signal_times: expectedInstants(),
      observations: workedObservations(),
    });
    expect(short.readable).toBe(false);
    expect(short.reached).toBe(5);
    expect(short.unreadable_reason).toBe(
      'the period reached 5 observations against the 6 declared before it started',
    );
  });

  test('a late signal is a discrepancy with a magnitude, and a duplicate is one at zero', () => {
    // A discrepancy says how far the feed sat from the instant the replay decided at; a duplicate
    // is reported at magnitude zero rather than dropped.
    const observations = workedObservations();
    const late = {
      ...observations[1],
      signal_at: observations[1].signal_at + NS,
      eligible_fill_at: observations[1].eligible_fill_at + NS,
    };
    const report = shadowReport({
      plan: workedPlan(),
      contract: TEMPORAL_CONTRACT,
      expected_signal_times: expectedInstants(),
      observations: [
        observations[0],
        observations[1],
        observations[1],
        late,
        observations[2],
        observations[3],
      ],
    });
    expect(report.signal_discrepancies).toEqual([
      { expected_at: 604800 * NS, observed_at: 604800 * NS, magnitude: 0 },
      { expected_at: 604800 * NS, observed_at: 604800 * NS, magnitude: 0 },
      { expected_at: 604800 * NS, observed_at: 604800 * NS + NS, magnitude: NS },
    ]);
    expect(report.missing_observations).toBe(1);
    expect(report.diverged).toBe(true);
  });

  test('missing observations count the expected instants the feed never delivered', () => {
    // A decision the feed never delivered a signal for is the one divergence the period cannot show
    // any other way.
    const observations = workedObservations();
    const report = shadowReport({
      plan: workedPlan(2),
      contract: TEMPORAL_CONTRACT,
      expected_signal_times: expectedInstants(),
      observations: [observations[0], observations[4]],
    });
    expect(report.eligible_decisions).toBe(5);
    expect(report.observed_decisions).toBe(2);
    expect(report.missing_observations).toBe(3);
    expect(report.findings).toContain('3 observations were never delivered');
    expect(report.diverged).toBe(true);
  });

  test('the cost difference is outside the diverged test and appears only as a finding', () => {
    // A period can be not diverged while the observed cost differs, which is exactly why the
    // difference is a finding line rather than part of the test.
    const filledRows = workedObservations().map((observation, index) => ({
      ...observation,
      fill_price: index === 3 ? null : 25,
      modelled_cost: index === 3 ? null : 0.5,
      observed_cost: index === 3 ? null : index === 0 ? 0.62 : 0.5,
      non_fill_reason: '',
    }));
    const quiet = shadowReport({
      plan: workedPlan(),
      contract: TEMPORAL_CONTRACT,
      expected_signal_times: expectedInstants(),
      observations: filledRows,
    });
    expect(quiet.non_fills).toHaveLength(0);
    expect(quiet.signal_discrepancies).toHaveLength(0);
    expect(quiet.missing_observations).toBe(0);
    expect(quiet.diverged).toBe(false);
    expect(quiet.cost_difference).toBe(0.12);
    expect(quiet.findings).toEqual(['the observed cost differed from the modelled one by 0.12']);
  });
});

describe('what the monitor refuses', () => {
  test('a plan that declared no observation count is refused before the period runs', () => {
    // A report that chose its own threshold after seeing the period could always be made to pass.
    expect(
      shadowReport({
        plan: workedPlan(0),
        contract: TEMPORAL_CONTRACT,
        expected_signal_times: expectedInstants(),
        observations: workedObservations(),
      }),
    ).toEqual({
      ok: false,
      code: 'minimum_not_declared',
      reason: 'the plan declares 0 as its minimum observation count',
    });
  });

  test('the feed refuses an observation eligible before its own signal', () => {
    // A fill that precedes its own signal is not a divergence but a broken record: the run used a
    // price it could not have had at the instant it decided.
    const state = watch({ plan: workedPlan(), expectedSignalTimes: expectedInstants() });
    expect(
      observe(state, {
        signal_at: 100 * NS,
        intended_entry: true,
        eligible_fill_at: 50 * NS,
        fill_price: 25,
        modelled_cost: 0.5,
        observed_cost: 0.5,
        non_fill_reason: '',
      }),
    ).toEqual({
      ok: false,
      code: 'eligible_before_signal',
      reason: `an observation at ${100 * NS} is eligible at ${50 * NS}`,
    });
    expect(REFUSAL_CODES).toEqual([
      'minimum_not_declared',
      'eligible_before_signal',
      'non_fill_without_reason',
    ]);
  });

  test('an intended entry that did not fill and gives no reason is refused rather than reported', () => {
    // A report that cannot say why a fill was missing is not diagnosable, and the refusal says
    // which signal it is about.
    const observations = workedObservations();
    const unexplained = {
      ...observations[2],
      non_fill_reason: '',
    };
    expect(
      shadowReport({
        plan: workedPlan(),
        contract: TEMPORAL_CONTRACT,
        expected_signal_times: expectedInstants(),
        observations: [observations[0], unexplained],
      }),
    ).toEqual({
      ok: false,
      code: 'non_fill_without_reason',
      reason: `the intended entry at ${1209600 * NS} did not fill and gives no reason`,
    });
  });
});

describe('the watch', () => {
  test('a watch folds one observation at a time and reports the same counts as the report', () => {
    // The monitor holds no connection: the feed is fed in, and what it delivered is the record.
    let state = watch({ plan: workedPlan(), expectedSignalTimes: expectedInstants() });
    for (const observation of workedObservations()) {
      const step = observe(state, observation);
      expect(step.ok).toBe(true);
      state = step.state;
    }
    expect(state.observations).toHaveLength(5);
    const report = shadowReport({ ...state });
    expect(report.observed_decisions).toBe(5);
    expect(report.reached).toBe(workedTotals().reached);
    for (const observation of state.observations) {
      expect(Object.keys(observation)).toHaveLength(7);
    }
    expect(() => observe(undefined, state.observations[0])).toThrow(
      'a shadow watch starts from a watch state, and none was given',
    );
  });
});
