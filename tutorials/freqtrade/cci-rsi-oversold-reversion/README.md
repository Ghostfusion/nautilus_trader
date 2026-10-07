# Buying after a deep fall: the CCI and RSI rule in SwingHighToSky.py

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                             |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Cryptocurrency pairs on a spot exchange                                                                                                                                                                                           |
| How often it trades       | Often: the entry condition appears whenever a price falls far below its own recent average, which on a fifteen-minute chart is several times a day in a busy market                                                               |
| What you need             | A spreadsheet and a table of fifteen-minute open, high, low and close prices                                                                                                                                                      |
| Where the rules come from | [SwingHighToSky.py in the freqtrade strategies repository](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/SwingHighToSky.py)                                                                    |
| The underlying research   | Donald Lambert's [Commodity Channel Index](https://store.traders.com/-v01-c05-comm-pdf.html) (1980) and J. Welles Wilder's [New Concepts in Technical Trading Systems](https://archive.org/details/newconceptsintec00wild) (1978) |
| How well it held up       | Weak: a mean-reversion rule whose numbers came from one parameter search, with no independent test and no cost line                                                                                                               |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                   |

## The idea in one paragraph

Two numbers, called the commodity channel index and the relative strength index, each compare a
price with its own recent behaviour. The first asks how far the typical price of a candle sits below
the average of the last several typical prices, divided by how much those prices normally wobble.
The second asks how much of the recent movement was upward and how much downward. When both numbers
say the price is very low, this rule buys, betting that a fall that steep does not continue and that
the price will spring back. The position is closed by a profit ladder, a loss limit, or by both
numbers turning positive. The strategy trades fifteen-minute candles, so it looks many times a day.
The file's name suggests a rule about breaking above a recent high; the code does the opposite, so
this tutorial explains what the file actually does.

## Why anyone believed it

A price does not move because of a number on a chart; it moves because people are buying and
selling, and some of those people have no choice. When a price falls fast, the sellers are often
those who borrowed to buy and are being forced to repay, or whose automatic loss limits have all
fired at once, or who simply panic. Those sales push the price below where a calm buyer would pay,
and when the forced selling stops, the price springs back. The person running this rule is the calm
buyer, stepping in while the forced sellers are still pushing, and being paid for taking the other
side of a sale that had to happen.

## An everyday comparison

Think of a shop that marks down a jacket every day it does not sell. After a week the price can be
well below what other shops charge for the same jacket, not because the jacket changed, but because
this shop needs to clear it. A shopper who knows the jacket's worth buys when the markdown looks too
large, and waits for the price to return to normal before selling it on. The commodity channel index
and relative strength index are ways of measuring how deep the markdown is and how one-sided the
recent sales have been, so the shopper buys when the discount is extreme rather than merely present.

## The rules, step by step

1. Collect fifteen-minute prices for one cryptocurrency pair: for each fifteen-minute candle, the
   open, the highest price, the lowest price and the close.
2. For each candle, compute the typical price: the average of its high, low and close.
3. Compute the commodity channel index over the last 72 candles, and the relative strength index
   over the last 36 candles. The formulas are in the next section.
4. Buy when both of these hold: the 72-candle commodity channel index is below minus 175, and the
   36-candle relative strength index is below 90. The minus-175 level is far below zero. The
   below-90 level is high enough that it is almost always true, because the relative strength index
   sits above 90 only after an extremely strong run, so in practice the first condition decides.
5. Hold the position. Close it when a profit target, a loss limit, or the exit condition says so.
6. The written exit condition is: the 66-candle commodity channel index is above minus 106, and the
   45-candle relative strength index is above 88. Because the second part is again rare, the written
   exit rarely fires, and most trades are closed by the ladder or the loss limit instead.
7. Profit target ladder. Measured from the opening price: 27.058 percent at once, 8.53 percent after
   33 minutes, 4.093 percent after 64 minutes, and any profit at all after 244 minutes.
8. Loss limit. If the position is down 34.338 percent from the opening price, it is sold.
9. No trailing stop is used. The strategy needs enough candles to fill its longest average before it
   can signal, and it reviews the pair every fifteen minutes.

The numbers above are not the ones written beside the parameter names in the file; the file declares
search ranges there and then overrides them with a block labelled as the result of a search. The
effective values are the ones used here: 72 and 36 for the two lookbacks on the way in, 66 and 45 on
the way out, and minus 175, 90, minus 106 and 88 for the four levels.

Two words in the file's name need explaining, because they describe a different idea. A swing high
is a local peak: a candle whose highest price is above the highest prices of the candles just before
and just after it, so the price climbed into it and fell away again. A breakout is what happens when
the price rises above a level it had not exceeded for some time, such as the last swing high, on the
argument that the sellers who defended that level have gone. Breaking out is a bet that a rise
continues. This file buys after falls, not after rises, so it is the opposite kind of rule: a
mean-reversion rule, not a breakout rule.

## The maths, with every symbol named

The typical price of one candle is a simple average of three of its four prices:

```text
TP = (high + low + close) / 3
```

- `high`, `low` and `close` are the candle's highest price, lowest price and closing price.
- `TP` is the typical price, a single number standing for the candle.

The commodity channel index compares the latest typical price with the average of the last `N`
typical prices, scaled by how much those typical prices usually differ from that average:

```text
SMA  = (TP_1 + TP_2 + ... + TP_N) / N
MD   = (abs(TP_1 - SMA) + ... + abs(TP_N - SMA)) / N
CCI  = (TP_N - SMA) / (0.015 * MD)
```

- `N` is the number of candles in the average, 72 on the way in and 66 on the way out.
- `TP_N` is the typical price of the latest candle; `TP_1` is the oldest of the window.
- `SMA` is the simple moving average of the typical prices.
- `MD` is the mean deviation: the average distance of the typical prices from `SMA`, written
  without regard to sign.
- `0.015` is a constant chosen by the indicator's author so that a value near plus or minus one
  hundred is a normal reading. A value of minus 175 is a large distance below the average, measured
  in units of typical wobble.

The relative strength index compares the average size of upward moves with the average size of
downward moves:

```text
average_gain = (sum of the up moves over the last N candles) / N
average_loss = (sum of the down moves over the last N candles) / N
RS           = average_gain / average_loss
RSI          = 100 - 100 / (1 + RS)
```

- `N` is the number of candles, 36 on the way in and 45 on the way out.
- An up move is a candle that closed above the previous close; a down move is a candle that closed
  below it.
- `RS` is the ratio of up to down, and `RSI` turns that ratio into a number between 0 and 100. An
  index of 100 means every recent move was upward; an index of 0 means every move was downward. The
  library computes a smoothed version of the same idea rather than a plain average; the comparison
  the rule makes is unchanged.

The profit ladder is a step function of the elapsed holding time `T` in minutes:

```text
required(T) = 0.27058 if T < 33
              0.0853  if 33 <= T < 64
              0.04093 if 64 <= T < 244
              0.0     if T >= 244
```

- `T` is minutes since the position was opened.
- `required(T)` is the smallest gain, as a fraction of the opening price, that closes the position.

The cost of a completed round trip is `2 * c`, where `c` is the charge per side as a fraction of the
amount traded. A realistic figure on a large crypto exchange is 0.0005 to 0.001, that is 0.05 to
0.10 percent per side, which is the level the cost primer in this collection uses.

## A worked example

Ten invented fifteen-minute candles for one pair, falling from 104 to 91. Each candle has a high one
unit above its close and a low one unit below, so its typical price equals its close, which keeps
the arithmetic short.

| Candle | High   | Low    | Close  | Typical price |
| ------ | ------ | ------ | ------ | ------------- |
| 1      | 105.00 | 103.00 | 104.00 | 104.00        |
| 2      | 104.00 | 102.00 | 103.00 | 103.00        |
| 3      | 103.00 | 101.00 | 102.00 | 102.00        |
| 4      | 102.00 | 100.00 | 101.00 | 101.00        |
| 5      | 101.00 | 99.00  | 100.00 | 100.00        |
| 6      | 100.00 | 98.00  | 99.00  | 99.00         |
| 7      | 99.00  | 97.00  | 98.00  | 98.00         |
| 8      | 98.00  | 96.00  | 97.00  | 97.00         |
| 9      | 97.00  | 95.00  | 96.00  | 96.00         |
| 10     | 92.00  | 90.00  | 91.00  | 91.00         |

At candle 10, using the last ten typical prices, the average is 99.10 and the mean deviation, the
average distance of the ten typical prices from 99.10, is 2.90. So:

```text
CCI = (91.00 - 99.10) / (0.015 * 2.90) = -8.10 / 0.0435 = -186.2
```

That is below minus 175, so the first condition holds. Over the ten candles the close fell at every
step, so there were no up moves at all: `average_gain` is 0, `RS` is 0, and the relative strength
index is 0. That is below 90, so the second condition holds and the rule buys. The rule fires on
candle 10. In the real file the windows are 72 and 36 candles rather than ten; ten is used here so
the arithmetic fits on the page, and it changes the size of the numbers, not the method.

The position is opened at the next candle's open, candle 11, at 91.00. The price recovers and stands
at 94.80 once the position has been held for 64 minutes. The rung of the ladder at that point is
4.093 percent, and the gain is 4.176 percent, above the rung, so the position is sold.

```text
Gross gain      = 94.80 / 91.00 - 1 = 0.04176, that is 4.176 percent
Round-trip cost = 2 * 0.001 = 0.002, that is 0.20 percent
Net gain        = 4.176 - 0.20 = 3.976 percent
```

The 34.338 percent loss limit was never approached, and the written exit never fired, because the
relative strength index would have to climb above 88 for it to trigger. The example shows the
arithmetic, not that the rule earns anything; the two costs that matter are the exchange charge on
each side and the gap between the buying and selling price, and on a fast fifteen-minute timetable
both recur far more often than in the daily example above.

## What the research actually found

The two indicators are old and widely used, and the rule built from them here was never tested.

| Source                          | What it measured                                                        | Result                                                                                                                                         |
| ------------------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Lambert (1980)                  | The commodity channel index, introduced in a magazine article           | The method and its constant; no claim of a tested profit, and no sample                                                                        |
| Wilder (1978)                   | The relative strength index, introduced in a book on trading systems    | The method and its averaging rule; again a practitioner's introduction rather than a study                                                     |
| Park and Irwin (2007)           | A survey of the profitability of technical trading rules across markets | Some rules showed promise in currency and futures markets before the 1990s, mostly before costs, and the picture became much weaker afterwards |
| The parameters inside this file | A block the author labels as the result of a search                     | Four lookback lengths and four levels, chosen together; the file records no pairs, no period and no result of any test                         |

Read together, the picture is this. Mean reversion over very short horizons is a real and studied
effect, but it is small, it competes with the cost of trading often, and the specific levels here
were selected by a search rather than derived from a measurement. The file gives no performance
figure, no pair list and no period, so there is nothing here to replicate. The honest grade is weak.

## How this project relates to it

The idea that a fast fall is driven by traders who have to sell is studied in this repository's
[market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md)
brief, which covers how an urgent seller pays a higher price for speed, and in the
[order flow and toxicity](../../../strategies/books/07_order_flow_and_toxicity.md) brief, which
covers who is on the other side and why informed and forced flow differ. The cost side, which is
what decides whether a fast mean-reversion rule survives, is set out in the collection's primer
[costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md).

The risk of tuning four lookbacks and four levels at once is the subject of
[overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).

## Where it goes wrong

- A price that has fallen a lot is a fact about the past; that it will rise is a prediction, and
  this rule offers no measurement of how often the prediction came true.
- Four conditions on the same candles fit almost any past. Here the four are the two long lookbacks
  and the two thresholds, and their neighbours would fit a chart just as well; the point is that a
  rule with several free choices can be made to look right on the sample it was chosen from.
- Oversold can stay oversold. In a genuine collapse the price keeps falling below the level and the
  rule keeps buying, which is why the loss limit is so important and why the 34.338 percent stop is
  a confession of how far the trades were allowed to go wrong.
- The relative strength index gates are inert. Requiring the index to be below 90 on entry and above
  88 on exit removes almost nothing from the rule, so the strategy is effectively one indicator
  wearing two names.
- Cost on a fast timetable. Fifteen-minute candles mean many trades; each round trip pays the
  exchange charge on both sides and the gap between the buying and selling price, and in a thin
  pair that gap is wider and eats more of a small gain.
- The parameters came from a search, not a theory. The file's own labels call them search results,
  which means they were chosen because they fitted the data the author had, and there is no record
  of what they did on data the author did not see.

## Try it yourself

You need a spreadsheet and a public source of fifteen-minute prices for one coin pair, or you can
use daily prices if fifteen-minute prices are hard to find.

1. Make columns: candle, high, low, close, typical price.
2. Fill the typical price with the average of high, low and close.
3. Add a column with the average of the last five typical prices.
4. Add a column with the average distance of those five typical prices from that average.
5. Add a column with (typical price minus the average) divided by (0.015 times the average
   distance). This is a five-candle version of the commodity channel index.
6. Add a column for the close-to-close change, a column for up moves and a column for down moves.
7. Add a column with the five-candle relative strength index: 100 minus 100 divided by (1 plus the
   average up move divided by the average down move).

What to notice: the index swings far below minus 175 whenever the price falls hard, and often keeps
falling for several candles afterwards. The rule buys during that stretch, so a single purchase is
not automatically right; the collection's cost primer shows why, over many trades, the fees have to
be paid whether or not the bounce arrives.

## Where this came from

- [SwingHighToSky.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/SwingHighToSky.py),
  the file that states the rules: the fifteen-minute timeframe, the two indicator lookbacks, the
  four levels, the profit ladder, the loss limit and the written exit.
- Donald Lambert, [Commodity Channel Index: Tool for Trading Cyclic Trends](https://store.traders.com/-v01-c05-comm-pdf.html)
  (1980), the article that introduced the first indicator.
- J. Welles Wilder, [New Concepts in Technical Trading Systems](https://archive.org/details/newconceptsintec00wild)
  (1978), the book that introduced the second.
- Park and Irwin,
  [What do we know about the profitability of technical analysis?](https://onlinelibrary.wiley.com/doi/10.1111/j.1467-6419.2007.00519.x)
  (2007), the survey of how such rules have held up across markets.
- [Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  [costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md) and
  [overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's studies of the three forces this rule depends on.

## Words used in this tutorial

- breakout: buying because a price rose above a level it had not exceeded for a while.
- commodity channel index: how far the typical price sits from its own average, measured in units of
  typical wobble, on a scale where about minus 100 to plus 100 is normal.
- mean reversion: the idea that a price which has moved far from its usual level tends to come back.
- relative strength index: the share of recent movement that was upward, on a scale from 0 to 100.
- swing high: a local peak where the price climbed into a candle and fell away again.
- typical price: the average of one candle's high, low and close.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
