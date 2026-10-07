# Supertrend: a line that follows the price at a distance set by how much it has been moving

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                  |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A cryptocurrency pair, such as Bitcoin priced in United States dollars, on a crypto exchange; the futures version also sells a pair it does not own, which is called selling short                                                                                                                                     |
| How often it trades       | On one-hour candles, often several times a week, in bursts when a new trend begins and then again when it ends                                                                                                                                                                                                         |
| What you need             | Nothing but this page and a pencil; the arithmetic is a short average and two subtractions                                                                                                                                                                                                                             |
| Where the rules come from | [Supertrend.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/Supertrend.py) and [futures/FSupertrendStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/FSupertrendStrategy.py)                                                  |
| The underlying research   | The indicator is described at [Investopedia](https://www.investopedia.com/supertrend-indicator-7976167), which credits Olivier Seban in 2009; the measured case for trend following is [Moskowitz, Ooi and Pedersen, Time Series Momentum (2012)](https://www.sciencedirect.com/science/article/pii/S0304405X11002613) |
| How well it held up       | Weak: the file itself says the implementation is not validated, no performance measurement is published with it, and the repository's own README calls the strategies starting points rather than strategies to trade                                                                                                  |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                                                                                        |

## The idea in one paragraph

A price moves around, and the size of those moves can be measured: on a calm day a pair moves a
little, on a wild day it moves a lot. This strategy draws a line above or below the price at a
distance equal to a fixed multiple of that recent movement, and moves the line only in the direction
that does not loosen the trend. While the price stays above the line the pair is said to be in an
uptrend; when the price drops through the line the line flips to above the price and the pair is said
to be in a downtrend. The strategy buys when several such lines agree the pair is in an uptrend. The
line is a moving stop as much as a signal: it follows the price up while the trend lasts.

## Why anyone believed it

A price does not move in a straight line. It rises, pauses, falls back a little, and rises again. A
rule that buys on every one of those pauses loses money to the gap between the buying and selling
price each time. A rule that waits for the price to be a whole typical move above its own recent
average is trying to sit through the pauses and only act on the part of the move that is real. The
line is exactly that: it is placed far enough below the price that ordinary noise does not cross it,
and close enough that a real reversal does.

The counterparty is the trader who sells into a rise because it looks too steep, or buys into a fall
because it looks cheap. If moves tend to continue, the person acting against them keeps being early
while the person following the line is late but on the right side. The idea rests entirely on whether
moves continue; when they do not, the line produces long strings of small losses.

## An everyday comparison

Think of a hiker following a ridge in fog. The rule is simple: keep walking while the ground
underfoot keeps rising, and turn back only when it has fallen a few steps below the highest point
reached. A few steps absorbs the dips and stones of a real ascent, and it also means the walker gives
back a few steps when the hill ends. Set the tolerance too small and every pebble ends the walk; set
it too large and the walker is far down the other side before noticing. This strategy's distance is
set the same way, from how rough the ground has recently been.

## The rules, step by step

1. Use one-hour candles: each row is one hour of trading, with the highest price reached, the lowest,
   and the last price in that hour.
2. Build six separate lines, each from the same recipe but with a different pair of settings. The
   three lines used to decide a purchase have a multiplier and a period of 4 and 8, 7 and 9, and 1
   and 8. The three lines used to decide a sale have 1 and 16, 3 and 18, and 6 and 18.
3. For each line, work out the average size of the last few hourly moves (the period above), and place
   the line that same number of average moves multiplied by the multiplier away from the middle of the
   hour's range.
4. A line is above the price while the price has been falling, and flips below when the price closes
   above it; it is below while the price has been rising, and flips above when the price closes below
   it. Call the position of the line its direction: down when above the price, up when below.
5. Buy when all three purchase lines are up and at least some volume was traded in the hour.
6. Sell an existing purchase when all three sale lines are down, in the plain spot version of the
   strategy.
7. In the futures version, close a purchase when the middle sale line (3 and 18) turns down, and also
   sell short when all three sale lines are down.
8. Hold each trade only until a profit target, a stop, or the trailing stop is reached. The exact
   settings in the two files are these.

| Setting       | Spot Supertrend.py                                                              | Futures FSupertrendStrategy.py                                                                                |
| ------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Stop loss     | 26.5 percent, trailing below the highest price seen                             | 26.5 percent, trailing below the highest price seen                                                           |
| Trailing stop | tightens to 5 percent once the trade is up 14.4 percent                         | tightens to 5 percent once the trade is up 10 percent                                                         |
| Profit ladder | 8.7% now, 5.8% after 372 minutes, 2.9% after 861 minutes, 0% after 2221 minutes | 10% now, then 75% after 30 minutes (an odd, non-descending step), 5% after 60 minutes, 2.5% after 120 minutes |

## The maths, with every symbol named

The line is built from one measure of movement, the average true range.

The true range of one candle is the largest of three distances:

```text
TR_t = max(High_t - Low_t, |High_t - Close_{t-1}|, |Low_t - Close_{t-1}|)
```

- `TR_t` is the true range of candle `t`, the honest size of that hour's move.
- `High_t` and `Low_t` are the highest and lowest prices during candle `t`.
- `Close_{t-1}` is the price at the end of the previous candle, so the last two terms catch a gap
  between one candle and the next, which the high and low alone would miss.

The average true range is the plain average of the last `N` true ranges:

```text
ATR_t = (TR_t + TR_{t-1} + ... + TR_{t-N+1}) / N
```

- `ATR_t` is the average true range at candle `t`.
- `N` is the period; the strategy uses 8, 9, 16 or 18 depending on the line.

Note a real difference. Wilder, who invented the average true range in 1978, smoothed it so old
candles fade slowly; the freqtrade implementation here takes a plain average of the last `N` candles,
which reacts faster and will not match a charting website's line exactly. The file warns about this
itself. The recipe below is the one in the code.

The two provisional bands sit one multiple of the average movement above and below the candle's middle:

```text
BasicUpper_t = (High_t + Low_t) / 2 + M * ATR_t
BasicLower_t = (High_t + Low_t) / 2 - M * ATR_t
```

- `M` is the multiplier; the strategy uses 1, 3, 4, 6 or 7 depending on the line.
- `(High_t + Low_t) / 2` is the middle of the hour's range, a rough centre of price.

Two final bands are then kept, and they are the ones that matter. Each final band is allowed to move
only in the direction that tightens it, except when price has already broken through it:

```text
FinalUpper_t = BasicUpper_t if BasicUpper_t < FinalUpper_{t-1} or Close_{t-1} > FinalUpper_{t-1}
               else FinalUpper_{t-1}

FinalLower_t = BasicLower_t if BasicLower_t > FinalLower_{t-1} or Close_{t-1} < FinalLower_{t-1}
               else FinalLower_{t-1}
```

- `FinalUpper_t` is the running upper band. It can fall but not rise, until price closes above it,
  which resets it.
- `FinalLower_t` is the running lower band. It can rise but not fall, until price closes below it.

The line itself is whichever band is active, and it switches only when price crosses it:

```text
Line_t = FinalUpper_t if Line_{t-1} = FinalUpper_{t-1} and Close_t <= FinalUpper_t
         else FinalLower_t if Line_{t-1} = FinalUpper_{t-1} and Close_t > FinalUpper_t
         else FinalLower_t if Line_{t-1} = FinalLower_{t-1} and Close_t >= FinalLower_t
         else FinalUpper_t
```

- `Line_t` is the value of the supertrend line at candle `t`.
- The last line of the rule says that if price fell through the lower band, the line jumps to the
  upper band.

The direction, which is all the strategy actually uses, is one comparison:

```text
direction_t = "down" if Close_t < Line_t else "up"
```

## A worked example

The arithmetic below uses `N = 3` and `M = 1`, smaller than the real settings so that the sums fit on
the page; the recipe is identical. The candles are one hour each, prices in dollars.

| Candle | High  | Low   | Close |
| ------ | ----- | ----- | ----- |
| 1      | 11.40 | 11.00 | 11.10 |
| 2      | 11.30 | 10.80 | 10.90 |
| 3      | 11.00 | 10.50 | 10.60 |
| 4      | 10.80 | 10.30 | 10.40 |
| 5      | 10.70 | 10.30 | 10.65 |
| 6      | 11.00 | 10.60 | 10.95 |
| 7      | 11.30 | 10.90 | 11.25 |
| 8      | 11.50 | 11.10 | 11.45 |
| 9      | 11.40 | 11.00 | 11.05 |
| 10     | 11.10 | 10.70 | 10.75 |
| 11     | 10.80 | 10.40 | 10.50 |
| 12     | 10.60 | 10.20 | 10.30 |

The true range of candle 1 is just its high minus its low, 0.40, because there is no earlier close.
For candle 6, the high minus the low is 0.40; the high minus the previous close is 11.00 - 10.65 =
0.35; the low minus the previous close is 10.60 - 10.65, which is -0.05, and its size is 0.05. The
largest is 0.40. The table now carries the whole calculation; `ATR` is the average of the last three
true ranges, and the first two rows are missing because the line needs three candles to start.

| Candle | TR   | ATR    | Middle | Upper  | Lower  | Final upper | Final lower | Line   | Direction |
| ------ | ---- | ------ | ------ | ------ | ------ | ----------- | ----------- | ------ | --------- |
| 3      | 0.50 | 0.4667 | 10.750 | 11.217 | 10.283 | 11.217      | 10.283      | 11.217 | down      |
| 4      | 0.50 | 0.5000 | 10.550 | 11.050 | 10.050 | 11.050      | 10.283      | 11.050 | down      |
| 5      | 0.40 | 0.4667 | 10.500 | 10.967 | 10.033 | 10.967      | 10.283      | 10.967 | down      |
| 6      | 0.40 | 0.4333 | 10.800 | 11.233 | 10.367 | 10.967      | 10.367      | 10.967 | down      |
| 7      | 0.40 | 0.4000 | 11.100 | 11.500 | 10.700 | 10.967      | 10.700      | 10.700 | up        |
| 8      | 0.40 | 0.4000 | 11.300 | 11.700 | 10.900 | 11.700      | 10.900      | 10.900 | up        |
| 9      | 0.45 | 0.4167 | 11.200 | 11.617 | 10.783 | 11.617      | 10.900      | 10.900 | up        |
| 10     | 0.40 | 0.4167 | 10.900 | 11.317 | 10.483 | 11.317      | 10.900      | 11.317 | down      |
| 11     | 0.40 | 0.4167 | 10.600 | 11.017 | 10.183 | 11.017      | 10.183      | 11.017 | down      |
| 12     | 0.40 | 0.4000 | 10.400 | 10.800 | 10.000 | 10.800      | 10.183      | 10.800 | down      |

Read the flip at candle 7. Through candles 3 to 6 the line sat above the price, correctly reading
"down" while the price fell. At candle 7 the close of 11.25 rose above the final upper band of 10.967,
so the line jumped to the final lower band at 10.70 and the direction became "up". At candle 10 the
close of 10.75 fell back below the line of 10.90, and the line read "down" again.

Now a trade. Suppose all three purchase lines agree the pair is up at candle 7, so the strategy buys
at the next candle's open, taken here as 11.25. It sells when the sale lines agree the pair is down,
taken here as candle 10, so the exit is at the next candle's open, 10.75. The return before costs is:

```text
(10.75 - 11.25) / 11.25 = -0.50 / 11.25 = -0.0444, that is -4.44 percent
```

The exchange fee is 0.10 percent of the amount on each side, so paying 0.001 twice is 0.20 percent,
and crossing the gap between the buy price and the sell price is taken as another 0.05 percent each
way. The total cost is 0.30 percent, so the net return is about -4.74 percent. This is the honest
shape of the rule: the crossover came, the line followed, and the move reversed before any profit was
reached. The stop and the profit ladder given above would not have changed the outcome.

## What the research actually found

The supertrend line itself has no published measurement. The Investopedia page that describes it lists
no performance figure, and the header of the strategy file says plainly that the implementation is not
validated against the source or any trusted study. What has been measured is the general idea behind
it, that prices which have been rising keep rising for a while.

| Source                                            | What it measured                                                   | Result                                                                                                                                                                    |
| ------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Moskowitz, Ooi and Pedersen, Time Series Momentum | The past twelve months of price for 58 futures and forward markets | A positive relationship between the past year and the next month in every one of the 58 markets, with the effect reversing at horizons of one to five years, before costs |
| Hurst, Ooi and Pedersen, A Century of Evidence    | Trend following on a century of data                               | Positive returns from following trends after fees and costs, described by the authors as consistent over the century                                                      |
| The strategy file itself                          | Nothing; it ships no sample                                        | The header states the implementation is not validated and the parameters came from a search over one period of Bitcoin data                                               |

Read together: there is published evidence that following trends has paid over long samples, mostly on
futures and mostly as a portfolio of markets rather than one. None of it tests this line, these
settings, or a single crypto pair; the numbers in this file came from a parameter search.

## How this project relates to it

Two pieces of this repository speak to the same question. The first is
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
a survey of what is actually predictable once many tests are counted; it is the reason a single
backtest on one pair is treated here as a hypothesis rather than a result. The second is
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
which shows how a rule chosen after seeing the results can look strong and then earn nothing on later
data; the parameters in this file were chosen exactly that way.

The sibling tutorial [A profit target that falls the longer a trade is held](../roi-target-ladder/README.md)
explains the other half of this strategy, the ladder that closes a trade once a shrinking profit
target is met.

## Where it goes wrong

- It is a lagging rule. The line only flips after the price has crossed it, so every entry and exit
  happens after the move it follows has partly happened. In the worked example the loss came because
  the flip down arrived one candle after the top.
- It whipsaws in a sideways market. When the price drifts without direction, the line flips up and
  down and each flip costs the full fee plus the gap between prices. The strategy file's own author
  notes this pattern.
- The parameters were searched, not tested. Six settings and a profit ladder were chosen by trying
  combinations on one period of data, so the winning combination fits that period and says nothing
  about the next one.
- The implementation may not match the familiar indicator. A plain average of true ranges replaces
  Wilder's smoothed one, so the line drawn here can differ from a charting site's line. The file warns
  about this.
- Longing and shorting in the futures version double the number of flips, and each flip pays the cost
  twice over; a whipsaw that costs one trade in the spot version costs two in the futures version.

## Try it yourself

You need nothing but a spreadsheet and a public source of hourly prices, which most exchanges publish.

1. Copy the twelve candles from the worked example into a sheet, one row per candle, with columns for
   the high, the low and the close.
2. Add a column `TR` holding the true range, `MAX(high-low, ABS(high-previous close), ABS(low-previous close))`.
3. Add a column `ATR` holding the average of the last three `TR` values.
4. Add columns `Upper` and `Lower`, being the middle of the range plus and minus `ATR`.
5. Add a column `Line`: start it at the first `Upper` value, then copy the previous line down unless
   the close has crossed it, in which case take the other band.
6. Add a column `Direction` reading "down" when the close is below the line and "up" otherwise.

What to notice: the line stays on one side for several candles and then jumps, and the distance it
sits from the price grows while the market is calm and shrinks while it is wild. Change the multiplier
in step 4 from 1 to 3 and repeat: the line sits further away, flips less often, and gives back more of
each move when it does flip. That trade-off between fewer flips and later flips is the whole tuning
problem, and it is why a search over the settings suits one period and one pair only.

## Where this came from

- [Supertrend.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/Supertrend.py)
  and its [futures variant](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/FSupertrendStrategy.py),
  the files whose rules, settings and warning are quoted above.
- [Investopedia: the supertrend indicator](https://www.investopedia.com/supertrend-indicator-7976167),
  the plain-language description and the account of where the indicator came from.
- [Moskowitz, Ooi and Pedersen, Time Series Momentum](https://www.sciencedirect.com/science/article/pii/S0304405X11002613),
  the measured trend-following result, and Hurst, Ooi and Pedersen,
  [A Century of Evidence](https://www.aqr.com/Insights/Research/Journal-Article/A-Century-of-Evidence-on-Trend-Following-Investing).
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
  and [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own reviews of what survives a test.
- The freqtrade discussion [issue 30](https://github.com/freqtrade/freqtrade-strategies/issues/30),
  where the implementation used here was worked out and the difference from other average-range
  measures was noted.

## Words used in this tutorial

- candlestick: one row of price history for a fixed period, holding the highest, lowest, first and last price.
- crossover: the moment a price crosses a line, which is where this strategy acts.
- futures: a contract to trade at a fixed price on a future date; the futures variant can sell what it does not own.
- long: owning something, so that a rise is a gain.
- short: selling something you do not own, so that a fall is a gain.
- spread: the gap between the best price at which you can buy and the best price at which you can sell.
- stop loss: an instruction that closes a position once its price reaches a chosen level, limiting the loss.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
