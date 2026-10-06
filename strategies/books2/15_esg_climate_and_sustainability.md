# ESG, climate and sustainability: the strongest signal is disagreement between raters

Date: 2026-10-06. Revision 1.

This brief reads ten papers from the arXiv `q-fin.GN` (General Finance) harvest, on the ESG, climate and sustainability slice: 79 matching records published between 2010 and 2026, 16 of them with a journal reference. It covers what is measurable and tradeable here - the behaviour of ESG ratings and labels, the pricing of transition risk, carbon markets and permit allocation, divestment dynamics and social tipping, and the data-quality limits that make reported ESG results fragile. It is written for an operator running a Rust/Python event-driven algorithmic trading platform across equities, futures, FX, crypto, options and prediction markets, so each section ends at a place where a data field, a risk limit or a simulation assumption would change.

## What this covers

The slice arcs from econophysics-style permit allocation and carbon-economy modelling (2011-2012) through transition-risk asset pricing and ESG fund performance (2018-2022) to firm-level climate stress testing and rating-disagreement measurement (2024-2026). Its single most consequential finding is that the choice of ESG rating lens is not a measurement detail but a first-order driver of the result: on the identical 200-firm European sample, replacing the CDP climate score with the LSEG environmental pillar score eliminates the flagship-index-membership effect on the disclosure-performance gap while the renewable-energy effect survives, so the detected "greenwashing" is conditional on the rater (`2606.31469v1`, p.1).

## The corpus slice

| Metric | Value |
|---|---|
| Records matching the category | 79 |
| Years spanned | 2010-2026 |
| Records with a journal reference | 16 |
| Candidate papers screened | 40 |
| Papers read in depth | 10 |

Selection rule: the ten records with the strongest combination of a measurable object (a rating, a price, an exposure or an allocation rule), a quantitative result, and a clear operational implication for a trading platform.

## The short answer

1. ESG ratings disagree among themselves far more than credit ratings do, and the disagreement is structure rather than noise: inter-agency correlations frequently fall below 0.60 versus roughly 0.99 for credit ratings (`2606.31469v1`, p.2).
2. The determinants of a firm's disclosure-performance gap flip when the rating provider is swapped, so any ESG signal must be tested per provider before it is trusted (`2606.31469v1`, p.1).
3. Aggregated ESG scores throw away the risk signal: selection on LSEG raw variables explains Energy-sector return volatility with %dev 0.51, and 0.62 in combination with financial factors, versus 0.32 for financial factors alone (`2508.18679v2`, p.13).
4. Simple ESG best-in-class filtering produced small but negative excess returns and negative information ratios across MSCI World regions and thresholds (`2002.07477v2`, p.4).
5. Non-linear screening of the same ESG features produced a positive-screening portfolio that beat the benchmark by 2.76% annualised and best-in-class by 2.94% over Jan 2013-Mar 2018, but the learned rules decayed and required yearly retraining (`2002.07477v2`, p.14, p.16).
6. Higher-ESG socially responsible funds held up better only in the downturn: during 2005-2008 the top-ESG tertile had monthly return -0.32% and Sharpe 0.56, versus 0.30 for the lowest tertile (`1806.09906v2`, p.7).
7. Transition risk is priced in expectations, not in current emissions: a rise in the likelihood of a climate transition raised oil production and lowered the oil spot price, amplified and statistically significant mainly over 2009-2019 (`2410.00902v1`, p.4).
8. At the EU ETS II price cap of 45 EUR/t, direct real-economy losses were about 1.3% of sales and direct bank equity losses about 1.2%, but supply-chain contagion could amplify these by 300% to 4000% (`2503.10644v1`, p.1, p.4).
9. Emission permits can be allocated by a maximum-entropy Boltzmann rule whose single tuning parameter, beta, shifts responsibility between large-population and high-per-capita emitters (`1108.2305v2`, p.16, p.14).
10. A support-vector-regression carbon-price model projected the EU price at 114.7 EUR in 2030 under one Paris target and 203.0 EUR under a tighter one, on very limited training data (`2212.11787v1`, p.5, p.12).
11. ESG scores are definitionally fragile: log volatility fits the risk relation better than raw volatility, and missing ESG booleans are imputed as zero rather than dropped (`2508.18679v2`, p.3, p.4).
12. Aggregation itself removes signal: a model built from raw variables inside a single ESG category outperforms a model that uses all ten pre-aggregated category scores (`2508.18679v2`, p.5).

