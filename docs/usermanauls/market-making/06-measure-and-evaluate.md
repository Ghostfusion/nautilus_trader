# 06 - Measure and evaluate

Lecture `03` printed three reports. This lecture explains every column of all three, adds the
portfolio statistics, and shows the fee arithmetic the engine is doing behind the scenes.

The report formats and column meanings are documented in `docs/concepts/reports.md`. The accounting
rules behind the PnL numbers are in `docs/concepts/accounting.md`.

## The account report

`engine.generate_account_report(SIM)` returns one row per account-state change. The lecture `03`
run produced thousands of rows, because every order acceptance, cancel, and fill locks or frees
cash. The last three rows of that report were:

```text
                                total   locked        free currency account_id account_type                                            margins  reported info base_currency
2019-01-01 23:16:39+00:00  9999970.00  5997.07  9993972.93      USD    SIM-001       MARGIN  [{'type': 'MarginBalance', 'initial': '1498.38...     False   {}           USD
2019-01-01 23:16:39+00:00  9999970.00  4498.69  9995471.31      USD    SIM-001       MARGIN  [{'type': 'MarginBalance', 'initial': '0.00', ...     False   {}           USD
2019-01-01 23:16:39+00:00  9994569.62     0.00  9994569.62      USD    SIM-001       MARGIN                                                 []     False   {}           USD
```

Every column in plain words:

| Column             | Meaning                                                                               |
| ------------------ | ------------------------------------------------------------------------------------- |
| `ts_event` (index) | When this account state was recorded.                                                 |
| `total`            | Total account value in `currency`.                                                    |
| `locked`           | Cash reserved against open orders, margin, and pending fees.                          |
| `free`             | `total - locked`, the cash you could withdraw or commit elsewhere.                    |
| `currency`         | The currency of the balance row. One row per currency per state.                      |
| `account_id`       | The account, here `SIM-001`.                                                          |
| `account_type`     | `MARGIN` or `CASH`.                                                                   |
| `margins`          | A list of margin balances: `initial` margin locked and `maintenance` margin required. |
| `reported`         | `True` if the venue reported this balance, `False` if the engine inferred it.         |
| `info`             | Venue-specific extra fields. Empty in this simulation.                                |
| `base_currency`    | The account's base currency, here USD.                                                |

The string `...` in the `margins` cell is pandas shortening a long cell to fit. Nothing is missing.

**What a good value looks like:** `free` stays close to `total`, because a market maker does not want
its entire balance reserved by resting orders. **What a bad value looks like:** `locked` grows until
`free` approaches zero. At that point the risk engine rejects new orders for insufficient margin.

## The order fills report

`engine.generate_order_fills_report()` returns one row per order that filled, indexed by
`client_order_id`. The full column list is:

| Column                    | Meaning in plain words                                 |
| ------------------------- | ------------------------------------------------------ |
| `client_order_id` (index) | Your own ID for the order.                             |
| `trader_id`               | The trader that owns the order.                        |
| `strategy_id`             | The strategy that submitted it.                        |
| `instrument_id`           | What was traded.                                       |
| `side`                    | `BUY` or `SELL`.                                       |
| `type`                    | `LIMIT`, `MARKET`, and so on.                          |
| `quantity`                | The order's original size.                             |
| `price`                   | The limit price, or null for a market order.           |
| `status`                  | Final status, normally `FILLED`.                       |
| `time_in_force`           | `GTC` (rests), `GTD` (expires), `IOC`, `FOK`.          |
| `expire_time_ns`          | Expiry timestamp in nanoseconds, for `GTD`.            |
| `is_post_only`            | `True` if the order was only allowed to rest.          |
| `is_reduce_only`          | `True` if the order may only reduce a position.        |
| `is_quote_quantity`       | `True` if `quantity` is in quote currency, not base.   |
| `filled_qty`              | How much actually traded.                              |
| `init_id`                 | Internal event ID that created the order.              |
| `ts_init`                 | When the order was created.                            |
| `ts_last`                 | When the order last changed, here its fill time.       |
| `commissions`             | The fees charged, as a list of `Money`.                |
| `venue_order_id`          | The venue's own ID for the order.                      |
| `display_qty`             | Visible size, for an iceberg order.                    |
| `emulation_trigger`       | The event type that releases an emulated order.        |
| `trigger_instrument_id`   | The instrument used to trigger a conditional order.    |
| `contingency_type`        | `OCO`, `OTO`, `OUO`, or none.                          |
| `order_list_id`           | The list an order belongs to.                          |
| `linked_order_ids`        | Other orders linked to this one.                       |
| `parent_order_id`         | For a contingent child order.                          |
| `exec_algorithm_id`       | The execution algorithm that handled the order.        |
| `exec_algorithm_params`   | The algorithm's parameters.                            |
| `exec_spawn_id`           | The primary order ID this order was spawned from.      |
| `tags`                    | Free-form labels.                                      |
| `account_id`              | The account the fill settled to.                       |
| `slippage`                | The slippage the slippage model applied.               |
| `position_id`             | The position the fill belongs to.                      |
| `liquidity_side`          | `MAKER` or `TAKER`. The column a market maker watches. |
| `last_trade_id`           | The venue trade ID of the last fill.                   |
| `avg_px`                  | The average fill price, which may differ from `price`. |

