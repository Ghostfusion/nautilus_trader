# Entry/Exit Price Engine: design

Date: 2026-10-02. Status: design for review, not implemented.

## Scope

This extends the sector regime engine built in this repository: the thesis in
[`sector_rotation_strategies.md`](sector_rotation_strategies.md) and the application in
[`../implementation/sector-regime-engine/`](../implementation/sector-regime-engine/).

The two source documents reviewed are `entry_exit.md` (3,088 lines, 103 sections) and `entry_exit2.md`
(4,861 lines, 202 sections), from a separate research collection. They are written against a different
architecture, a score-based system with fundamental, valuation, technical, momentum, regime, risk,
news, sentiment and event scores. Section 12 maps the same formulas onto that shape, because the
formulas are architecture-independent and the other project may want them.

## 1. What the source documents are

Both are AI-assembled catalogues of entry and exit price formulas, with web citations in the margins.
Assessed honestly:

| Property        | Finding                                                                                                                                                                  |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Coverage        | Exhaustive. Reference prices, moving averages, volatility bands, valuation models, execution models, stops, targets, sizing, exits, regime adjustments                   |
| Overlap         | Very high. Bollinger, ATR, VWAP, Donchian, Kelly, break-even and regime adjustment all appear in both                                                                    |
| Formula quality | The formulas themselves are standard and correct where I checked them. Every formula adopted in this design was read in the source and is quoted with its section number |
| Labelling       | Loose. Many sections titled "entry price" produce a *condition*, not a price. The second document says so itself in its own first section                                |
| Provenance      | AI-generated with citations to secondary sources. No derivations, no empirical validation, no sample                                                                     |

The one genuinely valuable idea in the pair is a structural one, and both documents state it: keep
**four separate engines** (signal, price, risk, execution) and **four separate prices** (fair value,
entry price, target price, execution price), and never collapse them into one score-to-price formula.
The second document's section 202 renders that as a seven-layer stack ending in a `Gate` that can
reject the trade entirely.

That idea maps onto this engine almost exactly, because we already have the gate and we are missing
everything else.

## 2. Verdict

Yes, the documents can enhance the project, but the usable surface is much smaller than 305 sections,
and the boundary is not a matter of taste. **Anything that requires forecasting a return is out of
scope, because the thesis in Section 13 of the sector rotation note forbids acting on a forecast we
cannot validate.** What remains is execution realism and risk boundaries: machinery that cannot create
edge, only measure and bound it.

| Disposition | Content                                                                                                                                                                        | Why                                                                                                    |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| Take now    | Spread, slippage, market impact, participation caps, execution benchmarks, implementation shortfall                                                                            | Pure cost realism, always admissible, makes the current flat cost assumption honest                    |
| Take, gated | Volatility-scaled stops, trailing and chandelier stops, triple-barrier exits, time and alpha-decay exits, risk-budget sizing, correlation caps                                 | Position management for a directional book. Only meaningful when a regime admits a directional overlay |
| Adapt       | The four-engine and four-price separation, the layer stack, the `min` ceiling for entry limits                                                                                 | Structural. Adopt the shape, populate only the layers we can populate honestly                         |
| Reject      | DCF, relative valuation, analyst aggregation, scores-to-price mapping, HMM, RL, Bayesian, Ichimoku, fundamental deterioration, options-derived targets, Hurst-adjusted targets | Each requires a forecast, a calibrated mapping, or data we do not have                                 |

## 3. The governing constraint

The sector rotation note concludes that under memoryless leadership there is no directional edge, and
that overlays are admitted only on significant, material, out-of-sample evidence. An entry/exit price
engine does not change that, and this design must not become a back door for reintroducing prediction.

Two rules follow, and they constrain every formula below:

1. **Execution changes the cost of a decision, never the decision.** An execution model can only make
   a strategy worse; it can never make an inadmissible strategy admissible. This is why the execution
   layer can be enabled unconditionally while everything else must be gated.
