# Liquidity and Systemic Risk: Deleveraging, Flash-Crash Propagation and Contagion

Date: 2026-10-05. Revision 1.

This brief reads 104 harvested records on liquidity and systemic risk, spanning 2004-2026, of which 10 carry
a journal reference. From 40 screened candidates, twelve papers are covered: leverage-driven deleveraging and
flash-crash propagation, flash-crash simulation with participation limits and market-maker inventory caps,
asset-manager liquidity stress testing and cost models, crowding and portfolio overlap, clearing-mechanism
biases, intraday drawdown tail laws, and network contagion.

## What this covers

The slice couples liquidity and leverage: forced selling into a shallow book produces endogenous, non-linear
price impact, and the flash crash is treated as a repeated, well-documented failure mode rather than a one-off.
It covers agent-based flash-crash models, transaction-cost and redemption stress frameworks, crowding measured
from co-trading, clearing-order bias, and centrality-based contagion. The single most consequential thing in
the slice is that individually prudent risk controls - per-trader inventory caps and VaR constraints - do not
prevent, and can worsen, system-wide crashes, so the systemic parameter lives at the level of the venue or the network, not the participant.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category | 104 |
| Years spanned | 2004-2026 |
| Records with a journal reference | 10 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

Selection rule: the paper must carry a quantitative, reproducible claim about liquidity risk, stress
propagation, clearing, margining or deleveraging that a trading platform can act on; pure mechanism-design,
optimal-execution and prediction papers were dropped.

## The short answer

1. Deleveraging is itself a contagion channel: in a 50-fund, 50-asset network a margin call produces a flash crash within about two minutes of distressed selling when the sale is 9% of trailing-minute volume every 10 s (`1805.08454v1`, p.20).
2. Flash-crash amplitude is non-monotone in participation: on E-mini S&P 500 data it rises with order size but flattens once the sell algorithm exceeds 5% of the prior minute's volume (`2208.13654v1`, p.25).
3. Market-maker inventory caps are systemic: amplitude rises below an inventory limit of about 8000 and falls above it, and an infinite cap would remove the crash entirely (`2208.13654v1`, p.25-26).
4. Cost models are liquidity-bucket specific: large caps follow ccc = 1.25 s + 0.40 sigma x^0.5, but sovereign bonds use a 0.25 power, ccc = 1.25 s + 3.00 sigma y^0.25 (`2105.08377v1`, p.36-48).
5. Liquidity risk is a surface over size and horizon: a 3000-lot meta-order over 5 minutes yields square-root impact though no impact law is encoded, with cost over 400 Monte Carlo runs (`2505.15296v1`, p.6-8).
6. Redemption coverage depends on the liquidation policy: the same fund shows RCR(1) = 52.53% under naive pro-rata but RCR(1) = 59.01% and RCR(2) = 116.90% under a waterfall (`2110.01302v1`, p.9-12).
7. Clearing order is not neutral: sequential per-asset clearing gives P_T = 16.61 and sigma = 3.188 with ranking A > B > C > D > E, versus 16.94 and 0.241 in parallel (`2509.01683v1`, p.9-11).
8. The evidence disagrees on crowding: clustering raises kurtosis and mainly the upside tail (`2002.03319v1`, p.4), while a micro-macro ABM finds crowding can be beneficial at high leverage (`1805.08454v1`, p.22).
9. Intraday drawdowns obey a power law only in the body: exponents run 4 to 5.67, yet the May 6 2010 flash crash and a CAC drawdown of 214.99 sigma sit above the fit (`1407.5037v2`, p.13-18).
10. Prudent individual limits do not prevent systemic failure: stablecoin failure time is dominated by collateral returns and is largely independent of the speculator's risk method (`1906.02152v3`, p.19-20).

## 1. Leverage, margin calls and deleveraging spirals

