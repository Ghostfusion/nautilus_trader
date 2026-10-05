# Optimal Execution and Liquidation: The Almgren-Chriss Tradeoff, VWAP Optimality, and the Gap to Real Fills

Date: 2026-10-05. Revision 1.

This brief covers the optimal-execution and liquidation slice of the corpus: 214 records spanning 2004-2026, of which 40 candidate papers were screened and 12 read in depth. It is written for an operator running a Python/Rust algorithmic trading platform across equities, futures, FX, crypto, options and prediction markets, and treats the scheduler as a shared component that research, backtesting, paper and live execution must all reproduce.

## What this covers

The slice runs from the Almgren-Chriss risk/cost tradeoff and its sinh closed form, through volume- and depth-dependent extensions, to VWAP optimality and guaranteed-VWAP pricing. It treats risk measures beyond variance - CVaR, p-variation and time-averaged VaR - and the multi-asset cross-impact case where cointegration changes the individual-asset curves. Reinforcement-learning execution is covered together with what it assumes about counterfactual feedback, and the slice closes on the gap between a schedule and real fills. The single most consequential thing in the slice is that VWAP/TWAP optimality is a theorem, not folklore: under a volume-dependent temporary impact and a risk-neutral objective the VWAP schedule is provably optimal (`1408.6118v4`), and adaptation adds nothing when volume is geometric Brownian (`1701.08972v2`).

## The corpus slice

| Metric | Value |
|---|---|
| Records matching the category | 214 |
| Years spanned | 2004-2026 |
| Records with a journal reference | 19 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

Selection rule: cover the classic risk/cost tradeoff and its closed forms, volume- and depth-dependent extensions, VWAP and guaranteed-VWAP, multi-asset cross-impact, RL execution and what it assumes, and one paper that calibrates impact to a real order book and measures schedule performance on it.

## The short answer

1. The Almgren-Chriss sinh schedule is the reference front-loaded risk/cost optimum; risk aversion steepens it, the volatility term drops out, and the risk-neutral limit collapses it to TWAP (`1204.2717v4`).
2. VWAP is provably optimal for a risk-neutral trader under volume-dependent temporary impact, with harmonic-mean volume weighting (not arithmetic) in the static class (`1408.6118v4`).
3. Adaptation pays only in mean-reverting regimes: under time-dependent Black-Scholes volume the adaptive optimum equals the expected-VWAP strategy and J_adap = J_stat (`1701.08972v2`).
4. The optimal dynamic policy beats the optimal static policy by 5-15% and the optimised constant-rate (VWAP) policy by 15-25% for moderate risk aversion, with the gap growing as risk aversion tends to the risk-neutral limit (`2201.11962v1`).
5. The optimal schedule depends only on the martingale property of the unaffected price, not on the law of the price process and not on its volatility (`1204.2717v4`).
6. Guaranteed VWAP is a priced derivative, not a schedule: a broker guaranteeing the benchmark must charge an indifference premium that depends on size, liquidity and risk aversion (`1306.2832v4`).
7. For a general jump Levy process the mean-variance and expected-exponential-utility optima do not coincide, so a single "risk aversion" control silently swaps objectives (`2002.03376v2`).
8. Multi-asset execution must be solved jointly: correlation and cointegration change the individual-asset curves, and a mean-reverting optimum can buy (`2103.13773v4`).
9. The fitted impact functional form can flip the sign of the optimal schedule, and back-loaded (concave) paths won in a calibrated crypto limit-order book (`2303.10043v1`).
10. RL execution reported +23 bps over TWAP and a book-depth VWAP at 7200 s on BTC-USD replays, but only inside a parametric impact simulator and never against a calibrated Almgren-Chriss schedule (`2511.07434v1`).

## 1. The Almgren-Chriss tradeoff and its closed forms

