# Order Flow and Toxicity: Persistence, Diffusive Prices and Adverse Selection

Date: 2026-10-05. Revision 1.

This brief covers the 234 catalogued records on order flow and toxicity, spanning 2001-2026, of which 26 carry a journal
reference. Twelve papers were read in depth from 80 candidates screened across two merged themes.

## What this covers

The slice runs from the microscopic origin of order-flow long memory, through the diffusive-price paradox and the impact
exponent that resolves it, to order-flow imbalance as a predictor, price-discovery measurement across venues, adverse
selection and informed trading, and market-data integrity. The single most consequential result is that signed flow is
genuinely predictable over thousands of orders, yet prices stay diffusive because price impact is concave: the
square-root law keeps a long-memory flow process from producing tradeable drift. That result sits on two same-author
theory papers, so it is load-bearing but not yet independently replicated.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category | 234 |
| Years spanned | 2001-2026 |
| Records with a journal reference | 26 |
| Candidate papers screened | 80 (two merged themes) |
| Papers read in depth | 12 |

Selection required mandatory coverage of long memory, the flow/price paradox, OFI prediction, price discovery and
adverse selection, plus two real-data anchors for FX long memory and Hyperliquid sunshine trading.

## The short answer

1. Signed order flow is positively autocorrelated out to tens of thousands of orders, i.e. many days, on the London
   Stock Exchange (`1108.1632v2`).
2. Below a few hours - roughly 500 transactions or less - that persistence is overwhelmingly order splitting by
   individual members, not herding (`1108.1632v2`).
3. In FX spot the long memory is genuine: Hurst H ~ 0.7 for EUR/USD, GBP/USD and EUR/GBP on each of 30 trading days in
   2010 (`1504.04354v2`).
4. Long memory collapses to one microscopic parameter: about 25% of TSE traders are splitters but issue about 80% of
   market orders, with 1 < alpha < 2 and gamma = alpha - 1 (`2301.13505v2`).
5. Prices stay diffusive despite predictable flow if and only if impact is concave enough: superdiffusion arises if and
   only if 2delta > alpha (`2502.17906v4`, `2608.00988v1`).
6. The square-root law delta = 1/2 sits inside the diffusive region and reproduces the inverse-cubic law with beta =
   alpha/delta ~ 3 (`2502.17906v4`).
7. OFI-type variables explain most contemporaneous mid-price change: average out-of-sample R^2 rises from 32.89 (OFI at
   30s) to 83.57 (log-GOFI at 30s) (`2112.02947v1`).
8. OFI is stationary but non-Gaussian and cross-excited, so buy and sell arrivals need a bivariate Hawkes process, not a
   Poisson count (`2408.03594v1`).
9. Price discovery is venue- and method-dependent: Binance leads the Uniswap v2 pool on all five 2024 dates (Hasbrouck
   shares 0.628-0.995), but the BTC futures lead is weak and spot leads on 5 Aug 2024 (`2506.08718v1`).

## 1. Long memory in signed order flow and its splitting origin

Signed order flow is not memoryless. On London Stock Exchange data with broker identifiers, buy and sell signs are
positively autocorrelated out to tens of thousands of orders, i.e. many days (`1108.1632v2`, abstract p.1). Decomposing
that autocorrelation into a same-member component C_same and a cross-member C_other shows that below a few hours
-roughly 500 transactions or less, typically about an hour - persistence is overwhelmingly due to autocorrelated trading
by individual members rather than interactions between them (p.35). The herding null hypotheses are strongly rejected
while splitting is not rejected (p.36); after a price-changing market order other brokers lean slightly more to the
opposite side (negative C_other), while after a non-price-changing order they lean the same way (pp.34-35).

The effect is not an equity or data artefact. On Hotspot FX, a multi-institution platform, the Hurst exponent is H ~ 0.7
for EUR/USD, GBP/USD and EUR/GBP on every one of 30 trading days in May-June 2010, and Lo's modified R/S test rejects
short memory at the 5% level for every day and pair (`1504.04354v2`, p.10). Short-range negative autocorrelations up to
about 25 events, magnitude below ~0.1, precede the long-range positive ones (p.10), so two-regime flow models are
needed. On the Tokyo Stock Exchange, a binomial test against symmetric Bernoulli at theta = 0.01 classifies about 25% of
traders as splitters who nonetheless issue about 80% of all market orders; the metaorder-length exponent typically
satisfies 1 < alpha < 2 (Toyota 2020 alpha ~ 1.62) and the autocorrelation exponent obeys gamma = alpha - 1, the first
quantitative validation of LMF (`2301.13505v2`, Fig. 2 p.3, Fig. 3 p.3, Fig. 4 p.4). The prefactor-based splitter count is
a downward-biased lower bound (p.4).

