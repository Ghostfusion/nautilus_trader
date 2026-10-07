# Paired switching: holding whichever of two assets did better last quarter

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                    |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Two investments that tend to move in opposite directions, such as a stock fund and a government bond fund                                                                                                                                                |
| How often it trades       | About once a quarter, and only when the better performer changes                                                                                                                                                                                         |
| What you need             | A spreadsheet and a table of quarterly prices for the two investments                                                                                                                                                                                    |
| Where the rules come from | [QuantConnect strategy library, paired switching](https://www.quantconnect.com/tutorials/strategy-library/paired-switching) and the [Quantpedia entry](https://quantpedia.com/strategies/paired-switching) it cites                                      |
| The underlying research   | Maewal and Bock, [Paired-Switching for Tactical Portfolio Allocation](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1917044)                                                                                                                        |
| How well it held up       | Mixed: the source paper and a later study of hundreds of pairs report gains, but the result depends heavily on which pair is chosen and over which period, and the timing rule can lose money when it is wrong even slightly more often than it is right |
| Also appears in           | The [sector momentum tutorial](../sector-momentum/README.md) in this collection, which generalises switching to ten industries                                                                                                                           |

## The idea in one paragraph

Pick two investments that tend to move in opposite directions, such as shares and government bonds.
Every quarter, look at how each one performed over the previous three months, then put all the money
into whichever did better and hold it for the next quarter. When the next quarter arrives, repeat the
comparison and switch only if the other investment has taken the lead. Because the two move in
opposite directions, simply holding a fixed half in each tends to give the average of the two, so if
the rule for choosing is even slightly better than a coin toss, switching can beat the fixed mix.
This is one of the simplest forms of a family of strategies that rotate between assets.

## Why anyone believed it

If one investment rises when the other falls, then some part of the time each one is on a good run
and the other is on a bad run. A person who holds half in each always gets one good performer and one
bad one, so the fixed mix pulls the result toward the middle. A person who holds only the better one
gives up the middle and takes the better one, provided the rule for spotting the run is accurate.
The paper this strategy rests on argues exactly that: if the switching rule is even minimally
accurate at finding the periods when each asset is doing well, the switched portfolio can earn the
same or more while moving around less.

The counterparty is the investor who cannot or will not switch. A fund with a fixed target of sixty
percent shares and forty percent bonds must rebalance back to that target regardless of what is
happening, which means it sells the winner and buys the loser. A person who chases the asset that
just made the headlines buys late. Against those, an investor who quietly moved to the better of two
assets a quarter ago is taking the other side of a trade that someone else was obliged to make for
reasons other than the outlook.

## An everyday comparison

Think of two bridges across a river on the way to work. Some mornings one is fast and the other is
jammed, and the pattern is partly the reverse of yesterday because the traffic that clogged one
bridge yesterday has moved. A driver who always takes the same bridge gets the average. A driver who
takes whichever bridge was quicker last week does better whenever the pattern repeats, but on the
weeks the jam moves bridges faster than the driver can react, the driver takes the slow one and does
worse. The strategy here is to take whichever route was faster in the last quarter and reconsider
every three months.

## The rules, step by step

1. Choose two investments that have historically moved in opposite directions. The Quantpedia
   example uses one American stock fund and one American government bond fund. The QuantConnect page
   leaves the pair open and the reader supplies it.
2. Decide how to measure performance over the previous quarter: take today's price of each
   investment, divide by its price ninety days ago, and subtract one. Call these the equity return
   and the bond return.
3. If the equity return is higher, hold the equity fund. Otherwise hold the bond fund.
4. Put all the money into the chosen investment. There is no half-and-half holding and no cash
   position in the simple version.
5. Hold for one quarter, that is roughly three months, without trading in between.
6. At the end of the quarter, measure both returns over the quarter just finished and repeat from
   step 3. If the leader has changed, sell the one you hold and buy the other; if not, do nothing.
7. The QuantConnect version computes the ninety-day performance from a history request and sets the
   holding to the whole account, which is the same rule written in code.

## The maths, with every symbol named

Correlation measures whether two things move together or in opposite directions:

```text
correlation = (average of the two returns multiplied together) / (spread of A * spread of B)
```

- `correlation` is a number between minus one and plus one. Plus one means they move identically,
  minus one means they move exactly opposite, and zero means there is no relationship.
- Two investments are negatively correlated when the number is below zero, which is the condition
  this strategy wants.
- `spread of A` and `spread of B` are how much each return wanders around its own average; the
  technical name is standard deviation.

The performance of each investment over the ranking quarter:

```text
R_A = P_today / P_90_days_ago - 1
R_B = P_today / P_90_days_ago - 1
```

- `R_A` and `R_B` are the quarterly returns of the two investments, written as decimals.
- `P_today` and `P_90_days_ago` are the prices of the same investment at the two dates.

The rule for what to hold:

```text
hold A if R_A is greater than R_B
hold B otherwise
```

Then the account's return over the coming quarter is the return of whichever investment was chosen,
less the cost of switching if the choice changed:

```text
R_portfolio = R_chosen
Cost = traded_fraction * cost_per_trade   (only in a quarter where the choice changed)
R_net = R_portfolio - Cost
```

- `R_chosen` is the next quarter's return of the investment held.
- `traded_fraction` is 1.0 when the whole account is moved from one investment to the other, because
  the sale and the purchase each count against the money moved.
- `cost_per_trade` is the cost of one trade as a fraction of the amount traded, covering commission
  and the gap between buying and selling prices. For large index funds and one move a quarter, 0.002,
  that is two tenths of a percent, is a realistic round-trip figure.

Over several quarters the results compound, meaning each quarter's net return multiplies the value
left by the previous quarter.

## A worked example

Two invented funds, one holding shares and one holding government bonds. The quarterly returns below
are made up, but the pattern is the one the strategy needs: the two move in opposite directions, and
each tends to continue for a quarter before turning. The decision for each quarter uses the returns
of the quarter before it and the holding is stated inside the table.

| Quarter | Prior equity | Prior bond | Chosen this quarter | Return earned | Switched | Cost | Net   |
| ------- | ------------ | ---------- | ------------------- | ------------- | -------- | ---- | ----- |
| 1       | n/a          | n/a        | Equity (to start)   | +6.00         | no       | 0.00 | +6.00 |
| 2       | +6.00        | -3.00      | Equity              | +5.00         | no       | 0.00 | +5.00 |
| 3       | +5.00        | -2.00      | Equity              | -4.00         | no       | 0.00 | -4.00 |
| 4       | -4.00        | +3.00      | Bonds               | +2.00         | yes      | 0.20 | +1.80 |
| 5       | -3.00        | +2.00      | Bonds               | -2.00         | no       | 0.00 | -2.00 |
| 6       | +4.00        | -2.00      | Equity              | +6.00         | yes      | 0.20 | +5.80 |
| 7       | +6.00        | -3.00      | Equity              | -5.00         | no       | 0.00 | -5.00 |
| 8       | -5.00        | +4.00      | Bonds               | +2.00         | yes      | 0.20 | +1.80 |

The strategy moved between the two funds three times in eight quarters. Compounding the net column:

```text
1.06 * 1.05 * 0.96 * 1.018 * 0.98 * 1.058 * 0.95 * 1.018 = 1.0907
```

That is a gain of 9.07 percent over the two years. For comparison, holding the equity fund alone
compounded to 6.37 percent, holding the bond fund alone to 0.71 percent, and holding a fixed half in
each and rebalancing every quarter to 4.04 percent. The switched portfolio beat both the single
funds and the fixed mix in this made-up run.

The example also shows the danger. In quarters 3 and 7 the rule had just chosen an asset from its
strong previous quarter, and that asset then fell: minus four and minus five percent. A rule that
lags the turns this way can lose. The source paper's claim is careful: it needs the criterion to be
at least minimally accurate, and if the signal is wrong even slightly more often than right, the
switching adds cost and takes on more risk than the fixed mix.

## What the research actually found

| Source                                               | What it measured                                                         | Result                                                                                                                                                                                              |
| ---------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, the simple strategy above                | One stock fund against one government bond fund, quarterly, 1991 to 2011 | 11.3 percent a year, volatility 9.3 percent, worst fall 18.41 percent, reward-to-risk 0.78                                                                                                          |
| Maewal and Bock, the source paper                    | Negatively correlated pairs of shares, funds and funds                   | Some very simple switching criteria led to lower volatility without a significant penalty in returns; the gain requires the criterion to be at least minimally accurate                             |
| Schizas and Thomakos, cited by Quantpedia            | 351 pairs of exchange-traded funds                                       | Rotation between two assets offered an advantage even with the simplest signal, and the size of the advantage depended on the two assets' returns, their correlation and, less so, their volatility |
| Clare, Seaton, Smith and Thomas, cited by Quantpedia | American shares and bonds from 1925                                      | Smoothing returns by switching between an asset and cash on a simple trend rule raised sustainable withdrawal rates, which is a related but different use of the same idea                          |

The direction is that a simple switching rule over a negatively correlated pair can help, and the
mechanism is understood rather than mysterious. What the sources do not agree on is how much of that
comes free. The Quantpedia figure of 11.3 percent a year is from one specific pair over one specific
twenty-year window, chosen because those two funds are clean proxies for shares and bonds. The
meta-study over 351 pairs is more useful precisely because it does not pick one: it finds the
advantage exists but that its size is governed by which two funds you picked and how they related
over the period, which is another way of saying the answer is in the pair, not in the rule.

Note also what the source paper claims and what it does not. It does not claim higher average
returns than the better of the two assets; it claims lower volatility without a significant penalty
in returns. That is a smaller claim, and it is the one the later evidence supports.

## How this project relates to it

The repository's brief on changing states is
[Regimes and change points](../../../strategies/books2/24_regimes_and_change_points.md). It reports,
on the paper `2309.00875v3`, that pairs trading in crude oil futures worked best when the spread
between the two was modelled as switching between states and estimated as data arrived, rather than
with a fixed rule, and that the only model in that slice which traded out of sample won with the
online filter rather than with a label read off the whole history. It also records, on `1410.6005v1`,
that a fitted state is usually a volatility state rather than a direction, which is a warning
against treating the switch here as a genuine forecast of the future.

The closely related study in this repository is
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md). Its third
section tests 1,022 rotation rules across ten industries and finds an average monthly return of 0.86
percent against 0.89 percent for simply holding the market, with only 132 of the rules beating it -
the clearest available demonstration that a rotation idea can look good in one pair and dissolve
when every version of it is tried. The implemented tool,
[the sector regime engine](../../../implementation/sector-regime-engine/README.md), measures whether
a market state justifies switching at all and blocks the switch in its default memoryless universe;
its user manual states plainly that every directional strategy in that document failed either the
significance test or the cost test.

