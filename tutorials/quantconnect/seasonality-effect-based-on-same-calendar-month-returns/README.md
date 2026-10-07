# Buying the shares that did well in this calendar month in past years

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of the hundred most heavily traded American companies and exchange-traded funds priced above five dollars                                                                                                   |
| How often it trades       | Once a month, at the end of the month                                                                                                                                                                              |
| What you need             | A spreadsheet and a few years of monthly prices for a hundred shares                                                                                                                                               |
| Where the rules come from | [QuantConnect strategy library, seasonality effect based on same calendar month returns](https://www.quantconnect.com/tutorials/strategy-library/seasonality-effect-based-on-same-calendar-month-returns)          |
| The underlying research   | Keloharju, Linnainmaa and Nyberg, [Common Factors in Return Seasonalities](https://www.nber.org/papers/w20815), building on Heston and Sadka                                                                       |
| How well it held up       | Mixed: the grade rests on a seasonality that reappears in several countries and asset classes in the published record, against one implementation whose reward for the risk was far below simply holding the index |
| Also appears in           | nothing else in this collection                                                                                                                                                                                    |

## The idea in one paragraph

Some shares behave differently in one calendar month than in the others. A department store might tend
to rise every December, a travel company every June, and this tendency can repeat year after year
for reasons that have little to do with the general market. This strategy looks at how each share
performed in this same month in the previous year, buys the ones that did best, sells the ones that
did worst, and repeats at the end of every month. It is a bet that the month's own pattern will show
up again, not that the share is generally good or bad.

## Why anyone believed it

The counterparty is an investor who trades the general outlook and ignores the calendar, or one who
is forced to sell at a particular time of year for reasons outside the market. Money managers who
report in December often tidy their holdings then and re-buy in January, which tilts prices in a
pattern that repeats. Companies that sell seasonal goods report earnings on a seasonal schedule, so
good or bad news clusters in the same months. And investors who remember that a share rose last
August tend to buy it in August again, which is a self-fulfilling habit as long as enough people
share it.

The argument for persistence is that the calendar is known in advance and never changes. A bank
holiday, a tax deadline, an index rebuild and an earnings season all recur in the same month every
year, so a share whose business or ownership is sensitive to any of them can carry the same tilt for
years. The argument against persistence is equally simple: any pattern this easy to describe should
attract money and disappear.

## An everyday comparison

Think of a cafe beside a university. In term time it is full and in the summer holidays it is empty,
and this happens every year whether or not the coffee improves. Someone who watched only last
September would already know something useful about this September: the students come back. The
strategy here is the same observation applied to shares. It is not claiming the cafe is a good
business; it is claiming that the same month tends to bring the same crowd, and it ranks cafes by how
full they were in that month a year ago.

## The rules, step by step

1. Build a list of the hundred most heavily traded American shares and exchange-traded funds whose
   price is above five dollars. "Most heavily traded" means the largest dollar volume on the day of
   the review, where dollar volume is the number of shares traded multiplied by the price.
2. For each name on the list, find the same calendar month one year ago. To work the rule at the end
   of August 2024, use August 2023.
3. Compute that month's return for each name: the price at the end of the month divided by the price
   at the start of the month, minus one. A name that went from 100.00 to 108.00 returned 0.08, that
   is 8 percent.
4. Rank the hundred names by that return, best first.
5. Buy the top ten. Sell short the bottom ten. Selling short means borrowing the shares, selling them
   now, and buying them back later in the hope the price falls.
6. Give every selected name the same weight: one divided by twenty, applied to the twenty names, with
   the bought names receiving a positive amount and the sold names a negative one. The QuantConnect
   code leaves the number ten as a setting.
7. Hold through the next month and do not look at the prices in between.
8. At the end of that month, sell whatever has dropped out of the lists, buy whatever has entered,
   and repeat from step 2. The whole list is rebuilt from scratch each month.

## The maths, with every symbol named

The whole strategy is one return computed per share, one sort, and one average.

The same-calendar-month return from a year ago:

```text
m_i = P_end / P_start - 1
```

- `m_i` is the seasonality score of share `i`, as a decimal: 0.08 means 8 percent.
- `P_end` is the closing price at the end of that month one year ago.
- `P_start` is the closing price at the start of that same month.

Rank the hundred shares by `m_i`, largest first, and keep the top ten and the bottom ten. Give each
selected share the same weight:

```text
w_i = 1 / 20  for each of the twenty selected shares, with the sign negative for the bottom ten
```

- `w_i` is the fraction of the money placed in share `i`; a positive sign is a purchase and a
  negative sign is a sale.
- The positive weights add to +0.5 and the negative weights to -0.5, so the book is 50 percent bought
  and 50 percent sold, with no net exposure to the general market.

The month's return of the book, and the cost of rebuilding it:

```text
R_month = SUM( w_i * r_i )
Cost    = t * c
```

- `R_month` is the return of the whole book for the month.
- `r_i` is the return of share `i` over that month.
- `t` is the traded fraction. Selling the old holdings and buying the new ones counts twice, so a
  full rebuild costs `t` of about 2.0, and a half-replaced book costs about 1.0.
- `c` is the cost per side as a fraction of the amount traded, covering the bid-ask gap and any
  commission. For heavily traded American shares a realistic figure is 0.001, that is ten basis
  points, where one basis point is one hundredth of one percent.

## A worked example

Six shares, and their returns in the same month of the previous year. The numbers are invented but of
the size these returns actually take.

| Share | Return in this month last year (%) | Rank | Action     |
| ----- | ---------------------------------- | ---- | ---------- |
| A     | +8                                 | 1    | buy        |
| B     | +5                                 | 2    | buy        |
| C     | +2                                 | 3    | none       |
| D     | -1                                 | 4    | none       |
| E     | -4                                 | 5    | sell short |
| F     | -7                                 | 6    | sell short |

The top two are bought and the bottom two are sold, each with a weight of 0.25. Now suppose the
following month produced these returns.

| Share | Weight | Direction | Month return | Contribution    |
| ----- | ------ | --------- | ------------ | --------------- |
| A     | +0.25  | bought    | +3.0 percent | +0.7500 percent |
| B     | +0.25  | bought    | +2.0 percent | +0.5000 percent |
| E     | -0.25  | sold      | -1.0 percent | +0.2500 percent |
| F     | -0.25  | sold      | +0.5 percent | -0.1250 percent |
| Total | 0.00   |           |              | +1.3750 percent |

The book gained 1.375 percent before costs. Suppose two of the four names change at the next
rebuild, so half the book is replaced and the traded fraction is `t = 2 * 0.5 = 1.0`. At a cost of
ten basis points per side the cost is 1.0 * 0.001 = 0.001, that is 0.10 percent, and the net return
for the month is 1.375 - 0.10 = 1.275 percent. Twelve months at that rate is about 17.8 percent a
year before costs and 16.4 percent after. That is faster than the published figures below, which is
the point of the example: it shows the arithmetic, not a forecast.

## What the research actually found

The published record is broad, and the two sources below do not state the same number.

| Source                                                     | What it measured                                                                  | Result                                                                                                                                                                                                                                                                      |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Keloharju, Linnainmaa and Nyberg, the paper the page cites | A long-short strategy ranked on historical same-calendar-month returns, US shares | An average return of about 13 percent per year, with the same pattern in anomalies, commodities, international indices and at the daily frequency (paper abstract)                                                                                                          |
| The same paper, on where the effect comes from             | Correlations between seasonality strategies                                       | The correlations are modest, so the paper argues the seasonalities come from several common factors rather than one distinct effect (paper abstract)                                                                                                                        |
| QuantConnect, the implementation on the library page       | The top hundred liquid names, long ten and short ten, rebalanced monthly          | Over ten years, a reward-for-risk figure of 0.128 against 0.773 for the index it compared with; the page states the paper reports an average monthly return of 1.88 percent, which is about 23 percent a year and does not match the 13 percent in the paper's own abstract |
| Quantpedia, the seasonality entry it cites                 | Cross-sectional seasonality of shares, 1963 to 2015                               | The pattern persists in the same month for ten years and more; a one-standard-deviation increase in the historical same-month return is associated with an average 23 percent higher return over the next ten years in the relevant months (Quantpedia entry)               |
| Heston and Sadka, the study the paper builds on            | Monthly returns of US shares                                                      | Established the same-month repeat in the cross-section that later work extended                                                                                                                                                                                             |

The disagreement is worth stating plainly. The paper's abstract says 13 percent a year; the
QuantConnect page says 1.88 percent a month, which annualises to roughly 23 percent; and the
implementation's own ten-year test rewarded risk far below the index. The most likely reading is that
the raw effect is real in the historical data over long periods, that averaging many years of the
same month rather than one single year is what makes it stable, and that a one-year-lookback
implementation on a hundred liquid American names, traded monthly with costs, does not capture much
of it. The page itself suggests looking back several years instead of one, and using a different way
of forming the universe.

## How this project relates to it

This repository does not implement a calendar-month seasonality rule. The nearest thing is the brief
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
which surveys what is actually predictable in returns. Its central finding is directly relevant to
any seasonality rule: the cross-sectional findings it reviews are mostly genuine rather than
statistical accidents, but the binding constraints are decay, crowding and the integrity of the
evaluation, and a factor claim is much weaker once it is restated under value weighting. A
seasonality rule is a cross-sectional claim of exactly that kind.

The other connection is methodological. The brief reports that momentum-style rules are positively
skewed by design and that a strategy's reported shape depends on the horizon it is measured at,
citing `2101.01006v2`. Any month-by-month rule should be judged the same way: measured at the
horizon it trades, not at the horizon of the data, and with the cost of its turnover included. The
sector study [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md) is
the repository's worked example of that discipline, and it reaches the same conclusion for a
different ranking rule: a rule that keeps re-sorting a list pays costs whether or not the sort
predicts anything.

## Where it goes wrong

- One year is a very thin signal. A single month's return is mostly noise, so a rule built on the
  previous year's same month may be measuring luck. The paper's own advice, reflected on the
  QuantConnect page, is to average several years of the same month, which the page's implementation
  does not do.
- The universe is chosen with hindsight. The hundred most-traded names of today are not the hundred
  most-traded names of ten years ago, and funds that did not exist cannot be backtested. Using a
  current list on old data silently gives the rule the winners of the last decade.
- Costs on a monthly rebuild. Twenty positions rebuilt every month is a lot of trading. At ten basis
  points per side and a full rebuild, the annual cost is about 2.4 percent, which is larger than the
  measured edge of many seasonality rules.
- The effect is small and crowded. The paper's own correlations between seasonality strategies are
  modest, and a rule that is published and easy to trade through funds is a rule whose reward is
  being competed away, as the repository brief on predictability argues.
- The number of ways to define the rule is large: one year or five, top ten or top five, monthly or
  weekly, share or fund. Choosing the best of those after seeing the results is how a weak effect
  is turned into a confident-looking one.
- What would have to be true for the idea to be false: that a share's return in one calendar month
  carries no information about its return in the same month next year. The published work argues
  against that, but the page's own test is a caution that the tradable version may be much weaker.

## Try it yourself

You need a spreadsheet and monthly closing prices for a few dozen well-known shares, which any
finance website will give you.

1. Build a sheet with one column per share and one row per month, going back at least six years.
2. In a second block, for each share and each month, compute that month's return: this month's price
   divided by last month's price, minus one.
3. In a third block, for each month, fetch each share's return from the same calendar month one year
   earlier. Call this the seasonality score.
4. For each month, rank the scores and write down the names in the top five and the bottom five.
5. In the row for the following month, average the next-month returns of the top five and subtract
   the average of the bottom five. That is the strategy's return for the month, before costs.
6. Subtract 0.20 percent from every month to stand for a full rebuild, and start a second column that
   simply holds all the shares equally.

What to notice: in many months the top and bottom lists barely change, so the rule trades little and
earns little, and in a few months the whole list turns over. Compare the two columns over the whole
period rather than month by month; the difference will be small and will depend heavily on whether
you subtract the cost. Then repeat the exercise using the same month across the last three years
averaged together instead of only the last year, and compare. That difference is the single biggest
choice in the rule, and the page's own suggestion points at it.

## Where this came from

- [QuantConnect strategy library: seasonality effect based on same calendar month returns](https://www.quantconnect.com/tutorials/strategy-library/seasonality-effect-based-on-same-calendar-month-returns),
  the universe, the one-year lookback, the long and short lists and the reported reward for risk.
- Keloharju, Linnainmaa and Nyberg, [Common Factors in Return Seasonalities](https://www.nber.org/papers/w20815),
  the 13 percent per year result and the finding that seasonalities span asset classes.
- [Quantpedia: seasonalities in stock returns](https://quantpedia.com/seasonalities-in-stock-returns/),
  the restatement of cross-sectional seasonality and the ten-year persistence figures.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on what survives out of sample and how the evaluation should be done.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- bid-ask gap: the difference between the price at which you can sell and the price at which you can
  buy, which is a cost every time you trade.
- cross-section: a comparison of many things at one moment in time, as opposed to one thing over
  time.
- dollar volume: the number of shares traded in a period multiplied by the price, a measure of how
  heavily a share is traded.
- exchange-traded fund: a fund that holds a basket of shares and trades on an exchange like a single
  share.
- return: the change in price over a period, written as a fraction of the starting price.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- universe: the list of things a strategy is allowed to choose from.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
