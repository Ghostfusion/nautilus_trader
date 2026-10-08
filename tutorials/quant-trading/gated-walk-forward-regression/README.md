# Trading a currency's break from oil, but only when the fit was strong

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                           |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A currency's exchange rate against a crude-oil price - in the author's tests the Norwegian krone and the Colombian peso, each against a local crude grade                                                                                                                                                                                       |
| How often it trades       | On daily prices; a few positions a year, each held for at most ten trading days                                                                                                                                                                                                                                                                 |
| What you need             | Python and a data file                                                                                                                                                                                                                                                                                                                          |
| Where the rules come from | [The Oil Money project page](https://github.com/je-suis-tm/quant-trading/tree/master/Oil%20Money%20project)                                                                                                                                                                                                                                     |
| The underlying research   | Ferraro, Rogoff and Rossi, [Can Oil Prices Forecast Exchange Rates?](https://www.barcelonagse.eu/research/working-papers/can-oil-prices-forecast-exchange-rates) (2011), which the source names as the closest published approach; the momentum exit is the author's own                                                                        |
| How well it held up       | Weak: one self-reported backtest per currency over a few years of daily data, with no trading costs and a position size chosen with hindsight, and no independent replication                                                                                                                                                                   |
| Also appears in           | The idea of a currency driven by a commodity in [the commodity currencies page](../../backtrader/commodity-currency/README.md); the siblings [The sign-constrained regression](../sign-constrained-regression/README.md) and [The model-free volatility index](../model-free-volatility-index/README.md) come from the same external repository |

## The idea in one paragraph

A currency's exchange rate and the price of the oil its country sells often move together, because
the same buyers and sellers act in both markets. This strategy refits a straight line between the two
every day, using only the most recent fifty observations. If that line is a strong description of
those fifty days, it waits for the currency to jump well away from the line and bets that the jump is
the start of a move rather than a mistake that will be corrected. If the line is weak, it refuses to
trade at all. A position - the amount of the currency the strategy currently owns or owes - is closed
after ten days or as soon as the currency has moved half a point from where it was bought, and then
the whole thing starts again. The unusual part is the refusal: the model has to have been demonstrably
good a moment before the departure it trades, so the departure is read as the relationship starting to
fail rather than as a price being cheap.

## Why anyone believed it

The economic story is that an oil exporter earns its foreign currency from crude, so the price of
crude and the price of the country's money should be tied together. A trader who sees the two move
together day after day comes to trust the tie, and starts to read the currency through the oil price.
The other side of the trade is everyone who has made that same assumption: producers who must
convert oil revenue, importers who must pay for it, and funds that buy the currency because oil is
rising. When the tie breaks and the currency leaves the line the oil price implies, some of those
holders are wrong-footed and have to trade out of their positions, and their own selling or buying
can push the currency further in the direction it has already moved. That is the momentum the author
claims to be capturing.

The author's own reading is more specific. While testing whether oil forecasts a petrocurrency, the
surprise was that the currency did little while the fit held, and moved sharply only after the fit
fell apart. The break is taken to mean that something fundamental has changed - a new outlook, not a
rounding error - and that a change in outlook tends to last for a while.

## An everyday comparison

An ice-cream van's daily takings follow the thermometer closely for weeks, so the owner starts to
predict each day's sales from the forecast temperature. One Saturday the takings jump far above what
the temperature predicts. That is not a bargain to be snapped up before the price returns to normal;
it is a sign that something new is happening, such as a festival in town, and new things tend to last
a few days. But the owner would trust that reading only if the sales had tracked the thermometer
closely right up to the jump; if the link had already been loose, an odd Saturday would say nothing.

## The rules, step by step

The rule runs once per day, on daily prices.

1. Collect two price series with matching dates: the currency's exchange rate, and the price of one
   crude-oil benchmark that carries the same story. The currency is the column to be explained; the
   benchmark is the column that explains it. The author uses five years of daily data.
2. On each day, look back only at the most recent fifty days.
3. Fit a straight line to those fifty days by least squares, the method that chooses the height and
   the slope so that the total squared distance between the line and the observed prices is as small
   as it can be.
4. Measure the strength of that fit. Refuse to trade unless the strength exceeds 0.70.
5. If it does, measure the typical size of the departures from the line, called sigma. Set two
   thresholds: the fitted value plus twice sigma, and the fitted value minus twice sigma.
6. If the currency's price is above the upper threshold, buy it, which is called going long. If it is
   below the lower threshold, sell it, which is called going short. The direction is the point of
   the rule: a break is treated as the start of a move, not as a reversion to the line.
7. Size the trade by dividing the starting cash by the highest price in the whole series. The author
   uses 5,000 units of cash and one position at a time.
8. Exit on the first of two conditions: the position has been held more than ten days, or the absolute
   distance between the current price and the entry price has reached 0.5 in the units of that price
   series. Then go back to step 2 and refit from the newest fifty days.

## The maths, with every symbol named

The line fitted to the currency on a given day:

```text
fitted_value = intercept + slope * benchmark_value
```

- `fitted_value` is the price the line predicts for the currency.
- `benchmark_value` is that day's crude-oil price.
- `intercept` is the height of the line where the benchmark is zero.
- `slope` is how much the currency price changes for a one-unit rise in the benchmark.

The slope and the height, computed from the fifty days in the window:

```text
slope = sum over the window of (benchmark_gap * price_gap) / sum over the window of (benchmark_gap * benchmark_gap)
intercept = average_price - slope * average_benchmark
```

- `benchmark_gap` is a day's benchmark minus the average benchmark over the window.
- `price_gap` is a day's currency price minus the average price over the window.
- `average_benchmark` and `average_price` are the two averages over the window.
- It means: the slope is the typical price move per unit of benchmark move, and the line is then
  anchored to pass through the two averages.

The strength of the fit and the size of the departures:

```text
strength = 1 - total_squared_departure / total_squared_price_gap
sigma = square root of (total_squared_departure / number_of_observations)
```

- `total_squared_departure` is the sum of the squared distances between each price and the fitted
  line.
- `total_squared_price_gap` is the sum of the squared distances between each price and the average
  price.
- `strength`, called R squared, is the share of the price movement the line accounts for; 1 is a
  perfect straight line through the points and 0 means the line does no better than the average
  price. `sigma` is the typical size of a departure from the line.
- It means: the gate opens only when the line already explains most of the recent price, and the two
  thresholds below then sit at a stated distance from it.

The thresholds and the two trade conditions:

```text
upper = fitted_value + 2 * sigma
lower = fitted_value - 2 * sigma
```

- Enter long when `price > upper`, enter short when `price < lower`.
- Exit when `days_held > 10` or when the absolute value of `price - entry_price` is at least 0.5.

## A worked example

Five days of a made-up currency and a made-up crude benchmark. The currency is quoted in units, the
benchmark in dollars per barrel, and the numbers are chosen to be plausible rather than real. The
first table works out the fit.

| Day | Benchmark (dollars per barrel) | Price (currency units) | Benchmark gap | Price gap | Gap product | Squared benchmark gap |
| --- | ------------------------------ | ---------------------- | ------------- | --------- | ----------- | --------------------- |
| 1   | 70                             | 9.00                   | -4.000        | -0.144    | +0.576      | 16.000                |
| 2   | 72                             | 9.02                   | -2.000        | -0.124    | +0.248      | 4.000                 |
| 3   | 74                             | 9.20                   | 0.000         | +0.056    | 0.000       | 0.000                 |
| 4   | 76                             | 9.16                   | +2.000        | +0.016    | +0.032      | 4.000                 |
| 5   | 78                             | 9.34                   | +4.000        | +0.196    | +0.784      | 16.000                |
| Sum | 370                            | 45.72                  | 0.000         | 0.000     | +1.640      | 40.000                |

The average benchmark is 370 divided by 5, which is 74.000, and the average price is 45.72 divided by
5, which is 9.144. The slope is the gap-product total divided by the squared-benchmark-gap total,
1.640 divided by 40.000, which is 0.041. The intercept is 9.144 minus 0.041 times 74.000, which is
6.110, so the line is price = 6.110 + 0.041 * benchmark.

The second table checks the line's strength.

| Day | Price | Fitted | Departure | Squared departure |
| --- | ----- | ------ | --------- | ----------------- |
| 1   | 9.00  | 8.980  | +0.020    | 0.000400          |
| 2   | 9.02  | 9.062  | -0.042    | 0.001764          |
| 3   | 9.20  | 9.144  | +0.056    | 0.003136          |
| 4   | 9.16  | 9.226  | -0.066    | 0.004356          |
| 5   | 9.34  | 9.308  | +0.032    | 0.001024          |
| Sum | 45.72 |        | 0.000     | 0.010680          |

The total squared departure is 0.010680. The total squared price gap is 0.077920, from squaring the
price gaps in the first table. The strength is 1 minus 0.010680 divided by 0.077920, which is
0.8629. That is above the 0.70 floor, so the gate is open. The sigma is the square root of 0.010680
divided by 5, which is 0.046217, so twice sigma is 0.092434.

Now a sixth day arrives. The benchmark is 80 and the currency is quoted at 9.50. The line, extended
to a benchmark of 80, predicts 6.110 plus 0.041 times 80, which is 9.390. The two thresholds are
9.390 plus 0.092, which is 9.482, and 9.390 minus 0.092, which is 9.298. The price of 9.50 sits above
the upper threshold, so the rule buys one unit at 9.50.

Suppose the price keeps climbing and, five days later, reaches 10.00, which is 0.50 above the entry
price, so the absolute stop closes the position. The gross gain is 0.50 per unit. The fee is invented
for this page, at one hundredth of a unit each side, so the round trip costs 0.020 and the net gain
is 0.480 per unit, or 5.05 percent of the 9.50 paid. The author's own numbers carry no trading cost.

## What the research actually found

The data are daily currency and crude prices committed with the project, five years ending in 2018,
one currency at a time, scored by the rule's own author. Every number below is gross of costs.

- For the Norwegian krone against Brent crude, the author reports about 2 percent over the test
  period before any tuning. Across a grid of settings the average return is about 2 percent with a
  range from about minus 6 percent to plus 6 percent. The best holding period in that grid is 9
  trading days, and the best stop is somewhere between 0.6 and 1.05.
- For the Colombian peso against Vasconia crude, the author reports a mean return of about 7 percent,
  with roughly 70 percent of the signals producing income, and a best setting of a 17-day hold with a
  stop between 0.002 and 0.0045 giving about 9 percent.
- The Russian ruble is rejected outright. The write-up says the currency is too driven by sanctions
  and political events for the method, and ends its section with "DON'T TRADE IT".
- The Canadian dollar's petrocurrency story is rejected, though the rule is still applied to it.

The author's central claim is directional rather than numerical: the money is made when the model
begins to break, not while it holds. The write-up says the strategy works only when the fit has
started to fail, and that a model 80 percent right is more useful here than one 100 percent right.
That claim is the author's reading of the same backtest, not a separate test.

Two limits come from the author. The repository states on its front page that all trades are assumed
frictionless, with no slippage, no surcharge and no illiquidity, so the percentages above ignore
costs. The author's own reading of the heatmaps is that the stop matters much less than the hold.

## How this project relates to it

This repository ships a runnable version of the rule in
[validity_gated_walk_forward.py](../../../examples/backtest/validity_gated_walk_forward.py), run from
the repository root with:

```bash
python examples/backtest/validity_gated_walk_forward.py
```

It builds two synthetic futures contracts - a futures contract being an agreement to buy or sell a
fixed quantity at a fixed price on a later date - one a benchmark and one whose level follows it. It
refits the line on every bar and applies the same 0.70 strength gate, entering long above the fitted
value and short below it, which is the source rule's direction convention. The price paths come from
a fixed formula, so the printed numbers are the same on every run and mean nothing about any real
market. What the example does show is the gate: it opens while the two prices move together and
closes once the traded price stops following the benchmark, exactly the precondition the rule needs.

Other pages in this collection that fit the same idea:

- [The commodity currencies page](../../backtrader/commodity-currency/README.md) explains the plain
  case of a currency following a commodity.
- [Scoring a strategy on its evidence, not its profit](../../project/scoring-evidence-not-profit/README.md)
  explains why a claim is graded on its evidence rather than on one profit number.
- [Telling which kind of market you are in](../../project/measuring-the-regime/README.md) covers why a
  relationship that holds in one period may not hold in the next.
- [How a backtest lies](../../foundations/07_how-a-backtest-lies.md) shows how a fit that matches
  history can still be the weakest kind of evidence.
- [The QuantConnect pair-trading pages](../../quantconnect/optimal-pairs-trading/README.md) re-test a
  rolling relationship and trade its departure, the same machinery without the gate.

## Where it goes wrong

- The position size is chosen from the whole sample. The portfolio code divides the starting cash by
  the maximum price in the whole series, so the units bought in one year depend on a price reached in
  a later year. That hindsight constant contaminates the whole equity curve.
- Two of the currency studies call a fit "out of sample" that was fitted on the out-of-sample part.
  The Canadian dollar and Colombian peso scripts fit the line on the test slice itself and print the
  resulting strength as out of sample, contradicting the write-up's claim that the out-of-sample fit
  beats the in-sample one. Those strength figures do not support the conclusion they are used for.
- The walk-forward test quietly ends. In the Norwegian script the loop breaks out entirely at the
  first two-sigma stop, so no further trades are generated and the later "performance" is simply
  undefined. The parameter heatmap is drawn from that truncated run.
- No costs are modelled anywhere. The repository assumes frictionless trading, so every number is
  gross. A strategy that refits daily and holds for a few days is one of the most cost-sensitive
  kinds there is.
- The samples are small and self-scored. Each currency gets a few years of daily data, one test by
  the rule's own author, and no independent replication, which is why the grade above is Weak.
- The relationship is regime-dependent. The author's own pre-2017 and post-2017 splits show the oil
  tie holding in one period and vanishing in the next, so the gate can leave the strategy idle for
  long stretches, and the headline returns come from the few windows where it happened to open.

## Try it yourself

You need a spreadsheet and no code.

1. Make five rows. Put the benchmark in one column, 70, 72, 74, 76, 78, and the currency price in the
   next, 9.00, 9.02, 9.20, 9.16, 9.34.
2. Add a column for the benchmark gap (the benchmark minus 74) and a column for the price gap (the
   price minus 9.144).
3. Add a column multiplying the two gaps, and a column squaring the benchmark gap. Total both. The
   slope is the first total divided by the second; reproduce 0.041 and the intercept 6.110.
4. Add a column for the fitted price, then a column for the departure (price minus fitted) and a
   column squaring the departure. The squared-departure total should be 0.010680.
5. Divide the squared-departure total by the squared-price-gap total of 0.077920 and subtract from 1;
   you should get about 0.863, above the 0.70 floor.
6. Take the square root of 0.010680 divided by 5 to get sigma, double it, and add and subtract the
   result from the fitted price at a benchmark of 80.

What to notice: the line fits these five points well enough to open the gate, and the size of the
opening decides how far the price must travel before a trade is allowed. A weaker fit, with a larger
sigma, would need a bigger jump for the same entry, and a fit below 0.70 would allow no trade at all.

## Where this came from

- [The Oil Money project README](https://github.com/je-suis-tm/quant-trading/tree/master/Oil%20Money%20project),
  the rules and the author's per-currency results.
- [Oil Money Trading backtest.py](https://raw.githubusercontent.com/je-suis-tm/quant-trading/master/Oil%20Money%20project/Oil%20Money%20Trading%20backtest.py),
  the rolling refit, the 0.70 strength gate, the two-sigma entry and the exit rules.
- [Oil Money NOK.py](https://raw.githubusercontent.com/je-suis-tm/quant-trading/master/Oil%20Money%20project/Oil%20Money%20NOK.py),
  the same rule with the holding-period and stop sweep, and the loop that stops at the first stop.
- [The repository's front page](https://github.com/je-suis-tm/quant-trading), the frictionless-trading
  caveat; the files read here are at commit 611b73f2 on the master branch.
- [validity_gated_walk_forward.py](../../../examples/backtest/validity_gated_walk_forward.py), this
  repository's runnable version of the gate.
- Ferraro, Rogoff and Rossi, [Can Oil Prices Forecast Exchange Rates?](https://www.barcelonagse.eu/research/working-papers/can-oil-prices-forecast-exchange-rates)
  (2011), the closest published approach the source names.

## Words used in this tutorial

- backtest: a test of a rule on past prices to see what it would have done.
- gate: a condition that must be met before a trade is allowed, here a strength floor of 0.70.
- least squares: the fitting method that makes the total squared distance between a line and the data
  as small as possible.
- long: owning the thing, so the position gains when its price rises.
- residual: the distance between an observed price and the value the fitted line predicts for it.
- sigma: the typical size of the residuals, used here to set the entry thresholds.
- short: owing the thing, so the position gains when its price falls.
- stop: a rule that closes a position once its price has moved a stated distance from the entry.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