2. **Every new mechanism gets its own attribution line.** A stop that improves the Sharpe ratio by
   truncating the right tail is not alpha, and the attribution must make that visible rather than
   letting it hide inside the total.

A third follows from the first two, and is worth stating because it is counter-intuitive:

3. **The risk layer applies to the overlay only, never to the structural book.** Layer A is a
   diversified allocation with no thesis to invalidate. Attaching a stop to it would be market timing
   by another name, and the note's Section 9 already says market timing of that kind is not admissible.

## 4. Where it sits

The source documents propose four engines. Mapped onto this project:

```
Source engine          Our component                              Status
---------------------  -----------------------------------------  --------------------------
Signal engine          regime classifier + eligibility gate        exists
Price engine           (not present)                               this design, partially
Risk engine            (not present)                               this design, gated
Execution engine       flat costBps on turnover                    this design, always on

Source document        Our pipeline stage
---------------------  ------------------------------------------
EntryDecision =        eligibility gate AND participation gate AND data gate
  Signal AND
  ExpectedReturnGate AND
  RiskGate AND
  LiquidityGate
```

The `ExpectedReturnGate` term in the source's conjunction has no counterpart here and is deliberately
dropped: we do not estimate expected returns, and inventing one would be the forecast the thesis
forbids.

## 5. The four prices, for this engine

The source's central distinction, applied honestly to what we hold:

| Price           | Source meaning                                 | Ours                                                                                             | Populated?                      |
| --------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------- |
| Fair value      | What the asset is theoretically worth          | No fundamental model, no valuation inputs                                                        | No, permanently for this engine |
| Entry price     | Maximum price at which the trade is acceptable | A limit price for the overlay's entries, derived from execution and risk, never from a forecast  | Yes, from Layer 2               |
| Target price    | Price at which profit is realised              | An exit ladder: stop, trailing, time barrier. No valuation target, because we cannot compute one | Partially                       |
| Execution price | Price actually obtainable                      | A modelled fill: spread, slippage, impact, participation-capped                                  | Yes, from Layer 1               |

Consequence worth stating plainly: **the entry price we produce is a limit, not a target.** It says
"do not pay more than this", which is a statement about cost and risk, not about value. The document's
own `min(FairValueEntry, ExpectedReturnEntry, RiskAdjustedEntry, ExecutionEntry)` is a ceiling
construction, and a ceiling is exactly what a limit order is.

## 6. Layer 1: execution model, always on

Replaces the current single `costBps` assumption. Sources: doc1 sections 71 to 78, doc2 sections 22 to
29. All formulas verified in the source.

**Effective price.** For a buy, the effective entry is above the signal price; for a sell, below:

```
P_exec_buy  = P_signal + half_spread + slippage + impact
P_exec_sell = P_signal - half_spread - slippage - impact
```

**Slippage.** Either a fixed proportion of price, or volatility-scaled:

```
slippage = c_sigma * sigma_daily * P_signal
```
where `c_sigma` is a fraction of a daily standard deviation, defaulting to 0.1.

**Impact.** The square-root form is the defensible one for a portfolio of this size (doc2 section 25,
doc1 section 75):

```
impact = Y * sigma_daily * (Q / ADV)^alpha,   alpha = 0.5 typically
```
with `Q` the order quantity and `ADV` the average daily volume. For our sector ETFs, orders are small
relative to ADV, so this term will usually be negligible. It is included because it becomes the
binding constraint if position size is ever raised, and because the participation cap depends on it.

**Participation cap.** An order may not exceed a fraction of market volume (doc1 section 76):

```
Q_max = p_max * Volume,   p_max default 0.05
```
Orders above the cap are clipped, and the clip is reported. This is the honest way to express a
capacity limit, and it is the one constraint that can silently invalidate a backtest if omitted.

**Benchmark and shortfall.** Where the strategy chooses to schedule execution rather than take the
close, the benchmarks are VWAP and TWAP (doc1 sections 77 and 78, doc2 sections 27 to 29):

