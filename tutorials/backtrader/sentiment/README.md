# Sentiment: buying when the crowd is frightened, and the trouble with a signal that fires twice a year

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                  |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A fund holding the largest American shares, and in one case Bitcoin                                                                                                                                                                                                                                                    |
| How often it trades       | Two to six times in eleven years for the share rules, and about twice a year for Bitcoin                                                                                                                                                                                                                               |
| What you need             | A spreadsheet and a public history of the sentiment measure                                                                                                                                                                                                                                                            |
| Where the rules come from | [The Strategy Compendium, article 28, sentiment](https://backtrader.readthedocs.io/en/latest/strategies-series/en/28-sentiment.html)                                                                                                                                                                                   |
| The underlying research   | None for the exact thresholds: the rules apply the library's own numbers to commercial and exchange-published measures of fear, without a source paper behind them                                                                                                                                                     |
| How well it held up       | Weak: three rules, one sample from 2011 to 2021 when the American market rose in almost every year, and only two or three closed trades each                                                                                                                                                                           |
| Also appears in           | [The VIX tutorial](../../quantconnect/vix-predicts-stock-index-returns/README.md), [news sentiment on drug makers](../../quantconnect/using-news-sentiment-to-predict-price-direction-of-drug-manufacturers/README.md) and [news sentiment with newsdata](../../fastquant/news-sentiment/README.md) in this collection |

## The idea in one paragraph

There are numbers, published every day, that measure how frightened the crowd is. One is a survey of
investor mood scored from 0 to 100. One counts how many insurance-like option contracts are being
bought against shares, relative to the optimistic ones. One prices the expected size of future
swings directly. This category watches those numbers and bets against the crowd: when the measure is
at an extreme of fear, it buys shares, and when it reaches an extreme of greed, it sells. Between
those extremes it does nothing. Because the extremes are rare, the strategy is idle most of the
time and trades only a few times a decade.

## Why anyone believed it

The buying opportunity exists because other people cannot take it. When fear peaks, some investors
are forced to sell for reasons that have nothing to do with value: a fund meeting withdrawals, a
trader whose risk limits have been breached, a person who simply cannot stand the swings any longer.
They sell at prices that a calmer buyer would not accept, and the buyer who arrives with cash is
being paid to take the other side of a temporary panic rather than to forecast the economy.

The measure itself is the reason a rule can be written at all. Without a number, "buy when others are
fearful" is advice, not a system; with a number that reaches an extreme a handful of times a decade,
it becomes testable, and its weak point is that it then has very few independent bets.

## An everyday comparison

A town has a single umbrella shop. In ordinary weather the umbrellas sell at the normal price. When
a storm warning is issued, everyone rushes to buy at once, the shop raises prices, and a few people
still queue. A shopper who buys a season's umbrellas at the moment the warning is at its loudest is
paying the panic price, and the question is whether the storm that follows is usually milder than
the warning suggested. The sentiment rule bets that the loud warning is an overreaction; the
counterparty is the person buying at the peak of the queue.

## The rules, step by step

The category holds four backtests, all sharing one skeleton: read a fear measure, compare it with two
thresholds, buy at the fear extreme and sell at the greed extreme.

| Strategy          | Measure                                                     | What it does                                                      |
| ----------------- | ----------------------------------------------------------- | ----------------------------------------------------------------- |
| Fear and greed    | A 0-to-100 investor sentiment score                         | Buy below 10, sell above 94                                       |
| Put/call ratio    | Option volume in pessimistic contracts over optimistic ones | Buy above 1.0, sell below 0.45                                    |
| VIX               | The expected size of index swings                           | Buy above 35, sell below 10                                       |
| Bitcoin sentiment | Google search heat for Bitcoin                              | Buy above the top band, sell below the bottom, flat at the middle |

1. Fear and greed. One price per day for a fund of large American shares, from 2011 to 2021, plus the
   published fear-and-greed score. Put the whole account into the fund when the score is below 10,
   which means "more fearful than 90 percent of past readings". Sell the whole holding when the
   score rises above 94. Stay invested until the greed extreme arrives; there is no time limit.
2. Put/call ratio. Same shares and dates, same shape. The ratio is the number of pessimistic option
   contracts traded divided by the number of optimistic ones. Buy the whole account when the ratio
   rises above 1.0, meaning more people are buying insurance than lottery tickets. Sell the whole
   holding when the ratio falls below 0.45.
3. VIX. Same shares and dates. Buy the whole account when the VIX rises above 35, a level reached
   almost only during a crash. Sell when the VIX falls below 10, a level reached almost only in
   complacent calm.
4. Bitcoin sentiment. Weekly Bitcoin prices and the Google search heat for the word "Bitcoin", from
   2018 to 2020. Compute a moving average of the search heat over 10 weeks and a band one standard
   deviation above and below that average. Buy when the search heat closes above the top band, sell
   short when it closes below the bottom band, and close any position when it returns between the
   bands.
5. Position size in every rule is the whole account, and every rule is checked once a day or once a
   week. The three share rules never sell short; the Bitcoin rule does.

## The maths, with every symbol named

The put/call ratio is a single division:

```text
PCR = put volume / call volume
```

- `put volume` is the number of option contracts traded during the day that give their owner the
  right to sell shares at a set price.
- `call volume` is the number traded that give the right to buy at a set price.
- A ratio above 1 means more contracts were bought to protect against a fall than to bet on a rise. A
  ratio of 0.45 means the opposite by more than two to one.

The fear-and-greed score is a composite. Each of its seven inputs, covering price momentum, the
number of shares rising against falling, the VIX, and a few others, is converted to a scale from 0
to 100, and the seven are averaged:

```text
Score = (s_1 + s_2 + ... + s_7) / 7
```

- `s_1` to `s_7` are the seven sub-scores, each already between 0 and 100.
- The composite is built so that 0 means the most fearful reading the authors allow and 100 the most
  greedy. It is a description of the present, not a forecast, and it is a commercial product whose
  history is reconstructed rather than a statistic an exchange publishes.

The Bitcoin rule uses an average and a band:

```text
mid_t   = average of search heat over the last 10 weeks
sd_t    = standard deviation of the same 10 weekly values
upper_t = mid_t + 1 * sd_t
lower_t = mid_t - 1 * sd_t
```

- `mid_t` is the middle line, the ordinary level of public interest.
- `sd_t` is how far the weekly values typically sit from that middle; it grows when interest is
  erratic.
- `upper_t` and `lower_t` are the same line moved up and down by one typical distance. The rule used
  one standard deviation, the library's setting, rather than the more common two.

The VIX itself is the market's expected size of index swings over the next thirty days, stated as a
yearly percentage:

```text
VIX = annualised expected volatility of the index over the next 30 days, in percent
```

- `volatility` is how much a price moves around its average, expressed as a percentage per year.
- A VIX of 35 says the market expects swings of about 35 percent a year, which is the neighbourhood
  of a crisis. A reading of 9 is an unusually placid market.

## A worked example

First the fear-and-greed rule, on eight made-up days. The share price, the published score and the
action are shown; the fund trades as a whole number of shares, so the arithmetic is done in percent.

| Day | Score | Share price | Action         |
| --- | ----- | ----------- | -------------- |
| 1   | 55    | 300.00      | watch          |
| 2   | 22    | 292.00      | watch          |
| 3   | 8     | 285.00      | buy at 285.00  |
| 4   | 30    | 296.00      | hold           |
| 5   | 60    | 310.00      | hold           |
| 6   | 88    | 322.00      | hold           |
| 7   | 95    | 330.00      | sell at 330.00 |
| 8   | 70    | 326.00      | flat           |

On day 3 the score is 8, below the buy line of 10, so the rule puts the whole account into the fund
at 285.00. On day 7 the score is 95, above the sell line of 94, so it sells at 330.00.

```text
Gross = 330.00 / 285.00 - 1 = 0.157895, that is 15.79 percent
Cost  = 2 * 0.0005 = 0.0010, that is 0.10 percent
Net   = 15.69 percent
```

The library charges no commission in these files. The 0.05 percent per side used above is this
tutorial's own stand-in for commission and for the gap between the price at which the fund can be
bought and the price at which it can be sold. Without it the table would show 15.79 percent, and the
difference between the two numbers is exactly the sort of cost a backtest is tempted to leave out.

Now the VIX rule, on six made-up days.

| Day | VIX | Share price | Action         |
| --- | --- | ----------- | -------------- |
| 1   | 18  | 300.00      | watch          |
| 2   | 26  | 288.00      | watch          |
| 3   | 38  | 270.00      | buy at 270.00  |
| 4   | 30  | 282.00      | hold           |
| 5   | 16  | 300.00      | hold           |
| 6   | 9   | 312.00      | sell at 312.00 |

On day 3 the VIX is above 35, so the rule buys at 270.00, which is what a frightened market looks
like. On day 6 the VIX has collapsed to below 10, so the rule sells at 312.00.

```text
Gross = 312.00 / 270.00 - 1 = 0.155556, that is 15.56 percent
Cost  = 0.0010, that is 0.10 percent
Net   = 15.46 percent
```

Both tables are invented and both happen to win. That is the luck of a short table. What the tables
do show is why the rule is idle so much of the time: eight days contained one extreme and six days
contained one, and in a real eleven-year file that produced six purchases in the first case and
three in the second.

## What the research actually found

The library reports one run per rule on one sample. Its own numbers are below.

| Rule           | Sample                       | Buys | Sells | Trades closed | Final value | Reward for risk | Worst fall |
| -------------- | ---------------------------- | ---- | ----- | ------------- | ----------- | --------------- | ---------- |
| Fear and greed | Large shares, 2011 to 2021   | 6    | 2     | 3             | 280,860     | 0.89            | 24.28%     |
| Put/call       | Large shares, 2011 to 2021   | 6    | 3     | 3             | 240,069     | 0.83            | 24.77%     |
| VIX            | Large shares, 2011 to 2021   | 3    | 1     | 2             | 261,274     | 0.92            | 33.68%     |
| Bitcoin        | Bitcoin weekly, 2018 to 2020 | 16   | 16    | 16            | 15,301      | 0.80            | 17.49%     |

The share figures are on 2,445 daily bars and start from the library's account size; the Bitcoin
figures start from 10,000. Two readings of this table matter more than the returns. The first is the
number of trades. Three closed trades cannot pass a significance test, and the claim that "extreme
fear always rebounds" is tested here on a period when the American market recovered from every
setback. Run the same 10 and 94 thresholds over 2000 to 2010, which contained two long falls, and the
answer may differ entirely. The second is that the three share rules share a fear measure, enter
about equally often, and end in three different places, from 240,069 to 280,860: the difference is
almost entirely the exit rule, not the entry. Sentiment's apparent value hides in when it decides to
leave.

The Bitcoin rule is the odd one out. It is a fast variable traded against the trend, it took 16 round
trips in two years, and its result was close to a coin flip.

The library's own tests assert the final value, the reward for risk and the worst fall against numbers
captured when each strategy was migrated. Passing those assertions proves the engine computes exactly
what the file says, to the cent and to the sixth decimal, in both engine modes. It proves nothing
about whether the strategy earns anything; a matching final value is a consistency check, not
evidence of an edge.

The repository's reading of the research is in
[Stylized facts, tails and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md). One
finding there is close to this category: Google search series for the shares of the Dow Jones index
were strongly persistent, with a measure of memory between 0.8 and 1.1, and yet their correlation
with traded volume and volatility was weak, scale-dependent and not the same in sign across shares,
most coefficients falling between minus 0.2 and 0.2 (`1502.00225v1`). Persistence in a search series
is not a tradable signal. A second, from
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
is that a news-driven strategy of the same family reported a reward for risk above 1 after a 0.80
percent yearly cost drag over 1995 to 2015, on one market and one strategy shape
(`1508.04332v2`), and that a language model asked to score headlines can partly be recalling the
outcome rather than reading the text, a contamination that collapses to nothing after its training
cutoff (`2512.23847v2`). Any sentiment measure that is a language model, or is built from one, needs
that check before its backtest means anything.

## How this project relates to it

This repository does not implement a fear-and-greed or put/call strategy. The closest material is the
brief [Stylized facts, tails and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md),
whose finding on search-series persistence is the honest warning for the Bitcoin rule, and the brief
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
which covers news-based strategies and the leakage test a modern sentiment signal must pass.

Two finished tutorials in this collection run the VIX idea directly. The
[VIX tutorial](../../quantconnect/vix-predicts-stock-index-returns/README.md) uses trailing
percentiles of the VIX and finds the same asymmetry this page describes: the evidence is much
stronger for buying after high turbulence than for shorting after calm. The repository's own
volatility brief, [Volatility and microstructure noise](../../../strategies/books/06_volatility_and_microstructure_noise.md),
records a study in which the VIX improved volatility forecasts by 35 to 46 percent against a simple
model but made no statistically distinguishable difference against a stronger baseline, and found
directional return prediction essentially absent (`2407.16780v1`). That is the sharpest statement of
the problem: the VIX measures turbulence well and direction badly, and this category asks it for
direction.

## Where it goes wrong

- The sample is one long rise. Three of the four rules were tested on American shares from 2011 to
  2021, and that market recovered from every fall. A rule that buys after a 10-out-of-100 fear
  reading has not been tested by a decade that stayed fearful.
- Three closed trades prove nothing. With so few decisions, one lucky entry or one delayed exit
  decides the whole result, and the difference between the fear rule's 280,860 and the put/call
  rule's 240,069 is a single exit choice.
- The measure is not the thing. The fear-and-greed score is a commercial composite with a
  reconstructed history; the put/call ratio counts contracts of many different kinds; the VIX is a
  price for insurance, not a promise about the future. Each is a proxy, and a proxy can drift when
  its inputs change.
- Long-only timing is mostly market exposure. The three share rules never sell short, so they make
  money whenever the market rises and lose when it falls; what is being measured is a slightly
  different way of holding the market, not an independent source of return.
- The thresholds were chosen after seeing the history. Ten and 94, 1.0 and 0.45, 35 and 10 are round
  numbers, but the family of plausible thresholds is large and the best one on one sample is not the
  best on the next. A rule that only works at 10 and not at 15 is a fitted rule.
- Extremes cluster. VIX readings above 35 arrive in crises, so a rule that trades them has very few
  independent episodes and the outcome is dominated by whether the market recovered within the
  holding period.

## Try it yourself

You need nothing but a spreadsheet and a public history of one of the measures; the VIX is published
by the exchange that calculates it and most finance sites carry it.

1. Put a date column and a VIX column down a sheet, one row per trading day, for twenty years. Add
   the closing price of a large index fund beside it.
2. Add a column that flags days with a VIX above 35, and another for days below 10.
3. Count how many days of each kind there are, and how many separate stretches they fall into. A
   stretch is a run of consecutive flagged days.
4. For each flagged day, compute what the index fund did over the next sixty days.
5. Average that sixty-day move for the high-VIX days and for all other days.
6. Repeat the exercise with the threshold at 30 and then 40, and see how much the average move and
   the count change.

What to notice: almost all the high-VIX days will sit in a handful of stretches, so the average is
based on a few episodes rather than on hundreds of independent observations. Then notice how much
the answer moves when the threshold moves from 35 to 30: a real effect should be present across a
range of nearby thresholds, and one that appears only at a single round number is a description of
the sample.

## Where this came from

- [The Strategy Compendium, article 28, sentiment](https://backtrader.readthedocs.io/en/latest/strategies-series/en/28-sentiment.html),
  the category inventory, the four strategies, the thresholds and every performance figure quoted
  above.
- [Stylized facts, tails and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md),
  this repository's brief, including the Google search persistence result (`1502.00225v1`).
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief, including the news-driven strategy figures (`1508.04332v2`) and the
  lookahead test for language-model forecasts (`2512.23847v2`).
- [The VIX tutorial](../../quantconnect/vix-predicts-stock-index-returns/README.md) and the
  [volatility and microstructure noise brief](../../../strategies/books/06_volatility_and_microstructure_noise.md),
  the two other places in this collection where the VIX is used to pick a direction.

## Words used in this tutorial

- implied volatility: the size of price swings the market expects, read from the prices of option
  contracts rather than measured from past prices.
- long: owning an investment, so that a price rise is a gain.
- put option: a contract giving its owner the right to sell something at a set price before a set
  date, which is what investors buy when they fear a fall.
- sentiment: a measure of how optimistic or frightened market participants are, as opposed to what
  prices have already done.
- short selling: borrowing something you do not own, selling it, and buying it back later, so that a
  price fall is a gain.
- VIX: an index, published from option prices, that measures how much the share market expects to
  swing over the next month, stated as a yearly percentage.
- volatility: how much a price moves around its average, measured as a percentage per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
