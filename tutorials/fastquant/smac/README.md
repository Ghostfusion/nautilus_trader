# The moving average crossover: buying when two averages swap places

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                    |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | One company's shares, or one fund that tracks an index, bought and sold whole                                                                                                                                                                                            |
| How often it trades       | A few times a year; quiet markets produce nothing for months, and choppy ones produce a stream of small losses                                                                                                                                                           |
| What you need             | A spreadsheet                                                                                                                                                                                                                                                            |
| Where the rules come from | The strategy table in the [fastquant](https://github.com/enzoampil/fastquant) README (alias `smac`) and its [source file](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/ma_crossover.py)                                                |
| The underlying research   | none, this is a practitioner's rule of thumb; the earliest careful test is Brock, Lakonishok and LeBaron (1992), [Simple Technical Trading Rules and the Stochastic Properties of Stock Returns](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1992.tb04681.x) |
| How well it held up       | Weak: the famous result on a century of American data stopped working once the same rules were tested on the next ten years and once the search over many rules was taken into account                                                                                   |
| Also appears in           | [emac](../emac/README.md), the same rule built on an exponential average, in this same library                                                                                                                                                                           |

## The idea in one paragraph

Take the average of the last ten closing prices, and the average of the last thirty. When prices are
drifting down, the ten-day average sits below the thirty-day average, because it is made of more
recent, lower prices. When prices start to rise, the ten-day average moves first and crosses above the
thirty-day average. This strategy buys at that crossing and holds until the short average crosses back
below the long one, at which point it sells and waits. The bet is that a rise strong enough to turn the
short average has more of itself still to come.

## Why anyone believed it

News does not reach everyone at once, and neither do the decisions that follow. A supplier's contract
is renewed, a drug trial reports, a central bank changes its tone: each piece of information is
digested over days and weeks by analysts, funds and ordinary savers in turn. The buyers who act late
push the price higher after the early buyers have done so, which is why an uptrend can continue for a
while after it is obvious.

The counterparty is the slow seller: the holder who is gradually liquidating an old position, the fund
that trims a winner each month to keep its weights in line, the trader who sells at a round number
simply because it is a round number. If those sellers keep appearing, the trend persists, and a rule
that waits for the trend to be confirmed and then joins it can pick up the remainder. What none of
this explains is why the confirmation should mark the start of anything rather than the middle.

## An everyday comparison

A small shop records its daily takings. A three-day average tells you how the last three days went; a
ten-day average tells you how the last fortnight went. In a quiet season both averages sit close
together and their order swaps back and forth for no reason, which is the shopkeeper learning nothing.
In a busy season the three-day average climbs and moves above the ten-day average, and the crossing is
a fair summary of what has already happened. The shopkeeper who orders extra stock only on the day the
averages cross is not predicting the busy season; she is reacting to it, and she will react late.

## The rules, step by step

1. Pick one thing to trade: one company's shares, or one fund that tracks an index.
2. Get the daily closing price. Every rule below uses closes only.
3. Choose the length of the short average, called `fast_period`, and of the long average, called
   `slow_period`. The fastquant defaults are 10 and 30. `fast_period` must be smaller than
   `slow_period`; the library does not check this for you.
4. On each day, compute the simple average of the last `fast_period` closes and the simple average of
   the last `slow_period` closes.
5. Buy when the short average crosses from at or below the long average to above it. In fastquant this
   single crossing is the whole condition: the source compares the two lines and buys when the
   comparison changes sign.
6. Sell when the short average crosses from at or above the long average to below it.
7. Hold, doing nothing, at every other time. There is no separate stop loss or profit target unless you
   add one; the library's defaults leave both switched off.
8. Review every day, at the close.

By default the library buys with all the available cash and sells the whole position, controlled by
`buy_prop` and `sell_prop`, both 1.

## The maths, with every symbol named

The simple moving average of the last `N` closes:

```text
SMA_t(N) = (P_t + P_(t-1) + ... + P_(t-N+1)) / N
```

- `SMA_t(N)` is the average of the last `N` closing prices, computed on day `t`.
- `P_t` is today's closing price, `P_(t-1)` yesterday's, and so on backwards.
- `N` is the number of days in the average: `fast_period` for the short line, `slow_period` for the
  long one.

Divide by `N` because there are `N` prices being averaged; the result is in the same units as the
price, so an average of 101.40 means the relevant prices were around 101.40.

The crossing is then a comparison of the two lines on two consecutive days:

```text
Buy  when SMA_t(fast) >  SMA_t(slow) and SMA_(t-1)(fast) <= SMA_(t-1)(slow)
Sell when SMA_t(fast) <  SMA_t(slow) and SMA_(t-1)(fast) >= SMA_(t-1)(slow)
```

- `SMA_t(fast)` is the short average today and `SMA_t(slow)` the long average today.
- The subscript `t-1` means the same comparison made on yesterday's prices.
- The pair of conditions is what makes it a crossing rather than a state: the lines must be on one side
  yesterday and the other side today.

Nothing in the formula refers to tomorrow. The two lines are built entirely from prices that have
already happened, which is why the signal always arrives after the move it describes.

## A worked example

Ten made-up closes, with `fast_period` set to 3 and `slow_period` set to 5, so that the arithmetic fits
on one page. The library's defaults are 10 and 30, and the rule is identical.

| Day | Close | SMA(3) | SMA(5) | Signal                          |
| --- | ----- | ------ | ------ | ------------------------------- |
| 1   | 100   | -      | -      |                                 |
| 2   | 99    | -      | -      |                                 |
| 3   | 98    | 99.00  | -      |                                 |
| 4   | 97    | 98.00  | -      |                                 |
| 5   | 97    | 97.33  | 98.20  |                                 |
| 6   | 98    | 97.33  | 97.80  | still below                     |
| 7   | 102   | 99.00  | 98.40  | buy: 99.00 crossed above 98.40  |
| 8   | 106   | 102.00 | 100.00 | hold                            |
| 9   | 98    | 102.00 | 100.20 | hold                            |
| 10  | 90    | 98.00  | 98.80  | sell: 98.00 crossed below 98.80 |

The first six days are the quiet period: a slow drift down in which the short average stays below the
long average, and the two lines creep closer and closer together because the prices are moving only a
little. On day 7 the price jumps from 98 to 102. The three-day average jumps to 99.00 while the
five-day average only reaches 98.40, because four of its five prices are still the old low ones. That
first crossing after the quiet spell is a small event dressed up as a big one: nothing about the
prices yet says the fall is over, only that the last three days were better than the five-day average
of the days before.

The trade then goes badly. Buy 100 shares at 102 on day 7, costing 10,200.00, and sell them at 90 on
day 10, bringing in 9,000.00. The gross loss is 1,200.00. At 0.10 percent per side, the costs add
10.20 and 9.00, so:

```text
Net loss = -1,200.00 - 19.20 = -1,219.20
Return on the money used = -1,219.20 / 10,200.00 = -12.0 percent
```

The example was built this way on purpose. A crossover rule waits for a move and joins it partway, so
it is always exposed to the move reversing just after it acts. The numbers here are made up, but the
shape is not: a signal that confirms a three-day rise can be followed by a three-day fall.

## What the research actually found

The reference study is Brock, Lakonishok and LeBaron (1992). They applied 26 simple rules, including
moving average crossovers of the kind described here, to the Dow Jones Industrial Average from 1897 to
1986. Every one of the 26 beat the benchmark of holding cash, and buy signals after a crossover were
followed by higher returns than sell signals. It was the strongest evidence the idea had ever had.

Sullivan, Timmermann and White (1999) then did two things. They corrected for the fact that 26 rules
had been chosen from a much larger family of rules, and they extended the sample by ten years.

| What was measured                                    | What came out                                                                                                    |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Best rule of the original 26 over 1897 to 1996       | A 50-day average with a 0.01 band, about 9.4 percent a year, still beating the benchmark in sample               |
| The same rules on 1987 to 1996, ten years never used | The best rule's data-snooping-corrected p-value was 0.15, that is, no longer convincing                          |
| 7,846 rules tested on 1897 to 1996                   | The best in-sample rule earned 17.2 percent a year, but it traded 6,310 times and only 2,501 of those trades won |
| The best rule on S&P 500 futures, 1984 to 1996       | Data-snooping-corrected p-value 0.91, meaning the result was indistinguishable from luck                         |

Two further studies fill in the rest. Strobel and Auer (2018) examined variable moving average rules
across many developed markets and individual shares from 1972 to 2015 and found that the predictive
power declined steadily over that period, a result they attribute to the falling tendency of prices to
follow their own recent direction. Lento and Gradojevic (2022) tested many rules on five markets
during the crash of early 2020 and found that the moving average rules did not cover their costs;
only Bollinger Bands and trading range breakouts did. A century-long test where roughly 40 percent of
trades win, with an average gain of 0.29 percent per trade, is a business whose costs decide
everything.

And the defaults. The fastquant defaults of 10 and 30 are two round numbers, printed in the library's
README next to one year of one Philippine stock. That README reports the default pair turning 100,000
into 95,902.74 over 2018, a loss of about 4 percent, and the alternative pair of 15 and 40 turning the
same 100,000 into 102,272.90. Both numbers are one stock, one year, with a commission of zero, and no
test on any other sample. Changing 10 and 30 to 15 and 40 changed the answer from a loss to a profit,
which is the clearest possible warning that the parameter values are part of the result and not a
neutral setting.

## How this project relates to it

The repository implements the simple moving average in Rust, in
[crates/indicators/src/average/sma.rs](../../../crates/indicators/src/average/sma.rs). That file is the
same running average as the table above, kept in a form that updates one day at a time instead of
adding up the window again; the equality of the two is worth checking once. The repository's survey of
what technical rules have actually delivered,
[08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md),
is the place to read about decay and crowding, the two forces that turn a published rule into a
crowded one.

## Where it goes wrong

- The signal is late by construction. The short average cannot cross the long one until the price has
  already moved enough to lift it, so every trade gives up the first part of the move. In exchange the
  rule claims to miss the fall after a top, which it does only partly.
- False crossings in quiet markets. When prices drift sideways the two averages lie almost on top of
  one another and swap sides repeatedly. Each swap is a trade with a cost, and the losses from the
  choppy years are what a decade of profits has to pay for.
- The parameters decide the answer. There is no reason a 10 and a 30 should be right and a 15 and a 40
  wrong; the two produced opposite results on the same year of the same stock. Choosing a pair after
  seeing the backtest is a way of fitting the past.
- The test that mattered was the out-of-sample one. The rules that beat the market over 1897 to 1986
  did not over 1987 to 1996, and the correction for having tried thousands of rule variations removed
  the rest of the result.
- Costs on every crossing. The defaults trade whenever the lines swap, which in a sideways market can
  be several times a year, and the more often the rule changes its mind the more it pays.
- What would have to be true. The idea needs a price move to keep going long enough after the crossing
  to pay for the crossing and the one after it. If trends still exist but start and end faster than
  the averages can follow, the rule buys late and sells late and only ever pays the spread.

## Try it yourself

You need a spreadsheet and twenty to thirty daily closes from any public price page.

1. Put dates in column A and closes in column B.
2. In C3 write `=AVERAGE(B1:B3)` for a three-day average, and in D5 write `=AVERAGE(B1:B5)` for a
   five-day average, then drag both down the sheet.
3. In column E, for each row from the sixth onwards, write `=IF(C5>D5,1,0)` so the cell is 1 when the
   short average is above the long one.
4. In column F write `=E5-E4`. A value of 1 is a buy day and -1 is a sell day.
5. Count the number of buy days over the whole sheet, then divide by the number of years the sheet
   covers.

What to notice: a 20-row sheet often produces two or three crossings, all of them caused by quite
small moves. Now replace the 3 and the 5 with 10 and 30 and look at how the count changes. The slow
pair trades far less and reacts far later. Neither version knows anything about next week; both are
descriptions of the week that has just ended.

## Where this came from

- The [fastquant](https://github.com/enzoampil/fastquant) README, for the alias `smac`, the parameter
  names `fast_period` and `slow_period`, and the one-stock example results.
- The [moving average crossover source](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/ma_crossover.py),
  for the exact rule: a crossover of the two averages, up to buy and down to sell.
- The [backtest documentation](https://github.com/enzoampil/fastquant/blob/master/docs/docusaurus/docs/backtest.md),
  for the parameters that control how much is bought and sold.
- Brock, Lakonishok and LeBaron (1992), [Simple Technical Trading Rules and the Stochastic Properties of Stock Returns](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1992.tb04681.x),
  the 26 rules on the Dow Jones from 1897 to 1986.
- Sullivan, Timmermann and White (1999), [Data-Snooping, Technical Trading Rule Performance, and the Bootstrap](https://www.kevinsheppard.com/files/teaching/mfe/advanced-econometrics/Sullivan_Timmermann_White.pdf),
  the data-snooping correction, the ten-year out-of-sample test, and the trade counts.
- Strobel and Auer (2018), [Does the predictive power of variable moving average rules vanish over time and can we explain such tendencies?](https://doi.org/10.1016/j.iref.2017.10.012),
  the decline in predictive power from 1972 to 2015.
- Lento and Gradojevic (2022), [The Profitability of Technical Analysis during the COVID-19 Market Meltdown](https://www.mdpi.com/1911-8074/15/5/192),
  for the finding that moving average rules did not survive costs in that episode.
- The repository's own survey,
  [08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md).

## Words used in this tutorial

- benchmark: the thing a strategy is compared with, usually simply owning the index.
- closing price: the price of the last trade of the day, used as that day's price.
- commission: the fee a broker charges for buying or selling.
- drawdown: the fall from a peak to the following low, measured in percent.
- moving average: the plain average of the last few prices, recomputed each day.
- spread: the gap between the price at which you can sell and the price at which you can buy.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