Almgren-Chriss is the universal reference for the whole slice. In `1204.2717v4` Theorem 1 (pp.5-7) gives the unique optimum for any right-continuous square-integrable martingale as x*_t = (X/sinh(nu T)) sinh(nu(T-t)) - (lambda/2nu) int_0^t S_s^0/(1+cosh(nu(T-s))) ds, with nu^2 = lambda_tilde*gamma/eta, where lambda_tilde is risk aversion, gamma permanent and eta temporary impact. The strategies are independent of the volatility sigma and depend only on the martingale property of the unaffected price, not on its law (pp.5-6); Corollary 1 (p.7) shows they minimise a robust cost functional, the worst case over the measure Q. For a general square-integrable semimartingale, Theorem 2 (pp.8-9) puts the drift only in integrated form, so a mis-specified drift averages out - in contrast to transient-impact models where the strategy depends on the derivative of the drift and small model errors are much more damaging (Lorenz and Schied 2012). Remark 1 (p.8) notes the optimum can go negative, forcing early termination and loss of optimality.

`2002.03376v2` pushes the model to Levy prices and CARA utility. The CARA expected-utility problem reduces to a deterministic optimisation over liquidation trajectories via a change of measure, and an explicit inverse G of x -> x^2 F'(x) gives the optimal speed in feedback form xi_t = G(Lambda_A(Y_t)/A) (Theorem 4.2, pp.13-14). Optimal liquidation speed is strictly increasing in both position size and risk aversion A (p.14). If the Levy process is a strict submartingale (drift > 0) the problem is ill-posed: no optimal admissible strategy exists and holding the shares is optimal (pp.11-12). For martingale Levy, finite liquidation time holds only when the impact exponent alpha exceeds 1 (Prop 4.3, p.14). The numerical examples use a variance-gamma model with theta = 0.002, nu = 0.02, sigma = 0.6, s = 100 and daily volume 2x10^6 shares, a power-law impact F(x) = eta x^alpha with alpha = 0.6 and eta = 4.7x10^-5 taken from Almgren et al. (2005), and a position of 2x10^5 shares, 10% of daily volume (pp.19-20). With A = 10^5 the time to liquidate 40% is about 0.00018 in volume-time units; with A = 10^4 the time to liquidate 90% is about 1.34x10^-14, which the authors call impractical (p.21). The 0.6 power-law can produce unrealistically fast VG liquidation, and matching the Brownian trajectory requires the impact to grow faster than any power (Section 6.2, p.22).

`1409.2618v2` provides the three myopic closed forms that a scheduler can switch between (Lemma 2, pp.9-10): linear inventory risk gives x_t = x(T-t)/T, which is TWAP/VWAP; quadratic risk c*x^2 gives the sinh schedule x_t = x sinh(sqrt(c)(T-t))/sinh(sqrt(c)T) of original AC; linear-in-value risk c*x gives a quadratic-in-time schedule that can require buying unless constrained.

## 2. Volume-dependent execution and VWAP optimality

`1408.6118v4` proves that VWAP is optimal, not a benchmark convention. With an explicit positive trading-volume process v_t and temporary impact kappa_tilde*xi/v, Theorem 1 (p.5) shows the exact VWAP strategy xi_t = v_t Phi/V_T is optimal among anticipating strategies - a trader who knows the future cumulative volume V_T, which is not implementable. Theorem 2 (p.5) shows the implementable version, the "expected VWAP" xi_t = u_t Phi/U_T with u_t = 1/E[1/v_t], the harmonic mean, is optimal among static (deterministic) strategies. Theorem 3 (p.5) shows that under geometric-Brownian volume the same expected-VWAP strategy is optimal in the adaptive class, so adaptation adds nothing there. Remark 3 (p.5) confirms constant volume reduces everything to TWAP xi = Phi/T, and Remark 4 (p.7) shows the mean-variance case solves a second-order ODE and recovers the AC cosh/sinh schedule under constant volume.

`1701.08972v2` attacks the adaptive problem directly. Theorem 1 (p.8) builds a verification theorem by penalisation, so penalised optimisers converge to an adaptive optimum. Theorem 2 (p.10) shows that for a time-dependent Black-Scholes volume model the adaptive optimum equals the expected-VWAP strategy and J_adap = J_stat: adaptation cannot improve expected cost. The Euler-Maruyama numerics (pp.13-14) use an OU log-volume with parameters kappa = 0.0001, kappa_tilde = 0.01, T = 1, X_0 = 10, sigma = 0.3, u_t = 100: at low mean-reversion rho = 0.3 the adaptive path resembles the exact anticipatory VWAP but costs about the same as the static one, while at high mean-reversion rho = 2 and 5 the adaptive strategy clearly lowers execution cost. The asymptotic expansion is explicitly formal, its accuracy only checked numerically.

