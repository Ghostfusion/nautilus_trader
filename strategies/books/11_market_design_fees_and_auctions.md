# Market Design, Fees and Auctions

Date: 2026-10-05. Revision 1.

This brief distils the market-design corpus slice: 213 harvested records spanning 2002-2026, 32 of them
carrying a journal reference. It reports the 12 papers read in depth on maker-taker fees and rebates, tick
size and its unintended effects, price limits versus circuit breakers, and call and closing auction
mechanics.

## What this covers

The slice covers the venue levers under every strategy: who pays whom for liquidity, how fine the price grid
is, what happens at a halt, and how batch auctions clear. The single most consequential fact is that no lever
is neutral: a maker rebate of one tenth of a tick can delete a Nash equilibrium while 80% of learning agents
still find it, and a tick-size cut that narrows spreads can raise the cost of the largest orders by more than
80% in one sample. Fee schedules belong inside a strategy's action space; tick and halt parameters belong
inside its execution and queue models, not on an invoice.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category in the harvest | 213 |
| Years spanned | 2002-2026 |
| Records with a journal reference | 32 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

Selection rule: keep papers with a quantitative or structurally precise claim about maker-taker fees and
rebates, tick size, price limits versus circuit breakers, or call and periodic auction mechanics and
duration; drop crypto and AMM fee work, plugin-history papers, and agent-based markets with no design lever.

## The short answer

1. Fee changes are regime changes: a 0.1-tick maker rebate removes the joint action that was a Nash equilibrium, yet 80% of Q-learning instances still converge to it (`2211.00496v1`).
2. The rebate-to-cost map is non-monotone: mean net fee falls from 1.887 with no rebate to 1.207 at beta=0.2, then rises to 1.304 at beta=0.3 (`2211.00496v1`).
3. At gamma=0.99 and beta=0.1 the net fee is 1.948, above the 1.887 no-rebate baseline (`2211.00496v1`).
4. With 4 makers net fee is 1.877 and 9.075 orders; with 8 it is 1.004 and 13.249, so enough makers erase the rebate's effect (`2211.00496v1`).
5. Maker-taker can lower volatility, impact and measured inefficiency yet raise all-in taker cost above the recorded 0.327% (`2010.08992v1`).
6. A tick-size cut narrowed spreads but raised 10MM+ bucket impact by 74% to 84% while the 5-10MM bucket fell 5% to 23% (`1602.00839v7`).
7. Price limits and circuit breakers match only when band lookback exceeds cancel horizon; at tr=1000 and Pr=10 falling depth is 298 versus 152 (`2309.10220v1`).
8. Optimal auction duration is asset-specific, of order 2 to 10 minutes for 77 Euronext stocks, versus the 100 ms BATS-Cboe periodic auction (`1906.01713v3`).
9. In illiquid call auctions the no-trade probability runs from 4% to 33%, versus below 1e-5 in a liquid one (`1407.4512v2`).
10. A quadratic half-spread fee restores a Nash equilibrium where a price-discovery penalty cannot (`2307.15805v1`).

## 1. Maker-taker fees and rebates as payoff regimes

Fees enter the payoff matrix, not just the invoice. In a dealer market with N=2 makers quoting one asset
across K=4 price levels and posting 20 limit orders per side, the no-fee joint action (ask 2, bid 2) is both
the Nash equilibrium and the cooperative strategy, with highest one-period reward 34.2 and lowest-spread
reward 26.5 (`2211.00496v1`, p.5). With maker rebate 0.1 tick and taker fee 0.15 that action stops being a
Nash equilibrium, yet 80% of independent Q-learning instances still converge to it (`2211.00496v1`, p.5); at
beta=0.2 the undercutting payoff rises to 34.4 from 32.0 (`2211.00496v1`, p.5).

The rebate-to-cost map is non-monotone. At discount factor 0.95 the mean net fee (standard deviation) and
order count are 1.887 (0.040) and 9.03 with no rebate, 1.792 (0.342) at beta=0.1, 1.207 (0.053) at beta=0.2
and 1.304 (0.046) at beta=0.3 (`2211.00496v1`, p.6). At gamma=0.99 and beta=0.1 the net fee is 1.948, above
the 1.887 no-rebate baseline (`2211.00496v1`, p.6). A taker-maker schedule with beta=-0.75 reaches net fee
1.254 and 15.69 orders, roughly matching maker-taker at beta=0.2 (`2211.00496v1`, p.6), and maker count
erases the effect: 4 makers give 1.877 and 9.075 orders, 6 give 1.061 and 12.965, 8 give 1.004 and 13.249
(`2211.00496v1`, p.8).

