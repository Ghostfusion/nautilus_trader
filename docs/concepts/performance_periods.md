# Performance periods

A `PerformancePeriod` is one row of a periodic result frame: it attributes accounting, trading
activity and exposure to a calendar period. The frame is a **projection of authorities that already
exist**, never a second ledger. It lives in `nautilus_analysis`. See
[Portfolio](portfolio.md) for the authority it reduces, and [Accounting](accounting.md) for the
rule that governs it.

## The record

A row is one period, and its fields are grouped by what they are rather than presented as one flat
bag:

| Group                  | Fields                                                                                          |
| ---------------------- | ----------------------------------------------------------------------------------------------- |
| Accounting             | Period bounds, starting equity, ending equity, realised PnL, unrealised PnL, commission, fees and slippage. |
| Trading activity       | Volume, turnover, trade count, winning trades and losing trades.                                 |
| Exposure               | Open position count, gross exposure and net exposure.                                            |
| Derived performance    | Net PnL, net return, drawdown and drawdown percentage against the running equity peak.           |

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

Four statistics are views over the frame or the returns series rather than each reporting path
reconstructing the same numbers:

| Statistic                       | Defined over            |
| ------------------------------- | ----------------------- |
| `MaxDrawdownDuration`           | The frame's equity.     |
| `ExponentiallyWeightedSharpe`   | A returns series, with a configurable halflife. |
| `TotalTurnover`                 | The frame.              |
| `TotalCommissions`              | The frame.              |

The statistic framework gains a performance-period input beside returns, realised PnL and positions:
`PortfolioStatistic::calculate_from_periods`, the `MetricInput::PerformancePeriods` vocabulary entry,
and `PortfolioAnalyzer::report_period_metrics`. A registered statistic declares its
`MetricDefinition` as any other does, so it appears in a report with its units, direction and inputs
like the rest.

**Undefined is not zero.** A statistic that cannot be computed returns the not-available state with
its reason, never `0.0`, and the objective layer continues to treat a missing metric as an error. A
genuine zero stays a real zero: no commissions is `0`, which is a measurement, while a Sharpe ratio
that cannot be computed is not.

The composite performance ratio is deliberately not implemented.

## Where it lives

`crates/analysis/src/period.rs` holds the record and the reducer; the four statistics are one file
each under `crates/analysis/src/statistics/`, registered with the rest.
