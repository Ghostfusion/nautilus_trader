# Limit Order Book Dynamics: Queueing, Imbalance, and Passive-Fill Risk

Date: 2026-10-05. Revision 1.

This brief covers the limit order book as a state-dependent queueing system: order-flow imbalance and short-horizon price change, fill
probability and queue position, adverse selection of passive fills, latent liquidity and the concave impact law, resilience after market
orders, and deep-learning LOB models scored at transaction level. The slice holds 420 records matching the category, spanning 1999-2026, of
which 50 carry a journal reference; 40 candidates were screened and 12 are treated here in depth.

## What this covers

The sub-topics run from the statistical mechanics of the book (arrival, cancellation and market-order intensities as functions of queue state)
through order-flow imbalance, order placement and fill probability under time priority, the adverse selection passive fills carry, latent
liquidity as the source of concave impact, resiliency after liquidity-taking shocks, and deep-learning LOB models with transaction-level
evaluation. The most consequential point is that passive fills are not free: every serious model agrees a limit order fills preferentially when
the market moves against it, so a backtest that fills limit orders randomly against market orders is optimistic. The second is that the book is
reactive - intensities depend on queue state - so constant-intensity, constant-depth simulations mis-shape the book they are meant to
represent.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category | 420 |
| Years spanned | 1999-2026 |
| Records with a journal reference | 50 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

Selection kept papers whose results rest on stated data and either an explicit model a matching engine could reproduce or a quantified
empirical regularity, dropping uncalibrated theory and duplicate topics.

## The short answer

1. At a single price the book behaves as an M/M/1+M queue and its average shape satisfies b(p) = h(p) * C(p), incoming flow times cancellation probability, `1311.5661v1`.
2. Cancellation is increasing and concave in queue size, not linear, and market-order intensity falls exponentially with available volume, from Paris stocks with spreads of 1.43 and 1.99 ticks, `1312.0563v2`.
3. Order-flow imbalance explains about 65% of 10-second mid-price variation across 50 S&P 500 stocks, with slope inversely proportional to depth (depth-regression R^2 of 74%), `1011.6402v3`.
4. A generalized, stationarized imbalance raises out-of-sample R^2 from 32.89% to 83.57% at a 30-second horizon when snapshots are coarse and quotes move several ticks, `2112.02947v1`.
5. Fill probability is a first-passage event: 99.9% of limit orders are eventually cancelled, over 90% of executions happen at the best quotes, and beyond one tick from the touch it is negligible, `2403.02572v2`.
6. The conditional mid-price drift after a passive fill is negative: a live 10-year Treasury future study measured -0.0065 against a tick of 0.015625, close to half a tick, `2407.16527v1`.
7. Multi-venue placement is a convex cost problem solved by stochastic approximation: at S=500 shares the optimal two-venue cost was -0.85 cents/share versus 2.35 for pure market orders, `1210.1625v4`.
8. Monotone impact is concave and better explained by latent depth than visible depth; Euro Stoxx conversion rates sit an order of magnitude below the crisis threshold, `1808.09677v2`.
9. Spread and depth recover within about 20 best-limit updates after an effective market order, but recovery is order-type dependent, `1602.00731v2`.
10. DeepLOB mid-price MCC is 0.29 for large-tick stocks but 0.11 and below for small-tick names, where transaction probability collapses above a 0.5 threshold, `2403.09267v4`.

## 1. The book as a state-dependent queueing system

`1311.5661v1` models one side of the book with limit orders arriving as a spatial Poisson process, unit market orders as a Poisson process, and
standing limit orders cancelled after an exponential lifetime - an M/M/1+M queue per price. Birth-and-death algebra yields a closed-form
average shape (Eq. 12, p.8) and a law of conservation of flows, b(p) = h(p) * C(p), where C(p) is the probability a limit order at price p is
cancelled before execution (Prop. 2, p.8). For fixed incoming limit volume, larger average limit-order size implies a deeper book near the
spread (p.1, p.17). A panel regression over 14 Paris CAC 40 stocks, 391 half-hour intervals each, gives a coefficient of -0.2446 (t=-12.18) on
the number of limit orders, 8.71 (t=11.60) on average limit size and -4.85e-4 (t=-5.27) on market-order volume, with R^2 of only 0.079-0.084
(Table 2, p.22-23); empirical lifetimes are power-law, not exponential (p.5).

