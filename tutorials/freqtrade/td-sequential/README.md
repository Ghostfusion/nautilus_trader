# Counting down: the TD Sequential rule that buys on the ninth lower close

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Cryptocurrency pairs on a spot exchange                                                                                                                                                  |
| How often it trades       | Occasionally: a completed downward count of nine takes about two days of hourly candles, and they are not common                                                                         |
| What you need             | A spreadsheet and a table of hourly open, high, low and close prices                                                                                                                     |
| Where the rules come from | [TDSequentialStrategy.py in the freqtrade strategies repository](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/TDSequentialStrategy.py) |
| The underlying research   | Tom DeMark, the Sequential indicator, described in [The New Science of Technical Analysis](https://catalogimages.wiley.com/images/db/pdf/9781576603147.excerpt.pdf) (1994)               |
| How well it held up       | Weak: a named counting system with a practitioner following, tested here once by its author with no independent test and no cost line                                                    |
| Also appears in           | nothing else in this collection                                                                                                                                                          |

## The idea in one paragraph

Instead of looking at the shape of one candle, this rule counts. Each hour it asks a single
question: is the price lower than it was four hours ago? If yes, it adds one to a running count; if
no, it resets the count to zero. When nine such hours arrive in a row, the rule treats the market as
exhausted and buys. The idea is that a long run of falling prices happens because the same sellers
keep selling, and that after nine steps the sellers have mostly finished, so the price is due to
turn. A second, optional condition asks the ninth hour's lowest price to dip below the lowest price
of the sixth or seventh hour of the count, which the system calls a perfect setup. The position is
closed when the opposite count of nine higher closes appears, or by a fixed loss limit.

## Why anyone believed it

A run of lower closes looks like a trend, but the person running this rule reads it as a queue that
is emptying. Everyone who wanted to sell for the original reason has sold, so the pressure that
pushed the price down is nearly spent, and the smallest buying interest can lift the price. The
counterparty is the late seller: the trader who waited until the fall was obvious before selling,
the short-seller who borrowed coins and now has to buy them back, or the holder whose nerve broke on
the ninth day. A buyer who steps in just as that queue runs out gets the shares or coins from the
last person who had to be out.

## An everyday comparison

Think of a shop that cuts the price of a coat by the same small amount every day it fails to sell.
On its own, each cut says nothing; the shop simply wants the coat gone. But if you count nine
consecutive cuts, you learn that the shop is under real pressure to clear stock, and the ninth cut
is often the most aggressive, because the manager is running out of patience. The strategy counts
the cuts and buys the coat on the ninth one, betting that the shop will not keep cutting forever.
The same logic applies to a queue of people leaving a stadium: the crowd thins quickly at the end,
and the person who waits for the rush to finish walks out without being pushed.

## The rules, step by step

1. Collect hourly prices for one cryptocurrency pair: for each hour, the open, the highest price,
   the lowest price and the close.
2. For each hour, compare its closing price with the closing price of the hour four hours earlier.
3. If the close is lower, add one to a running count. If the close is not lower, reset the count to
   zero.
4. When the count reaches nine, the pair is said to be in a downward setup of nine.
5. Buy on that ninth hour only if its lowest price is below the lowest price of the sixth or seventh
   hour of the count. This is the perfection condition. In the file the condition is checked on the
   ninth hour and also on the eighth hour, and either one is accepted.
6. Hold the position. Close it when the opposite count appears: nine hours in a row whose close is
   higher than the close four hours earlier. The file also closes the position earlier if the
   matching perfection condition on the upside appears.
7. Profit target. The file asks for a 500 percent gain, which in practice never arrives, so the
   written target almost never closes a trade.
8. Loss limit. If the position is 5 percent below the opening price, it is sold. This is a tight
   limit for a market that moves as much as a cryptocurrency pair does in an hour.
9. No trailing stop is used. The strategy needs thirty hours of candles before it can produce its
   first signal, and it reviews the pair once an hour.

The system was published by Thomas DeMark and is usually called TD Sequential. The file names
`@bmoulkaf` as the author of this particular implementation and links a beginner's guide to the
system, and the underlying idea of counting a run to a threshold is DeMark's, not the implementer's.

## The maths, with every symbol named

The whole rule is one comparison and one count.

For the hour numbered `t`, with closing price `C_t`, define the down test:

```text
down_t = 1 if C_t < C_(t-4), otherwise 0
```

- `C_t` is the closing price of the hour `t`.
- `C_(t-4)` is the closing price four hours before hour `t`.
- `down_t` is one when the price has fallen over that four-hour window, and zero when it has not.

The count is a running total that resets:

```text
count_t = count_(t-1) + 1, if down_t = 1
count_t = 0,              if down_t = 0
```

- `count_t` is the number of consecutive hours up to and including hour `t` that passed the down
  test.
- Each failure wipes the count back to zero, so the count only grows while the falling is unbroken.

The buy signal is:

```text
buy when count_t >= 9 and (low_t < low_(t-3) or low_t < low_(t-2) or the previous hour passed the same test)
```

- `low_t` is the lowest price of hour `t`.
- The two comparisons use the lowest prices of the sixth and seventh hours of the run, which sit
  three and two hours before the ninth.
- The last clause repeats the test one hour earlier, which is the form the file actually evaluates.

The loss limit converts the stop into a price:

```text
stop_price = entry_price * (1 - 0.05)
```

- `entry_price` is the price at which the position was opened.
- `0.05` is the five percent loss limit.

The cost of a completed round trip is `2 * c`, where `c` is the charge per side as a fraction of the
amount traded. On a large crypto exchange a realistic value of `c` is 0.0005 to 0.001, that is 0.05
to 0.10 percent per side, so a round trip costs 0.10 to 0.20 percent; the cost primer in this
collection uses 0.05 percent per side and a 0.1 percent spread.

## A worked example

Twelve invented hourly candles for one pair. The count grows while the close stays below the close
four hours earlier. Prices start near one hundred units.

| Hour | Close  | Close four hours earlier | Lower? | Count |
| ---- | ------ | ------------------------ | ------ | ----- |
| 1    | 100.00 | not available            | -      | 0     |
| 2    | 99.50  | not available            | -      | 0     |
| 3    | 99.00  | not available            | -      | 0     |
| 4    | 98.50  | not available            | -      | 0     |
| 5    | 98.00  | 100.00                   | yes    | 1     |
| 6    | 97.60  | 99.50                    | yes    | 2     |
| 7    | 97.20  | 99.00                    | yes    | 3     |
| 8    | 96.80  | 98.50                    | yes    | 4     |
| 9    | 96.40  | 98.00                    | yes    | 5     |
| 10   | 96.00  | 97.60                    | yes    | 6     |
| 11   | 95.80  | 97.20                    | yes    | 7     |
| 12   | 95.50  | 96.80                    | yes    | 8     |
| 13   | 95.30  | 96.40                    | yes    | 9     |

The count reaches nine at hour 13. The sixth hour of the count is hour 10, with a low of 95.90, and
the seventh is hour 11, with a low of 95.70. Hour 13's low is 95.60, below both, so the perfection
condition holds and the rule buys.

The position is opened at the next hour's open, hour 14, at 95.30. The price then climbs hour by
hour, so the downward count resets and a rising count begins. The rising count reaches nine at hour
26, when the close is 99.20, which is higher than the close four hours earlier, 98.00. That closes
the position at the next hour's open, hour 27, at 99.20.

```text
Gross gain      = 99.20 / 95.30 - 1 = 0.0409, that is 4.09 percent
Round-trip cost = 2 * 0.001 = 0.002, that is 0.20 percent
Net gain        = 4.09 - 0.20 = 3.89 percent
```

The 5 percent loss limit was never reached, because the price never fell below 95.30 times 0.95,
which is 90.535. The 500 percent profit target never arrived either; on this example the position
closed only because the opposite count completed. The arithmetic shows how the rules work, not that
they work.

## What the research actually found

There is very little independent evidence for TD Sequential, and none of it is a controlled test of
this file.

| Source                                               | What it measured                                 | Result                                                                                                                                                |
| ---------------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Thomas DeMark, The New Science of Technical Analysis | His own system, presented with worked charts     | The origin of the counting rule and the perfection condition; a practitioner's book, not a study with a sample and costs                              |
| The beginner's guide the strategy file links         | The nine-close rule explained for crypto traders | Restates the buy and sell triggers; no sample, no period and no cost figure are given                                                                 |
| The file's own comments                              | The parameters the implementer chose             | A five percent loss limit, a five hundred percent profit target and no trailing stop, with no note of the pairs, the period or the result of any test |

What is missing matters more than what is present. Nobody in this file reports how many times a
count of nine appeared, how often it was followed by a rise, or what the result was after the gap
between buying and selling prices. The system's reputation rests on its author's books and on the
community of traders who use it, which is not the same as a measurement. The honest grade for this
version is weak: one implementation, one set of chosen numbers and no test that a reader can check.

## How this project relates to it

The mechanism this rule leans on, that selling pressure persists and then exhausts, is studied in
this repository's [order flow and toxicity](../../../strategies/books/07_order_flow_and_toxicity.md)
brief. Its first finding is that buy and sell order flow is strongly persistent over tens of
thousands of orders, which is the empirical version of "the sellers keep selling", and its later
sections explain why the price impact of that flow does not always follow.

The count-and-threshold style of rule is the same trap described in
[overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md):
choosing the number nine, or the four-hour lookback, or the perfection version, by which fits the
past is exactly the wide search that brief warns about.

## Where it goes wrong

- A run of nine lower closes is observed in the data; whether it predicts a rise is a separate
  claim, and no one here tested it. Counting a pattern that is simply there is not evidence.
- The threshold is arbitrary. Nine could be eight or ten, and the four-hour lookback could be three
  or six. Each of those choices is a separate rule, and picking the one that looked best is a form
  of using the answer.
- Conditions pile up on the same candles. The ninth-hour perfection test adds two more comparisons
  to the count, and on a short sample a rule with several conditions on the same few candles will
  almost always fit the past, because there are enough ways to describe the data that one will work.
- The five percent loss limit is tight. A crypto pair can move five percent in a single hourly
  candle, so the stop will often be hit by ordinary noise before any turnaround has time to happen.
- The written profit target is five hundred percent, so in practice the target is decorative. The
  real exits are the opposite count and the loss limit, neither of which the file reports having
  tested.
- No costs, no pairs and no period are recorded. A reader cannot tell whether the numbers were run
  over a rising market, a falling one, or a single lucky pair.

## Try it yourself

You need a spreadsheet and a public source of hourly prices for one coin pair.

1. Make columns: hour, close, close four hours earlier, lower, count.
2. Fill the lower column with yes or no: is the close below the value four columns up?
3. Fill the count column with a running total that adds one on a yes and returns to zero on a no.
4. Make a new column that says signal when the count equals nine.
5. In the rows after each signal, record the return over the next one, four and twenty-four hours.
6. Average those returns over every signal.
7. Repeat the whole sheet with the threshold set to seven instead of nine.

What to notice: a count of nine appears rarely, and the few appearances give an average built on
almost nothing. When you change the threshold to seven, the average usually moves a lot, which tells
you the number was doing more work than the idea.

## Where this came from

- [TDSequentialStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/TDSequentialStrategy.py),
  the file that states the rules: the hourly timeframe, the count against the close four hours
  earlier, the perfection condition, the five percent loss limit and the five hundred percent
  target.
- Thomas DeMark, [The New Science of Technical Analysis](https://catalogimages.wiley.com/images/db/pdf/9781576603147.excerpt.pdf)
  (1994), the book that introduced the counting system, and the
  [Sequential indicator](https://demark.com/sequential-indicator/) page maintained by its publisher.
- The [beginner's guide to the system](https://hackernoon.com/how-to-buy-sell-cryptocurrency-with-number-indicator-td-sequential-5af46f0ebce1)
  that the strategy file itself links as its source.
- [Order flow and toxicity](../../../strategies/books/07_order_flow_and_toxicity.md) and
  [overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's studies of the two ideas the rule depends on.

## Words used in this tutorial

- count: the number of consecutive hours that have passed the down test without a break.
- countdown: DeMark's name for the second, later phase of his system, not used by this file.
- exhaustion: the point at which the traders pushing a price one way have mostly finished.
- loss limit: a price below the entry at which the position is sold, also called a stop loss.
- perfection: DeMark's term for the extra condition that the eighth or ninth step undercuts an
  earlier low.
- setup: DeMark's name for the nine-step count itself.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
