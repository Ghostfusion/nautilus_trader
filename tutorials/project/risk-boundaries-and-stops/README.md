# Risk boundaries and stops: three ways a position can end

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                     |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Positions in the directional overlay, by putting a price limit under each one and a time limit around it                                                                                                                                                  |
| How often it trades       | Every period the position is checked, and the position is closed on the period one of the limits is reached                                                                                                                                               |
| What you need             | Nothing but this page and a pencil; the arithmetic is small                                                                                                                                                                                               |
| Where the rules come from | [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Section 7, and [Entry/Exit Price Engine: implementation](../../../strategies/entry_exit_engine_implementation.md), Section 6                                          |
| The underlying research   | None directly: the stop and barrier forms are standard practitioner conventions gathered in the two engine documents; the closest measured evidence is [Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md) |
| How well it held up       | Weak: a stop reshapes the distribution of outcomes rather than adding return, and this project's own design requires the layer be off by default until it proves itself on held-out data                                                                  |
| Also appears in           | [Foundations: return, risk and drawdown](../../foundations/05_return-risk-and-drawdown.md) and [Foundations: orders and how they execute](../../foundations/03_orders-and-how-they-execute.md)                                                            |

## The idea in one paragraph

A stop is a rule that closes a position once its price falls to a chosen level, so a loss cannot grow
without limit. This engine treats stops and the other limits as a separate layer, switched off by
default, because a limit does not decide what to buy; it only decides when to give up on something
already bought. The cleanest way to write all the limits at once is the triple barrier: a profit
target above the entry, a loss limit below it, and a time limit on the calendar, whichever is reached
first closes the position. A time exit or a decay exit is the same idea without a price, ending a
position because it has been held long enough or because the reason for holding it has faded.

## Why anyone believed it

A stop appeals because it converts an open-ended loss into a named one. If you buy at 100 and place a
stop at 92, the most you plan to lose is 8 percent, and you can sleep. Anyone who has watched a losing
position grow knows the feeling this addresses.

The counterparty is the person on the other side of the stop order. When a price falls through a
level where many stops are resting, those orders arrive together as market orders, and they push the
price further down before they are filled. That is why the engine never places a stop exactly on an
observed support level: everyone else's stop is likely to be there too, and being first in that queue
is not a plan. This is a real cost to the person setting the stop, and a real income to whoever knows
where the stops are.

There is a harder truth behind the appeal. A stop does not change the average outcome of a fair bet;
it changes the shape. It cuts off large losses, which raises how often the strategy wins, but it also
cuts off large gains, because the position that would have recovered is closed. Whether that trade is
good depends entirely on whether prices bounce after falling, which is not something a stop can know.

## An everyday comparison

Think of a shop that keeps a display of fresh bread. The shop sets a rule: any loaf unsold by closing
time is sold half price rather than thrown away. That rule bounds the loss on each loaf, which is what
its owner wanted. It also means the shop never earns the full price on a loaf that would have sold
tomorrow. If bread goes stale quickly, the rule is right. If a loaf would have kept, the rule gave away
money. The rule itself is neither good nor bad; it depends on how fast the thing decays and on how
often the alternative would have worked. A stop is exactly that rule applied to a price.

## The rules, step by step

1. The layer is off until the market state admits a directional bet. If it does, and only then, the
   layer may be switched on.
2. Decide the stop method. The engine offers five: a fixed number of a typical day's range below the
   entry, a fixed percentage below the entry, a level below a recent support price, a volatility
   multiple, and a chandelier stop that trails the highest price seen.
3. Compute the stop level from the entry price and the chosen method. Never place it exactly on the
   support level itself; offset it below.
4. Decide the profit target, a level above the entry.
5. Decide the time limit, a number of periods after which the position is closed regardless of price.
6. Once a period, check all three barriers. If the price reached the target, close and call it a
   target exit. If it reached the stop, close and call it a stop exit. If neither, and the position has
   been held for the time limit, close and call it a time exit.
7. If the price reached both the target and the stop inside the same period, and there is no record of
   which came first, call it a stop exit. The engine does this on purpose, because assuming the
   favourable order would be dishonest.
8. If the position is still open and the stop is a trailing kind, raise the stop as the price rises,
   never lower it.
9. Record the reason for every exit and report the layer's effect with the win rate and both tails of
   the trade distribution beside it. Never call it a source of return.

## The maths, with every symbol named

The five stop forms. Each produces the price at which the position is closed:

```text
P_stop = P_entry - k * ATR_n           (a typical day's range)
P_stop = P_entry * (1 - k * sigma)     (a multiple of volatility)
P_stop = support - k * ATR_n           (below a recent support level)
P_stop = P_entry * (1 - stopPercent)   (a fixed percentage)
```

- `P_stop` is the price at which the position is closed.
- `P_entry` is the price at which the position was opened.
- `k` is a multiple chosen in advance. The engine's default is 2.
- `ATR_n` is the typical size of a day's range over the last `n` periods.
- `sigma` is the volatility of the position, as a decimal per period.
- `support` is a price below which the thing has recently stopped falling.
- `stopPercent` is the fixed fraction below the entry. The engine's default is 0.08, or 8 percent.

The chandelier stop and the trailing stop move the limit up as the price rises:

```text
CE_long = HighestHigh_n - k * ATR_n
Stop_t  = max( Stop_t-1 , P_t - k * ATR_t )
```

- `CE_long` is the chandelier level, hanging below the highest price seen.
- `HighestHigh_n` is the highest price over the last `n` periods.
- `Stop_t` is the trailing stop today, and `Stop_t-1` is the same stop yesterday.
- `P_t` is today's price.

The `max` is the whole point. It says today's stop is the higher of yesterday's stop and today's
candidate, so a trailing stop can only move up. A stop that loosens is a bug, and the implementation
tests for it.

The triple barrier, the engine's default structure for an exit:

```text
Upper = P_entry + u
Lower = P_entry - d
TimeBarrier = T periods
Exit = first barrier reached
```

- `Upper` is the profit target, a distance `u` above the entry.
- `Lower` is the loss limit, a distance `d` below the entry.
- `TimeBarrier` is a number of periods `T` after which the position ends.
- `Exit` is whichever of the three is reached first.

Writing all three at once forces the exit rule to be explicit about every way a position can end. The
engine's previous design held a position until the next rebuild, which is a time barrier that nobody
chose, and the triple barrier names it instead of leaving it implicit.

The decay exit is the more principled version of a time limit. The reason for holding a position is
assumed to fade at a steady rate:

```text
alpha(t) = alpha_0 * exp( -lambda * t )
exit when alpha(t) < alpha_min
```

- `alpha(t)` is how strong the reason for holding is at time `t`.
- `alpha_0` is how strong it was at the start.
- `lambda` is the rate at which it fades.
- `alpha_min` is the level below which the position is not worth keeping.

Solving that for the time gives the maximum holding period directly:

```text
t_exit = ln( alpha_0 / alpha_min ) / lambda
```

With `alpha_0` of 0.02, `alpha_min` of 0.005 and `lambda` of 0.05, the holding period is about 27.7
periods, which is the number the implementation's own test expects. The point of this form is that the
time limit is set from a fading reason rather than picked because a round number sounded good.

## A worked example

One position, held through the overlay. It is opened at 100.00. The profit target is set 10 percent
above, at 110.00, the loss limit 8 percent below, at 92.00, and the time limit at 60 periods. Round-trip
trading cost is 0.28 percent, from the execution tutorial, and it is subtracted from every gross
result below. The three rows are three separate runs of the same rules, each hitting a different
barrier.

| Run   | What happened over the period                                                                    | Exit price | Gross result | Net result |
| ----- | ------------------------------------------------------------------------------------------------ | ---------- | ------------ | ---------- |
| One   | The price reached 110.50 in period 12, so the target was the first barrier reached               | 110.00     | +10.00%      | +9.72%     |
| Two   | The price reached 91.50 in period 9, so the loss limit was the first barrier reached             | 92.00      | -8.00%       | -8.28%     |
| Three | Neither price barrier was reached; after 60 periods the time limit closed the position at 103.00 | 103.00     | +3.00%       | +2.72%     |

The arithmetic for the first row is the pattern for all three:

```text
Gross = (110.00 - 100.00) / 100.00 = 0.10 = +10.00%
Net   = 0.10 - 0.0028 = +9.72%
```

Two notes on honesty in this table. First, the exit is modelled at the barrier level, so the first row
pretends the position was sold at 110.00 even though its high was 110.50. That is close to how a profit
target behaves, and it is deliberately pessimistic about the gain. Second, the stop row pretends the
position was sold at 92.00 even though its low was 91.50. Real stops fill lower than their level when
the market gaps through them, so the modelled stop exit is optimistic on that count. The engine takes
the conservative choice where it can, and states where it cannot.

Now the stop construction, for the same entry of 100.00:

| Method          | Inputs                                 | Stop price |
| --------------- | -------------------------------------- | ---------- |
| A typical range | `k` 2, typical daily range 3.00        | 94.00      |
| Percentage      | `stopPercent` 0.08                     | 92.00      |
| Volatility      | `k` 2, volatility 0.04                 | 92.00      |
| Chandelier      | highest high 120.00, `k` 2, range 3.00 | 114.00     |

The chandelier stop starts far above the entry here because the price has already run up, which is what
a trailing stop does: it protects gains rather than only limiting losses.

## What the research actually found

This project's design note is unusually blunt about what is and is not known here. The stop and barrier
forms are standard practitioner conventions; they are not the product of one paper showing that stops
add return. What the design does say, in its own words, is that the risk layer "either improves
something measurable or it is removed", and that it must be judged on the overlay against the same
overlay without the layer, on data kept aside, with the full distribution of outcomes reported.

The design also makes a claim that is easy to miss: a stop mechanically raises the win rate and
truncates both tails of the trade distribution. A higher win rate sounds like an improvement and is
not, on its own, one. A strategy that wins nine small times and loses badly once can have a high win
rate and a poor average. That is why the engine refuses to call the risk layer alpha and requires the
win rate and both tails to be printed next to it.

The measured evidence in this collection about risk measures and drawdowns points the same way: the
tail-sensitive measures that would justify a particular stop level are exactly the ones whose estimates
one large loss can move without bound. There is no number from the research that says where a stop
should sit.

## How this project relates to it

The layer is specified in the [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md)
Section 7, and implemented in [Entry/Exit Price Engine: implementation](../../../strategies/entry_exit_engine_implementation.md)
Section 6. The design's Section 3 states the three rules that govern it: execution changes the cost of
a decision but never the decision, every new mechanism gets its own attribution line, and the risk
layer applies only to the directional overlay and never to the structural book of sector funds, which
has no thesis to invalidate.

In the implementation, the layer is a master switch called `riskLayer`, defaulted off, and it is inert
unless the market state admits a directional bet. When it is on, the backtest runs one extra series
beside the existing books, and the layer's effect is measured as the difference between the book with
stops and the book without them. A reader can find that series and its attribution line in the
implementation document, Sections 6 and 8.

## Where it goes wrong

- A stop converts a loss that might have recovered into a certain loss. Whether that helps depends on
  whether prices bounce after falling, which the stop cannot know.
- The distribution changes shape, not its average. Win rate rises while the largest gains disappear.
  Reporting the higher win rate without both tails is the most likely way this layer misleads.
- The level cannot be filled reliably. A stop becomes an ordinary order once its trigger is crossed,
  and if many stops sit together they can push the price further as they arrive.
- The parameters are chosen, not measured. A multiple of two, or eight percent, is convention. Every
  such choice is another degree of freedom, and the design requires each one be frozen and re-tested
  rather than tuned until the past looks good.
- On synthetic data the stops look better than reality. Synthetic prices have no overnight gaps, so a
  stop is never jumped over. The design says to treat any stop result as an upper bound.
- Applied to the wrong book it is market timing. On a diversified allocation with no thesis, a stop is
  just a way to sell low and buy back higher, which the source study does not admit.
- Decay is assumed, not measured. The fade rate `lambda` turns a guess into a holding period; a wrong
  guess is a wrong holding period wearing a formula's clothes.

## Try it yourself

You need a pencil and a published table of daily prices for one fund.

1. Pick a starting date and an entry price. Write down a profit target 10 percent above it, a loss
   limit 8 percent below it, and a time limit of 60 days.
2. Walk down the price column one day at a time. The moment the price touches the target, stop, or the
   day count reaches 60, write down which barrier it was and the price at which you closed.
3. Now do the whole thing again on the same prices, but this time do not stop out: hold all the way to
   the 60-day mark.
4. Compare the two lists of outcomes: the win rate and the biggest gain and biggest loss of each.

What to notice: the first list will usually have a higher win rate, because small losses were cut off.
The second will usually have a bigger single gain, because the position that fell and then recovered
was allowed to. Neither list is the "true" answer; the exercise is to see that a stop trades one shape
of outcomes for another, and that the trade is not free.

## Where this came from

- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Section 3 (the
  three governing rules), Section 7 (Layer 2, stop construction, the triple barrier and the time and
  decay exits), Section 11 (parameter discipline) and Section 13 (what would falsify the design).
- [Entry/Exit Price Engine: implementation](../../../strategies/entry_exit_engine_implementation.md),
  Section 6 (the stop, barrier and time functions and the tie-break), Section 8 (the extra series and
  the attribution line) and Section 10 (the known-answer tests, including the chandelier and decay
  numbers used above).
- [Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md),
  the brief on why tail-sensitive risk numbers are fragile and one large loss can move them.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), whose Section 9
  eligibility decision is why the layer is off by default: it is only meaningful when a directional
  overlay is admitted at all.
- The 10 percent target, the 8 percent stop and the 0.28 percent round-trip cost are teaching numbers,
  chosen so the arithmetic is easy to follow. They are not measurements.

## Words used in this tutorial

- stop: a price at which a position is closed to limit a loss.
- profit target: a price above the entry at which a position is closed to take a gain.
- triple barrier: the rule that closes a position at whichever of a profit target, a loss limit or a
  time limit is reached first.
- trailing stop: a stop that is raised as the price rises, and never lowered.
- support: a price below which something has recently stopped falling.
- volatility: how much a price moves around its average.
- decay exit: closing a position because the reason for holding it has faded, not because of a price.
- drawdown: the fall from a peak to a later low, measured as a percentage of the peak.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
