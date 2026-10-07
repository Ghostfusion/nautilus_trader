# Contrarian reversion: buying what has just fallen

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Baskets of sector funds, the sectors that have performed worst over the recent past                                                                                                                                                 |
| How often it trades       | About once a month, when the worst sectors are reconsidered                                                                                                                                                                         |
| What you need             | A spreadsheet and a few years of sector returns                                                                                                                                                                                     |
| Where the rules come from | [Sector Regime Engine README](../../../implementation/sector-regime-engine/README.md), the contrarian overlay and the gate that admits it                                                                                           |
| The underlying research   | Hameed and Mian, [Industries and Stock Return Reversals](https://www.cambridge.org/core/journals/journal-of-financial-and-quantitative-analysis/article/abs/industries-and-stock-return-reversals/92B5101FBEA7B40717D46DB60913BF41) |
| How well it held up       | Weak: the reversal evidence is strongest for individual shares and is reported only at summary level here, and it does not transfer to sectors without an independent test, which has not been done                                 |
| Also appears in           | [Sector momentum](../../quantconnect/sector-momentum/README.md) in this collection, its exact opposite, and [Telling which kind of market you are in](../measuring-the-regime/README.md)                                            |

## The idea in one paragraph

Instead of buying the sectors that have done well, buy the ones that have done badly. Every month,
rank the sectors by how they performed over the recent past and put your money in the worst few. The
bet is that a fall was an overreaction, so the price comes back. For this to earn anything, the sectors
have to show a real tendency to reverse, which is a different thing from simply being unpredictable.
Even when such a tendency exists, at the level of whole sectors the profit is small and the cost of
trading every month eats most of it. The idea is the mirror image of the momentum rule already in this
collection, and the two cannot both be right at the same time.

## Why anyone believed it

A price falls for one of two reasons. Either the outlook genuinely got worse, or someone was forced to
sell. The second reason is the interesting one, because it has nothing to do with value. A fund facing
redemptions has to raise cash and sells whatever it can. An investor who has just seen a large loss
sells to stop the pain. At the end of a year, shares that have fallen are sold to realise a tax loss.
An index provider that rebalances its list dumps the shares leaving it, regardless of price.

If enough of those sellers appear at once, the price is pushed below what the business is worth, and
the person who buys from them is paid for providing the cash they needed. The forced seller keeps being
forced because their reasons are not about the price, so the pattern can persist. The Contrarian belief
is that this happens often enough to be worth harvesting, especially after a market-wide fall or in
volatile periods, when more investors are being pushed around.

It was also believed because it looks like the other side of a well-documented fact: over days and
weeks, prices tend to bounce after a sharp move. The mistake is to assume that fact is about whole
sectors. It is mostly about individual shares, and the two are not the same.

## An everyday comparison

A bakery reduces the price of everything left on the shelf in the last hour before closing. The bread
has not become bad; the baker simply needs the shelf empty. A customer who knows the shop, and knows
that day-old bread is fine, buys cheap and is better off for it. But if the customer assumes every
discount means a bargain, they will eventually buy bread that really was stale.

The point of the strategy is to be the customer who understands why the discount appeared, not the one
who buys anything marked down.

## The rules, step by step

1. Choose a set of sectors whose prices you can follow, such as the ten or eleven industry funds of one
   market.
2. First, test the data for reversal. Measure whether the sectors' relative ranks tend to reverse from
   one period to the next. If they do not, this strategy should not be run at all. Every major
   collection that describes a rule like this one gates it on that measurement.
3. Decide the measurement window in advance, such as the past month, and decide how many sectors to
   buy, such as the worst two or three.
4. At each decision date, rank the sectors by their return over that window, worst first.
5. Buy the worst two or three in equal amounts, one half or one third each.
6. Hold for one month, or whatever period you chose, without looking at the prices in between.
7. At the next decision date, rank again, sell anything that has left the worst group and buy whatever
   has entered.
8. Subtract the cost of every trade: the gap between the buying and selling price plus any commission,
   applied to everything bought and everything sold.

Two refinements matter. The measurement window should avoid being too short, because very short moves
are dominated by the mechanics of trading rather than a change in outlook. And the whole thing should
be decided on one stretch of history and then checked on data that was not used to decide it.

## The maths, with every symbol named

The bet pays off only if relative returns have negative memory:

```text
E[r_next | losers this period] > E[r_next]
```

- `E[...]` means "the expected value of", the long-run average.
- `r_next` is a sector's return in the next period.
- The condition says: the sectors that lost this period should, on average, do better than an average
  sector next period. Without that, the strategy has no expected edge before costs.

The size of the raw opportunity is the measured reversal, and the cost of taking it is:

```text
Cost = t * c
```

- `t` is the traded fraction. Buying a whole new set of sectors and selling the old ones counts as 2.0
  if everything is replaced, and less when holdings are kept.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying and
  selling price plus commission.
- The strategy earns something only when its monthly gross return exceeds `Cost`. With a monthly
  rebuild that replaces everything, `t` is about 2.0, so at `c = 0.0015` the monthly cost is about
  0.003, or 0.30 percent.

For the sector case, the relevant question is not whether individual shares reverse but whether sector
ranks reverse:

```text
IC_contrarian = Corr(Rank_t, Rank_t+1)
```

- `IC_contrarian` is the rank information coefficient; a negative value is what a contrarian rule
  needs.
- `Rank_t` is the sectors sorted by their return in period `t`.
- The same measurement that a momentum rule reads as positive, a contrarian rule reads as negative; one
  number, two opposite strategies.

## A worked example

Six months, each holding the single worst-performing sector of the previous month. The returns below
are invented but of the size sector moves actually take.

| Month | Sector bought (worst last month) | Its return next month |
| ----- | -------------------------------- | --------------------- |
| 1     | Energy                           | +0.8 percent          |
| 2     | Utilities                        | -0.4 percent          |
| 3     | Health care                      | +0.9 percent          |
| 4     | Technology                       | +0.2 percent          |
| 5     | Financials                       | +0.6 percent          |
| 6     | Energy                           | +0.5 percent          |

The average of the six gross returns is `(0.8 - 0.4 + 0.9 + 0.2 + 0.6 + 0.5) / 6 = 0.4333` percent a
month. That is the raw signal, before any cost. In this example the worst sector changed almost every
month, so the whole holding had to be sold and replaced each time; at 15 basis points a trade, where
one basis point is 0.01 percent, a full replacement costs about `2 * 0.0015 = 0.30` percent a month.

```text
Gross per month            0.4333 percent
Cost per month             0.3000 percent
Net per month              0.1333 percent
```

So the cost consumes about two thirds of the raw signal, leaving about 0.13 percent a month, or roughly
1.6 percent a year before tax and before any other friction. That is not a large prize, and it rests
entirely on whether a +0.43 percent monthly reversal is real at the level of sectors, which the study
reports as unconfirmed.

Two things are worth noticing. First, four of the six months were positive, which is why the average
looks decent, but a different sample could easily have two or three large negative months and flip the
sign. Second, if the cost per trade were 20 basis points instead of 15, the cost would be 0.40 percent
and the net would be 0.03 percent a month, essentially nothing. Small differences in a cost assumption
decide whether this strategy exists.

## What the research actually found

The project's own study is careful about three levels that are not the same thing: whether a sector's
own return reverses, whether relative sector ranks reverse, and whether individual shares reverse. A
result at one level does not carry to another, and the study makes it a rule never to transfer a
share-level result to sectors without an independent sector-level test.

The reversal evidence it cites is at the share level and is labelled summary only, meaning it was
reported with a citation but the original paper was not reachable to confirm the magnitudes. Hameed and
Mian found pervasive monthly reversals within industries, stronger after market declines and in
volatile periods; an industry-adjusted share reversal of about 0.53 percent a month with a
reward-to-risk ratio near 0.74 was reported; and long-horizon reversals in the 1.4 to 2.5 percent a
month range were reported. None of those is a tradable sector rotation estimate, and the study says so.

Two consequences follow. A share-level reversal is partly noise specific to the firm, and averaging
shares into a sector removes much of that noise, so the effect at the sector level should be expected
to be smaller, not larger. And the cost has to be beaten: at 10 to 20 basis points a trade, a monthly
sector rotation consumes much of a 0.5 percent monthly gross signal.

The evidence also cuts the other way for the strategy it mirrors. The pre-registered test of four
popular rotation states on 2010 to 2026 sector data found none of 24 cells confirmed, and the one effect
visible in the first decade shrank to nothing in the data held back. Its authors note that one-month
relative sector returns tend to reverse, which is why the academic momentum rules use six- to
twelve-month windows and skip the most recent month. So the reversal tendency is real enough to spoil
momentum at very short horizons, but the direct tests of trading it at sector level have not confirmed
a usable edge.

## How this project relates to it

This is the mirror image of the momentum rule already in this collection,
[sector momentum](../../quantconnect/sector-momentum/README.md), which buys the three sectors that did
best over twelve months. Both read the same measurement and demand opposite signs: momentum needs
sector ranks to persist, contrarian needs them to reverse, and
only one can hold at a time. The momentum tutorial covers the twelve-month rule; this one covers the
short-horizon reversal it relies on not happening.

In [src/engine.js](../../../implementation/sector-regime-engine/src/engine.js) the contrarian overlay
is admitted only when the primary tests admit negative dependence, and the eligibility function shows
"Contrarian overlay" as eligible in that world and blocked otherwise. The
[user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) calls this world
NEGATIVE_DEPENDENCE in section 3 and stresses the warning that makes the whole subject hard: "no
pattern" and "a pattern that flips back" are not the same thing, so a coin toss is no better to bet
against than to bet on. Section 5 of
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md) is the full
treatment, including the three levels and the cost threshold.

