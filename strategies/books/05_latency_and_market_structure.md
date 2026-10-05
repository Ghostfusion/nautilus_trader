# Latency and Market Structure: The Economics of Speed

Date: 2026-10-05. Revision 1.

This brief covers the latency-and-market-structure slice: 348 harvested records spanning 1998-2026, of which 39 candidate papers were screened and 12 read in depth. The slice covers the economics of speed, measured latency budgets and engine design, latency arbitrage, speed bumps, timestamp integrity, and the profitability ceiling of latency-dependent strategies.

## What this covers

The slice has five load-bearing sub-topics: whether speed value is ordinal or absolute; the software and hardware levers that set a microsecond-level latency floor; the way true latency enters as a causality and timestamp constraint; the measured effects of venue design (speed bumps, fragmentation, on-demand capacity); and the profit ceiling of speed-dependent strategies. The single most consequential result is that a latency edge is insurance against competitors, not a clock reading: ranking first among peers with the same signal pays, while shaving nanoseconds after you are already fastest does not. The second consequential result is that the feed, not the concurrency machinery, sets the tail.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category in the harvest | 348 |
| Years spanned | 1998-2026 |
| Records with a journal reference | 54 |
| Candidate papers screened | 39 |
| Papers read in depth | 12 |

Selection rule: keep papers carrying a quantitative claim about low-latency engineering design, the measured economic value or effect of speed, speed bumps and fragmentation, timestamp or clock accuracy, or the measured activity and profitability of latency-sensitive trading.

## The short answer

1. Latency value is ordinal: the closest order-book-imbalance liquidity trader averaged $2,681.04 per day while the second closest lost $3,297.30, so the edge lives in rank, not in nanoseconds (`2006.08682v1`).
2. In a stylized fragmented-venue model the bid-ask spread depends only on relative speed, not absolute latency (`1907.10720v1`).
3. Synchronous in-process message delivery runs in tens of nanoseconds, roughly two orders of magnitude below the ungrouped asynchronous path (`2609.21173v1`).
4. Cache warming cut a microbenchmark from 267,685,006 ns to 25,635,035 ns, about 90%, while the cache-miss rate barely moved (`2309.04259v1`).
5. A lossless ring buffer beat std::queue by 38% on average, from 11.9% at 10 events to 55.2% at 10,000 events (`2309.04259v1`).
6. Aggressive-HFT upper-bound profit for 19 NASDAQ stocks fell from $3.4 billion at a 10 s holding period to $62,000 at 10 ms (`1007.2593v2`).
7. Between 10% and 15% of Eurex BUND events occur less than 250 us after the previous event, inside the causality window and therefore not attributable to it (`2101.06348v1`).
8. Cross-venue lead-lag carries a single-vantage constant-offset ambiguity of about +/-99 ms even when drift is bounded to at most 6 ms (`2607.26245v1`).
9. Asymmetric speed bumps cut low-latency investment by 20% in a laboratory market, while a symmetric bump was neutral (`1910.03068v1`).

## 1. Speed is ordinal, not absolute

The strongest single piece of evidence is an agent-based study of order-book-imbalance trading (`2006.08682v1`). A preliminary experiment found a strong inverse Pearson correlation of r = -0.775 between absolute latency and profit (p.4). The rank experiment then showed the closest liquidity trader earning a mean $2,681.04 per day (std $1,389.37), the second closest losing $3,297.30 (std $1,661.30), and the furthest losing $24,473.64 (p.6). The closest trader rarely lost and the second closest rarely made money regardless of absolute distance (p.6). Crossing the control trader's latency transferred profit immediately, while further absolute reduction once faster than the control changed nothing (p.5). The paper resolves its own tension by treating rank as causal, but the preliminary absolute-latency correlation and the rank result are still in tension as readings.

Theory reaches the same place. In the centralized-versus-decentralized exchange model of `1907.10720v1`, the bid-ask spread depends only on relative speed and is the same in both market structures (p.5); low-latency sprints speed up price discovery without harming liquidity (p.5). The laboratory market of `1910.03068v1` is consistent: competitors, including a computer market maker, matter as much as any single trader.

