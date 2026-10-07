# Execution at the price you get: why the fill is not the price on the screen

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                        |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing on its own. It models the cost of whatever the rest of the system decides to trade, and it applies to every trade                                                                                                    |
| How often it trades       | Every time the system places an order                                                                                                                                                                                        |
| What you need             | Nothing but this page, plus a spreadsheet if you want to follow the arithmetic                                                                                                                                               |
| Where the rules come from | [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Section 6, and [Entry/Exit Price Engine: implementation](../../../strategies/entry_exit_engine_implementation.md), Section 5             |
| The underlying research   | The Almgren-Chriss tradeoff between execution cost and timing risk, and the square-root law of market impact, both covered in [Optimal Execution and Liquidation](../../../strategies/books/01_execution_and_liquidation.md) |
| How well it held up       | Mixed: the pieces are individually well measured, but no source behind this engine confirms that the assembled model improves any decision, and the design is explicit that it can only make a result worse                  |
| Also appears in           | [Foundations: orders and how they execute](../../foundations/03_orders-and-how-they-execute.md) and [Foundations: costs, fees and taxes](../../foundations/06_costs-fees-and-taxes.md)                                       |

## The idea in one paragraph

The price you see on a screen is a snapshot from a moment ago. By the time your order travels to the
exchange and is matched, the best prices may have moved, and a large order may have to eat through
several price levels to fill. The price you actually get is therefore worse than the one you planned
around. This tutorial explains the gap in plain terms: the half-spread you cross, the slippage from the
market moving, and the impact of your own order. It then explains the one limit the engine places on
itself, a cap on how much of the day's volume an order may take, and the break-even identity that says
how far a price must move just to get your money back.

## Why anyone believed it

Nobody believes an execution model makes money. The belief is narrower and more useful: that a strategy
which looks profitable in a test is often only profitable because the test charged too little for
trading. If the fill is modelled honestly, fewer strategies look good, which is the direction honest
work should push.

The counterparty here is the market itself. When you buy at the ask and later sell at the bid, the gap
between those two prices goes to whoever stood on the other side, usually a dealer quoting both prices
all day. When your order is large, the people who see it coming can trade ahead of it, and the price
drifts up as you buy. Those people are not doing anything wrong; they are doing what the market pays
them to do. The cost they take is a real transfer out of your account, and a test that ignores it is
not measuring the strategy you would actually run.

## An everyday comparison

Imagine buying a hundred loaves of bread at a farmers' market. The sign says two pounds a loaf, and if
you wanted one loaf that is what you would pay. But the stall only has thirty loaves on the table. For
the first thirty you pay two pounds. For the next thirty the stallholder has to send for more, and
charges a little more to cover the trouble. For the last forty you are competing with the afternoon
shoppers and the price creeps up again. Your average price for the hundred loaves is higher than the
sign, even though nobody cheated you. That gap between the sign and your average is exactly what this
tutorial is about, and the stallholder who raises the price as your order grows is the market's price
impact.

## The rules, step by step

1. Start from a signal price. This is the price the system saw when it made its decision, such as the
   closing price of the period. It is not a price anyone can trade at.
2. Add the half-spread. Every asset has a buy price and a sell price, and they differ by the spread.
   If you buy, you pay the higher one, so you pay half the spread above the middle. If you sell, you
   receive half the spread below it.
3. Add slippage. This is a further cost for the market moving between the moment of the decision and
   the moment of the fill. The engine scales it by how much the price normally moves in a day.
4. Add impact. This is the cost of your own order changing the price as it fills. It grows with the
   size of the order relative to the volume normally traded in that thing.
5. Add commission and fees, kept separate from the spread, then add up all the pieces to a total cost
   in basis points. One basis point is one hundredth of one percent.
6. Check the participation cap. If the order is larger than the allowed fraction of the day's volume,
   cut it down to the cap, report how much was cut, and leave the rest in cash. Do not pretend the
   whole order happened.
7. Charge the cost against the trade. The engine charges it as a drag on the account rather than
   adjusting the price, but the fill price implied by the drag and the drag itself must agree.

The engine does this on every order, and it is switched on permanently, unlike the risk layer in the
next tutorial, which is switched off by default. The reason is that this layer cannot invent an edge.
It can only subtract.

## The maths, with every symbol named

The price a buy actually gets, and the price a sell actually gets:

```text
P_exec_buy  = P_signal + half_spread + slippage + impact
P_exec_sell = P_signal - half_spread - slippage - impact
```

- `P_exec_buy` is the effective price paid when buying.
- `P_exec_sell` is the effective price received when selling.
- `P_signal` is the price seen when the decision was made.
- `half_spread` is half the gap between the buy price and the sell price.
- `slippage` is the cost of the market moving after the decision.
- `impact` is the cost of your own order moving the price.

Both lines say the same thing: buying costs more than the signal price and selling brings in less.

Slippage, scaled by how much the thing normally moves in a day:

```text
slippage = c_sigma * sigma_daily * P_signal
```

- `c_sigma` is the fraction of a typical daily move charged as slippage. The engine's default is 0.1.
- `sigma_daily` is the typical size of one day's price move, as a decimal. A value of 0.01 means one
  percent a day.
- `P_signal` is the same signal price as before.

The impact of an order, which grows with its size but less than in direct proportion:

```text
impact = Y * sigma_daily * (Q / ADV) ^ alpha
```

- `Y` is a coefficient setting how strongly impact responds to order size. The engine's default is 0.1.
- `Q` is the quantity the order wants to trade.
- `ADV` is the average daily volume, the amount normally traded in a day.
- `alpha` is the exponent. The engine uses 0.5, the square-root law, which is the form that survives
  testing on real trades.

Because `alpha` is 0.5, doubling an order's size increases its impact by about forty percent, not by
one hundred percent. The cost never disappears, and it is added on top of the spread rather than
replacing it.

The participation cap, the one limit the engine places on itself:

```text
Q_max = p_max * Volume
```

- `Q_max` is the largest quantity the order is allowed to trade this period.
- `p_max` is the fraction of the period's volume allowed. The engine's default is 0.05, one twentieth.
- `Volume` is the volume actually traded in that period.

An order above `Q_max` is clipped to it. The clip is not charged as a cost, because the untraded part
simply stays in cash and never takes part in the return. It is reported separately as foregone
exposure. This is the honest way to express a capacity limit, and leaving it out is the one omission
that can silently make a whole test meaningless.

The break-even identity, which answers how far the price must move before the trade is even:

```text
P_break_even_long = P_entry + (C_entry + C_exit) / Q
```

- `P_break_even_long` is the price at which holding the position neither gains nor loses money.
- `P_entry` is the price paid to enter.
- `C_entry` is the entry cost in currency.
- `C_exit` is the expected exit cost in currency.
- `Q` is the quantity held.

The formula says the price must rise by the total trading cost divided by the quantity, which is the
cost per unit, before any profit is real. A second identity adds a required return:

```text
P_exit_min_long = P_entry * (1 + r_required) + Costs / Q
```

- `P_exit_min_long` is the price at which the position earns the return you required of it.
- `r_required` is that required return, as a decimal.
- `Costs` is the total cost of entering and exiting, in currency.
- `Q` is the quantity held.

The first identity tells you the break-even. The second tells you the price that clears the bar you set.

## A worked example

One order: buy 10,000 shares of a sector fund whose signal price is 100.00, so the order is worth
1,000,000. The inputs are a half-spread of 2 basis points, commission plus fees of 1 basis point, a
daily move of 1 percent, a slippage fraction of 0.1, a size coefficient `Y` of 0.1, and an order that is
1 percent of the fund's average daily volume, so the square root of the participation is 0.1.

| Piece            | Formula                                     | Result   |
| ---------------- | ------------------------------------------- | -------- |
| Half-spread      | given                                       | 2.0 bps  |
| Commission, fees | given                                       | 1.0 bps  |
| Slippage         | 0.1 * 0.01 * 100.00 = 0.001 of price        | 10.0 bps |
| Impact           | 0.1 * 0.01 * (0.01) ^ 0.5 = 0.0001 of price | 1.0 bps  |
| Total            | 2 + 1 + 10 + 1                              | 14.0 bps |

The total of 14 basis points is 0.0014 of the price. The fill price the buyer should expect:

```text
P_exec_buy = 100.00 * (1 + 0.0014) = 100.14
```

Now the break-even identity, using the cost in currency and a round-trip cost of 14 basis points on
each side. The entry cost is 1,000,000 * 0.0014 = 1,400, and the assumed exit cost on the same money is
another 1,400.

```text
P_break_even_long = 100.00 + (1,400 + 1,400) / 10,000 = 100.00 + 0.28 = 100.28
```

The fund must reach 100.28, a rise of 0.28 percent, before the trade is even. If the account demands a
required return of 1 percent on the position:

```text
P_exit_min_long = 100.00 * 1.01 + 2,800 / 10,000 = 101.00 + 0.28 = 101.28
```

Finally the participation cap. Suppose the order had wanted to trade 8 percent of the fund's average
daily volume, but the cap is 5 percent.

```text
Q_max = 0.05 * Volume
```

Of an intended order worth 100 percent of its own size, 5 parts of 8 can trade and 3 parts cannot, so
37.5 percent of the intended order is clipped and stays in cash. That is reported as a separate line,
not as a cost.

The lesson of the table is in the size of the slippage line. Ten basis points of slippage, chosen
because the price moves about 1 percent a day, is five times the spread. A strategy that trades often
pays that many times over, and it is the reason this layer is built before any others.

## What the research actually found

The execution model is not a strategy, so it has no return to report. Its pieces do have measurements
behind them, and this repository's briefs collect them.

| Piece                                | What was measured                                                                                                                   | Where                                                                                            |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Market impact, the square-root law   | The average impact of an order grows roughly with the square root of its size relative to normal volume, with the exponent near 0.5 | [Market Impact and Trading Cost](../../../strategies/books/02_market_impact_and_trading_cost.md) |
| Order-book walk and fill probability | Each price level holds limited size, so a large order walks through the book; most limit orders are cancelled rather than filled    | [Limit Order Book Dynamics](../../../strategies/books/03_limit_order_book_dynamics.md)           |
| Scheduling cost and risk             | The Almgren-Chriss tradeoff balances the cost of trading fast against the risk of trading slowly                                    | [Optimal Execution and Liquidation](../../../strategies/books/01_execution_and_liquidation.md)   |

What the sources do not establish is that this assembled model improves any decision. The design
document says so plainly: the layer "can only make a strategy worse", and if it changes the ranking of
frequencies and holding rules, that is the interesting result. Its value is honesty, not edge.

## How this project relates to it

The model above is specified in this repository's
[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md) Section 6, as Layer
1, and worked out in code-level detail in the
[implementation document](../../../strategies/entry_exit_engine_implementation.md), Sections 5 and 8.
Section 5 of the implementation gives the exact formulas, the participation clip and the break-even
functions; Section 8 shows where in the backtest loop the model is called, and how the four new
attribution lines are kept additive with the existing ones.

That engine sits behind the
[sector-regime-engine](../../../implementation/sector-regime-engine/README.md), which measures the
market state rather than predicting returns. The execution layer is the one part that is always
switched on, because a cost model cannot create a false positive.

## Where it goes wrong

- The model can be too optimistic. Real fills include jumps an analytic model smooths over, so a
  result that improves because of a cost assumption should be treated as an upper bound.
- Impact is skipped when volume is missing. If there is no data on how much is normally traded, the
  impact line becomes zero, which looks the same as measuring no impact. The design's answer is to
  report it as skipped rather than to print a silent zero, and a reader should check that flag.
- Slippage is a guess. It is a fixed fraction of a typical daily move, not a measurement of this
  order. Change the fraction and the cost changes; the default is convention, not evidence.
- The participation cap hides a real limit. An order clipped to 5 percent of volume is not free; the
  untraded part is exposure the strategy wanted and did not get, which changes the strategy's own
  behaviour and is easy to forget when reading only the return.
- Delay cost can dominate, and the model is only analytic. The fill is modelled a period after the
  decision, so the cost of that delay, called implementation shortfall, is often the largest line of
  all; and the model knows nothing about the venue, the queue, or who takes the other side.

## Try it yourself

You need a spreadsheet and nothing else.

1. Build four columns: half-spread in basis points, commission plus fees in basis points, a daily move
   in percent, and the order's size as a fraction of average daily volume.
2. Add a slippage column that multiplies the daily move by 0.1 and turns it into basis points, using
   the fact that 1 percent is 100 basis points.
3. Add an impact column that multiplies 0.1 by the daily move by the square root of the participation,
   again in basis points.
4. Add a total column that sums the four.
5. Add a break-even column: the signal price plus the total cost applied twice, divided by the price.
   Then double the participation and watch impact rise by less than double, because of the square root.

What to notice: on plausible inputs, the sum of the spread and commission is tiny next to the slippage
and the break-even move. That is the whole argument for building this layer. A test that charges only
the spread is not charging the cost that actually decides whether a busy strategy survives.

## Where this came from

- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Section 5 (the
  four prices) and Section 6 (Layer 1, the execution model, the participation cap, the fill price and
  the break-even identities).
- [Entry/Exit Price Engine: implementation](../../../strategies/entry_exit_engine_implementation.md),
  Section 5 (execution code and signatures), Section 8 (integration into the backtest loop) and
  Section 10 (the known-answer tests the numbers above follow).
- [Optimal Execution and Liquidation](../../../strategies/books/01_execution_and_liquidation.md), the
  brief on the Almgren-Chriss tradeoff and VWAP scheduling.
- [Market Impact and Trading Cost](../../../strategies/books/02_market_impact_and_trading_cost.md), the
  brief on the square-root law and its measured exponent.
- [Limit Order Book Dynamics](../../../strategies/books/03_limit_order_book_dynamics.md), the brief on
  how a large order walks through the book and how often resting orders fill.
- The 14 basis points and the 100.28 break-even are worked for this page from the formulas in the two
  engine documents. They are teaching numbers, not measurements of a real order.

## Words used in this tutorial

- spread: the gap between the buy price and the sell price of the same thing at one moment.
- basis point: one hundredth of one percent, so 14 basis points is 0.14 percent.
- slippage: the difference between the price you expected and the price you actually got.
- market impact: the tendency of your own order to move the price against you as it fills.
- participation: the size of an order as a fraction of the volume normally traded.
- break-even: the price at which a position has earned back exactly what it cost to open and close.
- implementation shortfall: the cost of the delay between deciding to trade and actually trading.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
