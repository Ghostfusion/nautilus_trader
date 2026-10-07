# TWAP: cutting one order into equal slices spread over time

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                      |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto pairs on a centralised exchange, bought for a rise or sold short for a fall                                                                                                                         |
| How often it trades       | A signal opens a trade, then the order is filled in ten slices roughly one minute apart                                                                                                                    |
| What you need             | Nothing but this page; the slicing is a rule about how to fill an order, not about what to buy                                                                                                             |
| Where the rules come from | [TWAPStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/TWAPStrategy.py)                                                                                        |
| The underlying research   | None for the rule itself; time-weighted average price execution is a standard industry method, described in [this repository's execution brief](../../../strategies/books/01_execution_and_liquidation.md) |
| How well it held up       | Weak: the file is a code example whose entry and exit rules are marked as placeholders, and no backtest result is published for it                                                                         |
| Also appears in           | [Almgren-Chriss execution](../almgren-chriss-execution/README.md), which generalises it, and [Orders and how they execute](../../foundations/03_orders-and-how-they-execute.md)                            |

## The idea in one paragraph

You have decided to buy a thousand coins, and you could send one order for the whole thousand. That
one order would eat through the exchange's list of sellers and push the price up as it filled, so the
average price you paid would be worse than the price you saw. Instead the rule splits the order into
ten equal pieces and sends one piece about every minute. The average price paid across the ten pieces
is close to the average of the prices that were on the screen during those minutes, which is where the
name comes from: the order is weighted by time rather than sent all at once. The same slicing is used
to sell the position when it is time to leave.

## Why anyone believed it

An exchange is not a warehouse; it is a queue of people who have said what price they will accept. If
you ask for a thousand coins at once, you take every price on offer until you have your coins, so the
last coins you buy are bought at prices pushed up by your own earlier coins. That is market impact:
your own order moves the price against you. Someone on the other side profits from the move, which is
exactly why they were resting there.

Standing aside for a minute between slices lets new sellers arrive and refill the queue, so the next
slice is again taken near the price on the screen. The belief is that no single slice is big enough to
be noticed, and that over ten slices the friction is much smaller than the friction of one large
order. The counterparty is the ordinary flow of buyers and sellers going about their day, not a
single opponent who is deliberately taking the other side of you.

## An everyday comparison

Think of a commuter buying groceries for a large party from a small corner shop. Buying every item in
one visit would clear the shelves and force the shopkeeper to raise prices on the last few items. The
patient customer instead returns once an hour and buys the same small basket each time, letting the
shop restock in between, and ends up paying close to the ordinary shelf price. The strategy here is
the patient customer. The catch, as the next sections show, is that a shopkeeper who notices the
pattern can raise the price just before each visit.

## The rules, step by step

1. Work on a fifteen-minute timeframe, so each candle covers fifteen minutes.
2. Use the relative strength index, a number from 0 to 100 that compares the size of recent upward
   moves with recent downward moves. As the file does, buy when it is below 45 and the candle has
   traded some volume, and sell short, meaning sell first and aim to buy back lower, when it is
   above 55 with volume.
3. When a trade opens, compute the intended total size and divide it by ten. This first slice, and
   only this slice, is sent immediately.
4. Wait until one minute has passed since the last filled slice. Then send the next slice, which is
   the amount still outstanding divided by the number of slices still to come.
5. Keep sending slices one minute apart until ten slices have filled, or until the remaining amount
   falls below the exchange's smallest allowed order.
6. To leave, do not look for an exit signal; instead begin selling in slices when the index has
   crossed back above 55 after a long, or below 45 after a short. Sell one tenth of the position per
   minute, again ten slices in total.
7. The stop loss is a fixed minus ten percent, and the minimal return target is a flat two percent
   from the moment the trade opens. Trailing stops are not used.

The exact settings in the file are: timeframe `15m`, stop loss `-0.10`, minimal return
`{"0": 0.02}`, short selling allowed, position adjustment enabled, `twap_num_slices` of 10, and
`twap_interval_minutes` of 1. The entry rules are the two index-and-volume conditions above; the
exit trend itself returns nothing, so the partial-exit slices do all the closing.

## The maths, with every symbol named

The rule is arithmetic about an average. If the order is cut into `N` equal pieces and the piece
filled at time `i` gets price `P_i`, then the average price achieved is:

```text
TWAP = (P_1 + P_2 + ... + P_N) / N
```

- `TWAP` is the time-weighted average price, the average of the prices at which the slices filled.
- `N` is the number of slices, ten in this file.
- `P_i` is the price obtained on slice `i`.

Because every slice has the same size, the average of the prices is also the average price paid for
the whole order. The total amount spent is the sum of the slice values:

```text
Cost = Q_1 * P_1 + Q_2 * P_2 + ... + Q_N * P_N
```

- `Cost` is the total cash paid for the order.
- `Q_i` is the number of coins bought in slice `i`, which is the total order size divided by `N`.
- `P_i` is the price of that slice.

The volume-weighted version of the same idea replaces the equal weights with weights by traded
volume:

```text
VWAP = (Q_1 * P_1 + ... + Q_N * P_N) / (Q_1 + ... + Q_N)
```

- `VWAP` is the volume-weighted average price, where slices with more coins count for more.
- `Q_i` and `P_i` are the size and price of slice `i`.

The difference between the two is that `TWAP` gives every minute equal importance while `VWAP` gives
busy minutes more. The cost that slicing is meant to reduce is market impact, which in measured data
grows with the size of the order less than proportionally: on a survey of stocks, the exponent of
the size-to-impact relation is about 0.5, meaning a tenfold larger order costs about three times as
much rather than ten times (`2411.13965v3`). Splitting therefore does reduce impact, but the size of
surviving impact depends on the market, not on the number of slices.

## A worked example

A made-up pair, and an order for 1,000 coins split into ten slices of 100 coins, one slice per
minute. The exchange charges 0.10 percent of the amount traded on each side.

| Slice | Minute | Price  | Coins | Value      |
| ----- | ------ | ------ | ----- | ---------- |
| 1     | 1      | 100.00 | 100   | 10,000.00  |
| 2     | 2      | 100.20 | 100   | 10,020.00  |
| 3     | 3      | 100.10 | 100   | 10,010.00  |
| 4     | 4      | 99.90  | 100   | 9,990.00   |
| 5     | 5      | 99.80  | 100   | 9,980.00   |
| 6     | 6      | 100.00 | 100   | 10,000.00  |
| 7     | 7      | 100.30 | 100   | 10,030.00  |
| 8     | 8      | 100.50 | 100   | 10,050.00  |
| 9     | 9      | 100.40 | 100   | 10,040.00  |
| 10    | 10     | 100.20 | 100   | 10,020.00  |
| Total |        |        | 1,000 | 100,140.00 |

The ten prices add up to 1,001.40, so:

```text
TWAP = 1,001.40 / 10 = 100.14
Buy fee = 100,140.00 * 0.001 = 100.14
Total paid = 100,140.00 + 100.14 = 100,240.14
```

So the average price is 100.14 and the buy fee is 100.14. Now suppose the same thousand coins were
sent as one market order and the sweep filled at an average of 100.60, because the order took the
higher prices resting above 100.20. That single order would have cost 100,600.00, which is 460.00
more than the sliced order. The fee would have been 100.60 instead of 100.14, since fees are
proportional to the amount traded. On this made-up path the split saved roughly 460 units of
currency, and the saving is exactly the impact the slices avoided.

The number depends entirely on the path chosen. If the price had risen steadily to 101.00 by the end,
the sliced buyer would have paid an average of about 100.50 while a single order at the start would
have paid near 100.05. TWAP aims at the average, not at the best price, and a trader who expects the
price to rise should prefer to trade sooner.

## What the research actually found

The execution literature in this repository does not test this file. It tests the ideas the file is
built from, and it is worth separating them.

- Time slicing is the risk-neutral case of a larger rule. The reference solution in the
  Almgren-Chriss framework front-loads the trade, and when the trader is assumed not to care about
  risk the schedule collapses exactly to TWAP (`1204.2717v4`, as reported in
  [the execution brief](../../../strategies/books/01_execution_and_liquidation.md)). So TWAP is what
  you get when you refuse to guess the direction.
- The volume-weighted version is the provably optimal static schedule for a risk-neutral trader when
  the cost of impact depends on traded volume (`1408.6118v4`, same brief). A fixed time schedule that
  ignores volume is therefore leaving part of the improvement untaken, which the file accepts by
  construction.
- Reacting to conditions pays only in some regimes. Under a volume process that mean-reverts, the
  adaptive optimum equals the expected volume-weighted schedule, so there is nothing extra to gain
  (`1701.08972v2`, same brief).
- Order splitting is visible in the data and is large. On nine years of exchange account data,
  splitting traders are about 25 percent of accounts but submit about 80 percent of market orders,
  and the practice is the measured origin of the long memory in order flow (`2308.01112v1`, reported
  in [this repository's other execution brief](../../../strategies/books2/25_execution_impact_and_order_book.md)).
  That result is descriptive, not a performance claim, but it shows that a sliced order is a feature
  other participants can measure.

None of these papers measures the entry and exit rules in `TWAPStrategy.py`; the file itself says
those are examples to be adapted. The evidence covers the slicing, not the strategy around it.

## How this project relates to it

This repository does not run a TWAP order. It studies execution instead.
[The execution and liquidation brief](../../../strategies/books/01_execution_and_liquidation.md) is
the index of the optimal-execution literature and states the closed form that TWAP is the limit of
[The market impact brief](../../../strategies/books/02_market_impact_and_trading_cost.md) collects the
measurements of how much an order of a given size moves a price, which is the quantity the slice size
is trying to keep small, and
[the order-flow and impact brief](../../../strategies/books2/25_execution_impact_and_order_book.md)
shows that splitting is measurable from the outside. [The foundations page on orders and
execution](../../foundations/03_orders-and-how-they-execute.md) defines the order types and the
spread that the slicing is meant to avoid paying.

## Where it goes wrong

- The schedule is predictable. Ten equal pieces one minute apart produce a recognisable pattern in
  the order flow. Another trader who detects it can buy just before each slice and sell into it,
  taking the price improvement the slicer was trying to earn. The research brief on execution treats
  this predatory trading as a game with known equilibria, not a rare event.
- Slicing is a timing bet even when you do not mean it to be. Stretching a buy over ten minutes is a
  decision to trade later, and if the price rises in the meantime the average is worse than a single
  order at the start would have got.
- It ignores volume. A fixed slice taken in a quiet minute moves the price more than the same slice in
  a busy one, which is exactly what the volume-weighted result says a schedule should account for.
- The signal and the fill are separated. The index condition is judged when the trade opens, but the
  position is not fully owned until ten minutes later; if the move the signal detected was short
  lived, part of it is paid away before the position exists.
- Small orders lose more than they save. Every extra slice is an extra order, and venues that charge
  per order, or refuse orders below a minimum size, turn the last slices into cost or nothing at all.
  The file itself skips a slice that is smaller than the exchange minimum.
- The stop and the slices interact. A ten percent stop can be reached while the position is only
  partly built, so the loss is taken on the slices already filled and the rest of the plan is
  abandoned.

## Try it yourself

You need a spreadsheet and one listing of recent prices for a single pair, with a row per minute.

1. Note a starting reference price, call it `P0`, and write it at the top.
2. Build columns `minute`, `price` and a `slice value` equal to `price * 100`, representing a hundred
   coins bought each minute for ten minutes.
3. Sum the ten prices and divide by ten. That is the TWAP. Compare it with `P0`.
4. Sum the ten slice values and add a fee of 0.10 percent of that sum.
5. Now add a column `single order` that assumes the same thousand coins were bought at `P0` multiplied
   by 1.004, a made-up four-tenths of a percent of extra impact for the larger order, and add the same
   fee.
6. Subtract the two totals.

What to notice: on some days the sliced result is better and on others the single order is better, and
the winner is decided by which way the price drifted during the ten minutes, not by the slicing
itself. Then ask the practical question: could someone watching the same prices have guessed the next
slice and traded ahead of it?

## Where this came from

- [TWAPStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/TWAPStrategy.py),
  the settings, the entry conditions and the slice arithmetic used above.
- [The execution and liquidation brief](../../../strategies/books/01_execution_and_liquidation.md),
  this repository's collection of the optimal-execution results, including the papers `1204.2717v4`,
  `1408.6118v4` and `1701.08972v2` quoted above.
- [The market impact brief](../../../strategies/books/02_market_impact_and_trading_cost.md), the
  source, via `2411.13965v3`, of the roughly 0.5 exponent used in the maths section.
- [The order-flow and impact brief](../../../strategies/books2/25_execution_impact_and_order_book.md),
  the source, via `2308.01112v1`, of the measurement that splitting traders submit about 80 percent
  of market orders.

## Words used in this tutorial

- market impact: the amount your own order moves the price against you as it fills.
- market order: an instruction to trade immediately at whatever price is available.
- relative strength index: a number from 0 to 100 comparing the size of recent up moves with down
  moves.
- short: selling something you do not own first, so that you gain if the price falls.
- slice: one of the smaller orders that together make up the whole order.
- TWAP: time-weighted average price, the average price achieved when equal slices are spread evenly
  over time.
- VWAP: volume-weighted average price, the average price where busier minutes count for more.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
