# Crypto Venues, AMMs and Perpetual Futures

Date: 2026-10-05. Revision 1.

This brief covers 232 records on crypto venues, automated market makers and perpetual futures,
spanning 2012-2026, of which 25 carry a journal reference. From 40 candidates I read 12 papers in
depth on CFMM invariants and arbitrage-freeness, LP loss versus arbitrage extraction, Uniswap v3
data, perpetual funding and open interest, stablecoin deleveraging and collateral chains,
wash-trading diagnostics, and prediction markets as instruments.

## What this covers

The slice answers five questions a crypto execution platform must ask: what makes an AMM
arbitrage-free, who loses to arbitrageurs and how much, what concentrated-liquidity venues look like
at event level, how perpetual funding and open interest behave, and where DeFi leverage unwinds. The
most consequential finding is that the inputs most platforms treat as ground truth - open interest,
volume, and the cost of an on-chain leg - each fail a consistency or realism test when examined
directly. Venue speed, fee regime and funding clock are first-order drivers of which trades exist.

## The corpus slice

| Metric | Value |
|---|---|
| Records matching the category | 232 |
| Years spanned | 2012-2026 |
| Records with a journal reference | 25 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

The selection rule took the mandated works on CFMM invariants, LVR, Uniswap v3 empirics,
perpetual open interest, perpetual simulation, fee effects and CEX-DEX subslots, then added five
papers for direct platform relevance: stablecoin deleveraging, debt-financed collateral,
perpetual volatility transmission, wash trading and prediction markets. Forecasting, ML and
generic survey work was dropped.

## The short answer

1. Every order-continuous market system admits an invariant, and a DeFi system is arbitrage-free
   exactly when that invariant is strictly increasing (`2310.09782v2`).
2. Fee-collecting AMMs are provably incomplete and admit multiple invariants, so accounting
   belongs on per-LP-share balances, not raw reserves (`2310.09782v2`).
3. Two constant-product pools that are each individually arbitrage-free can be jointly exploited,
   so venue-level safety does not compose (`2310.09782v2`).
4. On two live dynamic-weight pools, extraction hit 88.6% of the theoretical optimum in one
   window, then fell to $22.00 against a $35.82 optimum in another (`2602.22069v1`).
5. ByBit's BTC linear perpetual showed excess open-interest variation of $15.34B in one month and
   $56.84B across July-September 2023, so venue OI is untrustworthy (`2310.14973v2`).
6. Perpetual funding times create predictable liquidity and volatility spikes in the first five
   minutes of 00:00, 08:00 and 16:00 UTC (`2107.00298v3`).
7. Moving from 12-second to 1-second subslots raised simulated CEX-DEX arbitrage transaction
   counts by 535% and volume by 203%, but the model assumed zero gas (`2601.00738v1`).
8. Stablecoin leverage unwinds non-linearly: a worked spiral has ETH 85 -> 81 force three
   liquidation waves and push the stablecoin price to 1.126 (`1906.02152v3`).

## 1. Invariants and arbitrage-freeness for AMMs

`2310.09782v2` proves every order-continuous market system admits an invariant (p.12, Thm 3.6): a
DeFi system is arbitrage-free exactly when it has a strictly increasing invariant (p.14, Thm 3.12 /
Thm 7.1), and a CFMM has a unique invariant exactly when complete (p.21, Thm 5.1).

Constant-product with a 0.3% fee has invariant x1*x2 (p.4, Example 2.2), and Uniswap v2 with LP
operations has increasing invariant x1*x2/x^2 (p.16, eq. 4.8). Example 4.9 (p.17) has two
independent constant-product pools on the same assets admitting collective arbitrage, (8,2,1,9) to
(4,4,3,3), though each pool alone is arbitrage-free; a stylized Uniswap v3 network is arbitrage-free
against an LP-weighted dominance order (p.19, Prop 4.11), but typical fee-collecting CFMMs are
incomplete and admit multiple invariants (p.21, Remark 5.2).

## 2. LP loss versus arbitrage extraction

