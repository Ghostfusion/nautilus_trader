# What the general-finance literature can teach this platform

Date: 2026-10-06. Revision 1.

This directory holds twenty-eight learning briefs written from a local harvest of the arXiv
`q-fin.GN` (General Finance) category: 3095 records spanning 1997 to 2026. Each brief takes one
slice of that corpus, reads a set of papers in depth, and states what the evidence changes about how
a strategy is built, simulated, allocated, and evaluated. The briefs are deliberately operational:
the last table of each one maps its findings onto concrete parts of this repository.

`strategies/books/` holds the companion set, written from a harvest of `q-fin.TR` (Trading and
Market Microstructure): 2370 records, sixteen briefs covering execution, impact, the order book,
market making, latency, volatility, order flow, and the trading-side machine-learning and crypto
material. Read the two together. That harvest is where the microstructure evidence lives; this one
is where the general-finance evidence lives: who lends, who insures, who is regulated, what a
rating, a filing or a text signal is worth, how a portfolio should be built when the covariance is
estimated rather than known, and how financial research fails to replicate. Where a category
overlaps, the brief here says so and adds rather than restates.

## The harvest

| Field | Value |
| --- | --- |
| Source | arXiv full listing for `cat:q-fin.GN`, captured 2026-10-05 |
| Records harvested | 3095 |
| PDFs on disk | 3066 (29 besides are withdrawn or return HTTP 404) |
| Size | 3700.2 MB, downloaded over 185 minutes with a 3.0 s delay |
| Years spanned | 1997-2026 |
| Records with a journal reference | 792 (26%) |
| Records with a DOI | 918 (30%) |
| Records from 2021 onward | 933 (30%) |
| Primary categories | 1758 q-fin.GN, 290 physics.soc-ph, 139 econ.GN, 92 q-fin.ST, 82 cond-mat.stat-mech, 62 q-fin.TR, 58 q-fin.RM, 51 q-fin.PR |

Every record is cross-listed into `q-fin.GN`, arXiv's general-finance category and a cross-listing
attractor: it collects the econophysics ancestors (`cond-mat.*`, `physics.soc-ph`, `nlin.*`), the
general-economics cross-lists (`econ.GN`), and the applied work that does not fit a narrower `q-fin`
category. The early material is sociophysics and statistical mechanics, the middle is credit, macro
and portfolio work, and the recent material is dominated by crypto, machine learning and language
models. Practitioner microstructure is largely absent, because that material is primary in
`q-fin.TR` and is covered in `strategies/books/`.

The harvest itself is not vendored into this repository. It is kept outside it, with the full record
set in `.state/metadata.jsonl` beside the PDFs, and the briefs cite papers by arXiv id, which is the
durable reference.

## How the briefs were produced

1. **Census.** Every one of the 3095 records was scored against the topic patterns of every
   category over its title and abstract, which produced the per-category counts in the table below.
   This is a census of the whole corpus, not a sample.
2. **Shortlist.** The highest-signal forty records per category became the candidate list, fewer
   where the category is smaller, with the score favouring title matches, journal publication and
   recency.
3. **Deep reading.** Between six and thirteen papers per category were read from the PDFs, targeting
   the abstract, the model or data setup, the results tables, and the conclusion. Every number kept
   in a brief was read in the PDF and carries a page marker.
4. **Writing.** Each brief was written from those notes only, and cites papers by arXiv id, which is
   the durable reference anyone can re-fetch.
5. **Verification.** A script kept beside the harvest checks every brief for ASCII and line-feed
   hygiene, the required sections, whether every cited arXiv id resolves against the harvest
   metadata, whether every paper listed as read in depth has a local PDF, and whether every
   repository path cited in a take-away table exists. All twenty-eight files pass with zero
   findings.

## The corpus by category

`Records` counts harvest records whose title or abstract matches the category, so a paper can belong to more than one. `Recent` counts records from 2021 onward. `Read` counts the papers read in depth for that brief.

