# Opening range breakout: trading the London open from Tokyo's last hour

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | The price of one British pound in American dollars (the GBP/USD currency pair), read once a minute                                                                                                                 |
| How often it trades       | At most once a day, in the first thirty minutes after the London session opens, and on many days not at all                                                                                                        |
| What you need             | Python and a data file of one-minute currency prices (the source reads a file it calls gbpusd.csv)                                                                                                                 |
| Where the rules come from | [London Breakout backtest.py](https://github.com/je-suis-tm/quant-trading/blob/master/London%20Breakout%20backtest.py), the script that states every rule and parameter                                            |
| The underlying research   | None: a practitioner's rule of thumb; the session rhythm it leans on is measured in `1103.5664v1`, but that paper never tests this rule                                                                            |
| How well it held up       | Open question: the source states the rule and reports no measured result for it                                                                                                                                    |
| Also appears in           | [Breakout: buying when a price reaches a new high](../../backtrader/breakout/README.md) and [Dual Thrust](../../quantconnect/dual-thrust-trading-algorithm/README.md), which draw opening-range lines of their own |

## The idea in one paragraph

Currencies are bought and sold around the clock on weekdays, but the desks doing it sit in different
cities as the planet turns: Tokyo trades first, London takes over in the European morning, and New
York follows in the afternoon. This rule watches the quiet last hour before London opens and notes the
highest and lowest price in it. When London opens it draws one line at that hour's high and another at
its low. If, in the next half hour, the price pushes above the high line it bets the pound will keep
rising, and if it drops below the low line it bets the pound will keep falling. Once in, it closes the
bet when the price moves half of a small fixed distance either way from where it entered, and closes
anything left at a fixed hour in the afternoon, whatever the price is then.

## Why anyone believed it

Foreign exchange, the market where one country's money is swapped for another's, has no single
building and no closing bell: a currency pair (the price of one currency in another) can be traded in
Sydney, Tokyo, London and New York, and on weekdays at least one of them is always open. A London desk
can therefore watch the price while its home market sleeps. The claim is that Tokyo's last hour, when
few large orders are working, quietly collects the news that arrived overnight, and that the London
desks price it in when they arrive at eight in the morning, so the first move of the day carries on.

There is a measured version of half that story. Cotter and Dowd studied real DEM/USD trades (the
German mark against the dollar) on an electronic inter-dealer system over one week in October 1997,
130,535 trades in all, and found a strong daily rhythm: most trading fell between 9:00 and 18:00 GMT,
when almost 70 percent of all orders were handled, while the quietest stretch, 21:00 to 24:00 GMT,
carried less than one percent (`1103.5664v1`, p.7). Heston, Korajczyk and Sadka found a related rhythm
in share prices: a stock's return in one half hour tends to repeat in the same half hour on following
days, and the effect lasted at least forty trading days across 1,715 New York-listed firms from 2001
to 2005 (`1005.3535v1`, p.1). Neither measurement is about this rule, but together they say the clock
matters: who is awake changes what the market does.

The counterparty is the trader who must act at the open: a European bank starting its day, a company
settling a foreign bill, or a dealer passing a client order along. If those orders arrive in one
direction just after the open, the price that just left the Tokyo range keeps travelling.

## An everyday comparison

A village market square fills at eight in the morning. Before eight, a few early sellers trade on the
pavement and settle on a narrow band of prices. When the square opens, the first real prices of the day
are struck against that band. If the first buyers pay well above what the pavement charged, the price
keeps climbing, because everyone who turns up later sees the higher price and joins it. The rule here
does not guess the price; it waits for the square to open and follows whichever way the first trades
break out of the pavement band.

## The rules, step by step

1. Use one-minute prices for GBP/USD, the number of American dollars needed to buy one British pound.
   A one-minute bar is that minute's opening, highest, lowest and closing price. The source file is
   stamped in New York time, which its comments give as five hours behind GMT.
2. Take the reference hour: every one-minute price whose clock hour is 2 in New York time, which is
   07:00 to 07:59 GMT, Tokyo's last hour before London opens (script line 111).
3. At the first minute of hour 3 (08:00 GMT, the London open) write down the upper line, the highest
   price of the reference hour, and the lower line, the lowest (script lines 125 and 126).
4. Watch only the next thirty minutes, minutes 0 to 29 of hour 3 (`open_minutes = 30`, script line 90).
5. Buy one unit when a minute price is strictly above the upper line; sell one unit when it is strictly
   below the lower line. One unit at a time.
6. Refuse a breach that is too violent. The script sets a risk distance `risky_stop = 0.01` (script line
   83), which it calls one hundred basis points: if a buying bar is more than 0.01 above the upper line,
   do not buy, and if a selling bar is more than 0.01 below the lower line, do not sell (lines 159 to
   184).
7. Take only the first accepted breach. Once a position is open, ignore every later breach in the same
   window, so the running count of open positions never exceeds one (script lines 162 and 180).
8. Record the price of the accepted breach. That is the entry price, and both exits are measured from
   it.
9. After the first thirty minutes, close an open position when a minute price is more than half the
   risk distance above the entry or more than half below it: half of 0.01 is 0.005, so the target and
   the loss limit are both 0.005, and either one flattens the position (script lines 204 and 207).
   Because exits are checked only outside the entry window, a position opened at minute 7 cannot close
   before minute 30 even if the price has already passed the target.
10. At the first minute of hour 12 (17:00 GMT, five in the afternoon in London), close anything still
    open (script lines 190 to 192), whatever the price is doing. This is the only exit not fixed by
    distance.
11. Review the price once a minute and trade one unit; the amount is not stated in the script.

Why each part is there. The height of the reference hour (item 3) is a stand-in for how far this pair
normally travels in a day, so a price outside that band is unusual for the hour. The thirty-minute
window (item 4) stops the rule from chasing a move that is already hours old. The tolerance (item 6) is
the brake: a breach far past the line is a violent gap the strategy cannot manage, so it stands aside
instead of buying it. The half-distance exit and the fixed afternoon close (items 9 and 10) mean the
bet never runs past the day.

## The maths, with every symbol named

```text
upper = the largest P(t) for every minute t whose clock hour is 2
lower = the smallest P(t) for every minute t whose clock hour is 2
```

- `P(t)` is the price at minute `t`, and the clock hour is the hour on its timestamp in New York time.
- `upper` and `lower` are the two threshold prices, fixed at the London open; they are simply the high
  and the low of the reference hour.

```text
buy  when upper < P(t) <= upper + S, in hour 3 and before minute 30
sell when lower - S <= P(t) < lower, in hour 3 and before minute 30
exit when P(t) > E + S/2, or P(t) < E - S/2
force exit when the clock hour is 12
```

- `S` is the risk distance, 0.01 in the script, which its author calls one hundred basis points.
- `E` is the entry price of the first accepted breach.
- `S/2` is 0.005, half the risk distance, used as both the profit target and the loss limit.

The comparisons are strict at the line, so a price exactly on the upper line does not buy. The second
comparison on each entry line is the rejection rule: once the price is more than `S` beyond the line,
that breach is not traded. On GBP/USD one pip, the smallest quoted step, is 0.0001, so 0.01 is one
hundred pips and 0.005 is fifty. Note that 0.01 is one percent only when the price is 1.00; at GBP/USD
near 1.40 it is about 0.71 percent, so a distance fixed in price changes meaning as the rate drifts.

## A worked example

Two made-up GBP/USD sessions, trading one unit. The risk distance is 0.01 and half of it is 0.005.

Day one. In the reference hour (07:00 to 07:59 GMT) the price wanders between 1.3995 and 1.4025, so at
the London open the upper line is 1.4025 and the lower line is 1.3995, 30 pips apart.

| Time  | Price  | Upper  | Lower  | Action          |
| ----- | ------ | ------ | ------ | --------------- |
| 03:05 | 1.4010 | 1.4025 | 1.3995 | nothing         |
| 03:07 | 1.4032 | 1.4025 | 1.3995 | buy 1 at 1.4032 |
| 03:12 | 1.4040 | 1.4025 | 1.3995 | ignored         |
| 03:20 | 1.4028 | 1.4025 | 1.3995 | hold            |
| 03:41 | 1.4086 | 1.4025 | 1.3995 | close at 1.4086 |

At 03:07 the price is 1.4032, which is 0.0007 above the upper line: a breach, but a small one, inside
the 0.01 tolerance, so it is accepted and the entry price is 1.4032. The target is 1.4032 plus 0.005,
or 1.4082. At 03:12 the price is again above the line, but a position is open and the running count
would reach two, so the script refuses it. Exits are not checked inside the window, so the position
sits untouched through 03:20 even though the price slipped back. At 03:41, outside the window, the
price 1.4086 is above 1.4082, so the position closes there.

The arithmetic, in dollars per pound and on a standard lot of 100,000 pounds. The source charges no
costs; the two-pip spread below is this page's illustration, not a figure from the script.

```text
price move       = 1.4086 - 1.4032 = 0.0054
gross per pound  = 0.0054 dollars
gross on 100,000 = 0.0054 * 100,000 = 540.00 dollars
round-trip cost  = 0.0002 per pound
net per pound    = 0.0054 - 0.0002 = 0.0052
net on 100,000   = 0.0052 * 100,000 = 520.00 dollars
```

Day two. The reference hour is busier: the high is 1.4050 and the low is 1.4020, so the upper line is
1.4050. London opens on a surprise release and the price gaps far above the line.

| Time  | Price  | Distance above upper | Decision |
| ----- | ------ | -------------------- | -------- |
| 03:01 | 1.4162 | 0.0112               | refused  |
| 03:14 | 1.4175 | 0.0125               | refused  |

At 03:01 the breach is 1.4162 minus 1.4050, or 0.0112, more than the 0.01 tolerance, so the script
refuses it; every later breach in the window is refused too, so the day passes with no trade. The
script does not forbid the whole day outright; it simply will not buy a breach that far past the line,
and the price never returns to within one cent of it before the window closes.

## What the research actually found

The source measures nothing. It fixes no dates, loads whatever file it is given, takes the first day
in that file and plots that single day, printing no profit, no trade count and no ratio of reward to
risk; a closing comment points the reader to a different script by the same author for statistics.
There is therefore no number from the source to quote for this rule, which is why the grade above is
Open question: a rule is stated, but no measurement of it is supplied.

The nearest measurement in this collection is of a close relative, not this rule. An independent study
tested an opening range breakout on Micro E-Mini Nasdaq 100 futures over 947 complete trading days of
five-minute bars from December 2021 to August 2025, walk-forward, charging 2.0 index points round trip
on every trade (`2605.04004v3`, p.1). The opening-range family failed its gates: buying the breakout
and holding 15 bars averaged +2.82 points net over 447 out-of-sample trades with a t-statistic of 0.88,
below the study's 2.0 threshold; buying and holding one bar gave -0.82 points; selling the downside
breakout and holding one bar gave -3.45 points (p.4). The same paper found an Asia-session range
expansion moved against the breakout direction, t-statistic -11.52 at one bar (p.5). That study is an
independent manuscript, not a peer-reviewed article, and it tests a different instrument, session and
exit, so it is context for this rule rather than a verdict on it.

The background is better measured than the rule. Cotter and Dowd showed on one week of DEM/USD that
returns and their volatility have a real intraday seasonality, concentrated in the GMT day
(`1103.5664v1`, p.7). Heston, Korajczyk and Sadka showed a share's half-hour return repeats in the same
half hour over following days for at least forty trading days (`1005.3535v1`, p.1). Both prove the time
of day shapes the market; neither shows that trading the clock earns a profit after costs.

## How this project relates to it

This repository does not implement the London breakout. The closest thing it holds is the breakout
group at [Breakout: buying when a price reaches a new high](../../backtrader/breakout/README.md),
which collects the Donchian channel, Dual Thrust, R-Breaker and volume breakout files, and of those
[Dual Thrust](../../quantconnect/dual-thrust-trading-algorithm/README.md) is the nearest relative,
since it too draws two lines around the session open. The lines come from different places: Dual
Thrust measures the range of the last several days and attaches it to today's own opening price, while
this rule measures another session's high and low and attaches them to the next one.

Trading by the clock appears twice more. [Trading the clock](../../backtrader/time-session-system/README.md)
holds seven strategies that buy and sell at fixed hours, one of which builds a box from the previous
hours and trades its edges, and [Trading only during chosen hours of the day](../../freqtrade/time-of-day-filter/README.md)
is entirely about which hours a strategy may be active. For why one day proves nothing, [How a
backtest lies](../../foundations/07_how-a-backtest-lies.md) is the primer.

## Where it goes wrong

- The source assumes trading is free. The external repository's README says at line 19 that "the
  assumption is that all trades are frictionless. No slippage, no surcharge, no illiquidity", and the
  script charges nothing, so the 540 dollars above is a gross figure a real spread would cut.
- One day is not a test. The script fixes no dates, takes the first day in the file and plots it
  without statistics, so any impression comes from a single session chosen by accident. The idea would
  be false if Tokyo's last hour carries no information the London open has not already priced: a
  breach of its range would be noise, and a coin toss paying the spread loses on average.
- The risk distance is fixed in price, not in the pair's own range. 0.01 is always 100 pips, but that
  is one percent of the price at 1.00 and only about 0.71 percent of it at 1.40, so the same distance
  is a tighter filter in some years than others.
- The exits are not limit orders, and there are none inside the window. The script closes at whatever
  the minute price is when a boundary is crossed, not at the boundary, so a fast market can fill well
  past the half-distance target; and a position opened at minute seven is left untended until minute
  thirty even if the price has already run past the stop.
- The clock is assumed, not checked. A daylight-saving shift, or a file stamped in another zone, moves
  both windows with no warning in the output, turning the rule into a different rule that still looks
  correct on the chart.
- A session rule is visible to everyone, so any edge gets traded away until the move no longer covers
  the spread; the failure of the opening-range family in the futures study (`2605.04004v3`, p.4) is the
  shape crowding takes.

## Try it yourself

No money and no code are needed. Use a spreadsheet and a few days of one-minute prices for one
currency pair, inventing them if you cannot find a file, with every timestamp in the same time zone.

Columns, left to right:

- A: Time, as text such as 03:07; and B: Price, that minute's closing price.
- C: In reference hour, 1 if the hour is 02:xx, else 0.
- D: Upper, the largest price in column B among the rows where C is 1; and E: Lower, the smallest.
- F: Above upper, B minus D; and G: Below lower, E minus B.
- H: Accepted buy, 1 if F is greater than 0 and at most 0.01 and no position is open yet; and I:
  Accepted sell, the same test using G.
- J: Action, written by hand as the running story: buy, hold or close, following the rules above.

Then count over several days how many produce no accepted breach, and how often a breach is refused
because column F or G is greater than 0.01. What to notice: on a quiet open the rule usually does
nothing, because the price stays between the lines; most accepted breaches sit within a few pips of a
line rather than far beyond it; and the violent-gap days are refused exactly when they were the most
tempting. That is the shape of the rule: built to sit out, not to act.

## Where this came from

- [London Breakout backtest.py](https://github.com/je-suis-tm/quant-trading/blob/master/London%20Breakout%20backtest.py),
  the script read in full for every rule, including the Tokyo hour, the thirty-minute window, the 0.01
  risk distance and the 12:00 flatten.
- [The external repository README](https://github.com/je-suis-tm/quant-trading/blob/master/README.md),
  whose London Breakout section states the idea and whose line 19 states the frictionless caveat.
- [Cotter and Dowd, Intra-Day Seasonality in Foreign Exchange Market Transactions](https://arxiv.org/abs/1103.5664v1),
  `1103.5664v1`, the FX intraday-seasonality measurement.
- [Heston, Korajczyk and Sadka, Intraday Patterns in the Cross-section of Stock Returns](https://arxiv.org/abs/1005.3535v1),
  `1005.3535v1`, the half-hour return-continuation measurement.
- [Mesfin, Structural Limits of OHLCV-Based Intraday Momentum Signals in MNQ Futures](https://arxiv.org/abs/2605.04004v3),
  `2605.04004v3`, the opening-range breakout failure.
- [Awesome oscillator](../awesome-oscillator/README.md), a sibling page drawn from the same external
  repository.

## Words used in this tutorial

- pip: the smallest quoted step of a currency pair, 0.0001 for GBP/USD.
- basis point: one hundredth of one percent, used here for a price move of 0.01.
- currency pair: the price of one currency expressed in another, such as GBP/USD.
- session: the hours during which one financial centre is actively trading.
- long: a bet that the price will rise.
- short: a bet that the price will fall.
- spread: the gap between the price at which you can buy and the price at which you can sell.
- opening range: the highest and lowest price reached during a chosen window at the start of, or just
  before, a session.

- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
