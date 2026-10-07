# The January barometer: staying in shares only after a rising January

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of large American companies, or short-term government bills as a safe parking place when the rule steps aside                                                                                                                                                                             |
| How often it trades       | Once a year, at the end of January; in between it does nothing                                                                                                                                                                                                                                   |
| What you need             | A spreadsheet and one year of monthly prices                                                                                                                                                                                                                                                     |
| Where the rules come from | [QuantConnect strategy library, January barometer](https://www.quantconnect.com/tutorials/strategy-library/january-barometer) and the [Quantpedia entry](https://quantpedia.com/strategies/january-barometer/) it cites                                                                          |
| The underlying research   | Cooper, McConnell and Ovtchinnikov, [What's the Best Way to Trade Using the January Barometer?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1436516)                                                                                                                                     |
| How well it held up       | Mixed: one very long American sample shows a large gap between years that followed a rising January and years that did not, but the gap is absent in the earlier half of that sample, vanishes in independent tests abroad, and shrinks toward nothing once trading a bill fund is accounted for |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                                                  |

## The idea in one paragraph

Every January, the broad American share market either finishes higher than it started or lower. This
rule looks at which one happened and then chooses where to keep the money for the remaining eleven
months. If January finished higher, the money stays in shares for the rest of the year. If January
finished lower, the money is moved into short-term government bills, which are a very safe place to
park cash, and stays there until the next January. During January itself the money is always in
shares, because January is what the rule is reading. The bet is that the direction of the first month
carries information about the direction of the year.

## Why anyone believed it

Two stories are usually offered. The first is about the calendar itself. In the last weeks of
December, many investors sell shares that have fallen in order to set the loss against tax, which
pushes prices down; when the selling stops in January, prices tend to bounce, and a strong January
may simply mean that the tax-driven selling was heavy the month before. The second story is about
attention. Pensions, savings plans and year-end bonuses are put to work in January, so the month's
tone is set by a wave of new money, and if that wave is strong the market hears it as a vote of
confidence that then feeds on itself.

The counterparty, in both stories, is the investor who reacts to the early tone rather than to
value: someone frightened by a weak January who sells at a price below what the shares are worth,
leaving the later buyer a bargain. Unlike the momentum story for shares, however, this account names
no forced seller and no slow reader of news, which is why the paper behind the rule is cautious about
whether the pattern has a reason at all.

## An everyday comparison

A teacher marks the first test of the year and uses the result to guess how the class will finish.
There is something in it: a class that did well in the first test is more likely to do well in June
than a class that failed it. But one test is a small sample, the same score can come about through a
single lucky or unlucky question, and the guess says nothing about the individual pupil. The January
barometer reads one month out of twelve in the same spirit.

## The rules, step by step

1. Choose one broad market fund. Both sources use the American S&P 500 index through its tracking
   fund, whose ticker is SPY.
2. On the first trading day of January, hold the full amount in that fund. If the money was parked in
   bills, sell the bills and buy the fund; if it was already in the fund, do nothing.
3. At the end of January, measure the January return: take the fund's price on the first trading day
   of January, take the price at the end of the month, and divide. A price that went from 400.00 to
   408.00 is a January return of plus 2 percent.
4. If the January return is greater than zero, keep the money in the market fund for the rest of the
   year, through December.
5. If the January return is zero or negative, sell the market fund and put the whole amount into a
   short-term government bill fund, whose ticker is BIL, and leave it there through December.
6. Repeat every January. There is no short selling in this version, and no second look at the prices
   during the year.
7. Note the one refinement the paper tested: it defines January as positive when the return is
   greater than zero, not merely better than the interest a bill would have paid, and it finds that
   convention works at least as well.

## The maths, with every symbol named

The rule turns on one number, the January return, and on one choice between two places to keep the
money.

The January return:

```text
J = P_end_of_January / P_start_of_January - 1
```

- `J` is the January return, written as a decimal: 0.02 means plus 2 percent, -0.03 means minus 3
  percent.
- `P_start_of_January` is the fund's price on the first trading day of January.
- `P_end_of_January` is the fund's price on the last trading day of January.

The return of the rule over the whole year then depends on `J`:

```text
If J > 0:   R_year = (1 + J) * (1 + r_rest) - 1
If J <= 0:  R_year = (1 + J) * (1 + b_rest) - 1
```

- `R_year` is the rule's return for the year, as a decimal.
- `r_rest` is the market fund's return from the start of February to the end of December.
- `b_rest` is the return of the bill fund over the same eleven months, a small positive number, for
  example 0.004 for four tenths of one percent.
- Multiplying the two pieces `(1 + J)` and `(1 + r_rest)` compounds them, the same as a price that
  rises in two steps.

The cost of the switch, on the years when the rule changes its holding, is:

```text
Cost = c
```

- `c` is the cost of one switch, as a fraction of the amount switched: the gap between the buying
  and selling price of the fund plus any commission. For a large market fund a realistic figure is
  0.0005 to 0.001, that is five to ten basis points, where one basis point is one hundredth of one
  percent. A year with no switch costs nothing, because the rule leaves the holding alone.

## A worked example

Eight made-up years, of the size that yearly market returns actually take. The rule's reading is
taken at the end of each January.

| Year | January return | Reading  | Market return, February to December | Bill return, 11 months | Rule return before cost | Switches | Cost   | Rule return after cost |
| ---- | -------------- | -------- | ----------------------------------- | ---------------------- | ----------------------- | -------- | ------ | ---------------------- |
| 1    | +2.5 percent   | positive | +8.0 percent                        | -                      | +10.70 percent          | 1        | 0.05pp | +10.65 percent         |
| 2    | -1.0 percent   | negative | -6.0 percent                        | +0.4 percent           | -0.60 percent           | 1        | 0.05pp | -0.65 percent          |
| 3    | +1.2 percent   | positive | +12.0 percent                       | -                      | +13.34 percent          | 1        | 0.05pp | +13.29 percent         |
| 4    | -3.0 percent   | negative | +14.0 percent                       | +0.4 percent           | -2.61 percent           | 1        | 0.05pp | -2.66 percent          |
| 5    | +4.0 percent   | positive | -10.0 percent                       | -                      | -6.40 percent           | 1        | 0.05pp | -6.45 percent          |
| 6    | +0.5 percent   | positive | +6.0 percent                        | -                      | +6.53 percent           | 0        | 0.00pp | +6.53 percent          |
| 7    | -2.0 percent   | negative | +9.0 percent                        | +0.4 percent           | -1.61 percent           | 1        | 0.05pp | -1.66 percent          |
| 8    | +3.0 percent   | positive | +5.0 percent                        | -                      | +8.15 percent           | 1        | 0.05pp | +8.10 percent          |

Here `pp` means percentage points of the whole account. In years 2, 4 and 7 the rule was parked in
bills, so it collected the small bill return instead of the market's February-to-December result.
Now compound the after-cost returns starting from 10,000.00:

| After year | Account   |
| ---------- | --------- |
| start      | 10,000.00 |
| 1          | 11,065.00 |
| 2          | 10,992.70 |
| 3          | 12,454.10 |
| 4          | 12,122.60 |
| 5          | 11,341.70 |
| 6          | 12,082.30 |
| 7          | 11,882.00 |
| 8          | 12,844.00 |

The rule ends with about 12,844.00, a gain of 28.4 percent over eight years, or roughly 3.2 percent a
year compounded. Two things are worth noticing. First, holding the market through all eight years
without the rule would have given about 14,878.00, because the rule sat in bills during year 4 and
year 7, two years in which a falling January was followed by a large rise. Second, the rule does
avoid some damage: it stepped aside for the whole of year 2, when the market fell. The example is not
evidence that the rule works; it shows how the arithmetic behaves and how easily the missed good
years outweigh the avoided bad ones.

## What the research actually found

The paper behind the rule measures a very long American record.

| Source                                                             | What it measured                         | Result                                                                                                                                                                                                                                                                                                                                                                            |
| ------------------------------------------------------------------ | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cooper, McConnell and Ovtchinnikov (2009), updating the 2006 study | American market, 1857 to 2008, 152 years | Across 100 positive Januaries the following 11 months averaged +11.01 percent; across 52 negative Januaries they averaged +2.84 percent, a gap of 8.17 percent with a t-statistic of 2.77. The gap is large and, over the full sample, statistically significant                                                                                                                  |
| The same paper                                                     | The best way to use the signal           | The best rule was not long-and-short but long-the-market after a positive January and parked in short-term bills after a negative one. That "long or bills" rule returned 10.38 percent a year against 9.98 percent for simply holding the market, with a lower standard deviation of 16.8 percent against 19.2, and a Sharpe ratio of 0.38 against 0.31                          |
| The same paper                                                     | Costs and taxes                          | The study states plainly that it includes neither. It also notes the rule would have been long the market during four of the five worst post-January stretches in 152 years, and out of the market for none of the five best                                                                                                                                                      |
| Quantpedia, the entry the library page cites                       | Its own summary of the same sample       | Indicative return 10.38 percent a year against a 9.98 percent benchmark, volatility 16.8 percent, Sharpe ratio 0.38, one instrument, backtest 1857 to 2008. It rates confidence "moderately strong" but records that an out-of-sample test is slightly negative and that the edge appears to be deteriorating                                                                     |
| Quantpedia, summarising independent work                           | Replications by other authors            | Marshall and Visaltanachoti find the strategy underperforms simply holding the market before and after adjusting for risk; Huang finds it adds nothing out of sample; Stivers, Sun and Sun conclude the effect is a United States phenomenon that has weakened over time. Quantpedia's own verdict is that the pattern is probably data mining and should be treated with caution |

The picture that comes out of these sources is narrow. The effect is real in the sense that a very
long American sample shows a large gap, and the gap is strongest after 1940 (a spread of 12.76
percent, t-statistic 3.57) and not statistically distinguishable from zero before 1940 (a spread of
4.37 percent, t-statistic 0.97). But the same long sample is the one that defined the rule, so it
cannot also be the clean test, and the studies that do look elsewhere, or later, do not confirm the
prize. The rule's value, on the paper's own account, comes from being out of the market during a few
bad years such as 2008, while it stayed invested through most bad years and all the good ones.

## How this project relates to it

This repository does not implement the January barometer. The closest thing is its own study of
return predictability,
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
That brief reviews a long-run test of a different timing signal, valuation ratios, and finds that it
fails a broad out-of-sample panel and that the portfolios it generates trade several times more than
a benchmark. Its general lesson applies directly here: a timing rule that looks strong in the sample
that inspired it must be re-tested on data the rule never saw and re-checked after costs, because
discovery is easy and survival is not.

The second useful piece is the foundations page [How to read a claim](../../foundations/10_how-to-read-a-claim.md),
which turns the same doubt into eight concrete questions and applies them to a market-timing claim
with arithmetic.

## Where it goes wrong

- One observation a year. The whole strategy is a single yes-or-no per year, so 152 years is only
  152 data points, and they are lopsided: 100 positive Januaries against 52 negative ones. A large
  average gap can rest on a handful of unusual years, and the standard error of an average of 152
  yearly numbers is wide.
- The sample that found it is the sample that tested it. The rule was chosen after the pattern was
  noticed in the same history, which is exactly the situation the foundations page warns about. The
  clean tests, in other countries and in later decades, come back empty or negative.
- The obvious short version fails. Following a negative January the market still rose on average
  over the next eleven months, so shorting it loses money; that is why the paper's own long-and-short
  version finishes far behind. Parked in bills, the rule avoids falls but also misses the bull years
  that often follow a bad January, such as the years after 2002 and 2008.
- It is still the market most of the time. The rule holds shares in January and, historically, in
  roughly two thirds of the following eleven-month stretches, so it is a slightly modified version of
  owning the market, not a hedge against one.
- No named counterparty. The stories about tax selling and January money flows do not identify who is
  forced to sell at a loss to the rule, and the paper itself says it offers no guarantee that the
  pattern persists.
- The effect can simply be the market's long rise. Over a century in which American shares rose
  strongly, almost any rule that is out of the market only occasionally will have a decent average
  return, which is why the comparison that matters is against holding the market, not against cash.

## Try it yourself

You need a spreadsheet and a public source of monthly prices for the market fund and a short-term
bill fund. Both the library page and any finance website will do.

1. Build one row per year for the last twenty-five years with these columns: year, price on the first
   trading day of January, price at the end of January, January return, the decision (shares or
   bills), the market return from February to December, and the bill return over those eleven months.
2. In the decision column write "shares" when the January return is positive and "bills" otherwise.
3. Compute the rule's return for each year: the January return compounded with the market return when
   the decision is shares, and with the bill return when the decision is bills.
4. Subtract a cost of 0.05 percent in every year in which the decision differs from the previous
   year's decision.
5. Add two more columns: the market's return for the whole year, and a running account total for the
   rule and for holding the market.

What to notice: how few years the rule changes anything, and how the outcome turns on two or three
years in which a falling January was followed by a strong rise. The rule will usually be close to the
market, sometimes ahead and sometimes behind, and the differences will be smaller than the gap the
long study reports. That difference is the honest sign that most of the reported edge comes from a
particular century rather than from a law.

## Where this came from

- [QuantConnect strategy library: January barometer](https://www.quantconnect.com/tutorials/strategy-library/january-barometer),
  the rules as implemented: hold the market fund in January, keep it after a positive January and
  switch to a bill fund after a negative one.
- [Quantpedia: January barometer](https://quantpedia.com/strategies/january-barometer/),
  the performance and risk figures, the sample period, the instrument count, the grade of confidence
  and the list of independent studies that fail to confirm the effect.
- Cooper, McConnell and Ovtchinnikov, [What's the Best Way to Trade Using the January Barometer?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1436516),
  the 1857 to 2008 test of the rule and of five ways to trade it, including the finding that the
  long-and-short version loses.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on which timing signals survive an out-of-sample test.
- [How to read a claim](../../foundations/10_how-to-read-a-claim.md), the checklist this tutorial
  applies in "Where it goes wrong".

## Words used in this tutorial

- treasury bill: a short-term loan to the government, repaid within a year, used as a safe place to
  park money; a bill fund such as BIL holds a rolling basket of them.
- return: the change in value of an investment over a period, expressed as a percentage.
- long: owning an investment, so that a price rise is a gain.
- short selling: borrowing something you do not own, selling it, and buying it back later, so that a
  price fall is a gain.
- index: a single number that tracks the combined value of many shares, such as the S&P 500.
- Sharpe ratio: the average return above the bill rate divided by how much the return wobbles; a
  higher number means more reward for the same wobble.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