`1306.2832v4` treats general permanent impact and confirms the shape result: Theorem 3.1 (p.9) gives a unique deterministic minimiser q* satisfying q*(t) <= q_0 (the curve never buys back in the deterministic case), and without permanent market impact the optimal trading curve has the same shape as the market volume curve (Section 4, p.12). Permanent impact deforms it.

## 3. Guaranteed VWAP, Target-Close and participation constraints

`1306.2832v4` separates an agency VWAP, which is benchmarked, from a guaranteed VWAP, in which the broker guarantees the benchmark and charges a premium pi(q_0); the paper prices that premium by indifference, using CARA utility E[-exp(-gamma(X_T - q_0 VWAP_T))]. This makes guaranteed VWAP a contract whose premium depends on order size, liquidity and risk aversion, not a free schedule label.

`1205.3482v6` supplies the discrete machinery for the two most common benchmark types. It extends AC to a general p-variation risk measure and a nonlinear impact h(v) = lambda_n v_n/V_n, gives recursive first-order conditions for Target Close (TC) and Implementation Shortfall (IS), and shows IS is the time-reversed mirror of TC. An explicit participation (PVol) constraint is added by a TC/PVol switching algorithm, because TC and PVol are mutually exclusive and Lagrange multipliers do not work (p.12). The real-data example uses Air Liquide (AIRP.PA), an order of 150,000 shares, minimum slice 500, maximum participation 20%, with volatility and volume curves and impact parameters from Credit Agricole Cheuvreux (p.14); the algorithm starts at pillar n_0 = 34 and switches to PVol at pillar n_1 = 94, about 103 pillars in total. TC weights forward volatility sigma_{n+1} and IS weights spot volatility sigma_n when volatility is non-constant, and the two coincide under constant volatility (p.12). The link p = 1/H ties a self-similar process with Hurst exponent H to the p-variation measure, and increasing p makes the TC algorithm start later and execute faster (p.16, p21-p.22). No P&L numbers are printed for the real-data example.

## 4. Risk measures beyond variance

`1204.2717v4` already replaces the variance penalty with the Gatheral-Schied time-averaged Value-at-Risk cost functional E[C(x) + lambda_tilde x_t (S_t^0 + gamma x_t)].

`2201.11962v1` goes further and makes CVaR the objective. It dualises CVaR into a scaled CVaR with a quantile state Q_t, casting a continuous-time zero-sum game between the trader and an adversary whose separable HJB is Emden-Fowler type and has a closed-form value V(x,q) (eq 11, p.11). The optimal dynamic policy is liquidate-only (p.4) and exhibits "aggressiveness-in-the-money": trade faster when the price moves favourably, slower when it moves unfavourably (pp.4-5). For moderate risk aversion the dynamic policy beats the optimal static policy by 5-15% and the optimised constant-rate (VWAP) policy by 15-25%, with the relative improvement depending only on the risk-aversion level q and growing without bound as q -> 1 (p.5). It also improves worst-tail and median outcomes versus deterministic policies (p.6). The paper is a preliminary version, ignores permanent impact, and admits the candidate policies are shown feasible only via a converging sequence (p.6).

`1205.3482v6` offers p-variation as a practitioner knob: one parameter p replaces variance and lets a desk translate an observable (start time, maximum participation) into a risk attitude without refitting a full model.

## 5. Multi-asset and order-flow-aware execution

