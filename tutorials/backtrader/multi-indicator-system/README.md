# Multi-indicator systems: waiting for several signals to agree

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                               |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Spot gold on 15-minute bars, with one rule using daily gold and two using other instruments                                                                                                         |
| How often it trades       | From dozens to hundreds of round trips in three months, depending on how many conditions must agree                                                                                                 |
| What you need             | Python and a file of 15-minute price bars                                                                                                                                                           |
| Where the rules come from | [The Strategy Compendium, article 07, multi-indicator systems](https://backtrader.readthedocs.io/en/latest/strategies-series/en/07-multi-indicator-system.html)                                     |
| The underlying research   | None as a paper: the assembly method is MetaQuotes' MQL5 Wizard, which combines signal modules by vote or by weighted score, and the efficiency ratio is Perry Kaufman's 1995 adaptive-average work |
| How well it held up       | Weak: no independent test, three months of one instrument at zero commission, and the library's own ports finish between nine percent down and four percent up                                      |
| Also appears in           | Nothing else in this collection                                                                                                                                                                     |

## The idea in one paragraph

A single indicator is a dictator: when it says buy, you buy, and nothing argues. This category builds
a committee instead. Trend, momentum and location each get a seat, and the system acts only when
enough seats agree. There are two ways to run the meeting. In a vote, every seat must say yes before
anything happens, which produces very few trades. In a score, each seat contributes points and the
total crossing a line opens a trade, which produces more trades but hides the fact that some seats
are worth more than others. The library holds 29 such systems, most of them ports of commercial
platform expert advisors.

## Why anyone believed it

Every indicator measures one slice of the market, and each slice has a failure mode. A momentum
measure gives false signals in a flat market; a trend measure is late at turning points; a location
measure is fooled by a strong trend. The committee idea says that if you require the measures to
agree, you will skip the moments when any one of them is wrong. It also feels like risk control: more
conditions passing at once sounds like more confidence.

The counterparty on the other side is the same forced seller as in other reversal systems, and the
same late buyer as in trend systems. The committee is not a new edge; it is a claim that a
combination of weak edges is safer than one, which is true only when the members disagree for
different reasons.

## An everyday comparison

A hiring panel of four interviewers, each of whom asks a different question. If everyone must approve
before a candidate is hired, the company hires very few people, and the ones it does hire tend to be
unobjectionable rather than excellent. Adding interviewers does not make the panel wiser if they all
ask about the same thing; it just makes the decision slower and the shortlist shorter. The committee
of indicators behaves the same way. Agreement is only informative when the members are measuring
different things.

## The rules, step by step

The category holds 29 systems. Most run on 15-minute gold from 3 December 2025 to 10 March 2026, on a
million-dollar account with no commission. The Kaufman efficiency-ratio system uses daily gold from
2008 to 2025.

1. Camel CCI MACD, the unanimous-vote template. Compute three things: the Commodity Channel Index
   over 30 days, the MACD with its signal line, and two exponential averages, a slow one of the highs
   and a fast one of the lows, which together form a channel.
2. Camel, long entry. All four conditions on the previous bar: the index above +100, the MACD line
   above zero, the MACD line above its signal, and the close above the slow upper channel average.
   Only then buy.
3. Camel, short entry. The exact mirror: index below -100, MACD line below zero, MACD line below its
   signal, and close below the fast lower channel average.
4. Camel, exits. Any one of these closes a long: the MACD line falls back under its signal, the index
   drops back inside 100, or the price touches a take-profit 40 pips away.
5. MQL5 Wizard, the scoring template. Give the MACD a weight of 0.9 and the Parabolic SAR a weight of
   0.1. Each casts plus or minus 100 times its weight, so the MACD controls 90 points and the SAR 10.
6. Wizard, entries and exits. A total of +20 or more buys, -20 or less sells short. A total of -100
   closes a long and +100 closes a short. A fixed stop and target, 50 and 115 of the instrument's
   smallest quoted units, also close the trade.
7. The rest of the set divides the work differently. One rule uses the SAR for direction, a trend
   strength reading above 20 for permission, and a 100-day average for the trend. One requires the
   efficiency ratio to exceed 0.3 before trusting an adaptive-average breakout. One feeds five
   indicators into a weighted perceptron, and one compresses seven into a single wave. One pairs a
   direction reading with a doubling-after-losses sizing rule.
8. Every rule sizes positions as a fixed lot and reviews on either the 15-minute bar or the daily bar,
   according to its market.

## The maths, with every symbol named

The committee can be written down in two ways, and the category rests on one more measure.

The vote, where every condition must pass:

```text
Trade = 1 only if condition_1 AND condition_2 AND ... AND condition_k are all true
```

- `k` is the number of conditions; the Camel rule uses four.
- `AND` means the trade happens only when all of them hold on the same bar, so one failure blocks it.

The score, where contributions are added:

```text
Score = w_1 * vote_1 + w_2 * vote_2 + ... + w_k * vote_k
```

- `vote_i` is plus 100 when indicator `i` is bullish, minus 100 when bearish, and zero when it is
  neutral.
- `w_i` is the weight of indicator `i`; the Wizard uses 0.9 for the MACD and 0.1 for the SAR.
- `Score` ranges from minus 100 to plus 100. The library opens a trade above +20 or below -20 and
  closes it at the full opposite +100 or -100.

The Kaufman efficiency ratio, the filter one rule uses:

```text
ER = |C_t - C_(t-n)| / (sum of |C_i - C_(i-1)| over the last n days)
```

- `C_t` is today's close and `C_(t-n)` the close `n` days ago, so the numerator is the net distance
  travelled.
- The denominator adds up every daily move, up or down, so it is the total distance walked.
- `ER` is near 1 when the price moved in a straight line and near 0 when it wandered. Above 0.3, the
  library decides the market is trending enough to follow.

The exponential moving average, the smoothing inside the channel and the MACD:

```text
EMA_t = EMA_(t-1) + alpha * (C_t - EMA_(t-1))
```

- `EMA_t` is today's smoothed value and `C_t` today's close.
- `alpha` is the smoothing rate, `2 / (N + 1)` for a window of `N` days; a larger `alpha` tracks the
  price more closely.
- Each new value leans toward the price by the fraction `alpha`, which is why the average is called
  exponential: older prices fade away by a fixed ratio rather than dropping out all at once.

## A worked example

The Wizard's scoring rule over six made-up bars. The MACD line and its signal are in price units, the
SAR is a trailing stop below or above the price, and the price is near 2,000. The MACD casts a vote
of plus or minus 90 and the SAR plus or minus 10.

| Bar | MACD | Signal | Close  | SAR    | MACD vote | SAR vote | Total | Action         |
| --- | ---- | ------ | ------ | ------ | --------- | -------- | ----- | -------------- |
| 1   | 0.50 | 0.40   | 2000.0 | 1995.0 | +90       | +10      | +100  | buy at 2000.0  |
| 2   | 0.45 | 0.42   | 2002.0 | 1996.0 | +90       | +10      | +100  | hold           |
| 3   | 0.30 | 0.38   | 1998.0 | 1997.0 | -90       | +10      | -80   | hold           |
| 4   | 0.20 | 0.35   | 1996.0 | 1999.0 | -90       | -10      | -100  | sell at 1996.0 |
| 5   | 0.10 | 0.30   | 1995.0 | 1998.0 | -90       | -10      | -100  | sell short     |
| 6   | 0.05 | 0.25   | 1997.0 | 1996.0 | -90       | +10      | -80   | hold short     |

On bar 1 the MACD line of 0.50 is above its signal of 0.40, so it votes plus 90; the close is above
the SAR, so the SAR votes plus 10; the total of +100 is above the +20 opening line, so the rule buys.
On bar 3 the MACD has crossed below its signal but the total of -80 is not yet at the -100 closing
line, so the long is held. On bar 4 the total reaches -100, the full opposite vote, and the long is
closed at 1996.00. On bar 5, now flat and with the total still at -100, the rule opens a short.

The long trade is worth, per unit of the instrument:

```text
Gross change = 1996.00 - 2000.00 = -4.00
Percentage   = -4.00 / 2000.00 = -0.20 percent
Cost         = 0 in this file: the library charges no commission and no spread
```

The illustration follows only the score; the file also applies a fixed stop and target, which would
often close the trade earlier. The arithmetic shows how a committee that is 90 percent one member
behaves like that one member: the MACD decides, and the SAR only tips a close call. In this short
example the committee lost money, which matches the file's own result.

## What the research actually found

The library reports 29 backtests. The two headline systems are the vote and the score, and they end
far apart.

| System                | Bars  | Trades | Wins  | Final value  | Change |
| --------------------- | ----- | ------ | ----- | ------------ | ------ |
| Camel CCI MACD        | 6,071 | 687    | 352   | 1,038,763.00 | +3.88% |
| MQL5 Wizard MACD PSAR | 3,077 |        | 48.6% | 910,005.00   | -9.00% |

The unanimous-vote Camel system ground out a small gain from 687 trades, with 352 wins against 335
losses. The scoring Wizard made 3,077 trades, won under half of them, and finished nine percent down.
Both ran at zero commission. The lesson is not that votes beat scores; it is that in a market as
choppy as 15-minute gold, a faint signal cannot survive even a sliver of friction, and the scoring
system's extra trades only gave it more chances to pay that friction.

Now the honest part, which the whole category turns on. Requiring four conditions to agree mostly
reduces the number of trades rather than improving the odds. Suppose each condition passes on half
the bars. If the conditions were independent, all four would pass on one sixteenth of the bars, so
the trade count falls by sixteen times. But the conditions here are not independent. The Commodity
Channel Index, the MACD and a moving-average channel are all transformations of the same recent price
path, so they pass together far more often than independent events would. Agreement therefore removes
trades without adding new information. A filter changes the average outcome per trade only if it is
informative; with no edge, it merely leaves a smaller sample, and a smaller sample's result is
noisier, so the apparent win rate swings more and invites the reader to believe a fluke. On top of
that, each condition is a parameter, so more conditions means more chances to fit the sample while
the sample shrinks. That is the worst possible combination.

One thing must be said plainly, because it is the heart of this whole group of tutorials. Every
backtest in the compendium asserts its final value, its reward-to-risk ratio (the return earned per
unit of the strategy's own wobble, also called the Sharpe ratio) and its worst fall against numbers
captured when the strategy was migrated. The Camel file asserts 6,071 bars, 687 trades, 352 wins and
a final value of 1,038,763.00 to the cent. Passing proves the engine computes exactly what the file
says. It proves nothing about whether the strategy earns anything.

## How this project relates to it

The repository's brief
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
states the principle this category illustrates. Each extra indicator adds a period, a weight or a
threshold, and every added knob buys more power to fit history while quietly spending the reliability
of a future test. The brief records that sorting on simple functions of 240 accounting variables
produced 18,113 candidate strategies of which about 30 percent cleared a conventional significance bar
(`2209.13623v3`, p.7), which is the same arithmetic of a wide search applied to indicators rather than
to accounting data.

The companion brief
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
adds the measured version: indicator-only rules win under half their trades, and adding a
price-pressure measure, not another oscillator, is what lifts the win rate (`2206.12282v1`).

## Where it goes wrong

- Agreement is not independence. When every condition reads the same prices, the committee is one
  member wearing several hats, and the extra votes buy no new information.
- The score hides the weights. The Wizard gives the MACD 90 percent of the vote, so the system is
  really a MACD rule with a ceremonial second member. Reading the weights is essential.
- A small sample lies louder. Fewer trades means the result is dominated by a handful of outcomes, so
  the win rate and the worst fall can swing widely between samples.
- Entry gets later with every condition. Demanding confirmation means buying after more of the move
  has already happened, which lowers the payoff on the winners even when the direction is right.
- Costs eat faint edges. The scoring system made 3,077 trades over three months; at any realistic
  cost per trade, that many round trips is a large bill against a very small edge.
- One bad sizing rule can undo everything. The Universum system doubles its stake after each loss,
  which turns a missing edge into a large one at the wrong moment.

## Try it yourself

You need a spreadsheet and one instrument's daily high, low and close.

1. Put the date, high, low and close in four columns.
2. Add a 20-day moving average of the close and a 20-day momentum column, the close minus the close
   twenty rows ago.
3. Add a column that marks a first signal when the close is above its average, and a second signal
   when the momentum is above zero.
4. Add a filter, a relative strength reading, and mark a third signal when it is above 50.
5. Count the rows where the first signal alone appears, then the rows where the first and second both
   appear, then all three. Do not trade anything; just count.
6. For each set of rows, compute what the price did over the next five days on average.

What to notice: the row count collapses as conditions are added, and the average next-five-day move
usually changes little. The extra conditions bought fewer trades, not better ones. If one filter does
improve the average, ask whether that filter is measuring something the others were not, or just
re-describing the same prices more slowly.

## Where this came from

- [The Strategy Compendium, article 07, multi-indicator systems](https://backtrader.readthedocs.io/en/latest/strategies-series/en/07-multi-indicator-system.html),
  the 29-system inventory, the Camel rule, the Wizard scoring rule and the backtest figures quoted
  above.
- [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief on the cost of adding parameters and running wide searches (`2209.13623v3`).
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  the brief recording that indicator-only rules win under half their trades (`2206.12282v1`).

## Words used in this tutorial

- Commodity Channel Index: a measure of how far a price has strayed from its own recent average.
- committee: the informal name for a set of indicators that must agree before a trade.
- efficiency ratio: net distance travelled divided by total distance walked, near 1 for a straight
  trend and near 0 for a wandering market.
- exponential moving average: an average that leans toward recent prices and fades older ones.
- MACD: a momentum measure built from two exponential averages and a signal line.
- Parabolic SAR: a trailing stop that tightens as a trend continues.
- perceptron: a weighted sum of inputs that produces a single score, a tiny neural network.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
