# The Simple Strategy: a published day-trading system, cut down to one MACD rule

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                               |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto pairs on a crypto exchange, such as Bitcoin priced in United States dollars                                                                                                                                                                                  |
| How often it trades       | A few times a day; it watches five-minute candles and holds a position while the trend it detected lasts                                                                                                                                                            |
| What you need             | A spreadsheet, plus the section below if you want to follow the averaging by hand                                                                                                                                                                                   |
| Where the rules come from | [ASDTSRockwellTrading.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/ASDTSRockwellTrading.py), which names the video [A Simple Day Trading Strategy](https://www.youtube.com/watch?v=mmAWVmKN4J0) as its source |
| The underlying research   | The book "The Simple Strategy" by Markus Heitkoetter and Mark Hodge ([free book chapter](https://rockwell-files.s3.amazonaws.com/ebook-Simple-Strategy.pdf)), the published system the video and the file both follow                                               |
| How well it held up       | Weak: the book gives an illustrative ten-trade example rather than a measured backtest, and the crypto file's own adaptation is never measured at all                                                                                                               |
| Also appears in           | [The smallest strategies](../minimal-examples/README.md), whose Simple.py implements the same book, and [the ROI ladder](../roi-target-ladder/README.md), which explains the falling profit target this file uses                                                   |

## The idea in one paragraph

There is a published day-trading system called "The Simple Strategy", taught by Markus Heitkoetter of
Rockwell Trading and written up in a book of the same name. Its central idea is that a short-term
trend is easiest to follow when two averages of the price have lined up in the same direction, so it
buys when that alignment is present and gets out when it breaks. This repository ships a
cryptocurrency file, `ASDTSRockwellTrading.py`, that keeps only the heart of the system: it watches a
standard indicator called MACD on five-minute candles, buys when that indicator is above zero and
above its own signal line, and sells when it falls back below the signal line. The file also sets a
profit target that shrinks as the trade gets older, and a fixed stop loss.

## Why anyone believed it

Trends in prices arise because news and money arrive gradually. When a piece of good news reaches a
market, the first buyers push the price up, and later buyers, who hear it afterwards, keep pushing.
The MACD is designed to detect the moment this has begun: it measures the gap between a fast average
of the price, which reacts to new moves quickly, and a slow average, which reacts slowly. When the
fast average is above the slow one, recent prices are higher than the older ones, which is what a
young uptrend looks like. When the indicator is also above zero, the move has been large enough to
clear the long-run average, which the system's author treats as confirmation.

The counterparty is the trader who sells too early. Someone who bought before the move and takes a
small profit the moment the price ticks up is supplying the shares that the trend follower buys, and
they keep doing it because their own rule tells them to. If enough sellers behave that way, a young
trend keeps going for a little longer, which is the small window the system tries to catch.

## An everyday comparison

Think of a bus route where the timetable matters. When one bus is delayed, the passengers at the stop
pile up. The next bus to arrive is fuller than usual and stops for longer, which delays it too, so
the bus behind it finds a busier stop and is delayed in turn. A passenger who joins at the moment
the pile-up starts gets a quick ride down the route before the crowd thins out. The Simple Strategy
joins a price trend at the moment the fast average overtakes the slow one, on the belief that the
pile-up of buyers is only just beginning.

## The rules, step by step

`ASDTSRockwellTrading.py`, on five-minute candles:

1. Compute the MACD from the recent prices. It is built from two averages of the price, a fast one
   and a slow one, and a third line called the signal, which is an average of the MACD itself.
2. Buy when both of these are true at once: the MACD is above zero, and the MACD is above its signal
   line. The file's own comment calls this an uptrend.
3. Sell when the MACD is below its signal line. The file's own comment calls this the sell rule, and
   it is the only sell signal the file states.
4. Also sell when the trade's gain reaches a target that falls as the trade ages: 5 percent while the
   trade is under 20 minutes old, 4 percent from 20 to 29 minutes, 3 percent from 30 to 59 minutes,
   and 1 percent from 60 minutes onward. This is the `minimal_roi` ladder
   `{"60": 0.01, "30": 0.03, "20": 0.04, "0": 0.05}`, read from the largest minute mark the trade has
   passed.
5. Also sell when the trade is 30 percent below its entry price. That is the `stoploss` of `-0.3`.

For comparison, the published system the file is cut down from states four things the file leaves
out. It uses Bollinger bands over 12 candles at 2 standard deviations and takes an entry only when
the upper band is pointing up and the price closes at or near it; it uses an optional RSI over 7
candles, above 70, as a strength filter; it places a buy-stop order one tick above the high of the
signal candle; and it sets the profit target and the stop loss as fixed fractions of the market's
average daily range, 15 percent and 10 percent respectively.

## The maths, with every symbol named

The indicator at the centre of the file is the MACD, which is short for moving average convergence
divergence. It was published by Gerald Appel in the late 1970s and is built from two exponential
moving averages of the price. An exponential moving average is an average that leans on the newest
prices:

```text
EMA_today = k * price_today + (1 - k) * EMA_yesterday
```

- `EMA_today` is the average for the current candle.
- `price_today` is that candle's closing price.
- `EMA_yesterday` is the average from the previous candle.
- `k` is the weight given to the newest price, equal to `2 / (n + 1)`, where `n` is the number of
  candles in the average. A short `n` gives a jumpy average; a long `n` gives a smooth one.

The MACD is the gap between a fast average and a slow average:

```text
MACD = EMA_fast - EMA_slow
```

- `EMA_fast` is the exponential average over the short window, 12 candles in the library default the
  file uses.
- `EMA_slow` is the exponential average over the long window, 26 candles by the same default.
- The difference is positive when recent prices sit above the older ones, which the file calls an
  uptrend once it is also above zero.

The signal line is an exponential average of the MACD itself, over 9 candles by the same default:

```text
signal = EMA(MACD, 9)
```

- `signal` smooths the MACD so that a single jumpy candle does not flip the rule.
- The file buys when `MACD > 0` and `MACD > signal`, and sells when `MACD < signal`.

The falling target is read from the ladder, exactly as on the
[ROI ladder](../roi-target-ladder/README.md) page:

```text
target(elapsed) = ladder[K], where K is the largest key not greater than elapsed
```

- `elapsed` is the number of whole minutes since the trade opened.
- `ladder` is the list of minute-and-gain pairs above.
- `K` is the largest minute mark the trade has passed, so the target is 5 percent for the first 20
  minutes, 4 percent for the next 10, 3 percent until minute 59, and 1 percent from minute 60 on.

The worked result of a trade is its gain less the round-trip cost:

```text
gain = (price_now / entry_price) - 1
cost = spread + 2 * fee
net  = gain - cost
```

- `gain` is the trade's progress as a fraction.
- `spread` is the gap between the best buying and selling prices, paid once per round trip.
- `fee` is the exchange's charge per side, so two sides are paid, `2 * fee`.
- `net` is what reaches the account after costs.

## A worked example

The arithmetic is easiest to follow with short averaging windows, so the table below uses a fast
average of 3 candles and a slow average of 6, with a signal average of 3, instead of the library's
12, 26 and 9. The method is identical; the numbers are simply hand-checkable. The prices are invented
but of the size a five-minute candle actually shows.

| Candle | Close  | EMA fast | EMA slow | MACD   | Signal | Rule says |
| ------ | ------ | -------- | -------- | ------ | ------ | --------- |
| 1      | 100.00 | 100.000  | 100.000  | 0.0000 | 0.0000 | nothing   |
| 2      | 100.40 | 100.200  | 100.114  | 0.0857 | 0.0429 | buy       |
| 3      | 100.90 | 100.550  | 100.339  | 0.2112 | 0.1270 | buy       |
| 4      | 101.60 | 101.075  | 100.699  | 0.3759 | 0.2515 | buy       |
| 5      | 102.40 | 101.738  | 101.185  | 0.5524 | 0.4019 | buy       |
| 6      | 103.30 | 102.519  | 101.789  | 0.7294 | 0.5657 | buy       |
| 7      | 104.20 | 103.359  | 102.478  | 0.8813 | 0.7235 | buy       |
| 8      | 104.90 | 104.130  | 103.170  | 0.9596 | 0.8415 | buy       |
| 9      | 105.30 | 104.715  | 103.779  | 0.9362 | 0.8889 | buy       |
| 10     | 105.60 | 105.157  | 104.299  | 0.8584 | 0.8736 | sell      |

The buy condition is first true at candle 2 and the position is entered at 100.40. From there the
gain is `price_now / 100.40 - 1` on each candle. At candle 8 the elapsed time is `6 * 5 = 30`
minutes, so the ladder's key is 30 and the target is 3 percent; the gain there is
`104.90 / 100.40 - 1 = 4.48` percent, which is above the target, so the position is sold at 104.90.
The sell signal itself arrives only at candle 10, where `0.8584 < 0.8736`, so in this example the
ladder exits first, two candles and a lower price before the trend actually turned.

The money, for an account that bought 1,000 units, with a fee of 0.10 percent per side and a spread
of 0.10 percent:

```text
Buy      1,000 at 100.40 = 100,400.00, fee 0.10% = 100.40, total paid 100,500.40
Sell     1,000 at 104.90 = 104,900.00, fee 0.10% = 104.90, received 104,795.10
Fees     buy fee 100.40 plus sell fee 104.90 = 205.30 in total
After   104,795.10 - 100,500.40 = 4,294.70, or 4.27 percent of the amount paid, once fees are paid
Cost    the spread as well: 0.10% of about 102,650 = 102.65
Net     4,294.70 - 102.65 = 4,192.05, or 4.17 percent of the amount paid
```

Two things are worth noticing. First, the ladder sold the trade 4.48 percent ahead when a flat
5 percent target would have waited; the trade eventually reached 5.18 percent, so the falling target
gave up a little of the move in exchange for closing sooner. Second, the gain and the costs are both
small fractions, so the spread of 0.10 percent is a visible slice of the 4.5 percent gross result.
This example is invented to show the arithmetic; it is not a measurement, and it does not say whether
the rule earns over a real year.

## What the research actually found

There is no measured result behind this file, and the system it comes from does not publish one
either. The book's evidence is an illustration rather than a study: it asks the reader to imagine ten
trades with five winners at 150 dollars each and five losers at 100 dollars each, and shows a profit
of 250 dollars before commissions, about 200 dollars after. From that it concludes that the system
can make money with a winning percentage of only 50. That arithmetic is correct as arithmetic, but
the 150-to-100 reward-to-risk ratio is an assumption about the exits, not a measurement of how often
the entries win.

The book does state a specific mechanism for those exits: the profit target is 15 percent of the
market's Average Daily Range and the stop loss is 10 percent of it, so the intended reward is one and
a half times the risk. The crypto file replaces that with far different numbers, a target of 1 to 5
percent and a stop of 30 percent, so the adaptation actually risks several times more than it aims to
make. Whether the entries still have an edge under that change is exactly the question the file does
not answer.

The repository's own README states that the files "mostly should serve as a starting point for your
own strategies, not as ready to use strategies", and that results "heavily depend on the pairs,
timeframe and timerange used to backtest". The video the file cites is a teaching video for the
day-trading system, not a study of the cryptocurrency version.

## How this project relates to it

This repository has no Freqtrade engine and no code that runs this MACD rule, so nothing here
implements it. The closest companions are the other pages in this collection that explain the parts.
[The smallest strategies](../minimal-examples/README.md) shows `Simple.py`, which takes the same
book's entry conditions and keeps two of them (Bollinger and RSI) that this file drops, so reading
the two together shows what the file chose to remove. [The ROI ladder](../roi-target-ladder/README.md)
explains the shrinking-target exit in more detail than this page can, including why a target tied to
a clock rather than to evidence is hard to justify.

The wider question of whether a trend-following entry survives its costs is covered by this
repository's brief on overfitting and research integrity, at
[strategies/books2/28_overfitting_and_research_integrity.md](../../../strategies/books2/28_overfitting_and_research_integrity.md)
which is where the demand for a measured, out-of-sample result comes from.

## Where it goes wrong

- The reward and the risk are inverted. The published system aimed for one and a half times its risk;
  the crypto file aims for 1 to 5 percent while risking 30 percent, so a small number of losses
  outweighs a large number of wins.
- The MACD is a lagging indicator. Averages are built from prices that already happened, so the
  signal appears after the move has begun, which is precisely when many of the buyers have already
  arrived and the price is about to pull back.
- Almost every trend signal is duplicated many times. The MACD sits above its signal for long
  stretches, so the entry condition is true far more often than a single trade can use, and the
  result depends heavily on exactly when the bot first notices.
- The entry was never tested in the shortened form. The book's entry also required a rising Bollinger
  band and, optionally, a strong RSI reading, and the file removed both, so the file's rule is not
  the tested rule.
- Costs are charged on every flip. A rule that sells the moment the MACD dips below its signal can
  trade very often, and each flip pays the spread and the fee on both sides.
- The book's example is not evidence. A ten-trade illustration with an assumed win rate can be made
  to show any result by changing the assumed numbers, and it says nothing about the real distribution
  of wins and losses.

## Try it yourself

You need a spreadsheet and a public chart of a crypto pair with five-minute candles. The exercise is
to compute the two averages by hand and watch the indicator flip.

1. Copy about sixty closing prices, one row per five-minute candle, into a `close` column.
2. In a second column called `price change`, subtract the previous close from the current one, then
   set the first row's change to zero.
3. In a third column called `MACD`, which is an approximation of the library's 12, 26 and 9, add
   one tenth of the current price change to the previous MACD value: start the column at zero and
   carry each row forward.
4. In a fourth column called `signal`, apply the same one tenth rule to the MACD column to smooth it
   again.
5. In a fifth column called `hold`, write yes on every row where MACD is above zero and above signal,
   and no on every row where MACD is below signal.
6. Count the number of times the `hold` column changes from yes to no, and multiply by the round-trip
   cost you measured from the pair's buying and selling prices.

What to notice: the MACD approximation is still above its signal long after the price has stopped
rising, because it is built from averages that only turn slowly. That delay is why the signal is
never early, and why the number of flips, and therefore the total cost, is larger than a glance at
the chart suggests.

## Where this came from

- [ASDTSRockwellTrading.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/ASDTSRockwellTrading.py),
  the file this page describes, including its comment defining an uptrend as MACD above zero and
  above its signal.
- [A Simple Day Trading Strategy](https://www.youtube.com/watch?v=mmAWVmKN4J0), the Rockwell Trading
  video the file names as its source.
- [The Simple Strategy](https://rockwell-files.s3.amazonaws.com/ebook-Simple-Strategy.pdf), Markus
  Heitkoetter and Mark Hodge, the published system: MACD above zero and above signal, a rising upper
  Bollinger band over 12 candles at 2 standard deviations, an optional RSI over 7 above 70, and
  exits at 15 percent and 10 percent of the Average Daily Range.
- [MACD](https://en.wikipedia.org/wiki/MACD), the plain-language description of the indicator and its
  origin with Gerald Appel.
- [The Freqtrade strategies README](https://github.com/freqtrade/freqtrade-strategies/blob/main/README.md),
  which states that the files are starting points and that results depend on the pairs, timeframe and
  period chosen.

## Words used in this tutorial

- average daily range: the average of how far a market moves from its high to its low over recent
  days, used to set exits that scale with the market's normal movement.
- Bollinger bands: a pair of rails drawn a fixed number of standard deviations above and below a
  moving average of the price.
- exponential moving average: an average of recent prices that gives more weight to the newest ones.
- MACD: a pair of averages of the price whose difference, and the average of that difference, are
  used as an indicator of trend.
- moving average: the average of the last few prices, recalculated as time moves on.
- RSI: a number from 0 to 100 describing how one-sided the recent price moves have been.
- signal line: a smoothed copy of the MACD, used as the level the MACD must cross to give a signal.
- stop loss: the fixed loss at which a trade is closed to limit the damage.
- trend: a stretch in which a price moves mostly in one direction.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
