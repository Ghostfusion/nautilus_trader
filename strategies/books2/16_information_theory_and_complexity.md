# Information theory, entropy and complexity: a parameter-free roughness statistic with an exact null

Date: 2026-10-06. Revision 1.

This brief covers the arXiv `q-fin.GN` records that treat information theory, entropy and complexity as measurement instruments rather than as metaphors. The slice holds 61 records spanning 2007-2026, 11 of them with a journal reference, and 16 published in 2021 or later. Ten were read from the PDFs. The subject is tooling: transfer-entropy-style information flow, entropy-based predictability estimators, Hurst and fractal-dimension estimators, and - the part that decides whether any of it is usable - how those estimators behave on short, noisy, non-stationary samples. It is written for an operator running a Rust/Python event-driven algorithmic trading platform across equities, futures, FX, crypto, options and prediction markets, who wants to know which of these statistics can be computed on a backtest window without lying.

## What this covers

The arc runs from entropy as an inequality/inefficiency accounting device (`1301.5504v1`, `1812.02371v1`) through entropy and fractal-dimension estimators applied to price series (`1206.1007v1`, `2205.00104v1`, `2310.18903v3`) to the recent, sharper question of estimator behaviour under finite samples and non-stationarity (`2512.02352v3`, `2512.23596v2`). The single most consequential finding is in `2512.02352v3`: the forward visibility horizon of a path is exactly a first-passage time, its i.i.d. survival law is exactly `Pr[L+ >= k] = 1/k`, and the roughness exponent it estimates is `theta(H) = 1 - H` (p.5, p.7). That gives a rank-invariant, parameter-free statistic with a closed-form null - and a measured finite-sample bias that says when not to trust it (p.8).

## The corpus slice

| Metric | Value |
|---|---|
| Records matching the category | 61 |
| Years spanned | 2007-2026 |
| Records with a journal reference | 11 |
| Candidate papers screened | 40 |
| Papers read in depth | 10 |

Selection rule: the shortlist was filtered for papers whose contribution is a computable estimator or a measured estimator failure mode, and the ten read are the ones that report a formula, an ensemble size and a page-anchored error figure.

## The short answer

1. The forward visibility horizon of a path is exactly its first-passage time, with an exact i.i.d. null `Pr[L+ >= k] = 1/k` and infinite mean and variance, so roughness must be read off a tail exponent, not a moment (`2512.02352v3`, p.5).
2. That tail exponent satisfies `theta(H) = 1 - H` for fractional Brownian motion, and recovers it within one cross-replicate standard deviation for `H <= 0.2` at `T = 2^16` (`2512.02352v3`, p.7, p.8).
3. The same statistic carries a positive finite-size bias that grows with `H`: `+0.075` at `H = 0.5` and `+0.323` at `H = 0.9`, so smooth paths need much longer series than rough ones (`2512.02352v3`, p.8).
4. It still separates rough Bergomi from classical Heston by `0.3355` in exponent units (combined Monte-Carlo SE `0.00024`), but rough Bergomi from FIGARCH by only `0.0580` (`2512.02352v3`, p.13).
5. A DFA scaling range is a linear function of series length and fit quality, `lambda = (a u + a0) L + b` with `u = 1 - R2`, so the usual `tau_max ~ L/4` rule is a choice with a quantified rejection rate (`1206.1007v1`, p.3, p.4).
6. In that DFA model the scaling range grows only 3-10% for a `0.1` change in Hurst exponent, so a short window cannot resolve `H` precisely regardless of the estimator (`1206.1007v1`, p.7).
7. Entropy is a workable market-level volatility estimator: the cross-sectional intrinsic entropy is at least ten times more variable than index volatility estimators, and index beta against it is 50-90% lower (`2205.00104v1`, p.1).
8. Efficiency has a clean information-theoretic definition, `Eff(X|Y) = H(X|Y) / H(X)`, and inefficiency has two separable sources: predictability and wrong pricing (`1812.02371v1`, p.6, p.11).
9. Model complexity and training-window length must be chosen jointly under non-stationarity; the prediction-error bound adds a total-variation drift term, and the procedure improves out-of-sample R2 by 14% and Sharpe from 0.30 to 0.33 (`2512.23596v2`, p.3, p.15).
10. Mutual information detects nonlinear dependence that Pearson correlation misses, and it changes the network verdict: the MI minimum spanning tree shows power-law weighted degree where the correlation tree shows log-normal (`1711.06185v3`, p.14).
11. Entropy decomposes hierarchically, recovers the Theil index as a special case, and imposes a balance constraint `sum_j p_j (H_j - H^j) = 0` between cash-inflow and cash-outflow diversities (`1301.5504v1`, p.2, p.4).
12. Price-probability forecasts are argued to be capped near Gaussian accuracy because volatility predictions need second-order trade-moment models that do not exist (`2309.02447v2`, p.3, p.6).

