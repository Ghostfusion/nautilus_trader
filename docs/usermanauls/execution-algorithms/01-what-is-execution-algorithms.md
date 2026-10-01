# 01 - What is an execution algorithm

This lecture is the primer. It explains, without any code, what an execution algorithm does, why a
large order costs more than a small one, and how to put a number on that cost. Read it before the
other lectures; every later lecture uses the words defined here.

## The problem: a big order moves the price

Imagine a market where a shop is willing to sell apples for 1.00 each, but only 10 apples. Behind
that shop is another seller at 1.01 who has 10 apples, another at 1.02 with 10 apples, and so on.
This ordered list of sellers is the **ask side** of the order book. The best ask is the cheapest
apple, 1.00. The **bid side** is the list of buyers, and the best bid is the highest price someone
will pay.

If you buy 10 apples, you pay 1.00 each. If you buy 30 apples in one go, you clear the 1.00 seller,
then the 1.01 seller, then the 1.02 seller. Your average price is 1.01, not 1.00. The act of buying
more than the market is showing pushed your own price up. That is **market impact**: the price move
you cause yourself.

A **market order** is an instruction to buy or sell immediately at whatever price is available. A
**limit order** is an instruction to buy at no more than a stated price, or sell at no less than a
stated price, and it may wait unfilled. The order book, the market order and the limit order are
defined in [Orders](../../concepts/orders/index.md).

The practical response is to break the order up. An **execution algorithm** receives one order (the
**parent order**) and creates a sequence of smaller orders (the **spawned orders**) over time. This
manual shows the built-in algorithms that do this in NautilusTrader, and how to measure whether the
slicing helped.

## Two kinds of cost

When you compare the price you got with a reference price, the gap is **slippage**. Two references
matter:

- **Arrival price**: the price when you decided to trade, and started working the order. Slippage
  against arrival measures what the market did while you executed, including your own impact.
- **Decision price**: the price when the idea was formed, which may be earlier than arrival. The gap
  between decision and arrival is the **delay cost**; you spent time before you started.

**Implementation shortfall** (also called **implementation shortfall cost**) joins these into one
number: the difference between the price you achieved and the decision (or arrival) price, in money
and in **basis points**. A basis point is one hundredth of one percent, written **bps**. 1 bps of a
100.00 price is 0.01.

The sign convention in this repository is **cost positive**: for a buy, paying above the reference is
positive; for a sell, receiving below the reference is positive. See the function `cost_bps` in
`crates/trading/src/analytics/metrics.rs:250`.

## Worked example 1: implementation shortfall

You decide to buy 1,000 shares. The arrival price is 100.00. You work the order, and your fills
average 100.05.

```
Cost per share        = 100.05 - 100.00        = 0.05
Total cost in money   = 0.05 * 1,000           = 50.00
Cost in basis points  = (0.05 / 100.00) * 10,000 = 5 bps
```

The shortfall is **50.00 of the traded currency, which is 5 bps**. That is the number to report. If
you had bought all 1,000 in one market order you might have paid 100.20, which is 200.00 of cost and
20 bps; slicing turned 20 bps into 5 bps.

## Worked example 2: VWAP by hand

**VWAP** is the volume-weighted average price. You take every trade in a window, multiply its price
by its size, add those products, and divide by the total size. Big trades pull the average toward
their price; small trades barely move it.

Three trades:

| Trade | Price  | Size |
| ----- | ------ | ---- |
| 1     | 100.00 | 100  |
| 2     | 100.50 | 300  |
| 3     | 101.00 | 100  |

```
Value       = 100.00*100 + 100.50*300 + 101.00*100
            = 10,000 + 30,150 + 10,100
            = 50,250
Total size  = 100 + 300 + 100 = 500
VWAP        = 50,250 / 500 = 100.50
```

The **VWAP benchmark** for an execution is the VWAP of the market over the same window. If your fills
averaged 100.60 while the market VWAP was 100.50, your **VWAP slippage** is 0.10, or 10 bps for a buy
(0.10 / 100.50 * 10,000 = 9.95, which rounds to 10 bps).

## Worked example 3: TWAP by hand

