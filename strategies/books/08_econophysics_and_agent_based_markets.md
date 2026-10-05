# Econophysics and Agent-Based Markets

Date: 2026-10-05. Revision 1.

This brief distils the econophysics and agent-based-market slice of the corpus: 441 matching records spanning 1998-2026, of which 108 carry a journal reference. From 40 candidates screened, 12 papers were read in depth and are the evidence base for every number below.

## What this covers

The slice runs from the statistical laws of price change to the machinery that generates them. It covers power-law tails and the heavier-tailed distributions of drawdowns and "dragon-king" outliers, the long memory of order flow and its microstructural origin in order splitting by large traders, nonlinear self-exciting Hawkes processes and the universality of their power-law intensity distributions, and zero-intelligence continuous-double-auction agents. The single most consequential thread is methodological: continuous-double-auction agent models reproduce the canonical stylized facts almost regardless of how intelligent the agents are, so stylized-fact matching is weak validation, and the papers that take calibration seriously replace it with explicit parameter degeneracy checks, null controls and robustness sweeps.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category in the harvest | 441 |
| Years spanned | 1998-2026 |
| Records with a journal reference | 108 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

The selection rule was to cover the required themes directly, keeping papers with printed numbers, real data or an explicit validation discipline, and dropping pure derivations and speculative late-2026 abstracts.

## The short answer

1. Return tail exponents cluster in a narrow band: `1407.5037v2` prints drawdown exponents of 4.00-5.67 against negative-return exponents of 3.53-5.93 (p.14).
2. Drawdowns are not repackaged returns: drawdown exponents exceed negative-return exponents by 0.29 on average and drawups exceed positive returns by 0.43 (p.13) in `1407.5037v2`.
3. The largest moves are a distinct regime: about the 10 largest drawdowns and drawups branch above the fitted power law (p.23) in `1407.5037v2`, including the May 6 2010 flash crash (p.25).
4. Nonlinear self-excited Hawkes processes generate power-law intensity universally: `2102.00242v2` derives P(lambda) ~ lambda^{-1-a} for fast-accelerating intensity maps (pp.2-3), with Zipf's law lambda^{-2} as the special case a = 0 (p.3).
5. Inhibition controls tail weight: with negative-mean marks the asymptotic tail is thinner the more negative the mean (p.2) in `2110.01523v2`.
6. Order flow has long memory across markets: `1504.04354v2` finds Hurst H ~ 0.7 for EUR/USD, GBP/USD and EUR/GBP on every one of 30 trading days (p.17).
7. That memory is attributable to order splitting: `2301.13505v2` finds about 25% of Tokyo Stock Exchange traders are splitters yet issue about 80% of market orders (p.3), consistent with gamma = alpha - 1.
8. The most naive auction agents do well on aggregate statistics: `1002.0917v1` shows zero-intelligence agents produce the most realistic return PDFs, while their degree exponent is -0.51 versus -2.14 for the real prediction market (pp.10-13).
9. Stylized-fact match does not identify a model: `1606.01495v4` finds no unique parameter set for a continuous-double-auction ABM, with most confidence intervals too wide to identify optimum (pp.12-13).
10. Genuine ABM instabilities need nulls: `2607.08907v1` sees its liquidity-stress order parameter reach about 0.34 but zero across all 42 scrambled-sign null cells (p.2).

## 1. Power laws, drawdowns and dragon-kings

`1407.5037v2` studies tick data for 20 of the most liquid world index futures from Jan 1 2005 to Dec 30 2011, at 30-second time steps, using epsilon-drawdowns with epsilon_0 = 1 (pp.2-3). Fitting power-law tails to normalized drawdown returns, it reports drawdown exponents of 4.00-5.67 and negative-return exponents of 3.53-5.93 (p.14); drawup exponents of 4.48-5.55 and positive-return exponents of 3.89-5.65 (p.14). Exponents concentrate in 4 < alpha < 5 for drawdowns and 3.5 < alpha < 4.5 for log returns (p.13). The systematic gap matters: drawdown exponents exceed negative-return exponents by 0.29 on average (p.13).

