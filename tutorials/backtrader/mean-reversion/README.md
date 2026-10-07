# Mean reversion: buying a short fall while the longer trend is still up

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                     |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Spot gold priced in American dollars, on daily and 15-minute bars, plus a large batch of rules ported from commercial trading platforms                                                                                                                                                                   |
| How often it trades       | From a few times a week for the classic rules to several hundred times in three months for the ported ones                                                                                                                                                                                                |
| What you need             | A spreadsheet and daily prices for the classic rules; Python and a data file for the rest                                                                                                                                                                                                                 |
| Where the rules come from | [The Strategy Compendium, article 02, mean reversion](https://backtrader.readthedocs.io/en/latest/strategies-series/en/02-mean-reversion.html)                                                                                                                                                            |
| The underlying research   | None with a paper trail: the two-to-five-day reversal thresholds come from Larry Connors' practitioner books and are restated in the library article                                                                                                                                                      |
| How well it held up       | Weak: the classic rules were published by their author without an independent replication, and the library's own numbers are single backtests of gold over 2008 to 2025                                                                                                                                   |
| Also appears in           | [Short term reversal](../../quantconnect/short-term-reversal-strategy-in-stocks/README.md) and [statistical arbitrage](../../quantconnect/mean-reversion-statistical-arbitrage-strategy-in-stocks/README.md) in this collection, and [contrarian reversion](../../project/contrarian-reversion/README.md) |

## The idea in one paragraph

Prices drift down and up over years, but over a few days they often jump too far in one direction
and then partly come back. This category buys the short-term fall, but only when the longer trend is
still rising. Two shapes dominate. In the oscillator version, a fast mood meter of the last two days
is driven to its floor and the strategy buys, selling again when the meter recovers. In the channel
version, the price prints the lowest close of the last seven days while still above its long-term
average, and the strategy buys, selling at the highest close of the next seven days. The bet is the
same in both: a sudden drop in a rising market is more often an overreaction than a change in the
trend.

## Why anyone believed it

The counterparty is an impatient or forced seller. A fund meeting withdrawals, a manager cutting a
position in a hurry, or simply someone who read one frightening headline and sold first, all push the
price below where the slower crowd would have it. That push is a discount, and a patient buyer
collects it when the price drifts back. The second counterparty is the overreacting crowd itself:
during a panic, many people sell at the same moment, and the price overshoots because sellers must
compete for the few buyers present. As the buyers arrive, the price recovers.

The reason the strategy insists the long-term trend be up is the difference between a pullback and a
collapse. A price below its own long average is a market in trouble, and a fall there is often the
start of more falling, not an overreaction. By demanding the price be above its long average, the rule
declines to catch a knife that is still falling and only buys dips inside an uptrend.

## An everyday comparison

A supermarket marks the fruit down late in the day. The discount is not a statement that the fruit is
bad; it is the price of emptying the shelf before closing. A shopper who arrives at that hour buys
the marked-down fruit cheaply, because the shop wants the shelf clear more than it wants the last few
pennies. But a shopper would not buy from a shop that marks everything down because the whole
neighbourhood is emptying, which is what the long-average filter is checking.

## The rules, step by step

The category holds 331 backtests, and they all share one skeleton: a short-horizon extreme, a
longer-horizon filter, and an exit when the extreme heals. The two classic rules are stated exactly
as the library implements them.

1. Use daily closing prices for spot gold, one price per trading day.
2. Compute a long moving average of the close, usually the average of the last 100 or 200 days. Every
   rule below buys only when the close is above this line.
3. The Connors oscillator rule. Compute a two-day mood meter, the relative strength index, from the
   last two up-and-down moves. Buy when the meter falls below 5 while the close is above the
   100-day average. Sell the whole position when the meter rises above 30.
4. The Double 7s channel rule. Look at the lowest close of the last 7 days. Buy when today's close
   is at or below that low and also above the 200-day average. Sell the whole position when today's
   close is at or above the highest close of the last 7 days.
5. Position size is one fixed amount per signal. The library uses a fixed number of contracts on a
   million-dollar account, and the size does not change between trades.
6. There are no price stops in these two rules. The exit is the healing condition, so the position is
   held through further weakness until the meter or the channel recovers. This is a deliberate choice
   and it is where the largest falls come from.
7. Review once a day, at the close. Do not act on prices during the session.
8. The rest of the category repeats this skeleton with other measures. Consecutive down days buys
   after three to five falling days and holds one day. The efficiency-ratio rule buys the oscillator
   extreme only when the market is measured as choppy. The KDJ and DiNapoli rules slow a fast
   oscillator with several smoothing passes. Bollinger rules trade a touch of a wide band. About 256
   of the 331 are ports of commercial-platform expert advisors that keep their original pip and lot
   settings.

## The maths, with every symbol named

The two classic rules rest on three formulas.

The relative strength index, or RSI, the fast mood meter:

```text
RS  = average gain over the last N days / average loss over the last N days
RSI = 100 - 100 / (1 + RS)
```

- `N` is the number of days in the window. The classic setting is 14; Connors cut it to 2.
- `average gain` is the mean of the up-moves over the window. A day that fell counts as a gain of
  zero.
- `average loss` is the mean of the down-moves over the window, written as a positive number. A day
  that rose counts as a loss of zero.
- `RSI` runs from 0 to 100. Zero means every recent move was down; 100 means every move was up.
- When the average loss is zero, the meter is 100.

This implementation is worth a warning. The library file for the oscillator rule computes the two
averages as a plain mean of the last `N` numbers, not as Wilder's original smoothed average. The two
differ, so a reader who copies Wilder's textbook formula will not reproduce the library's numbers.

The simple moving average, the long filter:

```text
SMA_t = (C_t + C_(t-1) + ... + C_(t-M+1)) / M
```

- `C_t` is the closing price on day `t`.
- `M` is the window in days, here 100 or 200.
- `SMA_t` is the average closing price over the last `M` days.

The seven-day channel:

```text
Low7_t  = the smallest of C_t, C_(t-1), ..., C_(t-6)
High7_t = the largest of  C_t, C_(t-1), ..., C_(t-6)
```

- `Low7_t` is the lowest closing price in the last seven days, today included.
- `High7_t` is the highest closing price in the last seven days, today included.
- The two numbers are recomputed every day and both move as old days drop out of the window.

Finally, the return after costs, which every rule in the category shares:

```text
Net = (P_sell / P_buy - 1) - 2 * c
```

- `P_buy` is the price paid at entry, `P_sell` the price received at exit.
- `c` is the cost of one side, as a fraction of the price: commission plus half of the gap between
  the buying and selling prices.
- The factor 2 counts both the purchase and the sale. The library's gold files charge 0.02 percent
  per side and no spread.

## A worked example

First the oscillator rule, on six made-up but plausible daily closes. Suppose the 100-day average is
1,900.00, below every price here, so the trend filter is always satisfied.

| Day | Close  | Change | RSI(2) | Action         |
| --- | ------ | ------ | ------ | -------------- |
| 1   | 2000.0 |        |        | watch          |
| 2   | 1985.0 | -15.0  |        | watch          |
| 3   | 1970.0 | -15.0  | 0.0    | buy at 1970.0  |
| 4   | 1955.0 | -15.0  | 0.0    | hold           |
| 5   | 1990.0 | +35.0  | 70.0   | sell at 1990.0 |
| 6   | 2000.0 | +10.0  | 100.0  | flat           |

On day 3 the two most recent moves are both losses of 15.00. The average gain is zero and the average
loss is 15.00, so `RS` is zero and `RSI` is `100 - 100 / 1 = 0`, below the buy threshold of 5. The
close of 1970.00 is above the 1,900.00 average, so the rule buys at 1970.00. On day 5 the two moves
are a fall of 15.00 and a rise of 35.00, so the average gain is 17.50 and the average loss is 7.50.
`RS` is `17.50 / 7.50 = 2.333`, and `RSI` is `100 - 100 / 3.333 = 70.0`, above the exit threshold of
30, so the rule sells at 1990.00.

```text
Gross return = 1990.00 / 1970.00 - 1 = 0.01015, that is 1.015 percent
Cost        = 2 * 0.0002 = 0.0004, that is 0.040 percent
Net return  = 1.015 - 0.040 = 0.975 percent
```

Now the channel rule, on ten made-up daily closes, with the 200-day average at 1,900.00.

| Day | Close  | Low7   | High7  | Action         |
| --- | ------ | ------ | ------ | -------------- |
| 1   | 2030.0 |        |        | watch          |
| 2   | 2025.0 |        |        | watch          |
| 3   | 2020.0 |        |        | watch          |
| 4   | 2015.0 |        |        | watch          |
| 5   | 2010.0 |        |        | watch          |
| 6   | 2005.0 |        |        | watch          |
| 7   | 2000.0 |        |        | watch          |
| 8   | 1985.0 | 1985.0 | 2025.0 | buy at 1985.0  |
| 9   | 1995.0 | 1985.0 | 2020.0 | hold           |
| 10  | 2028.0 | 1985.0 | 2028.0 | sell at 2028.0 |

On day 8 the lowest close of days 2 to 8 is 1985.00, and the close equals it, so the rule buys at
1985.00 while the close stays above the 1,900.00 average. On day 9 the highest close of the last
seven days is 2020.00 and the close is 1995.00, so there is no exit. On day 10 the highest close of
days 4 to 10 is 2028.00, and the close equals it, so the rule sells at 2028.00.

```text
Gross return = 2028.00 / 1985.00 - 1 = 0.02166, that is 2.166 percent
Cost        = 2 * 0.0002 = 0.0004, that is 0.040 percent
Net return  = 2.166 - 0.040 = 2.126 percent
```

Both examples are invented and both happen to win; that is the luck of a short table, not a claim.
The arithmetic shows only how the rules are applied. Notice also the gap between buying and selling
prices: the real cost is more than commission, because a buyer pays a little above the mid-price and
a seller receives a little below it, and the library's gold files ignore that gap entirely.

## What the research actually found

The library reports single backtests of spot gold from 2008 to 2025, a million-dollar starting
account, and 0.02 percent commission. The numbers below are its own.

| Rule                       | Trades | Wins   | Final value | Worst fall |
| -------------------------- | ------ | ------ | ----------- | ---------- |
| Connors oscillator (RSI 2) | 311    | 67.85% | 1,703,436   | 17.37%     |
| Double 7s channel          | 148    | 66.89% | 2,138,568   | 30.35%     |
| ConnorsRSI composite       | 38     | 78.95% | (not given) | 6.42%      |

The composite rule piles three measures together, price momentum, the length of the current winning
or losing run, and where the price sits in its range, and waits for a limit order below the market to
be filled. It trades far less and its worst fall is far smaller, which is the pattern a reader should
expect whenever the entry is made pickier.

Nothing here is a paper. The oscillator and channel rules come from Larry Connors' practitioner books
and were published by their author on one instrument over one period, with no independent
replication. The library's figures are one run each, on gold only, at zero spread. A separate weekly
reversal test in this collection, on large American shares, lost to the index over 2016 to 2021, and
independent academic work finds the plain short-term reversal effect has weakened. Read together, the
honest grade is weak.

One thing must be said plainly, because it is the heart of this whole group of tutorials. Every
backtest in the compendium asserts its final value, its reward-to-risk ratio (the return earned per
unit of the strategy's own wobble, also called the Sharpe ratio) and its worst fall against numbers
captured when the strategy was migrated. Passing those assertions proves the engine
computes exactly what the file says, to the cent and to the sixth decimal. It proves nothing about
whether the strategy earns anything. A winning final value that matches its assertion is a
consistency check, not evidence of an edge.

## How this project relates to it

The repository's reading of the research on reversal and momentum is in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
One finding there is close enough to quote: ranking shares by how far they have recovered from a
drawdown rather than by raw past return earned 0.146 percent per week against 0.073 percent on the
KOSPI 200 index (`1403.8125v4`, p.8), and roughly halved the worst fall. The brief also records that
these comparisons are before costs, which is exactly where a fast-repeating rule is most at risk.

The finished tutorial [Short term reversal](../../quantconnect/short-term-reversal-strategy-in-stocks/README.md)
covers the weekly, cross-sectional version of the same idea, buying the month's worst performers
across a hundred large shares. That one is a portfolio of many positions rebuilt weekly; this one is
a single instrument flipped by a two-day meter. Comparing them side by side makes the horizon
question concrete.

## Where it goes wrong

- The periods are the strategy. Over days, prices tend to reverse. Over weeks and months, prices tend
  to continue. The rules here act on a two-to-seven-day horizon, and that is why they look like the
  opposite of momentum. A rule that held the same position for three months would be testing a
  different effect, and probably the wrong one.
- A fall can be news. If the price fell because the company or the country is genuinely in trouble,
  the price is not wrong and it will not bounce. The rule cannot tell an overreaction from a repricing
  and buys both.
- No stop means a large fall. Double 7s carried a 30 percent worst fall in the library's own test.
  That is the deliberate cost of never being shaken out at the worst moment.
- The backtest ignores the gap between buying and selling prices. On a daily gold series the library
  charges no spread, which flatters any rule that trades often. Add a realistic spread and a
  high-turnover rule loses a large part of its gross edge.
- Parameters fit the sample. Two days or three, threshold 5 or 10, average 100 or 200: there are many
  nearby choices, and the best of them on one sample is not the best on another.
- Fills are assumed. The composite rule enters with a limit order and quietly records the orders that
  expired unfilled, which is honest, but a rule that could not have been filled at the price it asked
  is still a rule that never traded.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily gold or share prices.

1. Put dates down one column and closing prices in the next.
2. Add a column for the two-day change: today's close minus yesterday's close.
3. Add a column for the two-day average gain and one for the two-day average loss, counting a down
   day as a zero gain and an up day as a zero loss.
4. Add a column for `100 - 100 / (1 + gain/loss)`, the meter.
5. Add a column for the 100-day average close.
6. Add a column that says buy when the meter is below 5 and the close is above the 100-day average,
   and sell when the meter is above 30 while a position is open.
7. Mark each entry and exit, then compute what each round trip would have returned, subtracting
   0.04 percent for the two sides of the trade before you look at the total.

What to notice: how often the meter touches its floor in a market that is falling for months, and how
the 100-day filter throws all of those signals away. Then notice how few signals survive in a calm
market. Most of the strategy's behaviour lives in that filter, not in the meter.

## Where this came from

- [The Strategy Compendium, article 02, mean reversion](https://backtrader.readthedocs.io/en/latest/strategies-series/en/02-mean-reversion.html),
  the category inventory, the deep dives and the gold backtest figures quoted above.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's reading of the reversal and recovery-ranking research, including `1403.8125v4`.
- [Short term reversal](../../quantconnect/short-term-reversal-strategy-in-stocks/README.md), the weekly portfolio
  version of short-horizon reversal in this collection.

## Words used in this tutorial

- drawdown: the fall from a peak to the following low, measured in percent.
- mean reversion: the tendency of a price that has moved too far to move part of the way back.
- momentum: the tendency of something that has been rising to keep rising for a while.
- overreaction: a price move that goes further than the news appears to justify.
- relative strength index: a number from 0 to 100 that measures how one-sided the recent price moves
  have been.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- spread: the gap between the price at which something can be bought and the price at which it can be
  sold.
- trend: the broad direction of a price over many weeks, as opposed to its day-to-day wobble.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
