# Market Impact and Trading Cost: The Square-Root Law, Its Mechanisms, and What Fills Can Identify

Date: 2026-10-05. Revision 1.

This brief covers a slice of the market-impact corpus: 435 records span 1999 to 2026, and 12 papers were read in depth from a 40-candidate list, on the square-root law (SRL), its prefactor and bias corrections, its mechanical versus informational origins, latent-order-book transient/permanent decomposition, cross-impact, and impact games. It is written for an operator running a Python/Rust algorithmic trading platform across equities, futures, FX, crypto, options and prediction markets, and it treats the impact model as a component that research, backtesting, paper and live execution must all share.

## What this covers

The slice contains three quantitative SRL measurements on real data (Tokyo Stock Exchange, AAPL, MtGox Bitcoin), one agent-based counterfactual that isolates the mechanism, one reaction-diffusion theory that reconciles trade-sign memory with the SRL, latent-order-book and nonlinear-permanent-impact theory, term-structure cross-impact theory, and two equilibrium games on predatory trading. The single most consequential thing in the slice is that the concave power law is robust while its surface explanation is not: the size-distribution and microstructure-exponent derivations are rejected or contradicted, the prefactor is an order of magnitude more uncertain than the exponent, and the transient-versus-permanent split is provably not identifiable from execution data. A platform should therefore treat the SRL as a baseline cost shape with an estimable-but-biased prefactor, run its own null tests, and refuse to interpret a measured decay kernel as a physical constant.

## The corpus slice

| Metric | Value |
|---|---|
| Records matching the category | 435 |
| Years spanned | 1999-2026 |
| Records with a journal reference | 63 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

Selection rule: one paper per claim-bearing item in the brief, prioritising empirical SRL measurements with printed numbers, mechanism claims, cross-impact and impact-game theory, and first-hand sources for the square-root and fair-pricing formulas.

## The short answer

1. Average metaorder impact is a concave power law, I/sigma_D = c (Q/VD)^delta, with delta ~ 0.5: the TSE survey reports <delta> = 0.489 +/- 0.0015 (SEM) over 2,299 stock-datapoints (`2411.13965v3`).
2. The prefactor is far less stable than the exponent: c ranges from 0.34 after anonymous-tape de-biasing to 0.69 raw on AAPL, 0.842 on TSE stocks and 1.501 on TSE traders (`2606.24019v1`, `2411.13965v3`).
3. Metaorder reconstruction from an anonymous tape inflates the prefactor roughly twofold, so an AAPL backtest reading public trades must divide c_raw = 0.69 by 2 to get c_eff = 0.34 and should report both bounds (`2606.24019v1`).
4. The SRL is not an artefact of HFT, arbitrage or a power-law size distribution: it holds over four decades in volume on MtGox Bitcoin with ~0.6% fees and almost no market making (`1412.4503v3`).
5. Impact is largely schedule- and identity-invariant: ID-reshuffled synthetic metaorders obey the same square-root law and completion impact is independent of execution time T (`2502.16246v2`).
6. A minimal agent-based model reproduces the SRL only when order splitting and market-maker replenishment are both present; removing splitting drops delta from 0.549 to 0.324 and removing HFT replenishment drops it to 0.386 (`2607.04280v1`).
7. Visible-book-only (LOB-walking) capacity models understate impact, while volume-tail-exponent models overstate it; both are rejected against the TSE target (`2607.04280v1`).
8. Post-trade impact reverts by roughly one third on average (permanent ~ 2/3 of peak) in both an equilibrium theory and the Bitcoin data (`1102.5457v4`, `1412.4503v3`).
9. Permanent impact need not be linear: making it path-dependent on cumulated volume preserves no-dynamic-arbitrage while admitting nonlinearity (`1305.0413v4`).
10. Measured "transient" decay can be an artefact of observing only part of the order flow, and the implied kernel is non-unique, so execution data cannot identify the true impact kernel (`2205.00494v2`).

## 1. The square-root law: measured exponent and prefactor