## 1. A rank-invariant roughness statistic with an exact null

`2512.02352v3` is the anchor of the slice. It studies the horizontal visibility graph (HVG) of a path and isolates the forward visibility horizon `L+(t)`, the largest `k` such that `t+k` is HVG-visible from `t` and `x_{t+k} >= x_t` (p.4). The load-bearing result is Lemma 1: for a path with no ties, each uncensored horizon equals the classical first-passage time `tau+(t) = inf{k >= 1 : x_{t+k} >= x_t}` (p.4). Terminal non-crossings are treated as right-censored, which is a real finite-sample issue rather than a technicality.

With that identity, Proposition 2 gives the exact i.i.d. survival law `Pr[L+ >= k] = 1/k`, which is Renyi's record statistic; the mean and variance both diverge (p.5). This is why the paper estimates a power-law tail exponent `theta` instead of a mean, and why any mean-horizon estimator is ill-posed. Corollary 3 then combines the identity with discrete-grid persistence theory for fractional Brownian motion to predict `theta(H) = 1 - H` (p.7).

The numerical validation uses `N = 10,000` fBm paths of length `T = 2^16` sampled by Davies-Harte (p.7). The estimator is a Hill maximum-likelihood fit with a Clauset-Shalizi-Newman threshold and a stationary block bootstrap, mean block length `ceil(T^(1/3))`, `B = 200` resamples (p.6). Table 1 reports the recovery: at `H = 0.1`, `theta-hat = 0.900` with `sigma = 0.013` and bias `-0.0003`; at `H = 0.2`, `0.810` with bias `+0.010`; at `H = 0.5`, `0.575` with bias `+0.075`; at `H = 0.9`, `0.423` with bias `+0.323` (p.8). The bias is the honest part: it decays with `T` (at `H = 0.1` it is `+0.078` at `T = 2^12` and `-0.010` at `T = 2^18`, p.17), but for `H >= 0.7` no length below `T = 2^18` brings it under `0.1` (p.16).

Control processes confirm the null is right in practice: i.i.d. uniform gives `theta-hat = 1.021` and i.i.d. Gaussian `1.024` against the asymptotic `1.00` (p.9), and the empirical i.i.d. null at window `W = 1024` has mean `1.166` with a 2.5% quantile of `1.017` (p.18). On the variance trajectory, rough Bergomi with `H = 0.1` gives `0.8989`, classical Heston `0.5635`, GARCH(1,1) `0.7234`, and FIGARCH(`d = 0.4`) `0.8409` (p.12). The rough-versus-Heston gap is `0.3355` (combined SE `0.00024`); the rough-versus-FIGARCH gap is `0.0580` (combined SE `0.00020`), which the author flags as too small to falsify long-memory volatility on samples shorter than about ten years of daily data (p.13). Applied to daily FRED VIX from 2000-01-03 to 2026-04-30 (`n = 6651`, p.13), the rolling estimate over 45 four-year windows has mean `0.910` and standard deviation `0.19` (p.14), against an overlapping-window i.i.d. null mean of `1.160`, with Monte-Carlo `p ~ 0.001` (p.15). The paper is careful to call this a diagnostic of an option-implied proxy, not an estimate of the latent variance Hurst index (p.14).