`2103.13773v4` is the portfolio-level reference. It models d assets with multivariate OU fundamental prices dS = R(S_bar - S)dt + V dW and market prices dS~ = dS + K v dt, solves the HJB into a system of ODEs including a Matrix Riccati ODE, and proves existence and uniqueness under exponential utility via a priori estimates and a verification theorem, since the classical theorems do not apply. The OU (ACOU) optimal liquidation is significantly slower than the classical Brownian (AC) schedule for the same order, because a mean-reverting trader perceives less timing risk (p.15). When the mean-reversion speed R is large, for example 10/day, the optimum approximates a VWAP/TWAP schedule plus a mean-reverting statistical-arbitrage overlay: selling faster above S_bar and slower, even buying, below it (pp.17-18), with the terminal penalty dominating near T. The illustrative real data is CDU1 CAD futures on 11-13 August 2021, q_0 = 2250 contracts (about 5% of average daily volume), with eta = 5x10^-3 $/day, S_bar = $79,887 and R = 5.1/day; the stat-arb study over 1500 simulated paths earns an average final PnL of $107,698 with standard deviation $57,791 (p.19). Cross-asset correlation and cointegration are what change the individual-asset curves, and single-asset models do not balance cost and risk at portfolio level (pp.2-3).

`1409.2618v2` makes order-flow imbalance a first-class state: a mean-reverting OU factor Y_t with information leakage, adverse-selection cost theta*Y_t^2 and an endogenous horizon T_0. Endogenising the horizon creates dynamic price-adaptive strategies even when the asset value is a martingale, unlike classical price-impact-only models (pp.2-3), and a receding-horizon (recompute-and-roll) approximation is reported "very accurate" against the indefinite-horizon solution (p.9).

## 6. RL execution and what it assumes

`2511.07434v1` is the strongest RL evidence in the slice. A PPO agent trades BTC-USD on 1-second snapshots of the top 20 levels, trained on January 2020 (31 days) and tested on February 2020 (28 days) under a strict temporal split with no shuffling. The environment is a replay-with-impact simulator that injects transient impact with exponential resilience, partial fills, maker/taker fees and a fixed latency so that decisions affect prices one tick later, with a sell-only inventory constraint. Against TWAP and a VWAP-like baseline that allocates by opposite-side displayed depth, the agent's edge grows with horizon (Table 1, p.1): 1,800 s +2.25 to +2.68 bps (p_adj < 0.01); 3,600 s +7.59 to +7.70 bps (p about 3-5 x 10^-6); 7,200 s +22.96 to +23.02 bps (p about 1 x 10^-8), with all bootstrap 95% confidence intervals positive. The evaluation protocol is itself the contribution: 10 independent intraday start times per test day aggregated into one daily score to avoid pseudo-replication, one-sided Wilcoxon signed-rank tests on daily differences, Benjamini-Hochberg FDR correction and bootstrap confidence intervals. The paper does not compare against a calibrated Almgren-Chriss or percentage-of-volume schedule, explicitly leaving that to future work.

`2307.11685v1` explains why RL execution overfits. Its offline-RL-with-dynamic-context bound, Theorem 1 (p.3), states that any algorithm needs at least about |C| log(|C|/delta) / ((1-gamma)^3 eps^2) context samples for a small generalisation gap, so sample cost scales with the context-space size |C| - the root of overfitting in execution. Theorem 2 (p.4) shows a context decoder cuts the requirement to the latent space |X|, much smaller than |C|. On a simplified task (Table 1, p.6) the base DDPG agent reaches train reward 4.0340 but eval reward 1.4083 at 1k samples (a gap of 2.63) and only 1.8291 at 100k, while CASH reaches eval 1.9557 at 1k and 1.8028 at 100k with much smaller gaps, and a capacity-bottleneck baseline collapses to about 0.0. The overfitting is visible as an agent that "liquidates most inventory on the highest price" seen in training (Figure 2, p.3).

## 7. From a schedule to real fills

`2303.10043v1` is the closest thing in the slice to a schedule-versus-real-book measurement. It calibrates temporary and permanent impact from BNB on Binance, using a limit order book on 6 February 2022 at 5-second frequency, 100 ticks of depth and 1440 observations (2 hours), with momentary spread mu = 0.100069 and volatility sigma = 0.009388 (p.9). Impact functional form depends on order size relative to book depth: small size fits a power with exponent greater than 1, large size a power with exponent in (0,1), and intermediate size a near-linear exponent of 1 (p.3, p.8); power fits better than linear in every scenario (p.9). It then simulates liquidation of 4000 BNB over 1800 s at 5-second intervals assuming a resilient book (p.14). Cumulative revenue as a percentage of a naive one-shot strategy is 1.1473% for the best policy (ATPPP) and 1.1454% (ATLPP), against a worst 0.6268% (OTLPL) and 0.7157% (OTPPL) (Table 1, p.14). Performance is driven by policy shape rather than functional form: concave optimal inventory paths (start slowly, then accelerate; exponent d_2 > 1 in q(t) = Q*(t/T)^{d_2}) earn the most, convex paths earn the least, and the ranking is stable as inventory varies 500-4000 (pp.14-16). The optimal policies were verified price-independent in that setting (p.10).

