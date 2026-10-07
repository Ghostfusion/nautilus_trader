# Attribution of results: splitting one trading result into the decisions that produced it

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                       |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: it is the arithmetic that splits the result of a strategy that does trade                                                                                                                                |
| How often it trades       | As often as the strategy it describes; the split is computed once for each reported period                                                                                                                                  |
| What you need             | Nothing but this page; a spreadsheet makes the exercise easier                                                                                                                                                              |
| Where the rules come from | [Entry/exit price engine: design, section 9](../../../strategies/entry_exit_engine_design.md), and [entry/exit price engine: implementation, sections 8.2 and 8.3](../../../strategies/entry_exit_engine_implementation.md) |
| The underlying research   | [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this repository's own study, which supplies the comparison books the split is measured against                                         |
| How well it held up       | Strong as a definition: the split is an accounting identity rather than a measured effect, it is additive by construction, and the engine requires the leftover to stay at floating-point noise and tests that identity     |
| Also appears in           | [Scoring a strategy on its evidence, not its profit](../scoring-evidence-not-profit/README.md) and [Cost, and the break-even price](../cost-basis-and-breakeven/README.md)                                                  |

## The idea in one paragraph

A strategy's result is one number: the account held some amount at the start of the period and some
amount at the end. That number is the joint product of several separate decisions - which things to
hold, how often to reset the holdings, whether to make a bet on the direction of the market on top,
and how much the buying and selling cost. Attribution means splitting the one number into a short
list of parts, one part per decision, where the parts add back up to the number. The engine here
does that by running several copies of the same portfolio side by side, each copy differing from the
next by exactly one decision, and reporting the gap between neighbouring copies as that decision's
part. The parts are recorded as logarithms, which turn multiplying into adding, so they add exactly,
and whatever is left over is printed rather than quietly absorbed.

## Why anyone believed it

A profit number answers a question that nobody with money can act on. "The account made nine
percent" does not say whether the nine percent came from choosing the right things to own, from a
good year for the market as a whole, from trading in and out at lucky moments, or from a cost that
has simply not been subtracted yet. Anyone paying for the result - a client, a committee, the person
running it - wants to know which decision to keep and which to change, and a single number cannot
say.

The belief behind attribution is that a result can be taken apart honestly, the way a shopkeeper's
accounts are taken apart, by changing one decision at a time and watching what moves. The belief is
sound, and it is exactly as sound as the comparison it rests on: a part is only the effect of one
decision if the two books being compared really do differ in only that one way.

## An everyday comparison

A school wants to know why its exam results rose by five points. Three explanations arrive at once:
the children worked harder, the new textbook is better, or the weakest class left for another
school. The average mark cannot separate them. So the school runs shadow classes that differ in one
thing only: the same children, the same hours, the same teacher, but one class gets the new textbook
and another keeps the old one. The gap between those two classes is the textbook's contribution.
Attribution is that experiment, run on an account instead of a classroom: several books in parallel,
neighbouring books differing by one decision, and the gap between neighbours reported as that
decision's part.

## The rules, step by step

1. Fix the period. The split explains what happened over one stretch of time - a month, a quarter, a
   year - and says nothing about any other stretch.
2. Write down the account's value at the start and at the end of that period, in one currency.
   Everything below is a growth rate measured from the start.
3. Run the first comparison book: a **drifting basket**. It holds the same list of things from the
   first day to the last and never touches them, so their weights drift as their prices move.
4. Run the second book: a **rebalanced** version of the same list. At a fixed rhythm it is reset
   back to equal weights.
5. Run the third book: the book actually run, which is the second book plus whatever directional bet
   the strategy adds on top. This engine calls that addition an **overlay**.
6. Run the fourth book: the third book with the trading and holding costs paid.
7. Report the four gaps as the four parts. The first part is the drifting basket's own growth, from
   the start to the end. The second is the rebalanced book's end value divided by the drifting
   book's end value. The third is the actual book's end divided by the rebalanced book's end. The
   fourth is the cost, which is negative.
8. Add the parts and compare the total with the account's own growth. The difference, printed as the
   residual, must be zero apart from the rounding of computer arithmetic.
9. Report every part in the same units, and never call a part by the name of a decision that was not
   isolated by that step.
10. When the strategy later gains a new decision - a stop that exits a position early, a rule that
    clips an order too large to fill - give it its own part rather than letting it hide inside the
    total.

Note one subtlety the engine keeps explicit. The rebalancing part can be measured against two
different baselines: against the drifting basket, which is what a holder who did nothing would have
had, or against the weighted average of the individual things' growth rates. The engine reports both
and labels them type B and type A, because they answer slightly different questions and mixing them
would make the same number mean two things.

## The maths, with every symbol named

Growth is measured with natural logarithms. A logarithm answers the question "what power of e gives
this number", where e is a fixed number about 2.718. Its only purpose here is that it turns
multiplication into addition: if a price doubles and then halves, the two moves are plus 0.693 and
minus 0.693 in logarithms and add to zero, whereas the percentages 100 and -50 do not add to zero.
So the log of 1.10 is 0.0953, which says the same thing as a gain of 10 percent in a form that can be
added.

The four parts, written for a period in which the books start at 1:

```text
part_exposure  = ln(B_end)
part_rebalance = ln(S_end) - ln(B_end)
part_overlay   = ln(A_end) - ln(S_end)
part_costs     = ln(N_end) - ln(A_end)
```

- `ln` is the natural logarithm, defined above.
- `B_end` is the end value of the drifting basket, starting from 1.
- `S_end` is the end value of the rebalanced book, starting from 1.
- `A_end` is the end value of the book actually run, before costs, starting from 1.
- `N_end` is the end value of that book after costs, starting from 1.

Adding the four parts telescopes: the intermediate values cancel, leaving the account's own log
growth. That is the identity the engine checks:

```text
part_exposure + part_rebalance + part_overlay + part_costs = ln(N_end)
residual = (sum of the parts) - ln(N_end)
```

- `residual` is what is left when the parts are added up and compared with the account's own
  logarithm. It must be zero apart from the rounding error of computer arithmetic, which is a
  number around 0.000000000000001; a residual larger than that means the accounting itself is
  wrong, not the strategy.

To turn a log total back into the percentage a reader recognises:

```text
growth = e ^ (ln(N_end)) - 1
```

- `e ^ x` means e multiplied by itself the number of times the exponent describes, the inverse of
  the logarithm; `e ^ 0.0953` is 1.10, the 10 percent growth written as a factor.

Costs are kept apart for a reason worth stating. The engine records the trading cost and the holding
cost as their own negative logarithms, one entry per rebalance, so a part can be read on its own
rather than inferred from the gap between two books.

## A worked example

Six months of one invented account. The figures are in **log points**, the unit of the logarithm, and
a log point of 0.01 is about 1 percent for movements this small. Each row adds across to the month's
net figure.

| Month | Exposure | Rebalancing | Overlay | Costs   | Net     |
| ----- | -------- | ----------- | ------- | ------- | ------- |
| 1     | +0.0120  | +0.0010     | -0.0020 | -0.0015 | +0.0095 |
| 2     | +0.0080  | +0.0005     | +0.0030 | -0.0015 | +0.0100 |
| 3     | -0.0150  | +0.0020     | -0.0040 | -0.0020 | -0.0190 |
| 4     | +0.0200  | +0.0015     | -0.0010 | -0.0015 | +0.0190 |
| 5     | +0.0040  | +0.0005     | -0.0025 | -0.0015 | +0.0005 |
| 6     | +0.0060  | +0.0010     | -0.0015 | -0.0015 | +0.0040 |
| Total | +0.0350  | +0.0065     | -0.0080 | -0.0095 | +0.0240 |

Every row adds: month 1 is 0.0120 + 0.0010 - 0.0020 - 0.0015 = 0.0095, and so on down the table. The
total row is the column sum, and the check is 0.0350 + 0.0065 - 0.0080 - 0.0095 = 0.0240.

Convert the total to the account's growth with the formula above:

```text
growth = e ^ 0.0240 - 1 = 1.0243 - 1 = 0.0243, that is 2.43 percent
```

An account of 100,000.00 dollars therefore ends at 102,430.00 dollars. Adding the six monthly net
figures as plain percentages would have said 100,000.00 times (1 + 0.0240) = 102,400.00, which is
about 30 dollars low; that gap is compounding, and it is the reason the engine works in logarithms.

Now the same six months as one result attributed to causes:

| Cause                                            | Account it is measured on                   | Contribution |
| ------------------------------------------------ | ------------------------------------------- | ------------ |
| The things held, and what the market did to them | The drifting basket                         | +0.0350      |
| Resetting the weights each month                 | Rebalanced book against the drifting basket | +0.0065      |
| The overlay's timing                             | Actual book against the rebalanced book     | -0.0080      |
| Results before costs                             |                                             | +0.0335      |
| Costs of trading and holding                     | Actual book against its after-cost version  | -0.0095      |
| Result                                           |                                             | +0.0240      |

Read plainly: the account grew 2.43 percent; the things it held would have produced 3.50 log points
on their own; resetting the weights added 0.65; the overlay took away 0.80; and costs took away
0.95. The overlay was the part that lost money, and no single headline number would have shown it.

## What the research actually found

Attribution is measurement, not a strategy, so the "research finding" it depends on is about how
much of a result is real. Two numbers from this repository's own study make the point.

The study of sector rotation tested 1,022 different rotation rules on ten sectors over 1948 to 2018.
The average rule returned 0.86 percent a month against 0.89 percent for simply holding the market,
and only 132 of the 1,022 rules beat buy and hold. The authors concluded the apparent gains came
from trying so many rules rather than from the rules. A portfolio built from a rotation rule reports
one number, and that number is a selection, not an estimate. Splitting it into parts does not fix the
selection problem - the parts of a lucky result are still lucky - but it does stop the luck being
attributed to a decision that had nothing to do with it.

The second finding is why the parts must be reported with their distribution, not only their total.
The design notes that a stop, which exits a position after it has fallen a set distance, raises the
observed win rate by construction and truncates both tails of the distribution of outcomes. For this
reason the risk-layer part is explicitly not allowed to be called alpha, where alpha means the part
of the return that reflects skill rather than market exposure; it must be printed next to the win
rate and the sizes of the best and worst trades.

## How this project relates to it

The engine that performs this split is the
[sector regime engine's engine.js](../../../implementation/sector-regime-engine/src/engine.js). Its
`backtest` function builds four value paths - `basket`, `struct`, `actual` and `net` - and its
`attribution` function turns them into the five lines above, returning the parts, their sum, the
account's own log growth and the residual. A reader can see the identity rather than take it on
trust.

The design that extends the list is
[strategies/entry_exit_engine_design.md](../../../strategies/entry_exit_engine_design.md), section 9.
It keeps the existing lines and appends a split of the cost into spread and fees, slippage and market
impact; the cost of the delay between deciding and executing; the risk layer's own effect; and the
effect of clipping an order that was too large to place. Section 8.2 of
[the implementation document](../../../strategies/entry_exit_engine_implementation.md) says how the
extra book is run in parallel: one more value path, so that the risk layer's part is the difference
of two logarithms and nothing else. The design is not yet implemented, and its own first page says
so.

The [user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) states what the
engine does and does not do, which is worth reading before the code.

## Where it goes wrong

- The comparison decides the answer. If two books differ by two decisions, the gap between them is
  the effect of both together, and naming it after one of them is simply wrong.
- The residual is a defect detector, not a tolerance. The engine requires the residual to sit at
  floating-point noise; a residual that does not is telling you that a part is missing, double
  counted, or measured against the wrong book.
- A part that is positive in one period can be negative in the next. Rebalancing helps in a market
  that chops sideways and hurts in a market that trends; attribution describes the period it was run
  on, not the next one.
- A behavioural part is not skill. A stop changes the shape of the outcomes, raising the recorded
  win rate and cutting both the best and the worst trades, so a part that looks like improvement can
  be a change in the accounting rather than a gain.
- Attribution does not repair a selection. If the rule was chosen after seeing many other rules, the
  parts of its good year are as selected as the total was.
- Costs spread over several parts can be counted twice. The parts must stay additive and the
  residual must be printed, or the same fee quietly appears in two lines.

## Try it yourself

You need nothing but a spreadsheet and six months of made-up or real prices for four things you
already know: for instance four large company funds.

1. Column A: the month number, 1 to 6.
2. Columns B to E: the monthly return of each of the four funds, in percent.
3. Column F, the drifting basket: start at 100.00 in the first row and multiply each month by the
   average of that month's four returns plus one. This is the holder who never touches anything.
4. Column G, the rebalanced book: the same start, but each month multiply by the average of the four
   returns and ignore that the weights have drifted. This is the holder who resets every month.
5. Column H, the costs: each month subtract 0.05 percent of the book, standing in for a monthly
   rebalance at ten basis points of the amount traded, where a basis point is one hundredth of one
   percent.
6. Column I: the account, which is column G minus column H.
7. Finally, compute the logarithms of the end values of F, G and I, take the three differences, and
   check that they add up to the logarithm of I with nothing left over.

What to notice: the rebalanced book and the drifting basket differ even though they hold the same
four funds, because the drifting basket lets the winner grow into a bigger share of the money. In
some six-month stretches that helps and in others it hurts. When you change the number of funds, or
the cost per trade, the rebalancing part changes sign - which is why a single headline number cannot
tell you whether resetting the weights was worth doing.

## Where this came from

- [Entry/exit price engine: design](../../../strategies/entry_exit_engine_design.md), section 9, the
  list of attribution lines and the two properties required of them: the residual stays at
  floating-point noise, and the risk-layer line is never called alpha.
- [Entry/exit price engine: implementation](../../../strategies/entry_exit_engine_implementation.md),
  sections 8.2 and 8.3, the parallel books and the order in which the lines are reported.
- The [engine code](../../../implementation/sector-regime-engine/src/engine.js), the working
  `backtest` and `attribution` functions that produce the four paths and the five lines.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), section 3.3,
  the 1,022-rule experiment: 0.86 percent a month on average against 0.89 percent for buy and hold,
  and 132 of 1,022 rules beating it.
- [Sector Regime Engine: user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md),
  what the engine does and, more importantly, what it refuses to do.

## Words used in this tutorial

- attribution: splitting a single result into the parts that produced it, one part per decision.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- book: one complete portfolio run from start to finish, including the trades it would have made.
- drifting basket: the same holdings left alone, so their shares of the money change as prices move.
- logarithm: a way of writing growth so that growth rates add rather than multiply.
- overlay: an extra bet placed on top of the holdings, here a bet on market direction.
- rebalancing: resetting the holdings back to their intended shares by buying and selling.
- residual: what is left when all the parts are added up and compared with the whole result.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
