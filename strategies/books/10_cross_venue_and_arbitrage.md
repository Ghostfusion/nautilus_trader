# Cross-Venue Structure and Arbitrage: Fragmentation, Feeds and Net-of-Cost Opportunity

Date: 2026-10-05. Revision 1.

This brief covers the cross-venue and arbitrage slice of the corpus: 200 records
spanning 1999-2026, of which 23 carry a journal reference. It condenses twelve
papers read in depth out of 39 candidates screened, on fragmentation, consolidated
feeds, dark pools, settlement latency, cyclic and AMM arbitrage, and the limits of
multi-venue simulation.

## What this covers

The slice runs from the microstructure of fragmented lit markets to constant-product
AMMs and perpetual oracles. Its sub-topics are: whether fragmentation closes the law
of one price; the accuracy of consolidated feeds; dark-pool price discovery; settlement
latency, gas and fees as the binding constraint; triangular and cyclic arbitrage; AMM
liquidity-provider losses; and what multi-venue simulation can and cannot establish.
The most consequential thing here is that gross spreads are not profit: once gas,
settlement latency and fees are priced, many printed opportunities disappear, and the
estimators used to say "who leads" can be powerless when venues reference each other.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category | 200 |
| Years spanned | 1999-2026 |
| Records with a journal reference | 23 |
| Candidate papers screened | 39 |
| Papers read in depth | 12 |

Selection rule: take every required paper, add the substantive multi-venue or
AMM-arbitrage papers with real data, printed numbers or a directly relevant
execution result, and drop single-venue market making, generic statistical-arbitrage
ML, power or battery arbitrage, prediction-market credit design, quantum annealing
and option-surface construction.

## The short answer

1. A consolidated feed can misorder the tape: 66.2% of AAPL's 482,578 trades were out of sequence on 11 Aug 2015, and the count scales with volume at slope 0.66 (R-squared 0.99) (`1810.11091v1`).
2. Cross-venue agreement is not price discovery: opposite oracle topologies (W=0 vs a cycle with spectral radius 0.77) coincide to machine precision (max gap 1.1e-13) with proxy R-squared 0.9997 and lead-lag share 0.007 (`2608.09188v1`).
3. Settlement latency is a price-risk cost, not a delay: latency uncertainty is more than 40% of marginal arbitrage costs across 16 CEXes and 120 pairs (`1812.00595v4`).
4. Gas can exceed the deviation: on 5 Mar 2024 an ETH CEX-DEX arbitrage with a USD 35.04 deviation netted USD 0.11 after roughly USD 30.58 of gas paid twice, gas equal to 87.3% of the deviation (`2506.08718v1`).
5. DEX cyclic arbitrage is real but gross: 292,606 transactions over eleven months returned more than USD 138 million, of which 8,458 ETH went to gas (`2105.02784v3`).
6. AMM fees often do not cover adverse selection: in the WETH-USDC 5bp pool fees hover near 80% of arbitrage losses, while Uniswap v2 fees run about three times losses in the second year (`2404.05803v2`).
7. Faster blocks reduce LP losses: 100 ms versus Ethereum's 12 s cuts arbitrageur losses by 20% to 70% (`2404.05803v2`).
8. Fragmentation's sign is implementation-dependent: under an alternative greedy-strategy interpretation it decreases execution times in all experiments and raises welfare in most (`2604.20067v1`).
9. Dark-pool price discovery is conditional, not signed: adding a dark pool enhances discovery at high information precision and impairs it at low precision (`1612.08486v1`).
10. "Statistical arbitrage" is weaker than arbitrage: positive expected gain can coexist with no-arbitrage, so a screen must name its definition (`1907.09218v2`).

## 1. Fragmentation does not close the law of one price

