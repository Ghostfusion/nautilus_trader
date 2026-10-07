# Candlestick patterns: buying a shape that appears in the last few candles

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                  |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Cryptocurrency pairs on a spot exchange (buying and holding the coin itself, not a contract on it)                                                                                                                                                     |
| How often it trades       | A few times a month: the chosen daily shape appears on roughly one day in ten                                                                                                                                                                          |
| What you need             | A spreadsheet and a table of daily open, high, low and close prices                                                                                                                                                                                    |
| Where the rules come from | [PatternRecognition.py in the freqtrade strategies repository](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/PatternRecognition.py)                                                                                 |
| The underlying research   | Steve Nison, [Japanese Candlestick Charting Techniques](https://archive.org/details/japanesecandlest0000niso) (1991), and the evaluation in [Marshall, Young and Rose (2006)](https://www.sciencedirect.com/science/article/abs/pii/S0378426605002116) |
| How well it held up       | Weak: a scan over every named shape, tuned once by its author, with no independent test, no stated cost line and an empty sell condition                                                                                                               |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                        |

## The idea in one paragraph

A day of trading can be drawn as a single picture called a candle: the price where the day opened,
the highest price, the lowest price and the price where the day closed. Certain arrangements of two
or three candles in a row have names, such as the hammer, the engulfing pattern and the morning
star. This file computes the value of every shape name it can find, every day, for a pair of coins.
It then buys when one chosen shape appears, and it sells only according to a profit target, a loss
limit and a trailing stop rather than on a shape. The default shape the author selected after a
search is a high-wave candle, which is a day of indecision with a tiny body and long shadows on both
sides. The bet is that the shape marks a moment when sellers have run out and buyers are about to
take over.

## Why anyone believed it

A candle is a summary of a fight. On a day when the price opened low, fell further and then closed
near its high, buyers won the argument after being beaten for most of the day, and the candle shows
it. A day when the price opened and closed in the middle of a wide range, with the price swinging
far up and far down and settling back, shows nobody winning. If a crowd is watching the same shapes,
the story goes, then a hammer after a fall tells the crowd that the fall is finished, and the crowd
buys. The counterparty is the seller who is being forced out of a losing position and cannot wait
for a better price: someone whose stop was hit, someone unwinding borrowed money, or someone
panicking after a long slide. The buyer with cash takes the other side and is paid for waiting.

## An everyday comparison

Picture a tug of war judged once an hour. At the end of each hour a referee writes down four things:
where the rope started, how far each side dragged it at the most, and where the rope ended when the
whistle blew. Most hours the rope simply drifts one way. Some hours one side drags it almost to the
winning line and then the other side hauls it all the way back, so the start and the end are close
together but the middle of the hour was wild. The set of three such hourly records, taken together,
is supposed to tell you which team is about to win. The strategy reads those records and buys the
side that looks exhausted.

## The rules, step by step

1. Collect daily prices for one cryptocurrency pair: for each day, the open, the highest price, the
   lowest price and the close. Everything here uses one day per candle.
2. For each day, work out the shape. The body is the part between the open and the close; it is
   green, or bullish, when the close is above the open and red, or bearish, when the close is below
   the open. The thin line drawn from each end of the body to the highest and lowest prices is
   called a shadow.
3. On each day, evaluate every named shape in the library the file uses. The library, called TA-Lib,
   contains about sixty of them, including engulfing, hammer, morning star, doji, shooting star and
   hundreds of variations in sign.
4. Buy when the chosen shape is present on that day. The file's chosen shape, taken from the
   author's own search, is the high-wave candle with a bearish body. In the library's notation a
   bearish appearance of a shape has the value minus one hundred, so the rule reads: today's
   high-wave value equals minus one hundred.
5. Hold the position. There is no shape-based sell rule in the file: the sell list is empty. The
   position closes when a profit target, a loss limit or a trailing stop says so, as set out in
   steps 6 to 8.
6. Profit target ladder. The target is measured from the price at which the position was opened and
   becomes more forgiving the longer the position is held: 93.6 percent at once, 33.2 percent after
   3.66 days, 8.6 percent after 12.60 days, and any profit at all after 33.44 days. The moment the
   current gain reaches the target for the elapsed time, the position is sold.
7. Loss limit. If the position is ever down 28.8 percent from the opening price, it is sold.
8. Trailing stop. Once the position is up 8.4 percent, a moving floor is set 3.2 percent below the
   highest gain reached. If the gain then falls back through that floor, the position is sold.
9. Only one position per pair is open at a time, and the strategy looks once a day, when the daily
   candle closes.

The shape whose name the library reports is only a label. The library decides for itself what counts
as a high-wave candle, using rules about how small the body is compared with the whole day's range
and how long the two shadows are. The strategy does not look at why the shape appeared, only at the
label the library returns.

## The maths, with every symbol named

A single candle is four numbers. Writing them in order:

```text
body        = abs(close - open)
upper_shadow = high - max(open, close)
lower_shadow = min(open, close) - low
range       = high - low
```

- `open`, `high`, `low` and `close` are the four recorded prices for the day.
- `abs` means the size of a number ignoring its sign, so a body of 2.0 units is the same whether
  the day rose or fell.
- `max(open, close)` is the higher of the two, and `min(open, close)` is the lower.

A day is named a high-wave candle when the body is small compared with the range and both shadows
are long, in words: the body is a small fraction of the day's range and each shadow is at least as
long as the body. A working definition a reader can apply is:

```text
body < 0.25 * range    and    upper_shadow > 2 * body    and    lower_shadow > 2 * body
```

- `range` is the distance between the highest and lowest price of the day.
- The three conditions must all hold. The first caps the body, the second and third demand two long
  shadows.

Two shapes in the family, written the same way, are the engulfing pattern and the hammer. The
bullish engulfing pattern is two days, in which the second day is bullish and its body covers the
whole body of the first, bearish day:

```text
close_2 > open_2           and    open_2 < close_1           and    close_2 > open_1
```

- `close_1` and `open_1` are the close and open of the first day; the rule says the second day's
  body reaches below the first day's crowd of prices and finishes above them.

The hammer is one day with a small body near the top and a long lower shadow:

```text
lower_shadow >= 2 * body    and    upper_shadow <= body
```

- The lower shadow is where sellers dragged the price before buyers pulled it back, and the short
  upper shadow says buyers kept control at the end.

The morning star is three days: a long bearish first day, a small-bodied second day that dips
lower, and a long bullish third day that closes well back inside the first day's body:

```text
close_3 > (open_1 + close_1) / 2
```

- `(open_1 + close_1) / 2` is the middle of the first day's body, and the rule says the third day
  climbs back above that middle.

The profit ladder is a step function. For an elapsed holding time `T` in minutes, the required gain
is:

```text
required(T) = 0.936 if T < 5271
              0.332 if 5271 <= T < 18147
              0.086 if 18147 <= T < 48152
              0.0   if T >= 48152
```

- `T` is the number of minutes since the position was opened.
- `required(T)` is the smallest gain, as a fraction of the opening price, that triggers a sale.

The cost of a completed trade, added at the end, is:

```text
cost = 2 * c
```

- `c` is the charge for one side of the trade as a fraction of the amount traded. On a large crypto
  exchange a realistic figure is 0.0005 to 0.001, that is 0.05 to 0.10 percent per side, which is
  the level the cost primer in this collection uses (a commission of 0.05 percent per side and a
  spread of 0.1 percent).
- The factor 2 counts the buy and the sell, so a round trip costs 0.10 to 0.20 percent.

## A worked example

Ten invented daily candles for one pair, priced around one hundred units. The columns on the right
are the numbers from the formulas above.

| Day | Open   | High   | Low    | Close  | Body | Upper shadow | Lower shadow | Range | High-wave? |
| --- | ------ | ------ | ------ | ------ | ---- | ------------ | ------------ | ----- | ---------- |
| 1   | 100.00 | 102.00 | 99.00  | 101.50 | 1.50 | 0.50         | 1.00         | 3.00  | no         |
| 2   | 101.50 | 103.00 | 100.50 | 102.00 | 0.50 | 1.00         | 1.00         | 2.50  | no         |
| 3   | 102.00 | 104.00 | 101.00 | 103.50 | 1.50 | 0.50         | 1.00         | 3.00  | no         |
| 4   | 103.50 | 105.00 | 102.50 | 104.00 | 0.50 | 1.00         | 1.00         | 2.50  | no         |
| 5   | 104.00 | 105.50 | 101.50 | 101.60 | 2.40 | 1.50         | 0.10         | 4.00  | no         |
| 6   | 101.80 | 105.50 | 99.50  | 101.60 | 0.20 | 3.70         | 2.10         | 6.00  | yes        |
| 7   | 101.60 | 106.00 | 101.00 | 105.00 | 3.40 | 1.00         | 0.60         | 5.00  | no         |
| 8   | 105.00 | 108.00 | 104.00 | 107.50 | 2.50 | 0.50         | 1.00         | 4.00  | no         |
| 9   | 107.50 | 110.00 | 106.50 | 109.80 | 2.30 | 0.20         | 1.00         | 3.50  | no         |
| 10  | 109.80 | 112.00 | 109.00 | 111.00 | 1.20 | 1.00         | 0.80         | 3.00  | no         |

Day 6 is the shape. Its body is 0.20, its range is 6.00, so the body is 3.3 percent of the range,
well under the quarter threshold. Its upper shadow is 3.70 and its lower shadow is 2.10, both far
more than twice the body. Because the day closed below its open, the library reports the bearish
value, minus one hundred, which is exactly the value the file waits for. The rule fires on day 6.

The position is opened at the next day's open, day 7, at 101.60. It rises to 111.00 by day 10, a
gain of 9.25 percent, and then drifts between 110.50 and 112.00 for the next few days. The ladder's
third rung is 8.6 percent once 12.60 days have passed; on day 13, past that point, the gain of 9.25
percent is above the rung, so the position is sold at 111.00.

```text
Gross gain   = 111.00 / 101.60 - 1 = 0.0925, that is 9.25 percent
Round-trip cost = 2 * 0.001 = 0.002, that is 0.20 percent
Net gain     = 9.25 - 0.20 = 9.05 percent
```

Two things are worth noticing. First, the first rung of the ladder asks for a 93.6 percent gain
immediately, so almost every trade would be held until a lower rung applies; the ladder is not a
gentle profit-taking rule but a sign of how far the author's search pushed the targets. Second, the
trailing stop would have been armed once the gain passed 8.4 percent on day 10, setting a moving
floor 3.2 percent below the highest gain; in this example the price rose without a 3.2 percent dip,
so the trailing stop never fired. The example is not evidence that the shape predicts anything; it
only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

The published record for candlestick shapes is old and mostly negative, and this file adds no test
of its own.

| Source                                         | What it measured                                                             | Result                                                                                                                                                                                                            |
| ---------------------------------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Marshall, Young and Rose (2006)                | Forty-three candlestick rules on the Dow Jones Industrial Average, 1992-2002 | Every rule, with and without a filter, failed to outperform holding the index; the profits after transaction costs were indistinguishable from zero                                                               |
| A 2016 student study of the same question      | The predictive power of common patterns on a set of shares                   | Some patterns carried a little information in certain shares, but the effect was small, unstable across shares and largely eaten by costs                                                                         |
| The author's own note inside the strategy file | A search over one thousand parameter sets on the pairs and period chosen     | The best run reported 510 trades with 408 wins, 14 draws and 88 losses, an average profit of 2.35 percent per trade and a total profit of 542 percent, with an average holding time of 7 days 11 hours 54 minutes |

Read together, the picture is this. The shape of a candle is a fact about the past, and the library
will label it reliably. Whether that label carries any information about the next day is a separate
question, and the studies above say the answer is usually no, once costs and the number of rules
tried are counted. The author's 542 percent is a single un-audited run from a search over a thousand
settings. The file chooses which shape to trade from the library after that search, which is the
same as choosing the shape that happened to fit. That is the definition of a result that is likely
to vanish on new data.

## How this project relates to it

This repository has a study of exactly this failure:
[overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).
Section 2 of that brief collects the evidence on multiple testing and the price of a wide search,
which is what selecting one shape out of sixty after a thousand runs is.

The same brief and [market data quality](../../../strategies/books2/27_market_data_quality.md)
explain why a shape scan over a chosen pair list can look better than it is: pairs that no longer
exist, or that were added after they had already risen, flatter the result. The collection's own
primer [how a backtest lies](../../foundations/07_how-a-backtest-lies.md) walks through the traps.

## Where it goes wrong

- A shape that has been seen in past data is not a shape that predicts anything. Anyone can find
  hammer candles in a chart after the fact; the question is whether the next day's price rises more
  often after a hammer than after any other day, and that is what the negative studies test.
- Four conditions on the same candles will almost always fit the past. The high-wave test above uses
  three comparisons on one day, and a full pattern such as the morning star uses conditions on the
  open, high, low and close of three days at once. With that many ways to describe a shape, some
  shape will appear to predict a rise in any sample by luck, especially when the shape is chosen
  after looking.
- Choosing the shape after the search. The file does not fix the high-wave candle in advance; the
  author's search selected it, and a rule chosen because it fitted has already used the answer.
- The empty sell condition. There is no shape-based exit, so all selling comes from a profit ladder
  that was itself tuned. The 93.6 percent first rung is not a considered target; it is what the
  search left behind.
- Costs and thin pairs. On a daily timetable the cost per trade is small, but the coin pairs in the
  comment list are not all liquid, and a wider gap between buying and selling price quietly removes
  part of any gain.
- The sample is not fixed. The author's note gives one pair list, one period and one search count;
  none of it was held back to test the rule afterwards.

## Try it yourself

You need a spreadsheet and a public source of daily prices for one coin pair.

1. Make columns: date, open, high, low, close.
2. Add a body column: the close minus the open, ignoring the sign.
3. Add an upper-shadow column: the high minus the larger of the open and the close.
4. Add a lower-shadow column: the smaller of the open and the close minus the low.
5. Add a range column: the high minus the low.
6. Add a high-wave column that says yes when the body is under a quarter of the range and both
   shadows are more than twice the body, otherwise no.
7. In the next rows, add the return over the following one, three and five days.
8. Average those returns over every row marked yes, and average them over every row marked no.

What to notice: the two averages are usually close, which is the honest finding of the literature.
If the yes rows look dramatically better on one pair, try a second pair and a second period before
believing it, because a single sample of a few dozen shapes will show a flattering number by chance.

## Where this came from

- [PatternRecognition.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/PatternRecognition.py),
  the file that states the rules: the daily timeframe, the scan over every named shape, the chosen
  high-wave shape, the profit ladder, the loss limit and the trailing stop.
- Steve Nison, [Japanese Candlestick Charting Techniques](https://archive.org/details/japanesecandlest0000niso)
  (1991), the book that named and popularised the shapes described above.
- Marshall, Young and Rose,
  [Candlestick technical trading strategies: can they create value for investors?](https://www.sciencedirect.com/science/article/abs/pii/S0378426605002116)
  (2006), the evaluation of forty-three candlestick rules on the Dow Jones.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's study of what a search over many rules does to a reported result.

## Words used in this tutorial

- body: the part of a candle between the open and the close.
- bullish: describing a candle or a market that rose, so the close is above the open.
- bearish: describing a candle or a market that fell, so the close is below the open.
- candle: one period of trading drawn from its open, high, low and close prices.
- engulfing pattern: a two-candle shape in which the second candle's body covers the first.
- hammer: one candle with a small body and a long lower shadow, said to mark the end of a fall.
- morning star: a three-candle shape that ends with a strong rise, said to mark a bottom.
- shadow: the thin line from the body of a candle to its highest or lowest price.
- trailing stop: a sell order that follows the price up and sells if it slips back by a set amount.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