The micro-macro model of `1805.08454v1` places an exogenous distressed seller in one asset of a bipartite
fund-asset network (n_f = n_a = 50), runs 100,000 steps of 50 ms (about 1.5 hours of market time) and 160 Monte
Carlo trials, and defines a flash crash as a price fall exceeding 5% in a 5-minute window (p.19). Endogenous fund
deleveraging via margin calls, fire sales over shared assets and bank liquidation of defaulters turn one shock
into a cascade. Calibrating the order size to 9% of rolling one-minute volume every 10 s induces a flash crash
within about two minutes (p.20), and a sweep over 210 combinations of leverage, threshold and capital shows phase
changes from a stable to an unstable regime (p.20). Contagion speed is non-monotone in portfolio diversification in high-crowding regimes (p.22).

A crypto analogue reaches the same conclusion from a different direction. `1906.02152v3` models a Dai-like
non-custodial stablecoin over 10,000 paths of 1000 daily steps with t-distributed collateral returns (df = 3),
using n0 = 400 (4x overcollateralisation) and beta = 1.5. When the leverage constraint is non-binding the system
converges exponentially to a steady state (Theorem 1, p.16); when it binds, deleveraging feedback causes
illiquidity and simulated stablecoin volatility can reach the order of actual Ether volatility even under "nice"
return assumptions (pp.17-19). Failure time is dominated by collateral returns and is largely independent of the
speculator's risk-management method, while volatility is strongly affected by it (pp.19-20). Real implementations
created 5-13% arbitrage around liquidations (pp.21-22), and on Black Thursday mempool manipulation cleared $8m of
Dai liquidation auctions at near-zero prices (p.22). The network review `1912.05273v1` frames this: financial
networks are "robust-yet-fragile", so collapse is unlikely but extensive (p.1); fire-sale contagion through overlapping portfolios depresses a common asset for all holders and depends on market depth (pp.4-5).

## 2. Flash-crash simulation: non-monotone participation and inventory limits

`2208.13654v1` builds a single-security high-frequency ABM on the E-mini S&P 500 futures book, step 100 ms and
T = 324,000 steps per 9-hour day, calibrated to CME data for May 3-6 2010 with 10 book levels per side. The 2010
event is replicated with an institutional sell algorithm targeting 9% of the previous minute's traded volume:
amplitude is about 7% from 14:30 versus the historical ~7%, and execution takes about 17 minutes versus about 20
minutes historically (pp.22-23). Between 14:40:50 and 14:42:20 price falls more than 4.16% in under two minutes to
an intraday low of 1053.5; average depth of about 5000 per side collapses to under 1000 and reaches zero on the
bid side for more than a minute, with the spread widening beyond 20 ticks (p.22).

The central structural finding is non-monotonicity. Amplitude versus the sell algorithm's participation rate r
rises for small r but is essentially flat after r exceeds 5%, because a fixed inventory is exhausted at a bounded
low price (p.25). Amplitude versus the market-maker inventory limit is also non-monotone, increasing below roughly
8000 and decreasing above it, and with an infinite limit there would be no flash crash (pp.25-26). Higher
fundamental-trader trading frequency monotonically reduces amplitude (p.26), and separately a spiking trader
triggers mini flash crashes of roughly 80 bps in seconds whose frequency and amplitude depend strongly on the
inventory limit but not on fundamental-trader frequency (pp.27-29).

`2505.15296v1` adds a real matching engine: a Simudyne/HKEX continuous double-auction model calibrated to the
Hang-Seng Index futures front contract (HSIZ2) on 2022-12-23, with 3.4 million order operations, 85,000 trades and
a 20 ms step. Single-trade impact is fitted as f_mi(Q) = 0.561 sqrt(Q). A meta-order of 3000 executed as market
orders every 10 s over 5 minutes produces emergent transient and permanent impact following the square-root law
although the model encodes neither explicitly (pp.6-7). Liquidity risk is computed as a surface over horizons and
sizes with 400 Monte Carlo runs, cost rising concavely in size (pp.7-8); the Bloomberg transaction-cost model underestimates the liquidity risk relative to the ABM for HSI (p.8).

## 3. Liquidity stress testing and pre-trade cost models

