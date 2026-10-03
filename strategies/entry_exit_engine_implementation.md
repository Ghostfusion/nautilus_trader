# Entry/Exit Price Engine: implementation

Date: 2026-10-02. Status: implementation plan for review, not implemented.

Companion to [`entry_exit_engine_design.md`](entry_exit_engine_design.md). That document decides what to
build and why; this one specifies how, precisely enough to write the code against. Section numbers in
brackets refer to that design document; source sections refer to the two reviewed catalogues.

## 1. Sequencing

Three phases, each independently useful, each with an acceptance test that can fail.

| Phase | Content                                      | Gate                               | Ships even if the next never does                     |
| ----- | -------------------------------------------- | ---------------------------------- | ----------------------------------------------------- |
| P1    | Execution and cost model                     | None. Always admissible [design 6] | Yes. Replaces a flat assumption with a structural one |
| P2    | Risk boundaries: stops, barriers, time exits | Regime gate only [design 7]        | Yes, but defaulted off                                |
| P3    | Risk-budget sizing                           | Regime gate, plus ADV              | Yes, but defaulted off                                |

P1 first, deliberately: it is the only phase that cannot create a false positive, because a cost model
can only make results worse.

## 2. Module boundary

Three new pure modules, no new dependencies, engine API kept backwards compatible.

```
src/execution.js    Layer 1  cost and fill model            new
src/risk.js         Layer 2  stops, barriers, exits         new
src/sizing.js       Layer 3  risk-budget sizing             new
src/engine.js       integration into backtest() and attribution()
src/data.js         OHLCV-capable data, synthetic volumes
src/ui.js           new panels and attribution rows
tests/execution.test.mjs  tests/risk.test.mjs  tests/sizing.test.mjs   new
tests/engine.test.mjs     extended, existing assertions unchanged
```

Rules that keep the existing suite meaningful:

- Every new `DEFAULT_CONFIG` key is additive with a default that reproduces today's behaviour exactly.
- `backtest()` keeps its signature and return shape; new fields are added, none renamed.
- `attribution()` keeps its five existing lines and their order; new lines are appended.

## 3. Data model changes

The current model is a return matrix, which is enough for volatility but not for volume or intrabar
range. Two changes, both optional and both detectable at runtime.

**Prices.** An index path per sector is required to talk about levels at all: stops, targets and entry
limits are levels, not returns. `pricesFromReturns` already exists, so this is a derivation, not new
data. `backtest()` will carry an internal price path built from the matrix, starting at 1.0.

**Volumes.** Optional. Used for the participation cap and for notional impact. Two sources:

- Synthetic: `syntheticUniverse` gains a `volumes` output, lognormal around a base ADV with a
  configurable dispersion, plus a `dailyNotional` convenience array.
- CSV: header columns prefixed `vol:` are read as volume columns alongside the price columns, for
  example `date,XLC,vol:XLC,XLY,vol:XLY,...`. The loader already splits on the header, so this is a
  prefix test rather than a format change.

**ADV.** Average daily notional per sector, either computed from volumes or supplied as a constant. If
neither is available, impact and participation are **skipped and reported as skipped**. Silent zeros are
the failure mode to avoid here: an impact model that returns 0.0 when it has no data is indistinguishable
from one that measured no impact.

**ATR.** True range needs OHLC, which the return matrix does not carry. P2 therefore uses a
close-to-close proxy by default:

```
sigma_daily   = stdev(returns)                    (per sector, rolling window)
ATR_proxy     = sigma_daily * P_ref
```

This is documented as a proxy, not as ATR. True ATR is a later optional enhancement requiring OHLC in
the CSV and an extension to the EODHD fetch tool, which already receives open, high, low, close and
volume and currently discards all but the adjusted close.

## 4. Configuration additions

Appended to `DEFAULT_CONFIG`. Defaults chosen from convention, not from fitting [design 11].

