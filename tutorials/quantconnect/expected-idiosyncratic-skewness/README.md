# Expected idiosyncratic skewness: ranking shares by how lopsided their unusual moves are

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                         |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A basket of American shares, buying the small slice whose unusual moves are expected to be least lopsided towards jackpot gains                                               |
| How often it trades       | Once a month, when the slice is rebuilt                                                                                                                                       |
| What you need             | Python and a data file, plus the published market, size and value factor figures                                                                                              |
| Where the rules come from | [QuantConnect strategy library, expected idiosyncratic skewness](https://www.quantconnect.com/tutorials/strategy-library/expected-idiosyncratic-skewness)                     |
| The underlying research   | Boyer, Mitton and Vorkink, [Expected Idiosyncratic Skewness](https://academic.oup.com/rfs/article-abstract/23/1/169/1578688)                                                  |
| How well it held up       | Weak: the effect rests on one influential paper and one short implementation sample, with no independent out-of-sample replication shown here                                 |
| Also appears in           | [Fama-French five factors](../fama-french-five-factors/README.md) in this collection, whose three older influences are what this strategy strips out to isolate the leftovers |

## The idea in one paragraph

Between the daily ups and downs, shares differ in the shape of their unusual moves. One share might
mostly drift and then, rarely, jump up a lot; another might drift and then, rarely, fall a lot. That
lopsidedness is called skewness. Shares with many small losses and the odd huge gain feel like
lottery tickets, and this strategy's argument is that investors pay too much for that feeling, so
such shares earn less on average than shares whose rare big moves point the other way. The strategy
measures how lopsided each share's unusual moves have been, predicts how lopsided they are likely to
be next month, ranks every share on that prediction, and buys the slice expected to be the least
lottery-like.

## Why anyone believed it

Some investors enjoy the lottery. They will accept a lower average return for the chance of a
jackpot, in the same way that people buy lottery tickets with a negative average payoff because the
dream is worth something to them. If enough investors feel that way, the shares that offer the
jackpot dream are bought up until they are expensive, and expensive means a lower future return. The
shares with the opposite lopsidedness, the ones with the occasional big drop, have to offer a higher
return to persuade anyone to hold them.

The counterparty, then, is the lottery-seeking investor who buys the jackpot share, and the
risk-averse investor who refuses the crash-prone share unless paid. The two groups pull in the same
direction for this strategy: they make the lottery-like shares dear and the crash-like shares cheap,
so a rule that buys the crash-like shares and shuns the lottery-like ones is positioned on the
cheap side. There is a second and competing explanation in academic finance, that the crash-like
shares earn more simply because they are genuinely more dangerous, in which case the extra return is
payment for risk rather than a mistake to exploit.

## An everyday comparison

Think of two charity raffles at the same fair. The first sells cheap tickets for small, frequent
prizes: you usually win a little and almost never lose much. The second sells expensive tickets for
one enormous prize: almost everyone wins nothing, but a single winner takes a fortune. People queue
for the second raffle even though the first returns more money on average per ticket, because the
enormous prize is exciting to imagine. The stallholders know this and price the second raffle's
tickets higher relative to what they pay out. The strategy here is to ignore the exciting raffle and
buy tickets in the boring one.

## The rules, step by step

1. Build a universe of American shares that have published financial data and trade at a reasonable
   price, ranked by the value traded daily, keeping the top two hundred by liquidity.
2. For every trading day, work out each share's excess return: its return minus the return of a
   short-term government bill.
3. Fit the three-factor relationship for each share using the past month of daily data: its excess
   return against the market excess return, and against the size and value baskets. The part of the
   return the three influences do not explain is the residual, and it is called idiosyncratic
   because it is specific to the share.
4. From that month's residuals, compute two summary numbers for each share: the idiosyncratic
   volatility, which is how large the residuals usually are, and the historical idiosyncratic
   skewness, which is how lopsided they are.
5. At the end of each month, look across all shares and fit a simple straight-line rule that
   predicts each share's skewness from its own past skewness and its past volatility. This is what
   turns history into a forecast.
6. Use that rule to compute an expected idiosyncratic skewness for every share for the coming month.
7. Rank the shares from the lowest expected skewness to the highest.
8. Buy the lowest 5 percent, that is the bottom slice of the ranking, weighting each share by its
   total market value so bigger companies get more money.
9. Hold for one month, then recompute everything and rebuild.

## The maths, with every symbol named

Skewness measures how lopsided a set of numbers is. Compute it as the average of the cubed distances
from the average, divided by the cube of the usual wobble:

```text
skewness = average of ((value - average)^3) / (standard deviation)^3
```

- `value` is one observation, for example one day's idiosyncratic residual.
- `average` is the mean of those observations.
- `standard deviation` is the usual size of the wobble around the average.
- Cubing the distance makes large deviations count far more than small ones, and keeps their sign, so
  a few big gains push the number positive and a few big losses push it negative.

The first step is to isolate the idiosyncratic part. Fit each share's excess return to three common
influences over the past month of daily data:

```text
R_i - R_f = alpha_i + beta_i * (R_M - R_f) + s_i * SMB + h_i * HML + e_i
```

- `R_i` is the return of share `i` on that day.
- `R_f` is the safe rate; `R_M` is the market return.
- `SMB` is the return of small companies minus large ones; `HML` is cheap companies minus expensive.
- `beta_i`, `s_i` and `h_i` are the share's sensitivities to those three influences.
- `e_i` is the residual: the part of the day's return the three influences do not explain.

From the residuals of the month, the two summary numbers are:

```text
idiosyncratic volatility = square root of the average of the squared residuals
```

```text
historical skewness = (average of the cubed residuals) / (idiosyncratic volatility)^3
```

- The first says how big the typical leftover move was.
- The second says how lopsided those leftover moves were, in units of the typical size.
- The library page writes the divisor of the second formula as the volatility raised to the power
  1.5, which is not the usual standardisation; the version above is the standard one, and the
  difference is worth knowing if you compare the two.

Then the forecast. At the end of each month, fit a straight line across all shares:

```text
skewness_now = b0 + b1 * skewness_last_month + b2 * volatility_last_month + error
```

- `b0`, `b1` and `b2` are the fitted coefficients, recomputed every month from that month's data.
- The fitted line says, on average across shares, how this month's skewness relates to last month's
  skewness and volatility.

Finally, apply the same line to each share's current numbers to get the forecast:

```text
expected skewness next month = b0 + b1 * skewness_now + b2 * volatility_now
```

- The result is a single number per share. Sorting these numbers and buying the lowest 5 percent is
  the whole strategy.

## A worked example

Eight shares, each with a historical idiosyncratic skewness and a historical idiosyncratic
volatility, both in decimal form. Suppose the month's cross-sectional fit gave `b0` = 0, `b1` = 0.4
and `b2` = 0.2; these coefficients are invented for the example, and a real fit produces different
ones each month. The rules buy the lowest 5 percent of the universe; the example buys the lowest two
of these eight to keep the table short.

| Share | Past skewness | Past volatility | Expected skewness | Rank |
| ----- | ------------- | --------------- | ----------------- | ---- |
| J     | -0.9          | 0.03            | -0.354            | 1    |
| K     | -0.4          | 0.04            | -0.152            | 2    |
| L     | -0.2          | 0.02            | -0.076            | 3    |
| M     | 0.0           | 0.03            | 0.006             | 4    |
| N     | 0.3           | 0.05            | 0.130             | 5    |
| O     | 0.6           | 0.04            | 0.248             | 6    |
| P     | 0.9           | 0.06            | 0.372             | 7    |
| Q     | 1.4           | 0.05            | 0.570             | 8    |

The expected number is the coefficient times the input, added up. For share J:

```text
Expected = 0 + 0.4 * (-0.9) + 0.2 * 0.03 = -0.36 + 0.006 = -0.354
```

Shares J and K have the two lowest expectations, so they are bought. Suppose their total market
values are 100 and 900, so under value weighting J gets 0.1 of the money and K gets 0.9. Now suppose
the next month gives J a return of +6 percent and K a return of +1 percent:

| Share | Weight | Next-month return | Contribution   |
| ----- | ------ | ----------------- | -------------- |
| J     | 0.100  | +6 percent        | +0.600 percent |
| K     | 0.900  | +1 percent        | +0.900 percent |
| Total | 1.000  |                   | +1.500 percent |

So the basket gained 1.5 percent before costs, against 0.5 percent for the market. If the whole
slice is replaced at the monthly rebuild, the traded fraction is 2.0 and the cost is:

```text
Cost = 2.0 * 0.001 = 0.002, that is 0.2 percent
Net return for the month = 1.500 - 0.200 = 1.300 percent
```

Notice that the whole result came from one share, K. With only two holdings the outcome is dominated
by whichever share moved most, which is exactly why the published version uses the bottom 5 percent
of a large universe rather than a handful of names.

## What the research actually found

| Source                                                                 | What it measured                                                                      | Result                                                                                                                                                                                                                                     |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Boyer, Mitton and Vorkink, Expected Idiosyncratic Skewness             | American shares, using a fitted model to forecast skewness                            | Shares with low expected skewness earned a monthly return, after adjusting for three common influences, that exceeded the high group by 1.00 percent; past volatility was a stronger predictor of skewness than past skewness itself       |
| The library page's own backtest                                        | Two hundred liquid shares, long the lowest 5 percent, monthly, July 2009 to July 2019 | A reward-to-risk ratio of 0.947 against 0.87 for the market fund over the same period, so a modest improvement over holding the index                                                                                                      |
| Quantpedia, multi-asset skewness                                       | Long the lowest-skewness and short the highest-skewness assets                        | Equity index futures returned 0.96 percent a year with 11.34 percent volatility from 2000 to 2020, while currencies returned 1.76 percent; a levered combination across asset classes reached 7.67 percent a year at 15 percent volatility |
| Barunik and Nevrla, common idiosyncratic factors (`2208.14267v5`, p.1) | American shares, the tails of the leftovers rather than the whole shape               | A premium of about 7 to 8 percent a year for shares exposed to the lower tail of idiosyncratic moves, which survives the standard factors; the sign agrees with this strategy but the method is different                                  |

Read together, the picture is this. The theory is clean and the original measurement is striking, but
the evidence here is thinner than for the value or momentum ideas. The library's own test covers a
ten-year stretch in which the market rose and shows only a small edge over the index, the original
number comes with no independent replication, and the neighbouring research uses a different measure
and reports a different size. Two opposite stories, overpaying lottery buyers and paid-for crash
risk, both predict the same direction, so the strategy's success would not by itself tell you which
story is true.

## How this project relates to it

This repository's portfolio and allocation brief has a section on the third moment of returns,
[the third-moment extension](../../../strategies/books2/10_portfolio_and_allocation.md). It reports
a study that replaces the usual symmetric assumption with a lopsided one and finds that the standard
mean-and-variance recipe is then no longer the right tool: volatility and skewness have to be handled
together. That is the practical consequence of this tutorial's idea, seen from the portfolio side
rather than the share-picking side.

The predictability brief,
[the predictability and trading strategies brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
adds a warning from a different corner. Its section on momentum and skewness reports that the
skewness a strategy shows depends on the holding horizon and on the rule that opens and closes the
trades, not only on the shares held. Read beside this tutorial, it means a measured skewness is a
property of a set of choices as much as of a share.

## Where it goes wrong

- The forecast is fitted to the past. The coefficients that predict next month's skewness are
  estimated from previous months, and in calm periods they will say everything is calm. When the
  market turns violent, the forecast lags and the ranking loses its meaning exactly when it matters.
- Skewness is hard to measure. It is the third moment, so it is driven by a few extreme days, and a
  handful of observations can flip its sign. Two different windows, or two different ways of
  standardising, give two different rankings, and choosing between them after seeing results is a
  hidden degree of freedom.
- The value weighting concentrates the bet. Weighting by market value means the few largest shares in
  the bottom slice dominate the basket, so the strategy's result over a short sample is often the
  story of a handful of companies rather than the story of skewness.
- The two explanations are not the same trade. If the extra return is payment for crash risk, then a
  strategy that buys the crash-prone shares is buying the crashes along with the return, and its
  worst months will be very bad. If it is overpricing, the return is a mistake being corrected. The
  same numbers fit both stories.
- One sample and one implementation. The striking original result and the modest library backtest
  are the whole of the record shown here. There is no second paper reaching the same number on a
  separate sample, so the honest reading is that this idea is promising rather than established.
- Costs on a monthly rebuild. Buying the bottom slice every month trades a large share of the
  account, and the bottom slice is chosen to be the shares other people avoid, which is often where
  the gap between the buying and the selling price is widest.

## Try it yourself

You need nothing but a spreadsheet, a public source of daily prices for a list of companies, and the
published size and value factor series.

1. Build a sheet with one row per company per day and columns for the company's return, the market
   return, and the size and value factor returns.
2. For each company, use the spreadsheet's regression tool, or its slope function, to estimate how
   much it moves with each of the three factors.
3. Add a column for the residual: the company's return minus the fitted part from the three factors.
4. Over one month of residuals, compute the average and the wobble, then the skewness using the
   formula in this tutorial. A spreadsheet can do the cubing.
5. Repeat for every company and rank them by skewness, lowest first.
6. Record what the bottom fifth and the top fifth of the ranking did over the following month.

What to notice: the bottom and the top of the ranking look different when you list them. The top
usually contains small, exciting, fast-moving names, and the bottom usually contains dull, large
ones. That difference is a warning as much as the point: skewness is tangled up with company size,
and if the bottom slice is simply a collection of large companies, the exercise may be measuring
size rather than lopsidedness.

## Where this came from

- [QuantConnect strategy library: expected idiosyncratic skewness](https://www.quantconnect.com/tutorials/strategy-library/expected-idiosyncratic-skewness),
  the rules as implemented: two hundred liquid shares, a monthly forecast, the lowest 5 percent held
  long, and the reward-to-risk figures.
- Boyer, Mitton and Vorkink, [Expected Idiosyncratic Skewness](https://academic.oup.com/rfs/article-abstract/23/1/169/1578688),
  the original measurement and the 1.00 percent monthly figure.
- [Quantpedia: multi-asset skewness trading strategy](https://quantpedia.com/multi-asset-skewness-trading-strategy/),
  the cross-asset figures, including the equity and currency returns and the levered combination.
- Barunik and Nevrla, [Common Idiosyncratic Quantile Factors and Asset Prices](https://arxiv.org/abs/2208.14267),
  the lower-tail premium of 7 to 8 percent a year.
- [The portfolio and allocation brief](../../../strategies/books2/10_portfolio_and_allocation.md)
  and [the predictability and trading strategies brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's own studies of the third moment and of skewness as a design property.

## Words used in this tutorial

- idiosyncratic: specific to one share, as opposed to shared with the whole market.
- market value: the total value of a company's shares, price times number of shares.
- residual: the part of a move that the fitted relationship does not explain.
- skewness: how lopsided a set of numbers is, with rare large gains on one side and rare large
  losses on the other.
- standard deviation: the usual size of the wobble of a set of numbers around their average.
- three-factor model: a way of describing a share's return using the market, size and value.
- value weighting: giving each holding an amount of money in proportion to its market value.
- volatility: how much a price moves around its average, usually reported per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
