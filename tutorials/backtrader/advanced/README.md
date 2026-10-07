# Advanced framework patterns: optimisation, signals and choosing a strategy at runtime

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | An American share index represented by daily bars, and in one file two daily price series at once                                                                                                                                   |
| How often it trades       | From ten trades in a year for the optimised rule to about once a month for the signal rule, and every few months for the multi-series rule                                                                                          |
| What you need             | Python and a data file if you want to reproduce the runs; this page explains the ideas with arithmetic alone                                                                                                                        |
| Where the rules come from | [Strategy compendium, category 27, advanced](https://backtrader.readthedocs.io/en/latest/strategies-series/en/27-advanced.html)                                                                                                     |
| The underlying research   | The overfitting literature on how a wide search flatters a result, in the manner of Bailey, Borwein, Lopez de Prado and Zhu, [The Probability of Backtest Overfitting](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253) |
| How well it held up       | Weak: these five files show how to manage many strategies, not a trading idea, and the optimiser's three-cell grid is a textbook example of choosing a parameter on one year of data                                                |
| Also appears in           | nothing else in this collection; the lesson behind it, the price of a wide search, is the subject of [overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)                      |

## The idea in one paragraph

Writing one strategy is easy. Managing fifty of them is not. When you have fifty ideas, each with
three settings, and each setting needs ten years of prices, you stop needing cleverer signals and
start needing better tools. The five files in this folder demonstrate five such tools: running a
single backtest over every combination of settings and keeping the best; declaring a rule as a
signal line instead of writing out the trading logic; switching between two strategies at the
moment the run starts; managing several open positions inside one strategy; and feeding two price
series into one backtest so that one can generate the signal and the other receive the orders.
None of these is a trading idea. They are the workshop around the trading ideas.

## Why anyone believed it

There is no signal here to believe in, only a workflow and a warning. The workflow is sound: if
you have an idea, and the idea has a dial, you should test the dial rather than guess it, and a
tool that tests the dial for you is worth having. The warning is that the tool is dangerous. The
moment you keep the setting that scored best on a year of data, you have used that year's answer
to set the rule, and the setting is no longer an honest test of anything. Both the workflow and
the warning are in these five files, which is why they belong together. Someone on the other side
of the trade is not relevant here; the counterparty is your own past self, who chose the settings
after seeing the results.

## An everyday comparison

Think of a person trying to find the fastest route to work by trying twelve routes on one Monday
and picking the winner. The winner looks fastest, but Monday had unusual traffic, and the route
was chosen because it happened to be fastest that morning. A better test is to pick the route on
one week and then drive it on the next. That is the whole content of this folder: a tool that
tries all twelve routes, a warning that the winner of the first week is partly luck, and a second
run on fresh data. The signal and multi-series files are the equivalent of deciding what to carry
in the car rather than which road to take.

## The rules, step by step

This folder holds five files. Each demonstrates one capability, listed below.

| File                              | Capability it demonstrates                                                           |
| --------------------------------- | ------------------------------------------------------------------------------------ |
| `test_51_optimization.py`         | Runs a strategy over three settings, keeps the best by reward-to-risk, and reruns it |
| `test_44_signals_strategy.py`     | Declares a rule as one signal line, with the position size set by the signal's value |
| `test_48_strategy_selection.py`   | Chooses between two strategy classes at the moment the backtest starts               |
| `test_45_multitrades_strategy.py` | Keeps three independent positions open inside one strategy, each booked separately   |
| `test_59_multidata_strategy.py`   | Computes the signal on one price series and sends the orders to another              |

The shared mechanism is a workflow rather than a signal, and it has four steps.

1. State the strategy and mark the settings you intend to try. The optimiser file uses a moving
   average convergence divergence rule and varies the length of a simple moving average.
2. Enumerate the settings. The file tries the moving-average length at ten, eleven and twelve
   days, with the three convergence periods fixed, so there are three combinations.
3. Run every combination and record a score for each. The file records the annualised
   reward-to-risk ratio, called the Sharpe ratio, and keeps the combination with the highest one.
4. Rerun the winner on its own, with the full set of measurements, and assert the result. The file
   ends with 221 bars and ten trades, a final value of 100,150.06 from 100,000, and a Sharpe ratio
   of 0.4979.
5. For the signal file, define the signal as today's price minus a thirty-day average of the
   price. A positive signal means hold a long position; the size of the position is proportional
   to the size of the signal, so the further the price has run from its average, the larger the
   holding.
6. For the strategy-selection file, write both strategies with the same interface, and pass the
   one you want into the run as a parameter. This turns the choice of strategy itself into
   something you can vary and compare.
7. For the multiple-trades file, allow three trade identifiers to rotate, so that each new entry
   books its profit or loss separately and can be closed on its own.
8. For the multi-series file, attach two price series, compute the average on the first, and send
   the buy and sell orders to the second. The engine aligns the two series by their timestamps
   automatically.

## The maths, with every symbol named

The optimiser needs one score to rank the settings, and it uses the Sharpe ratio.

```text
Sharpe = ( average return per day - cash rate ) / standard deviation of daily returns
annualised Sharpe = Sharpe * square root of 252
```

- `average return per day` is the mean of the daily percentage returns of the account.
- `cash rate` is the return on holding cash, set to zero in the file.
- `standard deviation of daily returns` measures how much the daily returns jump around.
- `252` is roughly the number of trading days in a year, so multiplying by its square root
  converts a daily figure to an annual one.
- The ratio says how much return was earned for each unit of wobble; a bigger number means a
  smoother ride for the same reward.

The signal rule is one subtraction.

```text
signal_t = close_t - average( close over the last 30 days )
size_t   = max( signal_t, 0 ) * units_per_dollar
```

- `close_t` is today's closing price.
- The average is the plain mean of the last thirty closes.
- `signal_t` is positive when the price is above its average and negative when below.
- `size_t` is the number of units to hold: the strategy holds nothing when the signal is negative,
  and holds an amount proportional to the signal when it is positive, so the position is largest
  when the price is furthest above its average.

The convergence rule used by the optimiser is two averages and their difference.

```text
fast  = average of the last 12 closes, weighted toward recent days
slow  = average of the last 26 closes, weighted toward recent days
MACD  = fast - slow
signal_line = average of the last 9 values of MACD
```

- `fast` and `slow` are exponential averages, which give more weight to the most recent days than
  a plain average does.
- `MACD` is positive when the fast average is above the slow one, which happens when the price has
  been accelerating upward.
- `signal_line` is a smoothed version of `MACD`, and the rule buys when `MACD` crosses above it and
  sells when it crosses below.

## A worked example

The optimiser tries three moving-average lengths on one year of data. The scores below are
invented, but they are of the size the file reports.

| Moving-average length | Final value | Sharpe ratio |
| --------------------- | ----------- | ------------ |
| 10 days               | 100,150.06  | 0.4979       |
| 11 days               | 100,090.20  | 0.2103       |
| 12 days               | 99,980.40   | -0.1500      |

The rule keeps the highest Sharpe ratio, which is the ten-day setting, and reruns it alone. The
rerun processes 221 bars and makes ten trades, ending at 100,150.06, a gain of 0.15 percent over
the year. Now compare the three final values: 100,150.06, 100,090.20 and 99,980.40. The difference
between the best and the worst is 169.66 on a 100,000 account, about two tenths of one percent.
That tiny spread is the point. Three neighbouring settings produced almost identical results, so
the "best" one is not better in any meaningful sense; it is the winner of a very close and very
short race.

Now the signal rule, on invented daily prices with the thirty-day average given. The signal is the
close minus that average.

| Day | Close | Thirty-day average | Signal | Position       |
| --- | ----- | ------------------ | ------ | -------------- |
| 1   | 49.00 | 49.60              | -0.60  | none           |
| 2   | 49.20 | 49.55              | -0.35  | none           |
| 3   | 49.70 | 49.50              | +0.20  | long, small    |
| 4   | 50.10 | 49.48              | +0.62  | long, larger   |
| 5   | 50.40 | 49.46              | +0.94  | long, largest  |
| 6   | 50.20 | 49.45              | +0.75  | long, smaller  |
| 7   | 49.90 | 49.44              | +0.46  | long, smaller  |
| 8   | 49.50 | 49.44              | +0.06  | long, smallest |
| 9   | 49.30 | 49.43              | -0.13  | closed         |

The position grows as the price runs above its average and shrinks as it falls back, so the rule
holds the most when the price is furthest from the average, which is exactly when a reversal costs
the most. To keep the arithmetic simple, suppose a fixed size of twenty units were held instead,
bought at the day 4 open of 49.85 and sold on day 9 at the close of 49.30.

```text
Gross gain = (49.30 - 49.85) * 20 = -11.00
Commission = 0.0005 on each side, so (49.85 + 49.30) * 20 * 0.0005 = 0.9915
Net gain   = -11.00 - 0.99 = -11.99
```

The real rule would have held a larger position on the way up and a smaller one on the way down,
and the compendium's own result for this file is a small profit with a very poor reward-to-risk
ratio and a worst fall of about sixty-four percent. Neither example says the rule works; they show
how the two tools are used and why the sizing amplifies the risk.

## What the research actually found

These five files are not trading ideas, so there is no published record of them earning anything.
What the published record does cover is the danger of the optimiser, which is the one file here
that could mislead a beginner.

| Source                                                       | What it measured                                               | Result                                                                                                                        |
| ------------------------------------------------------------ | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Bailey, Borwein, Lopez de Prado and Zhu                      | The chance that the best of many tested strategies is a fluke  | The probability rises with the number of trials, and a backtest that looks strong can be the expected result of a wide search |
| The compendium's own optimiser run                           | A three-cell grid over one year of daily prices, 2006          | Best setting ten days, 221 bars, ten trades, final value 100,150.06, Sharpe ratio 0.4979                                      |
| The compendium's own signal run                              | Two years of daily prices, 50,000 starting cash                | Twenty-one trades, final value 50,607.58, Sharpe ratio -12.58, worst fall 64 percent                                          |
| The compendium's own multi-series run                        | Two daily price series, orders on one and signals on the other | A complete run in both engine modes, with 0.5 percent commission on the orders                                                |
| The compendium's own strategy-selection and multi-trade runs | Daily prices, 2005 to 2006                                     | Both run twice, once in each engine mode, and assert identical results, which is what these files are for                     |

Read together, the picture is this. The optimiser does exactly what it promises and finds the best
of three nearby settings; the harm is not in the tool but in the reading, because the winner of a
close race on one year is mostly luck. The signal rule is the clearest warning in the folder: it
produced a small profit with a reward-to-risk ratio of minus twelve and a worst fall of sixty-four
percent, which is a way of saying the profit was tiny and the ride was violent.

This is the honest heart of this group, and it applies to every file here. Every backtest in the
compendium asserts its final value, its reward-to-risk ratio and its worst fall against a
baseline. Passing that assertion proves the engine computes exactly what the file says. It does
not prove the strategy earns anything. Here the point is sharpest of all: the assertions in these
five files are mostly about running twice, once in each engine mode, and agreeing. They certify
the workshop, not any product built in it.

## How this project relates to it

The warning behind the optimiser is the subject of
[overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).
Its second section gives the numbers: of 207 published predictors, 183 had a t-statistic above 2.0
and 74 above 4.0, and mining 240 accounting variables produced 18,113 strategies of which about
thirty percent cleared the usual threshold purely by chance. The remedy it states is to log every
candidate evaluated, to carry the number of trials beside the result, and to freeze the data set
so the search cannot change between the run that found the edge and the run that reported it. A
three-cell grid is small, but the habit is the same, and the honest use of the optimiser is to
split the data, tune on the first part and test on the second.

The plain-language version of the same lesson is
[how a backtest lies](../../foundations/07_how-a-backtest-lies.md), which explains survivor bias,
look-ahead and the cost of a search for a reader who does not read code. The multi-series file
touches a separate theme, the cost of sending orders to one instrument while reading another; that
is the territory of
[market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md).

## Where it goes wrong

- Choosing on one year. The optimiser picks its winner on 2006 alone, so the "best" setting is
  partly the accident of that year. The real remedy, tuning on the first half and testing on the
  second, is not in the file.
- A close race has no winner. The three settings here end within two tenths of a percent of each
  other, so naming one the best is a statement about noise, not about skill.
- The selection metric is a choice too. Ranking by Sharpe favours low-volatility combinations with
  few trades, and a different metric such as the reward-to-risk against the worst fall, or a
  minimum number of trades, would crown a different setting.
- Declarative signals hide the risk. Because the position grows with the distance from the
  average, the rule holds the most exactly when a reversal hurts most, which is where the
  sixty-four percent worst fall comes from.
- Two engines, one answer. Running once in each engine mode and asserting identical results proves
  the engine is consistent; it says nothing about whether the strategy has any merit, and reading
  the assertion as a result is the mistake this folder invites.
- The tools scale the temptation. A three-cell grid is harmless; the same tool with three hundred
  cells, a metric chosen after the fact, and no held-back data is how a fitted result is born.

## Try it yourself

You need a spreadsheet and a public source of daily prices for one index.

1. Build columns for date, close, and a thirty-day moving average of the close.
2. Add a signal column: the close minus the average.
3. Add a size column: the signal when it is positive, and zero otherwise.
4. Work out two rules for the same days: one that holds a fixed number of units when the signal is
   positive, and one whose size equals the signal. Hold from the day after the signal turns
   positive to the day it turns negative.
5. Compute the profit of each rule, and subtract 0.05 percent for each side of every trade.
6. Separately, build a small table of two or three moving-average lengths, run each one on the
   same prices, and write down the final value for each.

What to notice: the variable-size rule holds a much larger position than the fixed rule at the
turning points, so its result is more extreme in both directions. Then look at how close the
different averages come to each other: the difference between the best and the worst is usually a
fraction of a percent, which is the arithmetic form of the warning about choosing a setting after
seeing the answer.

## Where this came from

- [Strategy compendium, category 27, advanced](https://backtrader.readthedocs.io/en/latest/strategies-series/en/27-advanced.html),
  the five files and the backtest numbers quoted above, including the optimiser's grid, the signal
  rule's losing risk profile and the multi-series run.
- Bailey, Borwein, Lopez de Prado and Zhu,
  [The Probability of Backtest Overfitting](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253),
  the paper behind the warning about wide searches.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief on the price of a wide search.
- [How a backtest lies](../../foundations/07_how-a-backtest-lies.md), the plain-language primer.
- [Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  the brief behind the cost of trading one instrument while reading another.

## Words used in this tutorial

- backtest: a simulation that applies a rule to past prices and reports what it would have done.
- commission: a fee charged by a broker for executing an order.
- grid search: trying every combination of a set of settings and keeping the one that scored best.
- look-ahead: using information that was not available at the time, which flatters a backtest.
- moving average: the average of the last few prices, used as a smoothed version of the price.
- out of sample: the part of the data not used to choose the rule, kept for an honest test.
- overfitting: fitting a rule so closely to the past that it captures noise rather than a pattern.
- Sharpe ratio: the average return divided by how much the return wobbles, a reward per unit of
  risk.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
