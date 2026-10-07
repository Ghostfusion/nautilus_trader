# Holding an asset class only while it is above its ten-month average

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                              |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Five kinds of investment, one fund each: American shares, foreign shares, government bonds, property companies and commodities; any of them can instead be held as cash                                                                                                            |
| How often it trades       | Once a month, when the lines are checked, and only the slices that have crossed their line change                                                                                                                                                                                  |
| What you need             | A spreadsheet and fifteen years of monthly prices for five funds                                                                                                                                                                                                                   |
| Where the rules come from | [QuantConnect strategy library, asset class trend following](https://www.quantconnect.com/tutorials/strategy-library/asset-class-trend-following) and the [Quantpedia entry](https://quantpedia.com/strategies/asset-class-trend-following) it cites                               |
| The underlying research   | Mebane Faber, [A Quantitative Approach to Tactical Asset Allocation](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=962461), first circulated in 2006, and its freely available working version, [the whitepaper](https://www.trendfollowing.com/whitepaper/CMT-Simple.pdf)   |
| How well it held up       | Mixed: the original study covers more than a century on one index and three decades on five asset classes with large reductions in the worst falls, but later independent work finds that the reported edge shrinks once realistic costs and the number of rules tried are counted |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this collection, the closest relative: it ranks baskets by past return rather than testing each one against its own average                                                                                                     |

## The idea in one paragraph

Divide the money into five equal slices, one for each kind of investment: American shares, foreign
shares, bonds, property companies and commodities. Once a month, for each of the five funds, draw a
slow reference line through the last ten month-end prices by averaging them. If this month's price is
above that line, keep that fund; if it is below, move that slice into cash for the month. Repeat every
month. In good times most slices are invested and the portfolio behaves like a sensible mix of assets.
In bad times slices drop out one by one and the portfolio holds more cash, which is what softens the
worst falls.

## Why anyone believed it

Prices move in long waves, not only in day-to-day noise. When an economy weakens or a market turns
down, the turn is rarely visible in one month of data; it shows up as a run of months going the same
way. Averaging the last ten months gives a baseline, and requiring the current price to be above it is
a cheap mechanical way to ask whether the recent path agrees with the longer one. If it does, the
asset is in an up-wave and worth holding; if it does not, the up-wave has ended and the money is
better off in cash.

The person on the other side is the investor who holds through the whole decline. Many people sell
only after a fall has become painful, which is late, and many funds must stay fully invested whatever
the market does. A rule that steps aside early is selling to, and buying from, those who cannot or will
not. The claim is not that the rule predicts the future; it is that it removes the money from the
worst stretches, and that avoiding a 50 percent fall is worth more than capturing every last rise.

## An everyday comparison

A shop that restocks a product only while its sales are above their ten-month average is not
forecasting fashion. It is applying a slow test: has this line been selling above its own long-run
pace? A single good week will not pass the test, because one week barely moves a ten-month average,
and a single bad week will not fail it either. The shelf changes only when the slow average is
crossed, which is rare and deliberate. This strategy does the same to five investment lines at once,
moving a slice of money in or out of each.

## The rules, step by step

1. Choose five funds, one for each kind of investment. In the American market the usual choices are
   SPY for American shares, EFA for foreign shares, BND for bonds, VNQ for property companies and GSG
   for commodities.
2. Give each fund one fifth of the money, so each slice is 20 percent.
3. Once a month, at the month end, compute the ten-month average for each fund: add the last ten
   month-end prices and divide by ten.
4. Compare the current month-end price with that average. If the price is above the average, keep the
   slice in that fund for the coming month. If the price is below the average, move that slice into
   cash for the coming month.
5. Do this for each of the five funds separately. Some months all five are held, some months none are.
6. Hold for one month, then repeat from step 3. Trade only the slices that changed state; a slice that
   stays held is left alone.
7. Cash earns a small interest rate, which the original study estimates with the rate on short-term
   commercial paper, the interest large companies pay to borrow for a few months.

The rule is never in a hurry. The ten-month average moves slowly because each month is only a tenth of
it, so a single month's jump or fall rarely crosses the line by itself; crossings happen when the
trend has genuinely changed.

## The maths, with every symbol named

The slow reference line is the ten-month simple moving average:

```text
SMA_t = (P_t + P_(t-1) + P_(t-2) + ... + P_(t-9)) / 10
```

- `SMA_t` is the ten-month average as of this month.
- `P_t` is this month's month-end price, `P_(t-1)` is last month's, and so on back for ten months, to
  `P_(t-9)`.
- Dividing by 10 turns the sum of ten monthly prices into their average.

The signal is a single comparison:

```text
hold the slice  if  P_t > SMA_t
hold cash       if  P_t < SMA_t
```

- `P_t` is this month's month-end price.
- `SMA_t` is the average computed just above.
- A tie is treated as cash, which is the cautious choice the implementations make.

Each slice carries a fixed weight, and a slice that is in cash still counts as a slice:

```text
w = 1 / 5 = 0.20 for each of the five asset classes
```

- `w` is the share of the money assigned to one asset class, whether it sits in the fund or in cash.

The portfolio's return for the month is the average of what the five slices earned:

```text
R_portfolio = (0.20 * R_1) + (0.20 * R_2) + (0.20 * R_3) + (0.20 * R_4) + (0.20 * R_5)
```

- `R_1` to `R_5` are the monthly returns of the five slices.
- For a held slice, `R_i` is the fund's return that month. For a cash slice, `R_i` is the interest
  earned on cash that month.

The cost of trading, as elsewhere in this collection:

```text
Cost = t * c
```

- `t` is the traded fraction of the account: the fraction sold plus the fraction bought. A slice that
  moves from cash into a fund is bought, costing one fifth; a slice that moves from a fund into cash
  is sold, costing one fifth; a slice that does not move costs nothing.
- `c` is the cost per unit traded, covering the gap between buying and selling prices plus commission;
  a realistic figure for large funds is about 0.0005, that is five basis points, where one basis point
  is one hundredth of one percent.

## A worked example

Twelve months of month-end prices for one invented fund, to show how the line is built and crossed.
The average needs ten months before it exists.

| Month | Price ($) | Ten-month average ($) | Signal |
| ----- | --------- | --------------------- | ------ |
| 1     | 100       | not available yet     | hold   |
| 2     | 101       | not available yet     | hold   |
| 3     | 102       | not available yet     | hold   |
| 4     | 103       | not available yet     | hold   |
| 5     | 104       | not available yet     | hold   |
| 6     | 105       | not available yet     | hold   |
| 7     | 106       | not available yet     | hold   |
| 8     | 107       | not available yet     | hold   |
| 9     | 108       | not available yet     | hold   |
| 10    | 109       | 104.5                 | hold   |
| 11    | 108       | 105.3                 | hold   |
| 12    | 103       | 105.5                 | cash   |

The month 10 average is the mean of prices 100 through 109, which is 1045 divided by 10, or 104.5, and
109 is above it. The month 11 average drops 100 and adds 108, giving 105.3, and 108 is still above it.
The month 12 average drops 101 and adds 103, giving 105.5, and 103 is below it, so the slice moves to
cash. Notice how slowly the line moves: the price fell six points while the average barely changed.

Now the whole portfolio in one month. The prices and averages are invented.

| Asset class     | Fund | Price ($) | Ten-month average ($) | Signal | Next-month return |
| --------------- | ---- | --------- | --------------------- | ------ | ----------------- |
| American shares | SPY  | 300       | 280                   | hold   | +2.0 percent      |
| Foreign shares  | EFA  | 70        | 68                    | hold   | +1.0 percent      |
| Bonds           | BND  | 72        | 71                    | hold   | +0.5 percent      |
| Property        | VNQ  | 80        | 84                    | cash   | +0.1 percent      |
| Commodities     | GSG  | 20        | 21                    | cash   | +0.1 percent      |

Three slices are held and two are in cash. The cash slices are credited with 0.1 percent for the
month, which is a low interest rate used as a placeholder.

| Slice                 | Weight | Return this month | Contribution  |
| --------------------- | ------ | ----------------- | ------------- |
| American shares       | 0.20   | +2.0 percent      | +0.40 percent |
| Foreign shares        | 0.20   | +1.0 percent      | +0.20 percent |
| Bonds                 | 0.20   | +0.5 percent      | +0.10 percent |
| Property (in cash)    | 0.20   | +0.1 percent      | +0.02 percent |
| Commodities (in cash) | 0.20   | +0.1 percent      | +0.02 percent |
| Total                 | 1.00   |                   | +0.74 percent |

The portfolio gained 0.74 percent before costs. Suppose that this month only the property slice
changed, moving from the fund into cash. The sale of one fifth is the only trade, so:

```text
t = 0.20
Cost = 0.20 * 0.0005 = 0.0001, that is 0.01 percent
Net return for the month = 0.74 - 0.01 = 0.73 percent
```

That small cost is the point of the rule. The portfolio trades only when a line is crossed, and most
months it does not trade at all. The original study averages about one round trip per asset class per
year, which is why this style survives costs where weekly strategies do not.

## What the research actually found

Faber's study is the source of the rule, and it reports two sets of results. On the American share
index from 1900 to 2005, the rule returned 10.66 percent a year against 9.75 percent for simply
holding the index, with lower volatility, 15.38 percent against 19.91 percent, a better reward-to-risk
ratio, 0.43 against 0.29, and a much smaller worst fall, 49.98 percent against 83.66 percent. The
portfolio was in the market about 70 percent of the months and made fewer than one round trip trade
per year. The study is explicit that the headline results exclude taxes, commissions and slippage, and
that the rule still underperformed the index in roughly 40 percent of the years since 1900.

The five-asset-class version over 1972 to 2005 reduced risk in every asset class on every measure the
study reports, standard deviation, worst fall, worst year and a duration-weighted measure of falls,
while keeping returns roughly the same. Across the five classes the average time in the market was
73.73 percent of the months, and the average was 0.69 round trips per year. Quantpedia reports the
tradable five-fund version over 1973 to 2008: 11.27 percent a year, volatility 6.87 percent, worst
fall 29.43 percent and a reward-to-risk ratio of 1.06, using six instruments, rebalanced monthly and
based on exhibit 19 of the source paper.

The caution comes from independent work. Zakamulin revisits the reported performance of moving-average
and time-series momentum rules and argues that it contains a considerable data-mining bias and ignores
market frictions; in out-of-sample tests that account for realistic trading costs, he finds at best a
marginal improvement over simply buying and holding. Guilleminot, Ohana and Ohana compare trend-driven
and risk-driven allocation over 1993 to 2012 and find the trend-driven edge is not stable over time,
because periods with exploitable trends alternate with long stretches that have none. Hutchinson and
O'Brien, using almost a century of data, find that in the period following financial crises trend
following earns less than half its no-crisis average. Read together: the risk reduction is the better
documented result, and the extra return is the fragile one.

## How this project relates to it

The ten-month average is a regime filter, and this repository's brief
[Regimes, breakpoints and state switching](../../../strategies/books2/24_regimes_and_change_points.md)
reads the evidence on exactly that kind of tool. Two findings there bear on this rule. First, the only
regime model in the brief that survived an out-of-sample trading test was re-estimated as data arrived
rather than read off the whole sample afterwards; a moving average computed from past prices only is
already on the right side of that line. Second, the states such models find are mostly volatility
states, and the relationship between risk and return changes sign across them, so "above the average"
works as a filter on the market's state rather than a forecast of direction.

The tool [implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md)
turns that idea into something measurable. It does not run trend following. It measures whether a
market is in a state where a directional rule is eligible to run at all, reports the evidence for that
verdict, and blocks the rule when the evidence is not there. Its
[user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) states plainly what it does
not do, and its default example ends with the honest outcome that the directional overlays failed
either the significance test or the cost test on the data available.

## Where it goes wrong

- Whipsaw. When a price hovers around its average, the rule changes its mind, trading in and out and
  paying a spread each time. The study's low turnover is an average, not a promise.
- It is all or nothing. A price just below the line moves the whole slice to cash, so the decisions
  turn on a rounding difference.
- The parameters were chosen after the fact. The ten-month window is one of many, the five funds are
  one of many lists, and the rule was tested on data that already existed. Faber shows the result is
  broadly stable across moving-average lengths of three to twelve months, which helps, but the choice
  of the assets and the window is still a choice.
- Look-ahead in the average. The average must be built only from prices that existed at the moment of
  the decision. A backtest that uses a later price is using information the rule could not have had.
- A regime can end. The long stretches without a trend that Guilleminot and his co-authors describe
  are exactly when the rule pays costs for nothing, and they can last years.
- Costs and taxes. The headline numbers exclude them. A taxable account sold out of a fund and back
  in a few months later can lose most of the advantage to the tax bill, so the study suggests holding
  the rule inside a tax-sheltered account.
- The cash rate matters. Over long periods the interest earned on the cash slices is part of the
  return, and assuming a higher rate than is realistic flatters the result.

## Try it yourself

You need nothing but a spreadsheet and a public source of monthly prices.

1. Download fifteen years of month-end closing prices for one large share index fund.
2. Add a column next to the price that computes the ten-month moving average: the average of the
   current price and the nine above it. Leave the first nine rows blank.
3. Add a column called signal that says "hold" when the price is above the average and "cash" when it
   is below.
4. Add a column for the strategy's monthly return: the fund's return when the signal says hold, and
   the cash rate when it says cash, such as 0.25 percent a year divided by twelve.
5. Add a column that compounds both the strategy's return and the fund's own return into two growth
   numbers, and compare them at the end.
6. Count the number of times the signal changes from hold to cash or back. That is the number of
   trades, and each one pays a spread.

What to notice: the strategy gives up some of the biggest rises, because the rule needs the price to
be above the average before it buys again, and those rises start suddenly. What it buys is avoidance
of the deep falls, so a sheet ahead on the worst fall but behind on the return shows the pattern.

## Where this came from

- [QuantConnect strategy library: asset class trend following](https://www.quantconnect.com/tutorials/strategy-library/asset-class-trend-following),
  the five funds, the ten-month average and the hold-or-cash rule.
- [Quantpedia: asset class trend-following](https://quantpedia.com/strategies/asset-class-trend-following),
  the indicative performance, volatility, worst fall, sample period, instrument count and the source
  paper's abstract.
- Mebane Faber, [A Quantitative Approach to Tactical Asset Allocation](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=962461),
  the 2006 working paper, and [the whitepaper version](https://www.trendfollowing.com/whitepaper/CMT-Simple.pdf),
  which carries the 1900 to 2005 index results and the 1972 to 2005 five-asset results.
- Zakamulin, [The Real-Life Performance of Market Timing with Moving Average and Time-Series Momentum Rules](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2242795),
  the out-of-sample and friction critique.
- [Regimes, breakpoints and state switching](../../../strategies/books2/24_regimes_and_change_points.md),
  this repository's reading of the regime-filter literature that sits behind the ten-month average.

## Words used in this tutorial

- asset class: a broad kind of investment, such as shares, bonds, property or commodities.
- cash: money held uninvested, earning a small interest rate, used here as the safe place for a slice.
- drawdown: the fall from a peak to the following low, measured in percent.
- regime: a period during which the market behaves in a consistent way, such as a rising trend or a
  falling one.
- simple moving average: the plain average of the last few prices, recalculated each period.
- tactical asset allocation: changing how much is held in each asset class in response to conditions,
  rather than keeping fixed weights forever.
- trend following: holding something while its price is rising and stepping aside while it falls.
- whipsaw: repeated small losses from a rule that keeps changing its mind when a price hovers around
  its signal.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
