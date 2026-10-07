# Mean reversion in country equity indexes: buying the markets that have done worst

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                            |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Funds that each track one country's main stock market index, bought when a country has done worst over three years and sold short when it has done best                                                                                                                                                          |
| How often it trades       | About once every three years, when the holding list is rebuilt                                                                                                                                                                                                                                                   |
| What you need             | A spreadsheet and three years of prices for a list of country index funds, plus a way to sell short                                                                                                                                                                                                              |
| Where the rules come from | [QuantConnect strategy library, mean reversion effect in country equity indexes](https://www.quantconnect.com/tutorials/strategy-library/mean-reversion-effect-in-country-equity-indexes) and the [Quantpedia entry](https://quantpedia.com/strategies/mean-reversion-effect-in-country-equity-indexes) it cites |
| The underlying research   | Richards, [Winner-Loser Reversals in National Stock Market Indices](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=883937), 1997                                                                                                                                                                            |
| How well it held up       | Weak: the original sample found reversals around a three-year horizon but could not explain them, and Quantpedia's own later test of the same rule on the same kind of data came out negative, which suggests the original result may have been chosen after the fact                                            |
| Also appears in           | [Momentum effect in country equity indexes](../momentum-effect-in-country-equity-indexes/README.md), which makes the opposite bet on the same data                                                                                                                                                               |

## The idea in one paragraph

Over any three-year stretch, some national stock markets do far better than others and some do far
worse. This strategy takes the four countries that did worst over the past three years and buys
them, and it takes the four that did best and sells them short, meaning it profits if their prices
fall. The money is split evenly, half on the losers and half against the winners, and the whole
portfolio is left alone for three years. At the end of that time it looks again and repeats. The bet
is that the extremes are temporary, so the countries that have been punished hardest tend to catch
up to the ones that have run furthest ahead.

## Why anyone believed it

The story is about overreaction. A country that has been through a currency crisis, a banking bust
or a political shock attracts fear, and investors sell it as if the trouble will never end. A
country that has boomed attracts admiration, and investors buy it as if the good years will never
stop. Both prices then move further from the country's normal level than the facts justify, and over
years they drift back. Economists call that drift mean reversion: the pull of a price back toward
its usual level.

The counterparty, then, is the crowd that chases what has just done well and abandons what has just
done badly, and, on the short side, the investor who borrowed money to buy a booming market and is
forced to sell when it turns. If those people keep appearing, the worst markets keep catching up.

## An everyday comparison

Think of two students and their test scores. One scores far above her usual level on one exam, the
other far below. Part of each result was skill and part was luck, and luck does not repeat, so on
the next exam the high scorer usually comes down and the low scorer usually comes up. Nothing about
either student changed; the extreme result simply pulled back toward normal. The strategy here is to
buy the country that scored unusually low over three years and sell the one that scored unusually
high, on the bet that both return toward their usual level.

## The rules, step by step

1. Choose a list of country index funds, each holding the main shares of one country. The library
   page uses nineteen such funds; the source paper used sixteen.
2. For each fund, compute its return over the past three years: take the price thirty-six months
   ago, take today's price, and divide. A fund that went from 100.00 to 80.00 has a three-year
   return of minus 20 percent.
3. Rank the funds by that return, best first.
4. Buy the four funds with the lowest return, and sell short the four funds with the highest return.
5. Give each of the eight chosen funds the same slice of the money. Half the money ends up on the
   four losers, held long, and half is bet against the four winners, held short. The two sides are
   equal, so the portfolio gains nothing from the market as a whole rising or falling and gains only
   from the losers beating the winners.
6. Hold for three years. Do not buy or sell in between.
7. At the end of the three years, recompute step 2 for all the funds and repeat. Sell everything and
   buy the new list.
8. To sell short is to borrow a fund you do not own, sell it, and buy it back later, hoping to pay
   less than you received. It requires a broker willing to lend the fund, and it loses money if the
   price rises. This tutorial describes the rule as written; doing it safely is a separate matter.

## The maths, with every symbol named

The strategy is one calculation repeated for each country fund, one sort, and a cost line.

The three-year return, called the reversal score:

```text
M = P_today / P_36_months_ago - 1
```

- `M` is the three-year return of the country fund, as a decimal: -0.20 means minus 20 percent.
- `P_today` is the fund's price today.
- `P_36_months_ago` is the fund's price three years earlier.

Rank the funds by `M`, from lowest to highest. Buy the four lowest and short the four highest, then
give each of the eight the same weight:

```text
w_i = +1 / 8 for each of the four lowest-return funds
w_i = -1 / 8 for each of the four highest-return funds
```

- `w_i` is the fraction of the money placed in fund `i`; a plus sign means bought, a minus sign
  means sold short.
- The four longs add up to +0.5 and the four shorts add up to -0.5, so the two sides are equal in
  size and the account is neither betting on markets rising nor on them falling.

The portfolio's return over the holding period is the sum of each weight times that fund's return:

```text
R_portfolio = (w_1 * R_1) + (w_2 * R_2) + ... + (w_8 * R_8)
```

- `R_i` is the return of fund `i` over the next three years.
- For the short positions a fall in price produces a positive contribution, because the weight is
  negative and so is the return.

The cost of rebuilding and the cost of borrowing:

```text
Cost = t * c + f
```

- `t` is the traded fraction: 2.0 when the whole portfolio is sold and replaced at once, because
  each sale and each purchase counts as one trade.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. For country funds a realistic figure is 0.0005 to 0.001, that
  is five to ten basis points, where one basis point is one hundredth of one percent.
- `f` is the fee paid to borrow the funds that are sold short, charged per year on the amount
  borrowed. A realistic figure for large country funds is 0.002 to 0.01 a year, that is 0.2 to 1
  percent a year.

## A worked example

The table below uses a made-up list of eight country funds. The rule is to buy the four worst and
short the four best. The returns are invented but are of the size national markets actually take
over three years.

| Country fund  | Price three years ago | Price today | Three-year return | Position    |
| ------------- | --------------------- | ----------- | ----------------- | ----------- |
| Argentina     | 30.00                 | 15.00       | -0.50 (-50 pct)   | buy, +1/8   |
| Russia        | 80.00                 | 48.00       | -0.40 (-40 pct)   | buy, +1/8   |
| Spain         | 100.00                | 80.00       | -0.20 (-20 pct)   | buy, +1/8   |
| Italy         | 90.00                 | 76.50       | -0.15 (-15 pct)   | buy, +1/8   |
| Canada        | 70.00                 | 77.00       | +0.10 (+10 pct)   | short, -1/8 |
| Germany       | 110.00                | 126.50      | +0.15 (+15 pct)   | short, -1/8 |
| Japan         | 120.00                | 150.00      | +0.25 (+25 pct)   | short, -1/8 |
| United States | 130.00                | 175.50      | +0.35 (+35 pct)   | short, -1/8 |

Now suppose the next three years produce these returns:

| Fund held             | Weight | Return over three years | Contribution   |
| --------------------- | ------ | ----------------------- | -------------- |
| Argentina (buy)       | +0.125 | +60 percent             | +7.500 percent |
| Russia (buy)          | +0.125 | +30 percent             | +3.750 percent |
| Spain (buy)           | +0.125 | +10 percent             | +1.250 percent |
| Italy (buy)           | +0.125 | +5 percent              | +0.625 percent |
| Canada (short)        | -0.125 | +20 percent             | -2.500 percent |
| Germany (short)       | -0.125 | +15 percent             | -1.875 percent |
| Japan (short)         | -0.125 | +5 percent              | -0.625 percent |
| United States (short) | -0.125 | +10 percent             | -1.250 percent |
| Total                 | 0.000  |                         | +6.875 percent |

So the portfolio gained 6.875 percent over the three years before costs. The losers gained 26.25
percent on average, the winners 12.50 percent, and the rule earns half the 13.75-point gap because
only half the money is on the long side.

The list turns over completely every three years, so the whole portfolio is traded on both sides:

```text
t = 2.0
Trading cost = 2.0 * 0.001 = 0.002, that is 0.20 percent every three years
Borrow fee = 0.005 per year * 0.5 borrowed * 3 years = 0.0075, that is 0.75 percent over three years
Net over three years = 6.875 - 0.20 - 0.75 = 5.925 percent, about 1.9 percent a year
```

Two things are worth noticing. First, the borrow fee on the short half is larger than the trading
cost, so the short side is not free. Second, the example says nothing about whether the strategy
works. It only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

The published record points in different directions, and the disagreement is the finding.

| Source                                       | What it measured                                                                          | Result                                                                                                                                                                             |
| -------------------------------------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Richards, the source paper                   | Winner-loser reversals in the national indices of 16 countries                            | Reversals are strongest around a three-year horizon; there is no evidence that the loser countries were riskier, and the small markets showed larger reversals than the large ones |
| Quantpedia, summarising the paper            | Long the four worst and short the four best, over 36 months, rebalanced every three years | 6.4 percent a year, worst fall 80.98 percent, over 1969 to 1995                                                                                                                    |
| Quantpedia's own verdict                     | The same rule tested later, outside the original sample                                   | The later test was significantly negative, and Quantpedia records that the original may have been chosen after seeing the data                                                     |
| Balvers, Wu and Gilliland                    | National index prices for 18 countries, 1969 to 1996                                      | Strong evidence of reversion, with a half-life of three to three and a half years, meaning a shock is half gone after that long                                                    |
| Spierdijk, Bikker and Van den Hoek           | Seventeen developed countries, 1900 to 2008                                               | Half-lives ranged from 2.1 to 23.8 years, and in many periods there was no meaningful reversion at all                                                                             |
| Smith and Pantilei, "Dogs of the World"      | Country index funds since 1971 and single-country funds since 1997                        | The rule produced higher average returns than a comparable market index, though with higher volatility                                                                             |
| Shi and Zhou, on Chinese and other exchanges | Cross-sectional contrarian effects, market by market                                      | The contrarian effect appears in some markets and periods and not others, and it depends on the market's state (p.5, p.7)                                                          |

Read together, the picture is this. Country-level reversion is real enough that several
independent studies find some version of it, and the half-life found by Balvers and his co-authors
sits close to the three years the rule uses. But the estimate of the half-life itself ranges from
about two years to more than twenty, many periods show no reversion at all, and the one attempt in
the record to test this exact rule on later data came out negative. That combination of a real
underlying tendency and an unreliable rule built on it is why the grade above is Weak rather than
Strong.

## How this project relates to it

This repository studies the opposite of reversion, and therefore the background this strategy
depends on, in [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md).
Section 5 of that document calls the question contrarian reversion and gives the condition this
strategy needs: over a few years the winners and the losers must swap places. Section 3.2 explains
why that matters, by showing that a ranking rule pays its costs with certainty and earns its return
only if relative performance depends on the past in a consistent way. Section 7.1 records a
pre-registered test on 2010 to 2026 data in which 24 of 24 rotation cells came out not confirmed,
which is the same warning as Quantpedia's negative out-of-sample result.

The research brief
[the momentum design brief](../../../strategies/books2/08_predictability_and_trading_strategies.md)
adds the measurement warning from the broader predictability literature: the number of rules tried
is itself a reason to distrust the best-looking result, which is precisely how Quantpedia reads the
original three-year reversal paper.

## Where it goes wrong

- Data mining. The original study looked at many countries and many horizons and reported the
  three-year one as strongest. Quantpedia's later test of the same rule came out negative, which is
  the signature of a result chosen after the fact.
- The three-year window is a choice, not a fact. Estimated half-lives run from about two years to
  more than twenty, so a rule that happens to use three years may be tuned to one sample.
- Reversion can end. The crises that create persistent losers can spread rather than fade, and the
  countries that are cheap can stay cheap for a decade or more.
- Short selling is dangerous and costly. A short position loses without limit if the price rises,
  the fund must be borrowed and paid for, and the smallest countries are the hardest and most
  expensive to borrow.
- Survivorship. The list of country funds reflects the countries that exist and have funds today;
  markets that closed, defaulted or left the index are missing, which flatters any historical test.
- The long side is still equity exposure. If the short side is dropped, the strategy becomes a bet
  on poorly performing markets, which is a different and riskier proposition than the rule
  describes.

## Try it yourself

You need nothing but a spreadsheet and a public source of country index prices.

1. Build a sheet with one column per country and one row per month for the last twenty years.
2. For each month, compute the three-year return: today's price divided by the price thirty-six
   rows up, minus one.
3. Split the countries into a best group and a worst group of equal size, as the rules describe.
4. For each month, record the average return over the following three years of the worst group and
   of the best group separately.
5. Subtract the best group's average from the worst group's average. That difference is what the
   rule tries to capture.

What to notice: the difference will be positive in some decades and negative in others, and it will
depend heavily on which countries you happened to include. If you move the look-back from three
years to four or to two, the sign of the average often flips. If your sheet shows the rule winning
by a wide margin, the likely cause is that you chose the look-back, the number of countries or the
period after seeing the answer.

## Where this came from

- [QuantConnect strategy library: mean reversion effect in country equity indexes](https://www.quantconnect.com/tutorials/strategy-library/mean-reversion-effect-in-country-equity-indexes),
  the rules as implemented: nineteen country funds, the 36-month ranking, the four worst bought and
  the four best sold short, rebalanced every three years.
- [Quantpedia: mean reversion effect in country equity indexes](https://quantpedia.com/strategies/mean-reversion-effect-in-country-equity-indexes),
  the indicative performance, the instrument count, the list of underlying papers and the weak
  confidence grade with its negative out-of-sample note.
- Richards, [Winner-Loser Reversals in National Stock Market Indices](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=883937),
  the source paper behind the rule.
- Balvers, Wu and Gilliland, the paper that estimates a three- to three-and-a-half-year half-life
  for national index prices.
- Spierdijk, Bikker and Van den Hoek, the century-long study that shows the half-life itself is
  unstable.
- Shi and Zhou, arXiv `1707.05552v1`, used here for its survey of how contrarian effects vary
  across markets and periods.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's own study, especially Sections 3.2 and 5.
- [the momentum design brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on predictability, decay and the cost of trying many rules.

## Words used in this tutorial

- contrarian: a strategy that bets against the recent crowd, buying what has just fallen or selling
  what has just risen.
- drawdown: the fall from a peak in value to a later low, measured as a percentage of the peak.
- half-life: the time it takes for half of a price's gap from its usual level to close.
- mean reversion: the idea that a price which has moved far from its usual level tends to come back.
- out-of-sample: data kept aside and not used while building a rule, then used once to test it
  honestly.
- rebalance: adjusting a portfolio back to its intended weights by buying and selling.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- universe: the full set of things a rule is allowed to choose from.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