A separate agent-based market with 990 normal agents, 10 algorithm takers and one position-based market maker,
holding exchange revenue at R_EX = 0.100%, finds volatility, impact and market inefficiency all decrease as
the maker rebate rises (`2010.08992v1`, pp.4-5). A maker can quote a narrower spread because expected return
per trade depends on rebate plus spread, theta_M = R_eM - 2 R_M (`2010.08992v1`, eq 14, p.3). Yet the total
cost of a taking order is generally higher under maker-taker, and without it is 0.327% (`2010.08992v1`, p.5);
it is lower only when the impact reduction exceeds the taker-fee increment (`2010.08992v1`, p.5).

## 2. Rebate design and market-maker incentives

Rebate design is an exchange control problem. With two competing strategic makers, non-strategic investors
paying a fixed fee d, and rebates redistributed, the optimal transaction fee is d_hat = 3.0 for Apple and 2.0
for Alphabet, with revenue rho(d) first decreasing then rising as d grows (`2501.12591v2`, p.38). Exchange
value moves from rho = 454 (Apple) and 487 (Alphabet) at zero fees to rho(d_hat) = -45119 and -44003 under
the optimal fee and incentive, negative meaning a gain (`2501.12591v2`, p.41). Market efficiency, the
expected squared spread between clearing and efficient price, improves from 454 and 487 at d=0 to 144 and 127
at the optimum (`2501.12591v2`, p.42); optimal compensations are negative, discouraging late arrivals, and
arrival-linked incentives rise then fall over the auction (`2501.12591v2`, pp.39-40).

Fee design can also buy participation. In a two-player one-period periodic double auction with private
estimates, sigma_i uniform on [sigma_-, sigma_+] and Corr(eps_a, eps_b) = rho, there is no Nash equilibrium
with finite delta without incentives (`2307.15805v1`, Prop 2.6, p.6); a price-discovery penalty
gamma(mid - P_inf)^2 does not restore one (`2307.15805v1`, Prop 3.1, p.7); a quadratic fee on the half-spread
gamma*delta^2 does, producing a trade (`2307.15805v1`, p.8). For symmetric players the optimal fee is
gamma* = phi(y*)/(2 sigma_+^2) with y* solving 1 - Phi(y*) - y* phi(y*) = 0 (`2307.15805v1`, Theorem 2,
p.8), and a calibration at sigma_-=0.1, sigma_+=1.1, rho=0 needs gamma >= 1.5, suggesting gamma ~ 1.5
against the analytic gamma* ~ 0.1 for sigma_-=sigma_+=1.1 (`2307.15805v1`, pp.10-11).

## 3. Tick size and its unintended effects

Tick-size changes are not just finer grids. In Tokyo's TOPIX 100, phase 1 went live 14-Jan-2014 for quotes
above JPY 3000 (39 stocks) and phase 2 on 22-Jul-2014 brought decimal yen ticks below JPY 5000 (80 stocks).
Spreads decreased and volume and trade counts increased, but average execution size decreased and large-order
trading costs increased (`1602.00839v7`, p.23). Market impact in the 10MM+ notional bucket rose by 74%, 81%
and 84% in samples S2, S3 and S5, while the 5-10MM bucket fell by 5%, 23% and 11% (`1602.00839v7`,
pp.20-21); under a 25MM+ categorisation the 10-25MM bucket rose by 119%, 110% and 123% while 1-10MM fell by
31%, 10% and 17% (`1602.00839v7`, pp.21-22).

For large-tick assets the observed spread is the wrong state variable. An uncertainty-zones model defines an
implicit spread 2*eta*alpha estimated by alpha_hat = N_continuations / (2 N_alternations) (`1207.6325v2`).
The spread-volatility relation holds when the observed spread is replaced by this implicit spread
(`1207.6325v2`, p.17); alpha near 1/2 means the tick is optimal and the last traded price behaves like
sampled Brownian motion, while alpha < 1/2 means a very large tick, strong mean reversion and observed
realised variance above efficient-price variance (`1207.6325v2`, pp.14-16).

Clustering survives decimalisation. At EBS, before the March 2011 change taking EUR/USD from 1e-4 to 1e-5
and USD/JPY from 1e-2 to 1e-3, the spread was one tick 65% of the time and two ticks otherwise
(`1307.5440v3`, p.4). After decimalisation the EUR/USD spread distribution is bimodal at 9 and 13 ticks,
where 10 equals one old pip, and the book peaks at 5, 10, 15 and 20 ticks (`1307.5440v3`, p.4). About 50% of
trades and 20% of limit orders sit at integer prices, with digit 1 next and 9 on the ask, consistent with
stepping one tick ahead of clusters (`1307.5440v3`, pp.8-9); Poisson arrivals are rejected in favour of
Hawkes-like flow, and order-sign autocorrelation decays in about 5 minutes for limit orders and 2 minutes for
market orders (`1307.5440v3`, pp.6-7).

## 4. Price limits versus circuit breakers

