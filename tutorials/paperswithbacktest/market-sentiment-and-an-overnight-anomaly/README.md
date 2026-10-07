# Market sentiment and an overnight anomaly: buying at the close only when the mood is good

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                        |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A single fund that tracks the whole American market, held only from one day's close to the next day's open                                                                                                                                                                                   |
| How often it trades       | Every trading day, twice: once at the close and once at the next open, but only on days when at least one of three conditions holds                                                                                                                                                          |
| What you need             | A spreadsheet and daily opening and closing prices, plus a volatility series and a sentiment series                                                                                                                                                                                          |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/market-sentiment-and-an-overnight-anomaly.py) and the [Quantpedia entry](https://quantpedia.com/strategies/market-sentiment-and-an-overnight-anomaly) it cites |
| The underlying research   | Vojtko and Hanicova, [Market Sentiment and an Overnight Anomaly](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3829582)                                                                                                                                                                |
| How well it held up       | Mixed: positive over the one four-year window measured, but that window is short, the sentiment series only starts in 2018, and the standalone overnight effect is not tradable once the cost of trading twice a day is counted                                                              |
| Also appears in           | [Overnight anomaly](../../../tutorials/quantconnect/overnight-anomaly/README.md), the plain version of the same nightly hold                                                                                                                                                                 |

## The idea in one paragraph

A trading day has two parts: the hours the market is open and the hours it is closed. In many markets
the closed hours have risen more than the open ones, so a fund bought at the close and sold at the
next open tends to capture a small gain. This strategy adds a mood filter. It buys the market fund at
the close only when three things are true: the fund is above its recent average price, the market's
fear gauge is below its recent average, and a published sentiment index is above its recent average.
Each condition that holds adds a third to the position. At the next open it sells everything and
waits for the next close.

## Why anyone believed it

Most news arrives while the market is shut. A company reports after the close, a government publishes
a jobs number before the open, an overseas market moves in the American night. When the market
reopens, the price has to jump to the new level, and that jump happens entirely overnight. During the
day the market is crowded and liquid, and much of the trading is people adjusting positions rather
than reacting to fresh news, so the daytime hours earn less.

The sentiment filter is meant to answer a second question: not just when the overnight gain happens,
but whether it is likely to happen tonight. A market that is rising, calm and cheerful is one where
buyers are willing, and the same people who feel good today are the ones whose orders arrive at the
next open. The counterparty is whoever sells at the open, often a fund tracking an index that has to
buy wherever the market is, or a trader acting only after reading the morning news.

## An everyday comparison

Think of a shop that shuts at six in the evening and reopens at nine the next morning. Almost all of
the news that matters to the shop's prices arrives after closing, so the shelves are repriced
overnight, and the shop opens at the new level. Through the day it trades at roughly that level. The
strategy buys at the evening close, before the overnight repricing, and sells at the morning open,
after it. The sentiment conditions are a check on whether the shop is likely to open higher or lower,
since on a gloomy morning the overnight repricing can be downward.

## The rules, step by step

1. Choose the instruments: a fund that tracks the S&P 500 index, the CBOE Volatility Index, often
   written VIX, and a published market-sentiment index.
2. Each day, sixteen minutes before the close, compute a twenty-day average for each of the three.
   The twenty-day average is the sum of the last twenty daily values divided by twenty.
3. Check three conditions. First, is the index fund's price above its twenty-day average? Second, is
   the VIX below its twenty-day average? Third, is the sentiment index above its twenty-day average?
4. Give each condition that is true one third of the intended position. All three true means the full
   position; one true means one third; none true means no trade.
5. Buy the index fund at the closing price with the money the conditions allow.
6. Sell the whole position at the next day's opening price.
7. Repeat from step 2 every day. The authors suggest using this as a filter on other trades rather
   than as a standalone strategy.

A note on the three inputs. The VIX measures how much movement the options market expects, and a low
VIX means calm. The published sentiment index is built from text and market data by a data provider,
and it is a number the provider computes, not an official statistic; the particular index used here
only begins in 2018, which is why the sample below is short.

## The maths, with every symbol named

The twenty-day average of a series:

```text
SMA_20 = (value_1 + value_2 + ... + value_20) / 20
```

- `SMA_20` is the simple moving average: the mean of the last twenty daily values.
- `value_1` through `value_20` are the most recent twenty daily readings of the series.

The size of the position as a fraction of the account:

```text
weight = (1/3) * (a + b + c)
```

- `weight` is the fraction of the account placed in the fund, from 0 to 1.
- `a` is 1 if the index fund is above its own average, otherwise 0.
- `b` is 1 if the VIX is below its own average, otherwise 0.
- `c` is 1 if the sentiment index is above its own average, otherwise 0.

The gain from holding overnight:

```text
R_overnight = Open_next / Close_today - 1
```

- `R_overnight` is the overnight return, as a decimal: 0.002 means two tenths of a percent.
- `Open_next` is the price at the next day's opening auction.
- `Close_today` is the price at today's close.

The contribution to the account is `weight * R_overnight`. The cost is paid twice each day there is a
trade:

```text
Cost = weight * 2 * c
```

- `c` is the cost of one trade as a fraction of the amount traded, the gap between the buying and the
  selling price plus commission. A realistic figure for a large index fund is 0.0005, five basis
  points per side, where one basis point is one hundredth of one percent.
- The two is because the position is bought at the close and sold at the open.

## A worked example

Five nights of invented but plausible values. Each condition either holds or does not, the weight is
one third per condition, and the overnight return is measured from the close to the next open.

| Night | Fund above average | VIX below average | Sentiment above average | Weight | Overnight return | Contribution |
| ----- | ------------------ | ----------------- | ----------------------- | ------ | ---------------- | ------------ |
| 1     | yes                | yes               | yes                     | 1.000  | +0.25%           | +0.250%      |
| 2     | yes                | yes               | no                      | 0.667  | -0.05%           | -0.033%      |
| 3     | yes                | no                | no                      | 0.333  | +0.10%           | +0.033%      |
| 4     | yes                | yes               | yes                     | 1.000  | +0.20%           | +0.200%      |
| 5     | no                 | yes               | yes                     | 0.667  | -0.15%           | -0.100%      |
| Total |                    |                   |                         |        |                  | +0.350%      |

The contribution of each night is the weight multiplied by the overnight return. For night 2 that is
0.667 * -0.0005 = -0.00033, a loss of 0.033 percent. The total over five nights is 0.350 percent, an
average of 0.070 percent per night, which over 252 trading days would compound to about 19 percent a
year before costs.

Now the cost. The average weight across the five nights is (1.000 + 0.667 + 0.333 + 1.000 + 0.667) / 5
= 0.733, and the cost per night is 0.733 * 2 * 0.0005 = 0.073 percent. Subtract that from the average
return of 0.070 percent and the net is -0.003 percent per night, about -0.8 percent a year. At a
tighter one basis point per side the cost falls to 0.015 percent a night and the net becomes about
+14 percent a year.

Notice how heavily the answer depends on the cost of trading twice a day. The worked example says
nothing about whether the strategy works; it shows that a gross edge of about seven hundredths of a
percent a night is smaller than the bridge between buying and selling in all but the cheapest markets.

## What the research actually found

| Source                                                                                                              | What it measured                                               | Result                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Vojtko and Hanicova, Market Sentiment and an Overnight Anomaly                                                      | The overnight filter on three sentiment measures, 2018 to 2021 | The equally weighted portfolio using a twenty-day average earned about 15.58 percent a year with volatility 7.33 percent, a worst fall of 10.97 percent and a reward-to-risk of 2.12, taken from the paper's table on page 5 |
| Quantpedia, summarising that paper                                                                                  | The same back-test                                             | The vendor grades the anomaly as strong but notes that the sentiment input only begins in 2018, so the test window is short; the authors themselves suggest using the rule as an overlay rather than on its own              |
| This repository, [news and language-model signals](../../../strategies/books/14_news_and_language_model_signals.md) | The plain overnight effect, across markets and decades         | The overnight minus daytime return is about 2.75 basis points a day, about 7.2 percent a year, and the same brief records it as not viable once cost and turnover are counted (`2507.04481v1`)                               |
| The list's replication record                                                                                       | 4,843 coded papers, each over its own full history             | The median replication has a reward-to-risk of 0.37, 48 percent clear a t-statistic of 1.96, the median test window is 34 years, and the median carries a market exposure of +0.17                                           |

Read together, the picture is this. There is a real difference between the overnight part of a day and
the daytime part, documented across many markets and decades. The plain version of it is not tradable
after costs, because it requires buying and selling twice a day. The version on this page adds a mood
filter, which reduces how often it trades and improves the result in the one short window measured,
but a four-year window and a sentiment series that begins in 2018 is not much evidence.

## How this project relates to it

The finished tutorial
[overnight anomaly](../../../tutorials/quantconnect/overnight-anomaly/README.md) covers the plain
version of this idea, owning the market overnight and not during the day, with no filter at all. Read
it beside this page: it reports the same overnight-versus-daytime gap and reaches the grade that the
gap is not tradable once the cost of trading twice a day is counted. The repository brief
[manipulation, fraud and governance](../../../strategies/books2/19_manipulation_fraud_and_governance.md)
records a darker reading of the same pattern, an overnight and daytime split across sixteen major
indices that one paper argues is the footprint of deliberate price pushing rather than of news; that
paper is a polemic with no trade-level evidence, and the brief treats it as such. The brief
[news and language-model signals](../../../strategies/books/14_news_and_language_model_signals.md) is
where the 2.75-basis-point overnight gap and its cost problem are measured.

## Where it goes wrong

- Costs are the whole decision. The position is bought at the close and sold at the open, so every
  night in the market pays the gap between buying and selling prices twice. A gross edge of a couple
  of hundredths of a percent a night cannot survive that in most markets.
- The sample is short. The sentiment index only begins in 2018, so the four-year result includes one
  bull market and one sharp fall and very little else; a longer record is not available.
- The sentiment index is a provider's product. It is built by a company from text and market data,
  and if the provider changes the recipe the signal changes with it, which makes the rule hard to
  reproduce.
- Three conditions on one price. The fund's own trend and the calm gauge both partly measure the same
  thing, so the three conditions are not three independent votes; on many days they all move together.
- Crowding. An overnight rule that can be run by anyone with a broker account is about as easy to copy
  as a rule gets, and the more money runs it the smaller the gap it captures.
- The whole idea would be false if the overnight gain were simply payment for holding risk while the
  market is shut, in which case it is not free money but a different way of being paid for being
  invested. Measuring the overnight return against its risk, not just its average, is what would
  settle it.

## Try it yourself

You need a spreadsheet, daily opening and closing prices for one index fund, and a volatility series.
You do not need any money.

1. Build a sheet with one row per trading day: the date, the opening price and the closing price.
2. Add a column for the overnight return: the next day's opening price divided by today's closing
   price, minus one.
3. Add a column for the twenty-day average of the closing price, and a column that is 1 when the
   close is above that average.
4. Add the same two columns for the volatility series, but with the mark set to 1 when the volatility
   is below its average.
5. Average the overnight return over the days when the fund was above its average, and again over the
   days when it was not.
6. Subtract 0.10 percent from that average, which is the cost of two trades at five basis points a
   side.

What to notice: the overnight return is positive on average, and it is concentrated in a small number
of nights, often the ones after a sharp fall. Notice how quickly the 0.10 percent cost eats the
average, and how much of the average comes from a handful of nights. If your sheet shows the filtered
version winning by a wide margin, check whether the sentiment series you used was published on the day
you used it, because a series that becomes available later is a form of looking into the future.

## Where this came from

- [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/market-sentiment-and-an-overnight-anomaly.py),
  the rules as coded: the three conditions, the one-third weights, the close-to-open hold.
- [Quantpedia: market sentiment and an overnight anomaly](https://quantpedia.com/strategies/market-sentiment-and-an-overnight-anomaly),
  the indicative performance figures and the description of the three inputs.
- Vojtko and Hanicova, [Market Sentiment and an Overnight Anomaly](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3829582),
  the original study over 2018 to 2021.
- [News and language-model signals](../../../strategies/books/14_news_and_language_model_signals.md),
  this repository's brief, which supplies `2507.04481v1`, and
  [manipulation, fraud and governance](../../../strategies/books2/19_manipulation_fraud_and_governance.md),
  which supplies the competing reading of the overnight pattern.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- index fund: a fund that buys every share in an index, so its price follows the index.
- moving average: the mean of the most recent chosen number of values, here twenty days.
- overnight return: the price change from one day's close to the next day's open.
- sentiment: a number meant to summarise the market's mood, often built from news or social text.
- spread: the gap between the price at which something can be bought and the price at which it can be
  sold.
- VIX: the CBOE Volatility Index, a number that measures how much movement the options market expects
  and rises when investors are nervous.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