Two robustness results matter for a trading stack. Additive Gaussian noise pushes the estimate toward the i.i.d. null (`0.984` at relative noise `sigma = 1.0`, below `0.02` bias for `sigma <= 0.3 std(x)`, p.17), and random thinning at rates up to 50% leaves the estimate unchanged within Monte-Carlo SE (p.17). Rank-invariance is why geometric Brownian motion gives the same answer as arithmetic Brownian motion (p.9).

## 2. How many points a Hurst estimate needs

`1206.1007v1` attacks the question every DFA user faces: over which box sizes is the power law `F2(tau) ~ tau^(2H)` actually linear? The paper fixes `tau_min = 8` because shorter boxes manufacture artificial autocorrelation, and notes the folk rule `tau_max ~ L/4` (p.3). It then simulates ensembles of `5 x 10^4` series with lengths `5 x 10^2 <= L <= 2 x 10^4` (p.4), and later `5 x 10^2 <= L <= 10^4` for the correlated case (p.5), generated by Fourier filtering for `0.5 < H < 0.9` (p.5).

The scaling range `lambda` (identified with `tau_max` at a chosen confidence level) is a clean linear function of both `L` and `u = 1 - R2`: `lambda(u, L) = A(u) L + B(u)`, with `A` linear in `u` and `B` nearly constant (p.4, p.5). That collapses to `lambda = (a u + a0) L + b` (p.5), and Table 1 gives the fitted coefficients: at `H = 0.5` and 97.5% confidence, `a = 6.02`, `a0 = 0.0034`, `b = -92`; at `H = 0.8`, `a = 6.88`, `a0 = 0.0136`, `b = -100`, with mean absolute error 1.5-2.5% (p.6). The unified form `lambda = ((alpha H + beta) u + alpha0) L + gamma` uses four free parameters, fitted as `alpha = 3.40`, `beta = 4.16`, `alpha0 = 0.0097`, `gamma = -96` at 97.5% confidence (p.7).

The practical content is in two numbers. First, negative `lambda` from the fitted formula means no scaling range exists at that confidence level and series length - the paper says so explicitly (p.5). Second, the scaling range increases by only 3-10% for every `delta H = 0.1` (p.7, Eq. 9). A Hurst estimate on a few hundred points therefore carries irreducible resolution error, which is the same lesson `2512.02352v3` reaches from the other direction: short windows and smooth paths are the hard regime.

## 3. Visibility-graph topology on a real futures panel

`2310.18903v3` converts WTI, Brent and Shanghai (SC) crude oil futures prices into visibility graphs across daily, 5-minute, 15-minute and 30-minute frequencies, from 2018-03-26 to 2023-07-20 (p.3). Frequencies are chosen at 5/15/30 minutes specifically to suppress noise (p.3). The degree distributions show power-law tails with exponent `alpha` fluctuating between 2 and 3.5 (p.9). The interesting part is the regime dependence: in the pre-COVID sub-sample all three markets sit at `alpha = 3.5`, and after the COVID outbreak every value falls; over the whole sample all exponents drop below 3, which the authors read as more frequent extreme price moves (p.9).

Global clustering coefficients hover around `0.7` and peak at `0.7442` (p.10), and the VGs satisfy the small-world relation `L(N) ~ ln N` (p.11). Assortativity is where the markets separate. During the pandemic sub-sample the SC daily assortativity coefficients fall to `0.0205` and `0.0611`, essentially zero, while WTI and Brent show their lowest values after the Russia-Ukraine conflict (p.10). The lesson for a backtest is that graph topology is a noisy but regime-sensitive summary, and the choice of sampling frequency changes it materially.

## 4. Entropy as a market-wide volatility estimator

