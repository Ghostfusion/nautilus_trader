# 06 - Measure and evaluate

This lecture explains how to read what a backtest produced. Every number is defined, and every
number is shown with output from the run in lecture 05, so you can match the explanation to the
evidence.

## Where the numbers come from

The engine turns its own record of orders, fills, positions and account states into pandas tables.
The methods are on `BacktestEngine`, and the same reports are available from any cache through
`ReportProvider` (`docs/concepts/reports.md`):

- `engine.generate_account_report(venue)` for the account over time.
- `engine.generate_orders_report()` for every order.
- `engine.generate_order_fills_report()` for one row per filled order.
- `engine.generate_fills_report()` for one row per individual fill.
- `engine.generate_positions_report()` for every position, including closed cycles.

Statistics come from `engine.get_result()`. This is the analysis surface:

| Attribute              | What it is keyed by                   |
| ---------------------- | ------------------------------------- |
| `result.stats_pnls`    | Currency code, then statistic name    |
| `result.stats_returns` | Statistic name, computed from returns |
| `result.stats_general` | Statistic name, not PnL or returns    |

## The account report, column by column

The account report is one row per balance entry, so a multi-currency account has several rows per
event. It answers "how much money do I have, and how much is committed".

| Column             | Plain meaning                                                   |
| ------------------ | --------------------------------------------------------------- |
| `ts_event` (index) | When the account state was recorded                             |
| `account_id`       | The account's identifier                                        |
| `account_type`     | CASH, MARGIN, and so on                                         |
| `base_currency`    | The currency the account is denominated in                      |
| `total`            | Total balance in that currency                                  |
| `free`             | Balance not tied up in orders or margin                         |
| `locked`           | Balance committed to open positions or resting orders           |
| `currency`         | The currency of this row                                        |
| `reported`         | Whether the venue reported the balance or the engine derived it |
| `margins`          | Margin details, a list, empty when none                         |
| `info`             | Venue-specific extras                                           |

The last four rows of the run in lecture 05, with the display shortened to the money columns:

```text
                          account_type      total       free  locked currency
2019-01-02 01:12:00+00:00       MARGIN  999473.39  999173.39  300.00      USD
2019-01-02 01:37:00+00:00       MARGIN  999406.55  999406.55    0.00      USD
2019-01-02 01:38:00+00:00       MARGIN  999404.55  999104.55  300.00      USD
2019-01-02 01:46:39+00:00       MARGIN  999758.11  999758.11    0.00      USD
```

Read it as follows. The account started at 1,000,000 USD. It lost money over the run, and the final
row is 999,758.11 USD, which is a loss of 241.89 USD, far too much to be only commissions. The
`locked` value is 300.00 whenever a position is open and 0.00 when flat; that is the maintenance
margin the simulated venue reserves. The `reported` column, which the shortened display hides,
distinguishes a balance the venue reported from one the engine derived; the starting balance is
reported, and the intermediate states after fills are derived.

## The orders and fills reports, column by column

The orders report has one row per order; the order fills report keeps only orders with a positive
filled quantity and converts two timestamps to datetimes (`docs/concepts/reports.md`). Both carry
the same columns:

| Column                    | Plain meaning                                               |
| ------------------------- | ----------------------------------------------------------- |
| `client_order_id` (index) | The engine's name for the order                             |
| `instrument_id`           | What was traded                                             |
| `strategy_id`             | Which strategy sent it                                      |
| `trader_id`               | Which trader it belongs to                                  |
| `account_id`              | Which account it touched                                    |
| `venue_order_id`          | The venue's name for it, once accepted                      |
| `side`                    | BUY or SELL                                                 |
| `type`                    | MARKET, LIMIT, STOP_MARKET, and so on                       |
| `status`                  | INITIALIZED, SUBMITTED, ACCEPTED, FILLED, DENIED, and so on |
| `quantity`                | The ordered size                                            |
| `filled_qty`              | The filled size                                             |
| `price`                   | The limit price, when the order type has one                |
| `avg_px`                  | The average fill price                                      |
| `time_in_force`           | GTC, IOC, and so on                                         |
| `commissions`             | Fees paid, one entry per currency                           |
| `ts_init` and `ts_last`   | Timestamps                                                  |

The first two rows of the orders report from lecture 05:

```text
                               type  side quantity filled_qty  status   avg_px commissions
client_order_id
O-20190101-232900-001-000-1  MARKET  SELL   100000     100000  FILLED  109.334   [219 JPY]
O-20190101-235300-001-000-2  MARKET   BUY   100000     100000  FILLED  109.548   [219 JPY]
```

The first order sold 100,000 at an average price of 109.334 and paid 219 yen. The second bought the
same size back at 109.548 and paid the same. The second row is the cause of the first loss: sold at
109.334, bought back at 109.548, which is 0.214 yen worse on each of 100,000 units, or 21,400 yen
before fees.

A `DENIED` status is not the same as a rejection by the venue. Denied means the risk engine stopped
the order before it reached the venue; lecture 07 shows a whole run of denials.

## The positions report, column by column

A position is the net holding in one instrument. The report includes historical cycles as snapshot
rows, so a single instrument can appear many times (`docs/concepts/reports.md`).

| Column                                         | Plain meaning                                             |
| ---------------------------------------------- | --------------------------------------------------------- |
| `position_id` (index)                          | The position's identifier                                 |
| `instrument_id`                                | What was held                                             |
| `strategy_id`                                  | Which strategy opened it                                  |
| `trader_id`                                    | Which trader owns it                                      |
| `account_id`                                   | Which account it affects                                  |
| `opening_order_id`                             | The order that opened it                                  |
| `closing_order_id`                             | The order that closed it                                  |
| `entry`                                        | The side of the opening order                             |
| `side`                                         | LONG, SHORT, or FLAT                                      |
| `quantity`                                     | Current size                                              |
| `peak_qty`                                     | Largest size reached                                      |
| `avg_px_open`                                  | Average entry price                                       |
| `avg_px_close`                                 | Average exit price                                        |
| `commissions`                                  | Fees paid                                                 |
| `realized_pnl`                                 | Realised profit and loss, in the position's cost currency |
| `realized_return`                              | The same as a fraction, so 0.05 means 5 percent           |
| `ts_init`, `ts_opened`, `ts_last`, `ts_closed` | Timestamps                                                |
| `duration_ns`                                  | How long it was held                                      |
| `is_snapshot`                                  | True for an archived closed cycle                         |

The shortened positions report from lecture 05:

```text
                                                    side quantity  avg_px_open  avg_px_close realized_pnl  is_snapshot
position_id
USD/JPY.SIM-IntradayRuleStrategy-000-33bdd80d-e...  FLAT        0      109.334       109.548   -21838 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000-d4feceff-7...  FLAT        0      109.608       109.472   -14038 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000-ecea7d58-1...  FLAT        0      109.413       109.527   -11838 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000-4982d1b2-5...  FLAT        0      109.587       109.494    -9738 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000-88c167ee-6...  FLAT        0      109.434       109.505    -7538 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000                FLAT        0      109.565       109.956    38661 JPY        False
```

Five snapshot rows and one live row, all flat because the run has ended. The realised profit and
loss is in yen, because yen is the position's cost currency; the account is in USD, so the
statistics convert. Do not add the yen column and compare it to a dollar figure.

## The statistics, in plain words

The statistics for the same run, printed directly:

```text
stats_pnls: {'USD': {'Avg Winner': 351.59, 'Min Winner': 351.59, 'Avg Loser': -118.69000000000003, 'Max Winner': 351.59, 'PnL% (total)': -0.024189000000013037, 'PnL (total)': -241.89000000013039, 'Win Rate': 0.16666666666666666, 'Max Loser': -199.36, 'Expectancy': -40.31000000000003, 'Min Loser': -68.84}}
stats_returns: {'Average (Return, simple)': -0.0001209356549090046, 'Sortino Ratio (simple, population, 252 days)': -8.946770580788236, 'Sharpe Ratio (simple, sample, 252 days)': -7.437307156740296, 'Risk Return Ratio (simple, sample)': -0.46850631335084997, 'Returns Kurtosis (simple, sample)': nan, 'Average Loss (Return, simple)': -0.00030346130981806496, 'Tail Ratio (simple)': 0.15194988011811428, 'Profit Factor (simple)': 0.2029583278243312, 'Returns Volatility (simple, sample, 252 days)': 0.004097690789797422, 'Returns Skewness (simple, sample)': nan, 'Average Win (Return, simple)': 6.159000000005577e-05}}
stats_general: {'Long Ratio': 0.5}
```

