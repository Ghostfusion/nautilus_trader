# Order types: the promises a broker makes and the risk you keep

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                    |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | One share or a convertible-bond index on daily bars, small sizes; the point is the mechanics of the order, not the market                                                                                                |
| How often it trades       | Depends on the rule: the bracket made eight round trips in two years, the trailing-stop test twelve, the stop-order test over two hundred orders                                                                         |
| What you need             | Nothing but this page, plus a spreadsheet if you want to follow the arithmetic                                                                                                                                           |
| Where the rules come from | [Strategy Compendium, article 25, order_types](https://backtrader.readthedocs.io/en/latest/strategies-series/en/25-order-types.html)                                                                                     |
| The underlying research   | None directly: these are the interfaces of the backtrader order system, and the measured behaviour of real orders is in [Market Impact and Trading Cost](../../../strategies/books/02_market_impact_and_trading_cost.md) |
| How well it held up       | Weak: the order mechanics are well measured, but this category makes no claim that any of its strategies earns anything, and one test named for a trailing stop does not actually use one                                |
| Also appears in           | [Foundations: orders and how they execute](../../foundations/03_orders-and-how-they-execute.md) and [Execution at the price you get](../../project/execution-at-the-price-you-get/README.md) in this collection          |

## The idea in one paragraph

A strategy decides whether to buy or sell. An order type decides how. The difference matters more
than beginners expect, because each kind of order is a promise the broker makes and a risk it leaves
with you. A market order promises speed and gives up the price. A limit order promises the price and
gives up the certainty of trading. A stop order promises to act once a level is touched, and leaves
you the gap between the trigger and the price you actually get. A bracket order packs an entry, a
stop and a target into one unit, so the stop exists from the instant the entry fills. A
one-cancels-other group places several possible trades and lets the market choose one. This category
is six small backtests, one for each promise, and its value is the tour of the ordering system rather
than any claim about returns.

## Why anyone believed it

Nobody trades order types to make money; they trade them to control what happens after a decision.
The belief behind the category is narrower and correct: the exact way an order is placed can matter
more to the final result than the signal that produced it. A perfect entry that is left without a
stop, because the person who placed it went to lunch, is a real and common failure, and the bracket
order exists to make that failure impossible by construction. A trader who wants to buy a dip but
does not know how deep it runs can place several limit orders and let the market decide, rather than
guessing one price.

The counterparty is whoever is on the other side of each promise. When you place a limit order, you
are offering to trade only at your price, and the people who take it are often better informed about
which way the price is about to go, which is why limit orders that fill tend to fill just before the
price moves against you. When you place a stop order, you are telling the market at what price you
will act, and traders who can see the level know where the orders will arrive. Each order type
distributes the risk differently between you and the dealer quoting the other side.

## An everyday comparison

Think of buying a train ticket three ways. A market order is buying the next ticket whatever the
price, so you travel but might pay a lot. A limit order is saying "I will pay at most twenty pounds",
which guarantees the price but not the seat, and the train may leave without you. A stop order is
saying "if the fare falls below twenty pounds, buy me one", which sounds like a bargain until the fare
jumps from twenty-one to eighteen in one move and you pay eighteen, or from twenty-one to thirty
because the cheap tickets sold out first. A bracket is buying the ticket plus insurance plus a
refund rule in one purchase, so you cannot forget the insurance.

## The rules, step by step

Each test in this category demonstrates one order type. Read them as a menu of promises.

The bracket order:

1. Watch two moving averages. When the short one crosses above the long one, prepare an entry.
2. Compute three prices from today's close: the main entry price, 0.5 percent below the close; the
   stop, 2 percent of the close below the entry price; and the target, 2 percent of the close above
   the entry price.
3. Submit all three at once as one group. The main order is a limit buy, so it fills only at the entry
   price or better, and it is valid for three days. The stop and the target are children of the main
   order.
4. The children arm only when the main order fills. If the stop fills, the target is cancelled; if the
   target fills, the stop is cancelled. The group is atomic: you cannot end up with the entry and
   neither protective order.

The one-cancels-other group:

1. On the same moving-average cross, compute three buy prices: 0.5 percent below the close, 2 percent
   below, and 4.5 percent below.
2. Submit all three as limit buys, and link them so that the first to fill cancels the others.
3. Give the nearest order a short life, three days, so that if the dip never comes it expires. Give
   the deeper two a long life, so they wait for a larger fall.
4. Hold whatever fills for ten bars, then close it.

The trailing stop:

1. Enter on a moving-average cross, as before.
2. Set a stop that follows the highest price reached since the entry, always a fixed percentage below
   it. It may only move up, never down.
3. Exit when the price falls to the stop.
4. The test for this category keeps a trailing percentage of 0.02 in its settings but does not
   actually attach it; the strategy it runs is a plain moving-average cross. Rewriting it as a real
   trailing stop is left as the exercise, and it shows how easily a promising name hides a different
   mechanism.

The remaining three are one line each. A plain stop order, used on a convertible-bond index, places a
sell stop 3 percent below the buy price and cancels it before the close if the exit signal comes
first. A target-position order declares the fraction of the account you want and lets the broker
compute the trade needed to get there. A close order fills at the bar's closing price rather than at
the next bar's open, removing a one-bar delay.

## The maths, with every symbol named

The bracket prices come from the close and two percentages:

```text
p1 = close * (1 - limit)
p2 = p1 - s * close
p3 = p1 + s * close
```

- `close` is today's closing price.
- `limit` is 0.005, so the entry waits 0.5 percent below the close.
- `p1` is the entry price, a limit buy.
- `s` is 0.02, the size of the stop and the target as a fraction of the close.
- `p2` is the stop price, a stop sell below the entry.
- `p3` is the target price, a limit sell above the entry.

What it means: the distance from the entry to the stop and from the entry to the target are equal, so
the arrangement is symmetric: a win and a loss are the same size before costs.

The one-cancels-other prices use growing offsets:

```text
p1 = close * (1 - 1 * 1 * limit)
p2 = close * (1 - 2 * 2 * limit)
p3 = close * (1 - 3 * 3 * limit)
```

- `limit` is 0.005, as before.
- The multipliers 1, 4 and 9 are the squares of 1, 2 and 3, so the three prices are 0.5, 2.0 and 4.5
  percent below the close.

What it means: the possible entries get cheaper in steps that grow with depth, so a shallow pullback
is caught quickly and a deep one is worth waiting for.

The trailing stop is a peak with a gap below it:

```text
stop = peak_price * (1 - trailpercent)
```

- `peak_price` is the highest price seen since the entry.
- `trailpercent` is 0.02, the gap as a fraction of the peak.
- `stop` is the level at which the position exits; it only ever rises.

What it means: as the price climbs, the stop climbs with it, so a profit is protected a fixed fraction
below the best level reached. When the price turns down and touches the stop, the position is closed.

## A worked example

First the bracket. Suppose the moving averages cross on a day that closes at 100.00, the limit is
0.5 percent and the stop-and-target size is 2 percent of the close.

| Quantity     | Formula               | Value  |
| ------------ | --------------------- | ------ |
| Close        | given                 | 100.00 |
| Entry price  | 100.00 * (1 - 0.005)  | 99.50  |
| Stop price   | 99.50 - 0.02 * 100.00 | 97.50  |
| Target price | 99.50 + 0.02 * 100.00 | 101.50 |

The entry waits at 99.50 and is valid three days. Suppose the price dips to 99.50 on the second day,
so the entry fills, and both children arm. Two futures are now possible. If the price rises to 101.50,
the target fills and the stop is cancelled: a gain of `101.50 - 99.50 = 2.00` per unit. If instead the
price falls to 97.50, the stop fills and the target is cancelled: a loss of `99.50 - 97.50 = 2.00` per
unit. The two outcomes are the same size, and whichever happens first cancels the other.

Now the costs, on ten units, with commission at 0.1 percent of each side and a spread of 0.02 a unit.
On the winning path:

```text
gross gain = (101.50 - 99.50) * 10 = 20.00
commission = (99.50 + 101.50) * 10 * 0.001 = 2.01
spread     = 0.02 * 10 = 0.20
net gain   = 20.00 - 2.01 - 0.20 = 17.79
```

On the losing path the arithmetic is the same with a negative sign: `-20.00 - 1.97 - 0.20 = -22.17`.
Because the two sides of a round trip cross the spread twice, the break-even move is the cost divided
by the size, `(2.01 + 0.20) / 10 = 0.221` a unit, above the entry. A bracket that wins exactly as
often as it loses still loses money, by the cost of the spread and commission on every round trip.

Second, the one-cancels-other group, with the close at 100.00.

| Quantity | Formula              | Value |
| -------- | -------------------- | ----- |
| Order 1  | 100.00 * (1 - 0.005) | 99.50 |
| Order 2  | 100.00 * (1 - 0.02)  | 98.00 |
| Order 3  | 100.00 * (1 - 0.045) | 95.50 |

Seven periods, showing how the short life of order 1 changes which order can fill:

| Period | Close  | Order 1 | Order 2 | Order 3 | Event                      |
| ------ | ------ | ------- | ------- | ------- | -------------------------- |
| 1      | 100.00 | live    | live    | live    | none                       |
| 2      | 99.80  | live    | live    | live    | none, never touched 99.50  |
| 3      | 99.70  | live    | live    | live    | none                       |
| 4      | 99.60  | expired | live    | live    | order 1 reaches three days |
| 5      | 99.20  | -       | live    | live    | none, above 98.00          |
| 6      | 97.80  | -       | fills   | cancel  | order 2 fills at 98.00     |
| 7      | 96.00  | -       | held    | -       | position held for ten bars |

The subtlety is that all three orders are buys at falling prices, so if the price slides straight down
through 99.50, order 1 fills first and the group is over. Order 2 fills only because order 1 expired
during the three days the price hovered just above it. Without the short life, the deep orders would
rarely be reached at all, which is why the near order is given only three days and the far ones a
thousand.

Third, the trailing stop. Buy at 100.00 with a trailing percentage of 0.02. If the price then climbs
to a peak of 120.00, the stop is `120.00 * (1 - 0.02) = 117.60`, and it never falls below that level.
If the price slips to 117.60, the position closes for a gain of `117.60 - 100.00 = 17.60` a unit. If
the price had only reached 105.00 before turning, the stop would have been `105.00 * 0.98 = 102.90`,
a gain of 2.90. The stop locks in more of the move the further the price runs, and it is the same
monotonic rule that this repository's own engine design writes as
`Stop_t = max(Stop_{t-1}, P_t - k * ATR_t)`.

## What the research actually found

The category article reports six small backtests, and it is candid that the numbers confirm the
mechanics rather than any profit. The bracket, on 2005 to 2006 daily data over 497 bars, made 8 buys
and 8 sells and 8 closed trades, four wins and four losses, a clean 50 percent, and finished at
99,875.56 against a starting 100,000, a reward-to-risk ratio of minus 1.43, and a worst fall of 2.57
percent. The one-cancels-other group finished at 99,936.20 with a reward-to-risk ratio of about minus
728, an extreme number that comes from one-share positions, sparse trades and near-zero annualised
volatility, not from a real disaster; the test's own comment says the numbers confirm the cancellation
mechanism works, not that the strategy profits. The stop-order test placed 211 buys on a
convertible-bond index, of which 106 were stopped out. The trailing-stop test finished at 105,190.30,
a reward-to-risk ratio of 1.19 and a worst fall of 3.26 percent, and it is worth remembering that this
result came from a plain moving-average cross, because the trailing stop in its settings was never
used. The target-position and close-order tests finished at 102,995.50 and the like.

Those numbers are asserted, and the assertion is the point. Every backtest in the compendium asserts
its final portfolio value, its reward-to-risk ratio and its worst fall against a baseline recorded
when the test was migrated. Passing that assertion proves the engine computes exactly what the file
says, in both its computation modes, and nothing more. In this category that is the whole claim: the
tests prove that a bracket arms its children correctly and that a cancellation happens, which is
framework behaviour, not a strategy result. The article says so itself of the one-cancels-other test:
the numbers confirm the mechanism.

What is measured about real orders comes from other work. A study of one electronic order book found
that 99.9 percent of limit orders were eventually cancelled rather than executed, that more than 90
percent of executions happened at the best prices, and that fill probability was negligible more than
one tick away from the best price (`2403.02572v2`, cited in the foundations page). On a live treasury
future, the average price move after a passive fill was about half a tick in the wrong direction, a
cost called adverse selection (`2407.16527v1`). In plain words, the person taking your limit order
often knows something, and you find out after the fill.

## How this project relates to it

This repository has its own design for exactly these mechanics,
[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md). Its Section 5
keeps four prices apart: fair value, entry price, target price and execution price, and it is explicit
that the entry price the engine produces is a limit, meaning "do not pay more than this", not a
forecast of value. Its Section 6 defines the effective buy and sell prices as the signal price plus or
minus half the spread, slippage and market impact, caps an order at a fraction of the day's volume,
and gives the break-even identity `P_break_even_long = P_entry + (C_entry + C_exit) / Q`, which is
the same arithmetic as the bracket example above.

Two tutorials in this collection cover the same ground from the reader's side.
[Foundations: orders and how they execute](../../foundations/03_orders-and-how-they-execute.md) walks
through the bid, the ask, the three common order types and the journey an order takes.
[Execution at the price you get](../../project/execution-at-the-price-you-get/README.md) explains why
the fill is not the price on the screen, in the same words the engine design uses. The engine is
careful to say that an execution model can only make a strategy worse, never better, which is why it
is applied unconditionally while everything else is gated.

## Where it goes wrong

- A bracket's entry may never fill. The main order is a limit, so in a market that rises without
  pulling back, the whole package expires and the move is missed. That is the price of demanding a
  better price.
- A stop is not a guarantee of a price. It promises to send an order when the trigger is touched, not
  to fill at the trigger. In a fast or gappy market the fill can be far away, and the loss larger than
  planned.
- A trailing stop is not free, and one test here does not even use it. The named trailing-stop test
  runs a plain moving-average cross, which shows how a label can mislead; always read which mechanism
  is actually attached.
- Costs decide the outcome. The bracket example loses at break-even because every round trip pays the
  spread and commission twice; a strategy that wins half its trades and pays them still loses.
- Limit orders fill when it is bad for you. The measured adverse selection means a passive fill is a
  weak signal about the next move, which is a real drag on any rule built on resting orders.
- One-cancels-other depth is often unreachable. All the buys are at falling prices, so a straight
  slide fills the nearest and cancels the rest; the deep orders matter only if the near one expires.

## Try it yourself

You need a spreadsheet and no market access.

1. Take a closing price, say 100.00, and write three rows for the bracket: entry, stop and target,
   with the formulas from the worked example.
2. Add columns for a cost of 0.1 percent of each traded amount and a spread of 0.02 a unit.
3. Work out the net result of a winning round trip and a losing round trip, and the break-even move
   needed to cover the costs.
4. Then change the assumed spread to 0.10 a unit and redo both.
5. Finally, imagine an order book where 99.9 percent of limit orders are cancelled and fill is
   negligible more than one tick away, and ask how often the deep orders would fill.

What to notice: with the wider spread, the break-even move grows and the strategy has to win by more
than half its trades just to stay level. The order type does not change the signal; it changes how
much of the signal's value survives the trip to the market.

## Where this came from

- [Strategy Compendium, article 25, order_types](https://backtrader.readthedocs.io/en/latest/strategies-series/en/25-order-types.html),
  the six order types, the deep dives and the reported baselines.
- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Sections 5 and
  6, for the four prices, the effective fill price, the participation cap and the break-even identity.
- [Foundations: orders and how they execute](../../foundations/03_orders-and-how-they-execute.md), for
  the bid, the ask and the measured fill statistics (`2403.02572v2`, `2407.16527v1`).
- [Execution at the price you get](../../project/execution-at-the-price-you-get/README.md), for the
  half-spread, slippage and impact that separate the screen price from the fill.
- [Market Impact and Trading Cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  for the square-root law of impact and the uncertainty in its prefactor.

## Words used in this tutorial

- market order: an instruction to trade immediately at whatever price is available.
- limit order: an instruction to trade only at a stated price or better, which may never fill.
- stop order: an instruction that becomes a market order once a chosen price is touched.
- trailing stop: a stop that follows the price upward and never moves back down.
- bracket order: an entry, a stop and a target submitted together as one unit.
- one-cancels-other: a group of orders in which the first to fill cancels the rest.
- spread: the gap between the best buying price and the best selling price, paid on every round trip.
- adverse selection: the tendency of a resting order to fill just before the price moves against you.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
