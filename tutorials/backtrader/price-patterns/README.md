# Price patterns: candle shapes, narrow ranges, fractals and boxes

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Gold, quoted in American dollars per ounce, on a fifteen-minute timetable for the candle shapes and a daily timetable for the range and box rules                                                                                                                                                     |
| How often it trades       | Rarely. The plain candle shapes fire a handful of times in three months; the narrow-range rule fires a few times a month over many years                                                                                                                                                              |
| What you need             | A spreadsheet and a table of open, high, low and close prices, one row per period                                                                                                                                                                                                                     |
| Where the rules come from | [Strategy compendium, category 04, price patterns](https://backtrader.readthedocs.io/en/latest/strategies-series/en/04-price-patterns.html)                                                                                                                                                           |
| The underlying research   | Steve Nison, [Japanese Candlestick Charting Techniques](https://archive.org/details/japanesecandlest0000niso) (1991), Toby Crabel's range-contraction work, and the negative evaluation in [Marshall, Young and Rose (2006)](https://www.sciencedirect.com/science/article/abs/pii/S0378426605002116) |
| How well it held up       | Weak: the one independent evaluation of candle shapes finds no value after costs, and the compendium's own runs are mostly losing or flat, with the win rate rising only when an oscillator is added as a second vote                                                                                 |
| Also appears in           | [Candlestick patterns](../../freqtrade/candlestick-patterns/README.md) in this collection, which covers the same family of shapes on cryptocurrency pairs                                                                                                                                             |

## The idea in one paragraph

A period of trading can be drawn as a small picture called a candle: the price where the period
opened, the highest price, the lowest price and the price where it closed. Certain arrangements of
one, two or three candles in a row have names, such as the engulfing pattern, the hammer and the
morning star. This category tests whether those named shapes predict the next move. Alongside the
candles it tests three structure rules: the narrowest daily range of the last seven days, a box
that a price has been oscillating inside, and bricks that only appear once the price has moved a
fixed distance. The shared bet is that a visible change in the fight between buyers and sellers,
or a squeeze in the range, marks the start of the next move.

## Why anyone believed it

A candle is a summary of an argument. On a day when the price opened low, fell further and then
closed near its high, buyers won after being beaten for most of the day, and the candle records it.
A day when the price opened and closed in the middle of a wide range records nobody winning. If a
crowd is watching the same shapes, the story goes, then a hammer after a fall tells the crowd the
fall is finished, and the crowd buys. The counterparty is a seller who is being forced out and
cannot wait for a better price: a stop that was hit, a borrowed position being unwound, or a
panic after a long slide. The buyer with cash takes the other side and is paid for waiting. The
same story is told for a squeezed range, on the idea that a quiet market is a coiled spring.

## An everyday comparison

Think of a tug of war judged once an hour. At the end of each hour a referee writes down four
things: where the rope started, how far each side dragged it at the most, and where the rope ended
when the whistle blew. Most hours the rope simply drifts one way. Some hours one side drags it
almost to the winning line and then the other side hauls it all the way back, so the start and the
end are close together but the middle of the hour was wild. A run of three such records is
supposed to tell you which team is about to win. The candle rules read those records; the
narrow-range rule instead waits for the quietest hour of the day and bets on the noise returning.

## The rules, step by step

This category holds forty-four strategies. The ones a reader would meet are listed below, with one
clause on each.

| Strategy family             | Count of files | What it does                                                                  |
| --------------------------- | -------------- | ----------------------------------------------------------------------------- |
| Engulfing                   | 2              | One candle covering the one before it, plain and with an oscillator gate      |
| Hammer, star and doji       | 3              | One- and three-candle reversals, gated by position or a momentum reading      |
| Three inside and line break | 2              | Shorter reversal shapes, and a chart that only prints new extremes            |
| Heikin Ashi                 | 2              | Smoothed candles whose colour flip is the only signal                         |
| Narrow range seven (NR7)    | 3              | Trades the break of the narrowest of the last seven daily ranges              |
| Fractals                    | 2              | A peak or trough with lower or higher neighbours, computed on closing prices  |
| Darvas boxes                | 1              | A box around a recent high and low; trades the transition of the box colour   |
| Renko bricks                | 2              | Bars that only print after a fixed or volatility-scaled move, then trade them |
| Doji and other candles      | 27             | The remaining ports, mostly variants of the same shapes with different gates  |

Taken together, the shared mechanism is this. Define a candle from the four prices of a period.
Define each named shape as an exact set of comparisons on the last one, two or three candles.
When the shape appears, enter in the direction the shape is said to point. Exit either on the
opposite shape, on a fixed profit target and loss limit, or after a fixed number of periods.

1. Collect, for each period, the open, the highest price, the lowest price and the close. Draw the
   body between the open and the close; it is green when the close is above the open and red when
   it is below. The thin lines from each end of the body to the high and the low are the shadows.
2. A bullish engulfing pattern is two candles: the first is red, the second is green, and the
   second candle's body and both shadows reach beyond the first candle's body and both shadows.
   The bearish version is the mirror image.
3. A hammer is one candle with a small body near its top and a lower shadow at least twice the
   body. A hanging man is the same shape after a rise. A doji is a candle whose open and close are
   almost equal.
4. A morning star is three candles: a long red one, a small-bodied one that dips lower, then a
   long green one that closes back above the middle of the first candle's body. The evening star
   is the mirror image.
5. The narrow-range seven rule finds the day whose high minus low is smaller than the range of
   each of the previous six days. On the next day, buy if the close is above that day's high, or
   sell if the close is below its low. The loss limit is the entry price minus two and a half
   times the average daily range, the profit target is the entry price plus four times that
   average, and the position is closed after five days whatever happens.
6. A fractal peak is a period whose high is above the highs of the periods on both sides of it;
   a trough is the mirror image. The compendium computes these on closing prices, which uses fewer
   shadow extremes, and refuses to trade when the price is not far enough beyond the last fractal.
7. A Darvas box is a high and a low taken from the recent past. The indicator paints the box green
   or red, and the rule trades only the transition from one colour to the other, never the colour
   itself. The exits are a fixed loss limit and a fixed profit target measured from the entry.
8. A Renko brick is drawn only after the price has moved a fixed distance, or a distance scaled to
   the market's recent volatility. Noise smaller than one brick is simply invisible. The rules
   trade brick-colour changes and a trendline drawn through the bricks.
9. Review open positions on the same timetable that produced the signal, at most once per period.
   Every strategy in this category uses fixed trade sizes, so the same signal is the same bet each
   time.

## The maths, with every symbol named

A candle is four numbers, and every shape is written from them.

```text
body         = abs(close - open)
upper_shadow = high - max(open, close)
lower_shadow = min(open, close) - low
range        = high - low
```

- `open`, `high`, `low` and `close` are the four recorded prices for the period.
- `abs` is the size of a number ignoring its sign, so a body of 2.00 is the same whether the
  period rose or fell.
- `max(open, close)` is the larger of the two and `min(open, close)` is the smaller.

The bullish engulfing pattern, written on two consecutive candles numbered 1 (earlier) and 0
(now), is this:

```text
close_0 > open_0        and    open_1 > close_1
high_0  > high_1 + d    and    close_0 > open_1 + d
open_0  < close_1 - d   and    low_0  < low_1 - d
```

- `d` is a small margin in price units, called `distance` in the code; it is set to zero here, so
  the second candle must reach beyond the first by any amount at all.
- The first line says the new candle is green and the old one red; the next four say the new
  candle's high, low, open and close each reach past the old candle's matching level.

The narrow-range rule needs the range and then an average of the range.

```text
range_t  = high_t - low_t
NR7_t    = 1 if range_t < min(range_(t-6) ... range_(t-1)) else 0
```

- `range_t` is today's daily range.
- `min(range_(t-6) ... range_(t-1))` is the smallest range among the previous six days, so
  together the seven days give the name.
- `NR7_t` is one when today is the narrowest of those seven days and zero otherwise.

The exits use the average true range, which is the average of the largest of three daily moves.

```text
TR_t  = max( high_t - low_t, abs(high_t - close_(t-1)), abs(low_t - close_(t-1)) )
ATR_t = average of the last 14 values of TR
stop  = entry - 2.5 * ATR        for a long position
target = entry + 4.0 * ATR       for a long position
```

- `TR_t` is the true range of the day: the day's own range, or the jump from yesterday's close to
  today's high, or the jump from yesterday's close to today's low, whichever is largest.
- `ATR_t` is the average of the last fourteen true ranges, a plain measure of how far the price
  typically travels in a day.
- The stop and target are the loss limit and the profit goal, both scaled to that typical travel,
  so a wild market gets a wider pair and a quiet market a narrower one.

## A worked example

Nine invented daily candles for gold, priced around two thousand dollars an ounce. The body and
shadows are shown so the engulfing test can be checked.

| Day | Open    | High    | Low     | Close   | Body  | Upper shadow | Lower shadow | Colour |
| --- | ------- | ------- | ------- | ------- | ----- | ------------ | ------------ | ------ |
| 1   | 2000.00 | 2008.00 | 1988.00 | 1990.00 | 10.00 | 8.00         | 2.00         | red    |
| 2   | 1990.00 | 1996.00 | 1976.00 | 1978.00 | 12.00 | 6.00         | 2.00         | red    |
| 3   | 1978.00 | 1984.00 | 1964.00 | 1966.00 | 12.00 | 6.00         | 2.00         | red    |
| 4   | 1964.00 | 1992.00 | 1958.00 | 1988.00 | 24.00 | 4.00         | 6.00         | green  |
| 5   | 1988.00 | 2000.00 | 1980.00 | 1996.00 | 8.00  | 4.00         | 8.00         | green  |
| 6   | 1996.00 | 2004.00 | 1988.00 | 2000.00 | 4.00  | 4.00         | 8.00         | green  |
| 7   | 2000.00 | 2010.00 | 1994.00 | 2004.00 | 4.00  | 6.00         | 6.00         | green  |
| 8   | 2006.00 | 2012.00 | 1982.00 | 1986.00 | 20.00 | 6.00         | 4.00         | red    |
| 9   | 1986.00 | 2004.00 | 1984.00 | 1998.00 | 12.00 | 6.00         | 2.00         | green  |

Day 4 is a bullish engulfing of day 3. Check each comparison, with the margin `d` at zero: day 4
open 1964.00 is below day 3 close 1966.00; day 4 close 1988.00 is above day 3 open 1978.00; day 4
high 1992.00 is above day 3 high 1984.00; day 4 low 1958.00 is below day 3 low 1964.00. All four
hold, so the pattern is present. The order is filled at the next day's open, day 5 at 1988.00.

Day 8 is a bearish engulfing of day 7: day 8 open 2006.00 is above day 7 close 2004.00; day 8
close 1986.00 is below day 7 open 2000.00; day 8 high 2012.00 is above day 7 high 2010.00; day 8
low 1982.00 is below day 7 low 1994.00. So the opposite signal appears and the long position is
closed at the next day's open, day 9 at 1998.00.

```text
Gross gain   = 1998.00 / 1988.00 - 1 = 0.005030, that is 0.503 percent
Round-trip cost = 2 * 0.0005 = 0.001, that is 0.10 percent
Net gain     = 0.503 - 0.10 = 0.40 percent
```

Now the narrow-range rule. Suppose the daily ranges of days 1 to 6 are 20.00, 18.00, 25.00,
12.00, 22.00 and 19.00, and day 7's range is 11.00. The smallest of the previous six is 12.00, so
day 7 is the narrowest of the seven and is marked. On day 8 the close is 1986.00, which is not
above day 7's high of 2010.00, so no breakout. Suppose instead day 8 closes at 2015.00, above the
2010.00 high. The rule buys, and the entry is 2015.00. With an average true range of 18.00:

```text
stop   = 2015.00 - 2.5 * 18.00 = 2015.00 - 45.00 = 1970.00
target = 2015.00 + 4.0 * 18.00 = 2015.00 + 72.00 = 2087.00
```

Suppose the price reaches 2090.00 on the third day held, above the target, and the position is
sold at 2087.00.

```text
Gross gain   = 2087.00 / 2015.00 - 1 = 0.035732, that is 3.573 percent
Round-trip cost = 2 * 0.0002 = 0.0004, that is 0.04 percent
Net gain     = 3.573 - 0.04 = 3.53 percent
```

Notice what the two examples have in common: the arithmetic is simple and the result depends
entirely on whether the price moved after the shape. The example proves nothing about the shape;
it only shows how to apply the rules and how the costs enter.

## What the research actually found

The published record for candle shapes is old and mostly negative, and the compendium's own tests
are the only data this category rests on.

| Source                                                         | What it measured                                                             | Result                                                                                                                                                               |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Marshall, Young and Rose (2006)                                | Forty-three candlestick rules on the Dow Jones Industrial Average, 1992-2002 | Every rule, with and without a filter, failed to beat holding the index; after costs the profits were indistinguishable from zero                                    |
| The compendium, plain engulfing                                | Three months of fifteen-minute gold                                          | Exactly one trade and zero wins, final value 990,348.20 from one million, a Sharpe ratio of -8.34                                                                    |
| The compendium, engulfing plus RSI(11)                         | The same data                                                                | Twenty-seven trades and ten wins, a 37.04 percent win rate, final value 996,678.80; still not profitable                                                             |
| The compendium, hammer below its average plus RSI(14)          | The same data                                                                | A win rate of 52.38 percent, the best of the ladder, still without a stated profit                                                                                   |
| The compendium, narrow-range seven over eighteen years of gold | Daily gold, 2008 to 2025                                                     | One hundred thirty-two trades, a 48.48 percent win rate, final value 1,310,862.61, a gain of 31.09 percent, a Sharpe ratio of 0.46 and a worst fall of 49.46 percent |
| The compendium, Darvas boxes                                   | Three months of fifteen-minute gold with a four-hour signal                  | Eleven trades, every one of them a short, three wins and eight losses, final value 999,221.40                                                                        |

Read together, the picture is this. The famous shapes implemented to textbook standard were noise
at these frequencies. The only ladder that improved anything was pattern plus oscillator plus
location, and the improvement was in the win rate, not in the profit. The one rule that finished
well, the narrow-range breakout over eighteen years, did so with a worst fall of nearly half the
account, on one commodity and one period.

This is the honest heart of this group, and it applies to every file here. Every backtest in the
compendium asserts its final value, its reward-to-risk ratio and its worst fall against a baseline.
Passing that assertion proves the engine computes exactly what the file says it computes. It does
not prove the strategy earns anything. A test that asserts a loss has passed just as truly as one
that asserts a gain.

## How this project relates to it

This repository already has a tutorial on the same family of shapes, on cryptocurrency pairs:
[Candlestick patterns](../../freqtrade/candlestick-patterns/README.md). It states the same four
candle components, defines the high-wave candle, the engulfing pattern, the hammer and the morning
star, and reaches the same conclusion from the same negative studies. That page explains the
candle language; this one adds the range, fractal, box and brick rules.

The failures collected here are the subject of
[overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).
Its second section sets out the price of a wide search: on 207 published predictors, 183 had a
t-statistic above 2.0 and 74 above 4.0, and the statistical hurdle rises with the number of rules
tried. Choosing one candle shape out of dozens, with a chosen margin and a chosen oscillator
threshold, is the same act.

## Where it goes wrong

- A shape seen in past data is not a shape that predicts anything. Anyone can find a hammer in a
  chart after the fact; the question is whether the next period is more likely to rise after a
  hammer than after any other period, and that is what the negative studies test.
- The rules were chosen after looking. The compendium's own ladder shows a plain shape with one
  trade, then the same shape with an added body-size test and an oscillator, then location tests.
  Each addition was selected because it improved the win rate on the same data, so part of the
  improvement is the fitting.
- The sample is narrow and the spread is paid often. The candle tests use three months of gold on
  a fifteen-minute timetable, where every trade pays the gap between the buying and selling price.
- The narrow-range gain came with a near-half-account fall. A 31 percent gain over eighteen years
  with a 49 percent worst fall is not the same thing as a comfortable profit, and the reward for
  the risk is modest.

## Try it yourself

You need a spreadsheet and a public source of daily prices for one commodity, fund or share.

1. Make columns: date, open, high, low, close.
2. Add a body column: the close minus the open, ignoring the sign.
3. Add a lower-shadow column: the smaller of the open and the close, minus the low.
4. Add a range column: the high minus the low.
5. Add a hammer column that says yes when the lower shadow is at least twice the body and the
   upper shadow is no longer than the body, otherwise no.
6. Add an engulfing column that says yes when today is green, yesterday was red, today's close is
   above yesterday's open and today's open is below yesterday's close, otherwise no.
7. In the next rows, add the return over the following one, three and five days.
8. Average those returns over every row marked yes, and over every row marked no.

What to notice: the two averages are usually close, which is the honest finding of the literature.
The hammer count is high, because the shape is common, and a shape that appears on one day in five
carries almost no information. If the yes rows look strongly better on one instrument, try a
second instrument and a second period before believing it, because a single sample of a few dozen
shapes will show a flattering number by chance.

## Where this came from

- [Strategy compendium, category 04, price patterns](https://backtrader.readthedocs.io/en/latest/strategies-series/en/04-price-patterns.html),
  the rules and the backtest numbers quoted above, including the engulfing ladder, the narrow-range
  result and the Darvas baseline.
- Steve Nison, [Japanese Candlestick Charting Techniques](https://archive.org/details/japanesecandlest0000niso)
  (1991), the book that named and popularised the shapes.
- Marshall, Young and Rose,
  [Candlestick technical trading strategies: can they create value for investors?](https://www.sciencedirect.com/science/article/abs/pii/S0378426605002116)
  (2006), the evaluation of forty-three candlestick rules on the Dow Jones.
- Toby Crabel, *Day Trading with Short Term Price Patterns and Opening Range Breakout* (1990), the
  origin of the narrow-range contraction idea, as cited by the category article.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's study of what a search over many rules does to a reported result.
- [How a backtest lies](../../foundations/07_how-a-backtest-lies.md), the plain-language primer on
  the same traps.

## Words used in this tutorial

- body: the part of a candle between the open and the close.
- bullish: describing a candle or a market that rose, so the close is above the open.
- bearish: describing a candle or a market that fell, so the close is below the open.
- candle: one period of trading drawn from its open, high, low and close prices.
- engulfing pattern: a two-candle shape in which the second candle's body and shadows reach beyond
  the first.
- fractal: a peak whose high is above the highs either side of it, or a trough that is the mirror
  image.
- Renko brick: a bar that only prints after the price has moved a fixed distance.
- reward-to-risk: the average gain per trade divided by the average loss per trade.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
