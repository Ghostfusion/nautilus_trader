/**
 * The campaign's declared candidate: the twelve blocks the schema requires, as plain data.
 *
 * This is a DECLARATION rather than a transcription: the values follow the schema's shape and are
 * drawn from the policy's declared sets, and nothing here asserts that they are admissible - the
 * candidate engine's `validateSpecification` is what proves that, in the campaign's second stage.
 *
 * The search moves genes of this template rather than of an invalid seed, which is what the
 * repository's own search does: it starts from candidates the compiler produced, and every later
 * candidate comes from an operator.
 */

/** The twelve blocks, in the order the schema declares them, with a declared value in each. */
export function declaredCandidate({ seed = 1 } = {}) {
  const comparison = (alias) => ({ feature: alias, comparison: 'gt', value: 0 });

  return {
    version: 'discovery.specification/v1',
    identity: {
      family: 'cross_sectional_rank',
      hypothesis: 'rank the eligible set and hold the leaders',
      mechanism: 'a cross-sectional rank of the declared feature',
      falsification: 'the rank carries no information about forward returns',
    },
    universe: {
      asset_classes: ['EQUITY', 'ETF'],
      membership_policy: 'point_in_time',
      source: 'the declared snapshot',
      min_price: 5,
    },
    data: {
      signal_frequency: 'daily',
      decision_time: 'session_close',
      execution_eligibility: 'next_session_open',
      corporate_action_treatment: 'adjusted',
    },
    features: { declared: { trend: { feature: 'close', lookback: 20 } } },
    selection: { mode: 'rank', feature: 'trend', percentile: 0.1, tie_break: 'feature_desc' },
    entry: { direction: 'long', conditions: { all: [comparison('trend'), comparison('trend')] } },
    exit: {},
    portfolio: {
      position_count: 20,
      weighting: 'equal',
      max_position_weight: 0.2,
      cash_policy: 'leave_idle',
    },
    rebalance: { frequency: 'weekly', timing: 'next_session_open' },
    risk: {},
    evaluation: {
      benchmark: 'campaign_benchmark',
      cost_model: 'campaign_cost_model',
      validation_policy: 'campaign_validation_policy',
    },
    provenance: { generator: 'hand-written', seed },
  };
}