```
executionModel: 'flat' | 'structural'   'flat'          reproduces today's behaviour
halfSpreadBps:  number                  0               per side, so a round trip pays twice
commissionBps:  number                  0               explicit only, never bundled with spread
feeBps:         number                  0               regulatory and exchange fees
slippageSigma:  number                  0.1             fraction of a daily sigma
impactY:        number                  0.1             impact coefficient
impactAlpha:    number                  0.5             square-root law
maxParticipation: number                null            fraction of ADV notional; null disables
assumedAdvNotional: number|null         null            fallback when volumes are absent
riskLayer:      false                   boolean         P2 master switch
stopMethod:     'atr'|'vol'|'support'|'percent'|'chandelier'
stopK:          number                  2.0
stopPercent:    number                  0.08            used by 'percent'
timeBarrier:    number|null             null            in trading periods
exitOnBarrier:  'first'|'stop-first'    'first'         tie-break when both barriers hit in one period
sizingMode:     'equal'|'risk-budget'   'equal'
riskPerTrade:   number                  0.005           fraction of notional risked per leg
maxGrossExposure: number                1.0
kellyLambda:    number|null             null            null disables the Kelly cap entirely
maxCorrelation: number|null             null            heuristic cap [design 7]
```

Because every key enters `configFingerprint`, adding any of them changes the fingerprint and therefore
invalidates previous validation sweeps. That is intended and must be stated in the interface.

## 5. Phase 1: `src/execution.js`

Pure functions over per-trade inputs. All costs returned in **basis points of notional**, so weights can
be charged without needing share counts.

```
SPREAD_SOURCE     doc2 23
SLIPPAGE_SOURCE   doc1 74, doc2 24
IMPACT_SOURCE     doc1 75, doc2 25, 26
PARTICIPATION     doc1 76
BENCHMARK         doc1 77, 78, doc2 27 to 29
BREAK_EVEN        doc1 71, 72, 73
```

### 5.1 Signatures

```js
export function executionCost({ notional, advNotional, sigmaDaily, config })
// -> { spreadBps, feeBps, slippageBps, impactBps, totalBps, participation, clipped, clippedBps, notes }
```

```
spreadBps    = config.halfSpreadBps
feeBps       = config.commissionBps + config.feeBps
slippageBps  = config.slippageSigma * sigmaDaily * 10000
participation = advNotional && advNotional > 0 ? notional / advNotional : null
impactBps    = participation === null ? 0
               : config.impactY * sigmaDaily * Math.pow(participation, config.impactAlpha) * 10000
totalBps     = spreadBps + feeBps + slippageBps + impactBps
```

Edge cases that must be explicit in code and in tests:

- `advNotional` null or zero: `impactBps = 0` **and** `notes` gains `'impact skipped: no ADV'`.
- `participation > maxParticipation`: the order is clipped to the cap, `clipped = true`,
  `clippedBps` is the notional left untraded, expressed in bps of the intended order.
- `sigmaDaily` non-finite: treated as 0 and noted. This happens on the first periods of a series before
  a window is available, and a silent NaN propagating into the equity path is the worst outcome here.

### 5.2 Participation clip

```
if maxParticipation !== null and participation !== null and participation > maxParticipation:
    executedNotional = maxParticipation * advNotional
    clippedNotional  = notional - executedNotional
    clippedBps       = (clippedNotional / notional) * 10000
```

The clip is charged as **foregone exposure**, not as a cost: the weights are scaled to the executed
notional, the remainder stays in cash, and the effect is attributed on its own line [design 9].
Charging it as a cost would double-count it, because the untraded portion simply never participates in
the return.

### 5.3 Fill price

Where the interface needs a level rather than a cost:

```js
export function effectivePrice({ price, side, costBps })
// buy:  price * (1 + costBps / 10000)
// sell: price * (1 - costBps / 10000)
```
Source: doc1 section 74, doc2 section 24. This exists for the levels panel and for the trade log; the
backtest itself charges costs as drags rather than adjusting prices, which is why the two must agree in
a test (section 8.3).

### 5.4 Break-even identities

```js
export function breakEven({ entry, quantity, costsCurrency, borrowCurrency = 0, side = 'long' })
// long:  entry + (costsCurrency + borrowCurrency) / quantity
// short: entry - (costsCurrency + borrowCurrency) / quantity

export function minimumProfitableExit({ entry, requiredReturn, costsCurrency, quantity })
// entry * (1 + requiredReturn) + costsCurrency / quantity
```
Source: doc1 sections 71 to 73. Tested with hand-computed numbers, not with the implementation's own
arithmetic rearranged.

