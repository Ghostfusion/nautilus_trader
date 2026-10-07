# A profit target that falls the longer a trade is held: the ROI ladder

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto coins on a crypto exchange, such as Bitcoin priced in United States dollars                                                                                                                                                          |
| How often it trades       | Often; on five-minute candles the entry rule can fire many times a day                                                                                                                                                                      |
| What you need             | Nothing but this page; the ladder is a rule about when to sell, not a way to choose what to buy                                                                                                                                             |
| Where the rules come from | [Strategy001.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/Strategy001.py) through [Strategy005.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/Strategy005.py) |
| The underlying research   | None, this is a practitioner's rule of thumb that ships as a default in the Freqtrade bot                                                                                                                                                   |
| How well it held up       | Weak: the same ladder is copied between several files, no measurement is published for it, and the repository's own README calls the strategies starting points rather than strategies to trade                                             |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                             |

## The idea in one paragraph

Buy a cryptocurrency pair and immediately ask a high price for your profit: five percent, and the
position is sold the moment it is five percent ahead. If the price has not moved that far, the
target falls as the clock runs: four percent after twenty minutes, three percent after thirty, and
one percent after an hour. Whenever the trade's current gain reaches the target that is in force at
that moment, the position is sold. A loss of ten percent closes the trade at any time, whatever the
clock says. The bet is that most quick moves fade, so a profit taken early is worth more than a
profit waited for.

## Why anyone believed it

A fast move in a crypto price is mostly noise: a few large orders hit the exchange, the price jumps,
and it drifts back when the buying stops. Whoever is on the other side of that jump has a reason to
sell into it, because they were already holding and the jump is their chance to get out at a good
price. A rule that sells into the jump can therefore collect the part of the move that is real before
the fade takes it back.

The falling target adds a second belief. A trade that is still open after an hour has not gone the
way the entry signal suggested, so the signal was probably weak; if the trade is a little ahead at
that point, take what is there rather than keep waiting for a number that the evidence no longer
supports. The ladder is also a way of limiting how long money sits in one position, so the same
account can be reused instead of being tied up.

## An everyday comparison

Think of a market stall selling fresh fish. At opening time the stallholder asks a high price,
because there is all day to sell and no need to bargain. As closing time approaches the fish are
still there and will not keep, so the asking price drops: a smaller margin now beats no sale at all.
The stallholder is not predicting the market, just trading a higher price for a higher chance of
selling. The ROI ladder applies the same habit to a trade: a big profit if it comes quickly, a
smaller profit if it does not.

## The rules, step by step

1. Pick a cryptocurrency pair, for example Bitcoin priced in a stablecoin, and a timeframe of five
   minutes, so each candle on the price chart covers five minutes of trading. A candle is the
   opening price, highest price, lowest price and closing price over one interval.
2. Watch for the entry condition used by Strategy001: the twenty-period exponential moving average
   (an average of the last twenty prices that leans more on the most recent ones) crosses up
   through the fifty-period one, the smoothed heikin-ashi closing price is above the twenty-period
   average, and the heikin-ashi candle is green, meaning its close is above its open.
3. When that condition is true, buy. Record the entry price and the time of the purchase.
4. At every later candle, compute the current gain: today's price divided by the entry price, minus
   one.
5. Find how many minutes have passed since the purchase. Read the ladder for the entry with the
   largest number of minutes that is not greater than the elapsed time. That entry's value is the
   target.
6. If the current gain is at least the target, sell the whole position.
7. If instead the current gain is at or below minus ten percent, sell, because that is the stop
   loss: the fixed level at which the strategy admits the trade is not working.
8. After selling, wait for the next entry condition and repeat.

For the files in this repository the whole default set is fixed: the ladder is
`{"60": 0.01, "30": 0.03, "20": 0.04, "0": 0.05}`, the stop loss is minus ten percent, the timeframe
is five minutes, trailing stops are switched off, and the exit signal is a separate condition (the
fifty-period average crossing up through the hundred-period one while the smoothed close is below
the twenty-period average and the candle is red). Strategy005 keeps the same stop loss and timeframe
but lengthens the ladder to `{"1440": 0.01, "80": 0.02, "40": 0.03, "20": 0.04, "0": 0.05}`.

