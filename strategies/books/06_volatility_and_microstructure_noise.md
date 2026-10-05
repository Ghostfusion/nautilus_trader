# Volatility and Microstructure Noise: Estimators, Rough Volatility, and Weak Return Predictability

Date: 2026-10-05. Revision 1.

This brief covers a corpus slice of 399 harvested records spanning 1999-2026, of which 61 carry a journal reference and 76 candidates were screened across two merged themes. Twelve papers were read in depth: high-frequency noise estimation and liquidity, the Epps effect and bid-ask bounce, realized-volatility estimator comparison, rough volatility versus long memory, Hawkes-based volatility, and regime-conditional forecasting.

## What this covers

The slice is about what high-frequency prices actually contain and what can be inferred from them. It covers the size and economics of additive microstructure noise, the way bid-ask bounce and trade asynchrony manufacture the Epps effect and signature-plot decay, the ranking of noise-robust realized-volatility estimators as a function of the assumed data-generating process, whether volatility is genuinely long-memory or merely rough, Hawkes point-process volatility, and the weak, state-dependent value of return prediction. The single most consequential finding is that naive fine-grid realized volatility is not just noisy but divergent under additive noise, so estimator choice and sampling frequency are first-order modeling decisions rather than implementation details.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category | 399 |
| Years spanned | 1999-2026 |
| Records with a journal reference | 61 |
| Candidate papers screened | 76 (two merged themes) |
| Papers read in depth | 12 |

The selection rule kept papers with quantitative claims or closed-form results on microstructure-noise estimation and liquidity, the Epps effect and bid-ask bounce, realized-volatility measurement and estimator comparison, and volatility forecasting and regimes, dropping purely descriptive or off-topic entries.

## The short answer

1. On NYSE data the average noise standard deviation is 0.050% (5 bps) against a fundamental volatility of 34.8%, giving a noise-to-signal ratio of 36.6% `0906.1444v1`.
2. As the sampling interval shrinks, the noise-to-signal ratio tends to 1 and realized volatility diverges as 2 n a^2, so raw fine-grid RV is unusable rather than merely imprecise `0906.1444v1`.
3. Intraday bid-ask spread explains most of the cross-sectional noise variation, with adjusted R2 = 63.44%, while price level explains most among daily measures at 27.57% `0906.1444v1`.
4. Trade asynchrony is the dominant cause of the Epps effect down to about 10 minutes; overlap compensation restores cross-correlation almost completely down to about 3 minutes `1009.6157v1`.
5. Bid-ask bounce and random trade delays can generate signature-plot decay and the Epps effect with no change in true co-movement, and the noise strength S = D_micro/D_true = 1/(1+B(0)) is independent of the inter-trade-interval distribution `1202.3915v1`.
6. Volatility is rough with Hurst estimates between 0.08 and 0.2, and the log-volatility autocovariance is linear in Delta^(2H) rather than a power law, which makes classical long memory spurious `1410.3394v1`.
7. A rough-volatility forecast beats HAR and AR out of sample: SPX 20-day ratios are RFSV 0.606, HAR 0.656, AR(10) 0.694, AR(5) 0.764 `1410.3394v1`.
8. Nearly unstable heavy-tailed Hawkes order flow has a scaling limit that is a rough fractional process with H = alpha - 1/2, giving roughness a microstructure microfoundation `1504.03100v1`.
9. Estimator ranking is data-generating-process dependent: the Fourier estimator wins on spot volatility and average MSE in a Queue-Reactive simulation, while MLE is preferred on empirical NYSE data `2202.12137v2` and `0906.1444v1`.
10. Return prediction stays weak and non-robust: full-sample correlation 0.0449, hit ratio 53.49%, and a net Sharpe of 0.255 with bootstrap p = 0.182 `2606.09478v1`.

## 1. How large the noise is and why it exists