`2105.08377v1` fits a two-regime power-law cost model ccc(x) = c_s * s + c_phi * sigma * x^beta on proprietary
transaction data. Large caps give ccc = 1.25 s + 0.40 sigma x^0.5 with unconstrained exponent beta1 = 0.5873 and
per-stock average R2c = 55.7% against a pooled 97.84% (pp.36-38). Small caps give 1.40 s + 0.50 sigma x^0.5, i.e.
+12% fixed cost and +25% price impact (pp.40-41). Sovereign bonds switch to a 0.25 power, 1.25 s + 3.00 sigma
y^0.25 with beta1 = 0.2037 and R2c = 28.94% (pp.43-48), while corporate bonds use 1.50 s + 0.125 DTS y^0.25 with a
DTS-model R2c = 46.45% versus 41.66% for a volatility model and a +25% impact premium over sovereigns (pp.48-50).

Stress multipliers matter as much as the base curve. Over 7850 VIX observations from January 1990 to February
2021, weekly multiplicative volatility stress is 1.50 (+9.66% additive), two-year weekly 1.80 (+17%) and monthly
2.66 (+29%) (pp.55-56); daily volume falls about 25% over a week and about 50% over a month (factors 0.75 and
0.48, p.56-57); and two-year weekly bid-ask spread stress is a factor 3 (+6.5 bps), p.58-59. The data-quality
warning is concrete: end-of-day spreads can jump from 2 to 80 bps, the reported negative-spread frequency is 0.01%
(FactSet) and 0.24% (Bloomberg), and Pr(m_s > 10) is about 0.6-0.8% for EuroStoxx 50 names (pp.57-58).

On the asset-liability side, `2110.01302v1` defines the redemption coverage ratio RCR = liquid assets / net
outflows and the liquidity shortfall LS = max(0, net outflows - liquid assets). For an illustrative fund with TNA
$141.734 mn and a 20% ($28.347 mn) redemption shock, naive pro-rata liquidation gives RCR(1) = 52.53% and LS(1) =
9.49% with five days needed for RCR = 100%, optimal pro-rata gives RCR(5) = 114.92%, and a waterfall gives
RCR(1) = 59.01% and RCR(2) = 116.90% (pp.9-12). Under the HQLA route a public-equity fund shows RCR = 2.5 at a 20%
shock and falls below 1 only above a 50% shock, while an HY fund has RCR = 0 for any shock (p.15); IMF FSAP work
cited there finds about 30 Luxembourg bond funds with RCR below 1, half of which have a shortfall above the 10%
UCITS borrowing limit (p.5). Optimal cash buffers are generally 0% or 100%, rarely intermediate, hitting 10% when
expected redemption is 50% with a trading limit of 10% (pp.38-40). Swing pricing removes the first-mover advantage
at the cost of NAV volatility, with a typical swing threshold of 5% normal and 2% stress (pp.46-50); during
February-March 2020 at least 215 European investment funds (EUR 73.4 bn) used redemption suspensions (p.41, p.48).

## 4. Crowding, clearing mechanics and network contagion

`2002.03319v1` measures crowding from MiFID post-trade records of 86 Dutch banks and investment firms over January
2009 to April 2015, building a bipartite investor-stock network and a maximum-entropy null from degree sequences
alone, with market clustering m_s,t = observed motifs / expected motifs. It finds a robust positive relation
between clustering and return kurtosis (p.4), a relation with the positive tail (outlier count, tail index, VLuck)
but interestingly not with the negative tail (p.4, p.14), and the relation vanishes for a control group of stocks
mainly traded by non-Dutch investors (p.4, p.15). In a dynamic panel the effect operates through larger changes in
downside VaR during turmoil and not in calm periods, again stronger for the positive tail (p.5, p.16). This
empirical sign conflicts with the ABM result that crowding can aid stability at high leverage (`1805.08454v1`,
p.22); the two use different data and tail measures, and the dispute is unresolved.

