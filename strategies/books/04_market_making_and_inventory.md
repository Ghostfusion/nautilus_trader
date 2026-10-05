# Market Making and Inventory: Quoting, Learning and Internalization

Date: 2026-10-05. Revision 1.

This brief distills 205 harvested records on market making and inventory, spanned across 2000-2026, of which 28 carry a journal reference. It covers the canonical Avellaneda-Stoikov and Cartea-Jaimungal lineage, its multi-asset, multi-tier and multi-currency extensions, online parameter learning, and the empirical economics of spread, impact and volatility.

## What this covers

The slice gathers the closed-form quoting lineage and the papers that extend it: the forced uniqueness theorem that unifies Avellaneda-Stoikov (AS) and Cartea-Jaimungal (CJ), logarithmic-regret learning of fill intensity, adverse selection and price reading under client tiers, multi-currency FX inventory control, multi-contract event-time quoting, hedging with market impact, and the self-exciting dynamics of the spread itself. It also includes two simulation studies, a reinforcement-learning dealer and an agent-based model, that stress-test the analytic rankings. The single most consequential thing in the slice is that the inventory-penalty and risk-aversion parameters cannot be calibrated independently: `2606.01477v3` proves that under five axioms the preference functional is forced to be entropic with a unique gamma, pinning phi = gamma*sigma^2/2, so desks that tune the running penalty and the quote-side risk aversion separately are structurally misspecified.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category in the harvest | 205 |
| Years spanned | 2000-2026 |
| Records with a journal reference | 28 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

Selection kept the canonical AS/CJ inventory lineage and its unification, learning and regret, adverse selection and quoting, multi-contract event-time quoting, multi-currency FX inventory, spread self-excitation, reinforcement-learning and agent-based dealer strategies, and the empirical spread/impact/volatility economics; sibling-category records were left to the limit-order-book, econophysics, market-design and options slices.

## The short answer

1. Optimal quoting reduces to an inventory-aware reservation price plus a spread, and `2606.01477v3` shows the whole AS/CJ family is forced to one risk scalar gamma.
2. The running inventory penalty is forced to phi = gamma*sigma^2/2 and the terminal penalty to alpha = (1/2)L''(0), with gamma = 2*phi/sigma^2 (`2606.01477v3`).
3. Closed-form quotes skew to mean-revert inventory and the spread widens with sigma^2 and narrows with liquidity (A and k), but is ambiguous in gamma (`1105.3115v5`).
4. Fill-intensity learning can achieve regret R(T) <= C1 ln^2 T + C2, with simulations fitting 0.008 ln^2 T + 0.0867 at R^2 = 0.966 (`2409.02025v2`).
5. Client flow must be tiered: fitted logistic intensities separate a price-sensitive Tier 1 (beta = 5 bp^-1) from Tier 2 (beta = 15 bp^-1) (`2112.02269v3`).
6. Adverse selection and price reading are distinct risks, and the sign of the optimal response flips when signal speed crosses fill sensitivity (`2508.20225v6`).
7. Multi-currency inventory control is scalable through a matrix Riccati approximation, with internalization near the 80% reported for G10 top-tier banks (`2207.04100v4`).
8. Hedging costs and permanent impact create a pure-internalization no-trade plateau, so hedging should be a threshold policy (`2106.06974v6`).
9. The spread is a state-dependent self-exciting jump process with power-law kernel memory, and a Hawkes model beats ACDP at 3-30 s horizons (`2303.02038v2`).
10. Empirical spread is mostly adverse selection, not inventory rent, with an intercept near zero on electronic markets (`physics/0603084v3`).

## 1. The AS/CJ lineage and its forced unification

`2606.01477v3` treats the market maker as an agent with a dynamic preference functional over liquidation-adjusted terminal wealth and imposes five axioms: cash-additivity, normalization, concavity, strong dynamic consistency and law-invariance. Under these, the Kupper-Schachermayer representation theorem forces the functional to be the entropic certainty-equivalent with a unique gamma > 0 (Theorem 10, p.16). CARA ceases to be a modelling choice and becomes a theorem; CRRA and HARA fail cash-additivity, and path-dependent running penalties used as a primitive violate wealth-summary.

