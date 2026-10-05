# What the trading-and-microstructure literature can teach this platform

Date: 2026-10-05. Revision 1.

This directory holds sixteen learning briefs written from a local harvest of the arXiv
`q-fin.TR` (Trading and Market Microstructure) category: 2370 records spanning 1997 to 2026.
Each brief takes one slice of that corpus, reads a set of papers in depth, and states what the
evidence changes about how a strategy is built, executed, simulated, and evaluated. The briefs are
deliberately operational: the last table of each one maps its findings onto concrete parts of this
repository.

## The harvest

| Field                            | Value                                                                                              |
| -------------------------------- | -------------------------------------------------------------------------------------------------- |
| Source                           | arXiv full listing for `cat:q-fin.TR`, captured 2026-09-23                                         |
| Records harvested                | 2370                                                                                               |
| PDFs on disk                     | 2359 (11 besides are withdrawn or return HTTP 404)                                                 |
| Size                             | 3658.5 MB, downloaded in 132 minutes with a 3.0 s delay                                            |
| Years spanned                    | 1997-2026                                                                                          |
| Records with a journal reference | 378 (16%)                                                                                          |
| Records with a DOI               | 568 (24%)                                                                                          |
| Primary categories               | 1400 q-fin.TR, 124 q-fin.ST, 92 q-fin.MF, 89 cond-mat.stat-mech, 83 econ.GN, 79 q-fin.CP, 61 cs.LG |

Every record is cross-listed into `q-fin.TR`, which is what makes the corpus coherent: it is the
market-microstructure and trading half of quantitative finance, not a general finance dump. The
early material is econophysics (`cond-mat.*`, `physics.*`, `nlin.*`), the middle is market
microstructure and optimal execution, and the recent material is dominated by machine learning and
crypto venues.

The harvest itself is not vendored into this repository. It is kept outside the repository, with
the full record set in `.state/metadata.jsonl`. The briefs therefore cite papers by arXiv id, which
is the durable reference; anyone can re-fetch an id from arXiv without the local copy.

## How the briefs were produced

1. **Census.** Every one of the 2370 records was scored against eighteen topic patterns over its
   title and abstract, which produced the per-category counts in the table below. This is a census
   of the whole corpus, not a sample.
2. **Shortlist.** The highest-signal forty records per category became the candidate list, with the
   score favouring title matches, journal publication, and recency.
3. **Deep reading.** Twelve papers per category (192 in all, plus the ones cited as leads) were read
   from the PDFs, targeting the abstract, model or experiment setup, results tables, and conclusion.
   Notes were kept with page anchors for every number.
4. **Writing.** Each brief was written from those notes only. No number in these documents was
   entered without a page-anchored source in the notes.
5. **Verification.** A script checks every file for ASCII and line-feed hygiene, resolves every
   cited arXiv id against the harvest metadata, checks that every numeric literal appears in the
   source notes for that category, and resolves every repository path cited in a take-away table.
   All sixteen files pass with zero findings.

Two labels appear in the briefs. **Verified** means the claim was read in the paper, and the page
marker is given inline, for example `(p.6)`. **Lead** means the paper is listed at the end under
"Where to next in the corpus" and has not been read; leads are pointers, not evidence.

## The corpus by category

`Records` counts harvest records whose title or abstract matches the category, so a paper can belong
to more than one. `Recent` counts records from 2021 onward, which is the bulk of the machine-learning
and crypto material. "Candidate papers screened" inside each brief is the size of that category's
candidate list after the census ranked the matches, which is why it is smaller than `Records`.

