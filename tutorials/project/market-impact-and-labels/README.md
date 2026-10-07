# Market impact and its labels: how your own order moves the price

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                             |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | It changes the price of whatever a strategy buys or sells, in proportion to the size and speed of the order                                                                                                                       |
| How often it trades       | On every order large enough to move the price; small orders cost nothing under the model                                                                                                                                          |
| What you need             | Nothing but this page; a calculator makes the worked example quicker                                                                                                                                                              |
| Where the rules come from | [Entry/exit price engine: design, section 6](../../../strategies/entry_exit_engine_design.md) and the implemented models in [crates/execution/src/models/market_impact.rs](../../../crates/execution/src/models/market_impact.rs) |
| The underlying research   | [Price impact: the square-root law and its evidence](../../../strategies/books/02_market_impact_and_trading_cost.md), this repository's brief written from the market-microstructure literature                                   |
| How well it held up       | Mixed: the shape of the law is robust across markets, while the prefactor in front of it is uncertain by roughly a factor of two                                                                                                  |
| Also appears in           | [Execution at the price you get](../execution-at-the-price-you-get/README.md) and [Cost, and the break-even price](../cost-basis-and-breakeven/README.md)                                                                         |

## The idea in one paragraph

Market impact is the change in price caused by your own order. A small order is absorbed by whoever
was already waiting to trade and leaves the price where it was; a large order eats through the
waiting sell orders one by one, so each further slice is bought at a higher price. Two things decide
how bad that is: the size of the order relative to the amount normally traded, and the speed at which
it is worked, because a patient order gives the sellers time to come back. The project models this as
a whole number of price increments the fill price moves against the order, growing with the square
root of the size, and it labels each estimate with the provenance of that number.

## Why anyone believed it

Someone has to take the other side of your purchase. In a liquid market those counterparties are
mostly firms quoting a price in the hope of earning the gap between the buying and the selling price.
That work is not free: holding the thing they just bought carries risk, and the risk is larger the
more they are forced to hold. So they widen the price at which they are willing to sell the more of it
you want, and the more urgently you want it.

The belief behind the model is not that prices are inefficient. It is that liquidity has a price, and
that the price can be measured rather than guessed. Ask for much more than the market normally trades
and you are asking a few traders to carry a lot of risk quickly; ask slowly and more sellers arrive.

## An everyday comparison

A supermarket puts out twenty cartons of milk at one pound each. One shopper takes two and pays two
pounds. A second shopper wants all twenty: the shelf empties, and the shopper behind the counter,
seeing the demand, marks the next batch at a pound and five pence. The second shopper's average price
is worse than the shelf price for two reasons - the shelf did not hold enough, and the price moved
because the demand was visible. Had the second shopper come back over the afternoon and bought two
cartons an hour, the shelf would have refilled and the price would not have moved. Size and speed
decide the cost, and the counterparty is a market that has to be persuaded.

## The rules, step by step

1. Decide what one price increment is. A price increment is the smallest amount by which the listed
   price can change; for a price quoted to two decimals it is 0.01 of the currency.
2. For each fill, measure its size. The project measures the fill in the units of the thing being
   traded, and compares it with a reference quantity: the size at which the model's prefactor is
   defined to move the price by its own value in increments.
3. Compute the number of increments the fill should move the price, using the square-root rule below,
   round it down to a whole number, because a price can only move in whole increments, and cap it at
   a maximum chosen when the model is set up, so that a mistaken order size cannot produce an absurd
   price.
4. Apply the adjustment against the direction of the order: a buy is filled at a higher price by that
   many increments, a sell at a lower one.
5. Apply it only to a fill that takes an existing quote from the top of the book, which makes the order
   the aggressor of that trade. A deeper book prices its own depth as the order walks the levels, and
   adding the adjustment there would count the same size twice.
6. Calibrate the prefactor from data before trusting it. Calibrating means working out which value
   reproduces an observed impact: from each observation divide the increments the price moved by the
   square root of the observed size relative to the reference, and take the smallest and largest.
7. Record where the prefactor came from, because a number inferred from a public tape is not the same
   thing as a number measured from the venue's own records.