The mechanism behind the rank result is winner-take-all allocation: when many traders act on the same stale quote or imbalance, only the first order to reach the market is filled, so the marginal value of being faster collapses once a trader is no longer first. That is why `2006.08682v1` reports the closest trader rarely losing and the second closest rarely making money at any absolute distance (p.6), and why crossing the control's latency transfers profit immediately rather than gradually (p.5). For a backtest this means the competitor latency distribution is a first-class input, not a sensitivity knob.

## 2. Measured latency budgets and engine design

Two engineering papers bracket the software budget. `2609.21173v1` adapts the actor model with four HFT-specific mechanisms: fast_send, where the sender's thread runs the receiver's handler inline under the receiver's exclusive lock and returns the reply as a value; actor groups co-scheduled behind one shared mailbox; per-actor mailbox queue selection; and memory pooling. A synchronous round trip is tens of nanoseconds, roughly two orders of magnitude below the ungrouped asynchronous path (p.1). Drain-side batching (1.63x) plus memory pooling raise asynchronous throughput 2.46x, and the pool gives a two-order-of-magnitude reduction in the latency-tail maximum (p.1, p.3). Live CME socket-to-book latency decomposes into a roughly 7 us decode-and-book floor plus a per-message slope, with the framework's own contribution under 1% of that floor (p.1, p.3). The tail is set by the feed, in-packet position and mailbox backlog on bursts, attributed to the non-Poisson clustered arrival process (p.4). Survey Table 1 shows that for every mainstream actor runtime surveyed, the sender's thread does not run the receiver's handler (p.5).

`2309.04259v1` enumerates the C++ levers one at a time. Cache warming: 267,685,006 ns cold versus 25,635,035 ns warm, about 90%, while the miss rate moved only from 73.96% to 71.56% (p.12). Constexpr factorial(10): 0.245 ns versus 2.69 ns, about 90.88% (p.14). Loop unrolling 72.24% (p.16); short-circuiting about 50% (p.16); slowpath removal about 12% (p.17); branch reduction about 36% (p.18); prefetching about 23.5% (p.19); mixing float/double about 52% (p.21); SIMD about 49% (p.22); lock-free atomic versus mutex increment about 63% (p.23). A Disruptor ring buffer beat std::queue by 38% on average, from 11.9% at 10 events to 55.2% at 10,000, and 884,871,405 ns versus 543,171,556 ns at 1,000,000 events (p.35). Combining the optimisations cut latency 87.33% (517,559 ns to 65,588 ns) with the standard deviation falling from 4,233 to 400 ns (p.31, p.36). Both papers agree that memory behaviour and the feed path, not the concurrency model, set the floor and the tail.

The engineering numbers carry a methodology warning of their own. `2309.04259v1` cites a case where clock_gettime appeared 85x slower than TSC on a benchmark site but only 2x on the real server (p.5), and its benchmarks isolate CPU-bound work without network, co-location, or hardware timestamping (p.8). A microsecond win measured on a laptop is not a production win: the benchmark context must match deployment, and the clock source itself is part of the measurement.

## 3. True latency as a causality and timestamp boundary

`2101.06348v1` shifts exponential Hawkes kernels right by the latency so an event can only influence another if their central-book time difference exceeds it. On Eurex BUND futures over 20 days of April 2014 with a 250 us latency, the latency itself determines most of the decays (p.1), and 10% to 15% of events in each series occur less than 250 us after the previous event and so should not be attributed as caused by it (p.14). Estimated exogeneity ratios are much higher than in Bacry et al. 2016, explained as reassignment of events within the latency (p.15). T-to-P and P-to-P kernel decays are about 3000 in latency units, matching Bacry et al. 2016 after about 250 us (pp.15-16), and the latency-aware likelihood removes the need for negative P-to-T intensities (p.16). The cost is speed: at end time 10000 the likelihood took 6.02 s versus 143 s builtin and 7.29 s versus 181 s custom (p.7). The lesson is that any model, simulator, or signal fitted on order-book events must encode latency as a hard cutoff or it will overstate endogeneity.