## 1. ESG ratings disagree, and the disagreement is the measurement

The founding problem of this slice is that ESG is measured, not observed. `2606.31469v1` reports that ESG ratings from different agencies frequently correlate below 0.60, against roughly 0.99 for credit ratings, and traces how this "aggregate confusion" reappears inside a single estimate (p.2). `2508.18679v2` shows why the underlying dataset is hard to use: the LSEG structure is explicitly hierarchical, with 663 raw variables aggregating into 10 category scores, then 3 pillar scores, then one overall score (p.4). The provider expanded its collected raw variables from 400 in 2017 to 600 in 2023, largely as a marketing exercise, which leaves the database sparse and the newly added columns unusable for older years (p.5). For the Energy sector the raw panel of 617 variables and 695 company-year observations reduces to 255 variables and 422 observations after data-quality filters (p.4).

The operational consequence is direct: a single "ESG score" field is not a sufficient statistic. When `2508.18679v2` models log return volatility, selected raw variables reach R2 0.51 against 0.39 for plain volatility (p.3), and they beat every aggregated level, including the full set of ten category scores (p.5).

## 2. Labels as signals: what actually predicts the talk-walk gap

`2606.31469v1` gives the cleanest falsifiable result in the slice. It defines a Disclosure-Performance Gap as the within-sector standardised distance between a firm's voluntary disclosure and its realised emissions intensity (Scope 1+2 per unit revenue) (p.1). Across 200 large STOXX Europe 600 firms in Energy, Materials, Industrials and Utilities in fiscal 2023, the gap has standard deviation 1.267 and runs from -3.22 to +5.28 (p.11).

Four predictors survive a six-stage selection over 421 candidate specifications: flagship index membership widens the gap (beta +0.78, p<0.01), TCFD support widens it (beta +0.86, p<0.05, identified off only 19 non-supporters), renewable-energy use narrows it (beta -0.31, p<0.01) and environmental capex narrows it (beta -0.22, p<0.05) (p.13). Bivariate correlations agree (renewable r=-0.33, capex r=-0.19, index r=+0.31, TCFD r=+0.28) while governance controls sit near zero (p.13). The symbolic signals are saturated: about 55% of firms are flagship index constituents and about 91% are TCFD supporters (p.12). The evidentiary limit is that this is one cross-section, so the coefficients are conditional associations and not treatment effects (p.6).

## 3. Filtering versus learning: where ESG data pays

The practical question is whether any of this survives costs. `2002.07477v2` finds that ESG best-in-class filtering on the MSCI World universe of more than 1,600 companies produced small but negative excess returns and negative information ratios over August 2009 to March 2018, with Europe and low thresholds the only partial exception (p.4, p.5). It also shows the signal is non-linear: only 12 of 40 World Developed sector-and-metric portfolios had positive excess return (p.6).

Screening the same granular ESG features with a learned rule set instead produced a positive portfolio that beat the benchmark by 2.76% annualised, best-in-class by 2.94% and a negative-score portfolio by 4.77% over Jan 2013 to Mar 2018 (p.14); the number of active rules moved between 31 and 73 (p.16), and the learned rules decayed to zero or negative excess return within about a year without retraining (p.16). `1806.09906v2` adds the crisis asymmetry from 73 US socially responsible mutual funds: in 2005-2008 the highest-ESG tertile had monthly return -0.32% and Sharpe 0.56, versus 0.30 for the lowest tertile, while outside the crisis the lowest-ESG funds had the higher raw returns and the largest inflows (p.7). Expense ratios were negative for both risk-adjusted return and flows (p.7, p.8).

## 4. Transition risk is a forward-looking, state-dependent shock

`2410.00902v1` builds a general-equilibrium model in which the arrival rate of a climate-linked transition shock rises with temperature, and shows two opposite responses: anticipation of a technology-driven transition produces a "run on fossil fuel" with accelerated extraction and falling spot prices, while anticipation of a tax-driven transition produces a "reverse run" with restrained production and higher prices (p.1).