8. Report the label rather than hiding it. If the model's parameters could not be recovered from
   data, the cost figures it produced carry a label naming the failing parameter.

## The maths, with every symbol named

The concave model, which is the one this project calls the square-root model:

```text
increments = floor(prefactor * sqrt(Q / Q_ref))
```

- `increments` is the number of price increments the fill price moves against the order; zero leaves
  the price unchanged.
- `floor` rounds a number down to the nearest whole number, because a price moves in whole
  increments.
- `prefactor` is the number in front of the square root: how many increments a fill of exactly the
  reference size moves the price by.
- `Q` is the size of the fill being priced.
- `Q_ref` is the reference quantity, the size the prefactor is defined at.
- `sqrt` is the square root, so a fill four times the reference size moves the price twice as far as
  the reference fill, not four times as far. That is what "concave" means here: the cost grows with
  size, but more and more slowly.

The simpler linear model, kept for comparison, moves the price by one increment for every
`Q_per_increment` units filled, so its adjustment grows in exact proportion to the size:

```text
increments = floor(Q / Q_per_increment)
```

The calibration estimate, which inverts the formula for a single observed impact:

```text
prefactor_from_one_observation = increments_observed / sqrt(Q / Q_ref)
```

- `increments_observed` is how far the price actually moved, in whole increments, when a known
  quantity was traded.
- The result is what the prefactor would have to be for the model to reproduce that observation. A
  fit takes the smallest and the largest of these estimates as the range, because a single number
  would claim more precision than the data supports.

The de-bias, applied when the observations came from an anonymous public tape rather than from the
venue's own records:

```text
debiased_prefactor = prefactor / 2.0
```

- The divisor 2.0 is the project's recorded figure for the inflation caused by reconstructing whole
  orders from a tape of separate printed trades; the reconstruction makes the measured number about
  twice what it should be, so both ends of the range are halved. The model applies the upper end of
  the range, so an uncertain prefactor cannot make a simulated result look better than it is.

## A worked example

One fill at a time, with a prefactor of 5, a reference quantity of 1,000 units, and a cap of 10
increments. Each row is one fill.

| Fill size (units) | Size / reference | Square root | Prefactor times it | Increments charged |
| ----------------- | ---------------- | ----------- | ------------------ | ------------------ |
| 100               | 0.10             | 0.3162      | 1.5811             | 1                  |
| 250               | 0.25             | 0.5000      | 2.5000             | 2                  |
| 500               | 0.50             | 0.7071      | 3.5355             | 3                  |
| 1,000             | 1.00             | 1.0000      | 5.0000             | 5                  |
| 2,000             | 2.00             | 1.4142      | 7.0711             | 7                  |
| 4,000             | 4.00             | 2.0000      | 10.0000            | 10                 |
| 8,000             | 8.00             | 2.8284      | 14.1421            | 10 (capped)        |

Read the middle of the table: doubling the fill from 1,000 to 2,000 units moves the price from 5
increments to 7, and 5 times the square root of 2 is 7.07, rounded down to 7. Quadrupling it to 4,000
moves the price to exactly 10. The last row is why the cap exists: without it an 8,000-unit fill would
move the price 14 increments, so a mistaken order would move it as far as the arithmetic allows.

Now put a price on it. Suppose the thing trades at 50.00 dollars and the increment is 0.01 dollars, so
the 7-increment fill is filled at 50.07 instead of 50.00:

```text
adjustment in basis points = 0.07 / 50.00 * 10,000 = 14 basis points
```

A basis point is one hundredth of one percent, so the fill cost 0.14 percent of the price. At the cap
of 10 increments the adjustment is 0.10 dollars, which is 20 basis points, or 0.20 percent.

Now the calibration, from six observed impacts on one instrument, measured against a reference of 1,000
units. Each observation is one metaorder, a single large intention worked as several separate orders.

