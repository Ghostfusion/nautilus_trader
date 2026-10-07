# Cost, and the break-even price: measuring a trading result in hundredths of a percent

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                           |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: it measures what a strategy that does trade pays, and the cost level at which its measured edge disappears                                                                                                                   |
| How often it trades       | Once per reported frame, which is a week, a month or a quarter, not once per individual trade                                                                                                                                                   |
| What you need             | A spreadsheet, and one currency throughout                                                                                                                                                                                                      |
| Where the rules come from | [Breakeven cost](../../../crates/analysis/src/statistics/breakeven_cost.rs) and [cost in basis points](../../../crates/analysis/src/statistics/cost_basis_points.rs), the two measurements the analyzer registers together                      |
| The underlying research   | [Entry/exit price engine: design, section 6](../../../strategies/entry_exit_engine_design.md), which states the break-even identities, and [the measurement work](../../../strategies/books2/implementation_plan.md) the repository is building |
| How well it held up       | Strong as a definition: both figures are arithmetic identities checked by unit tests against exact expected values; the size of the edge they measure is a different question and is not graded here                                            |
| Also appears in           | [Market impact and its labels](../market-impact-and-labels/README.md) and [What costs a trade](../../foundations/06_costs-fees-and-taxes.md)                                                                                                    |

## The idea in one paragraph

Every trade pays something, and the something is a fraction of the amount traded: the gap between the
price at which you can buy and the price at which you can sell, the commission, and the cost of your
own order moving the price. That fraction is tiny per trade, which is why it is quoted in hundredths
of a percent, called basis points. A break-even cost is the answer to a sharper question: how large
could that fraction have been before this strategy's measured advantage disappeared? If the strategy
earned 12 basis points of gross profit for every 100,000 traded and paid 5, then the break-even cost
is 12 and the strategy has 7 basis points of room; if it had paid 12, it would have made nothing at
all. Reporting the break-even is more honest than reporting a return, because it says how much the
result depended on trading being cheap instead of hiding that dependence inside one number.

## Why anyone believed it

Costs are the part of a backtest that is easiest to leave out and hardest to see. A rule that trades
every month pays the cost every month, and a rule that trades rarely pays it rarely, so two rules with
the same apparent return can have very different real ones. The people who care first are the ones who
have to fund the trading: a cost of ten basis points per round trip, repeated monthly, is roughly 2.4
percent a year, which is the same order of size as the advantage most published rules claim.

The belief behind reporting break-even is that the honest question is not "what did it return" but
"how much would the cost have to rise before this stops working". That question has a definite answer
in arithmetic, it does not need a forecast, and it turns a fragile result into an obviously fragile
one. What it cannot do is prove that the measured advantage was real in the first place.

## An everyday comparison

A market trader buys apples for 50 pence and sells them for 55 pence, and comes home pleased with
5 pence of profit each. Then the numbers arrive: the pitch costs 40 pence a day, the bus 60 pence, and
the bags 5 pence. The 5 pence per apple is a margin, not a profit. What the trader actually wants to
know is the price at which the apple must sell for the day to break even, and that number can be
worked out exactly: the stall, the bus and the bags all have to be covered before any apple is
profit. Reporting "the break-even apple price is 54 pence" is more useful than reporting "I made
twelve pounds last Saturday", because the first tells you how much room there is and the second only
tells you what happened once.

## The rules, step by step

1. Fix the frame: one stretch of time, one currency. Costs and profits must be counted in the same
   currency or the division at the end is meaningless.
2. Add up the turnover. Turnover is the money value of everything bought and sold in the frame,
   counting each fill separately, so buying 10,000 and later selling 10,000 is 20,000 of turnover.
3. Add up the commission paid in the frame. This is the fee the broker charges per trade, and in this
   project it is the whole recorded cost of the frame.
4. Add up the profit before costs, called the gross profit. It is what the trades made with the
   commission added back.
5. Divide the commission by the turnover and multiply by 10,000: that is the cost in basis points.
6. Divide the gross profit by the turnover and multiply by 10,000: that is the break-even cost in
   basis points.
7. Compare the two. If the break-even is below the cost, the frame lost money to its costs whatever
   its gross profit said. If the break-even is above the cost, the difference is what was left.
