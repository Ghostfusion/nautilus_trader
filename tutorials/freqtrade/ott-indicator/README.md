# The optimized trend tracker: one line that follows the price, and a band that turns the trade around

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                              |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto coin futures contracts on a crypto exchange, taken in both directions, for example Bitcoin priced in a stablecoin                                                                                                           |
| How often it trades       | On one-hour candles, a few times a month, but as often as the two lines cross                                                                                                                                                      |
| What you need             | A spreadsheet and one column of hourly closing prices                                                                                                                                                                              |
| Where the rules come from | [FOttStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/FOttStrategy.py), the file in the Freqtrade community strategy repository                                               |
| The underlying research   | None, this rests on a published chart indicator rather than a paper: the [Optimized Trend Tracker](https://www.tradingview.com/script/zVhoDQME/) on TradingView, whose adaptive average comes from the Chande Momentum Oscillator  |
| How well it held up       | Weak: no result is published for this file, its settings were produced by a parameter search over one period and one set of pairs, and the repository's own README calls its files starting points rather than strategies to trade |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                    |

## The idea in one paragraph

This strategy watches two lines drawn from the hourly price of a cryptocurrency. The first is a
self-adjusting average: an average of the price that leans hard on the newest price when the market
is moving and leans on its own past value when the market is quiet. The second line is a band drawn
a fixed percentage away from the first, and it is only allowed to move one way while a trend lasts,
so it acts like a climbing floor under a rising price or a sinking ceiling above a falling price.
The strategy buys when the average crosses above the band and sells short, a bet that the price will
fall, when the average crosses below it. A separate measure of trend strength, the average
directional index, closes whichever position is open once it reads above 60.

## Why anyone believed it

Prices in a market do not absorb news instantly. When a large buyer works through an order over
hours, or when a piece of news is understood gradually, the price keeps drifting in the same
direction, and a rule that waits for the drift to start and then stays in it collects part of the
move. The band adds a second belief: most of the movement inside an hour is noise, so a rule that
only turns around when the price has travelled a set distance, rather than whenever it ticks the
wrong way, is not shaken out by the noise.

The counterparty is the trader who is forced to transact regardless of the price: someone closing a
leveraged position that has moved against them, a fund that must sell to meet withdrawals, or a
hedger who needs to lock in a price today. Those sellers keep pushing a falling price down, which is
what the short side of this rule tries to collect.

## An everyday comparison

Think of a room thermostat with a wide dead zone. It does not switch the heater on the instant the
temperature dips below the setting and off the instant it rises above it; it turns on only when the
temperature has fallen a set amount below the target, and off only when it has risen a set amount
above. The dead zone costs a little comfort and saves the machinery from flickering on and off every
minute. The band in this strategy is that dead zone: the trade only turns around when the price has
travelled far enough to matter.

## The rules, step by step

1. Use candles covering one hour of trading, where a candle records the opening price, the highest
   price, the lowest price and the closing price of that hour.
2. For each candle compute the upward move and the downward move of the closing price: the upward
   move is the close minus the previous close if that is positive and zero otherwise, and the
   downward move is the previous close minus the close if that is positive and zero otherwise.
3. Add the last nine upward moves together to get a number called the up sum, and the last nine
   downward moves together to get the down sum.
4. Divide the difference between those two sums by their total and keep the answer without a sign.
   That number, between 0 and 1, says how one-sided the last nine hours have been; call it the
   momentum reading.
5. Update the adaptive average with the rule in the next section. It uses the closing price and the
   momentum reading.
6. Draw a band around the adaptive average at a distance of 1.4 percent of its own value.
7. Compare the average with the band. If the average has moved from below the band to above it,
   buy. If it has moved from above the band to below it, sell short.
8. Hold the position until the average crosses the band the other way, or until the average
   directional index, a measure of how strong and one-sided the recent movement has been, reads
   above 60, whichever happens first.
9. Three automatic exits apply at every candle. The position closes if it is ahead by the amount in
   the ladder: 10 percent before minute 30, 75 percent from minute 30, 5 percent from minute 60 and
   2.5 percent from minute 120. It closes if it is behind by 26.5 percent. A trailing stop also
   holds the exit 26.5 percent below the highest price the trade has seen, tightening to 5 percent
   below it once the gain passes 10 percent.

## The maths, with every symbol named

The two components of the indicator are the adaptive average, which the file calls the VAR line, and
the band, which it calls the OTT line. Both are computed from the closing price alone.

The momentum reading, which decides how fast the average may move:

```text
UD = sum of the upward moves over the last 9 candles
DD = sum of the downward moves over the last 9 candles
CMO = |UD - DD| / (UD + DD)
```

- `UD` is the up sum: how much, in price units, the close rose across the nine hours, counting only
  the hours in which it rose.
- `DD` is the down sum: how much it fell across the nine hours, counting only the hours in which it
  fell, written as a positive number.
- `CMO` is the momentum reading, between 0 and 1. It is 1 when every one of the nine hours moved the
  same way and 0 when the rises and the falls are equal. The bars mean "ignore the sign".

The adaptive average:

```text
alpha = 2 / (period + 1), with period = 2, so alpha = 0.6667
VAR_t = alpha * CMO_t * close_t + (1 - alpha * CMO_t) * VAR_(t-1)
```

- `alpha` is the fixed weight the formula is allowed to place on the newest price at most.
- `period` is the averaging length, set to 2 in this file.
- `CMO_t` is the momentum reading at this candle.
- `close_t` is the closing price at this candle.
- `VAR_t` is the average at this candle, and `VAR_(t-1)` is the average one candle earlier.
- The weight on the newest price is `alpha * CMO`, so at most 0.6667 of the average can come from the
  newest close; during a choppy stretch the weight falls and the average freezes.

The band and the direction:

```text
fark = VAR_t * percent / 100, with percent = 1.4
raw_floor = VAR_t - fark
raw_ceiling = VAR_t + fark
```

- `fark` is the half-width of the band, 1.4 percent of the average.
- `raw_floor` and `raw_ceiling` are the two candidate edges of the band.

The edges are then ratcheted, which is what makes the band one-directional:

```text
floor_t   = max(raw_floor, floor_(t-1))     while the average is above the previous floor
ceiling_t = min(raw_ceiling, ceiling_(t-1)) while the average is below the previous ceiling
```

- `floor_t` only ever rises while the trend is up, so it can never loosen its grip on a rising price.
- `ceiling_t` only ever falls while the trend is down.
- The direction takes the value +1 after the average crosses above the ceiling and -1 after it
  crosses below the floor.

Finally the line that is drawn:

```text
mt  = floor if the direction is +1, otherwise ceiling
OTT = mt * (200 + percent) / 200   when VAR_t > mt
OTT = mt * (200 - percent) / 200   otherwise
```

- `mt` is the active edge of the band, floor or ceiling depending on the direction.
- `OTT` pushes that edge a small step of 0.7 percent further away, in whichever direction the average
  last crossed it.
- The line a chart shows is this value from two candles earlier, so the signal is never drawn on a
  candle whose close is not yet known.

The trade signal is then the crossing of the two lines:

```text
enter long   when VAR crosses above OTT
enter short  when VAR crosses below OTT
exit both    when the average directional index, computed over 14 candles, is above 60
```

- `VAR` and `OTT` are the two lines above.
- The average directional index runs from 0 to 100 and is high when the recent movement is strong and
  one-sided; a reading above 60 is rare.

## A worked example

Six hourly candles. For readability the momentum reading is taken as 0.50 on every candle, so the
weight on the newest close is 0.6667 times 0.50, which is 0.3333. Before the table the adaptive
average stands at 100.00 and the ceiling stands at 102.00, meaning the market has been falling. The
prices below are invented but of the size that an hourly crypto candle can take.

| Candle | Close  | Weight on close | VAR    | fark | New ceiling | Ceiling after the ratchet | Band   |
| ------ | ------ | --------------- | ------ | ---- | ----------- | ------------------------- | ------ |
| before |        |                 | 100.00 |      |             | 102.00                    |        |
| 1      | 100.00 | 0.3333          | 100.00 | 1.40 | 101.40      | 101.40                    | 100.69 |
| 2      | 99.00  | 0.3333          | 99.67  | 1.40 | 101.07      | 101.07                    | 100.36 |
| 3      | 98.00  | 0.3333          | 99.11  | 1.39 | 100.50      | 100.50                    | 99.80  |
| 4      | 97.00  | 0.3333          | 98.41  | 1.38 | 99.79       | 99.79                     | 99.09  |
| 5      | 99.00  | 0.3333          | 98.61  | 1.38 | 99.99       | 99.79                     | 99.09  |
| 6      | 101.00 | 0.3333          | 99.41  | 1.39 | 100.80      | 99.79                     | 99.09  |

Read one row at a time. At candle 4 the average is 98.41, the half-width is 98.41 times 0.014, which
is 1.38, so the new ceiling is 99.79, and the ratchet keeps it there because 99.99 is higher than
99.79 at candle 5. The band is the ceiling times 0.993, because the average sits below the ceiling.

The signal compares today's average with the band from two candles earlier. At candle 5 the average
is 98.61 and the band from candle 3 is 99.80, so the average is still below. At candle 6 the average
is 99.41 and the band from candle 4 is 99.09, so the average has crossed above the band: the
direction flips to up and the buy signal fires.

Now the trade. The entry price is the close of candle 6, 101.00. Suppose the price keeps rising and
the average directional index reaches 60 at a price of 103.50, which closes the position:

```text
Gross gain = 103.50 / 101.00 - 1 = 0.0248, that is +2.48 percent
Cost       = 2 sides * 0.075 percent = 0.15 percent
Net gain   = 2.48 - 0.15 = +2.33 percent
```

The fee of 0.075 percent per side is inside the range a crypto exchange charges for a small order,
0.05 to 0.10 percent of the amount traded. Two points are worth noticing. First, the average sat
below its own ceiling for five candles while the price fell, and the ratchet meant the ceiling never
loosened; the band is doing the work of a trailing stop. Second, the automatic exits would not have
closed this trade at 2.33 percent, so the exit came from the strength reading instead.

## What the research actually found

Nothing was measured for this file. The repository prints no backtest, no number of trades and no
result for `FOttStrategy.py`; the only trace of a measurement is a comment in the file saying that
the ladder, the stop loss, the trailing stop and the parameters were produced by
`freqtrade hyperopt --strategy Supertrend --hyperopt-loss ShortTradeDurHyperOptLoss
--timerange=20210101- --timeframe=1h --spaces all`, that is, by a search over settings on the pairs
and the period the author had loaded, from January 2021 onward. A parameter search of that kind
picks the settings that did best on that slice, and the repository's own README warns that results
depend heavily on the pairs, the timeframe and the period, and that the files are starting points
rather than strategies to trade.

The indicator itself carries no measured claim either. The published chart indicator states a rule,
not a result. The nearest body of measurement is for trend following in general: Moskowitz, Ooi and
Pedersen found, in 58 futures markets from 1965 to 2009, that the past twelve-month return of a
contract predicts its next return over about a year, and Hurst, Ooi and Pedersen extended the same
test back to 1880 across 67 markets and found positive average returns in every decade. Those tests
are about holding a contract when its own past return was positive, at monthly frequency. They say
nothing about a two-period adaptive average on hourly candles, and nothing about this file.

## How this project relates to it

This repository implements the interesting half of the indicator, the adaptive average, in
[crates/indicators/src/average/vidya.rs](../../../crates/indicators/src/average/vidya.rs), under the
name `VariableIndexDynamicAverage`. That file's average, like the VAR line here, weights the newest
price by the
[Chande momentum oscillator](../../../crates/indicators/src/momentum/cmo.rs) over a nine-bar window,
so a reader can see the same arithmetic written as production code with tests around it. What the
repository does not implement is the band, the ratchet or the crossing rule, so nothing here runs
this strategy; the file is a sibling implementation to compare against.

## Where it goes wrong

- No evidence. There is no paper, no published backtest and no independent replication behind this
  file's settings; the settings came from one search over one period and one set of pairs, which is
  the definition of a fitted rule rather than a measured one.
- The exit is unusual and may be unreachable. Ending the trade only when the average directional
  index passes 60 means most trades end by the ladder or the stop loss instead, and those numbers
  were fitted by the same search, so the exit that actually fires is the least examined part.
- The ladder is not a declining ladder: the target rises to 75 percent in the second half hour, so a
  reader should not assume a ladder always shrinks; in this file it does not.
- Costs and funding. Every crossing pays the gap between the buying and the selling price plus the
  exchange fee, on both sides, and a crypto perpetual futures contract also charges a funding
  payment for as long as the position is held, which no chart shows.
- The band delays the turn. The line a chart draws is the band from two candles earlier, and the
  ratchet holds the edge far from the price after a fast move, so the strategy gives back part of a
  sharp reversal before it turns.
- Crypto pairs are thin. In a pair with few buyers the gap between the buying and selling price is
  wide, and that gap is paid on the way in and on the way out, which can be larger than the whole
  gain in the example above.

## Try it yourself

You need a spreadsheet and sixty hourly closing prices from any public price chart.

1. Column A holds the candle number, column B the closing price.
2. Column C is the upward move: `=MAX(0, B3-B2)`. Column D is the downward move:
   `=MAX(0, B2-B3)`.
3. Column E is the up sum, the total of the last nine values of column C: `=SUM(C3:C11)` on the row
   where nine candles exist. Column F is the same for column D.
4. Column G is the momentum reading: `=ABS(E3-F3)/(E3+F3)`.
5. Column H is the average: `=0.6667*G3*B3+(1-0.6667*G3)*H2`, with the first cell of the column set
   to the first closing price.
6. Column I is the band half-width: `=H3*0.014`, and columns J and K are `=H3-I3` and `=H3+I3`.

What to notice: at the top of the sheet the average is pinned to the first price and takes many
hours to pull away from it, which is why the file tells the bot to ignore the first eighteen candles.
Then watch a stretch of quiet, choppy prices: the momentum reading collapses toward zero and the
average turns nearly flat, so the band hugs it closely and the rule fires on small moves; a rule that
speeds up in quiet markets is the opposite of what the story in the previous sections promises.

## Where this came from

- [FOttStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/FOttStrategy.py),
  the rules as implemented: the two lines, the 1.4 percent band, the 60 reading exit, the ladder, the
  stop loss, the trailing stop and the one-hour timeframe.
- [Optimized Trend Tracker on TradingView](https://www.tradingview.com/script/zVhoDQME/), the chart
  indicator the file adapts, including the adaptive average, the operating period of 2 and the band
  width of 1.4 percent.
- [Indie Script's Optimized Trend Tracker port guide](https://indie-script.github.io/indicators/Optimized%20Trend%20Tracker/),
  which sets out the same formulas in words, states that the band is drawn from two bars earlier so
  that it does not repaint, and lists the sources of the indicator.
- [The Freqtrade strategy repository README](https://github.com/freqtrade/freqtrade-strategies/blob/main/README.md),
  which states that the files are starting points rather than ready strategies and that results
  depend on the pairs, timeframe and period chosen.
- Moskowitz, Ooi and Pedersen, [Time Series Momentum](https://www.aqr.com/Insights/Research/Journal-Article/Time-Series-Momentum),
  the paper behind the general claim that recent direction predicts near-future direction.
- Hurst, Ooi and Pedersen, [A Century of Evidence on Trend-Following Investing](https://www.aqr.com/Insights/Research/Journal-Article/A-Century-of-Evidence-on-Trend-Following-Investing),
  the long out-of-sample test of that claim across 67 markets from 1880 to 2016.

## Words used in this tutorial

- candle: one interval of trading, described by its opening, highest, lowest and closing price.
- crossing: one line moving from one side of another line to the other side.
- futures contract: an agreement to buy or sell something at a set date, commonly used to bet on a
  price with borrowed money.
- long: owning something, so that a rise in its price is a gain.
- short: selling something borrowed, so that a fall in its price is a gain.
- stop loss: the loss at which a position is closed automatically.
- timeframe: the length of one candle, so a one-hour timeframe means each candle covers an hour.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
