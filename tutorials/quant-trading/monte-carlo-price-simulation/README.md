# Monte Carlo price simulation: many possible futures from one past

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                             |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing. It draws hundreds of possible future price paths for one share from that share's own past prices, and places no order                                                                                                                                    |
| How often it trades       | Never. The simulation is run once per experiment, and the source repeats it five hundred to fifteen hundred times to test it                                                                                                                                      |
| What you need             | Python and a data file (a daily price history)                                                                                                                                                                                                                    |
| Where the rules come from | [The Monte Carlo project page](https://github.com/je-suis-tm/quant-trading/blob/master/Monte%20Carlo%20project/README.md) and its [script](https://raw.githubusercontent.com/je-suis-tm/quant-trading/master/Monte%20Carlo%20project/Monte%20Carlo%20backtest.py) |
| The underlying research   | The standard [geometric Brownian motion](https://en.wikipedia.org/wiki/Geometric_Brownian_motion) entry that the script's own comment cites for its step                                                                                                          |
| How well it held up       | Weak: one author's own two-share experiment found the fitted path missed both the direction and the crash low, and the extreme-event test is partly rigged because the noise is bell-shaped and the winning path is chosen with hindsight                         |
| Also appears in           | Nothing else in this collection simulates a price path; the nearest teaching is [Telling which kind of market you are in](../../project/measuring-the-regime/README.md)                                                                                           |

## The idea in one paragraph

Monte Carlo simulation, named after the casino in Monaco, is a way to manufacture many possible
futures instead of one. You take a stretch of a single share's past daily prices and measure two
numbers: how much the price drifted, and how widely it jumped (the spread). Starting from the last
price, you add one random step drawn from a bell curve, then another, and another, building one
possible future path. Repeat the whole path hundreds of times and the paths spread into a fan: most
hug the middle and a few stray far. The source then asks two questions of that fan - does any path
reach the lowest price the share actually reached, and does the single best-fitting path get the
direction of travel right?

## Why anyone believed it

A share's price moves because people buy and sell it, and each of those small pushes is hard to
predict. But a long run of small, unrelated pushes tends to settle into a bell curve, the way the
heights of a crowd do, because of a mathematical fact called the central limit theorem: sums of many
independent random nudges look bell-shaped. If the future moves like the past - same drift, same
spread - then simulating the past's physics should give a fair picture of the future's range rather
than one lucky guess. The source also hoped for something stronger: a hidden pattern inside the past
daily changes, which the best-fitting path might pick up and carry forward. The person on the other
side of the prices, if there is one, is simply the crowd of buyers and sellers pushing the price
around; the appeal of the method is that nobody has to be identified as the loser, so it looks like
a way to manage risk the way an insurer prices rare events by simulating them.

## An everyday comparison

You have a long car journey ahead and you know two things from past trips: your average speed and
how much it wobbles, because traffic, weather and service stations vary. You start the clock, add
one random deviation to your speed for the first hour, then for the second, and so on, and you read
off a possible arrival time. Do this a hundred times and you get a fan of arrival times that widens
the further the destination lies: early estimates cluster, late ones spread wide. The Monte Carlo
price simulation is the same device with a share price in place of the clock, and the fan widens
with every step.

## The rules, step by step

The exercise is a forecast, so no shares change hands and there is no entry or exit rule. What the
reader would do, in order, is this.

1. Take the daily closing prices of one share over a chosen stretch. In the source the first case is
   General Electric from 2016-01-15 to 2019-01-15, and the second is Nvidia from 2006-01-15 to
   2009-01-15.
2. Cut the stretch in half. The first half is the training half, the past the model may look at; the
   second half is the testing half, the future it must not look at.
3. Turn the training prices into daily changes in the natural logarithm of the price. The natural
   logarithm, written ln, turns a ratio of two prices into a change you can add up.
4. Compute two summary numbers from those changes: the drift, the average change adjusted for how
   much it wobbles, and the spread, which is the standard deviation, a measure of how far the
   changes scatter from their average.
5. Start at the first price of the training half. For each day, draw a random number g from a bell
   curve centred on zero with a spread of one. The day's change in logarithm is `drift + spread * g`
   and the new price is the old price multiplied by the exponential of that.
6. Keep stepping for as many days as the training half and the testing half together, so the path
   covers both halves. One pass gives one possible path.
7. Repeat the whole path-building many times; the source uses five hundred to fifteen hundred paths,
   and keeps every one.
8. Choose one path as the best: compare each simulated path with the actual training prices, take
   the standard deviation of the differences, and keep the path whose differences are smallest. That
   single path is the forecast. Score it two ways: does its direction - up if the last testing value
   is above the last training value, down otherwise - match the actual direction, and does the
   lowest value across all the paths reach the actual lowest value.

## The maths, with every symbol named

The daily change used to fit the model is the log return.

```text
r(t) = ln(P(t) / P(t-1))
```

- `r(t)` is the change for day `t`, expressed as a logarithm.
- `P(t)` is the closing price on day `t`.
- `ln` is the natural logarithm; it makes a doubling and a halving cancel to zero.

The two fitted numbers.

```text
drift = mean(r) - variance(r) / 2
spread = std(r)
```

- `mean(r)` is the average of all the daily log returns in the training half.
- `variance(r)` is their variance, the average squared distance from that mean.
- `std(r)` is the standard deviation, the square root of the variance.
- `drift` is the average log return less half the variance; the subtraction keeps the average path
  level right, because an up move and an equal-percentage down move do not quite cancel.

One simulated step.

```text
step = drift + spread * g
P(next) = P(now) * exp(step)
```

- `g` is one draw from a standard bell curve (mean zero, spread one), different every step.
- `exp` is the exponential function, the inverse of `ln`.
- `P(next)` is the simulated price one day later.

Choosing the best path and scoring its direction.

```text
best = the path whose std(P(simulated) - P(actual)) over the training half is smallest
direction = sign(P(last testing day) - P(last training day))
```

- `std(P(simulated) - P(actual))` is how far a path sits from the actual training prices, in price
  units, on average; smaller is closer.
- `sign(...)` is plus one when the last testing price is above the last training price and minus one
  when it is below; the same rule is applied to the best path and the two are compared.

## A worked example

Take a made-up share at 100.00, a daily drift of 0.0005 (a twentieth of a percent) and a daily
spread of 0.02 (two percent). Five paths of three steps are enough to see the fan widen. The
multiplier for one step is `exp(drift + spread * g)`; for g of plus one it is 1.02071, for g of
minus one it is 0.98069, for g of plus a half it is 1.01056, for g of minus a half it is 0.99054,
and for g of zero it is 1.00050.

| Path | g step 1 | g step 2 | g step 3 | Price step 1 | Price step 2 | Price step 3 |
| ---- | -------- | -------- | -------- | ------------ | ------------ | ------------ |
| A    | +1.0     | +1.0     | +1.0     | 102.071      | 104.185      | 106.343      |
| B    | -1.0     | -1.0     | -1.0     | 98.069       | 96.175       | 94.318       |
| C    | +0.5     | -0.5     | +0.5     | 101.056      | 100.100      | 101.157      |
| D    | -0.5     | +0.5     | -0.5     | 99.055       | 100.100      | 99.154       |
| E    | 0.0      | 0.0      | 0.0      | 100.050      | 100.100      | 100.150      |

Read path A: every draw is high, so the price climbs from 100.00 to 102.071, then 104.185, then
106.343. Path B does the opposite and falls to 94.318. Path E takes the drift alone and drifts
gently to 100.150. Now look at the range across the five paths at each step: at step one it is
102.071 minus 98.069, or 4.00; at step two it is 104.185 minus 96.175, or 8.01; at step three it is
106.343 minus 94.318, or 12.03. The fan widens with every step, exactly as the arrival-time fan
does. This is a simulation, not a trade: there is nothing to buy or sell and no cost to subtract.

## What the research actually found

The source is one author's own experiment, and it reports its outcome as a "house of cards". The
data is downloaded at run time from Yahoo Finance; the first case is General Electric daily closes
from 2016-01-15 to 2019-01-15 split in half, and the second is Nvidia from 2006-01-15 to 2009-01-15.
No trading is simulated, so costs never enter, and the source says plainly that its whole repository
assumes frictionless trades: "no slippage, no surcharge, no illiquidity".

Two measurements are reported. The first is the extreme-event test: across five hundred simulated
paths the lowest General Electric price reached was 10.99, while the share's real bottom was 6.71;
for Nvidia the lowest simulated price was 6.086 against a real bottom of 5.9. The simulated fan
never fell as far as the market did. The second is the direction test: the source repeats the whole
simulation from five hundred paths up to fifteen hundred in steps of fifty and compares the best
path's direction with the actual direction, and reports that the accuracy is unrelated to the number
of paths and amounts to "tossing a coin". No single accuracy figure is printed; the finding is the
flatness of the result as the path count rises.

What the experiment proves and what it assumes are different things. It shows that, on these two
shares, a drift-and-spread model fitted to the first half did not carry the second half, and it
shows that the author's own selection rule found a path that looked excellent in the training half
and then diverged. It assumes, without testing, that frictionless trading would be available, and it
assumes that the randomness is bell-shaped, which is the very thing the extreme-event test is meant
to probe.

## How this project relates to it

This repository implements no Monte Carlo price-path simulator; there is no drift-and-spread path
generator under `crates/` or `python/`, so a reader cannot run the rule here. What the repository
has is the teaching the same experiment leans on. [Telling which kind of market you are
in](../../project/measuring-the-regime/README.md) shows why a model fitted to one stretch of prices
need not describe the next, which is the failure the source ran into. [Scoring a strategy on its
evidence, not its profit](../../project/scoring-evidence-not-profit/README.md) is the discipline of
asking how much of a good-looking result could be luck, which is exactly the question the source's
accuracy-versus-path-count figure poses. The arithmetic of how a bumpy path turns into a return, a
volatility and a worst fall is set out in [Return, risk and
drawdown](../../foundations/05_return-risk-and-drawdown.md). The sibling page [The model-free
volatility index](../model-free-volatility-index/README.md) comes from the same external repository
and reads the market's own expectation of how far it will move.

## Where it goes wrong

- The winning path is chosen with hindsight. The best path is the one whose deviations from the
  same training prices that fitted the drift and the spread are smallest, so the selection already
  knows the answer it is being scored on. A later literature calls this decision-time leakage: a
  pipeline can look out-of-sample and still quietly use information no real trader had
  (`2605.23959v1`, p.1).
- The noise is bell-shaped by construction, so extreme moves are excluded before the test begins.
  The exercise asks whether the simulation can catch a crash, but its random steps come from a curve
  that makes large jumps vanishingly rare; one study argues that returns at a given level of market
  stress are close to normal and that fat tails appear only because a long history mixes calm and
  stressed stretches (`1310.4538v2`, p.1). A single fitted spread cannot contain that.
- The direction test is close to a coin toss, and that is not a surprise. Long-history work across
  sixteen countries and three asset classes finds that no country-and-asset combination predicts
  consistently in sample and out of sample (`2209.00121v1`, p.1). A model leaning on the past's
  average drift inherits that difficulty.
- Costs are assumed away. The source says all its trades are frictionless, but a fan of paths is
  not even a trade, and the step from a distribution of prices to money made needs order sizes,
  fills and fees that the experiment never states.
- The sample is small and chosen after the fact. Two shares, each split in half once, and the first
  was picked because it was the worst performer of its year; a different share or a different split
  could look different, and there is no replication.
- It will not run as written. The script imports `fix_yahoo_finance`, a package since renamed, so
  the code as committed is a dead dependency.

## Try it yourself

You need a spreadsheet and sixty daily closing prices for any share.

1. Column A is the day, column B the closing price.
2. Column C is the log change: in C2 put `=LN(B2/B1)` and copy it down.
3. Put the average of column C in one cell with `=AVERAGE(C2:C61)`, its variance in another with
   `=VAR.P(C2:C61)`, and its standard deviation with `=STDEV.P(C2:C61)`.
4. The drift is the average minus half the variance; write it in its own cell.
5. In a new column, simulate three steps from the last price. For each step you supply a random
   number g yourself: roll three dice, add the faces, subtract 10.5, and divide by 2.96. That gives
   a number drawn roughly from a bell curve. The step is `drift + spread * g` and the new price is
   the previous price times the exponential of that.
6. Do this for five separate rows, each with its own three rolls, and record the three prices for
   each row.

What to notice: at the first step the five prices sit close together, and by the third step they
have spread apart. Now reroll the dice and the best path changes completely, because nothing about
the winning path was more truthful than the others; it was simply the closest to an answer that was
already known. The standard ways a history can fool you, and the two arithmetic facts that measure
it, are set out in [How a backtest lies](../../foundations/07_how-a-backtest-lies.md).

## Where this came from

- [The Monte Carlo project page](https://github.com/je-suis-tm/quant-trading/blob/master/Monte%20Carlo%20project/README.md)
  and its [script](https://raw.githubusercontent.com/je-suis-tm/quant-trading/master/Monte%20Carlo%20project/Monte%20Carlo%20backtest.py),
  the rules, the two shares and the reported numbers.
- The [geometric Brownian motion](https://en.wikipedia.org/wiki/Geometric_Brownian_motion) entry
  that the script cites for its step.
- Gremm, The Origin of Fat Tails, `1310.4538v2`, on bell-shaped returns within a stress level.
- Bai, 150 Years of Return Predictability Around the World, `2209.00121v1`, on the weakness of
  return prediction.
- Zhang and others, When Alpha Disappears, `2605.23959v1`, on decision-time leakage in backtests.
- [Telling which kind of market you are in](../../project/measuring-the-regime/README.md) and
  [Scoring a strategy on its evidence, not its profit](../../project/scoring-evidence-not-profit/README.md),
  this repository's own teaching on why a fitted past need not repeat.

## Words used in this tutorial

- bell curve: the familiar symmetric hump shape of values that cluster near the middle; formally the
  normal distribution.
- drift: the average daily change in a price's logarithm, adjusted downward by half the variance.
- fat tail: a larger-than-bell-curve chance of an extreme move, far from the middle.
- log return: the natural logarithm of the ratio of one price to the previous one, so movements add
  up.
- Monte Carlo simulation: manufacturing many random possible outcomes by computer and reading the
  range they produce.
- path: one complete simulated run of prices from start to finish.
- spread: here the standard deviation of the daily log returns, a measure of how widely they
  scatter.
- standard deviation: the square root of the variance, quoted in the same units as the data.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