| Statistic                                       | Meaning                                       | A good value looks like                    | A bad value looks like                                   |
| ----------------------------------------------- | --------------------------------------------- | ------------------------------------------ | -------------------------------------------------------- |
| `PnL (total)`                                   | Money made or lost in the currency            | Positive and larger than costs             | Negative, as here                                        |
| `PnL% (total)`                                  | The same as a fraction of starting balance    | Positive                                   | -0.0242, a 2.4 percent loss                              |
| `Win Rate`                                      | Fraction of closed positions that made money  | Above about 0.4 with a positive expectancy | 0.1667, one in six                                       |
| `Expectancy`                                    | Average money per position                    | Positive                                   | -40.31 USD                                               |
| `Avg Winner` / `Avg Loser`                      | Average size of a winning and losing position | Winner much larger than loser              | Winner 351.59 against loser -118.69, but too few winners |
| `Profit Factor (simple)`                        | Total won divided by total lost               | Above 1                                    | 0.203                                                    |
| `Sharpe Ratio (simple, sample, 252 days)`       | Return per unit of volatility, annualised     | Above about 1                              | -7.44                                                    |
| `Sortino Ratio (simple, population, 252 days)`  | Like Sharpe but only penalising downside      | Above about 1                              | -8.95                                                    |
| `Returns Volatility (simple, sample, 252 days)` | How much the returns bounce around            | Compared with your target                  | 0.0041                                                   |
| `Tail Ratio (simple)`                           | Right tail size against left tail size        | Above 1                                    | 0.152                                                    |
| `Long Ratio`                                    | Fraction of time spent long                   | Depends on the rule                        | 0.5, a balanced rule                                     |

Two cautions about the table. First, `NaN` is not zero; `Returns Skewness (simple, sample)` is `nan` because there
are too few returns to compute it, and the engine reports that state rather than inventing a number
(`docs/concepts/performance_periods.md`). Second, a Sharpe ratio computed from two hours of data
annualised to 252 days is arithmetic theatre. It is shown here because it is part of the surface,
not because it is meaningful at this sample size.

## Periodic frames

Statistics answer "how did it go overall". A periodic frame answers "how did it go each day, week
or month". A `PerformancePeriod` is one row of that frame, with accounting, trading activity,
exposure and derived performance grouped together. Boundaries are plain UTC calendar boundaries and
no exchange calendar is applied, which is why a real clock and a simulated clock produce the same
rows for the same period (`docs/concepts/performance_periods.md`). The frame is a projection of the
portfolio's own numbers, never a second ledger, and a period with no observation has no row.

## The three most common beginner misreadings

### 1. A profitable backtest from too few trades

The lecture 03 run printed:

```text
PnL (total): 2707.229999999865
Win Rate: 1.0
```

Six fills, three round trips, every one a winner. It looks excellent and it means nothing. Three
trades cannot distinguish skill from luck, and the data is a smooth sine wave, which is exactly the
shape a moving-average rule is built to follow. Treat any result with only a handful of trades as a
test that the program runs, not as evidence about the rule.

### 2. Ignoring fees

The same rule run with the identical data and zero fee rates instead of 0.00002:

```text
PnL (total): -217.89000000001397
```

The fee-paying run lost 241.89 USD; the zero-fee run lost 217.89 USD. The difference is exactly
24.00 USD, about ten percent of the loss, and it is the total of the twelve commissions in the fills
report. Fees did not change the sign here, but they decided a tenth of the outcome. On a rule with a
small edge, that tenth is the whole edge.

### 3. Ignoring the spread

The fills report shows the data's spread without naming it. The fixture's ask is always 0.010 above
its bid (lecture 04), and every order in this manual is a market order, so every order pays that
spread once. A round trip pays it twice:

1. One crossing costs `0.010 * 100,000 = 1,000` yen.
2. A round trip costs twice that, 2,000 yen.
3. Six round trips cost about 12,000 yen, which is roughly 110 USD at this price.

That is more than five times the commissions, and it is invisible in the statistics, because the
statistics measure the outcome rather than the cost that produced it. The only way to see it is to
look at the difference between the price you intended and the price you got, which is what the
fills report is for.

## The honesty rule

A number you cannot explain is not evidence. For every result you keep, be able to answer: how many
trades, what did costs take, what was the spread, and how much of the period was flat. If you cannot,
the backtest is decoration.

Next: what breaks in production, in [07-risks-and-limits](07-risks-and-limits.md).