8. Report both, side by side. The two are measured on the same basis on purpose, so one can be read
   against the other, and neither is the cost as a fraction of the money in the account, which is a
   different and larger number.
9. Never present a break-even without saying how uncertain it is. The design document states plainly
   that a result above the cost it pays is not evidence of an advantage until the estimate carries its
   own uncertainty.

## The maths, with every symbol named

Both figures are the same shape: one money amount divided by another, scaled to basis points.

```text
cost_bp = commission / turnover * 10,000
breakeven_bp = gross_profit / turnover * 10,000
```

- `cost_bp` is the all-in cost of the frame in basis points of turnover. A basis point is one
  hundredth of one percent, so 10,000 basis points make 100 percent, and 5 basis points is 0.05
  percent of the amount traded.
- `breakeven_bp` is the cost rate the frame could have paid and still broken even. It is the same as
  the advantage the frame earned per unit traded.
- `commission` is the fee paid on the frame's trades, in the frame's currency.
- `turnover` is the money value of every fill in the frame, in the same currency.
- `gross_profit` is the frame's profit with the commission added back, so it is the profit before
  costs rather than after.
- The factor 10,000 is what converts a plain fraction into basis points. Without it,
  `100 / 10,000` would be 0.01, meaning 1 percent; multiplied by 10,000 it becomes 100 basis points.

The relationship between the three is fixed:

```text
breakeven_bp - cost_bp = net_profit / turnover * 10,000
```

- `net_profit` is the profit after the commission has been subtracted, so `gross_profit - commission`.
- The right-hand side is the frame's own result per unit traded. When it is zero the break-even and
  the cost are equal, which is exactly what breaking even means.

There is a second, price-level form of the same idea for one position, where the question is what the
price has to do rather than what the cost rate has to be:

```text
P_break_even = P_entry + (C_entry + C_exit) / Q
```

- `P_break_even` is the price at which a long position, meaning one that gains when the price rises,
  just covers its costs.
- `P_entry` is the price at which the position was bought.
- `C_entry` and `C_exit` are the costs of buying and of selling, in money.
- `Q` is the quantity held, in units.
- Dividing the costs by the quantity turns a total cost into a cost per unit, which is the amount the
  price has to rise by.

## A worked example

An invented account starting at 100,000.00 dollars, measured month by month for six months. All
figures are in that one currency; the two rates are in basis points of that month's turnover.

| Month | Turnover | Gross profit | Commission | Break-even | Cost | Net |
| ----- | -------- | ------------ | ---------- | ---------- | ---- | --- |
| 1     | 40,000   | +120         | 20         | 30         | 5    | 25  |
| 2     | 40,000   | +80          | 20         | 20         | 5    | 15  |
| 3     | 60,000   | -60          | 30         | -10        | 5    | -15 |
| 4     | 40,000   | +160         | 20         | 40         | 5    | 35  |
| 5     | 80,000   | +40          | 40         | 5          | 5    | 0   |
| 6     | 40,000   | +20          | 20         | 5          | 5    | 0   |
| Total | 300,000  | +360         | 150        | 12         | 5    | 7   |

Take month 1 row by row. Turnover is 40,000. Gross profit is 120, so the break-even is
120 / 40,000 * 10,000 = 30 basis points. Commission is 20, so the cost is
20 / 40,000 * 10,000 = 5 basis points. The net is 120 - 20 = 100, and
100 / 40,000 * 10,000 = 25 basis points, and indeed 30 - 5 = 25.

Month 3 is the one that goes the other way: gross profit is -60, so the break-even is
-60 / 60,000 * 10,000 = -10 basis points. A negative break-even means the frame traded at a loss
before any costs at all, which is a different complaint from paying too much.

Now the whole frame. Turnover totals 300,000, gross profit 360, commission 150, so the net profit is 210:

```text
break-even = 360 / 300,000 * 10,000 = 12 basis points
cost       = 150 / 300,000 * 10,000 = 5 basis points
net        = 210 / 300,000 * 10,000 = 7 basis points
```

So the strategy earned 12 basis points of advantage per unit traded and paid 5, leaving 7. The
statement "the break-even cost is 12 basis points" is the useful one: it says the cost could have
been more than twice what it was before the advantage vanished. Had the cost been 12 basis points,
the net rate would have been zero, and the 210 dollars of profit would not have existed.

