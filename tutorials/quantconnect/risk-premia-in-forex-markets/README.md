# Currency trades built on the shape of the return distribution, not its average

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Four currency pairs: the euro against the dollar (EURUSD), the Australian dollar against the US dollar (AUDUSD), the US dollar against the Canadian dollar (USDCAD) and the US dollar against the Japanese yen (USDJPY) |
| How often it trades       | About once a week, when the whole list of four pairs is reviewed                                                                                                                                                        |
| What you need             | A spreadsheet and a few years of daily prices for the four pairs                                                                                                                                                        |
| Where the rules come from | [QuantConnect strategy library, risk premia in forex markets](https://www.quantconnect.com/tutorials/strategy-library/risk-premia-in-forex-markets)                                                                     |
| The underlying research   | Lemperiere, Deremble, Nguyen, Seager, Potters and Bouchaud, [Risk Premia: Asymmetric Tail Risks and Excess Returns](https://arxiv.org/abs/1409.7720)                                                                    |
| How well it held up       | Weak: the grade rests on one published implementation, whose own backtest returned about -0.33 percent a year over a decade, while the paper's evidence is about the carry trade as a class, not this selection rule    |
| Also appears in           | nothing else in this collection                                                                                                                                                                                         |

## The idea in one paragraph

A currency price has a usual day-to-day wobble and, occasionally, a very large day. The very large
days are called the tails of the distribution. For some currency pairs the large down days are bigger
and more frequent than the large up days: the return distribution leans left. This strategy measures
that lean from recent daily prices and treats a strongly left-leaning pair as one that pays a reward
for being held, so it buys that pair. A pair that leans right, with unusually large up days, is sold
instead. The whole list is reviewed about once a week and the money is split equally over whatever
pairs qualify.

## Why anyone believed it

The person on the other side of this trade is an investor or a company that is willing to pay to
avoid the large down days. A firm that must pay a bill in foreign currency will accept a slightly bad
average price in exchange for knowing the worst case. Exporters and importers, funds that insure
themselves against currency crashes, and savers sending money home all buy protection against the big
adverse move, and that buying pushes the expected return of the left-leaning pair up.

The reason the reward should persist is that the risk is unpleasant to hold. A strategy that earns a
little most of the time and loses a great deal occasionally is unattractive to anyone who is judged
by their worst quarter, so fewer people hold it than a plain average would suggest. The paper's
argument is not that everyone misprices currency but that investors are not much worried by small
wobbles and are very worried by large losses, so the price of the large losses is what drives the
reward. That is why the signal here is built on the tails rather than on the average or the size of
the wobble.

## An everyday comparison

Think of a small shop in a district where the weather is calm most days but there is a storm season.
The shop earns a steady few hundred a day for most of the year, and in a bad storm week it loses a
month of takings at once. Nobody wants to own that shop for the same average income as a shop in a
sheltered district, so the storm-district shop has to be cheaper to buy, which means a higher reward
for whoever does own it. The strategy is a way of working out which shops sit in the storm belt, by
looking at how lopsided their recent daily takings have been, and then collecting the discount.

## The rules, step by step

1. Take the four pairs EURUSD, AUDUSD, USDCAD and USDJPY. The written order is always the pair
   "base over quote": for EURUSD, the price is how many dollars one euro costs.
2. Collect daily closing prices for each pair over a lookback window. The QuantConnect code leaves
   the window length as a setting; this tutorial uses sixty trading days, about three months.
3. For each day, compute that day's return: today's price divided by yesterday's price, minus one.
   A price that went from 1.1000 to 1.1055 returned 0.005, that is 0.5 percent.
4. For each pair, compute the skewness of its recent daily returns using the formula in the next
   section. The skewness says how lopsided the distribution is: zero is symmetric, a negative number
   means the large moves are mostly down, a positive number means they are mostly up.
5. Buy any pair whose skewness is below -0.6. Sell short any pair whose skewness is above +0.6.
   Selling short means borrowing the pair, selling it now, and buying it back later in the hope the
   price falls.
6. Give every selected pair the same weight, one divided by the number of selected pairs. If four
   pairs qualify, put a quarter of the money in each. A sold pair counts in the same total.
7. Hold for one week and do not look at the prices in between.
8. At the end of the week, close everything that is no longer selected, set up whatever is now
   selected, and repeat from step 2. The QuantConnect code works in hours but the lookback is daily.

The threshold -0.6 and +0.6 is a choice, not a law. Lowering it to -0.3 selects more pairs more
often and pays more in costs; raising it to -1.0 selects fewer and trades less.

## The maths, with every symbol named

The one number the strategy needs, computed once per pair per week, is the skewness.

The daily return:

```text
r_t = P_t / P_(t-1) - 1
```

- `r_t` is the return on day `t`, written as a decimal: 0.005 means 0.5 percent.
- `P_t` is the closing price on day `t`.
- `P_(t-1)` is the closing price on the previous day.

The average of those returns and their typical spread:

```text
mean = (1/N) * SUM(r_t)
std  = SQRT( (1/N) * SUM( (r_t - mean)^2 ) )
```

- `N` is the number of daily returns in the window, sixty in this tutorial.
- `SUM` means add up the quantity after it over all `N` days.
- `mean` is the average daily return.
- `std` is the standard deviation, the typical distance of a day from the average.

The skewness, which is the average of the cubed distances divided by the spread cubed:

```text
S = ( (1/N) * SUM( (r_t - mean)^3 ) ) / std^3
```

- `S` is the skewness. A negative `S` means the days far below the average are bigger and more
  numerous than the days far above it, which is the left-leaning shape the strategy buys.
- The cube keeps the sign, so a large negative day contributes a large negative amount, while a
  large positive day contributes a large positive amount.
- Dividing by `std^3` makes the number comparable across pairs whose swings are of different sizes.

Then the selection and the weighting:

```text
long_pairs  = the pairs with S < -0.6
short_pairs = the pairs with S > +0.6
w = 1 / (number of long_pairs + number of short_pairs)
```

- `long_pairs` are bought; `short_pairs` are sold.
- `w` is the fraction of the money given to each selected pair, the same for every one of them.

The week's return on the book and its cost:

```text
R_week = w * SUM(long returns) - w * SUM(short returns)
Cost   = t * c
```

- `R_week` is the return of the whole book for the week.
- For a pair that was bought, its return adds; for one that was sold, its return subtracts.
- `t` is the traded fraction: close to 2.0 when the whole list is replaced, counted twice because
  the old positions are closed and the new ones opened, and lower when some names are kept.
- `c` is the cost per side as a fraction of the amount traded. For a large, liquid currency pair a
  realistic figure is one "pip", which is 0.0001, that is one hundredth of one percent.

## A worked example

Eight daily returns for EURUSD, in percent, the last of them a bad day. The average is -0.05 percent
per day and the standard deviation is 0.7433 percent.

| Day | Return (%) | Return - mean | (Return - mean)^2 | (Return - mean)^3 |
| --- | ---------- | ------------- | ----------------- | ----------------- |
| 1   | 0.2        | 0.25          | 0.0625            | 0.0156            |
| 2   | 0.3        | 0.35          | 0.1225            | 0.0429            |
| 3   | 0.1        | 0.15          | 0.0225            | 0.0034            |
| 4   | 0.4        | 0.45          | 0.2025            | 0.0911            |
| 5   | 0.2        | 0.25          | 0.0625            | 0.0156            |
| 6   | 0.3        | 0.35          | 0.1225            | 0.0429            |
| 7   | 0.1        | 0.15          | 0.0225            | 0.0034            |
| 8   | -2.0       | -1.95         | 3.8025            | -7.4149           |
| Sum | -0.4       | 0.00          | 4.4200            | -7.2000           |

The average of the cubed distances is -7.2000 / 8 = -0.9000. The spread cubed is
0.7433 * 0.7433 * 0.7433 = 0.4105. So the skewness is -0.9000 / 0.4105 = -2.19. That is well below
the -0.6 threshold, so this pair is bought. Notice that the single -2.0 day does almost all of the
work: without it the skewness of the other seven days is a small positive number.

Now suppose the review leaves two pairs selected, EURUSD to buy and USDJPY to sell, so each gets
weight 0.5. Over the following week EURUSD returns +1.0 percent and USDJPY returns +0.4 percent.

| Pair   | Skewness | Weight | Direction | Week return | Contribution |
| ------ | -------- | ------ | --------- | ----------- | ------------ |
| EURUSD | -2.19    | +0.50  | bought    | +1.0%       | +0.5000%     |
| USDJPY | +0.80    | -0.50  | sold      | +0.4%       | -0.2000%     |
| Total  |          | 0.00   |           |             | +0.3000%     |

The book gained 0.30 percent before costs. Both selected pairs changed since the previous week, so
the traded fraction is 2.0 and the cost is 2.0 * 0.0001 = 0.0002, that is 0.02 percent. The net
return for the week is 0.30 - 0.02 = 0.28 percent. Forex costs are small in this book because the
pairs are the most liquid in the world; the reward is also small, so the two have to be compared
carefully.

## What the research actually found

The strategy on the QuantConnect page is a translation of one result from a much larger study.

| Source                                               | What it measured                                                                                          | Result                                                                                                                                                                                                           |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lemperiere and co-authors, the underlying paper      | The link between risk-adjusted return and skewness across equities, bonds, currencies, options and credit | A roughly linear relation, Sharpe ratio about 1/3 minus the skewness divided by four; across 10 carry deciles on 20 developed currencies the correlation between skewness and Sharpe ratio is -0.76 (p.12, p.14) |
| Lemperiere and co-authors, on the tail itself        | The five percent largest moves of the US index since 1928                                                 | Those five percent of days wipe out about half of the premium earned by all the smaller days (p.4-5)                                                                                                             |
| Lemperiere and co-authors, the full carry book       | An equal-weighted carry strategy over ordered currency pairs, 1974 to 2014                                | Sharpe ratio about 0.85 with skewness about -0.94, that is a large reward attached to a large left tail (p.12)                                                                                                   |
| QuantConnect, the implementation on the library page | The four-pair skewness rule above, weekly rebalancing, over about a decade                                | Annual return about -0.33 percent; the page itself attributes the result to a universe that is too small, thresholds that may be wrong, and too short a history                                                  |
| Quantpedia, summarising the same paper               | The paper's cross-asset result                                                                            | States that most risk premiums are better explained by tail-risk skewness than by volatility, and quotes the paper's own finding that trend following is the exception (Quantpedia entry)                        |

Read together the picture is this. The paper measures a real relationship between the lopsidedness of
a return distribution and the reward earned for holding it, and the relationship is strong across the
carry trades it studies. The QuantConnect page takes that relationship and turns it into a rule that
selects individual pairs by their own skewness, on a fixed four-pair list, and the rule lost a small
amount of money over the decade it was run. The paper does not test that rule; it tests carry, which
is a different way of choosing currency positions. A reader should treat the two as different claims
about the same family of ideas, not as one claim with two results.

## How this project relates to it

This repository does not implement a currency strategy, and the closest thing to this paper in it is
the risk brief [Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md).
That brief lists this exact paper, `1409.7720v3`, as the natural next step if the platform wants to
price tail exposure rather than only measure it, and it states plainly that the paper had not been
read when the brief was written, so it makes no claim about it. The brief's own finding is about the
other direction: tail-sensitive measures such as expected shortfall are fragile on short samples,
because one large loss can move the estimate without bound.

The mechanical piece that this strategy would need already exists. The skewness of a return series is
computed by [returns_skewness.rs](../../../crates/analysis/src/statistics/returns_skewness.rs), one
of the statistics the platform reports for every backtest, so a reader who wanted to reproduce the
signal would be reusing a number the platform already produces. For the currency side, the brief
[Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md) covers what
the repository does know about exchange rates, which is that a currency signal can be built but that
costs and a single vendor's data are the limiting factors.

## Where it goes wrong

- The tail can flip. The skewness of a short window is dominated by its largest days, so a pair can
  be bought because of one crash that has already happened and will not repeat in the coming week.
  A sixty-day window contains about a quarter's worth of news and no more.
- It is a bet on an average, not a rule that always wins. The paper's own story says the strategy
  earns a little most weeks and loses a lot occasionally; the QuantConnect decade is an example of
  the losing side of that, and a decade is short enough that this could be luck in either direction.
- Four pairs is not a portfolio. With four names, and often fewer selected, a single currency's
  politics or central bank can dominate the whole result. The page itself names the small universe
  as a likely cause of the poor result.
- Costs and the bid-ask gap. Currency trading looks cheap per trade, but a weekly rule trades a lot.
  At one pip per side and a full weekly rebuild, the annual cost is about two percent, which is the
  same size as the reward the paper describes.
- The sample and the rule were chosen together. The paper chose its thresholds and its windows after
  looking at the data; copying those choices into a live rule uses information the rule would not
  have had at the time.
- What would have to be true for the idea to be false: that the lopsidedness of past daily returns
  carries no information about the reward over the next week. That is a live possibility, and the
  paper's evidence does not rule it out for a rule this specific.

## Try it yourself

You need a spreadsheet and daily closing prices for the four pairs, which any finance website
publishes. This is a paper exercise; do not trade it.

1. Put one column per pair, one row per day, for three years of daily closes.
2. In the next column, for each pair, compute the daily return: today over yesterday, minus one.
3. Add a header cell for the window length, say 60. In a new column, for each day, compute the
   skewness of the last 60 returns of that pair, using the cubed-distance formula above. Most
   spreadsheets have a built-in skewness function; if it differs from the formula by a small
   constant factor that is the difference between the population and sample definitions and does not
   change the ranking.
4. In one more column, write "buy" when the skewness is below -0.6, "sell" when it is above +0.6,
   and leave it blank otherwise.
5. In the final column, average the next week's returns of the bought pairs and subtract the average
   of the next week's returns of the sold pairs. That is the strategy's weekly return before costs.
6. Subtract 0.02 percent per week from every week to represent the cost of a full rebuild.

What to notice: for long stretches the skewness column sits between -0.6 and +0.6 and nothing trades,
and then one bad day flips a pair into the list and everything changes. Count how often the list
changes, and compare the strategy's average weekly number with simply holding the four pairs in equal
amounts. The difference will be small relative to the week-to-week noise, which is the honest
difficulty with a signal built on rare events.

## Where this came from

- [QuantConnect strategy library: risk premia in forex markets](https://www.quantconnect.com/tutorials/strategy-library/risk-premia-in-forex-markets),
  the four pairs, the skewness thresholds, the weekly rebuild and the reported backtest return.
- Lemperiere, Deremble, Nguyen, Seager, Potters and Bouchaud,
  [Risk Premia: Asymmetric Tail Risks and Excess Returns](https://arxiv.org/abs/1409.7720),
  `1409.7720v3`, the source of the skewness-and-reward relationship, the carry deciles and the
  five-percent tail calculation.
- [Quantpedia: asset class risk premiums explained by skewness](https://quantpedia.com/asset-risk-premiums-explained-by-skewness/),
  the restatement of the paper and the note that trend following is the exception.
- [Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md),
  this repository's brief that names this paper as the next step for pricing tail exposure.

## Words used in this tutorial

- bid-ask gap: the difference between the price at which you can sell and the price at which you can
  buy, which is a cost every time you trade.
- carry trade: borrowing in a currency with low interest rates to hold one with high interest rates,
  earning the interest difference.
- distribution: the collection of all the possible outcomes of something and how often each occurs.
- pip: the smallest usual move in a currency price, 0.0001, that is one hundredth of one percent.
- Sharpe ratio: the reward earned divided by how much the thing moved around, both measured per year.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- skewness: a number describing how lopsided a distribution is, negative when the big moves are down.
- standard deviation: how far a typical day sits from the average, measured in the same units.
- tail: the rare, far-from-average outcomes at either end of a distribution.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