Across the three real-data measurements the exponent clusters tightly at one half. The TSE complete order-lifecycle survey covers 4 Jan 2012 to 2 Nov 2019, 2,299 stock-datapoints (942 liquid stocks before the 2015 arrowhead upgrade, 1,357 after; "liquid" = more than 10^5 metaorders), and reports <delta> = 0.489 +/- 0.0015 with a cross-sectional dispersion sigma_delta = 0.071 (`2411.13965v3`). The AAPL study uses 178 trading days of Nasdaq TotalView-ITCH (2 Dec 2024 to 19 Aug 2025, roughly 0.5 billion events at about 3.2x10^6 events/day), fixes delta at 1/2 on the first 20 days and reports a free fit of delta_hat = 0.50 [0.32, 0.66] (`2606.24019v1`). The Bitcoin study reconstructs more than one million metaorders from about 13M MtGox trades (Aug 2011 to Nov 2013) and finds the SRL holding over four decades in volume (`1412.4503v3`).

The prefactor is where the numbers diverge. TSE stock-level <c> = 0.842, trader-level <c^(i)> = 1.501 (`2411.13965v3`); AAPL c_raw = 0.69 [0.64, 0.75] and walk-forward c_raw = 0.75 [0.63, 1.01] (`2606.24019v1`); the ABM baseline c = 0.982 and cross-sectional <c> = 0.934 +/- 0.130 (`2607.04280v1`). The practical range is roughly 0.3 to 1.0.

Only one study fits the exponent per name on a single asset and gets a wide interval (`2606.24019v1`), which is the expected finite-sample behaviour: fitting a per-name delta and calling 0.5 a result is over-fitting noise, because the true cross-sectional dispersion is comparable to the estimation error.

## 2. Bias correction and what your own tape cannot see

Anonymous-feed reconstruction conditions the metaorder definition on what the public tape shows, which inflates the measured prefactor by about twofold via leakage into the aggressor-sign and run detection. The AAPL paper states this explicitly and adopts c_eff = c_raw / 2 = 0.34 while reporting c_raw = 0.69 (`2606.24019v1`). The correction follows the Naviglio et al. result the notes cite; the honest output is a bounded range (0.34 to 0.69), not a point estimate.

The same study mixes in the non-parametric evidence needed to trust an SRL fit: the size tail exponent beta = 1.54 +/- 0.15, a model comparison where sqrt beats linear by delta-AIC = 22 and beats log by +5, a child-count slope of 0.62 [0.52, 0.71], a sign autocorrelation gamma = 0.66 and a mid-price Hurst exponent of 0.49 (`2606.24019v1`). Model-comparison numbers matter: a "fit" that never competes sqrt against linear or log has not established the shape.

Two null tests give cheap falsification. Shuffling signs collapses the sign-alignment diagnostic from 86% to a null 50%, and scrambling chronology destroys the SRL in 0 of 80 calibrations (`2606.24019v1`). Any platform can run both against its own impact calibration before believing it.

## 3. Mechanical versus informational origins

The mechanism dispute is unresolved and the slice contains both camps. The "mechanical" reading treats incoming orders as a hot-potato game among market makers until a counterparty is found at a displacement satisfying Gamma (dp)^2/2 = q; on TSE the child-order impact is J(q,i) ~ sigma_D sqrt(q/VD) * (sqrt(i+i0) - sqrt(i0)) with i0 ~ 4 and a fitted exponent of 0.48, and completion impact recovers the SRL up to a factor running from 0.31 to 1 as the child count N goes from 2 to infinity (`2502.16246v2`). The same study finds slow market orders are 40-50% of volume and fast traders 50-60% of volume, contradicting the standard latent-order-book assumption that slow volume is orders of magnitude smaller, and it measures a post-fill liquidity-provider degradation with a refill-length power law psi(n) ~ n^-mu_p, mu_p in [1.4, 2.4] (`2502.16246v2`).

The "informational/equilibrium" reading derives the SRL from a fair-pricing condition and a martingale condition R_t^+/R_t = (1 - P_t)/P_t, where P_t is the probability the metaorder continues; with a Pareto size distribution impact grows roughly as the square root of size and average permanent impact relaxes to about two thirds of peak (`1102.5457v4`). The Bitcoin study splits the difference empirically: uninformed metaorders uncorrelated with residual flow have little permanent impact while the informed component drives it, yet the SRL survives even in a venue with almost no statistical arbitrage or market making (`1412.4503v3`).

