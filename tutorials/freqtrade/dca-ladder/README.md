# Dollar-cost averaging: buying more as the price falls

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                               |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One cryptocurrency pair on one exchange, for example Bitcoin priced in US dollars, bought in several fixed dollar amounts                                                                                                                           |
| How often it trades       | A first purchase, then one more purchase each time the price falls a further step (in the source files the fall is measured in candles, so it varies)                                                                                               |
| What you need             | A spreadsheet                                                                                                                                                                                                                                       |
| Where the rules come from | [BinHV27.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/BinHV27.py) and [BinHV45.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/BinHV45.py) |
| The underlying research   | Vanguard, [Cost averaging: Invest now or temporarily hold your cash?](https://corporate.vanguard.com/content/dam/corp/research/pdf/cost_averaging_invest_now_or_temporarily_hold_your_cash.pdf), which follows Constantinides (1979)                |
| How well it held up       | Weak: the ladder itself has no published test at all, and the closest academic evidence finds that investing the whole amount at once beat spreading it out about two thirds of the time                                                            |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                     |

## The idea in one paragraph

Instead of buying a coin once with all the money you intend to use, you split that money into
equal slices and buy one slice now. If the price later falls by a fixed step, you buy another
slice; if it falls another step, you buy a third. Because the later slices are bought at lower
prices, the average price you paid drifts down, so a smaller recovery is enough to get back to
break-even. The cost is that your money is going into something that is currently losing value, and
the total amount at risk grows with every step. The two source files supply the moment to place the
first slice, not the ladder itself.

## Why anyone believed it

A falling price is the market telling you that other people are selling. Some of those sellers have
reasons unrelated to what the coin is worth: a large holder reducing a position, a leveraged trader
forced to sell by a lender, or an exchange-traded fund processing withdrawals. If the seller is
forced and the buyer is patient, buying into the fall can be buying at a price that will not last.

The reason to do it in steps rather than all at once is that nobody knows where the fall stops. A
single buyer who commits everything at the first drop may have nothing left to buy at any lower
price. Somebody has to be willing to hold a falling asset and be wrong for a while; the ladder is a
way of arranging to be wrong with a smaller average error.

## An everyday comparison

Think of a shop that sells fresh bread at full price in the morning. Late in the day it marks the
loaves down, then down again, because day-old bread is worth less to the shop than an empty shelf.
A customer who buys four loaves over the afternoon at 4 dollars, 3.60, 3.24 and 2.92 has paid an
average of about 3.44, well below the morning price, but has also committed four times as much
money to bread that might end up stale. The shop is not wrong about the price falling; the ladder
is a bet that the fall will stop before the bread is worthless.

## The rules, step by step

1. Choose one cryptocurrency pair on one exchange, for example Bitcoin against a stablecoin.
2. Choose a step, for example a fall of 10 percent from the last purchase price.
3. Choose a fixed amount, for example 100 dollars, to spend at every step. The amount is the same
   every time; that is what makes it a ladder rather than a bet.
4. Place the first slice when the entry condition in the source file is met. The two files use
   different conditions and different candle lengths:
   - BinHV45 reads one-minute candles. It buys when the previous candle closed below the lower
     Bollinger band of 40 one-minute prices at two standard deviations, the close fell from the
     previous close by more than 1.7 percent of the price, the candle closed in its lower part, and
     the gap between the middle and the lower band is more than 0.7 percent of the price. The three
     percentages are the stored settings `buy_bbdelta = 7`, `buy_closedelta = 17` and `buy_tail = 25`,
     each divided by 1000 in the code.
   - BinHV27 reads five-minute candles. It buys when the close is below both a 60-period and a
     120-period exponential average, the minus direction indicator is above its own 25-period
     average, a 5-period momentum index is not falling, and one of four trend combinations holds:
     the trend strength index above 25 while the market is not in an uptrend, above 30 while it is,
     or above 30 to 35 with the smoothed momentum index at 20 or below.
5. Do not sell on the first slice. If the price falls another full step, spend the next 100 dollars,
   and so on.
6. Take the profit when the file says so. BinHV45 has no sell rule at all; it leaves the position
   until the profit target or the stop. BinHV27 also lets the profit target and the stop do most of
   the work.
7. The stored settings are these. BinHV45: `timeframe = '1m'`, `stoploss = -0.05` (a 5 percent
   loss closes the position), `minimal_roi = {"0": 0.0125}` (sell as soon as the profit reaches
   1.25 percent). BinHV27: `timeframe = '5m'`, `stoploss = -0.50` (a 50 percent loss), and
   `minimal_roi = {"0": 1}` (sell only at a 100 percent profit).

## The maths, with every symbol named

One purchase, then the average of all of them.

```text
u_i = D / P_i
```

- `u_i` is the number of coins bought in purchase `i`.
- `D` is the fixed amount of money spent in each purchase, in the same currency throughout.
- `P_i` is the price of one coin at purchase `i`.

So a 100 dollar purchase at a price of 90 dollars buys 1.1111 coins. Now add the purchases up.

```text
U = u_1 + u_2 + ... + u_n
C = D + D + ... + D = n * D
A = C / U
```

- `U` is the total number of coins held after `n` purchases.
- `C` is the total cash spent.
- `A` is the average price: total cash divided by total coins.

With equal amounts at each step, this average is the harmonic mean of the prices, which is always
at or below the ordinary average of the prices:

```text
A = n / (1/P_1 + 1/P_2 + ... + 1/P_n)
```

Finally, the position is worth less than it cost whenever the market price is below `A`:

```text
unrealised return = P_current / A - 1
```

- `P_current` is the latest market price.
- A negative result is a paper loss; nothing is realised until you sell.

## A worked example

You buy 100 dollars at each of four prices, the price falling by 10 percent each time. The
arithmetic below uses a 0.10 percent exchange fee on each purchase, which is inside the realistic
range of 0.05 to 0.10 percent per side.

| Purchase | Price  | Cash spent | Fee  | Coins bought |
| -------- | ------ | ---------- | ---- | ------------ |
| First    | 100.00 | 100.00     | 0.10 | 0.9990       |
| Second   | 90.00  | 100.00     | 0.10 | 1.1100       |
| Third    | 81.00  | 100.00     | 0.10 | 1.2333       |
| Fourth   | 72.90  | 100.00     | 0.10 | 1.3704       |
| Total    |        | 400.00     | 0.40 | 4.7127       |

The average price is `400.00 / 4.7127 = 84.88`, against a first-purchase price of 100.00. Now follow
the position at the moment of the fourth purchase and after a recovery.

| What we look at                 | Arithmetic          | Result         |
| ------------------------------- | ------------------- | -------------- |
| Value at the current 72.90      | 4.7127 * 72.90      | 343.56         |
| Paper loss at 72.90             | 343.56 / 400.00 - 1 | -14.11 percent |
| Value after a recovery to 90.00 | 4.7127 * 90.00      | 424.14         |
| Sell fee at 0.10 percent        | 424.14 * 0.001      | 0.42           |
| Cash after selling              | 424.14 - 0.42       | 423.72         |
| Net profit                      | 423.72 / 400.00 - 1 | +5.93 percent  |

Compare the same 400 dollars placed as a single purchase at 100.00. That buys 4.0000 coins, worth
360.00 at a price of 90.00, a loss of about 10 percent. The ladder has converted a loss into a small
gain, because it bought more coins at the lower prices.

Now the honest half. At the moment the average price is at its best (84.88) the paper loss is at its
worst in dollars: 56.44 out of 400. If the price keeps falling to 60.00, the ladder holds 4.7127
coins worth 282.76, a loss of 29.3 percent, while a single purchase at 100.00 would hold 4.0000
coins worth 240.00, a loss of 40 percent. The ladder loses a smaller percentage but it has 400
dollars committed rather than the smaller amount that a cautious buyer would have committed, and
BinHV27's stop loss would accept a loss of 50 percent before closing. Neither outcome is a floor.

## What the research actually found

The ladder itself has no published test. What exists is research on the simpler question: is it
better to invest a sum of money all at once, or spread it over weeks and months?

| Source                                                         | What it measured                                                         | Result                                                                                                                                                         |
| -------------------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Constantinides, Journal of Financial and Quantitative Analysis | The mathematics of averaging into a market with positive expected return | Called dollar-cost averaging suboptimal as an investment policy, because cash held back earns no market return                                                 |
| Vanguard, 2023                                                 | Global stock and bond markets 1976 to 2022, plus 10,000 simulated paths  | Investing the whole amount at once beat a three-month averaging plan in 68 percent of historical cases; the average one-year gap was about 2 percent of wealth |
| Vanguard, 2023                                                 | The same data, split into risk percentiles                               | Averaging lost in the median case but did better in the worst 5 percent of outcomes, which is its only measurable benefit                                      |

Read that carefully. The evidence does not say that averaging into a fall is profitable. It says
averaging slightly lowers both the return and the chance of buying at the single worst moment. In
crypto the coin can also go to zero, which no market average includes.

## How this project relates to it

The repository's research brief on trading costs,
[Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
explains why a ladder pays more than its fee schedule suggests: each purchase is an order that moves
the price against you, and the brief reports a measured square-root relation between the size of an
order and the price move it causes. A ladder of four small orders is cheaper than one large order,
which is a real argument for stepping in, though a different one from the one the ladder is usually
sold on.

The brief on [Market making and inventory](../../../strategies/books/04_market_making_and_inventory.md)
describes the dealer on the other side of your purchases and how that dealer prices the risk of
holding what you sell. The brief on
[Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
is the reason to distrust any ladder whose step and amount were chosen by looking at past prices.

## Where it goes wrong

- There is no floor. Averaging down works on the price history you can see and fails completely if
  the coin loses its market; the average price then only measures how much you lost.
- The step and the amount are free parameters. A 10 percent step with 100 dollar slices is one
  choice among thousands, and BinHV45's own numbers (7, 17 and 25, divided by 1000) came from a
  search over the author's sample, which is exactly how a rule gets fitted to the past.
- The stop and the ladder fight each other. BinHV27 tolerates a 50 percent loss, so a ladder built
  on its entry rule can be stopped out far below the average price, realising the loss the ladder
  was meant to avoid.
- Costs and the gap between buying and selling prices. On a thin pair the gap can be several times
  the 0.10 percent fee, and every slice pays it. A ladder that trades five times pays five times.
- The evidence points the other way. The largest published study of this exact decision found that
  investing at once won most of the time, so the ladder is a preference for a smaller worst case,
  not a source of extra return.
- A recovery is assumed. Every version of the arithmetic above ends with the price rising back
  through the average. Nothing in the setup makes that happen.

## Try it yourself

You need a spreadsheet and any coin's daily closing prices for a few years.

1. Column A: the date. Column B: the closing price. Column C: the price's percentage fall from the
   highest price seen so far.
2. Column D: put 100 in a cell whenever the fall in column C crosses a new multiple of 10 percent,
   and leave it empty otherwise.
3. Column E: the coins bought, which is 100 divided by the price in column B on those rows.
4. Column F: the running total of column E, and column G: the running total of the cash spent.
5. Column H: the average price, which is column G divided by column F, shown only on rows where you
   bought.
6. Column I: the paper profit or loss, which is the price in column B divided by the average in
   column H, minus one.

What to notice: the average price in column H falls for a while and then stops falling, while the
paper loss in column I keeps growing whenever the price keeps dropping. Also compare the number of
rows where you bought with the total number of rows; on most days the ladder does nothing, which is
the point and also the cost.

## Where this came from

- [BinHV45.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/BinHV45.py),
  the one-minute dip-buying rule, its stored parameters and its profit target and stop.
- [BinHV27.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/berlinguyinca/BinHV27.py),
  the five-minute dip-buying rule with the 50 percent stop and the 100 percent target.
- [The freqtrade strategies README](https://github.com/freqtrade/freqtrade-strategies/blob/main/README.md),
  which states that the files are starting points for your own work and not ready-to-use strategies.
- Vanguard, [Cost averaging: Invest now or temporarily hold your cash?](https://corporate.vanguard.com/content/dam/corp/research/pdf/cost_averaging_invest_now_or_temporarily_hold_your_cash.pdf),
  the lump-sum versus averaging comparison quoted above.
- Constantinides, [A Note on the Suboptimality of Dollar-Cost Averaging as an Investment Policy](https://www.cambridge.org/core/journals/journal-of-financial-and-quantitative-analysis/article/abs/note-on-the-suboptimality-of-dollarcost-averaging-as-an-investment-policy/0C483B96429655B24F34FB628CF9CEEB),
  the 1979 argument in the other direction.
- [Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  this repository's brief on the cost of each order.

## Words used in this tutorial

- average price: the total money spent divided by the total coins bought.
- candle: one fixed time slice of a price chart, such as one minute, holding the open, high, low and
  close for that slice.
- dollar-cost averaging: investing a fixed amount on a schedule, or at fixed price steps, instead of
  all at once.
- drawdown: the fall from a peak to the following low, here measured on the position rather than on
  a market.
- gap between buying and selling price: what you pay to buy versus what you receive to sell;
  narrower on busy pairs, wider on quiet ones.
- stop loss: a standing instruction to close the position once it has lost a set percentage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
