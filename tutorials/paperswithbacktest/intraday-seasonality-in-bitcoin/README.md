# Intraday seasonality in Bitcoin: the hours of the day when the price behaves differently

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                     |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Bitcoin, priced in American dollars on a cryptocurrency exchange                                                                                                                                                                                          |
| How often it trades       | Every day: one two-hour holding in the evening, and no position for the other twenty-two hours                                                                                                                                                            |
| What you need             | A spreadsheet and two years of hourly Bitcoin prices                                                                                                                                                                                                      |
| Where the rules come from | [Quantpedia, intraday seasonality in Bitcoin](https://quantpedia.com/strategies/intraday-seasonality-in-bitcoin/)                                                                                                                                         |
| The underlying research   | Padysak and Vojtko, [Seasonality, Trend-following, and Mean reversion in Bitcoin](https://ssrn.com/abstract=4081000)                                                                                                                                      |
| How well it held up       | Weak: rests on one study on one asset over 2015 to 2021 that reported 33 percent a year with a Sharpe ratio of 1.58, and on a finding that is a handful of hours selected out of twenty-four from a single sample, the smallest kind of evidence there is |
| Also appears in           | [Seasonality based on the same calendar month](../../../tutorials/quantconnect/seasonality-effect-based-on-same-calendar-month-returns/README.md), a different calendar effect, in this collection                                                        |

## The idea in one paragraph

Bitcoin trades every hour of every day, so there is no opening bell and no closing bell. But the people who trade
it still sleep, work and eat on a clock, and the big traditional share markets still open and close at fixed
times. If you average Bitcoin's return in each of the twenty-four hours of the day over several years, the average
is not the same in every hour. This strategy holds Bitcoin only from 22:00 in the evening until midnight, London
time, and holds cash for the rest of the day. The bet is that this two-hour window has a positive average return
that is large enough to survive the costs of buying and selling every day. It is the smallest kind of evidence,
and the rest of this page says why.

## Why anyone believed it

Traditional markets have well-documented intraday rhythms: for example, prices often move most at the open and the
close, because that is when the most people want to trade. Bitcoin has no open and no close, but it has something
equivalent. When the North American share market closes in the late afternoon and Europe and Asia are quiet,
Bitcoin is one of the very few liquid markets still running, and the flow that would normally go elsewhere lands on
it. The paper also points to the timing of large stablecoin creations, which some studies show move the Bitcoin
price within the following half hour.

The counterparty is the trader who has to transact at a particular hour, the small crowd that is awake and active
at that hour, and anyone whose automated system runs on a fixed clock. If those flows keep arriving in the same
window, the window keeps a small positive average, and the seller of Bitcoin in the other twenty-two hours gives
it up.

## An everyday comparison

A twenty-four-hour diner never closes, yet it is not equally busy at every hour. At seven in the morning there is
a queue, at three in the afternoon it is empty, and at two in the morning it is full of people who have just
finished work. A supplier who only delivers to the diner during the busy two hours will sell more, on average,
than one who delivers at random, even though the diner never shut. The catch is that you have to observe the
diner for many mornings before you can be sure the queue is a habit rather than a coincidence.

## The rules, step by step

1. Choose Bitcoin against the dollar, on one exchange that publishes continuous hourly prices. The paper uses
   Gemini; the implementation uses Bitfinex.
2. Decide the time zone. The paper works in London time, written in the study as UTC plus zero, and that is the
   clock the strategy uses.
3. Every day, at 22:00, buy Bitcoin with the whole account.
4. Every day, at 00:00, two hours later, sell the whole position and hold cash.
5. Hold cash for the other twenty-two hours of the day. Do not trade at any other time.
6. Repeat the next day, every day of the year, including weekends and holidays.
7. Keep the size of each purchase the same, so that the strategy is a fixed bet each day rather than a growing one.
8. If you want to study it before trading it, first compute the average return of each of the twenty-four hours
   separately over the whole sample. The rule then picks the window that is most positive in the past. The published
   window is 22:00 to midnight.

The implementation adds one detail worth knowing: it buys with all the money at the opening hour and sells
everything at the closing hour, and it uses a fee model that charges a small fraction of the traded value on each
of the two trades.

## The maths, with every symbol named

For each day, the return of the holding window:

```text
r_day = P_00 / P_22 - 1
```

- `r_day` is that day's return for the two-hour hold, as a decimal.
- `P_22` is the Bitcoin price at 22:00 London time.
- `P_00` is the price at 00:00, two hours later.

The average across all the days in the sample:

```text
r_mean = (r_1 + r_2 + ... + r_T) / T
```

- `r_mean` is the average window return, per day.
- `T` is the number of days in the sample.
- If `r_mean` is 0.001, that is 0.10 percent per day.

The measure of whether the average is real or luck, written as a t-statistic:

```text
t = r_mean / ( s / square root of T )
```

- `s` is the standard deviation of the daily window returns, which measures how much they scatter around the
  average.
- `s` divided by the square root of `T` is the standard error, the typical distance between the sample average and
  the true average.
- `t` larger than about 2 is the usual bar for calling a result hard to explain by chance alone; a t-statistic of
  1.96 or more is the convention the list itself uses when it reports that 48 percent of the papers it coded clear
  it.

The annualized return, if the same daily average repeated every day of the year:

```text
annual = (1 + r_mean) raised to the power 365, minus 1
```

- `annual` is the compounded yearly return.
- 365 is the number of days in a year.
- The formula assumes the gains are reinvested each day, which is what a fully reinvested account does.

The cost line, charged twice a day because the position is bought once and sold once:

```text
cost_per_day = 2 * c
```

- `c` is the cost of one side, covering the gap between the buying and selling price plus commission. For a liquid
  crypto pair a figure of 0.05 percent (five basis points) per side is realistic; the implementation's own fee
  model charges a good deal less than that, at 0.005 percent per trade, which is why its costs look small.

## A worked example

Eight days of the window, with an invented but plausible price path and a cost of 0.10 percent per day, being
0.05 percent on each of the two trades.

| Day | Price at 22:00 | Price at 00:00 | Gross return | Cost  | Net return |
| --- | -------------- | -------------- | ------------ | ----- | ---------- |
| 1   | 100.00         | 101.20         | +1.20        | -0.10 | +1.10      |
| 2   | 101.20         | 100.29         | -0.90        | -0.10 | -1.00      |
| 3   | 100.29         | 100.79         | +0.50        | -0.10 | +0.40      |
| 4   | 100.79         | 99.28          | -1.50        | -0.10 | -1.60      |
| 5   | 99.28          | 101.36         | +2.10        | -0.10 | +2.00      |
| 6   | 101.36         | 101.06         | -0.30        | -0.10 | -0.40      |
| 7   | 101.06         | 101.87         | +0.80        | -0.10 | +0.70      |
| 8   | 101.87         | 100.75         | -1.10        | -0.10 | -1.20      |

All the return columns are in percent. The gross returns add to +0.80 percent, so the average gross return is
+0.10 percent a day. The net returns add to 0.00 percent, so the average net return is 0.00 percent a day. The
standard deviation of the eight gross returns is 1.2547 percent, so the t-statistic is:

```text
t = 0.10 / (1.2547 / square root of 8) = 0.10 / 0.4436 = 0.23
```

A t-statistic of 0.23 is nowhere near the 1.96 bar, and that is the first honest lesson: eight days can never tell
you anything. Now suppose the same average and the same scatter held over four years, which is about 1,460 trading
windows:

```text
t = 0.10 / (1.2547 / square root of 1460) = 0.10 / 0.0328 = 3.05
```

At four years the same numbers clear the bar comfortably. The second lesson is the cost line. The gross average is
0.10 percent a day, which compounds to about 44 percent a year:

```text
annual = (1.001) to the power 365, minus 1 = 0.44, that is 44 percent
```

but the net average is 0.00 percent a day after the two trades, which compounds to nothing. The published figure is
33 percent a year, and the gap between 44 and 33 is the room for costs and for the difference between a sample
average and the long-run average. Whether the edge survives depends almost entirely on how cheaply the two daily
trades can be made.

## What the research actually found

| Source                                                              | What it measured                                                                  | Result                                                                                                                                                                    |
| ------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Padysak and Vojtko, Seasonality, Trend-following and Mean reversion | Bitcoin returns by hour of the day, Gemini data, 2015 to 2021                     | The hourly distribution of returns is not uniform, and several hours had statistically significant positive average returns while the negative hours were not significant |
| The same study                                                      | A strategy that holds Bitcoin only from 22:00 to midnight, London time, every day | 33 percent a year, an annualized volatility of 20.93 percent, a Sharpe ratio of 1.58 and a worst fall of 34.04 percent                                                    |
| The same study, quoted discussion                                   | Why those hours                                                                   | The 22:00 and 23:00 returns dominated, and the window is one in which every other major exchange is closed, so Bitcoin is one of very few places to trade                 |
| The same study, other aims                                          | Trend-following and mean reversion on the same data                               | The study also examines both, which matters for interpretation: several effects were tested, so the best one is expected to look strong by chance alone                   |

The list that carries this strategy does not publish its own Sharpe ratio for it. Its general replication record,
which is the vendor's own measurement, is a median Sharpe ratio of 0.37 across the papers it has coded, with 48
percent of them clearing a t-statistic of 1.96 and a median test window of 34 years. A Sharpe ratio of 1.58 from a
six-year window on a single asset is far outside that record, and the list's own warning applies directly: a
strategy needs roughly the square of 1.96 divided by its Sharpe ratio, in years, to prove itself, which for a
Sharpe ratio of 1.58 is about 1.5 years only if the effect is real and stable. Six years of daily data spread
across twenty-four hours leaves a few hundred observations per hour, and after choosing the best hour that is not
much.

## How this project relates to it

The repository's crypto survey,
[Crypto, DeFi and perpetuals](../../../strategies/books2/03_crypto_defi_and_perpetuals.md), documents a directly
related intraday finding: a study of 367 stablecoin creation events found that one billion dollars minted lifted
Bitcoin by 0.24 percent over five minutes and 0.68 percent over thirty minutes, and that the response was larger
when sentiment was positive and the event was publicly announced. That is a mechanism for why particular hours
might behave differently, and it also shows how small the moves are: fractions of a percent, measured with event
windows, on samples of a few hundred events.

The second link is
[Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md). It is
the right thing to read alongside this tutorial, because the whole question here is whether the best hour out of
twenty-four, out of one six-year sample, is a real effect or the survivor of a search. The list's own record, that
half of its coded papers cannot be distinguished from zero, is the same warning in numbers.

## Where it goes wrong

- The search is the biggest problem. Twenty-four hours were examined and the best were picked. On completely random
  data, the best of twenty-four averages will look impressive, and the reported significance does not account for
  the other twenty-three hours that were tried.
- Six years is short. The sample runs from 2015 to 2021, a period in which Bitcoin rose enormously. A rising asset
  has positive average returns in many hours, so a chosen window may be capturing the trend rather than a clock
  effect.
- Costs are charged twice a day, every day. The worked example shows the whole gross average vanishing once a
  realistic spread is applied. If the exchange fee is a fraction of a basis point, the trade is a fee arbitrage
  rather than a seasonality trade, and it would not survive a wide spread.
- One asset, one exchange. The result is Bitcoin on Gemini. Data from another exchange, or an average across
  exchanges, can shift the exact hours, and the paper's own implementation uses a different exchange.
- The effect can move or disappear. If enough people trade the same two hours, the buying happens earlier and the
  return is shared with more participants, and the tiny average is the first thing to go.
- Small edges and large moves. Even if the average is real, any single two-hour window can lose several percent,
  and the measured worst fall of 34 percent is what that looks like when it accumulates.

## Try it yourself

You need a spreadsheet and two years of hourly Bitcoin prices from a public source.

1. Build a sheet with one row per hour, and columns named Timestamp, Price, Hour of day.
2. Add a column for the one-hour return, being this price divided by the previous price minus one.
3. Add a pivot table, or a set of average formulas, that computes the average one-hour return for each of the
   twenty-four hours separately.
4. For each hour, also compute the standard deviation of its returns and divide the average by the standard
   deviation divided by the square root of the number of observations. That is the t-statistic for that hour.
5. Sort the hours by average return, best first.
6. Multiply the best hour's average by 365 to see what it would compound to if it repeated every day.

What to notice: almost certainly one or two hours will look good and have a t-statistic above 2, even if Bitcoin
has no real clock. Look at how many hours you tested to find them. Then split your two years in half and check
whether the same hour is best in both halves; if it is not, you have found a coincidence rather than a pattern.

## Where this came from

- [Quantpedia, intraday seasonality in Bitcoin](https://quantpedia.com/strategies/intraday-seasonality-in-bitcoin/),
  the rules, the performance figures and the discussion of why the window is when other exchanges are closed.
- The implementation file the list carries, `static/strategies/intraday-seasonality-in-bitcoin.py`, whose header
  states the 22:00 opening, the two-hour hold and the exchange used.
- Padysak and Vojtko, [Seasonality, Trend-following, and Mean reversion in Bitcoin](https://ssrn.com/abstract=4081000),
  the study behind the rules.
- [Crypto, DeFi and perpetuals](../../../strategies/books2/03_crypto_defi_and_perpetuals.md) and
  [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md), this
  repository's surveys of the crypto and overfitting evidence.
- A finished tutorial in the sibling library examines a different calendar effect:
  [seasonality tutorial](../../../tutorials/quantconnect/seasonality-effect-based-on-same-calendar-month-returns/README.md).

## Words used in this tutorial

- annualized: expressed as if it happened at the same pace for a whole year.
- basis point: one hundredth of one percent, so five basis points is 0.05 percent.
- long: owning something, so that you gain when its price rises.
- Sharpe ratio: a reward-per-risk measure; the average return divided by how much the return wobbles.
- spread: the gap between the price at which you can buy and the price at which you can sell.
- standard deviation: a measure of how far values typically sit from their average.
- t-statistic: the sample average divided by its standard error; a large value means the average is hard to explain
  by chance alone.
- UTC: the clock that is used as the world's common reference time; London time in winter equals it.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
