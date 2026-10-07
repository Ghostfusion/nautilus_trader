# Oscillators that disagree: the commodity channel index and several strength bands

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Crypto pairs on a crypto exchange, such as Bitcoin priced in United States dollars                                                                                                                                                                                                                                                                                                                     |
| How often it trades       | Several times a day; these files watch one-minute and five-minute candles and buy after a price has been pushed sharply one way                                                                                                                                                                                                                                                                        |
| What you need             | A spreadsheet, plus the short calculation below if you want to follow the commodity channel index by hand                                                                                                                                                                                                                                                                                              |
| Where the rules come from | [CofiBitStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/CofiBitStrategy.py), [CCIStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/CCIStrategy.py) and [MultiRSI.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/MultiRSI.py) |
| The underlying research   | The commodity channel index, published by Donald Lambert in 1980 ([plain-language description](https://en.wikipedia.org/wiki/Commodity_channel_index)); the RSI and the stochastic oscillator are older published indicators with no study behind these files                                                                                                                                          |
| How well it held up       | Weak: the only published test attached to the commodity channel index is a single vendor study of 43,297 stock trades that was never independently repeated and does not cover crypto; the other two files have no measurement at all                                                                                                                                                                  |
| Also appears in           | [The printing press that loses to costs](../printing-press-scalping/README.md), whose scalp file combines the same indicators, and [the smallest strategies](../minimal-examples/README.md), whose Simple.py uses the RSI alone                                                                                                                                                                        |

## The idea in one paragraph

An oscillator is a measured number that swings back and forth between fixed bounds, or around a fixed
middle, and is meant to say when a price has moved too far one way. High readings are called
overbought, meaning buyers have pushed the price hard; low readings are called oversold. The files on
this page watch several oscillators at once. `CofiBitStrategy.py` buys a dip that the fast stochastic
oscillator has turned up from, `CCIStrategy.py` buys when two commodity channel indexes are both
deeply oversold at once, and `MultiRSI.py` buys when a fast strength reading sits far below a slow one.
The interest of the page is what happens when measures built from the same prices disagree.

## Why anyone believed it

A sharp price move in a crypto market often comes from a few impatient orders rather than from any
change in the coin's prospects. When a large seller must get out quickly, the price is pushed below
where most holders value it, and it tends to spring back once the selling stops. An oscillator that
reads the price as oversold is trying to catch that spring.

The counterparty is the impatient or frightened seller, or a program with a deadline that must sell
regardless of price. The reason to combine several oscillators is the thought that if price, volume
and momentum all say the price is stretched, the stretch is more likely to be real.

## An everyday comparison

Think of several people standing at different distances from a bonfire, each asked whether the fire
is "too hot". The closest feels a scorching heat and says yes; someone a few steps back feels warmth
and says it is pleasant; someone far away, feeling the evening chill, says it is too cold. They all
measure the same fire, but at different distances and over different amounts of time. The files on
this page are those observers: one watches the last five candles, another the last 34, another the
last 170, and a single fire can make all three disagree.

## The rules, step by step

`CofiBitStrategy.py`, on five-minute candles:

1. Compute a five-candle exponential moving average of the high, low and close, the fast stochastic
   oscillator, whose two parts run from 0 to 100, and ADX, a number for trend strength.
2. Buy when all of these are true: the candle opened below the five-candle average of the lows, the
   faster stochastic part crossed above the slower one, both parts are below 25, and the ADX is above
   25.
3. Sell when the candle opened at or above the five-candle average of the highs, or when either
   stochastic part crosses above 75.
4. Also sell when the gain reaches a target that falls with time: 10 percent under 20 minutes old, 7
   percent from 20 to 29 minutes, 6 percent from 30 to 39 minutes, and 5 percent from 40 minutes on.
   The ladder is `{"40": 0.05, "30": 0.06, "20": 0.07, "0": 0.10}`.
5. Also sell when the trade is 25 percent below its entry price. All three thresholds, 25, 25 and 75,
   are parameters the file marks as chosen by a parameter search, with those values as its defaults.

`CCIStrategy.py`, on one-minute candles combined into five-minute candles:

1. Compute two commodity channel indexes, one over 170 candles and one over 34, an RSI over 14
   candles, a money flow index, and a measure of buying and selling pressure called the Chaikin money
   flow, plus a 25-candle, a 50-candle and a 200-candle simple average of the price on the combined
   candles.
2. Buy when all of these are true: the 170-candle commodity channel index is below minus 100, the
   34-candle one is also below minus 100, the Chaikin money flow is below minus 0.1, the money flow
   index is below 25, the 50-candle average is above the 25-candle average, and the price is above
   the 200-candle average. The last two are a filter that keeps the rule to dips inside a rising
   market.
3. Sell when all of these are true: both commodity channel indexes are above plus 100, the Chaikin
   money flow is above plus 0.3, and the averages are stacked downwards, the 100-candle below the
   50-candle below the 25-candle.
4. Also sell when the gain reaches 10 percent, and sell when the trade is 2 percent below its entry
   price.

`MultiRSI.py`, on five-minute candles:

1. Compute a five-candle and a 200-candle simple average of the price, and an RSI over 14 candles.
2. Also combine the candles into ten-minute and forty-minute candles and compute a 14-candle RSI on
   each of those.
3. Buy when the five-candle average is at or above the 200-candle average and the five-minute RSI is
   more than 20 points below the forty-minute RSI. The file's comment calls this bearish, but the code
   actually requires a rising market.
4. Sell when the five-minute RSI is above both the ten-minute and the forty-minute readings.
5. Also sell when the gain reaches 1 percent, and sell when the trade is 5 percent below its entry
   price.

## The maths, with every symbol named

The four oscillators below are built from the same candle prices, and each answers a different
question. The first is the one `CCIStrategy.py` is named after. The commodity channel index, published
by Donald Lambert in 1980, begins with a typical price:

```text
TP = (high + low + close) / 3
```

- `TP` is the typical price of one candle, an average of its highest, lowest and closing prices.
- `high`, `low` and `close` are that candle's prices.

Then it measures how far the typical price sits from its own average, in units of how much the typical
price usually wanders:

```text
SMA = the simple average of TP over the last n candles
MD  = the average of the distances between each TP and SMA over those n candles
CCI = (TP - SMA) / (0.015 * MD)
```

- `SMA` is the ordinary average of the typical price over the last `n` candles.
- `MD` is the mean absolute deviation: the average distance of those typical prices from `SMA`,
  ignoring the sign of each distance.
- `n` is the number of candles, 170 or 34 in the file, and 5 in the worked example below.
- `0.015` is a constant Lambert chose so that roughly 70 to 80 percent of readings fall between minus
  100 and plus 100.
- `CCI` is above zero when the typical price is above its average and below zero when it is below.

The RSI, published by Welles Wilder, compares the average size of up-moves with the average size of
down-moves over `n` candles:

```text
RS = (average gain over n candles) / (average loss over n candles)
RSI = 100 - 100 / (1 + RS)
```

- `RS` is the ratio of the average up-move to the average down-move.
- `RSI` is the result, from 0 to 100; above 70 is treated as overbought and below 30 as oversold.

The fast stochastic oscillator compares the closing price with the range of recent candles:

```text
%K = 100 * (close - lowest low over n) / (highest high over n - lowest low over n)
%D = the simple average of %K over m candles
```

- `%K` is the position of the closing price inside the recent range, from 0 at the low to 100 at the
  high.
- `%D` is a smoothed copy of `%K`, so the two parts cross and uncross as the price moves.

The money flow index is the RSI formula above applied to money rather than to price alone: each
candle's typical price times its volume is added to a positive or negative running total depending on
whether that typical price rose or fell, and the ratio of the two totals becomes a reading from 0 to
100. Because it uses volume, it can read oversold while the price-only oscillators read neutral.

Why several of these disagree is now visible from the formulas. They use different numbers of candles,
so a 5-candle oscillator reacts to a move a 170-candle one barely notices; they use different inputs,
because the stochastic and the CCI use price alone while the money flow index and the Chaikin money
flow also use volume; they have different scales, the stochastic and the RSI bounded between 0 and
100 while the CCI has no fixed bounds; and they measure different things, one the position inside the
range, another the size of up-moves against down-moves, another the distance from an average.

## A worked example

A commodity channel index computed by hand. The file uses 170 and 34 candles, but the demonstration
uses 5 so the arithmetic fits; the method is the same. The prices are invented.

| Candle | High   | Low    | Close  | Typical price |
| ------ | ------ | ------ | ------ | ------------- |
| 1      | 101.00 | 99.00  | 100.00 | 100.0000      |
| 2      | 102.00 | 100.00 | 101.00 | 101.0000      |
| 3      | 103.00 | 101.00 | 102.00 | 102.0000      |
| 4      | 102.00 | 100.00 | 101.00 | 101.0000      |
| 5      | 101.00 | 99.00  | 100.00 | 100.0000      |
| 6      | 100.00 | 98.00  | 99.00  | 99.0000       |
| 7      | 99.00  | 97.00  | 98.00  | 98.0000       |
| 8      | 100.00 | 98.00  | 99.50  | 99.1667       |

At candle 6, the last five typical prices are 101, 102, 101, 100 and 99, so their average `SMA` is
`(101 + 102 + 101 + 100 + 99) / 5 = 100.6000`. Their distances from the average are 0.4, 1.4, 0.4,
0.6 and 1.6, which sum to 4.4, so the mean absolute deviation `MD` is `4.4 / 5 = 0.8800`. The index is
then `(99 - 100.6) / (0.015 * 0.88) = -1.6 / 0.0132 = -121.21`. The same calculation on the last
five candles of each other row gives the series below.

| Candle | Window of typical prices  | SMA      | MD     | CCI     | Below minus 100? |
| ------ | ------------------------- | -------- | ------ | ------- | ---------------- |
| 5      | 100, 101, 102, 101, 100   | 100.8000 | 0.6400 | -83.33  | no               |
| 6      | 101, 102, 101, 100, 99    | 100.6000 | 0.8800 | -121.21 | yes              |
| 7      | 102, 101, 100, 99, 98     | 100.0000 | 1.2000 | -111.11 | yes              |
| 8      | 101, 100, 99, 98, 99.1667 | 99.4333  | 0.8533 | -20.83  | no               |

`CCIStrategy.py` buys when its commodities channel indexes are below minus 100. Here the reading
passes that level at candle 6 and stays there at candle 7, when the price has fallen from 100.00 to
98.00, and it returns above minus 100 at candle 8 even though the price is still below where it
started. That return to normal is the oversold reading expiring: the average has caught down to the
lower price, so the price is no longer far from it. Now the money, for a trade bought at candle 7 at
98.00 and sold at candle 8 at 99.50, on 1,000 units with a fee of 0.10 percent per side and a spread
of 0.10 percent:

```text
Buy      1,000 at 98.00 = 98,000.00, fee 0.10% = 98.00, total paid 98,098.00
Sell     1,000 at 99.50 = 99,500.00, fee 0.10% = 99.50, received 99,400.50
Gross    the price rose 1.53 percent, or 1,500.00 on the units held
Cost     spread 0.10% of about 98,750 = 98.75, plus 197.50 of fees = 296.25
Net      1,500.00 - 296.25 = 1,203.75, or 1.23 percent of the amount paid
```

Two things are worth noticing. First, the same candles give a different answer at another window
length: a 10-candle index would sit near zero at candle 6, because the lower prices are still only a
small part of its window. Second, the file's real windows are 170 and 34 candles, and a longer window
keeps the reading inside the minus 100 to plus 100 band far more of the time, so its entry condition
is much rarer than this five-candle demonstration suggests.

## What the research actually found

There is no independent measurement behind any of these three files. The one published test attached
to the commodity channel index is reported by the encyclopedia article above: a study of 43,297
backtested stock trades on the American S&P 500 index from 2003 to 2023 reported that a 50-candle
index crossing above minus 100 on daily charts returned 1,108 percent against 555 percent for simply
holding the index. It is one study on one market, it was never independently repeated, and it tested
daily stock-index charts, not the crypto five-minute candles here. The same article reports that the
index on five-minute charts "is not recommended due to poor performance", the setting this file uses.

For the RSI and the stochastic oscillator, the files here add conditions the original authors never
published together, and no study covers the combination, so there is nothing to report beyond the
arithmetic above. The repository's own README states that the files "mostly should serve as a
starting point for your own strategies, not as ready to use strategies", and that results "heavily
depend on the pairs, timeframe and timerange used to backtest". What would settle the question, one
comparison of every variant on the same prices and the same costs, does not exist here.

## How this project relates to it

This repository has no Freqtrade engine and no code that runs these oscillator rules, so nothing here
implements them. The nearest readings are the pages that explain the parts. [The printing press that
loses to costs](../printing-press-scalping/README.md) shows a scalp file that combines the same
stochastic, money flow and commodity channel readings and explains why the small targets such files
aim for can be smaller than the cost of a round trip. [The smallest strategies](../minimal-examples/README.md)
shows `Simple.py`, which uses the RSI on its own, so a reader can compare the single-oscillator
approach with the multi-oscillator one.

The general problem of a rule chosen because it looked good on one stretch of prices is the subject of
this repository's brief on overfitting and research integrity, at
[strategies/books2/28_overfitting_and_research_integrity.md](../../../strategies/books2/28_overfitting_and_research_integrity.md).
A second oscillator, or a third, is choosing between many versions of the same idea after seeing the
data, which is exactly the habit that brief warns against.

## Where it goes wrong

- The oscillators disagree by construction. A fast reading can be deeply oversold while a slow one is
  still neutral, and a second oscillator built from the same candles does not add independent
  evidence; it makes the rule look careful without making it stronger.
- Overbought and oversold are not forecasts. A price that reads oversold can keep falling, and one
  that reads overbought can keep rising; the numbers describe the past move, not the next one.
- The thresholds were searched for. The CofiBit file states that its 25, 25 and 75 were chosen by a
  parameter search, so they are fitted to the pairs and period the author tested.
- The stops and the targets are far apart. `CCIStrategy.py` risks 2 percent for a 10 percent target
  while `CofiBitStrategy.py` risks 25 percent for a 1 to 10 percent target, so the two cannot both be
  right about the same dip.
- The one published test is on the wrong market. The commodity channel study is on daily stock-index
  charts, and the source itself reports poor results for five-minute charts, the setting this file
  uses.

## Try it yourself

You need a spreadsheet and about thirty rows of prices for one crypto pair.

1. Build columns `high`, `low`, `close`, one row per candle.
2. Add a `typical price` column: high plus low plus close, divided by three.
3. Add a short index: for each row, average the last five typical prices, take the distance of the
   current typical price from that average, and divide it by `0.015` times the average distance of
   those five prices from their average.
4. Repeat step 3 using the last twenty typical prices, then mark each row where either index is below
   minus 100.
5. Count the rows where only one of the two marks is set.

What to notice: on most rows the short index moves a great deal and the long one hardly moves, so many
rows have one reading oversold and the other not. That gap is not a mistake in the sheet; it is why
the files ask several oscillators to agree before they buy.

## Where this came from

- The three source files, whose rules are given above:
  [CofiBitStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/CofiBitStrategy.py),
  [CCIStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/CCIStrategy.py)
  and [MultiRSI.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/MultiRSI.py).
- [Commodity channel index](https://en.wikipedia.org/wiki/Commodity_channel_index), the plain-language
  description of Lambert's 1980 indicator, its formula, and the 43,297-trade study.
- [The Freqtrade strategies README](https://github.com/freqtrade/freqtrade-strategies/blob/main/README.md),
  which states that the files are starting points and that results depend on the pairs, timeframe and
  period chosen. The price table, the five-candle arithmetic and the 0.10 percent spread and fee above
  are made up for teaching, and the arithmetic is the only claim being made.

## Words used in this tutorial

- commodity channel index: an oscillator measuring how far a candle's typical price sits from its own
  recent average, in units of how much that price usually wanders.
- divergence: a disagreement between an oscillator and the price it is measuring.
- mean absolute deviation: the average distance of a set of values from their own average, ignoring
  the sign of each distance.
- oscillator: a number calculated from prices that swings between fixed bounds, or around a fixed
  middle, used to judge when a move has gone too far.
- overbought: an oscillator reading at the high end of its range, meaning buyers have pushed the
  price hard.
- oversold: an oscillator reading at the low end of its range, meaning sellers have pushed the price
  hard.
- stochastic oscillator: an indicator whose two parts run from 0 to 100 and compare the closing price
  with the recent range.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
