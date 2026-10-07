# Hyperopt: searching thousands of parameter sets for the best past result

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                      |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto pairs on twelve-hour candles, bought for a rise                                                                                                                                                                     |
| How often it trades       | Rarely; the recorded winning run made nine trades over the whole search sample                                                                                                                                             |
| What you need             | Python and a data file, and a computer that can run thousands of backtests                                                                                                                                                 |
| Where the rules come from | [GodStra.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/GodStra.py) and [GodStraHo.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/hyperopts/GodStraHo.py) |
| The underlying research   | None for the search itself; the measured warnings are in [this repository's brief on overfitting](../../../strategies/books2/28_overfitting_and_research_integrity.md)                                                     |
| How well it held up       | Weak: the search optimises the past, the recorded winner is nine trades, and the repository's own README calls the strategy files starting points rather than strategies to trade                                          |
| Also appears in           | [How a backtest lies](../../foundations/07_how-a-backtest-lies.md), which explains the failure this page is about                                                                                                          |

## The idea in one paragraph

A backtest replays a rule on prices that already happened and reports what it would have returned. A
hyperopt search automates that: it runs the same strategy thousands of times, each time with different
numbers, and keeps the version that scored best on the past. The GodStra files go further than
choosing thresholds. They let the search pick which indicators to compare, which comparison to use,
and the numbers in the comparison, so the search is choosing the shape of the rule and not only its
settings. The winning combination is then written back into the strategy file and used in trading. The
result looks excellent on the history it was chosen from, which is exactly what the search was asked
to produce.

## Why anyone believed it

The promise is that the history contains information about the future, so the numbers that worked
before should work again, and a search finds them faster than a person can by hand. That is a real
argument when the search is used to check whether a rule is sensitive to a setting: if a strategy is
profitable for every nearby value of a parameter, the parameter is not carrying the result.

The counterparty in the trade is the same as for any other rule: someone on the other side who is slow
to update or who has to trade for reasons of their own. The search does not care who that is, which is
the first sign of trouble. A search cannot tell you why a pattern existed, only that it did, and a
pattern with no reason tends to be a coincidence of the particular months in the sample.

## An everyday comparison

Imagine trying a hundred keys on a locked door. Ninety-nine of them are ordinary keys and fail, but
one turns and the door opens. It would be a mistake to conclude that the one key is special, because
if you are allowed to try a hundred keys, opening one of them by chance is not surprising; a hundred
attempts on a lock with a hundred tumblers will sometimes succeed. The search here is the hand trying
keys, the past is the lock, and the number that turned is the parameter set. To know whether the key
is real you must try it on a second door, which is the out-of-sample test the search does not run.

## The rules, step by step

1. Pick the strategy whose numbers are to be searched, and a backtest period and a list of pairs. The
   GodStra file asks for a pairlist filter that only includes pairs listed for at least thirty days,
   and suggests the smallest possible number of open trades.
2. Define what may vary. In `GodStraHo.py` the searchable items are grouped into "genes". Each gene
   holds two indicator names drawn from the roughly 88 indicators the `ta` library computes, one whole
   number between -1 and 101, one decimal number between -1.1 and 1.1, and one operation from a list
   of twelve. The operations are: disabled, greater than, less than, equal to, crossed above, crossed
   below, and the same three comparisons against the whole number or the decimal number.
3. Define the score. The file header records a command that uses the daily Sharpe loss, and the result
   table the file prints is headed out of 500 epochs, where an epoch is one candidate tried. A loss is
   a single number that summarises a backtest; the search makes it as small as it can.
4. Run the search. At each epoch it proposes a combination, builds the entry and exit rules from it,
   backtests those rules on the chosen history, and records the score.
5. Keep the best combination, and write its values into the strategy file as `buy_params` and
   `sell_params`. In `GodStra.py` these are already written in, so the file a reader sees is the
   output of a past search, not its input.
6. Trade with the winning numbers.

The winning numbers currently in `GodStra.py` are: timeframe `12h`, stop loss `-0.34549`, minimal
return `{"0": 0.3556, "4818": 0.21275, "6395": 0.09024, "22372": 0}`, trailing stop on with a positive
step of `0.22673` and an offset of `0.2684`, the buy gene `trend_ichimoku_base` compared with the
decimal `0.06295` using "less than", and the sell gene `volume_mfi` compared with the decimal `0.8779`
using "equal to". The whole-number part of each gene, 42 on the buy side and 98 on the sell side, is
not used by the operations the search chose. The file's own comment records that the best score at
epoch 5 of 500 made nine trades, eight wins and one loss, with an average profit of 21.83 percent and
a total of 1,060.11 units of currency, or 196.50 percent.

## The maths, with every symbol named

A search is a loop that finds the smallest value of a score.

```text
best = the combination c for which score(c) is smallest, over all c tried
```

- `c` is one candidate: a complete set of values for every searchable parameter.
- `score(c)` is the loss from backtesting the strategy with candidate `c`, one number per backtest.
- `best` is the candidate kept at the end. The search reports one winner and, unless it is asked, not
  the number of candidates it rejected.

The reason a winner looks good is the arithmetic of the best of many draws. If the rules being tried
have no real edge, their scores scatter around some average by chance, and the best of `N` tries sits
roughly this far above the average:

```text
expected best = average + sqrt(2 * ln(N)) * spread, approximately
```

- `N` is the number of candidates tried.
- `ln` is the natural logarithm, the function that answers "how many times must one multiply 2.718 by
  itself to get this number".
- `spread` is how much a single backtest result varies by chance, for example 20 percent a year.
- `sqrt` is the square root.

The term `sqrt(2 * ln(N))` grows slowly but without limit: about 2.15 standard deviations for ten
candidates, 3.03 for a hundred, 3.72 for a thousand and 4.29 for ten thousand. Put into money, if one
random strategy's yearly result has a spread of 20 percent, then the best of a thousand random
strategies sits near `3.72 * 20 = 74` percent above zero before any cost, with no edge anywhere in the
thousand.

## A worked example

Take ten candidate versions of one rule, all tested on the same past year and then watched through the
following year without changing anything. The numbers are made up, but the pattern they show is the
one the search produces.

| Candidate | Setting         | Past-year return | Next-year return |
| --------- | --------------- | ---------------- | ---------------- |
| 1         | index level 10  | -8.0%            | +3.0%            |
| 2         | index level 20  | -3.0%            | -1.0%            |
| 3         | index level 30  | +2.0%            | +1.0%            |
| 4         | index level 40  | +1.0%            | +5.0%            |
| 5         | index level 50  | +5.0%            | -2.0%            |
| 6         | index level 60  | +9.0%            | -4.0%            |
| 7         | index level 70  | +14.0%           | -1.0%            |
| 8         | index level 80  | +6.0%            | +2.0%            |
| 9         | index level 90  | -2.0%            | +4.0%            |
| 10        | index level 100 | -11.0%           | +4.0%            |

The average past-year return of the ten is `13.0 / 10 = +1.3 percent`, and the search keeps candidate
7, because it returned 14.0 percent. In the next year that same rule returns minus 1.0 percent, while
the ten rules average `11.0 / 10 = +1.1 percent`. The rule that did worst in the past, candidate 10 at
minus 11.0 percent, returned plus 4.0 percent in the next year. The search selected the one candidate
that went on to do worse than the group.

Trading costs make the comparison starker. If the rule trades about four times a year and each round
trip costs 0.2 percent, the cost is `4 * 0.2 = 0.8 percent` a year, so the selected rule's minus 1.0
percent becomes about minus 1.8 percent, against an average of plus 1.1 percent for the ten.

The same arithmetic drives the published experiment below: mining enough variations produces many
apparently strong results, and the strongest of them is the least likely to be real.

## What the research actually found

The measured evidence does not come from this repository's strategies, and that is the point: nothing
in the Freqtrade strategies repository publishes a search protocol or a held-out test. What exists is
research on how often a selected result survives, collected in [this repository's brief on
overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).

- Mining 240 accounting variables in every simple combination produced 18,113 strategies. Under a
  strict significance bar, 30.17 percent of them cleared it, against about 4.55 percent expected if
  there were no effect at all; under a stricter bar, 8.40 percent cleared against 0.0063 percent
  expected (`2209.13623v3`). Trying many things manufactures winners.
- The same review finds that the correction for publication bias is modest, between 10 and 15 percent
  of the in-sample return, and that the larger practical risk is the analyst's own choice of data and
  period rather than a broken threshold (`2209.13623v3`). A decayed strategy is evidence that the
  opportunity changed, not proof that the original finding was false.
- A single erroneous data row can erase a result. In one betting study, reported returns of 17.29 and
  28.82 percent turned into losses of 7.36 and 6.31 percent once one bad price row was corrected
  (`2306.01740v4`). Backtest data is as much of a risk as the search method.
- The same correction study found that after cleaning, one strategy survived at 12.44 percent and then
  earned nothing over three further years, with its predictors no longer significant (`2306.01740v4`).

Against that, the outcome the GodStra file itself prints is instructive. Its comment records the best
run at epoch 5 of 500 as nine trades, eight wins and one loss, with an average profit of 21.83 percent
and a total of 196.50 percent. Nine trades is a very small sample: one trade changing its outcome
moves the average profit by several percentage points, and the 196.50 percent total arrived over nine
decisions rather than over a broad set of independent bets.

## How this project relates to it

The search problem is treated at length in
[the overfitting and research integrity brief](../../../strategies/books2/28_overfitting_and_research_integrity.md).
Its section on what a protocol must record is the direct answer to this page: it lists the dataset
identity, the full candidate list and trial count, the significance policy, the in-sample and
out-of-sample dates, the cleaning rules and the metric used to rank candidates, none of which the
GodStra search states. [How a backtest lies](../../foundations/07_how-a-backtest-lies.md) names the
same failure modes for a reader who has not met them. The related question of what predictability
survives once a search is honestly costed is the subject of [the predictability
brief](../../../strategies/books2/08_predictability_and_trading_strategies.md).

## Where it goes wrong

- Overfitting is the method, not a side effect. The search's only instruction is to make the past
  result good, so it will use any accidental pattern that the sample contains, including ones that
  exist in no other period.
- The trial count is missing. A result reported without the number of candidates tried cannot be
  judged; the accepted significance bar is a policy on that count, and the file records only the few
  winning epochs, not the hundreds that failed.
- The search chooses structure, not only numbers. Because it may pick which of the roughly 88
  indicators to use and how to compare them, the space of possible rules is enormous, which makes the
  best-of-many effect far larger than a threshold search.
- There is no held-out period. Nothing in the files reserves a stretch of history that the search is
  forbidden to see, so every reported number comes from the same data that chose the rule.
- Trade counts are tiny. Nine trades with one loss means a single trade carries several percent of
  the average, so the result is closer to a coin toss than to a measurement.
- The data can leak. One of the strategy files in the same repository notes in a comment that a
  rolling average over the whole data frame would contain look-ahead if used in a backtest. A search
  over thousands of candidates will happily exploit such a leak and report the result as skill.
- Costs and execution are assumed away. The search scores fills at prices the backtest invents, while
  a twelve-hour candle hides everything that happened inside the twelve hours.

## Try it yourself

This exercise needs only a spreadsheet and its random number function; no market data and no code.

1. Make twenty columns, each with 120 rows standing for ten years of monthly returns. Fill each
   column with a random function such as `=NORM.INV(RAND(), 0.005, 0.05)`, which draws a monthly
   return averaging 0.5 percent with a spread of 5 percent.
2. In each column, compound the first sixty rows into a five-year result.
3. Find the highest five-year result and note which column won. That column is your "searched"
   strategy.
4. Compound the remaining sixty rows of that winning column, the five years the search did not see.
5. Also compound the remaining sixty rows of every column and average them.

What to notice: the winning column is usually far above the average in the first five years, by the
best-of-many arithmetic, and close to the average in the next five, because nothing about a random
column makes it stay a winner. Repeat the sheet a few times and watch the winner change. That is what
a hyperopt search does with real prices, with the difference that real prices carry a small genuine
signal which the random columns do not, and which is exactly what the search makes hard to judge.

## Where this came from

- [GodStra.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/GodStra.py),
  the strategy the winning parameters were written into, including the recorded search results and
  the minimal return ladder, stop loss and trailing stop quoted above.
- [GodStraHo.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/hyperopts/GodStraHo.py),
  the search definition: the gene list, the twelve operations and the indicator list.
- [The Freqtrade strategies README](https://github.com/freqtrade/freqtrade-strategies/blob/main/README.md),
  which states that the files are starting points and that results depend on the pairs, timeframe and
  period used.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  the source, via `2209.13623v3` and `2306.01740v4`, of the mining and data-error figures above.

## Words used in this tutorial

- backtest: a replay of a strategy's rules over prices that already happened.
- epoch: one candidate tried by the search, one backtest.
- hyperopt: the Freqtrade tool that searches parameter values by scoring many backtests.
- in-sample: the data a rule was chosen on, as opposed to untouched data used to test it.
- loss: a single number that summarises a backtest, made as small as possible by the search.
- out-of-sample: data kept aside and used only once, to test a rule the search did not choose it with.
- overfitting: fitting rules so closely to past data that they capture accidents and fail on new data.
- parameter: one of the numbers a strategy uses, such as an indicator threshold or a stop level.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