The consequences pin the CJ control parameters. The running penalty is forced to phi = gamma*sigma^2/2 (Corollary 20, p.27) and the terminal penalty to alpha = (1/2)L''(0) (Corollary 22, p.28), so (phi, alpha, gamma) collapse to a single preference scalar (Corollary 17, p.25). The inverse relation gamma = 2*phi/sigma^2 (Corollary 23, p.29) is a direct consistency check: a constant per-second penalty implies an implied gamma_t = 2*phi/sigma_t^2 that drifts with volatility, a structural misspecification in stochastic-volatility regimes. Clock-invariance means gamma is identical in wall-clock and quadratic-variation business time, and the running penalty must scale as phi_t = gamma*sigma_t^2/2 (Remark 13, pp.22-23; Proposition 37).

This rests entirely on the existence of a well-defined preference functional satisfying the axioms; it does not apply to desks using CJ heuristically as an approximation. There is no data, no calibration and no PnL evidence.

## 2. Closed-form quoting: spread, volatility and liquidity

`1105.3115v5` solves the inventory-risk problem with a change of variables that linearizes the HJB equation, giving a spectral characterization of the long-horizon solution and asymptotic quotes delta^{b*} -> (1/gamma) ln(1 + gamma/k) + (1/k) ln(f0_q / f0_{q+1}), where f0 is the eigenvector of the smallest eigenvalue of M (pp.32-33). The spread psi* widens with sigma^2, decreases with the fill intensity A, is ambiguous in gamma because two effects oppose, and decreases in k (pp.16-19). Quotes skew to mean-revert inventory (p.16).

The paper adds a verification theorem absent from the original AS treatment and a one-day backtest on France Telecom (15 March 2012, ATS = 1105 shares, 10:00-16:00). The strategy earns a few hundred EUR while a naive maker quoting only at the first book limit loses money (Figs 11-12, pp.21-22). The gamma used was set arbitrarily so intraday inventory stayed in [-10,10] ATS, and fills are assumed complete whenever a trade prints at or above the quote, which is optimistic.

## 3. Online learning and regret

`2409.02025v2` works in the ergodic (infinite-horizon) AS model with an unknown liquidity-taker price sensitivity kappa, estimated online by a regularized maximum-likelihood estimator from Bernoulli fill signals. The regret against the optimal long-run average reward is bounded by R(T) <= C1 ln^2 T + C2 (Theorem 17, p.19). Simulations with lambda = 0.4/s, kappa* = 10, sigma = 0.01 s^{-1/2}, qbar = 30, phi = 1e-6, T = 1000 s and 1000 scenarios (pp.20-21) fit 0.008 ln^2 T + 0.0867 (R^2 = 0.966) and 0.0107 ln^2 T + 0.0417 (R^2 = 0.934); the ln T fits give R^2 of 0.905 and 0.880, so the ln^2 T form is better supported (Fig 3, pp.21-22). A myopic strategy posting at 1/kappa_t has linearly growing regret versus sublinear regret for the optimal-from-estimate policy (Fig 4, p.22), and sliding-window and EWMA re-estimation handle a time-varying kappa.

The guarantee depends on the Poisson fill model being correctly specified with only kappa unknown. There is no adverse selection, alpha, fees or multi-asset effect, and the objective is stationary rather than a finite-horizon PnL.

## 4. Adverse selection, price reading and client tiers

`2508.20225v6` models a dealer facing both adverse selection from informed clients and price reading from skew sniffers who infer inventory from quotes. Using a first-order perturbation around the no-informational-risk baseline, it separates a global value-function effect from a tier-specific quote effect. When inventory is zero the spread widens to compensate informational risk, and the net second-order effect is de-skewing, strongest at the top of book, with magnitude set by the safe volume share (pp.14-16). In PnL and risk simulations "No Action" loses versus the no-skew-sniffer baseline, a naive "No Skew" restores PnL but raises risk, while the "Optimal" first-order policy raises PnL and reduces risk, most efficient when few sniffers are present (Figs 3-5, pp.15-17).

Adverse selection depends on signal speed relative to fill sensitivity. With slow signals (beta < kappa) the dealer acts less risk-averse to the first tier, tightens its spread, widens the spread to the informed tier, and can use informed top-of-book prices as signal subscription; with sharp footprints (beta > kappa) the response reverses to protection (pp.17-18, Fig 6). Cross-effects between adverse selection and price reading appear only at second order and are dropped, and the local expansion is least reliable for large inventories.

