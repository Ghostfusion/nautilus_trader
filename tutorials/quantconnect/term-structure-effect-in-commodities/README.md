# Commodity term structure: getting paid to hold a raw material for later delivery

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                  |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Futures contracts, which are agreements to buy or sell a raw material at a fixed price on a future date, over a basket of such materials                                                                                                                               |
| How often it trades       | About once a month, when the holding list is rebuilt                                                                                                                                                                                                                   |
| What you need             | A spreadsheet and, for each raw material, two futures prices and the two delivery dates                                                                                                                                                                                |
| Where the rules come from | [QuantConnect strategy library, term structure effect in commodities](https://www.quantconnect.com/tutorials/strategy-library/term-structure-effect-in-commodities) and the [Quantpedia entry](https://quantpedia.com/strategies/term-structure-effect-in-commodities) |
| The underlying research   | Fuertes, Miffre and Rallis, [Tactical Allocation in Commodity Futures Markets](https://ssrn.com/abstract=1127213)                                                                                                                                                      |
| How well it held up       | Mixed: measured in several long samples with a positive average, but the long-and-short version fell about 78 percent, it performs worst when share volatility jumps, and implementers find the profit concentrated before about 2007                                  |
| Also appears in           | The combined momentum-and-term-structure tutorial in this collection, which trades this signal together with momentum                                                                                                                                                  |

## The idea in one paragraph

Raw materials are traded through futures, which are agreements to buy or sell something at a fixed
price on a future date. On any day there is a price for delivery soon and a price for delivery later.
Usually the later delivery costs more, because someone has to store the material and wait for the
money. Sometimes the later delivery costs less, because the material is scarce right now. This
strategy sorts a basket of raw materials by which of those two situations holds, buys the ones where
delivery later is cheapest relative to delivery soon, and sells the ones where it is most expensive.
It holds those positions for a month and then rebuilds the list. The bet is that the gap between the
two delivery prices is a fair payment for taking on the risk that material producers want to unload.

## Why anyone believed it

A producer of a raw material, say a wheat farmer or an oil company, wants to know today what price
they will receive at harvest. To get that certainty they sell a futures contract; in market language
they go short, which means they promise to sell later at a price fixed now. A consumer, say a bakery
or a refinery, wants the opposite certainty and buys, or goes long. When producers and consumers do
not want the same delivery dates in the same amounts, a speculator has to stand on the other side.

The economist John Maynard Keynes argued in 1930 that when producers are the more eager side, the
price for later delivery must be biased downward: the futures price is set below where the material
is expected to be, so that whoever holds the contract for the producer is paid to do so. That payment
is the gap between the delivery-soon price and the delivery-later price. If the argument is right,
the buyer of a contract in a market where later delivery is cheapest collects that payment month
after month, as compensation for accepting the risk the hedger is passing on.

## An everyday comparison

Think of a market gardener who grows potatoes and needs to know the price months in advance. The
warehouse that buys the crop for delivery today must store it, insure it and tie up money, so it
charges more for potatoes delivered in three months than for potatoes delivered this week: the later
price is higher. That is the ordinary case, and rolling a promise forward from one month out to three
months out costs the holder money, because the contract being bought is more expensive than the one
being sold. But in a bad harvest year potatoes are scarce right now, and today's potatoes cost more
than next season's. Then someone holding a promise for this month can roll it into the cheaper later
contract and be paid for the wait. The strategy simply buys the materials where the wait pays and
sells the materials where the wait costs.

## The rules, step by step

1. Choose a basket of raw materials that have traded futures for a long time, for example crude oil,
   heating oil, natural gas, copper, gold, silver, corn, wheat, soybeans and sugar.
2. For each material, find the contract closest to its delivery date and the contract furthest away,
   and note today's price of each plus the two delivery dates.
3. Compute the fraction of the year that separates the two delivery dates, then scale the price
   difference to a rate per year. That number is called the roll return and is defined in the next
   section. A positive roll return means later delivery is cheaper than sooner delivery.
4. Rank the materials by roll return, best first.
5. Buy the top fifth of the list, in equal amounts, by purchasing their futures contracts and going
   long. Sell short the bottom fifth, in equal amounts, which means promising to sell contracts you
   do not own and buying them back later. In a basket of ten materials, that is two bought and two
   sold short.
6. Hold for one month without looking at the prices in between, then close everything and repeat from
   step 2 at the start of the next month.
7. The version in the sources is long and short at the same time, so the two sides partly offset each
   other and the whole position is only exposed to the difference between raw materials, not to the
   direction of the commodity market as a whole.

When a futures contract nears its delivery date, a fund that wants to keep holding the material must
sell the expiring contract and buy a later one. This is called rolling the position, and the roll
return is named after it because it is the profit or cost of doing that.

## The maths, with every symbol named

The roll return of one raw material over the year:

```text
R = ( ln(P_near) - ln(P_far) ) * 365 / (D_far - D_near)
```

- `R` is the roll return for a whole year, written as a decimal, so 0.10 means 10 percent a year.
- `P_near` is today's price of the futures contract with the nearest delivery date.
- `P_far` is today's price of the contract with the furthest delivery date.
- `ln` is the natural logarithm, a standard spreadsheet function; it turns a price ratio into a
  difference, and it is what the library algorithm uses.
- `D_near` and `D_far` are the numbers of days from today until each contract's delivery date.
- The factor `365 / (D_far - D_near)` stretches a price difference over a short gap into a rate per
  year, so materials with different contract spacing can be compared.

What it means: when the later contract is cheaper than the earlier one, `P_far` is smaller than
`P_near`, so `R` is positive and the curve of prices across delivery dates slopes downward. When the
later contract is dearer, `R` is negative and the curve slopes upward.

The equal-weighted weight of each selected material:

```text
w_i = 0.5 / k on the long side, and w_i = -0.5 / k on the short side
```

- `w_i` is the fraction of the money placed in material `i`; a negative value is a short position.
- `k` is the number of materials on each side, here one fifth of the basket.
- The long weights add up to 0.5 and the short weights add up to minus 0.5, so the two sides are equal
  in size and the net exposure to the direction of the market is near zero.

The portfolio's return over the next month:

```text
R_portfolio = sum over held materials of ( w_i * r_i )
```

- `r_i` is the next-month return of material `i`, meaning its futures price at the end of the month
  divided by its price at the start, minus one, signed so that a short position gains when the price
  falls.
- For a short position the term is `w_i * r_i` with `w_i` negative, so a price fall produces a gain.

The cost of rebuilding the list each month:

```text
Cost = t * c
```

- `t` is the traded fraction of the account. Closing a full position and opening a new one counts
  both sides, so `t` is about 2.0 when the whole book turns over.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the price
  at which you can buy and the price at which you can sell, plus any commission. A realistic figure
  for liquid commodity futures is 0.0005 to 0.001, that is five to ten basis points, where one basis
  point is one hundredth of one percent.

## A worked example

Ten raw materials, ranked by roll return. The prices, dates and returns below are invented, but they
are of the size these numbers actually take. The near contract is 30 days away; `D_far - D_near` is
shown in the last column.

| Material    | P_near  | P_far   | Days apart | Roll return R |
| ----------- | ------- | ------- | ---------- | ------------- |
| Sugar       | 0.22    | 0.21    | 90         | +0.1887       |
| Heating oil | 2.40    | 2.34    | 90         | +0.1027       |
| Silver      | 24.00   | 23.60   | 90         | +0.1022       |
| Copper      | 4.20    | 4.16    | 60         | +0.0582       |
| Wheat       | 6.10    | 6.02    | 150        | +0.0402       |
| Soybeans    | 13.50   | 13.80   | 150        | -0.0669       |
| Corn        | 4.80    | 4.95    | 150        | -0.0936       |
| Gold        | 1900.00 | 1930.00 | 60         | -0.0953       |
| Crude oil   | 80.00   | 84.00   | 90         | -0.1979       |
| Natural gas | 2.90    | 3.05    | 60         | -0.6136       |

The top fifth is two materials, sugar and heating oil, so the strategy buys those. The bottom fifth
is crude oil and natural gas, so it sells those short. Each of the four positions gets a quarter of
the account, so the long side is sugar and heating oil at 0.25 each, and the short side is crude oil
and natural gas at minus 0.25 each. Suppose the next month produces these price moves.

| Position           | Weight | Next-month price change | Contribution    |
| ------------------ | ------ | ----------------------- | --------------- |
| Sugar, long        | +0.25  | +3 percent              | +0.7500 percent |
| Heating oil, long  | +0.25  | +2 percent              | +0.5000 percent |
| Crude oil, short   | -0.25  | -5 percent              | +1.2500 percent |
| Natural gas, short | -0.25  | -4 percent              | +1.0000 percent |
| Total              |        |                         | +3.5000 percent |

The portfolio gained 3.5000 percent before costs. Because futures are traded on margin, meaning only
a fraction of the contract value is posted as a deposit, a small price move is large relative to the
deposit, in both directions.

Now the cost. Assume the whole book is closed and reopened each month, so `t = 2.0`, and assume ten
basis points per trade, `c = 0.001`:

```text
Cost = 2.0 * 0.001 = 0.002, that is 0.20 percent
Net return for the month = 3.5000 - 0.20 = 3.30 percent
```

Twelve months at that rate compounds to about 47 percent a year, but that is a property of the
invented numbers, not a forecast. The same arithmetic with the sign of the next-month moves reversed
loses about 3.5 percent before costs. The example shows only how the rules are applied and how the
arithmetic behaves.

## What the research actually found

| Source                                         | What it measured                                               | Result                                                                                                                                                                      |
| ---------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fuertes, Miffre and Rallis                     | Commodity futures from 1979 to 2004, sorting on term structure | An average abnormal return of about 12.66 percent a year for the term-structure strategy, and the authors report it is robust to transaction costs                          |
| Quantpedia, summarising the same work          | The long-short version, monthly rebalancing                    | 11.73 percent a year, volatility 23.84 percent, maximum fall 78.06 percent, reward-to-risk 0.49, over 1979 to 2004, and it rates its confidence in the effect as strong     |
| Quantpedia, on the same page                   | The carry return against share-market stress                   | The carry factor earns less exactly when share-market volatility rises, and its high average return is described as payment for that bad payoff in stressed periods         |
| Zaremba                                        | Commodity markets with different levels of outside money       | Momentum and term-structure strategies performed better in markets with less investor participation and earned little where participation was high                          |
| QuantConnect, in the discussion under the page | The published algorithm re-run on current data                 | A staff member and a user both report the profit concentrated between about 1997 and 2007 and difficult to reproduce afterward, with the user pointing at large price jumps |

Read together: the payment for holding a hedger's risk is real enough to have been measured
repeatedly in the older sample, and the size of that measurement is large. But it is paid as
insurance, so it fails in the worst moments, which is where the 78 percent fall comes from, and the
most recent decade is where the measurement is weakest.

## How this project relates to it

This repository has no commodity term-structure strategy, and it says so rather than pretending
otherwise. The closest thing is a study of the same shape of data in
[Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md), a research
brief written from a harvest of academic papers. Its section on curves beyond power reports that
fitting a curve to 21 commodity futures, a level, a slope and a curvature term explained 96.5 percent
of the change in the price curves, and that trading the continuing direction of the slope earned an
annualised excess return of 1.77 percent before costs, which shrank to between 0.55 and 1.31 percent
after three cost assumptions and fell sharply after the year 2000. That brief measures a different
signal, the day-to-day drift of the curve rather than the level of the roll return, but it is the same
raw material and the same lesson about costs and decay.

## Where it goes wrong

- The payment is insurance, so it is small in calm times and painful in bad times. The carry return is
  worst when share-market volatility jumps, which is when an investor is least able to absorb a loss,
  and that is the source of the measured 78 percent fall.
- Crowding and financialisation. Once outside money pours into commodity futures, the simple version
  of the signal gets arbitraged, and the studies above find the effect weaker in exactly the markets
  with the most such money.
- The long-and-short construction is fragile. Both sides use futures on margin, so a sharp move in
  either leg can force the position to be reduced at the worst price, and the net exposure being near
  zero does not make either leg riskless.
- Costs and the roll itself. Every month the book turns over, and the very act of rolling into the
  later contract is the thing being measured; a backtest that ignores the buying-and-selling gap will
  overstate the result.
- A single bad data choice. Choosing a distant contract that trades rarely, or comparing materials
  with very different contract spacing, changes the ranking, and the formula's annualisation can turn
  a small price gap into a large-looking number.
- The signal can be an artefact of storage costs. When later delivery is dearer for purely mechanical
  reasons such as warehouse fees and interest rates, the negative roll return is not information
  about scarcity, and a rule that treats it as such is reading a cost as a signal.

## Try it yourself

You need a spreadsheet and a public source of futures prices; exchange websites and finance pages
publish them with delivery months. Pick six raw materials.

1. Build a sheet with one row per material and columns: name, price of the nearest contract, price of
   the furthest contract, days to the nearest delivery, days to the furthest delivery.
2. Add a column `R` with the formula from the maths section, using the spreadsheet's natural-log
   function.
3. Sort by `R`, best first, and mark the top and bottom material.
4. Track what the price of each marked material's near contract does over the next month.
5. Write down what the pair would have earned if you had bought the best and sold the worst, then
   subtract 0.20 percent for the round trip.

What to notice: the ranking is driven by the delivery dates, so if you use only one contract per
material the number is meaningless, and the sign of the monthly result often comes from the short leg
reversing. Over a few months, the pair will sometimes win and sometimes lose, which is the honest
picture the older studies draw.

## Where this came from

- [QuantConnect strategy library: term structure effect in commodities](https://www.quantconnect.com/tutorials/strategy-library/term-structure-effect-in-commodities),
  the rules as implemented: split the basket by roll return, buy the top fifth, sell short the bottom
  fifth, hold one month.
- [Quantpedia: term structure effect in commodities](https://quantpedia.com/strategies/term-structure-effect-in-commodities),
  the performance figures, the instrument count and the underlying papers.
- Fuertes, Miffre and Rallis, [Tactical Allocation in Commodity Futures Markets: Combining Momentum
  and Term Structure Signals](https://ssrn.com/abstract=1127213), the paper the rules are built from.
- [Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md), this
  repository's own brief on commodity curves, for the section on curves beyond power.

## Words used in this tutorial

- contango: the ordinary case where a futures contract for later delivery costs more than one for
  sooner delivery.
- backwardation: the unusual case where a contract for later delivery costs less than one for sooner
  delivery.
- futures: an agreement made today to buy or sell something at a fixed price on a future date.
- long: owning something, or holding a contract that gains when its price rises.
- margin: the deposit a trader must post to hold a futures position, which is smaller than the value
  of the contract.
- roll return: the profit or cost of selling an expiring futures contract and buying a later one.
- short: selling something you do not own yet, and buying it back later, so the position gains when
  the price falls.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