The largest events do not sit on the fit. Most extreme drawdowns and drawups deviate above the fitted power law, with roughly the 10 largest branching off (p.23); the May 6 2010 flash crash appears in DJ, NQ and ES at 13:41:30 local time (p.25). The generalized non-parametric DK-test and the parametric U-test are used to decide whether these belong to the power law, and they give contradictory results, which the authors attribute to test power (p.23), with U-test results depending on the tail-exponent calibration (p.25). This is the dragon-king claim: extreme events may be a separate regime rather than the tail of the same distribution.

Two structural findings constrain how the tails should be modelled. More than 50% of large drawdowns and drawups contain zero tail returns, and the mean or median relative contribution of large individual returns is 0.35-0.45, so no more than about half the move is attributable to single large returns (pp.15-16). Tail dependence between size and speed is about 0.09-0.12 for drawdowns and 0.075-0.12 for drawups, while size-duration tail dependence falls to zero (pp.26-27). Extreme size co-occurs with speed, not with duration.

## 2. Long memory in order flow and its order-splitting origin

`1504.04354v2` measures sign series for three liquid FX pairs on a large electronic platform over 30 trading days and finds H ~ 0.7 for both arrival and departure signs, on every day, with DFA means of 0.70-0.72 and log-periodogram means of 0.74-0.79 (p.17). Cross-day concatenated series give very similar estimates, rejecting the hypothesis that daily-boundary nonstationarity creates the apparent memory (pp.18-19). But the estimates are estimator-dependent: log-periodogram estimates depend heavily on the number of Fourier frequencies, and an alternative rule-of-thumb gives H ~ 0.5, which the authors regard as not sensible (pp.15-16).

The origin of that memory is pinned down by `2301.13505v2` on nine years of full Tokyo Stock Exchange order flow, Jan 4 2012 to Dec 30 2020. Using a binomial-test clustering that separates order-splitting traders (STs) from random traders (RTs), it finds that typically about 25% of traders are STs but they issue about 80% of all market orders (p.3). Metaorder-length exponents are typically 1 < alpha < 2, with Toyota Motor 2020 at alpha ~ 1.62 (p.3). The alpha-versus-gamma scatter agrees with the Lillo-Mike-Farmer relation gamma = alpha - 1 quantitatively (pp.3-4), while the LMF estimator of the number of STs systematically under-estimates the true count, N_LMF <= N_ST, and is non-robust to heterogeneous splitting intensities (p.4).

The review `0909.1974v2` frames the same fact with a dispute. It cites Lillo-Farmer for a trade-sign ACF decaying as n^{-alpha} with alpha ~ 0.5, but measures about 0.7 in its own high-frequency data (p.7); in tick time the first-lag sign ACF is about 0.10 versus about 0.3 in trade time (p.8). It also stresses that the choice of clock - calendar, event, trade or tick - changes estimated statistics, with faster convergence to Gaussian in trade time (pp.6-7).

## 3. Hawkes processes and universal power-law intensity

`2102.00242v2` solves the field master equation for a nonlinear self-excited marked Hawkes process with tension psi(t) = sum y_i h(t - t_i) and intensity lambda = g(psi). A wide class of such processes has a power-law intensity PDF provided g is fast-accelerating (g > O(psi^2)), the mark distribution is two-sided with non-positive mean, and mark tails decay fast (p.2). The steady intensity PDF is P_ss(lambda) ~ lambda^{-1-a} with a = (dg/dpsi)^{-1} c / h(0) at a root c of a Laplace transform (pp.2-3). Zipf's law P_ss ~ lambda^{-2} appears as the a = 0 special case, exact for zero-mean marks and approximate for symmetric mark distributions (p.3); exponential intensity g ~ e^{beta psi} gives ~ psi^{-2}, and power intensity g ~ psi^n with n > 2 gives ~ psi^{-2-1/n} (p.3).

