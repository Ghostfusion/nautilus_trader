# News and Language-Model Signals

Date: 2026-10-05. Revision 1.

This brief covers the "news and language-model signals" slice: 117 category records spanning
2000-2026, 16 with a journal reference, drawn from 40 screened candidates of which 12 were read in
depth. It is written for a practitioner researching, backtesting, paper-trading and running live
text-driven signals on a Python/Rust platform.

## What this covers

The slice spans finance-domain language models versus lexicon sentiment, event-time observability
and publication-timestamp rules, model data-freshness cutoffs, transaction-cost and turnover
modeling, ADV participation caps, short-sale constraints, deflated significance, and the
microstructure and horizon variables deciding where a text effect concentrates. The single most
consequential finding is that every strong headline number here is gross-of-frictions, and the
ordering of published Sharpe ratios tracks the strength of the cost, capacity and multiple-testing
discipline imposed rather than the quality of the text model. A platform should treat observability,
cost, capacity and deflated-Sharpe gates as pipeline stages, not post-hoc checks.

## The corpus slice

| Metric | Value |
| --- | --- |
| Records matching the category | 117 |
| Years spanned | 2000-2026 |
| Records with a journal reference | 16 |
| Candidate papers screened | 40 |
| Papers read in depth | 12 |

Selection rule: the seven mandate-listed papers plus five additions chosen to close named gaps in
publication-timestamp evidence, macro cross-asset textual alpha, sentiment-factor sizing,
dissemination breadth, and event-driven trading with explicit commissions, preferring papers that
print cost assumptions, timestamps and out-of-sample construction.

## The short answer

1. Finance-domain transformers dominate lexicons: LLaMA-3 accuracy 0.787, AUC 0.846 versus 0.503 and 0.512 (`2609.23703v1`).
2. Net performance collapses with cost and turnover: 111.78% annualized at 0 bps falls to 67% at 5 bps (`2507.18417v1`).
3. Same model, different protocol: OPT Sharpe 3.05 at 10 bps under random splits versus 2.45 at 5 bps under ADV caps (`2412.19245v1`, `2609.23703v1`).
4. Publication timing is a data contract: pre-09:30 ET news trades same day, after-close news only next day (`2609.23703v1`).
5. Freshness cutoffs decide validity: the out-of-sample window starts Jun 2024 to postdate LLaMA-3 pretraining (`2609.23703v1`).
6. Capacity binds where signal is strongest: a 10% ADV cap gates deployment (`2609.23703v1`); low-breadth effects run 1.72-8.69 bps (`2509.11970v1`).
7. Deflated significance separates real alpha from mining: deflated Sharpe 2.19 (p=0.014), White p=0.018, Hansen SPA p=0.026 (`2609.23703v1`).
8. A statistically strong effect is not tradable: overnight-minus-intraday 2.75 bps/day, about 7.2%/year, is not viable (`2507.04481v1`).
9. Effects concentrate by liquidity, session and horizon: coefficients rise into low-liquidity stocks (`2609.23703v1`); network risk peaks at 21-30 days (`1706.05812v2`).

## 1. Text beyond lexicon sentiment

Domain-adapted language models carry return-relevant information a dictionary cannot. On 973,481
tradable items for 3,452 firms, LLaMA-3 reports accuracy 0.787, AUC 0.846, Brier 0.151 and expected
calibration error 0.032 against Loughran-McDonald's 0.503, 0.512, 0.249 and 0.143, and the
predictive regression separates them at coefficient 0.312 (t=6.44) on N=190,236 versus 0.049
(t=1.31, insignificant) (`2609.23703v1`, pp.17, 29-30). The ordering replicates on 965,375 Refinitiv
articles, where OPT scores 0.744 accuracy, BERT 0.725, FinBERT 0.722 and the dictionary 0.501
(`2412.19245v1`, p.5), and FinDPO reaches weighted F1 0.846 against FinGPT v3.3's 0.762 and FinBERT's
0.611 (`2507.18417v1`, p.6).

Not all text signal is polarity. Quarterly co-occurrence networks from 17,398 SeekingAlpha articles
flag a 55.68% chance of decline at a 28-day delay against a 42.54% benchmark, a 13.14
percentage-point spread (`1706.05812v2`, p.22). A corporate-event detector is the other non-polarity
family, reaching a 54.5% win rate and 1.74% average return on one-day holds, ahead of BERT-CRF at
1.60% and Vader at 0.06% (`2105.12825v2`, p.7).

