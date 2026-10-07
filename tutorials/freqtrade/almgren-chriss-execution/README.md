# Almgren-Chriss: choosing how fast to trade by balancing impact against risk

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                               |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Crypto pairs on a centralised exchange, bought for a rise or sold short for a fall                                                                                                                                                                                                                  |
| How often it trades       | A signal opens a trade, which is then filled in ten slices; the timing of the slices is adjusted as the trade unfolds                                                                                                                                                                               |
| What you need             | Nothing but this page; the model decides the pace of an order, not what to buy                                                                                                                                                                                                                      |
| Where the rules come from | [AlmgrenChrissStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/AlmgrenChrissStrategy.py) and the reference implementation it adopts, [joshuapjacob/almgren-chriss-optimal-execution](https://github.com/joshuapjacob/almgren-chriss-optimal-execution) |
| The underlying research   | Robert Almgren and Neil Chriss, [Optimal execution of portfolio transactions](https://www.risk.net/journal-risk/2161150/optimal-execution-portfolio-transactions), Journal of Risk, volume 3, number 2 (Winter 2000)                                                                                |
| How well it held up       | Mixed: the model is a published and widely cited result with a closed-form solution, but its inputs must be guessed from data, and the file publishes no measurement of the strategy it wraps                                                                                                       |
| Also appears in           | [TWAP execution](../twap-execution/README.md), which is this model with the risk term switched off                                                                                                                                                                                                  |

## The idea in one paragraph

You have a large order to fill, and there are two ways to get it wrong. Trade it all at once and your
own buying pushes the price up, so you pay a bad average price. Trade it very slowly and the price
may move against you while you wait, so you carry risk for longer. This model writes both costs down
as numbers, adds them with a weight on risk, and solves for the pace that makes the total smallest.
The answer is a schedule in which you trade more at the start and less later, by an amount that
depends on how risky the pair is and how much you dislike risk. When you dislike risk not at all, the
schedule becomes the even, equal-slice TWAP of the companion tutorial.

## Why anyone believed it

The two costs have opposite appetites. Market impact is created by your own order and grows the
faster you trade, so it pushes toward patience. The risk of holding the position is created by the
price moving on its own, so it pushes toward speed. Anything that trades a large quantity faces both,
which is why the problem has a mathematical answer rather than one rule of thumb.

The counterparty is different for the two costs. The impact cost is paid to the traders who were
resting orders in the exchange's queue and who sell to you at rising prices. The risk cost is paid to
nobody: it is the possibility that the market moves before you are done, measured as the spread of
outcomes rather than as a fee. The model assumes nobody is deliberately trading ahead of you.

## An everyday comparison

Consider a traveller who must buy a hundred tickets for a popular concert from a box office where the
price rises as the seats sell. Buying all hundred at once at the opening price is impossible, because
the later seats cost more than the earlier ones, and the rush itself pushes the price up. Waiting too
long is also risky, because the concert may sell out and the price on the resale market may jump. The
sensible buyer spreads the purchases over the day but buys more in the morning than at closing time,
and by how much depends on how afraid the buyer is of being left out. That is the Almgren-Chriss
trade-off, a pace that depends on cost and on fear.

## The rules, step by step

1. Work on a fifteen-minute timeframe, so each candle covers fifteen minutes.
2. Use the relative strength index, a number from 0 to 100 comparing recent upward moves with
   downward moves. Buy when it is below 45 and the candle has traded some volume; sell short, meaning
   sell first and aim to buy back lower, when it is above 55 with volume.
3. On each candle, look back over the last 96 candles, which is the previous twenty-four hours on a
   fifteen-minute chart. From that window compute four quantities: how much the price varied, how wide
   the candles were on average, how many coins traded on average, and the size of one slice relative
   to one candle.
4. Combine them into a single number called kappa, explained in the next section. Kappa measures how
   much the model wants to hurry. A large kappa means trade fast and front-load; a kappa of zero
   means trade evenly.
5. When a trade opens, send only the first slice. Its size is the whole intended order multiplied by
   the first slice's share, which is large when kappa is large.
6. One minute after each fill, recompute the share for the slices that remain and send the next slice.
   The share of what is left is found from kappa and the number of slices still to come.
7. To leave, sell in slices in the same way once the index has crossed back above 55 after a long, or
   below 45 after a short. The stop loss is a fixed minus ten percent, and the minimal return target
   is a flat two percent.

The exact settings in the file are: timeframe `15m`, stop loss `-0.10`, minimal return
`{"0": 0.02}`, short selling allowed, position adjustment enabled, `twap_num_slices` of 10,
`twap_interval_minutes` of 1, `vol_window` of 96, `factor_lambda` of 0.01, `eta_volume_fraction` of
0.01, `gamma_volume_fraction` of 0.1, a fallback `kappa_default` of 0.6 when the data are missing, and
a ceiling `kappa_max` of 5.0. The entry and exit-index rules are the same placeholders as in the TWAP
file.

## The maths, with every symbol named

The model starts from the one number that tells the schedule how much to front-load. The file builds
it in stages.

```text
tau = interval_minutes / candle_minutes
```

- `tau` is the length of one slice as a fraction of one candle. With a one-minute slice on a
  fifteen-minute candle it is 0.0667.

```text
eta = avg_spread / (eta_volume_fraction * avg_volume)
gamma = avg_spread / (gamma_volume_fraction * avg_volume)
```

- `eta` is the temporary impact coefficient: the extra price paid per coin per unit of trading speed.
- `gamma` is the permanent impact coefficient: the part of the price move that stays after your order.
- `avg_spread` is the average width of the last 96 candles, measured as the high price minus the low.
- `avg_volume` is the average number of coins traded per candle over the same window.
- The two volume fractions are the assumed fractions of the market's volume that your order represents;
  a smaller fraction means a larger coefficient and therefore a slower schedule.

```text
eta_tilde = eta - 0.5 * gamma * tau
```

- `eta_tilde` is the temporary impact after allowing for part of the permanent move arriving during
  the same slice. The file never lets it fall below `eta` itself, so the correction can only help.

```text
sigma_tau = sigma * sqrt(tau)
kappa_tilde_sq = factor_lambda * sigma_tau * sigma_tau / eta_tilde
```

- `sigma` is the standard deviation of the closing price over the last 96 candles, a measure of how
  much the price moves on its own.
- `sigma_tau` is that movement scaled to the length of one slice.
- `factor_lambda` is the risk-aversion weight: how many units of risk the trader is willing to trade
  against one unit of cost.

```text
kappa = arccosh(0.5 * kappa_tilde_sq * tau * tau + 1) / tau
```

- `kappa` is the hurry factor. `arccosh` is the inverse of the hyperbolic cosine, a standard
  mathematical function that turns the combined quantity back into a rate. The file caps `kappa` at
  5.0 and uses 0.6 when the inputs are missing.

The schedule itself is then a fraction of what is still outstanding. With `m` slices still to go, the
next slice takes

```text
f(m) = 1 - sinh(kappa * (m - 1)) / sinh(kappa * m), with f(1) = 1
```

- `f(m)` is the share of the remaining order sent in the next slice.
- `sinh` is the hyperbolic sine function, the same family as the `arccosh` above.
- `m` counts the slices still to come, including the one about to be sent.
- When `kappa` is zero, `f(m)` equals `1 / m`, which is exactly the equal-slice TWAP rule. That is
  stated in the file itself.

The trade-off hidden in these formulas is that the temporary impact cost of a slice is proportional
to the square of its size, so the sum of the squares of the slice sizes measures how much impact the
schedule pays. That sum is smallest when every slice is equal, so an even schedule is the cheapest in
impact, and front-loading buys speed at the price of impact.

## A worked example

A made-up pair with a price of 100.00. Over the last 96 candles the standard deviation of the closing
price is 2.00, the average high-minus-low is 0.50, and the average volume is 5,000 coins. The
risk-aversion weight is 0.01, the volume fractions are 0.01 and 0.1, and the slice is one minute on a
fifteen-minute candle.

```text
tau            = 1 / 15           = 0.0667
eta            = 0.50 / (0.01 * 5,000)   = 0.0100
gamma          = 0.50 / (0.1 * 5,000)    = 0.0010
eta_tilde      = 0.0100 - 0.5 * 0.0010 * 0.0667 = 0.00997
sigma_tau      = 2.00 * sqrt(0.0667)     = 0.5164
kappa_tilde_sq = 0.01 * 0.5164 * 0.5164 / 0.00997 = 0.2676
kappa          = arccosh(0.5 * 0.2676 * 0.0667 * 0.0667 + 1) / 0.0667 = 0.0345 / 0.0667 = 0.52
```

Kappa is about 0.52, so the schedule leans toward the front but is far from trading everything at
once. The order is 1,000 coins and the ten slices follow the fraction `f(m)` above:

| Slice | m remaining | f(m)  | Coins this slice | Coins remaining |
| ----- | ----------- | ----- | ---------------- | --------------- |
| 1     | 10          | 0.406 | 405.8            | 594.2           |
| 2     | 9           | 0.405 | 240.9            | 353.3           |
| 3     | 8           | 0.406 | 143.3            | 210.0           |
| 4     | 7           | 0.406 | 85.3             | 124.7           |
| 5     | 6           | 0.408 | 50.8             | 73.9            |
| 6     | 5           | 0.412 | 30.4             | 43.5            |
| 7     | 4           | 0.423 | 18.4             | 25.1            |
| 8     | 3           | 0.456 | 11.4             | 13.7            |
| 9     | 2           | 0.561 | 7.7              | 6.0             |
| 10    | 1           | 1.000 | 6.0              | 0.0             |

Compare this with the equal slices of TWAP, which sends 100.0 coins every minute, and with the
version of this same schedule when kappa is zero, which is TWAP exactly. Two numbers show the
trade-off:

```text
Impact proxy = sum of the squares of the slice sizes
TWAP         = 10 * 100.0 * 100.0            = 100,000
Almgren-Chriss = 164,673 + 58,033 + ... + 36 = 254,586
Ratio        = 254,586 / 100,000            = 2.55
```

So the front-loaded schedule pays about two and a half times the temporary impact of the even one.
What it buys is time out of the market:

```text
Average minutes before a coin is bought
TWAP           = (1 + 2 + ... + 10) / 10             = 5.50 minutes
Almgren-Chriss = (405.8*1 + 240.9*2 + ... + 6.0*10)/1,000 = 2.44 minutes
```

The model's settings chose a point on that line: 2.55 times the impact cost in exchange for holding the
money at risk for 2.44 minutes on average instead of 5.50. Setting `factor_lambda` to zero would give
kappa of zero and the even TWAP schedule.

## What the research actually found

The Almgren-Chriss paper is a mathematical result, not a measurement. It assumes that the temporary
impact of trading is proportional to how fast you trade, that the permanent impact is proportional to
how much you trade in total, that the price otherwise follows a random walk with constant
volatility, and that the trader dislikes the variance of the total cost. Under those assumptions it
derives the front-loaded schedule above and shows it is the one that minimises cost plus a risk
penalty. The trade-off itself, trade fast and pay impact against trade slowly and bear risk, is the
contribution; the parameters are the reader's problem.

This repository's own collection of the execution literature puts the result in context.
[The execution and liquidation brief](../../../strategies/books/01_execution_and_liquidation.md)
reports that the schedule is the reference front-loaded optimum, that raising the risk-aversion weight
steepens the front-loading, and that the risk-neutral limit collapses exactly to TWAP
(`1204.2717v4`). The same brief reports that the volume-weighted analogue is provably optimal for a
risk-neutral trader when the impact depends on traded volume (`1408.6118v4`), and that adapting the
schedule to conditions only helps when the volume process mean-reverts, in which case the best
adaptive schedule equals the expected volume-weighted one (`1701.08972v2`).

The measurement gap matters more than the theory. The same brief notes that the schedule is only a
plan and that real fills diverge from it, and
[the other execution brief](../../../strategies/books2/25_execution_impact_and_order_book.md) shows
that impact is concave and transient rather than linear and permanent, which is not the shape the
model assumes. The file in this repository publishes no backtest, so what the theory is worth for
this particular strategy is untested.

## How this project relates to it

[The execution and liquidation brief](../../../strategies/books/01_execution_and_liquidation.md) is
the direct treatment: it states the closed-form schedule, its limits and the papers quoted above.
[The market impact brief](../../../strategies/books/02_market_impact_and_trading_cost.md) measures
the size-to-impact relation that the model compresses into a single coefficient, and shows why the
linear assumption is only a local approximation. [The foundations page on orders and
execution](../../foundations/03_orders-and-how-they-execute.md) supplies the mechanics, the spread and
the queue, that the model abstracts away.

## Where it goes wrong

- The inputs are guesses. The risk weight, the two volume fractions and the volatility window are all
  chosen by hand, so a kappa computed from a quiet window can differ by a large factor from one
  computed from a busy window, and the same order gets a different pace on different days.
- The impact shape is wrong. The model treats temporary impact as proportional to trading speed, so
  that an even schedule is cheapest. Measured impact is concave in size, closer to a square root, and
  part of it decays after the order rather than staying. A model built on the wrong shape can still
  produce a plausible schedule that pays more than expected.
- Risk here means variance, and variance is symmetric. The schedule is penalised for the possibility
  of a favourable price move just as much as for an unfavourable one, so it hurries to avoid a risk
  that a directional trader might be happy to hold.
- The file clamps the result. Kappa is capped at 5.0, missing inputs fall back to 0.6, and the last
  slices shrink below the exchange minimum and are skipped. Those practical fixes mean the executed
  schedule is not the closed-form optimum the theory describes.
- It says nothing about the signal. The model is a way to fill an order that has already been
  decided; a good schedule around a bad entry still loses.

## Try it yourself

You need a spreadsheet and nothing else; the arithmetic is the point.

1. Put `kappa = 0` in one cell and, in a column, the numbers `m = 10` down to `m = 1`.
2. In the next column, compute `f = 1 - sinh(kappa*(m-1)) / sinh(kappa*m)` and watch every value come
   out as `1/m`, which is one tenth. This is the TWAP limit.
3. Change `kappa` to 0.52 and recompute. The first value becomes about 0.41 and the later values grow
   toward one.
4. Chain the values into slice sizes for a 1,000-coin order: each slice is the fraction times what is
   still outstanding. Sum the slice sizes and check that they add to 1,000.
5. Add a column of the slice size squared, sum it, and compare with the ten-times-one-hundred square
   total of the even schedule. Then set `kappa` to 2.0 and repeat, to see how much more goes in the
   first two slices and how the sum of squares rises again.

What to notice: kappa alone decides the pace, kappa of zero reproduces the even schedule exactly, and
the impact proxy and the average time at risk always move in opposite directions. The model is not
choosing a winner; it is choosing a point on that line.

## Where this came from

- [AlmgrenChrissStrategy.py](https://github.com/freqtrade/freqtrade-strategies/blob/main/user_data/strategies/AlmgrenChrissStrategy.py),
  the source of the settings, the kappa computation and the slice fractions reproduced above.
- [joshuapjacob/almgren-chriss-optimal-execution](https://github.com/joshuapjacob/almgren-chriss-optimal-execution),
  the reference implementation the file says it adopts.
- Robert Almgren and Neil Chriss, [Optimal execution of portfolio transactions](https://www.risk.net/journal-risk/2161150/optimal-execution-portfolio-transactions),
  Journal of Risk, volume 3, number 2 (Winter 2000), the original model; a readable copy is also hosted at
  [quantitativebrokers.com](https://quantitativebrokers.com/s/Optimal-Execution-of-Portfolio-Transaction-_-AlmgrenChriss-1999.pdf).
- [The execution and liquidation brief](../../../strategies/books/01_execution_and_liquidation.md),
  this repository's collection of the execution results, including the papers `1204.2717v4`,
  `1408.6118v4` and `1701.08972v2` quoted above.
- [The market impact brief](../../../strategies/books/02_market_impact_and_trading_cost.md), the
  source of the square-root impact law that the linear model approximates.

## Words used in this tutorial

- front-load: to trade a larger share of the order earlier rather than equally across the schedule.
- kappa: the hurry factor of the model, near zero for an even TWAP schedule and larger for a
  front-loaded one.
- permanent impact: the part of your price move that stays after the order finishes.
- risk aversion: how much a trader dislikes not knowing the final outcome, here written as the weight
  on the variance of the cost.
- temporary impact: the part of your price move that fades once you stop trading.
- variance: a measure of how widely a result can scatter around its average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