`1312.0563v2` generalizes this: within periods of constant reference price the LOB is a 2K-dimensional continuous-time Markov jump process whose
limit, cancellation and market-order intensities depend only on current queue state, with reference-price moves switching the model. Estimated
intensities show limit insertion roughly constant in queue size, cancellation increasing and concave - contrary to the linear cancellation of
earlier work - and market-order rate decreasing exponentially with available volume, described as rushing for liquidity when it is rare
(pp.8-9). Ergodicity holds under negative individual drift and bounded incoming flow (Thm. 2.1, p.5), invariant distributions track empirical
book distributions closely (p.10), and the maximal mechanical volatility is below realised volatility, so an exogenous news component is
required (p.3). `2403.02572v2` carries the same view into FX, computing fill time as the first-passage time of a pure-death process representing
the order's queue position (pp.22-24).

## 2. Order-flow imbalance and short-horizon price change

`1011.6402v3` defines order flow imbalance (OFI) as the signed net contribution of all events - limit orders, market orders, cancellations - to
the bid and ask queues, and regresses 10-second mid-price changes on OFI across 50 S&P 500 stocks in April 2010. The linear relation reaches
average R^2 of 65% with average slope beta = 0.0398 (t=11.47) and an intercept near zero (Table 2, p.11). The impact coefficient is inversely
proportional to depth: pooled estimates c^ = 0.45 and gamma^ = 0.98 with depth-regression R^2 about 74% (Table 3, p.13). Excluding
price-changing events lowers R^2 into the 35%-60% region (p.8), and a quadratic term raises average R^2 only from 65% to 68% with an
insignificant coefficient in most samples (p.9). Depth is about twice as low at the open as on average, so impact is twice average at the open
and five times average at the close (p.14).

`2112.02947v1` argues one-tick OFI is biased when snapshots are coarse. On CSI 500 constituents sampled every 3 seconds, generalized OFI (GOFI),
which allows the best quote to move more than one tick between snapshots, and a stationarized log version improve linear mid-price prediction.
Average out-of-sample R^2 rises from OFI at 32.89%, 38.13% and 42.57% to log-GOFI at 83.57%, 85.37% and 86.01% at 30 seconds, 1 minute and 5
minutes; GOFI alone reaches 45.73%, 51.88% and 56.76% (pp.5-6). Both papers agree best-quote imbalance is the robust short-horizon impact
variable, but they disagree on the volume-based square-root law: `1011.6402v3` calls it a statistical artifact of aggregating trades, while the
latent-book papers below treat it as mechanically reproduced. These are different objects - aggregated volume versus a single metaorder
- and should not be conflated.

## 3. Fill probability, queue position, and placement

Fill probability is a queue-depletion event under time priority, not a fixed decay in distance. `2403.02572v2` reports on LMAX EUR/USD spot that
more than 90% of executions occur at the best quotes and about 85% of executed limit orders were submitted within one tick of the best quote,
while 99.9% of limit orders are eventually cancelled (pp.22-23). Fill probabilities beyond one tick from the best quote are negligible (p.3), so
a placement optimizer can truncate there. The model assumes state-dependence through observed covariates only and FIFO time priority, so later
arrivals do not change an order's position.

`1210.1625v4` frames placement as convex cost minimization over the split between market and limit orders and across K exchanges, with cost
including half-spread, fees, effective rebates net of adverse selection, market impact and asymmetric under/over-fill penalties. On a single
exchange the optimal split is a quantile of the queue-outflow distribution (Eq. 9, p.14); limit size is bounded while market-order size grows
linearly once limit capacity saturates. A Robbins-Monro scheme solves the multi-exchange problem in 200 ms for 12 exchanges, converging in
1,000-10,000 iterations and within 2% of optimal after 50 (pp.18-19). For S=500 shares the optimal two-venue cost is -0.85 cents/share against
2.35 for pure market orders, 2.22 for a single limit venue and -0.57 for an equal split (Table 1, p.26). Because fills across venues are
imperfectly correlated, the solution overbooks and should raise consolidated depth (p.6, p.22).