The gap the other papers leave open is exactly this one. The closed forms above generally ignore fees, spread, latency and fill risk; only `2511.07434v1` and `2303.10043v1` include them, and only in simulation. Almgren et al. (2005) impact parameters, used by `2002.03376v2`, are in volume time (footnote p.20), so translating them to clock time is unsafe. `2103.13773v4` uses an illustrative single path and simulated PnL; `2307.11685v1` and `2511.07434v1` assume a parametric impact process and a pristine footprint with no interacting agents.

## What this project can take from it

| Finding | Where it lands | What to do |
|---|---|---|
| Closed-form sinh/AC schedule with nu^2 = lambda_tilde*gamma/eta, volatility-free | python/nautilus_trader/execution, backtest | Ship one AC schedule parameterised by risk aversion; do not re-fit per venue |
| VWAP is provably optimal under volume-dependent impact; harmonic-mean static weighting | data layer, execution layer | Size VWAP slices by harmonic-mean volume curve, not arithmetic mean |
| Adaptation adds nothing under GBM volume; pays under mean-reverting liquidity | python/nautilus_trader/execution | Gate adaptive overlays on a mean-reversion test on the volume process |
| CVaR objective, aggressiveness-in-the-money, 5-15% / 15-25% gains | risk layer, execution layer | Expose CVaR as an alternative objective and a single "faster when in the money" rule |
| PVol and minimum-slice constraints are mutually exclusive with TC; need switching | python/nautilus_trader/execution | Implement TC/PVol switching logic, not a Lagrange multiplier |
| Guaranteed VWAP is an indifference-priced contract | risk layer, reports | Price any guaranteed benchmark off size, liquidity and risk aversion |
| Multi-asset execution needs a joint Riccati solve and signed inventory | python/nautilus_trader/portfolio | Solve correlated assets jointly; allow a buying overlay when mean-reversion is strong |
| Order-flow imbalance is a tradeable microstructure state | python/nautilus_trader/execution | Read an imbalance proxy and lean with flow; use receding-horizon roll |
| Impact form flips schedule sign; back-loaded wins in a crypto book | backtest, python/nautilus_trader/analysis | Stress-test schedules across multiple impact calibrations |
| RL evaluation needs strict temporal split, per-day aggregation, paired tests, FDR | python/nautilus_trader/analysis | Adopt the RL-Exec protocol; report train-eval gap, not just mean cost |
| Closed forms ignore fees, spread, latency and fill risk | live, python/nautilus_trader/risk | Add a TCA layer over simulated schedules before sizing live |

## Caveats

Most of the slice is theory with no market data: `1204.2717v4`, `1408.6118v4`, `1409.2618v2`, `1701.08972v2`, `1306.2832v4` and `2201.11962v1` contain no empirical validation at all, so their closed forms establish internal consistency rather than realised P&L. Only `1205.3482v6`, `2103.13773v4` and `2303.10043v1` use real market data, and `1205.3482v6` is a single Air Liquide example with historical volume and volatility curves, `2103.13773v4` uses a single illustrative liquidation path with simulated stat-arb PnL, while `2303.10043v1` is one asset on one day with a resilient, fully replenished book. `2511.07434v1` measures a real out-of-sample month but inside a parametric impact simulator with no interacting agents, so its edge over TWAP and book-VWAP is a simulated footprint, not live fills. `2303.10043v1` found that its optimal policy was price-independent under a driftless resilient book, yet `1409.2618v2`, `2201.11962v1` and `2103.13773v4` all make the schedule explicitly price-adaptive; the disagreement is not resolved by the evidence here. `2201.11962v1` is preliminary and cannot directly show its candidate policies are admissible, and it ignores permanent impact. The reported dynamic-versus-static gains of 5-15% and 15-25% in `2201.11962v1` sit directly against `1701.08972v2`'s proof that J_adap = J_stat under time-dependent Black-Scholes volume; the reading offered here is that the benefit is regime-dependent, but no paper in the slice tests that resolution on data. The sign of the optimal schedule is itself contested: classic AC and `2201.11962v1` front-load, while `2303.10043v1` finds back-loaded paths winning, and its own results reverse the schedule when the permanent-impact form changes from linear to power. Nothing in the slice establishes costs, latency or queue dynamics against a calibrated live venue.