`2602.22069v1` measures extraction on two live dynamic-weight pools: Safe Haven on Ethereum mainnet
(~$300k TVL) and Base Macro on Base L2 (~$50k TVL). Safe Haven ran 20 trades over 606 blocks in July
2025, extracting $51.55 against a $58.19 optimum, 88.6% efficiency (p.5); in January 2026 it ran 78
trades over 591 blocks, with per-trade extraction $2.58 -> $0.28, total $51.55 -> $22.00, optimum
$58.19 -> $35.82, max allocation drift 0.65% -> 0.45% and blocks between trades 30 -> 7.5 (pp.5-6).

Base Macro's January window shows 202 balance changes from 225 transactions, mean per-trade
extraction $0.0004, 98% of trades under one cent and many below zero (p.6); over Aug 2025 to Jan 2026
it beat frictionless LVR by roughly 27 percentage points and RVR by roughly 43 (p.7). All-in cost per
transaction was $1.08 in July, 82% base fee, and $0.20 in January, 59% priority fee (pp.11-12).
Arbitrage profit is quadratically flat, so arbers under-trade at 54-71% of optimal size for 81-99% of
the profit.

## 3. What concentrated liquidity looks like in the data

`2301.13009v2` characterises Uniswap v3 from a 15 Nov 2022 snapshot, filtering 6,000+ pools to 34
liquidity-taker-relevant and 19 liquidity-provider-relevant pools over Jan-Jun 2022, with 282 pools
passing a $1M proxyTVL screen. Clustering takers through a modified, time-weighted graph2vec
embedding gives seven clusters sized 304/142/512/978/379/186/914, with adjusted Rand indices about
0.75 (8 dimensions) and about 0.90 (16 against 32/64), so at least 16 dimensions are needed (p.13);
clusters differ by pool focus, fee tier (500 vs 3000 vs 10000), trade size and inter-trade time
(pp.14-15).

The "ideal crypto law" P_vol * V_stab = n_fee * R_pool * T_liq regresses daily volume on fee tier,
pool constant and TVL; "cryptoness" is its R^2, averaging 0.44 for stablecoin-only pools against 0.21
for others (p.20), and highest for WETH-CRV/10000 (pp.20-21). Daily swaps vary more than 100x, 6,181
per day in USDC-WETH/500 versus 32 in SHIB-WETH/10000, while USDC-WETH/3000 shows about 95 mints and
70 burns per day (p.21).

## 4. Perpetual futures: funding, premium and open interest

`2310.14973v2` gives a data-integrity identity: total volume must satisfy V >= |O_{ti+1} - O_ti| (eq.
1), with excess open-interest variation XTV = max(OTV - VT, 0) (eq. 6). ByBit's BTC linear perpetual
shows OTV of $45.66B against volume of $30.32B in Jan 2023, excess $15.34B, while other exchanges
reconcile (p.6, Table 2); in Jul-Sep 2023 it shows $129.48B against $72.64B, excess $56.84B, with OKX
at $17.36B and Binance's inverse contract at $38.2M (p.6, Table 3). Believing the reported OI implies
volume above $128bn, while healthy OTV/VT ratios imply $156-213bn, "highly improbable" (pp.5-6).

Excess is positive on 100% of 1-day, 99.8% of 1-hour and 75.8% of 1-minute windows, with
E[XTV|XTV>0] = $627k at one minute (p.7, Table 5); only HTX, Kraken and partly BitMEX reconcile.
`2107.00298v3` supplies the clock: the tether-margined perpetual is the dominant volatility emitter
over Jan-Mar 2021 (p.4), with ADV $15,006M and trading in 99% of one-second intervals against
Coinbase's $1,188M (p.10, Table 2), and volume and volatility spike in the first five minutes of
00:00, 08:00 and 16:00 UTC - the funding windows (p.5/p.10). `2501.09404v1` instead imports spot
exogenously and has the peg holding with cross-correlation peaking at lag 2 (pp.12-13), reversing the
observed lead-lag.

## 5. Stablecoin deleveraging and collateral chains

