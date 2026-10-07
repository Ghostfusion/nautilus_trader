# Position sizing: how much you hold matters more than what you buy

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing on its own. It sets the amount held in each position the rest of the system decides to open                                                                                                                                 |
| How often it trades       | Each time a position is opened, and again whenever its stop changes                                                                                                                                                                 |
| What you need             | Nothing but this page and a pencil; the arithmetic is four multiplications                                                                                                                                                          |
| Where the rules come from | [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Section 7, and [Entry/Exit Price Engine: implementation](../../../strategies/entry_exit_engine_implementation.md), Section 7                    |
| The underlying research   | Risk-budget and Kelly sizing conventions gathered in the two engine documents, with the capacity limits covered in [Portfolio Construction Under Frictions](../../../strategies/books/15_portfolio_construction_under_frictions.md) |
| How well it held up       | Weak: sizing is a choice about risk, not a source of return, and the source study does not establish that risk-budget sizing beats equal weight on the overlay; the design treats the more elaborate forms as unproven              |
| Also appears in           | [Foundations: return, risk and drawdown](../../foundations/05_return-risk-and-drawdown.md) and the sibling tutorial on risk boundaries and stops                                                                                    |

## The idea in one paragraph

The signal says what to buy and which way; the size says how much, and the size is what decides how
much the outcome matters. A rule that wins six times out of ten still loses money if the four losing
positions are three times the size of the six winners. There are three common ways to choose the size:
a fixed amount of money every time, a fixed fraction of whatever the account is worth today, and a
fraction set from how much risk you are willing to take. The last one ties the size to the distance to
the stop: a wide stop gets a small position and a tight stop gets a large one, so that a single loss
costs about the same either way.

## Why anyone believed it

A signal is a guess, and guesses are uncertain. Size is the dial that decides how much a wrong guess
costs, and unlike the guess it can be set from something known in advance: the account you have, the
stop you placed, and how much of the account you are prepared to lose on one trade. Everyone on the
other side of your trade is also choosing a size, and the ones who choose badly are the reason large
moves happen: a leveraged fund forced to sell as prices fall, or a trader who risked the whole account
on one idea.

The deeper reason sizing is taken seriously is compounding. An account that loses 50 percent needs a
100 percent gain just to get back to even, so a single oversized loss is harder to recover from than
the same loss split across several positions. Sizing rules exist mostly to stop a bad run from turning
into a permanent one. That is a risk decision, not a forecast.

## An everyday comparison

Think of a person moving furniture in a pickup truck. The route is the signal and the load is the size.
Choosing a clever route does not help much if the load is piled so high that the first pothole tips the
truck; choosing a cautious load does not help if the route drives off a cliff. The two choices are
independent, and the load is the one that determines whether a single mistake is survivable. A driver
who carries a third of a load and hits three potholes is fine; a driver who carries a full load and
hits one is not.

## The rules, step by step

1. Work out the signal first. Sizing never decides which position to open; it only decides how much.
2. Decide which sizing rule to use. The engine supports equal weight and risk-budget weight, and the
   simpler equal weight is the default.
3. For fixed amounts, choose a constant sum of money and put that in every position, whatever the
   account is worth at the time.
4. For fixed fractions, choose a fraction and put that share of the account's current value into the
   position, so the position grows when the account grows and shrinks when it shrinks.
5. For risk-budget sizing, decide the fraction of the account you are willing to lose if the stop is
   hit, then divide it by the distance from the entry price to the stop price to get the position.
6. Sum the weights you want. If the total is larger than the gross exposure limit, scale every weight
   down by the same factor so the total equals the limit. The part you did not allocate stays in cash.
7. If the more elaborate Kelly cap is switched on, use a win rate measured on the data you are
   allowed to look at, and label it as fitted. Otherwise leave it off.
8. Write down every parameter you chose. Each one is a degree of freedom, and each one must be frozen
   and re-tested rather than tuned until the past looks good.

The order matters. The engine's risk-budget rule needs the entry price and the stop price, so it can
only run after the risk boundary is known. That is why sizing is the last of the three layers built,
and why it is off unless the earlier layers are on.

## The maths, with every symbol named

The amount to hold, given a risk budget and a stop:

```text
Q = ( A * r ) / ( P_entry - P_stop )
```

- `Q` is the quantity to hold.
- `A` is the value of the account.
- `r` is the fraction of the account you are willing to lose on this position if the stop is hit.
- `P_entry` is the price at which the position is opened.
- `P_stop` is the price at which the position is closed for a loss.

The formula says the quantity is the money at risk divided by the money lost per unit if the stop is
hit. If you are willing to lose 1,000 and the stop is 6 below the entry, you can hold 1,000 / 6 units.
The loss if the stop triggers is then exactly the budget, whatever the stop distance happens to be.

Written as a fraction of the account, which is the form the implementation uses per leg:

```text
w = r / ( ( P_entry - P_stop ) / P_entry )
```

- `w` is the weight, the fraction of the account placed in this position.
- `r` is the same risk fraction.
- The bracket is the stop distance as a fraction of the entry price.

The implementation computes one weight per leg this way, then, if the weights sum to more than the
gross exposure limit `maxGrossExposure`, scales all of them down together so the sum equals the limit.
Whatever is left over stays in cash. Two consequences are worth stating: a volatility-scaled stop
automatically shrinks the position when the market gets wild, because the stop distance widens; and a
tight stop produces a larger position for the same risk, which increases the swings inside the
position even though the loss at the stop is unchanged.

The optional Kelly cap, used only as a ceiling and never as the primary rule:

```text
kellyCap = lambda * ( b * p - q ) / b
```

- `kellyCap` is the largest fraction the rule will allow on one position.
- `lambda` is a fraction below one that shrinks the raw figure. The engine's default is to disable
  the cap entirely.
- `p` is the measured chance of winning, taken from the observation window only.
- `q` is `1 - p`, the chance of losing.
- `b` is the ratio of the average gain to the average loss, sometimes written `G / L`.

The engine's own caution is the important part: `p` must come from data that was not used to build
the rule, and the resulting cap must be reported beside the realised win rate so a fitted number is
visible as a fitted one. Estimating a win rate from a small sample of past trades and then sizing on
it is fitting noise.

The gross exposure limit itself:

```text
sum of all weights <= maxGrossExposure
```

- The left side is the total of every position's weight, with a loss-making short counted as a
  positive amount of exposure.
- `maxGrossExposure` is the cap. The engine's default is 1.0, meaning the whole account and no
  borrowing.

## A worked example

One signal, applied four times, through a losing streak. The account starts at 100,000. Each trade
loses 8 percent of whatever is placed in it, because the stop is hit every time. The same signal is
sized three ways. The fixed-amount rule puts 25,000 in every trade. The fixed-fraction rule puts 25
percent of the account's current value in. The risk-budget rule risks 1 percent of the account per
trade, and the stop distance changes from trade to trade as the market's volatility changes.

The stop distances for the risk-budget column, and the position each one implies:

| Trade | Stop distance | Budget   | Position  | Loss on the stop |
| ----- | ------------- | -------- | --------- | ---------------- |
| 1     | 8 percent     | 1,000.00 | 12,500.00 | 1,000.00         |
| 2     | 6 percent     | 990.00   | 16,500.00 | 990.00           |
| 3     | 5 percent     | 980.10   | 19,602.00 | 980.10           |
| 4     | 4 percent     | 970.30   | 24,257.48 | 970.30           |

The budget column is 1 percent of the account as it stands at the start of each trade, and the position
column is the budget divided by the stop distance. The last column shows that the loss at the stop is
the budget, exactly, whatever the distance. Now the three methods side by side:

| Trade | Fixed amount position | Account after | Fixed fraction position | Account after | Risk budget position | Account after |
| ----- | --------------------- | ------------- | ----------------------- | ------------- | -------------------- | ------------- |
| Start |                       | 100,000.00    |                         | 100,000.00    |                      | 100,000.00    |
| 1     | 25,000.00             | 98,000.00     | 25,000.00               | 98,000.00     | 12,500.00            | 99,000.00     |
| 2     | 25,000.00             | 96,000.00     | 24,500.00               | 96,040.00     | 16,500.00            | 98,010.00     |
| 3     | 25,000.00             | 94,000.00     | 24,010.00               | 94,119.20     | 19,602.00            | 97,029.90     |
| 4     | 25,000.00             | 92,000.00     | 23,529.80               | 92,236.82     | 24,257.48            | 96,059.60     |

The arithmetic for one row, so the table can be checked. Fixed fraction, trade 2: the account is
98,000, so the position is `0.25 * 98,000 = 24,500`, the stop distance is 8 percent, the loss is
`24,500 * 0.08 = 1,960`, and the account becomes `98,000 - 1,960 = 96,040`. Risk budget, trade 2: the
account is 99,000, the budget is `0.01 * 99,000 = 990`, the stop distance is 6 percent, the position is
`990 / 0.06 = 16,500`, and the account becomes `99,000 - 990 = 98,010`.

Three things to notice. The fixed-amount rule loses the most, because it keeps risking 25,000 of a
shrinking account, so each loss is a larger share of what is left. The fixed-fraction rule is a little
kinder, because the position shrinks with the account. The risk-budget rule ends highest of the three,
but it also started smallest, with 12,500 rather than 25,000, so the comparison is about the shape of
the path and not the level of the first trade. And notice the risk-budget position rising from 12,500
to 24,257 as the stop tightens: the rule holds more when the stop is close, which is correct for the
risk budget and uncomfortable to live through.

## What the research actually found

Sizing is not a strategy, so there is no return to report, and this repository's design note refuses to
pretend otherwise. Its Section 7 keeps only two forms and labels the rest:

| Form                   | Status in the design                                                                                                                                                     |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Equal weight           | The default, because it needs no estimate and no view                                                                                                                    |
| Risk-budget weight     | Accepted, because it couples entry, stop and size with one number and needs no forecast                                                                                  |
| Kelly, fractional      | Optional cap only. The design says a win rate estimated from a small sample of trades is fitting noise, so fractional Kelly at a low fraction is the only defensible use |
| Correlation adjustment | A heuristic, not a standard result; it must be reported as a chosen parameter, never as an axiom                                                                         |

The design's one firm result about sizing is a boundary rather than a method: because the position is
capped, the entry price is also capped, by the identity

```text
P_entry <= P_stop + RiskBudget / Q_max
```

- `P_entry` is the most you may pay and still keep the risk budget.
- `P_stop` is the stop price.
- `RiskBudget` is the money you are willing to lose.
- `Q_max` is the largest quantity the participation cap allows you to hold.

The cap on how much you can trade and the cap on how much you can risk meet in that one line, and it is
the reason sizing needs the execution model from the previous tutorial. The brief on portfolio
construction under frictions reaches the same point from the other side: once liquidity and impact are
priced, the largest position an account can hold without eroding its own result is much smaller than it
looks.

## How this project relates to it

The rules above are specified in the [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md)
Section 7 and worked out in the [implementation document](../../../strategies/entry_exit_engine_implementation.md)
Section 7, which is the section this tutorial's rules come from. That section gives the weight per leg,
the gross-exposure scaling, the leftover-in-cash rule, and the Kelly cap with its warning about where
the win rate may come from. The configuration lists `sizingMode` defaulting to equal weight, a risk per
trade of 0.005, and a gross exposure limit of 1.0.

The design's Section 11 ties it together: every one of these parameters goes into the frozen
configuration and into the fingerprint, changing one changes the instrument, and the parameters are
counted and printed so the number of degrees of freedom is impossible to miss. A reader can see the
whole chain in the two documents, and how sizing is called inside the backtest loop in the
implementation document's Section 8.

## Where it goes wrong

- Sizing does not fix a bad signal. A losing signal loses more slowly if it is sized small, and it is
  still a losing signal. Size controls how much a mistake costs, not whether it is a mistake.
- A fixed amount gets more dangerous as the account shrinks. The same 25,000 is a quarter of a
  100,000 account and half of a 50,000 one, so the losses grow as a share of what is left.
- Risk-budget sizing is only as good as the stop. If the stop is set by convention rather than by
  anything real, the position size inherits that guess, and a stop that is too tight produces a
  position that is too large.
- The parameters are chosen, not measured. A risk fraction of 0.005 and a gross cap of 1.0 are
  conventions. Each one is another way to fit the past.
- Kelly invites a fitted win rate. The formula looks like arithmetic and is only as good as the `p`
  inside it; estimating `p` from the same trades you are sizing is circular.
- Correlation is only a heuristic here. The engine's correlation adjustment is a chosen parameter,
  not a derived one, and a book whose positions move together is more concentrated than the weights
  suggest.
- Capacity is the hidden cap. A position large enough to move the price is paying part of its own
  return away, and the entry-price ceiling above is where that limit finally becomes visible.

## Try it yourself

You need a pencil and a calculator.

1. Start with an account of 10,000 and a stop 5 percent below the entry.
2. Fixed amount: decide to place 2,000 in each of four trades. Write down the account after four
   losses of 5 percent of 2,000 each.
3. Fixed fraction: place 20 percent of the account in each of the same four trades. Recompute the
   position from the account value each time before working out the loss.
4. Risk budget: risk 1 percent of the account per trade. Since the stop is 5 percent, the position is
   `0.01 / 0.05 = 20 percent` of the account. Run the same four losses.
5. Put the three ending values side by side.

What to notice: fixed amount and risk budget end in different places even though both started with a
20 percent position, because the fixed-amount rule keeps risking the same money as the account shrinks
while the risk-budget rule keeps risking the same fraction. Then change the stop to 10 percent and
watch the risk-budget position halve, while the other two rules do not move at all. The size is an
output of the rule, not an input you pick.

## Where this came from

- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Section 7
  (risk-budget sizing, the entry-price ceiling, the Kelly form and the correlation heuristic) and
  Section 11 (parameter discipline).
- [Entry/Exit Price Engine: implementation](../../../strategies/entry_exit_engine_implementation.md),
  Section 7 (the sizing rules the engine uses, including the weight per leg, the gross-exposure
  scaling, the leftover in cash and the Kelly cap) and Section 8 (where sizing is called).
- [Portfolio Construction Under Frictions](../../../strategies/books/15_portfolio_construction_under_frictions.md),
  the brief on how liquidity and impact cap the size an account can hold before it erodes its own
  result.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), whose Section 9
  eligibility decision is why sizing is a risk control rather than a way to manufacture return.
- The 100,000 account, the 8 percent losses and the 1 percent risk budget are teaching numbers chosen
  for easy arithmetic. They are not measurements of any strategy.

## Words used in this tutorial

- position sizing: choosing how much money to place in a trade, as opposed to which trade to place.
- risk budget: the fraction of the account you are willing to lose on one trade if its stop is hit.
- fixed fraction: a rule that places a constant share of the account's current value in each trade.
- gross exposure: the total of every position's size, counting longs and shorts together.
- stop: the price at which a position is closed to limit a loss.
- leverage: using borrowed money so a given price move produces a larger gain or loss.
- Kelly criterion: a formula for the share of an account to bet from the chance of winning and the
  ratio of gains to losses.
- drawdown: the fall from a peak to a later low, measured as a percentage of the peak.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