## Where it goes wrong

- It is not the same as an unpredictable market. A coin toss does not reverse; it just does not carry
  on. A contrarian rule needs genuine negative dependence, and running one in a memoryless market loses
  money through costs, exactly as momentum would.
- Share-level evidence does not become sector-level evidence. Averaging shares into a sector removes
  much of the reversal, and the study forbids assuming that a result travels between the two levels.
- Costs decide the answer. A monthly rebuild pays both sides of every position, and at 10 to 20 basis
  points a trade that consumes most of a 0.5 percent monthly signal.
- The reversal is conditional. The strongest results are after market declines and in volatile
  periods, so an unconditional average can hide a pattern that only appears in states you cannot always
  identify in advance.
- Crowding. Once a rule is published and easy to trade, the buying happens sooner, the bounce is
  smaller, and the latecomers pay for it.
- The horizon is fragile. At very short horizons the effect is entangled with the mechanics of trading
  rather than with a change in outlook, and skipping even one period can remove it entirely.
- It loses when momentum is right. In a trending market the recently strongest sectors keep leading,
  and a contrarian rule is systematically buying the ones left behind.

## Try it yourself

You need a spreadsheet and monthly returns for three or more sectors, from any public source.

1. Columns: `Month`, one column per sector, and a column `Worst sector`.
2. In the `Worst sector` column, each month, name the sector with the lowest return that month.
3. Add a column `Next month return` holding that sector's return in the following month.
4. Average the `Next month return` column over as many months as you have. That is the gross signal.
5. Add a `Cost` column holding 0.30 percent for every month in which the worst sector changed from the
   previous month, and zero otherwise.
