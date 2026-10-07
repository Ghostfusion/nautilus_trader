# Dispersion and relative value: trading the gap between two baskets, not the direction

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Two baskets of shares at the same time, one bought and one sold short, so the bet is on their prices relative to each other and not on the market as a whole                                                         |
| How often it trades       | When the gap between the two baskets stretches beyond a chosen level, and again when it narrows back                                                                                                                 |
| What you need             | A spreadsheet and about five years of monthly prices for two comparable funds                                                                                                                                        |
| Where the rules come from | [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), Sections 6 and 9                                                                                                                |
| The underlying research   | The pairs-trading study of Gatev, Goetzmann and Rouwenhorst and a cointegration test on sector pairs, both reported in [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md) Section 6 |
| How well it held up       | Weak: the classical pairs figure is reported second-hand without confirmation, the sector-level version is sensitive to how it is specified, and no net-of-cost sector estimate was found                            |
| Also appears in           | Nothing else in this collection describes the same idea                                                                                                                                                              |

## The idea in one paragraph

Dispersion is how far apart the returns of the members of a group are from the group's own average in
a single period. When two similar baskets drift apart, one has become expensive relative to the other.
A relative-value book does not guess which way the market will go: it buys the one that has fallen
behind and sells short the one that has run ahead, putting the same amount of money on each side. The
profit arrives when the gap narrows, whichever way both prices move. The bet is not that the market
rises or falls, but that the relationship between the two baskets is steady enough for a stretched gap
to close.

## Why anyone believed it

Two things that should be worth about the same can drift apart for reasons that have nothing to do
with their value. A fund facing withdrawals sells whatever is easiest to sell, which may be one sector
and not another. An index rebuild forces money out of a company that is leaving the index and into its
replacement. A large investor moving between sectors pushes one price up and another down. The person
on the other side of a relative-value trade is often someone who is trading for a reason other than
the outlook: they need cash, they need to match a benchmark, or they are moving a position that has
grown too large.

If those flows reverse once the forced selling is done, the gap closes and the trade is paid. The
belief is that the pressure is temporary and the relationship is not.

## An everyday comparison

Picture a market with two stalls selling the same crate of apples. Most days they charge about the
same. One morning one stall has taken delivery of far too many crates and drops its price by a fifth
to clear them. The apples are identical, so the gap is clearly a temporary accident of supply, not a
sign that one crate is better. If you could buy at the cheap stall and, at the same moment, sell
crates at the expensive one, you would pocket the difference once the cheap stall sold out and its
price came back up. You would not care whether apple prices in general rose or fell that day, because
you hold one crate and owe one crate, and the two cancel.

## The rules, step by step

1. Choose two funds that hold similar things, such as two broad funds tracking the same index, or two
   sector funds whose businesses overlap enough to be substitutes.
2. Once per period, work out the gap between them. Take the price of the first fund, divide it by the
   price of the second, and subtract one. Write the result as a percentage.
3. Over a look-back window, say the last 60 periods, record the average gap and how widely the gap has
   swung around that average.
4. If the current gap is far above its average, the first fund is expensive relative to the second:
   sell short the first and buy the second. If the gap is far below its average, do the reverse.
   "Far" means some number of typical swings, commonly two.
5. Put the same amount of money on each side, so a move in the market as a whole pushes one leg up and
   the other down by a similar amount.
6. Close the position when the gap returns close to its average, or when a fixed number of periods has
   passed, whichever comes first.
7. Hold nothing in between. Do not widen the position because the gap stretched further.

This is one of only two rows in the eligibility table of the source that a memoryless market does not
rule out, and the source adds a warning: it is eligible only if the spread relationship is stable, and
the evidence is thin.

## The maths, with every symbol named

Dispersion is a single number describing one period. It is the standard deviation of the returns of
the members of a group around the group's own average:

```text
D_t = sqrt( (1 / N) * sum over i of ( r_i,t - r_bar_t ) ^ 2 )
```

- `D_t` is the dispersion in period `t`, written as a decimal.
- `N` is the number of members in the group.
- `r_i,t` is the return of member `i` in period `t`.
- `r_bar_t` is the average of those returns in that same period.

A large `D_t` means the members went very different ways that period. This is the thing a
relative-value book tries to sell, and it is also what the rebalancing premium of a fixed-weight
portfolio pays on.