## 2. Costs and turnover collapse the net number

The ordering of published Sharpes tracks friction discipline, not model quality. FinDPO returns
747.10% cumulative and Sharpe 3.41 at zero cost, but at 5 bps annualized falls from 111.78% to 67%
and Sharpe from 3.41 to 2.0, with every other method turning low or negative at 4-5 bps
(`2507.18417v1`, pp.6-7). The mechanism is turnover: a daily full rebalance of a 35/35 equal-weighted
book means the net collapse measures how much of gross alpha is transaction-cost velocity, not signal.

Where authors impose cost as a first-class operator, the survivor set narrows. Under 5 bps one-way
cost and a 10% ADV cap, LLaMA-3 clears Sharpe 2.85, mean daily 0.34%, volatility 1.89%, MDD -12.3%
and cumulative 180% net, while OPT reaches 2.45 and the dictionary 0.68 with cumulative -9%
(`2609.23703v1`, p.30). The overnight effect makes the same point: even at 2.75 bps/day, the authors
state the extreme turnover required means the finding falls short of a viable strategy
(`2507.04481v1`, pp.2-3). The two papers omitting cost entirely are exactly the ones whose tradability
claims are out of scope (`2010.12002v1`, `2507.04481v1`).

## 3. Publication timestamps and data-freshness cutoffs as leakage controls

The time between publication and the first eligible decision is where leaks hide. The MFAST
event-time rule encodes it: pre-09:30 ET articles may trade same day; intraday, after-close, weekend
and holiday articles only from the next trading day; the day-0 return counts only for pre-open news;
and label windows are truncated at chronological split boundaries (`2609.23703v1`, pp.15-22). The price
of ignoring this is quantified: the tradable Day 1 portfolio earns Sharpe 1.64 while non-tradable
Day 0 and Day -1 look-ahead variants reach 3.66 and 4.20, so the 2.0-2.6 Sharpe gap is the value of
information latency (`2010.12002v1`, p.3).

The second leakage channel is the model. Because pretraining may have seen the evaluation period,
out-of-sample windows must postdate disclosed cutoffs: the primary test window Jun 2024-Jan 2026
postdates LLaMA-3 8B's March 2023 and 70B's December 2023 cutoffs, with Jan-May 2024 held back as a
release-date buffer and retrieval disabled (`2609.23703v1`, pp.15-22). By contrast, a random 20% test
split can leak both cross-sectional and time structure (`2412.19245v1`). Two look-ahead artifacts are
flagged rather than hidden: the Day 0/-1 portfolios (`2010.12002v1`) and the Trade-At-Best policy,
reported at 9.11% and 11.53% average return but unreachable live (`2105.12825v2`, p.7).

## 4. Capacity, participation caps and short-sale constraints

Capacity decides which effects deploy. Only one paper imposes and tests a participation cap,
restricting |dq| to 10% of average daily dollar volume (`2609.23703v1`, pp.15-22); most report no
capacity limit. The macro XGBoost study prints Sharpe 5.87 with CAGR 55.4% on EUR/USD and cumulative
cost 0.232, but no capacity or position-sizing analysis, which is why daily-signal Sharpes above 4.6
remain unreconciled with realistic frictions (`2505.16136v1`, p.7).

Short-sale and borrow constraints bind where the cross-sectional signal is strongest. A one-standard-
deviation sentiment innovation moves prices 1.06 bps with rho=0.940 and an 11.2-month half-life, and
the D10-D1 sort earns 4.0 bps/month at Sharpe 0.18-0.85, but effects concentrate in retail-tilted and
non-optionable stocks where shorting is hardest, with triple interactions reaching 31.0-12.0 bps in
high-VIX regimes; net Sharpe at 0/5/10 bps costs is 0.31/0.27/0.23, so the realistic ceiling is near
0.3 (`2509.11970v1`, abstract, p.4). Shared news topics also create correlation between seemingly
unrelated names, so a risk model treating names as independent will misstate overnight exposure
(`2507.04481v1`), meaning capacity must be analyzed jointly with correlation structure.

## 5. Deflated significance and the multiple-testing gap

Only the MFAST paper reports the full battery of block bootstrap, Diebold-Mariano, White reality
check, Hansen SPA and deflated Sharpe (`2609.23703v1`, pp.15-22). Its numbers are the ones to trust
precisely because the corrections are printed: Sharpe 95% CI [2.31, 3.34], daily alpha 0.182% with
interval [0.109%, 0.252%], Diebold-Mariano -7.12 at p<0.001, White p=0.018, Hansen SPA p=0.026 and
deflated Sharpe 2.19 at p=0.014; the gap from naive 2.85 to deflated 2.19 is the mining haircut
(`2609.23703v1`, p.32).

