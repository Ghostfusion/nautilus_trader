# Time-series momentum: judging each market by its own past rather than against the others

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Futures contracts on four groups at once: 24 commodities, 12 currency pairs, 9 developed equity indices and 13 developed government bond futures                                                                                                                                            |
| How often it trades       | Once a calendar month, when every position is reviewed                                                                                                                                                                                                                                      |
| What you need             | A spreadsheet, twelve months of monthly prices and a few months of daily prices for a list of futures                                                                                                                                                                                       |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/time-series-momentum-effect.py) and the [Quantpedia entry](https://quantpedia.com/strategies/time-series-momentum-effect/) it repeats                         |
| The underlying research   | Moskowitz, Ooi and Pedersen, [Time Series Momentum](http://pages.stern.nyu.edu/~lpederse/papers/TimeSeriesMomentum.pdf)                                                                                                                                                                     |
| How well it held up       | Mixed: the paper reports a large, multi-market result and Quantpedia grades it Strong, yet the list's own run returned a Sharpe ratio of 0.64 against the paper's claimed 1.31, and later research argues that much of the return is the volatility scaling rather than the momentum signal |
| Also appears in           | [Asset class trend following](../../quantconnect/asset-class-trend-following/README.md) and its close relative [Momentum and style rotation](../../quantconnect/momentum-and-style-rotation-effect/README.md) in this collection                                                            |

## The idea in one paragraph

Most momentum rules compare assets with each other: they buy whichever few rose the most and sell
short whichever fell the most, so there is always a winner and a loser. This strategy does not
compare at all. It looks at each market on its own, asks whether that market's own price rose or
fell over the past twelve months, and takes a position in that direction and no other. If almost
everything rose, it can be long almost everything; if almost everything fell, it can be short
almost everything. Each position is then sized so that calm markets get a bigger bet and wild ones
a smaller one, and the whole book is scaled to a chosen level of risk.

## Why anyone believed it

The story is the same under-reaction that drives ordinary momentum, but applied one market at a
time. When news reaches a market, the price adjusts slowly, so a market that has been rising tends
to keep rising for a while and a market that has been falling tends to keep falling. The
researchers find this persistence out to about a year and a partial reversal beyond it, and they
report that the winners are speculators and the losers are hedgers: the producers and consumers who
use futures to protect themselves against price moves are handing a premium to those who take the
other side.

The contrast with the cross-sectional version matters and is easy to miss. A cross-sectional rule is
dollar-neutral by design: it is long some assets and short others, and it profits from the gap
between them, so it makes money whether the market rises or falls. A time-series rule has no such
guarantee. It can be entirely long, entirely short or empty on either side, so it carries the risk
of the whole market direction, and it can be right about relative winners while losing money. The
reward for accepting that is that the time-series version can go short a falling market and can
profit when everything moves together, which is when a cross-sectional rule is least useful.

## An everyday comparison

Imagine judging swimmers not against each other but against their own earlier times. A swimmer whose
last few races were all faster than the ones before is improving, whatever the others are doing, and
you would bet on another good time. A swimmer whose times are slipping is declining, and you would
bet against them, even if they are still the fastest in the pool. The cross-sectional version would
instead always back the fastest few and bet against the slowest few; the time-series version backs
whoever is getting better and fades whoever is getting worse, and it is happy to back the whole
field.

## The rules, step by step

1. Assemble a list of futures contracts across the four groups. The paper and the code use 58 liquid
   instruments: 24 commodities, 12 currency pairs, 9 developed equity indices and 13 developed
   government bond futures.
2. At the end of each month, for every contract compute its return over the past twelve months,
   measured above the interest you could have earned with no risk. A positive number means the
   market rose more than cash; a negative number means it fell.
3. Decide the direction with the sign alone. If the past twelve-month return is positive, take a
   long position; if it is negative, take a short position. There is no ranking and no comparison
   between contracts.
4. Measure each contract's recent volatility from the last sixty trading days of daily returns, then
   multiply by the square root of 252 to put it on a yearly scale.
5. Size each position inversely to that volatility, so a contract that swings twice as much gets
   half the money, and normalise the positions in each leg so they add to one.
6. Scale the whole book so its expected volatility is a chosen target, ten percent a year in the
   code, with a cap of four times the money to stop the bet growing without limit.
7. Rebuild once a month, using the newest twelve-month return and the newest volatility estimate.
8. Pay the trading costs, the cost of rolling a futures position from one delivery month to the next,
   and the financing embedded in the futures price.

## The maths, with every symbol named

The signal is a sign, not a size:

```text
s_i = +1 if R_i is positive, and -1 otherwise
```

- `s_i` is the direction taken in contract `i`: plus one means long, minus one means short.
- `R_i` is contract `i`'s return over the past twelve months, measured as the excess return over cash.

The size comes from volatility:

```text
vol_i = standard deviation of the last 60 daily returns * square root of 252
```

- `vol_i` is contract `i`'s recent volatility, expressed as a yearly percentage.
- Multiplying the daily standard deviation by the square root of 252 converts it from a daily to a
  yearly scale, because a year holds about 252 trading days.
- The signal used in the source paper is a more elaborate estimate, called a GARCH model; the code
  replaces it with this simple version, which is why the paper's numbers and the code's differ.

The weight of each contract inside its leg, and then the whole book:

```text
raw_i = s_i / vol_i
w_i   = raw_i / (sum of raw over the same leg)
R     = L_long  * (sum of w_i * r_i over the long leg)
      + L_short * (sum of w_i * r_i over the short leg)
```

- `raw_i` is the inverse-volatility size, before normalising.
- `w_i` is the fraction of its leg given to contract `i`, so the weights in each leg add to one.
- `r_i` is contract `i`'s return over the following month.
- `L_long` and `L_short` are the two legs' scale factors, set to `target / leg volatility`; the code
  uses a target of 0.10 and caps each at four.

Costs are simpler than for shares because futures are not borrowed:

```text
Cost_month = gross_exposure * turn * c + roll + financing
```

- `gross_exposure` is the total size of the long and short legs together.
- `turn` is the fraction of that exposure traded in the month.
- `c` is the cost of one trade as a fraction of its value; futures are cheap, so 0.0005, five basis
  points, is realistic. One basis point is one hundredth of one percent.
- `roll` is the cost of moving a contract to the next delivery month, and `financing` is the interest
  embedded in the futures price; both are small for liquid contracts but not zero.

## A worked example

Six invented contracts, one from each corner of the world of futures, with plausible past returns
and volatility estimates.

| Contract     | Past 12-month return | Yearly volatility | Direction | Weight  |
| ------------ | -------------------- | ----------------- | --------- | ------- |
| Gold         | +0.18                | 0.12              | long      | 0.1778  |
| Crude oil    | +0.05                | 0.32              | long      | 0.0667  |
| Corn         | -0.12                | 0.22              | short     | -1.0000 |
| Euro         | +0.02                | 0.08              | long      | 0.2667  |
| 10-year note | +0.04                | 0.06              | long      | 0.3556  |
| S&P 500      | +0.15                | 0.16              | long      | 0.1333  |

Corn is the only market that fell, so it is the only short. In the long leg the inverse-volatility
raw sizes are `1 / 0.12 = 8.33`, `1 / 0.32 = 3.13`, `1 / 0.08 = 12.50`, `1 / 0.06 = 16.67` and
`1 / 0.16 = 6.25`, which add to 46.88. Gold's weight is therefore `8.33 / 46.88 = 0.1778`, the note
gets the largest weight because it is the calmest, and corn has a weight of one in the short leg
because it is alone. The example then scales each leg by two, standing in for a target of ten
percent when the unscaled book runs at five percent.

| Month | Long leg | Short leg | Gross    | Cost    | Net      | Cumulative |
| ----- | -------- | --------- | -------- | ------- | -------- | ---------- |
| 1     | +0.7512% | -0.5000%  | +2.5024% | 0.0600% | +2.4424% | +2.4424%   |
| 2     | -0.2334% | +0.3000%  | -1.0667% | 0.0600% | -1.1267% | +1.2881%   |
| 3     | +0.3600% | -0.4000%  | +1.5200% | 0.0600% | +1.4600% | +2.7670%   |
| 4     | +0.2289% | +0.2000%  | +0.0578% | 0.0600% | -0.0022% | +2.7647%   |
| 5     | -0.0111% | -0.3000%  | +0.5779% | 0.0600% | +0.5179% | +3.2969%   |

The long and short columns are the weighted sums of the contracts' returns in each leg. The gross
column is `2 * (0.7512 - (-0.5000)) = 2.5024` in month one, because the short leg's fall of 0.5
percent is a gain once it is subtracted, and the whole thing is doubled by the scale factor. The
cost is `4 * 0.30 * 0.0005 = 0.0006`, that is 0.06 percent a month, on a gross exposure of four
times the money with 30 percent of it turning over. The example ends about plus 3.3 percent over
five months. Nothing here says the strategy works; it shows how a single sign per market, a
volatility estimate and a scale factor combine into a portfolio return.

## What the research actually found

| Source and what it measured                                     | Result                                                                                                                                                                                                                                                |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Moskowitz, Ooi and Pedersen, 58 futures contracts, 1965 to 2009 | Persistence in returns from one to twelve months that partly reverses later; a diversified portfolio across all four groups delivers substantial abnormal returns with little exposure to standard risk factors, and performs best in extreme markets |
| Quantpedia's summary of the paper                               | 20.7 percent a year of estimated alpha at 15.7 percent volatility, worst fall 33.9 percent, a Sharpe ratio of 1.31 over 1965 to 2009, graded Strong                                                                                                   |
| Kim, Tse and Wald, removing the volatility scaling              | The estimated alpha falls from 1.27 percent a month with volatility-scaled weights to 0.41 percent without them, so much of the result is the risk-parity weighting rather than the momentum signal                                                   |
| Huang, Li, Wang and Zhou, testing each market on its own        | Little evidence of predictability in asset-by-asset regressions, and the profitable strategy performs about the same as a rule based on each market's historical average that requires no predictability at all                                       |
| The list's own measurement, on its own data, 1975 to 2026       | A Sharpe ratio of 0.64, annual return 8.63 percent, volatility 14.52 percent, worst fall 42.8 percent (the vendor's own measurement)                                                                                                                  |

The two sets of numbers are far apart, and the difference is the honest headline. The paper's
Sharpe ratio of 1.31 comes from a model-based volatility estimate, a particular construction and a
sample ending in 2009. The list's own run of the simpler version over 1975 to 2026 returns less than
half that, at a worst fall of about 43 percent, which is larger than the paper's 34 percent. The
scaling critique and the failed asset-by-asset tests point the same way: part of the measured
return is the way the positions are sized rather than the sign of the past return. The list's
aggregate record, a median replication Sharpe ratio of 0.37 across thousands of papers, is the
scale on which to read any single positive figure.

## How this project relates to it

The finished tutorial [Asset class trend following](../../quantconnect/asset-class-trend-following/README.md)
in this collection works the same single-asset-at-a-time idea on broad asset classes, using a moving
average rather than a fixed past return, and its relative
[Asset class momentum](../../quantconnect/momentum-and-style-rotation-effect/README.md) shows the
cross-sectional version that picks winners from a group, which is the contrast drawn above.

On the measurement side, [Stylized facts, tails and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md)
reads the research on volatility scaling and tail behaviour, which is exactly what decides how much
of this strategy's return comes from the signal and how much from the sizing.
[Options, hedging and derivative instruments](../../../strategies/books/16_options_and_derivative_instruments.md)
covers the mechanics of futures and the cost discipline a backtest of this kind needs, since every
position here is a contract that must be funded and rolled.

## Where it goes wrong

- The sizing does much of the work. Removing the volatility scaling cuts the measured alpha sharply,
  and the scaled and unscaled versions are not the same strategy, so it is easy to credit the signal
  for the sizing.
- The signal is weak market by market. Tests of each contract on its own find little predictability,
  and a rule that simply follows each market's historical average does about as well, which suggests
  the sign carries little information on its own.
- Flat markets cause whiplash. When a market drifts sideways, the twelve-month return flips between
  positive and negative month after month, so the position flips and the costs pile up.
- The answer depends on the calendar. Changing the starting day, the window length or the volatility
  estimator changes the result, and a rule that only works on one set of dates is not a rule.
- Futures are not free. Every position must be rolled and financed, and a strategy that is short a
  rising equity market pays the market's own upward drift, which is a real headwind.
- It can be long everything. Because there is no comparison, the book can accumulate the same
  directional risk everywhere, so a single bad month for the whole market hits every leg at once.

## Try it yourself

You need a spreadsheet and monthly closing prices for five or six markets you can find publicly,
plus a short rate for the cash return.

1. Put one market in each row, and paste about thirteen monthly closing prices across the row.
2. Add a column for the twelve-month return: the latest price divided by the price twelve months
   back, minus one, minus the yearly cash rate.
3. Add a column that writes long if that number is positive and short if it is negative.
4. Add a column for an approximate yearly volatility: the standard deviation of the last few monthly
   returns, multiplied by the square root of twelve.
5. Add a column for the raw size, one divided by the volatility, and a column for the weight, the raw
   size divided by the total in its leg.
6. Take next month's returns, multiply each market's return by its weight and its direction, add
   them up, and multiply by a scale factor of two. Subtract about 0.06 percent for costs.
7. Repeat each month for a year.

What to notice: on some months every market points the same way and the book is entirely long or
entirely short, and the outcome is dominated by the single biggest market move rather than by the
cleverness of the signs. That is the cross-sectional contrast in action, and it is why the sizing
and the target risk matter as much as the signal.

## Where this came from

- [The list's implementation of this strategy](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/time-series-momentum-effect.py),
  which states the four groups of contracts, the twelve-month sign, the inverse-volatility sizing and
  the target volatility.
- [Quantpedia: time series momentum effect](https://quantpedia.com/strategies/time-series-momentum-effect/),
  the performance figures, the confidence grade and the source-paper link.
- Moskowitz, Ooi and Pedersen, [Time Series Momentum](http://pages.stern.nyu.edu/~lpederse/papers/TimeSeriesMomentum.pdf),
  the paper behind the rule.
- Kim, Tse and Wald, [Time Series Momentum and Volatility Scaling](http://world-finance-conference.com/papers_wfc2/468.pdf),
  the test that separates the signal from the sizing.
- Huang, Li, Wang and Zhou, [Time-Series Momentum: Is It There?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3165284),
  the asset-by-asset test that finds little predictability.
- Maymin, Maymin and Fisher, [Momentum's Hidden Sensitivity to the Starting Day](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1899000),
  on how much the result depends on the calendar.
- The list's own measurement page at
  [paperswithbacktest.com/strategies/time-series-momentum](https://paperswithbacktest.com/strategies/time-series-momentum).
- [Stylized facts, tails and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md),
  this repository's brief on volatility scaling and tails.

## Words used in this tutorial

- derivative: a contract whose value depends on something else, such as a share, an index or a commodity.
- excess return: the return above what cash would have paid, which is what the sign is measured on.
- future: a standard contract to buy or sell something at a fixed price on a fixed future date.
- leverage: using borrowed money so that a given price move produces a larger gain or loss.
- momentum: the idea that what has recently risen tends to keep rising for a while, and the strategies that trade on it.
- volatility: how much a return moves around its average, usually quoted as a yearly percentage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
