# Breakout: buying when a price reaches a new high

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                      |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of one large American software company (Oracle) in the daily versions, and the price of glass and rebar futures (contracts to buy or sell a commodity at a set future date) on minute bars in the intraday versions |
| How often it trades       | Rarely in the daily versions, a few times a year; many times a day in the futures versions                                                                                                                                 |
| What you need             | A spreadsheet, and a data file of daily highs and lows                                                                                                                                                                     |
| Where the rules come from | [backtrader strategy compendium, No. 15, breakout](https://backtrader.readthedocs.io/en/latest/strategies-series/en/15-breakout.html)                                                                                      |
| The underlying research   | None: a practitioner's rule of thumb from the 1980s Turtle experiment, not a peer-reviewed study                                                                                                                           |
| How well it held up       | Weak, resting on a famous live experiment and a thin published record; the category's own bare Donchian backtest ends at a small loss                                                                                      |
| Also appears in           | [Dual Thrust trading algorithm](../../quantconnect/dual-thrust-trading-algorithm/README.md)                                                                                                                                |

## The idea in one paragraph

Pick a window of recent days, say twenty. Over that window, note the highest price the market
reached and the lowest. If today's price rises above the highest point of the window, buy, betting
that the new high marks the start of a climb. Sit still until the price falls below the lowest point
of the window, then sell. The intraday versions do the same thing inside one trading day, drawing a
line above and a line below the day's opening price and trading whenever the price touches one of
them. The whole family rests on one belief: a price that just left its range tends to keep going.

## Why anyone believed it

The belief was made famous by an experiment in the 1980s, when Richard Dennis and William Eckhardt
taught a group of novices nicknamed the Turtles a rule that bought new highs and sold new lows; the
source article reports the group averaged about eighty percent a year in its first years.

Any strategy must say who takes the other side. Someone sells to the buyer at a new high: a trader
anchored to the old range, a fund raising cash, or a latecomer who notices the move too late. If
those sellers keep appearing, the price keeps going a little further than anyone expects.

## An everyday comparison

A shop sold apples between 1.00 and 1.20 for a month. One morning it posts 1.35 and sells out, so
buyers used to the old range take two bags instead of one. The breakout is that first unusual high.

## The rules, step by step

The shared mechanism, which the named variants change only in window, instrument and clock:

1. Gather daily open, high, low and close (and volume for one variant); the channel uses the high
   and the low.
2. Build the channel: the upper line is the highest high of the last N days, the lower line the
   lowest low.
3. If you hold nothing and today's close is strictly above yesterday's upper line, buy; use
   yesterday's line so today's own high cannot push the line up and hide the breakout.
4. If you hold shares and today's close is below yesterday's lower line, sell everything and wait
   for the next signal.
5. Review once a day, at the close; the classic version buys ten shares each time it enters.

### What is in this category

| Name                | What it does in one clause                              | Source file                           |
| ------------------- | ------------------------------------------------------- | ------------------------------------- |
| Donchian classic    | Enter on a 20-day high, exit on a 20-day low            | test_105_donchian_channel_strategy.py |
| Donchian backhacker | The same idea with a different parameter pair           | test_66_donchian_channel_strategy.py  |
| Dual Thrust         | Intraday glass-futures bands anchored at the day's open | test_09_dual_thrust_strategy.py       |
| R-Breaker           | Six rebar-futures pivot levels, trend plus reversal     | test_10_r_breaker_strategy.py         |
| Volume breakout     | A breakout that must also show a jump in volume         | test_115_volume_breakout_strategy.py  |
| Price channel       | Enter on a 20-day high, exit on a 10-day low            | test_117_price_channel_strategy.py    |

### Donchian channel

The exact rules, from the class `DonchianChannelStrategy` with `stake=10` and `period=20`:

- Buy: when flat, if `close[0] > highest[-1]`, the highest high of the last twenty bars; strict, and
  using the value from the bar before.
- Sell: when holding, if `close[0] < lowest[-1]`, the lowest low of the last twenty bars; also
  strict, also the previous bar's value.
- Size: ten shares per entry, one position at a time.

### Dual Thrust

The exact rules, from the class `DualThrustStrategy` with `look_back_days=10`, `k1=0.5`, `k2=0.5`:

- Range: the larger of HH minus LC and HC minus LL, where HH is the highest high, LL the lowest low,
  HC the highest close and LC the lowest close, over the last ten sessions.
- Lines: upper equals today's open plus 0.5 times the range; lower equals today's open minus 0.5
  times the range.
- Entry: during 21:00 to 23:00 or 09:00 to 11:00, buy one contract when the minute close is above
  the upper line, or sell one when it is below the lower line.
- Reversal: a long (a bet that the price rises) below the lower line is closed and a short (a bet
  the price falls) opened; the mirror image above the upper line reverses a short back to a long.
- Exit: any open position is flattened at 14:55, before the day settles.

### R-Breaker

The exact rules, from the class `RBreakerStrategy` with `k1=0.5`, `k2=0.5`:

- From yesterday's high H, low L and close C, compute pivot, R1, R3, S1 and S3 (formulas below).
- Trend mode: during 21:00 to 23:00 or 09:00 to 11:00, from flat, a minute close above R3 buys one
  contract and a close below S3 sells one.
- Reversal mode: a long that falls back through R1 is closed and reversed to a short; a short that
  rises through S1 is closed and reversed to a long.
- Exit: everything is flattened at 14:55.

## The maths, with every symbol named

First the channel:

```text
upper = the highest high over the last N days
lower = the lowest low over the last N days
```

- `upper` and `lower` are the channel's top and bottom, in price units.
- `N` is the window length in days: 20 in the classic test, 10 in the backhacker.

A close above `upper` or below `lower` is a breakout.

Second, the Dual Thrust range and bands for one session:

```text
range = max(HH - LC, HC - LL)
upper = open_today + k1 * range
lower = open_today - k2 * range
```

- `HH` and `HC` are the last N sessions' highest high and highest close; `LL` and `LC` the lowest.
- `open_today` is the first price of the session; `k1` and `k2` are the multipliers, both 0.5.

The wider range sets the band, so a jumpy market gets wider lines and fewer false touches.

Third, the R-Breaker ladder from yesterday:

```text
pivot = (H + L + C) / 3
r1 = pivot + k1 * (H - L)
r3 = pivot + (k1 + k2) * (H - L)
s1 = pivot - k1 * (H - L)
s3 = pivot - (k1 + k2) * (H - L)
```

- `H`, `L` and `C` are yesterday's high, low and close; `k1` and `k2` are the spacings, both 0.5.

R3 and S3 sit two steps from the pivot and mark strong breakouts; R1 and S1 sit one step away and
mark where a position is abandoned.

## A worked example

### Donchian channel

Ten shares on the 20-day channel; each line shown is yesterday's value, what the rule compares to.

| Day | Upper line (yesterday) | Lower line (yesterday) | Close | Action           |
| --- | ---------------------- | ---------------------- | ----- | ---------------- |
| 1   | 42.00                  | 38.00                  | 42.50 | Buy 10 at 42.50  |
| 2   | 42.50                  | 38.20                  | 43.10 | Hold             |
| 3   | 43.10                  | 38.60                  | 43.60 | Hold             |
| 4   | 43.60                  | 39.10                  | 42.90 | Hold             |
| 5   | 43.60                  | 39.50                  | 39.20 | Sell 10 at 39.20 |

The buying price is 42.50 and the selling price is 39.20, so the price moved against the position by
3.30 a share:

```text
value change = 10 * (39.20 - 42.50) = 10 * (-3.30) = -33.00
buy commission  = 425.00 * 0.001 = 0.43
sell commission = 392.00 * 0.001 = 0.39
total costs = 0.82
net result = -33.00 - 0.82 = -33.82
```

Thirty-three dollars and eighty-two cents lost on a 100,000 account. Only the 0.1 percent commission
the file sets is modelled; a real spread between buying and selling would make the loss slightly
larger, the same shape as the small loss the category's own Donchian file reports over five years.

### Dual Thrust

The ten-session range for this made-up day: highest high 1500, lowest close 1420, highest close
1480, lowest low 1390. The range is the larger of 80 and 90, so 90. The session opens at 1455,
giving an upper band of 1500 and a lower band of 1410.

| Minute | Close | Upper | Lower | Action                             |
| ------ | ----- | ----- | ----- | ---------------------------------- |
| 1      | 1480  | 1500  | 1410  | Between the bands, nothing         |
| 2      | 1508  | 1500  | 1410  | Above upper, buy 1                 |
| 3      | 1490  | 1500  | 1410  | Long, hold                         |
| 4      | 1408  | 1500  | 1410  | Below lower, close long and sell 1 |
| 5      | 1400  | 1500  | 1410  | Flatten the short at 14:55         |

The contract multiplier is 20 and commission is 26 per contract per side. The long runs from 1508
down to 1408; the short runs from 1408 up to 1400:

```text
long  = (1408 - 1508) * 20 = -2000
short = (1408 - 1400) * 20 = +160
gross = -2000 + 160 = -1840
commissions = 4 fills * 26 = 104
net result = -1840 - 104 = -1944
```

One thousand nine hundred and forty-four lost on the day, on a 50,000 account, with the spread
between where the band was crossed and where the fill landed left out.

### R-Breaker

Yesterday: high 3700, low 3600, close 3660. The pivot is 10960 / 3, or 3653.33; the levels step out
by half the 100-point range: R1 3703.33, R3 3753.33, S1 3603.33, S3 3553.33.

| Minute | Close | Action                          |
| ------ | ----- | ------------------------------- |
| 1      | 3730  | Below R3, nothing               |
| 2      | 3760  | Above R3, buy 1                 |
| 3      | 3720  | Long, hold                      |
| 4      | 3695  | Below R1, close long and sell 1 |
| 5      | 3700  | Flatten the short at 14:55      |

The contract multiplier is 10, and commission is 0.0003 of the traded value, the price times the
multiplier:

```text
long  = (3695 - 3760) * 10 = -650
short = (3695 - 3700) * 10 = -50
gross = -650 - 50 = -700
commissions = 37600 * 0.0003 + 36950 * 0.0003 + 36950 * 0.0003 + 37000 * 0.0003
            = 11.28 + 11.09 + 11.09 + 11.10 = 44.56
net result = -700 - 44.56 = -744.56
```

Seven hundred and forty-four dollars and fifty-six cents lost on the day, on a 50,000 account,
because the reversal bought high and sold low; the spread is again left out.

## What the research actually found

Every backtest here asserts three things: the final account value, the reward-to-risk ratio (the
Sharpe ratio, the average return divided by how much it wobbled) and the worst fall from a peak (the
maximum drawdown, in percent). Each is compared against a hard-coded expected number, the final value
to within one cent and the others to within a millionth. Passing the assertion proves only that the
engine computes what the file says it should, not that the strategy earns anything.

| Test                       | Data                      | Period                                | Start   | Final value | Reward-to-risk | Worst fall    |
| -------------------------- | ------------------------- | ------------------------------------- | ------- | ----------- | -------------- | ------------- |
| Donchian (test_105)        | Oracle daily              | 2010-2014, 1238 bars                  | 100,000 | 99,965.62   | -0.188         | 14.87 percent |
| Price channel (test_117)   | Oracle daily              | 2010-2014, 1238 bars                  | 100,000 | 100,050.36  | 0.559          | 6.63 percent  |
| Volume breakout (test_115) | Oracle daily              | 2010-2014, 1238 bars                  | 100,000 | 99,987.80   | -0.155         | 5.24 percent  |
| Dual Thrust (test_09)      | Glass futures minute bars | 2020-01-01 to 2020-06-30, 32,220 bars | 50,000  | 48,300.00   | -2.288         | 4.44 percent  |
| R-Breaker (test_10)        | Rebar futures minute bars | last 20,000 bars                      | 50,000  | 34,144.78   | -11.346        | 36.63 percent |

The two daily Oracle tests and the volume variant move the account by a few dollars up or down over
five years, essentially noise. The R-Breaker test is the loudest: 322 trades wearing the account
down by about sixteen thousand on a 50,000 start, which is what a reward-to-risk of minus eleven
looks like when a reversal system buys every failed breakout. These are the file's own measurements;
the article says the regression library treats strategies as things to be compared, not performed.

## How this project relates to it

Two briefs in this repository bracket what a breakout trade must overcome. The first,
[Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
reports that the average price impact of a large order follows a concave power law,
`I/sigma = c * (Q/VD)^delta`, where `delta` measures how much less than proportionally the price
bends as the order grows; it cites a Tokyo Stock Exchange survey with a mean `delta` of 0.489
(standard error 0.0015) over 2,299 stock-datapoints, and a prefactor `c` range of roughly 0.3 to
1.0. The second,
[Energy and commodities](../../../strategies/books2/18_energy_and_commodities.md), reports that a
commodity curve slope-continuation strategy earned an annualised mean excess return of 1.77 percent
(t-statistic 7.23, reward-to-risk 1.41) before costs, but only 0.55 to 1.31 percent a year after
three cost scenarios, with its net reward-to-risk falling from 1.74 to 0.71 after 2000. Together
they say the price move a breakout chases must exceed the impact the trade itself creates, and the
costs of trading intraday are precisely the drag that can erase a before-cost edge.

## Where it goes wrong

- The trade moves the price. Every entry is a buy into a rising market and every exit a sell into a
  falling one, so the order shrinks the very move it is trying to catch, in proportion to its size.
- Costs on a fast clock. The daily versions trade a few times a year and the drag is small; the
  intraday ones trade many times a day, and the R-Breaker file shows 322 trades, so commissions and
  the spread between touch and fill mount up quickly.
- False breakouts in a quiet range. When a market is not trending, a new high is often just the edge
  of a sideways shuffle; the price pokes above the line, the system buys, the price falls back, and
  the system sells. The bare Donchian test is exactly this picture.
- The famous sample is a story, not a replication. The Turtle returns were a live account in one
  decade, run by a small group, with no independent out-of-sample test.

## Try it yourself

You need nothing but a spreadsheet and any daily price series with highs and lows, such as a share
price you can download from a finance website.

1. Put the dates down one column, and the open, high, low and close in the next four.
2. In a fifth column compute the highest high of the last twenty rows, and in a sixth the lowest low;
   shift both down one row so each day sees yesterday's values.
3. In a seventh column write "breakout up" when the close is above that shifted high, and "breakout
   down" when it is below the shifted low.
4. For each "breakout up" day, look at where the price was five days later and count how often it was
   still higher; do the same for the "breakout down" days.

What to notice: on a trending series the breakouts are followed by more moves in the same direction
more often than not; on a sideways series the two counts come out close to even, and once you
subtract the cost of each buy and each sell the exercise stops showing a gain.

## Where this came from

- [backtrader strategy compendium, No. 15, breakout](https://backtrader.readthedocs.io/en/latest/strategies-series/en/15-breakout.html),
  the article that states the category's rules and reports the Donchian result.
- [test_105_donchian_channel_strategy.py](https://github.com/cloudQuant/backtrader/blob/development/tests/functional/strategies/breakout/test_105_donchian_channel_strategy.py),
  the Donchian channel class and its asserted backtest values.
- [test_09_dual_thrust_strategy.py](https://github.com/cloudQuant/backtrader/blob/development/tests/functional/strategies/breakout/test_09_dual_thrust_strategy.py),
  the Dual Thrust class, its bands and its asserted backtest values.
- [test_10_r_breaker_strategy.py](https://github.com/cloudQuant/backtrader/blob/development/tests/functional/strategies/breakout/test_10_r_breaker_strategy.py),
  the R-Breaker class, its pivot ladder and its asserted backtest values.
- [test_117_price_channel_strategy.py](https://github.com/cloudQuant/backtrader/blob/development/tests/functional/strategies/breakout/test_117_price_channel_strategy.py)
- [test_115_volume_breakout_strategy.py](https://github.com/cloudQuant/backtrader/blob/development/tests/functional/strategies/breakout/test_115_volume_breakout_strategy.py)
- [Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  the market-impact brief on how a large order moves the price it is trying to buy.
- [Energy and commodities](../../../strategies/books2/18_energy_and_commodities.md), the brief on
  commodity curve strategies and their cost sensitivity.

## Words used in this tutorial

- breakout: a price moving beyond the edge of its recent range, in either direction.
- commission: the fee a broker charges per trade, here a fraction of the value traded.
- drawdown: the fall from a peak to the following low, in percent; the worst one is the maximum.
- long: owning something, so a rise in its price makes money and a fall loses money.
- short: selling something borrowed, so a fall in its price makes money and a rise loses money.
- spread: the gap between the price at which you can buy and the price at which you can sell.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
