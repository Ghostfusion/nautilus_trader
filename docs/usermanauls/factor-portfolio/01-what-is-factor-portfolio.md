# 01 - What a factor and a portfolio are

This lecture has no engine code. It builds the words and the arithmetic you will use for the rest of
the manual. Read it once, slowly. Everything later is these ideas with more precision.

## Prices, returns, and why you rank

A **price** is the amount of money one unit of something costs right now. If a share of a company
costs 100.00 today and 102.00 tomorrow, its **return** over that day is the change divided by the
start:

```text
return = 102.00 / 100.00 - 1 = 0.02 = 2 percent
```

That is a **simple return**. It is the smallest useful piece of market data.

If you have one hundred instruments, you cannot watch all of them equally. You want a rule that
puts them in an order, so that the ones near the top deserve more of your money and the ones near
the bottom deserve less. That rule is a **factor**.

> A factor is a number you compute for every instrument from data you already have, arranged so
> that a higher number means "more attractive by this one idea".

A factor is not a prediction. It is a measured characteristic. "How much did the price rise over
the last five days" is a factor. "Is this company profitable" is a factor. "How much does the price
usually move in a day" is a factor. Each one gives every instrument a score, and the scores sort.

The idea behind factor investing is old and simple: some measured characteristics have historically
been followed by better returns than others, on average, across many instruments. Momentum (things
that have risen keep rising for a while) and value (cheap things beat expensive things over long
periods) are the two famous ones. You do not have to believe any particular story to use the
machinery; you have to test it.

## What a portfolio is

A **portfolio** is the collection of positions you hold. A **position** is an amount of one
instrument: positive if you own it (long), negative if you owe it (short). Your **equity** is the
cash plus the current value of everything you hold.

A portfolio can be described by **weights**. A weight is the fraction of your equity that one
position represents:

```text
weight of A = value of the A position / total equity
```

Weights are the natural language of factor portfolios, because a factor gives you a *relative*
opinion, not a dollar amount. "Hold twice as much A as B" is a statement about weights.

Two weight schemes appear constantly in this manual:

- **Equal weight**: every instrument you like gets the same fraction. If you like two instrument
  out of four, each gets 0.5 and the other two get 0.0.
- **Rank weight**: the weight is proportional to the instrument's rank, so the best gets the most
  and the worst gets the least (often a negative weight, meaning you short it). A rank-weighted
  book that adds up (in absolute value) to 1.0 and sums to 0.0 is called **dollar-neutral**: the
  longs and the shorts are the same size, so a market-wide move does not push you one way or the
  other.

**Rebalancing** is the act of trading back to the target weights after prices have moved you away
from them. A factor portfolio usually rebalances on a schedule (every day, every week, every
month). Each rebalance costs money in **commission** (the fee the venue charges) and **slippage**
(the difference between the price you expected and the price you got). Rebalancing too often is one
of the most common ways a beginner loses money.

## Where the profit comes from

A factor portfolio makes money when the factor's ranking separates future winners from future
losers, and the winners you hold earn more than the losers you short lose, after costs. Nothing
else. It does *not* make money from clever order placement, from leverage, or from holding a rising
market: a dollar-neutral book has no market exposure by construction.

The hard part is that a factor can look good for the wrong reasons:

- **Noise.** With four instruments and thirty days, a factor's ranking is mostly luck. You need
  many instruments and many independent periods before a result means anything.
- **Look-ahead.** If the factor used a number that was not known at the decision instant, the
  result is fiction. This is the single most common beginner mistake, and lecture 05 shows it.
- **Survivorship.** If your history only contains instruments that still exist today, it silently
  excludes the ones that failed, and fails flatter you.
- **Multiple testing.** If you try five hundred factors and report the best one, you have reported
  a maximum, and a maximum of random numbers is positive even when none has skill. Lecture 06
  gives the correction.
- **Costs.** Once you trade daily, commission and slippage can eat a small factor edge entirely.
- **Crowding.** If everyone runs the same factor, the edge disappears and the exits get crowded.

## The vocabulary

| Term           | One-sentence meaning                                                            |
| -------------- | ------------------------------------------------------------------------------- |
| Instrument     | One thing you can trade; identified by a symbol and a venue, such as `AAA.SIM`. |
| Venue          | The place the instrument trades; `SIM` here is a simulated venue.               |
| Price          | The money one unit costs now.                                                   |
| Return         | The change in price over a period, divided by the price at the start.           |
| Factor         | A number computed for every instrument that ranks them by one idea.             |
| Score          | The factor's value for one instrument at one instant.                           |
| Signal         | A statement of view (long, short, or flat) about one instrument.                |
| Target         | The exposure you want in one instrument, in quantity, weight, or notional.      |
| Order          | The instruction that actually reaches a venue.                                  |
| Portfolio      | The collection of positions you hold.                                           |
| Position       | A signed amount of one instrument; positive long, negative short.               |
| Weight         | One position's value divided by total equity.                                   |
| Equity         | Cash plus the current value of your positions.                                  |
| Long           | Owning the instrument; you gain when its price rises.                           |
| Short          | Owing the instrument; you gain when its price falls.                            |
| Dollar-neutral | Longs and shorts are equal in size, so market direction does not move the book. |
| Rebalance      | Trade back to the target weights.                                               |
| Commission     | The fee a venue charges per trade.                                              |
| Slippage       | The gap between the price you expected and the price you got.                   |
| Universe       | The set of instruments eligible at a given instant.                             |
| Point-in-time  | The state of the world as it truly was at a past instant, not as it is now.     |
| Label          | A future outcome you measure, used to judge whether a factor worked.            |
| Sharpe ratio   | Return per unit of risk taken; the standard single-number score for a strategy. |

