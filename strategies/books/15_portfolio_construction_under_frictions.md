# Portfolio Construction Under Frictions: Execution-Aware Allocation, Cross-Impact, and Capacity

Date: 2026-10-05. Revision 1.

This brief covers the portfolio-construction slice: 331 records matching q-fin.PM / q-fin.TR construction topics, spanning 1998-2026, 45 of them carrying a
journal reference. From 39 candidate papers screened, 12 were read in full or by targeted sampling, including the 5 mandated gap papers (1610.07694v3,
1811.05524v1, 1510.09110v1, 1701.05016v1, 0912.4723v3).

## What this covers

The slice asks what changes once portfolio weights are not freely attainable and the trade that reaches them is paid for. Five strands run through it:
liquidity- and impact-aware dynamic allocation with a capacity ceiling; cross-impact, which couples execution across assets and breaks separability;
time-consistent dynamic mean-variance liquidation with state- and signal-dependent schedules; well-posedness conditions (nonnegative-definite propagators)
that exclude manipulation and arbitrage-like round trips; and the myopic-versus-RL dispute over which policy class survives costs. The most consequential
claim is that ignoring execution and liquidity cost does not merely round the answer - it can invert it: a liquidity-blind single-stock book loses hundreds
of basis points a year at scale while optimal exposure collapses toward zero.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category | 331 |
| Years spanned | 1998-2026 |
| Records with a journal reference | 45 |
| Candidate papers screened | 39 |
| Papers read in depth | 12 |

Selection kept only papers making a quantitative claim about execution-aware or liquidity-aware portfolio choice, cross-sectional or cross-impact
construction, dynamic mean-variance liquidation, cost-aware implementable weights, or documented capacity limits; pure price forecasting, single-asset
hedging, VIX manipulation and pure AMM mechanism design were dropped.

## The short answer

1. Liquidity-blind backtests are systematically optimistic: at (sigma=12.5, Vol=12m, W0=1.0b) a single-stock SPY book shows CER 7.3 bp liquidity-aware versus
-239.8 bp blind `1610.07694v3` (p.22).
2. Optimal exposure can fall to cash as size grows: initial stock weight drops from 0.52 at W0=100m to 0.32 at W0=1b, and to 0 under severe illiquidity for
W0>=700m `1610.07694v3` (p.24).
3. Cross-impact couples portfolio execution: with index-fund share of volume theta=0.21, coupled liquidation saves up to 6.2% versus a separable VWAP-like
schedule `1811.05524v1` (p.17-18, p.22).
4. The separable Almgren-Chriss / VWAP vector is a special case, valid only when liquidity provision is single-stock and time-invariant `1811.05524v1` (p.3)
or cross terms vanish `2403.10273v2`.
5. Time-consistent mean-variance liquidation yields coth-type schedules; a random pricing signal makes the control adapted and regime/state-dependent
`1510.09110v1` (p.6, p.9-10).
6. Well-posedness has a testable condition: price manipulation is excluded if and only if the cross-impact propagator is nonnegative definite `2403.10273v2`
(Thm 2.15, p.11).
7. In action-independent markets myopic convex rebalancing can dominate RL, with the probability of positive RL backtest bias around 0.6 to 0.8
`2509.12764v1` (pp.27-28).
8. Real traders behave as cost-aware mean-variance optimizers: mean turnover scales with account value at exponent 0.54-0.89 across client classes
`0912.4723v3` (Table 5, p.9).
9. Flow shocks are absorbed by cash, not alpha: every INR 100 of inflows raises contemporaneous cash by INR 32 `2510.02741v1` (pp.15-16).
10. The deployable route to execution-aware risk management is a quadratic program, not an HJB solve `2309.15767v1` (p.5).

## 1. Liquidity-aware dynamic allocation and its capacity ceiling

The reference construction is a simulation-and-regression scheme for one cash-plus-stock portfolio with CRRA/CARA utility, monthly rebalancing over horizons
up to 15 years, endogenous state variables (wealth and prices shifted by permanent impact) and switching costs. Liquidity follows Almgren et al. (2005) power
laws: Theta=988m shares, trade duration delta=5 min, daily volatility in [2,13]% and daily volume in [10m,120m] shares; control randomization is replaced by
control-space discretization over 100,000 Monte Carlo paths `1610.07694v3` (pp.11-14).