`1906.02152v3` proves deleveraging feedback creates illiquidity in crises and exacerbates collateral
drawdown, observed in Dai's Black Thursday of March 2020 (pp.3-4): supply cannot fall by more than the
"free supply" y = L(1 - w^D) (p.13, Prop 4). A worked spiral moves ETH 85 -> 83 -> 82 -> 81, driving
three liquidation waves, with the stablecoin at 0.994 -> 1.026 -> 1.071 -> 1.126 and reserves 1.800
-> 1.644 (p.14, Table 1). Black Thursday was an approximately 50% ETH crash on 12 Mar 2020; premiums
reached about 10%, stayed above 2% weeks later, and Maker auctions cleared near zero at a cost of $8m
(p.14). With unit-elastic demand and a non-binding constraint the system converges exponentially when
delta <= 1/2 (p.16, Thm 1), but binding states show tail volatility and stricter pro-cyclical limits
raise volatility without improving safety (pp.17-19).

`2204.11107v2` quantifies collateral reuse from Ethereum logs at blocks 10,000,000 (4 May 2020) to
11,700,000 (21 Jan 2021), grouping 28,599 address groups holding 47,944 addresses across two or more
protocols. Monthly debt-financed collateral was 30.3% in July 2020 (2,183 of 7,214), then 18.2%,
10.9%, 11.1%, 14.1% and 11.7% to December, and 19.6% in January 2021 (3,786 of 19,314) (p.6, Table
V), with Compound largest and DAI then USDC the dominant debt-financed currencies (p.7). ETH day-one
price change correlates with day-two debt-financed collateral at R = 0.154, p<0.05 (p.7, Table VI).
Leverage is interlinked, so per-venue caps miss contagion and linear margin models understate cost.

## 6. Fees, regimes and wash trading as data-integrity problems

`2501.05299v1` shows cost is a regime, not a haircut: over 1 Jul 2020 to 14 Nov 2022 the mean daily
Ethereum fee was $13.18, and only transactions above $262 beat a PayPal comparison of 5% plus $0.05
(p.15, note 5). Time-varying Granger causality finds bidirectional CEX volume/fee causality with Wald
up to 55.79 and for Bridges, DEX volume causing fees at 32.2-36.7 but fees causing DEX volume only
intermittently, and stablecoin activity causing fees mainly from Q2 2022 (recursive Wald 40.98)
(p.24, Table 4); bridge feedback fades after Q2 2022 and MEV causality weakens from Q2 2022 (p.26,
p.32).

`2411.05803v4` decomposes liquidity fluctuation into jump and diffusion: US large caps show at most
1.09% of days with beta_sigma >= 1 (NVDA) and jumps on 7.05-38.27% of days (p.17), while crypto
both-high days run from 8 days / 0.51% (BTC) to 545 days / 34.56% (BCH) (pp.17-18). BTC diffusion is
0.95% of days, below NVDA, but all other crypto assets except BTC and ETH show beta_sigma > 3
(pp.19-20). A treatment cutting Q3 minute volume by 50% and Q4 by 75% leaves about 40% of the amount,
flagging about 60% as wash against Cong et al.'s 46.47% for Binance (p.21); detection is a heuristic,
not a label.

## 7. Prediction markets as instruments

`2606.19517v1` prices a Polymarket "BTC above $27,000 at end of September?" contract against the
matching Binance call BTC-230929-27000-C over 214 aligned hourly observations. The mean gap versus
the discounted risk-neutral binary value is 5.6 percentage points (D=0.0558, s=0.1264, t=6.46,
p~6.9e-11), with HAC 95% CI [0.0228, 0.0889] and block-bootstrap [0.0212, 0.0911] (p.8, Table 2);
three pooled BTC markets give 6.3 points over 287 observations (p.1). The gap has an AR(1)
half-life of about 4 hours (p.1), is largest at low implied probabilities and long maturities, is
11 points in a Deribit extension with Ethereum mixed (p.1/p.5), and leaves a delta-hedged arbitrage
proxy profitable after costs but with marginal precision (p.1). Identical payoffs can hold
multi-hour wedges, and the right benchmark is the option-implied risk-neutral value.

## What this project can take from it