In an artificial market with 1000 fundamental and technical agents, erroneous orders from t=30000 to t=60000
at p_m=0.15 and stop-loss orders at p_l=0.35, a price limit and a circuit breaker are about equally effective
when Pr and tr match (`2309.10220v1`, pp.4). The equivalence breaks when the band lookback is short relative
to order lifespans: the price limit is less effective when tr < tc (`2309.10220v1`, p.5). Average falling
depth at tr=1000 and Pr=10 is 298 for the price limit against 152 for the breaker, and at Pr=20 it is 577
against 297 (`2309.10220v1`, tables pp.4); the difference shrinks toward zero as tr grows beyond tc, from 146
at tr=1000, Pr=10 to 11 at tr=20000, Pr=10 (`2309.10220v1`, p.4). The mechanism is queue accumulation: sell
orders pile at the lower limit and when tr < tc the limit price moves before they cancel, leaving a wall that
blocks recovery (`2309.10220v1`, p.5 and Fig 5).

## 5. Call and closing auction mechanics and duration

In the standard call auction with Poisson arrivals and prices iid from F, traded volume is hypergeometric
conditional on the counts and the clearing-price bounds are order statistics, so the volume law does not
depend on F (`1407.4512v2`, Lemma 1, p.5). As liquidity lambda*T grows, volume and the clearing bounds are
asymptotically normal and the clearing-price range is asymptotically exponential (`1407.4512v2`, Prop 2,
p.8). Volume has mean lambda T alpha (1-alpha) and sd sqrt(lambda T alpha (1-alpha) (1 - 2 alpha (1-alpha)))
with alpha = lambda_A/(lambda_A+lambda_B) (`1407.4512v2`, Prop 2, p.8); in an illiquid market (lambda=10) the
no-trade probability runs from 4% to 33% by imbalance, and at lambda=100 it is below 1e-5 even when
unbalanced (`1407.4512v2`, p.8).

Batch auctions invite sniping: a strategic seller who controls arrival time benefits from arriving at the last
moment, using accumulated information and maximising impact, which moves the clearing price from the efficient
price (`2405.09764v2`, p.5). Randomising the closing time and charging arrival-time-indexed fees both push
submission earlier and improve efficiency (`2405.09764v2`, p.5); London Stock Exchange adds a 30-second
random period to opening and closing auctions, and Cboe randomises the periodic-auction duration from 0 to
100 ms (`2405.09764v2`, p.5).

Duration is a lever with a computable optimum. With the clearing price set as the volume-maximising price and
loss the time-weighted expected squared deviation from the efficient price, the optimal duration for 77
Euronext stocks is 2 to 10 minutes (`1906.01713v3`, abstract), or 1 to 5 minutes (`1906.01713v3`, p.5); estimates
in seconds with 90% intervals include Orpea 834 [822;846], Carrefour 410 [407;413], Ipsen 827 [817;838],
Natixis 351 [348;354], EDF 341 [338;344] and Axa 252 [251;254] (`1906.01713v3`, p.19). Continuous limit
order books are the h=0 case and are usually sub-optimal, though only moderately impairing price formation
(`1906.01713v3`, p.5), while the speed-driven alternative is about 100 ms and the BATS-Cboe periodic auction
runs a pre-fixed maximum of 100 milliseconds (`1906.01713v3`, p.3).

## 6. Fee restructuring, impact and cost models

Removing an explicit fee can raise implicit cost. On the JSE Top 40 in 2013 the 30-Sep-2013 change removed a
4.00 ZAR minimum per trade, made fees pro-rata on value and capped them at 300 ZAR, leaving 250 trading days,
186 before and 64 after (`1602.04950v3`). The Lillo master curve, a power-law relation between price
increment and transaction size, is confirmed and survives rescaling by a liquidity proxy (`1602.04950v3`,
p.6). After the fee reduction the price impact of small trades was larger than expected, most in Financials
where average increments for small trades rose above 1e-3.8 and less in Resources and Industrials
(`1602.04950v3`, pp.2); a decrease in direct costs coincided with an increase in indirect costs via impact,
with flagged consequences for hedging derivative books (`1602.04950v3`, p.6), and one proposed but unverified
explanation is lower order-book resiliency for low-volume price-moving trades (`1602.04950v3`, p.6).

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| A rebate flips the undercut incentive (`2211.00496v1`) | python/nautilus_trader/risk | Expose fee and rebate inputs per venue as configuration. |
| Net taker cost is non-monotone in the rebate (`2211.00496v1`) | docs/usermanauls/market-making | Backtest maker-taker and taker-maker as separate regimes. |
| Maker-taker tightens spreads, raises all-in cost (`2010.08992v1`) | docs/concepts/execution | Sum fees and impact in one cost model. |
| Tick reduction helps small and hurts large orders (`1602.00839v7`) | docs/usermanauls/microstructure-signals | Re-estimate the impact model after a tick change. |
| Tick size is summarised by one alpha (`1207.6325v2`) | python/nautilus_trader/data | Estimate alpha from alternations and continuations. |
| Clustering persists after decimalisation (`1307.5440v3`) | docs/concepts/order_book | Model non-uniform priority at round price levels. |
| Price bands and halts are not equivalent (`2309.10220v1`) | python/nautilus_trader/backtest | Represent bands and halt windows in the simulator. |
| Auction duration optimum is asset-specific (`1906.01713v3`) | docs/usermanauls/intraday-systematic | Keep auction frequency configurable and score discovery error. |
| Sniping is mechanical in batch auctions (`2405.09764v2`) | docs/concepts/live | Treat randomised close and late-arrival fees as order attributes. |
| Clearing distributions follow from flow (`1407.4512v2`) | docs/concepts/backtesting | Use the normal and exponential laws for no-trade and spread baselines. |
| Fee reduction can raise impact (`1602.04950v3`) | python/nautilus_trader/analysis | Recalibrate a liquidity-proxy impact curve after fee events. |

