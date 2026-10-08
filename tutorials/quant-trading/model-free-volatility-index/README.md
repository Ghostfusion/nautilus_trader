# Model-free volatility index: reading expected movement from option prices

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing. It is a measuring device, not a trade: it reads the option prices on one commodity, here natural gas, and reports a single number                                                                                                                                                                                                                                                        |
| How often it trades       | Never. The number is recomputed from that day's option prices; no order is ever sent                                                                                                                                                                                                                                                                                                              |
| What you need             | Python and a data file (one day of option prices, a holiday calendar and a table of government interest rates)                                                                                                                                                                                                                                                                                    |
| Where the rules come from | [VIX Calculator.py, je-suis-tm/quant-trading](https://github.com/je-suis-tm/quant-trading/blob/master/VIX%20Calculator.py)                                                                                                                                                                                                                                                                        |
| The underlying research   | The [Cboe Volatility Index methodology](https://cdn.cboe.com/api/global/us_indices/governance/VIX_Methodology.pdf) and Demeterfi, Derman, Kamal and Zou, [More Than You Ever Wanted to Know About Volatility Swaps](https://www.researchgate.net/publication/246869706_More_Than_You_Ever_Wanted_to_Know_About_Volatility_Swaps)                                                                  |
| How well it held up       | Open question: the source states the construction and prints one value for one date, and this collection found no measured result for the generalized rule                                                                                                                                                                                                                                        |
| Also appears in           | The QuantConnect pages [exploiting the term structure of VIX futures](../../quantconnect/exploiting-term-structure-of-vix-futures/README.md) and [VIX predicts stock index returns](../../quantconnect/vix-predicts-stock-index-returns/README.md), and the paper [dispersion trading](../../paperswithbacktest/dispersion-trading/README.md), which use a volatility index rather than build one |

## The idea in one paragraph

An option is a contract that gives its holder the right to buy or sell something later at a fixed
price, and the price of that contract carries what the market expects the underlying thing to do.
This page explains how a whole menu of such contracts is squeezed into one number: how far, in
percentage terms, the market expects a commodity to move over a chosen period. The method adds up the
prices of the contracts that sit away from the current level, weights each by how finely its fixed
price is spaced, and scales the total by the time left. It is called model-free because it never
assumes a formula for how prices move; it uses only the prices themselves. Two nearby expiry dates,
the dates on which the contracts end, are blended so the answer describes a horizon that may fall
between them. The result is a single percentage, the same kind of number the well-known VIX reports
for a large American share index.

## Why anyone believed it

An option seller takes on a risk someone else wants to hand over. A farmer who must sell a crop
later, or a factory that must buy fuel later, will pay to lock in a price, and the seller charges for
the risk of being wrong. When the market is frightened, more buyers want protection, the
out-of-the-money contracts they buy become expensive, and their prices rise.

If those prices are set by many competing buyers and sellers, they carry the market's own view of how
much the thing will move, and nobody has to publish a forecast. Adding up the cheap, far-away
contracts and the expensive, near ones turns that view into one percentage that can be compared
across days and commodities. The bet is that this single number measures expected movement better
than any one contract, because it uses the whole menu instead of one price.

## An everyday comparison

Imagine a village where insurers sell policies that pay out if the harvest fails by at least ten
percent, at least twenty percent, at least thirty percent, and so on up to a complete failure. Nobody
publishes a weather forecast, yet the prices of those policies, taken together, tell you what the
village expects: cheap cover against a small shortfall and expensive cover against total failure mean
the village fears a severe bad year. Add the prices of every policy, weighted by how far apart the
payout levels are, and you get one number that stands for the whole menu. The insurance market did
the forecasting; the number only reads it back.

## The rules, step by step

The source is a calculator, not a trading rule, so there is no entry, no exit and no position size.
It runs once on one day of option prices for one commodity: a day's chain of prices, a list of public
holidays, and government interest rates for two-month and three-month maturities.

1. Choose the commodity and two expiry horizons. The script uses natural gas options, with a front
   horizon of two months and a rear horizon of three months.
2. For the front expiry, collect every contract: its fixed price (the strike), its price, and whether
   it is a call (the right to buy) or a put (the right to sell). Use the previous day's official
   settlement price.
3. Find the strike where the call and the put prices are closest, and nudge it up to the forward level
   by adding the call-minus-put difference grown by the interest rate over the time left. The forward
   is the price the market expects at expiry.
4. Take the reference strike `K0` to be the highest listed strike at or below the forward, and keep
   the out-of-the-money stripes: calls with strike above `K0`, puts with strike below it.
   Out-of-the-money means the contract would pay nothing if the price stopped moving right now.
5. Walk outward from `K0` on each side and stop at the first place where two neighbouring strikes in
   a row both have a price of zero, dropping those and everything further out. A run of zeros usually
   means nobody trades that far away, so the prices there are not trustworthy.
6. For each kept strike, measure the spacing: half the distance from the strike below to the strike
   above, or the one-sided distance to the single neighbour at the end of a list.
7. Multiply each kept strike's price by its spacing and divide by the square of its strike, grow the
   result by the interest rate over the time left, and add them all up.
8. Turn that total into the variance for the expiry: multiply by two and divide by the time left in
   years, then subtract a small correction for the gap between the forward and the reference strike.
9. Repeat steps 2 to 8 for the rear expiry, then blend the two variances by time, take the square root
   and multiply by 100. Recompute from the next day's chain whenever a fresh number is wanted.

Variance is the square of volatility; volatility is the size of price movement, quoted as a
percentage per year.

## The maths, with every symbol named

The forward comes from the strike with the smallest gap between its call and its put:

```text
forward = K_near + e^(r * T) * (call_near - put_near)
```

- `forward` is the level the market expects at expiry.
- `K_near`, `call_near` and `put_near` are the strike where the call and the put prices are closest,
  and those two prices.
- `r` is the interest rate for the time left as a decimal (two percent is 0.02), `T` is the time left
  in years, and `e` is the number about 2.7183 that turns a growth rate into a growth factor.

The reference strike is the highest listed strike not above the forward, written `K0`. Each kept
strike then contributes a term, and the terms are added:

```text
total = sum over kept strikes of price_i * e^(r * T) * spacing_i / K_i^2
```

- `total` is the sum of the contributions for one expiry.
- `price_i` and `K_i` are the settlement price and the strike of kept strike number `i`.
- `spacing_i` is half the gap from the strike below to the strike above, or the one-sided distance at
  the end of the list.

The total becomes the variance for the expiry:

```text
variance = total * 2 / T - (forward / K0 - 1)^2 / T
```

- `variance` is the annualised variance, the square of the expected movement in percent per year. The
  source names this quantity `sigma`, but it is a variance, not a volatility.

The two expiries are blended, with `M_year` minutes in a year and `M_target` minutes in the horizon:

```text
w_front = (T_rear * M_year - M_target) / (T_rear * M_year - T_front * M_year)
w_rear  = (M_target - T_front * M_year) / (T_rear * M_year - T_front * M_year)
index   = 100 * sqrt((T_front * variance_front * w_front + T_rear * variance_rear * w_rear) * M_year / M_target)
```

- `T_front` and `T_rear` are the times to the two expiries in years, and `variance_front` and
  `variance_rear` are the two variances above.
- `M_year` is the minutes in a year, 525600, and `M_target` is the minutes in the chosen horizon; the
  script sets it to the rear horizon, three months, which is 129600 minutes.
- `w_front` and `w_rear` are the blend weights, which add to one, and `index` is the reported
  percentage.

## A worked example

The numbers are invented but plausible, and the interest rate is zero so the growth factors equal
one. The commodity is priced near 100.00 and the strikes are 90.00, 95.00, 105.00, 110.00 and 120.00.
At the strike of 100.00 the call and the put are closest, at 4.30 and 4.00, so the forward is
`100.00 + 1 * (4.30 - 4.00) = 100.30` and the reference strike `K0` is 100.00. The kept calls are
above 100.00 and the kept puts are below it. No strike has two zero prices in a row, so nothing is
cut off by the rule in step 5.

Front expiry, `T_front = 2 / 12 = 0.166667` year:

| Strike | Type | Price | Spacing | Price * spacing / strike^2 |
| ------ | ---- | ----- | ------- | -------------------------- |
| 90.00  | put  | 0.20  | 5.00    | 0.000123                   |
| 95.00  | put  | 0.75  | 5.00    | 0.000416                   |
| 105.00 | call | 2.40  | 5.00    | 0.001088                   |
| 110.00 | call | 0.80  | 7.50    | 0.000496                   |
| 120.00 | call | 0.25  | 10.00   | 0.000174                   |
| Total  |      |       |         | 0.002297                   |

The spacing of 110.00 is half the gap from 105.00 to 120.00, that is `(120.00 - 105.00) / 2 = 7.50`;
the end strikes use the one-sided distance, so 90.00, 95.00 and 105.00 get 5.00, and 120.00 gets
`120.00 - 110.00 = 10.00`.

Rear expiry, `T_rear = 3 / 12 = 0.25` year, the same strikes with higher prices because more time
remains:

| Strike | Type | Price | Spacing | Price * spacing / strike^2 |
| ------ | ---- | ----- | ------- | -------------------------- |
| 90.00  | put  | 0.35  | 5.00    | 0.000216                   |
| 95.00  | put  | 1.05  | 5.00    | 0.000582                   |
| 105.00 | call | 3.10  | 5.00    | 0.001406                   |
| 110.00 | call | 1.20  | 7.50    | 0.000744                   |
| 120.00 | call | 0.45  | 10.00   | 0.000313                   |
| Total  |      |       |         | 0.003260                   |

The correction is `(forward / K0 - 1)^2 = (100.30 / 100.00 - 1)^2 = 0.003^2 = 0.000009` for both
expiries, because they share the forward and reference strike. The two variances are:

```text
variance_front = 0.002297 * 2 / 0.166667 - 0.000009 / 0.166667 = 0.027563 - 0.000054 = 0.027509
variance_rear  = 0.003260 * 2 / 0.25     - 0.000009 / 0.25     = 0.026080 - 0.000036 = 0.026044
```

With 525600 minutes in a year, the front expiry is `0.166667 * 525600 = 87600` minutes away, the rear
is `0.25 * 525600 = 131400` minutes away, and the target horizon is 129600 minutes:

```text
w_front = (131400 - 129600) / (131400 - 87600) = 1800 / 43800 = 0.041096
w_rear  = (129600 - 87600)  / (131400 - 87600) = 42000 / 43800 = 0.958904
```

The rear term carries almost all the weight, because the target horizon sits just below the rear
expiry. The index is then:

```text
index^2 = (0.166667 * 0.027509 * 0.041096 + 0.25 * 0.026044 * 0.958904) * 525600 / 129600
        = (0.000188 + 0.006243) * 4.055556 = 0.006432 * 4.055556 = 0.026084
index   = 100 * sqrt(0.026084) = 100 * 0.161507 = 16.15
```

The reported number is about 16.15, meaning the option market prices in a movement of roughly 16
percent per year. There is no cost line because nothing is bought or sold; the missing costs are
exactly the weakness the source's own caveat admits.

## What the research actually found

The source measures nothing. It is a calculator that reads one day of natural gas option prices, for
a single date of 2020-11-12, and prints one number; recomputing that printed value is the only test it
offers, and it states no sample, period or costs.

The construction it generalizes is the one behind the VIX, the Cboe Volatility Index, which reports
the market's expectation of movement in the S&P 500 share index over the next 30 calendar days
(`1806.07556v2`, p.1). That index rests on a deep, continuously traded chain; the source applies the
same recipe to a thinner commodity chain on a chosen horizon and does not report whether the result
means anything.

Two nearby measurements are worth naming. Over January 2007 to February 2018 the VIX and several
share markets showed frequent structural breaks (`1806.07556v2`, p.1). Separately, a model can fit the
volatility surface of option prices closely and still imply a variance term structure far from the
one those prices contain, so pricing variance swaps or VIX derivatives calls for an explicit penalty
on variance (`2509.08096v1`, p.1). Neither tests this script: the exact rule here, on a commodity
chain, was not measured by any source this collection holds.

## How this project relates to it

This repository implements the same construction in Rust, in
[volatility.rs](../../../crates/research/src/volatility.rs), written in this same change. A reader
would see there the pieces this page describes: the forward from the closest call and put, the
out-of-the-money selection, the strike-spacing weights, the correction for the gap between the
forward level and the reference strike, and the two-term blend that turns two variances into one
annualised percentage. The module keeps the reference strike in the sum as the average of its two
prices, which is the published convention and not what the source script does. The unit tests in that
file carry the worked arithmetic; from the repository root,
`cargo test -p nautilus-research volatility` runs them.

Elsewhere in the collection a volatility index is used rather than built. The QuantConnect page on
[exploiting the term structure of VIX futures](../../quantconnect/exploiting-term-structure-of-vix-futures/README.md)
takes the index and its futures as data it is handed, the page on
[whether VIX predicts stock index returns](../../quantconnect/vix-predicts-stock-index-returns/README.md)
treats the same number as a signal, and the paper
[dispersion trading](../../paperswithbacktest/dispersion-trading/README.md) uses volatility across
many names. None computes the index from an option chain, which is the gap this page fills. The local
brief [Options and derivative instruments](../../../strategies/books2/12_options_and_derivatives.md),
whose Section 3 explains that fitting the volatility surface does not pin the variance term
structure, is the closest research note.

## Where it goes wrong

- The prices are not tradable prices. The script uses the previous day's official settlement price
  because it has no buying and selling quotes, so the number is a reference level, not one at which
  the whole stripe could be bought. The source's own caveat, at README line 19, assumes all trades are
  frictionless, with "No slippage, no surcharge, no illiquidity".
- The reference strike is left out of the sum. The script keeps calls strictly above `K0` and puts
  strictly below it, so `K0` contributes nothing, whereas the Cboe method includes it using the
  average of its call and put prices.
- The cutoff on zero prices can truncate the tail: a single stale or untraded strike can end the
  stripe early and remove real variance, and the far-out stripes the method relies on are the least
  reliable prices in the chain.
- The target horizon and the rear expiry nearly coincide, so the rear term takes almost all the
  weight, about 96 percent above, and the front term barely matters.
- Nothing here was measured: the source tests no sample, includes no costs and reports no comparison,
  so there is no evidence that this index tracks or predicts anything for a commodity.

## Try it yourself

You need nothing but a spreadsheet and a public source of option prices.

1. Find a published worked example of a volatility index, such as the one in the Cboe methodology
   document linked at the end of this page, which lists strikes with call and put prices for one
   expiry.
2. Make columns headed `strike`, `type` (call or put), `price`, `spacing`, `price * spacing`, and
   `price * spacing / strike^2`, fill in the strikes and prices, and set `spacing` by the rule in
   step 6 above: half the distance between the neighbouring strikes, one-sided at the ends.
3. Add up the last column, multiply by two and divide by the time left in years, subtract the
   correction term, and compare the square root times 100 with the index the document reports.

What to notice: the far-out strikes add very little even when their prices are large, because the
division by the strike squared shrinks them, so most of the number comes from the strikes nearest the
reference level. Try deleting the cheapest far strike: the total barely moves, which shows why the
cutoff on zero prices rarely matters, and why a missing near-the-money stripe would.

## Where this came from

- [je-suis-tm/quant-trading](https://github.com/je-suis-tm/quant-trading), section 15 of its README
  for the description and the parameters, and
  [VIX Calculator.py](https://github.com/je-suis-tm/quant-trading/blob/master/VIX%20Calculator.py)
  for the rules as implemented, including the frictionless caveat at README line 19.
- The [Cboe Volatility Index methodology](https://cdn.cboe.com/api/global/us_indices/governance/VIX_Methodology.pdf),
  the standard construction this script generalizes.
- Demeterfi, Derman, Kamal and Zou, [More Than You Ever Wanted to Know About Volatility Swaps](https://www.researchgate.net/publication/246869706_More_Than_You_Ever_Wanted_to_Know_About_Volatility_Swaps),
  the variance-swap replication the sum comes from.
- `1806.07556v2` (p.1) for the definition of the VIX as a 30-day expectation and its structural
  breaks, and `2509.08096v1` (p.1) for the gap between a fitted volatility surface and the implied
  variance term structure.
- [Options and derivative instruments](../../../strategies/books2/12_options_and_derivatives.md),
  Sections 3 and 6, this repository's brief on variance swaps and VIX derivatives.
- [Sign-constrained regression](../sign-constrained-regression/README.md), the sibling tutorial in
  this collection written from the same external repository.

## Words used in this tutorial

- call: an option giving its holder the right to buy the underlying thing later at a fixed price.
- put: an option giving its holder the right to sell the underlying thing later at a fixed price.
- strike: the fixed price written into an option contract, at which the holder may buy or sell.
- out-of-the-money: said of an option that would pay nothing if the price stopped moving right now.
- forward: the price at which the market expects the underlying thing to trade at expiry.
- variance: the square of volatility, a measure of how spread out the possible movements are.
- volatility: the size of price movement, usually quoted as a percentage per year.
- settlement price: the official end-of-day price a market publishes for each contract.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