## 4. Adverse selection of passive fills

`2407.16527v1` proves, in a trinomial model and a compound Hawkes process, that conditional on a fill the mid-price drift is negative whenever
fills always occur on adverse moves and only sometimes on favourable ones; formally E[d | fill] < 0 whenever the fill rate R_f < 1 (Eq. 3,
p.11). A live simulator on 10-year US Treasury bond futures, 6AM-1PM EST on 21 Nov 2023, filled 1,683 orders of which about one third stayed
unfilled, and measured an average post-fill drift of -0.0065 against a tick of 1/64 = 0.015625, close to half a tick (p.16). A calibrated model
with P(U) = P(D) = 0.0173, P(M) = 0.965, R_f = 0.018 and P(fill | down) = 0.99 gives a theoretical drift of -0.48 ticks against an empirical
-0.45 (Table 1-2, p.20). Three fill simulators compared against a live baseline of 60% on 30 Nov 2023 produced global fill rates of 85% (fill
against market orders), 30% (exponential fills) and 65% (adverse fills plus Bernoulli) (Table 5, p.23), and fill inter-arrival times are
heavier-tailed than exponential (p.23). Simple alpha signals reach only 15-25% correlation with future mid moves, so adverse fills cannot be
avoided. This contradicts the exponential fill-intensity assumption of the market-making literature and reinforces `1210.1625v4`, which prices
adverse selection into the effective rebate.

## 5. Latent liquidity and the concave impact law

`1802.06101v4` extends the latent order book by adding mean reversion of agents toward a reference price at rate lambda, recovering the original
model as lambda goes to 0. With a constant trading rate the impact is concave, so mean reversion counters the square-root decay kernel (p.8).
There is no closed-form impact solution, but a flexible solution family supports calibration, and existence extends to the original model (p.4,
p.11).

`1808.09677v2` adds a distance-dependent mechanism converting latent to revealed liquidity and fits it to Euro Stoxx futures and over 100
large-cap US stocks, Aug 2017 to Apr 2018 (p.9). It derives a market instability threshold: if conversion is too slow, revealed liquidity
vanishes in a liquidity crisis, with a critical kappa*theta_c near 1.875 for equal diffusivities and a stability condition kappa*theta < 2 when
latent diffusivity is zero (p.7). Fitted Euro Stoxx parameters are L=4599, k=2.12, theta_l=0.042 and theta_r=0.0084, with kappa*theta typically
an order of magnitude below the critical line and a diffusivity ratio of 0.01-0.1 (p.9). Calibration around the 5 Feb 2018 flash crash moves
the point closer to the critical line (p.10), and metaorder impact is a robust square root in both fast and slow regimes but diverges near the
liquidity-crisis point (p.12); visible depth is under 1% of daily volume. Here the square root is an endogenous, mechanically reproduced
stylized fact, in direct dispute with `1011.6402v3`, which reads the volume-based square root as an aggregation artifact; the two describe
different quantities.

## 6. Resiliency after market orders

`1602.00731v2` classifies effective market orders on the Shenzhen Stock Exchange in 2003 by penetrability, averaging spread, depth and intensity
over the 20 best-limit updates around each order with intraday seasonality removed. Spread and depth return to the sample average within about
20 best-limit updates after the shock (p.1, p.8). Effective market orders are more likely when spreads are low, same-side depth is high and
opposite-side depth is low, so liquidity taking is procyclical with displayed depth. Price resiliency dominates after aggressive orders with p
> 1, while price continuation follows less-aggressive orders (p.1). For a 1-tick initial spread, effective buy market orders attract more buy
limit orders via asymmetric stimulus, and Type-3 and Type-9 orders (filled, p=1) are the most numerous effective market orders (Table 1, p.6).
Data are 31 A-share and 12 B-share stocks, with results shown for one of each. Replenishment is therefore neither instantaneous nor uniform: a
matching engine should not assume depth returns at once, and post-fill drift should be conditioned on the aggressiveness of the triggering
order.

