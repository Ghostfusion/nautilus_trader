# Optimal pairs trading: working out the best levels at which to open and close a bet on two shares

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                          |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Two American shares or funds whose prices usually move together, one bought and one sold short at the same moment                                                                                              |
| How often it trades       | A few times a year at most: once when the combined holding falls to the entry level, once when it rises to the exit level                                                                                      |
| What you need             | Python and a data file                                                                                                                                                                                         |
| Where the rules come from | [QuantConnect strategy library, optimal pairs trading](https://www.quantconnect.com/tutorials/strategy-library/optimal-pairs-trading)                                                                          |
| The underlying research   | Leung and Li, [Optimal Mean Reversion Trading with Transaction Costs and Stop-Loss Exit](https://arxiv.org/abs/1411.5062), International Journal of Theoretical and Applied Finance, 2015, arXiv `1411.5062v3` |
| How well it held up       | Weak: the levels come from a model whose assumptions are never tested, the paper reports no profit of its own, and the only trading record here is one implementation that made twelve trades in five years    |
| Also appears in           | [Dispersion and relative value](../../project/dispersion-and-relative-value/README.md), the closest relative in this collection, which trades the same kind of gap without the derivation                      |

## The idea in one paragraph

Buy one share and sell another short at the same time, choosing the amounts so that the combined
holding is worth roughly the same whatever the market does. Then the only thing that moves the value
of that holding is the gap between the two prices. Most pairs traders decide by eye how far the gap
has to stretch before it is worth trading and how far it has to come back before it is worth closing.
This strategy replaces the eyes with arithmetic. It assumes the gap is pulled back towards an average,
measures how strong that pull has been, and computes the two levels at which opening and then closing
give the best expected outcome once trading costs are paid.

## Why anyone believed it

Two companies in the same business, or two funds holding the same kind of thing, are substitutes. If
one becomes cheap relative to the other, buyers move towards the cheap one and push the gap back. The
counterparty is whoever moved the price in the first place: a fund that received a wave of money and
put it into one name, a manager selling a holding that had grown too large, an index rebuild forcing
money out of a share that is leaving a list and into its replacement, or an investor chasing
yesterday's winner.

Those flows reverse once they are finished, and the gap closes. The belief is that the relationship is
stable and the separation temporary. The mathematical version of that belief is that the pair's value
behaves like a weight on a spring: it wanders, but it is dragged back towards a resting level. If that
is true, the trade can be timed properly, and the timing is a solved problem.

## An everyday comparison

Think of walking a dog on a long leash. The dog wanders ahead, stops, doubles back and inspects a
gate, but the leash always brings it back towards you. You do not bet on the dog's direction at every
moment; you wait until it has gone about as far as the leash comfortably allows, and only then bet
that it will return. Two things decide how good that bet is: how far the leash stretches and how
briskly the dog comes back. The model here measures exactly those two things, and the entry and exit
levels are where the leash is stretched and where the dog is back at your side.

## The rules, step by step

1. Choose two shares or funds whose prices have moved together, and decide which one is bought and
   which is sold short. The library's illustration pairs a gold fund, GLD, with a gold miners fund,
   GDX.
2. Decide the relative amounts. Write the candidate second amount as `beta` and try every value from
   0.01 to 1.00 in steps of 0.01. For each, form a hypothetical holding of one unit of money in the
   first asset minus `beta` in the second, and record its value on each of the last 252 trading days.
3. Fit the mean-pulling model to those 252 values and keep the `beta` whose fit is best. Call it
   `beta*`. This is the same calculation repeated for every candidate, so it is a search rather than a
   formula.
4. From the fitted model, compute two levels: the closing level `b*`, where a position already held is
   sold, and the opening level `d*`, where a new position is bought. The best amount `beta*` and the
   level `b*` are different numbers that happen to share a letter, which is a quirk of the paper.
5. Watch the value of one unit of the first asset minus the chosen amount of the second.
6. When that value falls to the opening level or below, buy the pair: the first asset long, the second
   sold short in the chosen amount, scaled up to the money you want to commit.
7. When the value rises to the closing level, close both legs at once.
8. Retrain every quarter on the most recent 252 days, because the best amounts and the levels change.
9. If the value falls to the floor you have set as a maximum loss, close the position wherever it is.
   The paper solves this version too, and proves that a higher floor makes the trader take the winning
   side earlier, at a lower level.

One warning about the library page: its derivation calls the opening level `d*` and the closing level
`b*`, but a sentence in its trading section says the opposite. The derivation and the paper's own
numbers agree that the opening level is the lower one, which is what is used here.

## The maths, with every symbol named

The model writes the value of the pair as a process dragged towards an average:

```text
dX = mu * (theta - X) dt + sigma * dB
```

- `X` is the value of the combined holding in money, and `theta` the resting level it is pulled
  towards, which is not known in advance and is estimated from the data.
- `mu` is the strength of the pull: a large `mu` snaps back quickly, a small one drifts for a long
  time. `sigma` is the size of the random jitter added each instant, `dB` a small random step, and
  `dt` a small step in time.

The three numbers are chosen to make the observed series least surprising, by maximising:

```text
L = -(1/2) * ln(2 * pi) - ln(sigma_tilde)
    - (1 / (2 * n * sigma_tilde^2)) * sum over i of
        [ x_i - x_(i-1) * exp(-mu * dt) - theta * (1 - exp(-mu * dt)) ]^2
```

- `L` is the average log-likelihood: larger means the model explains the data better. `x_i` is the
  recorded value of the pair on day `i` and `x_(i-1)` the day before.
- `n` is the number of recorded values and `dt` the time between two of them in years, one divided by
  252 for daily data. `sigma_tilde^2` is `sigma^2 * (1 - exp(-2 * mu * dt)) / (2 * mu)`, the one-day
  variance the model implies, where `exp` is the exponential and `ln` the natural logarithm.

This is the method of maximum likelihood: choose the settings under which the sequence that actually
happened is least surprising. The same calculation is repeated for every candidate amount, and the
amount with the largest `L` wins.

The two functions the levels are built from:

```text
F(x) = integral from 0 to infinity of u^(r/mu - 1) * exp( sqrt(2*mu/sigma^2)*(x - theta)*u - u^2/2 ) du
G(x) = integral from 0 to infinity of u^(r/mu - 1) * exp( sqrt(2*mu/sigma^2)*(theta - x)*u - u^2/2 ) du
```

- `u` is the variable being integrated over, and `r` the discount rate the investor chooses, a fixed
  number saying how much a gain later is worth less than one now.
- `F` rises as `x` rises and `G` falls; nobody integrates these by hand, so the library approximates
  the slopes it needs with `f'(x)` approximately `(f(x + h) - f(x)) / h`, with `f` whichever function
  is in use and `h` = 0.0001.

The value of holding the pair, with the closing level in it:

```text
V(x) = (b* - c) * F(x) / F(b*)   when x is below b*
V(x) = x - c                     when x is at or above b*
```

- `V(x)` is the expected value of holding the pair and later closing it when the pair is worth `x`,
  and `b*` is the closing level, still unknown at this point.
- `c` is the cost of one trade, in the same units as `x`, paid once when the position is closed.

The closing level, from equation 4.3, solved for `b` to give `b*`:

```text
F(b) = (b - c) * F'(b)
```

- `F'` is the slope of `F`. The paper proves the solution is unique and sits above both the cost `c`
  and the resting level `theta`.

The opening level, from equation 4.11, solved for `d` to give `d*`:

```text
G(d) * (V'(d) - 1) = G'(d) * (V(d) - d - c_entry)
```

- `c_entry` is the cost of opening the trade, which may differ from the cost of closing it. The
  equation says the opening level is where the extra value of waiting a little longer exactly equals
  the cost of buying now.

The penalty for a round trip is `2 * c`, charged once on opening and once on closing, in the same
units as the pair's value. The library follows the paper in setting both costs and the discount rate
to 0.05, five cents per unit, a demonstration value chosen to make the arithmetic visible rather than
a realistic cost: for two heavily traded funds it is nearer 0.0005 per side, five basis points.

## A worked example

The paper fits its model to a portfolio of one dollar in GLD with 0.454 dollars of GDX sold short,
using daily prices from August 2011 to May 2012. It reports a resting level of 0.5388, a pull of
16.6677 and a jitter of 0.1599, and with a maximum-loss floor two standard deviations below the
resting level it prints an opening level of 0.4978 and a closing level of 0.5570. The daily values
below are invented, but they are of the size that pair produces.

| Day | Value of the pair | What the rule does                                                                         |
| --- | ----------------- | ------------------------------------------------------------------------------------------ |
| 1   | 0.5400            | Nothing. The value is near its resting level of 0.5388.                                    |
| 2   | 0.5230            | Nothing. It has fallen, but the opening level is 0.4978.                                   |
| 3   | 0.5060            | Nothing. Still above the opening level.                                                    |
| 4   | 0.4970            | Open. The value is at or below 0.4978, so buy 1 dollar of GLD and sell 0.454 of GDX short. |
| 5   | 0.5120            | Hold.                                                                                      |
| 6   | 0.5340            | Hold.                                                                                      |
| 7   | 0.5480            | Hold.                                                                                      |
| 8   | 0.5575            | Close. The value is at or above 0.5570, so sell the GLD and buy back the GDX.              |

One unit means one dollar of GLD against 0.454 dollars of GDX sold short, committing 1.454 dollars.

```text
Gross gain per unit = 0.5570 - 0.4978 = 0.0592
1,000 units gross  = 1,000 * 0.0592 = 59.20
Money committed    = 1,000 * 1.454 = 1,454
Gross return       = 59.20 / 1,454 = 4.07 percent

At 5 basis points per side, c = 0.0005:
    cost per unit = 2 * 0.0005 = 0.0010, so 1.00 on 1,000 units
    net = 59.20 - 1.00 = 58.20, which is 4.00 percent of the 1,454 committed

At the paper's demonstration value, c = 0.05:
    cost per unit = 2 * 0.05 = 0.10, so 100.00 on 1,000 units
    net = 59.20 - 100.00 = -40.80, a loss
```

Two things are worth noticing. The two answers have opposite signs and differ only in the cost
assumption, which is why the levels are only as meaningful as the cost behind them: they were computed
with a charge of five cents per unit on a holding worth about half a dollar. And the eight values are
invented, so the example shows the arithmetic of one round trip and nothing about the profit.

## What the research actually found

| Source                                                                          | What it measured                                                                                                     | Result                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Leung and Li, Optimal Mean Reversion Trading                                    | A mathematical derivation with one illustration, GLD against GDX and GLD against SLV, 200 daily points, 2011 to 2012 | The opening and closing levels are derived rather than chosen by eye, and the fitted model reproduces itself when simulated and refitted. The paper reports no trading profit and states that its aim is the timing problem, not a study of pairs trading |
| Quantpedia, Pairs Trading with Stocks (the classic method, not this derivation) | American shares, 1962 to 2002, the twenty closest pairs, after estimated transaction costs                           | 11.16 percent a year, volatility 5.85 percent, worst fall 17 percent, reward-to-risk 1.22, across 40 traded instruments. Quantpedia labels its confidence in the effect Strong                                                                            |
| Do and Faff, summarised in the Quantpedia entry                                 | The same method extended to June 2008                                                                                | Profitability keeps declining, and the cause is not hedge fund activity but the fact that pairs which were close substitutes over twelve months were less often close substitutes over the following six                                                  |
| QuantConnect implementation                                                     | Five years of one pair, levels recomputed quarterly                                                                  | Reward-to-risk of 0.815 against 0.612 for holding the S&P 500, with twelve trades in the whole period; the page notes the small number of trades and suggests adding more pairs                                                                           |

Read together: the mathematics is sound and checkable, the classic pairs trade has a long and
impressive published record, and this derivation has almost no trading evidence behind it. The one
implementation made twelve trades in five years, so a single unusual pair decides the result, and the
general pairs record is one of steady decay as more money ran the same idea.

## How this project relates to it

This repository's brief on predictability,
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
covers a close relative of this rule. Its Section 6 reports a pairs strategy that models the gap
between prices with a hidden state that switches between regimes rather than with one fixed set of
parameters (`2309.00875v3`). On three crude oil contracts from March 2018 to June 2023 it measures
transaction costs contract by contract: 5.80 basis points for Brent, 20.24 for West Texas Intermediate
and 53.71 for the Shanghai contract. The model-based strategy returns 15.18 percent a year with a
reward-to-risk ratio of 1.18, against a crude oil fund's -28.47 percent over the same window. The brief
is explicit that this is three contracts and a one-year test window, and the costs differing by an
order of magnitude between contracts are the point to carry to any pairs trade.

The project's design note on entry and exit prices,
[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), is a design for
review rather than an implementation. It describes a three-barrier exit, an upper level, a lower level
and a time limit, and states the rule that constrains everything here: execution changes the cost of a
decision and never the decision. Its Section 6 gives the break-even identity a short leg needs, which
is the piece a pairs trade is most likely to get wrong. The closest tutorial here is
[dispersion and relative value](../../project/dispersion-and-relative-value/README.md), which trades a
gap between two baskets with a fixed distance rule instead of derived levels.

## Where it goes wrong

- The pull towards the average is an assumption. If the two companies genuinely changed, or one was
  taken over or ran into trouble, the value never returns to its resting level and the opening level
  becomes a place where money keeps disappearing.
- The parameters are estimated on the past and used on the next quarter, and the amount that fitted
  best last year need not fit best next year.
- The levels are not reproducible without software. They come from maximising a likelihood and finding
  the root of an equation built from two integrals, so a reader cannot check them in a spreadsheet,
  and a small change in one fitted number moves them.
- Twelve trades decide the record. The one implementation here made twelve trades in five years, so a
  single bad pair can dominate the result.
- The cost assumption does the work, and the short leg has no natural floor. The levels were computed
  with a charge of 0.05 per unit of the pair's value, far more than two liquid funds cost to trade, and
  change that number and the levels change; the short leg's loss meanwhile grows without limit while
  the long leg can only fall to zero.
- Choosing the pair after knowing which pairs worked is the classic way to make this look better than
  it is.

## Try it yourself

You need a spreadsheet and daily prices for two funds that move together. The paper's own illustration
uses GLD with GDX, which is a good choice because a fund website will give you both.

1. Make one row per day and these columns: Date, Price of the first fund, Price of the second fund,
   Value of the pair, Average value, and Distance.
2. Value of the pair is the first price minus 0.454 multiplied by the second price.
3. Average value is the average of that column over the last 60 rows.
4. Distance is the value minus the average, divided by the standard deviation of the value column over
   the same 60 rows. It answers "how many typical wobbles is the pair away from its usual level".
5. Mark every row where the distance is below -2, which is where this simplified rule would open a
   position, and the next row where the distance rises above 0, which is where it would close.

What to notice: this simplification replaces the paper's derived levels with a fixed distance of two
standard deviations, and it opens only a handful of times in five years. Count how many of the marked
openings were followed by the value returning to its average before it fell another two standard
deviations. Then subtract twice your assumed trading cost from the change in the value over each round
trip and see how much survives: that subtraction is the honest test of any pairs trade.

## Where this came from

- [QuantConnect strategy library: optimal pairs trading](https://www.quantconnect.com/tutorials/strategy-library/optimal-pairs-trading),
  the rules as implemented: a search over the relative amount, a maximum likelihood fit, a derived
  opening and closing level, quarterly retraining, and the figures quoted above.
- Leung and Li,
  [Optimal Mean Reversion Trading with Transaction Costs and Stop-Loss Exit](https://arxiv.org/abs/1411.5062),
  International Journal of Theoretical and Applied Finance, 2015, arXiv `1411.5062v3`. Equations 2.1,
  2.2, 3.3, 3.4, 4.2, 4.3 and 4.11 are used above, and Table 1 and Figure 7 are the fitted numbers.
- [Quantpedia: Pairs Trading with Stocks](https://quantpedia.com/strategies/pairs-trading-with-stocks),
  the classic distance method, its indicative performance over 1962 to 2002, its instrument count and
  the Do and Faff summary of the decline.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  whose Section 6 covers a regime-switching pairs model and per-contract costs, and which cites
  `2309.00875v3`, at [2309.00875](https://arxiv.org/abs/2309.00875).
- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Sections 5 and 6.

## Words used in this tutorial

- mean reversion: the idea that a price which has moved far from its usual level tends to come back.
- Ornstein-Uhlenbeck process: the mathematical name for a value that is jittered at random while being
  pulled back towards a resting level.
- maximum likelihood: choosing a model's numbers so that the sequence of values that actually happened
  is the least surprising the model can produce.
- spread: here, the difference between the value of one asset and the value of another, which is what
  the trade bets on.
- short selling: borrowing something you do not own, selling it, and buying it back later, so you gain
  when its price falls.
- stop-loss: a pre-set level at which a losing position is closed, so the loss cannot grow beyond it.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