`2112.02269v3` grounds tiering empirically by fitting logistic intensities Lambda_k(delta) = lambda_k / (1 + exp(alpha_k + beta_k delta)) to anonymized EURUSD client flow from January-April 2021 and clustering individual parameters into two tiers: Tier 1 with alpha = -0.3, beta = 5 bp^-1, and Tier 2 with alpha = -1.9, beta = 15 bp^-1, with lambda amplitudes proportional to (0.4, 0.25, 0.19, 0.1, 0.05, 0.01) (pp.4-5). The inventory-neutral top-of-book spread is 0.26 bps for the more price-sensitive tier and 0.55 bps for the less sensitive tier, against a market composite of 0.23 bps and a primary venue of 0.65 bps, again without feeding any market spread into the model (p.8).

## 5. Multi-currency and multi-contract quoting

`2207.04100v4` extends inventory control to a d-currency FX cash dealer with correlated Brownian exchange rates, client tiers and dealer-to-dealer hedging under linear permanent impact. A quadratic Hamiltonian approximation produces a matrix Riccati-like ODE for A(t) and B(t) solved by an Euler scheme, scalable to any number of pairs without an inventory grid. Validating d = 2 against a monotone implicit-Euler grid solution shows significant deviations only under extreme conditions: order-flow asymmetry up to fivefold and very high or low risk aversion (p.6). Figures show GBP inventory producing correlation-aware protective pricing of EURUSD and the EURGBP cross (Figs 1-3, pp.6-9), and inventory autocorrelation decaying far more slowly than portfolio risk autocorrelation, so the dealer offloads risk quickly while turning over inventory slowly to save impact cost (Fig 5, p.10). The internalization ratio is consistent with the roughly 80% previously reported for G10 top-tier banks (p.8).

`2507.05749v2` addresses multi-contract quoting for a NIFTY futures calendar spread (February 2022 versus March 2022), quoting one leg while designating a reference leg ex ante. On NSE tick data for 2022-02-07 the two contracts carry 49,828,303 and 16,349,197 tick events (Table 6, p.16). A multivariate Hawkes arrival ratio agrees with the hindsight spread-minimizing oracle at 0.6856, a Composite Liquidity Factor CLF4 at 0.6358, CLF3 at 0.5919, CLF2 at 0.4754 and CLF1 at 0.3796, with joint agreement 0.4149 and both rules wrong in 9.34% of windows (Table 8, pp.19-20). Hawkes is better in high-persistence regimes and CLF4 in low or medium persistence (Fig 7, p.21). The illustrative case shows a current-month reference realizing 56.2 against a target of 58.5 (slippage 2.3 INR) while the next-month reference realizes 58.0 (slippage 0.5 INR), so stability can beat instantaneous spread width (Tables 2-4, pp.6-7).

## 6. Hedging, impact and internalization bands

`2106.06974v6` combines finite-difference inventory terms with an Almgren-Chriss-like derivative term in a partial integro-differential equation for a dealer quoting clients and hedging in a liquidity pool. The model produces a pure-flow internalization plateau around zero inventory where the dealer chooses not to trade externally (Fig 4, p.23), with an execution rate that is nonincreasing in inventory and near-linear outside the plateau (pp.22-24). Without any market spread input it yields a 1M$ bid-ask spread of 0.32 bps against an actual composite interbank USDCNH spread of 0.38 bps (p.23). The comparative statics widen the internalization area for higher execution cost, higher permanent impact, a larger franchise and lower risk aversion (Figs 7-10, pp.26-28), and the PDE value function matches Monte Carlo under the optimal controls (Fig 6, p.26).

`2303.02038v2` models the spread itself as a state-dependent self-exciting Hawkes process over spread jumps of size 1..K on CAC40 names. Estimated kernels show contrarian dominance: a jump of size e is mostly triggered by past jumps of size -e (p.11), with power-law decay t^{-beta} where beta is around 1 and a latency bump near 0.2 ms (p.11). The mean spread is 1.51 ticks for the CAC40 future, 2.44 for AXA, 2.54 for BNP and 3.52 for NOKIA, against calendar means of 1.40, 3.04, 3.17 and 4.43 (Table 1, p.8). At a 3 s horizon on AXA the model's MSE is 0.363 versus 0.408 for a Last-value benchmark and 0.514 for ACDP; at 30 s it is 0.648 versus 1.082 and 0.808 (Table 3, p.19).

