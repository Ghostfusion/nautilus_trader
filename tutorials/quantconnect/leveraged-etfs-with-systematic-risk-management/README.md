# A leveraged index fund held only while the trend is up

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                           |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A fund that aims to double the daily move of the American share index, and a fund of short-term government bonds for the days it is out                                                                                                         |
| How often it trades       | A few times a year, only when the fund's price crosses its own average                                                                                                                                                                          |
| What you need             | A spreadsheet                                                                                                                                                                                                                                   |
| Where the rules come from | [QuantConnect strategy library, leveraged ETFs with systematic risk management](https://www.quantconnect.com/tutorials/strategy-library/leveraged-etfs-with-systematic-risk-management)                                                         |
| The underlying research   | Gayed and Bilello, [Leverage for the Long Run](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2741701)                                                                                                                                     |
| How well it held up       | Mixed: one five-year test shows a small edge over holding the index, while a later study of a century of data finds that trend filters raise both the return and the risk of leveraged funds, leaving the reward for the risk roughly unchanged |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                                                 |

## The idea in one paragraph

A leveraged fund is a basket of shares that aims to move twice as much as an index each day, so it
rises faster than the index when the market rises and falls faster when the market falls. Holding it
through a long decline is what turns a temporary fall into a permanent loss, because the fund itself
does not recover its losses the way the index can. This strategy holds the fund only while its price
is above its own average over the past two hundred trading days, and swaps into short-term government
bonds whenever the price drops below that average. The average is the only switch: above it, the fund
is held; below it, the money waits in bonds. The claim is that a slow average captures whether the
market is in a rising phase and steps aside during the long falling phases, when the doubling hurts
most.

## Why anyone believed it

The story rests on the idea that markets fall in long, drawn-out phases rather than all at once, and
that a slow average detects those phases early enough to matter. When an ordinary investor sees
shares fall, the instinct to sell is usually strongest near the bottom, after months of losses; the
average rule sells earlier, and it sells mechanically, which is a way of replacing the investor's
nerve with a rule.

The counterparty is the investor who freezes, the fund forced to sell into a falling market, and the
trader who borrows to buy on the way down and is later forced to liquidate. The rule also has a second
source of comfort: by stepping out of the leveraged fund during declines, and into government bonds,
it reduces the violent swings that the doubling produces. Since the 1960s people have claimed that
moving averages add value; the part specific to this strategy is that the value is supposed to come
from avoiding the crashes rather than from catching the rises.

## An everyday comparison

Think of driving up a mountain road in fog. The speed limit is set for clear weather, but a careful
driver goes slower when the road curves and speeds up only when the road ahead is straight and
visible. The driver is not predicting the weather; a long, straight stretch of road is a fact now,
not a forecast. The moving average is that stretch of straight road: two hundred days of prices
rising is a statement about the recent past, and this strategy stays fast only while it lasts. The
moment the road bends, the driver slows down, even though the bend may turn out to be nothing.

## The rules, step by step

1. Choose two funds. The library page uses SSO, which aims to double the daily move of the S&P 500
   share index, and SHY, which holds American government bonds of one to three years.
2. Each day, compute the two hundred day simple moving average of SSO's price: add the closing prices
   of the last two hundred trading days and divide by two hundred.
3. If today's closing price of SSO is above that average, hold SSO with all of the money.
4. If today's closing price is below the average, sell SSO and hold SHY instead.
5. When the signal changes, rotate: sell the fund being held and buy the other one.
6. Do nothing in between. The average is deliberately slow, so most days bring no change, which keeps
   the number of trades, and the cost, low.
7. Pay the cost of every rotation, described below, including the gap between buying and selling
   prices.

The library page also offers a refinement: instead of all the money in one fund, hold the same
position in a leveraged fund as in the ordinary index but use cash so that the risk matches. It is
not needed to see the idea, and it does not change the switch.

## The maths, with every symbol named

The average that drives the switch:

```text
SMA(t) = (P(t) + P(t - 1) + ... + P(t - 199)) / 200
```

- `SMA(t)` is the two hundred day simple moving average on day `t`.
- `P(t)` is the closing price of the leveraged fund on day `t`, and the other terms are the prices on
  the previous days.

The daily return of the leveraged fund:

```text
r_L(t) = beta * r_I(t) - f
```

- `r_L(t)` is the fund's return on day `t`, in decimals.
- `r_I(t)` is the index's return on day `t`.
- `beta` is the leverage, two for SSO.
- `f` is the fund's daily fee and financing cost, which is deducted each day. For an annual fee of
  0.9 percent, `f` is about 0.9 percent divided by 252, or 0.0036 percent a day.

Over several days the fund does not simply give `beta` times the index's total return. The fund's
value follows the product of its daily returns, while the index follows the product of its own:

```text
R_L = product over t of (1 + r_L(t)) - 1
R_I = product over t of (1 + r_I(t)) - 1
```

- `R_L` and `R_I` are the cumulative returns of the fund and the index over the same days.

The effective leverage over the window is the ratio of the two, and it is the number that replaces
the promise of two:

```text
EL = R_L / R_I
```

- `EL` is the effective leverage actually delivered over the window rather than over one day.

The cost of a rotation is the cost rate multiplied by the value traded. A rotation trades twice, once
out and once in:

```text
Cost = 2 * c
```

- `Cost` is the cost of one rotation, as a fraction of the account.
- `c` is the cost of one one-way trade, covering the gap between the buying and selling price and any
  commission, about 0.0005 for a large index fund.

## A worked example

Eight days of made-up but plausible daily moves. The index starts at 100 and the leveraged fund at
60. The fund's daily return is shown as exactly twice the index's; its annual fee, about 0.0036
percent a day, is left out of the table because over eight days it is only about 0.03 percent. The
average that governs the switch is fixed at 59.00 for this illustration, which stands for a slow
average that has barely moved.

| Day | Index return | Index price | Fund return | Fund price | Above 59.00? | Held  |
| --- | ------------ | ----------- | ----------- | ---------- | ------------ | ----- |
| 1   | +3.0 percent | 103.000     | +6.000 pct  | 63.600     | yes          | fund  |
| 2   | -4.0 percent | 98.880      | -8.000 pct  | 58.512     | no           | fund  |
| 3   | +2.0 percent | 100.858     | +4.000 pct  | 60.852     | yes          | bonds |
| 4   | -1.0 percent | 99.849      | -2.000 pct  | 59.635     | yes          | fund  |
| 5   | +3.0 percent | 102.845     | +6.000 pct  | 63.213     | yes          | fund  |
| 6   | -2.0 percent | 100.788     | -4.000 pct  | 60.684     | yes          | fund  |
| 7   | +1.0 percent | 101.796     | +2.000 pct  | 61.898     | yes          | fund  |
| 8   | +2.0 percent | 103.832     | +4.000 pct  | 64.374     | yes          | fund  |

The decision for each day is made at the previous close, so the fund is sold after day 2, when its
price of 58.512 sits below 59.00, and bought back after day 3, when it is back above. That is the
whole switch in this window.

Now the arithmetic. The index went from 100 to 103.832, a return of +3.832 percent. The fund went
from 60.000 to 64.374, a return of +7.290 percent. The effective leverage was
`7.290 / 3.832 = 1.902`, so the fund delivered about 1.9 times the index, not two. A fund that really
doubled the whole window's return would have given +7.664 percent; the difference, 0.374 percentage
points over eight days, is the drag from compounding daily returns.

The moving-average rule held the fund for seven of the eight days and sat in bonds on day 3, when the
fund would have gained 4 percent. Its return was therefore the fund's return without day 3:

```text
(1.06 * 0.92 * 0.98 * 1.06 * 0.96 * 1.02 * 1.04) - 1 = +3.16 percent
```

The rule rotated out and back in once, which is two rotations, or four one-way trades:

```text
Cost = 4 * 0.0005 = 0.0020, that is 0.20 percent
Net = 3.16 - 0.20 = 2.96 percent
```

So in this made-up week the fund returned +7.29 percent, the index +3.83 percent, and the rule
+2.96 percent. The rule did worse than the fund because it missed a good day. That is not a mistake in
the arithmetic; it is the honest behaviour of a slow filter, which gives up some good days in exchange
for stepping aside during long declines. In an eight-day window no long decline appears, so the rule
only pays its cost. The example says nothing about whether the strategy works over years; it only
shows how to apply the rules and why daily leverage is not the same as borrowing to hold.

## What the research actually found

| Source                                                | What it measured                                                   | Result                                                                                                                                                                                                                                       |
| ----------------------------------------------------- | ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Gayed and Bilello, the source of the rules            | A moving-average risk rule on a leveraged index fund, long history | The rule is claimed to reduce the depth of the worst falls substantially compared with holding the fund, while keeping most of the rise                                                                                                      |
| QuantConnect's own implementation                     | SSO and SHY, a five-year test                                      | Reward-to-risk 0.555 against 0.524 for simply holding the index, a small edge over a short window                                                                                                                                            |
| Quantpedia, asset allocation study of leveraged funds | Simulated double-leveraged funds, 1926 to 2025                     | Replacing plain funds with leveraged ones worsened risk-adjusted results in every portfolio tested; adding a ten-month moving-average filter raised both the return and the volatility of the leveraged fund, leaving reward-to-risk similar |
| Hsieh, Chang and Chen                                 | About twenty years of daily data, with a model of compounding      | The fund's cumulative return beats a simple doubling of the index when the market has been trending, but falls short when the market has been swinging back and forth; fees and trading costs take about 0.8 to 1.0 percentage points a year |

The honest reading is this. The moving-average rule does reduce the worst falls, which is what it was
designed to do and what the source paper reports. But whether that translates into a better reward
for the risk taken depends on the sample: the five-year test shows a small edge, and the century-long
study finds the filter raising return and volatility together, so the reward for the risk stays
roughly the same. The grade is Mixed because the main selling point, protection in crashes, is
documented, while the advantage over simply holding the index is small and not robust across samples.
Nothing here says the strategy would make money for a reader.

## How this project relates to it

This repository does not implement a leveraged-fund strategy. The closest thing it has is a design
note,
[strategies/entry_exit_engine_design.md](../../../strategies/entry_exit_engine_design.md). Its
Section 2 puts moving-average rules and volatility-scaled stops in a category it calls take, gated:
position management that is only meaningful when a market regime admits a directional overlay, and
its Section 3 states that any overlay is admitted only on significant, material, out-of-sample
evidence. That is a deliberately stricter standard than a five-year test, and it is the right way to
read the table above.

The second related piece is
[strategies/books2/10_portfolio_and_allocation.md](../../../strategies/books2/10_portfolio_and_allocation.md).
Its Section 7 reports that a downside-aware allocation policy earned a reward-to-risk of 0.94 with a
21.28 percent worst fall, against 0.87 and 21.39 percent for the unmanaged benchmark. The lesson
carries over: a risk-management overlay is judged by what it does to the worst fall and to turnover,
not by the return alone.

## Where it goes wrong

- The average is slow. A two hundred day average turns only after a fall has been going for months,
  so the rule gives back the first part of every decline. In a market that falls and recovers quickly,
  it sells near the bottom and buys back higher.
- Daily leverage is not holding borrowed stock. The fund resets its borrowing every day, so a path
  that goes up and then down leaves the fund below where a simple doubling would put it. The worked
  example shows 1.9 times instead of two over just eight days.
- Costs and taxes on a rotation. Each switch trades the whole account twice. A rule that switches
  often in a choppy market pays for the privilege, and in a taxable account the switches can also
  create a tax bill.
- The rule was tested on a favourable sample. Five years ending in a long bull market will make any
  trend filter look good; the source paper's own long history and the later study are a better guide,
  and they disagree.
- The bond fund can fall too. Short-term government bonds can lose money when interest rates rise, so
  the safe parking place is not perfectly safe.
- The whole idea fails if markets stop producing long declines. The value of the rule comes from
  stepping aside during drawn-out falls; if central banks lean against such declines, or if crashes
  become shorter, the rule pays its costs without earning its insurance.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily prices for one leveraged fund and its
index, for example SSO and the S&P 500.

1. Build a sheet with one row per day for ten years: the date, the index level, and the fund's price.
2. Add a column for the index's daily return and a column for twice that return, less a daily fee of
   0.0036 percent. That is a model of the fund before its own compounding.
3. From both, build two price paths starting at the same number, one compounding the index each day
   and one compounding the modelled fund.
4. Add a column for the two hundred day average of the modelled fund: the average of the last two
   hundred rows.
5. Add a column for the decision: hold the fund if its price is above that average, otherwise hold
   cash.
6. Build a third path that follows the fund only on the days you decided to hold it, and earns
   nothing on the other days.
7. Finally, count the switches, and subtract 0.20 percent for each rotation.

What to notice: in a long, steep fall the third path falls far less than the fund, which is the whole
point. But over a whole decade the three paths often end up surprisingly close, because the rule
misses part of the recoveries. If your third path wins by a wide margin, check that the average was
computed from prices available before the decision, not after.

## Where this came from

- [QuantConnect strategy library: leveraged ETFs with systematic risk management](https://www.quantconnect.com/tutorials/strategy-library/leveraged-etfs-with-systematic-risk-management),
  the rules as implemented: SSO, SHY, the two hundred day average, the rotate-out rule.
- Gayed and Bilello, [Leverage for the Long Run](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2741701),
  the source of the rules.
- Hsieh, Chang and Chen, [Compounding Effects in Leveraged ETFs](https://arxiv.org/abs/2504.20116),
  the worked eight-day arithmetic is the same idea as its Example 2.1 (p.3), and its empirical
  section reports where leveraged funds beat or fall short of their target, cited here from
  `2504.20116v1`.
- [Quantpedia: leveraged ETFs in asset allocation, opportunity or trap](https://quantpedia.com/leveraged-etfs-in-asset-allocation-opportunity-or-trap/),
  the century-long study of leveraged funds with and without trend filters.
- [strategies/entry_exit_engine_design.md](../../../strategies/entry_exit_engine_design.md), this
  repository's design note on when a moving-average or stop overlay is admitted.

## Words used in this tutorial

- drawdown: the fall from a peak to the following low, measured in percent.
- ETF: a fund whose shares trade on an exchange like a single share.
- leverage: using borrowed money, or the equivalent, so that a price move is magnified.
- moving average: the average of the last fixed number of prices, updated as new prices arrive.
- rotation: selling one holding and buying another, usually on the same day.
- slippage: the difference between the price you expected and the price you actually got.
- volatility: how much a price moves around its average, usually quoted per year.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