The size-dependent penalty is the point. With gamma=5 the liquidity-aware certainty-equivalent return is 57.8 bp versus 56.6 bp blind at low impact
(sigma=2.5, Vol=120m, W0=0.1b), but 7.3 bp versus -239.8 bp under severe illiquidity (sigma=12.5, Vol=12m, W0=1.0b), and the gamma=10 panel shows blind CER
down to about -150 bp `1610.07694v3` (Table 4, p.22). The optimal initial weight falls from 0.52 to 0.32 with size and reaches zero for W0>=700m under severe
illiquidity, so a cost-aware optimizer must be able to answer "do not trade" and a capacity layer must let exposure approach zero rather than lever up
`1610.07694v3` (Table 7, p.24). Convergence must also be reported: at large liquidity effects the control-randomization benchmark gives CER 12.8 bp at M=10^3
paths versus 19.0 bp (I=0) and 23.5 bp (I=1) for the proposed method, the gap narrowing by M=10^6 (21.6 / 24.0 / 24.1 bp) `1610.07694v3` (Table 3, p.20).

## 2. Cross-impact and non-separable multi-asset execution

Once a fraction of liquidity providers trade portfolios rather than single names, single-name impact is not the whole story. A stylized clearing model with
index-fund portfolio liquidity gives linear cross-asset impact whose coefficient matrix is the inverse of a diagonal (single-stock liquidity) plus a low-rank
(portfolio liquidity) matrix. Intraday volume-surprise correlations across S&P 500 constituents are positive all day and roughly double over the last 1-2
hours; the estimated index-fund share of traded volume is theta=0.21, with portfolio investors exceeding single-stock investors in the last hour
`1811.05524v1` (abstract p.1, p.17-18).

The optimal schedule is therefore coupled, saving up to 6.2% over separable execution in the calibrated worst case; severability holds only without portfolio
liquidity or with constant intraday intensity, and the benefit sign turns on eta_1 versus theta/(1-theta) ~ 0.27 `1811.05524v1` (p.3, p.22). Estimation must
be low-rank (K+1 parameters) because pairwise estimation is intractable `1811.05524v1` (p.17). The propagator view generalizes this to arbitrary decay
kernels: maximizing terminal wealth with a Markowitz risk penalty under a matrix-valued Volterra propagator G(t,s) plus temporary impact Lambda reduces to a
coupled system of stochastic Fredholm equations of the second kind, solved in closed form via operator resolvents, nesting Garleanu-Pedersen (exponential)
and Alfonsi et al. (discrete) and extending them to power-law propagators with stochastic signals `2403.10273v2` (Thm 2.8, p.3-10). In the numerics (T=10,
X0=(10,0), Lambda=0.03*I), positive cross-impact induces a round trip in Asset 2 (short, long, liquidate) and more aggressive trading; with signals of
differing alpha decay the strategy may short the fast-decaying asset to exploit cross-impact on the slow-decaying one, and a three-asset chain propagates the
effect with no direct first-to-third coupling `2403.10273v2` (pp.12-17).

## 3. Time-consistent mean-variance liquidation and state-dependent schedules

Agency execution under dynamic mean-variance can be made time-consistent rather than pre-committed, via an HJB with a reconsidered mean-variance objective
and the law of total variance. In the base model the optimal speed is upsilon*(t,X) = X(t)*(mu*sigma^2/eta)^(1/2)*coth((mu*sigma^2/eta)^(1/2)*(T-t)),
identical to Almgren (2012) when the control is deterministic - the same coth shapes Almgren-Chriss implementations already use, parameterized by (mu, sigma,
eta) `1510.09110v1` (p.6).