## 7. Agent-based and reinforcement-learning evidence on dealer risk aversion

`2312.05943v1` embeds an AS dealer, a Fushimi-Gonzalez Rojas inventory-skew (IR) dealer and a naive fixed-size dealer in a synthetic agent-based limit-order-book market with 1000 agents (450 fundamentalists, 450 chartists, 99 noise traders, 1 monopolistic dealer) and 100 simulations of 40,000 steps. In the probabilistic benchmark the naive dealer outperforms the AS dealer and random order sizes reduce AS profit (Fig 1, pp.9-10). In the ABM the naive dealer posts a 30.0% return at 48.9% volatility, AS a 25.3% return at 2.7% volatility and IR a 1.5% return at 1.3% volatility, so risk-adjusted performance favours AS (p.16). Higher dealer risk aversion raises wealth with diminishing returns for AS and near-linear gains for IR (Fig 3, p.18). Lower inventory skew raises IR return by about 26% and lowers market volatility by about 20% (pp.20-21), while larger order size lowers market volatility and kurtosis and higher AS risk aversion raises volatility but lowers kurtosis (Figs 6-7, pp.22-23).

`1911.05892v1` trains a PPO reinforcement-learning maker in a multi-agent OTC dealer simulator with 20 investor agents and partial observability, using 15-minute timesteps over 5000+ steps and 5 random seeds. The RL agent beats random and persistent competitors and clearly beats an adaptive mean-variance competitor at risk aversion gamma = 2, though the adaptive agent closes or overtakes at low risk aversion (Tables 1-2, pp.6-7). It recovers competitor pricing with mean buy and sell spreads from -0.03 to -0.22, matching the theorem-optimal point of 0 against Unif[-1,1], -0.25 against Unif[-0.5,0.5] and about 0.6 against the adaptive agent (Table 3, Fig 1, pp.6-7). It learns inventory skew and, under a drift, holds average -16.06 units when mu = -1 (Fig 4, p.7); all three risk penalties reduce inventory and inventory-PnL standard deviation (Fig 5, p.7). The reward penalty weights, however, are ad hoc.

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| Risk aversion is one scalar gamma and phi = gamma*sigma^2/2 (`2606.01477v3`) | risk layer, model | Cross-check inventory-penalty and quote-risk calibration; reject independent tuning |
| Inventory penalty must track realized variance, not wall-clock seconds | risk layer | Parameterize the penalty on a variance clock |
| Spread widens with sigma^2 and narrows with A and k (`1105.3115v5`) | execution layer, backtest | Make spread sensitivity a calibration checklist |
| Full-fill assumptions dominate backtest edge (`1105.3115v5`) | backtest, data layer | Model queue and latency, not just quote-level fills |
| Online kappa learning gives ln^2 T regret (`2409.02025v2`) | execution layer | Add sliding-window or EWMA fill-intensity re-estimation |
| Client tiers differ in price sensitivity (`2112.02269v3`) | execution layer, adapters | Cluster fitted intensities and serve tier-specific ladders |
| Adverse selection and price reading separate (`2508.20225v6`) | risk layer | Keep toxicity and quote-leakage controls distinct |
| Multi-pair risk needs portfolio-level hedging (`2207.04100v4`) | risk layer, model | Track correlated exposure, not per-pair limits |
| Hedging has a no-trade plateau (`2106.06974v6`) | execution layer | Use threshold rebalancing, not continuous hedging |
| Leg choice is execution risk (`2507.05749v2`) | execution layer, data layer | Make anchor leg an explicit execution decision |
| Spread is self-exciting with power-law memory (`2303.02038v2`) | docs/usermanauls/microstructure-signals | Model spread dynamics over 3-30 s horizons |
| Analytic rankings can invert under feedback (`2312.05943v1`) | backtest | Validate in a market-impact simulation before trusting |

## Caveats