## Three worked examples

All three use four made-up instruments, `A`, `B`, `C` and `D`, and one made-up factor. The factor
scores are:

| Instrument | Factor score |
| ---------- | ------------ |
| A          | +5.0         |
| B          | +3.0         |
| C          | -1.0         |
| D          | -4.0         |

### Example 1: equal weight, long the top two

Sort by score, best first: A, B, C, D. Take the top two, A and B, and give each the same weight.
The other two get zero.

```text
weight(A) = 0.5
weight(B) = 0.5
weight(C) = 0.0
weight(D) = 0.0
```

The weights sum to 1.0. This is a long-only book: you own A and B and own nothing else. If the
whole market rises, you gain.

### Example 2: rank-weight, dollar-neutral

Give ranks by score, best = 4 and worst = 1: A = 4, B = 3, C = 2, D = 1. Subtract the average rank,
2.5, so the ranks become +1.5, +0.5, -0.5, -1.5, then divide by the sum of their absolute values,
4.0:

```text
weight(A) = +1.5 / 4.0 = +0.375
weight(B) = +0.5 / 4.0 = +0.125
weight(C) = -0.5 / 4.0 = -0.125
weight(D) = -1.5 / 4.0 = -0.375
```

The weights sum to 0.0, so the book is dollar-neutral, and their absolute values sum to 1.0, so the
total amount traded is the same as example 1. The best instrument is held three times as heavily as
the second-best, and the worst is shorted three times as heavily as the second-worst. That is what
"rank weight" means.

### Example 3: one day of profit and loss

Suppose the next day's returns are A +0.5%, B -1.0%, C +2.0%, D -3.0%. Multiplying each weight by
its return and adding gives the portfolio's return for that day:

```text
+0.375 * (+0.005) = +0.001875   (A, long)
+0.125 * (-0.010) = -0.001250   (B, long)
-0.125 * (+0.020) = -0.002500   (C, short, so a rise is a loss)
-0.375 * (-0.030) = +0.011250   (D, short, so a fall is a gain)
                              ----------
portfolio return   = +0.009375 = +0.9375 percent
```

You made money even though two of the four positions lost. The short in D, the worst-ranked
instrument, was the biggest winner. That is the shape of a working factor portfolio: you do not
need to be right on everything, you need the ranking to be right on average.

Here is the same arithmetic as a program, so you can check it. Save it as `lecture01.py` and run
it with the command in [03](03-first-run.md):

```python
scores = {"A": 5.0, "B": 3.0, "C": -1.0, "D": -4.0}
next_day_returns = {"A": 0.005, "B": -0.010, "C": 0.020, "D": -0.030}

order = sorted(scores, key=scores.get, reverse=True)
top = order[:2]
equal = {name: (1.0 / len(top) if name in top else 0.0) for name in scores}
print("example 1: equal weight, long the top two")
print("  long:", top, "weights:", equal)

mid = (len(scores) + 1) / 2
raw = {name: rank + 1 - mid for rank, name in enumerate(reversed(order))}
gross = sum(abs(value) for value in raw.values())
ranked = {name: raw[name] / gross for name in scores}
print("example 2: rank-weighted, dollar-neutral")
for name in sorted(scores):
    print(f"  {name}: score {scores[name]:+.1f}  weight {ranked[name]:+.4f}")
print("  gross:", sum(abs(w) for w in ranked.values()), "net:", sum(ranked.values()))

pnl = sum(ranked[name] * next_day_returns[name] for name in scores)
print("example 3: one-day PnL of the rank-weighted book")
print(f"  portfolio return = {pnl:+.6f} = {pnl * 100:+.4f} percent")
```

```text
example 1: equal weight, long the top two
  long: ['A', 'B'] weights: {'A': 0.5, 'B': 0.5, 'C': 0.0, 'D': 0.0}
example 2: rank-weighted, dollar-neutral
  A: score +5.0  weight +0.3750
  B: score +3.0  weight +0.1250
  C: score -1.0  weight -0.1250
  D: score -4.0  weight -0.3750
  gross: 1.0 net: 0.0
example 3: one-day PnL of the rank-weighted book
  portfolio return = +0.009375 = +0.9375 percent
```

## What goes wrong, in one paragraph

If the factor used tomorrow's price, example 3 is a lie. If `D` did not exist on the day you ranked,
including it is a lie. If you tried twenty factors and showed only this one, the +0.9375% is the
best of twenty, not an expectation. If the four instruments were chosen because they are the four
that survived, the ranking is flattered. And if commission is 0.02% of each trade, the four trades
here cost about 0.08%, which the 0.9375% of one lucky day would not survive every day. The rest of
this manual is the machinery for telling those cases apart from a real edge.

Next: [02 - How this repository models the style](02-the-engine-view.md).