| Observed size | Increments the price moved | Implied prefactor |
| ------------- | -------------------------- | ----------------- |
| 1,000         | 3                          | 3.000             |
| 2,000         | 4                          | 2.828             |
| 4,000         | 5                          | 2.500             |
| 500           | 2                          | 2.828             |
| 8,000         | 7                          | 2.475             |
| 1,000         | 4                          | 4.000             |

The implied prefactor is the increments divided by the square root of the size ratio, for example
5 / sqrt(4.00) = 2.500 in the third row. The smallest is 2.475 and the largest is 4.000, so the fitted
range is 2.475 to 4.000 and the model applies 4.000, the pessimistic end.

If those six observations had been reconstructed from an anonymous public tape rather than taken from
the venue's own records, both ends would be halved first:

```text
lower = 2.475 / 2.0 = 1.2375      upper = 4.000 / 2.0 = 2.0000
```

and the label on the calibration would say so. That is the point of carrying the label: the same
digits mean different things depending on whether they were measured or inferred, and the project
refuses to let a tape-derived number be read as if it had been measured.

## What the research actually found

The brief behind this tutorial reads twelve papers on the square-root law; three of them measure it on
real data.

The largest sample is a complete survey of the Tokyo Stock Exchange covering 4 January 2012 to 2
November 2019 and 2,299 stock-datapoints, where a datapoint is one stock's set of trades. It reports
an exponent of 0.489 with a standard error of 0.0015, against the one-half the model assumes. The
prefactor is where the studies disagree. The same survey reports an average prefactor of 0.842 at the
stock level and 1.501 at the level of individual traders. A study of a single large American share,
Apple, over 178 trading days reports a raw prefactor of 0.69 with a range of 0.64 to 0.75, and notes
that reconstructing whole orders from an anonymous tape inflates the number roughly twofold, so it
adopts 0.34 after halving. Across the three markets the prefactor runs from 0.34 to 1.50.

Two further points matter. The exponent is stable enough to fix at one half while the prefactor is
not, which is exactly what the project's code does. And the mechanism is unsettled: one agent-based
model reproduces the law only when both splitting and market-maker replenishment are present.

The briefs are candid about the limits: the three measurements are three markets over three periods -
Tokyo 2012 to 2019, one American share over 178 days, and Bitcoin on one exchange in 2011 to 2013 - so
nothing here is a cross-asset law, and one study imposes its exponent rather than measuring it.

## How this project relates to it

The implemented models are in the
[market impact module](../../../crates/execution/src/models/market_impact.rs). It holds a trait with
one method, which returns the number of increments a fill moves the price, plus a linear model and
the concave square-root model. The prefactor is not a bare number but an interval with a source, and
the model applies the interval's upper bound. The
[Python bindings](../../../crates/execution/src/python/market_impact.rs) are beside it, so a caller
can also supply their own model as a Python object with an `impact_increments` method.

The calibration tooling is in the
[impact calibration module](../../../crates/execution/src/models/market_impact_calibration.rs). It
fits the prefactor from observed impacts, distinguishes a fit from the venue's own fills from a fit
from an anonymous tape, applies the de-bias as an explicit step, and measures whether the fitted
interval contains the known value it was fitted on, reporting that coverage beside the interval.

The label lives in the [venue config](../../../crates/backtest/src/config.rs). A venue declares the
verdict of a parameter-recovery check - identified, weakly identified or unidentified - and, when a
parameter failed, its name. The label is carried into the run summary under keys built from the venue,
such as `market_impact.BINANCE.identification` and `market_impact.BINANCE.failed_parameter`, and into
the report, where the [analyzer](../../../crates/analysis/src/analyzer.rs) appends it in square
brackets to each cost row the model's fills contribute to. The
[unidentified impact model test](../../../crates/backtest/tests/unidentified_impact_model.rs) pins
both halves: an unidentified model marks the rows and changes none of their values, and an identified
model prints no label at all. The wider cost picture, including a cap on an order's share of a day's
volume, is in section 6 of the [design](../../../strategies/entry_exit_engine_design.md).

## Where it goes wrong

- The prefactor is not recoverable from one instrument. The brief records that fitting the exponent
  per name produces a wide interval, and that the spread across names is comparable to the
  measurement error, so a prefactor fitted on one share is a guess about that share.
