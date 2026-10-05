# Options, Hedging and Derivative Instruments

Date: 2026-10-05. Revision 1.

This brief covers 242 records on options and derivative instruments, spanning 1998-2026, of which
23 carry a journal reference. From 40 candidates I read 12 papers in depth on hedging-induced
market impact, joint option-market-making control, closed-form hedging cost and margin,
Greek-weighted cross-impact, perpetual funding and event-linked contracts, liquidation margins, and
backtest cost realism.

## What this covers

The slice asks what happens to a derivatives desk once hedging and execution are real frictions. It
covers hedging as a metaorder that moves the underlying, joint quoting and hedging for an option
market maker, closed-form hedging cost and margin for a delta-hedged book, Greek-weighted
cross-impact, perpetual funding and event-linked mechanics, fat-tailed liquidation margins, and
cost discipline in backtests. The most consequential point is that option hedging is not an overlay
on pricing: it is a metaorder whose permanent and transient impact feeds back into quotes,
inventory and price, and whose cost grows with the square of size. The work splits between formal
microtheory, calibrated simulation and real-data empirics, each with visible limits.

## The corpus slice

| Metric | Value |
|---|---|
| Records matching the category | 242 |
| Years spanned | 1998-2026 |
| Records with a journal reference | 23 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

The selection rule took a mandated must-cover list on hedging impact, option-market-making control,
margin and cross-impact, then added the strongest real-data and methodology papers on hedging cost,
perpetual open interest, liquidation margin and backtest validity; theme duplicates were dropped.

## The short answer

1. Re-hedging a short option is a metaorder whose impact series converges iff kappa is in (-1,1),
   with the metaorder regime kappa in (0,1) (`1910.05056v2`).
2. The impact of an (N,K) split is bounded, S + S N <= S_N,K <= S e^N, so a linear impact model
   understates large hedges (`1910.05056v2`).
3. The permanent-to-transient impact ratio of a hedging metaorder lies in [1/2, 1], near 2/3 for
   intraday flow and near 0.5 for 50-day reversion (`1910.05056v2`).
4. Under hedging impact the best option-market-making policy is a partial hedge plus inventory
   control through quotes, not a full immediate delta hedge (`2511.02518v2`).
5. Expected liquidity cost of a delta-hedged book is N^2 S0 I, quadratic in the number of options
   N and linear in the supply-curve slope (`2103.15302v1`).
6. Once execution costs matter, the option value is size- and inventory-dependent and convex in the
   underlying position (`1311.4342v8`).
7. Cross-impact is set by a small factor covariance and the Greek sensitivity matrix, and only
   cross-impact models explain implied-volatility level moves (`2102.02834v2`).
8. A perp's peg to spot emerges from a simple funding/basis mechanism, so funding and order-lifetime
   design can be swept safely in simulation (`2501.09404v1`).
9. Gaussian margins understate required collateral in crypto perps by roughly a factor of two, and
   inverse contracts need direction-specific margins (`2102.04591v1`).
10. Full fee, slippage and funding accounting can roughly halve apparent CAGR (`2512.22476v3`).

## 1. Hedging is a metaorder with permanent and transient impact

Re-hedging a short European option is itself a market impact scenario, a metaorder, and its
convergence is characterisable. Under local linear impact S -> S + S^(1+kappa) N the scenario
converges iff kappa lies in (-1,1) (Theorem 1, p.7), with the metaorder regime exactly kappa in
(0,1) (Definition 8, p.19) (`1910.05056v2`). For an (N,K) equal-child split the average execution
price is bounded, S + S N <= S_N,K <= S e^N (Theorem 3, p.9); as K grows without bound the impact
tends to S(e^N - 1) and the average price to S(e^N - 1)/N, so manipulation vanishes at infinite
trading frequency (Theorem 4, p.10). The pricing PDE @t u + (1/2) sigma^2 s^2 @ss u/(1 - lambda) =
0 with lambda = s^(1+kappa) @ss u stays parabolic and replicates every European claim with sup
s^(1+kappa) @ss u < 1 (Theorem 7, p.14). The permanent/transient ratio I/I_temp lies in [1/2, 1]
(Theorem 10, p.22), cited near 2/3 for intraday metaorders and 0.5 for 50-day reversion. Re-hedging
frequency sets the pricing operator, so a backtest must use that frequency's impact law.

## 2. Joint quoting, hedging and inventory control for an option market maker

