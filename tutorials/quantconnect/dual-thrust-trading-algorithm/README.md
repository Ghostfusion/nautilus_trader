# Dual Thrust: setting a buy price and a sell price before the day begins

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One widely traded thing at a time, such as a fund that tracks a stock index, a single share or a currency pair                                                                                          |
| How often it trades       | At most a few times a day; it holds whatever it bought until the opposite price level is touched                                                                                                        |
| What you need             | A spreadsheet and daily records of the high, low, close and opening price                                                                                                                               |
| Where the rules come from | [QuantConnect strategy library, dual thrust trading algorithm](https://www.quantconnect.com/tutorials/strategy-library/dual-thrust-trading-algorithm)                                                   |
| The underlying research   | A practitioner rule attributed to Michael Chalek and restated by Gang Wei, [Dual Thrust Intraday Strategy](https://www.xcf.cn/zjfxs/yjzs/jrgc/201210/P020121004348795101761.pdf) (in Chinese), May 2012 |
| How well it held up       | Weak: this is a practitioner's rule of thumb with no peer-reviewed test, and the library's own backtest on an index fund reported a negative reward-for-risk figure and a 41.1 percent worst fall       |
| Also appears in           | Nothing else in this collection describes the same rule; the closest is the execution tutorial in the project folder, which is about what a trade actually costs                                        |

## The idea in one paragraph

Each market day a price wanders up and down from where it opened. This strategy looks at how far the
price has swung over the last few days, turns that swing into a distance, and draws two lines for
today: one above the opening price and one below it. If the price climbs through the upper line, it
buys. If the price falls through the lower line, it sells. The bet is that a day which starts moving
in one direction keeps going that way long enough to pay for the trip. The strategy does not predict
which line will be touched; it waits for the market to choose one and then follows.

## Why anyone believed it

Most of the time prices wander without a direction, but sometimes a piece of news arrives and the
price keeps drifting the same way for hours because buyers and sellers do not all react at once.
Traders who must sell for reasons of their own, such as a fund raising cash, push the price down
regardless of the news, and the price keeps falling past where the news alone would have taken it.

The counterparty here is the trader who is slow to act, or who is forced to trade for reasons
unrelated to the outlook. If enough of those orders arrive in the same direction, the price that
crosses a line keeps going instead of turning back, and the strategy is paid for waiting for the
crossing instead of guessing it.

## An everyday comparison

Think of a bus stop where one bus is due every ten minutes. On a normal morning the crowd at the stop
grows slowly. On the morning after a big event nearby, the crowd is already larger than usual before
the first bus arrives, and once it starts growing it keeps growing, because each new arrival sees the
crowd and joins it. The strategy is not trying to guess how big the crowd will be. It waits until the
crowd is visibly larger than the usual swings allow, then joins, betting that a crowd that starts
growing for a real reason keeps growing for a while.

## The rules, step by step

1. Choose one market that trades continuously through the day with a tight gap between its buying and
   selling prices, such as a fund that tracks a broad stock index.
2. Keep a record of each day's highest price traded (high), lowest price traded (low) and closing
   price (close).
3. Before the market opens, look at the last four completed days, which is the number the library
   page uses. From those four days take the highest high, the lowest low, the highest close and the
   lowest close.
4. Work out two candidate distances and keep the larger one:
   `highest high minus lowest close`, and `highest close minus lowest low`.
5. Write today's opening price, the first price at which the market trades when it opens.
6. Set a buy line at `opening price + 0.5 times the distance from step 4`, and a sell line at
   `opening price - 0.5 times the distance from step 4`. The two multipliers are commonly both one
   half, written `K1 = K2 = 0.5`.
7. During the day, if the price trades up to or through the buy line, buy with the whole account. If
   it trades down to or through the sell line, sell with the whole account.
8. This is a reversal system. If you are already short when the price touches the buy line, buy back
   the short first and only then go long. If you are already long when the price touches the sell
   line, sell the long first and only then go short.
9. Hold the position until the opposite line is touched, which can be the next day or several days
   later. Recompute both lines each morning from the freshly completed four days.

One detail deserves care. A larger `K1` puts the buy line further above the opening price, so the buy
line is harder to touch, not easier. A larger `K2` puts the sell line further below the opening price,
so the sell line is harder to touch. If you want the buying side to trigger more often, make `K1`
smaller than `K2`. The library page says the opposite in one sentence and then says the correct thing
in the next; the arithmetic above is the rule to trust.

## The maths, with every symbol named

The distance the strategy uses is built from four numbers taken from the recent days:

```text
range = max(HH - LC, HC - LL)
```

- `range` is the size of the recent swing, in the same units as the price.
- `HH` is the highest high over the last `N` completed days.
- `LL` is the lowest low over the same days.
- `HC` is the highest close over the same days.
- `LC` is the lowest close over the same days.
- `N` is the number of days looked back, four on the library page.
- `max(...)` means take whichever of the two subtractions is larger.

Subtracting the lowest close from the highest high measures the full span the price covered. Taking
the larger of the two ways of measuring it makes the swing a little wider than either measure alone,
so the lines sit a little further from the opening price.

The two lines for the day are then:

```text
buy_line  = open + K1 * range
sell_line = open - K2 * range
```

- `buy_line` is the price at or above which the strategy buys.
- `sell_line` is the price at or below which the strategy sells.
- `open` is today's opening price.
- `K1` and `K2` are multipliers that set how far each line sits from the open. The library page uses
  `K1 = K2 = 0.5`.

Finally, the money result of one completed trip. Buying at the buy line and later closing at price
`P_out` on a fixed number of units `Q`:

```text
gross profit = Q * (P_out - P_in) - Q * c * (P_out + P_in)
```

- `Q` is the number of units bought.
- `P_in` is the price paid on the way in.
- `P_out` is the price received on the way out.
- `c` is the cost of one crossing, as a fraction of the amount traded, covering the gap between the
  buying and selling price plus any commission. A realistic figure for a large index fund is 0.0005,
  that is five basis points, where one basis point is one hundredth of one percent.
- The last term is the total cost, because the trade crosses the spread twice, once going in and once
  coming out.

## A worked example

Four completed days, from which the swing is measured.

| Day | High   | Low    | Close  |
| --- | ------ | ------ | ------ |
| 1   | 100.00 | 98.00  | 99.00  |
| 2   | 101.00 | 99.00  | 100.00 |
| 3   | 102.00 | 100.00 | 101.00 |
| 4   | 103.00 | 101.00 | 102.00 |

From these four days: `HH = 103.00`, `LL = 98.00`, `HC = 103.00`, `LC = 99.00`. Then
`HH - LC = 103.00 - 99.00 = 4.00`, and `HC - LL = 103.00 - 98.00 = 5.00`, so `range = 5.00`.

Day 5 opens at 102.50. With `K1 = K2 = 0.5`:

```text
buy_line  = 102.50 + 0.5 * 5.00 = 105.00
sell_line = 102.50 - 0.5 * 5.00 = 100.00
```

During day 5 the price rises through 105.00, so the strategy buys at 105.00. The account holds
100,000.00, so the number of units is `100,000.00 / 105.00 = 952.38`, rounded down to 952 units. The
position stays open while the price keeps rising, and the lines are recomputed each morning.

| Day | Open   | Range | Buy line | Sell line | What happened                          |
| --- | ------ | ----- | -------- | --------- | -------------------------------------- |
| 5   | 102.50 | 5.00  | 105.00   | 100.00    | Price reached 105.00, bought 952 units |
| 6   | 105.50 | 7.00  | 109.00   | 102.00    | Neither line touched, held             |
| 7   | 106.50 | 8.00  | 110.50   | 102.50    | Price reached 110.50, already long     |
| 8   | 110.80 | 11.00 | 116.30   | 105.30    | Neither line touched, held             |

For the day 6 range, the four completed days are days 2 to 5, whose highest high is 106.00, lowest
low is 99.00, highest close is 106.00 and lowest close is 100.00, giving
`max(106.00 - 100.00, 106.00 - 99.00) = max(6.00, 7.00) = 7.00`. The other rows are built the same
way from the four days above them.

Suppose the trader closes the position at the end of day 8 at 112.50, which the rules do not require
but which ends the example. The 952 units were bought at 105.00 and sold at 112.50:

```text
gross profit = 952 * (112.50 - 105.00) = 952 * 7.50 = 7,140.00
cost         = 952 * 0.0005 * (112.50 + 105.00) = 952 * 0.0005 * 217.50 = 103.53
net profit   = 7,140.00 - 103.53 = 7,036.47, which is 7.04 percent of the 100,000.00 account
```

Two things to notice. The cost of a single round trip is small, about 103.53 on a 100,000.00 account,
so the cost is not what makes this strategy fail; the risk is the many small losses from lines that
are touched and then immediately reversed. And the position was held for four days because neither
line was touched on days 6 and 8, which is the opposite of a buy-in-the-morning, sell-in-the-evening
pattern many readers expect from the word intraday.

## What the research actually found

The library page ran the rule on the SPDR S&P 500 fund, a fund that tracks a broad index of American
shares, from 2004 to 2017, with hourly prices and the four-day swing. It reported a reward-for-risk
figure, the Sharpe ratio, of -0.17 and a worst fall from a peak of 41.1 percent, which are poor
outcomes. A Sharpe ratio below zero means the returns were worse than holding cash over the period.

The same page contradicts itself. Its summary sentence says the result "suggested that the strategy
beat the market", while its own conclusion states the negative Sharpe ratio and the 41.1 percent
drawdown. Read together, the honest report is that the test did not work. The page also tested
individual shares and said the rule behaved better in a market that was already trending and worse in
a choppy market, where it triggered many signals that were immediately reversed.

The reference list has a single entry, the Chinese-language note by Gang Wei from May 2012, which
states the rule and its formulas. The original idea is credited to Michael Chalek, a practitioner.
There is no peer-reviewed study, no independent replication with costs included, and no entry on
Quantpedia, the online encyclopedia of quantified strategies. Nothing in the sources measures the
slippage of an intraday order or the number of times the price touches a line and then turns back.

## How this project relates to it

This repository holds a design for the machinery a breakout rule like this would need in order to be
measured honestly, in [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md).
Its Section 6 defines the effective price of a buy or a sell, including the spread, the slippage and
the market impact, which is exactly what stops the lines above from being touched for free. Section 7
defines the triple-barrier exit, the clean way to say when a position ends. The companion document,
[Entry/Exit Price Engine: implementation](../../../strategies/entry_exit_engine_implementation.md),
specifies how those pieces would be added to the sector regime engine that already exists in
[implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md). None
of it runs this strategy today; it is the measurement layer the strategy would need.

The evidence on rules of this family is collected in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
whose Section 5 reports that plain indicator rules win under half their trades and that the apparent
gain from tuning their parameters is driven by a handful of lucky outcomes.

## Where it goes wrong

- Whipsaw. A choppy day touches the buy line and then the sell line and then the buy line again, and
  each crossing pays the cost of a round trip while the price goes nowhere. This is the failure the
  library page names directly.
- The costs are paid per crossing, not per day. A reversal system that flips several times a day
  crosses the spread many times, and on a thin or fast-moving market the cost per crossing is much
  larger than the five basis points assumed above.
- The levels are drawn from a short, fixed window. Four days of history can be very quiet, making the
  swing small and the lines close to the open, so ordinary noise triggers the trade; or very wild,
  making the lines so far away that the trade almost never happens.
- The origin of the rule is a practitioner's note. There is no sample, no stated costs and no
  out-of-sample test behind the reference, so the only measurement is the library's own negative one.
- Being on the other side of yourself. If many traders watch the same four-day swing and the same
  round number, they reach the same lines, and the price can be pushed through a line to catch their
  orders before falling back, which is a real hazard for a published rule.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily prices with highs, lows and closes.

1. Make columns for the date, the opening price, the high, the low and the close, one row per trading
   day, for a share or fund you can look up.
2. Add a column for `HH - LC` over the last four days, and another for `HC - LL`. Use the four rows
   above the current one.
3. Add a column for the range, the larger of the two.
4. Add columns for the buy line (`open + 0.5 * range`) and the sell line (`open - 0.5 * range`).
5. Add a column that records whether the day's high reached the buy line, whether the day's low
   reached the sell line, or neither.
6. Assume the trade acts only once a day, at the first line touched, and count how many days were
   crossings and how many of those crossings were followed the next day by a move in the same
   direction.

What to notice: change the look-back from four days to ten and watch how much the count of crossings
falls, because the range widens and the lines move further away. Also notice how often both lines are
touched on the same day, which is the whipsaw the sources warn about, and which a single-price
backtest hides.

## Where this came from

- [QuantConnect strategy library: dual thrust trading algorithm](https://www.quantconnect.com/tutorials/strategy-library/dual-thrust-trading-algorithm),
  the rules as implemented, the parameter choices, the SPY 2004 to 2017 result and the reference list.
- Gang Wei, [Dual Thrust Intraday Strategy](https://www.xcf.cn/zjfxs/yjzs/jrgc/201210/P020121004348795101761.pdf),
  May 2012, the Chinese-language note the library page cites for the formulas. The note is in Chinese
  and was used only as the source of attribution, not read for numbers.
- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Sections 6 and 7,
  for the effective price of a trade and the triple-barrier exit.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  Section 5, for the evidence that indicator rules of this kind win under half their trades.
- [Execution at the price you get](../../project/execution-at-the-price-you-get/README.md), the
  project tutorial that explains what a fill actually costs, which is the piece this strategy's
  sources leave out.

## Words used in this tutorial

- basis point: one hundredth of one percent, so five basis points is 0.05 percent.
- breakout: a rule that acts only after the price leaves a previously defined range or level.
- drawdown: the fall from a peak to the following low, measured in percent.
- intraday: happening within a single trading day, rather than held from one day to the next.
- long: owning something, so that a rise in its price is a gain.
- short selling: borrowing something you do not own, selling it, and buying it back later, so that a
  fall in its price is a gain.
- slippage: the difference between the price you expected and the price you actually got.
- whipsaw: a price that crosses a level and then immediately crosses back, producing a small loss.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