The sentiment-shock study is the only other paper applying family-wise corrections, using Romano-Wolf
and Holm adjustments (`2509.11970v1`). Some report only weak significance: simple news trading carries
a one-sided Wilcoxon p=0.0025 and t-test p=0.0052 (`1807.06824v1`, p.21). The transfer-entropy study
corrects with Benjamini-Yekutieli FDR<0.05 and still finds significant uncertainty reduction from
lagged hourly sentiment, persisting for two-hour-old sentiment (`2010.12002v1`, p.3). Without a common
standard, published Sharpes spanning 5.87 down to 0.05-0.12 cannot be compared at face value.

## 6. Where the effects concentrate

Three axes recur. Liquidity first: transformer coefficients rise from high- to low-liquidity stocks
for every model, steepest for LLaMA-3 and OPT (`2609.23703v1`, p.31). Session second: intraday and
overnight returns behave as separate regimes with opposite news sensitivities - intraday-to-intraday
autocorrelation 0.19, overnight-to-overnight 0.29, intraday-to-overnight -0.27, overnight-to-intraday
-0.26, close-to-close only 0.04 - with topic persistence at 0.94 combined, 0.9 intraday and 0.79
overnight and a characteristics-adjusted intraday-minus-overnight baseline of -3.6 bps
(`2507.04481v1`, pp.3, 9, 18-19). Horizon third: news-network risk peaks at 51.59% versus 42.00% in
the 21-30 day bucket, a 9.59 percentage-point and 13.41 standard-deviation difference
(`1706.05812v2`, pp.22-23).

Two further concentrators shape the pipeline. A two-mode jump model over 72 five-minute bins a day
selects about 10% of intraday releases at gamma=0.5, and the classifier trained on those
liquidity-screened items separates positive from negative post-news drift better than return-only
labelling up to 150 minutes (`2304.05115v1`, pp.12-14). Dissemination breadth does the same: BERTopic
clustering lifts binary accuracy from 55.0% to 63.0%, degrading below a 40% cohesion ratio
(`2412.10823v2`, p.4).

## 7. Operational diagnostics as model-selection criteria

Once a text model clears the alpha gates, its compute profile becomes a selection variable. LLaMA-3
peaks at 54.2 GB GPU memory and 112.0 ms per article for 8.9 articles/s, RoBERTa at 4.8 GB and 9.4 ms
for 106.4/s, and the LM dictionary at 0.5 GB and 2.3 ms for 434.8/s (`2609.23703v1`, p.32). A slower,
higher-memory model is only worth its cost if net alpha survives cost, capacity and multiple-testing
correction. Training itself can be modest: a preference-tuned 8B model trains in 4.5 hours on a
single A100 40 GB with 41.9M trainable parameters, 0.52% of the base, via LoRA rank 16
(`2507.18417v1`, pp.2-6).

The staging that makes diagnostics meaningful is to compute calibrated probability score, rank,
participation cap and net return as gross minus one-way cost in one pass, reporting deflated Sharpe
and reality checks rather than assuming them (`2609.23703v1`). Rankability is a requirement: demand a
probability output rather than a discrete valence label, with temperature scaling fitted on data
disjoint from the traded corpus (`2507.18417v1`, pp.2-6).

## What this project can take from it

| Finding | Where it lands | What to do |
| --- | --- | --- |
| News time maps to different eligible decision times | data layer, docs/concepts/data | Encode the timestamp-to-decision rule in the event model |
| Chronological splits with truncated labels | backtest, docs/concepts/backtesting | Truncate labels at split boundaries |
| Net return is gross minus cost and turnover | execution layer, docs/concepts/execution | Rerun signals at 1-5 bps and print turnover elasticity |
| 10% ADV participation cap | risk layer, python/nautilus_trader/risk | Cap order size at a fraction of ADV before ranking |
| Strongest effects in hard-to-short names | risk layer, python/nautilus_trader/risk | Check borrow and shortability before sizing |
| Deflated Sharpe, reality check, SPA | python/nautilus_trader/analysis | Add multiple-testing corrections to the report |
| Calibration precedes a rankable score | python/nautilus_trader/model | Return probabilities, not discrete classes |
| Liquidity-regime state gates signal-bearing news | docs/usermanauls/microstructure-signals | Screen news by a 5-minute mode switch |
| Overnight and intraday are different regimes | docs/usermanauls/intraday-systematic | Scope signals by session, do not pool returns |
| Novelty screens and dissemination breadth | data layer, python/nautilus_trader/data | Deduplicate and feed cluster size as a feature |
| Event taxonomy beats pure sentiment for single names | python/nautilus_trader/decision_bridge | Model event type and directional mapping |
| Latency and memory are model-selection inputs | docs/usermanauls/production-operations | Report cost per article alongside net alpha |

