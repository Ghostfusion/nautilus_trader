# Statistical arbitrage: trading a crowd of shares when they wander away from their usual pattern

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                     |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Twenty of the most heavily traded American shares, bought when they fall unusually far below their normal relationship with the market                                                                    |
| How often it trades       | Every thirty days, when the basket is rebuilt                                                                                                                                                             |
| What you need             | Python and a data file of daily share prices                                                                                                                                                              |
| Where the rules come from | [QuantConnect strategy library, mean reversion statistical arbitrage strategy in stocks](https://www.quantconnect.com/tutorials/strategy-library/mean-reversion-statistical-arbitrage-strategy-in-stocks) |
| The underlying research   | Avellaneda and Lee, [Statistical Arbitrage in the U.S. Equities Market](https://www.math.nyu.edu/faculty/avellane/AvellanedaLeeStatArb071108.pdf)                                                         |
| How well it held up       | Mixed: a long published record of profits that has been shrinking, and a large fall of about half the account in the library's own version                                                                |
| Also appears in           | Nothing else in this collection                                                                                                                                                                           |

## The idea in one paragraph

Shares do not move independently. When the whole market falls, most shares fall together, and each
share has its own usual amount by which it moves with the market. This strategy measures that usual
relationship for each share, then watches for the days when a share moves unusually far away from
where the relationship says it should be. When a share has fallen much further than its normal
relationship with the market would explain, the strategy buys it, on the bet that it will drift back.
It does this for twenty shares at once, buying the ones that are furthest below their normal level,
and holds them for a month. Trading many shares at once on the expectation that unusual moves
partly reverse is called statistical arbitrage.

## Why anyone believed it

The counterparty is the impatient seller. When a large investor needs to sell a share quickly,
perhaps to raise cash, they must offer a price low enough to attract buyers fast. That pushes the
price below the level the relationship with the market would justify, and it stays there only until
enough patient buyers arrive. The strategy is on the patient side: it buys the pushed-down share and
waits for the price to recover.

There is a second reason the pattern can persist. A share can fall sharply because of a piece of
news that affects only it, and some sellers react faster than others. If the news is small or
temporary, the fastest sellers overshoot and the price drifts back as everyone else digests it. The
strategy does not try to judge the news. It bets only that a large move away from the usual pattern
is more often an over-reaction than a permanent change, and it relies on many small bets rather than
on any one of them being right.

## An everyday comparison

Think of a commuter train timetable. Most mornings the trains run on time together, and when one is
delayed the delays ripple through the line, so a late train is usually late along with its
neighbours. Now and then a single train is twenty minutes late while the others are fine. Most of
the time that train catches up over the next few stops, because the delay was a slow door or a
crowded platform. A passenger who bets on the late train catching up is usually right. But once in a
while the train is late because the track ahead is closed, and then it never catches up, and the bet
loses badly. Statistical arbitrage is the passenger's bet, applied to hundreds of trains.

## The rules, step by step

1. Build a universe of American shares that trade above 5.00, ranked by the total value of shares
   traded in a day, and keep the twenty most heavily traded.
2. For each share, take the natural logarithm of its daily closing price. The logarithm turns an
   equal percentage move into an equal-sized step, which makes shares of different prices comparable.
3. Find the three common patterns that explain most of the daily movement of the twenty log prices.
   The method is called principal component analysis; think of it as finding a small number of
   shared drumbeats that most of the shares move to. The first pattern is usually the whole market
   moving, and the second and third capture other shared behaviour.
4. For each share, fit a straight-line rule that predicts its daily move from the three common
   patterns, using the past window of prices. The share's sensitivity to each pattern is the slope of
   the line.
5. Take the part of each day's move that the three patterns do not explain; this leftover is called
   the residual. Over the past window, standardise each share's residuals by subtracting their
   average and dividing by their usual wobble. The result of the last day is the z-score.
6. Select the shares whose z-score is below -1.5, meaning today's move was unusually far below the
   share's normal relationship with the common patterns.
7. Give each selected share a weight equal to how far below it is. A share that is twice as far below
   as another gets twice the money. The weights are set to add up to 1, so the whole account is
   invested in the selected shares, and no short selling is used.
8. Hold the basket for thirty days, then recompute everything and rebuild.

A caution from the library page's own discussion thread: users reported that a straightforward copy
of the code traded only one or two shares, because of how the data was set up. Any real test has to
confirm that the universe is really twenty shares before trusting the result.

## The maths, with every symbol named

The whole method is a one-line idea: separate what the shares have in common from what is special to
each share, then bet on the special part coming back.

Write each share's log price as the sum of a common part and a leftover:

```text
x_i = common part + e_i
```

- `x_i` is the natural logarithm of share `i`'s price.
- `common part` is whatever the three shared patterns say the move should be.
- `e_i` is the residual, the part of the log price not explained by the shared patterns.

The z-score asks how large the residual is compared with its own history:

```text
z_i = (e_i - average of past e_i) / (standard deviation of past e_i)
```

- `average of past e_i` is the typical level of the residual over the recent window.
- `standard deviation of past e_i` is the size of the residual's usual wobble.
- A z-score of -2 means the residual is two usual wobbles below its typical level.

Then the weight given to each selected share is:

```text
w_i = z_i / (sum of |z_j| over the selected shares)
```

- `w_i` is the fraction of the money placed in share `i`.
- `|z_j|` is the z-score without its sign, so a share that is 2.5 wobbles below gets a bigger weight
  than one that is 2.0 wobbles below.
- Because the selected z-scores are all negative, each weight comes out positive, and they add to 1.

Finally the cost of rebuilding:

```text
Cost = t * c
```

- `t` is the traded fraction: 2.0 if every share is sold and replaced, because selling and buying
  count twice, and less when some shares are kept.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and the selling price plus commission, around 0.0005 to 0.001 for large American shares.

## A worked example

To keep the arithmetic to one page, the example uses a single common pattern, the market, instead of
three. Everything else follows the rules. Six shares, one day on which the market fell 1.5 percent.

| Share | Sensitivity to market | Actual move | Fitted move | Residual | Past wobble | z-score |
| ----- | --------------------- | ----------- | ----------- | -------- | ----------- | ------- |
| A     | 1.0                   | -1.50       | -1.50       | 0.00     | 0.8         | 0.00    |
| B     | 1.2                   | -3.00       | -1.80       | -1.20    | 1.0         | -1.20   |
| C     | 0.9                   | -3.75       | -1.35       | -2.40    | 1.2         | -2.00   |
| D     | 1.1                   | +0.50       | -1.65       | +2.15    | 1.5         | +1.43   |
| E     | 1.0                   | -3.75       | -1.50       | -2.25    | 0.9         | -2.50   |
| F     | 0.8                   | -3.20       | -1.20       | -2.00    | 1.4         | -1.43   |

The fitted move is the sensitivity times the market move; for share C that is 0.9 times -1.5 equals
-1.35. The residual is the actual move minus the fitted move; for share C that is -3.75 minus -1.35
equals -2.40. The z-score is the residual divided by the past wobble; for share C that is -2.40
divided by 1.2 equals -2.00.

Shares C and E have z-scores below -1.5, so they are selected. Their weights are:

```text
w_C = 2.00 / (2.00 + 2.50) = 0.444
w_E = 2.50 / (2.00 + 2.50) = 0.556
```

Now suppose the next thirty days give share C a return of +4 percent and share E a return of +2
percent:

| Share | Weight | Next-month return | Contribution   |
| ----- | ------ | ----------------- | -------------- |
| C     | 0.444  | +4 percent        | +1.778 percent |
| E     | 0.556  | +2 percent        | +1.111 percent |
| Total | 1.000  |                   | +2.889 percent |

The cost, assuming both positions change and the whole account is traded twice:

```text
t = 2 * 1.0 = 2.0
Cost = 2.0 * 0.001 = 0.002, that is 0.2 percent
Net return for the month = 2.889 - 0.20 = 2.689 percent
```

Notice how much depends on the past wobble. Share C and share E fell by the same amount, but E's
wobble is smaller, so E looks more unusual and gets more money. If the wobble is measured on a quiet
stretch and the next month is violent, every share will look unusual and the selection becomes
meaningless. That fragility is the heart of what follows.

## What the research actually found

| Source                                                                | What it measured                                                       | Result                                                                                                                                                                                    |
| --------------------------------------------------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Avellaneda and Lee, Statistical Arbitrage in the U.S. Equities Market | A large universe of American shares, 1997 to 2007                      | Taking out the common market patterns and trading the leftovers produced higher reward-to-risk than simpler methods over the sample; the paper is the source of the library page's method |
| The library page's own backtest                                       | Twenty shares, three common patterns, thirty-day rebuild, 2010 to 2019 | Above 6 percent a year, but with a worst fall of about 49 percent, meaning the account lost roughly half its value at the low point                                                       |
| Quantpedia, pairs trading with stocks                                 | Long the loser and short the winner of a matched pair, 1962 to 2002    | 11.16 percent a year, volatility 5.85 percent, worst fall 17 percent, reward-to-risk 1.22 using forty instruments rebuilt daily; the entry calls its confidence strong                    |
| Quantpedia, citing Chen, Chen and Li and Do and Faff                  | Whether the pairs idea still works in later samples                    | The returns to simple pairs trading have diminished over time, as pairs that moved together in the past stop moving together in the future                                                |
| Miroslav Fil, cited by Quantpedia                                     | Pairs trading on American shares, 1990 to 2020, including the pandemic | The strategy overall failed to beat the market benchmark even after tuning its settings, though it performed strongly in falling markets                                                  |

Read together, the picture is this. There is a real and long-documented tendency for short-term
extreme moves to partly reverse, and statistical arbitrage was one of the most successful styles in
the industry for a generation. But the profits have been shrinking as more money ran the same idea,
the simple versions have stopped beating the market in recent samples, and this particular
implementation gave up about half the account at its worst point. A method that looks reliable in a
calm sample can be destroyed by the one month everything moves at once.

## How this project relates to it

This repository has a brief on arbitrage across venues,
[cross-venue and arbitrage](../../../strategies/books/10_cross_venue_and_arbitrage.md). One of its
core points is the one this tutorial needs: statistical arbitrage is weaker than arbitrage. A
strategy can have a positive expected gain on average and still contain no guaranteed profit at all,
because the gain is a tendency, not a certainty, and a screen that does not say which definition it
means is measuring nothing precise. The brief cites the formal treatment of that distinction.

The repository's predictability brief,
[the predictability and trading strategies brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
has a section on regime-switching statistical arbitrage in crude oil futures. It reports a study
that models the spread between related contracts as switching between states, and finds that the
regime matters: a fixed rule that works in one state fails in another. That is the same lesson this
tutorial's last section reaches from a different direction.

## Where it goes wrong

- The relationship can break instead of holding. If the fall was caused by real, lasting news, the
  share will not return to its old pattern, and the strategy has bought a falling share at what
  looked like a discount but was not. This is the single largest risk.
- Everything becomes unusual at once. In a market-wide panic, almost every share moves far below its
  usual relationship, so the method selects nearly the whole universe and the diversification it
  relies on disappears. The library page's worst fall of about half the account is this happening.
- Measuring the wobble is fragile. The past wobble is estimated from a short window, and it changes
  with the window length. The same rule with a different look-back selects different shares and can
  produce a different result, which is a large degree of freedom being chosen after the fact.
- Short samples and many choices. Three patterns, twenty shares, a threshold of -1.5 and a thirty-day
  hold are all settings. Trying many combinations and keeping the best is how a backtest turns into a
  story, and the library page invites exactly that tuning without guarding against it.
- Costs on a high-turnover rule. A thirty-day rebuild that changes most of the basket pays roughly
  2.4 percent a year at ten basis points per trade, and the pushed-down shares are the ones whose
  buying and selling prices are furthest apart.
- The idea is old and crowded. Trading patterns like this one has been a profession for decades. The
  counterparty who was once slow is now a competitor with better data, and the edge shrinks as more
  people stand on the same side of the trade.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily prices for a handful of large
companies and a market index.

1. Build a sheet with one column per share and one row per day, holding the natural logarithm of
   each price. Your spreadsheet will do logarithms with a built-in function.
2. Add a column for the market index, also in logarithms.
3. For each share, work out its sensitivity by eyeballing the past twenty days: how much does the
   share move when the index moves one unit? Better, use the spreadsheet's slope function.
4. Add a column for the residual: the share's daily move minus its sensitivity times the index's
   daily move.
5. Over the first twenty days, compute the average and the wobble of the residual, then compute the
   z-score of the residual on day twenty-one for every share.
6. Note any share with a z-score below -1.5, and write down what it did over the next twenty days.

What to notice: most of the time the pushed-down share does recover a little, which is the effect the
strategy is built on. Now and then it keeps falling. If you repeat the exercise over a year, the
handful of times it kept falling will account for most of the difference in the result, which is why
the published record includes a worst fall near half the account.

## Where this came from

- [QuantConnect strategy library: mean reversion statistical arbitrage strategy in stocks](https://www.quantconnect.com/tutorials/strategy-library/mean-reversion-statistical-arbitrage-strategy-in-stocks),
  the rules as implemented: the twenty most liquid shares, three common patterns, a z-score threshold
  of -1.5, and a thirty-day rebuild.
- Avellaneda and Lee, [Statistical Arbitrage in the U.S. Equities Market](https://www.math.nyu.edu/faculty/avellane/AvellanedaLeeStatArb071108.pdf),
  the source of the method and the comparison between pattern-based and fund-based baskets.
- [Quantpedia: pairs trading with stocks](https://quantpedia.com/strategies/pairs-trading-with-stocks),
  the performance figures, the instrument count, and the citations on profits shrinking over time.
- [Cross-venue and arbitrage](../../../strategies/books/10_cross_venue_and_arbitrage.md) and
  [the predictability and trading strategies brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's own studies of arbitrage definitions and of regime-switching relative-value
  trading.

## Words used in this tutorial

- arbitrage: buying and selling related things to lock in a gain from a price difference.
- dollar volume: the total value of a share's trades in a day, price times number of shares traded.
- log price: the natural logarithm of a price, which turns equal percentage moves into equal steps.
- mean reversion: the tendency of something that has moved far from its usual level to move back.
- principal component analysis: a method for finding a few shared patterns in many moving series.
- residual: the part of a move that the fitted relationship does not explain.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- z-score: how many usual wobbles a value sits away from its own average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
