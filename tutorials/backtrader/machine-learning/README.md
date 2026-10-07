# Machine learning: scores, clusters and labels, and the gap between a prediction and a profit

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                             |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Spot gold priced in American dollars, mostly on daily bars, with a few tests on Bitcoin, a gold fund and baskets of funds                                                                                                                                                                         |
| How often it trades       | From a handful of trades in eighteen years for the slow score rules to several hundred in three months for the ported ones                                                                                                                                                                        |
| What you need             | Python and a data file                                                                                                                                                                                                                                                                            |
| Where the rules come from | [The Strategy Compendium, article 12, machine learning](https://backtrader.readthedocs.io/en/latest/strategies-series/en/12-machine-learning.html)                                                                                                                                                |
| The underlying research   | None with a finance paper behind it: the rules are the library's own specifications and ports of MetaTrader expert advisors, and the two fitted models use the standard clustering and random-forest methods with no published trading result                                                     |
| How well it held up       | Weak: the one genuinely fitted model lost all 70 of its out-of-sample trades in the library's own test, and the score rules are single backtests on gold, not replications                                                                                                                        |
| Also appears in           | [Gaussian naive Bayes](../../quantconnect/gaussian-naive-bayes-model/README.md), [gradient boosting](../../quantconnect/gradient-boosting-model/README.md) and the [temporal CNN forecaster](../../quantconnect/forecasting-stock-prices-using-a-temporal-cnn-model/README.md) in this collection |

## The idea in one paragraph

Machine learning in this category means one of three things. The first is a score: several simple
measures of the market, each turned into a number between zero and one, added together and averaged.
The second is a label: past days are grouped by the shape of their price bars, and the rule buys when
today's bar looks like the group that has lately been followed by rises. The third is a hand-written
value that copies the form of a reinforcement-learning agent without ever learning anything. In every
case the model reads only past prices, turns them into one number, buys when that number is high and
sells when it falls. The model predicts direction, up or down, never the size of the move, and the
strategy lives or dies on whether the direction is right often enough to pay for the trading.

## Why anyone believed it

The story has two parts. The first is that a market contains many weak clues, and a person can only
watch one or two at a time while a model can weigh twenty at once. The second is that these rules are
fitted to real prices rather than written by hand, so they adapt to changing markets in a way a fixed
formula cannot. The counterparty is the slower participant: a person reading the news after the
screen has already moved, or a fund forced to sell for reasons that have nothing to do with value. If
that flow leaves a recognisable shape on the price chart, a model trained on such charts can buy into
it.

The trouble is that the same flexibility that finds a real pattern also lets a model memorise noise,
and the history that appears to prove a pattern is usually the history the model was fitted on.

## An everyday comparison

Picture a football tipster who charges a booking fee on every bet and gets the result right 55 times
in 100. If you bet a fixed amount each time and pay the fee twice a week, being right more often than
wrong is not enough; the weeks when the tipster is wrong are large enough, or the fees frequent
enough, that the account can still fall. The tipster is not lying about the 55 percent. It is just
that a prediction and a profit are two different numbers, and only one of them is what the model
produces.

## The rules, step by step

The category holds 21 backtests. They share one skeleton: take prices, turn them into a score with
only past data, buy when the score is high, and sell or go flat when it weakens. The table lists the
ones a reader is most likely to meet.

| Strategy                        | What it does                                                                 |
| ------------------------------- | ---------------------------------------------------------------------------- |
| Gold ML Prediction              | Averages three scores and buys above 0.6                                     |
| KMeans candle classification    | Groups price bars and follows the group that has been rising more            |
| Reinforcement learning          | Averages two distances into a "q score" and trades at plus or minus 0.2      |
| Fuzzy logic                     | Turns five indicators into one score and buys below 0.25, sells above 0.75   |
| Extreme short-term gain         | Buys after a multi-day surge and holds a fixed number of days                |
| Random forest ratios            | Classifies made-up financial ratios across five funds                        |
| Sentiment signal                | Multiplies a return score by a volume score as a stand-in for news sentiment |
| RNN port (0187)                 | Matches a price state against hand-set probabilities                         |
| MTC neural network plus MACD    | A "neural network" indicator stacked on a moving-average rule                |
| AML adaptive average            | A moving average whose speed adapts to the market                            |
| 1225 AML, ZeroLagEA, JBrainSig1 | Ported trend engines with moving averages and oscillators                    |

The remaining entries are further ports of commercial expert advisors, most of which keep their
original pip and lot settings. Four of the rules are worth stating exactly.

1. Gold ML Prediction. Use daily gold. Build three scores from the last 14, 20, 60, 20 and 252 days:
   a mood score from the 14-day relative strength index, a trend score that is one when the 20-day
   average is above the 60-day average and zero otherwise, and a calm score that is one minus the
   percentile rank of the 20-day wobble over the last 252 days. Average the three. Buy the whole
   account when the average is above 0.6, and close it when the average falls below 0.4.
2. Clustering. Use daily gold. Each day compute three features from the bar, all divided by the
   14-day average true range so that a busy market is not mistaken for a large one: the distance
   from the open to the high, from the open to the low, and from the open to the close. Group the
   last 756 such days into four clusters, refit every 20 days. Keep the cluster whose average
   next-day return is highest and above the average of all training days. Buy at the next open when
   today's bar is assigned to that cluster, and close the position before the same session ends.
3. Reinforcement learning. Use daily gold. Compute a q score as the average of two distances from
   neutral: how far the 14-day relative strength index is from 50, and how far the price is from its
   50-day average. Buy the whole account when the q score is above 0.2 and close it when the score
   falls below minus 0.2.
4. Fuzzy logic. Use 15-minute gold. Turn five smoothed indicators into a score between 0.1 and 0.9
   by weighted membership. Buy when the score is below 0.25 and sell short when it is above 0.75,
   protecting each position with a 60-pip stop and a 20-pip target. A pip is the smallest quoted
   price step.
5. In all four rules the position is a fixed fraction of the account, reviewed once a day or once
   per bar. The first and third rules use the whole account through a contract whose multiplier is
   100, and the clustering rule uses 95 percent of it.

## The maths, with every symbol named

The composite score is an average of three numbers, each already on a zero-to-one scale:

```text
rsi_score = 1 - RSI / 100
ma_score  = 1 if MA_fast > MA_slow, otherwise 0
vol_score = 1 - rank_pct( wobble )
score     = (rsi_score + ma_score + vol_score) / 3
```

- `RSI` is the relative strength index, a number from 0 to 100 built from the average up-move and
  the average down-move over the window. Zero means every recent move was down.
- `MA_fast` and `MA_slow` are the average closing prices over the fast and slow windows, here 20 and
  60 days. The score is one when the fast average is above the slow one.
- `wobble` is the standard deviation of daily returns over the 20-day window, which measures how far
  returns scatter around their own average.
- `rank_pct` is the percentile rank of today's wobble among the last 252 days: 0.10 means calm
  relative to the past year, 0.90 means turbulent. Subtracting from one makes calm days score high.
- `score` runs from 0 to 1. The rule buys above 0.6 and sells below 0.4. A single component can
  never take the score above two thirds, so a buy needs at least two components to agree.

The q score is a distance from neutral, again built from two measures:

```text
q = ( (RSI - 50) / 50 + (C - MA) / MA ) / 2
```

- `RSI` is the 14-day relative strength index. `(RSI - 50) / 50` is 0 when it sits at 50, plus 1
  when it reaches 100 and minus 1 when it reaches 0.
- `C` is today's closing price and `MA` is its 50-day average. `(C - MA) / MA` is the price's
  distance from that average as a fraction, about plus 0.04 when the price is 4 percent above it.
- `q` is the average of the two, so both a strong mood and an elevated price push it up.

The clustering rule rests on a gap and on an edge:

```text
ho = (high - open) / ATR      lo = (open - low) / ATR      co = (close - open) / ATR
gap  = |ho - ho_c| + |lo - lo_c| + |co - co_c|
edge = mean(next-day return of the group) - mean(next-day return of all training days)
```

- `high`, `low`, `open` and `close` are the day's four prices; `ATR` is the 14-day average true
  range, a measure of how far the day travelled.
- `ho_c`, `lo_c` and `co_c` are the same three numbers for the typical day in a group. `gap` is the
  total difference between today's three numbers and those three, ignoring sign, and the group with
  the smallest gap is the one the day belongs to. The file squares each difference before adding,
  which does not change which group is nearest.
- `edge` is how much better the group's days have been than an average training day. The rule trades
  only if the best group's edge is positive.

## A worked example

First the composite score, on seven made-up daily closes. To keep the arithmetic short the mood
window is two days and the averages are 3 days and 5 days; the file uses 14, 20 and 60. The calm
column is the rule's own volatility rank, read from the past year as the rule reads it.

| Day | Close | RSI(2) | rsi_score | ma_score | vol_score | score  | Action        |
| --- | ----- | ------ | --------- | -------- | --------- | ------ | ------------- |
| 1   | 100.0 |        |           |          |           |        | watch         |
| 2   | 100.0 |        |           |          |           |        | watch         |
| 3   | 96.0  | 0.0    | 1.0000    | 0        | 0.90      | 0.6333 | buy at 96.00  |
| 4   | 94.0  | 0.0    | 1.0000    | 0        | 0.85      | 0.6167 | hold          |
| 5   | 99.0  | 71.43  | 0.2857    | 0        | 0.50      | 0.2619 | sell at 99.00 |
| 6   | 103.0 | 100.0  | 0.0000    | 1        | 0.40      | 0.4667 | flat          |
| 7   | 101.0 | 66.67  | 0.3333    | 1        | 0.60      | 0.6444 | buy at 101.00 |

On day 3 the two moves are 0.00 and minus 4.00, so the average gain is zero and the average loss is
2.00, giving `RSI = 0` and an rsi_score of 1.0; the fast average is below the slow one, so the
ma_score is 0, and the three terms average to 0.6333, above 0.6. On day 5 the moves are minus 2.00 and
plus 5.00, the ratio is 2.50 and `RSI = 71.43`, so the rsi_score is 0.2857 and the average falls to
0.2619, below 0.4.

```text
Gross = 99.00 / 96.00 - 1 = 0.03125, that is 3.125 percent
Cost  = 2 * (0.0002 + 0.0003) = 0.0010, that is 0.10 percent
Net   = 3.125 - 0.10 = 3.025 percent
```

The library charges 0.02 percent commission per side and no spread. The extra 0.03 percent per side
in the line above stands in for the gap between the price at which something can be bought and the
price at which it can be sold, which a real order pays. That gap is the difference between the
backtest and the account.

Now the q-score rule, on seven made-up closes, with the average shrunk to five days.

| Day | Close | MA(5)  | (C - MA) / MA | RSI(2) | rsi_norm | q       | Action         |
| --- | ----- | ------ | ------------- | ------ | -------- | ------- | -------------- |
| 1   | 100.0 |        |               |        |          |         | watch          |
| 2   | 102.0 |        |               |        |          |         | watch          |
| 3   | 104.0 |        |               |        |          |         | watch          |
| 4   | 106.0 |        |               |        |          |         | watch          |
| 5   | 108.0 | 104.00 | +0.03846      | 100.0  | +1.000   | +0.5192 | buy at 108.00  |
| 6   | 110.0 | 106.00 | +0.03774      | 100.0  | +1.000   | +0.5189 | hold           |
| 7   | 104.0 | 106.40 | -0.02256      | 25.0   | -0.500   | -0.2613 | sell at 104.00 |

On day 7 the two moves are plus 2.00 and minus 6.00, so the average gain is 1.00 and the average
loss is 3.00. The ratio is 0.333 and `RSI = 100 - 100 / 1.333 = 25.0`, giving an rsi_norm of minus
0.5. The price is 104.00 against a 5-day average of 106.40, a distance of minus 0.02256. The average
of minus 0.5 and minus 0.02256 is minus 0.2613, below minus 0.2, so the rule closes the position.

```text
Gross = 104.00 / 108.00 - 1 = -0.037037, that is -3.7037 percent
Cost  = 0.0010, that is 0.10 percent
Net   = -3.8037 percent
```

Two things to take from the pair of tables. The first trade won and the second lost, which is what a
51-percent hit rate looks like over a handful of rows; neither table says anything about whether the
rules earn. The second is the cost line: at 0.10 percent for a round trip, a rule that trades twice a
month pays roughly 2.4 percent a year before it has made anything.

## What the research actually found

The library reports one run per rule on one instrument. Its own numbers are below.

| Rule              | Sample                       | Trades | Wins               | Final value | Reward for risk | Worst fall |
| ----------------- | ---------------------------- | ------ | ------------------ | ----------- | --------------- | ---------- |
| Composite score   | Gold daily, 2008 to 2025     | 39     | 20 of 38 (51.3%)   | 3,334,048   | 0.64            | 34.93%     |
| q score           | Gold daily, 2008 to 2025     | 56     | 23 of 55 (41.1%)   | 1,956,007   | 0.35            | 44.85%     |
| Fuzzy logic       | Gold 15-minute, 2025 to 2026 | 634    | 360 of 634 (56.8%) | 5,704,430   | 14.97           | 58.99%     |
| KMeans clustering | Gold daily, 2022 to 2025     | 70     | 0 of 70 (0.0%)     | not given   | not given       | not given  |

The final values start from one million units of currency. The fuzzy reward-for-risk figure of 14.97
is not comparable with the others: it is computed from 15-minute bars and scaled up to a year, a
procedure that makes a small, steady edge look enormous. The clustering result is the one to read
twice: zero wins in seventy trades, the loss nailed into the test as an assertion. A group that looked
profitable inside the 756-day training window was worthless the moment it left it.

The library file also has to be read for what it is. Passing its assertion proves the engine
computes exactly what the file says, to the cent and to the sixth decimal, in both of the engine's
modes. It proves nothing about whether the strategy earns anything, and the 0-for-70 case shows that
a passing test can record a total failure.

The repository's reading of the research is in
[Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md). Two of
its findings bear directly on this category. First, in a controlled comparison of nine model
families across twelve instruments, the choice of architecture explained 99.90 percent of the
variation in forecast error while the random seed explained 0.01 percent, and yet directional
accuracy was statistically indistinguishable from a coin flip in all 54 combinations of model,
asset class and horizon (`2603.16886v1`). Second, in a matched experiment where only the training
objective changed, a classification objective produced a value-weighted reward for risk of 2.08
against 1.39 for the regression objective (`2108.02283v7`): the loss function moved the portfolio
outcome more than the model did. A third finding is that performance stops improving after about
seven input features (`2311.14577v1`), which is a warning against the instinct to add another
indicator to the score.

## How this project relates to it

The closest thing in this repository to these rules is its own reading of the machine-learning
literature, [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md).
Its opening summary is the honest frame for this whole tutorial: general-finance work has moved from
"can it predict" to "what exactly was measured", and the same paper that ranks models best by error
finds that none of them calls direction better than chance. A reader who wants to know what a fitted
model would have to do before it were believed will find the checklist there: chronological splits,
reported seed dispersion, feature-count discipline and a cost-inclusive evaluation.

Two finished tutorials in this collection run the same kind of question on other data. The
[Gaussian naive Bayes](../../quantconnect/gaussian-naive-bayes-model/README.md) and
[gradient boosting](../../quantconnect/gradient-boosting-model/README.md) pages use standard
classifiers on market data, and the
[temporal CNN forecaster](../../quantconnect/forecasting-stock-prices-using-a-temporal-cnn-model/README.md)
uses a deep network for the same one-day-ahead direction. Read together with this page they show
that the library's version of machine learning is deliberately small: one score, one threshold, one
assertable rule, with no model file to lose.

## Where it goes wrong

- Overfitting is not a risk here, it is the recorded result. The clustering rule's 0-for-70 comes from
  a group whose in-sample edge vanished out of sample; a rule that keeps its training window and its
  test window the same will not show this until it trades real money.
- The label is direction, not profit. Every rule here is judged on whether the next move is up or
  down, and that is a different question from whether the next move is large enough to cover the
  round-trip cost. The football tipster in the comparison above is the same trap.
- The features can see the future. `rank_pct` over the last 252 days is fine only if the rank is
  computed from data up to today; the same number computed over the whole file, or a group fitted
  including today, leaks tomorrow's prices into today's decision. The library shifts some signal
  columns by one bar to prevent this and not others.
- The samples are tiny and single-instrument: four rules, one metal, one 18-year window.
- A high hit rate and a high reward-for-risk can both be artefacts. The fuzzy rule's 56.8 percent win
  rate is real in the file, but its 14.97 reward for risk is a scaling choice, and its 59 percent
  worst fall is the cost the reader is not shown beside it.

## Try it yourself

You need nothing but a spreadsheet and about sixty daily closes of any share or metal.

1. Put the dates down one column and the closing prices in the next.
2. Add a column for the two-day change and two more columns for the two-day average up-move and
   average down-move, counting a fall as a zero up-move and a rise as a zero down-move.
3. Add a column for the mood score, `1 - RSI / 100`, where `RSI = 100 - 100 / (1 + up / down)`.
4. Add a column for a trend score: 1 if today's 5-day average close is above the 10-day average,
   otherwise 0.
5. Add a column that averages the mood score and the trend score, and a column that says buy when
   that average is above 0.6 and sell when it is below 0.4.
6. Count how many buys and sells the rule produces, and for each buy compute what the next ten days
   did. Now move the two thresholds to 0.55 and 0.45 and count again.

What to notice: how few signals a 0.6 threshold produces compared with 0.55, and how little the
outcome changes when you move it. A rule whose result survives a small move of its threshold is
telling you something about the market; a rule whose result appears only at one exact setting is
telling you about the sample. Then add a 0.10 percent cost to every round trip and watch the
two-day version of the rule fall towards zero.

## Where this came from

- [The Strategy Compendium, article 12, machine learning](https://backtrader.readthedocs.io/en/latest/strategies-series/en/12-machine-learning.html),
  the category inventory and the deep dives into the composite score, the clustering rule and the
  q score, including every performance figure quoted above.
- [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md),
  this repository's brief on model evaluation, including `2603.16886v1`, `2108.02283v7` and
  `2311.14577v1`.
- [Gaussian naive Bayes](../../quantconnect/gaussian-naive-bayes-model/README.md),
  [gradient boosting](../../quantconnect/gradient-boosting-model/README.md) and the
  [temporal CNN forecaster](../../quantconnect/forecasting-stock-prices-using-a-temporal-cnn-model/README.md),
  the finished classifiers and forecasters in this collection.

## Words used in this tutorial

- overfitting: a model that has memorised the past it was shown rather than learned a pattern that
  carries into new data.
- percentile rank: the share of past values below today's value, written from 0 to 1.
- reward for risk (Sharpe ratio): the average return divided by how much it wobbled, usually written
  per year; zero means no reward once the wobble is counted.
- spread: the gap between the price at which something can be bought and the price at which it can
  be sold.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