One asset, many venues. `2604.20067v1` replicates Wah and Wellman's agent-based model
of latency arbitrage in a fragmented market: one asset, zero-intelligence traders, 1 or
2 continuous double auction exchanges, a consolidated SIP feed with a latency delta,
and an optional latency-arbitrage agent. The canonical means fall outside the
confidence intervals for every fragmented configuration, and further away when latency
is nonzero (Table 6, pp.16-17); latency-arbitrage surplus alignment is rejected in every
experiment (Table 7, p.17). Under an alternative greedy-strategy interpretation,
fragmentation decreases execution times in all experiments and increases trader
welfare in most (p.25). The sign of the fragmentation effect is not robust.

`1204.3422v3` makes the same point structurally in FX. Reformulating multiplicative
rate updates as max-plus matrix products, a 4-currency world tends to periodic or
exponential sequences and a 5-currency world admits a double exponential law, so
profitable arbitrage operations are endemic rather than self-extinguishing
(abstract p.1). A constructed product's spectral radius is the golden ratio
(1+sqrt5)/2 (pp.21-end). Persistent deviation, not transient only, is the baseline.

## 2. Consolidated feeds and the accuracy of the tape

`1810.11091v1` compares the consolidated Securities Information Processor feed to
direct exchange feeds for 14 stocks on 11 Aug 2015. Out-of-sequence reports reach
66.2% for AAPL (482,578 trades), 56.3% for GOOG, 50.1% for BAC and 43.0% for XOM,
falling to 10.3% for EYES, 0.3% for BRKA and 0.0% for ACU, with a linear
volume relation of slope 0.66 (R-squared 0.99, p.14). Median SIP latencies sit within
700 ms of each other across SIPs (p.8), and the latency window commonly contains
hundreds to thousands of events for AAPL (p.13). Crossed (negative spread) and locked
(zero spread) markets appear at the SIP while individual exchanges are never crossed
(pp.11-12); a cited arbitrage window may last 500 microseconds or less (p.14). The tape
can print an impossible state, so ordering and quote sanity are preconditions for any
cross-venue research.

## 3. Dark pools: sorting and conditional price discovery

`1612.08486v1` builds a noisy-rational-expectations equilibrium with trader sorting:
strong signals trade on the exchange, moderate signals in the dark pool and weak
signals do not trade (abstract p.1). Adding a dark pool enhances price discovery when
information precision is high and impairs it when precision is low (Prop. 4, p.36); the
likelihood that dark trading harms discovery decreases in precision and increases in
the relative measure of informed traders (Prop. 5, p.36). No single sign is claimed: the
paper reconciles conflicting prior empirical findings and offers testable predictions.

`1205.4008v4` asks the execution-model question rather than the information question.
With Almgren-Chriss impact at the traditional exchange and dark matching at the exchange
price, regularity can be excluded only by rather unrealistic parameter choices (p.2), and
generically any genuine price impact yields profitable manipulation unless remaining
parameters are extreme (p.2). Price manipulation in an impact model is the analogue of
arbitrage in a pricing model, and belongs in the execution criteria.

## 4. Crypto cross-venue: settlement latency, gas and who leads

`1812.00595v4` uses minute-level BTC/USD books from 16 centralized exchanges, 120
feasible exchange pairs and 3.9 million cross-exchange transactions averaging USD 72
million daily. Settlement-latency uncertainty contributes more than 40% of marginal
arbitrage costs (p.5); price differences coincide with high settlement latency, high
latency variance and high spot volatility, and are narrower between exchanges with more
funds under custody (pp.5-6). Exchange-held Bitcoin exceeded USD 12.4 billion in Oct
2019, up 25.8% from Jan 2018 (p.5). Pre-positioned inventory removes latency risk but
adds exchange default risk.

`2506.08718v1` measures CEX versus DEX for ETH on five 2024 event days. Hasbrouck
centralized information shares are 0.959, 0.965, 0.628, 0.986 and 0.995 (Table 3.19,
p.66); Gonzalo-Granger alpha vectors show the decentralized market reacting more
(pp.54-66); Hayashi-Yoshida lead-lag ratios of 1.86, 1.47, 1.22, 1.20 and 1.11 all
indicate the centralized venue leading. But `2608.09188v1` proves the estimators can
be powerless: under self or peer reference the reduced form does not identify
anchoring (Theorem 1, p.7) and lead-lag, Granger, common-factor and information-share
statistics have power equal to size (Theorem 2, p.8). The settings differ, spot
CEX-DEX versus closed-window oracle marks, but a measured leader may be an artefact.