## 2. The diffusive-price paradox and the concavity of impact

The central puzzle is that predictable order flow does not become predictable prices. Generalising LMF to nonlinear
impact I(Q) ~ Q^delta with 0 < delta <= 1 gives an exact mean-squared displacement E[dm^2(t)] ~ t^(1+2delta-alpha) when
2delta > alpha and ~ t otherwise, so superdiffusion arises if and only if 2delta > alpha (`2502.17906v4`, Eq. 6 p.3).
Under the standard range 1 < alpha < 2, price dynamics is always normal diffusion for delta <= 1/2; the square-root law
delta = 1/2 therefore guarantees diffusive prices despite long-memory flow (p.3). The phase boundary is 2delta = alpha
(Fig. 3a p.4), and the model reproduces the inverse-cubic law P(dm) ~ (dm)^(-beta) with beta = alpha/delta ~ 3 for delta
= 1/2, alpha ~ 3/2, plus volatility clustering CV(tau) ~ tau^(-zeta) with zeta ~ alpha - 1 (Eqs. 7a-7c, p.4).

The companion exactly solvable model reaches the same conclusion: delta <= 1/2 is necessary and sufficient for diffusion
for any gamma in (0,1), with the same inverse-cubic law and zeta ~ alpha - 1 (`2608.00988v1`, p.2, Figs. 2-3 p.12-13).
Numerically, MSD is normal for delta in {0.25, 0.50} and shows a superdiffusion-to-normal crossover for delta in {0.75,
1.00} (Fig. 1 p.11; Fig. 15 p.24). Both papers are same-author, so this is a mutually reinforcing result rather than
independent evidence, and neither uses real data.

## 3. Order flow imbalance as a state variable and a forecast

OFI is a strong contemporaneous state variable. On CSI 500 snapshots every 3 seconds, relaxing the original OFI
assumption that the best quote moves by at most one minimum quotation unit yields GOFI and log-GOFI; averaged across 10
stocks, out-of-sample R^2 for predicting mid-price change rises from 32.89 (OFI at 30s) to 83.57 (log-GOFI at 30s), from
38.13 to 85.37 at 1 minute, and from 42.57 to 86.01 at 5 minutes (`2112.02947v1`, abstract p.1, Tables 1-3 p.5-6). The
sample dates are not stated in the sections read, the table columns are implicitly labelled, and high R^2 on the same
interval is not forecast skill.

On one NSE NIFTY futures day, buy and sell market-order arrivals are modelled as a bivariate Hawkes process with self-
and cross-excitation. SPA p-values are 0.002 (exponential), 0.743 (sum-of-exponential), 0.257 (conditional law), 0.101
(VAR), and 0.0 for the EM, Poisson and power-law kernels (`2408.03594v1`, Table 4 p.16). The sum-of-exponential kernel is
favoured operationally because its Markovian structure removes simulation and is far cheaper (p.17). OFI is stationary
by ADF (stat -12.878, p = 4.7e-24) but not normal (K-S p = 3e-20) (Appendix I p.20), so forecast the distribution, not
just the mean.

## 4. Price discovery across venues

Price discovery is measurable but method- and venue-dependent. Comparing Binance 1-second data with a Uniswap v2 pool on
five 2024 dates, Binance leads on all five: Hasbrouck information shares of 0.959, 0.965, 0.628, 0.986 and 0.995;
Gonzalo-Granger agrees in 4 of 5 and Hayashi-Yoshida in all 5 (`2506.08718v1`, Table 3.36 p.87). For BTC futures versus
spot, futures lead in 4 of 5 dates on Hasbrouck (shares 0.563, 0.552, 0.521, 0.158, 0.539) but spot leads on 5 Aug 2024
(share 0.158; lag -0.571 s), and Gonzalo-Granger is inconclusive with both orderings' nulls rejected (pp.81-85). The
authors conclude the futures lead is weaker than in 2017-2020, attributing this to spot ETFs and improved efficiency
(pp.85-86). Single-metric conclusions are fragile.

## 5. Adverse selection, informedness and the Kyle foundation