It cites the standard stranding benchmark that holding warming below 2C requires leaving 30% of oil and 80% of coal reserves unburned (p.7). The empirical test is an event study on transition-risk events plus a structural VAR: sectors with the highest transition-risk exposure show the largest cumulative abnormal returns, and an increased transition likelihood leads to higher oil production and a lower oil spot price, statistically significant mainly over 2009-2019 (p.4). The model is stylised and the empirical response is state-dependent, so the sign of the effect is the robust output, not its magnitude (p.4, p.5).

## 5. Carbon pricing: exposure and how far it propagates

`2503.10644v1` is the most operationally concrete paper in the slice. Using VAT transaction records it estimates CO2 emissions for all 410,523 Hungarian firms from their oil and gas purchases, expanding coverage from the 119 firms reporting under ETS I to 185,783 firms with positive emissions (p.2). It finds 45% of firms directly exposed to carbon pricing, accounting for 70% of sales (p.2), and the emissions distribution is heavy tailed with a power-law exponent near -1.05 (p.2).

At the EU ETS II price cap of 45 EUR/t, direct real-economy losses are about 1.3% of sales and direct bank equity losses about 1.2% (p.4); system-wide equity losses rise to 0.3%, 1.2%, 2.2%, 4.7% and 21% at 10, 45, 100, 200 and 1000 EUR/t (p.4). The headline result is contagion: once defaults cascade along supply chains, optimistic-substitution losses reach 12.3% of gross output at 200 EUR/t, and a pessimistic no-substitution scenario jumps past 50% of output and 43% of bank equity at a 30 EUR/t price when a systemically central firm fails (p.6). The authors call the estimates upper bounds because firms are assumed not to adapt in advance (p.7).

## 6. Carbon markets as instruments: allocation and price

`1108.2305v2` derives an emissions-permit allocation from the Boltzmann distribution, in which a single parameter beta sets how strongly the allocation favours high per-capita emitters (p.5, p.10). Applied to eight countries and a 2008 target of 17,084,135 thousand tonnes of permits (p.12), the least-squares reference value beta=0.0966 gives China 41% and the US 32% of permits, leaving both as net sellers and six countries as buyers (p.16, p.18); below beta=0.1164 China receives more permits than the US, and above it the US receives more, so one parameter reassigns responsibility (p.14).

`2212.11787v1` forecasts the EU carbon price with support vector regression on oil, coal, gas, the DAX and Paris emission targets, projecting 114.7 EUR in 2030 under a 2,137,554 kt target and 203.0 EUR under a 1,603,165.5 kt target (p.5). The forecast is explicitly data-limited: the author states the study has limited training data and only two policy targets (p.12).

## 7. Divestment, beliefs and social tipping

`1902.07481v1` models divestment as a social-dynamics problem. Its agent-based model couples an adaptive investor network, where beliefs about future carbon policy spread by contagion, to a stock market and a fixed 250 GtCO2 carbon budget (p.5). Six behavioural regimes emerge, and the central result is a tipping point: a socially responsible investor share of only 10% to 20% is sufficient to burst the carbon bubble when social interaction is fast, consistent with a Pareto-principle minority driving change (p.1, p.13). Empirically, more than 1,000 institutions holding about USD 7.93 trillion had pledged to divest by 2017, although fossil fuel stocks are typically only 5-10% of a portfolio (p.2, p.3). The model is conservative in that it holds the SRI share fixed at 15% and lets beliefs, not social norms, spread (p.5), and it assumes a constant fuel price and identical investor wealth (p.13).

## 8. News-based ESG scoring and its accuracy limits

`2212.11765v1` predicts Refinitiv Asset4 combined ESG scores, built from 186 metrics grouped into 10 themes on a 0-100 scale (p.3), from news for about 3,000 US companies and 3,739,871 articles (p.1, p.3). A DistilBERT relevance classifier reaches 83% accuracy (p.6), but predicting the exact 0-100 rating as a 100-class problem reaches only 2-3% accuracy, so the task is better framed as regression (p.6, p.7).