A stochastic-control model of one European call on a book with permanent plus transient impact and
Hawkes order flow makes the feedback explicit. No-arbitrage is structural: no instantaneous round
trip, P_B - P_A - 2c <= -delta q - 2c < 0 (Lemma 3.3, p.11); pure-execution PnL <= -(delta/2) V_T
- c H_T <= 0 (Proposition 3.2, p.11); transaction-triggered manipulation is ruled out (Proposition
3.3, p.12); a terminal channel exists but is bounded (Proposition 3.4, p.14) with a finite value
function (Theorem 3.1, p.14); Hawkes intensities are subcritical, kappa/theta < 1 (Assumption 2.3,
p.6) (`2511.02518v2`). The learned policy beats a naive benchmark most at large inventory: terminal
cash (Table 3, p.24) is -263.43 vs -232.58 at I0 = -100, -91.45 vs -81.62 at -50, 55.68 vs 55.43 at
0 and 181.35 vs 252.13 at +100; at I0 = -100 a full delta-neutral hedge would need about 75 units,
but the learned agent takes about 50 and unwinds the rest through quotes (p.23). Mean-PnL
sensitivity to the hedging penalty is monotone: kappa = 0.5 gives -199.72, 2 gives -223.93, 4 gives
-232.62, 8 gives -245.89 and 16 gives -263.02 (Table 4, p.26); under asymmetric intensities the
learned mean is 72.16 against 72.00 (Table 5, p.28) (`2511.02518v2`).

## 3. Hedging cost and margin in closed form

With cost the integral of price difference against a linear supply curve, the risk-neutral expected
liquidity cost is N^2 S0 I, linear in the supply-curve slope, the stock price and the unit cost I,
and quadratic in the number of options N (Theorem 2, p.10) (`2103.15302v1`). The closed form uses
the first-order supply slope under continuous trading and a different slope beta_0 under discrete
rebalancing (p.11); the unit cost I is roughly 0.21 at the money and near-flat in volatility and
maturity (p.15). At sigma = 0.3, r = 0.05, K = 1.0, T = 0.1 the numerical value 0.2040 matches
simulation values 0.2039 (hourly) and 0.2035 (threshold), with a 99% CI half-length of 0.0030
(Table 2, p.17). Under CARA-utility indifference pricing with convex execution cost L(rho) =
eta |rho|^(1+phi), the price theta is at least the frictionless price when mu = r = 0 (Proposition
1, p.8) and is convex in the underlying position q (Proposition 2, p.9) (`1311.4342v8`); it is not
proportional to nominal, and rescaling a nominal N introduces a gamma*N term (p.9).

## 4. Greeks as the aggregation layer for cross-impact

Modelling derivatives as deterministic functions of stochastic factors, impact contributions can be
absorbed into a Brownian motion, so Greeks are unchanged by order-flow dynamics and standard pricing
methods still apply (Proposition 1, p.5) (`2102.02834v2`). The full cross-impact matrix is
determined by the small factor-factor matrix and the sensitivity matrix Psi (Proposition 2, p.5),
with the calibration formula in Proposition 3 / eqn 12 (p.6); it stays well-defined when factors are
not traded provided Psi^T eta_QQ Psi is positive definite (p.6). Delta-weighted option volume
therefore forms one pool with the underlying and vega-weighted volume another. Explained variance
by direction (Table 1, p.12) is about 0.18-0.20 for spot under all models, but for the level factor
it is -0.03 for Black-Scholes / direct-2d, -0.14 for direct-4d, 0.12 for 2d Kyle and 0.14 for 4d
Kyle. Only the cross-impact models explain the level factor; single-factor spot models cannot
(p.12), and every score is in-sample.

## 5. Perpetual contracts, funding, margin and liquidation

A perp's peg can be reproduced from a minimal mechanism. In an agent-based CLOB with an exogenous
geometric-Brownian spot signal and chartist/noise agents, the perp tracks spot and the Pearson
cross-correlation peaks at lag 2 (pp.12-13) (`2501.09404v1`); chartist-only flow gives the tightest
control bounds of -5.6 and 5.7 while noise-only flow drifts more (p.16), a sample Shewhart chart
shows UCL 8.5, LCL 5.7, center 1.4, 89 violations and 148 consecutive runs (p.13). Funding and
order-lifetime are design parameters worth sweeping, though the simulator is synthetic.