These accounts conflict. `1102.5457v4` predicts the sqrt law from a power-law size distribution, but Bitcoin finds no clear power law in size or duration and still finds the sqrt law (`1412.4503v3`). The microstructure-exponent predictions GGPS (delta = beta - 1) and FGLW (delta = alpha - 1) show no correlation at all with measured delta on TSE (`2411.13965v3`) and are rejected in simulation, where LOB-walking (delta = 1/(1+gamma)) under-predicts and the other two over-predict (`2607.04280v1`).

The simulation settles only that the law needs both ingredients. Ablating splitting or replenishment breaks it (0.324 and 0.386 versus a 0.549 baseline), while price limits leave 0.529, low liquidity 0.543, momentum 0.549, uniform splitting 0.547 and front-loading 0.502 (`2607.04280v1`). The authors note the real TSE delta distribution is wider ([0.2, 0.7]) than their model's, and the result is a necessary-condition statement, not evidence that the mechanism is what operates live.

## 4. Latent liquidity, transient decay, and non-identifiability

Two papers attack the transient/permanent split from opposite directions. On the modelling side, a latent order book with a linear latent density near the efficient price generates square-root impact, and adding mean-reversion towards a reference price reduces long-run impact with an impact negatively related to reversion speed, which is described as another dimension of liquidity acting on medium-to-long horizons (`1802.06101v4`). A coupled lit/latent reaction-diffusion model with non-uniform event times and metaorder source terms produces transient impact proportional to sqrt(t) and completion impact proportional to sqrt(Q) at constant participation rate, and recovers the Lillo-Mike-Farmer sign-memory relation gamma = alpha - 1, framing the SRL as a physical-time viability statement and the sign law as an event-time memory statement (`2606.16269v2`).

The modelling side also flags an internal tension: a propagator decaying as (t)^-1/2 would imply strongly mean-reverting prices, at odds with diffusive prices, and `2606.16269v2` proposes subordination and state-dependent liquidity as the resolution while leaving the quantitative killing of the tension open. Separately, permanent impact can be made path-dependent on cumulated volume and still exclude dynamic arbitrage, so the classical constraint that permanent impact must be linear is an artefact of writing it as a function of volume alone (`1305.0413v4`).

The identification side is the sharper warning. Solving a permanent-impact Nash game and letting an external observer estimate impact by regressing price on past order flow yields an implied transient kernel even though the underlying impact is permanent; the first estimation approach gives a unique implied kernel, but the inverse-optimal-execution approach admits infinitely many solutions and can always be made to fit a linear kernel (`2205.00494v2`). The transient appearance is therefore an artefact of observing only the directional trader's flow, and a fitted decay constant is a model-family object, not a physical measurement. The Bitcoin reading that impact reverts by about one third (`1412.4503v3`) and the theoretical two-thirds permanent figure (`1102.5457v4`) may be consistent with the truth, but neither is identified from the fills alone.

## 5. Cross-impact and the multi-asset case

A single-asset impact model is wrong wherever instruments share a curve or a factor. In fixed income, extending one-factor short-rate and then HJM dynamics with instantaneous and transient price impact on each maturity makes cross-impact endogenous to the term structure: trading one maturity impacts other maturities on the same currency curve, and impact can be embedded in a modified risk-neutral measure via an "impacted" market price of risk, preserving no-arbitrage in both frameworks (`2011.10113v2`). Numerical examples show impact changes the shape of the yield curve (`2011.10113v2`).

The construction is purely theoretical, with no empirical estimate of cross-impact magnitude and no comparison to observed bond reactions, and it is motivated by reported sovereign-bond cross-impact rather than fitted to it (`2011.10113v2`). The take-away is structural: a multi-asset execution or hedging model should carry a cross-impact matrix and preserve no-arbitrage by construction rather than by post-hoc clipping. The same logic applies to related instruments on a platform (an index future and its constituents, an option and its hedge), but the slice provides no numbers for those cases.

## 6. Impact games, predatory trading, and transaction-cost thresholds