The best model, a deep CNN regression, achieves a mean absolute difference of about 11.97 to 13.04 rating points against the provider score (p.7), and predicts small-cap firms better (10.28) than large-cap firms (15.42) (p.7). The same paper cites a Bank of America estimate that S&P 500 companies lost more than USD 600 billion of market value to ESG issues over seven years (p.1). The limit is that the ground truth is one provider's score, so the model learns to imitate a rater rather than to measure sustainability.

## 9. Definitional choices that make reported results fragile

Every headline in this slice rests on a definitional choice that is invisible from the number alone. `2606.31469v1` converts the ordinal CDP climate grade to equally spaced integers (A=8 down to F=0) before standardising, an equal-interval assumption it defends only because a rank-based recoding leaves the signs and significance unchanged (p.8). It also codes firms that failed to disclose as zero rather than dropping them, so silence is treated as a strategic signal (p.7). `2508.18679v2` makes its own imputation choices: missing boolean ESG variables are set to zero on the argument that firms report positive policies and that adverse events would surface in the media (p.4), and missing controversy values are likewise set to zero (p.19). Those choices matter: models built from raw variables inside a single ESG category outperform a model using all ten pre-aggregated category scores, direct evidence that aggregation dilutes the most risk-relevant variables (p.5). On top of this sits provider dependence, where the same 200 firms yield different determinant structures depending on the rating lens (p.1). No paper here tests how sensitive its conclusions are to all of these choices jointly; each study varies one or two at most.

## What this project can take from it

| Finding | Where it lands | What to do |
|---|---|---|
| ESG rating effects flip when the provider is swapped (`2606.31469v1`) | the data engine | version every ESG field by provider and never merge providers into one column |
| Raw ESG variables beat aggregated scores for risk (`2508.18679v2`) | python/nautilus_trader/data | persist raw ESG fields alongside the composite and select per sector |
| Best-in-class ESG filtering is not alpha (`2002.07477v2`) | python/nautilus_trader/portfolio | measure net-of-cost excess return for any ESG screen instead of assuming a premium |
| Learned ESG rules decay within a year (`2002.07477v2`) | python/nautilus_trader/backtest | require walk-forward retraining before an ESG-factor signal goes live |
| Transition risk is a state-dependent shock (`2410.00902v1`) | the backtest engine | expose a transition-risk scenario switch and stress under "run" and "reverse run" |
| Carbon-price shocks propagate down supply chains (`2503.10644v1`) | crates/risk | model issuer and counterparty supply-chain exposure, not only direct emissions |
| EU ETS II caps define a policy calendar (`2503.10644v1`) | docs/concepts/custom_data.md | encode the 45 EUR/t cap and its scheduled reset as calendar events |
| Permit allocation rules are parameterised (`1108.2305v2`) | python/nautilus_trader/model | represent carbon allowance instruments and their allocation parameters explicitly |
| Carbon-price forecasts carry wide data limits (`2212.11787v1`) | crates/analysis | report training-window bounds and uncertainty for any carbon price input |
| Divestment has a tipping point, not a trend (`1902.07481v1`) | the risk engine | scenario-test a sudden fossil-fuel de-rating instead of a linear glide path |
| ESG fund protection is crisis-specific (`1806.09906v2`) | python/nautilus_trader/analysis | evaluate ESG portfolios per regime, not on one full-period Sharpe ratio |
| News-based scores imitate one rater (`2212.11765v1`) | crates/analysis | tag the source rater on any derived ESG score and cross-check a second provider |
| Missing ESG booleans are imputed, not dropped (`2508.18679v2`) | python/nautilus_trader/data | document and version the missing-value policy for ESG booleans instead of defaulting silently |
| The ESG-volatility relation is non-linear and heteroskedastic (`2508.18679v2`) | crates/analysis | model log volatility and use robust standard errors when regressing ESG factors |

## Caveats

The slice is thin in places and the papers are not comparable to one another. `2606.31469v1` is a single fiscal-year (2023) cross-section of 200 European firms in four carbon-intensive sectors, so it establishes associations, not causal effects.

