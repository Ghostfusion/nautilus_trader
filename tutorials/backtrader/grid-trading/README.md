# Grids and martingale: adding to losing positions

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                 |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Spot gold (XAUUSD) on fifteen-minute bars, with leverage of about one hundred times the money deposited                                                                                                               |
| How often it trades       | Very often. One system in the category placed 1,591 trades in about three months, and another 909                                                                                                                     |
| What you need             | Python and a data file for the full backtests; a spreadsheet is enough to follow the arithmetic, which is the part that matters                                                                                       |
| Where the rules come from | [Strategy Compendium, article 18, grid_trading](https://backtrader.readthedocs.io/en/latest/strategies-series/en/18-grid-trading.html)                                                                                |
| The underlying research   | None. These are ports of commercial MetaTrader expert advisors, and the arithmetic of a doubling series is school mathematics, not a published result                                                                 |
| How well it held up       | Weak: the family wins often by construction and hides an unbounded loss in the tail; the sources are commercial expert advisors with no independent test, and the honest content here is the size of what can be lost |
| Also appears in           | [Dollar-cost averaging: buying more as the price falls](../../freqtrade/dca-ladder/README.md) and [Position sizing](../../project/position-sizing/README.md) in this collection                                       |

## The idea in one paragraph

A grid system buys in small pieces as the price falls, then waits for one bounce to sell the whole
pile. Because the later pieces are bought cheaper, the average price paid drops, so a modest recovery
is enough to sell everything at a small profit. A martingale is the same idea with a twist: after a
loss, the next piece is larger, often double, on the theory that one win will recover all the earlier
losses at once. The reason the idea is appealing is real: most of the time it works, because most
markets spend most of their time oscillating rather than falling in one direction. The reason it is
dangerous is equally real: when the fall continues instead of bouncing, the amount you have committed
grows faster than the price falls, and the loss you were avoiding arrives all at once.

## Why anyone believed it

The appeal is a high win rate. A grid closes most of its trades at a small profit, so a backtest of
a few months shows a long series of wins and a smooth upward curve. Anyone who judges a strategy by
how often it wins, rather than by how much it can lose, will find a grid irresistible.

The counterparty story is a version of the one that supports mean reversion: some sellers have
reasons unrelated to value. A fund trimming a position, a trader taking profits, or an index
rebalancing may push a price down and then stop. If the seller is finished and the buyer is patient,
buying into the fall can be buying at a price that will not last. In a genuinely ranging market,
there is someone willing to sell you a dip and buy your bounce, and the grid collects the difference.

The deeper reason people keep doing it is that a loss is easier to bear while it is unrealised. A
grid that has added several times has not yet booked a loss; it is waiting. The waiting feels like
discipline until the day it is not.

## An everyday comparison

Think of a shopper who keeps buying more of a shirt as it goes on sale, convinced it will sell out at
the original price. Each markdown makes the average price paid look better, and if it does bounce
back, the shopper sells the pile on at a small profit. The plan only breaks if the shirt keeps being
marked down and never recovers, in which case the shopper is left holding a pile of shirts worth less
than was paid for all of them, and has spent more money the further the price fell. The shop is not
wrong about the price falling; the shopper's ladder is a bet that the fall stops before the shirts
are worthless.

## The rules, step by step

The averaging grid, as the category's largest test implements it:

1. Watch the price on fifteen-minute bars. Open a first position when the price has pulled back a
   small percentage from the day's extreme and the previous bar was moving in the opposite direction.
2. Place each following add at a fixed distance below the last one, and let that distance grow a
   little with each add, so the deeper the fall the longer the system waits before adding again.
3. Scale the size of each add. In the mild version the size is the base amount multiplied by the
   number of positions already open, so the third add is twice the first. In the doubling version
   each add is twice the one before.
4. Compute the weighed average price of everything held, where each position counts for its size.
5. Sell the whole pile when the price touches that average plus a small margin, or take a fixed small
   profit when only one position is open.
6. Repeat.

The capped martingale, which trades a signal and sizes separately:

1. Use two moving-average convergence-divergence indicators on the midpoint of high and low, one fast
   and one slow, to decide direction. Buy when the fast one turns up from a local low and the slow
   one confirms; sell when the mirror image happens.
2. Every trade carries a fixed stop, 500 points below the entry, and a fixed target, 1,500 points
   above it, where a point is the smallest quoted step.
3. Size the trade from the money in the account, then double the size after each losing trade, but
   only once. After a second loss the size returns to the base.
4. Cap the size at a maximum, and step the size down if the money in the account is not enough to
   cover the margin the position requires.
5. Skip the trade entirely if even the smallest allowed size cannot be afforded.

There is also a deliberate control group in this category: three systems that pick a direction by
flipping a coin, then apply a fixed stop and target. They exist so that any real strategy on the same
data has something to beat.

## The maths, with every symbol named

The grid rests on one calculation, the weighed average cost of everything held:

```text
avg = ( p_1 * q_1 + p_2 * q_2 + ... + p_n * q_n ) / ( q_1 + q_2 + ... + q_n )
```

- `p_1` to `p_n` are the prices paid for each position, oldest first.
- `q_1` to `q_n` are the sizes bought at each price.
- `avg` is the weighed average price of the whole pile.
- The exit target is `avg` plus a margin, or `avg` minus a margin for a short position.

What it means: the more you buy at the lower prices, the more those low prices pull the average down,
so a smaller bounce clears the whole pile. That is the entire mechanism, and it is also where the
danger is, because the pile itself keeps growing.

The martingale rests on a geometric series, and this is the formula a reader must understand before
doing anything else:

```text
size at add n = base * 2^(n - 1)
total size after n adds = base * (2^n - 1)
```

- `base` is the size of the first position.
- `n` is the number of the add, counted from one.
- `2^(n - 1)` means two multiplied by itself `n - 1` times.
- `total size` is everything held after the nth add, if nothing has been sold.

What it means: the required position does not grow steadily, it doubles each step. After ten adds the
tenth position alone is 512 times the first, and the total is 1,023 times the first. This is the
reason the plan can be comfortable for a long time and then fail in a single move.

## A worked example

First, the doubling, with a base of one unit and a price that falls by ten each time, starting at 100.
This table is the heart of the category; read it twice.

| Add | Units added | Total units | Price | Capital added | Capital committed | Average cost |
| --- | ----------- | ----------- | ----- | ------------- | ----------------- | ------------ |
| 1   | 1           | 1           | 100   | 100           | 100               | 100.00       |
| 2   | 2           | 3           | 90    | 180           | 280               | 93.33        |
| 3   | 4           | 7           | 80    | 320           | 600               | 85.71        |
| 4   | 8           | 15          | 70    | 560           | 1,160             | 77.33        |
| 5   | 16          | 31          | 60    | 960           | 2,120             | 68.39        |
| 6   | 32          | 63          | 50    | 1,600         | 3,720             | 59.05        |
| 7   | 64          | 127         | 40    | 2,560         | 6,280             | 49.45        |
| 8   | 128         | 255         | 30    | 3,840         | 10,120            | 39.69        |
| 9   | 256         | 511         | 20    | 5,120         | 15,240            | 29.82        |
| 10  | 512         | 1,023       | 10    | 5,120         | 20,360            | 19.90        |

The arithmetic is checkable. After three adds the total is `1 + 2 + 4 = 7` units at a cost of
`1 * 100 + 2 * 90 + 4 * 80 = 100 + 180 + 320 = 600`, so the average is `600 / 7 = 85.71`. After ten
adds the capital committed is 20,360, more than two hundred times the 100 that started it.

Now read the last two columns together with the price. At add ten the price is 10 and the average cost
is 19.90, so the position needs the price to almost double before it breaks even. And suppose the fall
does not stop there. If the price keeps going to zero, the whole 20,360 is lost, and it is lost after
the account has already committed everything it had. With leverage, the end comes sooner: the broker
demands more margin as the position grows, and when the money runs out it closes the position at the
worst possible moment. The loss is not a small one taken early; it is the account.

For comparison, the mild version used by the averaging grid scales sizes linearly, not by doubling.
With a base of 0.1 lots and gold near 2000: buy 0.1 at 2000.00, add 0.1 at 1990.00, add 0.2 at
1980.00. The weighed average is `(2000.00 * 0.1 + 1990.00 * 0.1 + 1980.00 * 0.2) / 0.4 =
(200.00 + 199.00 + 396.00) / 0.4 = 795.00 / 0.4 = 1987.50`. With a margin of 0.50, the whole pile is
sold at 1988.00, which is far below the first entry of 2000.00 yet still a profit on the average. The
profit is `0.4 * 0.50 = 0.20` before costs, which is the point and the problem: each basket earns a
trifle, so the system needs hundreds of baskets, and each basket commits a growing amount.

## What the research actually found

The category contains one of the rare things in this compendium: nine strategies that run on the same
instrument and the same window, gold fifteen-minute bars from December 2025 to March 2026, with
1,000,000 of starting money, no commission, and a leverage multiplier of one hundred. The article's
own numbers, taken from the tests:

| System       | Trades | Win rate | Final value  | Worst fall |
| ------------ | ------ | -------- | ------------ | ---------- |
| VR-SETKA-3   | 1,591  | 67.94%   | 1,077,029.70 | 18.70%     |
| MartGreg     | 687    | 35.66%   | 1,032,971.20 | 5.14%      |
| Random robot | 909    | 66.23%   | 1,005,472.40 | 0.56%      |

The averaging grid made the most money, 7.70 percent in three months, and also had by far the worst
fall, 18.70 percent, with no extreme one-directional move in that window. The capped martingale made
3.30 percent with a worst fall of 5.14 percent, and note its win rate: only 35.66 percent, because
its wide stop and tight target mean it loses often and wins big, the opposite of the grid's profile.
The random robot, which flips a coin for direction, finished at 1,005,472.40, up 0.55 percent with a
worst fall of 0.56 percent. That is the number to hold on to: on this data, a coin toss with an
asymmetric stop and target beat the elaborate martingale on a risk-adjusted basis, which is precisely
why the random systems are in the category as controls.

Two more of the category's systems show how the family is varied. The `test_0002` system waits for the
price to stray 240 points from a ten-bar extreme, then places doubling limit orders every 35 points,
and cashes the whole basket at 40 dollars of floating profit. The `test_0004` system is the one above.
Both are ports of real commercial expert advisors.

One habit applies to every number above. Every backtest in the compendium asserts its final portfolio
value, its reward-to-risk ratio and its worst fall against a baseline recorded when the test was
migrated; the grid ports pin twenty or more metrics each. Passing that assertion proves the engine
computes exactly what the file says, in both its modes, and nothing more. In this category the
distinction matters more than anywhere else: the assertions fix the size of a tail that the rule
itself cannot bound, and a green test says the loss was measured, not that it is safe.

## How this project relates to it

The idea of buying more as the price falls appears in this collection as
[Dollar-cost averaging](../../freqtrade/dca-ladder/README.md), which follows the same ladder on a
cryptocurrency pair and reaches the same conclusion: the ladder has no published test at all, and the
closest academic evidence finds that investing the whole amount at once beat spreading it out about
two thirds of the time.

The sizing question is covered by [Position sizing](../../project/position-sizing/README.md), which
explains why a single oversized loss is harder to recover from than the same loss split across
positions, and points at this repository's own engine design,
[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md). That design's
Section 8 rejects grid and martingale machinery, and its Section 7 caps the size of any position by
the risk budget and the distance to the stop, a discipline aimed exactly at the unbounded escalation
this category is built on.

The wider research is in this repository's brief,
[Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md).
Its most relevant finding is blunt: on a finite-horizon binary payoff, any leverage above one leaves
a terminal shortfall if the position is carried to the adverse outcome, independent of the path the
price took to get there (`2605.10400v2`, p.7). A martingale is a machine for increasing leverage as
the adverse outcome approaches.

## Where it goes wrong

- The win rate is a trap. A grid wins most of the time by design, because it refuses to close a
  losing basket. The failures are rare, large and clustered, which is the opposite of what a win rate
  measures.
- The required position grows geometrically. After ten doubling adds the position is 512 times the
  first, so a market that trends one way does not produce a bad trade, it produces the end of the
  account. The fall continuing past the last affordable add is the whole risk, and it is not rare
  enough to ignore.
- Margin calls arrive at the worst moment. With leverage, the broker closes the position when the
  money runs out, which is by construction near the bottom. The rule does not get to wait for the
  bounce it was built for.
- The tests are three months long and charge no commission. A window with no extreme trend flatters
  the family enormously, and a system that trades 1,591 times in three months would pay a large
  spread in a real account.
- The only reference the family has is commercial expert advisors. There is no independent, published,
  cost-inclusive test showing the approach survives a full market cycle, and the arithmetic of the
  doubling series is the same one that has destroyed accounts for as long as leverage has existed.
- The cap is the honest part. The capped martingale limits the doubling to one step and caps the size,
  and that is why its worst fall was 5.14 percent instead of the grid's 18.70. The cap is doing the
  risk control, not the strategy.

## Try it yourself

You need a spreadsheet and nothing else; this exercise does not touch a market.

1. In one column write the add number, 1 to 12. In the next write the units added, starting at 1 and
   doubling each row.
2. In a third column write a price that falls by 10 each row, starting at 100.
3. Multiply the units column by the price column to get the capital added, and keep a running total of
   capital committed.
4. Divide the running capital by the running units to get the average cost.
5. Write a formula that, for each row, says what the price would have to be for the whole pile to
   break even, and how far that is above the current price as a percentage.

What to notice: the break-even price falls more and more slowly as the adds get larger, while the
capital committed grows faster and faster. By the last rows, a small extra fall requires a large extra
commitment, and the required bounce, though smaller in price terms, applies to a pile that is now many
times the original.

## Where this came from

- [Strategy Compendium, article 18, grid_trading](https://backtrader.readthedocs.io/en/latest/strategies-series/en/18-grid-trading.html),
  the category inventory, the three deep dives, the same-data comparison and the reported baselines.
- [Dollar-cost averaging: buying more as the price falls](../../freqtrade/dca-ladder/README.md), this
  collection's tutorial on the same ladder idea.
- [Position sizing](../../project/position-sizing/README.md), for why an oversized single loss is
  harder to recover from than several smaller ones.
- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Sections 7 and
  8, for the risk-budget size cap and the rejection of grid and martingale machinery.
- [Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md),
  for the leverage result (`2605.10400v2`) and the fragility of tail estimates on short samples.

## Words used in this tutorial

- grid: a system that buys in several pieces at lower and lower prices.
- martingale: a system that doubles the size after a loss to recover it in one win.
- averaging down: buying more of something that has fallen, to lower the average price paid.
- leverage: using borrowed money so a given price move produces a larger gain or loss.
- margin: the money a broker requires you to set aside as a deposit when you borrow.
- margin call: a demand for more money, after which the broker may close the position.
- stop: an instruction to close a position once a chosen price is reached.
- drawdown: the fall from a peak in account value to a later low.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