```
VWAP_slippage = (P_exec - VWAP) / VWAP
implementation_shortfall_per_share = P_exec - P_decision
```
`P_decision` is the arrival price, the price at the moment the decision was made. Implementation
shortfall is the right measure because it charges the delay as well as the spread, and it becomes its
own attribution line.

**Break-even and minimum profitable exit.** These are accounting identities worth having explicitly
(doc1 sections 71 to 73):

```
P_break_even_long = P_entry + (C_entry + C_exit) / Q
P_exit_min_long   = P_entry * (1 + r_required) + Costs / Q
P_break_even_short = P_entry - (Costs + BorrowCost) / Q
```
The short form matters even though we are long-only today, because a market-neutral dispersion book is
listed as potentially eligible in the note's Section 9.

## 7. Layer 2: risk boundaries, gated

Enabled only when the regime admits a directional overlay. Sources: doc1 sections 32 to 36, 44 to 46,
58 to 59, 63 to 64, 66 to 70; doc2 sections 79, 82, 85 to 87.

**Volatility-scaled stop.** Three variants, all standard:

```
P_stop = P_entry - k * ATR_n                    (ATR stop)
P_stop = P_entry * (1 - k * sigma)              (volatility stop)
P_stop = support - k * ATR_n                    (support stop)
```
The support variant carries a real practical point from the source: do not place the stop exactly on
the observed support level, because that is where everyone else's is. Offset it.

**Trailing and chandelier.** Monotone by construction:

```
Stop_t = max(Stop_{t-1}, P_t - k * ATR_t)       (trailing)
CE_long = HighestHigh_n - k * ATR_n             (chandelier)
```
The monotonicity is the whole point and is testable: a trailing stop must never loosen.

**Triple barrier.** The cleanest formalisation of an exit in either document, and the one worth
adopting as the default structure:

```
Upper = P_entry + u
Lower = P_entry - d
TimeBarrier = T periods
Exit = first barrier reached
```
It forces the exit rule to be explicit about all three ways a position ends, which is exactly the
discipline the current overlay lacks (it holds until the next rebalance, which is an implicit time
barrier nobody chose).

**Time and decay exits.**

```
time exit:      t - t_entry >= N
alpha decay:    alpha(t) = alpha_0 * exp(-lambda * t),  exit when alpha(t) < alpha_min
                equivalently t > ln(alpha_0 / alpha_min) / lambda
```
The decay form is the principled version of a maximum holding period: it says the position is held
while its reason for existing is still strong enough to matter.

**Sizing.** Replaces equal weight on the overlay legs. Risk-budget sizing is the standard coupling
between entry, stop and size:

```
Q = (A * r) / (P_entry - P_stop)
```
and it produces a second, independent ceiling on the entry price, which is a nice consequence:

```
P_entry <= P_stop + RiskBudget / Q_max
```
Kelly appears in the source with the standard form `f* = (b*p - q) / b` and fractional scaling
`f = lambda * f*`. It is included only as an optional cap, never as the primary rule: Kelly requires a
win probability we can estimate for a *measured* process, and estimating one from a sample of overlay
trades would be fitting noise. Fractional Kelly at a low lambda is the only defensible use.

Correlation adjustment (doc1 section 70) is a heuristic, not a standard result, and is marked as such:
`Risk_adj = Risk_i * (1 + lambda * rho_i,p)`. If it is implemented it must be reported as a heuristic
parameter, not as an axiom.

## 8. Rejected, and why

This list matters as much as the accepted one, because accepting any of these would contradict the
thesis the engine is built on.