Two-agent games show that competition is not unambiguously cost-reducing. With transient exponentially decaying impact G(t - t_k), quadratic transaction costs and a general positive-definite decay kernel, small costs produce a hot-potato equilibrium in which the same position is sold back and forth, and there is a critical transaction-cost threshold theta* above which all oscillations vanish and strategies become buy-only or sell-only (`1305.4013v7`). Expected costs can be lower with transaction costs than without, and cost increases with trading frequency in the no-cost case but decreases with frequency once costs are high enough, because higher frequency creates more opportunities for predatory reaction by the competitor (`1305.4013v7`).

The motivating anecdote is the 6 May 2010 Flash Crash: between 2:45:13 and 2:45:27 HFTs traded over 27,000 E-Mini contracts, about 49% of total volume, while net-buying only about 200 (`1305.4013v7`). The model assumes a zero bid-ask spread, justified on the grounds that a hot-potato game cannot be profitable if the full spread is paid each trade, so the oscillation mechanism is a model artefact whose real-market prevalence is argued only from that anecdote (`1305.4013v7`). The permanent-impact companion game shows that observed transience and apparent arbitrage can co-exist with a fully permanent underlying impact (`2205.00494v2`). For a platform, the operational reading is that latency and frequency are non-monotone cost parameters and that fees, rebates or taxes can stabilise oscillatory predatory equilibria.

## 7. What execution data identifies and what it does not

Assembling the slice gives a short identifiability ledger. Identifiable from a sufficient sample: the aggregate exponent delta, which needs many names or a fixed-delta assumption to resolve (`2411.13965v3`, `2606.24019v1`); the prefactor c, but only up to a reconstruction bias of about twofold (`2606.24019v1`); the shape preference for sqrt over linear and log, via information criteria (`2606.24019v1`); and the sign-memory exponent gamma and price Hurst exponent, as feed-integrity diagnostics (`2606.24019v1`). The cross-sectional finite-sample errorbar is the honest uncertainty measure: the serial-correlation-aware <<sigma_delta>> = 0.063, versus a naive OLS figure of 0.014-0.022 that understates it (`2411.13965v3`).

Not identifiable: the transient-versus-permanent kernel, because a permanent game generates transient-looking measured impact and the inverse problem has infinitely many solutions (`2205.00494v2`); and the mechanism, since the size-distribution and microstructure-exponent derivations are rejected on real TSE data (`2411.13965v3`) and in simulation (`2607.04280v1`). What an operator can do instead is run the falsifiable checks: a fair-pricing test (executed VWAP against post-trade price) that flags alpha or timing skill when systematically violated (`1102.5457v4`), and the sign-shuffle and chronology-scramble null tests, which are valid way to stress-test an impact model without proprietary parent-order data (`2606.24019v1`, `2502.16246v2`).

## What this project can take from it

| Finding | Where it lands | What to do |
|---|---|---|
| SRL: delta ~ 0.5 holds | engine, backtest | Default the cost model shape to c (Q/VD)^0.5 |
| Prefactor c is 0.3-1.0 | python/nautilus_trader/backtest | Calibrate c per venue and report its interval |
| Tape reconstruction inflates c ~2x | python/nautilus_trader/analysis | De-bias tape-derived c and report both bounds |
| Per-name delta is noise | python/nautilus_trader/backtest | Report the cross-sectional spread, not one name's delta |
| Impact is schedule-invariant | docs/usermanauls/execution-algorithms | Key cost estimates off Q/VD, not the schedule |
| Splitting and replenishment are jointly needed | docs/concepts/order_book | Require both in any book simulation |
| Visible depth understates cost | docs/usermanauls/intraday-systematic | Size from Q/VD rather than displayed depth |
| Makers skew quotes after passive fills | docs/usermanauls/market-making | Model post-fill skew in quoting |
| Permanent impact is about two thirds of peak | docs/concepts/positions | Assume partial reversion in mark-to-market decay |
| Permanent impact can be nonlinear | docs/concepts/execution | Allow path-dependent permanent impact |
| Cross-impact is curve-endogenous | python/nautilus_trader/risk | Carry a cross-impact matrix in multi-asset risk |
| The decay kernel is not identifiable | python/nautilus_trader/backtest | Fit a kernel family and run the sign-shuffle, chronology-scramble and Hurst null tests |

## Caveats

