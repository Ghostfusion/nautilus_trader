# Custom ternary: a signal that only says buy, sell or wait

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares or crypto of one thing, held while a three-state signal says buy and sold when it says sell                                                                                       |
| How often it trades       | Only when the signal changes state, so it can be calm for long stretches and then trade repeatedly                                                                                       |
| What you need             | Python, a data file of prices, and a column of values that are only -1, 0 or 1                                                                                                           |
| Where the rules come from | [fastquant strategy library table](https://github.com/enzoampil/fastquant) and its [custom.py](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/custom.py) |
| The underlying research   | none, this is a practitioner's rule of thumb; the signal encoding is the library's own and no study tests it                                                                             |
| How well it held up       | Weak: the library publishes no measurement of this strategy at all, only the rule                                                                                                        |
| Also appears in           | [Custom prediction](../custom-prediction/README.md) in this collection, the two-limit version this one is a special case of                                                              |

## The idea in one paragraph

Sometimes a model does not need to say how much, only which way. This strategy takes a column whose
values are only three things: buy, sell, or wait. The library writes those as 1, -1 and 0. When the
value is 1 it buys, when the value is -1 it sells, and when it is 0 it does nothing. The model's job
is reduced to a single decision a day, which is simpler to build, simpler to explain, and harder to
fit too closely to the past. The price of that simplicity is that everything about how big the move
might be is discarded before the rule even sees it.

## Why anyone believed it

The appeal is honesty about what most models can actually do. A model that claims to forecast a price
to the cent is usually overfitting, dressing a vague signal in false precision. A model that says
only "up", "down" or "no view" makes a weaker claim, and a weaker claim is easier to test and harder
to fool yourself with. Two people building a three-state rule tend to agree on the signals, whereas
two people fitting a continuous forecast will disagree about the last decimal place and about the
thresholds.

The counterparty is the same as for any forecast rule: the trader who has not yet acted on whatever
the state is reading. The three-state rule does not change who is on the other side; it only changes
how much noise the strategy lets into its own decisions.

## An everyday comparison

Think of a traffic light rather than a speedometer. A traffic light tells you stop, go, or wait, and
nothing about how fast to drive. A speedometer gives a precise number that a careful driver reads
differently in different conditions. A three-state signal is the traffic light: cheap to build,
impossible to misread the same way twice, and blind to anything between the extremes. The speedometer
version can be better, but only if the measurement is trustworthy; the traffic light is the safer
choice when it is not.

## The rules, step by step

1. Build a column of values, one row per day, named `custom` unless you give another name with
   `custom_column`. Every value must be one of the three states.
2. Choose which number means buy, which means sell, and which means wait. The library's defaults are
   `buy_int` equal to 1, `sell_int` equal to -1, and 0 for no action.
3. Buy when the value equals the buy number. The library's rule is `int(custom) == buy_int`.
4. Sell when the value equals the sell number. The library's rule is `int(custom) == sell_int`.
5. Do nothing when the value is anything else, which in practice is only 0. The library converts the
   value to a whole number first, so 1.2 and 0.8 both become 1, and a stray 0.6 becomes 0.
6. Buy with all the cash by default, and sell the whole holding, at the next day's closing price.
7. Notice that there is no sign flip here, unlike the two-limit version. You choose the encoding, so
   you label a forecast of a rise as 1 and a forecast of a fall as -1, and the rule reads them in the
   natural direction. The trap that exists in the two-limit version, where the library's example has
   to multiply the forecast by -1, does not exist here.

## The maths, with every symbol named

The whole rule is two equality tests:

```text
buy when value == buy_int
sell when value == sell_int
```

- `value` is the number in the custom column for that day.
- `buy_int` is the number that means buy, 1 by default.
- `sell_int` is the number that means sell, -1 by default.
- `==` means "is equal to"; the numbers are rounded down to whole numbers first, so the comparison is
  exact.

Because only equality matters, the rule is immune to scale. Doubling every value in the column, or
changing 1 to 100 and -1 to -100, leaves the signals unchanged as long as the two numbers you name as
`buy_int` and `sell_int` change with them. That is the property that makes the three-state rule
robust: it does not care how large the model's numbers are, only what label they were given.

The cost is the same as for any strategy, and it is usually the deciding term:

```text
Cost = 2 * c
```

- `2` counts the buy side and the sell side of a round trip.
- `c` is the cost per side as a fraction, covering the gap between the buying and selling price plus
  any commission. At 0.1 percent per side, a round trip costs about 0.2 percent of the amount traded.

The information the rule throws away can be written plainly. A continuous forecast `F` is turned into
a state by a mapping:

```text
state = 1  if F is a strong buy
state = -1 if F is a strong sell
state = 0  otherwise
```

- `F` is whatever the model produces.
- `state` is the only thing the strategy sees.
- The size of `F` is gone: a forecast of +0.6 and a forecast of +6.0 can both become the same 1.

## A worked example

The same eight days as the two-limit tutorial, so the two rules can be compared directly. The model's
direction is encoded as three states: 1 for a predicted rise of more than 1 percent, -1 for a
predicted fall of more than 1 percent, and 0 otherwise.

| Day | Model's view for tomorrow | Value in the column | Signal | Action                                     |
| --- | ------------------------- | ------------------- | ------ | ------------------------------------------ |
| 1   | small rise, +0.4 percent  | 0                   | none   | none                                       |
| 2   | clear rise, +1.8 percent  | 1                   | buy    | the order fills the next close             |
| 3   | clear rise, +2.1 percent  | 1                   | buy    | buy the whole account at 204.41            |
| 4   | small rise, +0.6 percent  | 0                   | none   | hold                                       |
| 5   | small fall, -0.3 percent  | 0                   | none   | hold                                       |
| 6   | clear fall, -1.9 percent  | -1                  | sell   | the order fills the next close             |
| 7   | clear fall, -2.4 percent  | -1                  | sell   | sell the whole holding at 205.34           |
| 8   | small fall, -0.2 percent  | 0                   | none   | nothing to do; the account is already flat |

The prices are the same as in the two-limit tutorial, starting from 200.00 and following the model's
predictions closely:

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

The trade is the same one the two-limit version made, because on these particular days both rules
crossed on the same dates: buy at day 3's close of 204.41 and sell at day 7's close of 205.34.

```text
price gain = 205.34 / 204.41 - 1 = 0.00455, that is 0.455 percent
buy cost = 0.1 percent slippage + 0.1 percent commission = 0.2 percent
sell cost = 0.1 percent slippage + 0.1 percent commission = 0.2 percent
total cost = 0.4 percent
net = 0.455 - 0.4 = 0.055 percent
on 10,000.00 that is about 5.50
```

The two rules agreed here, but that is a coincidence of the example, and it is worth seeing where
they would differ. Suppose day 4's model output had been a large +2.5 percent instead of a small
+0.6 percent. The two-limit version would have held its existing position either way, so its trade
would be unchanged. The three-state version would also have held, because it was already in the
market. Now suppose the first clear signal had come on day 4 rather than day 2. The two-limit version
would have bought one day earlier than the three-state version, because a small prediction just above
its lower limit is still a buy, while the three-state version waits for a value of exactly 1. The
difference is entirely about where the boundary between "small" and "clear" is drawn, and both rules
depend on that choice.

## What the research actually found

There is nothing to report, and that is the honest answer. The library's strategy table lists the
ternary strategy with its three parameters, and the source file gives the rule in a handful of lines,
but no test, no data period and no cost assumption appear anywhere in the project. The three-state
encoding is a convenience for the user's own model, and its evidence is exactly the evidence for that
model, which this page cannot supply.

What can be said from principle is narrower. A three-state rule discards the size of the signal, so
it can only be as good as the accuracy of the state, and it must pay one round trip for each genuine
change of state. If the model's states change often, the costs add up; if they change rarely, the
costs are small but so is the number of chances to profit. The library provides no measurement of how
those trade off in practice.

## How this project relates to it

This tutorial and [custom prediction](../custom-prediction/README.md) describe the same strategy
family; the three-state version is what you get when you take the two-limit version, replace the two
limits with a single threshold on the model's own output, and throw away the magnitude. The
repository's [machine learning for finance brief](../../../strategies/books2/04_machine_learning_for_finance.md)
is the place to read about the modelling questions that sit underneath: how a signal is built, how it
is tested out of sample, and why a simple label is often more honest than a precise-looking number.

## Where it goes wrong

- The encoding hides the model's confidence. Two very different forecasts become the same 1, so a
  model that is barely convinced and one that is certain produce identical trades.
- The boundaries are still chosen by hand. Moving the line between "small" and "clear" from 1 percent
  to 0.5 percent changes every signal, and the library gives no guidance and no test to set it.
- Zero is doing double duty. It can mean "no view", "a small move in either direction", or "not
  enough data yet", and the rule treats all three as "do nothing", which may hide a model that has
  stopped working.
- The strategy has no exit other than the sell state. If the model never emits a -1, the position is
  never closed, so the result depends on the model's willingness to say "sell" as much as on its
  accuracy.
- Costs still apply at every change of state. A model that flickers between 0 and 1 forces a round
  trip each time, and at 0.2 percent per round trip a chatty signal is expensive.
- The library rounds before comparing. A value of 1.4 counts as 1 and a value of 0.6 counts as 0, so
  a model that outputs probabilities rather than clean states will behave in ways its author may not
  expect.

## Try it yourself

You need only a spreadsheet.

1. Take the value column you built in the two-limit exercise, or make a new one with a few dozen
   numbers between -3 and +3.
2. Add a column `State` that puts a 1 where the value is above +1, a -1 where it is below -1, and a 0
   in between.
3. Add a column for the made-up daily price change, then work out what the rule would hold each day:
   after a 1 you are in, after a -1 you are out, and a 0 changes nothing.
4. Count the number of round trips and multiply by 0.2 percent of the money to get the cost.
5. Now double every number in the value column and run it again.

What to notice: the second run produces exactly the same states and the same trades, no matter how
large the numbers became. That immunity to scale is the strength of the three-state idea. Then look
at the days whose value was near +1 and ask how the result would change if the line were at +0.5
instead. That sensitivity to an arbitrary boundary is its weakness.

## Where this came from

- [fastquant](https://github.com/enzoampil/fastquant), whose strategy table lists the ternary
  strategy with the parameters `buy_int`, `sell_int` and `custom_column`, and its
  [custom.py](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/custom.py),
  which buys when the value equals `buy_int` and sells when it equals `sell_int`.
- [Custom prediction](../custom-prediction/README.md), the two-limit version of the same rule, in
  this collection.
- [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md),
  this collection's brief on how signals are built and tested.

## Words used in this tutorial

- signal: an observable value a strategy uses to decide whether to act.
- state: one of a small fixed set of values, here buy, sell or wait.
- scale: how large the numbers are, as opposed to what order they are in.
- overfitting: fitting the past so closely that the rules stop working on new data.
- round trip: one buy and the matching sell of the same position.
- out of sample: data kept aside and not used when the model or the rules were chosen.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