`2205.00104v1` defines the cross-sectional intrinsic entropy (CSIE) from daily OHLC and volume for every symbol traded on NYSE and NASDAQ, weighting each symbol's traded value into a market-wide entropy (p.4, p.8). The headline claim is that CSIE variance is consistently at least one order of magnitude above the variance of index volatility estimators, for every time interval and rolling window tested, and that it is at least ten times more sensitive to market changes (p.1, p.15).

Index beta against CSIE is 50-90% lower than the market's own volatility risk, and lower on shorter intervals: over 90% lower at 30 days and over 75% at 60 days (p.19). Correlation between CSIE and index estimators rises with the window, from `0.445` for close-to-close at 30 days with a 5-day moving average up to `0.895` at 5295 days (p.17). The data cover `5295` reference points from 2001-01-01 to 2022-01-21 (p.9), with a single NYSE file holding `3562` symbols (p.4). One structural caveat the paper states plainly: the number of traded symbols is itself non-stationary, from about 1000 in 2001 to over 3500 in 2022, and is treated as the number of microstates (p.5).

## 5. Efficiency as a conditional-entropy ratio

`1812.02371v1` replaces the binary efficient-market debate with a measure. Efficiency of system `X` relative to information `Y` is `Eff(X|Y) = H(X|Y) / H(X)` (p.6), where `H` is Shannon entropy in bits. A fully efficient system has `H(X|Y) = H(X)` and mutual information `M(X, Y*) = 0`; a fully inefficient one has `H(X|Y) = 0` and `M(X, Y*) = H(X)` (p.8). The same expression is derived from Kelly's optimal-growth result, where maximal capital growth is exactly the mutual information `G_max(X|Y_i) = H(X) - H(X|Y_i)` (p.9), and from Fama's informational definition, which is what makes it more than a restatement.

The extension to unfair quotes splits inefficiency into two nearly independent sources: predictability, which lowers `H(X|Y)`, and wrong pricing, which raises the denominator `H(q)` (p.11). The coin-tossing illustration gives the shape of the predictability effect: efficiency is still around 50% when the outcome can be predicted with nearly 90% accuracy, a 10% departure from unpredictability moves efficiency by less than 3%, and that 3% is itself a 3% mean return per toss with no bankruptcy risk (p.14). The message for signal research is that a naive accuracy number overstates inefficiency, while the entropy ratio is directly comparable to achievable growth.

## 6. Complexity must be chosen with the training window

`2512.23596v2` formalises why more data does not always help. Under non-stationarity the joint distribution `P_tau` of predictors and returns drifts, so a longer window imports stale regimes. The prediction-error bound (Theorem 3.1) decomposes excess risk into model misspecification, local Rademacher statistical uncertainty, a `log(1/delta)/n` term, and a new term `M^2 max_tau TV(P_tau, P_t)`, the largest total-variation shift between any training period and the prediction date (p.15). A matching lower bound shows the drift term is not a proof artifact: an error of order `V` is unavoidable over a class of environments with drift magnitude `V` (p.18).

In a stylised example the optimal window scales as `k* ~ eta^(-1/2)` for a linear class and `k* ~ eta^(-3/5)` for a kernel class, with `eta` the drift severity; the kernel class wins when `eta = O(gamma^5)` and the linear class wins when `eta >> gamma^5` (p.18). Empirically, on 17 US industry portfolios from 1990-2016, an adaptive procedure lifts out-of-sample R2 to `0.049`, a 14% improvement over the best fixed-window benchmark, and monthly mean-variance Sharpe from `0.30` to `0.33` without higher turnover (p.3). The gains concentrate in the three NBER recessions: `0.027` in 1990, `0.125` in 2001, and `0.041` in 2008 (p.4). The empirical illustration uses a 64-month recent window against all-history training (p.10).

## 7. Mutual information for nonlinear dependence