Adverse selection is a first-order cost whose magnitude depends on the state of informedness. In an agent-based market
with reinforcement-learning makers, informed flow is most damaging when aggregate informedness is low, while
profitability trends upward in the informedness fraction psi (`2606.05882v2`, abstract p.1). Low-volatility OLS slopes of
mean final wealth on psi are 1.12 (R^2 0.62) for aggressive makers, 1.57 (R^2 0.65) for conservative, and 5.05 (R^2
0.64) summed; high-volatility slopes are mixed (sum 0.05, R^2 0.33) (Table 1 p.18). Mean posted spread is 2.41 versus
2.64 ticks, about 9.5% higher for conservative quoting, but spread-versus-psi regressions have R^2 below 0.05, so the
trend is not quoting width (p.19). Inventory-aware quoting narrows half-spreads by roughly 1.6-3.2x and raises posted
volume about 35%-120% (pp.19-20).

The theoretical foundation is the Kyle game. In discrete time a sequential equilibrium exists for any finite-support
noise and value distributions, but generally only in mixed strategies - Example 2.21 has no pure-strategy equilibrium
and at epsilon = 1/8 the mixed buy probability solves a degree-7 polynomial with root alpha* = 0.77464 (`2312.00904v2`,
Theorem 2.16 p.10, p.14-15). The price function need not be nondecreasing in true value (Counter-Example 3.6 p.15-17).
The insider camouflages with noise, so toxicity is only partially observable.

In real data, order exposure changes adverse selection. On Hyperliquid, visible protocol-native TWAPs have about 9 basis
points lower temporary impact than latent metaorders and leave roughly 5 basis points less post-execution displacement
(`2606.15715v1`, p.3). Hidden metaorders executed with visible same-side TWAP flow face about 0.8-0.9 bp higher
displacement per 10 percentage point rise in same-side visible dominance (p.3). Median statistical metaorder is ~USD
8.5k at participation eta ~ 3.4%, median TWAP ~USD 9.1k at eta ~ 0.45%, and hidden schedules are front-loaded and
U-shaped (p.3, p.6-7).

## 6. Market-data integrity

Consolidated feeds can corrupt the series strategies are built on. On 11 Aug 2015, 66.2% of AAPL's 482,578 trades
printed out of sequence at the SIP, with BAC 50.1%, GOOG 56.3%, XOM 43.0%, IBM 26.4% and small caps near zero
(`1810.11091v1`, Table 1 p.16). The out-of-sequence count scales linearly with trade count, slope 0.66 and R^2 0.99
(p.14). Median SIP-to-exchange latencies are within 700 ms of each other (p.8), and negative NBBO spreads appear at the
SIP although no single exchange is crossed (pp.10-11). Integrity risk is worst exactly where strategies trade most.

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| Sub-hour flow persistence is splitting (`1108.1632v2`) | docs/usermanauls/microstructure-signals | Model own and others' child-order autocorrelation |
| Hurst ~ 0.7 in FX spot (`1504.04354v2`) | docs/concepts/data | Calibrate synthetic FX flow to H ~ 0.7 with two regimes |
| gamma = alpha - 1 from public data (`2301.13505v2`) | python/nautilus_trader/data | Calibrate synthetic flow from one microscopic parameter |
| delta <= 1/2 keeps prices diffusive (`2502.17906v4`, `2608.00988v1`) | docs/concepts/backtesting | Cap impact concavity; reject flow-to-drift simulators |
| GOFI R^2 32.89 to 86.01 (`2112.02947v1`) | python/nautilus_trader/indicators | Build quote-movement-aware OFI features |
| OFI non-Gaussian and cross-excited (`2408.03594v1`) | python/nautilus_trader/model | Forecast the OFI distribution via bivariate Hawkes |
| Discovery metrics disagree (`2506.08718v1`) | docs/usermanauls/cross-venue-relative-value | Report Hasbrouck, Gonzalo-Granger and Hayashi-Yoshida together |
| SIP misorders trades, scaling with volume (`1810.11091v1`) | docs/concepts/data | Use direct feeds for execution and bar construction |
| Adverse selection worst at low informedness (`2606.05882v2`) | docs/usermanauls/market-making | Gate quoting on toxicity; skew by inventory |
| Toxicity only partially observable (`2312.00904v2`) | python/nautilus_trader/risk | Prefer probabilistic detection over deterministic rules |
| Visible flow is cheaper, hidden pays more (`2606.15715v1`) | python/nautilus_trader/execution | Model visibility and anti-gaming |
| Real metaorders are front-loaded, U-shaped (`2606.15715v1`) | docs/usermanauls/execution-algorithms | Simulate U-shaped schedules, not uniform TWAP |

