# Chaikin money flow: buying while the volume-weighted crowd is selling

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                              |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Cryptocurrency pairs on a spot exchange                                                                                                                                                            |
| How often it trades       | Often: the entry condition is true roughly half the time, so on five-minute candles the rule buys and sells many times a day                                                                       |
| What you need             | A spreadsheet and a table of five-minute open, high, low, close and volume figures                                                                                                                 |
| Where the rules come from | [TechnicalExampleStrategy.py in the freqtrade strategies repository](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/TechnicalExampleStrategy.py)   |
| The underlying research   | Marc Chaikin, [the origin of the money flow measure](https://chaikinanalytics.com/powerfeed/articles/the-birth-of-chaikin-money-flow); the rule built on it here is a practitioner's rule of thumb |
| How well it held up       | Weak: an illustration file the author never claimed earned anything, with no published test and no cost line                                                                                       |
| Also appears in           | nothing else in this collection                                                                                                                                                                    |

## The idea in one paragraph

This rule uses one number, called Chaikin money flow, which mixes price and volume. For each candle
it asks where the closing price sat between that candle's lowest and highest price, and it weights
that position by how much was traded. A close near the high scores as buying pressure, a close near
the low scores as selling pressure, and heavy volume makes the score count for more. The last
twenty-one candles are combined into a single number between minus one and plus one. When that
number is below zero, meaning the recent volume-weighted activity has been closing nearer the lows,
the rule buys. When the number turns above zero, it sells. The file is an example written to show
how to reuse a shared indicator, so the numbers below describe what it does, not a claim that it
earns.

## Why anyone believed it

A price alone does not say how hard the market agreed with it. A candle that closes near its high on
heavy volume means buyers were willing to pay up and were matched by a lot of supply; the same close
on tiny volume means almost nobody cared. Chaikin's measure tries to capture the difference, so that
a rise backed by volume is read as strength and a rise on a quiet day is read as noise. The rule
here reads a negative reading as a moment when sellers have been dominant, and buys, betting that
the selling has gone far enough to be reversed. The counterparty is the seller who closes near the
day's low on heavy volume, which on a falling chart is often someone selling in a hurry rather than
someone selling calmly.

## An everyday comparison

Picture a meeting where everyone raises a hand to vote, and each hand is weighted by how strongly
that person feels. A quiet "yes" counts a little; an angry, emphatic "no" counts a lot. The money
flow measure is that weighted vote over the last twenty-one candles: each candle's close near the
high is a strong "yes" for the buyers, each close near the low a strong "no", and the volume decides
how loudly each candle speaks. The rule buys while the weighted vote is still negative, on the
theory that the loudest voters were the ones being forced to leave.

## The rules, step by step

1. Collect five-minute candles for one cryptocurrency pair: for each candle, the open, the highest
   price, the lowest price, the close, and the volume, which is the amount traded in that five
   minutes.
2. For each candle, compute the money flow multiplier: where the close sat in the candle's range,
   written so that a close at the high is plus one, a close at the low is minus one, and a close in
   the middle is zero.
3. Multiply that multiplier by the candle's volume to get the money flow volume: a positive number
   for candles that closed high in their range, a negative number for candles that closed low.
4. Add the money flow volume of the last twenty-one candles, and divide by the total volume of those
   same twenty-one candles. The result is the money flow reading, between minus one and plus one.
5. Buy when that reading is below zero.
6. Sell when that reading is above zero.
7. Profit target. The file asks for a 1 percent gain measured from the opening price, which is a
   small target for a pair that can move that much in a few five-minute candles.
8. Loss limit. If the position is down 5 percent from the opening price, it is sold.
9. No trailing stop is used. The file does not state how many candles it needs before it can start,
   though the twenty-one-candle average needs twenty-one candles of history to be complete.

The sell side of the file carries a note from the author explaining that a different, simpler rule
is used for exits, chosen so that the example could be copied without change. That note is worth
taking at face value: this file is a demonstration of how to bring in an outside indicator, not a
finished strategy with a considered exit.

## The maths, with every symbol named

Three small calculations, repeated for each candle and then combined.

The money flow multiplier asks where the close sat within the candle:

```text
MFM = ((close - low) - (high - close)) / (high - low)
```

- `close`, `low` and `high` are the candle's closing, lowest and highest prices.
- The top of the line is positive when the close is nearer the high than the low, and negative when
  it is nearer the low.
- `MFM` is plus one when the close equals the high, minus one when it equals the low, and zero when
  the close is exactly halfway between them.

The money flow volume weights that position by the trading:

```text
MFV = MFM * volume
```

- `MFV` is the candle's contribution: positive for a candle that closed high and traded a lot,
  negative for one that closed low and traded a lot.
- `volume` is the amount traded during that candle.

The money flow reading is the sum over the window divided by the total volume:

```text
CMF = (MFV_1 + MFV_2 + ... + MFV_N) / (volume_1 + volume_2 + ... + volume_N)
```

- `N` is the number of candles in the window, twenty-one in this file.
- `CMF` is the result, between minus one and plus one. A reading of plus one would mean every
  candle in the window closed at its high; minus one would mean every candle closed at its low.

The profit target and the loss limit are single numbers rather than a ladder:

```text
exit if gain >= 0.01
stop price = entry_price * (1 - 0.05)
```

- `0.01` is the 1 percent profit target, measured from the opening price.
- `entry_price` is the price at which the position was opened, and `0.05` is the 5 percent loss
  limit.

The cost of a completed round trip is `2 * c`, where `c` is the charge per side as a fraction of the
amount traded. A realistic figure on a large crypto exchange is 0.0005 to 0.001, that is 0.05 to
0.10 percent per side, the level the cost primer in this collection uses. Two sides therefore cost
0.10 to 0.20 percent, which is between a tenth and a fifth of the 1 percent target.

## A worked example

Five invented five-minute candles for one pair. The columns are the numbers from the formulas above.
The range column is the high minus the low.

| Candle | High  | Low  | Close | Volume | Range | MFM   | MFV    |
| ------ | ----- | ---- | ----- | ------ | ----- | ----- | ------ |
| 1      | 10.00 | 9.00 | 9.80  | 100    | 1.00  | 0.60  | 60.0   |
| 2      | 10.00 | 9.20 | 9.30  | 120    | 0.80  | -0.75 | -90.0  |
| 3      | 9.60  | 8.80 | 8.90  | 150    | 0.80  | -0.75 | -112.5 |
| 4      | 9.20  | 8.40 | 8.60  | 200    | 0.80  | -0.50 | -100.0 |
| 5      | 9.00  | 8.20 | 8.40  | 180    | 0.80  | -0.50 | -90.0  |

The sum of the money flow volume column is 60.0 minus 90.0 minus 112.5 minus 100.0 minus 90.0,
which is minus 332.5. The total volume is 100 plus 120 plus 150 plus 200 plus 180, which is 750. So:

```text
CMF = -332.5 / 750 = -0.4433
```

That is below zero, so the rule buys. The position is opened at the next candle's open, 8.400. The
price rises to 8.500, a gain of 1.19 percent, which is above the 1 percent target, so the position
is sold.

```text
Gross gain      = 8.500 / 8.400 - 1 = 0.0119, that is 1.19 percent
Round-trip cost = 2 * 0.001 = 0.002, that is 0.20 percent
Net gain        = 1.19 - 0.20 = 0.99 percent
```

The 5 percent loss limit was never reached. The example shows the arithmetic, not that the rule
earns anything. One detail is worth pausing on: the cost is 0.20 percent against a 1 percent target,
so a fifth of the gain goes to the exchange and to the gap between the buying and selling price
before anything else. If the same rule trades every few minutes, that fraction repeats many times a
day.

## What the research actually found

There is no test of this rule, and the file does not claim one.

| Source                           | What it measured                                   | Result                                                                                                                                                |
| -------------------------------- | -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Chaikin's account of the measure | The origin of money flow and its multiplier        | The method and the reasoning behind it; a description rather than a study with a sample and costs                                                     |
| Park and Irwin (2007)            | A survey of technical trading rules across markets | Some rules showed promise in currency and futures markets before the 1990s, mostly before costs, and the effect weakened afterwards                   |
| The file itself                  | The rule the author wrote                          | A 1 percent target, a 5 percent loss limit and no trailing stop, with a note that the sell side is a placeholder; no pairs, period or result recorded |

Read together, the picture is this. The measure is a sensible description of what happened in a
candle: it says, honestly, how much volume arrived on candles closing near their high or low. The
rule built on top of it, buy while the reading is negative, is a choice nobody here tested. The
author's own note marks the file as an example rather than a strategy, and nothing in the repository
contradicts that reading.

## How this project relates to it

The measure sits in the same territory as order flow, and this repository studies that territory in
its [order flow and toxicity](../../../strategies/books/07_order_flow_and_toxicity.md) brief, whose
first finding is that buy and sell flow is strongly persistent: heavy selling tends to be followed
by more heavy selling, which is exactly the kind of it-cuts-both-ways fact a negative reading
obscures. The cost of trading this often is set out in
[costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md), and the reason a rule chosen
by eye fits its sample is the subject of
[overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).

## Where it goes wrong

- The measure is observed in past data; that a negative reading predicts a rise is a separate claim.
  Nobody in this file tested the second claim, and the file itself is marked as an example.
- The rule looks thin but the measure is not. A rule that reads "buy when the reading is negative"
  sounds like one condition, yet the reading blends the open, high, low, close and volume of
  twenty-one candles. Choosing the window length and the zero threshold after seeing results fits
  the past in the same way a rule with several conditions on the same candles does.
- Negative readings are common. A reading below zero happens roughly half the time, so the rule is
  in the market most of the time and pays the exchange charge and the gap between prices on every
  round trip, not only when something unusual happens.
- The target is small against the cost. A 1 percent target with a 0.20 percent round trip leaves
  0.80 percent when the target is reached, and the many losing trades eat into that.
- The exit is a placeholder. The author says the sell side is a different, simpler rule used to make
  the example copyable, so the file's exits were not chosen with the entry in mind.
- Close-position can be manufactured. A candle that closes near its high in a quiet, thin market
  scores just as strongly as one that closes near its high on real volume, because the measure uses
  volume relative to the other candles in the window, not volume compared with the wider market.

## Try it yourself

You need a spreadsheet and a public source of five-minute open, high, low, close and volume figures
for one coin pair.

1. Make columns: candle, high, low, close, volume.
2. Add a range column: the high minus the low.
3. Add a multiplier column: the close minus the low, minus the high minus the close, all divided by
   the range.
4. Add a money-flow-volume column: the multiplier times the volume.
5. Add a reading column using the last five candles: the sum of the money-flow-volume values divided
   by the sum of the volumes.
6. Add a signal column that says buy when the reading is below zero.
7. In the next rows, record the return over the following one, three and twelve candles.
8. Average those returns over every buy row, and then add the cost of 0.2 percent to each trade.

What to notice: the reading is negative on long stretches and positive on long stretches, so the
signal is not rare. When you subtract the cost from each trade, the average usually falls below
zero, which is the honest difficulty with a rule that is always either buying or selling on a
five-minute chart.

## Where this came from

- [TechnicalExampleStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/TechnicalExampleStrategy.py),
  the file that states the rules: the five-minute timeframe, the twenty-one-candle money flow
  reading, the below-zero entry, the above-zero exit, the 1 percent target and the 5 percent loss
  limit.
- Marc Chaikin, [the birth of Chaikin money flow](https://chaikinanalytics.com/powerfeed/articles/the-birth-of-chaikin-money-flow),
  the origin of the measure used here.
- Park and Irwin,
  [What do we know about the profitability of technical analysis?](https://onlinelibrary.wiley.com/doi/10.1111/j.1467-6419.2007.00519.x)
  (2007), the survey of how such rules have held up across markets.
- [Order flow and toxicity](../../../strategies/books/07_order_flow_and_toxicity.md) and
  [costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md), this repository's studies of
  the two forces behind a volume-based rule.

## Words used in this tutorial

- Chaikin money flow: a measure that weighs where each candle closed within its own range by that
  candle's volume, then sums over a window.
- money flow multiplier: the part of that measure that says where the close sat between the low and
  the high, from minus one to plus one.
- money flow volume: the multiplier multiplied by the candle's volume.
- order flow: the record of buying and selling that reaches the market, which the measure is a
  simple version of.
- volume: the amount traded during a period.
- weighted vote: an average in which some voters count for more than others, here the louder ones
  being the candles with more volume.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