## The maths, with every symbol named

The ladder is a list of pairs. Each pair says: from this many minutes onward, this is the profit
needed to sell.

```text
elapsed = (now - open_time) in minutes
```

- `elapsed` is how long the trade has been open, counted in whole minutes.
- `now` is the current moment, and `open_time` is the moment the buy was filled.
- Subtracting the two gives a length of time, which is then read as minutes.

```text
target(elapsed) = ladder[K], where K is the largest key in ladder that is not greater than elapsed
```

- `target(elapsed)` is the profit the trade must reach to be sold at this moment.
- `ladder` is the list of minute-and-profit pairs above.
- `K` is a key of that list, a whole number of minutes; the rule picks the largest key that has
  already been passed. With the default ladder, `K` is 0 during the first twenty minutes, 20 until
  minute 29, 30 until minute 59, and 60 afterwards.

```text
gain = (price_now / entry_price) - 1
```

- `gain` is the trade's progress as a fraction: 0.05 means five percent.
- `price_now` is the price of the pair at the current candle.
- `entry_price` is the price at which the position was bought.

The trade is sold when `gain` is at least `target(elapsed)`, or when `gain` is at or below `-0.10`.
Nothing else in the trade decides the exit; the entry and exit signals can also close a position,
but the ladder is the part this page is about.

## A worked example

A made-up pair enters at 100.00 and the account buys 1,000 coins. The exchange charges 0.10 percent
of the amount traded on each side, so the round trip costs about 0.20 percent. The default ladder is
in force.

| Minute | Price  | Gain  | Key in force | Target | Action |
| ------ | ------ | ----- | ------------ | ------ | ------ |
| 0      | 100.00 | 0.00% | 0            | 5.0%   | hold   |
| 5      | 101.00 | 1.00% | 0            | 5.0%   | hold   |
| 10     | 102.50 | 2.50% | 0            | 5.0%   | hold   |
| 15     | 103.00 | 3.00% | 0            | 5.0%   | hold   |
| 20     | 103.50 | 3.50% | 20           | 4.0%   | hold   |
| 25     | 103.90 | 3.90% | 20           | 4.0%   | hold   |
| 30     | 103.30 | 3.30% | 30           | 3.0%   | sell   |

At minute 30 the target has fallen to three percent and the gain, 3.30 percent, is above it, so the
position is sold even though the price has just slipped from 103.90. The arithmetic:

```text
Buy      1,000 coins at 100.00 = 100,000.00, fee 0.10% = 100.00, total paid 100,100.00
Sell     1,000 coins at 103.30 = 103,300.00, fee 0.10% = 103.30, total received 103,196.70
Profit   103,196.70 - 100,100.00 = 3,096.70
Return   3,096.70 / 100,100.00 = 0.0309, about 3.09 percent
```

The gross move was 3.30 percent and the fees cost 0.20 percent, leaving about 3.09 percent. Two
things are worth noticing. First, the ladder did the work: at minute 30 a flat five percent target
would still have been waiting, and the higher target might never have been reached. Second, if the
later ladder from Strategy005 had been in force, the target at minute 30 would still have been four
percent and the trade would have stayed open, so the same prices produce different exits.

## What the research actually found

No study is published for this ladder, so there is no measured result to report. What can be said is
about its provenance. The four-key ladder appears byte for byte in Strategy001, Strategy002,
Strategy003 and Strategy004, and Strategy005 keeps the same shape and adds two earlier rungs, which
tells a reader that the numbers were copied and extended rather than measured.

The repository the files come from states in its own README that the strategies "mostly should serve
as a starting point for your own strategies, not as ready to use strategies", and that results
"heavily depend on the pairs, timeframe and timerange used to backtest". A decaying target makes the
historical result depend on those choices even more than a fixed one, because the exit is tied to
candle timing: run the same rule on one-minute candles and the ladder reaches its low rungs sooner in
wall-clock terms, so different prices decide the exit.