## Caveats

None of the predictor or discovery papers reports P&L, capacity or transaction costs, so nothing here is a net-of-cost
strategy demonstration. Several rest on single-day or single-instrument samples - one NSE futures day, five crypto dates
with as few as 43 DEX observations, one representative day for some SIP figures - so temporal and cross-sectional
robustness is untested. Only 7 of the 12 papers use real markets: the paradox pair are theory checked by Monte Carlo,
and the market-maker profitability study is a reinforcement-learning simulation not calibrated to a named instrument.
The diffusive-price results assume homogeneous splitting intensities, no post-metaorder impact decay and unit
child-order size, all flagged by the authors as limitations. The source of long memory is contested: LSE order splitting
rejects herding below a few hours, while others report structural breaks rejecting long memory for roughly two thirds of
LSE series, though the FX evidence rejects the artefact explanation. Crypto futures-versus-spot discovery is weak and
date-dependent, so no unconditional "futures lead" should be assumed. No dedicated VPIN or Easley-O'Hara toxicity paper
was read, so toxicity is covered only indirectly through informed-fraction and PIN-adjacent measures.

## Papers read in depth

- `1108.1632v2` - Why is equity order flow so persistent? (2011). Splitting beats herding below a few hours on LSE member
   data; the reference decomposition.
- `2301.13505v2` - Can we infer microscopic financial information from the long memory in market-order flow? (2023). TSE
   validation of gamma = alpha - 1 and a lower-bound splitter count; the strongest microscopic test.
- `2502.17906v4` - Why do financial prices exhibit Brownian motion despite predictable order flow? (2025). Maps LMF to a
   Levy walk; delta <= 1/2 gives diffusion.
- `2608.00988v1` - Exactly solvable model for the diffusive price-dynamics paradox under long-range correlated
   market-order flow (2026). Companion exact solution, same threshold; reinforcing not independent.
- `2112.02947v1` - The Price Impact of Generalized Order Flow Imbalance (2021). GOFI and log-GOFI lift R^2 sharply;
   contemporaneous, not a forecast.
- `2408.03594v1` - Forecasting High Frequency Order Flow Imbalance (2024). Bivariate Hawkes on one day; sum-of-exponential
   kernel wins operationally.
- `2506.08718v1` - Price Discovery in Cryptocurrency Markets (2025). Three methods, venue-dependent lead; weak BTC futures
   lead.
- `1810.11091v1` - Price Discovery and the Accuracy of Consolidated Data Feeds in the U.S. Equity Markets (2018). SIP
   misordering scales with volume; direct feeds matter.
- `2606.05882v2` - Market Informedness and Market-Maker Profitability (2026). Adverse selection worst at low informedness;
   inventory-aware quoting dominates.
- `2312.00904v2` - Insider trading in discrete time Kyle games (2023). Existence in mixed strategies; toxicity only
   partially observable.
- `1504.04354v2` - The Long Memory of Order Flow in the Foreign Exchange Spot Market (2015). H ~ 0.7 across pairs and
   days; true long memory, not breaks.
- `2606.15715v1` - Trading in the Sunshine or in the Shade (2026). On-chain TWAP visibility lowers impact; hidden flow
   pays more.

## Where to next in the corpus

- `2505.17388v1` - an OU/Levy OFI-impact model on Chinese index futures; a second impact normalisation to compare
   against the paradox pair.
- `2307.02375v2` - Bayesian change-point regimes in order flow; regime switching behind apparent persistence.
- `2112.13213v4` - cross-impact of OFI in equities; extends single-asset OFI to cross-asset effects.
- `2601.23172v2` - a unified order flow, impact and volatility theory; a rival to the paradox pair.
- `1604.07556v1` - propagator impact models; adds transient impact decay to the concavity story.
- `2310.14144v2` - toxic flow and unwind scheduling; links toxicity to execution.
- `2510.08085v1` - a queue-reactive Hawkes LOB simulator; a testbed for persistence.
- `2606.23070v1` - AMM, MEV and oracle price discovery; extends discovery to on-chain venues.
- `2411.13564v1` - insider detection; the empirical counterpart to Kyle theory.
- `2109.13905v1` - GAN order-flow price simulation; a realism check for synthetic flow.