`2607.26245v1` attacks the timestamp side directly. A synchronized Polymarket-Binance corpus of 727,098,247 deduplicated rows over 54 Polymarket days shows an apparent 16 ms median source-clock lag, drift bounded to at most 6 ms over the archive, but a single-vantage constant-offset ambiguity of roughly +/-99 ms (p.1, p.8). Per-day minimum transport-delay envelopes were 99 ms for Binance and 7 ms for Polymarket, stable to within 4 ms and 2 ms (p.8). A synchronization-free event study on the collector clock showed Polymarket quotes responding to Binance moves of at least 5 bps within 1 s after a median 347 ms, on 4,272 matched moves out of 15,148 detected (p.8). Lead-lag is an event-timing difference and not an information-share measure, and the unresolved constant offset can shift it wholesale.

The practical consequence is that feed-accuracy metrics belong in the data layer as first-class checks. `2607.26245v1` stores both source and ingest timestamps for every record, validates clock offset, and bounds drift, which is how it can report a null result with a stated uncertainty rather than an apparent 16 ms edge. Any cross-venue engine should track drift, offset ambiguity, dropped messages, out-of-order events, and pairing-window sensitivity before it trusts a lead-lag or an information-share number.

## 4. Venue design: speed bumps, fragmentation, on-demand capacity

`1910.03068v1` ran a 32-round oTree laboratory market with 56 undergraduates. Without a speed bump, traders invested 75% of their endowment in speed; with a bump, 66%, a 12% decrease (p.5, p.19). Asymmetric bumps reduced low-latency investment by 20% (p.1) or 20.6% (p.19). Raising bump magnitude by one standard deviation, 2 seconds or 40% of unconditional exchange latency, cut investment a further 8.33% (p.1, p.6). A symmetric bump yielded the same investment as no bump at all (p.1, p.19). No significant difference between random and deterministic bumps was found, against a model prediction (p.6), a direct dispute with the theory that random delays stimulate speed investment. Real-world designs referenced include IEX and NYSE American at 350 us on all orders, TSX Alpha randomized 1-3 ms, a Cboe proposal of random 3-4 ms, and Eurex at 1 or 3 ms (p.3).

`2604.20067v1` independently replicated Wah and Wellman's 2016 agent-based multi-venue latency-arbitrage model, running 500,000 runs per experiment against the original 50,000 (p.5). Quantitative alignment was not rejected only for the consolidated single-exchange configuration and was rejected for every fragmented configuration, with divergence growing as latency became non-zero (p.17). All bootstrap confidence intervals for the latency-arbitrageur surplus excluded the original means and were much lower (p.17). The replication also rejected alignment between the original authors' released MarketSim codebase and the published results. Methodologically, a naive one-sample t-test self-alignment falsely rejected 17.0-48.8% of the time, versus a bootstrap 95% CI false-rejection rate of 4.7-7.8% (p.15). Under an alternative interpretation of the zero-intelligence greedy strategy, fragmentation decreased execution times in all experiments and increased welfare in most (p.1). Undocumented implementation choices flip qualitative venue conclusions.

`1907.10720v1` adds the capacity dimension: on a decentralized exchange, HFTs acquire more speed but for shorter timespans, speed rents fall, and fewer resources are locked in (p.1). Its cited context is a stress-test menu: 20% of trades cluster in sub-millisecond intervals, median arbitrage duration fell from 97 ms in 2005 to 7 ms in 2015, co-location fees ran $874M-1024M in 2018, average throughput was 3.21M messages/s surging to 25.2M in bursts, and about 90% of exchange infrastructure was idle about 90% of the day (pp.3-4).

## 5. The profitability ceiling and the horizon/cost tension

`1007.2593v2` uses an omniscient trader with perfect foresight on 2008 NASDAQ message data for the 19 most-liquid stocks. Its upper bound for the entire US equities universe is $21 billion at a 10 s holding period, down to $21 million or less at 10 ms (p.1). For the 19 stocks, 2008 profits were only $3.4 billion at 10 s, falling to $62,000 at 10 ms (p.7). The 1 s-to-10 s ratio was 0.036 in profitability, 0.13 in trades, and 0.039 in shares (p.8); composite all-venue profits averaged 2.1x primary-exchange profits (p.11). The regression power law had R^2 = 0.968, and 6,279 US stocks imply $21.3 billion at 10 s (p.12). Just 5 of 19 stocks accounted for 73% of profits while the worst 10 accounted for 13% (p.9). Because prediction difficulty, market impact, and fees are all removed, these are gross overestimates. `2407.21025v2` makes the cost side explicit: as sampling frequency 1/Delta rises, learning error falls but sample complexity rises in proportion to transaction costs, since each iteration is a quote (p.12).

