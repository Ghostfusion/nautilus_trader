# Trading only when the market is moving strongly in one direction

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                 |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One cryptocurrency pair on one exchange, bought and held as a single position, and on the futures file also sold short                                                                                                                                                |
| How often it trades       | A few times a month on hourly candles, because the trend filter rejects most of the time                                                                                                                                                                              |
| What you need             | A spreadsheet                                                                                                                                                                                                                                                         |
| Where the rules come from | [ADXMomentum.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/ADXMomentum.py) and [FAdxSmaStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/FAdxSmaStrategy.py) |
| The underlying research   | J. Welles Wilder, [New Concepts in Technical Trading Systems](https://archive.org/details/newconceptsintec00wild), 1978, which introduced the index and the directional measures                                                                                      |
| How well it held up       | Weak: the index itself is a well-known description of the recent past, but no independent study of this rule as coded has been published, and technical-trading results generally weakened once transaction costs and out-of-sample periods were included             |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                                       |

## The idea in one paragraph

Some of the time a price drifts sideways and the next move is a coin toss; the rest of the time it
marches in one direction and each new hour is more likely than not to continue. The average
directional index tries to measure which of those two states the market is in, on a scale where a
higher number means a stronger, more one-sided move. It deliberately does not say which way the
market is going. A separate pair of measures, the plus and minus directional indicators, gives the
direction. The two source files use the index as a permission switch: trade only when the market is
moving strongly, and let a normal rule decide which way.

## Why anyone believed it

A price move that has gathered momentum keeps going for a simple reason: the people who have not
yet acted are still acting. When an exchange listing, a regulatory decision or a large buyer pushes
a price up, the news travels to different people at different times, so the buying is spread over
days rather than happening in one instant. Money also follows money; a coin that is visibly rising
appears on more lists and attracts more attention.

The counterparty is the trader who is slow, or who fades the move by selling into it because the
price "looks too high". If enough people do that, the mover keeps climbing and each sale is filled
at a price the seller later regrets. The index is an attempt to detect the stretch when this
one-sidedness is present, so that a normal trend rule is only run when it has a chance.

## An everyday comparison

A speedometer and a compass answer different questions about a car. The speedometer says how fast
you are going and says nothing about whether you are heading north or south. The average
directional index is a speedometer for prices: a high reading means the price is covering ground
decisively, whether that ground is up or down. The directional indicators are the compass. The
source files refuse to set off at all unless the speedometer reads above a threshold, and only then
consult the compass, which is why they sit in cash through long, drifting stretches.

## The rules, step by step

1. Pick one cryptocurrency pair on one exchange, such as a Bitcoin pair against a stablecoin.
2. Use hourly candles. Both files read one hour at a time.
3. Compute the trend measures. The index is built from the true range, which is the largest of three
   distances on each candle, and from two directional movements, one counting upward pushes and one
   counting downward pushes. Those are combined into a plus directional indicator and a minus
   directional indicator, and the difference between them, smoothed, becomes the average
   directional index. The formulas are in the next section.
4. Buy when the index and the directional indicators agree that the market is trending upward, and
   only then. ADXMomentum requires the 14-period index above 25, the 14-period momentum measure above
   zero, the 25-period plus directional indicator above 25, and that indicator above the minus one.
   FAdxSmaStrategy requires the 14-period index above 30 and the 12-period simple moving average
   crossing above the 48-period one.
5. Sell, or for the futures file sell short, when the permission is withdrawn. ADXMomentum exits when
   the index is still above 25 but momentum is below zero and the minus directional indicator is
   above 25 and above the plus one. FAdxSmaStrategy exits as soon as the index falls below 30, and
   enters a short position when the short average crosses below the long one while the index is above
   30.
6. The stored settings are these. ADXMomentum: `timeframe = '1h'`, `stoploss = -0.25`,
   `minimal_roi = {"0": 0.01}`, and `startup_candle_count = 20`. FAdxSmaStrategy:
   `timeframe = "1h"`, `stoploss = -0.05`, `can_short = True`, `trailing_stop = False`,
   `startup_candle_count = 14`, and `minimal_roi = {"0": 0.05, "30": 0.1, "60": 0.075}`, where the
   keys are minutes held: 5 percent immediately, 10 percent after half an hour and 7.5 percent after
   an hour.

Two details are worth stating exactly because they look like errors. FAdxSmaStrategy defines a
parameter called `pos_exit_adx`, set to 30, and never uses it; the exit compares the index with the
entry parameter instead, which happens to be 30 as well. And its profit ladder is not monotonic: the
target rises from 5 to 10 percent between zero and thirty minutes and then falls to 7.5 percent,
which is the opposite of the usual shape of trading a position down over time.

## The maths, with every symbol named

The true range is how far the price actually travelled, counting gaps.

```text
TR_t = largest of (H_t - L_t), |H_t - C_(t-1)|, |L_t - C_(t-1)|
```

- `H_t` and `L_t` are candle `t`'s high and low.
- `C_(t-1)` is the previous candle's close, so a gap up or down is included.
- `| |` means the value without its sign.

The directional movements count pushes, and only one of them can be positive on a candle.

```text
up_t   = H_t - H_(t-1)
down_t = L_(t-1) - L_t
plus_dm_t  = up_t   if up_t > down_t and up_t > 0, otherwise 0
minus_dm_t = down_t if down_t > up_t and down_t > 0, otherwise 0
```

- `up_t` is how much higher this candle's high is than the previous high.
- `down_t` is how much lower this candle's low is than the previous low.
- `plus_dm_t` and `minus_dm_t` are the two directional movements, with the smaller one set to zero.

Divide the smoothed sums and you get the directional indicators and the index.

```text
plus_di  = 100 * smoothed_sum(plus_dm)  / smoothed_sum(TR)
minus_di = 100 * smoothed_sum(minus_dm) / smoothed_sum(TR)
DX       = 100 * |plus_di - minus_di| / (plus_di + minus_di)
ADX      = smoothed average of DX over 14 candles
```

- `smoothed_sum` is Wilder's running total, which nudges the previous total rather than using a
  plain sum; the files let the indicator library do this and use 14 candles.
- `DX` is one candle's disagreement between the two directions; it is zero when they are equal and
  100 when only one is active.
- `ADX` averages `DX`, so it rises only when the one-sidedness persists; it is slow on purpose.

The trend permission on the other file is a moving-average crossing:

```text
SMA_short = average of the last 12 closes
SMA_long  = average of the last 48 closes
buy when SMA_short crosses from below to above SMA_long, having been below it on the previous candle
```

- The 12 and 48 are the file's stored `sma_short_period` and `sma_long_period`.
- A crossing means the two lines swapped order, not merely that one is above the other.

## A worked example

Six candles of made-up but plausible hourly data, rising from 101 towards 109 with one dip, using a
simple sum in place of Wilder's smoothing and a single `DX` reading in place of the 14-candle
average, so the arithmetic fits on the page.

| Candle | High   | Low    | Close  | True range | Plus movement | Minus movement |
| ------ | ------ | ------ | ------ | ---------- | ------------- | -------------- |
| 1      | 102.00 | 99.00  | 101.00 |            |               |                |
| 2      | 104.00 | 100.00 | 103.00 | 4.00       | 2.00          | 0.00           |
| 3      | 106.00 | 103.00 | 105.00 | 3.00       | 2.00          | 0.00           |
| 4      | 108.00 | 105.00 | 107.00 | 3.00       | 2.00          | 0.00           |
| 5      | 109.00 | 104.00 | 105.00 | 5.00       | 0.00          | 0.00           |
| 6      | 108.00 | 103.00 | 104.00 | 5.00       | 0.00          | 1.00           |
| 7      | 110.00 | 104.00 | 109.00 | 6.00       | 2.00          | 0.00           |
| Total  |        |        |        | 26.00      | 8.00          | 1.00           |

The sums are over candles 2 to 7. So `plus_di = 100 * 8.00 / 26.00 = 30.77` and
`minus_di = 100 * 1.00 / 26.00 = 3.85`. The disagreement is
`DX = 100 * |30.77 - 3.85| / (30.77 + 3.85) = 77.76`, and with one reading the index equals it.
That is well above both 25 and 30, so the permission is granted.

Now the momentum and the average crossing at candle 7, using a 2-candle average against a 4-candle
one so that the sum is short enough to check. `MOM` is the close 14 candles ago subtracted from
today's close; with only seven candles, the file's period cannot be used and the whole-run difference
is 109.00 - 101.00 = +8.00.

| Candle | Close  | Short average (2) | Long average (4) | Short above long |
| ------ | ------ | ----------------- | ---------------- | ---------------- |
| 5      | 105.00 | 106.00            | 105.00           | yes              |
| 6      | 104.00 | 104.50            | 105.25           | no               |
| 7      | 109.00 | 106.50            | 106.25           | yes              |

At candle 6 the short average is below the long one and at candle 7 it is above, so it crossed up,
which is the entry for the trend-permission file. At the same candle the index is above 25, momentum
is positive and the plus indicator is above the minus one, which is the entry for ADXMomentum.

Both would buy at 109.00. ADXMomentum's target is 1 percent, so it sells at `109.00 * 1.01 = 110.09`
if that price arrives before the stop, and its stop would accept a 25 percent loss.

| Item                             | Arithmetic          | Result          |
| -------------------------------- | ------------------- | --------------- |
| Buy at 109.00 with 0.10 pct fee  | 109.00 * 1.001      | 109.11 paid     |
| Sell at 110.09 with 0.10 pct fee | 110.09 * 0.999      | 109.98 received |
| Return                           | 109.98 / 109.11 - 1 | +0.80 percent   |

The example also shows the cost of the filter. On candles 5 and 6 the index and the directional
measures were still strong but the average pair had crossed down, so no purchase happened; a rule
without the trend permission would have bought at candle 5 and been wrong on candle 6.

## What the research actually found

| Source                                                            | What it measured                                                              | Result                                                                                                                                             |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Wilder, New Concepts in Technical Trading Systems, 1978           | The author's own accounts, no stated sample, no costs                         | Introduced the index and the directional measures as a trend-following system; the claim is the author's, and the book reports no independent test |
| Park and Irwin, The Profitability of Technical Analysis: A Review | 95 modern studies of technical rules                                          | Roughly half found positive results before costs, but the studies are uneven, and results were weaker in the years after 1990                      |
| Sullivan, Timmermann and White, Journal of Finance, 1999          | A bootstrap test of the classic moving-average rules on the Dow, 1897 to 1996 | The in-sample results survived their correction for data mining, but a fresh ten-year sample from 1987 to 1996 did not confirm them                |
| This repository's brief on predictability                         | The cross-sectional and technical-analysis literature                         | Concludes that most published predictability is real but small, and that decay and data leakage are the binding constraints                        |

There is no published study of either file as coded, on crypto pairs, with fees. The index is a
legitimate description of what recently happened; whether it predicts what happens next, often
enough to pay 0.05 to 0.10 percent per side, is untested here.

## How this project relates to it

The repository's brief on
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
is the closest thing to a verdict: it finds that much published predictability is genuine but that
the decay of an edge and the leakage of information from the test are the constraints that matter,
which is exactly the risk in a filter such as this one.

The brief on
[Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md)
measures what each order costs, and the brief on
[Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
covers the hyperopt parameter searches that produced the numbers in both source files.

## Where it goes wrong

- The measure is late. The index is an average of an average, so it turns up only after the trend is
  already visible, and a threshold of 25 or 30 rejects the early part of almost every move. The
  strategy buys the middle of a trend, when the return for the risk is often at its worst.
- Strong trends reverse hardest. A market that has moved decisively in one direction is the market
  most likely to snap back, and the stop of 25 percent in one file allows the position to give back a
  quarter of its value first.
- Shorting has a running cost. The futures file can sell short, and a short position in a
  perpetual futures contract pays a funding fee when the crowd is positioned the other way, so the
  position bleeds even when the price does not move. That cost is not in the file.
- Dead and odd settings. One file computes a parabolic stop it never uses, and defines an exit
  threshold it never reads; its profit ladder rises for half an hour before falling. Those are signs
  the file was edited quickly and not tested as shipped.
- The parameters are searches. The default thresholds, periods and stop percentages came from
  looking at one author's sample, not from a published test, and the freqtrade project's own README
  says the files are starting points rather than strategies to trade.
- It is a filter, not a reason. Even if the index correctly labels the market as trending, the rule
  still needs the price to keep moving after the entry, and nothing in the setup arranges that.

## Try it yourself

You need a spreadsheet and hourly high, low and close prices for one coin for a year.

1. Column A: the hour. Columns B, C and D: the high, low and close.
2. Column E: the true range, which is the largest of the high minus the low, the high minus the
   previous close, and the low minus the previous close, each taken without a sign.
3. Column F: the upward push, which is today's high minus yesterday's high.
4. Column G: the downward push, which is yesterday's low minus today's low.
5. Column H: the plus movement, which is column F when it is positive and larger than column G, and
   zero otherwise. Column I: the minus movement on the same test applied to column G.
6. Column J: the plus indicator, which is 100 times the average of column H divided by the average of
   column E over the last 14 rows. Column K: the same with column I.
7. Column L: the index, which is 100 times the absolute difference between columns J and K divided by
   their sum, averaged over 14 rows.
8. Column M: a buy flag that says yes when column L is above 25 and column J is above column K.

What to notice: for long stretches column L sits below 25 and the flags in column M are all blank,
even though the price is moving. Then count how often the flags appear near the top of a rise rather
than the start of one. That delay is the whole question about using the index as a permission
switch, and you can settle it for one coin and one year without risking any money.

## Where this came from

- [ADXMomentum.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/ADXMomentum.py),
  the hourly rule with the 25 thresholds and the 1 percent target.
- [FAdxSmaStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/FAdxSmaStrategy.py),
  the futures rule with the moving-average crossing and the 5 percent stop.
- J. Welles Wilder, [New Concepts in Technical Trading Systems](https://archive.org/details/newconceptsintec00wild),
  1978, the original description of the true range, the directional movements and the index.
- Cheol-Ho Park and Scott H. Irwin, [The Profitability of Technical Analysis: A Review](https://farmdoc.illinois.edu/assets/marketing/agmas/AgMAS04_04.pdf),
  the survey of modern studies quoted above.
- Sullivan, Timmermann and White, Data-Snooping, Technical Trading Rule Performance, and the
  Bootstrap, Journal of Finance, 1999, reported in the Fang, Jacobsen and Qin paper cited in the
  band tutorial, for the out-of-sample failure of the classic moving-average rules.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on how much of the measured predictability survives.

## Words used in this tutorial

- average directional index: a 0 to 100 measure of how one-sided recent price moves have been,
  regardless of direction.
- directional indicator: one of a pair of measures, the plus one counting upward pushes and the
  minus one counting downward pushes.
- moving average: the mean of the last N closing prices, recomputed after each new close.
- short selling: borrowing something you do not own, selling it, and buying it back later; a profit
  if the price falls.
- stop loss: a standing instruction to close a position once it has lost a set percentage.
- true range: how far the price actually travelled during a candle, including any gap from the
  previous close.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
