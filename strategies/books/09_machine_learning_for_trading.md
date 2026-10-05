# Machine Learning for Trading: Benchmarks, Leakage, and Deployability

Date: 2026-10-05. Revision 1.

This brief covers machine learning for trading and its evaluation: large-scale risk-adjusted
benchmarks of deep sequence models, deep learning on order books, reinforcement learning for
trading and execution with its live-versus-backtest gap, LLM trading agents under bias-mitigated
and memory-controlled evaluation, lookahead and memorisation detection, data-snooping, and why
cost, turnover and latency decide deployability. The corpus slice holds 441 records matching the
category, spanning 2007-2026, of which 37 carry a journal reference; 40 candidates were screened
and 12 are treated here in depth.

## What this covers

The sub-topics are benchmark methodology that separates an edge from a lucky seed, joint bid/ask
and local-structure modelling of the order book, order-flow-imbalance alpha extraction feeding
reinforcement-learning agents, the live-versus-backtest gap from latency and alpha decay, the LLM
trading-agent literature and its bias-mitigated re-testing, memorisation diagnostics, and the
cost, turnover and attribution tests that decide deployability. The most consequential point is
that evaluation validity, not architecture, dominates: survivorship and lookahead bias,
training-window overlap, short test periods and absent costs inflate results across every model
family. The second is that the training loss and the reporting metric must be the trading
objective, since a single Sharpe hides turnover, tail risk and factor beta.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category | 441 |
| Years spanned | 2007-2026 |
| Records with a journal reference | 37 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

Selection covered every required item, then preferred papers carrying printed numbers and speaking
to execution realism, costs, capacity and evaluation validity over architecture-only work.

## The short answer

1. `2603.01820v1`: VLSTM Sharpe 2.40 (2010-2025), HAC t 8.81, CAGR 26.3%, against passive 0.48.
2. `2603.01820v1`: VLSTM turnover 966.9; the low-turnover iTransformer (36.3) still lags at 0.35 in the detail table (p.11).
3. `2603.01820v1` scores iTransformer 0.38 on the headline comparison (p.8); `2507.16548v2` ranks a Transformer above LSTM and buy-and-hold on risk-adjusted metrics.
4. Bid and ask move in lockstep only 17% of the time; model the joint distribution, `1601.01987v7`.
5. Spatial nets use 20,000 parameters vs 170,000 and 90 s vs 1,700 s on Amazon, `1601.01987v7`.
6. With predicted alpha the RL pipeline loses about 100,000/day on XAUUSD, `2311.02088v1`.
7. `2408.06361v2` reports 15-30% annualised; `2505.07078v6` finds no significant alpha (p>0.34).
8. 9 of 10 LLM agents post negative selection alpha despite large market and style beta, `2605.28359v1`.
9. The LAP x signal interaction is 0.162 (t=3.64) but t=1.06 post-cutoff, `2512.23847v2`.
10. JaxMARL-HFT runs 21,969 steps/s in 4 GB; learned market makers still lose 0.2 ticks, `2511.02136v1`.

## 1. Large-scale risk-adjusted benchmarks of sequence models

`2603.01820v1` is the reference protocol: daily futures and currencies 2010-2025 across five asset
classes, volatility targeted to 10 percent, gross optimisation with breakeven analysis added post
hoc, and a unified encoder-plus-linear-tanh head trained end to end on a differentiable annualised
Sharpe (pp.3-5). Printed Sharpes (p.8) run VLSTM 2.40 (2010-2025), LPatchTST 2.31 and TFT 2.27 down to
iTransformer 0.38 and Mamba 0.64, and Table 2 (p.11) shows why the mean misleads: VLSTM CAGR 26.3
percent, HAC t 8.81, turnover 966.9 and information ratio 0.854. The downside table (p.13) gives
VxLSTM maximum drawdown -11.8 percent (Calmar 1.64) against VLSTM -22.9 percent and passive -30.8
percent, while breakeven costs (pp.11-12) run from above 20 bps for VLSTM agricultural contracts to
tiny values on high-turnover liquid contracts. A 25-run top-5-seed re-run keeps VLSTM at Sharpe 2.40
and HAC t 8.86 (pp.12-14). The paper compares about fifteen models without a multiple-testing
correction and conditions everything on its protocol: no capacity or impact model, no intraday
realism.

## 2. Deep learning on order books and order-flow alpha

