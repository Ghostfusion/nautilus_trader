# The Fama-French five-factor model: splitting a share's return into five common influences

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | It is a measuring tool rather than a trade; the library page turns it into a portfolio of American shares, five held long and five held short                                                                           |
| How often it trades       | Every thirty days in the library's version                                                                                                                                                                              |
| What you need             | A spreadsheet, or Python and a data file, plus company accounts and the published factor figures                                                                                                                        |
| Where the rules come from | [QuantConnect strategy library, Fama French five factors](https://www.quantconnect.com/tutorials/strategy-library/fama-french-five-factors)                                                                             |
| The underlying research   | Fama and French, [A Five-Factor Asset Pricing Model](https://www.sciencedirect.com/science/article/pii/S0304405X14002323)                                                                                               |
| How well it held up       | Mixed: the model is a standard reference point, but two of its newer influences add little outside the original sample and the value influence became redundant in the authors' own tests                               |
| Also appears in           | [The price to earnings ratio](../price-earnings-anomaly/README.md) and [expected idiosyncratic skewness](../expected-idiosyncratic-skewness/README.md) in this collection, which use the same value and size influences |

## The idea in one paragraph

When you look at the return of a share or a portfolio, you are seeing the sum of several things. Some
of it is the whole market moving, some is the behaviour of small companies against large ones, some
is cheap companies against expensive ones, some is profitable companies against unprofitable ones,
and some is thrifty companies against free-spending ones. The Fama-French five-factor model says
that these five influences, which they call factors, explain most of the return, and that whatever
is left over is the part that is not explained. To use it as a strategy, rank shares on five company
measures, buy the ones that score best on all five, sell short the ones that score worst, and repeat
every month. The strategy is a demonstration of the model, not a claim that the model earns money by
itself.

## Why anyone believed it

A single influence, the market, explained surprisingly little. Two shares in the same market often
move very differently, and much of that difference lines up with how big the company is and whether
it is cheap or expensive. That observation is the reason factor models exist: once you know a share
tends to move when small companies move, you can set that part aside and look at what is left.

The story behind each factor is that the extra return is either a reward for a risk or a mistake that
persists. Cheaper and less profitable companies are often in trouble, so their excess return can be
payment for the danger of holding them. Small companies are less liquid, so their excess return can
be payment for the difficulty of trading them. The counterparty, in the telling, is the cautious
investor who avoids the awkward companies and the awkward corners of the market, and who is willing
to pay up for comfort.

## An everyday comparison

Think of how a person's monthly salary changes. Part of it moves with the whole company, when the
company has a good year and hands out raises to everyone. Part moves with their department, when the
sales team does better than the engineers. Part moves with their own grade, when a promotion lands.
A model that only used the company-wide number would mispredict most people badly. The five-factor
model is the equivalent of writing the salary as a company part, a department part, and a few
personal parts, and then noticing that whatever is left after all of those is either luck or a
personal difference worth investigating.

## The rules, step by step

1. Build a universe of American shares that have published accounts and trade above 5.00. The
   library page keeps the most heavily traded names in its data.
2. For each share you need five company measures: total equity (a size measure), book value per
   share (a value measure), operating profit margin (a quality measure), return on equity (a
   profitability measure), and the growth of total assets (an investment measure).
3. Rank the universe separately by each of the five measures. For four of them the best score goes
   to the highest value; for the growth of total assets the best score goes to the lowest value,
   because companies that spend less are preferred by the model.
4. Give each share a single score: the average of its five rankings, each ranking first multiplied
   by a chosen weight. The library page allows a different weight for each of the five.
5. Buy the five shares with the highest scores, an equal amount in each.
6. Sell short the five shares with the lowest scores, an equal amount in each. Selling short means
   borrowing a share you do not own, selling it, and buying it back later; if the price falls you
   gain, and if it rises you lose.
7. Hold for thirty days, then recompute the five measures and rebuild both lists.

One caution about the library page. Its code labels the five measures with the five factor names,
but a size factor is not the same thing as total equity, and a market factor is not book value per
share. The page says plainly that it is showing "one possible combination" of measures, and that is
how it should be read: a demonstration of the idea, not a faithful test of the published model.

## The maths, with every symbol named

The model writes the return of a share, or of a portfolio, as a sum of parts.

```text
R - Rf = alpha + b * MKT + s * SMB + h * HML + r * RMW + c * CMA + e
```

- `R` is the return of the share or portfolio over the period.
- `Rf` is the return of a short-term government bill, treated as the safe alternative.
- `R - Rf` is the excess return, the amount earned above the safe rate.
- `MKT` is the market excess return: the return of the whole market minus the safe rate.
- `SMB` means small minus big: the return of a basket of small companies minus the return of a
  basket of large ones.
- `HML` means high minus low: the return of companies that are cheap relative to their book value
  minus the return of companies that are expensive relative to it.
- `RMW` means robust minus weak: the return of highly profitable companies minus the return of
  weakly profitable ones.
- `CMA` means conservative minus aggressive: the return of companies that reinvest little minus the
  return of companies that reinvest a lot.
- `b`, `s`, `h`, `r` and `c` are the factor loadings, also called betas: how strongly the share
  moves when each influence moves. A loading of 1.2 on the market means the share tends to move 1.2
  percent for each 1 percent the market moves above the safe rate.
- `alpha` is the average part of the return the five influences do not explain.
- `e` is the random wobble around the fitted line on any single period.

Each factor's value is itself a difference between two baskets, so SMB, for example, is written:

```text
SMB = (return of small-company baskets) - (return of large-company baskets)
```

The equation then says something simple. Take what the five influences would have produced, given
the share's loadings, and compare it with what the share actually did. The gap is `alpha`. If `alpha`
is reliably positive, the share beat what its influences would predict.

The library's ranking version is a different calculation, not a regression. Each share is sorted
five times and given a point score:

```text
score = (b * rank_equity + s * rank_book + h * rank_margin + r * rank_roe + c * rank_growth) / 5
```

- `rank_equity` and the others are each share's position in the corresponding sorted list, written
  so that the most preferred share gets the highest number.
- `b`, `s`, `h`, `r`, `c` are the weights the page lets you choose; if all are equal the score is
  just the average ranking.

The ten shares with the highest and lowest scores are then bought and sold short.

## A worked example

One share with the following loadings: `b` = 1.1, `s` = 0.6, `h` = 0.3, `r` = 0.2, `c` = -0.1. The
negative value on `c` means the share behaves a little like an aggressive spender. The factor
returns below are invented, but they are of the sizes these factors actually take month to month.
All figures are percentages.

| Month | MKT  | SMB  | HML  | RMW  | CMA  | Fitted excess return | Actual excess return |
| ----- | ---- | ---- | ---- | ---- | ---- | -------------------- | -------------------- |
| 1     | 2.0  | 0.5  | -0.3 | 0.4  | 0.1  | 2.48                 | 2.60                 |
| 2     | -1.0 | 0.2  | 0.6  | -0.2 | 0.3  | -0.87                | -1.00                |
| 3     | 3.0  | -0.4 | 0.2  | 0.5  | -0.2 | 3.24                 | 3.00                 |
| 4     | 0.5  | 0.1  | -0.5 | 0.0  | 0.2  | 0.44                 | 0.50                 |
| 5     | -2.0 | 0.3  | 0.4  | 0.6  | 0.1  | -1.79                | -2.10                |
| 6     | 1.5  | 0.0  | 0.1  | -0.3 | 0.0  | 1.62                 | 1.40                 |

The fitted column is the loading times the factor, added up. For month 1:

```text
Fitted = 1.1 * 2.0 + 0.6 * 0.5 + 0.3 * (-0.3) + 0.2 * 0.4 + (-0.1) * 0.1
       = 2.20 + 0.30 - 0.09 + 0.08 - 0.01 = 2.48 percent
```

Now average both columns over the six months. The fitted average is 0.853 percent and the actual
average is 0.733 percent, so the leftover, the alpha, is about -0.12 percent a month. In words, over
these six months the share did slightly worse than its five exposures predicted. A single alpha from
six months is not meaningful; the point of the exercise is to show how the leftovers are computed.

Now the cost of the library's version. It holds five shares long and five short and rebuilds every
thirty days, so if all ten change:

```text
t = 2 * 1.0 = 2.0
Cost per month = 2.0 * 0.001 = 0.002, that is 0.2 percent
Cost per year = 0.2 * 12 = 2.4 percent
```

That annual cost is larger than the annual return the library's backtest reports below. A monthly
rebuild is expensive, and any comparison of these factors must include it.

## What the research actually found

| Source                                                                       | What it measured                                                                | Result                                                                                                                                                                                                                                                                  |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fama and French, A Five-Factor Asset Pricing Model                           | American shares, 1963 onward                                                    | The five-factor model describes average returns better than the older three-factor model; its main failure is small companies that invest a lot despite low profits, and once profitability and investment are added the value factor becomes redundant in their sample |
| Quantpedia, investment factor                                                | Long companies that reinvest little, short those that reinvest a lot, 1963-2013 | 3.54 percent a year, worst fall 19.81 percent, rebuilt yearly; the site grades its confidence as moderate and notes that an out-of-sample test was slightly negative                                                                                                    |
| Quantpedia, value factor                                                     | Long cheap shares, short expensive ones, 1926-2014                              | 3.6 percent a year, worst fall 55.99 percent, reward-to-risk 0.3                                                                                                                                                                                                        |
| Zhang, Yu, Ma and Yao, on China's A-share market (`2112.03170v1`, p.9, p.11) | 2005 to 2020, 25 size and value and size and investment portfolios              | Size, market and profitability were significant; value was not (a test statistic of -1.35) and investment was not (1.33), so both looked redundant there; the five-factor model still fitted better overall                                                             |
| The library page's own backtest                                              | Five long, five short, thirty-day rebuild, January 2010 to August 2019          | About 6.8 percent a year with a worst fall of 19.8 percent; the page says the factors used may not capture enough information and invites improvement                                                                                                                   |

Read together, the picture is this. The five-factor model is a serious and widely used way to
describe returns, and it is the vocabulary that most later work is written in. But two of its five
influences are fragile. The value influence, which was the star of the older model, became redundant
in the authors' own tests, and the investment influence has been weak out of sample and absent in
some markets. This is a model for explaining what happened, and a shaky foundation for a rule that
promises what will happen.

## How this project relates to it

This repository's portfolio and allocation brief has a section on factor models and the construction
of the test,
[factor models and the construction of the test](../../../strategies/books2/10_portfolio_and_allocation.md).
It reports a study finding that whether a factor model looks good depends heavily on the portfolios
used to test it: the same model can come first under one rebalancing rule and last under another. The
same brief's reading list points at the China A-share comparison cited above, and its cross-section
section reports that most published factors are probably real but that their measured power shrinks
once the shares are weighted by size and adjusted for other factors.

The predictability brief,
[the predictability and trading strategies brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
adds the same warning from the other direction: the number of factors tried in the literature is
large, value-weighting and factor adjustment raise the estimated share of false findings, and ten
thousand significant signals are not ten thousand independent bets.

## Where it goes wrong

- Two of five influences may not be real. The investment factor was weak out of sample in the
  Quantpedia data and insignificant in the China sample, and the value factor became redundant in
  the authors' own tests. If those two are noise, the model is being carried by its older parts.
- The size of a company and its cheapness travel together. Value and size are correlated, and
  profitability and investment are correlated with each other. Attributing a return to one influence
  when several move together is a judgement, not a measurement.
- The test portfolios decide the answer. Change how shares are grouped or weighted and the ranking of
  models changes, as the repository brief reports. A model that wins on one set of test portfolios
  can lose on another built from the same data.
- Alpha is not proof of skill. A positive leftover can mean a factor was missed, a data error, or
  luck. The model is fitted, and the coefficients were chosen to fit, so the leftovers in the sample
  used to fit it are smaller than they will be later.
- Short selling changes the trade. The library version sells short, which requires borrowing shares,
  pays a fee, and can lose far more than the money committed. The measured effect does not survive
  added borrow costs and cannot be traded by anyone who cannot short.
- The factors are published and famous. A premium that has been in textbooks for decades has been
  available to everyone for decades, which is a reason to expect it to be smaller, not larger.

## Try it yourself

You need nothing but a spreadsheet and a public source of the five factor series, which are
published free by the data library maintained at Dartmouth College.

1. Download the monthly series for the market, small minus big, high minus low, robust minus weak,
   and conservative minus aggressive, together with the safe rate.
2. Build a sheet with one row per month and one column per factor.
3. Add columns that compute a made-up portfolio's fitted return: choose loadings, say 1.0 on the
   market and 0.5 on small minus big and 0.3 on high minus low and 0.2 on robust minus weak and 0
   on conservative minus aggressive.
4. Multiply each factor's monthly value by its loading and add the results.
5. Graph the fitted column against the actual return of some real fund you can find published
   monthly returns for.

What to notice: for a fund that is broadly invested, the fitted line tracks the actual one closely,
and the two diverge in particular stretches. For a fund run by a person making active choices, the
gaps are larger. That gap, averaged over many months, is the alpha, and it is almost always small
and noisy. If your graph shows a large and steady gap, suspect the loadings were chosen after
looking at the answer.

## Where this came from

- [QuantConnect strategy library: Fama French five factors](https://www.quantconnect.com/tutorials/strategy-library/fama-french-five-factors),
  the rules as implemented: five company measures, a weighted ranking, five long and five short,
  rebuilt every thirty days.
- Fama and French, [A Five-Factor Asset Pricing Model](https://www.sciencedirect.com/science/article/pii/S0304405X14002323),
  the original model and the finding that the value factor becomes redundant once profitability and
  investment are added.
- [Quantpedia: investment factor](https://quantpedia.com/strategies/investment-factor) and
  [Quantpedia: value factor](https://quantpedia.com/strategies/value-book-to-market-factor),
  the performance figures, volatility, worst falls, instrument counts and out-of-sample notes.
- Zhang, Yu, Ma and Yao, [A revised comparison between FF five-factor model and three-factor model](https://arxiv.org/abs/2112.03170),
  the China A-share test and its test statistics.
- [The portfolio and allocation brief](../../../strategies/books2/10_portfolio_and_allocation.md)
  and [the predictability and trading strategies brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's own studies of factor models and the factor literature.

## Words used in this tutorial

- alpha: the average return left over after the influences in a model are accounted for.
- excess return: the return above the safe rate, or above a comparison rate.
- factor: a common influence that pushes many shares in the same direction at once.
- factor loading: how strongly a share moves when one influence moves, also called a beta.
- market cap: the total value of a company's shares, price times number of shares.
- regression: a method of fitting a line to data so that the average error is as small as possible.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