`1806.09906v2` covers 73 US funds over 2005-2016 and the crisis-versus-expansion split rests on three short sub-periods. `2002.07477v2` tests its machine-learning screen over only five years and three months (Jan 2013-Mar 2018), a strong-equity regime, and its own author notes this. `2212.11787v1` forecasts a single market (EU carbon) with limited data and two policy targets, and its 2030 numbers should be read as illustrations, not forecasts. `1108.2305v2` allocates permits for eight countries on 2008 data and defines allocation energy in the simplest possible way, negatives of per-capita emissions. `1902.07481v1` is a stylised agent-based model with a constant fuel price. `2503.10644v1` is one country (Hungary) and explicitly reports upper-bound losses with no anticipatory adaptation. `2410.00902v1` is a calibrated model whose empirical support is strongest only after 2009. Critically, no paper here establishes that ESG ratings or climate labels generate net-of-cost alpha, and none provides a validated, multi-provider, multi-market ESG signal. The slice disagrees with itself on the central question of whether ESG information is best used for return or for risk, and `2508.18679v2` and `2002.07477v2` come down on the risk side.

## Papers read in depth

- `2606.31469v1` - Same Firms, Different Verdicts: ESG Rating Choice and the Measurement of Greenwashing (2026). Firm-level disclosure-performance gap for 200 European firms; shows the determinants are conditional on the rating provider, but it is one cross-section.
- `2508.18679v2` - Identifying Risk Variables From Raw ESG Data Using Its Hierarchical Structure (2025/2026). Selects raw LSEG variables for return volatility and beats aggregated scores; US sectors only, and the ground truth is one provider's data.
- `2002.07477v2` - ESG investments: Filtering versus machine learning approaches (2020). Best-in-class filtering does not pay while learned non-linear screening does; short out-of-sample window in one market regime.
- `1806.09906v2` - Fund Characteristics and Performances of Socially Responsible Mutual Funds: Do ESG Ratings Play a Role? (2018). Crisis-conditional ESG fund performance; small sample of 73 US funds and coarse sub-periods.
- `2410.00902v1` - A Run on Fossil Fuel? Climate Change and Transition Risk (2024). General-equilibrium model of transition-risk expectations with an event study and VAR; stylised, and the empirical response is state-dependent.
- `2503.10644v1` - Combined climate stress testing of supply-chain networks and the financial system with nation-wide firm-level emission estimates (2025). Firm-level carbon stress test for all Hungarian firms including contagion; one country, upper-bound losses.
- `1108.2305v2` - Permit Allocation in Emissions Trading using the Boltzmann Distribution (2011). Maximum-entropy permit allocation with one fairness parameter; eight countries on 2008 data with a minimal energy definition.
- `2212.11787v1` - Macro carbon price prediction with support vector regression and Paris accord targets (2022). SVR forecast of the EU carbon price to 2030; very limited training data and only two policy targets.
- `1902.07481v1` - Divestment may burst the carbon bubble if investors' beliefs tip to anticipating strong future climate policy (2019). Agent-based divestment model with a social tipping point; stylised, constant fuel price, identical investor wealth.
- `2212.11765v1` - Predicting Companies' ESG Ratings from News Articles Using Multivariate Timeseries Analysis (2022). Deep-learning prediction of Asset4 scores from news; it learns one provider's score and exact-class accuracy is near zero.

## Where to next in the corpus

- `2407.20377v1` - Leveraging Natural Language and Item Response Theory Models for ESG Scoring. A lead on whether a second measurement model reproduces provider scores or reveals divergence.
- `1110.1567v3` - A Modified GHG Intensity Indicator: Toward a Sustainable Global Economy based on a Carbon Border Tax and Emissions Trading. A lead on carbon border adjustment, which the read set does not cover.
- `2408.02339v2` - Modeling the impact of Climate transition on real estate prices. A lead on physical and transition risk pricing in a non-equity asset class.
- `2006.11888v1` - Tri-criterion model for constructing low-carbon mutual fund portfolios. A lead on constrained portfolio construction with a carbon objective.
- `1603.06196v1` - Switching Economics for Physics and the Carbon Price Inflation: Problems in Integrated Assessment Models. A lead on the modelling assumptions behind carbon-price paths.
- `2510.00244v2` - Board gender diversity and emissions performance. A lead on the governance-to-emissions channel, tested here only as a null.