`1601.01987v7` trains a spatial network on NASDAQ Level III data, the first 50 bid and 50 ask
nonzero levels, across 489 US stocks from 1 Jan 2014 to 31 Aug 2015, about 50 TB raw, at a fixed
1-second horizon and at the next price move (about 5 and 2.5 billion samples; p.3, pp.28-29). It models
the joint distribution of future best bid and best ask changes through a locally-factored
geometric-like softmax, well-posed for bounded hidden units but not pure ReLU (pp.22-25), and beats a
standard 4-layer network on 94 percent of stocks (Case [1]) and 97 percent (Case [2]) with average
joint error decreases of 0.6 and 3.5 percent, and logistic regression and the naive empirical model
on 100 percent of stocks (pp.31-32). Lowest error on Amazon took about 1,700 s for the standard
network against 90 s for the spatial model, at 170,000 against 20,000 parameters (pp.41-42), and
Table 1 (pp.12-13) shows bid and ask move in lockstep only 17 percent of the time for half the stocks.
This is accuracy, not profitability; `2311.02088v1` supplies the alpha side, with out-of-sample R^2
from order-flow-imbalance features averaging 0.045 (XAUUSD) to 0.231 (DE40) (Table 7, p.23).

## 3. Reinforcement learning for trading and the live-versus-backtest gap

`2312.15730v1` models minute-bar Chinese index futures IF and IC with QTNet, a partially observable
MDP solved by Recurrent Deterministic Policy Gradient with an LSTM/GRU actor-critic, combining a
Dual Thrust demonstration buffer with behaviour cloning against a hindsight greedy expert and using
the Sharpe ratio as reward (fee 2e-5, slippage 0.15; pp.1-5). On IC futures (Table I, p.6) QTNet
returns 20.28 percent with Sharpe 0.562 and maximum drawdown 23.73 percent against Long & Hold -8.32
(Sharpe -0.318), and the ablation rises from GRU-only RDPG 8.96 percent (Sharpe 0.067) to the full
36.26 (0.742) (pp.6-7). A fixed Dual Thrust indicator earns Sharpe 0.810 on IC but -0.577 on IF,
whereas the learned policy gives 0.742 on IC and 0.523 on IF (p.7); results are single-run with no
seed variation. `2311.02088v1` is the failure manual: with true alphas tabular Q Learning earns
GBPUSD +18,600 and EURUSD +9,890 per day, but with predicted alphas the pipeline degrades by roughly
100,000/day on gold and only Q Learning on GBPUSD stays positive (pp.25-27); its Mann-Whitney win over
a random agent rests on 5 days and 1-hour live forward tests lose 2,400 and 2,640, attributed to
latency, since the bot skips timesteps and the live environment does not wait (pp.28-29).
`2511.02136v1` runs JaxMARL-HFT at 21,969 steps/s at 100 messages/step against PyMarketSim 463,
with a year of AMZN order data fitting in 4 GB (p.5); learned market-making still loses about 0.2
ticks with no net profit, and an adversarial MARL market maker degrades the execution agent (p.7).

## 4. LLM trading agents: promise, bias-mitigated reality, attribution, and behaviour

`2408.06361v2` surveys 27 hand-screened papers and reports the aggregate that LLM agents achieve
annualised returns of 15 to 30 percent over the strongest baseline in backtesting, presented as an
aggregate of heterogeneous studies rather than an independent result, with a median testing period
of 1.3 years (pp.5-6). `2505.07078v6` re-tests this with FINSABER on more than 7,000 US names from
2000-2024 including delisted S&P 500 constituents and real commissions (Moomoo standard US
$0.0049/share with a $0.99 minimum) under a selection-then-timing pipeline (pp.1-6): TSLA is the only
clear LLM win (buy-and-hold Sharpe 0.630 against FinMem 0.641 and FinAgent 0.546), buy-and-hold
beats both on AMZN and MSFT, paired t-tests show it significantly beats both LLMs (Random Five
buy-and-hold against FinMem p=3.0e-6), and neither produces significant CAPM alpha (all p>0.34)
(pp.4-7). `2605.28359v1` supplies the attribution: under a Qlib executor on CSI 300 with 5 bps buy
and 15 bps sell costs (pp.3-4), over 548 trading days qwen3.6-plus returns 85.29 percent (Sharpe
1.13) against an SFM Qlib baseline of 86.58 percent (Sharpe 2.02), and Barra attribution (Table 5,
p.8) shows 9 of 10 agents post negative selection alpha, Claude Opus 4.7 the only non-negative at
+0.2 percent, while market/Common contributions run +29.6 to +42.5 and style +11.8 to +29.2.

`2502.15800v3` asks whether LLM agents can stand in for human participants. In a controlled
experimental market with a constant fundamental, the mean squared errors against it (Table 1, p.4)
are Claude-3.5-Sonnet 0.536 and GPT-4o 0.789 against 429.8 for humans (text p.3), and although
Grok-2, Mistral-Large and GPT-3.5 produce human-like bubbles, none reaches human magnitude (p.4);
LDA shows 89.3 percent of human strategy texts load on "buy low; sell high" against 36.6 percent of
LLM texts, while 63.4 percent of LLM against 10.7 percent of human texts load on the
fundamental-value topic (p<.001). LLM agents under-generate bubbles and over-weight fundamentals.