## 6. Phase 2: `src/risk.js`

Gated on the regime admitting a directional overlay [design 3, rule 3]. Never applied to the structural
book.

```
STOP_SOURCE       doc1 32 to 36
TRAILING_SOURCE   doc1 44 to 46
BARRIER_SOURCE    doc2 79
TIME_SOURCE       doc1 58, 59
REGIME_SOURCE     doc2 86, 87
```

### 6.1 Stop construction

```js
export function initialStop({ entry, atr, sigma, support, config })
// atr:        entry - stopK * atr
// vol:        entry * (1 - stopK * sigma)
// support:    support - stopK * atr          never exactly on support (doc1 section 35)
// percent:    entry * (1 - stopPercent)
// chandelier: highestHigh - stopK * atr      needs a lookback, evaluated in state

export function updateStop({ previousStop, price, atr, config })
// trailing:   max(previousStop, price - stopK * atr)
```

Invariant to test: `updateStop` is monotone non-decreasing for a long. A trailing stop that loosens is a
bug, and it is the kind of bug that only shows up in a drawdown.

### 6.2 Triple barrier

```js
export function evaluateBarriers({ entry, priceHigh, priceLow, close, stop, target, periodsHeld, config })
// -> { exited, reason, level }
```

```
upperHit = target !== null && priceHigh >= target
lowerHit = stop   !== null && priceLow  <= stop
timeHit  = config.timeBarrier !== null && periodsHeld >= config.timeBarrier

reason: 'stop' | 'target' | 'time' | null
```

Tie-break matters and must be configured, not assumed [design 11]. When both barriers are touched inside
one period, the default is `'first'`, meaning the more adverse barrier wins, because without intrabar
sequencing the optimistic choice is not defensible:

```
exitOnBarrier = 'first'      -> if upperHit and lowerHit, reason = 'stop'
exitOnBarrier = 'stop-first' -> explicit statement of the same thing, kept for clarity in the config
```

Intrabar path is unknown in the data we hold, so the conservative tie-break is the only honest default.
This is note-worthy in the interface, because it makes stops look worse than an optimistic backtest
would, and that is the intended direction of the error.

### 6.3 Time and decay exits

```js
export function timeExitReached({ periodsHeld, config })            // periodsHeld >= timeBarrier
export function decayedHoldingPeriod({ alpha0, alphaMin, lambda })  // ln(alpha0 / alphaMin) / lambda
```
Source: doc1 sections 58 and 59. The second converts an alpha half-life into a maximum holding period,
which is the principled form of "hold for N days". It is provided as a helper so the time barrier can be
set from a decay assumption rather than picked arbitrarily.

## 7. Phase 3: `src/sizing.js`

```
SIZING_SOURCE     doc1 66 to 70
```

```js
export function riskBudgetWeights({ selected, entry, stop, riskPerTrade, maxGrossExposure })
// per leg:  w_i = riskPerTrade / abs(entry_i - stop_i) / entry_i
// then:      scale all weights down if the sum exceeds maxGrossExposure
//            unallocated weight stays in cash
```

```
KELLY_SOURCE      doc1 66
kellyCap = kellyLambda === null ? Infinity
         : kellyLambda * (b * p - q) / b       with p the measured win rate, b = G / L
```

Two honest constraints on this phase:

- `p` must come from the **observation window only**, and the resulting cap is reported alongside the
  realised win rate, so a fitted win rate is visible as a fitted win rate.
- The correlation cap is a heuristic [design 7] and is reported as a parameter, never as a derived
  quantity.

## 8. Integration into `engine.js`

### 8.1 The backtest loop

Current order of operations per period, and the change:

```
1. mark to market, renormalise weights                       unchanged
2. recompute the overlay target on its frequency             unchanged
3. NEW: evaluate barriers for open overlay legs              P2, before the rebalance decision
4. NEW: recompute wanted weights if a barrier fired          P2, exited legs go to cash
5. NEW: compute order notional, run the execution model,      P1
        clip to the participation cap
6. rebalance if the band breaches or the calendar fires      cost now from step 5
7. charge holding cost                                       unchanged
8. push equity paths                                         plus the new parallel books
```

Step 3 must precede step 6: a stop that fires today has to change today's target weights, otherwise the
book rebalances into a position it has already decided to leave.