`2012.04181v1` estimates a bivariate Hawkes flocking model on WTI (CL) and gasoline (RB) futures from January 2007
to December 2016, decomposing risk into endogeneity and interaction via branching ratios. The overall branching
ratio peaks at about 85% in mid-2008 just before the Lehman collapse and falls to about 63% in early 2011 (p.23).
Within-asset endogeneity runs 58%-82% for CL and 32%-60% for RB (pp.23-24), and interaction is strongly asymmetric:
RB affects CL with a branching ratio of 10%-55%, while CL affects RB by only 2%-8% (p.24). Delta CoVaR on the same
data rises in distressed periods but shows near-symmetric relative contributions, so the two measures disagree.

`2509.01683v1` shows that clearing order itself biases multi-asset results. With N = 10,000 zero-intelligence
traders, J = 5 assets, M = 200 cash and S = 10 shares at P0 = 10 over T = 5000 steps under a hard budget
constraint, sequential per-asset clearing proves E[V1] > E[V2] > ... > E[VJ] because each trader's cash is
depleted cumulatively, whereas parallel clearing gives E[V1] = ... = E[VJ] (pp.4-7). Sequential clearing yields
P_T = 16.61 with sigma = 3.188 and a clear alphabetic ranking A > B > C > D > E, while parallel clearing yields
P_T = 16.94 with sigma = 0.241 and intermingled trajectories (pp.9-11). Finally, `1906.01293v1` maps the Bitcoin
transaction network in quarterly snapshots from January 2009 to April 2013 (e.g. BC13Q1 with N = 5,997,717 users
and 15,205,087 directed links) and finds a phase transition at a bankruptcy threshold of about 0.1, above which
almost all users survive, plus a "house of cards" in which top PageRank/CheiRank users fail together.

## 5. Tail laws and intraday drawdown dragon-kings

`1407.5037v2` estimates power-law tails of 30-second epsilon-drawdowns and drawups across the most liquid index
futures from January 2005 to December 2011. Exponents run about 4 to 5.67 (AEX 4.25, DAX 4.00, ES 4.85, ASX
5.67), larger than return exponents of about 3.5 to 4.5, with average differences of +0.29 for drawdowns versus
negative returns and +0.43 for drawups versus positive returns (pp.13-14). More than 50% of tail drawdowns contain
no tail log-returns, and the mean/median relative contribution of large individual returns to a large drawdown is
only about 0.35-0.45 (pp.15-16): drawdowns are built from many moderate moves, not one jump.

The tail itself is not a single power law. Extreme events deviate significantly above the fitted power law as
dragon-kings: the May 6 2010 flash crash; CAC futures on December 27 2010 at 09:03 CET with a normalized drawdown
of 214.99 (about a 215-sigma event) followed by a drawup of 157.99; the July 7 2005 London bombings; and a TAMSCI
drawup of 127.45 on September 10 2009 (p.18). Large drawdowns and drawups tend to occur fast, with a clear
relation between size and speed but none between size and duration, so duration-based limits are the wrong tool and simple return VaR understates intraday gap risk.

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| Deleveraging is contagion (`1805.08454v1`) | docs/concepts/orders | Add a margin-call loop |
| MM inventory caps (`2208.13654v1`) | docs/concepts/order_book | Model inventory and depth |
| Cost: sqrt stocks, 0.25 bonds (`2105.08377v1`) | docs/concepts/execution | Stress vol and volume |
| Liquidity risk is size x horizon (`2505.15296v1`) | backtest | Cap per horizon; split impact |
| Redemption coverage depends on policy (`2110.01302v1`) | risk layer | Tie metrics to horizon |
| Sequential clearing bias (`2509.01683v1`) | backtest | Use batched clearing |
| Drawdown dragon-kings (`1407.5037v2`) | docs/concepts/reports | Avoid tail extrapolation |
| Crowding fattens tails (`2002.03319v1`) | risk layer | Monitor concentration |
| Branching ratios are asymmetric (`2012.04181v1`) | python/nautilus_trader/indicators | Add a self-excitation gauge to cross-asset risk |
| Crypto deleveraging spirals (`1906.02152v3`) | live | Model cascades; venue backstops |
| Network centrality fails (`1906.01293v1`) | docs/concepts/networking | Add concentration risk |
| End-of-day spreads are noisy (`2105.08377v1`) | docs/concepts/data | Filter before risk engine |

## Caveats