## 5. Detecting lookahead and memorisation

`2512.23847v2` provides a portable diagnostic: Lookahead Propensity, LAP = P(up) + P(down), from the
first-token probabilities of a date-only recall query giving only firm name, ticker and target date;
regressing the realised outcome on the LLM signal, LAP and their interaction makes a positive
interaction a one-sided diagnostic for memorisation applied into the forecast (pp.3-14). On a
2020-07-29 query with no headline, Kodak's P(up) is about 0.9999 (pp.2-3), and in sample mean LAP
peaks near 0.88 in 2020 for stock news, about 42 percent abstaining and about 23 percent saturated
at >=0.95 (p.11). The econometrics (pp.19-22) give a headline-direction signal of 0.21 percent higher
next-day return per one-step move (t=12.18), an interaction of 0.162 (t=3.64) and a marginal effect
of 0.303 percent when LAP=1 against 0.141 percent when LAP=0, while the post-cutoff placebo in 2024
shows the maximum LAP below 1e-4 across 7,806 firm-day queries and an insignificant interaction
(t=1.06). The test identifies the sign of contamination, not its magnitude. `2605.28359v1` adds the
input-side defence: data-side masking anonymises tickers, calendar dates and tool-return timestamps
across prompts and tools (p.2), and a ten-attacker de-anonymisation probe recovers the top-1 ticker
in at most 3.0 percent of cases and satisfies the strict joint test in at most 1.5 percent (pp.2-3).

## 6. Costs, turnover and loss design decide deployability

The corpus is not comparable on costs: `2603.01820v1` optimises gross and treats cost as post-hoc
breakeven, `2507.16548v2` does not deduct costs, and only a minority (`2605.28359v1`,
`2505.07078v6`, `2311.02088v1`) deduct realistic costs. `2507.16548v2` makes the training loss the
deployability question: a Transformer (2 attention layers, 4 heads, sequence length 3-4) and an
LSTM are both trained to minimise the Mean Absolute Directional Loss, MADL = -(1/N) * sum sign(R_i *
Rhat_i) * abs(R_i), which pays only for correctly signed realised returns, on daily equities (S&P
500, XOM, JPM) and crypto (BTC, ETH, LTC) to 2024-10-24 in a walk-forward design (pp.3-8), explicitly
avoiding inappropriate loss functions, absent out-of-sample testing, forward-looking bias,
data-snooping bias, survivorship bias and improper metrics (p.3). On JPM (Table 2, p.9) the
Transformer earns aRC 11.89, maximum drawdown 56.01 and information ratio 0.44 against buy-and-hold
11.06, and on BTC (Table 3, p.9) aRC 92.86 and IR 1.97 against buy-and-hold 86.35 (p.10), but returns
are gross, signals change almost daily (1,000-1,700 trades over about 5,000 observations), and
equity drawdowns reach 30-56 percent with crypto up to 78.9 percent. `2601.04602v1` predicts
10-day-ahead S&P 500 correlations as a residual to a rolling baseline with an explicit
60-trading-day embargo after 2019 (pp.5-6), cuts MAE from 0.3071 to 0.2302 and raises Pearson from
0.310 to 0.778 on 269,121,442 out-of-sample edges (Table 1, p.14), and earns annualised Sharpe 1.84
against 0.75 with maximum drawdown -9.4 percent against -33.9 percent (pp.15-16), though no
transaction-cost PnL is reported.

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| Single Sharpe hides tails and seed variance | docs/concepts/backtesting, python/nautilus_trader/analysis | Report Sharpe with HAC t, Calmar, turnover and seed rank stability together. |
| Breakeven cost differs by model and contract | python/nautilus_trader/backtest, docs/usermanauls/execution-algorithms | Publish a breakeven-cost column beside every gross return. |
| Joint bid/ask, not constant spread | docs/concepts/order_book, python/nautilus_trader/model | Model the joint best-bid/best-ask distribution for market making and risk. |
| Local book structure cuts model size | docs/usermanauls/ai-training, python/nautilus_trader/model | Prefer locally-factored architectures on deep-book size features. |
| The market does not wait for the agent | python/nautilus_trader/live, docs/concepts/live | Measure decision-to-execution delay as a first-class live metric. |
| Predicted-alpha degradation flips backtests | python/nautilus_trader/backtest, docs/usermanauls/ai-training | Re-run every RL pipeline with deployed alpha quality, not oracle alpha. |
| Learned-agent replay shows endogenous impact | python/nautilus_trader/adapters, docs/concepts/backtesting | Add adversarial or reactive counterparties to impact simulation. |
| Published LLM returns are unaudited | docs/usermanauls/ai-training, python/nautilus_trader/analysis | Demand long windows, delisted names and cost-inclusive PnL before belief. |
| Attribution separates skill from factor beta | python/nautilus_trader/analysis, docs/concepts/portfolio | Decompose returns into market, style and selection alpha before crediting skill. |
| Masking beats prompt instructions for leakage | python/nautilus_trader/decision_bridge, docs/usermanauls/ai-training | Anonymise identifiers and calendar, then certify with a de-anonymisation probe. |
| Date-only recall exposes memorisation | python/nautilus_trader/decision_bridge, python/nautilus_trader/testkit | Log model cutoff and run the LAP plus interaction test on any in-window signal. |
| Train on the trading objective | docs/usermanauls/ai-training, docs/concepts/optimization | Use a directional sign-and-magnitude loss, not point-forecast error. |