| Finding | Where it lands | What to do |
|---|---|---|
| Fee-collecting CFMMs are incomplete | python/nautilus_trader/model | Track per-LP-share balances, not raw reserves |
| Two safe pools can be jointly exploitable | docs/usermanauls/cross-venue-relative-value | Add collective-arbitrage checks to backtests |
| LVR is a no-cost ceiling, RVR is realistic | python/nautilus_trader/analysis | Report both alongside PnL for liquidity strategies |
| LP mint/burn events are far rarer than swaps | python/nautilus_trader/data | Drive LP events on their own clock, not swap ticks |
| Open interest must reconcile with volume | python/nautilus_trader/data | Validate feeds with V >= abs(delta OI) before use |
| Funding times create predictable spikes | docs/concepts/trading_calendars | Model 00:00/08:00/16:00 UTC funding windows |
| The tether perp leads spot in volatility | docs/usermanauls/cross-venue-relative-value | Weight leading venues in aggregation and hedging |
| The on-chain leg of CEX-DEX can fail | docs/usermanauls/execution-algorithms | Make DEX landing probability a strategy input |
| Deleveraging raises cost per unit repurchased | python/nautilus_trader/risk | Stress stablecoin exposure on non-linear paths |
| Collateral is reused across protocols | python/nautilus_trader/risk | Aggregate cross-protocol exposure, not per venue |
| Unregulated CEX volume carries wash signatures | python/nautilus_trader/data | Screen feeds with jump/diffusion diagnostics |
| Prediction-market prices are not probabilities | docs/usermanauls/options | Benchmark against option-implied risk-neutral values |

## Caveats

The weightiest results are theory: `2310.09782v2` has no data, so its safety conditions are design
constraints, not measured properties. The empirical AMM and DeFi findings rest on small samples - two
pools across three short windows in `2602.22069v1`, a snapshot and a six-month slice in `2301.13009v2`,
and one pre-Terra window of 28,599 address groups in `2204.11107v2`; the perpetual open-interest
findings are BTC-only and cannot separate deliberate misreporting from delayed liquidations.

Simulation realism is the other gap: `2601.00738v1` assumes zero gas, perfect validator participation,
infinite CEX liquidity and a fixed DEX landing probability, while `2501.09404v1` is an uncalibrated toy
environment with an exogenous spot and no funding formula; the fee-regime evidence suggests the
zero-gas gains are overstated. Several results are associations, not causal claims: cryptoness is a
regression R^2, the debt-financed-collateral correlation is not causal, `2501.05299v1` is Granger
causality on labelled-address proxies, and `2411.05803v4` detects a signature rather than wash trades.
No paper establishes out-of-sample live performance or nets an on-chain result into a tradable
return, and where papers disagree both readings are kept here.

## Papers read in depth

- `2310.09782v2` - All AMMs are CFMMs (2023). Theory; no-arbitrage as a strictly increasing invariant.
- `2602.22069v1` - Pools as Portfolios (2026). Two live pools; best extraction-efficiency measurement.
- `2301.13009v2` - Uniswap v3 data characterisation (2022). Event rates and taker clusters, filtered.
- `2310.14973v2` - Open Interest versus Traded Volume (2023/2024). Simple identity, large violations.
- `2501.09404v1` - Agent-Based Simulation of a Perpetual Futures Market (2025). Mechanism, no calibration.
- `2501.05299v1` - Time-Varying Causality Between Transaction Fees and Activity (2024). Fee regimes shift.
- `2601.00738v1` - 1-second subslots and CEX-DEX Arbitrage (2026). Slot-speed effects, zero-gas model.
- `1906.02152v3` - Deleveraging Spirals and Stablecoin Attacks (2019/2021). Non-linear deleveraging.
- `2204.11107v2` - Debt-Financed Collateral and Stability Risks (2022). Reuse, upward-biased heuristic.
- `2107.00298v3` - Binance in Bitcoin Volatility Transmission (2021). Perp leads; funding clock.
- `2411.05803v4` - Liquidity Jump, Liquidity Diffusion, and Crypto Wash Trading (2024). Heuristic screen.
- `2606.19517v1` - Do Prediction Markets Match Option Prices? (2026). Clean comparison; marginal precision.

## Where to next in the corpus

- `2508.03474v1` - prediction-market arbitrage via probabilistic forests; closest cross-venue lead.
- `2604.20067v1` - replication of the latency-arbitrage agent-based model; tests CEX-DEX robustness.
- `2102.00626v1` - flash-loan attack tooling; the liquidation and auction attack surface as a model.
- `2608.26174v1` - crypto volatility forecasting; would inform funding-aware perpetual signals.