`1711.06185v3` compares minimum spanning trees built from mutual information against trees built from Pearson correlation, on Brazilian equity returns sampled every 15 minutes across two 2015-2016 periods: 3888 returns in the Rousseff period (`-42%` index return) and 3969 in the transition period (`+50%`) (p.8). Mutual information is estimated non-parametrically by Gaussian kernel density estimation, and converted to a comparable coefficient `l_ij = sqrt(1 - exp(-2 I_ij))` (p.4, p.5). Significance is checked at `alpha = 0.01` by a chi-square test for MI and a t-test for correlation; `40.77%` of correlations are insignificant in the first period and `52.99%` in the second, while all MI estimates are significant (p.9).

The network verdicts differ. The MI trees have smaller mean distance and, in the more turbulent second period, power-law weighted-degree tails with fitted exponent `2.39` against `4.19` for the correlation tree (p.14). The robustness coefficient rises 27% in that period, and the mean MST distance falls 18% (p.15). A portfolio of the 20 most central assets under the MI tree would have returned 40% more than the correlation-based analogue in the transition period (p.14). The caveat is sample size: the MI estimates are averaged over each period and estimated by kernel density on a few thousand points.

## 8. Flow entropy, hierarchy and entropy balance

`1301.5504v1` models cash flows between `N` agents as probabilities that a random currency unit sits in a given flow, `H = -sum p log2 p` (p.2). Its structural result is the subdivision identity `H = H_g + sum_j p_j H_j`, which recursively yields inequality measures at every level of aggregation (p.2). Grouping agents into human, corporate and government sectors recovers a measure related to the Theil index as a special case (p.4).

The constraint is the interesting one: in a steady state, `sum_j p_j (H_j - H^j) = 0`, where `H_j` is the entropy of an agent's inflows and `H^j` of its outflows (p.4). No economy can have every agent more certain about inflows than outflows; entropy-increasing and entropy-decreasing agents must coexist. The three-agent worked example makes it concrete with inflow entropies `0.8617, 0.8048, 0.5275` and outflow entropies `0.4690, 0.8813, 0.8813` (p.7). For a trading firm this is a decomposition of fill and payment concentration, and a reminder that concentration measures are only meaningful relative to the flows they are drawn from.

## 9. Entropy scores in anger, and a ceiling on price-probability forecasts

`2308.02914v2` builds a correlation graph over 802 global stocks with 1847 observations from 2004-10-27 to 2019-03-15, keeps the top 1% of correlations, and trains a graph autoencoder whose anomaly score is Tsallis entropy (p.4). The structural finding is a sparsity paradox: in the top 1% of correlations the number of edges falls from 455 before the crisis to 294 during it, and the share of nodes with no edges rises from `39.52%` to `59.97%`, even though average correlation across all assets rises (p.5). Clustering falls from `0.30` to `0.26` and recovers to `0.37` (p.5), and the anomaly counts differ across periods at p-values from `1.57e-4` to `6.65e-10` (p.6). The paper states its own limit: there is no ground truth for the detected anomalies (p.6).

`2309.02447v2` is a theory paper with no data, and is included for its falsifiable claim. It derives market-based averages and volatilities of price and return from the statistical moments of trade values and volumes, e.g. price volatility depends on the second moments of trade value and volume and on their correlation (p.6). Because volatility forecasts therefore require models of second-order economic variables - sums of squares of trade values - and no such second-order macroeconomic theory exists, the paper argues that price and return probability forecasts are capped near Gaussian accuracy (p.3). This is a limitation statement, not a result to act on, and it should be read as such.

## What this project can take from it