## Caveats

The evidence is dominated by simulation and backtest rather than realised trading: most papers
report metrics or model-implied quantities and only a minority deduct realistic costs, so headline
edges are not comparable. Several results rest on a single market, period or venue - daily futures
and FX, NASDAQ over 20 months, five instruments over ten weeks, CSI 300, a single AMZN symbol-year
- so cross-asset generality is unproven. Out-of-sample discipline varies: `2312.15730v1` reports
one test year with no seed distribution, `2311.02088v1` draws significance from five days, and
`2603.01820v1` compares about fifteen models without a multiple-testing correction. The
architecture evidence is contested (weak in `2603.01820v1`, strong in `2507.16548v2` and
`2601.04602v1`), the LLM-agent evidence is contested the same way, and the leakage diagnostics
establish the sign of contamination, not its magnitude. No paper here provides a live,
cost-inclusive, multi-seed deployment record.

## Papers read in depth

- `2603.01820v1` - Deep Learning for Financial Time Series: A Large-Scale Benchmark of Risk-Adjusted Performance (2026). The reference protocol; Sharpe alone is not enough.
- `1601.01987v7` - Deep Learning for Limit Order Books (2016). Joint bid/ask modelling and local structure; huge compute, no trading test.
- `2311.02088v1` - Combining Deep Learning on Order Books with Reinforcement Learning for Profitable Trading (2023). Alpha degradation and latency dominate the live gap.
- `2312.15730v1` - Deep Reinforcement Learning for Quantitative Trading (2023). Reward and demonstration design matter; single-run results.
- `2511.02136v1` - JaxMARL-HFT (2025). GPU-scale MBO replay with adversarial agents; learned agents still lose ticks.
- `2502.15800v3` - LLM Agents Do Not Replicate Human Market Traders (2025). Bad proxies for human participants.
- `2408.06361v2` - Large Language Model Agent in Financial Trading: A Survey (2024). Useful taxonomy and a self-undercutting 15-30 percent headline.
- `2605.28359v1` - From Knowing to Doing: A Memory-Controlled Benchmark for LLM Trading Agents (KTD-FIN) (2026). Masking plus attribution shows factor beta, not skill.
- `2512.23847v2` - Detecting Lookahead Bias in LLM Forecasts (2026). A cheap, portable memorisation test with a cutoff placebo.
- `2505.07078v6` - Can LLM-based Financial Investing Strategies Outperform the Market in Long Run? (FINSABER) (2025). Long, breadth-correct, cost-aware; the edge vanishes.
- `2507.16548v2` - Alternative Loss Function in Evaluation of Transformer Models (2025). Directional loss aligns training with PnL; returns are gross.
- `2601.04602v1` - Forecasting Equity Correlations with Hybrid Transformer Graph Neural Network (2026). Residual correlation forecasting with an explicit embargo.

## Where to next in the corpus

- `2503.09655v2` - xLSTM deep RL for stock trading; revisit only if the xLSTM line needs a second source.
- `2508.02366v3`, `2502.11433v3`, `2509.11420v1` - LLM-agent variants overlapping the chosen benchmark studies.
- `2512.23515v2`, `2507.20474v3`, `2501.00826v3` - forecasting variants without cost-aware, bias-controlled evaluation.
- `2603.20965v2`, `2512.12250v1` - later LLM or forecasting preprints to test against the leakage diagnostics.
- `2505.19617v1`, `2105.00707v1`, `2501.17366v1`, `2406.18206v1` - hybrid and ARIMA-LSTM comparisons; architecture-report leads.
- `2411.12748v1`, `2407.16780v1`, `2407.18334v1`, `2505.19243v1` - volatility-forecast comparisons; leads for a risk-model axis.
- `2407.21791v1`, `2407.13688v1` - options and hedging work owned by the options category.
- `2307.10649v1`, `2212.14670v1` - RL VWAP and execution scheduling; belongs to the optimal-execution category.
- `2211.01346v2`, `2207.09951v1`, `2206.09041v1`, `2509.12456v2`, `2403.18831v1` - crypto AMM and market-making simulation leads owned by other categories.