What would settle the question is simple to state and, in this repository, absent: take one set of
entries, close them with the ladder and with a fixed target and with a plain time limit, and compare
the three on the same data with fees included. Until that comparison exists, the ladder is a
convention, not a finding.

## How this project relates to it

This repository contains no implementation of the ROI ladder: there is no Freqtrade engine here, and
no code that reads a target that changes with time. The nearest reading is the foundations pair on
what an exit has to clear. [Return, risk and drawdown](../../foundations/05_return-risk-and-drawdown.md)
defines the profit and the fall that the ladder is trying to shape, and
[costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md) shows how a 0.20 percent round
trip turns a small target into a smaller net gain. The general problem of a rule chosen after looking
at the past is the subject of [this repository's brief on
overfitting](../../../strategies/books2/28_overfitting_and_research_integrity.md).

## Where it goes wrong

- The ladder caps the winners. A trade that rises fifty percent inside the first twenty minutes is
  sold at five percent, because the rule fires as soon as the target is touched. The strategy keeps
  the small moves and gives up the large ones.
- It behaves like a bet against the position turning. A decaying target can close a trade near
  break-even just as it begins to move, and it does so on a clock rather than on evidence.
- The asymmetry is real. The stop is fixed at minus ten percent while the target shrinks to one
  percent, so late in a trade a loss is allowed to be ten times the gain the rule is willing to take.
- Costs are charged on top of the target. On a one-percent rung the 0.20 percent round trip consumes
  a fifth of the profit, and on a thin pair the gap between buying and selling prices can be wider
  still.
- The numbers were never measured. Because the ladder is copied between files, any error or
  misfitting is copied too, and the repository warns that the files are starting points.
- It says nothing about the entry. A perfect exit rule cannot turn random entries into a profit; the
  ladder only decides when to stop.

## Try it yourself

Take a spreadsheet and one week of five-minute prices for a single crypto pair from any public chart.
You need the closing price of each candle and the time of each candle.

1. Build two columns, `time` and `close`, one row per candle.
2. Mark a made-up entry at some row and copy its price into a third column, `entry price`, down the
   sheet so every later row carries it.
3. Add a `minutes elapsed` column: the candle time minus the entry time, in minutes.
4. Add a `target` column using the default ladder: 0.05 while elapsed is under 20, 0.04 from 20 to
   29, 0.03 from 30 to 59, and 0.01 from 60 onward.
5. Add a `gain` column: close divided by entry price, minus one.
6. Add an `exit` column that says sell when gain is at least the target or at most minus 0.10, and
   stop the row-by-row walk at the first sell.
7. Repeat the whole walk with a flat target of 0.05, and once more with a plain exit after sixty
   minutes, and subtract 0.20 percent for the round trip from every result.

What to notice: the ladder almost always sells earlier than the flat target, and sometimes sells on
a price that is lower than an earlier candle. Whether the earlier exit was better depends entirely on
what the price did next, which is the point. Doing this over one week will not tell you whether the
ladder is good; it will show you how much of the outcome depends on the exit timing you chose.

## Where this came from

- [Strategy001.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/Strategy001.py),
  the source of the default ladder, the ten percent stop and the entry and exit conditions used
  above.
- [Strategy005.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/Strategy005.py),
  which keeps the same stop and timeframe but lengthens the ladder with two slower rungs.
- [The Freqtrade strategies README](https://github.com/freqtrade/freqtrade-strategies/blob/main/README.md),
  which states that the files are starting points and that results depend on the pairs, timeframe and
  period chosen.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief on what an unreported and unmeasured rule choice is worth.

## Words used in this tutorial

- candle: the opening, highest, lowest and closing price of one fixed interval of trading.
- cryptocurrency pair: two coins quoted against each other, such as Bitcoin priced in a stablecoin.
- exponential moving average: an average of recent prices that gives more weight to the newest ones.
- minimal ROI: the Freqtrade name for the ladder of profit targets, where ROI stands for return on
  investment.
- stop loss: the fixed loss at which a trade is closed, here ten percent below the entry price.
- timeframe: the length of one candle, here five minutes.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