The program:

```python
fills = engine.generate_order_fills_report()
print(fills[["side", "type", "quantity", "price", "filled_qty", "avg_px", "liquidity_side", "commissions"]].to_string())
```

Observed on the lecture `03` run:

```text
                              side    type quantity    price filled_qty   avg_px liquidity_side commissions
client_order_id
O-20190101-230000-001-001-2   SELL   LIMIT   500000  109.538     500000  109.538          MAKER  [1095 JPY]
O-20190101-230000-001-001-4   SELL   LIMIT   500000  109.571     500000  109.572          MAKER  [1096 JPY]
O-20190101-230000-001-001-6   SELL   LIMIT   500000  109.604     500000  109.604          MAKER  [1096 JPY]
O-20190101-231639-001-001-13   BUY  MARKET  1500000      NaN    1500000  109.965          TAKER  [3299 JPY]
```

Notice order `-4`: the limit price is `109.571` but the average fill price is `109.572`. A sell
limit can only fill at or above its limit, and here it filled one tick above, because the market
had traded up to `109.572` by the time the order was touched. The two columns answer different
questions: `price` is what you asked for, `avg_px` is what you got. They differ whenever the market
fills you at a better price than your limit. A slippage draw moves the fill the other way, against
the order direction (`docs/concepts/backtesting/fill-models.md`); that is `prob_slippage`, which is
`0.0` in this run.

## The positions report

`engine.generate_positions_report()` returns one row per position cycle, indexed by `position_id`.
Under `NETTING` OMS, closed cycles are archived as snapshot rows with generated IDs so their PnL is
preserved (`docs/concepts/reports.md`).

```text
                        entry  side quantity peak_qty  avg_px_open  avg_px_close realized_pnl commissions
position_id
USD/JPY.SIM-GRID_MM-001  SELL  FLAT        0  1500000   109.571333       109.965  -597086 JPY  [6586 JPY]
```

Every column in plain words:

| Column                | Meaning                                        |
| --------------------- | ---------------------------------------------- |
| `position_id` (index) | The position identifier.                       |
| `type`                | Always `Position`.                             |
| `events`              | The fill events that built the position.       |
| `adjustments`         | Accounting adjustments, if any.                |
| `trader_id`           | The trader that owns it.                       |
| `strategy_id`         | The strategy that opened it.                   |
| `instrument_id`       | What was held.                                 |
| `account_id`          | The account that holds it.                     |
| `opening_order_id`    | The order that opened the position.            |
| `closing_order_id`    | The order that closed it.                      |
| `entry`               | `BUY` or `SELL`: which side opened the cycle.  |
| `side`                | `LONG`, `SHORT`, or `FLAT`.                    |
| `quantity`            | Current size; `0` once flat.                   |
| `peak_qty`            | The largest size reached during the cycle.     |
| `price_precision`     | Decimal places allowed on price.               |
| `size_precision`      | Decimal places allowed on size.                |
| `multiplier`          | Contract multiplier, usually `1` for FX.       |
| `is_inverse`          | `True` for inverse contracts.                  |
| `ts_init`             | Position creation time.                        |
| `ts_opened`           | When the position opened.                      |
| `ts_last`             | Last update time.                              |
| `ts_closed`           | When it closed, or null.                       |
| `duration_ns`         | How long the cycle lasted.                     |
| `avg_px_open`         | Average entry price.                           |
| `avg_px_close`        | Average exit price, or null if still open.     |
| `realized_return`     | Exit versus entry as a fraction, net of costs. |
| `realized_pnl`        | Profit or loss booked, in the cost currency.   |
| `venue_order_ids`     | The venue order IDs in the cycle.              |
| `trade_ids`           | The trade IDs in the cycle.                    |
| `buy_qty`             | Total quantity bought during the cycle.        |
| `sell_qty`            | Total quantity sold during the cycle.          |
| `commissions`         | Fees paid, one entry per currency.             |
| `is_snapshot`         | `True` for an archived closed cycle.           |