With a random pricing signal beta = S - alpha the optimal speed becomes upsilon* = (1/(2*eta))*(2*D(t)*X(t) + F(t)*beta(t)), making the control adapted and
signal-conditioned, while stochastic liquidity and volatility give a generalized HJB (Prop. 3, p.11) with an explicit solution under sigma(t) = X(t)/(T-t)
`1510.09110v1` (pp.9-12). A pre-commitment-only planner will mis-state early-exit decisions, and the execution model must expose eta and sigma as state
variables to the scheduler. A discrete-time variant solves only the horizon, not the schedule shape: with N risky assets liquidated at equal intervals and
quantities, N-dimensional Brownian prices, matrix permanent impact affecting all assets and diagonal temporary impact, the VaR objective E[C] +
Z_p*sqrt(Var[C]) yields T* = ( 2 x0' eta x0 / (3 Z_p x0' sigma sigma' x0) )^{1/3}, depending only on initial position, temporary impact and volatility - not
on permanent impact `2103.15400v1` (Thm 3, p.5-6). Simulation (2 assets, 10m and 8m shares, $50/$100, 1000 runs per case) gives a minimum cost rate of 8.77%
at eta=3e-8/5e-8, up to 13.79% at the top of the range, and T* falling from 3.14 to 2.40 with cost from 8.75% to 6.56% as volatility doubles `2103.15400v1`
(pp.7-10).

## 4. Well-posedness: nonnegative-definite propagators exclude manipulation

A construction model can be formally well-posed and still admit pathological strategies. The propagator framework gives a clean test: price manipulation is
excluded if and only if G is nonnegative definite, with a sufficient condition that the kernel H be nonincreasing, convex, nonnegative and symmetric; a
factorized G(t)=C*phi(t) with C positive semidefinite and phi nonnegative, nonincreasing and convex is admissible `2403.10273v2` (Thm 2.15, p.11; Cor 2.18,
p.12). Without this check the optimizer produces arbitrage-like round trips that look like alpha but are model artifacts, so an engine should validate the
impact/kernel matrix for nonnegative definiteness.

Well-posedness appears elsewhere as existence and stability of the value function. Dark-pool liquidation on a primary venue plus an impact-free venue with
Poisson fills requires bounds on matrix Riccati solutions to prove existence of the limiting value function and a verification theorem `1201.6130v2`
(abstract p.1), and friction-aware RL policies require existence of an optimal stationary policy plus monotone improvement under a KL trust region
`2510.02986v1` (p.6). Different technical conditions, same message: establish boundedness and convexity before trusting the output. Venue choice adds a
dimension - single-asset, an investor slowly trades out at the primary venue and places the remainder in the dark pool, but multi-asset this is generally not
optimal and it can be optimal to oversize dark-pool orders to turn a poorly balanced portfolio into one bearing less risk, so a venue-aware layer decides
per-portfolio rather than per-order `1201.6130v2` (abstract p.1).

## 5. Cost-aware policies: myopic versus RL

Two papers take opposite positions and do not resolve the disagreement. A friction-aware RL policy (PPO/TRPO variant) embeds proportional plus quadratic
transient impact in the reward, applies a trust region in trade space (penalizing change in inventory flow rather than logits), and conditions policy and
value on four regimes (LL/LH/HL/HH) crossed with five cost levels {0,5,10,25,50} bps over three seeds. It claims top average Sharpe across all 20 scenarios
with the flattest cost-performance slope versus vanilla PPO, mean-variance with/without caps and risk parity, and supplies a long-run turnover bound TO(pi)
<= (1-gamma)*r_bar/(gamma*lambda_tc*kappa) (Prop. 1, p.6), an inaction band tau ~ kappa_1/(kappa_2 + H) (Prop. 2, p.6), and a turnover-budget map lambda_tc
>= (1-gamma)*r_bar/(gamma*kappa*TO_max) `2510.02986v1` (abstract p.1; Section 5, p.9-13; Section 6.2, p.12). Its numeric Sharpe levels are figure-only, only
qualitative rankings and sign-test significance (Romano-Wolf stepdown, White Reality Check / SPA) are printed, and regimes are treated as observed rather
than estimated `2510.02986v1` (p.12).

Against this, a theoretical treatment decomposes RL backtest error into phantom profit, self-bias and solution bias and argues myopic optimization - a
sequence of one-period convex programs - dominates RL in action-independent markets: higher mean PnL, lower variance, lighter CVaR, lower costs, less model
risk. The probability that solution bias is positive is about 0.6 to 0.8 under typical settings (10^2-10^3 iterations, per-step SNR 0.03-0.10), with Table
2.1 mapping Zeff to P[bias>0] from 0.62 at 0.3 to 0.95 at 1.6 `2509.12764v1` (abstract p.1; p.27-28). RL adds value only under control-affects-dynamics,
where the CAD premium is positive to first order but vanishes in the nonatomic limit and needs a genuine counterparty behavioral shift `2509.12764v1` (Cor.
2.2-2.3, p.29-30). Both sides agree on the operational core: cost belongs inside the reward (ex-post adjustment is worse), and a flow-space penalty is a
portable low-turnover stabiliser.

## 6. Evidence from real traders and funds

The empirical anchor is a Swissquote client database: 19 million electronic orders from 120,000 professional and non-professional traders, January 2003 -
March 2009, with 65% of orders cancelled or expired, 30% filled and leveraged products excluded. Mean turnover follows a double log-linear law versus account
value with exponents 0.84 (low wealth) and 0.54 (high wealth) for individuals, 0.81/0.50 for companies and 0.89/0.52 for asset managers `0912.4723v3` (Table
5, p.9). Account values are log-normal, with a retail tail Pareto exponent of 2.30 on the first trade rising to 2.39 on the last `0912.4723v3` (Table 1-2,
p.5-6); the number of assets scales as N = (x*P_v)^0.5 in the high-diversification limit `0912.4723v3` (eq 18-19, p.16-17), and a broker fee curve F(x) =
min(C*x^gamma, F_max) with gamma = 0.63 [0.50,0.74] reproduces the observed laws `0912.4723v3` (p.13). Transaction costs are a first-order driver of the
weights real traders hold, not a rounding correction.

Fund data points the same way from the other side. For Indian open-ended equity funds, 2011-2023, every INR 100 of inflows raises contemporaneous cash by INR
32 (beta0=0.32, t=8.42); flows are negatively associated with portfolio illiquidity up to one quarter (beta1=-0.007, t=2.18) then positive (beta4=+0.007,
t=3.54), and one standard deviation higher portfolio illiquidity raises the cash response by 29% `2510.02741v1` (Table 2, p.15-16). The most liquidity-active
quintile outperforms the least by 0.53*** gross (NW t=2.92, FF4 alphas up to 0.55) and 0.54*** net (t=3.13), a 4.2%/yr (CAPM alpha) to 6.6%/yr (raw) spread
that survives fees `2510.02741v1` (Table 3, p.19-20); liquidity activeness is thus a usable construction signal and the cash buffer a separate, sizeable
bucket. The research-to-deployment bridge is a practitioner QP route - net notionals, invariant risk factors, unconstrained hedge x = -(H'CH)^{-1}H'C r, and
a symmetric-cost term lambda_c*c'|x| recast via an auxiliary v >= |x| and a regularizer lambda_0*(v'v - x'x) - where the forecasts of r, H and C are the real
risk, and asymmetric costs, discrete lots and impact can destroy convexity `2309.15767v1` (pp.2-6).

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| Blind CER -240 bp at size | docs/concepts/backtesting | Run aware and blind backtests |
| Weight hits zero as AUM grows | docs/concepts/portfolio | Allow a full cash allocation |
| Cross-impact breaks separability | docs/usermanauls/execution-algorithms | Couple liquidation |
| Separability needs no portfolio liquidity | docs/concepts/execution | Document valid cases |
| Non-psd impact admits manipulation | docs/concepts/optimization | Validate the kernel |
| Schedules need regime state | python/nautilus_trader/execution | Expose state to scheduler |
| Horizon scales as (impact/vol)^(1/3) | python/nautilus_trader/analysis | Sanity-check scheduling |
| Put cost inside the reward | docs/usermanauls/ai-training | Report the cost slope |
| Myopic convex is a strong baseline | docs/concepts/optimization | Benchmark RL against it |
| Turnover budget maps to a penalty | python/nautilus_trader/risk | Wire it into governance |
| Cash absorbs flow shocks | docs/concepts/portfolio | Size a separate cash bucket |
| QP is the deployable route | docs/concepts/optimization | Deliver hedging as a QP |

## Caveats

The evidence does not establish that these constructions work on real data. Nearly all construction papers in the slice are simulation or theory:
1610.07694v3 calibrates a VAR in-sample and reports simulated certainty-equivalents; 1510.09110v1 has no numerical calibration; 2403.10273v2 uses
illustrative propagator parameters with no market fit; and 2103.15400v1 and 1201.6130v2 are closed-form with no realized costs. The headline cost reductions
are model-dependent: the 6.2% cross-impact saving in 1811.05524v1 is a worst-case bound for plausible parameters, not a measured saving, and its model is not
calibrated to transaction data. Cross-impact magnitude is itself contested - 2403.10273v2 cites Capponi-Cont and Le Coz et al. as finding weak temporary
cross-impact, reconcilable with 1811.05524v1 only if the disputed term is the transient propagator rather than instantaneous impact. The myopic-versus-RL
dispute is unresolved because FR-LUX's Sharpe levels are figure-only while 2509.12764v1 supplies no experiment of its own. 1701.05016v1 models no
transaction, borrow or slippage costs, so its Sharpe spreads are gross construction results over about 2.4 years of market out-of-sample with in-sample
thresholds. 2510.02741v1 is not causal, its net results are fund-level returns net of expense fees rather than trading impact, and it covers a single market
and period, so it does not quantify capacity in currency terms. Capacity is explicit in only three of the twelve papers (1610.07694v3, 2510.02986v1,
2510.02741v1), and only 2510.02986v1 and 2510.02741v1 apply formal multiplicity control.

## Papers read in depth

- `1610.07694v3` - Dynamic portfolio optimization with liquidity cost and market impact: a simulation-and-regression approach (2019). The core
execution-aware allocator and its capacity ceiling; single-stock and simulated.
- `1811.05524v1` - Cross-Sectional Variation of Intraday Liquidity, Cross-Impact, and their Effect on Portfolio Execution (2018). Quantifies portfolio
liquidity and a worst-case coupling benefit; not calibrated to executions.
- `1510.09110v1` - Optimal Portfolio Liquidation and Dynamic Mean-variance Criterion (2015). Time-consistent coth schedules, signal- and state-conditioned;
theory only.
- `1701.05016v1` - Mean-Reverting Portfolio Design with Budget Constraint (2017). MRP as a constrained eigen-problem; no costs modelled.
- `2510.02986v1` - FR-LUX: Friction-Aware, Regime-Conditioned Policy Optimization for Implementable Portfolio Management (2025). Cost-in-reward RL with a
trade-space trust region; Sharpe levels figure-only.
- `2509.12764v1` - Myopic Optimality: why reinforcement learning portfolio management strategies lose money (2025). Analytic case that myopic dominates RL in
action-independent markets.
- `2403.10273v2` - Optimal Portfolio Choice with Cross-Impact Propagators (2026). Resolvent solutions and the nonnegative-definite no-manipulation condition.
- `0912.4723v3` - Turnover, account value and diversification of real traders (2010). Real-order evidence that fee schedules shape turnover and
diversification.
- `2309.15767v1` - Implementing portfolio risk management and hedging in practice (2023). QP reformulation route; no backtest and no performance numbers.
- `1201.6130v2` - Portfolio liquidation in dark pools in continuous time (2012). Venue choice under uncertain fills; multi-asset dark orders can be oversized.
- `2103.15400v1` - Research on Portfolio Liquidation Strategy under Discrete Times (2021). Closed-form VaR-optimal horizon; schedule shape fixed a priori.
- `2510.02741v1` - Do Mutual Funds Make Active and Skilled Liquidity Choices in Portfolio Management? Evidence from India (2025). Liquidity activeness as
paid capacity management; single market.

## Where to next in the corpus

- `2607.00475v1` - End-to-end parametric portfolio policies for cross-asset futures; the implementable-policy question without the friction calibration.
- `2605.17307v1` - Deep RL diversified portfolio management with turnover penalties; overlaps FR-LUX without its cost model.
- `2508.16598v1` - Kelly, VIX and hybrid put-writing sizing; position sizing for one options strategy, adjacent to capacity.
- `2507.10701v1` - Kernel learning for mean-variance trading; path-dependent policies that query the same implementability gap.
- `1805.11036v2` - Macroscopic portfolio model as an agent-based differential game; no implementable weights or costs.
- `1605.04600v1` - Learning zero-cost portfolio selection; adversarial online learning, not execution-aware construction.
- `2110.05299v2` - Automated portfolio trading system with recurrent RL; individual-investor framing, weak quantitative content.
- `2608.02917v1` - AMMs as verifiable portfolio products; portfolio-product reading of DeFi, no cross-sectional construction.