| Brief | Records | Years | Journal refs | Recent | Read |
| --- | --- | --- | --- | --- | --- |
| `01_networks_and_systemic_risk.md` | 322 | 2003-2026 | 124 | 56 | 12 |
| `02_banking_credit_and_funding.md` | 223 | 2000-2026 | 59 | 86 | 12 |
| `03_crypto_defi_and_perpetuals.md` | 193 | 2012-2026 | 40 | 121 | 12 |
| `04_machine_learning_for_finance.md` | 156 | 2008-2026 | 18 | 126 | 12 |
| `05_bubbles_crashes_and_criticality.md` | 153 | 2000-2026 | 55 | 24 | 12 |
| `06_econophysics_and_agent_based_markets.md` | 148 | 2000-2025 | 51 | 16 | 12 |
| `07_macro_rates_and_fx.md` | 140 | 2001-2026 | 30 | 30 | 12 |
| `08_predictability_and_trading_strategies.md` | 122 | 2001-2026 | 22 | 52 | 12 |
| `09_insurance_pension_and_household_finance.md` | 115 | 2002-2026 | 32 | 38 | 12 |
| `10_portfolio_and_allocation.md` | 112 | 2000-2026 | 15 | 52 | 12 |
| `11_language_models_news_and_text.md` | 100 | 2011-2026 | 23 | 73 | 12 |
| `12_options_and_derivatives.md` | 96 | 2008-2026 | 17 | 41 | 12 |
| `13_stylized_facts_and_scaling.md` | 93 | 1998-2026 | 35 | 14 | 13 |
| `14_volatility_and_microstructure_noise.md` | 86 | 1998-2026 | 19 | 40 | 12 |
| `15_esg_climate_and_sustainability.md` | 79 | 2010-2026 | 16 | 41 | 10 |
| `16_information_theory_and_complexity.md` | 61 | 2007-2026 | 11 | 16 | 10 |
| `17_quantum_and_exotic_methods.md` | 55 | 2003-2025 | 24 | 12 | 8 |
| `18_energy_and_commodities.md` | 53 | 2006-2026 | 14 | 15 | 12 |
| `19_manipulation_fraud_and_governance.md` | 47 | 2007-2026 | 8 | 27 | 10 |
| `20_market_design_regulation_and_fees.md` | 43 | 2007-2026 | 11 | 21 | 12 |
| `21_prediction_markets_and_betting.md` | 24 | 2009-2026 | 1 | 10 | 8 |
| `22_risk_measures_and_drawdowns.md` | 23 | 2009-2026 | 2 | 10 | 8 |
| `23_market_making_and_inventory.md` | 22 | 2006-2026 | 8 | 9 | 8 |
| `24_regimes_and_change_points.md` | 18 | 2006-2026 | 3 | 6 | 8 |
| `25_execution_impact_and_order_book.md` | 24 | 2009-2026 | 6 | 13 | 12 |
| `26_order_flow_hft_and_latency.md` | 29 | 2008-2026 | 2 | 13 | 10 |
| `27_market_data_quality.md` | 12 | 2016-2026 | 1 | 8 | 6 |
| `28_overfitting_and_research_integrity.md` | 9 | 2011-2025 | 3 | 5 | 6 |

Across the twenty-eight briefs, 424 distinct papers are cited, 127 of which have a journal reference, and 297 paper readings are listed in depth. `corpus_map.md` lists, for each category, the thirty highest-signal records that were not read, so the census is traceable and the next reading session has a starting point.

## The briefs

| Brief | Scope |
| --- | --- |
| `01_networks_and_systemic_risk.md` | direct interbank exposure is a weak channel that switches on when it meets overlapping portfolios |
| `02_banking_credit_and_funding.md` | funding is the residual of payment flows, and leverage rules can amplify the cycle |
| `03_crypto_defi_and_perpetuals.md` | the cost of consensus stayed pinned to transfer value while protocol control stayed concentrated |
| `04_machine_learning_for_finance.md` | the evaluation design, not the architecture, decides what survives validation |
| `05_bubbles_crashes_and_criticality.md` | bubble regimes are diagnosable, their timing is only probabilistic |
| `06_econophysics_and_agent_based_markets.md` | fitted wealth tails and fitted agent parameters are both non-identifiable |
| `07_macro_rates_and_fx.md` | central-bank announcements co-jump the curve, and AI-read data releases price the currency |
| `08_predictability_and_trading_strategies.md` | the cross-sectional findings are mostly real, so decay and leakage are the binding constraints |
| `09_insurance_pension_and_household_finance.md` | how a single-digit insurance loading and a two-stock household set the demand side |
| `10_portfolio_and_allocation.md` | estimator error, views, and what survives out of sample |
| `11_language_models_news_and_text.md` | model choice, cutoff discipline, and what filing text still carries |
| `12_options_and_derivatives.md` | implied volatility is a variance quantile and parity enforcement still costs |
| `13_stylized_facts_and_scaling.md` | why predictable order flow still leaves a random walk |
| `14_volatility_and_microstructure_noise.md` | hybrid volatility forecasts beat risk models, and most jumps are not news |
| `15_esg_climate_and_sustainability.md` | the strongest signal is disagreement between raters |
| `16_information_theory_and_complexity.md` | a parameter-free roughness statistic with an exact null |
| `17_quantum_and_exotic_methods.md` | a quantum annealer matches classical portfolio search while a quantum walk earns its keep as a return-distribution model |
| `18_energy_and_commodities.md` | storage is an option and the grid writes the strike |
| `19_manipulation_fraud_and_governance.md` | short-horizon settlement manipulation, and the false-positive cost of every detector |
| `20_market_design_regulation_and_fees.md` | rule changes reroute flow and rewrite what prices on |
| `21_prediction_markets_and_betting.md` | market-implied probabilities that beat the forecasters, and a tape that will fool your microstructure measures |
| `22_risk_measures_and_drawdowns.md` | why tail-sensitive measures are fragile and the axioms still force VaR |
| `23_market_making_and_inventory.md` | dealer pricing power sets the spread and the yield, and it acts through quantities rather than prices |
| `24_regimes_and_change_points.md` | the only regime model that survived out of sample was the one filtered online |
| `25_execution_impact_and_order_book.md` | what a general-finance harvest adds to the microstructure core |
| `26_order_flow_hft_and_latency.md` | speed is paid at the tape, not inside the book |
| `27_market_data_quality.md` | recorded outcomes are a collector's classification, and they do not survive a two-week holdout |
| `28_overfitting_and_research_integrity.md` | most published predictors replicate, and one bad data row can still erase a strategy |

