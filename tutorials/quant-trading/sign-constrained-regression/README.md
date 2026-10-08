# Fitting a line that economics will accept, and choosing what to grow

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Agricultural crops - the prices and planted quantities of thirty crops in one country - and, in the author's forward test, palm-oil futures                                                                                                      |
| How often it trades       | Once a year, when the next season's planting choice and price are recomputed                                                                                                                                                                     |
| What you need             | Python and a data file                                                                                                                                                                                                                           |
| Where the rules come from | [The Smart Farmers project page](https://github.com/je-suis-tm/quant-trading/tree/master/Smart%20Farmers%20project)                                                                                                                              |
| The underlying research   | Marshall's supply-and-demand picture, [Principles of Economics](http://files.libertyfund.org/files/1676/Marshall_0197_EBk_v6.0.pdf) (1890), and the instrumental-variables literature, [Angrist and Krueger](https://economics.mit.edu/files/18) |
| How well it held up       | Weak: one country over six years with no independent replication, and the author's own forward test against palm-oil futures did not reproduce the in-sample fit                                                                                 |
| Also appears in           | Nothing else in this collection applies a sign restriction to a fitted line; the sibling [Monte Carlo price simulation](../monte-carlo-price-simulation/README.md) comes from the same external repository                                       |

## The idea in one paragraph

A straight line fitted to a set of observations is the line that sits closest to them. Ordinary
fitting can return a slope with the wrong sign, such as a line saying that growing more of a crop
raises its price, when a flood of supply pushes the price down. A sign restriction keeps every
coefficient - the number multiplying an explaining column - on the side economics requires, and the
answer is then found by search rather than by a formula. The second idea is a farmer's planting
choice: a union of growers divides a fixed area of land among many crops, cannot grow the same crop
on the same field two seasons running, and each crop's price falls as the union's own output of it
rises. So the amount planted feeds back into the price the grower receives, and the choice is a small
profit-maximising problem rather than a forecast read off a chart. The whole model joins the two: fit
the price relationship once, then use it to work out the planting and the next price.

## Why anyone believed it

The price of a crop settles where what growers bring meets what buyers want. The other side of a
trade built on this is the whole market rather than one counterparty: growers who must sell a
perishable harvest, and buyers who change how much they eat when income or population changes. A
harvest takes a season, so supply cannot answer a price change immediately, and the amount planted
this year shows up as the price next year. If that link can be measured, next year's price can be
guessed from this year's planting decision, before the market has moved. A futures contract - an
agreement to buy or sell a fixed quantity at a fixed price on a later date - is the thing the
forecast would be used to trade.

Population and income per person are used as stand-ins for demand, because demand itself is not
observed. The source's claim is that population and income push demand but do not touch the crop
price directly, which is what makes them usable as stand-ins, and that the resulting price forecast
leads the futures price, so the gap between the two is worth acting on.

## An everyday comparison

A village holds one tomato market each Saturday. Every grower brings a load, and the price settles at
whatever clears the whole lot, because tomatoes cannot be kept. A visitor on the hill above the
market records, week after week, how much was brought and what it sold for, and draws a line through
the cloud of points. The line says that more tomatoes sell for more, because a few busy holiday weeks
happened to line up with high prices. The village committee refuses that line the moment it is shown,
on the plain ground that a glutted market cannot fetch a higher price, and asks for the closest line
whose slope does not rise. Separately, the committee must divide the shared field among crops, cannot
plant the same crop on the same strip two years running, and knows that the more of one crop the
village grows, the less that crop fetches next year.

## The rules, step by step

The model runs once a year, on annual numbers.

1. Gather annual figures for one country from a public agricultural database: for each crop, the
   tonnes produced and the average price paid to the grower; and for the country, the population and
   the income per person.
2. Lay them out with one row per crop per year. The price is the column to be explained. The columns
   that explain it are population, income per person, and the quantity produced with its sign
   flipped, that is, written as minus the quantity.
3. Fit the line by ordinary least squares, the method that chooses the height and the three slopes so
   that the total squared gap between the line and the observed prices is as small as possible.
4. Refit with a restriction: keep each of the three slopes at or above zero. Because the supply
   column was sign-flipped, a slope at or above zero says the same as price falling as supply rises.
   With an active restriction the answer is found by search, not by a formula.
5. Store the fitted height and the three slopes for every crop.
6. For each year, choose the quantity of every crop to grow so that total profit is as large as
   possible, with the total land area exactly used up and each crop's area between a lower bound and
   an upper bound. The bounds come from last year's area, the crop's expected lifespan, and the rule
   against growing the same crop two seasons running.
7. Work out the price implied by the chosen quantities using the stored slopes, and compare it with
   the market or futures price. The difference is the reason to trade, and it is recomputed once a
   year.

## The maths, with every symbol named

The line fitted to each crop's price:

```text
fitted_price_i = c + b_pop * population_i + b_income * income_i - b_supply * quantity_i
```

- `fitted_price_i` is the price the line predicts for crop `i`.
- `c` is the constant, the height of the line where every explaining column is zero.
- `b_pop`, `b_income` and `b_supply` are the three slopes, one number each.
- `population_i`, `income_i` and `quantity_i` are the explaining columns for crop `i`; the quantity
  enters with a minus sign, so a positive `b_supply` means a higher quantity goes with a lower price.

Because the quantity is entered negated, the restriction below is placed on `b_supply`, not on the
raw supply slope, which keeps the arithmetic in the solver simple.

The constrained fit as a small optimisation problem:

```text
minimise  sum over i of (price_i - fitted_price_i)^2
subject to  b_pop >= 0,  b_income >= 0,  b_supply >= 0
```

- `price_i` and `fitted_price_i` are the observed and the fitted price for crop `i`.
- The sum is the total squared gap, the same quantity ordinary least squares makes small.
- The three inequalities are the sign restriction, and the constant `c` is left free.
- It means: the closest obeying line to the data. This is a small optimisation problem, not a
  formula, because an inequality that is active has no closed-form answer.

The pricing mechanism, where the union's own output moves the price:

```text
price_i = equilibrium_i + alpha_i * (demand_i - supply_i)
```

- `equilibrium_i` is the price at which buyers' and sellers' plans would match.
- `demand_i` and `supply_i` are the quantities wanted and brought to market.
- `alpha_i` is a positive number converting a quantity mismatch into a price move.
- It means: a surplus, that is demand below supply, makes the bracket negative and drives the price
  down; a shortage drives it up. This is the supply-and-demand picture from Marshall.

The planting choice, which is a different kind of problem:

```text
maximise  sum over i of (price_i * q_i - cost_i * q_i)
subject to  sum over i of (land_i * q_i) = total_land
            lower_i <= q_i <= upper_i
```

- `q_i` is the quantity of crop `i` chosen for next season.
- `cost_i` is the cost of producing one tonne of crop `i`.
- `land_i` is the hectares needed for one tonne of crop `i`, and `total_land` is the fixed area
  available.
- `lower_i` and `upper_i` are the bounds from lifespan and rotation.
- It means: profit equals price times quantity minus cost times quantity, added over crops. Because
  `price_i` itself depends on `q_i` through the pricing equation, each term contains a `q_i` squared;
  the measure to improve is a curved surface, and the land total is an exact requirement, an equality
  rather than an inequality.

The difference matters. The fit minimises a measure of error under an inequality on a slope; the
planting problem maximises profit under an exact requirement on the land total and bounds on each
quantity. The source solves both with the same quadratic-programming tool, `cvxopt`.

## A worked example

Five seasons, one explaining column plus the constant, so a two-term line. The numbers are made up but
plausible, in one currency and one unit throughout.

| Season | Quantity (thousand tonnes) | Price (dollars per tonne) | Quantity gap | Price gap | Gap product | Squared quantity gap |
| ------ | -------------------------- | ------------------------- | ------------ | --------- | ----------- | -------------------- |
| 1      | 1                          | 11.00                     | -2.00        | -3.00     | +6.00       | 4.00                 |
| 2      | 2                          | 13.00                     | -1.00        | -1.00     | +1.00       | 1.00                 |
| 3      | 3                          | 12.00                     | 0.00         | -2.00     | 0.00        | 0.00                 |
| 4      | 4                          | 16.00                     | +1.00        | +2.00     | +2.00       | 1.00                 |
| 5      | 5                          | 18.00                     | +2.00        | +4.00     | +8.00       | 4.00                 |
| Sum    | 15                         | 70.00                     | 0.00         | 0.00      | +17.00      | 10.00                |

The average quantity is 3.00 and the average price is 14.00. The ordinary slope is the gap product sum
divided by the squared quantity gap sum, 17.00 divided by 10.00, giving 1.70. The constant is 14.00
minus 1.70 times 3.00, giving 8.90.

| Season | Price | Fitted | Gap   | Squared gap |
| ------ | ----- | ------ | ----- | ----------- |
| 1      | 11.00 | 10.60  | +0.40 | 0.16        |
| 2      | 13.00 | 12.30  | +0.70 | 0.49        |
| 3      | 12.00 | 14.00  | -2.00 | 4.00        |
| 4      | 16.00 | 15.70  | +0.30 | 0.09        |
| 5      | 18.00 | 17.40  | +0.60 | 0.36        |
| Sum    |       |        |       | 5.10        |

The unrestricted line is price = 8.90 + 1.70 * quantity, with a total squared gap of 5.10. Its slope
is positive, so the line says more supply fetches a higher price, the wrong sign.

The restriction holds the slope at or below zero. Any negative slope is worse than a flat line here,
because the data lean upward, so the best allowed slope is exactly zero and the line is flat at the
average price, 14.00. The squared gap becomes 9.00 + 1.00 + 4.00 + 4.00 + 16.00, which is 34.00.

The restriction made the fit worse on these five points, from 5.10 to 34.00, but kept the line on the
side economics requires. When the data already lean the right way, the two answers are the same; that
is why the source's ordinary and restricted fits differ for only some crops.

Now the trading step. Suppose the flat model says next season's grower price is 14.00 per tonne, and a
palm-oil futures contract trades at 15.00 per tonne for 25 tonnes, the size the source uses. The rule
sells one contract at 15.00 and buys it back at 14.00. The gross gain is (15.00 - 14.00) times 25, or
25.00. At 0.05 per tonne on each side, the round trip costs 0.10 times 25, or 2.50, so the net gain is
22.50 per contract. The commission is invented; the source's own numbers carry no trading costs.

## What the research actually found

The data are public agricultural extracts for Malaysia, 2013 to 2018: 61 crop categories, pruned to
30 by dropping those under one percent of land use. The 14-gigabyte source database is not committed.

The author's headline verdict is that the model is a useful tool on around 65 percent of the crops
(project README, Empirical Result). The crop write-ups are deliberately mixed: cabbage's estimate
lags the actual price by a year, cocoa's production forecast is called flawless but its demand seems
blind to income, coconut production is heavily overestimated, mango's price moves the wrong way, and
rubber's price prediction is called spectacular. Oil palm production has the smallest mean error.

The forward test is the author's own falsification attempt and it goes badly. The model is extended
to 2019 to 2025 on pre-pandemic population and income forecasts, then compared with real crude
palm-oil futures. The write-up says the forward test "isn't as fantastic" as the backtest: the model
gave a smooth backwardation then contango while the actual path was a W-shaped recovery. Backwardation
means near dates sit above far dates; contango is the opposite.

Two limits come from the author rather than a critic. First, the backtest uses population and income
already known years later, a view from above that a grower planning in 2013 could not have had, so the
author repositions the model as a scenario tool. Second, the restriction taught here was never measured
on its own: the source states the rule of keeping the slopes at or above zero and reports no measured
result for whether it improves the forecasts. The reported 65 percent is for the whole model.

One more discrepancy sits inside the source. The forward-test text assumes agricultural land grows 5
percent a year (project README, Discussion), while the code grows it 1 percent a year (`forecast.py`,
the line repeating `temp[-1]*1.01`), so the chart shows a scenario the text does not describe.

## How this project relates to it

The local repository implements no convex optimisation. A search of the workspace for a
quadratic-programming solver or its library name finds nothing, so this tutorial is the collection's
only treatment of the idea; the nearest material is research rather than code.

- [The brief on energy and commodities](../../../strategies/books2/18_energy_and_commodities.md) is
  this repository's notes on agricultural commodity markets, which list `1607.07582v1` as a lead.
- [Scoring a strategy on its evidence, not its profit](../../project/scoring-evidence-not-profit/README.md)
  explains why a claim is graded on its evidence, not on one profit number.
- [How a backtest lies](../../foundations/07_how-a-backtest-lies.md) shows why an in-sample fit that
  matches history so well is the weakest kind of evidence.
- The sibling [Monte Carlo price simulation](../monte-carlo-price-simulation/README.md) is the other
  forecasting experiment from the same external repository.

## Where it goes wrong

- Costs are assumed away. The repository states at its front page that all trades are frictionless,
  with no slippage, no surcharge and no illiquidity, so any trading here is reported gross of costs.
- The data are a view from above. The author admits the backtest uses population and income that were
  already known years later, so the measured accuracy is not what a real grower could have achieved.
- The missing prices were filled in by hand. The committed data file's gaps came from hard-coded
  lists, and the program that generated them is commented out, so they cannot be rebuilt.
- The raw data are not committed. The source says the public database is too large to upload, so a
  reader cannot reproduce the numbers without downloading it separately.
- The text and the code disagree. The write-up assumes agricultural land grows 5 percent a year while
  the code grows it 1 percent a year, so the forward chart does not match its own description.
- One short sample. One country, six years and thirty crops, scored by the model's own author, with no
  independent replication, so the 65 percent figure rests on a single self-assessment.
- A sign restriction can hide a real effect. If quantity and price are decided together, forcing the
  slope onto the "correct" side can conceal a genuine relationship instead of revealing one.

## Try it yourself

You need a spreadsheet and no code.

1. Make five rows with a quantity column holding 1, 2, 3, 4, 5 and a price column such as 11, 13, 12,
   16, 18 dollars per tonne.
2. Add a column for the quantity gap (quantity minus 3) and one for the price gap (price minus 14).
3. Add a column multiplying the two gaps, and a column squaring the quantity gap.
4. Total the products and the squares. The ordinary slope is the product total divided by the square
   total; reproduce 1.70 and the constant 8.90.
5. Add a column with the fitted price, then a column with the squared gap between fitted and actual;
   the total should be 5.10.
6. Copy the sheet, set the slope to zero, and recompute the fitted prices as the flat average price of
   14.00. Total the squared gaps again.

What to notice: the restriction makes the fit strictly worse on the very numbers that produced it, and
that is the point. The unrestricted line was closer to these five points and still said something
impossible. The size of the worsening shows how strongly the data disagree with the sign you imposed.

## Where this came from

- [The Smart Farmers project README](https://github.com/je-suis-tm/quant-trading/tree/master/Smart%20Farmers%20project),
  the rules, the crop-by-crop assay and the author's written verdicts.
- [estimate demand.py](https://raw.githubusercontent.com/je-suis-tm/quant-trading/master/Smart%20Farmers%20project/estimate%20demand.py),
  the ordinary and the sign-restricted line fits.
- [forecast.py](https://raw.githubusercontent.com/je-suis-tm/quant-trading/master/Smart%20Farmers%20project/forecast.py),
  the planting problem, the price update and the forward run.
- [The repository's front page](https://github.com/je-suis-tm/quant-trading), the frictionless-trading
  caveat quoted above; the files read here are at commit 611b73f2 on the master branch.
- [The brief on energy and commodities](../../../strategies/books2/18_energy_and_commodities.md), this
  repository's own notes, including `1607.07582v1` on the financialisation of agricultural markets.
- Marshall, [Principles of Economics](http://files.libertyfund.org/files/1676/Marshall_0197_EBk_v6.0.pdf),
  the origin of the supply-and-demand picture; Angrist and Krueger,
  [Instrumental Variables and the Search for Identification](https://economics.mit.edu/files/18), on
  using population and income as stand-ins for demand.

## Words used in this tutorial

- coefficient: the number multiplying an explaining column in a fitted line.
- constrained fit: the closest line to the data among the lines that obey a stated restriction.
- futures: an agreement to buy or sell a fixed quantity at a fixed price on a later date.
- least squares: the fitting method that makes the total squared gap between a line and the data as
  small as possible.
- quadratic program: an optimisation problem whose measure to improve has squares in it, solved by
  search.
- sign restriction: a rule that holds a fitted slope at or above, or at or below, zero.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