Event-linked contracts resist a universal engine: the contract is a tuple C = (g,T,S,C) of
geometry, temporal rule, settlement rule and venue/oracle composition (Definition 1, p.5)
(`2605.10428v2`). A shortfall occurs exactly when net collateral is below the adverse
entry-to-terminal loss, and a probability long with collateral x q0 / L is short for every L > 1
(Proposition 1, p.4); the ratio u/v diverges as v -> 0, so bounded leg errors give unbounded ratio
error (Proposition 2, p.7); after the first leg finalises the residual is affine, S_t = Y_A -
p_t^(B) (Proposition 3, p.8); entropy settles to 0 at binary finality with max pre-terminal entropy
log 2 (Proposition 5, p.11); and fixed-path replay does not identify deployment behaviour if the
contract changes its own reference process (Proposition 6, p.11).

Crypto perp margins are asymmetric and far above Gaussian estimates. Fitting generalized extreme
value tails to 5-minute BitMEX BTC perpetual prices (431,346 observations, 1 Jan 2017 - 6 Feb
2021), daily forced liquidation to open interest averages 3.51% long and 1.89% short (medians 1.22%
and 0.86%), and average liquidated-trader leverage is about 60X (58.13X long, 59.94X short),
described as lower bounds (Table 4, p.21) (`2102.04591v1`). At a 1% daily margin-call probability
the optimal margin is about 33% (3X) long and 20% (5X) short; 1-day figures are 32.56% long, 21.01%
short and 26.10% pooled (Table 3, p.20); a normal-return assumption understates optimal margins by
at least half (p.9); and the speculation index averages 3.75 against 0.15 for the S&P 500, 0.21 for
the Nikkei and 0.45 for the DAX (p.10).

## 6. Backtest cost discipline and the choice of hedging frequency

Writing SPXW weekly options and hedging with Black-Scholes or Variance-Gamma deltas over 2018-2023,
BSM generally hedges better than VG and intraday rehedging at 130 minutes is the best balance. The
buy-and-hold benchmark returns 9.889% annualised with stdev 0.206 and max drawdown 0.340; the best
short-call variant (BSM delta, 130-minute rehedge, 2% OTM) returns 6.504% with stdev 0.102 and max
drawdown 0.209; the highest cited risk-adjusted score is BSM delta, 130-minute, 5% OTM at IR***
13.962, return 3.674%, stdev 0.047 (Table 4 / p.11) (`2407.13908v1`). S&P 500 daily returns are
mean 0.046%, stdev 1.298%, skew -0.509 and kurtosis 12.878 (Table 3, p.8).

The same discipline governs perp backtests: on the BTC core stress configuration the cost ladder
(Table 16, p.32) gives rigorous full-cost CAGR 2.726, Sharpe 2.049, max drawdown 0.265; fee-only
4.308, 2.506, 0.262; and zero-cost 5.225, 2.711, 0.260 - fee-only inflates CAGR about 1.6x and
zero-cost about 1.9x (`2512.22476v3`). Two-stage screening over nine cost scenarios lowers training
monthly geometric return to 0.222 from 0.145 one-stage (Sharpe 3.164 vs 2.388); the block-bootstrap
95% CI for the difference is -0.016 [-0.040, 0.008]; and the selected configuration identity
toggles across a 3x3x3 policy grid with validation CAGR spanning 0.332-1.058 (pp.30-31). Data
integrity is a prerequisite: ByBit's BTC_USDT_P shows open-interest total variation of $45.66B
against traded volume of $30.32B, an excess of $15.34B in January 2023 and $56.84B in
July-September 2023, with violations in 72.4% and 75.8% of 1-minute intervals (`2310.14973v2`);
believing the reported open interest would require volume above $128bn, and scaling by honest
venues implies $156bn-$213bn (pp.5-6).

## What this project can take from it

