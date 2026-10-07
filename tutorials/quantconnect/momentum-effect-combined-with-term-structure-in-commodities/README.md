# Momentum inside term structure: choosing the best and worst raw materials within the cheap and dear groups

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                         |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Futures contracts, which are agreements to buy or sell a raw material at a fixed price on a future date, over a basket of about twenty materials                                                                              |
| How often it trades       | About once a month, when the holding list is rebuilt                                                                                                                                                                          |
| What you need             | A spreadsheet and, for each raw material, two futures prices with delivery dates plus a month of daily prices                                                                                                                 |
| Where the rules come from | [QuantConnect strategy library, momentum effect combined with term structure in commodities](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-combined-with-term-structure-in-commodities)             |
| The underlying research   | Fuertes, Miffre and Rallis, [Tactical Allocation in Commodity Futures Markets](https://ssrn.com/abstract=1127213)                                                                                                             |
| How well it held up       | Mixed: in the original sample the combined signal nearly doubled either signal alone and withstood costs, but later work finds it much weaker in markets flooded with outside money, and it rests on a costly monthly rebuild |
| Also appears in           | [Commodity momentum](../momentum-effect-in-commodities-futures/README.md) and [commodity term structure](../term-structure-effect-in-commodities/README.md), the two halves traded separately                                 |

## The idea in one paragraph

Two ideas about raw materials are used together. The first is term structure: whether the futures
price for delivery later is above or below the price for delivery soon. The second is momentum:
whether the price has been rising or falling over the past month. This strategy sorts the materials by
term structure, keeps the cheapest-to-hold third and the most expensive-to-hold third, and throws the
middle away. Inside the cheap-to-hold group it buys only the ones that also rose over the past month,
and inside the expensive-to-hold group it sells only the ones that also fell. It holds those positions
for a month and then rebuilds. The bet is that the two signals together pick out materials where both
the structure of prices and the recent direction agree.

## Why anyone believed it

Each of the two signals has its own economic story, and the argument here is that they reinforce each
other. The term-structure story is that raw-material producers want to lock in future prices, and the
gap between delivery dates is the payment a speculator receives for taking that risk off their hands.
The momentum story is that news about a raw material arrives gradually: a harvest report, a pipeline
closure or a run of cold weather is absorbed by traders over days and weeks rather than instantly, and
buyers who are slow to react keep pushing the price after the first move.

Combining them is meant to select the strongest cases. A material that is expensive to hold but has
been rising is a mixed signal, so the rule ignores it and keeps the middle group out of the
portfolio. A material that is cheap to hold and has also been rising is the case where the insurance
payment and the trend point the same way, and those are the ones the rule buys. The counterparty is
the hedger who is transferring risk and the slower trader who reacts to the news late.

## An everyday comparison

Think of a second-hand car auction that runs once a month. Some cars are being sold by owners who
must get rid of them quickly and will accept a low price, and some by owners who are in no hurry and
ask a lot. A buyer who only looks at the asking price cannot tell which is which. But if a car is
being offered cheaply and has also been sitting in the lot getting more attention, the two pieces of
evidence agree that the seller is keen. The rule here does the same: buy raw materials that are
cheap to hold and have been firm, and bet against the ones that are expensive to hold and have been
weak.

## The rules, step by step

1. Choose a basket of raw materials with long futures histories, for example about twenty across
   energy, metals and farm products. The library uses twenty-two.
2. For each material, find the contract nearest to its delivery date and the one furthest away, and
   compute the roll return from their two prices and their delivery dates, exactly as in the term
   structure tutorial. A positive roll return means later delivery is cheaper, so holding the material
   pays; a negative one means it costs.
3. Rank all the materials by roll return, best first, and split them into three equal groups: the
   highest third, the middle third and the lowest third labelled High, Med and Low. Discard the middle
   third entirely.
4. For every remaining material, compute its momentum: the average of its daily percentage price
   changes over the past month, about twenty-one trading days.
5. Within High, sort by momentum and keep the best half. These are the High-Winners, materials that
   are cheap to hold and have been rising. Buy them in equal amounts.
6. Within Low, sort by momentum and keep the worst half. These are the Low-Losers, materials that are
   expensive to hold and have been falling. Sell them short in equal amounts, meaning you promise to
   sell contracts you do not own and buy them back later.
7. Give the long side and the short side the same total size, hold for one month without looking in
   between, then close everything and repeat from step 2.

In the library code the ranking is done on individual contracts rather than on whole materials, so a
material with several listed delivery dates can contribute more than one row to the sort. The recipe
above is written per material so that a reader with one price series per material can follow it.

## The maths, with every symbol named

The roll return of one material, measuring whether holding it pays or costs:

```text
R = ( ln(P_near) - ln(P_far) ) * 365 / (D_far - D_near)
```

- `R` is the roll return for a whole year, written as a decimal, so 0.10 means 10 percent a year.
- `P_near` and `P_far` are today's prices of the nearest and furthest futures contracts.
- `D_near` and `D_far` are the days from today to each delivery date.
- `ln` is the natural logarithm, a spreadsheet function that turns a price ratio into a difference.
- A positive `R` means the later contract is cheaper, so rolling a position forward pays.

The momentum of one material:

```text
M = average of ( today's price / yesterday's price - 1 ) over the last 21 trading days
```

- `M` is the momentum score, written as a decimal per day, so 0.002 means about 0.2 percent a day.
- The daily changes are averaged rather than multiplied, so `M` is a typical daily move, not a total.

The groups and the weights:

```text
High = the best third by R, Low = the worst third by R, Med = the rest and is not traded
long set  = the best half of High by M
short set = the worst half of Low by M
w_i = +0.5 / n_long for each long, and w_i = -0.5 / n_short for each short
```

- `n_long` and `n_short` are the numbers of materials in the long and short sets.
- `w_i` is the fraction of the money placed in material `i`; the long weights add up to 0.5 and the
  short weights add up to minus 0.5, so the two sides are the same size and the direction of the
  commodity market as a whole is mostly cancelled out.

The portfolio's return over the next month and the cost of rebuilding:

```text
R_portfolio = sum over held materials of ( w_i * r_i )
Cost = t * c
```

- `r_i` is the next-month return of material `i`, its price at the end of the month divided by its
  price at the start minus one, signed so a short gains when the price falls.
- `t` is the traded fraction of the account, about 2.0 when the whole book turns over; `c` is the cost
  of one trade as a fraction of the amount traded, around 0.0005 to 0.001, that is five to ten basis
  points, where one basis point is one hundredth of one percent.

## A worked example

Twelve raw materials, with invented but plausible numbers. The roll return comes from the first
column's two prices and the days apart; the past-month momentum is given directly.

| Material    | P_near  | P_far   | Days apart | Roll return R | Past-month M | Group |
| ----------- | ------- | ------- | ---------- | ------------- | ------------ | ----- |
| Silver      | 24.00   | 23.40   | 60         | +0.1540       | +8 percent   | High  |
| Copper      | 4.20    | 4.10    | 60         | +0.1466       | +5 percent   | High  |
| Coffee      | 1.60    | 1.55    | 90         | +0.1288       | +7 percent   | High  |
| Heating oil | 2.40    | 2.34    | 90         | +0.1027       | +3 percent   | High  |
| Zinc        | 1.30    | 1.28    | 60         | +0.0943       | -3 percent   | Med   |
| Sugar       | 0.22    | 0.215   | 90         | +0.0932       | +2 percent   | Med   |
| Wheat       | 6.10    | 6.05    | 120        | +0.0250       | +1 percent   | Med   |
| Gold        | 1900.00 | 1915.00 | 60         | -0.0479       | -2 percent   | Med   |
| Soybeans    | 13.50   | 13.90   | 120        | -0.0888       | -6 percent   | Low   |
| Corn        | 4.80    | 4.95    | 120        | -0.0936       | -4 percent   | Low   |
| Crude oil   | 80.00   | 84.00   | 90         | -0.1979       | +6 percent   | Low   |
| Natural gas | 2.90    | 2.98    | 45         | -0.2207       | -10 percent  | Low   |

The highest third is Silver, Copper, Coffee and Heating oil. Within that group the best two by
momentum are Silver at +8 percent and Coffee at +7 percent, so the strategy buys those. The lowest
third is Soybeans, Corn, Crude oil and Natural gas. Within that group the worst two by momentum are
Natural gas at -10 percent and Soybeans at -6 percent, so the strategy sells those short. Each of the
four positions gets a quarter of the account.

| Position           | Weight | Next-month price change | Contribution    |
| ------------------ | ------ | ----------------------- | --------------- |
| Silver, long       | +0.25  | +4 percent              | +1.0000 percent |
| Coffee, long       | +0.25  | +1 percent              | +0.2500 percent |
| Natural gas, short | -0.25  | -3 percent              | +0.7500 percent |
| Soybeans, short    | -0.25  | +2 percent              | -0.5000 percent |
| Total              |        |                         | +1.5000 percent |

The portfolio gained 1.5000 percent before costs. Notice that the soybean short lost money even though
soybeans are in the expensive-to-hold group: the momentum half of the rule is not a guarantee, and a
short position gains only when the price actually falls.

Now the cost. Assume the whole book is closed and reopened each month, so `t = 2.0`, and take ten
basis points per trade, `c = 0.001`:

```text
Cost = 2.0 * 0.001 = 0.002, that is 0.20 percent
Net return for the month = 1.5000 - 0.20 = 1.30 percent
```

Twelve months at that rate is about 19.6 percent a year, but that figure is a property of the invented
numbers. The same arithmetic with the next-month moves reversed loses money. The example shows only
how the two sorts are applied and how the arithmetic behaves.

## What the research actually found

| Source                                         | What it measured                                                                               | Result                                                                                                                                                                             |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fuertes, Miffre and Rallis                     | Commodity futures from 1979 to 2004, momentum alone, term structure alone, and the double sort | Momentum alone about 10.14 percent a year and term structure alone about 12.66 percent, while the double sort that combines them reached an abnormal return of about 21.02 percent |
| The same paper                                 | Whether the double-sort result is an artefact                                                  | The authors report it cannot be explained by lack of liquidity or by data mining and that it survives transaction costs and different risk definitions                             |
| Zaremba                                        | Commodity markets sorted by how much outside money they contain                                | Both the momentum and the term-structure strategies performed better in markets with little investor participation and earned little where participation was high                  |
| Switzer and Jiang                              | Winner-and-loser commodity portfolios                                                          | Profits from a popular twelve-month momentum rule dissipated once the influence of hedger positioning was accounted for                                                            |
| Quantpedia, on the related term structure page | The carry return against share-market stress                                                   | The term-structure side earns least exactly when share-market volatility jumps, so the combined book inherits that weakness on its short and long carries                          |

Read together: the combination was a strong result in the sample where it was discovered, roughly
doubling either signal alone, and the authors defend it against the obvious criticisms. But the later
work points the same way as for the two signals separately, that the edge is concentrated in markets
and periods with less outside money, and that a monthly rebuild spends a real fraction of the return
on costs.

## How this project relates to it

This repository has no combined momentum-and-term-structure strategy. The closest material is in
[Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md), a research
brief built from a harvest of academic papers. Its section on curves beyond power reports that a
level, a slope and a curvature term fitted to 21 commodity futures explained 96.5 percent of the
change in the price curves, and that trading the continuing direction of the slope, a curve signal
close in spirit to the term-structure half of this rule, earned 1.77 percent a year before costs,
which fell to between 0.55 and 1.31 percent after costs and weakened sharply after the year 2000.
That is the same warning the momentum-and-carry papers give: the raw signal is visible in the data,
and what is left after costs and after the market fills with outside money is much smaller.

## Where it goes wrong

- Financialisation. As more outside money runs these rules, the returns arrive earlier and smaller,
  and the studies above find the effect weakest in exactly the markets that most outside money has
  entered.
- Two signals, one trade. When term structure and momentum disagree, the rule discards the material,
  so the portfolio can be empty or very concentrated, and a single bad reading in either signal moves
  a large weight.
- Monthly costs. The book turns over often, and each rebuild pays the buying-and-selling gap on both
  legs; the example's 0.20 percent a month is already a fifth of a typical monthly gain.
- The carry side fails in stress. The short side of the book can lose heavily when a shortage drives
  the price of a material that was expensive to hold sharply higher, which is the moment the insurance
  logic turns against the holder.
- Data choices. The ranking depends on which contract is called the distant one and on how the roll
  return is annualised, and different reasonable choices change which materials land in High and Low.
- The number of rules tried. Splitting into thirds, halves and a one-month window is one of many
  plausible arrangements, and choosing the arrangement after seeing the results is how a record like
  this one is made to look better than it is.

## Try it yourself

You need a spreadsheet and a public source of futures prices with several delivery months. Pick six
raw materials.

1. Build a sheet with columns: name, price of the nearest contract, price of the furthest contract,
   days to each delivery, and roll return using the formula above. Add a second sheet with one row
   per trading day per material and a column of daily percentage changes.
2. Compute each material's momentum as the average of its last twenty-one daily changes.
3. Sort by roll return and mark the top two and bottom two, ignoring the middle two.
4. For each marked material, write its momentum next to it. The one to buy is the marked High name
   with the higher momentum; the one to sell is the marked Low name with the lower momentum.
5. Track the price of each for the next month and work out the pair's return, then subtract 0.20
   percent for the round trip.

What to notice: the two sorts often pull in different directions, and some months the rule wants to
hold nothing at all. When the two agree, the positions tend to be concentrated in a handful of
materials, which is exactly when a single surprise matters most.

## Where this came from

- [QuantConnect strategy library: momentum effect combined with term structure in commodities](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-combined-with-term-structure-in-commodities),
  the rules as implemented: a third by roll return, then a half by one-month momentum, held a month.
- Fuertes, Miffre and Rallis, [Tactical Allocation in Commodity Futures Markets: Combining Momentum
  and Term Structure Signals](https://ssrn.com/abstract=1127213), the paper the rules are built from,
  including the 10.14, 12.66 and 21.02 percent figures.
- [Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md), this
  repository's own brief on commodity curves, for the section on curves beyond power.
- [Commodity momentum](../momentum-effect-in-commodities-futures/README.md) and [commodity term
  structure](../term-structure-effect-in-commodities/README.md) in this collection, the two single-signal
  cousins of this rule.

## Words used in this tutorial

- contango: the ordinary case where a futures contract for later delivery costs more than one for
  sooner delivery.
- backwardation: the unusual case where a contract for later delivery costs less than one for sooner
  delivery.
- futures: an agreement made today to buy or sell something at a fixed price on a future date.
- momentum: the tendency of something that has been rising to keep rising for a while.
- roll return: the profit or cost of selling an expiring futures contract and buying a later one.
- short: selling something you do not own yet and buying it back later, so the position gains when
  the price falls.
- tertile: one of three equal groups sorted from smallest to largest.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