## Papers read in depth

- `2002.03376v2` - Optimal liquidation trajectories for the Almgren-Chriss model with Levy processes (2020). Closed-form CARA/Levy feedback speed; mean-variance and expected-utility objectives diverge under jumps.
- `1408.6118v4` - VWAP Execution as an Optimal Strategy (2014/2017). Proves VWAP optimality and the harmonic-mean static weighting; adaptation is worthless under GBM volume.
- `1204.2717v4` - Robust Strategies for Optimal Order Execution in the Almgren-Chriss Framework (2012/2013). The reference closed-form sinh optimum; independent of volatility and of the price law.
- `1409.2618v2` - Optimal Execution with Dynamic Order Flow Imbalance (2014). Order flow as state, endogenous horizon, and three myopic closed forms a scheduler can expose.
- `2201.11962v1` - Risk-Sensitive Optimal Execution via a Conditional Value-at-Risk Objective (2022). CVaR objective, liquidate-only, aggressiveness-in-the-money; preliminary and unpriced for permanent impact.
- `1306.2832v4` - VWAP execution and guaranteed VWAP (2013/2014). Guaranteed VWAP as an indifference-priced contract and the volume-shape result.
- `1701.08972v2` - An Optimal Execution Problem in the Volume-Dependent Almgren-Chriss Model (2017). Adaptive optimum equals expected VWAP under Black-Scholes volume; gains only under mean-reverting volume.
- `1205.3482v6` - Optimal starting times, stopping times and risk measures for algorithmic trading (2012/2014). Target-Close/IS machinery, PVol switching and p-variation as a single risk knob.
- `2103.13773v4` - Multi-asset optimal execution and statistical arbitrage under Ornstein-Uhlenbeck dynamics (2021/2022). Joint Riccati solve; cointegration turns the schedule into a stat-arb overlay.
- `2303.10043v1` - Optimal liquidation with temporary and permanent price impact, an application to cryptocurrencies (2023). Real LOB calibration showing schedule sign flips with the fitted impact form.
- `2511.07434v1` - RL-Exec: Impact-Aware Reinforcement Learning for Opportunistic Optimal Liquidation (2025). Credible out-of-sample RL protocol; small, simulator-bound edge over TWAP.
- `2307.11685v1` - Towards Generalizable Reinforcement Learning for Trade Execution (2023). Sample-complexity bound showing why RL execution memorises context.

## Where to next in the corpus

- `1909.10464v2` - pathwise-good execution under linear temporary impact; conceptual overlap with the robustness result in `1204.2717v4`.
- `1901.02327v2` - VWAP under transient impact; would test whether the VWAP optimality survives transient dynamics.
- `2006.11426v2` - another explicit AC/GBM closed form; redundant with `2002.03376v2` but a useful cross-check.
- `1709.05837v1` - randomly-terminated liquidation horizon; specialist HJB/viscosity result for uncertain end times.
- `2504.06717v2` - execution and market making as a stochastic game; connects scheduling to quoting.
- `1204.0148v6` - limit-order posting model; the tactical placement layer the strategic schedules declare out of scope.
- `2607.04280v1` - square-root impact law mechanisms; the impact law the execution schedules take as input.
- `2307.10649v1` - hierarchical RL for VWAP tracking; an RL alternative to the book-depth comparator in `2511.07434v1`.
- `1607.04553v4` - multi-venue liquidation; the venue-selection dimension the single-venue schedules ignore.
- `2507.06345v2` - RL market-plus-limit order allocation; the limit-order dimension absent from the liquidation papers.