## Where it goes wrong

- The pair is chosen with hindsight. Any two investments can be shown to be negatively correlated
  over the years in which the strategy worked. Picking the pair after seeing the result is the same
  mistake as picking the winning strategy after trying many, and it is the most common way this idea
  is oversold.
- The correlation is not permanent. Shares and bonds have moved together for long stretches, and
  during those stretches the rule is choosing between two assets that are falling at the same time,
  which removes the whole point.
- Whipsaw. The rule lags turns, as quarters 3 and 7 of the worked example show. Each time it switches
  at the wrong moment it pays a cost and buys the asset just before it turns.
- Costs on every switch. A switch trades the entire account, so even a modest round-trip cost adds
  up over years, and a rule that switches often can hand back its advantage in costs alone.
- The benchmark is doing a lot of work. "Beats the mix" and "beats the better single asset" are
  different claims. The source paper makes the first, which is the weaker one.
- It is momentum in disguise. Taking the recent winner of two assets is a cross-sectional momentum
  rule with only two items in the ranking, so it inherits momentum's habit of crashing when trends
  reverse.

## Try it yourself

You need a spreadsheet and a public source of quarterly prices for two funds, one holding shares and
one holding government bonds.

1. Build a sheet with one row per quarter for five years and columns for the price of each fund.
2. Add a column for each fund that computes the quarterly return: this quarter's price divided by
   last quarter's price, minus one.