`2110.01523v2` extends this into a full classification by mapping the non-Markovian model to an SPDE and then a field master equation. With positive marks only, a non-universal power law a can take any value, including a true power law (a > 0) or an intermediate asymptotic (a ~ 0), in contrast to linear Hawkes where only a < 0 exists (p.2). With zero-mean marks a wide class of nonlinear Hawkes exhibits Zipf's law a ~ 1 universally for fast-accelerating maps (p.2), and with negative-mean marks the asymptotic tail becomes thinner the more negative the mean (p.2).

Together the two theory papers give a generator-side lesson for volatility: self-exciting dynamics with saturating or accelerating feedback produce power-law intensity distributions without exogenous jumps. They also warn that a calibrated Hawkes tail is not uniquely identified, because different mechanisms give similar-looking tails, and that models containing only excitation will misstate tail risk relative to ones with inhibitory feedback (p.2).

## 4. Zero-intelligence auctions and the weak stylized-fact criterion

`1002.0917v1` runs three classic continuous-double-auction ABMs - zero-intelligence (ZI), zero-intelligence-plus (ZIP) and Gjerstad-Dickhaut (GD) - with N = 2500 agents, 2000 rounds per day for 200 days, 10 runs averaged, and compares them with the human continuous-double-auction prediction market TAIPEX. The transaction-network degree distribution is power law with ZI exponent -0.51, ZIP -0.83, GD -0.78, an overall range of -0.84 to -0.51, while the real TAIPEX exponent is about -2.14 from 1985 traders (pp.10-13). Community-size exponents are ZI -1.36, ZIP -1.55, GD -1.50 versus TAIPEX about -1.2 (pp.7-11); inter-transaction interval exponents are ZI -1.36, ZIP -1.84 versus TAIPEX about -1.3 (pp.11-12). Only ZI shows non-Gaussian heavy-tailed normalized returns whose curves collapse across lags, while ZIP and GD converge to equilibrium and are near-Gaussian at long lags (pp.10-11, p.14).

The result is double-edged. The most naive agent model best reproduces the aggregate stylized facts, implying that institution and matching rules rather than agent intelligence drive many of them. Yet the authors state their exponents cannot be directly compared between real and simulated markets and differ substantially from the real market - the degree exponent gap of -0.51 versus -2.14 is the clearest case. `0909.1974v2` reaches a congruent structural conclusion: no order-book ABM handles the multidimensional multi-asset case (p.45), and it frames the standing trade-off that toy models are not calibratable while empirical models are not behaviourally grounded.

## 5. Calibration, nulls and falsification discipline

`1606.01495v4` calibrates an intraday continuous-double-auction ABM with high- and low-frequency traders to one JSE stock over one week, 1-5 Nov 2013, using 2300 one-minute bars from quote mid prices. Applying a method of simulated moments with five moments (mean, sd, kurtosis, KS, generalized Hurst) and 10 free parameters, it finds no unique parameter set and confidence intervals far too wide to identify optimum parameters (pp.12-13); only the order-price random-walk step d shows clear convergence, with s_y and s_z showing structure and the rest degenerate (pp.12-18). The model nevertheless reproduces the established log-return stylized facts and fits comparably to prior work, from which the authors argue that stylized-fact recovery is insufficient validation (pp.12-14). Cost is material: about 7 hours per Nelder-Mead experiment and 10 hours per genetic-algorithm experiment on 32 parallel workers (p.11).

`2208.13654v1` offers a constructive recipe. Modelling E-mini S&P 500 futures at 100-millisecond steps with full limit-order-book matching and five trader types, it builds a surrogate then grid-searches a stylized-facts distance, validating with moment-specific p-values and the Franke-Westerhoff moment coverage ratio. Calibrated distances per day are 0.1666-0.2269 (p.16); moment-specific p-values of 0.1833, 0.4167, 0.0833 and 0.6167 for May 3-6 all exceed 0.05, so the model is not rejected (p.17). Simulated flash-crash amplitude is about 7% from 14:30, with the Sell Algorithm running about 17 minutes versus about 20 minutes historically, depth falling to near zero and the spread widening beyond 20 ticks (pp.22-23). Crash amplitude responds non-monotonically to the Sell Algorithm percentage-of-volume r and saturates above r > 5%, non-monotonically to market-maker inventory limit with a turning point near 8000, and monotonically decreasing with fundamental-trader frequency (pp.25-26).

