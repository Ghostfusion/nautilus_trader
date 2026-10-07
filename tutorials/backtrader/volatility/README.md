# Volatility channels: Keltner, SuperTrend and the chandelier exit

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | One instrument at a time, usually a single share such as Oracle, or Chinese rebar futures in one variant                                                                                                                                   |
| How often it trades       | Days to weeks between trades; the rules wait for a price to break a channel rather than trading every day                                                                                                                                  |
| What you need             | Python and a data file for the full backtests; a spreadsheet is enough to follow the arithmetic here                                                                                                                                       |
| Where the rules come from | [Strategy Compendium, article 16, volatility](https://backtrader.readthedocs.io/en/latest/strategies-series/en/16-volatility.html)                                                                                                         |
| The underlying research   | Practitioner rules of thumb rather than published studies: Chester Keltner's 1960 channels, Linda Raschke's version using true range, Olivier Seban's SuperTrend and Chuck LeBeau's chandelier exit, all described in the category article |
| How well it held up       | Weak: on the single five-year share sample the naked systems came out close to flat, and only one variant improved when a momentum filter was added                                                                                        |
| Also appears in           | [Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md), which covers the trailing and chandelier stop forms as a separate layer                                                                                    |

## The idea in one paragraph

Give the price a pair of lines that breathe. When the market is calm the lines sit close to the
price; when it is wild they spread apart. The lines are built from a moving average plus or minus a
multiple of how far prices have recently been swinging. A price that pushes above the upper line is
said to be breaking out, and the rule buys, betting the move continues. A price that falls back to
the middle line is said to be losing strength, and the rule sells. One variant, SuperTrend, keeps
only a single line, on whichever side of the price the trend is, and treats a crossing as a reversal.
Another, the chandelier exit, hangs a stop below the highest price reached, by the same swinging
measure, and trails it upward.

## Why anyone believed it

A channel translates a vague word, volatile, into a number, and a number can be acted on. The claim
is narrower than it sounds: the rule does not predict direction, it only insists that a move be
large relative to recent noise before it acts. On a calm day an ordinary wobble crosses the line and
produces a false signal; on a wild day the line is far away, so only a genuine surge gets through.
This is why the width of the channel is the whole idea, and why a fixed percentage channel, as
Keltner first drew it, was replaced by one built from the true range, which counts the day's full
swing including the gap from yesterday.

The counterparty is the trader who is forced to act on noise. In a choppy market, people who buy
every small rise and sell every small fall hand their money to whoever waits for the larger move.
A channel is a way of waiting. The trailing stop has a different counterparty: the person who is
still holding a fallen position, hoping it comes back, and who sells only when the pain becomes too
much, which is often near the point the stop trades.

## An everyday comparison

Think of a fisherman setting the size of a net. In a calm, clear pool he uses a fine net, because the
small ripples are noise and he can see what is a fish. In a churning sea he uses a coarse net, because
a fine one catches only weed. The net size is a response to the conditions, not a forecast of where the
fish are. And once he has a fish, he ties the line to a float that only ever slides down the bank as
the water rises, never back up the bank, so a fish that pulls hard cannot drag the line with it.

## The rules, step by step

All three systems rest on one measure, so it is worth stating first. The true range of a day is the
largest of three numbers: the day's high minus its low, the day's high minus yesterday's close, and
yesterday's close minus the day's low. The average true range, or ATR, is simply the average of the
true range over the last N days. It says how far, on a typical day, the price travels, ignoring
direction.

The Keltner channel:

1. Compute a moving average of the closing prices, usually an exponential average over 20 days. That
   is the middle line.
2. Compute the ATR over 20 days.
3. The upper line is the middle line plus twice the ATR; the lower line is the middle line minus
   twice the ATR.
4. If you hold nothing and the close is above the upper line, buy.
5. If you hold a position and the close falls below the middle line, sell everything.

SuperTrend:

1. Compute the ATR over 10 days.
2. The basic upper line is the midpoint of the day, high plus low divided by two, plus three times
   the ATR. The basic lower line is the midpoint minus three times the ATR.
3. Carry one line forward as the SuperTrend line. In an uptrend the line is the lower line, and it
   may only rise, never fall. In a downtrend it is the upper line, and it may only fall, never rise.
4. Buy on the day the price crosses the line from below, which flips the trend to up.
5. Sell on the day the price crosses the line from above, which flips the trend to down. Entry and
   exit are the same event, so no separate stop is needed.

The chandelier exit, which supplies no entry of its own:

1. Choose an entry rule. The test uses a simple moving average of 8 days crossing above a simple
   moving average of 15 days, called a golden cross.
2. Compute the highest high of the last 22 days minus three times the 22-day ATR. That is the long
   chandelier line; the short version is the lowest low plus the same amount.
3. Buy when both the golden cross has happened and the close is above the long chandelier line.
4. Sell when both the death cross has happened, meaning the 8-day average has crossed below the
   15-day average, and the close is below the long chandelier line.

## The maths, with every symbol named

The shared measure is the true range and its average:

```text
TR_t = max( H_t - L_t , abs(H_t - C_{t-1}) , abs(L_t - C_{t-1}) )
ATR_t = ( TR_t + TR_{t-1} + ... + TR_{t-N+1} ) / N
```

- `H_t`, `L_t` and `C_t` are today's high, low and close.
- `C_{t-1}` is yesterday's close, so the second and third terms capture the gap between yesterday's
  close and today's extremes.
- `TR_t` is today's true range, the widest the price travelled, in price units.
- `ATR_t` is the average true range, the typical daily travel over the last `N` days.
- `N` is the look-back in days, 20 for the Keltner channel and 10 for SuperTrend.
- `max(...)` takes the largest of the three numbers; `abs(...)` takes the size without the sign.

What it means: ATR is a speedometer without a compass. It says how fast the price moves but nothing
about which way, which is why two strategies can share it and still point opposite ways.

The Keltner channel turns the average and the ATR into two prices:

```text
mid   = EMA(close, 20)
upper = mid + k * ATR_20
lower = mid - k * ATR_20
```

- `mid` is the 20-day exponential moving average of the close, a smoothed price.
- `k` is the multiplier, 2.0 in the test.
- `upper` and `lower` are the two channel lines.

What it means: the channel is the smoothed price with a margin on each side. The margin grows when
the market is wild and shrinks when it is calm.

SuperTrend and the chandelier both use the same margin around a different centre:

```text
basic_upper = (H + L) / 2 + k * ATR
basic_lower = (H + L) / 2 - k * ATR
chandelier_long = highest_high_22 - k * ATR_22
```

- `(H + L) / 2` is the day's midpoint, used by SuperTrend.
- `k` is 3.0 in both.
- `highest_high_22` is the highest high over the last 22 days.
- `chandelier_long` is the trailing stop level for a long position.

What it means: SuperTrend's line is the midpoint shifted by three days' typical travel; the
chandelier's is the peak price shifted down by the same amount, so it follows the trade upward.

## A worked example

First the shared input, the ATR, over six invented days. The numbers are gold-sized but made up.

| Day | High    | Low     | Close   | True range | Note                              |
| --- | ------- | ------- | ------- | ---------- | --------------------------------- |
| 1   | 2020.00 | 2005.00 | 2010.00 | 15.00      | no previous close, high-low       |
| 2   | 2035.00 | 2010.00 | 2030.00 | 25.00      | high minus low                    |
| 3   | 2050.00 | 2030.00 | 2045.00 | 20.00      | high minus low                    |
| 4   | 2075.00 | 2045.00 | 2070.00 | 30.00      | high minus low                    |
| 5   | 2080.00 | 2065.00 | 2072.00 | 15.00      | high minus low                    |
| 6   | 2055.00 | 2030.00 | 2035.00 | 42.00      | low is 42 below yesterday's close |

Day 6 shows why the true range matters: the day's own range is only 25.00, but the close fell 37.00
from 2072.00 to 2035.00, and the low is 42.00 below the previous close, so the true range is 42.00.
The 5-day average on day 6 is `(25.00 + 20.00 + 30.00 + 15.00 + 42.00) / 5 = 132.00 / 5 = 26.40`, so
ATR is 26.40 and three times it is 79.20.

The Keltner channel, using a middle line that drifts up with the trend, with 2 times ATR = 52.80:

| Day | Close   | Middle  | Upper   | Action                   |
| --- | ------- | ------- | ------- | ------------------------ |
| 1   | 2010.00 | 2005.00 | 2057.80 | hold                     |
| 2   | 2030.00 | 2008.00 | 2060.80 | hold                     |
| 3   | 2045.00 | 2012.00 | 2064.80 | hold                     |
| 4   | 2075.00 | 2020.00 | 2072.80 | buy, close above upper   |
| 5   | 2080.00 | 2030.00 | 2082.80 | hold                     |
| 6   | 2095.00 | 2042.00 | 2094.80 | hold                     |
| 7   | 2040.00 | 2050.00 | 2102.80 | sell, close below middle |

The entry is `2075.00 > 2020.00 + 52.80 = 2072.80`, true by 2.20. The exit is `2040.00 < 2050.00`,
the middle line. The trade buys at 2075.00 and sells at 2040.00, a gross loss of 35.00; commission at
0.1 percent of each side is `(2075.00 + 2040.00) * 0.001 = 4.12`, and a spread of 0.05 on each side
is 0.10, so the net loss is `35.00 + 4.12 + 0.10 = 39.22`, or `39.22 / 2075.00 = 1.89` percent.

SuperTrend, using the carried line with the same 79.20 margin:

| Day | Close   | SuperTrend line | Direction | Action                    |
| --- | ------- | --------------- | --------- | ------------------------- |
| 1   | 2010.00 | 2085.00         | down      | hold                      |
| 2   | 2030.00 | 2080.00         | down      | hold                      |
| 3   | 2045.00 | 2072.00         | down      | hold                      |
| 4   | 2075.00 | 2068.00         | down      | hold                      |
| 5   | 2080.00 | 2055.00         | up        | buy, price crossed above  |
| 6   | 2110.00 | 2060.00         | up        | hold                      |
| 7   | 2040.00 | 2075.00         | down      | sell, price crossed below |

The line sits above the price in a downtrend and below it in an uptrend, and it only moves in the
direction that tightens the stop. The buy is at 2080.00 when the price crossed above 2055.00; the
sell is at 2040.00 when it crossed below the carried 2075.00. Gross loss 40.00, net after the same
costs `40.00 + 4.12 + 0.10 = 44.22`, or 2.13 percent. This is the whipsaw the article warns about: a
single reversal in a choppy market turns a profit into a loss.

The chandelier exit, welded to the 8-day and 15-day average cross:

| Day | Close   | Average 8 | Average 15 | High 22 | Chandelier | Action                       |
| --- | ------- | --------- | ---------- | ------- | ---------- | ---------------------------- |
| 1   | 2040.00 | 2035.00   | 2040.00    | 2090.00 | 2010.80    | hold                         |
| 2   | 2050.00 | 2039.00   | 2040.00    | 2100.00 | 2020.80    | hold                         |
| 3   | 2060.00 | 2043.00   | 2041.00    | 2100.00 | 2020.80    | buy, cross up and above stop |
| 4   | 2150.00 | 2060.00   | 2048.00    | 2150.00 | 2070.80    | hold                         |
| 5   | 2250.00 | 2090.00   | 2060.00    | 2250.00 | 2170.80    | hold                         |
| 6   | 2300.00 | 2122.00   | 2075.00    | 2300.00 | 2220.80    | hold                         |
| 7   | 2280.00 | 2150.00   | 2095.00    | 2300.00 | 2220.80    | hold                         |
| 8   | 2200.00 | 2140.00   | 2110.00    | 2300.00 | 2220.80    | hold                         |
| 9   | 2100.00 | 2120.00   | 2125.00    | 2300.00 | 2220.80    | sell, cross down and below   |

The chandelier line is `2300.00 - 79.20 = 2220.80` once the peak is set, and it never falls. On day 9
both conditions finally hold: the 8-day average at 2120.00 is below the 15-day at 2125.00, and the
close 2100.00 is below the stop. The buy was at 2060.00 and the sell at 2100.00, a gross gain of
40.00; commission `(2060.00 + 2100.00) * 0.001 = 4.16` and a spread of 0.10 give a net gain of 35.74,
or 1.74 percent.

## What the research actually found

The category article reports what each test produced on its own sample, with commission at 0.1
percent. The naked SuperTrend on Oracle daily bars from 2010 to 2014, over 1,247 bars, finished at
99,999.23 against a starting 100,000, a reward-to-risk ratio of minus 0.004, and a worst fall of
11.22 percent. The Keltner channel over 1,238 bars finished at 100,039.51, a reward-to-risk ratio of
0.2796, with a worst fall of just 5.50 percent. The chandelier welded to the average cross over 1,235
bars finished at 100,018.36, a reward-to-risk ratio of 0.1430, with a worst fall of 8.41 percent.
Adding a momentum filter to SuperTrend produced the best result in the category, 100,085.04 with a
reward-to-risk ratio of 0.8988, while two other SuperTrend variants finished below 100,000. Two
implementations of the same Keltner idea, `test_70` and `test_108`, assert identical numbers, which
is the point of writing the same rule twice: they confirm each other.

Read carefully, these are five-year, one-instrument results, and the differences between them are
small. The category article is candid that the naked SuperTrend was "whipsawed on a chop-heavy
stock". Nothing here establishes that any of the three systems earns a return on data it has not
seen.

One habit applies to every number above. Every backtest in the compendium asserts its final
portfolio value, its reward-to-risk ratio and its worst fall against a baseline recorded when the
test was migrated. Passing that assertion proves the engine computes exactly what the file says, in
both its computation modes, and nothing more. A green test means the arithmetic is faithful to the
rules, not that the rules make money.

## How this project relates to it

The stop forms in this category are the direct subject of
[Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md) in this collection,
which writes the trailing stop and the chandelier stop as an explicit layer with a monotonic rule: a
trailing stop must never loosen. That page draws on this repository's own engine design,
[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), whose Section 7
gives the trailing and chandelier forms as `Stop_t = max(Stop_{t-1}, P_t - k * ATR_t)` and
`CE_long = HighestHigh_n - k * ATR_n`, exactly as used here.

The engine design is also blunt about this whole class of indicators. Its Section 8 rejects Keltner
channels along with Ichimoku, parabolic SAR and pivot points as "indicator proliferation", each one
another parameter with no independent evidence. So this category is worth reading as a careful tour
of how the indicators behave, not as a menu to adopt.

## Where it goes wrong

- One instrument, one window. Every result here is a single share over five years, and the same
  rules on a different name or period may behave differently. A test on Oracle daily bars is not
  evidence about gold or about next year.
- The channel width is a tuned choice. The ATR period and the multiplier set how often the rule
  trades and whether it is whipsawed; changing 3.0 to 2.5 changes the result, and choosing the best
  setting after seeing the curve is fitting noise.
- Choppy markets punish breakouts. A price that crosses a channel and falls straight back produces a
  loss twice, once on the entry and once on the exit, and there is nothing in the rule to tell a
  breakout from a wobble.
- Stops are not free. A trailing stop cuts large losses and also cuts the recoveries that would have
  repaid them; whether that helps depends on whether prices bounce after falling, which the stop
  cannot know.
- Costs accumulate. Every flip pays the spread and commission. A rule that flips often pays often,
  and the costs here were charged at a single rate on one liquid instrument, not on a spread that
  widens in stress.
- The good-looking variant was chosen after the fact. The best result in the category came from
  adding a momentum filter to SuperTrend; that filter was selected by looking at the results, which
  is the standard way a backtest is made to look better than it is.

## Try it yourself

You need a spreadsheet and a column of daily highs, lows and closes; any finance website gives them.

1. Build four columns: date, high, low, close.
2. Add a true-range column with the largest of: high minus low; high minus yesterday's close; and
   yesterday's close minus low.
3. Add a column for the 20-day average of the true range; call it ATR.
4. Add a column for the 20-day average of the close, and two more: the average plus 2 times the ATR,
   and the average minus 2 times the ATR.
5. Add a signal column: write "buy" on the first day the close is above the upper line while flat,
   and "sell" on the first day the close is below the middle line while holding.
6. Finally, list every completed trade and subtract the cost: about 0.1 percent of the trade value on
   each side.

What to notice: count how many signals are followed by a move that continues past the middle line
and how many turn straight back. On a choppy series the second group dominates, and the losses from
it are what the ATR multiplier is trying to reduce.

## Where this came from

- [Strategy Compendium, article 16, volatility](https://backtrader.readthedocs.io/en/latest/strategies-series/en/16-volatility.html),
  the category inventory, the three deep dives and the reported baselines.
- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Section 7, for
  the trailing, chandelier and volatility-scaled stop formulas, and Section 8, for the rejection of
  Keltner channels as indicator proliferation.
- [Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md), this collection's
  tutorial on the same stop forms as a separate layer.
- [Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md),
  for the finding that a stop reshapes the distribution of outcomes rather than adding return.

## Words used in this tutorial

- channel: a pair of lines drawn above and below a moving average, used as trigger levels.
- true range: the widest the price travelled in a day, counting the gap from the previous close.
- average true range (ATR): the average of the true range over a chosen number of days.
- exponential moving average: an average that gives more weight to recent prices than an old simple
  average does.
- breakout: a price moving beyond a level it had not exceeded for a while.
- whipsaw: a move that reverses immediately, causing two losses in a row.
- golden cross: a short moving average rising above a longer one, taken as a bullish sign.
- trailing stop: a stop level that follows the price upward and never moves back down.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