| Finding | Where it lands | What to do |
|---|---|---|
| Forward visibility horizon has an exact i.i.d. null and `theta = 1 - H` (`2512.02352v3`) | `python/nautilus_trader/analysis/statistic.py` | Add a roughness statistic that computes the horizon by monotone stack, fits the tail with Hill-MCS, and reports `theta-hat`, threshold and tail size together. |
| Finite-size bias of the tail exponent grows with `H` (`2512.02352v3`) | `crates/analysis/src/statistic.rs` | Gate the statistic on window length and refuse to emit a Hurst-like number where the bias table says it is unreliable. |
| DFA scaling range is `(a u + a0) L + b` (`1206.1007v1`) | `crates/analysis/src/statistic.rs` | Replace the fixed `tau_max ~ L/4` convention with the fitted scaling-range formula and log the chosen range. |
| Short windows cannot resolve `H` to better than a few percent (`1206.1007v1`) | `python/nautilus_trader/analysis` | Report Hurst-style statistics with an explicit resolution band rather than a point estimate. |
| Visibility-graph topology is regime-sensitive and frequency-dependent (`2310.18903v3`) | `python/nautilus_trader/analysis` | Compute degree exponent, clustering and assortativity as post-backtest diagnostics on the traded instrument and frequency. |
| Cross-sectional intrinsic entropy is far more variable than index volatility (`2205.00104v1`) | `crates/analysis/src/analyzer.rs` | Add a market-wide OHLCV entropy feature as a regime input, computed over all traded symbols per day. |
| Efficiency is `H(X|Y)/H(X)` and equals normalised Kelly growth (`1812.02371v1`) | `python/nautilus_trader/research` | Report a signal's conditional-entropy efficiency alongside accuracy, so capacity is compared on a growth scale. |
| Inefficiency splits into predictability and mispricing (`1812.02371v1`) | `crates/analysis/src/statistics` | Track the two components separately when attributing strategy edge. |
| Complexity and window length must be selected jointly (`2512.23596v2`) | `crates/backtest` | Add a training-window selector that co-varies with model class and records the total-variation drift of each window. |
| Non-stationarity imposes an unavoidable error floor (`2512.23596v2`) | `crates/backtest` | Report the drift term as part of the backtest report instead of hiding it in a fixed lookback. |
| Mutual information detects dependence correlation misses (`1711.06185v3`) | `python/nautilus_trader/analysis/tearsheet.py` | Add an MI-based dependence matrix and clustering view to the tearsheet for multi-asset portfolios. |
| Entropy decomposes hierarchically and recovers Theil (`1301.5504v1`) | `crates/analysis/src/statistics` | Reuse the subdivision identity to report fill and PnL concentration at portfolio, venue and symbol levels. |
| Correlation-graph sparsity is a crisis signal (`2308.02914v2`) | `crates/risk` | Add an edge-count and node-isolation monitor on the top-correlation graph as a risk regime indicator. |
| Volatility forecasts need second-order trade moments (`2309.02447v2`) | `crates/indicators/src/volatility` | Document that volatility features carry an irreducible forecast uncertainty and keep simulation to second moments rather than implying precision. |
| Entropy estimators are robust to thinning but not to noise (`2512.02352v3`) | `crates/data/src/aggregation.rs` | Enforce minimum bar counts before emitting entropy-based features from resampled data. |

## Caveats

Half of this slice is not about tradable instruments. `1301.5504v1`, `1812.02371v1`, `2309.02447v2` and the network papers (`1711.06185v3`, `2308.02914v2`) are about economies, games, firms and cross-sections, not about execution or price series, and no paper here measures transfer entropy or directed information flow on order flow - the closest is undirected mutual information in `1711.06185v3`. `1206.1007v1` and `2512.02352v3` are simulation studies, so their bias tables are synthetic and their empirical checks are single-series (VIX for the latter). `2310.18903v3` is three futures markets over one five-year window, `2205.00104v1` is US equities over one 21-year window with a changing symbol count, and `1711.06185v3` is one emerging market over two consecutive 2015-2016 periods. The slice disagrees with itself on whether entropy is a risk measure or a diversity measure: `2205.00104v1` treats CSIE as volatility while `1301.5504v1` treats entropy as inequality, and the two normalisations are not interchangeable. `2512.23596v2` is the only paper that supplies a lower bound, and even there the guarantee is on prediction error, not on portfolio outcomes. No paper here establishes that any of these statistics forecasts returns; they are measurement and validation instruments.