Using NYSE TAQ data from June 1 1995 to December 31 2005, a stock-day averaging 653 stocks per day and 910 transactions per stock, with at least 200 trades required, gives an average noise standard deviation of 0.050% (5 bps), a fundamental volatility of 34.8%, and a noise-to-signal ratio of 36.6% `0906.1444v1` (p.17). The closed-form NSR = 2a^2 / (sigma^2 Delta + 2a^2) implies that as Delta tends to zero the ratio tends to 1 and realized volatility diverges as 2 n a^2 `0906.1444v1` (Eq. 20, p.20). The authors note that noise is identifiable only under extra assumptions, since an Ito noise makes signal and noise indistinguishable `0906.1444v1` (p.9).

Noise is not merely statistical. Intraday spread explains most of its variation across stocks (adjusted R2 = 63.44%), price level explains most among daily measures (27.57%), and all measures together reach adjusted R2 = 72.55%, while the NSR counterpart reaches 31.74% `0906.1444v1` (pp.20-21). A common factor in stock-level noise has an average slope of 1.027 (t = 1.95), a median of 0.472, and is positive for 62.8% of names `0906.1444v1` (p.29). Most consequentially, noise is priced: sorting into quintiles on the noise parameter yields a highest-minus-lowest return of 44 bps per month, or 5.3% per year, monotonic across quintiles `0906.1444v1` (p.32).

## 2. Bounce, asynchrony, and the Epps effect

A decomposition of Pearson correlation into fractional time overlap (asynchrony) and tick-size discretization loss, validated on GARCH(1,1) data with 7.2E6 points and mean inter-trade waiting times of 15 and 25 seconds, restores the correlation almost completely down to about 3 minutes against a target saturation of 0.9 `1009.6157v1` (Fig. 5, p.6). Asynchrony dominates down to about 10 minutes, where the remaining Epps effect is on average less than 3% of the 30-minute saturation correlation, and the identified causes can contribute up to 75% of the effect for low-priced stocks `1009.6157v1` (pp.6-7). Below 3 minutes the compensation is unreliable because lead-lag dominates `1009.6157v1` (p.7).

A purely analytic model combining an ARFIMA long-memory component, a Bernoulli bid-ask bounce sign with distortion probability q, fat-tailed amplitudes, and non-Poisson trade times shows that bounce alone creates the short-lived negative autocorrelation that constitutes noise, with strength S = D_micro/D_true = 1/(1+B(0)) independent of the inter-trade-interval distribution `1202.3915v1` (Eqs. 45-48, pp.15-16). In the same model the Epps effect follows from random delays between the two assets' trade instants, as the overlapping interval shrinks when Delta goes to zero `1202.3915v1` (Assertion 3, p.19). A bivariate marked Hawkes model of Bund and Bobl futures reproduces both features endogenously: closed-form mean signature plots and a cross-correlation curve fit the empirical plots `1101.3422v1` (Fig. 7, p.22), and long-run variance is minimized when the mean-reversion strength equals 1/3, with empirical means of 0.29 for Bund and 0.36 for Bobl `1101.3422v1` (Fig. 8, p.23).

## 3. Estimator ranking depends on the data-generating process

A Queue-Reactive limit-order-book simulator calibrated on MSFT (theta = 0.6, theta_reinit = 0.85, reference spot variance 1.0387E-8) ranks integrated-volatility estimators differently by criterion: pre-averaging is best for bias while the Fourier estimator is best on average MSE, with unified best for mid and micro price and alternation best for trade price individually `2202.12137v2` (Tables 15-16, pp.35-36). For spot volatility the Fourier estimator is best in both bias and MSE for all three price series `2202.12137v2` (Tables 17-18). The same study finds estimator choice matters for execution: against an empirical VWAP cost-variance benchmark of 1.397, Fourier gives 1.235 and regularized 1.234, two-scale 1.030, and pre-averaging kernel 1.563 (worst), and the Almgren-Chriss formula tends to underestimate implementation-shortfall variance `2202.12137v2` (Table 14, p.30).

Empirically the ranking is not the same. On NYSE data the Ait-Sahalia-Mykland-Zhang MLE is robust to stochastic volatility, jumps and random sampling and is chosen as the baseline, whereas two-scale realized volatility is biased upward at coarse sampling, returning 1.26E-6 at 30 seconds against a true 1E-6 `0906.1444v1` (pp.12-14). Contradicting the standard divergence result, realized volatility for the Nikkei 225 index from May 2006 to December 2009 decreases as sampling frequency rises, opposite both to independent additive noise and to individual Japanese stocks, so the i.i.d. noise model does not apply to that index `1703.09386v1` (Fig. 4, p.6). Its standardized returns are nearly unit variance at all frequencies, with kurtosis 2.42 (morning) and 2.86 (afternoon) against 3, and sixth moment 9.17 and 11.6 against 15 `1703.09386v1` (Table 1, p.7).