Where `2208.13654v1` triggers its crash with an injected exogenous algorithm, `2607.08907v1` argues for endogenous discipline. Its herding order-book ABM reaches an order parameter phi_empty of about 0.34 at (phi, kappa) = (0.9, 1.0), zero across all 42 scrambled-sign null cells, requiring both high phi and kappa > 0 (p.2). Onset is a smooth crossover with boundary phi*(kappa) = {0.55, 0.45, 0.36} at kappa = {0.6, 0.8, 1.0}, not a discontinuous critical point (pp.2-3). Dry-up is rule-robust and horizon-robust, and reflexivity is rule-specific: price-momentum herding carries a reflexive component of +0.29 while the order-flow rule's is about 0 (pp.2-3).

## 6. Multifractal and DEX diagnostics

`1201.2825v1` shows that a simulated model can match return tails yet fail on temporal structure. In a Mike-Farmer order-driven market with signs from fractional Brownian motion of Hurst H_s and relative prices correlated to Hurst H_x, scaled recurrence intervals of large returns follow a power law with stretched-exponential cutoff, fitted by a generalized Gamma with a = 0.243, b = 0.665, c = -0.0174 (pp.3-4). The power-law exponent beta rises linearly with H_x and is essentially unaffected by H_s: at H_s = 0.7 it climbs 0.466 to 0.835 as H_x goes 0.5 to 0.9 (p.3). Yet the recurrence intervals show only weak long memory, DFA exponents slightly above 0.5, which the authors note is inconsistent with empirical findings (pp.4-5), and they are multifractal, with singularity width flat for H_x < 0.7 then rising (p.5).

`2411.05951v1` extends the diagnostics to a distinct market regime. On Uniswap v3, log-return tails follow a power law with exponent gamma about 3 (inverse cubic), slightly fatter than v2, with Binance in between; volume tails are heavier still, most so on Uniswap v3 ETH/USDT with gamma about 1.95, while Binance volume fits a stretched exponential with beta about 0.48 (pp.5-6). Binance log-return H is about 0.5; volume H is about 0.86 for Uniswap v2 and 0.72 for v3 (p.10). Multifractal spectra are left-asymmetric, indicating large fluctuations dominate, and nearly vanish under Fourier and shuffled surrogates (pp.10-12). The regime difference is operational: minimum inter-transaction time is 45 s on Uniswap versus 0.11 s on Binance, and volatility-volume cross-correlation is much weaker on Uniswap (rho < 0.1 versus 0.52 for Binance ETH/USDT and 0.37 for ETH/USDC) (p.4, p.13).

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| Drawdown tails heavier than returns | risk layer | Cap drawdown sequences, not single bars |
| ~10 biggest events branch above the fit | risk layer | Stress the extreme regime separately |
| Order-flow H ~ 0.7 | docs/usermanauls/execution-algorithms | Assume persistent sign |
| 25% of traders issue 80% of orders | python/nautilus_trader/analysis | Monitor splitter share |
| Nonlinear Hawkes gives power laws | docs/concepts/actors | Use accelerating intensity |
| Inhibitory marks thin the tail | docs/concepts/optimization | Add negative feedback |
| ZI agents match stylized facts | docs/concepts/backtesting | Test rules and clocks, not agent IQ |
| Stylized-fact match hides degeneracy | docs/concepts/backtesting | Add degeneracy checks |
| Crash size turns on MM inventory limit | risk layer | Treat inventory caps as tail controls |
| Herding stress is rule-specific | docs/usermanauls/microstructure-signals | Require nulls |
| DEX vol-volume coupling rho < 0.1 vs 0.52 | docs/concepts/data | Recalibrate crypto models |
| Tails right, interval memory wrong | python/nautilus_trader/backtest | Check temporal structure |

## Caveats

