# Trading the slow line: momentum in the smoothed part of a currency price

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | One currency pair at a time, such as the euro against the US dollar, as a bet on its direction                                                                                                                                                               |
| How often it trades       | Rarely; between ten and fifteen times over six and a half years in the test described below                                                                                                                                                                  |
| What you need             | A spreadsheet and several years of daily currency prices                                                                                                                                                                                                     |
| Where the rules come from | [QuantConnect strategy library, momentum based on the low frequency component of forex](https://www.quantconnect.com/tutorials/strategy-library/the-momentum-strategy-based-on-the-low-frequency-Component-of-forex-market)                                  |
| The underlying research   | Harris and Yilmaz, [A momentum trading strategy based on the low frequency component of the exchange rate](https://www.sciencedirect.com/science/article/pii/S0378426609000521), Journal of Banking and Finance, 2009                                        |
| How well it held up       | Weak: the authors report that the rule is robust and beats simple moving averages, while the only out-of-sample application found here returned near zero or negative on most pairs and the page itself reports that the result is sensitive to its settings |
| Also appears in           | [Telling which kind of market you are in](../../project/measuring-the-regime/README.md) in the project tutorials, which is about the same problem of separating a real trend from noise                                                                      |

## The idea in one paragraph

A currency price wiggles every day, but underneath the wiggle there is often a slower drift. This
strategy separates the price into a slow part and a fast part, using a smoothing method that draws a
gentle line through the price while ignoring the daily noise. It then watches only the slow line. When
the slow line turns upward, it buys the currency; when the slow line turns downward, it sells it short.
The bet is that a drift big enough to move the slow line keeps going long enough to pay for the trade.
The wiggle is treated as noise, not as a signal.

## Why anyone believed it

A currency's value is pushed around by interest rates, trade flows, central-bank policy and the mood
of the market. None of those forces change from one day to the next. A central bank that raises rates
usually does so over months, and the money that follows the higher rate arrives gradually. So there is
a reason to expect a slow, persistent drift in a currency's value, with a large amount of day-to-day
noise sitting on top of it.

The counterparty is anyone forced to trade at a bad moment, such as a company converting receipts at a
fixed date, or a trader who reacts to each day's headline. If those flows arrive in roughly the same
direction over weeks, the slow drift continues and whoever follows it is paid for waiting. Simple
moving-average rules try to catch the same drift, but they react late, because their average includes
old prices; smoothing differently is meant to react sooner.

## An everyday comparison

Think of watching a boat cross a large lake on a windy day. The boat's path, drawn minute by minute,
zigzags as gusts push it about. But the general direction of travel is obvious if you step back and
look at where it has got to over an hour: it has crossed the middle of the lake and is heading for the
far shore. This strategy ignores the zigzags and steers by the hour-long trend. It sets off once the
slow direction is clear, rather than trying to guess each gust.

## The rules, step by step

1. Choose one currency pair, such as the euro price of one US dollar, and collect its daily closing
   price for at least five years.
2. Apply the Hodrick-Prescott filter, a smoothing method, to the whole price series. The filter draws
   a slow line through the prices, and the difference between the price and the slow line is the fast
   part. The library page uses a smoothing setting of 100, which it labels lambda.
3. Update the filter each day with the new price, over a rolling window of the last five years, so the
   slow line is always drawn through the most recent data.
4. On the slow line, compute the moving-average rule `MA(1,2)`, which compares the newest value of the
   slow line with the average of the newest two values.
5. If that comparison turns from negative to positive, buy the pair and hold it. If it turns from
   positive to negative, sell the pair short and hold it.
6. Do nothing on any day when the comparison keeps the same sign as the day before. Hold the current
   position until the sign flips.
7. Repeat the whole calculation every day.

The library page calls the comparison `MA(1,2)` and the paper uses the same shorthand. Written out, a
moving-average rule with parameters `m` and `n` takes the average of the newest `m` values and
subtracts the average of the newest `n` values. With `m = 1` and `n = 2`, it takes the newest value
and subtracts the average of the newest two, so the signal is positive exactly when the newest value
is higher than the one before.

## The maths, with every symbol named

The Hodrick-Prescott filter splits the price into a slow part and a fast part:

```text
y_t = x_t + c_t
```

- `y_t` is the price observed on day `t`.
- `x_t` is the slow part, the trend, which is what the strategy trades on.
- `c_t` is the fast part, the remainder, which is discarded as noise.

The slow part is chosen as the one that best balances two goals at once, closeness to the price and
smoothness of the line:

```text
minimize over x:  sum( (y_t - x_t)^2 ) + lambda * sum( ( (x_{t+1} - x_t) - (x_t - x_{t-1}) )^2 )
```

- The first sum, `sum( (y_t - x_t)^2 )`, is small when the slow line stays close to the actual prices.
- The second sum measures how much the line bends from day to day. It is small when the line is
  straight.
- `lambda` sets the trade-off between the two. A large `lambda` makes a straighter, smoother line that
  ignores the price; a small `lambda` makes the line follow the price more closely, and at the extreme
  of zero it is the price itself.
- `minimize over x` means: choose the whole slow line, one value for every day, so that the combined
  total is as small as possible.
- `t` runs over the days in the window.

The library page explains that a common formula for `lambda` gives an enormous number for daily data,
which produces an almost perfectly straight line and is useless for trading, so it lowers the setting
until the line bends visibly but slowly, and settles on `lambda = 100`. That choice is a judgement, not
a derived result, and it is the reason the page warns that the result depends on it.

The signal is the moving-average rule applied to the slow line rather than to the price:

```text
MA(m,n) = (1/m) * sum over the newest m values of x - (1/n) * sum over the newest n values of x
```

- `MA(m,n)` is the signal.
- `x` is the slow line from the filter.
- With `m = 1` and `n = 2`, this becomes `x_today - (x_today + x_yesterday) / 2`, which has the same
  sign as `x_today - x_yesterday`. So the signal is positive when the slow line rose, and negative when
  it fell.

Finally the cost of a round trip, which for a currency pair is charged mostly through the gap between
the buying and the selling price:

```text
cost = notional * c_per_side * 2
```

- `notional` is the size of the position, for example 100,000 euros' worth.
- `c_per_side` is the cost of one crossing as a fraction of the amount, about 0.0001 for a widely
  traded pair, which is one basis point where a basis point is one hundredth of one percent.
- The two at the end is because the position is entered and later exited, so the gap is paid twice.

## A worked example

Eight daily observations. The middle column is the slow line that the filter draws through the prices;
the values are made up, but they have the shape the filter produces. The signal is
`(trend today - trend yesterday) / 2`, using the `MA(1,2)` rule.

| Day | Price  | Slow line | Signal   | Action                           |
| --- | ------ | --------- | -------- | -------------------------------- |
| 1   | 1.0790 | 1.0790    |          |                                  |
| 2   | 1.0795 | 1.0785    | -0.00025 | Nothing                          |
| 3   | 1.0800 | 1.0788    | +0.00015 | Turned positive, buy at 1.0800   |
| 4   | 1.0810 | 1.0795    | +0.00035 | Hold                             |
| 5   | 1.0825 | 1.0805    | +0.00050 | Hold                             |
| 6   | 1.0835 | 1.0812    | +0.00035 | Hold                             |
| 7   | 1.0830 | 1.0810    | -0.00010 | Turned negative, close at 1.0830 |
| 8   | 1.0815 | 1.0800    | -0.00050 | Sell short and hold              |

Notice that day 4 is a day the price rose while the signal was already positive, so nothing happened,
and day 6 is a day the price fell while the signal stayed positive, so nothing happened either. The
strategy acts only on the days the signal changes sign, which is why it trades so rarely.

The long trade ran from day 3 to day 7. With a notional of 100,000:

```text
return    = (1.0830 - 1.0800) / 1.0800 = 0.002778, which is +0.2778 percent
gross     = 100,000 * 0.002778 = 277.78
cost      = 100,000 * 0.0001 * 2 = 20.00
net       = 277.78 - 20.00 = 257.78, which is +0.2578 percent of the account
```

Then day 7 flips the position: the long is closed and a short is opened at the day-7 price of 1.0830.
By day 8 the price is 1.0815, so the short is ahead by
`(1.0830 - 1.0815) / 1.0830 = 0.001385`, about 138.50 on the same notional, before costs, and the
position is still open because the signal is still negative.

Two things to notice. The trade lasted five days and produced a quarter of a percent, so the whole
result depends on catching a handful of such moves. And the slow line on day 2 fell while the price
rose, and the line on day 7 fell while the price was still near its high: the smoothing deliberately
lags the price, which is what makes it quiet, and also what makes it late.

## What the research actually found

| Source                                            | What it measured                                                                              | Result                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Harris and Yilmaz, 2009                           | The slow-component momentum rule on currency data, versus ordinary moving-average rules       | Reported greater directional accuracy, higher returns and Sharpe ratios, a smaller worst fall and less frequent trading than the moving-average rules, and robustness across time periods and across the smoothing setting                                                      |
| The library page, applying the paper to six pairs | Daily prices, five years of training before 2011, out-of-sample from January 2011 to May 2017 | Mostly weak: EURUSD returned -3.652 percent a year with a Sharpe ratio of -0.309, USDCAD +0.702 with 0.107, USDCHF -0.154 with 0.044, EURGBP +4.035 with 0.480, USDNOK +2.187 with 0.210 and USDZAR +1.042 with 0.166; the worst falls ranged from 19.2 percent to 52.9 percent |
| The library page, on its own settings             | The same test with different lag settings in the rule                                         | The page reports that performance is sensitive to the lag parameters, in a non-monotonic way, which means that getting a better result by changing the settings is usually luck rather than skill                                                                               |

The disagreement is direct. The paper says the rule is robust and beats moving averages; the page that
applies it out of sample found that it does not, with only two of the six pairs making a positive
return and most of the worst falls above twenty percent. The page also names a mechanical problem: the
filter is designed to draw a line through a whole set of data at once, so adding each new day's price
changes the values the line assigns to past days. A trader using the rule in real time never had the
line that the backtest later drew through the same period, which is a subtle way a backtest can look
better than the trade ever did.

## How this project relates to it

This repository does not implement a Hodrick-Prescott trend rule. The closest material is the brief
[Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), which covers
the other way to turn macro information into a currency position: reading scheduled data releases and
taking a direction. Its own warning applies directly here, namely that the reported currency result is
measured before costs and rests on a single data source.

The more important relative is the brief
[Regimes, breakpoints and state switching](../../../strategies/books2/24_regimes_and_change_points.md).
Its Section 4 contrasts filtering with hindsight: a model that uses future data to draw a smooth curve
through the past will always look better than one that had to decide in real time. That is the exact
hazard in step 3 above, where the filter is recomputed over the whole window each day. The engine in
[implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md) was
built to avoid this class of mistake: it measures a market and returns one of four verdicts, and
refuses to switch on a rule that depends on a trend it cannot find. The project tutorial
[Telling which kind of market you are in](../../project/measuring-the-regime/README.md) explains that
verdict in plain words.

## Where it goes wrong

- The smoothing is chosen by eye. `lambda` is not estimated from the data; it is lowered until the
  line looks useful. Change it and the signals change, which the page itself reports. A finding that
  depends on a hand-picked setting is fragile.
- The filter rewrites the past. Each new price changes the slow line's values for earlier days, so the
  curve a backtest draws is not the curve a trader would have seen while trading. This is the single
  most likely reason a backtest of this rule beats its out-of-sample result.
- It reacts late by construction. A smooth line is behind the price, so the rule gives up part of the
  move at the start and keeps part of the reversal at the end. On a rule that trades ten times in six
  years, the two ends are most of the move.
- A drift that ends is indistinguishable from noise. The strategy has no way to tell a genuine turn
  from a pause, so a currency that stops drifting produces a slow bleed of small losses while the
  position waits for a signal that comes too late.
- The sample is small. Six pairs over six and a half years with ten to fifteen trades each is a few
  dozen trades in total, which is not enough to separate a real edge from luck.
- Costs and financing. Holding a currency position has a financing cost, in the form of the interest
  rate difference between the two currencies, which does not appear in the returns above and which for
  some pairs can be larger than the drift being traded.

## Try it yourself

You need a spreadsheet, a column of daily prices for any currency pair, and a way to smooth a series.
The smoothing can be done approximately with a long simple moving average if you do not have the
Hodrick-Prescott filter.

1. Put the date in one column and the daily closing price in the next, for at least 250 rows.
2. Add a column for the slow line: for the exact rule use the Hodrick-Prescott filter with lambda 100;
   for a rough version use a 20-day moving average of the price.
3. Add a column for the signal: this row's slow line minus the previous row's slow line.
4. Add a column that records the position: buy when the signal turns from negative to positive, sell
   short when it turns from positive to negative, and otherwise keep whatever position you had.
5. Add a column for the daily change in the price while the position is held, so the sheet shows what
   the rule would have earned each day.
6. Add a column that charges one basis point of the notional on every day the position changes.

What to notice: set the smoothing to a short average and then a long one and watch how the number of
trades rises and falls, and how the result changes with it. Then look at the days the signal flips and
ask whether they look like the start of a real move or like the day after it. On most pairs the flips
will cluster at turning points that have already happened, which is the lag the paper is trying to
trade against.

## Where this came from

- [QuantConnect strategy library: momentum based on the low frequency component of forex](https://www.quantconnect.com/tutorials/strategy-library/the-momentum-strategy-based-on-the-low-frequency-Component-of-forex-market),
  the rules as implemented, the choice of smoothing setting, the rolling estimate and the out-of-sample
  table for six pairs.
- Harris and Yilmaz, [A momentum trading strategy based on the low frequency component of the exchange rate](https://www.sciencedirect.com/science/article/pii/S0378426609000521),
  Journal of Banking and Finance, Volume 33, Number 9, 2009, pages 1575 to 1585, for the rule and for
  the authors' own claim about its robustness. The full text is behind a publisher paywall; only the
  abstract was read, so the paper's numbers other than its claims are not cited here.
- [Macro, rates and foreign exchange](../../../strategies/books2/07_macro_rates_and_fx.md), this
  repository's brief on turning macro information into a currency position.
- [Regimes, breakpoints and state switching](../../../strategies/books2/24_regimes_and_change_points.md),
  Section 4, for the distinction between a filter that runs in real time and a line drawn with
  hindsight.
- [Telling which kind of market you are in](../../project/measuring-the-regime/README.md), the project
  tutorial on separating a real trend from noise.

## Words used in this tutorial

- basis point: one hundredth of one percent, so one basis point of 100,000 is 10.00.
- cycle: the fast, short-lived part of a series once the slow trend has been removed.
- Hodrick-Prescott filter: a smoothing method that draws a slow line through a series while trying not
  to bend too much.
- momentum: the tendency of something that has been rising to keep rising for a while.
- moving average: the average of the most recent values of a series, updated as new values arrive.
- notional: the full size of the position, before any borrowed money is counted.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- trend: the slow, persistent direction of a series once the short-term wobbles are set aside.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