## 7. Deep-learning LOB models and transaction scoring

`1601.01987v7` trains a spatial neural network on NASDAQ Level III data for 489 stocks from 1 Jan 2014 to 31 Aug 2015, roughly 50 TB raw, with a
200-element input of the first 50 bid and 50 ask nonzero levels and about 5 billion samples at a 1-second horizon and 2.5 billion for the next
price move, on a 50-GPU cluster for over 3,000 node-hours. The spatial model beats a standard neural network on 94% of stocks for 1 second and
97% for the next move, with average error decreases of 0.6% and 3.5% (p.31); both beat logistic regression by about 10% and 20% and a naive
empirical model on nearly 100% of stocks (p.31). Local spatial structure is confirmed with a median coefficient ratio of 6.43 (p=0) and 12.83
(p=1) (Table 2, p.16). The spread is not constant: the median probability of bid and ask moving in lockstep is 17%, while a one-sided change
has median probability about 67% (Table 1, p.12).

`2403.09267v4` bounds how much of this is actionable. Using LOBSTER data for 15 NASDAQ stocks over 2017-2019 with DeepLOB across 10 levels and
horizons of 10, 50 and 100 book updates, average MCC without thresholds is 0.29/0.36/0.26 for large-tick stocks, 0.11/0.04/0.01 for small-tick
and 0.13/0.085/0.036 for medium-tick (pp.20-21); small-tick stocks misclassify 29% of true Up moves as Down at H10 (p.17), and large-tick F1
exceeds 0.45 without thresholds and 0.7 with probability thresholds (p.22). The transaction probability p_T falls as the threshold rises
because the signal sequence breaks: large-tick stocks retain p_T around 0.12 average at H10, while small-tick p_T goes to zero above a 0.5
threshold (Table 8, p.24-25). The dispute is real: `1601.01987v7` reports strong distributional gains, while `2403.09267v4` shows high MCC and F1
need not translate into executable transactions and are strongly tick-size dependent.

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| Book depth is endogenous to order size and cancellation rate | docs/concepts/order_book, python/nautilus_trader/model | Calibrate fills on per-order size and cancellation rate, not fixed depth. |
| Shape identity b(p) = h(p) * C(p) | docs/concepts/backtesting, python/nautilus_trader/backtest | Add a book-flow conservation check to any simulated book. |
| Intensities depend on queue state | python/nautilus_trader/model, docs/concepts/execution | Replace constant-intensity flow with queue-reactive intensities. |
| OFI explains about 65% of 10-second mid moves | python/nautilus_trader/indicators, docs/usermanauls/microstructure-signals | Compute best-quote OFI as a first-class feature, scaled by depth. |
| Impact coefficient is inversely proportional to depth | python/nautilus_trader/risk, docs/usermanauls/execution-algorithms | Calibrate impact against depth rather than one constant. |
| Coarse snapshots bias one-tick imbalance | python/nautilus_trader/data, python/nautilus_trader/model | Use generalized, log-stationarized imbalance on aggregated data. |
| Fill probability is first-passage of queue position | python/nautilus_trader/execution, docs/concepts/execution | Track queue position; truncate beyond one tick from the touch. |
| Passive fills carry negative drift | python/nautilus_trader/backtest, python/nautilus_trader/testkit | Make limit fills coincide with adverse moves; heavy-tailed inter-arrivals. |
| Latent depth drives concave impact | docs/usermanauls/execution-algorithms, docs/concepts/backtesting | Add a latent pool with a conversion-rate stress knob. |
| Resiliency takes about 20 best-limit updates | docs/concepts/data, python/nautilus_trader/data | Do not treat post-shock depth recovery as instantaneous. |
| Transaction probability, not MCC, measures usefulness | python/nautilus_trader/analysis, docs/usermanauls/ai-training | Score LOB models with a transaction-level metric alongside MCC/F1. |
| Predictability is tick-size dependent | docs/usermanauls/intraday-systematic, python/nautilus_trader/indicators | Gate LOB models by tick-size and liquidity regime. |