The same paper shows that a discrete-time grid is theoretically safe when the grid is fine: the discrete MDP converges to the continuous MDP with error bounded by O(Delta) (pp.9-10), and the two-player Nash equilibrium of the discrete game converges to the continuous-time equilibrium as Delta shrinks (p.15). The sample-complexity bound is polynomial in all parameters and grows as Delta shrinks (pp.11-12), so the approximation error and the quoting cost pull in opposite directions. That is a design rule for a tick-grid simulator: choose the grid for the error/cost balance, not for convenience.

## 6. Latency as risk: adverse selection and stale quotes

`1806.05849v3` models a single-venue market maker as a finite-horizon MDP with constant absolute latency. Market making is profitable iff the rate of uninformed market orders hitting the quotes exceeds the rate of adverse price-jump fills and the horizon is long enough (p.1, p.3); latency is an additional source of risk, with loss on outstanding and newly sent orders rising with it (p.1, p.3). `1610.00261v4` works the passive side: the added value of exploiting liquidity imbalance is eroded by latency, because predicting future liquidity-consuming flow is less useful when there is no time to cancel and reinsert (p.1, p.9). Empirically, high-frequency proprietary buy limit orders execute after the price has already risen, while institutional brokers buy into falling prices (p.9), and HF proprietary traders suffer less adverse selection than HF market makers (p.9). The benchmark is the microprice, the expected future price given imbalance, rather than the current mid (p.2, p.12). Together the two papers say a market-making engine should carry quote staleness as an explicit risk state and should value passive fills against a microprice, not raw mid.

`1806.05849v3` formalises the fill side with an "order value" that plays the role of a one-period reward and gives explicit criteria for positive order value under latency (pp.2-3). `1610.00261v4` supplies the empirical counterpart: high-frequency proprietary traders suffer less adverse selection than high-frequency market makers (p.9), which says the risk is concentrated in the quoting style rather than the venue. The two together turn latency from a speed parameter into a term in the risk calculation, and they give a strategy-viability test that couples fill rate to adverse-jump rate.

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| Speed value is ordinal and winner-take-all | docs/usermanauls/microstructure-signals | Model competitor latency rank, not own latency alone, in signal backtests |
| Non-Poisson burst arrivals set the feed tail | docs/concepts/data | Model clustered arrivals and mailbox backlog; add burst stress tests |
| Roughly 7 us decode-and-book floor | docs/concepts/backtesting | Use an order-of-magnitude ingest floor when simulating market-data latency |
| Ring buffer beats mutex queue | docs/concepts/message_bus | Use a lock-free internal bus for the hot path |
| Cache locality and compile-time dispatch dominate | docs/concepts/rust | Invest in memory layout and compile-time dispatch, not micro-tweaks |
| Latency is a hard causality cutoff | docs/usermanauls/microstructure-signals | Encode a latency window so same-window events cannot be treated as causes |
| Clock drift and constant offset ambiguity | docs/concepts/networking | Make drift, offset ambiguity, out-of-order and dropped messages first-class checks |
| Horizon versus cost binds aggressive execution | docs/usermanauls/execution-algorithms | Longer holding periods clear costs; short horizons need larger moves |
| Microprice beats mid for passive fills | docs/concepts/execution | Value fills against imbalance-adjusted microprice |
| Quote staleness is a risk state | python/nautilus_trader/risk | Track stale quotes as explicit risk, not just a speed parameter |
| Quoting frequency is an error/cost tradeoff | docs/usermanauls/market-making | Choose sampling frequency as an explicit tradeoff |
| Simulator runs need distributions, not means | docs/concepts/optimization | Report bootstrap confidence intervals across many runs |

## Caveats

