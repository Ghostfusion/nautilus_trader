# The return asymmetry effect in commodity futures: buying the raw materials whose bad days outnumber their good ones

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                               |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Futures contracts (agreements to buy or sell a fixed amount at a set date) on twenty-two raw materials, including corn, gold, cocoa, copper and crude oil                                                                                                           |
| How often it trades       | Once a month, when the ranking is redone and the portfolio is rebuilt                                                                                                                                                                                               |
| What you need             | A spreadsheet and twelve months of daily prices for each raw material                                                                                                                                                                                               |
| Where the rules come from | [Quantpedia, return asymmetry effect in commodity futures](https://quantpedia.com/strategies/return-asymmetry-effect-in-commodity-futures/)                                                                                                                         |
| The underlying research   | Durian and Padysak, [Return Asymmetry in Commodity Futures](https://ssrn.com/abstract=3918896), which uses the asymmetry measure of Jiang, Wu, Zhou and Zhu                                                                                                         |
| How well it held up       | Weak: rests on one study over 1991 to 2021 that reported 4.36 percent a year with a Sharpe ratio of 0.58, on the same account falling 47.55 percent at its worst, and on the measure being new enough that no independent sample was found in the sources used here |
| Also appears in           | [The skewness effect in commodities](../skewness-effect-in-commodities/README.md), the related strategy in this collection built on the third statistical moment                                                                                                    |

## The idea in one paragraph

Look at a raw material's daily price changes over the past year and count the days it moved unusually far. Some
materials produce more unusually large rises than unusually large falls, and others the opposite. Investors seem to
overpay for the chance of a large rise, in the way people overpay for a lottery ticket, so the materials with many
big rises are expensive and go on to earn less. This strategy buys the seven raw materials whose big falls have
outnumbered their big rises, sells short the seven with the opposite pattern, in equal amounts, and repeats each
month. It is a bet on the shape of each material's returns, not on its trend or its level.

## Why anyone believed it

A lottery ticket and a steady wage can have the same average value, yet people pay more for the lottery ticket. A
raw material with many large upward jumps looks like a lottery ticket: most of the time nothing much happens, and
occasionally it doubles. Investors who are drawn to that possibility push its price up, which lowers the return it
delivers afterwards. Materials whose big moves are mostly downward look the opposite way, are avoided, and must
offer a higher return to attract anyone.

The old theory of this is skewness, the third statistical moment, and the standard finding is that assets with
positive skewness, meaning a long right tail of gains, earn less than assets with negative skewness. The paper here
argues that skewness is noisy and hard to measure on a year of data, and replaces it with something simpler: count
the number of unusually large up days and unusually large down days, and subtract one from the other. The
counterparty is the investor who overpays for upside, and the investor who holds the material because they need it
physically and does not care about its statistical shape.

## An everyday comparison

Two farmers sell shares in their next crop. One grows a common vegetable that fetches a steady price, except in the
one year in a decade when it fails completely and the share is worthless. The other grows a fashionable fruit with
no floor and no ceiling, where most years are ordinary and one year in ten the price triples and the share soars.
Investors queue for the fashionable fruit, because the story of tripling is attractive, and they bid its price so
high that the average future return is poor. The strategy is to buy the boring vegetable and to avoid, or even bet
against, the fashionable fruit.

## The rules, step by step

1. Choose the twenty-two raw materials the paper uses: soybean oil, corn, cocoa, cotton, feeder cattle, gold,
   copper, heating oil, coffee, live cattle, lean hogs, natural gas, oats, orange juice, palladium, platinum,
   soybean, sugar, silver, soybean meal, wheat and crude oil. The implementation carries a slightly longer list,
   because it also holds some contracts the paper does not.
2. For each material, collect the last 260 daily prices. That is roughly one year of trading days.
3. Turn the prices into daily returns: today's price divided by yesterday's price, minus one.
4. Compute the average of those 260 daily returns.
5. Compute their standard deviation, which is a measure of how far the daily returns typically sit from that
   average.
6. Mark the day as an upside tail day if its return is greater than the average plus twice the standard deviation.
   Mark it as a downside tail day if its return is less than the average minus twice the standard deviation.
7. Count the upside tail days and the downside tail days. The asymmetry measure is the first count minus the
   second.
8. Rank the twenty-two materials by that measure, smallest first.
9. Buy the bottom seven, which are the materials whose big falls outnumber their big rises, giving each an equal
   share of the long side. Sell short the top seven, giving each an equal share of the short side. Selling short
   means borrowing something you do not own, selling it now, and buying it back later, hoping to buy it back
   cheaper.
10. Hold for one month. Then recompute from step 2 for every material and rebuild.

A note on the direction: the paper buys the lowest measure and sells the highest, because the high-measure
materials are the ones it believes are overpriced. The implementation does the same, sorting the measure from
smallest to largest and taking the first seven for the long side and the last seven for the short side.

## The maths, with every symbol named

The daily return of a material:

```text
r_t = P_t / P_t_minus_1 - 1
```

- `r_t` is the return on day `t`, written as a decimal: 0.02 means 2 percent.
- `P_t` is the price on day `t`.
- `P_t_minus_1` is the price on the day before.

The average and the standard deviation of the 260 daily returns:

```text
r_mean = (r_1 + r_2 + ... + r_N) / N
s = the square root of ( (1/N) * sum of (r_i - r_mean) squared )
```

- `r_mean` is the average daily return over the window.
- `N` is the number of daily returns, 260 in the paper.
- `s` is the standard deviation, the typical distance of a daily return from the average; a larger `s` means a
  wilder material.
- `sum` means add up the term for every day `i` from 1 to `N`.

The two thresholds and the measure itself:

```text
upper = r_mean + 2 * s
lower = r_mean - 2 * s
U = number of days with r_t greater than upper
D = number of days with r_t less than lower
IE = U - D
```

- `upper` and `lower` are the levels beyond which a day counts as extreme.
- `U` counts the extreme up days and `D` the extreme down days.
- `IE` is the asymmetry measure: positive means big rises dominate, negative means big falls dominate.
- The name comes from the word itself: `IE` stands for the imbalance between the two tails, the lopsidedness of the
  distribution of returns. Two times the standard deviation is the usual rough cut, because if daily returns were
  shaped like the familiar bell curve, only about 2.3 percent of days would fall beyond it, which is roughly six
  days out of 260.

## A worked example

Nine materials, each with 260 daily returns reduced to the four numbers that matter. The numbers are invented but of
the size commodity returns actually take.

| Material    | Average daily return | Standard deviation | Upper threshold | Lower threshold | Up days U | Down days D | IE  | Rank |
| ----------- | -------------------- | ------------------ | --------------- | --------------- | --------- | ----------- | --- | ---- |
| Cocoa       | 0.05 percent         | 1.5 percent        | 3.05 percent    | -2.95 percent   | 4         | 12          | -8  | 1    |
| Copper      | 0.01 percent         | 1.4 percent        | 2.81 percent    | -2.79 percent   | 5         | 12          | -7  | 2    |
| Corn        | 0.01 percent         | 1.3 percent        | 2.61 percent    | -2.59 percent   | 4         | 8           | -4  | 3    |
| Live cattle | 0.01 percent         | 0.9 percent        | 1.81 percent    | -1.79 percent   | 5         | 7           | -2  | 4    |
| Gold        | 0.03 percent         | 1.0 percent        | 2.03 percent    | -1.97 percent   | 6         | 6           | 0   | 5    |
| Wheat       | 0.00 percent         | 1.7 percent        | 3.40 percent    | -3.40 percent   | 7         | 6           | +1  | 6    |
| Sugar       | 0.02 percent         | 1.6 percent        | 3.22 percent    | -3.18 percent   | 8         | 5           | +3  | 7    |
| Coffee      | 0.04 percent         | 2.0 percent        | 4.04 percent    | -3.96 percent   | 9         | 4           | +5  | 8    |
| Silver      | 0.02 percent         | 2.2 percent        | 4.42 percent    | -4.38 percent   | 10        | 3           | +7  | 9    |

The paper holds seven of twenty-two, about a third of the universe; this example holds three of nine, also about a
third. So the long side is cocoa, copper and corn, one third of the long money in each, and the short side is
sugar, coffee and silver, one third of the short money in each. Now suppose the next month brings these returns.

| Material | Side  | Weight | Next-month return | Contribution |
| -------- | ----- | ------ | ----------------- | ------------ |
| Cocoa    | long  | 0.3333 | +2.0 percent      | +0.6667      |
| Copper   | long  | 0.3333 | +1.0 percent      | +0.3333      |
| Corn     | long  | 0.3333 | -1.0 percent      | -0.3333      |
| Sugar    | short | 0.3333 | -2.0 percent      | +0.6667      |
| Coffee   | short | 0.3333 | +1.0 percent      | -0.3333      |
| Silver   | short | 0.3333 | -3.0 percent      | +1.0000      |
| Total    |       | 2.0000 |                   | +2.0000      |

The contributions are in percent of the whole account. On the short side a fall is a gain, so sugar at minus 2.0
percent adds 0.6667 and silver at minus 3.0 percent adds 1.0000. The gross return for the month is +2.0 percent.
The portfolio holds 100 percent long and 100 percent short, so the money at work is twice the account. Replacing
every position completely trades four times the account's size, because each leg is closed and reopened:

```text
cost = 4 * 0.001 = 0.004, that is 0.4 percent of the account
net return for the month = 2.0 - 0.4 = 1.6 percent
```

- The 0.001 is ten basis points, that is 0.10 percent, the assumed gap between the buying and selling price of a
  liquid futures contract plus commission, and it is charged on each side.

The short positions here are futures, which do not require borrowing the underlying material, so there is no borrow
fee. If the same bets were made by borrowing and selling shares or by using a contract-for-difference, the seven
short positions would also pay a monthly borrow or financing fee, which for less liquid materials can exceed the
trading cost. Twelve months at 1.6 percent a month, compounded, would be about 21 percent a year before any month
in which the ranking does not change and the cost is lower. The published figure is 4.36 percent a year, which is
far below this example, and that gap is the point: a single lucky month can look like a strategy while the average
month does not.

## What the research actually found

| Source                                                     | What it measured                                                                         | Result                                                                                                                                                         |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Durian and Padysak, Return Asymmetry in Commodity Futures  | Buying the bottom seven and shorting the top seven of twenty-two materials, 1991 to 2021 | 4.36 percent a year, an annualized volatility of 7.53 percent, a worst fall of 47.55 percent and a Sharpe ratio of 0.58                                        |
| The same paper                                             | Whether the ranking is useful across many portfolio constructions                        | The high-measure materials were overvalued and the low-measure ones undervalued, and the resulting portfolios were negatively correlated with the stock market |
| Wu, Zhu and Chen (2020), quoted in the supporting material | The same measure on Chinese shares                                                       | The spread between the highest and lowest group was minus 0.81 percent a month on a value-weighted basis, the same direction as here but a different market    |
| Jiang, Wu, Zhou and Zhu, quoted in the same material       | The measure itself on American shares                                                    | The measure was related to, but not the same as, skewness, which is why the paper uses it                                                                      |

The list that carries this strategy does not publish its own Sharpe ratio for it. Its general replication record,
which is the vendor's own measurement, is a median Sharpe ratio of 0.37 across the papers it has coded, with 48
percent of them clearing a t-statistic of 1.96 and a median test window of 34 years. The 4.36 percent a year with
a Sharpe ratio of 0.58 sits above that median, but it comes from one thirty-year sample, one ranking measure and
one set of thresholds, so it is not the same as being replicated.

## How this project relates to it

The repository's survey of commodity research is
[Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md). It shows that the shape of
a commodity's future price curve, not just its past returns, carries information: the level, slope and curvature of
the curve of twenty-one futures explained on average 96.5 percent of the variation in those curves, and the slope
was the profitable part. A ranking that ignores the curve, as this strategy does, is therefore measuring only one
of the forces at work.

The second link is
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
That brief reports that cross-sectional predictors of this kind are mostly real rather than the product of
excessive searching, but that they weaken once positions are value-weighted and that many published findings are
only a few percent a year. It also warns that the number of rules tried is a binding constraint, which is precisely
the risk in a strategy with a threshold (twice the standard deviation) and a holding count (seven) that could be
varied until the results looked good.

## Where it goes wrong

- Two years of the same measure. Because the measure counts rare days, a material with no extreme days at all
  scores zero, which is the same score as a material with one up day and one down day. The measure throws away a
  lot of information about the middle of the distribution.
- The thresholds are arbitrary. Twice the standard deviation and seven names are choices. Different choices would
  produce different winners, and choosing them after seeing the results is how a strategy is fitted to its past.
- Crowding and shorting costs. The short leg holds materials that are rising and are easy to borrow only while
  everyone agrees they are expensive. When a shortage arrives the price soars, the short loses badly, and the
  materials can become hard or expensive to borrow exactly when the strategy needs them.
- Sample dependence. The published figures come from 1991 to 2021, a period that included several commodity booms
  and a long quiet stretch. A single different decade could plausibly give a different answer.
- Trading costs on a monthly rebuild. The paper's own construction trades the whole book every month. At ten basis
  points per side over four turns of the account, that is about 4.8 percent a year, which is larger than the
  reported return and would erase it for a small account.
- The measure is new. It was proposed in the 2020s and this is close to its first test in commodities. With one
  sample, a negative finding would be as easy to explain as a positive one.

## Try it yourself

You need a spreadsheet and a public source of daily prices for a handful of commodities; a finance website or a
futures exchange will provide them.

1. Build a sheet with one column per commodity and one row per trading day for the last two years.
2. Add a separate block that turns each pair of consecutive prices into a daily return.
3. For each commodity, compute the average and the standard deviation of its returns over the most recent 260 rows.
4. Add two columns, one for the average plus twice the standard deviation and one for the average minus twice it.
5. Add two counting columns, one counting the days above the upper level and one counting the days below the lower
   level, and subtract them to get the measure.
6. Rank the commodities by the measure, and note which three are lowest and which three are highest.
7. In the row below, average the next month's returns of the three lowest and separately of the three highest.

What to notice: the count of extreme days is tiny, often single digits out of 260, so a single additional extreme
day can move a commodity by several ranks. That sensitivity is the honest weakness of the measure. If your ranking
swings wildly from month to month, the signal is probably noise rather than information.

## Where this came from

- [Quantpedia, return asymmetry effect in commodity futures](https://quantpedia.com/strategies/return-asymmetry-effect-in-commodity-futures/),
  the rules, the performance figures and the reference to the underlying study.
- The implementation file the list carries,
  `static/strategies/return-asymmetry-effect-in-commodity-futures.py`, whose header states the twenty-two materials,
  the 260-day window, the two-standard-deviation thresholds and the seven-name long and short sides.
- Durian and Padysak, [Return Asymmetry in Commodity Futures](https://ssrn.com/abstract=3918896), the study the
  rules come from.
- [Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md) and
  [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's surveys of the commodity and predictability evidence.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- futures contract: an agreement to buy or sell a fixed amount of something at a set price on a set date.
- long: owning something, so that you gain when its price rises.
- rank: the position of a value in a sorted list, where rank 1 is the smallest or largest as stated.
- short selling: borrowing something you do not own, selling it now, and buying it back later.
- standard deviation: a measure of how far values typically sit from their average; a larger number means more
  spread out.
- tail: the far ends of a distribution, where the unusually large or unusually small values live.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