## Caveats

Several of the strongest numbers come from simulations with parameters chosen for convenience, so the fee
magnitudes should not be read as empirical estimates. The tick-size and fee-restructuring evidence is
closed-system and single-event, with no control group, so it cannot cleanly attribute cause, and the JSE
small-volume anomaly is described as unverified. The auction-duration and rebate-design results are
model-implied rather than causal tests of a live design change, and the numerical study behind the optimal fee
uses only two market makers. Several models omit latency, queue position, long memory and strategic maker
response, and the Poisson arrival assumption used in the auction models is rejected empirically in the FX
order-book study. The maker-taker papers disagree because they differ in objective and mechanism, fixed
exchange revenue versus strategic learning versus optimal design, so their results do not directly reconcile.
Coverage is narrow throughout, so nothing here should transfer across asset classes without re-estimation.

## Papers read in depth

- `2211.00496v1` - Can maker-taker fees prevent algorithmic cooperation in market making? (2022). Rebates are regime changes and the rebate-to-cost map is non-monotone, from simulation.
- `2010.08992v1` - Analysis of the impact of maker-taker fees on the stock market using agent-based simulation (2020). Maker-taker can tighten spreads while raising the all-in taker cost.
- `2501.12591v2` - Optimal Rebate Design: Incentives, Competition and Efficiency in Auction Markets (2026). Fees and rebates as an exchange optimisation with negative compensation for late arrivals.
- `1602.00839v7` - A Tale of Two Consequences: Intended and Unintended Outcomes of the Japan TOPIX Tick Size Changes (2016). Tick reduction helps small orders and raises large-order impact.
- `2309.10220v1` - Comparing effects of price limit and circuit breaker in stock exchanges by an agent-based model (2023). The two are equivalent only when the band lookback exceeds order lifespans.
- `1407.4512v2` - Exact and asymptotic solutions of the call auction problem (2014). Clearing volume and price bounds have closed-form and asymptotic laws.
- `1906.01713v3` - Optimal auction duration: A price formation viewpoint (2019). Optimal duration is asset-specific at minutes, against the millisecond design camp.
- `2405.09764v2` - Clearing time randomization and transaction fees for auction market design (2024). Random closes and late-arrival fees suppress sniping.
- `1602.04950v3` - Deviations in expected price impact for small transaction volumes under fee restructuring (2016). Lower explicit fees coincided with higher small-trade impact.
- `1207.6325v2` - Large tick assets: implicit spread and optimal tick size (2013). One estimated alpha links tick size to volatility per trade.
- `1307.5440v3` - Tick Size Reduction and Price Clustering in a FX Order Book (2013). Decimalisation leaves clustering at round prices intact.
- `2307.15805v1` - Equilibria and incentives for illiquid auction markets (2023). A quadratic half-spread fee induces participation where a price-discovery penalty does not.

## Where to next in the corpus

- `2401.06724v2` - Euronext closing-auction books with latent liquidity; descriptive, overlaps the auction papers read but may sharpen closing-auction depth modelling.
- `2607.08525v1` and `2403.03367v4` - AMM and gas-fee mechanism design; fee-adjacent but a different venue class, not directly mapable to a central limit order book.
- `1310.0057v1` and `1011.6284v2` - Regulatory-capture and short-selling-ban or Tobin-tax policy work; context on intervention design without an execution lever.
- `1305.2716v1` and `1503.00913v1` - Continuous double auction statistics and queueing models; the queueing view would complement the band-accumulation mechanism found here.
- `1401.2982v1` and `2010.13036v1` - Speed-of-light and leveraged-ETF market structure; adjacent speed and flow-effects work with no direct fee, tick or auction lever.
