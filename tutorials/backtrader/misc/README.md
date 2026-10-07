# The misc drawer: exhaustion counts, buying dips and the tests that hold the rest up

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | American shares, mostly Oracle daily bars, plus a test of Yahoo daily bars; the framework files in the same folder trade the same daily prices                                                                       |
| How often it trades       | From almost never to a few times a month, depending on the rule; the pinkfish rule trades once every few weeks, the counting rule almost never                                                                       |
| What you need             | A spreadsheet and a table of daily open, high, low and close prices                                                                                                                                                  |
| Where the rules come from | [Strategy compendium, category 09, misc](https://backtrader.readthedocs.io/en/latest/strategies-series/en/09-misc.html)                                                                                              |
| The underlying research   | Tom DeMark's TD Sequential, as specified by the compendium and the [public TD-seq implementation](https://github.com/mk99999/TD-seq) it follows; the challenge rule and the framework checks have no published paper |
| How well it held up       | Weak: the counting rule ends almost exactly flat on five years of Oracle, the challenge rule loses about a quarter of a percent a year, and the rest of the folder exists to verify the engine rather than to trade  |
| Also appears in           | nothing else in this collection                                                                                                                                                                                      |

## The idea in one paragraph

This category is the drawer where the rules that do not fit a theme are kept. Three trade: a
counting system that waits for a run of falling closes and then tries to buy the end of the fall;
a challenge that buys whenever the price makes a new twenty-day high and sells two days later
whatever the price; and three ways of buying a dip. The rest of the folder does not trade at all.
It exists to check that the engine's cost model, its commission schemes and its measures of
return and risk compute what they claim, because every other backtest in the compendium stands on
them.

## Why anyone believed it

The counting system comes from Tom DeMark, who argued that a market falls in a countable rhythm.
After nine closes below the close four days earlier, sellers are said to be running out of
ammunition, and after a further countdown of thirteen the fall is said to be exhausted. There is
no counterparty story beyond exhaustion: the last seller is done, and anyone who has to buy meets
a thin supply. The challenge rule rests on the oldest momentum idea there is, that strength begets
strength, with the twist that the exit is fixed by the calendar so that no judgement enters the
sell. The dip rules rest on the opposite idea, that a fall is a discount and the price tends to
recover, a story as old as markets themselves.

## An everyday comparison

Think of a shop that cuts the price of bread near closing time. Every few minutes the price drops
again, and a regular shopper learns to wait. Sometimes the price stops falling and the last loaves
go at the low, and the shopper who waited buys cheap. Sometimes the shop sells out and the waiter
gets nothing, or the price was low because the bread is stale and it stays stale. The counting
rule is the shopper who waits for a fixed number of cuts before buying; the dip rule is the
shopper who buys on every cut; the challenge rule is the shopper who buys on every rise and sells
two days later no matter what the bread is worth then.

## The rules, step by step

This category holds twenty-eight strategies. Roughly ten trade and the rest check the engine. The
ones a reader would meet are listed below.

| File                                | What it does                                                                   |
| ----------------------------------- | ------------------------------------------------------------------------------ |
| `test_65_td_sequential_strategy.py` | Counts a nine-bar setup and a thirteen-bar countdown, then buys the exhaustion |
| `test_46_pinkfish_strategy.py`      | Buys a new twenty-day high, holds exactly two bars, sells unconditionally      |
| `test_110_buy_the_dip_strategy.py`  | Buys after a fall of a chosen size, one of three dip parameterizations         |
| `test_39_btfd_strategy.py`          | Buy the dip, stated plainly, on daily standard prices                          |
| `test_79_buy_dip_strategy.py`       | The same idea with a different entry cadence                                   |
| `test_71_double_sevens_strategy.py` | Fades seven consecutive bars in the same direction                             |
| `test_76_heikin_ashi_strategy.py`   | Smoothed candles used as a trend filter                                        |
| `test_82_alligator_strategy.py`     | Three moving averages used to decide whether a trend exists                    |
| `test_47_slippage_strategy.py`      | A moving-average crossover run to test the engine's slippage model             |
| `test_49_calmar_analyzer.py`        | No trading idea; asserts the engine's reward-to-risk and return numbers        |

The shared mechanism for the trading rules is simple: a fixed condition on daily bars, a fixed
hold or a fixed exit, and a fixed trade size. The framework files share a different job: run a
known strategy and assert the analysis numbers to many decimal places.

1. The counting rule, step one, the setup. Look at each day's close and compare it with the close
   four days earlier. When the current close is below the close four days back, and the previous
   day was not, a down-setup begins. Each further day whose close is below the close four days
   back adds one to the count. When the count reaches nine, the setup is complete.
2. The counting rule, step two, the countdown. After the setup, start a second count. Add one each
   time the close is below the close two days earlier. When the countdown reaches thirteen, and
   the price at countdown bar thirteen is at or below the low recorded at countdown bar eight, the
   ideal buy point is confirmed, and the rule buys.
3. The counting rule, exits. The compendium's version closes the position on the opposite
   exhaustion signal. There is no stop and no target in the counting logic itself.
4. The counting rule, cancellations. Before the setup completes, if the price rises above the high
   of the setup range, the count is thrown away and starts again. Several such cancellation
   clauses are implemented, and they change how often the rule fires.
5. The challenge rule. If the current day's high equals or exceeds the highest high of the last
   twenty days, buy. Hold for exactly two bars. Sell at the close of the second bar, whatever the
   price. There is no stop and no target; the exit is the calendar alone.
6. The dip rules. Measure the fall from a recent high, or the number of consecutive falling days,
   and buy when the fall exceeds a chosen size. Exit on a recovery target, a fixed number of days,
   or an opposite condition, depending on the file.
7. Every one of these rules reviews its position once a day and uses a fixed trade size, so the
   same signal is the same bet each time.

## The maths, with every symbol named

The counting rule compares each close with the close a fixed number of days back.

```text
setup_day_t  = 1 if close_t < close_(t-4) else 0
countdown_day_t = 1 if close_t < close_(t-2) else 0
```

- `close_t` is today's closing price and `close_(t-4)` is the close four days earlier.
- A setup counts up while `setup_day_t` stays one, and completes at nine.
- A countdown counts up while `countdown_day_t` stays one, and completes at thirteen.

The challenge rule is a high-water mark and a calendar.

```text
highest_t = max(high_(t-19) ... high_t)
buy       if  high_t >= highest_t
sell      if  bars_held >= 2
```

- `high_t` is today's highest price.
- `highest_t` is the largest high of the last twenty days, today included.
- `bars_held` counts the days since the entry, so the sale happens on the second day after buying.

The dip rules need a fall from a reference level.

```text
fall_t = close_t / highest_(t-n) - 1
buy    if  fall_t < -d
```

- `highest_(t-n)` is the highest close of the last `n` days, a recent peak.
- `d` is the chosen dip size as a decimal, so 0.05 means a five percent fall from the peak.
- The rule buys when the fall is deeper than `d`.

The cost of any round trip, added at the end, is the same everywhere in this collection:

```text
cost = 2 * c
```

- `c` is the cost of one side as a fraction of the amount traded, covering the gap between the
  buying and selling price and any commission. The compendium's slippage test uses one percent,
  which is deliberately harsh; a realistic figure for a large American share is 0.0002 to 0.0005,
  that is two to five hundredths of a percent.
- The factor 2 counts the buy and the sell, so a round trip costs twice `c`.

## A worked example

First, the challenge rule, on invented daily prices for a share trading around thirty dollars. The
position is one hundred shares, held from the day after the signal.

| Day | High  | Twenty-day highest high | Buy? | Held since | Sell? | Price used |
| --- | ----- | ----------------------- | ---- | ---------- | ----- | ---------- |
| 1   | 29.40 | 30.10                   | no   |            |       |            |
| 2   | 29.80 | 30.10                   | no   |            |       |            |
| 3   | 30.20 | 30.20                   | yes  |            |       |            |
| 4   | 30.00 | 30.20                   |      | 1          | no    |            |
| 5   | 30.60 | 30.60                   |      | 2          | yes   | 30.60      |

The rule buys at the next day's open after the signal, which is day 4 at 30.10, and sells at the
close two bars later, day 5 at 30.60.

```text
Gross gain = (30.60 - 30.10) * 100 = 50.00
Commission = 0.001 on each side, so 30.10 * 100 * 0.001 + 30.60 * 100 * 0.001
           = 3.01 + 3.06 = 6.07
Net gain   = 50.00 - 6.07 = 43.93, on a 50,000 account that is 0.088 percent
```

Second, the counting rule, on a run of falling closes. The setup compares each close with the
close four days earlier.

| Day | Close | Close four days earlier | Below? | Setup count |
| --- | ----- | ----------------------- | ------ | ----------- |
| 1   | 100.0 |                         |        |             |
| 2   | 99.0  |                         |        |             |
| 3   | 98.0  |                         |        |             |
| 4   | 97.0  |                         |        |             |
| 5   | 96.0  | 100.0                   | yes    | 1           |
| 6   | 95.0  | 99.0                    | yes    | 2           |
| 7   | 94.0  | 98.0                    | yes    | 3           |
| 8   | 93.0  | 97.0                    | yes    | 4           |
| 9   | 92.0  | 96.0                    | yes    | 5           |
| 10  | 91.0  | 95.0                    | yes    | 6           |

The count reaches nine on day 13 if the fall continues at one dollar a day, and only then does the
countdown begin. Nine falling closes in a row is not a buy in itself, and that is the part most
retellings get wrong. The example also shows the cost of waiting: the rule can spend weeks
counting and never trade, which is exactly what happened in the compendium's five-year test. The
arithmetic proves nothing about whether exhaustion predicts anything; it only shows how to apply
the rules.

## What the research actually found

| Source                                  | What it measured                                                               | Result                                                                                                                                               |
| --------------------------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| The compendium, counting rule           | Five years of Oracle daily bars, 100,000 starting cash, 0.1 percent commission | After 1,257 bars the account stands at 100,002.91, essentially flat, with a Sharpe ratio of 0.0229                                                   |
| The compendium, challenge rule          | Yahoo daily bars, 2005 to 2006, 50,000 starting cash, one hundred shares       | After 484 bars the account is worth 49,739.00, a Sharpe ratio of -2.52 and an annual return of about -0.26 percent                                   |
| The compendium, framework checks        | The same daily prices through known strategies                                 | The slippage, commission, writer, Calmar, reward-to-risk and Sharpe tests assert numbers to many decimal places and agree across both engine modes   |
| DeMark, as summarised by the compendium | The counting system's specification                                            | Not a return study. The value recorded here is that the specification is complete enough to implement exactly, which technical indicators rarely are |

Read together, the picture is this. The counting rule is a fossil of a real phenomenon, the tendency
of a long fall to slow, but implemented literally it produced almost exactly zero dollars over five
years on one share, and zero is itself an achievement only in the sense that the engine computed
it precisely. The challenge rule lost money, which is what a momentum entry with a random holding
period does in a choppy market.

This is the honest heart of this group, and it applies to every file here. Every backtest in the
compendium asserts its final value, its reward-to-risk ratio and its worst fall against a baseline.
Passing that assertion proves the engine computes exactly what the file says. It does not prove the
strategy earns anything. The framework files in this folder are the clearest statement of that
distinction: their assertions are about the engine, never about a market.

## How this project relates to it

The framework files here are about the cost of trading and the measurement of results, which this
repository studies directly. The market-impact brief,
[market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
reports that execution prices deviate from the screen price in a systematic, measurable way, and
that a half percent deviation is the order of magnitude a strict backtest should carry. The
slippage test in this folder exists for exactly that reason: if the slippage model is wrong, every
high-frequency backtest in the compendium is self-deception.

The measurement side is covered by
[risk measures and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md), which
notes that the worst fall is model-free but depends on the order of returns, so it must never be
compared across resampled or reordered paths. The Calmar and reward-to-risk baselines in this
folder are the numerical form of that caution. The plain-language companion,
[costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md), explains why a rule that
looks flat before costs is a losing rule after them.

## Where it goes wrong

- A long run of falling closes can continue. Exhaustion is a story about who is left to sell, and
  in a falling market there is often one more seller. The counting rule's near-zero result is the
  measured version of that risk.
- The cancellations were chosen after the fact. The compendium's file implements several
  cancellation and recycling clauses, and each one changes how often the rule fires. Choosing the
  set that fits the sample is a search, not a specification.
- A fixed holding period is not a strategy. The challenge rule's two bars were chosen by its
  author, and changing them changes the result entirely; nothing in the rule says two is right.
- One share, one period. Oracle over five years and Yahoo over two are the whole sample for the
  trading rules here, and neither is evidence about any other share or any other decade.
- The framework numbers are not market results. A slippage or Calmar assertion says the engine is
  consistent, not that any strategy makes money; reading those assertions as performance is the
  most common mistake a newcomer makes with this repository.
- Commission schemes differ across the files. A rule that is flat at one tenth of a percent per
  side can be clearly negative at half a percent, so the cost line has to be stated before any
  result means anything.

## Try it yourself

You need a spreadsheet and a public source of daily prices for one share or fund.

1. Make columns: date, open, high, low, close.
2. Add a column for the highest high of the last twenty days, using a rolling maximum.
3. Add a column that says buy when today's high equals or exceeds that maximum.
4. Add a column that sells two rows after a buy row, by counting rows since the last buy.
5. In a second block, add a column that flags each day whose close is below the close four rows
   earlier, and a running count that starts over whenever the flag turns off.
6. Find the rows where that count reaches nine.
7. For both blocks, compute the return from the buy price to the sell price and subtract 0.1
   percent for each side.

What to notice: the twenty-day-high rule trades often and the buying price is always near a
recent extreme, so the two-day holding period sells into whatever comes next. The counting rule
trades almost never, and when the count reaches nine the price is usually still falling. Neither
outcome is a verdict; both show that the trigger and the exit are separate choices, and that a
good-looking trigger with a fixed exit can still lose.

## Where this came from

- [Strategy compendium, category 09, misc](https://backtrader.readthedocs.io/en/latest/strategies-series/en/09-misc.html),
  the rules and the backtest numbers quoted above, including the flat counting-rule result, the
  losing challenge rule and the list of framework checks.
- The [public TD-seq implementation](https://github.com/mk99999/TD-seq) the compendium's counting
  file follows, which states the two-stage setup-and-countdown structure.
- The original backtrader sample named after the pinkfish challenge, which the compendium's
  challenge rule ports.
- [Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  this repository's brief on why the fill price differs from the screen price.
- [Risk measures and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md), the
  brief behind the measurement checks in this folder.
- [Costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md), the plain-language primer.

## Words used in this tutorial

- close: the price at which a period of trading ends.
- commission: a fee charged by a broker for executing an order.
- countdown: the second stage of the counting rule, a run of thirteen closes below the close two
  days earlier.
- drawdown: the fall from a peak to a later low, measured as a percentage of the peak.
- exhaustion: the idea that a long run of falling prices ends because there are no sellers left.
- setup: the first stage of the counting rule, a run of nine closes below the close four days
  earlier.
- slippage: the difference between the price you expected and the price you actually got.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
