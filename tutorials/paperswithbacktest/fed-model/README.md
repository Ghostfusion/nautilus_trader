# The Fed model: comparing what shares earn with what government bonds pay

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A broad American share-market fund, held when a forecast is positive and swapped for a short-term government bond fund when it is not                                                                                                                                                                                                                                   |
| How often it trades       | About once a month, and only when the forecast changes sign                                                                                                                                                                                                                                                                                                             |
| What you need             | A spreadsheet and three published numbers: a share market's earnings yield, the ten-year government bond yield, and share prices                                                                                                                                                                                                                                        |
| Where the rules come from | [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), which keeps the coded rule, and the [Quantpedia Fed model entry](https://quantpedia.com/strategies/fed-model/) it cites                                                                                                                                        |
| The underlying research   | Asness, [The Fed Model and Expected Asset Returns](https://paperswithbacktest.com/strategies/the-fed-model-and-expected-asset-returns)                                                                                                                                                                                                                                  |
| How well it held up       | Disputed: credible researchers reach opposite conclusions, with some showing the gap between share earnings and bond interest forecasts later returns and others showing that it fails exactly where it is supposed to work and that its logic compares two unlike numbers                                                                                              |
| Also appears in           | No other tutorial in this collection states the Fed model; the nearest are [Price-earnings anomaly](../../quantconnect/price-earnings-anomaly/README.md), which uses the same earnings figure without the bond comparison, and [Can crude oil predict equity returns](../../quantconnect/can-crude-oil-predict-equity-returns/README.md), another market-timing overlay |

## The idea in one paragraph

Shares earn profits, and bonds pay interest, and investors can hold either. This strategy turns the share
market's profits into a percentage of its price, turns the bond's interest into a percentage of its price,
and looks at the difference. When shares earn a lot more than bonds pay, the strategy holds the share
market; when they do not, it moves the money into short-term government bonds. It checks the comparison
each month and switches only when the forecast flips. The bet is that the two percentages are rival offers
for the same money, so the one offering more should attract buying and rise.

## Why anyone believed it

For most savers, bonds and shares are the two places money goes, so comparing what each offers is the first
thing a careful investor does. If a government bond pays five percent and the whole share market, taken
together, earns only four percent of its price in profits, shares look expensive; if it earns eight
percent, they look cheap. The story says that investors noticing this will move money from the worse offer
to the better one, so the gap predicts which way the share market goes next.

The counterparty is the investor who is forced to be in one or the other: a pension fund that must hold
bonds to match its promises, or a fund that must hold shares to track an index. Their need to transact
keeps the prices away from the level a patient, unconstrained buyer would pay, which is the room the timing
rule hopes to exploit.

## An everyday comparison

Suppose you are choosing between two small shops to buy. The first earns five pounds of profit a year for
every hundred pounds of price, and the second earns four and a half pounds. On those numbers alone the
first looks better value. But the first shop's profits can grow as prices rise, while the second is a
vending machine on a fixed contract that pays the same number of pounds no matter what happens to prices.
Comparing five pounds with four and a half ignores that the two incomes are not the same kind of thing,
and that is exactly the criticism levelled at this model. The earnings from shares grow with the cost of
living; the interest from a bond does not.

## The rules, step by step

The coded rule uses a broad American share-market fund and a short-term government bond fund.

1. Each month, find the share market's earnings yield: the total profits of the companies in the index
   divided by the total price of the index, written as a percentage.
2. Find the yield on ten-year government bonds: the interest the government promises, divided by the price
   of the bond, written as a percentage.
3. Take the natural logarithm of each, and subtract the second from the first. The result is the yield gap.
4. Using all the monthly data available so far, fit a straight-line relationship that predicts the next
   month's share market move from the yield gap. At least six months of data are needed before the first
   trade, in the coded rule.
5. Use the fitted line and the current yield gap to forecast next month's share market move.
6. If the forecast is positive, hold the share-market fund. If it is negative, hold the short-term
   government bond fund instead. Hold all of one or all of the other, never a mix.
7. Repeat each month, switching only when the sign of the forecast changes.
8. If the earnings-yield data stops arriving for more than about a month, sell everything and wait.

## The maths, with every symbol named

Two yields, a difference in logarithms, and a fitted straight line.

The share market's earnings yield:

```text
EY = E / P
```

- `EY` is the earnings yield, written as a decimal: 0.05 means five percent.
- `E` is the total profit the companies earn, per share.
- `P` is the price of the index, per share.
- This is the inverse of the better-known price-to-earnings ratio: if shares cost twenty times their yearly
  profit, the earnings yield is one divided by twenty, or five percent.

The bond yield:

```text
y = the yearly interest a ten-year government bond pays, divided by its price
```

- `y` is the bond yield as a decimal: 0.045 means four and a half percent.
- It is the return a buyer would receive by holding the bond to maturity, if the bond is repaid as
  promised.

The yield gap:

```text
YG = ln(EY) - ln(y)
```

- `YG` is the yield gap.
- `ln` is the natural logarithm, a function that turns a ratio into an additive difference; using it means
  the gap behaves like a percentage difference between the two yields rather than a raw subtraction.
- A positive `YG` means shares earn more than bonds pay, and a larger `YG` means a wider gap.
- The source paper writes the same idea with one added to each yield, `ln(1 + E/P) - ln(1 + y)`, which is
  almost the same number when the yields are small; the coded rule uses the simpler form above.

The forecast, from a straight line fitted to history:

```text
forecast = a + b * YG
```

- `forecast` is the predicted share-market move for the coming month.
- `a` is the height of the fitted line, found from the data.
- `b` is its slope, also found from the data; it says how many percent the forecast rises for each unit of
  the yield gap.
- The line is fitted by least squares: among all straight lines, the one whose predictions come closest to
  the actual past monthly moves is chosen.

The position:

```text
if forecast > 0:  hold the share-market fund
if forecast < 0:  hold the short-term government bond fund
```

- The whole account is in one of the two at all times.

Costs, as always:

```text
Cost = t * c
```

- `t` is the traded amount, counted on both sides. A switch from the share fund to the bond fund and back
  is a traded amount of 2 each time, because the old holding is sold and the new one bought.
- `c` is the cost per trade, about 0.0002 to 0.0003 for large, cheaply traded funds, that is two to three
  basis points, where one basis point is one hundredth of one percent.

## A worked example

To keep the arithmetic short, use a fixed fitted line, as if the data had produced it: `forecast = 0.004 +
0.9 * YG`. The yields and the market moves are invented but of the size these numbers take. Figures are in
percent except the yields and the gap, which are decimals.

| Month | Earnings yield EY | Bond yield y | Yield gap YG | Forecast | Position | Next-month result |
| ----- | ----------------- | ------------ | ------------ | -------- | -------- | ----------------- |
| 1     | 0.050             | 0.040        | +0.2231      | +0.2048  | shares   | +2.0              |
| 2     | 0.052             | 0.042        | +0.2136      | +0.1962  | shares   | -1.0              |
| 3     | 0.045             | 0.048        | -0.0645      | -0.0541  | bonds    | +0.2              |
| 4     | 0.048             | 0.046        | +0.0426      | +0.0423  | shares   | +1.5              |
| 5     | 0.044             | 0.050        | -0.1278      | -0.1110  | bonds    | +0.2              |
| 6     | 0.047             | 0.045        | +0.0435      | +0.0431  | shares   | +0.8              |

The yield gap in month 1 is the logarithm of 0.050 divided by 0.040, which is the logarithm of 1.25, or
0.2231. The forecast is the fitted line applied to it: 0.004 plus 0.9 times 0.2231, which is 0.2048, a
positive number, so the account is in shares. The "next-month result" column shows what the held fund did,
so in month 3 the account was in bonds and earned 0.2 percent while shares rose 0.5 percent, which the
account missed; in month 5 it was in bonds again and avoided a 2.0 percent fall in shares.

Adding the six results:

```text
Total = 2.0 - 1.0 + 0.2 + 1.5 + 0.2 + 0.8 = 3.7 percent
```

Now the costs. The position switched five times, at the start of months 3, 4, 5 and 6 and once before the
table began, so the account was fully sold and rebuilt five times:

```text
t = 5 switches * 2 sides = 10
Cost = 10 * 0.0003 = 0.003, that is 0.3 percent
Net for the six months = 3.7 - 0.3 = 3.4 percent
```

That is six invented months. It shows how to apply the rules and how the arithmetic behaves, and it says
nothing about whether the idea works. Two things are worth noticing. First, the rule sat in bonds for two
of the six months and earned almost nothing, so the account's fate was decided by a handful of switches.
Second, the cost of switching is small here only because the rule changed its mind rarely; a version that
switched every month would pay several times as much.

## What the research actually found

| Source                                                                          | What it measured                                                 | Result                                                                                                                                                                                                                                         |
| ------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Asness, The Fed Model and Expected Asset Returns                                | The yield gap against later share and bond returns, 1959 to 2003 | The gap forecast positive excess share returns, more strongly at short horizons, and an investment rule built on it produced a higher reward for risk than simply holding shares or bonds                                                      |
| Quantpedia, summarising the source paper                                        | The coded timing rule, 1959 to 2003                              | 11 percent a year, volatility about 10 percent, worst fall 50.3 percent, reward-to-risk 0.7; the plain share market returned a similar 11.1 percent a year but with volatility of 15.14 percent, so the rule matched the return with less risk |
| Quantpedia's own note on the idea                                               | The coded rule, after the source sample                          | Confidence graded Moderate, and Quantpedia reports that its out-of-sample test showed slightly negative performance, with the apparent alpha deteriorating                                                                                     |
| Asness, Fight the Fed Model                                                     | The model's logic and its forecasting power                      | The model compares a real number with a nominal one, because share earnings grow with inflation while bond interest does not; and it fails the test of forecasting long-term returns, while a simple price-to-earnings comparison passes       |
| Maio, The Fed Model and the Predictability of Stock Returns                     | The yield gap against later returns                              | The gap forecasts positive excess market returns at short and long horizons, beats competing forecasting measures, and has some power outside the original sample                                                                              |
| The awesome-systematic-trading list's replication record, across all its papers | 4,843 coded strategies                                           | The median replication returned a Sharpe ratio of 0.37, and only 48 percent cleared a t-statistic of 1.96, so half the published record cannot be distinguished from zero on its own sample                                                    |

The disagreement is the finding. The same measure, on overlapping data, is defended by one group of
researchers and rejected by another, and international tests of the same idea are mixed. That is why the
grade for this tutorial is Disputed rather than Strong or Weak: the sources do not agree, and the
disagreement is not settled.

## How this project relates to it

This repository's brief on rates,
[Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), explains how a
government bond yield is built and what moves it, which is the input on the bond side of this comparison;
its Section 1 is the closest material to that.

The brief on research integrity,
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
is the more important companion here: its Section 2 explains how trying many versions of a forecasting
rule and keeping the best one produces a rule that looks better than it is, which is exactly the risk with
a model as widely varied as this one.

The finished tutorial closest to the idea is
[Price-earnings anomaly](../../quantconnect/price-earnings-anomaly/README.md), which uses the share market's
earnings yield on its own, without the bond comparison, and therefore tests the half of this model that its
critics say is the sound half.

## Where it goes wrong

- It compares two unlike numbers. The earnings of companies rise with inflation over time, while the
  interest on a bond is fixed in the money of the day. Comparing a growing percentage with a fixed one is
  the central theoretical objection, and several credible researchers make it decisively.
- The relationship is unstable. Over long stretches of history shares yielded more than bonds, and in
  recent decades they have yielded less. A rule that leans on the gap changing its meaning over the decades
  is leaning on something that has already shifted.
- The evidence points both ways. One school shows the gap forecasting later returns, another shows it
  failing exactly the long-horizon test it is supposed to pass. A reader cannot resolve this from the
  published work, and should not pretend to.
- It is a market timing overlay, not a way to beat the market by a wide margin. The vendor's own figures
  show a return close to simply holding shares, with less risk, and that comparison is the fair one.
- The measured window is old. The source study ends in 2003, and the vendor's own test outside the sample
  turned slightly negative, which is the relevant warning for anyone reading it today.
- For the whole idea to be false, it is enough that the gap reflects investors' own changing appetite for
  risk rather than a mispricing, so that the forecast is really a measure of the mood, and the mood is
  already in the price. The published disagreement is consistent with exactly that.

## Try it yourself

You need a spreadsheet and three public numbers: a share index's earnings yield, the ten-year government
bond yield, and the index's price. All three are published monthly by finance sites and by the government.

1. Build a sheet with one row per month for the last twenty years. Columns: the price, the earnings yield,
   the bond yield, and a column that is the logarithm of the earnings yield minus the logarithm of the bond
   yield.
2. Add a column holding the next month's change in the price, calculated from the row below.
3. Split the rows into two groups: those where the yield gap was positive and those where it was negative.
   Average the next-month price change in each group.
4. Now make the groups finer: the top quarter by yield gap, and the bottom quarter. Average the next-month
   change in each.
5. Finally, plot the yield gap and the next-month change against each other and look at the cloud of points.

What to notice: if the gap had real forecasting power, the top-quarter group would clearly beat the
bottom-quarter group. The clouds from step 5 are usually a shapeless blob, which is what a weak or absent
relationship looks like. Now do the same exercise using the earnings yield alone, ignoring the bond yield,
as the price-earnings tutorial does. Whichever version separates the groups more cleanly is the one the
data is actually supporting, and seeing which one it is, with your own arithmetic, is the point of the
exercise.

## Where this came from

- [FED Model](https://quantpedia.com/strategies/fed-model/), the page that states the rules and reports the
  source-paper figures and the confidence grade.
- Asness, [The Fed Model and Expected Asset Returns](https://paperswithbacktest.com/strategies/the-fed-model-and-expected-asset-returns),
  the study behind the coded rule.
- Asness, [Fight the Fed Model](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=381480), the paper that
  argues against the model.
- Maio, [The Fed Model and the Predictability of Stock Returns](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=889931),
  the paper that defends it.
- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading),
  which holds the coded rule and the project's own replication record.
- [Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md) and
  [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's briefs on yield curves and on how trying many rules misleads.

## Words used in this tutorial

- earnings yield: the yearly profit of a company or an index divided by its price, the inverse of the
  price-to-earnings ratio.
- excess return: the return of something above the return of a safe alternative.
- inflation: the rate at which the general level of prices rises, which erodes the value of fixed payments.
- least squares: the standard way of fitting a straight line by choosing the one whose misses are smallest
  overall.
- market timing: moving money in and out of the market to try to be invested when it rises and out when it
  falls.
- nominal: a number measured in the money of the day, without adjusting for inflation, unlike a "real"
  number.
- yield: the yearly income something pays, divided by its price, written as a percentage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