## Caveats

Most papers report R^2, classification metrics or model-implied probabilities, and few report net PnL after costs, so the evidence is largely
model-based rather than realised trading. Several results rest on a single market or venue - Paris and Shenzhen equities, NASDAQ stocks, LMAX
FX, one Treasury future - so cross-asset generality is unproven. The queueing and latent-book models are mean-field averages, not path-wise
reconstructions, and their stress regimes (liquidity crisis, flash crash) are model regimes rather than documented event analyses. Sample
periods are narrow: April 2010 for OFI, one day in 2023 for the fill-drift study, two stocks for the queue-reactive calibration. Cost, fee and
rebate effects are priced inside models such as the placement optimizer but rarely validated against live fills, and out-of-sample trading
tests are absent from most of the slice. Several results are contested - square-root law versus OFI, linear versus concave cancellation, and
whether deep LOB prediction is actionable - so none should be adopted as settled.

## Papers read in depth

- `1311.5661v1` - The order book as a queueing system: average depth and influence of the size of limit orders (2013). Closed-form average shape and conservation of flows; a bookkeeping identity for simulated books.
- `1312.0563v2` - Simulating and analyzing order book data: The queue-reactive model (2013). The minimum realistic intensity specification; state-dependence is not optional.
- `1011.6402v3` - The price impact of order book events (2010). OFI is the robust short-horizon impact variable, with slope inversely proportional to depth.
- `2112.02947v1` - The price impact of generalized order flow imbalance (2021). One-tick OFI is biased under coarse snapshots; the log-generalized form dominates.
- `1210.1625v4` - Optimal order placement in limit order markets (2012). Placement as convex cost minimization with explicit execution-risk penalties and multi-venue overbooking.
- `2407.16527v1` - The negative drift of a limit order fill (2024). Passive fills are adverse; exponential fill assumptions understate both drift and tails.
- `1802.06101v4` - Market impact in a latent order book (2018). Generative mean-reverting impact model; analytic and numerical only.
- `1808.09677v2` - How does latent liquidity get revealed in the limit order book? (2018). Calibrated latent conversion with a usable stability stress parameter.
- `1602.00731v2` - Limit-order book resiliency after effective market orders (2016). Recovery within about 20 best-limit updates and order-type-dependent continuation.
- `1601.01987v7` - Deep Learning for Limit Order Books (2016). Strong distributional gains from spatial structure; enormous compute and no trading test.
- `2403.02572v2` - Fill probabilities in a limit order book with state-dependent stochastic order flows (2024). Fill probability as first-passage of queue position; near-zero beyond one tick.
- `2403.09267v4` - Deep Limit Order Book Forecasting: A microstructural guide (2024). Predictability is tick-size conditional; MCC and F1 do not imply executable transactions.

## Where to next in the corpus

- `2106.11691v2` - two price regimes and a liquidity cushion; links latent depth to observable regime shifts.
- `2107.09629v3` - a Hawkes-Markovian queue model; an alternative queueing specification to compare.
- `2505.17388v1` - OFI stochastic modelling on CSI 300; tests generalized imbalance beyond 10 stocks.
- `2310.06079v4` - anomalous diffusion fluid-limit simulation; candidate generator for simulated book paths.
- `2301.08688v2` - reinforcement-learning execution on an agent-based simulator; connects the fill model to execution.
- `1810.10845v1` - attention network for jump prediction; a forecasting baseline against the deep-learning pair.
- `1204.1381v1` - price jump prediction via logistic regression; an older, weaker forecasting baseline.
- `1902.10743v2` and `2009.02808v1` - agent-based and equilibrium LOB models; sources for an endogenous price process.
- `1904.03058v2` and `1505.04936v1` - SPDE and Markovian order-book theory; grounding for a continuous-limit simulator.
- `2509.05107v1` and `2211.13777v3` - LOB generative and deep models; a pool to revisit if the two representatives prove insufficient.