## Caveats

The evidence is dominated by simulation on historical text and daily returns; one paper is explicit
that it is not an order-book simulator, excluding intraday liquidity, queue position, hidden
liquidity and strategic interaction (`2609.23703v1`, p.36). Almost everything is English-language U.S.
equities, with exceptions a German ad hoc-announcement study (`1807.06824v1`) and three macro
instruments (`2505.16136v1`). Several results rest on one short period: a two-year return window
(`2412.19245v1`), a 14-month event sample including the COVID crash (`2105.12825v2`), and 22 quarters of
news networks (`1706.05812v2`). Cost treatment is inconsistent, from 0.3% per transaction to 10 bps,
5 bps one-way, and none at all (`2105.12825v2`, `1807.06824v1`, `2412.19245v1`, `2609.23703v1`,
`2507.18417v1`, `2010.12002v1`, `2507.04481v1`). Only two papers apply family-wise multiple-testing
corrections, so most reported significance is raw. Contested results remain: cost survival differs between a
paper where only FinDPO is positive at 5 bps (`2507.18417v1`) and one where MFAST ranks stay stable to
50 bps (`2609.23703v1`), irreconcilable without matched turnover. No paper reports a forward
paper-trading record, so publication is not live validation.

## Papers read in depth

- `2609.23703v1` - Financial Language Models as Applied Artificial Intelligence Systems for News-Based Trading under Market Frictions (2026). The reference protocol for observability, cost, capacity and deflated-Sharpe gates.
- `2412.19245v1` - Sentiment trading with large language models (2024). Gross upper bound under random splits and no capacity cap.
- `2507.18417v1` - FinDPO: Financial Sentiment Analysis for Algorithmic Trading through Preference Optimization of LLMs (2025). Turnover, not signal, decides net performance.
- `1807.06824v1` - News-based trading strategies (2018). Small German sample with large daily returns unreconciled with frictions.
- `2507.04481v1` - Does Overnight News Explain Overnight Returns? (2025). Establishes the overnight/intraday regime split and states the effect is not tradable.
- `2304.05115v1` - Towards systematic intraday news screening: a liquidity-focused approach (2023). Liquidity-regime screening as a labelling gate; no P&L.
- `1706.05812v2` - News-sentiment networks as a risk indicator (2017). Cross-sectional risk indicator with a 21-30 day horizon, not a return signal.
- `2505.16136v1` - Interpretable Machine Learning for Macro Alpha: A News Sentiment Case Study (2025). Reproducible pipeline, but Sharpes far above the friction-aware literature.
- `2509.11970v1` - Sentiment Feedback in Equity Markets: Asymmetries, Retail Heterogeneity, and Structural Calibration (2025). Realistic net ceiling near 0.3 Sharpe; shortability binds.
- `2412.10823v2` - FinGPT: Enhancing Sentiment-Based Stock Movement Prediction with Dissemination-Aware and Context-Enriched LLMs (2024). Dissemination breadth as quality metadata on a small panel.
- `2105.12825v2` - Trade the Event: Corporate Events Detection for News-Based Event-Driven Trading (2021). Event taxonomy plus explicit commissions, with look-ahead policies flagged.
- `2010.12002v1` - On the impact of publicly available news and information transfer to financial markets (2020). Quantifies the price of information latency.

## Where to next in the corpus

- `2403.12285v1` - FinLlama family, subsumed by the preference-optimization paper that benchmarks against it and prints cost sensitivity.
- `2310.04027v2` - retrieval-augmented financial sentiment; classification benchmark worth mining for feature design, no frictions.
- `2206.00648v2` - PreBit, bitcoin extreme-movement prediction from tweets; a crypto text branch pending a cost rerun.
- `2501.05232v1` - Tether mint/burn events as an alternative to headline sentiment in crypto.