| Finding | Where it lands | What to do |
|---|---|---|
| Hedge metaorder (`1910.05056v2`) | python/nautilus_trader/backtest | Model two-part hedge impact |
| Hedge size bounds (`1910.05056v2`) | docs/concepts/execution | Add exponential impact ceiling |
| Partial hedge is best (`2511.02518v2`) | docs/usermanauls/market-making | Tune hedge ratio |
| Cost is N^2 S0 I (`2103.15302v1`) | python/nautilus_trader/risk | Scale margin checks with size |
| Inventory-dependent value (`1311.4342v8`) | docs/concepts/options | Make pricing position-aware |
| Greek-weighted impact (`2102.02834v2`) | docs/concepts/greeks | Key cross-impact by Greeks |
| Inventory dislocations (`2002.08207v1`) | python/nautilus_trader/analysis | Flag inventory risk |
| Funding peg design (`2501.09404v1`) | docs/concepts/continuous_futures | Sweep funding params |
| Event perp rules (`2605.10428v2`) | docs/concepts/synthetics | Add per-leg finality state |
| Margins are fat-tailed (`2102.04591v1`) | python/nautilus_trader/risk | Use fat-tailed margins |
| OI vs volume check (`2310.14973v2`) | python/nautilus_trader/data | Check OI against volume |
| Full costs halve CAGR (`2512.22476v3`) | docs/concepts/backtesting | Enforce t+1 execution |

## Caveats

The evidence does not establish several things. Much of the impact and cost result is theory or
simulation: the metaorder bounds, the pricing PDE and the option-MM policy are derived without
empirical calibration, and the perpetual and event-perp simulations are synthetic. Empirical
coverage is narrow: the VSTOXX study is one product over May 2016 to August 2019 with a random
train/test split, the SPX backtest is a single 2018-2023 window, and the margin study is one venue.
Market impact is excluded from the cost-stress backtest so its small-account results are not
scalable, while the closed-form cost assumes a linear supply curve and the SPX comparison is
confounded with its sizing. One contest stays open: whether infinite re-hedging frequency removes
manipulation (`1910.05056v2`) or a bounded terminal channel survives (`2511.02518v2`), and whether
VSTOXX deviations come from inventory (`2002.08207v1`) or order flow (`2102.02834v2`).

## Papers read in depth

- `1910.05056v2` - How Option Hedging Shapes Market Impact (2019). Turns re-hedging into a metaorder and bounds its impact; no data, but a structural template for hedger cost.
- `2511.02518v2` - Option market making with hedging-induced market impact (2025). Joint quoting/hedging/inventory control; simulated, and the learned policy beats naive full hedging.
- `2103.15302v1` - Analytic formula for option margin with liquidity costs under dynamic delta hedging (2021). Closed-form N^2 S0 I cost verified against simulation; no empirical calibration.
- `1311.4342v8` - Option pricing and hedging with execution costs and market impact (2013). Indifference pricing makes option value size- and inventory-dependent.
- `2501.09404v1` - Agent-Based Simulation of a Perpetual Futures Market (2025). Minimal funding mechanism pegs a synthetic perp; useful design tool, synthetic evidence.
- `2605.10428v2` - A Taxonomy of Event-Linked Perpetual Futures (2026). Contract tuple and six propositions; explicitly reports no new empirical estimates.
- `2002.08207v1` - Inventory effects on the price dynamics of VSTOXX futures (2020). Real Eurex data; inventory dominates deviations, but its train/test split is random.
- `2102.02834v2` - Cross impact in derivative markets (2021). Greek-weighted cross-impact fitted on real E-Mini/VIX data; explains the level factor, in-sample only.
- `2310.14973v2` - Reconciling Open Interest with Traded Volume in Perpetual Swaps (2023). A checkable invariant large venues violate; cannot tell which of OI or volume is wrong.
- `2102.04591v1` - Liquidation, Leverage and Optimal Margin in Bitcoin Futures Markets (2021). Extreme-value margins far above Gaussian; leverage inferred, one venue.
- `2407.13908v1` - Construction and Hedging of Equity Index Options Portfolios (2024). Real SPXW backtest showing rehedging frequency is a first-order lever; single window.
- `2512.22476v3` - AutoQuant (2025). Execution-constrained auto-tuning; shows cost realism and screening can roughly halve CAGR; no live trading, impact excluded.

## Where to next in the corpus

- `2605.10400v2` - resolution-aware perp companion; likely fills settlement-detail gaps in the event-perp taxonomy.
- `2511.22766v1` - beta-dependent gamma feedback; tests the hedging-feedback theory through a different channel.
- `2510.04569v1` - risk-sensitive option MM; an alternative control formulation to compare with the joint quoting/hedging model.
- `2510.15937v1` - SPX-VIX tail-safe hedging; adjacent tail-hedging design for the SPX backtest.
- `2010.12245v1` - RL option hedging; an alternative to the control-theoretic hedging policy.
- `2605.05089v1` - collateral control for perp basis trading; follow-on to margin mechanics.
