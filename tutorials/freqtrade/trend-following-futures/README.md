# Trend following on futures: buy when the price crosses its own average, sell short when it crosses back

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                    |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto coin futures contracts on a crypto exchange, in both directions, for example Bitcoin priced in a stablecoin                                                                                       |
| How often it trades       | Often; on five-minute candles the crossing can happen many times a day                                                                                                                                   |
| What you need             | A spreadsheet and a column of five-minute closing prices with the volume traded in each                                                                                                                  |
| Where the rules come from | [TrendFollowingStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/TrendFollowingStrategy.py), the file in the Freqtrade community strategy repository |
| The underlying research   | Moskowitz, Ooi and Pedersen, [Time Series Momentum](https://www.aqr.com/Insights/Research/Journal-Article/Time-Series-Momentum), and the century-long extension by Hurst, Ooi and Pedersen               |
| How well it held up       | Mixed: the idea has a century of out-of-sample evidence across 67 futures markets, but this file's five-minute settings have no published measurement and its written exit condition almost never fires  |
| Also appears in           | [Commodities futures trend following](../../quantconnect/commodities-futures-trend-following/README.md) in this collection, the same family of rules applied to commodity contracts                      |

## The idea in one paragraph

This strategy computes a single smoothed average of the price that leans on recent five-minute closes
more than on older ones. When the price moves from below that average to above it, and the volume
line is rising at the same time, the strategy buys. When the price moves from above the average to
below it, and the volume line is falling, the strategy sells short, a bet that the price will keep
falling. The position is held until the rules say the trend has turned, the price passes the profit
ladder, or a loss reaches 26.5 percent. The idea is that a price which has been rising tends to keep
rising for a while, so a rule that joins the move after it has started and leaves it after it has
turned collects a piece of the middle.

## Why anyone believed it

This is the oldest systematic trading idea there is: let your profits run and cut your losses short.
The reason it might work is that prices do not adjust instantly to news. A producer selling oil
forward, a central bank smoothing its currency, or an investor who anchors on an old price all trade
for reasons that are not about the current outlook, and they do it slowly. Their orders move the
price in the same direction over days and weeks, so a price that has been rising carries on rising
for reasons that have nothing to do with anyone's skill at forecasting.

The counterparty is the hedger: the farmer, the miner or the airline that has to lock in a price
whether or not the timing is good. Research on futures markets finds that speculative positions and
hedging positions sit on opposite sides of exactly this pattern, with the speculators following the
trend and the hedgers taking the other side. If the hedgers keep needing to trade, the trend keeps
being paid for.

## An everyday comparison

Think of a long queue at a ticket window. Nobody in the queue knows how fast it is moving, but
everyone can see whether it is moving. A person who joins when it is clearly advancing gets served in
the end; a person who joins when it is stuck has often chosen a queue that is stuck because the
window is closing. The rule here does not forecast the queue, it watches which way it is going, joins
in that direction, and leaves when it stops going that way. The volume line is a second, separate
sign that the movement is real: a queue that is moving while more people keep arriving is different
from one that is moving because people are leaving.

## The rules, step by step

1. Use candles covering five minutes of trading, and record the closing price and the volume traded
   in each.
2. Compute the average: a smoothed average of the closing price in which the newest close counts for
   about 9.5 percent of the answer and the previous value of the average for the rest. This is the
   exponentially weighted average with a span of twenty, and the weighting rule is in the next
   section.
3. Compute the volume line, called the on-balance volume: start at zero, then for each candle add the
   candle's volume if the close rose since the previous candle, subtract it if the close fell, and
   add nothing if the close was unchanged.
4. Buy when all three of these are true: the close is above the average; the previous close was at or
   below the previous value of the average, which means the price has just crossed upward; and the
   volume line is higher than it was on the previous candle.
5. Sell short when all three of these are true: the close is below the average; the previous close
   was at or above the previous value of the average; and the volume line is lower than it was on the
   previous candle.
6. Close a long position when the close is below the average and the previous close was at or above
   it, and the volume line is higher than on the previous candle. Close a short position with the
   mirror of that condition.
7. Three automatic exits also apply at every candle. The position closes if it is ahead by the amount
   in the ladder below, if it is behind by 26.5 percent, or when the trailing stop described in the
   next section is hit.

| Setting                          | Value                                                                                    |
| -------------------------------- | ---------------------------------------------------------------------------------------- |
| Profit target, minutes 0 to 29   | 15 percent                                                                               |
| Profit target, minutes 30 to 59  | 10 percent                                                                               |
| Profit target, minutes 60 onward | 5 percent                                                                                |
| Stop loss                        | 26.5 percent below the entry price                                                       |
| Trailing stop                    | On, 26.5 percent below the highest price, then 5 percent once the gain passes 10 percent |
| Timeframe                        | Five-minute candles                                                                      |

## The maths, with every symbol named

The average is the exponentially weighted average with a span of twenty, written in the file as a
twenty-span exponentially weighted mean without an adjustment term:

```text
weight = 2 / (span + 1), with span = 20, so weight = 0.0952
average_t = weight * close_t + (1 - weight) * average_(t-1)
```

- `span` is the number of candles the average is meant to resemble; twenty five-minute candles is
  one hour and forty minutes of trading.
- `weight` is the share of the answer that comes from the newest close, about 9.5 percent.
- `close_t` is today's closing price.
- `average_(t-1)` is yesterday's value of the average. The first value of the average is set equal to
  the first closing price, so the average starts on the price and drifts away from it.

The on-balance volume line, which measures whether volume has been arriving on rising or falling
prices:

```text
OBV_t = OBV_(t-1) + volume_t   when close_t > close_(t-1)
OBV_t = OBV_(t-1) - volume_t   when close_t < close_(t-1)
OBV_t = OBV_(t-1)              when the two closes are equal
```

- `OBV_t` is the running total, which starts at zero on the first candle.
- `volume_t` is the amount traded in the candle.
- The total has no upper or lower limit; it is only ever compared with its own previous value, so only
  its direction matters.

The entry and exit conditions, as the file writes them:

```text
enter long   when close_t > average_t  and  close_(t-1) <= average_(t-1)  and  OBV_t > OBV_(t-1)
enter short  when close_t < average_t  and  close_(t-1) >= average_(t-1)  and  OBV_t < OBV_(t-1)
exit long    when close_t < average_t  and  close_(t-1) >= average_(t-1)  and  OBV_t > OBV_(t-1)
exit short   when close_t > average_t  and  close_(t-1) <= average_(t-1)  and  OBV_t < OBV_(t-1)
```

- The first two lines are the entry rules; the last two are the exit rules.
- `and` means every part of the condition must hold at the same candle.
- Read the exit rules against the entry rules: the exit for a long needs the close to cross down
  through the average while the volume line is still rising, which is the one combination the volume
  line is least likely to produce on a falling candle. The exit rule as written, then, almost never
  fires, and this is stated here as a fact about the file rather than as a judgement on its
  author's intent.

The money at risk:

```text
stop price = entry price * (1 - 0.265)
trailing stop price = highest price since entry * (1 - 0.265)   while the gain is under 10 percent
trailing stop price = highest price since entry * (1 - 0.05)    once the gain has passed 10 percent
```

- `entry price` is the price at which the position was opened.
- `highest price since entry` is the best price the trade has seen; the trailing stop follows it up
  and never moves back down.
- The trailing stop begins working as soon as the trade is in profit, because the file turns off the
  setting that would otherwise delay it until the offset is reached.

## A worked example

Eight five-minute candles. The average is started at 100.00 before the first candle in the table,
and the on-balance volume starts at zero on the first candle. The numbers are invented but of the
size a five-minute candle in a large crypto contract can take.

| Candle | Close  | Volume | Average | Volume line | Close above average | Just crossed | Volume line rising | Signal |
| ------ | ------ | ------ | ------- | ----------- | ------------------- | ------------ | ------------------ | ------ |
| 1      | 100.00 | 10     | 100.00  | 0           | level               | no           | yes                | none   |
| 2      | 100.50 | 12     | 100.05  | 12          | yes                 | yes          | yes                | buy    |
| 3      | 100.20 | 8      | 100.06  | 4           | yes                 | no           | no                 | none   |
| 4      | 100.80 | 15     | 100.13  | 19          | yes                 | no           | yes                | none   |
| 5      | 101.40 | 20     | 100.25  | 39          | yes                 | no           | yes                | none   |
| 6      | 100.90 | 25     | 100.31  | 14          | no                  | yes          | no                 | none   |
| 7      | 100.60 | 30     | 100.34  | -16         | no                  | no           | no                 | none   |
| 8      | 99.80  | 5      | 100.29  | -21         | no                  | yes          | no                 | none   |

Read the table one line at a time. At candle 2 the close is 100.50, the average is 100.05, the
previous close of 100.00 was level with the previous average of 100.00, and the volume line has gone
from 0 to 12, so all three conditions of the buy rule hold and the trade is opened at 100.50.

At candle 6 the price crosses back below the average, which is the moment the exit rule was written
for. But the volume line fell from 39 to 14, and the exit rule demands that it be rising; the exit
does not fire. The same happens at candle 8: the price is below the average at 99.80 against 100.29
and the previous close was above the previous average, so the crossing part of the rule is satisfied,
but the volume line fell again, from -16 to -21, so the exit does not fire either.

The trade is therefore still open at the end of the table, and it is closed by the automatic rules
instead. Suppose the price climbs to 105.53 about an hour and a half after the entry, which is a
5.00 percent gain and past the 5 percent target that applies after the first hour:

```text
Gross gain = 105.53 / 100.50 - 1 = 0.0500, that is +5.00 percent
Cost       = 2 sides * 0.075 percent = 0.15 percent
Net gain   = 5.00 - 0.15 = +4.85 percent
```

For contrast, the stop loss sat at 100.50 times 0.735, which is 73.87, and the trailing stop followed
the highest price upward once the trade was in profit, first at 26.5 percent below that highest price
and then, after the gain passed 10 percent, at 5 percent below it. On this path none of the three
automatic exits was reached except the profit target, but on a path that went against the trade the
26.5 percent stop would have been the one that mattered, and that is a very large single loss.

Two honest observations. First, the profit in the example comes entirely from the ladder, not from
the rule the tutorial is about: the crossing and the volume line fired once, on the way in, and never
again. Second, the volume condition is doing very little work. On this path it only ever confirmed
what the price crossing had already said, and on the exits it blocked the rule from ever firing.

## What the research actually found

Nothing was measured for this file: the repository publishes no backtest, no trade count and no
result for `TrendFollowingStrategy.py`, and the file records no parameters beyond the ladder, the
stop loss and the trailing stop. What does exist is the strongest body of measurement in this whole
collection, and it is about the general idea rather than about five-minute candles.

Moskowitz, Ooi and Pedersen, in 2012, took 58 futures markets covering share indexes, currencies,
commodities and government bonds, from 1965 to 2009, and asked whether a contract's own past return
predicts its next return. It does: the past twelve months of return predict the next month
positively, the effect lasts about a year and then partially reverses over longer horizons, and the
sign of the past twelve-month return was positive for every one of the 58 contracts. A portfolio that
held each contract in the direction of its own past return, sized so that each contributed the same
risk, produced a reward per unit of risk above one, roughly two and a half times that of the share
market, and did best in the most extreme market months. That last property is what makes the idea
attractive to funds: it is a rule that tends to be on the winning side when shares fall hard.

Hurst, Ooi and Pedersen extended the test to 67 markets from 1880 to 2016, which is out-of-sample
relative to the first study, and found positive average returns in every decade, an average reward
per unit of risk of about 0.4 on the same measure, and positive returns in 8 of the 10 worst declines
of a portfolio of 60 percent shares and 40 percent bonds. They also subtract costs, assumed to be
twice as high from 1993 to 2002 and six times as high before 1993, and a management fee of 2 percent
plus 20 percent of the gains. Two limits must be stated with these results. Both studies measure
position changes about once a month, not once every five minutes, and both measure a portfolio of
many markets, not one contract. Nothing in either has been shown to survive being run every five
minutes on a single crypto pair with a fee on each side.

## How this project relates to it

This repository implements both indicators in code with tests around them: the exponentially
weighted average at
[crates/indicators/src/average/ema.rs](../../../crates/indicators/src/average/ema.rs) and the
on-balance volume line at
[crates/indicators/src/momentum/obv.rs](../../../crates/indicators/src/momentum/obv.rs). A reader who
wants to see exactly how the first value of an average is set, the detail that makes the first twenty
candles of any backtest unreliable, will find it there. The problem of a futures contract that
expires, and therefore of deciding when to move to the next contract, is set out in this repository's
[continuous futures documentation](../../../docs/concepts/continuous_futures.md); a rule held for
weeks will meet that problem, and the file says nothing about it.

## Where it goes wrong

- The exit rule is nearly unreachable. As the worked example shows, it requires the price to cross
  down through the average while the volume line rises, and those two conditions pull in opposite
  directions, so the trade ends at the ladder, the stop loss or the trailing stop instead.
- Five minutes is a very short horizon. Costs are charged per trade, the two studies above measure
  monthly position changes, and a rule that trades several times a day pays the fee and the price gap
  many times over.
- The parameters are stated, not justified: nothing published says the span should be twenty rather
  than ten, or the stop loss at 26.5 percent.
- The average is a lagging line. In a fast, choppy market the price crosses its own average
  repeatedly, so the rule buys and sells within minutes and pays the cost each time.
- A large fixed stop plus leverage is a large loss. A 26.5 percent stop on a leveraged position can
  take a serious part of the account in one trade, and crypto can travel that far in a session.
- Funding and contract rolls are invisible on the chart. A perpetual contract charges a funding
  payment every few hours, and an expiring contract must be moved to the next one at a cost.

## Try it yourself

You need a spreadsheet and one hundred five-minute closes with their volumes from any public price
chart.

1. Column A is the candle number, column B the close, column C the volume.
2. Column D is the average. Put `=B2` in the first cell, then `=0.0952*B3+0.9048*D2` below it.
3. Column E is the volume line. Put 0 in the first cell, then
   `=IF(B3>B2, E2+C3, IF(B3<B2, E2-C3, E2))` below it.
4. Column F flags a crossing upward: `=IF(B3>D3, IF(AND(B2<=D2, E3>E2), "buy", ""), "")`.
5. Column G flags a crossing downward with a falling volume line:
   `=IF(B3<D3, IF(AND(B2>=D2, E3<E2), "sell", ""), "")`.
6. Column H is the exit rule exactly as the file writes it:
   `=IF(B3<D3, IF(AND(B2>=D2, E3>E2), "exit long", ""), "")`.

What to notice: count how many times column F fires and how many times column H fires. On most real
price series column H will be almost always empty even though column G, which looks like the same
rule, fills up. That difference is not a market fact; it is a mistake in the written rule, and it is
the kind of mistake that changes a strategy's result without ever appearing in its profit figure.

## Where this came from

- [TrendFollowingStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/futures/TrendFollowingStrategy.py),
  the rules as implemented: the twenty-span average, the on-balance volume line, the crossings, the
  ladder, the 26.5 percent stop and the trailing stop on five-minute candles.
- Moskowitz, Ooi and Pedersen, [Time Series Momentum](https://www.aqr.com/Insights/Research/Journal-Article/Time-Series-Momentum),
  the 58-market study of a contract's own past return predicting its next return.
- Hurst, Ooi and Pedersen, [A Century of Evidence on Trend-Following Investing](https://www.aqr.com/Insights/Research/Journal-Article/A-Century-of-Evidence-on-Trend-Following-Investing),
  the extension of that finding back to 1880 across 67 markets, with costs and fees subtracted.
- [Commodities futures trend following](../../quantconnect/commodities-futures-trend-following/README.md)
  in this collection, which builds the same rule on commodity contracts rather than crypto ones.
- [The Freqtrade strategy repository README](https://github.com/freqtrade/freqtrade-strategies/blob/main/README.md),
  which states that the files are starting points and that results depend on the pairs, timeframe and
  period chosen.

## Words used in this tutorial

- candle: one interval of trading, described by its opening, highest, lowest and closing price.
- exponentially weighted average: an average in which the newest value counts for more than older
  ones, so it turns faster than a plain average.
- funding payment: a fee paid between holders of a perpetual futures contract, charged every few
  hours, which is not visible on a price chart.
- futures contract: an agreement to buy or sell something at a set date, commonly used to bet on a
  price with borrowed money.
- leveraged position: a position larger than the money put up, so that gains and losses are both
  multiplied.
- on-balance volume: a running total that adds the volume of up candles and subtracts the volume of
  down candles, used as a sign of whether more trade is happening on rising or falling prices.
- short: selling something borrowed, so that a fall in its price is a gain.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