Two honest footnotes. The net of 210 dollars on a starting value of 100,000.00 is 0.21 percent over
six months, so the cost of 5 basis points of turnover is 150 dollars, which is 0.15 percent of the
account, or 15 basis points of the account, over the same six months. Those are two different ratios
of the same 150 dollars, and the implementation notes say a report must never present them as one row.
And the price-level form, for a single position: buy 1,000 units at 100.00, paying 25.00 to buy and
25.00 to sell, and the break-even price is 100.00 + (25.00 + 25.00) / 1,000 = 100.05. The price has
to rise 0.05, which is 5 basis points of 100.00, matching the 50.00 of cost over the 100,000.00 of
turnover.

## What the research actually found

Cost models are not one number. The microstructure brief in this repository reports a model fitted for
large companies' shares in which the cost of completing a trade is 1.25 times the spread plus 0.40
times a volatility and a size term's square root, while government bonds in the same study need an
entirely different shape, 3.00 times the volatility and a size term's fourth root. The brief's point
is that a cost figure belongs to a kind of thing and a market, not to trading in general: the same
trade costs different amounts in two buckets, and a single assumed cost will be wrong in one of them.

The other finding that matters here is about where a result actually breaks. The brief on
overfitting and research integrity records a betting study whose two published returns of 17.29 and
28.82 percent fell to minus 7.36 and minus 6.31 percent once one erroneous data row was fixed, and
records that after the correction one surviving strategy at 12.44 percent earned nothing over three
further years. The point for this tutorial is the arithmetic of that collapse: costs and errors are
not a small deduction from a large number when the large number is small.

This repository's own accepted behaviour is modest and exact. The measurement plan states that a
backtest on a venue with zero commission must print a cost of 0.00 basis points and identical gross
and net returns, and that the same run with a fee must print net below gross with the gap equal to
the commission over the frame's starting equity, while the printed cost row equals the commission
over the frame's turnover. Those two ratios differ by construction, which is why the plan forbids
presenting them as one.

## How this project relates to it

The two measurements are the
[break-even cost](../../../crates/analysis/src/statistics/breakeven_cost.rs) and the
[cost in basis points](../../../crates/analysis/src/statistics/cost_basis_points.rs). Each declares
its own definition: basis points as the unit, `Maximize` as the direction of the break-even because a
higher one is a more robust result, `Minimize` as the direction of the cost, and the account as the
stage, because both are read from the ledger rather than from a forecast. Both are computed from the
same period frame, so they cannot be accidentally measured over different stretches of time.

The unit itself is defined in the
[metric declarations](../../../crates/analysis/src/metric.rs): basis points are the value's unit, the
fixed factor is 10,000 per unit, and the unit's own description says that an all-in cost of 5 means
five basis points. Because the unit is declared rather than written into a title, a reader can tell
what a row means without guessing.

The unit tests beside the break-even measurement pin the arithmetic exactly: a frame with a net
profit of 80, a commission of 20 and a turnover of 10,000 produces a break-even of 100 basis points,
a frame whose gross profit is -100 produces -100 basis points, a frame with no turnover produces no
value at all, and scaling every amount in the frame by five leaves the rate unchanged. That last test
is the important one: it says the figure is a rate per unit traded, not a size.

The design that makes the figure usable at the level of a single order is
[strategies/entry_exit_engine_design.md](../../../strategies/entry_exit_engine_design.md), section 6.
Its break-even identities convert a cost in money into a price the thing has to reach, for a long
position and for a short one, and its own note says the figure is the number a cost study reads a
claimed edge against. The plan that carries this into the report is
[strategies/books2/implementation_plan.md](../../../strategies/books2/implementation_plan.md), whose
first work item is that the default report render the cost row at all.

## Where it goes wrong

- Zero turnover has no rate. A frame that traded nothing has no cost per unit traded, and the
  measurement returns nothing rather than zero, because a rate that was never measured is not a rate
  of zero.
- Different currencies cannot be divided. If the commission is in one currency and the turnover in
  another, the measurement returns nothing rather than guessing an exchange rate.