| Rejected                                                                                 | Source sections                      | Reason                                                                                                                                                                      |
| ---------------------------------------------------------------------------------------- | ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| DCF, Gordon growth, EV/EBITDA, PEG, P/B, residual income, sum-of-the-parts entries       | doc1 22 to 26; doc2 52 to 62         | Requires fundamentals we do not hold and a valuation model with unvalidated inputs                                                                                          |
| Analyst target aggregation, consensus targets                                            | doc2 64, 128, 129                    | Third-party forecasts, and no vendor data in this project                                                                                                                   |
| Score-to-price mapping, multi-score targets                                              | doc2 83, 84                          | Requires a calibrated score to expected-return mapping. That calibration is precisely the forecast the note forbids                                                         |
| HMM, Bayesian regime targets, RL exits, hazard and survival models, conformal prediction | doc2 115 to 121, 169, 170            | Model classes with no validated performance on our data. Adding them would multiply parameters without adding evidence                                                      |
| Hurst-adjusted targets                                                                   | doc2 181                             | Direct conflict. The sector rotation note places the Hurst exponent in the diagnostic tier precisely because it is estimator-sensitive and does not establish tradable edge |
| Fundamental deterioration, thesis-break, news and event exits                            | doc1 60, 61; doc2 153                | No fundamental or event data, and a thesis we never stated for a rule-based overlay                                                                                         |
| Ichimoku, parabolic SAR, Keltner, market profile, Fibonacci, pivot points                | doc1 13, 14, 16; doc2 47, 44, 46, 21 | Indicator proliferation. Each is another parameter with no independent evidence, and the note's Section 11 warns about exactly this                                         |
| Options-derived targets, delta and gamma adjustments, implied-volatility targets         | doc2 98 to 101                       | Requires an options surface we do not model                                                                                                                                 |
| Fair-value and valuation exits                                                           | doc1 62; doc2 52                     | Same reason as fair value above                                                                                                                                             |
| Options-income and volatility-selling overlays                                           | not in these documents               | Mentioned in the review of the note and still unrankable on the evidence                                                                                                    |

## 9. Attribution extension

The current five additive log-return lines stay, and three are added. Everything remains additive by
construction, and the residual continues to be printed.

```
Underlying sector exposure (drifting basket)
Rebalancing contribution (type B)
Directional overlay contribution
Trading cost                      -> split into:
    Spread and fees
    Slippage
    Market impact
Implementation shortfall          -> NEW, the cost of delay between decision and execution
Risk-layer effect                 -> NEW, the distributional change from stops and sizing
Participation clip effect         -> NEW, present only when an order was clipped
Holding cost
```

Two properties are required and testable:

1. The residual stays at floating-point noise, as it is today.
2. **The risk-layer line is not allowed to be called alpha.** It is reported with the win rate and both
   tails of the trade distribution alongside it, because a stop mechanically raises the win rate and
   truncates both tails. Reporting the line without the distribution would be the most likely way for
   this whole design to mislead.

## 10. Gates and data requirements

The engine gains three gates beyond the existing regime gate. Each is a hard rejection, not a warning:

| Gate          | Condition                                                  | Rejection behaviour                                                                       |
| ------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Regime        | The existing classifier admits a directional overlay       | Risk layer stays off; execution layer still applies                                       |
| Data          | OHLCV present for ATR, ADV and volume-based sizing         | Risk layer unavailable; execution falls back to the configured constant cost, and says so |
| Participation | Order notional within the fraction of average daily volume | Order clipped, clip reported, clip effect attributed                                      |

The data requirement is a real change. The current data model is a return matrix with no volume and no
intrabar range, so ATR and ADV cannot be computed from it. The implementation document specifies the
extension, including how synthetic data supplies volume so the engine remains testable offline.

## 11. Parameter discipline

This design adds roughly a dozen parameters, which is the main risk. The note's Section 10.7 requires
pre-registration, and the same rule applies to these:

- Every new parameter goes into the frozen configuration and therefore into the fingerprint. Changing
  one changes the instrument, which means the validation sweep must be re-run.
- Defaults are chosen from convention, not from fitting: `alpha = 0.5` for impact, `k = 2` for ATR
  stops, `k = 2.5` to 3 for chandeliers, `c_sigma = 0.1` for slippage, `p_max = 0.05` for
  participation.
- Any parameter tuned on the sample must be tuned on the observation window only and reported with the
  held-out result beside it.
- The parameters are counted and printed. A strategy with twelve unexamined parameters has twelve
  degrees of freedom, and the note's evidence hierarchy exists to keep that number visible.

## 12. Portability to the score-based architecture