3. Add a column that names the fund with the higher return in that same row, which is what the rule
   would hold in the next quarter.
4. In the next row down, put that fund's return as the strategy's return for the quarter.
5. Add a column that shows a one when the choice changed from the previous row, and subtract 0.20
   percent from the strategy on those rows.
6. Build a separate column for a fixed half-and-half mix, rebalanced every quarter, so you have
   something to compare against.

What to notice: in the years when shares and bonds move in opposite directions the switching column
tends to win and to move less, while in the years when they move together the advantage shrinks or
disappears. Count how many quarters the choice changed: the more often it changed, the more the
cost column matters. If the strategy wins by a wide margin on your pair, try a different pair and a
different five-year window, and see whether the result survives.

## Where this came from

- [QuantConnect strategy library: paired switching](https://www.quantconnect.com/tutorials/strategy-library/paired-switching),
  the rules as implemented: measure ninety-day performance, hold whichever asset did better, keep it
  for a quarter and repeat.
- [Quantpedia: paired switching](https://quantpedia.com/strategies/paired-switching),
  the performance figures, the fund pair used, the quarterly rhythm and the sample period 1991 to
  2011.
- Maewal and Bock, [Paired-Switching for Tactical Portfolio Allocation](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1917044),
  the source paper, which argues that a minimally accurate switching rule can lower volatility
  without a significant penalty in returns.
- [Regimes and change points](../../../strategies/books2/24_regimes_and_change_points.md), this
  repository's brief on switching-state models, including the crude-oil pairs result from
  `2309.00875v3`.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's study of rotation rules, including the 1,022-rule experiment in its third section.

## Words used in this tutorial

- correlation: a number between minus one and one describing whether two things move together or in
  opposite directions.
- momentum: the tendency of something that has been rising to keep rising for a while.
- portfolio: the collection of investments held at one time.
- rebalancing: buying and selling to bring holdings back to a fixed set of weights.
- return: the percentage change in the value of an investment over a period.
- rotation: moving money from one investment to another according to a rule.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