| Brief                                          | Records | Years     | Journal refs | Recent |
| ---------------------------------------------- | ------- | --------- | ------------ | ------ |
| `01_execution_and_liquidation.md`              | 214     | 2004-2026 | 19           | 97     |
| `02_market_impact_and_trading_cost.md`         | 435     | 1999-2026 | 63           | 206    |
| `03_limit_order_book_dynamics.md`              | 420     | 1999-2026 | 50           | 184    |
| `04_market_making_and_inventory.md`            | 205     | 2000-2026 | 28           | 82     |
| `05_latency_and_market_structure.md`           | 348     | 1998-2026 | 54           | 166    |
| `06_volatility_and_microstructure_noise.md`    | 399     | 1999-2026 | 61           | 209    |
| `07_order_flow_and_toxicity.md`                | 234     | 2001-2026 | 26           | 132    |
| `08_econophysics_and_agent_based_markets.md`   | 441     | 1998-2026 | 108          | 144    |
| `09_machine_learning_for_trading.md`           | 441     | 2007-2026 | 37           | 345    |
| `10_cross_venue_and_arbitrage.md`              | 200     | 1999-2026 | 23           | 116    |
| `11_market_design_fees_and_auctions.md`        | 213     | 2002-2026 | 32           | 111    |
| `12_liquidity_and_systemic_risk.md`            | 104     | 2004-2026 | 10           | 59     |
| `13_crypto_amm_and_perpetuals.md`              | 232     | 2012-2026 | 25           | 200    |
| `14_news_and_language_model_signals.md`        | 117     | 2000-2026 | 16           | 80     |
| `15_portfolio_construction_under_frictions.md` | 331     | 1998-2026 | 45           | 185    |
| `16_options_and_derivative_instruments.md`     | 242     | 1998-2026 | 23           | 149    |

Across the sixteen briefs, 321 distinct papers are cited, 73 of which have a journal reference.
`corpus_map.md` lists, for each category, the thirty highest-signal records that were not read, so the
census is traceable and the next reading session has a starting point.

## The briefs

| Brief                                          | Scope                                                                                                                                                                          |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `01_execution_and_liquidation.md`              | Optimal execution and liquidation: the risk/cost tradeoff, VWAP optimality, risk-sensitive objectives, multi-asset schedules, reinforcement-learning execution                 |
| `02_market_impact_and_trading_cost.md`         | Price impact: the square-root law and its evidence, prefactor bias, mechanical versus informational origins, transient versus permanent decomposition, what fills can identify |
| `03_limit_order_book_dynamics.md`              | The book as a queueing system: order-flow imbalance, fill probability, adverse selection of passive fills, latent liquidity, resilience, deep-learning book models             |
| `04_market_making_and_inventory.md`            | Quoting and inventory: the Avellaneda-Stoikov and Cartea-Jaimungal framework, online parameter learning, adverse selection, client tiers, hedging with impact                  |
| `05_latency_and_market_structure.md`           | The economics of speed: ordinal latency, measured latency budgets, engine design, speed bumps, timestamp and clock integrity, profitability ceilings                           |
| `06_volatility_and_microstructure_noise.md`    | Realized volatility and noise: the size of microstructure noise, the Epps effect, estimator selection, rough volatility, weak return predictability                            |
| `07_order_flow_and_toxicity.md`                | Order flow: persistence and long memory, the diffusive-price paradox, imbalance as a predictor, price discovery measurement, adverse selection and informed flow               |
| `08_econophysics_and_agent_based_markets.md`   | Stylized facts and simulation: power-law tails, Hawkes processes, long memory, zero-intelligence auctions, and why stylized-fact matching is weak validation                   |
| `09_machine_learning_for_trading.md`           | Machine learning: risk-adjusted benchmarks, book models, reinforcement learning and its live gap, LLM agents under bias-mitigated evaluation, leakage detection                |
| `10_cross_venue_and_arbitrage.md`              | Fragmentation: feed accuracy, dark pools, settlement latency and fees, triangular and cyclic arbitrage, AMM loss, and the fragility of multi-venue simulation                  |
| `11_market_design_fees_and_auctions.md`        | Market design: maker-taker fees and rebates, tick size, price limits versus circuit breakers, auction mechanics and duration, clearing and priority                            |
| `12_liquidity_and_systemic_risk.md`            | Collective risk: leverage-driven deleveraging, flash-crash propagation, liquidity stress testing, crowding, clearing biases, drawdown tails                                    |
| `13_crypto_amm_and_perpetuals.md`              | Crypto venues: AMM invariants and LP loss, perpetual futures mechanics, funding, stablecoin deleveraging, wash trading, prediction markets                                     |
| `14_news_and_language_model_signals.md`        | Text signals: what language models add over lexicons, the collapse of net performance with costs and turnover, timestamp and data-freshness leakage controls, capacity         |
| `15_portfolio_construction_under_frictions.md` | Allocation under frictions: liquidity-aware and impact-aware construction, cross-impact coupling, time-consistent liquidation, well-posedness, capacity                        |
| `16_options_and_derivative_instruments.md`     | Options and derivatives: hedging as a metaorder, joint quoting and hedging, hedging cost and margin in closed form, cross-impact, perpetual design and liquidation             |