The gap between two funds, expressed so that both funds can be compared on the same scale:

```text
G_t = ( P_A,t / P_B,t ) - 1
```

- `G_t` is the gap in period `t`, a percentage.
- `P_A,t` is the price of fund A in period `t`.
- `P_B,t` is the price of fund B in the same period.

A positive `G_t` says fund A is expensive relative to fund B. To decide whether a gap is unusual, turn
it into a count of typical swings:

```text
z_t = ( G_t - G_bar ) / s_G
```

- `z_t` is how many typical swings the gap sits from its average.
- `G_bar` is the average gap over the look-back window.
- `s_G` is the standard deviation of the gap over the same window.

The rule "sell A and buy B when `z_t` is above two" says: act only when the gap is unusually wide by the
standards of its own history.

Once the book is open with the same money `M` on each leg, its profit is simple to write. For a long
position in A and a short position in B:

```text
Profit = M * ( R_A - R_B )
```

- `Profit` is the gross profit in the money of the account.
- `M` is the money placed on each leg.
- `R_A` is the return of A over the holding period, as a decimal.
- `R_B` is the return of B over the same period.

The market's common move sits inside both returns, so it is cancelled by the subtraction. The profit
depends only on the difference between the two.

## A worked example

Two sector funds, A and B, that hold broadly similar businesses. Six monthly observations. The gap is
`P_A / P_B - 1`, and the look-back window says the average gap is 0 percent with a typical swing of
1.5 percent, so "two typical swings" is a gap of 3 percent. The rule opens when the gap reaches 3
percent and closes when it shrinks to 1 percent.

| Period | Price of A | Price of B | Gap    |
| ------ | ---------- | ---------- | ------ |
| 1      | 100.00     | 100.00     | 0.00%  |
| 2      | 104.00     | 100.00     | +4.00% |
| 3      | 106.00     | 103.00     | +2.91% |
| 4      | 105.00     | 104.00     | +0.96% |
| 5      | 103.00     | 104.00     | -0.96% |
| 6      | 101.00     | 103.00     | -1.94% |

At the end of period 2 the gap is +4.00 percent, so A is expensive relative to B. The book sells short
fund A and buys fund B, with `M = 50,000` on each leg, inside a 100,000 account. It closes at the end
of period 4, when the gap has fallen to +0.96 percent. Both prices rose over the hold, which is exactly
the point: the direction did not matter.

| Leg    | Entry price | Exit price | Return |
| ------ | ----------- | ---------- | ------ |
| Fund B | 100.00      | 104.00     | +4.00% |
| Fund A | 104.00      | 105.00     | +0.96% |

```text
Profit = 50,000 * (0.0400 - 0.0096) = 50,000 * 0.0304 = 1,519.23
```

Now the costs, which this collection's execution tutorial builds in detail. Using a cost of 14 basis
points per side (one basis point is one hundredth of one percent), and remembering that there are four
crossings: buying B, selling short A, then closing each leg.

```text
Trading cost = 4 * 50,000 * 0.0014 = 280.00
Borrow cost  = 50,000 * 0.005 * (2 / 12) = 41.67   (0.5 percent a year on the short leg, held two months)
Net profit   = 1,519.23 - 280.00 - 41.67 = 1,197.56, which is 1.20 percent of the 100,000 account
```

Two things to notice. The four crossings plus the borrow fee took a quarter of the gross profit, so the
cost of putting the trade on matters as much as the size of the gap. And the short leg has no natural
floor: if the expensive fund had kept rising, the loss on that leg would have grown without limit while
the long leg only ever gains what a price can rise.

## What the research actually found

The source collects three pieces of evidence, and labels how far each one was confirmed. None of them
was read at the primary paper by the author of that document.

| Finding                                                                        | Label          | Detail                                                                                                                              |
| ------------------------------------------------------------------------------ | -------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Pairs trading on American shares, Gatev, Goetzmann and Rouwenhorst             | Summary only   | About 12 percent extra a year, with 4 to 6 percent annualised volatility                                                            |
| Cointegration-based sector pairs                                               | Summary only   | 1.01 percent total profit, reward-to-risk 0.28, against 0.81 for a lower-risk version                                               |
| Dispersion traded through options, one sector's volatility against the index's | Mechanism only | Earns when the realised correlation between sectors differs from the priced one, and loses badly when correlations jump in a crisis |

