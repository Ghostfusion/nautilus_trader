# Risk management: sizing down, standing aside and hedging the tail

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                               |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Mostly gold on daily bars; other variants hold small baskets of shares, bonds and gold funds, and one trades a hedge between gold and gold miners                                                   |
| How often it trades       | The moving-average switch acts once a month; the volatility-target systems check daily but act only when the target moves by more than a set band                                                   |
| What you need             | Python and a data file for the full backtests; a spreadsheet is enough to follow the arithmetic here                                                                                                |
| Where the rules come from | [Strategy Compendium, article 14, risk_management](https://backtrader.readthedocs.io/en/latest/strategies-series/en/14-risk-management.html)                                                        |
| The underlying research   | Mebane Faber's ten-month moving-average study, the volatility-targeting literature, and the classic constant-proportion idea, all described in the category article                                 |
| How well it held up       | Mixed: the ten-month rule has a long published record and does reduce the worst falls, but it does not remove them, and one system here has a genuine coding error that the test faithfully records |
| Also appears in           | [Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md) and [Position sizing](../../project/position-sizing/README.md) in this collection                                    |

## The idea in one paragraph

Most strategies argue about what to buy. This group barely argues about that at all. It assumes you
are already holding something worth holding, and asks a different question: how much of it should you
own right now? Three answers live here. The first is to hold less when the market is wild, so that a
swing costs the same whether the market is calm or excited. The second is to hold less when the
account has already fallen, in steps, the deeper the fall the smaller the position. The third is to
step aside entirely when the price drops below a long-term average, a simple rule that has been
published for a century. None of them forecasts a price. They change the amount at risk, not the
direction of the bet.

## Why anyone believed it

The argument starts from a fact about losing: an account that falls 50 percent needs a 100 percent
gain to get back to even. A large loss is not merely unpleasant, it is arithmetically harder to
recover from than the same loss split into several smaller ones. If you can make the bad stretches
smaller without giving up too much of the good ones, compounding works in your favour.

The counterparty is the forced seller. In a sharp fall, some holders must sell regardless of the
price: a leveraged fund meeting a lender's demands, a trader whose margin has run out, a fund
processing withdrawals. Their selling pushes the price down further, which is why falls accelerate.
A rule that steps aside early is trying to be somewhere else when that happens, and a rule that
shrinks its position is trying to make each dollar of that fall cost less.

The third reason is that you cannot know the future. You cannot tell whether a fall is a dip to be
bought or the start of a long decline. A rule like "own it while the price is above its long-term
average, and halve the holding when it is not" does not pretend to know; it just refuses to fight a
downtrend with a full position.

## An everyday comparison

Think of a homeowner preparing for winter storms. She does not know whether the next storm will hit,
so she does not board every window every day. Instead she has a rule: when the forecast wind passes a
level, she brings the garden furniture inside; when it passes a higher level, she closes the shutters
too. Each step reduces what can be damaged without abandoning the house. The storms she prepares for
may never come, and she will have carried furniture indoors for nothing some years. That wasted effort
is the price of not being caught once.

## The rules, step by step

The volatility-target system with a drawdown ladder, the most elaborate member:

1. Each day compute the return, meaning today's close divided by yesterday's close minus one.
2. Compute the annualised volatility over the last 20 days: the standard deviation of those returns,
   multiplied by the square root of 252.
3. Compute the position from volatility as the target volatility, 0.12, divided by the realised
   volatility, then clipped to lie between 0.25 and 1.0. A wild market gives a small number, a calm
   market a number near one.
4. Compute the fall from the peak: the highest close seen so far, minus today's close, divided by the
   highest close. This is a negative number once the price is below its peak.
5. Turn that fall into a second position multiplier by a ladder of thresholds.
6. Take the smaller of the two position numbers.
7. Smooth it: the new target is 85 percent of yesterday's target plus 15 percent of today's.
8. Trade only when the smoothed target moves by more than 0.05 from the position currently held;
   then buy or sell to match, in whole units of the account.

The monthly moving-average switch, the simplest member:

1. Reduce the daily closes to one close per month, the last close of each month.
2. Compute the average of the last 10 of those monthly closes.
3. Mark the month as a risk month if the monthly close is below that average.
4. Use the previous month's mark to decide this month's holding, so the decision never uses a close
   that has not happened yet.
5. Hold the full position, 100 percent of the account, in a normal month, and half the position, 50
   percent, in a risk month.
6. Rebalance only when the target has changed and the gap from the current holding is more than a 2
   percent band.

The risk-on, risk-off switch, for completeness:

1. Each day compute the annualised volatility over the last 60 days.
2. Check whether the close is above its own 100-day average.
3. Hold a long position only when both are true at once, that is, volatility below 20 percent and the
   price above its average; otherwise hold nothing.

## The maths, with every symbol named

The two measures the group rests on, both of a level of movement rather than a direction:

```text
sigma = sqrt( (1 / (N - 1)) * sum from i = 1 to N of (r_i - r_bar)^2 ) * sqrt(252)
```

- `sigma` is the annualised volatility, a decimal: 0.20 means 20 percent a year.
- `r_i` is the return on day `i`.
- `r_bar` is the average of the `N` returns in the window.
- `N` is the number of days in the window, 20 for the drawdown system and 60 for risk-on, risk-off.
- `252` converts a daily figure to a yearly one.

The fall from the peak, called the drawdown:

```text
drawdown = (close - peak) / peak
```

- `close` is today's closing price.
- `peak` is the highest close seen since the record began, or over the chosen window.
- `drawdown` is zero at a new high and negative below it: minus 0.10 means the price is 10 percent
  below its best level.

The volatility-target rule turns the first into a position:

```text
raw_position = target_vol / sigma
position = the smaller of 1.0 and the larger of 0.25 and raw_position
```

- `target_vol` is the risk the system wants to run, 0.12, that is 12 percent a year.
- `sigma` is the realised volatility from the formula above.
- `position` is the fraction of the account to hold, between 0.25 and 1.0.

What it means: if the market is delivering 20 percent a year of movement and the system wants 12, it
holds 0.12 / 0.20 = 0.60, or 60 percent of the account. If the market settles to 10 percent, it holds
the whole account.

The monthly moving average and the ladder:

```text
ma10 = ( m_1 + m_2 + ... + m_10 ) / 10
risk_month = 1 if monthly_close < ma10, otherwise 0
target = 0.5 if risk_month else 1.0
```

- `m_1` to `m_10` are the last ten month-end closes.
- `ma10` is their average.
- `monthly_close` is this month's month-end close.
- `target` is the fraction of the account to hold next month.

What it means: below the ten-month average, the system owns half as much. The average moves slowly, so
the signal is stable and changes rarely.

## A worked example

First the drawdown ladder, which the test records faithfully. The code below is the actual order of
the checks, and the order changes the answer:

```text
if drawdown < -0.03: return 1.0
elif drawdown < -0.06: return 0.75
elif drawdown < -0.10: return 0.5
else: return 0.25
```

Because a drawdown is never positive, any fall deeper than 3 percent satisfies the first check and
returns 1.0 immediately. That makes the 0.75 and 0.5 branches unreachable, and a fall shallower than
3 percent falls through every branch to 0.25. The intended ladder, the deeper the fall the smaller
the position, is inverted in the code. The table shows what the code actually returns.

| Fall from peak | Documented intent | What the code returns |
| -------------- | ----------------- | --------------------- |
| -0.01 (1%)     | 1.00              | 0.25                  |
| -0.04 (4%)     | 0.75              | 1.00                  |
| -0.08 (8%)     | 0.50              | 1.00                  |
| -0.12 (12%)    | 0.25              | 1.00                  |

Now combine that with the volatility target. On a day when the realised volatility is 20 percent,
`raw_position = 0.12 / 0.20 = 0.60`, so the volatility position is 0.60. If the price is 8 percent
below its peak, the ladder returns 1.00, and the combined target is `min(1.00, 0.60) = 0.60`. Suppose
yesterday's smoothed position was 0.50; the new smoothed position is
`0.50 * (1 - 0.15) + 0.60 * 0.15 = 0.425 + 0.090 = 0.515`. That is 0.015 above the current position,
less than the 0.05 band, so no trade happens. On a day when the volatility position falls to 0.30, the
combined target is `min(1.00, 0.30) = 0.30`, the smoothed value becomes
`0.50 * 0.85 + 0.30 * 0.15 = 0.425 + 0.045 = 0.470`, a move of 0.03, still inside the band. Only when
the target moves by more than 0.05 does the account trade.

Second, the monthly moving-average switch over six invented months, starting with an account of
1,000,000. The ten-month average is the value the last ten closes produce.

| Month | Close  | Ten-month average | Close below? | Risk mark | Target this month | Position  |
| ----- | ------ | ----------------- | ------------ | --------- | ----------------- | --------- |
| 1     | 100.00 | 95.00             | no           | 0         | 1.00              | 1,000,000 |
| 2     | 102.00 | 96.00             | no           | 0         | 1.00              | 1,000,000 |
| 3     | 104.00 | 97.00             | no           | 0         | 1.00              | 1,000,000 |
| 4     | 103.00 | 98.00             | no           | 0         | 1.00              | 1,000,000 |
| 5     | 98.00  | 99.00             | yes          | 1         | 1.00              | 1,000,000 |
| 6     | 92.00  | 99.00             | yes          | 1         | 0.50              | 500,000   |

Month 5 is the first month with the close below the average, but the holding is set from the previous
month's mark, which was normal, so the position stays full. Month 6 uses month 5's mark, so the
position halves to 500,000. That one-month delay is deliberate: it guarantees the decision never uses
a close that had not yet happened. The cost of halving and later restoring is one commission of 0.02
percent on the traded half, which is `500,000 * 0.0002 = 100`.

## What the research actually found

The category article reports what each test produced on gold from 2008 to 2025, with commission at
0.02 percent. The drawdown ladder system over 4,618 daily bars, with 289 rebalances, finished at
2,732,100.12 against a starting 1,000,000, a gain of 173.21 percent, a reward-to-risk ratio of 0.616,
and a worst fall of 31.43 percent. The monthly moving-average switch over 216 months finished at
3,806,875.01, a gain of 280.69 percent, with a reward-to-risk ratio of 0.555 and a worst fall of
39.41 percent. The risk-on, risk-off switch finished at 3,881,633.30, a gain of 288.16 percent, with
a reward-to-risk ratio of 0.746 and the best drawdown control of the group at 19.44 percent.

The article also reports the monthly switch's own accounting, which is the most useful part: of 216
months, 68 were risk months, about 31 percent; there were 32 switches. Of the 24 months that lost at
least 5 percent, 16, or about two thirds, occurred while the price was below the moving average. So
the switch really did keep most of the worst months out, and even so the worst fall was 39.41 percent
against gold's 2011 to 2015 decline. Halving a position softens a bear market; it does not immunise
against it.

The research behind volatility targeting is separate and worth stating plainly: sizing positions so
that the risk budget stays constant is a well-studied institutional technique, and the study in this
repository's own brief finds that volatility targeting alone is not enough and that a
downside-focused version earns a higher reward-to-risk ratio, 0.94 against 0.87, with a similar worst
fall (`2510.19271v2`, p.18). The difference is small, and the point is that these are risk choices,
not sources of return.

One habit applies to every number above. Every backtest in the compendium asserts its final portfolio
value, its reward-to-risk ratio and its worst fall against a baseline recorded when the test was
migrated. Passing that assertion proves the engine computes exactly what the file says, in both its
modes, and nothing more. In this category the distinction is sharpest: the drawdown ladder test
asserts the system's actual behaviour, bug included, which is exactly what an assertion should do, and
it says nothing about whether that behaviour is wise.

## How this project relates to it

The stop and barrier forms behind this category are the subject of
[Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md), which writes them as
an explicit layer, off by default, and insists that the layer's effect be reported separately rather
than called a source of return. The sizing side is covered by
[Position sizing](../../project/position-sizing/README.md), which explains the risk-budget rule and
why capacity limits bind it. Both draw on this repository's own engine design,
[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), whose Section 7
gives the risk-budget size `Q = (A * r) / (P_entry - P_stop)` and whose Section 9 requires the risk
layer's effect to be reported with the win rate and both tails of the trade distribution, because a
stop mechanically raises the win rate and truncates both tails.

The engine design is clear that sizing cannot manufacture an edge: execution can only make a decision
more expensive, and a risk layer applies to a directional overlay only when a regime admits one. The
category here is a catalogue of the mechanisms, not a promise about them.

## Where it goes wrong

- A bug can pass every assertion. The drawdown ladder here returns the wrong position at four of the
  threshold levels, and the test passes because it records what the code does, not what the
  documentation says. Anyone reading only the descriptions would misunderstand the system.
- Sizing down does not remove a fall. The monthly switch kept two thirds of the worst months out and
  the account still fell 39.41 percent, because gold fell far and the system still held half.
- Overlapping rules can cancel. The drawdown ladder never returns the deeper tiers, so most of the
  risk control in that test comes from the volatility target alone. A reader who assumes both layers
  are working would overestimate the system.
- Volatility targeting is a risk choice, not a return source. It changes how steady the returns are
  and how deep the falls go, and the evidence does not show it adds return.
- Costs and bands are tuned. Rebalancing too often pays commission for nothing, and the 2 percent and
  5 percent bands are chosen, not derived; a wider band trades less but reacts later.
- The sample is narrow. Gold over 2008 to 2025 is one market in one era, and these rules were fitted
  and checked on it. A rule that held on gold in that window is not evidence it holds elsewhere.

## Try it yourself

You need a spreadsheet and a column of daily closing prices for one thing, gold or a share index.

1. Build columns for the date and the close.
2. Add a drawdown column: the running highest close so far, minus today's close, divided by that
   running high.
3. Add a column that marks each month's last close, and a second column with the average of the last
   ten such closes.
4. Add a holding column: write 100 on months whose close is above the ten-month average and 50 on
   months whose close is below it, then shift that column down by one month.
5. On a second sheet, take a starting amount of 100 and apply the holding each month: multiply
   yesterday's value by the holding fraction plus the month's return times that fraction, and subtract
   a cost of 0.02 percent whenever the holding changes.

What to notice: the holding changes very rarely, so the cost is almost nothing, and the strategy mostly
follows the market with half the exposure during declines. Compare the worst fall of this column with
the worst fall of holding the whole time; the rule usually softens it, and never removes it.

## Where this came from

- [Strategy Compendium, article 14, risk_management](https://backtrader.readthedocs.io/en/latest/strategies-series/en/14-risk-management.html),
  the category inventory, the two deep dives, the disclosed coding issue and the reported baselines.
- [Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md), Sections 7 and
  9, for the risk-budget sizing rule and the requirement that the risk layer not be called alpha.
- [Risk boundaries and stops](../../project/risk-boundaries-and-stops/README.md) and
  [Position sizing](../../project/position-sizing/README.md), this collection's tutorials on the same
  two ideas as separate layers.
- [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
  for the downside-quantile sizing result (`2510.19271v2`) and the fragility of estimated inputs.
- [Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md),
  for why a drawdown measure is model-free but path-dependent.

## Words used in this tutorial

- volatility targeting: sizing a position so that the amount of movement the account experiences stays
  roughly constant.
- drawdown: the fall from a peak in value to a later low, as a percentage of the peak.
- position sizing: deciding how much of the account to hold in one bet.
- leverage: using borrowed money so a given price move produces a larger gain or loss.
- rebalance: adjusting a holding back toward its intended size.
- band: a small threshold around a target, used so that tiny differences do not trigger trades.
- hedge: a second position taken to reduce the risk of the first.
- tail risk: the chance of a rare, large loss that sits far from the usual behaviour.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
