# Forecasting with a temporal convolutional network: guessing a share's next few days from its last fifteen

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                             |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of a small group of large technology companies, one bet at a time                                                                                                                          |
| How often it trades       | One decision every day; each bet is held for between one and five days                                                                                                                            |
| What you need             | Python and a data file                                                                                                                                                                            |
| Where the rules come from | [QuantConnect strategy library, forecasting stock prices using a temporal CNN model](https://www.quantconnect.com/tutorials/strategy-library/forecasting-stock-prices-using-a-temporal-cnn-model) |
| The underlying research   | Passalis, Tefas, Kanniainen, Gabbouj and Iosifidis, [Temporal Logistic Neural Bag-of-Features for Financial Time Series Forecasting](https://arxiv.org/abs/1901.08280)                            |
| How well it held up       | Weak: the library page reports ten runs of its own code whose average reward for risk was negative while the technology index scored strongly positive, and no other group has repeated the test  |
| Also appears in           | nothing else in this collection                                                                                                                                                                   |

## The idea in one paragraph

A computer program is shown the last fifteen days of one company's share price and the number of shares
traded each day. From thousands of past examples it learns which short stretches of that history tend to
come just before the price is higher over the following five days. When it sees a stretch of history it
has met before, it does not answer "yes" or "no"; it reports how confident it is, as percentages for
three outcomes: the next five days will average higher, lower, or almost unchanged. The rules buy the
share when the confidence in "higher" passes 55 percent, and they sell it short when the confidence in
"lower" passes 55 percent. Each bet is closed after one to five days, a length chosen at random. The
whole thing is a pattern-recognition machine pointed at a price chart.

## Why anyone believed it

A convolutional network is the same tool that recognises faces in photographs, and a price chart is
also a picture with shapes in it: a slow rise, a flat spell, a sudden drop and recovery. If a photograph
can be sorted into "cat" and "dog" by learning the shapes, the argument goes, a chart can be sorted into
"about to rise" and "about to fall" the same way. The person on the other side of the trade is someone
who reacts slowly to news, or who is forced to sell for reasons that have nothing to do with the outlook,
so that the same shape of buying pressure appears again and again. The appeal is that the machine finds
the shape on its own, rather than a person deciding in advance what "a rise" means.

## An everyday comparison

Think of a nurse learning to read the long paper strip of a heart monitor. The strip is a wiggly line,
and a nurse who watches thousands of strips learns that a particular small shape, seen minutes earlier,
often comes before a specific problem later in the day. The nurse does not measure the whole line at
once. They slide a short window along it and ask, at every position, "does this little piece look like
the warning shape?". A convolutional network does exactly this: it slides a short window along the price
history and scores every window for the shapes it has learned to care about.

## The rules, step by step

1. Choose a small group of large technology shares, described on the library page as three shares.
2. Collect daily bars for each share: the opening price, the highest price, the lowest price, the
   closing price, and the number of shares traded (the volume).
3. Build an input window of the last fifteen daily bars, oldest first, with all five numbers for each
   day. Fifteen days times five numbers is seventy-five numbers, and that is what the model reads.
4. Build the answer the model must learn, using prices that come after the window. Take the average of
   the next five closing prices, then compute the percentage change from today's close to that average.
5. Label the example "up" when that percentage change is greater than 0.01 percent, "down" when it is
   less than minus 0.01 percent, and "stationary" for anything in between. The 0.01 percent is the
   library page's threshold, written there as 0.0001.
6. Train the network on many such windows and labels from the past, so that it learns the shapes.
7. When trading, feed in the most recent fifteen bars and read the three confidences the model returns.
8. Buy the share if the confidence in "up" is above 55 percent, and sell it short if the confidence in
   "down" is above 55 percent. Do nothing if neither passes.
9. Close the bet after a whole number of days between one and five, chosen at random, and start again
   from step 7 the next day.

## The maths, with every symbol named

The learning target is one average and one percentage change.

```text
A = average of the next five closing prices
g = (A - C) / C
```

- `A` is the average of the closing prices on the five days after the input window ends.
- `C` is the closing price on the last day of the input window.
- `g` is the forward change as a decimal: 0.0058 means the next five days average 0.58 percent higher.

The label is then chosen by comparing `g` with the threshold:

```text
label = "up" if g > 0.0001, "down" if g < -0.0001, otherwise "stationary"
```

- `0.0001` is one hundredth of one percent, which the library page writes as the stationary threshold.
- "Stationary" means the average is almost unchanged, within a hundredth of a percent either way.

The heart of the network is a sliding window, also called a filter or a convolution. At every position
along the price series it computes a weighted average of a few neighbouring days:

```text
w1 * x(k) + w2 * x(k+1) + w3 * x(k+2) + w4 * x(k+3)
```

- `x(k)` is the value (a price, or a volume) on day `k`.
- `w1`, `w2`, `w3` and `w4` are four numbers the training process adjusts; they are not chosen by hand.
- The filter is four days wide, so sliding it along fifteen days gives twelve positions, because
  15 - 4 + 1 = 12.

If all four weights were the same, 0.25 each, the filter would simply be the plain average of four
consecutive days. Training is what makes them unequal, so that one filter becomes a "sharpening" shape
and another becomes a "smoothing" shape. The library page uses thirty such filters at once.

The twelve positions are then split into three groups of four: the earliest four (called long term), the
middle four (mid term) and the latest four (short term). A second, one-wide weighted average squeezes
each of the three groups down to a short list of numbers, the lists are stacked into one list of twelve
numbers, and a final step turns those twelve numbers into three confidences that add up to 1. The
trading decision is a comparison with the confidence threshold:

```text
trade = long if P(up) > 0.55; short if P(down) > 0.55; otherwise do nothing
```

- `P(up)` and `P(down)` are the model's confidences, between 0 and 1. A confidence of 0.62 means the
  model places a 62 percent weight on that outcome.
- `0.55` is the library page's cutoff. Predictions below it are ignored.

The cost of a bet, and the return to the account, are the usual two lines:

```text
net return = position * price change - cost
cost = 2 * c, because a bet is opened and later closed
```

- `position` is +1 for a long bet and -1 for a short bet, meaning you profit when the price moves your
  way and lose when it moves against you.
- `price change` is the change in the price over the days the bet is held, as a decimal.
- `c` is the cost of one trade, covering the gap between the buying and the selling price plus any
  commission; the library page does not print a cost figure, so any number used below is an assumption.

## A worked example

The twelve closing prices below are invented, but they are of a size that a real share price takes.
Remember that each row uses the five days that follow it, so the last rows have no label yet.

| Day | Close  | Average of the next five closes | Forward change g | Label |
| --- | ------ | ------------------------------- | ---------------- | ----- |
| 1   | 100.00 | 101.18                          | +0.0118          | up    |
| 2   | 100.60 | 101.12                          | +0.0052          | up    |
| 3   | 101.20 | 100.84                          | -0.0036          | down  |
| 4   | 101.80 | 100.52                          | -0.0126          | down  |
| 5   | 101.40 | 100.38                          | -0.0101          | down  |
| 6   | 100.90 | 100.42                          | -0.0048          | down  |
| 7   | 100.30 | 100.66                          | +0.0036          | up    |

For day 1, for instance, the next five closes are 100.60, 101.20, 101.80, 101.40 and 100.90; they add to
505.90, and dividing by five gives 101.18. The change from 100.00 is 101.18 / 100.00 - 1 = 0.0118, which
is 1.18 percent, so the label is "up". As a first filter, the model with four equal weights applied to
the last four closes of the window (99.80, 100.20, 100.70, 101.10) would give 100.45, that is
(99.80 + 100.20 + 100.70 + 101.10) / 4.

Now suppose the trained model, looking at each day, returns the confidences below, and suppose each bet
is held exactly one day so that the arithmetic is easy to follow. The cost is taken as 0.0005 per side,
that is five basis points, so a round trip of one buy and one sell costs 0.001, that is 0.10 percent.
One basis point is one hundredth of one percent. These confidences and costs are invented for the
example; the point is the arithmetic, not a prediction.

| Day | Confidence | Decision | Entry  | Next close | Gross    | Cost    | Net      |
| --- | ---------- | -------- | ------ | ---------- | -------- | ------- | -------- |
| 2   | P(up)=0.62 | buy      | 100.60 | 101.20     | +0.5964% | -0.100% | +0.4964% |
| 3   | P(dn)=0.58 | sell     | 101.20 | 101.80     | -0.5929% | -0.100% | -0.6929% |
| 4   | P(dn)=0.61 | sell     | 101.80 | 101.40     | +0.3929% | -0.100% | +0.2929% |
| 5   | P(up)=0.57 | buy      | 101.40 | 100.90     | -0.4931% | -0.100% | -0.5931% |
| 6   | P(dn)=0.64 | sell     | 100.90 | 100.30     | +0.5946% | -0.100% | +0.4946% |
| 7   | P(up)=0.56 | buy      | 100.30 |   99.80    | -0.4985% | -0.100% | -0.5985% |

Days 1 and 8 are ignored because the confidence in the winning direction never passed 55 percent.

Add the six net returns: +0.4964 - 0.6929 + 0.2929 - 0.5931 + 0.4946 - 0.5985 gives about -0.60 percent.
The gross returns, before cost, add to almost exactly zero (-0.0006 percent): three of the six bets won
and three lost, which is what a coin-toss model produces. The costs then turned a flat result into a
loss of 0.60 percent over six days. That is the single most important arithmetic fact in this tutorial:
a model with no edge still trades, still wins half the time, and still loses money.

## What the research actually found

The library page reports what happened when its own code was run, and the authors were candid about it.

| Source                                                     | What it measured                                                                                   | Result                                                                                                                                                                                                                     |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| QuantConnect library page, forecasting with a temporal CNN | The page's own code, ten repeat runs, three technology shares                                      | Average reward for risk of -0.274, with an annual standard deviation of 0.139, against +0.877 for the technology index over the same period                                                                                |
| `2603.16886v1` (abstract; p.1)                             | Nine deep-learning architectures, 918 runs, three asset classes                                    | No architecture showed directional accuracy different from a coin toss across all 54 model-category-horizon combinations; architecture choice explained 99.90 percent of forecast error, the random seed only 0.01 percent |
| `2110.14914v2` (pp.5, 7, 8)                                | Logistic regression, random forests, feed-forward and recurrent networks on five commodity futures | Classifiers were 52 to 56 percent accurate on the next move, yet at a normal slippage of 0.3 ticks many configurations were not profitable; the authors note misclassifications have unequal cost                          |

The page's own result is the most direct evidence, and it is negative. The comparison table on the page
shows the strategy's reward for risk below zero while the technology index was strongly positive over
the same five-ish years. The two papers read for this tutorial say why that is not a surprise. The
first is a controlled comparison of nine deep-learning models on hourly data: even the best architecture
ranked prices accurately but had no directional skill, meaning it could not tell up days from down days
better than chance once the error was measured honestly. The second shows the gap directly between a
good accuracy score and a profitable strategy: a classifier right 55 percent of the time still lost money
after realistic trading costs, because the times it was wrong cost more than the times it was right. The
loss function, which is how the model is judged during training, also matters: several of the papers
collected in [the machine-learning brief](../../../strategies/books/09_machine_learning_for_trading.md) find that
training a model to predict the size of a move and then trading its sign is a different and harder task
than training it to get the sign right.

## How this project relates to it

The repository's own reading of this literature lives in three documents.

- [Machine Learning for Trading](../../../strategies/books/09_machine_learning_for_trading.md)
  is the closest match. Section 2 covers deep learning on order books and the difference between
  low prediction error and profitability; Section 6 is specifically about how costs and turnover decide
  whether a signal is deployable, and its table of takeaways ends with the rule "train on the trading
  objective, not point-forecast error".
- [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md)
  collects the evaluation designs: the loss-function experiment, the architecture comparison, and the
  finding that a model can be accurate and still have no directional skill. Its first short-answer
  point is that simple baselines (here, gradient boosting) often match the more complicated networks.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
  is where the honest arithmetic lives: one erroneous data row can turn a published gain into a loss,
  and a search over many settings will always produce one winner by chance. Its "what a research
  protocol must record" section lists exactly what a tutorial like this one should have printed and
  did not, such as the full trial count and the point-in-time data window.

The repository does not contain a runnable copy of this CNN. The nearest implemented thing is the
validation discipline those briefs describe, which is what a reader would use to judge the code if the
CNN were ported here.

## Where it goes wrong

- Overfitting. With fifteen days times five numbers as input and only a few years of daily data, a
  network can memorise the specific past rather than learn a shape. The tell is a model that is very
  accurate on the data it trained on and near a coin toss on data it has never seen. This is the whole
  subject of [the overfitting brief](../../../strategies/books2/28_overfitting_and_research_integrity.md).
- Direction is not money. A model can be right about the direction of the next five days and still lose,
  because the days it is wrong on are the large moves. Section 6 of
  [the machine-learning brief](../../../strategies/books/09_machine_learning_for_trading.md) makes this
  the central point: a forecast has to be worth more than the cost of acting on it.
- A high hit rate is not an edge. A classifier can call the direction right more often than not and
  still lose, because the days it is wrong on are the larger moves; see the accuracy figures above.
- The label uses the future. The five-day average that defines the label comes from prices after the
  window, which is correct for training but a defect if that calculation ever touches the trading
  period, because the model would be reading the answer. Keep the training window strictly before it.
- Scaling across the whole file. The library page rescales the inputs before training. If the highest
  and lowest prices used are taken from the whole file rather than the past only, the model has been
  told something about the future, which is small on a large backtest and decisive on a serious one.
- Costs. The page prints no cost for this strategy. Real costs for three shares are small per trade, but
  the strategy trades every day, and the worked example above shows how quickly 0.10 percent per round
  trip consumes a flat signal.
- A regime that ends. The belief rests on the shapes of buying pressure repeating. When the market's
  participants and rules change enough that they no longer repeat, there is nothing left to recognise.

## Try it yourself

You need a spreadsheet and a public source of daily prices for one large technology share. No money and
no code are involved.

1. Put the last six months of daily closing prices in column A, one price per row, with the date in
   column B.
2. In column C, compute the average of the five closes starting one row below the current row. This is
   the five-day forward average.
3. In column D, compute the forward change: the column C value divided by the column A value, minus one.
4. In column E, write the label: "up" if column D is above 0.0001, "down" if below minus 0.0001, and
   "stationary" otherwise.
5. Count how many rows are "up", how many "down" and how many "stationary". Write those three numbers
   in a corner of the sheet.
6. Repeat steps 1 to 5 for the technology index over the same dates, and compare the two sets of counts.

What to notice: the labels are close to a three-way split only if the stationary band is wide; with a
band as narrow as 0.01 percent, almost every row is "up" or "down", and the split between those two is
roughly even over any long stretch. That even split is the reason a classifier can score above 50
percent on a coin toss and still be worthless: 50 percent of a three-way problem is not the same as a
useful edge. Now look at your sheet again and ask how many of the "up" rows were followed by a big
down move in the following five days. Those are the rows that would cost a strategy the most money.

## Where this came from

- [the CNN page](https://www.quantconnect.com/tutorials/strategy-library/forecasting-stock-prices-using-a-temporal-cnn-model),
  the rules as implemented for the temporal CNN: a fifteen-day window of open, high, low, close and
  volume, a five-day
  forward average as the label, a 0.01 percent stationary band, a 55 percent confidence cutoff, and a
  holding period of one to five days. The research page is at
  [research/15263](https://www.quantconnect.com/research/15263/forecasting-stock-prices-using-a-temporal-cnn-model/).
- Passalis, Tefas, Kanniainen, Gabbouj and Iosifidis, `1901.08280`, "Temporal Logistic Neural
  Bag-of-Features for Financial Time Series Forecasting", the paper the library page cites. There is no
  local copy of this paper in the repository's harvested corpus, so it is named but not quoted.
- `2603.16886v1`, "A Controlled Comparison of Deep Learning Architectures for Multi-Horizon Financial
  Forecasting: Evidence from 918 Experiments", the source for the coin-toss directional accuracy and
  the 99.90 percent architecture-versus-seed split.
- `2110.14914v2`, "Trading via Selective Classification", the source for the 52 to 56 percent accuracy
  figures and for the finding that accuracy did not survive normal slippage.
- [Machine Learning for Trading](../../../strategies/books/09_machine_learning_for_trading.md),
  [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md) and
  [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own briefs on the evaluation questions a model of this kind raises.

## Words used in this tutorial

- basis point: one hundredth of one percent, so five basis points is 0.05 percent.
- confidence: the model's own statement of how strongly it favours one outcome, as a percentage.
- convolutional network: a pattern-recognition program that slides a short window along a series and
  scores the shape it finds at every position.
- forward change: the percentage difference between a later price and today's, used as the answer the
  model is trained to predict.
- overfitting: a model that has memorised the past it was shown rather than learned a pattern that
  carries into new data.
- reward for risk (Sharpe ratio): the average return divided by how much it wobbled, usually written
  per year; zero means no reward after the wobble is counted.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