## What the corpus says collectively

These are the findings that recur across independent slices, which is why they are worth acting on.

1. **The evaluation design, not the model, decides what survives.** A matched classification-versus-regression
   experiment moved a portfolio's value-weighted Sharpe from 1.39 to 2.08 with the same features and models
   (`2108.02283v7`); a GARCH-GRU hybrid cut volatility MSE by 72 percent while the resulting Value-at-Risk and
   Expected Shortfall models did not inherit it and the hybrid shortfall tests failed (`2310.01063v1`); and seven
   language models scoring the same earnings calls agreed at a mean pairwise rank correlation of only 0.52, so the
   provider is part of the measurement (`2609.31013v1`). The recurring practical lesson is to fix the scoring rule
   before comparing models, and to report the metric that is actually being optimised.

2. **Bookkeeping and definition errors are larger than most signal sizes.** Net-return and arithmetic-mean
   bookkeeping inflated the S&P 500 Sharpe ratio by nearly 30 percent and overstated the 1960-2020 terminal value by
   89 percent (`2405.10920v1`); a single erroneous odds row turned two published betting ROIs of 17.29 and 28.82
   percent into -7.36 and -6.31 percent while the coefficients and bet sequence reproduced exactly (`2306.01740v4`);
   and a recorded outcome label was shown to be a joint function of the platform event and the collector's polling
   design (`2607.02823v4`). Before tuning anything, check what the number means.

3. **Fitted models in this corpus are frequently non-identifiable, and the tail is often an artifact.** Kinetic
   wealth exchange has no stationary distribution for any finite population, and its power-law window lasts only
   about 300 steps at N = 10,000 (`0809.4139v2`); two parameters of an extended Chiarella agent-based model enter
   the same term so both cannot be calibrated (`2208.14207v1`); the log-periodic power law yields a distribution
   over the critical time rather than a date, and about one bubble in three ends without a crash (`1107.3171v3`);
   and the roughness statistic carries a finite-size bias of +0.323 at H = 0.9 (`2512.02352v3`). Report the
   identifiable part, and say which part is a convention.

4. **Tail-sensitive risk measures are fragile by construction, and the axioms force Value-at-Risk.** One observation
   beyond the quantile can make a historical expected-shortfall estimate arbitrarily large, while VaR needs more
   than (1-alpha)n points to move (`2206.02582v2`); any surplus-invariant, law-invariant, conic and
   truncation-closed acceptance set must be a VaR acceptance set (`1707.05596v2`); and close to three quarters of
   random equity windows look non-normal against one in six stress-ordered ones (`1310.4538v2`). Choose the risk
   measure as a policy, and test the estimator's stability on your own sample.

5. **Market data is not clean, and the defect is usually in the source, not the parser.** Public prediction-market
   book feeds recovered the trade aggressor only about 59 percent of the time, flipping the sign of
   direction-dependent measures on most markets (`2604.24366v2`); mini flash crashes turned out to be a venue-rule
   artefact in which 67.85 percent of the episodes were ISO-initiated (`1211.6667v1`); and comparability across
   providers is the exception rather than the rule, as ESG raters disagree far more than credit raters do
   (`2606.31469v1`). Validate a feed against an independent record of the same events before computing anything
   from it.

6. **Rules and incentives reroute activity rather than removing it.** Europe gained 13.88 additional ICOs per
   region-month after the DAO Report (`2602.00138v1`); a delisting mandate left aggregate stablecoin volume flat
   while shifting the venue cross-section by 0.818 pre-event standard deviations (`2607.09514v1`); and lengthening
   the settlement horizon of an ultra-short contract, rather than adding surveillance, removes the manipulation
   signature (`2606.31675v1`). Model the venue's rule set as part of the payoff, not as a footnote.