The evidence base is dominated by theory and simulation, not live trading: only `1105.3115v5` reports a backtest on real market data, and that is a single stock on a single day. `2112.02269v3` and `2207.04100v4` calibrate to one FX franchise's flow and use parameter sets the authors describe as illustrative rather than production. `2508.20225v6` relies on a first-order local expansion around a no-informational-risk baseline, so its conclusions are least reliable for large inventories or strong informational risk. `2409.02025v2` assumes the fill-probability functional form is exactly correct and only its slope is unknown. `1911.05892v1` and `2312.05943v1` are calibrated synthetic markets with no informed traders and no real data. The empirical `physics/0603084v3` results come from PSE, NYSE and index futures in 2002-2005, neglect direct transaction costs, and its authors note the argument does not directly apply to large-tick markets. Most empirical results span one market over one or two years, with no out-of-sample evaluation across venues or regimes. `2606.01477v3` proves a consistency condition but estimates no gamma or phi and offers no PnL evidence, so it constrains calibration without validating any strategy.

## Papers read in depth

- `2606.01477v3` - Avellaneda-Stoikov and Cartea-Jaimungal as One Framework: A Forced Uniqueness Theorem for Inventory Market Making (2026). Forced entropic preferences pin the AS/CJ parameters to one gamma, with no empirical validation.
- `1105.3115v5` - Dealing with the Inventory Risk. A solution to the market making problem (2011). Spectral closed-form quotes plus a verification theorem; optimistic fill assumptions.
- `2409.02025v2` - Logarithmic regret in the ergodic Avellaneda-Stoikov market making model (2024). ln^2 T regret for online kappa learning under a correct Poisson fill model.
- `2508.20225v6` - Optimal Quoting under Adverse Selection and Price Reading (2025). First-order policy reconciles toxicity and skew sniffing; local and simulation-only.
- `2207.04100v4` - Dealing with multi-currency inventory risk in FX cash markets (2022). Riccati approximation scales inventory control across correlated pairs.
- `2106.06974v6` - Algorithmic market making in dealer markets with hedging and market impact (2021). PIDE yields an internalization plateau and an endogenous spread.
- `2112.02269v3` - Market making by an FX dealer: tiers, pricing ladders and hedging rates for optimal risk control (2021). Data-driven client tiers and operational risk metrics.
- `2507.05749v2` - Event-Time Anchor Selection for Multi-Contract Quoting (2025). Hawkes beats book-state signals on oracle agreement; single day, no PnL.
- `2303.02038v2` - The self-exciting nature of the bid-ask spread dynamics (2023). State-dependent Hawkes spreads with power-law memory and better short-horizon forecasts.
- `2312.05943v1` - Dealer Strategies in Agent-Based Models (2023). Analytic dealer rankings invert once market feedback is added.
- `1911.05892v1` - Reinforcement Learning for Market Making in a Multi-agent Dealer Market (2019). PPO learns competitor pricing and inventory skew, with ad hoc risk weights.
- `physics/0603084v3` - Relation between Bid-Ask Spread, Impact and Volatility in Double Auction Markets (2006). Spread is mostly adverse selection on electronic markets; intercept near zero.

## Where to next in the corpus

- `2606.09454v1` - Axiomatic Market Making: same forcing territory as the unification theorem, a second route to the same constraints.
- `2606.06413v1` - Competition in dealer markets with internalisation/externalisation: fills the market-structure gap around internalization ratios.
- `2508.16588v1` - Robust Market Making: To Quote, or not To Quote: an alternative reinforcement-learning formulation to the PPO dealer.
- `1807.08278v3` - Liquidity in Competitive Dealer Markets: dealer-market economics behind the tiered ladder results.
- `2605.19742v4` - Glosten-Milgrom privacy subsidy: links adverse selection to privacy and disclosure design.
- `1911.02361v1` - HCR spread density: an alternative density model for the spread.
- `1606.07381v1` - Spread-vol-volume market-making profit: overlaps the empirical economics and may extend the calibration relations.
- `1511.07773v4` - Corporate bond RFQ behavior: RFQ flow behaviour outside FX, relevant to client-tier modelling.
- `2506.18147v2` - Bond MD2C causal inference: causal framing of dealer-to-client flow.