**What a good value looks like:** many cycles with a mixture of small wins and small losses, and
`realized_pnl` positive on average. **What a bad value looks like:** every cycle negative, or one
cycle with a huge negative `realized_pnl` from a position that got stuck. The lecture `05` run shows
the bad case: seven cycles, all negative, `-5376024 JPY` in total.

`peak_qty` is the number to watch for inventory risk. It records the worst inventory you actually
carried, which is what the drawdown and the margin were driven by.

## Fees done by hand

The report shows `commissions` as `[1095 JPY]` and so on. The fee model is
`MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002"))`, so the fee is
`filled_qty * avg_px * rate`, rounded to the currency's precision. Here is the engine's number
beside the hand calculation:

```python
maker_rate = Decimal("0.00002")
taker_rate = Decimal("0.00002")
for order_id, row in fills.iterrows():
    notional = Decimal(int(row["filled_qty"])) * Decimal(str(row["avg_px"]))
    rate = maker_rate if row["liquidity_side"] == "MAKER" else taker_rate
    fee = notional * rate
    print(
        f"{order_id}  {row['liquidity_side']:5s}  notional={notional}  fee={fee:.2f}  "
        f"engine={row['commissions']}",
    )
```

Observed output:

```text
O-20190101-230000-001-001-2  MAKER  notional=54769000.000  fee=1095.38  engine=['1095 JPY']
O-20190101-230000-001-001-4  MAKER  notional=54786000.000  fee=1095.72  engine=['1096 JPY']
O-20190101-230000-001-001-6  MAKER  notional=54802000.000  fee=1096.04  engine=['1096 JPY']
O-20190101-231639-001-001-13  TAKER  notional=164947500.000  fee=3298.95  engine=['3299 JPY']
```

Read the first line in words. `500,000 * 109.538 = 54,769,000` yen of value. At `0.00002`, the fee
is `1,095.38` yen, rounded to `1,095` yen because the yen's precision is zero. The engine and the
hand calculation agree on every line. The JPY in the commission is the quote currency of USD/JPY,
which is the position's cost currency.

Now the realized PnL by hand, for the position report row:

- The strategy sold `1,500,000` at an average of `109.571333`.
- It bought back `1,500,000` at `109.965`.
- Gross: `(109.571333 - 109.965) * 1,500,000 = -590,500.5` yen.
- Commissions: `1095 + 1096 + 1096 + 3299 = 6,586` yen.
- Net: `-590,500.5 - 6,586 = -597,086.5`, rounded to `-597086 JPY`.

That matches the report exactly. **`realized_pnl` is net of commissions**, so you do not add the
commissions column again when you total it.

## Portfolio statistics

For the whole run, `engine.get_result()` returns the aggregate statistics. The program:

```python
result = engine.get_result()
print("iterations:", result.iterations)
print("total orders:", result.total_orders)
print("total positions:", result.total_positions)
print("stats_pnls:")
for currency, values in result.stats_pnls.items():
    print(f"  {currency}")
    for key, value in values.items():
        print(f"    {key}: {value}")
print("stats_returns:")
for key, value in result.stats_returns.items():
    print(f"  {key}: {value}")
print("stats_general:")
for key, value in result.stats_general.items():
    print(f"  {key}: {value}")
```

Observed for the 10,000-quote run:

```text
iterations: 10000
total orders: 103
total positions: 7
stats_pnls:
  USD
    Min Loser: -4261.41
    Expectancy: -7006.355714285715
    Avg Winner: nan
    Win Rate: 0.0
    Avg Loser: -7006.355714285715
    PnL (total): -49045.75
    PnL% (total): -0.49045750000000005
    Max Winner: nan
    Min Winner: nan
    Max Loser: -8421.9
stats_returns:
  Average Loss (Return, simple): -0.0024551543498598782
  Sortino Ratio (simple, population, 252 days): -15.500882570623205
  Profit Factor (simple): 0.0
  Returns Volatility (simple, sample, 252 days): 0.012174477959753549
  Returns Kurtosis (simple, sample): nan
  Sharpe Ratio (simple, sample, 252 days): -50.819336829881934
  Risk Return Ratio (simple, sample): -3.201317310597456
  Tail Ratio (simple): 0.66834620803012
  Average (Return, simple): -0.0024551543498598782
  Average Win (Return, simple): nan
  Returns Skewness (simple, sample): nan
stats_general:
  Long Ratio: 0.43
```

### PnL statistics