### 8.2 Books computed in parallel

`backtest()` already runs four series (basket, struct, actual, net). P2 adds one:

```
basket   drifting, gross                                  existing
struct   drift-band rebalanced, gross                     existing
actual   the book actually run, gross of costs            existing
net      actual with trading and holding cost             existing
stops    NEW: actual plus the risk layer, gross of costs   P2
```

The risk-layer attribution line is then `log(stops) - log(actual)`, which is exactly the effect of the
risk layer and nothing else [design 9].

### 8.3 Attribution extension

```
existing, unchanged, in order:
    Underlying sector exposure (drifting basket)
    Rebalancing contribution (type B)
    Directional overlay contribution
    Trading cost
    Holding cost

appended:
    Spread and fees
    Slippage
    Market impact
    Implementation shortfall
    Risk layer (with win rate and both tails)
    Participation clip
```

"Trading cost" stays as the total and the new cost lines are its decomposition, so the identity must be
stated explicitly rather than left implied: either the decomposition is reported *instead of* the total,
or the total is reported and the parts are shown as memo lines excluded from the sum. The implementation
chooses the second, and a test asserts that the sum of the reported lines still equals the total, so the
decomposition never double-counts.

Implementation shortfall needs a decision price the current loop does not record. It is the close on the
period the rebalance decision was taken, and the fill is modelled one period later by default, which is
the honest reading of "trade at the next session's open" from the note's Section 10.4. With that
convention, implementation shortfall is the return of the sector over the intervening period, and it is
usually the largest of the cost lines. That is a finding, not a defect, and it is the reason the design
makes it a first-class line [design 13, item 3].

### 8.4 Compatibility

`flat` execution with all offsets at zero must reproduce today's paths exactly. That is the first test
written in P1, before any new behaviour is added, and it is how the integration can be proven not to
have changed the meaning of the existing numbers.

## 9. Interface changes

Config panel, grouped behind one collapsible section to avoid doubling the visible parameter count:

```
Execution and risk
    execution model      flat | structural
    half spread (bps)    commission (bps)    fees (bps)
    slippage (fraction of daily sigma)
    impact Y / alpha     max participation    assumed ADV notional
    risk layer on/off    stop method / k / percent    time barrier
    sizing mode          risk per trade    max gross    kelly lambda
```

New panels:

- **Attribution** gains the new rows, unchanged layout.
- **Levels** (P2, optional): for the overlay legs, the entry level, the live stop and the target over
  time, with markers where a barrier fired. A table is enough; a second chart is not required.
- **Trade log** (P2): one row per overlay leg with entry period, entry level, exit period, exit level,
  exit reason, realised return, and the cost breakdown. This is the artefact that makes the risk layer
  auditable, and it is the only place where the win-rate and tail claims can be checked directly.
- **Parameter count**, printed next to the fingerprint, because the design adds about a dozen degrees of
  freedom and that number should be impossible to miss [design 11].

## 10. Test plan

### 10.1 Known-answer tests

Each with numbers computed by hand, not by the code under test:

| Test                                                                              | Expected                                                  |
| --------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `executionCost` with 2 bps half spread, 1 bps fees, sigma 1 percent, slippage 0.1 | spread 2, fee 1, slippage 10 bps, impact noted as skipped |
| Impact with participation 1 percent, Y 0.1, alpha 0.5, sigma 1 percent            | `0.1 * 0.01 * 0.1 * 10000 = 10` bps                       |
| Participation clip at cap 5 percent with an order at 8 percent of ADV             | executed 5, clipped 3, clip effect 3/8 in bps of intent   |
| `effectivePrice` buy at 100 with 15 bps                                           | 100.15                                                    |
| `breakEven` long, entry 100, quantity 200, costs 40 currency                      | 100.20                                                    |
| `minimumProfitableExit` entry 100, r 2 percent, costs 40, quantity 200            | 102.20                                                    |
| Chandelier with highest high 120, k 2, ATR 3                                      | 114                                                       |
| `decayedHoldingPeriod` alpha0 0.02, alphaMin 0.005, lambda 0.05                   | about 27.7 periods                                        |

### 10.2 Invariants

- **Equivalence.** `executionModel: 'flat'` with every offset zero gives equity paths identical to the
  current engine within 1e-12. This is the regression test for the whole of P1.
