# Reading a five-minute chart and an hourly chart at once

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A cryptocurrency priced in a stablecoin, watched on five-minute candles, while the same coin's thirty-minute and one-hour charts and two Bitcoin ratios are read at the same moment                   |
| How often it trades       | Rarely; every one of seven conditions must point the same way, so entries are separated by long gaps                                                                                                  |
| What you need             | Nothing but this page; a spreadsheet and two series if you want to repeat the arithmetic                                                                                                              |
| Where the rules come from | [multi_tf.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/multi_tf.py), which states the rules                                                                   |
| The underlying research   | [Park and Irwin, What Do We Know About the Profitability of Technical Analysis?](https://ideas.repec.org/a/bla/jecsur/v21y2007i4p786-826.html), the survey of ninety-five tests of rules of this kind |
| How well it held up       | Weak: no measurement is published for this rule, and the survey that does exist names data snooping and rules chosen after the fact as unresolved problems                                            |
| Also appears in           | Nothing else in this collection; [using a second pair as a filter](../informative-pairs/README.md), in this same group, does the same thing across two different coins                                |

## The idea in one paragraph

Look at the same coin on two clocks at once. On the five-minute chart the coin may be briefly
oversold, meaning it has just fallen faster than usual and might bounce. On the one-hour chart the
same coin may still look oversold as well, which means the fall is not a five-minute flicker but an
hour-long move. This strategy buys only when every clock agrees that the coin is cheap, and it also
insists that Bitcoin and the Bitcoin-to-Ether ratio are cheap at the same time. It sells when the
five-minute chart is expensive again and is no longer cheaper than the hourly chart. The slower
charts are there to stop the fast chart from acting on noise.

## Why anyone believed it

A short-term rule sees only the last few minutes. Some of those minutes are real moves, but many are
a single large order, a thin moment in the order book, or the echo of an event that is already over.
A chart covering an hour cannot be pushed around by a single order, so it reports the state of the
market rather than the state of the last minute. Requiring agreement is a way of asking a second,
more patient observer whether the first one is right.

The counterparty is the trader who is forced to sell quickly and does not care about the price: a
leveraged holder who is being closed out, or a holder dumping a position near a round number. That
selling can push the five-minute chart far into oversold territory before the hour-long picture has
changed at all. If the forced seller finishes and no one else wants to sell, the price snaps back,
and a rule that waited for both clocks to agree has bought near the bottom of the snap rather than in
the middle of it.

## An everyday comparison

You are thinking of buying a bicycle from a shop that puts a red sticker on anything it wants to
clear. One sticker means little: the shop marks down one model every afternoon. But if the sticker
is on the bike and the shop's whole window display has also been repriced, and the two shops down the
road have done the same, then something is really on sale. The five-minute chart is the one red
sticker and the hourly chart is the whole window. The rule waits for the window, not the sticker.

## The rules, step by step

1. Trade on five-minute candles. A candle is the opening, highest, lowest and closing price of one
   interval, so a five-minute candle covers five minutes.
2. Compute an indicator called the relative strength index, or RSI, on the traded coin's
   five-minute closes. The RSI turns the recent gains and losses into a number from nought to one
   hundred; low numbers mean the price has been falling faster than it has been rising. This rule
   uses a fourteen-period version.
3. Compute the same fourteen-period RSI on the coin's thirty-minute candles and on its one-hour
   candles. Each of these is a separate series, calculated from that chart alone.
4. Compute four more RSI values on the one-hour chart: the fourteen-period RSI of Bitcoin against
   the stablecoin, the fourteen-period RSI of Ether priced in Bitcoin, and two fast versions of the
   Bitcoin ratio, using four periods and two periods.
5. For the entry, all of the following must hold at the close of the same five-minute candle: the
   Bitcoin hourly RSI is below thirty-five; the Ether-in-Bitcoin hourly RSI is below fifty; the
   fast four-period Bitcoin RSI is below forty; the very fast two-period Bitcoin RSI is below
   thirty; the coin's thirty-minute RSI is below forty; the coin's hourly RSI is below forty; the
   coin's five-minute RSI is below thirty; the five-minute RSI is below the hourly RSI; and volume
   is above zero.
6. If every condition holds, buy at the opening price of the next candle.
7. Sell when the five-minute RSI is above seventy, the five-minute RSI is no longer below the hourly
   RSI, and volume is above zero. The signal is allowed to fire whether the trade is ahead or
   behind, because the file does not require the exit to be in profit; the one-percent offset it
   carries is only consulted together with that requirement, so it changes nothing here.
8. A profit of twenty percent closes the trade at any moment, however long it has been open. A loss
   of ten percent closes it at any moment. Trailing stops, which follow a rising price upward, are
   switched off.
9. The file carries a note in capital letters that it is not to be used for live trading, and the
   author left it that way.

## The maths, with every symbol named

The only calculation that matters is the one behind every condition, so it is worth writing out. The
relative strength index compares recent gains with recent losses:

```text
RSI = 100 - 100 / (1 + RS)
RS  = average gain over the last N periods / average loss over the last N periods
```

- `RSI` is the number from nought to one hundred that the rules compare with thirty, forty and
  seventy.
- `RS` is the ratio of the typical gain to the typical loss over the window.
- `N` is the length of the window: fourteen periods in the rule, and three periods in the worked
  example below, shortened so the arithmetic can be followed by hand.
- `average gain` is the mean of the positive changes in the closing price over the last `N` periods;
  periods that fell count as zero.
- `average loss` is the mean of the sizes of the negative changes; periods that rose count as zero.

Two consequences follow from the formula. First, if the price never falls inside the window, the
average loss is zero, the ratio is infinite, and the RSI is one hundred. If it never rises, the RSI
is nought. Second, the number is built from the last `N` periods only, so a longer window forgets
more slowly: a one-hour RSI computed from fourteen hourly candles still contains moves that happened
fourteen hours ago, while a five-minute RSI has forgotten everything older than seventy minutes.

The rule then combines its conditions:

```text
enter = (rsi_5m < 30) and (rsi_5m < rsi_1h) and (rsi_1h < 40) and (rsi_30m < 40)
        and (btc < 35) and (eth_btc < 50) and (btc_fast < 40) and (btc_faster < 30)
```

- `rsi_5m`, `rsi_30m` and `rsi_1h` are the traded coin's RSI on the three charts.
- `btc` is the hourly fourteen-period RSI of Bitcoin against the stablecoin, and `eth_btc` is the
  hourly RSI of Ether against Bitcoin.
- `btc_fast` and `btc_faster` are the four-period and two-period hourly RSIs of the Bitcoin ratio.
- `and` means every bracket must be true at the same candle.

The cost line is the same as anywhere else. If each side of a trade costs a fraction `c` of the
amount traded, the round trip costs `2 * c`; at 0.10 percent a side that is 0.20 percent.

## A worked example

Ten five-minute closes of a coin we call AAA. The point of the example is how differently the two
clocks behave, so the RSI is computed with a three-period window and a plain average instead of the
software's fourteen-period version, which uses a smoothed average. The choice of window changes the
speed, not the mechanism.

| Candle | Close | Change | 3-period RSI (five-minute) | Hourly RSI in force | Both oversold? | Action            |
| ------ | ----- | ------ | -------------------------- | ------------------- | -------------- | ----------------- |
| 1      | 100   |        |                            | 45                  | unknown        | wait              |
| 2      | 102   | +2     |                            | 45                  | unknown        | wait              |
| 3      | 101   | -1     |                            | 45                  | unknown        | wait              |
| 4      | 103   | +2     | 80.0                       | 45                  | no             | wait              |
| 5      | 102   | -1     | 50.0                       | 38                  | no             | wait              |
| 6      | 100   | -2     | 40.0                       | 38                  | no             | wait              |
| 7      | 99    | -1     | 0.0                        | 38                  | yes            | buy at next open  |
| 8      | 98    | -1     | 0.0                        | 38                  | yes            | hold              |
| 9      | 100   | +2     | 50.0                       | 38                  | no             | hold              |
| 10     | 101   | +1     | 75.0                       | 38                  | no             | sell at next open |

The five-minute RSI at candle 4 is worth following in full, because it is the pattern for the rest:
the last three changes are plus two, minus one and plus two, so the average gain is 4 / 3 = 1.3333
and the average loss is 1 / 3 = 0.3333, the ratio is 4.0, and the index is 100 - 100 / 5 = 80.0. At
candle 7 the last three changes are all negative, so the average gain is 0 and the index is 0.0. At
candle 10 the last three changes are minus one, plus two and plus one, so the average gain is 3 / 3
= 1.0 and the average loss is 1 / 3 = 0.3333, giving a ratio of 3.0 and an index of 75.0.

The hourly RSI only moves when an hour has finished, so inside this fifty-minute table it changes
once, at candle 5. That is the whole point: the slow clock cannot be moved by a handful of quiet
minutes. The four Bitcoin conditions are assumed to be satisfied for the whole table, which is what
the rules require before any entry can exist.

The entry signal appears at candle 7, where the five-minute RSI is nought, below thirty, and the
hourly RSI is thirty-eight, below forty. The trade opens at the opening price of candle 8, which is
the closing price of candle 7, 99. The exit signal appears at candle 10, where the five-minute RSI is
seventy-five, above seventy, and no longer below the hourly thirty-eight. The trade closes at the
opening price of the next candle, which is 101.

```text
Buy   1,000 coins at 99 = 99,000, fee 0.10% = 99.00, total paid 99,099.00
Sell  1,000 coins at 101 = 101,000, fee 0.10% = 101.00, total received 100,899.00
Profit 100,899.00 - 99,099.00 = 1,800.00
Return 1,800.00 / 99,099.00 = 0.0182, about 1.82 percent
```

The price moved from 99 to 101, a gross gain of 2.02 percent, and the two fees took 0.20 percent,
leaving 1.82 percent. Notice that the slow clock did not choose the entry price, only the moment.
Removing it would not have changed this particular trade: the five-minute RSI is 50.0 at candle 5 and
40.0 at candle 6, so a rule watching only the fast chart would have waited for candle 7 as well. In
this example the hourly condition was true from candle 5 onward and removed nothing. That is what a
broad filter looks like: it demands agreement that is usually already there, and only bites on the
candles where the two clocks disagree.

## What the research actually found

There is no published study of this rule, so no result can be reported for it. What does exist is a
survey of the wider family. Park and Irwin reviewed the empirical literature on technical trading
rules and split it into early studies and modern ones. Early work found rules of this kind profitable
in foreign exchange and futures markets but not in shares. Among ninety-five modern studies,
fifty-six reported positive results, twenty reported negative results and nineteen reported mixed
results, and the positive results run at least until the early 1990s.

Those authors then say what the numbers are worth. Most of the studies, in their words, suffer from
problems in the way they were tested: data snooping, which means searching many rules on the same
data until one looks good; choosing the rule after seeing the results; and difficulty in measuring
the risk taken and the costs paid. Their conclusion is that the positive evidence is not conclusive
until those deficiencies are fixed. A seven-condition rule with unexplained thresholds of thirty,
thirty-five, forty and fifty is precisely the kind of object those criticisms are aimed at, and the
file itself gives no measurement.

Two further facts belong here. The relative strength index itself was published by J. Welles Wilder
in his 1978 book, New Concepts in Technical Trading Systems, as a description of a chart-reading
habit rather than as a tested forecast. And the practice of reading several timeframes at once is
standard in chart-based trading, where it is described as top-down analysis; it is a way of
organising attention, and organising attention is not the same as measuring an edge.

## How this project relates to it

This repository holds no Freqtrade engine and no implementation of this rule. The closest material
is its writing on the two ways this idea fails. [How a backtest lies](../../foundations/07_how-a-backtest-lies.md)
explains how a rule tested on the whole history at once can quietly read a candle that had not
finished, which is the specific hazard of merging a slow chart into a fast one.
[Regimes and why nothing lasts](../../foundations/08_regimes-and-why-nothing-lasts.md) explains why a
threshold that worked in one market state stops working in the next, which is what an unmeasured
threshold like thirty-five means in practice. The brief on what a wide search over rules and
parameters does to a result is
[Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).

## Where it goes wrong

- Multiplication of conditions shrinks the sample. Seven conditions, each of which a market obeys
  maybe a third of the time, produce very few trades, and few trades make any result unreliable.
- The slow clock is old news. By the time a one-hour RSI is below forty, the fall it describes began
  an hour ago. The rule buys the aftermath, and the rule's own exit, which waits for a five-minute
  RSI above seventy, then waits for a bounce that may never come.
- The thresholds are unexplained. Thirty, thirty-five, forty and fifty are round numbers with no
  stated reason, and the file gives no measurement that would show one of them is better than
  another.
- The reward and the risk are mismatched. Taking twenty percent and stopping at ten percent sounds
  even, but with an entry that waits for an oversold reading the more likely path is the stop, and
  the target is rarely reached.
- The exit signal is strict. Selling needs a five-minute RSI above seventy and no longer below the
  hourly value at the same candle, which may not happen before the price has turned and fallen back.
- The author's own warning is the strongest evidence on the page. The file opens with the comment
  that it is not to be used for live trading, written in capital letters with six exclamation marks.
- Crowding and data snooping. Coins, timeframes and thresholds are a large space, and a rule that
  looks good after searching that space is exactly the object the Park and Irwin survey warns about.

## Try it yourself

You need a spreadsheet and two series of closes for one coin: five-minute candles and one-hour
candles, both from a public chart.

1. In the five-minute sheet, build columns `time`, `close` and `rsi 5m`, computing the RSI over
   fourteen changes with whatever rolling formula your spreadsheet supports.
2. In the hourly sheet, build `time`, `close` and `rsi 1h`.
3. Copy the hourly RSI into the five-minute sheet by matching each five-minute time to the most
   recent hour that has already finished. Never use the hour that is still running.
4. Add an `enter` column that says buy when `rsi 5m` is below 30, `rsi 1h` is below 40, and
   `rsi 5m` is below `rsi 1h`; add an `exit` column that says sell when `rsi 5m` is above 70 and is
   no longer below `rsi 1h`.
5. Walk the rows and record every trade and its return after subtracting 0.20 percent.
6. Do the walk a second time using only the five-minute conditions, with the hourly requirement
   removed.
7. Count the trades in each version and write down the difference.

What to notice: removing the hourly requirement roughly doubles the number of trades and makes almost
all of the difference in the result. Then look at whether the extra trades, in the no-filter version,
were bought in the middle of a fall or near its end. That distinction is the entire claim of the
rule, and one week of one coin cannot settle it. If you want a harder version of the exercise, repeat
the whole thing with the thresholds you would have chosen before looking at the data, and compare.

## Where this came from

- [multi_tf.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/multi_tf.py),
  the source of the seven entry conditions, the exit condition, the twenty percent target and the ten
  percent stop.
- [The Freqtrade strategy documentation](https://www.freqtrade.io/en/stable/strategy-customization/),
  which explains how a strategy reads a second timeframe and warns that in a backtest the whole
  history is handed to the rule at once, so care is needed to avoid using future data.
- Cheol-Ho Park and Scott H. Irwin,
  [What Do We Know About the Profitability of Technical Analysis?](https://ideas.repec.org/a/bla/jecsur/v21y2007i4p786-826.html),
  Journal of Economic Surveys 21(4), pages 786 to 826 (2007), for the count of ninety-five modern
  studies and the criticism of their testing procedures.
- [How a backtest lies](../../foundations/07_how-a-backtest-lies.md),
  [Regimes and why nothing lasts](../../foundations/08_regimes-and-why-nothing-lasts.md) and
  [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own writing on the three failure modes this rule is exposed to.

## Words used in this tutorial

- candle: the opening, highest, lowest and closing price of one fixed interval of trading.
- indicator: a number calculated from past prices, such as an average or an index, used as an input
  to a rule.
- oversold: a reading that says a price has fallen faster than usual, which some traders expect to
  be followed by a bounce.
- relative strength index: the nought-to-one-hundred number built from recent gains and losses, where
  low values mean recent falls, and usually shortened to RSI.
- timeframe: the length of one candle, here five minutes for the traded chart and one hour for the
  slow chart.
- volatility: how much a price moves around, usually quoted as a yearly percentage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