The evidence does not establish that any of these models is a valid picture of a live venue. Several core results are simulation-only - `1002.0917v1`, `1201.2825v1`, `2607.08907v1` and the flash-crash mechanism of `2208.13654v1` - so they describe model behaviour under stated assumptions rather than observed markets. Long-memory and order-splitting findings rest on single markets or platforms: `2301.13505v2` is Tokyo-only, `1504.04354v2` is three FX pairs over 30 days, and neither is tested out of market. Period coverage is narrow where it matters most, with `1407.5037v2` limited to index futures over 2005-2011 and `1606.01495v4` calibrated to one stock and one week. Estimator choice demonstrably changes conclusions, so reported exponents such as H ~ 0.7 or tail indices near 3 carry estimator risk. Several results are explicitly contested: the dragon-king tests disagree in `1407.5037v2`, the sign-ACF exponent is about 0.5 in cited work versus about 0.7 measured in `0909.1974v2`, and crash causality is exogenous in `2208.13654v1` but endogeneity-tested in `2607.08907v1`. Transaction costs, fees and capacity are largely unaddressed across this slice, and none of these papers models a fee or impact constraint for a live system, so no profitability claim follows from any stylized fact here.

## Papers read in depth

- `1407.5037v2` - Power law scaling and "Dragon-Kings" in distributions of intraday financial drawdowns (2014). Drawdown tails are heavier than return tails and the largest moves may be a separate regime.
- `2102.00242v2` - Ubiquitous power law scaling in nonlinear self-excited Hawkes processes (2021). Fast-accelerating nonlinear feedback yields universal power-law intensity, Zipf included.
- `2110.01523v2` - Exact asymptotic solutions to nonlinear Hawkes processes (2021). Full parameter map showing how mark sign and curvature set tail weight.
- `1002.0917v1` - Statistical properties of agent-based models with continuous double auction (2010). Most naive agents best match aggregates, yet exponents miss the real market.
- `0909.1974v2` - Econophysics: Empirical facts and agent-based models (2009). Review that fixes the stylized facts and the realism-versus-calibration trade-off.
- `2301.13505v2` - Can we infer microscopic financial information from long memory in market-order flow? (2023). Order splitting quantitatively explains sign long memory on the TSE.
- `1504.04354v2` - The Long Memory of Order Flow in the Foreign Exchange Spot Market (2015). Robust H ~ 0.7 in FX across days, but estimator-sensitive.
- `1201.2825v1` - Effects of long memory in order submission on recurrence intervals (2012). Correct tails can coexist with wrong interval memory.
- `1606.01495v4` - The Problem of Calibrating an Agent-Based Model of High-Frequency Trading (2016). Stylized facts pass while parameters stay degenerate.
- `2208.13654v1` - High-frequency market simulation and flash crash scenarios (2022). Reproducible surrogate-assisted calibration and crash sensitivity to inventory limits.
- `2607.08907v1` - Herding and Liquidity in Order-Book Markets I (2026). A null-tested, rule-robust smooth liquidity-stress crossover.
- `2411.05951v1` - Approaching multifractal complexity in decentralized cryptocurrency trading (2024). DEX markets show the core facts but weaker vol-volume coupling.

## Where to next in the corpus

- `2006.02460v1` - shallow neural Hawkes kernel estimation; route to Hawkes calibration.
- `1504.03100v1` - rough fractional diffusions as Hawkes limits; links to rough volatility.
- `1301.5007v2` - constrained multivariate Hawkes ergodicity; needed for multi-asset fits.
- `1710.01452v1` - Hawkes transform analysis for dark pools; untested market-calibration claim.
- `1610.01149v1` - Taylor's law of illiquidity; single descriptive fact.
- `1403.0994v3` - Hawkes with different exciting functions; groundwork for intensity families.
- `2508.16589v1` - ARL market making with Hawkes; lead for execution.
- `2401.09361v3` - moment-based neural Hawkes estimation; estimation-focused lead.
- `1303.2044v4` - bubbles and jumps from properly anticipated prices; dragon-king theory.
- `1906.06000v1` - ABM for market design; policy-focused, overlaps the herding read.