- The cost per unit traded and the cost as a share of the account are different numbers. The first is
  the commission over the turnover; the second is the commission over the money in the account. Both
  are useful and neither can stand in for the other.
- A positive break-even is not proof of an advantage. It says the frame earned something before
  costs. Whether that something was a pattern rather than luck is a question for the evidence
  measurements, and the design document says a break-even above the cost paid is not evidence until
  the estimate carries its own uncertainty.
- The frame's cost row may only count the commission. The spread, the cost of waiting and the cost of
  the order moving the price are separate effects; the implemented cost row counts the recorded
  commission, and the wider cost model that adds the others is a design, not yet shipped.
- The measured edge can be an artefact of the sample. A break-even computed on a good stretch of data
  is a statement about that stretch, and the brief on replication shows how a single bad row can turn
  two large positive results into losses.

## Try it yourself

A spreadsheet and nothing else. The exercise is to find the cost level at which an invented strategy
stops working.

1. Build six rows, one per month, and five columns: turnover, gross profit, commission, break-even in
   basis points, and cost in basis points.
2. In the break-even column put the formula `= gross profit / turnover * 10000`, and in the cost
   column `= commission / turnover * 10000`. Format both to two decimal places.
3. Fill in your own numbers: turnover of 30,000 to 90,000 a month, gross profit generally positive
   but negative in one month, and a commission of exactly 0.05 percent of the turnover, which is
   5 basis points.
4. Add a total row and recompute both rates from the totals rather than averaging the monthly rates.
   Notice that the two answers differ, because a month that traded a lot carries more weight in the
   total than a quiet month does.
5. Now change the commission column to 0.12 percent of turnover, which is 12 basis points, and watch
   the total row: the net rate should land on zero while the gross profit has not moved at all.
6. Finally, write the two break-even prices for one position: buy 500 units at 40.00, cost 10.00 each
   way. The break-even is 40.00 + 20.00 / 500 = 40.04.

What to notice: the gross profit is identical no matter what you put in the commission column, so a
report that shows only the gross number is showing you a figure that cannot change when the costs do.
The break-even is the number that moves when the world gets more expensive, and it moves before the
reported profit does.

## Where this came from

- [The break-even cost](../../../crates/analysis/src/statistics/breakeven_cost.rs), the definition,
  the direction `Maximize`, and the unit tests: net 80 with commission 20 over a turnover of 10,000
  gives 100 basis points, a loss of 100 gives -100, no turnover gives nothing, and scaling every
  amount leaves the rate unchanged.
- [The cost in basis points](../../../crates/analysis/src/statistics/cost_basis_points.rs), the same
  shape with the commission in the numerator and the direction `Minimize`.
- [The metric declarations](../../../crates/analysis/src/metric.rs), where basis points are defined as
  one ten-thousandth of the ratio being measured and the factor of 10,000 is a named constant.
- [The entry/exit price engine design](../../../strategies/entry_exit_engine_design.md), section 6,
  the break-even and minimum-profitable-exit identities for a single position, and the note that a
  result above the cost it pays is not evidence until the estimate carries its uncertainty.
- [The measurement plan](../../../strategies/books2/implementation_plan.md), the work item requiring
  the report to render the cost row, and its acceptance line that a zero-fee run prints a cost of 0.00
  basis points with gross equal to net.
- [The liquidity and systemic risk brief](../../../strategies/books/12_liquidity_and_systemic_risk.md),
  the cost models for large companies' shares and for government bonds, reported at pages 36 to 48 of
  `2105.08377v1`.
- [The overfitting brief](../../../strategies/books2/28_overfitting_and_research_integrity.md), the
  correction study in which 17.29 and 28.82 percent became -7.36 and -6.31 percent.

## Words used in this tutorial

- basis point: one hundredth of one percent, so 100 basis points make 1 percent.
- break-even cost: the cost per unit traded at which a measured advantage would vanish entirely.
- commission: the fee a broker charges for each trade.
- edge: a small, persistent advantage that makes a strategy worth running after costs.
- gross profit: the profit before costs are subtracted.
- net profit: the profit after costs are subtracted.
- spread: the gap between the best price at which something can be bought and the best price at which
  it can be sold, which a trader crossing that gap pays.
- turnover: the money value of everything bought and sold in a period, counting each fill.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
