# Performance periods

A `PerformancePeriod` is one row of a periodic result frame: it attributes accounting, trading
activity and exposure to a calendar period. The frame is a **projection of authorities that already
exist**, never a second ledger. It lives in `nautilus_analysis`. See
[Portfolio](portfolio.md) for the authority it reduces, and [Accounting](accounting.md) for the
rule that governs it.

## The record

A row is one period, and its fields are grouped by what they are rather than presented as one flat
bag:

| Group               | Fields                                                                                                      |
| ------------------- | ----------------------------------------------------------------------------------------------------------- |
| Accounting          | Period bounds, starting equity, ending equity, realised PnL, unrealised PnL, commission, fees and slippage. |
| Trading activity    | Volume, turnover, trade count, winning trades and losing trades.                                            |
| Exposure            | Open position count, gross exposure and net exposure.                                                       |
| Derived performance | Net PnL, net return, drawdown and drawdown percentage against the running equity peak.                      |

Monetary fields are exact and multi-currency: a `CurrencyTotals` holds one `Money` per currency
rather than one number, so a run spanning currencies cannot silently add unlike amounts.

## The reduction

The portfolio is the authority for realised and unrealised PnL per instrument, for equity and for
the position and exposure counts, and fills are the authority for trading activity and per-fill
costs. The reducer differences the portfolio's own numbers across a period and never recomputes PnL
from prices:

- `PerformancePeriodReducer::on_fill` records the period's activity and costs from a fill;
- `PerformancePeriodReducer::observe` takes a `PeriodObservation` carrying the portfolio's realised
  and unrealised PnL per instrument, its equity, and the position and exposure counts;
- `advance` closes every period boundary the timestamp crossed and returns the rows;
- `flush` closes the period in progress on shutdown or on demand.

A period with no observation has no row: the portfolio's numbers are required, so an absent
authority is not rendered as a zero.

## Boundaries

Periods are half-open, `[start, end)`. Boundaries are plain UTC civil-calendar boundaries, from the
timestamp alone, and no exchange trading calendar is applied:

- `PeriodKind::Day`: one UTC day, beginning at `00:00:00` UTC.
- `PeriodKind::IsoWeek`: the ISO-8601 Monday-to-Sunday week, beginning at the Monday `00:00:00` UTC.
- `PeriodKind::Month`: one UTC calendar month, beginning on the first day at `00:00:00` UTC.

Because the boundary is derived from the timestamp alone, the same reducer is driven by a real clock
in a live run and by a simulated clock in a backtest, and both produce the same rows for the same
period and the same accounting. The trigger is an interval trigger rather than a backtest callback,
which is the property a frame that exists only inside a backtester cannot offer.

## The statistics

The statistic framework gains a performance-period input beside returns, realised PnL and positions:
`PortfolioStatistic::calculate_from_periods`, the `MetricInput::PerformancePeriods` vocabulary entry,
and `PortfolioAnalyzer::report_period_metrics`. A registered statistic declares its
`MetricDefinition` as any other does, so it appears in a report with its units, direction and inputs
like the rest. A statistic that declares the frame input and is registered with the analyzer
contributes a row; a metric requested over a frame it does not declare is reported `unavailable`
with `MetricReason.UnsupportedInput`, and an empty frame is `unavailable` with
`MetricReason.InsufficientData`.

The built-in statistics defined over the frame, with the row title each renders and the report
category the row lands in. A period row whose units are `Currency` lands in `stats_general`; every
other unit lands in `stats_returns`:

| Statistic                             | Row title                                                        | Category |
| ------------------------------------- | ---------------------------------------------------------------- | -------- |
| `CostBasisPoints`                     | `Cost (basis points of turnover)`                                | returns  |
| `BreakevenCost`                       | `Breakeven Cost (basis points of turnover)`                      | returns  |
| `GrossReturn`                         | `Gross Return`                                                   | returns  |
| `NetReturn`                           | `Net Return`                                                     | returns  |
| `TotalTurnover`                       | `Total Turnover`                                                 | general  |
| `TotalCommissions`                    | `Total Commissions`                                              | general  |
| `MaxDrawdownDuration`                 | `Max Drawdown Duration (days)`                                   | returns  |
| `WinningMonthShare`                   | `Winning Month Share`                                            | returns  |
| `AverageMonthlyReturn`                | `Average Monthly Return (all)`, and `(winning)`, `(losing)`      | returns  |
| `ExposureRatio`                       | `Exposure Ratio (share of periods held)`                         | returns  |
| `ArithmeticCompoundingImpliedEquity`  | `Arithmetic Compounding Implied Equity (simple, tolerance 0.01)` | general  |
| `ArithmeticCompoundingRealisedEquity` | `Arithmetic Compounding Realised Equity (simple)`                | general  |
| `ArithmeticCompoundingRatio`          | `Arithmetic Compounding Ratio (simple, tolerance 0.01)`          | returns  |
| `ArithmeticCompoundingFlagged`        | `Arithmetic Compounding Flagged (simple, tolerance 0.01)`        | returns  |

Three of the rows read a month frame rather than any frame, and each refuses a frame it cannot
reduce. `Winning Month Share` is the share of months that closed with a positive net return and
`Average Monthly Return` is the mean monthly return, registered three times over - every month, the
winning months and the losing months - and both return no value unless every period is a whole
calendar month, because a share of months cannot be read from a frame of days or weeks. A month whose
return could not be resolved is excluded from the denominator rather than counted as a loss, so a
breakeven month is neither winning nor losing.

`Exposure Ratio (share of periods held)` is the share of periods that ended with at least one
position open. It is a sampled proxy for time in market and not a time-in-market figure: an intraday
position that opened and closed inside a period is invisible to it, and a true figure would need an
exposure accumulator in the period reducer, summing held duration within each period. The proxy is
what the frame's end-of-period exposure supports, and the row's own name says so.

`Average Trade Duration ({outcome}, days)` is defined over closed positions rather than the frame:
the mean `duration_ns` of the selected closed trades, converted to days, where a winner is a closed
position with a positive realised PnL and a loser one with a negative, and a breakeven or unresolved
position is neither. It is registered three times, giving the mean holding time of every closed
trade and of the winners and the losers separately, and a smaller value is preferred.

The two cost rows measure the frame's trading cost as a rate rather than a drag on its return.
`Cost (basis points of turnover)` is the frame's commission over its notional turnover, the all-in
cost rate the frame paid per unit traded, and `Breakeven Cost (basis points of turnover)` is the
frame's gross PnL over its notional turnover, the cost rate the strategy could have paid and still
broken even.

The arithmetic-compounding rows compare the terminal equity the frame's arithmetic mean net return
implies with the terminal equity the frame actually realised. The implied and realised equity rows
are the two terminals, in money; the ratio is their quotient, and the flag is `1.0` when the ratio is
further from one than the declared tolerance and `0.0` otherwise. The implied-equity, ratio and flag
rows carry the tolerance (default `0.01`); the realised-equity row does not, because it is a
measurement rather than a comparison. A frame that ended flat has a realised terminal of zero, so the
ratio carries no division; the two equity rows are always present when the frame reduces.

`ExponentiallyWeightedSharpe` is defined over a returns series rather than the frame, with a
configurable halflife, and renders `Exponentially Weighted Sharpe (simple, population, {annualisation} days, halflife {halflife})`.

**Undefined is not zero.** A statistic that cannot be computed returns the not-available state with
its reason, never `0.0`, and the objective layer continues to treat a missing metric as an error. A
genuine zero stays a real zero: no commissions is `0`, which is a measurement, while a Sharpe ratio
that cannot be computed is not.

The composite performance ratio is deliberately not implemented.

## Where it lives

`crates/analysis/src/period.rs` holds the record and the reducer; the frame statistics are one file
each under `crates/analysis/src/statistics/`, registered with the rest.
