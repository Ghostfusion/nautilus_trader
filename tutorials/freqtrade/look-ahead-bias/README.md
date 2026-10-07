# Look-ahead bias: four strategies that read a price before it happened

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto coins on a crypto exchange, chosen from a fixed list of pairs and tuned on that list                                                                                                                                                 |
| How often it trades       | On four-hour candles in three of the four files, and on thirty-minute candles in the fourth                                                                                                                                                 |
| What you need             | Nothing but this page to follow the arithmetic; Python and a data file to run the files themselves                                                                                                                                          |
| Where the rules come from | [The lookahead bias folder](https://github.com/freqtrade/freqtrade-strategies/tree/main/user_data/strategies/lookahead_bias) in the Freqtrade community strategy repository, whose own readme states that the files are deliberately broken |
| The underlying research   | None; these are teaching examples written to demonstrate a mistake, not strategies anyone claimed would work                                                                                                                                |
| How well it held up       | Weak: nothing here can hold up, because every one of the four files scales its indicators using prices that had not happened yet, which is the mistake the folder exists to show                                                            |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                             |

## The idea in one paragraph

A backtest is a dry run of a strategy over old prices: the program walks through the past candle by
candle, makes the same decisions a trader would have made, and adds up the result. Look-ahead bias is
what happens when the program quietly uses a number it could not have known at the time. The four
files in this folder all make the same version of that mistake. Each one puts its indicators on a
scale from 0 to 1, and to build the scale it looks up the smallest and largest value of the
indicator over the entire history, including months and years that came after the candle being
judged. The backtest therefore knows the shape of the future before it decides. The result is a
record of trades that were never available, and a profit figure that no account could have earned.

## Why anyone believed it

Nobody in this folder believed it. The folder's readme says in its first line that the files have a
look-ahead bias and that they are there as practice, so that a reader can try to spot the mistake
before opening the answer. The four files are otherwise ordinary members of the repository: each was
tuned by a parameter search over a fixed list of pairs and a period, each carries a profit ladder and
a stop loss, and each prints a flattering result in a comment at the top.

The mistake is worth a page of its own because it is easy to make and hard to see. The code that
commits it is one short line, the same line in all four files, and it reads like a sensible piece of
tidying: put the numbers on a common scale before comparing them with a threshold. The arithmetic is
not the defect; the range of data it is applied to is.

## An everyday comparison

Imagine a school that marks an exam by a different rule: before marking, the teacher works out the
highest and lowest score anyone in the class will get, and rescales each pupil's paper against that
range. Then the teacher announces that this pupil scored 0.95 and that one 0.40, and the drama club
is picked from the pupils above 0.90. It is a perfectly consistent marking scheme, and it is also
useless, because the scale was decided by answers that had not been written yet and every pupil's
rank moved the moment the last paper was marked. A backtest that scales its indicators over the whole
history is doing the teacher's rescaling before the exam is over.

## The rules, step by step

1. A fixed list of pairs is supplied in the bot's configuration, and the pair list never changes: no
   pair is added or removed as the backtest moves through time. The list is the set of pairs that
   existed and were liquid when the author was working.
2. Each file computes one or more technical indicators from the candles, such as a moving average or
   a candle-pattern reading.
3. Each file then puts every indicator on a 0 to 1 scale, using the smallest and the largest value
   that indicator takes across the whole file of prices. This step is the mistake, and it is
   described in the maths section.
4. Each file then compares the scaled readings with thresholds that were chosen by a parameter
   search, called hyperopt, which tries many combinations and keeps the one that scored best on the
   period tested.
5. Each file then applies a fixed set of exits: a profit ladder that falls with the age of the trade,
   a stop loss, and in one file a trailing stop as well.

What each of the four files does, in one or two sentences each:

1. `Zeus.py` computes the base line of the ichimoku cloud and the difference line of the Know Sure
   Thing indicator on four-hour candles, rescales both against the whole history, buys when the
   rescaled base line is below 0.0128, and sells when the rescaled difference line is near 0.9455, a
   value it almost never takes.
2. `wtc.py` computes a pair of wave-trend lines and a stochastic reading on thirty-minute candles,
   rescales the whole block of numeric columns with the smallest and largest values found anywhere in
   the file, and buys when the wave-trend lines cross while the rescaled readings sit inside bands
   chosen by the search.
3. `GodStraNew.py` is a search engine more than a strategy: it takes hundreds of indicators, pairs
   them with twenty comparison operators and eight time periods, joins three conditions for the buy
   and three for the sell, and rescales every indicator against the whole history before comparing
   anything.
4. `DevilStra.py` does the same rescaling and then splits the fixed pair list among a list of
   "spells" copied out of a GodStraNew search, one spell per pair, each spell being three conditions
   joined together.

The settings each file prints, for reference:

| File            | Candles       | Profit ladder, minutes and target                                                           | Stop loss    | Trailing stop |
| --------------- | ------------- | ------------------------------------------------------------------------------------------- | ------------ | ------------- |
| `Zeus.py`       | Four-hour     | 0 minutes 56.4 percent, 567 minutes 27.3 percent, 2814 minutes 12 percent, 7675 minutes 0   | 25.6 percent | no            |
| `wtc.py`        | Thirty-minute | 0 minutes 30.9 percent, 569 minutes 16.7 percent, 3211 minutes 6.5 percent, 7617 minutes 0  | 12.8 percent | no            |
| `GodStraNew.py` | Four-hour     | 0 minutes 59.8 percent, 644 minutes 16.6 percent, 3269 minutes 11.5 percent, 7289 minutes 0 | 12.8 percent | no            |
| `DevilStra.py`  | Four-hour     | 0 minutes 57.4 percent, 1757 minutes 15.8 percent, 3804 minutes 8.9 percent, 6585 minutes 0 | 28 percent   | no            |

## The maths, with every symbol named

The rescaling, which is the mistake in all four files. It is written in the code as a single line,
`(value - value.min()) / (value.max() - value.min())`.

```text
lowest  = the smallest value the indicator takes anywhere in the data
highest = the largest value the indicator takes anywhere in the data
scaled_t = (value_t - lowest) / (highest - lowest)
```

- `value_t` is the indicator reading at candle `t`, the one being judged.
- `lowest` and `highest` are the smallest and largest readings over the entire file of prices,
  including candles that come after `t`.
- `scaled_t` is the reading expressed as a position between those two extremes: 0 means it was the
  lowest reading in the whole file, 1 means it was the highest.
- The defect is in the two words "anywhere in the data". The programme that decides at candle `t` may
  only use readings up to candle `t`.

The honest version uses the same formula on a smaller set:

```text
lowest_so_far  = the smallest value up to and including candle t
highest_so_far = the largest value up to and including candle t
scaled_t = (value_t - lowest_so_far) / (highest_so_far - lowest_so_far)
```

- `lowest_so_far` and `highest_so_far` are computed with the candles a trader has actually seen.
- The two versions give different answers for every candle except the first and the last, and the
  difference changes which candles pass a threshold.

The second mistake, which is the plainest, is deciding with one price and trading at the same price:

```text
gain on an up day = close_(t+1) / close_t - 1
```

- `close_t` is the closing price of day `t`, the final traded price of that day, printed after the
  day's trading has finished.
- `close_(t+1)` is the closing price of the next day.
- A rule that buys at `close_t` because it knows `close_(t+1)` is higher cannot be executed, because
  `close_t` is not known while it is still possible to trade at it. The best a real trader can do is
  send an order after the close and be filled at the next available price, which is a different
  number.

## A worked example

Two examples, both small enough to check by hand. The first is the rescaling mistake, since that is
what the four files actually do; the second is the closing-price mistake, since that is the clearest
way to see why a price cannot be both the decision and the trade.

The rescaling. Six daily readings of an indicator, and a rule that buys whenever the scaled reading
is above 0.9.

| Day | Reading | Scaled against the whole file | Scaled against the days seen so far | Whole-file rule fires | Honest rule fires |
| --- | ------- | ----------------------------- | ----------------------------------- | --------------------- | ----------------- |
| 1   | 10      | 0.000                         | 0.000                               | no                    | no                |
| 2   | 20      | 0.333                         | 1.000                               | no                    | yes               |
| 3   | 30      | 0.667                         | 1.000                               | no                    | yes               |
| 4   | 25      | 0.500                         | 0.750                               | no                    | no                |
| 5   | 15      | 0.167                         | 0.250                               | no                    | no                |
| 6   | 40      | 1.000                         | 1.000                               | yes                   | yes               |

The whole-file column uses the smallest reading of all six days, 10, and the largest, 40, so the
range is 30 and day 2 becomes (20 - 10) / 30, which is 0.333. The honest column on day 2 knows only
days 1 and 2, so the range is 10 and day 2 becomes (20 - 10) / 10, which is 1.000. The two rules
therefore buy on completely different days: the file's version buys only on day 6, and the version a
trader could have run buys on days 2, 3 and 6. Every trade in the file's backtest was chosen under
the first column, and the first column was built with day 6's high of 40. With three conditions joined
together, as in the two search files, a wrong scale on any one of them changes the whole trade list.

The closing price. Six daily closes, and a rule that buys at the close of any day whose close turns
out to be lower than the next day's close, selling the next day at that day's close.

| Day | Close | Up or down versus the next close | Trade                                           |
| --- | ----- | -------------------------------- | ----------------------------------------------- |
| 1   | 100.0 | up                               | buy at 100.0, sell at 101.0, gain +1.00 percent |
| 2   | 101.0 | up                               | buy at 101.0, sell at 103.0, gain +1.98 percent |
| 3   | 103.0 | down                             | no trade                                        |
| 4   | 102.0 | up                               | buy at 102.0, sell at 104.0, gain +1.96 percent |
| 5   | 104.0 | down                             | no trade                                        |
| 6   | 103.0 |                                  | no trade                                        |

Three trades, all of them winners, because the rule knew in advance which days were the up days:

```text
Compounded gain = 1.01 * (103.0 / 101.0) * (104.0 / 102.0) - 1
                = 1.01 * 1.019802 * 1.019608 - 1
                = 1.0502 - 1
                = +5.02 percent
Cost            = 3 trades * 2 sides * 0.075 percent = 0.45 percent
Net             = about +4.57 percent over five days
```

For comparison, simply holding the coin from the first close to the last earned 103.0 / 100.0 - 1,
which is +3.00 percent, with one purchase and one sale instead of six. The extra 1.6 points is
entirely the value of knowing the future, and a trader standing at the close of day 1 had neither
that knowledge nor any way to trade at day 1's close after discovering it.

## What the research actually found

There is no measurement of these four files, because each is a demonstration of a mistake rather than
a claim about markets. What can be reported is how much this class of mistake costs in work that has
been checked carefully, and this repository holds a survey of exactly that evidence in
[strategies/books2/28_overfitting_and_research_integrity.md](../../../strategies/books2/28_overfitting_and_research_integrity.md).

Two findings there are worth stating here in plain words. First, a study that replicated a published
betting strategy exactly, same data and same algorithm, found that one erroneous data row had
produced reported returns of 17.29 and 28.82 percent; correcting the row turned those same strategies
into losses of 7.36 and 6.31 percent. The code was right; one input was wrong; the published result
was the opposite of the truth. Second, a study that mined 240 accounting variables produced 18,113
trading strategies, of which 30.17 percent cleared the usual statistical hurdle of a score of 2 on
the same data, against a chance rate of 4.55 percent. Neither study is about the files in this
folder, and neither is about crypto. Both show the same thing from different directions: a result
that was fitted, or fed, with information from outside the trading moment does not survive contact
with a clean sample.

The survey also records the honest counterweight: the same review of 207 published predictors found
that almost all of them did replicate, with returns falling by about 26 percent out of sample, and
that publication-bias corrections are of the order of 10 to 15 percent rather than the collapse the
folklore describes. Look-ahead bias is not a reason to distrust all measurement; it is a reason to
insist that every number entering a test was available when the test pretends to trade.

## How this project relates to it

The brief above,
[strategies/books2/28_overfitting_and_research_integrity.md](../../../strategies/books2/28_overfitting_and_research_integrity.md),
is this repository's collected evidence on fitted and contaminated results, and it ends with a list
of the things a research protocol must record: the identity and digest of the dataset, the full
candidate list and the number of trials, the period held out for testing and named by date, and the
data-cleaning rules with their measured effect on the result. Two of those items are implemented in
this repository's own research code:

- [crates/research/src/dataset.rs](../../../crates/research/src/dataset.rs) pins a dataset
  declaration to a digest, so that a search cannot silently change the data between the run that
  found an edge and the run that reported it.
- [crates/research/src/measurement.rs](../../../crates/research/src/measurement.rs) carries the
  false-discovery policy and the trial count alongside the result, which is the systematic answer to
  the "best of thousands of tries" problem described above.

## Where it goes wrong

- The scale is built from the future. This is the defect in all four files: the smallest and largest
  readings of the whole history are used to place today's reading, so every candle's score depends on
  candles that had not happened. Fixing it means computing the smallest and largest readings with the
  data available up to that candle, and recomputing them on every candle.
- Being right for the wrong reason still shows in the profit figure. A contaminated backtest does not
  look broken, it looks excellent, which is why the mistake survives review; the reliable test is to
  rebuild the indicator from a truncated history and check that the signals match.
- A fixed pair list is itself a look-ahead choice. The pairs in the list are the ones that were
  liquid and successful when the file was written, so a search over that list is partly a search over
  survivors, a defect present in the two search files as well as in the rescaling.
- The search multiplies the problem. GodStraNew tries a very large number of indicator, operator,
  period and threshold combinations and reports the best one, which contains the luck of them all.
- The exit ladders were fitted too. A profit target of 56.4 percent at minute zero, as in Zeus, is
  not a rule anyone would choose from first principles; it is the residue of a search over the same
  period that the rescaling had already contaminated.
- One adaptation of a rule does not fix it. Running the corrected version on a further period and
  comparing is the cheapest real test, and a file whose result depends on the whole-file scale fails
  it.

## Try it yourself

You need a spreadsheet and sixty daily closing prices from any public price chart.

1. Column A is the day, column B the close.
2. Column C is the smallest close so far: `=MIN($B$2:B2)`, filled down. Column D is the largest so
   far: `=MAX($B$2:B2)`, filled down.
3. Column E is the smallest close in the whole column: `=MIN($B$2:$B$61)` in the top row only.
   Column F is the largest for the whole column: `=MAX($B$2:$B$61)`.
4. Column G is the honest scale: `=(B2-C2)/(D2-C2)`.
5. Column H is the contaminated scale: `=(B2-E2)/(F2-E2)`.
6. Column I counts the days on which each scale is above 0.9: `=IF(G2>0.9, 1, 0)` and the same for
   column H in column J.

What to notice: compare the totals of columns I and J, and look at which days each one picks. The
contaminated scale fires on a handful of days, all of them near the largest price of the whole
period, and those days are only recognisable as such after the period has ended. Then change the
last five prices in column B and recalculate: the contaminated signals for the earlier days move,
and the honest ones do not.

## Where this came from

- [The lookahead bias folder](https://github.com/freqtrade/freqtrade-strategies/tree/main/user_data/strategies/lookahead_bias)
  and its [readme](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/lookahead_bias/readme.md),
  which names each file and states the mistake in it.
- The four files themselves are `Zeus.py`, `wtc.py`, `GodStraNew.py` and `DevilStra.py`, each one
  inside the folder linked above, whose rules and settings are described in this page.
- The overfitting and research integrity brief,
  [strategies/books2/28](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own survey of overfitting, replication and research integrity, and the source of
  the betting-strategy correction, the multiple-testing numbers and the list of what a protocol must
  record.

## Words used in this tutorial

- backtest: a dry run of a rule over old prices, candle by candle, to see what it would have done.
- candle: one interval of trading, described by its opening, highest, lowest and closing price.
- closing price: the last traded price of a period, fixed only after that period has ended.
- hyperopt: the parameter search shipped with the Freqtrade bot, which tries many settings and keeps
  the best.
- indicator: a number calculated from prices, such as an average, used as an input to a rule.
- look-ahead bias: using, inside a backtest, any number that was not yet known at the moment being
  tested.
- overfitting: fitting a rule so closely to one stretch of history that it describes that stretch and
  nothing else.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