6. Subtract the cost and average again, and divide both averages by their spread, not just look at the
   raw mean.

What to notice: the average gross signal is small and the cost is not, so the net figure is decided by
how often the pick changed. If the pick changed most months, the cost column is large. If your gross
signal looks strongly positive, check how many months are doing the work; one or two months usually
carry the whole effect, which is not the same as a durable pattern.

## Where this came from

- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), section 5: the
  three levels of reversal, the summary-only evidence on Hameed and Mian, and the cost threshold.
- Hameed and Mian, [Industries and Stock Return Reversals](https://www.cambridge.org/core/journals/journal-of-financial-and-quantitative-analysis/article/abs/industries-and-stock-return-reversals/92B5101FBEA7B40717D46DB60913BF41),
  the share-level reversal evidence, cited in the project study as summary only.
- Quant Data, [Does sector momentum persist?](https://quantdata.uk/research/does-sector-momentum-persist),
  the pre-registered test of four short-horizon rotation states and its held-out window.
- [Sector momentum](../../quantconnect/sector-momentum/README.md), the opposite rule in this collection,
  and its twelve-month formation window.
- [Sector Regime Engine user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) and
  [src/engine.js](../../../implementation/sector-regime-engine/src/engine.js), where the contrarian
  overlay is gated on negative dependence.

## Words used in this tutorial

- basis point: one hundredth of one percent, so 15 basis points is 0.15 percent.
- contrarian: betting against whatever has recently been doing well, on the theory that it will reverse.
- correlation: a number between -1 and 1 describing whether two things move together; near 1 means they
  move almost identically, near 0 means they are unrelated.
- momentum: the tendency of something that has been rising to keep rising for a while.
- negative dependence: a tendency for whatever has just done well to do badly next, which is what a
  contrarian rule needs.
- rank: a sector's position in an ordered list, worst first here.
- reversion: a return toward an earlier level after moving away from it.
- turnover: how much trading happened, usually expressed as a proportion of the pot.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