- **Monotonicity.** Each cost parameter increases total cost and decreases net equity, one at a time.
- **Trailing monotonicity.** `updateStop` never loosens for a long.
- **Barrier order.** With a path that touches the upper barrier first and the lower barrier second, the
  reported reason is `target`. With both touched in one period, the reported reason is `stop`.
- **Participation.** No simulated order ever exceeds `maxParticipation`.
- **Gating.** With the regime at MEMORYLESS or UNCERTAIN, the risk layer is inert: the `stops` series is
  identical to `actual`, and the risk-layer attribution line is zero.
- **Additivity.** The existing residual assertion continues to hold at floating-point noise with all new
  lines present.
- **Skipped-not-zero.** With no ADV, the impact line is reported as skipped, and the flag is visible in
  the result, not merely in a log.
- **Determinism.** Same seed, same paths.

### 10.3 Cross-checks against the old model

The structural model should be *calibrated* to the flat model rather than merely replacing it: pick
parameters such that the resulting average cost per rebalance equals the 5 bps currently used, and check
that the grid's conclusions are unchanged. If a realistic structural model materially changes the
ranking of frequencies and bands, that is the most interesting result this whole plan can produce, and it
belongs in the note as a correction.

## 11. Acceptance criteria

| Phase | Criterion                                                                                                                                                                                                   |
| ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P1    | Zero-offset equivalence holds; every parameter is monotone; impact is either measured or reported as skipped; implementation shortfall is attributed; no existing test changes meaning                      |
| P2    | Risk layer is inert unless the regime admits an overlay; trailing stops are monotone; the trade log reproduces every attributed unit of the risk-layer line; win rate and both tails are reported beside it |
| P3    | Sizing respects the risk budget and the gross cap; the Kelly cap, if enabled, uses an observation-window win rate and reports it as fitted                                                                  |

## 12. Risks and mitigations

| Risk                                                                       | Mitigation                                                                                                                      |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Parameter explosion, roughly a dozen new degrees of freedom                | Print the count, freeze them in the fingerprint, re-run the validation sweep after any change, keep defaults conventional       |
| Synthetic data has no overnight gaps, so stops look better than reality    | State it in the interface; prefer the conservative barrier tie-break; treat any stop result as an upper bound on its real value |
| Stops presented as improvement when they only reshape the distribution     | Mandatory win-rate and tail reporting; the line is never labelled alpha                                                         |
| Impact silently zero when ADV is absent                                    | Skip-and-report, tested explicitly; never return a silent zero                                                                  |
| Implementation shortfall turns out to dominate                             | That is the designed outcome of measuring it; it becomes a first-class line and a scheduling decision, not a hidden cost        |
| The other project's score architecture tries to fill the fair-value layers | The mapping in design section 12 keeps those layers unpopulated and the gate in place                                           |

## 13. Non-goals

Repeated from the design document because an implementation plan is where scope creep happens:

- No fair value, valuation, analyst or fundamental inputs.
- No score-to-price mapping, and no calibration of a score to an expected return. The other project has
  the scores; this engine deliberately does not.
- No machine-learning exits, regime models, or survival models.
- No options surface.
- No live execution or venue integration. The fill is analytic.
- No change to the regime decision rule. This engine consumes the verdict, it does not vote.

## 14. Traceability

| Element here                               | Design section | Source sections              |
| ------------------------------------------ | -------------- | ---------------------------- |
| Execution cost model                       | 6              | doc1 74 to 78, doc2 22 to 29 |
| Participation cap                          | 6              | doc1 76                      |
| Break-even identities                      | 6              | doc1 71 to 73                |
| Stops and trailing                         | 7              | doc1 32 to 36, 44 to 46      |
| Triple barrier and time exits              | 7              | doc2 79, doc1 58, 59         |
| Risk-budget sizing and Kelly cap           | 7              | doc1 66 to 70                |
| Attribution lines and the risk line caveat | 9              | doc1 74, 78; doc2 29         |
| Gates: regime, data, participation         | 10             | doc1 76, doc2 202            |
| Parameter discipline                       | 11             | doc2 202                     |
| Portable mapping to the score architecture | 12             | doc2 202                     |
