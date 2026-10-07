# Custom prediction: trading your own model's forecasts

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares or crypto of one thing, held when a model's forecast is strong enough in one direction                                                                                            |
| How often it trades       | Whenever the forecast crosses one of two limits, which for a daily model can be often                                                                                                    |
| What you need             | Python, a data file of prices, and a column of numbers from your own model                                                                                                               |
| Where the rules come from | [fastquant strategy library table](https://github.com/enzoampil/fastquant) and its [custom.py](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/custom.py) |
| The underlying research   | none, this is a practitioner's rule of thumb; the library's own example uses [Prophet](https://facebook.github.io/prophet/) forecasts, but no study tests the rule itself                |
| How well it held up       | Weak: the library publishes no measurement of the strategy, only a picture of one run, and it states no costs                                                                            |
| Also appears in           | [Custom ternary](../custom-ternary/README.md) in this collection, the same idea with three states instead of two limits                                                                  |

## The idea in one paragraph

When you have a model that produces a number for each day, such as a forecast of tomorrow's return,
you need a rule that turns those numbers into trades. This strategy takes one column of predictions
and two limits, an upper one and a lower one. When the number falls below the lower limit, it buys.
When the number rises above the upper limit, it sells. Everything in between is ignored. The column
can hold any forecast you like, which is what makes the strategy useful as a testing harness, and
also what makes it dangerous, because a bad forecast will be traded just as faithfully as a good one.

## Why anyone believed it

It is not a market idea so much as a testing idea. Researchers who build forecasting models, whether
they predict a price, a return, or a probability, need a fixed way to convert the model's output into
simple trades, so that two models can be compared on the same rules. This strategy supplies that
harness: put the forecast in a column, set the two limits, and read the resulting profit or loss.

The counterparty story depends entirely on the model, not on the rule. If the forecast contains real
information that other traders have not yet priced, then buying when the forecast is high means
buying from someone who has not seen it. If the forecast is only the past price restated, then the
trades are just the crowd's noise, and the buyer on the other side is whoever is happy to take the
other end. The rule itself takes no view; it inherits whatever edge, or lack of one, the model has.

## An everyday comparison

Think of a weather forecast that gives a percentage chance of rain for each day. A gardener who
waters only when the chance is below 20 percent, and covers the plants only when it is above 80
percent, is using two limits, not one. The middle forecast, a hazy 50 percent, is not acted on at
all. That is exactly the shape of this strategy: two thresholds, a dead zone in between, and the
quality of the garden depends on whether the forecast is any good, not on the two numbers chosen.

## The rules, step by step

1. Build a column of predictions, one row per day, and name it `custom` unless you tell the strategy
   another name with `custom_column`.
2. Decide what the numbers mean. A prediction of a next-day return in percent is a common choice; so
   is a model's confidence between 0 and 1.
3. Set `upper_limit` and `lower_limit`. The library's defaults are 95 and 5, which suit an indicator
   that runs from 0 to 100, but they are wrong for a return in percent.
4. Buy when the value is strictly below the lower limit. The library's rule is
   `custom < lower_limit`.
5. Sell when the value is strictly above the upper limit. The library's rule is
   `custom > upper_limit`.
6. Do nothing when the value is between the two limits, and hold whatever position you already have.
7. Buy with all the cash by default, and sell the whole holding, at the next day's closing price.
8. Mind the direction. The rule was written to look like the popular RSI indicator, where a low
   reading is treated as cheap and a high one as expensive. If your column is a forecast of a future
   return, the meaning is reversed: a high forecast should trigger a buy, not a sale. The library's
   own example handles this by multiplying the forecast by -1 before placing it in the column, which
   turns "high forecast" into "low value" and makes the buy rule fire. You can flip the sign, or you
   can swap the two limits; just do not leave the direction accidental.

The library's defaults and the library's example point in opposite directions, and that is the first
trap of this strategy. The defaults, 95 above and 5 below, are for a contrarian indicator. The
example, plus 1.5 above and minus 1.5 below, is for a return forecast. Read the sign of your own
column before you trust the result.

## The maths, with every symbol named

The rule is two comparisons, and the interesting arithmetic is what they cost.

```text
buy when V < L_low
sell when V > L_high
```

- `V` is the value in the custom column for that day.
- `L_low` is the lower limit, 5 by default.
- `L_high` is the upper limit, 95 by default.
- A position is held, untouched, whenever `L_low <= V <= L_high`.

The two limits are not symmetric around zero because the rule does not need them to be. They are set
to match the range of whatever is in the column. For the 0-to-100 indicator the defaults were written
for, 5 and 95 are two symmetric tails. For a forecast in percent, the useful pair is usually small
and centred on zero, such as +1.5 and -1.5. For a probability between 0 and 1, the pair might be 0.65
and 0.35. The asymmetry you will see in practice comes from the sign convention, not from the rule:
with the library's sign flip, a forecast of +2 percent becomes -2 and falls below a lower limit of
-1.5, which is what makes it a buy.

The forecast the model gives can be decomposed into two parts, and this is where most of them fail:

```text
forecast = direction * size
```

- `direction` is which way the model expects the price to move, up or down.
- `size` is how far it expects the move to go.

The rule only ever reads the combined number against a threshold, so it can be right about the
direction and still lose. A model that says "up" correctly on every day but always with a small
predicted size may never cross the limit, so the strategy trades rarely or never. A model that says
"up" correctly but with a wildly overstated size will cross the limit all the time and pay a cost on
every crossing. Thresholds reward a model that is right about direction and honest about size, which
is a stricter test than simple accuracy.

The cost of a round trip is the same as anywhere:

```text
Cost = 2 * c
```

- `2` counts the buy side and the sell side.
- `c` is the cost per side as a fraction, covering the gap between the buying and selling price plus
  any commission. At 0.1 percent per side, a round trip costs about 0.2 percent of the amount traded.

## A worked example

Eight days of a made-up model. The model predicts tomorrow's return in percent, the prices move
almost exactly as it predicts, and the strategy is set up the way the library's example is: the
column holds the prediction multiplied by -1, and the limits are +1.5 above and -1.5 below. So a
predicted rise of more than 1.5 percent becomes a value below -1.5 and triggers a buy.

| Day | Model's prediction for tomorrow | Value in the column (prediction times -1) | Signal            | Action                                     |
| --- | ------------------------------- | ----------------------------------------- | ----------------- | ------------------------------------------ |
| 1   | +0.4 percent                    | -0.4                                      | none              | none                                       |
| 2   | +1.8 percent                    | -1.8                                      | buy (below -1.5)  | the order fills the next close             |
| 3   | +2.1 percent                    | -2.1                                      | buy               | buy the whole account at 204.41            |
| 4   | +0.6 percent                    | -0.6                                      | none              | hold                                       |
| 5   | -0.3 percent                    | +0.3                                      | none              | hold                                       |
| 6   | -1.9 percent                    | +1.9                                      | sell (above +1.5) | the order fills the next close             |
| 7   | -2.4 percent                    | +2.4                                      | sell              | sell the whole holding at 205.34           |
| 8   | -0.2 percent                    | +0.2                                      | none              | nothing to do; the account is already flat |

The prices follow the predictions, starting from a close of 200.00 on day 1:

| Day | Close  | Move that produced it |
| --- | ------ | --------------------- |
| 1   | 200.00 | -                     |
| 2   | 200.80 | +0.4 percent          |
| 3   | 204.41 | +1.8 percent          |
| 4   | 208.70 | +2.1 percent          |
| 5   | 209.95 | +0.6 percent          |
| 6   | 209.32 | -0.3 percent          |
| 7   | 205.34 | -1.9 percent          |
| 8   | 200.41 | -2.4 percent          |

The strategy buys at day 3's close of 204.41 and sells at day 7's close of 205.34. The price move
while it was held is small:

```text
price gain = 205.34 / 204.41 - 1 = 0.00455, that is 0.455 percent
buy cost = 0.1 percent slippage + 0.1 percent commission = 0.2 percent
sell cost = 0.1 percent slippage + 0.1 percent commission = 0.2 percent
total cost = 0.4 percent
net = 0.455 - 0.4 = 0.055 percent
on 10,000.00 that is about 5.50
```

The model was right about the direction. It correctly called the rises on days 2, 3 and 4, and it
correctly called the falls on days 6, 7 and 8. It was also roughly right about the size of every
move, which is why the price path follows the predictions exactly. And the trade still earned almost
nothing, because the entry came one day late and the position sat through a fall of 0.3 percent and
then 1.9 percent before the sell filled. The gap between "right about direction" and "worth trading"
is where most forecast-based strategies live, and it is entirely made of timing and costs.

## What the research actually found

The library publishes no test. Its documentation shows one run that uses Prophet forecasts on
Bitcoin and prints a chart, and while the chart looks impressive, the page gives no final value, no
costs and no comparison against simply holding Bitcoin. That is a demonstration of the plumbing, not
a result. Because the column is the user's own model, "the research" here means the literature on
forecasting financial prices, and that literature is largely a story of forecasts that look good in
sample and fade out of sample.

| Source                                               | What it measured                                                              | Result                                                                                                                                    |
| ---------------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| The library's own custom-strategy page               | One Bitcoin run using Prophet forecasts                                       | A chart with no reported final value, no costs and no benchmark, so nothing can be concluded from it                                      |
| This collection's machine-learning for finance brief | The record of statistical and machine-learning price forecasts surveyed there | Forecasts that fit the past well often lose their edge out of sample, and the trading costs of acting on every change are rarely included |

The honest summary is that the rule is a neutral test harness, and a test harness tells you about
the model you put inside it. If your column is random, the strategy will trade random numbers and
pay costs. If the column is a good forecast, the strategy may capture part of it. The literature on
price forecasting leans strongly toward the first case, but it is the model's fault, not the rule's.

## How this project relates to it

This repository does not implement this exact two-limit rule, so the closest reading is the brief
that covers what happens when a model's output is turned into trades:
[machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md). Its
sections on out-of-sample testing and on the gap between a model's accuracy and a trading result are
the honest background. If you use this strategy, the discipline that matters is the one that brief
describes: keep the model's fitting data and its trading data apart, and report costs.

## Where it goes wrong

- The thresholds are chosen after seeing the results. It is easy to try many pairs of limits and
  keep the best, which turns a modest signal into a number produced by searching. The provider of
  the study does not tell you how many pairs were tried.
- Direction is confused with value. A model can be right about which way the price moves and still
  be misleading about how far, and the threshold only ever sees the combined number.
- Costs on a daily signal are brutal. A forecast that changes its advice often forces a round trip
  that costs about 0.2 percent, so a signal must clear that every time it trades, not once in a
  while.
- The library's default limits do not match a return forecast. Using 95 and 5 with a column of
  percentages will produce either no trades or the wrong ones, and the sign convention in the
  library's example can hide that.
- The signal is actable only one day late. The library fills on the next close, which is a fair rule
  but also a real loss of information, and a slow model plus a delayed fill can turn a good forecast
  into a flat result.
- A model fitted on the same data it trades on will always look good. Without a held-back period, the
  column is not a forecast at all, it is a record of the past.

## Try it yourself

You need a spreadsheet and no model, because the exercise is to see what a threshold does to a
forecast.

1. Make a column `V` of numbers between -3 and +3, one per day, of your own choosing. Call it a
   forecast in percent.
2. Add a column `Value` that multiplies `V` by -1.
3. Add a column `Signal` that says "buy" when `Value` is below -1.5, "sell" when it is above +1.5,
   and "none" otherwise.
4. Add a column `Day change` with a made-up daily price change.
5. Compute what the strategy would have held each day, and subtract 0.2 percent for every round trip.

What to notice: how few days are actually traded, and how much of the result is decided by whether
the buy happened to fill just before a rise. Then double every number in `V` and run it again. The
signals are identical, because only the order of the values relative to the limits matters. That is
the sense in which the rule throws away the size of the forecast, and it is the same property that
the three-state version shares.

## Where this came from

- [fastquant](https://github.com/enzoampil/fastquant), whose strategy table lists the custom strategy
  with the parameters `upper_limit`, `lower_limit` and `custom_column`, and its
  [custom.py](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/custom.py),
  which buys below the lower limit and sells above the upper one.
- The library's custom-strategy section of its README, which documents the Prophet-on-Bitcoin example
  and the sign flip it applies to the forecast.
- [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md),
  this collection's brief on statistical forecasts and out-of-sample testing.

## Words used in this tutorial

- threshold: the value a signal must cross before the strategy acts.
- forecast: a statement about the future, which says nothing by itself about what to trade.
- in sample: the period a model was fitted on, where it always looks better than it really is.
- out of sample: data kept aside and not used when the model or the rules were chosen.
- round trip: one buy and the matching sell of the same position.
- slippage: the difference between the price you expected and the price you actually got.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