## 4. Rough volatility against spurious long memory

Estimating log-volatility smoothness from the scaling of moments yields Hurst exponents of 0.125 for DAX, 0.082 for Bund, 0.142 for S&P and 0.139 for NASDAQ, all between 0.08 and 0.2, with subsample splits between 0.06 and 0.20 and higher values in the crisis half `1410.3394v1` (pp.9-13). A simulation with true H = 0.14 returns an estimated 0.16 using 1-hour uncertainty-zone windows and 0.18 using daily realized variance, confirming an upward bias from longer windows `1410.3394v1` (p.21). The autocovariance of log-volatility is linear in Delta^(2H) rather than a power law in Delta, so the classical long-memory reading is spurious `1410.3394v1` (Sec. 3.2 and 4). The proposed rough fractional stochastic volatility model, a fractional Ornstein-Uhlenbeck process with H below 1/2, beats HAR, which beats AR, at all horizons on the ratio P = MSE over the variance of log-variance: SPX 1-day gives 0.313, 0.314, 0.317, 0.318 and SPX 20-day gives RFSV 0.606, HAR 0.656, AR(10) 0.694, AR(5) 0.764 `1410.3394v1` (Table 5.1, p.27).

The theoretical counterpart shows that for a nearly unstable Hawkes process whose regression kernel has a power-law tail with exponent alpha in (1/2,1), the rescaled law converges to an integrated fractional Cox-Ingersoll-Ross process with H = alpha - 1/2 below 1/2, while a light-tailed kernel instead gives a Brownian CIR limit `1504.03100v1` (pp.1-4). This is presented as the first agent-based explanation of roughness, where heavy tails (persistence) paradoxically yield a more irregular limit through an aggregation effect `1504.03100v1` (p.4). The dispute is explicit: the analytic bounce model builds long memory into its ARFIMA component `1202.3915v1` (p.4), whereas the rough-volatility account argues the same persistence is an artifact of the fractional scaling `1410.3394v1`.

## 5. Hawkes order flow as a volatility and liquidity state

Estimating a symmetric marked Hawkes model on IBM and CVX tick data from 2008 to 2011 gives, for example in the 01/03/2011 window, mu = 0.1080, alpha_s = 0.6577, alpha_c = 0.9956, beta = 2.2921 and eta = 0.1241 `1907.12025v1` (Table 7, p.19). Across monthly windows the self-excitation alpha_s ranges about 0.49-0.97 and the cross-excitation alpha_c about 0.86-1.38, with alpha_c frequently near or above 1, consistent with the nearly unstable regime that the theory maps to roughness `1907.12025v1` (pp.19-20). The linear mark-impact slope eta averages about 0.2 and collapses to about 0.01-0.02 on stress days such as September 29 2008, May 6 2010 and August 9 2011, against averages of 0.11-0.23 `1907.12025v1` (pp.19-20). The resulting Hawkes volatility tracks two-scale realized volatility in trend with a smaller standard error in simulation, and the i.i.d.-mark simplification is nearly identical to the full formula in practice; intraday U-shape and the Flash Crash surge are captured with 10-minute updates `1907.12025v1` (Figs. 10-12, pp.23-24).

Taken together with the two-futures signature-plot fit `1101.3422v1` and the scaling limit `1504.03100v1`, the picture is that endogenous, mutually exciting order flow with metaorder splitting generates both the observed short-horizon mean reversion and an apparent low Hurst exponent without either being imposed. The practical consequence is that branching ratio and mark-impact slope are directly estimable per instrument and can serve as liquidity and toxicity state variables, while a simulator generating nearly critical Hawkes flow reproduces roughness without hardcoding it.

## 6. Forecasting: regimes, implied volatility, and weak returns

