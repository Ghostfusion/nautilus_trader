# The smallest strategies, and the one that deliberately does nothing

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto pairs on a crypto exchange, such as Bitcoin priced in United States dollars; the do-nothing file trades nothing at all                                                                                                                                                                                                                                                                  |
| How often it trades       | The two live files can fire many times a day, because they watch one-minute and five-minute candles; the do-nothing file never trades                                                                                                                                                                                                                                                          |
| What you need             | Nothing but this page to understand the do-nothing file; a spreadsheet to follow the two live ones                                                                                                                                                                                                                                                                                             |
| Where the rules come from | [Simple.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/Simple.py), [Scalp.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/Scalp.py) and [DoesNothingStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/DoesNothingStrategy.py) |
| The underlying research   | For Simple.py, the book "The Simple Strategy" by Markus Heitkoetter and Mark Hodge ([free book chapter](https://rockwell-files.s3.amazonaws.com/ebook-Simple-Strategy.pdf)); the other two have none                                                                                                                                                                                           |
| How well it held up       | Weak: none of the three carries any published measurement, and the one that could be measured, the do-nothing file, is a test rather than a strategy                                                                                                                                                                                                                                           |
| Also appears in           | [ASDTS Rockwell](../asdts-rockwell/README.md), which implements the same published system, and [the ROI ladder](../roi-target-ladder/README.md), which explains the profit target all these files share                                                                                                                                                                                        |

## The idea in one paragraph

A strategy library needs a smallest example that a beginner can read in one sitting. This repository
ships three of them. `Simple.py` is the shortest complete trading rule it has: it watches a few
standard indicators and buys when several of them agree that a price is rising fast. `Scalp.py` is
the shortest rule built for speed: it watches one-minute candles, buys small dips, and aims to take a
tiny profit many times. `DoesNothingStrategy.py` is the odd one out: it contains no buy rule and no
sell rule, because its entire purpose is to trade nothing and show zero. It is the control.

## Why anyone believed it

The two live files each rest on a familiar story. `Simple.py` follows a published day-trading system:
when a short-term trend is up and the price has been pushed hard, the buyers may keep buying for a
little longer, so a trader who joins the move can ride it for a short while. `Scalp.py` follows a
different story: most of a fast move is noise that snaps back, so a rule that buys a sudden dip and
sells the first small bounce can collect a little from each swing, many times a day.

The do-nothing file rests on no story at all, and that is the point. Somebody had to write the
smallest file the bot will accept, to check that the machinery works and to serve as the baseline
every other strategy is measured against. It contains no opinion about the market, so any result it
produces should be nothing.

## An everyday comparison

Imagine a hospital testing a new medicine. Half the patients get the medicine and half get a sugar
pill that looks identical. The sugar-pill group is the control: if the patients who took the sugar
pill recover just as often as the others, the medicine did not do anything. Nobody expects the sugar
pill to cure anyone, and nobody is disappointed when it does not. A strategy that trades nothing is
the sugar pill of a backtest library. If a backtest reports a profit for it, the profit came from a
mistake somewhere in the test, not from the market.

## The rules, step by step

`Simple.py`, on five-minute candles:

1. Compute three indicators on the recent prices: MACD, which compares a fast and a slow moving
   average; RSI over seven candles, a number from 0 to 100 for how one-sided the recent moves have
   been; and Bollinger bands over twelve candles, an upper and lower rail drawn two standard
   deviations away from an average.
2. Buy when all four of these are true at once: the MACD is above zero, the MACD is above its own
   signal line, the upper Bollinger rail is higher than it was one candle ago, and the RSI is above
   70.
3. Sell when the RSI rises above 80.
4. Also sell when the trade's gain reaches one percent, because the file sets the profit target to
   1 percent at every elapsed time; and sell when the trade is 25 percent below its entry, the stop
   loss.

`Scalp.py`, on one-minute candles:

1. Compute a five-candle exponential moving average of the high, of the low and of the close; a
   fast stochastic oscillator, which is a pair of numbers from 0 to 100 comparing the closing price
   with the recent range; and ADX, a number for how strong a trend is.
2. Buy when all of these are true: the candle opened below the five-candle average of the lows, the
   ADX is above 30, both parts of the stochastic are below 30, and the faster part crosses above the
   slower one.
3. Sell when the candle opens at or above the five-candle average of the highs, or when either
   stochastic part crosses above 70, or when the profit target or stop loss is reached.
4. The file recommends holding at least sixty trades open at once and selling only on the profit
   target.

`DoesNothingStrategy.py`, on five-minute candles:

1. Compute nothing.
2. Never buy.
3. Never sell.

The file states its two exit levels in a peculiar way that is worth reading slowly: the profit target
is `100000`, and the stop loss is `-1`. Both are so far away that neither can be reached. The entry
rule and the exit rule are each an empty condition, so no candle is ever marked as a buy or a sell.

## The maths, with every symbol named

A Freqtrade file states its exit in two parts, and both are just numbers.

The profit target, called the minimal return on investment:

```text
minimal_roi = {"0": t}
```

- `t` is the gain, as a fraction, at which the position is closed. The key `0` means the target
  applies from the moment the trade opens.
- The file's `1`-percent target is written `0.01`, because 0.01 of an amount is one percent of it.
- The do-nothing file's `100000` is 10,000,000 percent, which no real price move reaches.

The stop loss:

```text
stoploss = -s
```

- `-s` is the loss, as a fraction, at which the position is closed. `-0.25` means a loss of 25
  percent of the amount invested, and the do-nothing file's `-1` means a loss of 100 percent, the
  whole position.

The entry and exit rules are conditions on each candle. A rule such as `Simple.py`'s can be written
as a row of yes-or-no answers:

```text
enter = (macd > 0) and (macd > signal) and (band > band_one_candle_ago) and (rsi > 70)
```

- `enter` is the answer for one candle: buy it or not.
- `macd`, `signal`, `band` and `rsi` are the indicator values for that candle, each defined in the
  rules above; `band` is the upper Bollinger rail.
- `and` means every part must be true; one false part makes the whole answer false.
- The do-nothing file has no parts between the brackets, so `enter` is false on every candle and the
  number of trades is exactly zero.

The money arithmetic for a single trade is the same as for any strategy:

```text
gain = (price_now / entry_price) - 1
```

- `gain` is the trade's progress as a fraction: 0.01 means one percent.
- `price_now` is the current price of the pair, and `entry_price` is the price at which it was bought.

The round-trip cost of one trade, with the exchange fee charged on each side:

```text
cost = spread + 2 * fee
```

- `spread` is the gap between the price at which you can buy and the price at which you can sell,
  paid once per round trip.
- `fee` is the exchange's charge per side, written as a fraction of the amount traded; `2 * fee`
  because a buy and a sell are two sides.

## A worked example

The point of the do-nothing file is easiest to see on a table. Below are eight made-up five-minute
candles for one coin pair. The two indicator columns are invented numbers of the size these
indicators actually take, included so the table reads like a real chart; the price column is also
invented.

| Candle | Close  | MACD  | Signal | RSI | Simple says buy |
| ------ | ------ | ----- | ------ | --- | --------------- |
| 1      | 100.00 | -0.20 | -0.10  | 45  | no              |
| 2      | 100.60 | 0.05  | 0.02   | 60  | no              |
| 3      | 101.40 | 0.30  | 0.15   | 72  | yes             |
| 4      | 102.00 | 0.55  | 0.30   | 78  | yes             |
| 5      | 102.30 | 0.62  | 0.45   | 81  | yes             |
| 6      | 101.80 | 0.50  | 0.48   | 76  | no              |
| 7      | 101.20 | 0.30  | 0.40   | 68  | no              |
| 8      | 100.70 | 0.10  | 0.25   | 61  | no              |

`Simple.py` buys at candle 3 and would not buy again on candles 4 or 5, because it already holds a
position; on candle 5 the RSI of 81 is above 80, so its exit rule sells. Suppose the account bought
1,000 coins at 101.40 and sold at 102.30, and the exchange charges a fee of 0.10 percent per side
while the gap between buying and selling prices is 0.10 percent.

```text
Buy      1,000 coins at 101.40 = 101,400.00, fee 0.10% = 101.40, total paid 101,501.40
Sell     1,000 coins at 102.30 = 102,300.00, fee 0.10% = 102.30, total received 102,197.70
Spread   the round trip crosses a 0.10% gap = about 101.90
Profit   102,197.70 - 101,501.40 - 101.90 = 594.40
Return   594.40 / 101,501.40 = 0.00586, about 0.59 percent
```

Now run `DoesNothingStrategy.py` over the same eight candles. Its entry condition is empty, so it
never marks a buy; its exit condition is empty, so it never marks a sell. The number of trades is
zero, the amount bought is zero, and the profit before costs is zero. Any backtest that reports a
nonzero profit for it is telling you that something in the machinery, not the market, produced the
number. Two things are worth noticing. First, the do-nothing file is the only one of the three whose
correct answer is known in advance, which is exactly what makes it useful. Second, the trade in the
table is a made-up one, so it says nothing about whether the rule earns money over a real year; it
only shows how the arithmetic runs.

## What the research actually found

There is no published measurement for any of these three files, so there is no result to report.

The book behind `Simple.py` presents its own result as an illustration, not a study. It asks the
reader to imagine ten trades, five winners at 150 dollars and five losers at 100 dollars, and shows a
profit of 250 dollars before commissions, or about 200 dollars after. That is a worked assumption
about a hypothetical win rate, not a measurement on any market, and it has no sample, no period and
no cost model behind it.

For the other two files there is even less. `Scalp.py` states that the reader should keep at least
sixty trades open to cover the losses, which is a suggestion for spreading risk, not evidence that
the rule earns anything. `DoesNothingStrategy.py` calls itself "just a skeleton".

What is measurable is the repository's own disclaimer. Its README says the strategies "mostly should
serve as a starting point for your own strategies, not as ready to use strategies", and that results
"heavily depend on the pairs, timeframe and timerange used to backtest". Taken at face value, that
warning applies to every file here, and it applies most sharply to the thinnest ones.

## How this project relates to it

The idea of a control is what makes the do-nothing file more than a curiosity, and this repository
covers the reason a control matters.
[How a backtest lies](../../foundations/07_how-a-backtest-lies.md) lists the standard ways a
simulated history can mislead, including look-ahead bias and a backtest that assumes every order
fills at the price on screen. A strategy that trades nothing is the simplest instrument for catching
those faults: if the machinery reports profit where there is no trade, the fault is in the machinery.

The second related piece is the repository's study of sector rotation, which used a plain
buy-and-hold holding as the yardstick for hundreds of rotation rules
([Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md)). A baseline is
the same idea as a control in a different costume: something simple and known, against which the
fancier rule is judged. The study found that most of its 1,022 rules did worse than simply holding
the market, which is the kind of result a control is designed to expose.

## Where it goes wrong

- A broken backtest can make the do-nothing file look profitable, and that is its job. If it does,
  the fault is usually a fee, a fill rule or a data problem, and every other result from the same
  backtest is suspect too.
- The empty condition is silent. In the code the entry rule is an empty bracket, which sets no buy
  flag; a reader who edits the file carelessly can fill the bracket by accident and give it a rule
  it was never meant to have.
- The two live rules are opposite in spirit. One buys a strong rising price and the other buys a
  sharp dip, so a reader who thinks they are the same kind of strategy will misread both.
- The profit targets are far apart. `Simple.py` and `Scalp.py` aim for one percent while
  `DoesNothingStrategy.py` sets ten million percent, so the three files are not comparable by their
  exit settings at all.
- Tuning erases the control's value. A parameter search run over a do-nothing file will still report
  a best set of numbers, which shows how easily a search manufactures a result from nothing.
- The repository's own warning stands. None of these files is offered as a strategy to trade, and
  the book's arithmetic is an illustration rather than a measurement.

## Try it yourself

You need a spreadsheet and one week of five-minute prices for a single crypto pair from any public
chart. The exercise is to fake a backtest and then catch it lying.

1. Build two columns, `time` and `close`, one row per five-minute candle.
2. In a third column called `buy`, put a `1` against a handful of rows of your choosing, and leave
   the rest blank. This stands for any rule at all.
3. In a fourth column called `sold`, put a `1` a few rows after each buy, and leave the rest blank.
4. Add a fifth column called `do-nothing buy`, and fill it with zeros on every row.
5. Count the trades: the number of `1`s in the `buy` column, and the number of `1`s in `do-nothing
   buy`, which must be zero.
6. Add a column that computes the gain of each pretended trade, and a row at the bottom that adds
   them up and subtracts 0.20 percent per trade for the round trip.

What to notice: the do-nothing column totals exactly zero no matter what the prices did, while the
made-up `buy` column totals whatever your chosen rows happened to produce. That contrast is the
whole lesson. A number is only meaningful next to a control that should produce nothing; if your
control produces something, stop and find out why before reading any other result.

## Where this came from

- [Simple.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/Simple.py),
  the shortest complete rule set in the repository.
- [Scalp.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/Scalp.py),
  the shortest fast-trading rule set.
- [DoesNothingStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/DoesNothingStrategy.py),
  the control that trades nothing.
- [The Simple Strategy](https://rockwell-files.s3.amazonaws.com/ebook-Simple-Strategy.pdf), Markus
  Heitkoetter and Mark Hodge, the book whose rules `Simple.py` simplifies; its ten-trade example is
  the illustration quoted above.
- [The Freqtrade strategies README](https://github.com/freqtrade/freqtrade-strategies/blob/main/README.md),
  which states that the files are starting points and that results depend on the pairs, timeframe
  and period chosen.
- [How a backtest lies](../../foundations/07_how-a-backtest-lies.md), this repository's own page on
  the ways a simulated history deceives, which is what the do-nothing control is built to catch.

## Words used in this tutorial

- candle: the opening, highest, lowest and closing price of one fixed interval of trading.
- control: a deliberately empty or neutral case, used to check that a test measures what it claims.
- cryptocurrency pair: two coins quoted against each other, such as Bitcoin priced in a stablecoin.
- exponential moving average: an average of recent prices that gives more weight to the newest ones.
- indicator: a number calculated from past prices, such as a moving average, used as a rule input.
- MACD: a pair of averages of the price whose difference and average are used as an indicator.
- minimal ROI: the Freqtrade name for the profit target, where ROI stands for return on investment.
- RSI: a number from 0 to 100 describing how one-sided the recent price moves have been.
- stop loss: the fixed loss at which a trade is closed to limit the damage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