| Statistic      | Meaning                                                                 |
| -------------- | ----------------------------------------------------------------------- |
| `PnL (total)`  | Total profit or loss, converted to the account base currency (USD).     |
| `PnL% (total)` | The same as a percent of the starting balance: `-0.49` percent here.    |
| `Win Rate`     | Fraction of cycles that ended positive. `0.0` means none did.           |
| `Avg Winner`   | Average profit of winning cycles. `nan` when there are no winners.      |
| `Avg Loser`    | Average loss of losing cycles.                                          |
| `Max Winner`   | Best cycle. `nan` when there are none.                                  |
| `Max Loser`    | Worst cycle, `-8421.90` USD here.                                       |
| `Min Winner`   | Smallest winning cycle.                                                 |
| `Min Loser`    | Smallest losing cycle, `-4261.41` USD.                                  |
| `Expectancy`   | Expected value per cycle. Negative means the strategy loses on average. |

`nan` is "not a number", Python's way of saying the statistic is undefined. A strategy with no
winners has an undefined `Avg Winner`; it is not zero.

Note the unit change. The positions report is in JPY, the instrument's cost currency. `stats_pnls`
is in USD, the account base currency. The total `-49,045.75` USD is the `-5,376,024` JPY converted
at roughly `109.6` USD per JPY division, that is about `-49,000` USD. If you compare the two without
converting, you will think one of them is wrong.

### Return statistics

| Statistic                                       | Meaning                                                                  |
| ----------------------------------------------- | ------------------------------------------------------------------------ |
| `Average (Return, simple)`                      | Average per-period return.                                               |
| `Average Win (Return, simple)`                  | Average per-period return on winning periods.                            |
| `Average Loss (Return, simple)`                 | Average per-period return on losing periods.                             |
| `Profit Factor (simple)`                        | Gross profit divided by gross loss. `0.0` means no profit at all.        |
| `Returns Volatility (simple, sample, 252 days)` | Annualised standard deviation of returns.                                |
| `Sharpe Ratio (simple, sample, 252 days)`       | Return per unit of volatility; negative here because the strategy loses. |
| `Sortino Ratio (simple, population, 252 days)`  | Like Sharpe but penalising only downside volatility.                     |
| `Returns Skewness (simple, sample)`             | Asymmetry of the return distribution.                                    |
| `Returns Kurtosis (simple, sample)`             | How fat the tails are; fat tails mean occasional extreme days.           |
| `Tail Ratio (simple)`                           | Right-tail versus left-tail magnitude.                                   |
| `Risk Return Ratio (simple, sample)`            | Return divided by drawdown risk.                                         |

**What a good value looks like:** `Win Rate` high, `Avg Winner` larger than `Avg Loser`,
`Profit Factor (simple)` above `1.0`, `Sharpe Ratio` positive. **What a bad value looks like:** exactly this
run: win rate zero, profit factor zero, Sharpe `-50.8`. Do not tune the strategy until `Profit Factor (simple)` is above `1.0` on data that at least oscillates.

### General statistics

| Statistic    | Meaning                                                     |
| ------------ | ----------------------------------------------------------- |
| `Long Ratio` | Fraction of position snapshots that were long. `0.43` here. |

## The three most common beginner misreadings

1. **"The account total barely moved, so I lost almost nothing."** The account total starts at ten
   million USD and the loss is about `49,000` USD, less than half a percent. But the strategy
   turned over millions per cycle to earn that. The meaningful number is `PnL% (total)` against the
   risk taken, not the absolute loss against a large starting balance.

2. **"My win rate is high, so the strategy is good."** A market maker can win `90` percent of round
   trips and still lose money, because the losing round trips are much larger. Look at
   `Avg Winner` versus `Avg Loser` and at `Max Loser`. A strategy that wins small and loses big has
   a high win rate and a negative expectancy.

3. **"`realized_pnl` is gross; I should subtract commissions."** It is not. `realized_pnl` already
   includes commissions, as the hand calculation above shows. Adding the commissions column again
   double counts them.

## Where the numbers come from

- Report columns and their meanings: `docs/concepts/reports.md`.
- PnL accounting and the authority for realized and unrealized PnL:
  `docs/concepts/accounting.md`.
- How positions aggregate fills and archive closed cycles: `docs/concepts/positions.md`.
- How the portfolio values positions and converts currencies: `docs/concepts/portfolio.md`.

Continue to [07-risks-and-limits.md](07-risks-and-limits.md).

Previous: [05-build-the-strategy.md](05-build-the-strategy.md) | Next: [07-risks-and-limits.md](07-risks-and-limits.md)