The honest summary from the source is a sentence worth repeating: no peer-reviewed, net-of-cost
estimate of a sector-level dispersion strategy was located in that research. The classical pairs
number is large and famous, but it comes from an earlier sample and is reported second-hand here. The
sector test exists but swings from a reward-to-risk of 0.28 to 0.81 depending on how it is specified,
which is the signature of a result that is not robust.

## How this project relates to it

The rules above are drawn from this repository's own study,
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md). Section 6 collects
the dispersion and relative-value evidence with its honesty labels, and Section 9 places pairs trading
in the eligibility table, where it is one of only two strategies allowed even in a strictly memoryless
market, with the note that the evidence is thin. Section 4.3 explains why the size of the prize is
small for a group of correlated sector funds.

The engine in [implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md)
does not run a dispersion book. Its companion design note,
[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), does provide one
piece of machinery a dispersion book would need: the break-even identity for a short position, in
Section 6, because the short leg is the one that is easiest to mis-cost. Nothing in this repository
trades the gap today. A reader who wants the parts that exist will find them in those two documents,
and a reader who wants the parts that do not exist will not be misled into thinking they do.

## Where it goes wrong

- A correlation spike turns the book directional. In a crisis, prices that normally move apart fall
  together. The short leg does not deliver the gains the book was counting on, and the hedge that made
  the trade feel safe stops working at the worst moment.
- The gap can widen forever. The rule assumes a stretched relationship snaps back. If the two funds
  were never really equivalent, the gap is not an accident of supply but a permanent change, and the
  losing leg keeps losing.
- Borrowing has a price and a risk. To sell short you must borrow the fund, pay a fee for as long as
  the position is open, and accept that the lender can ask for it back, forcing you to close early.
- The costs are heavy relative to the prize. Four crossings per round trip plus a borrow fee is a
  large fraction of a gap that may only be a few percent wide.
- The test can fool itself. Choosing pairs because their relationship held in the past, then measuring
  the result on that same past, is the classic way a relative-value backtest looks far better than the
  trade ever was.
- The measured results are not robust. A reward-to-risk that moves from 0.28 to 0.81 with a change of
  specification is not a number to build a business on.

## Try it yourself

You need a spreadsheet and two funds that hold similar things. Any finance website will give monthly
prices.

1. Make a column for the month, a column for the price of fund A, and a column for the price of fund B.
2. Add a column for the gap: price A divided by price B, minus one.
3. Add a column for the average gap over the whole period so far.
4. Add a column for the typical swing: the standard deviation of the gap column so far.
5. Add a column for the count of typical swings: the gap minus its average, divided by the typical
   swing. This is the `z` value above.
6. Mark every row where the count is above two or below minus two. Those are the rows the rule would
   have opened a position.

What to notice: count how many of those marks were followed by the gap returning toward its average
within a few periods, and how many were followed by the gap carrying on in the same direction. On real
pairs you will find both, and the distribution is the whole question. Also notice how sensitive the
answer is to the look-back window: a shorter window makes swings look bigger and opens the trade more
often.

## Where this came from

- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's study of the question. Section 6 is the dispersion and relative-value evidence,
  Section 9 is the eligibility table, Sections 2.1 and 4.3 give the dispersion and correlation
  background, and Section 8.2 defines the dispersion measure used above.
- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Sections 5 to 6,
  where the short-side break-even identity lives and where a market-neutral dispersion book is named as
  potentially eligible.
- [Cross-Venue Structure and Arbitrage](../../../strategies/books/10_cross_venue_and_arbitrage.md),
  this repository's brief on how printed price gaps survive or fail once costs are priced.
- [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
  the brief on how fragile weight and correlation estimates are.
- The pairs figures and the cointegration figures above are labelled summary only in the source, and
  are repeated here with that label rather than as confirmed measurements.

## Words used in this tutorial

- dispersion: how far apart the returns of a group's members are from the group's average in one period.
- relative value: judging one asset against another rather than on its own, and trading the gap.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- borrow fee: the rent paid to the lender of something you have sold short.
- market neutral: holding a book whose gains do not depend on the market rising or falling.
- spread: the gap between the buy price and the sell price of the same thing at one moment.
- z-score: how many standard deviations a value sits from its average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