7. **Every detector is a false-positive machine, and its cost is measurable.** A conventional Z-score spoofing screen
   on the LUNA tape flagged orders priced $0.01 and $0.11 far from the spread and missed inserted spoofing
   entirely, while a classifier's F1 halved from 80.40 to 46.08 once neutral book states entered the label set
   (`2308.08683v1`, `2403.13429v1`); and deflating the top two volume quantiles as suspected wash trading destroyed
   legitimate flow and cut a portfolio Sharpe from 1.41 to 0.96 (`2404.07222v3`). Report precision, recall and the
   cost of the errors, not the hit count.

8. **Costs and physical mechanics dominate apparent edges.** Battery arbitrage does not clear its own roughly
   100 EUR/MWh wear cost in most European day-ahead markets (`2112.09816v2`), with zero-wear profitability on only
   about 310 days of 2019 in Spain (`2007.00486v2`); the market-based variance of an allocation differs from the
   Markowitz variance by the coefficient of variation of trade volume (`2507.21824v1`); and option-implied discount
   factors sat about 37.50 bp above OIS while parity residuals were compressed to near zero (`2604.19604v6`).
   Where a mechanism has a denomination, model the denomination.

9. **Concentration is the norm on both sides of the market.** The top six mining pools held 89.4 percent of capacity
   (`2606.03153v1`); no observed governance-token distribution needed more than 100 addresses for a quorum
   (`2102.10096v2`); the median household portfolio never fell below an effective two stocks over 2001-2021
   (`2503.17778v1`); and dealer pricing power plus network transmission accounted for 2.5 to 5.3 percentage points
   of gilt yield deviation (`2603.10690v1`). Liquidity and counterparty assumptions that assume many participants
   are wrong in the tail.

10. **Contagion and stability are non-monotone in connectivity and leverage.** Direct interbank exposures are a weak
    standalone channel that becomes a strong amplifier once a common asset shock is added (`1306.3704v1`); the
    critical degree is a closed form, small (about 5 to 10) against an observed mean degree near 15 (`1402.4783v2`);
    and a 16 percent capital floor can force selling into a decline and turn an absorbable shock into a catastrophe
    (`1403.1637v1`). Risk limits deserve a simulated second-order test, not only a first-order one.

11. **Persistence, when measured properly, is real but state-dependent.** Publication-bias corrections across three
    independent teams shrank in-sample returns by only 10 to 15 percent, with false discovery rates under
    10 percent (`2209.13623v3`); the cross-sectional factor zoo's bound on false discovery is 8.5 to 25 percent
    (`2206.15365v10`); and yet the risk-return trade-off is positive and significant only in low-volatility states
    (`1410.6005v1`). The binding constraint is the analyst's own specification search and the regime, not a
    t-statistic threshold.

12. **Out-of-sample survival correlates with online re-estimation and with honest calibration.** The only regime
    model in its slice that was filtered and re-estimated online returned 15.18 percent annualised against
    -2.44 percent for a backward-looking rule (`2309.00875v3`); held-out calibration slope 0.013 and intercept
    +2.816 exposed a model whose development AUROC of 0.8594 collapsed to 0.4642 (`2607.02823v4`); and a maturity
    regression that reproduced coefficients exactly still failed the replication's data audit (`2306.01740v4`).
    Prefer the model that can be updated and checked over the one that fits best in hindsight.

## How to use these documents

- Treat the short answer at the top of each brief as the checklist for that topic, and the take-away
  table as the list of concrete places where this repository should change.
- Treat the caveats section as binding. These are mostly single-market, single-period studies, and
  the briefs say so where it matters.
- Where a brief names a paper as a lead rather than reading it, no claim is made about it; follow the
  id when the claim matters.
- Use `corpus_map.md` when a brief's caveats name a gap: the leads there are the papers that would
  fill it.

## Limits

Three limits apply to everything in this directory. First, the corpus is one arXiv category, and
`q-fin.GN` is a cross-listing attractor rather than a field: it over-represents econophysics and
sociophysics, and under-represents practitioner microstructure, which lives in `q-fin.TR` and in
`strategies/books/`. Second, the composition is uneven: credit, macro, crypto and machine learning
appear as bodies of evidence, while market data quality, overfitting and execution appear through
single papers, so those briefs are short and say so. Third, the general-finance literature is where
ESG and text-signal results with attractive numbers are published, so publication bias is real in
exactly those slices; those briefs are written accordingly.

None of these documents contains trading advice or a profitability claim. They are statements about
mechanisms, measurement and evaluation design, and every one of them is falsifiable against the
cited paper.