## Papers read in depth

- `1206.1007v1` - On the scaling ranges of detrended fluctuation analysis for long-memory correlated short series of data (2012). Gives a fitted formula for the usable DFA scaling range as a function of length, fit quality and Hurst exponent; entirely simulation-based and calibrated only up to `L = 2 x 10^4`.
- `2512.02352v3` - First-passage horizons in horizontal visibility graphs: a rank-invariant estimator of path roughness for rough volatility models (2025). Provides the exact i.i.d. null and `theta(H) = 1 - H`, with a measured finite-size bias; the rough-Bergomi result depends on an unproven persistence hypothesis and rough-versus-FIGARCH separation is small.
- `2310.18903v3` - Visibility graph analysis of crude oil futures markets: Insights from the COVID-19 pandemic and Russia-Ukraine conflict (2023). Shows power-law degree tails, high clustering and regime-dependent assortativity in three oil futures; descriptive, with no out-of-sample test and only one sample window.
- `2205.00104v1` - The Cross-Sectional Intrinsic Entropy. A Comprehensive Stock Market Volatility Estimator (2022). Defines a market-wide OHLCV entropy and shows it is far more variable than index volatility; single market family, and the non-stationary symbol count complicates comparability.
- `1812.02371v1` - Quantification of market efficiency based on informational-entropy (2018). Derives `Eff = H(X|Y)/H(X)` from both Fama and Kelly and splits inefficiency into two sources; illustrated only on a coin-tossing game, with no empirical market application.
- `2512.23596v2` - The Nonstationarity-Complexity Tradeoff in Return Prediction (2025). Proves a prediction-error bound with an explicit drift term and an adaptive joint selection procedure; the theory assumes i.i.d. batches and the empirical test is 17 US industry portfolios.
- `1711.06185v3` - Nonlinear dependencies on Brazilian equity network from mutual information minimum spanning trees (2017). Compares MI and correlation spanning trees and finds MI exposes power-law degree tails; two short adjacent periods in one emerging market.
- `1301.5504v1` - Cash Flow Entropy (2013). Builds a hierarchical flow-entropy measure recovering Theil and derives the entropy-balance constraint; purely formal, with no empirical calibration and no credit or time dimension.
- `2308.02914v2` - Anomaly Detection in Global Financial Markets with Graph Neural Networks and Nonextensive Entropy (2023). Uses Tsallis entropy as an autoencoder anomaly score and documents crisis-driven graph sparsification; no ground-truth labels for the anomalies.
- `2309.02447v2` - Economic Complexity Limits Accuracy of Price Probability Predictions by Gaussian Distributions (2023). Argues volatility forecasts need second-order trade-moment models and are therefore capped near Gaussian accuracy; a position paper with no data and no test.

## Where to next in the corpus

- `0909.3890v1` - The Building Blocks of Economic Complexity. The canonical fitness-complexity construction; a lead for turning the network diagnostics here into a ranking rather than a topology summary.
- `1705.02154v2` - Leontief Meets Shannon - Measuring the Complexity of the Economic System. A direct bridge between input-output accounting and information-theoretic complexity, relevant if flow entropy is to be applied to cash flows rather than returns.
- `1607.02481v4` - Inferring monopartite projections of bipartite networks: an entropy-based approach. A method for validating projected networks against an entropy null, which would harden the correlation and MI graphs used above.
- `2604.17166v2` - The Virtue of Sparsity in Complexity. A 2026 counterpoint to complexity-as-virtue; a lead for the joint complexity/window question in `2512.23596v2`.
- `2407.00022v1` - Entropy and Economics. A survey lead for the definitions used loosely across this slice.
- `2512.10121v1` - Workflow is All You Need: Escaping the Statistical Smoothing Trap via High-Entropy Information Foraging. Not read and not a finance paper in the usual sense; a lead on high-entropy sampling design that may bear on how training windows are chosen.