The real-data evidence is three markets on three periods: TSE 2012-2019, a single AAPL name over 178 days in 2024-2025, and MtGox Bitcoin 2011-2013, so no claim here is a cross-asset law. The AAPL exponent is imposed rather than independently measured over 20 calibration days, and its heavy tail is not separable from a lognormal by the author's own admission. The Bitcoin venue has an unusual fee and spread environment (about 0.6% per transaction) with almost no market making, so it is a clean low-arbitrage control but not a template for today's venues. The mechanism result is a simulation calibrated to a single target exponent and explicitly produces a narrower delta distribution than the real data. The cross-impact construction is theoretical with no estimated magnitudes, and the impact-game oscillation mechanism is asserted to matter in real markets only from a Flash Crash anecdote. No study here provides an out-of-sample cross-market test of the prefactor, and the transient-versus-permanent decomposition is contested and, by the identification result, not recoverable from execution data at all. Costs beyond fees and quadratic temporary impact, and market-specific rules such as price limits and auction crosses, are dropped or assumed away across most of the slice.

## Papers read in depth

- `2411.13965v3` - Strict universality of the square-root law in price impact across stocks (2025). Complete TSE survey; the strongest exponent evidence and a rejection of the GGPS/FGLW derivations.
- `2606.24019v1` - Empirical Confirmation of the Square-Root Law of Market Impact in a U.S. Large-Cap Equity (2026). One name, careful bias correction and null tests; the clearest prefactor caveat.
- `2502.16246v2` - The "double" square-root law: Evidence for the mechanical origin of market impact (2025). Child-order decomposition and synthetic-id experiments; schedule-invariance claim.
- `2607.04280v1` - Order Splitting and Liquidity Replenishment Are Jointly Necessary for the Square-Root Law (2026). ABM ablation; necessary conditions, not live evidence.
- `2606.16269v2` - Revisiting Trade-sign Long-memory and Square-root Law price impact (2026). Reaction-diffusion reconciliation; no numbers, flags the propagator/diffusivity tension.
- `1802.06101v4` - Price Impact in a Latent Order Book (2018). Mean-reversion extension; adds reversion speed as a liquidity dimension.
- `2011.10113v2` - Price Impact on Term Structure (2020). Endogenous cross-impact on a yield curve; fully theoretical.
- `1305.4013v7` - A market impact game under transient price impact (2013). Hot-potato equilibrium and the transaction-cost threshold theta*.
- `2205.00494v2` - Transient impact from the Nash equilibrium of a permanent market impact game (2022). The identification result: transience can be an observation artefact.
- `1102.5457v4` - How efficiency shapes market impact (2011). Fair pricing, martingale condition, sqrt-from-Pareto and the two-thirds permanent default.
- `1412.4503v3` - A million metaorder analysis of market impact on the Bitcoin (2014). The low-arbitrage control; SRL without HFT or power-law sizes.
- `1305.0413v4` - Permanent market impact can be nonlinear (2013). Removes the linearity constraint via cumulated-volume dependence.

## Where to next in the corpus

- `0907.3282v8`, `1007.0199v5`, `1506.02789v2`, `1706.09224v2` - optimal-execution HJB viscosity variants; control theory of the same problem, no new impact empirics.
- `2412.07461v2` - passive-order impact as a point process; would extend the model beyond market orders.
- `2606.13419v1` - realtime impact detection; practical monitoring layer for live execution.
- `2609.03115v1`, `2601.23172v2` - recent mean-field and unified order-flow theory; alternative mechanism framing.
- `2310.06079v4` - discrete-time random walk anomalous diffusion in an order book; overlaps the reaction-diffusion treatment.
- `1212.4770v1` - autocorrelated order-flow theory; superseded by the reaction-diffusion propagator work.
- `1807.05917v2`, `1910.05056v2`, `1902.05418v4` - option and fixed-income hedging under impact; the options-side extension not covered here.
- `1807.03813v2`, `1509.08281v5` - Nash/risk-averse and high-frequency limits; deeper on the game line.
- `1702.05434v2` - short dimensional-analysis note; reinforces universality without new evidence.
- `2312.00904v2`, `2307.09392v2` - formal Kyle-game existence/stability; theory without numbers.