## 5. Cyclic, AMM and LP economics

`2105.02784v3` measures Uniswap V2 over eleven months: 292,606 cyclic arbitrage
transactions and more than USD 138 million in revenue, with 8,458 ETH paid as gas
(p.2). The most profitable unexploited opportunity stays above 1 ETH (about USD 4,000)
in almost every block, and total exploitable revenue stays above 10 ETH over four
months (p.2); only 0.03% of traders route cyclically through multiple transactions,
the rest executing atomically in one blockchain transaction (p.2), which mitigates
price-impact risk. A worked example routes 285.71 USDC through four markets and
returns 303.68 USDC, a 17.97 USDC profit (Figure 1, p.2).

`2507.08302v3` shows the realised margin is set by a priority-fee auction: pure
symmetric equilibria do not exist, only mixed equilibria can be characterised
(Section 3), and empirically gas fees increase with the price discrepancy and
liquidity level (p.4). No-revert versus auto-revert versus selectable-revert settings
change both arbitrageur profit and market efficiency (pp.4-5). Against both, `2404.05803v2`
turns to the liquidity provider: in the WETH-USDC 5bp pool fees hover near 80% of
arbitrage losses (Figure 1b, p.3), Uniswap v2 fees run about three times losses in the
second year (p.3), and faster blocks (100 ms versus 12 s) cut arbitrageur losses by 20%
to 70% (first page). The apparent disagreement with `2105.02784v3` is about gross
versus net measurement.

## 6. What multi-venue simulation can and cannot establish

`2604.20067v1` is the cautionary case. With no original seeds, numerical identity is
impossible, and only mean surplus was reported, so distributional equivalence is not
testable. A one-sample t-test on the reported mean falsely rejects 17.0-48.8% of the
time at rho=0.05 and 6.3-35.9% at rho=0.01 (Table 5, p.14); a bootstrap method
false-rejects 4.7-7.8% at the 95% CI (Table 5, p.15). The released MarketSim codebase
yields zero-intelligence surplus about 10% above the original even for CDA, with
alignment rejected for all experiments (Table 8, pp.18-19). A hybrid BestGuess+MS
achieves relational equivalence for all three environments but still rejects
latency-arbitrage alignment everywhere (Table 10-11, pp.23-24). Pin and version the
micro-decisions, and report distributions rather than single means.

`1907.09218v2` anchors the definitional question. If an equivalent martingale measure
has a G-measurable density then NSA(G) holds (Prop. 3.1, p.3). Simulation of a
follow-the-trend strategy over 1 million runs gives gain per annum 27.8, median 164
and 0.95 VaR 4,180 (Table 5, p.21); the market-data application to Kellogg and
Deutsche Bank yields positive gains at every boundary, with non-optimal performance at
too-small or too-large boundaries (Table 13, p.29). Evaluation claims must name the
definition, because generalized gain is not classical arbitrage.

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| SIP can misorder most high-volume trades | python/nautilus_trader/data | Validate event sequence before research |
| Consolidated feed prints locked/crossed quotes | data layer | Reject impossible spreads as errors, not signals |
| Direct-feed latency monetises dislocations | python/nautilus_trader/adapters | Model direct-feed latency, not only consolidated |
| Settlement latency is a price-risk cost | python/nautilus_trader/execution | Price latency risk and cap executable size |
| Gas equals 87.3% of a small deviation | execution layer | Net fees and gas before ranking opportunities |
| AMM fees below arbitrage losses | python/nautilus_trader/analysis | Model discrete-block rebalancing for AMM valuation |
| Block time changes LP losses by 20-70% | python/nautilus_trader/backtest | Use per-chain block-time assumptions |
| Simulation micro-decisions flip results | python/nautilus_trader/testkit | Version routing logic; bootstrap and report distributions |
| Price-discovery estimators can be powerless | python/nautilus_trader/analysis | Stress-test information share for observational equivalence |
| Dark-pool fill probability gates execution | docs/concepts/execution | Model fill probability; venue choice is informational |
| Impact models can be manipulable | risk layer | Add no-manipulation regularity checks to execution models |
| Statistical arbitrage is not arbitrage | docs/concepts/backtesting | State which definition a screen uses |