The flash-crash evidence is simulation, not observation: `1805.08454v1` is not calibrated to any real fund
network, and `2208.13654v1` is calibrated to only four days of 2010 data with an uncalibrated spiking trader.
Several results rest on a single market or period: `2505.15296v1` uses one contract on one day with no drift,
`2105.08377v1` cannot calibrate its second cost regime from data and offers no out-of-sample validation of the
stress factors, and `2002.03319v1` covers one country and observes trades only when a Dutch firm is involved.
Assumptions frequently exclude real frictions: `1805.08454v1` assumes costless trading, a single venue, long-only
funds and uniform liquidation, while `2509.01683v1` and `1906.01293v1` carry no prices, costs or margin at all.
The crowding dispute between `1805.08454v1` and `2002.03319v1` is unresolved, and `2012.04181v1` shows two
systemic-risk measures disagreeing on the same data, so no single impact law or risk metric is settled. Finally,
`1407.5037v2` does not claim that dragon-kings are forecastable, and `1912.05273v1` offers review-level claims with no quantitative thresholds.

## Papers read in depth

- `1805.08454v1` - Understanding Flash Crash Contagion and Systemic Risk: A Micro-Macro Agent-Based Approach (2018). Deleveraging alone reproduces a crash; diversification effects are non-monotone.
- `2208.13654v1` - High-frequency financial market simulation and flash crash scenarios analysis (2022). Participation and inventory limits act non-monotonically on crash amplitude.
- `2505.15296v1` - Agent-based Liquidity Risk Modelling for Financial Markets (2025). A real matching engine yields square-root impact and a size x horizon risk surface.
- `2110.01302v1` - Liquidity Stress Testing in Asset Management - Part 3 (2021). RCR and shortfall depend on horizon and liquidation policy.
- `2105.08377v1` - Liquidity Stress Testing in Asset Management - Part 2 (2021). Bucket-specific power-law costs plus joint volatility, volume and spread stress.
- `2509.01683v1` - Sequential versus Parallel Clearing Mechanisms in Agent-Based Simulations (2025). Clearing order creates an artificial cross-sectional ranking.
- `1906.02152v3` - (In)Stability for the Blockchain: Deleveraging Spirals and Stablecoin Attacks (2021). Liquidation feedback is a real failure mode independent of individual risk controls.
- `2002.03319v1` - Crowded trades, market clustering, and price instability (2020). Clustering raises kurtosis and mainly the upside tail, causally in turmoil.
- `2012.04181v1` - Systemic Risk in Market Microstructure of Crude Oil and Gasoline Futures Prices (2020). Hawkes branching ratios expose a directional asymmetry that CoVaR hides.
- `1407.5037v2` - Power law scaling and "Dragon-Kings" in distributions of intraday financial drawdowns (2014). Tails are power-law in the body with dragon-kings above it.
- `1906.01293v1` - Contagion in Bitcoin networks (2019). Centrality plus a threshold produces simultaneous failure of top nodes.
- `1912.05273v1` - Systemic Risk: Fire-Walling Financial Systems Using Network-Based Approaches (2019). Robust-yet-fragile networks and depth-dependent fire-sale contagion.

## Where to next in the corpus

- `2605.10400v2` - perpetual futures on prediction markets stress test; a natural perp-venue extension kept out only to bound scope.
- `2607.08907v1` - herding liquidity-stress crossover ABM; relevant but ranked below the chosen empirical and ABM set.
- `2605.02436v1` - Cycles Protocol clearing; mechanism design rather than stress or impact evidence.
- `1912.04565v1` - market price of trading liquidity risk and market depth; impact estimation already covered by `2505.15296v1` and `2105.08377v1`.
- `2004.10951v1` - optimal execution with liquidity risk; belongs to the optimal-execution track.
- `2606.21784v2` - KineticSim GPU engine; performance work that could scale many-trajectory stress runs.
- `2601.02310v2` - T-KAN limit-order-book forecasting; prediction lead for anticipating stress rather than measuring it.
- `2002.07100v1` - world trade network contagion; a macro country-level analogue of the network channel.