On S&P 500 and VIX daily data from 2000 to 2023 (2252 points), out-of-sample MAE and RMSE are 1.56E-3 and 2.39E-3 for GARCH, 1.24E-3 and 1.55E-3 for LSTM, 1.01E-3 and 1.31E-3 for LSTM-GARCH, and 1.02E-3 and 1.30E-3 for LSTM-GARCH with VIX `2407.16780v1` (Table 9, p.23). Adding VIX improves on GARCH by 34.62% MAE and 46.03% RMSE, on LSTM by 17.74% and 16.13%, but relative to LSTM-GARCH it is -0.99% MAE and +0.76% RMSE, with Mann-Whitney p below 0.001 against GARCH and LSTM but p = 0.257 against LSTM-GARCH `2407.16780v1` (Table 11, p.23). By volatility quartile the VIX-augmented model is best in the lowest and highest quartiles while LSTM-GARCH is best in the middle ones `2407.16780v1` (Table 12, p.24).

On CSI 300 high-frequency data over 2005-2023, with a walk-forward out-of-sample window from 2014-12-17 to 2023-05-26, regime augmentation improves the HARQ baseline: with one-month re-estimation the logged MSE is 0.267902 versus 0.282423 (Diebold-Mariano p = 0.0013) and the Mincer-Zarnowitz R2 is 0.5398 versus 0.5215, with the strongest gains in high-volatility periods `2606.09478v1` (Tables 9-11, pp.28-29). Return prediction in the same study is weak: full-sample correlation 0.0449 with a 53.49% hit ratio, high-volatility correlation -0.0617 (p = 0.1928), and low-volatility correlation 0.0638 with a 53.77% hit ratio `2606.09478v1` (Table 5, p.22). The economic implementation, net of 5 bp costs, gives a low-volatility-gated weekly signal an annualized return of 1.93%, annualized standard deviation 7.56%, maximum drawdown -14.52%, Sharpe 0.255 and Sortino 0.394, neutral on 66.77% of days across 163 trades, against buy-and-hold at -1.11%, 23.38%, -50.60% and Sharpe -0.047 `2606.09478v1` (Table 6, p.25). The Sharpe bootstrap gives p = 0.182 and a confidence interval of [-0.683, 0.599], so the economic edge is not statistically strong `2606.09478v1` (Table 7, p.26), though event windows show 2015-16 at +10.24% versus -39.38% for buy-and-hold and 2021-23 at -0.12% versus -30.11% `2606.09478v1` (Table 8, p.27).

A single-day TSLA study reconstructs a realized local volatility surface from a Stick-Breaking Gaussian Mixture Model and finds realized local volatility about 20% higher than the 1-day-to-expiry implied volatility near spot, with a counterfactual scenario at spot 400 implying 123% conditional annualized volatility and a 95% credible interval of [100%, 147%] `2504.15626v2` (pp.15-17).

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| Fine-grid RV diverges; NSR 36.6% | data layer | Use noise-robust estimator or sparse sampling |
| Spread explains 63.44% of noise | risk layer | Proxy noise by effective spread |
| Asynchrony biases correlations down | execution layer | Use overlap-corrected covariance estimators |
| Bounce can fake the Epps effect | data layer | Do not read trade autocorrelation as signal |
| Estimator ranking is DGP-dependent | execution layer | Select estimator per price series |
| Rough-vol kernel beats HAR and AR | docs/usermanauls/intraday-systematic | Use a horizon-scaled memory window |
| Hawkes branching is a state variable | risk layer | Track branching ratio and mark impact |
| Regime probability aids vol forecasts | docs/usermanauls/intraday-systematic | Gate signals on high-vol probability |
| Return prediction is weak | docs/usermanauls/production-operations | Evaluate net of costs with bootstrap CI |
| Realized local vol 20% above implied | docs/usermanauls/options | Track both surfaces for the risk premium |
| Signature-plot and Epps diagnostics | docs/concepts/backtesting | Run both per instrument before trusting |
| Noise is a priced common factor | docs/usermanauls/factor-portfolio | Add a market-wide liquidity state |

## Caveats

