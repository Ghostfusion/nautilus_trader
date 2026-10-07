# Time-based strategies: when the engine is allowed to act

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                       |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A single price series, a stock index quoted once a day, bought and sold whole on a moving-average crossover                                                 |
| How often it trades       | A handful of times a year; the point of the category is when the engine is allowed to act, not how often                                                    |
| What you need             | Python and two columns of daily prices, or a spreadsheet for the arithmetic                                                                                 |
| Where the rules come from | [backtrader strategy compendium, category 21, time-based](https://backtrader.readthedocs.io/en/latest/strategies-series/en/21-time-based.html)              |
| The underlying research   | None: the signals are textbook dual moving-average crossovers; the category exists to test the engine's timers, resampling and replay, not to claim an edge |
| How well it held up       | Weak, resting on a plain crossover with a thin published record; the category itself makes no performance claim, and its numbers are engine checks          |
| Also appears in           | Nothing else in this collection                                                                                                                             |

## The idea in one paragraph

Most of this collection asks whether a trading rule makes money. This category asks a different
question: when, exactly, is the rule allowed to do anything at all? Every strategy in it is a plain
pair of moving averages that buys when the short average rises above the long one and sells when it
falls back below. A timer decides the strategy wakes at fixed moments, such as the opening of each
trading day. Resampling shows the rule only finished weekly bars, built by adding up the daily ones,
while replay hands it a weekly bar that is still being built, growing day by day. The rule never
changes; only the moments at which it sees the world do.

## Why anyone believed it

A moving-average crossover is one of the oldest trading rules, and the story is simple. If prices
wander around a long-run level, an average over many days is a slow estimate of that level and an
average over a few days reacts faster. When the fast average pulls above the slow one, the price has
recently been stronger than its own history, and someone believes the strength will persist. The
counterparty is the trader anchored to the older price, who keeps selling into the rise and keeps
being wrong for long enough that the trend continues.

Even a reader who thinks the crossover is worthless should care about the machinery: a strategy that
rebalances before the close, or that watches a bar grow through the week, lives or dies on the
engine's clock. Getting the clock wrong by a single bar silently changes every average and every
signal downstream, so the compendium's designers wrote these backtests to pin the clock down.

## An everyday comparison

Think about how you would follow a football match. Read only the final score and you learn the result
and nothing about how it happened: that is a resampled bar, one number per match. Watch the live
scoreboard update minute by minute and you can react before the whistle: that is a replayed bar,
rebuilt as the week goes on. A referee's whistle blows at fixed minutes regardless of the score:
that is a timer. None of these changes the teams or the goals.

## The rules, step by step

1. Take one instrument and collect its daily closing prices, one row per trading day.
2. Compute a fast simple moving average over the last few days and a slow one over the last many
   days: 10 and 30 days for the timer test, 5 and 15 for the replay and resample tests.
3. Buy when the fast average crosses from below to above the slow average; a cross above means the
   fast value was at or below the slow value yesterday and strictly greater today.
4. Sell and go flat when it crosses back below. Nothing is ever sold short.
5. Size every order the same way: 10 units in these tests.
6. Decide the timing rule, which is the point of the category: a scheduled wake-up, finished weekly
   bars, or a weekly bar rebuilt every day.
7. Start with 100,000 in cash and no commission, and record the account value after the last bar.

### What is in this category

| Strategy              | What it does in one clause                                   | Source file                         |
| --------------------- | ------------------------------------------------------------ | ----------------------------------- |
| Timer scheduling      | Dual crossover plus a timer that fires at every session open | `test_62_timers.py`                 |
| pandas loading        | A DataFrame fed straight into the engine                     | `test_52_data_pandas.py`            |
| Resampling            | Daily bars aggregated into finished weekly bars              | `test_53_data_resample.py`          |
| Data replay           | A weekly bar that grows day by day as daily bars arrive      | `test_58_data_replay.py`            |
| Replay with Bollinger | A Bollinger breakout on replayed weekly bars                 | `test_118_data_replay_bollinger.py` |
| Replay with EMA       | An EMA crossover on replayed weekly bars                     | `test_119_data_replay_ema.py`       |
| Replay with MACD      | A MACD crossover on replayed weekly bars                     | `test_120_data_replay_macd.py`      |

### Timers (test_62)

The class is `TimerStrategy`, with `fast_period=10`, `slow_period=30` and `when=session start`. The
data is the repository's 2005 to 2006 daily file, whose sessions run from 09:00 to 17:30.

- Entry: when the 10-bar average crosses above the 30-bar average, close any holding and buy 10
  units in the same bar, so a reversal never leaves the account in two positions at once.
- Exit: when the 10-bar average crosses below the 30-bar average, close the holding. The strategy
  is long-only and never sells short.
- The timing twist: a timer registered with `add_timer(when=session_start)` calls the strategy at
  the open of every trading day; here the callback only counts itself, but it is where a real
  strategy would do its daily housekeeping.

### Data replay (test_58)

The class is `ReplayMAStrategy`, with `fast_period=5` and `slow_period=15`, the same long-only
crossover, run with `preload=False` so the engine must feed one bar at a time.

- Entry: when the 5-bar average crosses above the 15-bar average, buy 10 units.
- Exit: when the 5-bar average crosses below the 15-bar average, close the holding.
- The timing twist: the weekly bar is rebuilt as the week unfolds, so the averages are recomputed on
  every daily step and a crossover can fire in the middle of a week.

`SimpleMAStrategy` in `test_53_data_resample.py` uses the same 5 and 15 parameters but resamples the
daily file into finished weekly bars. Resampling shows the rule only the completed week; replay shows
it the week in progress, which is why the same rule makes 13 trades under replay and 3 under
resampling.

## The maths, with every symbol named

The simple moving average:

```text
SMA(n, t) = (P(t) + P(t-1) + ... + P(t-n+1)) / n
```

- `SMA(n, t)` is the simple moving average of period `n` at bar `t`.
- `P(t)` is the most recent closing price, `P(t-1)` the bar before it, and so on.
- `n` is the number of bars in the window: 5, 10, 15 or 30 in these tests.

It says: add the last `n` closes and divide by `n`, giving every bar the same weight.

The crossover signal:

```text
signal(t) = +1 if SMA(fast, t) > SMA(slow, t) and SMA(fast, t-1) <= SMA(slow, t-1)
signal(t) = -1 if SMA(fast, t) < SMA(slow, t) and SMA(fast, t-1) >= SMA(slow, t-1)
signal(t) =  0 otherwise
```

- `signal(t)` is the crossover reading on bar `t`: plus one for an upward cross, minus one for a
  downward cross, zero when there was no change of side.
- `SMA(fast, t)` is the short average and `SMA(slow, t)` the long one, both at bar `t`.

It says: the signal fires only on the first bar where fast moves to the other side of slow.

Building one weekly bar from the daily bars inside it:

```text
open_week   = the open of the first daily bar of the week
high_week   = the largest of the daily highs so far this week
low_week    = the smallest of the daily lows so far this week
close_week  = the close of the most recent daily bar
volume_week = the sum of the daily volumes
```

- `open_week` is fixed on Monday; the other four are the running high, low, latest close and total
  volume of the week so far.

It says: a weekly bar is a summary of the daily facts so far, so a rule that reads it early in the
week is reading a moving target that resampling hides.

## A worked example

### Worked example: timers

Ten made-up daily closes, a 2-day fast average and a 4-day slow average, to keep the arithmetic
small. The account starts with 10,000 and trades 10 units at a time.

| Day | Close | 2-day average | 4-day average | Signal     | Action         |
| --- | ----- | ------------- | ------------- | ---------- | -------------- |
| 1   | 100   | not formed    | not formed    | none       | none           |
| 2   | 102   | 101.0         | not formed    | none       | none           |
| 3   | 101   | 101.5         | not formed    | none       | none           |
| 4   | 104   | 102.5         | 101.75        | cross up   | buy 10 at 104  |
| 5   | 110   | 107.0         | 104.25        | none       | hold           |
| 6   | 112   | 111.0         | 106.75        | none       | hold           |
| 7   | 111   | 111.5         | 109.25        | none       | hold           |
| 8   | 109   | 110.0         | 110.5         | cross down | sell 10 at 109 |

The averages are the closes above and to the left: the 4-day average on day 7 is
`(104 + 110 + 112 + 111) / 4 = 109.25`. The buy on day 4 is the first day both averages exist and the
fast one is already above; the exit is a real crossing, fast above on day 7 and below on day 8.

The timer point is in the counts. The timer fires ten times, once at each session open, but `next()`
runs only from day 4 onward, seven times. The gap of three is the 4-day average's warm-up, the
history an indicator needs before it has a value. The real test records the same fact: 512 timer
firings against 482 strategy bars, a gap of 30, the slow average's warm-up.

The money: 10 units bought at 104.00 for 1,040.00 and sold at 109.00 for 1,090.00, a gross gain of
50.00. A spread of 0.05 per unit and a commission of 0.01 per unit per side are then deducted:

```text
spread cost     = 10 * 0.05        = 0.50
commission cost = 10 * 0.01 * 2    = 0.20
total cost      = 0.50 + 0.20      = 0.70
net result      = 50.00 - 0.70     = 49.30
final account   = 10,000.00 + 49.30 = 10,049.30
```

On the real 2005 to 2006 run, with the crossover on the 10 and 30 day averages, the nine complete
trades took 100,000.00 to 104,966.80, a gain of 4,966.80 before any spread or commission.

### Worked example: data replay

Build one weekly bar from five daily bars. Monday to Friday of the current week:

| Day       | Open  | High  | Low   | Close | Volume |
| --------- | ----- | ----- | ----- | ----- | ------ |
| Monday    | 104.0 | 106.0 | 103.0 | 105.0 | 1,000  |
| Tuesday   | 105.0 | 107.0 | 104.0 | 106.0 | 1,200  |
| Wednesday | 106.0 | 107.0 | 102.0 | 103.0 | 900    |
| Thursday  | 106.0 | 109.0 | 105.0 | 106.0 | 1,100  |
| Friday    | 106.0 | 110.0 | 105.0 | 109.0 | 1,500  |

The finished weekly bar has open 104.0 (Monday's open), high 110.0 (Friday's high), low 102.0
(Wednesday's low), close 109.0 (Friday's close) and volume 5,700 (the sum of the five daily
volumes). With the previous week's close of 104, a 2-week slow average through the week is:

| Day       | Week-so-far close | 2-week average          | Fast vs slow |
| --------- | ----------------- | ----------------------- | ------------ |
| Monday    | 105.0             | (104 + 105) / 2 = 104.5 | above        |
| Tuesday   | 106.0             | (104 + 106) / 2 = 105.0 | above        |
| Wednesday | 103.0             | (104 + 103) / 2 = 103.5 | below        |
| Thursday  | 106.0             | (104 + 106) / 2 = 105.0 | above        |
| Friday    | 109.0             | (104 + 109) / 2 = 106.5 | above        |

Suppose the week before ended with the fast average below the slow one. Under replay the strategy
buys on Monday's cross above, sells on Wednesday's cross back below, then buys again on Thursday.
Under resampling the only weekly close it ever sees is Friday's 109.0, so it buys once, at the end
of the week, and never sees the Monday or Wednesday turns.

That extra round trip is the cost replay can incur and resampling avoids. Buy 10 units on Monday at
105.00 for 1,050.00 and sell them on Wednesday at 103.00 for 1,030.00, a gross loss of 20.00, with
the same 0.05 spread and 0.01 commission per side:

```text
spread cost     = 10 * 0.05        = 0.50
commission cost = 10 * 0.01 * 2    = 0.20
total cost      = 0.50 + 0.20      = 0.70
net result      = -20.00 - 0.70    = -20.70
```

On the real 2005 to 2006 data the resample run, seeing 89 weekly bars, made 3 trades and finished at
100,765.01, while the replay run, advancing 439 times, made 13 trades and finished at 108,263.90.
The replay figure is not a claim that replay trades better; it records that the two clocks produced
different trades from the same daily prices.

## What the research actually found

This category contains no study of whether a moving-average crossover earns anything. Every backtest
in this compendium asserts its final account value, its reward-to-risk ratio and its worst fall
against a baseline, and these are assertions rather than evidence of an edge. The timer test asserts
a final value of 104,966.80 within one cent, a reward-to-risk ratio of 0.7210685207398165, an annual
return of 0.024145144571516192 and a worst fall of 3.430658473286522 percent over 9 trades. The
replay test asserts 108,263.90, a ratio of 1.17880670695321 and a worst fall of 2.668267546216064
percent over 13 trades. The resample test asserts 100,765.01, a ratio of 1.0787422654055023 and a
worst fall of 0.3038892199564355 percent over 3 trades, and the pandas test asserts 100,496.68 over
482 bars and 9 trades.

The honest reading is the compendium's own: passing these assertions proves only that the engine
computes what the file says, in both its vectorised and its event-driven modes, and that the two
agree. It does not prove the rule earns anything, because the tests compare against baselines inside
the same narrow data set and leave spreads and commission at zero.

## How this project relates to it

Two briefs in this repository's research collection show why the choice of clock changes the numbers.

[Stylized facts and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md) collects the
evidence that tail statistics belong to the sampling window, not the asset: excess kurtosis, a
measure of how often extreme days occur, is 7.49 for the DAX over 1959 to 2018, 19.95 for the Dow
over 1896 to 2018 and 32.50 for the S&P over 1789 to 2018, more than a factor of four apart on the
same measure, and Bitcoin's kurtosis falls from 107.5 at five-hour sampling to 53.0 at twelve-hour
sampling.

[Portfolio and allocation](../../../strategies/books2/10_portfolio_and_allocation.md) makes the same
point about return bookkeeping: net returns inflate the S&P 500 reward-to-risk ratio by nearly 30
percent, and compounding an arithmetic mean instead of the true geometric mean overstates the 1960
to 2020 terminal value by 89 percent. The bar you choose, and how you build and time it, changes
every statistic computed from it.

## Where it goes wrong

- The rule itself is unremarkable. A dual moving-average crossover has a thin published record and
  often loses after costs, so nothing here is evidence that it works.
- Zero commission is unrealistic: the tests set no commission and fill at the bar's price, so they
  ignore the spread and any fee, which the worked examples add.
- The clock can invent trades. Recomputing an indicator on an unfinished bar lets a crossover fire
  mid-week and then reverse, producing trades that never exist once the week is complete, and timers
  fire before the indicators have enough history, a gap of exactly 30 bars here.
- One instrument, one short sample, and engine checks rather than performance: all seven backtests
  read the same 2005 to 2006 file of roughly 500 bars, and their numbers only guarantee that both
  engine modes agree.

## Try it yourself

You need a spreadsheet and a public source of daily closing prices for one instrument, such as a
large index. Take 250 daily closes, about one year of trading days.

1. Column A: the date. Column B: the closing price. Column C: the 5-day average, from row six
   onward. Column D: the 15-day average, from row sixteen onward.
2. Column E: the signal. Put `+1` where column C is greater than column D and the row above had C
   below D; put `-1` where C is below D and the row above had C above D; leave it blank otherwise.
   The count of nonzero cells is the daily-bar signal count.
3. Build weekly bars from the same closes: one row per week, the week's first open as the open and
   the week's last close as the close, in a new column.
4. Compute the same 5-week and 15-week averages on the weekly closes and the same signal column,
   and count the nonzero cells again.
5. Compare the two counts.

What to notice: the daily file produces many more signals than the weekly file from the same prices,
because a crossing that appears and disappears inside a week shows up in step 2 and vanishes in
step 4. That is the resample-versus-replay difference in miniature.

## Where this came from

- [backtrader strategy compendium, category 21, time-based](https://backtrader.readthedocs.io/en/latest/strategies-series/en/21-time-based.html),
  the article that states the rules and the asserted numbers.
- [test_62_timers.py](https://raw.githubusercontent.com/cloudQuant/backtrader/development/tests/functional/strategies/time_based/test_62_timers.py)
  and [test_58_data_replay.py](https://raw.githubusercontent.com/cloudQuant/backtrader/development/tests/functional/strategies/time_based/test_58_data_replay.py),
  the timer and replay backtests.
- [test_53_data_resample.py](https://raw.githubusercontent.com/cloudQuant/backtrader/development/tests/functional/strategies/time_based/test_53_data_resample.py),
  the resampling backtest used as the comparison.
- [Stylized facts and scaling](../../../strategies/books2/13_stylized_facts_and_scaling.md) and
  [Portfolio and allocation](../../../strategies/books2/10_portfolio_and_allocation.md), the two
  briefs cited above.

## Words used in this tutorial

- bar: one row of price data, holding the open, high, low and close for one stretch of time.
- commission: a fee charged for placing a trade, separate from the price.
- crossover: the moment a short average moves from one side of a long average to the other.
- moving average: the average of the last few prices, recomputed on each new bar.
- replay: running the engine on daily bars while presenting them as a bigger bar that grows each day.
- resampling: combining several small bars into one finished larger bar, such as daily into weekly.
- spread: the gap between the price at which you can buy and the price at which you can sell.
- timer: a schedule in the engine that wakes the strategy at a fixed moment, such as the open.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