Most of the slice is simulation or theory: `1910.03068v1`, `2006.08682v1`, `2604.20067v1`, `1907.10720v1`, `1806.05849v3`, `2407.21025v2`, and the simulation side of `2101.06348v1` are stylized or laboratory settings, so their magnitudes are not real-market estimates. Only `1007.2593v2`, `1610.00261v4`, and `2607.26245v1` use primary market data, and `2309.04259v1`'s strategy backtest runs on daily closes with no transaction costs or latency modelling. `1007.2593v2` deliberately removes prediction difficulty and market impact and excludes fees, so its $21 billion is an upper bound rather than a profit figure. `2604.20067v1` shows that ABM conclusions flip with undocumented strategy details, and that the original results it replicated could not be distributionally matched because only means were reported. `2607.26245v1` publishes a null out-of-sample cross-venue result and quantifies a +/-99 ms constant-offset ambiguity, which bounds how far any cross-venue lead-lag claim can be pushed. Sample sizes are thin in places: `1910.03068v1` used 56 students in three-person groups over 32 rounds, and `1610.00261v4` studies one stock on one venue. Several results are internal to a single implementation or machine, since `2309.04259v1` notes that benchmark context can differ 85x from production and that its numbers are machine- and compiler-specific. None of the engineering papers demonstrates the optimised stack carrying live trading at saturating throughput, and none establishes a causal link from latency investment to real firm profitability.

## Papers read in depth

- `2609.21173v1` - Adapting the Actor Model of Concurrency for High-Frequency Trading: Synchronous Message Delivery (fast_send) and a Tick-to-Book Latency Study (2026). Shows synchronous in-process delivery and a roughly 7 us feed floor, with the tail set by the clustered feed rather than the actor machinery.
- `2309.04259v1` - C++ Design Patterns for Low-latency Applications Including High-frequency Trading (2023). Quantifies the software levers and shows cache and memory behaviour dominate.
- `1910.03068v1` - Do speed bumps curb low-latency trading? Evidence from a laboratory market (2019). Finds modest, partly asymmetric effects and no experimental random-versus-deterministic difference.
- `2604.20067v1` - Testing replication for an agent-based model of market fragmentation and latency arbitrage (2026). Fails to replicate fragmented-configuration results and faults single t-test alignment methodology.
- `2101.06348v1` - Exponential Kernels with Latency in Hawkes Processes: Applications in Finance (2021). Makes latency a hard causality cutoff and shows 10-15% of events fall inside it.
- `2006.08682v1` - The Importance of Low Latency to Order Book Imbalance Trading Strategies (2020). Establishes that speed value is ordinal through a rank experiment in simulation.
- `1907.10720v1` - Liquid Speed: On-Demand Fast Trading at Distributed Exchanges (2019). Argues spread depends only on relative speed and supplies micro-burst capacity statistics.
- `1806.05849v3` - Optimal Market Making in the Presence of Latency (2020). Derives a formal fill-versus-adverse-selection profitability condition with latency as risk.
- `1610.00261v4` - Limit Order Strategic Placement with Adverse Selection Risk and the Role of Latency (2018). Shows latency erodes imbalance-signal value and motivates the microprice benchmark.
- `2407.21025v2` - Reinforcement Learning in High-frequency Market Making (2024). Links sampling frequency to transaction-cost complexity in a two-player game.
- `1007.2593v2` - Empirical Limitations on High Frequency Trading Profitability (2010). Bounds aggressive-HFT profit and quantifies the horizon/cost tension.
- `2607.26245v1` - OpenMarket: A Synchronized Polymarket-Binance Dataset for High-Frequency Prediction-Market Research (2026). Publishes a null cross-venue result and a quantified clock-offset ambiguity.

## Where to next in the corpus

- `2511.02136v1` - JaxMARL-HFT GPU environment; RL methodology rather than latency engineering.
- `2009.04200v1` - crypto intraday HFT patterns; empirical description without a latency measurement.
- `2302.11822v1` - multi-kernel Hawkes price dynamics; timescales without latency or clock treatment.
- `1311.4160v1` - synchronizing HFT across markets; price synchronization rather than latency measurement.
- `1211.1919v1` - NASDAQ price synchronization; real data but no latency measurement.
- `2010.13038v1` - HFT impact on artificial-market liquidity; ABM liquidity indicators only.
- `0906.1444v1` - microstructure noise estimates; data-quality adjacent but not timestamps or clocks.
- `2512.11765v1` - transient price impact game; optimal execution theory without a latency angle.