The evidence does not establish that any of these rankings transfer to this platform's venues, instruments, or sampling conventions. The central noise figures come from NYSE 1995-2005, a period spanning two tick-size reductions, so the 5 bps noise level and 36.6% noise-to-signal ratio are historically specific rather than universal. The estimator comparison is entirely simulation-based, so its Fourier-best conclusion holds only to the extent that the Queue-Reactive simulator is a faithful data-generating process. The analytic model is purely synthetic with no empirical calibration, and the local-volatility surface rests on one stock on one trading day, which is illustrative rather than evidential. Only one study applies transaction costs, and the resulting return strategy Sharpe of 0.255 is not statistically distinguishable from zero. Long memory is contested in the slice, with one paper building it in and another arguing the same persistence is spurious. Several findings are estimated once per asset or on two names, so parameter stability and out-of-sample robustness are largely untested.

## Papers read in depth

- `0906.1444v1` - High frequency market microstructure noise estimates and liquidity measures (2009). Quantifies noise at 5 bps and 36.6% noise-to-signal, ties it to spread and price level, and shows it is priced.
- `1009.6157v1` - Statistical causes for the Epps effect in microstructure noise (2010). Separates asynchrony from tick rounding and restores correlation down to about 3 minutes.
- `1101.3422v1` - Modeling microstructure noise with mutually exciting point processes (2011). Closed-form signature plots and Epps curve from a bivariate Hawkes fit on Bund and Bobl.
- `1202.3915v1` - A simple microstructure return model explaining microstructure noise and Epps effects (2012). Analytic and synthetic: bounce and delay suffice, but no market calibration.
- `1703.09386v1` - Analysis of Realized Volatility for Nikkei Stock Average on the Tokyo Stock Exchange (2017). Documents a case where the standard additive-noise model fails at index level.
- `2202.12137v2` - From Zero-Intelligence to Queue-Reactive: LOB modeling for high-frequency volatility estimation and optimal execution (2022). Simulation-only estimator ranking with an execution cost-variance twist.
- `1410.3394v1` - Volatility is rough (2014). Hurst near 0.1 and a rough-vol forecast that beats HAR and AR out of sample.
- `1504.03100v1` - Rough fractional diffusions as scaling limits of nearly unstable heavy tailed Hawkes processes (2015). The microfoundation linking critical order flow to roughness.
- `1907.12025v1` - Marked Hawkes process modeling of price dynamics and volatility estimation (2019). Branching and mark-impact estimates usable as liquidity state.
- `2407.16780v1` - The Hybrid Forecast of S&P 500 Volatility ensembled from VIX, GARCH and LSTM models (2024). VIX helps against GARCH and LSTM but not against LSTM-GARCH.
- `2504.15626v2` - Realized Local Volatility Surface (2023/2025). Single-day TSLA surface with credible intervals; illustrative, thin evidence.
- `2606.09478v1` - Volatility Forecasting and Return Prediction under Market Regimes (2026). Regime-augmented volatility helps, directional returns stay weak.

## Where to next in the corpus

- `2407.17401v3` - Bid-ask spread estimators with serial dependence: spread point estimation may refine the noise proxy used here.
- `1908.02847v2` - Instantaneous volatility invariant from volume, spread and LOB: a candidate composite noise state if the method can be reconstructed.
- `2606.29591v1` - Bounce sign and magnitude decomposition on SPY: tighter bounce accounting than the analytic model provides.
- `2503.08693v1` - Liquidity-adjusted ARMA-GARCH: bridge from volatility estimation to portfolio construction under liquidity.
- `1310.1601v1` - Random matrix analysis of volatility correlations: cross-sectional structure of the common noise factor.
- `0908.1555v2` - Leverage causes fat tails: an econophysics agent model on the tail-shape mechanism behind the observed kurtosis.
- `2311.04727v2` - Crypto-winter LSTM plus rough volatility: rough-volatility transfer to a different asset class.
- `2512.12250v1` - Stochastic volatility plus LSTM for S&P 500: a competing learned-volatility specification.
- `2401.02049v1` - Bitcoin HV/GARCH/IV comparison: implied-versus-realized comparison on a crypto venue.
- `2406.19405v1` - Electricity spot price stochastic volatility: a regime-switching analogue outside the financial-asset set.
