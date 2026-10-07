# The moving average convergence divergence: the gap between two averages

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                             |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One company's shares, or one fund that tracks an index, bought and sold whole                                                                                                                                     |
| How often it trades       | A handful of times a year, fewer than a plain crossover, because the rule also demands that a control average be pointing the other way                                                                           |
| What you need             | A spreadsheet                                                                                                                                                                                                     |
| Where the rules come from | The strategy table in the [fastquant](https://github.com/enzoampil/fastquant) README (alias `macd`) and its [source file](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/macd.py) |
| The underlying research   | Gerald Appel, who developed the indicator in the late 1970s, described in the [canonical entry](https://en.wikipedia.org/wiki/MACD)                                                                               |
| How well it held up       | Mixed: the simpler version of the indicator earned money in a few markets and the signal-crossing version used here lost money in others, and a recent study found the plain rules win under half their trades    |
| Also appears in           | [emac](../emac/README.md) and [smac](../smac/README.md) in this same library, whose crossover rule is the simpler cousin                                                                                          |

## The idea in one paragraph

Take two exponential averages of the price, a short one and a long one. Subtract the long from the
short. The gap between them grows when the price is accelerating and shrinks when it is slowing, so
the gap is a kind of speedometer for the trend. Now average that gap itself to get a second, smoother
line, and buy when the gap crosses from below the smoother line to above it. This library adds one
extra condition: it buys only when a plain, slower average of the price is still pointing down, and
sells only when that average is still pointing up. The bet is that a turn in the speedometer, taken
while the wider market is still going the other way, catches a change of direction early.

## Why anyone believed it

Two ideas are bundled together. The first is the trend story from the average crossover: news spreads
slowly, some buyers act late, and prices move in runs. The second is that the speed of the move
carries information of its own. A price that is rising, and rising faster, is being chased; a price
that is rising but slowing down is meeting resistance. The gap between a fast and a slow average
measures that change of speed rather than the level, and averaging the gap smooths out the one-day
noise.

The counterparty is the trader who reacts to levels rather than to speed, and the slow seller. While
they are still selling into a fall, the fast average has already stopped falling as quickly, the gap
turns up, and the rule buys from them. The library's extra filter, which insists the slower average
still point down, is an attempt to buy exactly at the moment when the crowd is still pessimistic.

## An everyday comparison

A courier company tracks how many parcels it delivers each day. The daily count is noisy, so the
manager keeps a short average of the last twelve days and a long average of the last twenty-six. When
the deliveries are speeding up, the short average runs ahead of the long one and the gap between them
grows. The manager does not act on the gap itself, because it jumps around; she keeps a second average
of the gap, and only takes on more vans when the gap climbs above its own average. Even then she wants
to see that annual demand is still falling, because in her experience the first sign of a genuine
turn is a jump in short-run deliveries while the yearly picture still looks gloomy. That last
condition is the fastquant rule in miniature.

## The rules, step by step

1. Pick one thing to trade: one company's shares, or one fund that tracks an index.
2. Get the daily closing price.
3. Choose three period lengths. `fast_period` is the short exponential average, `slow_period` the long
   one, and `signal_period` the length of the average applied to the gap. The fastquant defaults are
   12, 26 and 9, which are also the numbers in common use.
4. Choose two more lengths for the control condition: `sma_period`, the length of a plain average of
   the price, default 30, and `dir_period`, the number of days over which that average's direction is
   measured, default 10.
5. Each day, compute the gap and its average, as in the next section.
6. Compute the control value `smadir`: the plain average today minus the same plain average
   `dir_period` days ago. It is positive when the control average is rising and negative when it is
   falling.
7. Buy when the gap crosses from at or below its average to above it, and the control value is
   negative.
8. Sell when the gap crosses from at or above its average to below it, and the control value is
   positive.
9. Hold at every other time. Review each day at the close.

Both halves of the condition matter. A crossing on its own is not enough, and a falling control
average on its own is not enough; the library's source code requires the two together. That is why this
rule trades less often than the plain crossover.

## The maths, with every symbol named

The two exponential averages use the recursion from the exponential crossover:

```text
alpha_fast = 2 / (fast_period + 1)
alpha_slow = 2 / (slow_period + 1)
EMA_fast,t = alpha_fast * P_t + (1 - alpha_fast) * EMA_fast,(t-1)
EMA_slow,t = alpha_slow * P_t + (1 - alpha_slow) * EMA_slow,(t-1)
```

- `P_t` is today's closing price.
- `EMA_fast,t` and `EMA_slow,t` are the short and long exponential averages on day `t`.
- `alpha_fast` and `alpha_slow` are the weights given to today's price in each of them, and both lines
  start from the plain average of their first `fast_period` and `slow_period` prices.

The gap and its average are then:

```text
MACD_t   = EMA_fast,t - EMA_slow,t
signal_t = signal_(t-1) + alpha_signal * (MACD_t - signal_(t-1)), where alpha_signal = 2/(signal_period+1)
histogram_t = MACD_t - signal_t
```

- `MACD_t` is the gap on day `t`, in price units; a value of 0.50 means the short average is half a
  unit of price above the long one.
- `signal_t` is an exponential average of the gap itself, so it lags the gap.
- `histogram_t` is the distance between the two lines, positive when the gap is above its average.
- The signal line starts from the plain average of the first `signal_period` values of the gap.

The control value uses a plain, unweighted average of the price:

```text
SMA_t(N) = (P_t + P_(t-1) + ... + P_(t-N+1)) / N
smadir_t = SMA_t(sma_period) - SMA_(t - dir_period)(sma_period)
```

- `N` here is `sma_period`, the number of days in the control average.
- `smadir_t` is how much that average has changed over the last `dir_period` days.
- It is negative when the control average is lower than it was, that is, when the wider picture is
  still falling, which is the condition for buying.

Finally the two sides of the rule, written out:

```text
Buy  when MACD_t > signal_t and MACD_(t-1) <= signal_(t-1) and smadir_t < 0
Sell when MACD_t < signal_t and MACD_(t-1) >= signal_(t-1) and smadir_t > 0
```

- The first two conditions are the crossing of the gap through its own average.
- The last condition is the direction filter.
- Both must hold on the same day; there is no separate stop loss or target in the defaults.

## A worked example

Ten made-up closes. To keep the table readable the periods are shortened to `fast_period` 2,
`slow_period` 4, `signal_period` 2, `sma_period` 3 and `dir_period` 2. The rule and the parameter
names are exactly the ones above; only the lengths are smaller.

| Day | Close | EMA(2) | EMA(4) | MACD  | Signal | SMA(3) | smadir |
| --- | ----- | ------ | ------ | ----- | ------ | ------ | ------ |
| 1   | 110   | -      | -      | -     | -      | -      | -      |
| 2   | 108   | 109.00 | -      | -     | -      | -      | -      |
| 3   | 104   | 105.67 | -      | -     | -      | 107.33 | -      |
| 4   | 99    | 101.22 | 105.25 | -4.03 | -      | 103.67 | -      |
| 5   | 95    | 97.07  | 101.15 | -4.08 | -4.05  | 99.33  | -8.00  |
| 6   | 92    | 93.69  | 97.49  | -3.80 | -3.88  | 95.33  | -8.33  |
| 7   | 94    | 93.90  | 96.09  | -2.20 | -2.76  | 93.67  | -5.67  |
| 8   | 98    | 96.63  | 96.86  | -0.22 | -1.07  | 94.67  | -0.67  |
| 9   | 102   | 100.21 | 98.91  | 1.30  | 0.51   | 98.00  | +4.33  |
| 10  | 106   | 104.07 | 101.75 | 2.32  | 1.72   | 102.00 | +7.33  |

Read the table from the bottom of the fall. On day 5 the gap is -4.08, below its average of -4.05. On
day 6 the gap is -3.80, above its average of -3.88: the gap has crossed up. The control value that
day is -8.33, the three-day average of the price sitting well below where it was two days earlier, so
the wider picture is still falling. Both halves of the buy rule are satisfied, and the position is
bought at the close of 92.

Nothing sells by day 10. The gap stays above its average, and the control value has turned positive,
which is the condition for selling only when the gap crosses down; that crossing has not happened.
The position is still open at the last close of 106.

Now the money. Buy 100 shares at 92, costing 9,200.00, and hold. Mark the position at the last close of
106, which is what it would fetch if sold, and charge 0.10 percent on each side even though only the
buy has happened:

```text
Gross gain, marked to market = (106 - 92) * 100 = 1,400.00
Costs = 0.001 * 9,200.00 + 0.001 * 10,600.00 = 19.80
Net = 1,400.00 - 19.80 = 1,380.20, which is 15.0 percent of the 9,200.00 invested
```

The numbers are invented and were arranged so that a signal appears on the page. What they show is the
arithmetic of the rule, including the fact that the filter makes it wait for a day when the gap and the
control average disagree; on most days they do not, and no trade occurs.

## What the research actually found

Chong, Ng and Liew (2014) tested the MACD on five developed-market indices from 1976 to 2002, using
ten-day returns, and reported both the simple version of the indicator and the signal-crossing version
that this library implements:

| Market                | Version       | What was measured                                                              |
| --------------------- | ------------- | ------------------------------------------------------------------------------ |
| Milan Comit General   | MACD(12,26,0) | Buy-minus-sell 1.379 percent per ten days, significant at the 10 percent level |
| S&P/TSX Composite     | MACD(12,26,0) | Buy-minus-sell 1.335 percent per ten days, significant at the 5 percent level  |
| DAX 30                | MACD(12,26,9) | Buy-minus-sell -0.944 percent per ten days, significant at the 5 percent level |
| Dow Jones Industrials | MACD(12,26,9) | Buy-minus-sell -0.442 percent, not significant                                 |
| Nikkei 225            | MACD(12,26,9) | Buy-minus-sell 0.166 percent, not significant                                  |

So the indicator helped in two of the five markets when used the simplest way, and the version that
crosses the signal line lost money in Germany and added nothing in Japan or on the Dow. After
subtracting a round-trip cost of 1 percent, the simple version remained profitable in Italy and Canada,
the two markets where it had worked, with a net profit of 1.021 percent and 0.776 percent per pair of
trades. The authors also note that the results were not consistent across markets.

The more recent study summarised in this repository's own survey is sharper. It backtested MACD(12,26,9)
rules on the constituents of the Dow Jones, the Nasdaq and the S&P 500 from January 2015 to August
2021, and found that the signal-crossing rule won only 0.40 to 0.49 of its trades, the zero-crossing
rule 0.37, and that the version with the best win rate had a ratio of profits to losses below 1 in every
market, with single losses exceeding 80 percent of the money committed in the S&P 500. Adding a second
indicator, such as the relative strength index, raised the win rate above 0.78, but the study also
found that those filtered versions missed trends and ranked near the bottom on total profit.

The defaults, once more. The fastquant defaults of 12, 26, 9, 30 and 10 are the conventional numbers
for this indicator plus two round numbers of the library's own for the control average. The README
reports them turning 100,000 into 96,229.58 on one Philippine stock over 2018, a loss of about 4
percent, with a commission of zero, on one year of one share. There is nothing in the source to
suggest the defaults were chosen by testing, and the library's own documentation marks the related
Bollinger strategy as naive. A default parameter is where a newcomer starts, not what anyone
established.

## How this project relates to it

The repository implements the gap and its signal line in Rust, in
[crates/indicators/src/momentum/macd.rs](../../../crates/indicators/src/momentum/macd.rs), using the
same three exponential averages as the formulas above. Its survey of the evidence devotes a paragraph
to the recent MACD study quoted here, including the win rates and the outlier losses, in
[08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md).
That survey also records the relative strength index as the filter that lifts the win rate, which is
the link back to the [rsi](../rsi/README.md) tutorial.

## Where it goes wrong

- The filter cuts both ways. Demanding that the control average point down before buying means the
  rule misses every genuine uptrend that never had a final down-swing, which is why the same study
  found the filtered versions ranked near the bottom on total profit.
- Two averages of two averages. The signal line is a smoothed average of a difference of smoothed
  averages, so by the time a crossing occurs the price has usually moved a long way already. The
  indicator is a description of the recent past twice over.
- The parameters decide the answer. The fastquant source does not test the five lengths; they are
  conventions. Testing many combinations and keeping the best reintroduces exactly the trap that the
  data-snooping correction was invented to remove.
- A rising gap is not a reason for anything. The gap widens whenever the price rises, whatever the
  price is worth and whoever is buying. Nothing in the formula mentions a counterparty, a cost or a
  limit on how far the price can stretch.
- Win rate is not profit. A rule can win most of its trades and still lose money if the rare losses are
  much larger than the common wins, which is the pattern the recent study reports for the version with
  the best win rate.
- What would have to be true. The rule needs a turn in the speedometer, taken against the wider
  direction, to mark the beginning of a move rather than a pause inside an existing one. If most such
  turns are pauses, the rule is a way of paying costs on both sides of a fall.

## Try it yourself

You need a spreadsheet and forty daily closes, because a 26-day average needs a long run of data
before it exists at all.

1. Put dates in column A and closes in column B.
2. In C12 enter `=AVERAGE(B1:B12)` as the seed of the fast exponential average, then in C13 enter
   `=0.154*B13+0.846*C12` and drag down. The 0.154 is `2/(12+1)`.
3. In D26 enter `=AVERAGE(B1:B26)` as the seed of the slow average, then in D27 enter
   `=0.074*B27+0.926*D26` and drag down. The 0.074 is `2/(26+1)`.
4. In E27 enter `=C27-D27`, the gap. In F35 enter `=AVERAGE(E27:E35)` as the seed of the signal, then
   in F36 enter `=0.2*E36+0.8*F35` and drag down. The 0.2 is `2/(9+1)`.
5. In column G write `=E36-F36`. A change in the sign of this column is a crossing.
6. In H30 enter `=AVERAGE(B1:B30)` for the control average and drag it down, then in I40 enter
   `=H40-H30`, the change over ten days, and drag that down too.
7. Mark every row where the gap crosses above its average while column I is negative.

What to notice: over a year of daily prices this produces only a few marks, and every one of them
arrives after a fall has been going on for weeks. Check each mark against what the price did over the
following month. If the answer is sometimes right and sometimes wrong, that is the honest picture the
studies report, and it is why a single backtest proves nothing.

## Where this came from

- The [fastquant](https://github.com/enzoampil/fastquant) README, for the alias `macd` and the
  example result. Note that the README's parameter column contains two typos, `fast_perod` and
  `slow_upper`; the names used by the code, and throughout this tutorial, are `fast_period`,
  `slow_period`, `signal_period`, `sma_period` and `dir_period`.
- The [MACD source](https://github.com/enzoampil/fastquant/blob/master/python/fastquant/strategies/macd.py),
  for the exact buy and sell conditions, including the control average and its direction.
- The [backtest documentation](https://github.com/enzoampil/fastquant/blob/master/docs/docusaurus/docs/backtest.md),
  for the parameters that control how much is bought and sold.
- The [canonical description of the indicator](https://en.wikipedia.org/wiki/MACD), for its origin
  with Gerald Appel and the meaning of the three lines.
- Chong, Ng and Liew (2014), [Revisiting the Performance of MACD and RSI Oscillators](https://www.mdpi.com/1911-8074/7/1/1),
  the five-market test and the cost calculation.
- The recent study of MACD rules on American indices, summarised in this repository as `2206.12282v1`
  in
  [08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md).

## Words used in this tutorial

- benchmark: the thing a strategy is compared with, usually simply owning the index.
- convergence: the gap between two averages closing, which is what the name of the indicator refers to.
- exponential moving average: an average in which newer prices count for more than older ones.
- filter: an extra condition that a signal must satisfy before a trade is placed.
- histogram: the distance between the two lines of the indicator, drawn as bars.
- signal line: an average of the gap, used as the level the gap must cross.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