- Reconstruction from a public tape doubles the number. The published correction is to halve it, and
  the project records the label rather than applying the correction silently, which means a reader
  has to notice which number they are holding.
- The model sees size and not speed. The implemented adjustment is a function of the fill quantity
  alone, with no state and no randomness, so it cannot represent the extra cost of rushing an order.
  What the evidence does say is that the impact measured at the completion of an order is largely
  the same however the order was sliced, while the price paid along the way differs; the model
  captures the completion reading and not the path.
- Small fills are free and large ones are capped, both by choice. Rounding down to whole increments
  means a fill below one increment's worth of impact leaves the price unchanged, and the cap means a
  very large order is charged the same as the largest order the cap allows.
- It applies only to a fill that takes the top of the book. On a deeper book the levels themselves
  price the size, and charging the adjustment on top would count the same order twice, and the method
  can be an artefact of the market it was measured in, so the prefactor has to be refitted per venue.

## Try it yourself

You need a calculator and nothing else. The exercise is to see the shape of the square-root rule with
your own numbers.

1. Write down a prefactor of 2, a reference size of 1,000 units, and a cap of 20 increments.
2. Make a table with four columns: fill size, fill size divided by 1,000, the square root of that
   ratio, and the prefactor times it, rounded down to a whole number.
3. Fill in the rows for sizes 1,000, 2,000, 4,000 and 8,000. You should get 2, 2, 4 and 5 increments.
   Notice that quadrupling the size from 1,000 to 4,000 doubles the impact rather than quadrupling it.
4. Now go backwards: a fill of 4,000 units moved the price 5 increments against a reference of 1,000,
   so divide 5 by the square root of 4 and recover a prefactor of 2.5. Multiply it by 0.01 and divide
   by the price to turn increments into basis points of cost.

What to notice: the same fill size produces a different cost depending only on the reference size you
chose, and nothing in the arithmetic tells you which reference was right. That is why the project
carries a range and a label rather than a single number.

## Where this came from

- [The market impact module](../../../crates/execution/src/models/market_impact.rs), the trait and
  the two models, the formula `floor(prefactor * sqrt(Q / Q_ref))`, the interval that applies its
  upper bound, and the recorded inflation factor of 2.0 for an anonymous tape.
- [The impact calibration module](../../../crates/execution/src/models/market_impact_calibration.rs),
  the per-observation estimate `increments / sqrt(Q / Q_ref)`, the fit as a span of those estimates,
  the explicit de-bias step, and the coverage measurement.
- [The venue config](../../../crates/backtest/src/config.rs), the identification verdict and the label
  it produces, with [the analyzer](../../../crates/analysis/src/analyzer.rs) that appends the label to
  the cost rows.
- [The entry/exit price engine design](../../../strategies/entry_exit_engine_design.md), section 6,
  where the impact term, the participation cap and the break-even identities are set out.
- [The market impact brief](../../../strategies/books/02_market_impact_and_trading_cost.md), the source
  of the numbers above: the Tokyo survey's 2,299 stock-datapoints and exponent 0.489 with standard
  error 0.0015, the prefactors 0.842 and 1.501, the Apple raw 0.69 with range 0.64 to 0.75 falling to
  0.34 after the twofold correction, the range of 0.34 to 1.50 across three markets, and the square
  root beating the straight line by 22 information-criterion points.
- `2411.13965v3`, the complete Tokyo Stock Exchange survey, cited for the exponent and the two
  prefactor figures, and `2606.24019v1`, the Apple study, cited for the raw prefactor, its range and
  the twofold reconstruction bias.

## Words used in this tutorial

- aggressor: the side of a trade that crossed the price to get the trade done, as opposed to the side
  that was already waiting.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- market impact: the change in price caused by your own order.
- metaorder: a single large intention that gets split into many smaller orders by the buyer.
- prefactor: the number in front of the square root in the impact formula, which is the parameter
  that has to come from data.
- price increment: the smallest amount by which a quoted price can change.
- reference quantity: the size at which the prefactor is defined to move the price by its own value in
  increments.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