The source documents are written for a score system with fundamental, valuation, technical, momentum,
regime, risk, news, sentiment and event scores. If that project wants the same engine, the mapping is
direct, with the same boundary:

| Source layer              | Populated by                                   | Boundary                                                              |
| ------------------------- | ---------------------------------------------- | --------------------------------------------------------------------- |
| 1 Fair value              | Not populated                                  | Requires a valuation model                                            |
| 2 Entry valuation         | Not populated                                  | Requires a margin of safety on a fair value                           |
| 3 Technical entry         | Support, VWAP and ATR levels                   | Levels, never directional signals                                     |
| 4 Risk entry              | Stop, drawdown, realised volatility            | No VaR or CVaR unless the positions are marked to a covariance matrix |
| 5 Liquidity and execution | The full execution model above                 | Always admissible                                                     |
| 6 Regime adjustment       | The sector regime classifier, or an equivalent | Gate, not a multiplier                                                |
| 7 Final entry             | `Gate(...)` plus a limit price ceiling         | The gate may reject; the ceiling may not raise the price              |

The scores provide evidence and constraints; this engine converts constraints into price levels. What
it must never do is convert a score into a forecast of return, which is the failure mode the source
documents come closest to encouraging.

## 13. What would falsify this

The design is only worth building if it can be shown to be worthless when it is. Three measurements:

1. **Execution realism changes the conclusion, or it does not.** If replacing the flat cost with the
   structural model changes no decision and no ranking in the frequency-and-band grid, the layer is
   bookkeeping and should be documented as such rather than built out further.
2. **The risk layer either improves something measurable or it is removed.** It must be evaluated on
   the overlay only, against the ungated overlay, on the held-out window, with the full distribution
   reported. If it only raises the win rate while lowering expected net return, that is a finding, and
   the honest response is to report it and default the feature off.
3. **Implementation shortfall must be small enough to ignore, or it must be attributed.** If the
   modelled delay cost is material, scheduling becomes a first-class decision and belongs in the design
   rather than in a footnote.

## 14. Non-goals

- Not a signal generator. Nothing here produces a reason to trade.
- Not a fair-value or valuation engine. Section 5 explains why that is permanent for this project.
- Not an execution venue integration. The fill model is analytic, not a broker connection.
- Not an options layer. Dispersion, volatility selling and covered calls remain unranked on the
  evidence in the note's Section 6.
- Not a replacement for the regime gate. It sits behind it.

## 15. Source map

| Adopted element                                                                              | Source section            | Verified in source |
| -------------------------------------------------------------------------------------------- | ------------------------- | ------------------ |
| Four engines, four prices, `min` ceiling, `Gate`                                             | doc1 100 to 103; doc2 202 | Yes                |
| Effective price, break-even, minimum profitable exit                                         | doc1 71 to 73             | Yes                |
| Execution price, impact, participation, VWAP slippage, implementation shortfall              | doc1 74 to 78             | Yes                |
| Spread-adjusted and slippage-adjusted execution                                              | doc2 22 to 24             | Yes                |
| Impact models, Almgren-Chriss form                                                           | doc2 25, 26               | Yes                |
| TWAP and VWAP execution, implementation shortfall                                            | doc2 27 to 29             | Yes                |
| ATR, percentage, volatility and support stops                                                | doc1 32 to 36             | Yes                |
| Trailing stop, percentage trailing, chandelier                                               | doc1 44 to 46             | Yes                |
| Time exit, alpha decay exit, expected-return and expected-value exits                        | doc1 58, 59, 63, 64       | Yes                |
| Kelly, risk budget, position-size ceiling, portfolio risk, correlation adjustment            | doc1 66 to 70             | Yes                |
| Triple barrier, signal-strength exit, regime-adjusted stop and target, volatility multiplier | doc2 79, 82, 85 to 87     | Yes                |

Quality note carried forward: the two documents overlap by more than half, and their section titles do
not reliably describe their content. Every formula adopted here was read in the source and is quoted
above; nothing was taken from a heading.