**TWAP** is the time-weighted average price. Take the price at regular moments through a window and
average those samples, equally. It ignores size, so it answers "what did the price do over time",
not "what did the market trade at".

Four samples, one every 15 seconds:

```
09:00:00  100.00
09:00:15  100.20
09:00:30  100.40
09:00:45  100.30
TWAP = (100.00 + 100.20 + 100.40 + 100.30) / 4 = 100.225
```

The TWAP algorithm named in this manual takes its name from this idea: it does not weight by
volume, it splits your order into equal slices and sends them at equal intervals. An order of 6,000
units with a 60 second horizon and a 10 second interval becomes six slices of 1,000 units, one every
10 seconds. That is exactly the schedule you will see in
[03-first-run.md](03-first-run.md).

## Why VWAP and TWAP are different benchmarks

A trade you make is part of the market's volume, so a VWAP benchmark is partly your own footprint. A
TWAP benchmark is independent of size and is easy to reproduce from quotes alone. Neither is "the"
truth: VWAP answers whether you beat the crowd, TWAP answers whether you beat the clock. This
repository can compute both, but only from Rust; see
[06-measure-and-evaluate.md](06-measure-and-evaluate.md).

## Where the money comes from

Execution algorithms do not predict prices. Their edge is cost:

- **Less impact**: many small orders let the market absorb size between slices.
- **Less adverse selection**: resting passively means you trade against people who come to you.
- **Measured discipline**: a benchmark turns "I think we did fine" into a number.

The profit being protected is the difference between the decision price and the achieved price. If
your research says a share is worth 101 and you buy at 100.05 instead of 100.20, you kept 0.15 per
share of edge. At 1,000 shares that is 150.00.

## The main risks

- **Timing risk**. Slicing takes time. If the price runs away while you wait, you pay more, and the
  market never comes back. That is the other half of implementation shortfall.
- **Opportunity cost**. A slice that never fills leaves quantity unexecuted, which is a cost too.
- **Churn**. An algorithm that cancels and re-sends orders repeatedly can flood the venue and lose
  queue position. See [07-risks-and-limits.md](07-risks-and-limits.md).
- **Overfitting**. A schedule tuned to one quiet backtest week will underperform in a busy one.

## Vocabulary

| Term                     | Plain meaning                                                                        |
| ------------------------ | ------------------------------------------------------------------------------------ |
| Order book               | The list of resting bids and asks, best prices first.                                |
| Best bid / best ask      | The highest price a buyer shows and the lowest a seller shows.                       |
| Touch                    | The best bid and best ask together.                                                  |
| Spread                   | The best ask minus the best bid; the cost of crossing immediately.                   |
| Market order             | Buy or sell now at whatever price is available.                                      |
| Limit order              | Buy at or below a price, or sell at or above a price; may rest unfilled.             |
| Parent order             | The single order a strategy submits for the algorithm to slice.                      |
| Spawned order            | A child order the algorithm creates from the parent.                                 |
| Horizon                  | The time window the execution should complete within.                                |
| Participation rate       | The fraction of market volume you are willing to be.                                 |
| Price limit              | A price no child may trade outside.                                                  |
| Preference               | A request to be passive (rest) or aggressive (cross).                                |
| Slippage                 | The difference between a reference price and your achieved price.                    |
| Market impact            | The price move your own order causes.                                                |
| Implementation shortfall | Total execution cost measured against the decision or arrival price.                 |
| VWAP                     | Volume-weighted average price over a window.                                         |
| TWAP                     | Time-weighted average price over a window.                                           |
| Basis point (bps)        | One hundredth of a percent; 0.01 on a price of 100.00.                               |
| Adverse selection        | The tendency of your resting orders to fill just before the price moves against you. |

## What you can do with this engine today

- Slice a parent order with the native TWAP algorithm, configured from Python.
- Describe the intended execution with a small policy vocabulary (horizon, price limit, preference,
  and others that individual algorithms either honour or refuse).
- Measure implementation shortfall, VWAP slippage, spread capture and adverse selection, but only
  from the Rust analytics crate. There is no Python surface for those metrics yet.

Read [02-the-engine-view.md](02-the-engine-view.md) next to see how the engine models all of this.