## What the corpus says collectively

These are the findings that recur across independent slices, which is why they are worth acting on.

1. **Costs, turnover and latency decide deployability, not forecast accuracy.** Every slice that
   measured net-of-cost performance found it far below the gross figure: `14`, `09`, `16`, `03`.
2. **Average impact is concave in size, near a square root, and the mechanism behind it is not
   settled.** The exponent is robust across markets; the prefactor is uncertain by roughly a factor
   of two; `02`.
3. **Order flow is persistently autocorrelated, and prices stay diffusive only because impact is
   sublinear.** A linear-impact simulation manufactures predictable returns; `07`, `08`, `02`.
4. **Passive fills are adversely selected.** Fills line up with subsequent adverse moves, so
   full-fill or exponential-fill backtests overstate passive performance; `03`, `04`.
5. **Speed is ordinal.** What pays is latency rank against competitors, not absolute latency, and the
   profitable horizon for aggressive speed strategies is short and shrinking; `05`.
6. **Market data is not clean and must be treated as a risk.** Consolidated feeds reorder trades,
   cross-venue offsets are ambiguous at the tens-of-milliseconds level, and reported volume and open
   interest can violate their own identities; `05`, `10`, `13`.
7. **Simulation needs more than stylized-fact matching.** Parameter degeneracy, single-mean reporting
   and undocumented agent logic make agent-based conclusions fragile; validation needs nulls,
   rule and horizon robustness, and out-of-sample calibration; `08`, `12`, `10`.
8. **Text and machine-learning signals need explicit leakage controls.** Publication timestamps,
   model data-freshness cutoffs, chronological splits, and deflated significance move results from
   implausible to modest; `14`, `09`.
9. **Exchange design is part of the strategy's payoff function.** Fee, rebate, tick and halt rules
   change spreads, depth and even whether learned agents cooperate; they belong in the simulation,
   not in a footnote; `11`.
10. **Individually prudent risk limits do not prevent collective crashes, and can worsen them.**
    Non-monotone responses to participation limits and inventory caps appear in every flash-crash
    simulation; `12`.

## How to use these documents

- Treat the short answer at the top of each brief as the checklist for that topic, and the
  take-away table as the list of concrete places where this repository should change.
- Treat the caveats section as binding. These are mostly single-market, single-period studies, and
  the briefs say so where it matters.
- When a claim matters for a live decision, follow the id to the paper rather than trusting the
  brief. The page markers exist so that the step is cheap.
- Use `corpus_map.md` when a brief's caveats name a gap: the leads there are the papers that would
  fill it.

## Limits

Three limits apply to everything in this directory. First, the corpus is one arXiv category, so it
over-represents what academics find publishable and under-represents what venue operators, market
makers and execution desks know privately. Second, most results are from equities, index futures,
FX and crypto; prediction markets, options surfaces and fixed income appear through single papers
rather than bodies of evidence. Third, publication bias is real in exactly the subfields with the
most attractive reported numbers, which is the machine-learning and text slice; the brief for that
slice is written accordingly.

None of these documents contains trading advice or a profitability claim. They are statements about
mechanisms, measurement and evaluation design, and every one of them is falsifiable against the
cited paper.