## Caveats

The evidence does not establish that any of these results transfer to a different
market or period. `1810.11091v1` is one trading day, 14 stocks and one 2015 hardware
regime; `2506.08718v1` is five event days, and its 20 May pair even failed the rank-1
cointegration test; `2105.02784v3` is a single DEX protocol with no CEX-DEX
competition. Several results are theory without data: `1612.08486v1` reports no
estimated magnitudes, `1205.4008v4` has no empirical test, and `1204.3422v3` ignores
bid-ask spreads and transaction costs entirely. Costs are absent or partial in places:
`1907.09218v2` excludes transaction costs and normalises to one traded asset, while
`2404.05803v2` excludes gas and MEV. Some findings are contested rather than settled:
the dark-pool sign is conditional on the information environment, DEX opportunity is
gross versus net depending on the study, and a measured price leader can be an
artefact of feedback rather than a fact about the market. No study here provides a
clean out-of-sample test of a cost-gated cross-venue strategy.

## Papers read in depth

- `2604.20067v1` - Testing replication for an agent-based model of market fragmentation and latency arbitrage (2026). A published latency-arbitrage model was not quantitatively replicated; conclusions hinge on undocumented logic.
- `1612.08486v1` - Understanding the Impacts of Dark Pools on Price Discovery (2016). Theory: dark-pool effects on discovery are conditional on information precision.
- `1810.11091v1` - Price Discovery and the Accuracy of Consolidated Data Feeds in the U.S. Equity Markets (2018). The consolidated tape misorders up to two thirds of high-volume trades.
- `2608.09188v1` - When Cross-Venue Agreement Is Not Price Discovery (2026). Under cross-venue feedback, standard discovery estimands lose all power.
- `2506.08718v1` - Price Discovery in Cryptocurrency Markets (2025). CEX leads DEX in ETH; gas can erase the entire deviation.
- `1204.3422v3` - Double Exponential Instability of Triangular Arbitrage Systems (2012). Triangular arbitrage can grow rather than self-extinguish; loop guards needed.
- `1907.09218v2` - Generalized statistical arbitrage concepts and related gain strategies (2019). Positive expected gain can coexist with no-arbitrage.
- `1812.00595v4` - Building Trust Takes Time: Limits to Arbitrage for Blockchain-Based Assets (2023). Settlement latency is over 40% of marginal arbitrage cost.
- `2105.02784v3` - Cyclic Arbitrage in Decentralized Exchanges (2021/2022). Persistent unexploited DEX opportunity, mostly taken atomically.
- `2507.08302v3` - Arbitrage on Decentralized Exchanges (2025/2026). DEX margin is set by a priority-fee auction, not the raw gap.
- `2404.05803v2` - Measuring Arbitrage Losses and Profitability of AMM Liquidity (2024). Fees often fail to cover LVR; block time matters.
- `1205.4008v4` - Price manipulation in a market impact model with dark pool (2012/2014). Dark-pool impact models are generically manipulable.

## Where to next in the corpus

- `1112.5850v1` - four-currency periodic arbitrage; predecessor of `1204.3422v3` for the d-currency result.
- `2305.14604v2` - AMM arbitrage profits with fees; theory counterpart to the empirical LVR study.
- `2507.02027v2` - LVR with bounded liquidity; adjacent theory to `2404.05803v2`.
- `2602.22069v1` - dynamic-weight AMM LVR; a variant not covered by the full-range study.
- `2002.02583v1` - agent-based triangular arbitrage with cross-currency correlations; simulation adjacent to `1204.3422v3`.
