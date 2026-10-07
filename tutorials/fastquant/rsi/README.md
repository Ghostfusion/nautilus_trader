# The relative strength index: buying after a one-sided fall

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of one company, or one fund that tracks an index, bought and sold whole                                                                                                                                  |
| How often it trades       | A few times a year, whenever the index falls below 30 or rises above 70                                                                                                                                         |
| What you need             | A spreadsheet                                                                                                                                                                                                   |
| Where the rules come from | The strategy table in the [fastquant](https://github.com/enzoampil/fastquant) README (alias `rsi`) and its [source file](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/rsi.py) |
| The underlying research   | J. Welles Wilder, New Concepts in Technical Trading Systems (1978), the book that introduced the index, described in the [canonical entry](https://en.wikipedia.org/wiki/Relative_strength_index)               |
| How well it held up       | Mixed: the rules earned money in a few of the markets tested and lost it in others, so what was measured depends on which market and which window were chosen                                                   |
| Also appears in           | [macd](../macd/README.md) in this same library, where the index reappears as a filter                                                                                                                           |

## The idea in one paragraph

Look at the last two weeks of closing prices. Mark each day as a gain if the price rose from the day
before, and a loss if it fell, and add up how big the gains were and how big the losses were. Turn
those two totals into a single number between 0 and 100. A number near 100 means the recent rises
were much bigger than the recent falls; a number near 0 means the recent falls were much bigger than
the recent rises. The strategy buys when that number drops below 30, on the theory that a price which
has fallen almost every day for two weeks has been pushed too far down and is likely to bounce, and it
sells when the number climbs above 70.

## Why anyone believed it

Prices move because people buy and sell, and people do not all learn the same thing at the same time.
When bad news arrives, some holders must sell no matter what the news is worth: a fund whose investors
are asking for their money back, a borrower who is being forced to repay a loan, a trader whose own
rule tells them to cut a loss. Those sellers push the price below where the news alone would put it.
The counterparty in this trade is exactly those forced sellers, together with the crowd that reads a
fall as a reason to expect further falls.

If panic and forced selling keep recurring, then after a one-sided fall the price tends to snap back,
and whoever buys from the panicking seller earns that snap. That is the whole economic story, and it
does not require anyone to forecast the news, only to notice when the recent selling has become
one-sided.

## An everyday comparison

A shopkeeper writes down the daily takings and compares each day with the day before. Some days are
better, some worse. At the end of two weeks she adds up the size of the better days and the size of
the worse days and expresses the result as one number: nearly 100 if almost every day was an
improvement, nearly 0 if almost every day was a disappointment. That number describes the last two
weeks. It does not know whether the next two weeks will be busy or quiet, and a run of disastrous days
can push it to 0 and keep it there, because there is nowhere below 0 for it to go.

## The rules, step by step

1. Pick one thing to trade: one company's shares, or one fund that tracks an index.
2. Get the daily closing price, one price per trading day.
3. Choose the period, called `rsi_period` in fastquant. The library's default is 14 days, and that is
   also Wilder's original choice. The worked example below uses 5 days so that the arithmetic fits on
   the page.
4. Choose two lines, called `rsi_upper` and `rsi_lower`. The fastquant defaults are 70 and 30.
5. On each day, compute the index from the last `rsi_period` closes, as described in the next section.
6. Buy when the index is strictly below `rsi_lower`. In fastquant, with its default settings, the buy
   uses all the cash the backtest gives it.
7. Sell when the index is strictly above `rsi_upper`, which closes the whole position.
8. Review every day. The rules are checked on each daily close, and there is no fixed holding period:
   the position lasts until the opposite signal appears.

The library decides how much to buy with two further settings, `buy_prop` and `sell_prop`, both 1 by
default, meaning the whole of the available cash or the whole of the position.

## The maths, with every symbol named

The index is built from the daily changes. For each day, compare the close with the close of the day
before:

```text
u_t = max(P_t - P_(t-1), 0)
d_t = max(P_(t-1) - P_t, 0)
```

- `P_t` is the closing price on day `t`, and `P_(t-1)` is the closing price on the day before.
- `u_t` is the size of the day's rise, written 0 when the price fell.
- `d_t` is the size of the day's fall, written 0 when the price rose.

So on any day one of the two is zero and the other is the size of the move. Next, average the two
series. Wilder, and the library that follows him, do not use a plain average: they start with the
plain average of the first `N` days and then update it a little each day, giving the newest day a
weight of `1/N` and the running average a weight of `(N-1)/N`:

```text
A_t = (A_(t-1) * (N - 1) + u_t) / N
B_t = (B_(t-1) * (N - 1) + d_t) / N
```

- `A_t` is the smoothed average size of the rises, and `B_t` the same for the falls.
- `N` is `rsi_period`.
- The first `A` is the plain average of the first `N` rises, and the first `B` the plain average of
  the first `N` falls.

The index itself is then:

```text
RS_t  = A_t / B_t
RSI_t = 100 - 100 / (1 + RS_t)
```

- `RS_t` is the ratio of the average rise to the average fall, a number from 0 upward with no upper
  bound.
- `RSI_t` is the index, a number from 0 to 100.

Two edge cases follow from the formula. If every recent day was a fall, then `A_t` is 0, so `RS_t` is
0, and the index is 0. If every recent day was a rise, then `B_t` is 0, the ratio is enormous, and the
index is taken as 100. Because the index is a fixed fraction of a whole, it cannot leave the range 0
to 100 no matter what the price does.

## A worked example

Ten made-up closes, with `rsi_period` set to 5, `rsi_upper` to 70 and `rsi_lower` to 30. This is the
formula above, with `N` equal to 5. The averages are shown rounded to two decimals; the index is
computed from the numbers before rounding.

| Day | Close | Change | Average rise | Average fall | RSI  |
| --- | ----- | ------ | ------------ | ------------ | ---- |
| 1   | 100   | -      | -            | -            | -    |
| 2   | 97    | -3     | -            | -            | -    |
| 3   | 94    | -3     | -            | -            | -    |
| 4   | 91    | -3     | -            | -            | -    |
| 5   | 88    | -3     | -            | -            | -    |
| 6   | 86    | -2     | 0.00         | 2.80         | 0.00 |
| 7   | 90    | +4     | 0.80         | 2.24         | 26.3 |
| 8   | 95    | +5     | 1.64         | 1.79         | 47.8 |
| 9   | 100   | +5     | 2.31         | 1.43         | 61.7 |
| 10  | 105   | +5     | 2.85         | 1.15         | 71.3 |

The first index value appears on day 6, because five days of changes are needed before anything can be
averaged. Days 2 to 6 were all falls, so the average rise is 0 and the index sits at its floor of 0.
That is a buy signal under the rules, and the position is bought at the close of 86.

The index then climbs as the rises arrive: 26.3, 47.8, 61.7 and finally 71.3. The 71.3 on day 10 is
above the upper line of 70, so the position is sold at the close of 105.

Now the money. Buy 100 shares at 86, which costs 8,600.00, and sell them at 105, which brings in
10,500.00. The gross gain is 1,900.00. Suppose each trade costs 0.10 percent of its value, a plausible
figure for a liquid fund and far more than the fastquant default, which is zero. Then the costs are
8.60 on the way in and 10.50 on the way out, 19.10 together:

```text
Net gain = 1,900.00 - 19.10 = 1,880.90
Return on the money used = 1,880.90 / 8,600.00 = 21.9 percent
```

Two honest notes. The example made a profit because the numbers were chosen so that a signal and a
reversal both appear on the page; it is an arithmetic demonstration, not evidence. And the return is
computed on the 8,600 that was actually invested, not on the whole account, which sat in cash until
day 6.

## What the research actually found

The rules as fastquant implements them are Wilder's "overbought and oversold" version: buy below the
lower line, sell above the upper line. The best-known academic test of exactly that version is Chong,
Ng and Liew (2014), who applied it to five developed-market indices using daily data from January 1976
to December 2002.

| Where                 | Rule tested    | What it measured                                                                                                                                        |
| --------------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dow Jones Industrials | RSI(14, 30/70) | Buy signals averaged 1.017 percent over the next ten days, significant at the 5 percent level, though the buy-minus-sell difference was not significant |
| Milan Comit General   | RSI(14, 30/70) | Buy signals averaged -0.242 percent, and the buy-minus-sell difference was -1.025 percent, significant at the 10 percent level                          |
| DAX 30                | RSI(14, 30/70) | Buy-minus-sell difference -0.914 percent, significant at the 10 percent level                                                                           |
| Nikkei 225            | RSI(14, 30/70) | Buy signals averaged -0.114 percent, no significant difference from buy and hold                                                                        |

Three things stand out. First, the same rule helped in one market and hurt in another. Second, the
authors report that after subtracting a round-trip cost of 1 percent, only the centre-line version of
the rule (buy when the index crosses up through 50, sell when it crosses down) survived in the Italian
market, where it averaged about 5 percent a year; the version fastquant implements, buy below 30 and
sell above 70, was the weaker of the two. Third, the authors themselves conclude that the results are
"not robust to the choice of market". The canonical
description of the index also reports a study finding that the index can still produce good results in
a short test but is usually beaten by simply buying and holding over longer periods.

The paragraph that matters most for a beginner: fastquant's default settings of 14 days, 70 and 30 are
the round numbers from a 1978 book, copied into a teaching library. The library does not test them,
and its own README examples are single plots of a single stock over a single year. The README reports
that the RSI rule with those defaults turned 100,000 into 132,967.87 in the example run it prints, and
that number should be read as one year of one share, computed with a commission of zero, with no test
on any other stock or year. A default is a starting point for a conversation, not a finding.

## How this project relates to it

This repository implements the same calculation in Rust, in
[crates/indicators/src/momentum/rsi.rs](../../../crates/indicators/src/momentum/rsi.rs). That file is
the working version of the formula above, and reading it next to the table is the quickest way to see
that the index is only a pair of running averages. The repository's own survey of the evidence,
[08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md),
places indicator rules like this one inside a wider argument about decay and crowding, and is the
honest companion to the marketing pages of any trading library.

## Where it goes wrong

- A falling market keeps producing buy signals. The index is bounded, so in a sustained decline it
  parks near the floor and stays there; a rule that buys below 30 buys again and again into a fall
  that does not stop. The bound that makes the number easy to read is also what makes it blind.
- The lines are conventions, not laws. There is nothing special about 30 and 70 except that they were
  printed in a book in 1978 and copied ever since. A 25-line and a 35-line rule will give different
  answers on the same data, and choosing the pair after seeing the results is how a losing rule is
  dressed up as a winning one.
- Costs eat the low-end signals. The rule only trades occasionally, which is in its favour, but the
  measured profits were small enough that a 1 percent round-trip cost erased most of them.
- The number is not a measure of value. An index at 20 says the recent days were mostly falls. It says
  nothing about whether the company is cheap, whether the reason for the fall was real, or whether the
  fall will continue.
- The same input, a different average, a different answer. Wilder's smoothed average, a plain average,
  and an exponential average give three slightly different indices from the same prices. A result
  cannot be compared with another result unless the same averaging was used.
- What would have to be false. The whole idea rests on forced and panicking sellers pushing a price
  below its fair level and then stepping back. If the buyers know that pattern just as well, they
  arrive earlier, the bounce happens sooner and smaller, and what is left is a rule that trades for
  the benefit of the broker.

## Try it yourself

You need a spreadsheet and a public source of daily closing prices; any finance website will do.

1. Put the dates in column A and the closing prices in column B, about twenty rows.
2. In column C, write the daily change: the price in this row minus the price in the row above.
3. In column D, write the rise: the change if it is positive, otherwise 0. In column E, write the
   fall: the change turned positive if the price fell, otherwise 0.
4. In cell F6, average the first five values of column D, and in G6 average the first five values of
   column E.
5. In F7 write `=((F6*4)+D7)/5` and in G7 write `=((G6*4)+E7)/5`, then drag both down. The 4 and the 5
   are the period minus one and the period.
6. In column H write `=100-100/(1+F6/G6)` in each row, guarding against a G of 0.
7. Mark every row where the index is below 30 and every row where it is above 70.

What to notice: in a smooth rise the index spends weeks above 70 without a single new reason, and in a
smooth fall it sits below 30 the same way. Try a second sheet with `rsi_period` set to 7 instead of
14 and compare the two columns of signals. If the two sheets disagree about when to buy, the rule is
fragile in a way that has nothing to do with the market.

## Where this came from

- The [fastquant](https://github.com/enzoampil/fastquant) README, for the alias `rsi`, the parameter
  names `rsi_period`, `rsi_upper` and `rsi_lower`, and the example result on Jollibee shares.
- The [RSI strategy source](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/rsi.py),
  for the exact buy and sell conditions: buy below the lower line, sell above the upper line.
- The [backtest documentation](https://github.com/enzoampil/fastquant/blob/master/docs/docusaurus/docs/backtest.md),
  for the meaning of the parameters passed to the backtest function.
- Chong, Ng and Liew (2014), [Revisiting the Performance of MACD and RSI Oscillators](https://www.mdpi.com/1911-8074/7/1/1),
  the five-market test with the numbers quoted above.
- The [canonical description of the index](https://en.wikipedia.org/wiki/Relative_strength_index),
  for Wilder's 1978 book, the scale of the index, and the study finding that buy and hold usually wins.
- The repository's own survey,
  [08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md).

## Words used in this tutorial

- closing price: the price of the last trade of the day, used as that day's price.
- counterparty: the person on the other side of your trade, who is selling what you buy.
- index: here, a single number that summarises a pattern in prices; not a stock market index.
- period: the number of past days a calculation looks at.
- position: the amount of something you hold, or owe if it is negative.
- trend: a stretch in which the price mostly moves in one direction.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
