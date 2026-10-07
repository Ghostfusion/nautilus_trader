# The misfits: rotation, spreads and strategies that need two price series

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Two Chinese share funds that track large and small companies; two Chinese government-bond futures contracts; and baskets of Chinese convertible bonds, which are bonds that can be turned into shares                                                                                                                                                           |
| How often it trades       | From a few times a year for the bond spread to almost every month for the rotation and the bond baskets                                                                                                                                                                                                                                                         |
| What you need             | A spreadsheet and two aligned tables of daily prices, or one table with a second column for the other instrument                                                                                                                                                                                                                                                |
| Where the rules come from | [Strategy compendium, category 22, special](https://backtrader.readthedocs.io/en/latest/strategies-series/en/22-special.html)                                                                                                                                                                                                                                   |
| The underlying research   | Style rotation and relative strength in the manner of Mebane Faber's [Relative Strength Strategies for Investing](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1585517), and the classic calendar-spread arbitrage argument from futures textbooks; neither is a formal paper in this field                                                              |
| How well it held up       | Mixed: the rotation rule returned 16.19 percent a year in a single-sample backtest with a worst fall of 32 percent, while the spread rule loses money at its own fitted thresholds and the convertible-basket rule loses money despite a small worst fall                                                                                                       |
| Also appears in           | [Asset class momentum](../../quantconnect/asset-class-momentum/README.md) and [momentum and style rotation effect](../../quantconnect/momentum-and-style-rotation-effect/README.md) in this collection for the rotation idea, and [trading the WTI-Brent spread](../../quantconnect/trading-with-wti-brent-spread/README.md) for the two-instrument spread idea |

## The idea in one paragraph

Most strategies watch one price. The seven strategies in this folder watch two or more, and trade
the relationship between them. One compares two large funds that track different kinds of Chinese
company and holds whichever is stronger relative to its own average. One watches two government
bond futures contracts that expire at different times and trades the gap between their prices,
which theory says must close as the nearer one expires. One scores dozens of convertible bonds on
price and on the premium over their conversion value, and buys the cheapest. The shared skill is
not a signal but plumbing: lining up two price series that do not share the same trading calendar,
and making a judgement that depends on both.

## Why anyone believed it

For the two funds, the story is style rotation. Large companies and small companies respond
differently to the same economy, and money moves between them slowly, so the one that has been
strong tends to stay strong for a while. For the two bond contracts, the story is arithmetic. Two
contracts on the same bond deliver nearly the same thing at different dates, so their prices
cannot drift apart without a reason, and the gap has to close by the time the nearer contract
expires. The counterparty is a trader who needs to roll a position and cannot wait for a good
price, or a fund whose mandate forces it to hold one style and not the other. For the convertible
bonds, the story is that a bond with a share option attached is worth at least the bond and at
least the option, so a basket of the cheapest such instruments is paid to wait.

## An everyday comparison

Think of two neighbouring shops selling the same brand of coffee. Their prices cannot drift far
apart, because customers will walk to the cheaper one, and the gap closes whenever someone
notices. That is the spread trade. Now think of two neighbouring shops selling different things,
one selling umbrellas and one selling sunglasses. Their sales depend on the weather, and after a
run of sunny days the sunglasses shop is busier, so you buy a share of the sunglasses shop; that
is the rotation trade. The first comparison is about a gap that must close, the second about a
trend that may continue. The two ideas are in the same folder because both need two price lists.

## The rules, step by step

This folder holds seven strategies. The ones a reader would meet are listed below.

| Strategy                    | What it does                                                                             |
| --------------------------- | ---------------------------------------------------------------------------------------- |
| ETF rotation                | Compares two funds by price divided by their own moving averages, and holds the stronger |
| Treasury calendar spread    | Trades the price gap between two bond futures contracts and rolls to the new active pair |
| Convertible double-low      | Ranks bonds by price and by premium, blends the ranks, and buys the top twenty           |
| Premium-rate crossover      | A moving-average crossover on a single bond's premium over its conversion value          |
| Multi-source moving average | Thirty bonds, each with its own sixty-day average, held long when above it               |
| Fei A'li                    | An intraday breakout of the previous day's high and low on a rebar futures contract      |
| Hans123                     | The same opening-range breakout with a two-hundred-period average as a direction filter  |

The rotation and spread rules share the same shape: compute the relationship between two series,
compare it with a threshold, and switch holdings when the relationship crosses. The bond rules
share a different shape: score every instrument in a list, sort the list, and hold the best.

1. The rotation rule. Compute a twenty-day moving average for each of the two funds. If both
   closes are below their own averages, sell everything and hold cash. Otherwise, divide each
   fund's close by its own average and hold the fund with the larger ratio.
2. Size the rotation position as ninety-five percent of the account value divided by the price of
   the chosen fund, rounded down to a whole number of units. Sell the other fund first if it is
   held.
3. The spread rule. Each day, rank the bond futures contracts by their open interest and keep the
   two most active. Call the one with the nearer expiry the near contract and the other the far
   contract, and record the spread as the near price minus the far price.
4. If the spread is above 0.52, the near contract is rich, so sell it and buy the far contract. If
   the spread is below 0.06, the near contract is cheap, so buy it and sell the far contract. Hold
   one unit of each side.
5. Close the pair when the spread crosses back through the opposite threshold.
6. Roll to the new pair. When the most active contracts change, close the old two legs and reopen
   the same position, in the same direction, in the new pair.
7. The convertible-basket rule. Register each bond's price, its conversion value and its premium
   as extra columns. Rank every bond in the list by price, and separately by premium. Add the two
   ranks and buy the twenty bonds with the smallest total. Rebalance on the last trading day of
   each month.
8. Review the rotation and basket rules monthly and the spread rule daily, and use the same fixed
   size within each rule.

## The maths, with every symbol named

The rotation rule turns each fund into a comparable momentum number.

```text
ratio_i = close_i / MA_i
hold the fund with the larger ratio_i
```

- `close_i` is the latest closing price of fund `i`.
- `MA_i` is the average of fund `i`'s last twenty closes.
- The ratio is above one when the fund is above its own average, so comparing two ratios is the
  same as comparing two funds on the same scale, even when one trades at three dollars and the
  other at two.

The spread rule is one subtraction and two thresholds.

```text
spread = near_close - far_close
open short spread if spread > spread_high
open long spread if spread < spread_low
close when the spread crosses back
```

- `near_close` and `far_close` are the closing prices of the two contracts.
- `spread_high` is 0.52 and `spread_low` is 0.06 in the compendium's version, in the contract's
  own quote units.
- Selling the near contract and buying the far one is a bet that the spread will fall; buying the
  near and selling the far is the opposite bet.

The convertible rule blends two ranks, which puts two different scales on one footing.

```text
score = rank(price) + rank(premium)
buy the twenty bonds with the smallest score
```

- `rank(price)` is the bond's position when the list is sorted by price from cheapest upward.
- `rank(premium)` is its position when sorted by premium from cheapest upward.
- Ranks are used rather than the raw numbers because a price of 100 and a premium of 20 percent are
  not comparable directly; the sum of the two ranks is a score where a small number means cheap on
  both counts.

The cost of a two-instrument trade counts every leg.

```text
cost = n_legs * c
```

- `n_legs` is the number of buys and sells, so a switch from one fund to the other is two legs and
  a spread round trip is four.
- `c` is the cost of one leg as a fraction of the amount traded. The compendium's rotation test
  uses 0.0002, two hundredths of a percent, which is deliberately small.

## A worked example

First, the rotation rule, on invented daily prices. The twenty-day averages are shown as given, so
the comparison can be checked. The big-cap fund trades near 3.00 and the small-cap fund near 2.50.

| Day | Big close | Big average | Big ratio | Small close | Small average | Small ratio | Stronger |
| --- | --------- | ----------- | --------- | ----------- | ------------- | ----------- | -------- |
| 1   | 3.000     | 2.950       | 1.0169    | 2.400       | 2.500         | 0.9600      | big      |
| 2   | 3.020     | 2.955       | 1.0220    | 2.420       | 2.505         | 0.9661      | big      |
| 3   | 3.010     | 2.960       | 1.0169    | 2.520       | 2.510         | 1.0040      | big      |
| 4   | 3.040     | 2.965       | 1.0253    | 2.560       | 2.515         | 1.0179      | big      |
| 5   | 3.050     | 2.970       | 1.0269    | 2.600       | 2.520         | 1.0317      | small    |
| 6   | 3.040     | 2.975       | 1.0218    | 2.640       | 2.525         | 1.0455      | small    |
| 7   | 3.020     | 2.980       | 1.0134    | 2.700       | 2.530         | 1.0672      | small    |
| 8   | 2.950     | 2.985       | 0.9883    | 2.760       | 2.535         | 1.0888      | small    |

On day 1 the big-cap fund is above its average and the small-cap fund is below its own, so the big
one is held, bought at 3.000. On day 4 the small fund has climbed above its average and its ratio
is close to the big fund's but still lower, so nothing changes. On day 5 the small fund's ratio of
1.0317 passes the big fund's 1.0269, so the rule switches. The switch executes at the next day's
open, day 6, selling the big fund at 3.040 and buying the small fund at 2.608. By day 8 the small
fund closes at 2.760.

```text
First leg   = 3.040 / 3.000 - 1 = 0.013333, that is 1.333 percent
Second leg  = 2.760 / 2.608 - 1 = 0.058282, that is 5.828 percent
Combined    = 1.013333 * 1.058282 - 1 = 0.072374, that is 7.237 percent
Switch cost = 2 legs * 0.0002 = 0.0004, that is 0.04 percent
Net gain    = 7.237 - 0.04 = 7.20 percent
```

Second, the spread rule, on invented prices for the two contracts. The near contract is the one
with the nearer expiry.

| Day | Near price | Far price | Spread | Action                        |
| --- | ---------- | --------- | ------ | ----------------------------- |
| 1   | 102.30     | 101.90    | 0.40   | inside the band, wait         |
| 2   | 102.60     | 101.95    | 0.65   | above 0.52, open short spread |
| 3   | 102.80     | 102.15    | 0.65   | hold                          |
| 4   | 103.00     | 102.60    | 0.40   | hold                          |
| 5   | 103.20     | 103.10    | 0.10   | hold                          |
| 6   | 103.10     | 103.05    | 0.05   | below 0.06, close             |

The short spread was opened at a spread of 0.65 and closed at 0.05, a fall of 0.60 quote units per
pair of contracts.

```text
Gross gain per pair = 0.65 - 0.05 = 0.60 quote units
Money gain          = 0.60 * 10,000 = 6,000, using the contract's fixed multiplier
Cost per leg        = 0.0002 * 1,000,000 = 200, and there are four legs, so 800
Net gain per pair   = 6,000 - 800 = 5,200
```

The multiplier of ten thousand and the notional of one million are invented for the arithmetic;
the compendium's own results below use the contract's real specification. Both examples show how
two price series are combined; neither says the relationship predicts anything.

## What the research actually found

| Source                                 | What it measured                                                           | Result                                                                                                                                                 |
| -------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The compendium, ETF rotation           | Two Chinese funds from 2011, 50,000 starting cash, 0.02 percent commission | 2,600 bars, 266 buys and 265 trades, 16.19 percent a year, a worst fall of 32.03 percent, final value 235,146.29                                       |
| The compendium, treasury spread        | 1,990 bars of bond futures, 86 trades                                      | Sharpe ratio -2.24, final value 918,003.89 from one million; the fixed thresholds lose money in the sample                                             |
| The compendium, convertible double-low | 1,300 bars of bond data, 89 trades                                         | Sharpe ratio -2.97, worst fall 4.03 percent, final value not profitable; a low worst fall achieved by hiding in the bond floor while missing the trend |
| The compendium, multi-source average   | 4,434 bars across thirty bonds, 460 trades                                 | Final value 14,535,803 from one million; the figure and the account size have to be read together before it means anything                             |
| The compendium, intraday rebar rules   | 19,801 minute bars, ending flat at 14:55                                   | The naked breakout loses most of the account, while the same breakout with a direction filter loses less than a quarter as much                        |
| Faber, relative strength               | Style and sector momentum, long American samples                           | A relative-strength rule beat holding the index in roughly seventy percent of years; the gain came with large falls in the losing years                |

Read together, the picture is this. Rotation between two styles is a real, published idea with a
long sample behind it, and the compendium's version carries a positive return with a worst fall of
about a third of the account. The spread rule, which rests on an argument that looks like physics,
loses money at its fitted thresholds, because spreads do not revert on demand. The bond basket
loses money with a small worst fall, which is what a defensive-looking rule does in a market that
trends away from it. The two intraday futures rules show the value of one filter: the same
breakout, with a trend filter added, lost less than a quarter as much.

This is the honest heart of this group, and it applies to every file here. Every backtest in the
compendium asserts its final value, its reward-to-risk ratio and its worst fall against a
baseline. Passing that assertion proves the engine computes exactly what the file says. It does
not prove the strategy earns anything. The losing spread baseline passes its assertion just as
surely as the winning rotation does, because the assertion is about arithmetic and not about the
market.

## How this project relates to it

The rotation rule is the same relative-strength idea as two finished tutorials in this collection:
[asset class momentum](../../quantconnect/asset-class-momentum/README.md), which ranks five broad
funds by their twelve-month return and holds the top three, and
[momentum and style rotation effect](../../quantconnect/momentum-and-style-rotation-effect/README.md),
which measures the gain from switching between styles. Those pages carry the published record and
the caveats; this one only adds the two-fund, price-divided-by-average form used by the
compendium.

The two-instrument spread idea appears in
[trading the WTI-Brent spread](../../quantconnect/trading-with-wti-brent-spread/README.md), which
trades the gap between two oil benchmarks and reaches a similar warning: a relationship that
should hold can stay broken for a long time, and costs are paid on every leg. The bond side of the
same problem, and the fact that a futures position must outlive the roll date, is background in
[options and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md).
The rule that a multi-asset model should carry a cross-impact matrix rather than assume each
instrument moves alone comes from the market-impact brief,
[market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
and it applies directly to any two-leg trade.

## Where it goes wrong

- The spread need not close. The theory says two contracts on the same bond converge by expiry,
  but the path can go further against the position before it does, and a strategy with a small
  account cannot wait. The compendium's own loss is the measured form of that risk.
- Fit thresholds on one sample. The spread band of 0.06 to 0.52 was chosen by its author, and the
  same file loses money at those levels, which is the clearest possible sign that they were fitted
  rather than derived.
- The roll is the hard part. A futures position must be moved to a new contract as expiry
  approaches, and the cost of that roll lands on the spread strategy every time. A backtest
  without rollover logic is a toy.
- Two calendars, one clock. The two funds, and the many bonds, do not share the same trading days,
  so a rule that compares them can act on a stale price from one side. The compendium handles this
  by running its logic in the warm-up phase, which is a deliberate choice and not a neutral one.
- Ranks hide magnitudes. The convertible score adds two rankings, so a bond can rank high on price
  and low on premium and still win, even when the raw numbers say it is expensive on balance.
- The basket is small and defensive. A loss of only four percent on the bond basket still means
  holding twenty instruments, paying twenty spreads, and earning less than cash in a rising
  market.
- A big final value can be an artefact of the account size and the leverage. The multi-source
  average result must be read with the starting cash and the multiplier beside it.

## Try it yourself

You need a spreadsheet and two public price series that move together, such as two funds tracking
the same market, or two contracts on the same commodity.

1. Put the two series in adjacent columns, aligning them by date and leaving a blank where one
   series has no price.
2. Add a column for the difference between the two prices.
3. Add a column for the average and the standard deviation of that difference over the last sixty
   rows, so you can see what a large gap looks like for this pair.
4. Flag a row when the difference is more than two standard deviations away from its average.
5. In the next rows, add the change in the difference over the following five, ten and twenty
   days, and average those changes over every flagged row.
6. In a second block, compute each series's price divided by its own twenty-day average, and note
   which series is larger each day.

What to notice: if the gap really closed on demand, the average change after a flag would be
strongly negative. In the pairs you try, it is usually close to zero and sometimes the wrong sign,
which is why the compendium's spread rule loses. The rotation block will show long stretches where
the same fund is stronger, and sharp switches, which is where the cost of switching lands.

## Where this came from

- [Strategy compendium, category 22, special](https://backtrader.readthedocs.io/en/latest/strategies-series/en/22-special.html),
  the rules and the backtest numbers quoted above, including the rotation result, the losing
  spread baseline and the convertible basket.
- Mebane Faber, [Relative Strength Strategies for Investing](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1585517),
  the published relative-strength study behind the rotation idea.
- [Asset class momentum](../../quantconnect/asset-class-momentum/README.md) and
  [momentum and style rotation effect](../../quantconnect/momentum-and-style-rotation-effect/README.md),
  this collection's treatments of the rotation idea.
- [Trading the WTI-Brent spread](../../quantconnect/trading-with-wti-brent-spread/README.md), this
  collection's treatment of a two-instrument spread.
- [Options and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md)
  and [market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  the briefs behind the roll and the multi-leg cost points.

## Words used in this tutorial

- arbitrage: trading two things whose prices must agree, to profit from a temporary disagreement.
- basis: the difference between the price of a futures contract and the price of the thing it
  delivers.
- calendar spread: a position long one expiry of a contract and short another expiry of the same
  contract.
- convertible bond: a bond that the holder may exchange for a fixed number of shares.
- open interest: the number of contracts currently outstanding, used here to find the most active
  one.
- premium: the amount by which a convertible bond's price exceeds its conversion value.
- roll: to close a position in one expiring contract and open the same position in the next one.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
