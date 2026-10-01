# TradingAgents capability review: evidence integrity, provider governance and fill honesty

**Status:** design only. No code was changed by this probe.

The companion implementation plan is
[tradingagents_lessons_implementation.md](tradingagents_lessons_implementation.md).

TradingAgents is a local LLM research framework that already treats this repository as a teacher for
execution and evaluation rigour (`docs/design_nautilus_trader_enhancements.md` in that project).
This review asks the opposite question, and answers it item by item:

**Which research-integrity, provider-governance and fill-honesty mechanisms should this project
acquire, given what TradingAgents implements and what this project already is.**

| Revision | Change                                                                                                                                  |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| 1        | Initial record from a read-only probe of the local checkout at `a7fb99a`, the four-slice inventory and the fourteen candidate learnings |
| 2        | Citation corrections from an independent verification pass, and a per-item classification table added to section 8                      |

## 1. Purpose and scope

TradingAgents is not a trading engine. It reads market, fundamental, news, flow and macro data from
about thirty vendors, feeds a graph of LLM roles, and emits a written decision plus a JSON decision
document. It never routes a live order; it calls itself analysis-only and advisory. That is exactly
why it is interesting here: it has had to solve, in public and in code, three problems that this
repository also has, and it has solved them at a different point in the stack.

The three problems are:

1. **A data layer that must be trustworthy before anything is computed on it.** Vendors fail,
   rate-limit, return wrong-but-plausible values, and serve forming bars. A pipeline that treats a
   failed fetch as an empty value, or a stale bar as a current one, has no idea which of its numbers
   mean anything.
2. **A computation layer whose output must not be substitutable by a plausible story.** A number
   that a model narrated rather than computed is indistinguishable from one that was computed,
   unless the pipeline refuses to accept the narrated one.
3. **A fill model that must never invent a price.** A bar-level simulation can flatter a strategy
   by filling at a price the bar never offered, by ignoring a gap, or by reading a bar's own close
   as a signal and then filling on that same close.

This repository owns (1) for its adapters and catalog, has only partial answers for (2) in its
research layer, and owns (3) in its matching engine. The candidate learnings below are placed
accordingly.

### 1.1 The harvesting rule

Only mechanisms enter: a data-integrity contract, an evidence contract, a digest discipline, a
verification gate, a provider-governance practice, a fill-honesty guard. No LLM role, no prompt, no
agent architecture, and no market-specific semantic enters. Where an item is inseparable from the
source's LLM architecture, it is recorded as a *discipline* (a rule this project can state and test)
rather than as a capability, and where it is inseparable from a market rule this project has already
refused, it is recorded as refused.

The source is a decision-support system; this project is an execution and research platform. The
harvest is therefore two-sided: some items are capabilities this project lacks, and some are rules
this project already follows but has never written down or tested. Both kinds are recorded, because
an untested rule is a rule that will be broken by the next contributor.

### 1.2 The licence, dependency and provenance boundary

TradingAgents is **Apache-2.0** (`LICENSE:1-3`). This repository is **LGPL-3.0-only**. The source's
own dependency list includes `langchain-core`, `langgraph`, `langchain-openai`,
`langchain-anthropic`, `langchain-google-genai`, `langgraph-checkpoint-sqlite`, `backtrader`,
`finnhub-python`, `moomoo-api`, `yfinance`, `stockstats`, `parsel` and `redis`
(`pyproject.toml:4-42` in that project).

The engineering rule, stated as a boundary rather than as a legal opinion:

- **No TradingAgents source code, test, fixture, prompt text, or copied documentation may enter this
  repository**, and no implementation artifact may be derived by mechanical transformation of any of
  them.
- **No dependency of that project may become a dependency of this repository** as a result of this
  review: not `langgraph`, not `yfinance`, not `backtrader`, not the `moomoo` SDK, and not any LLM
  client library. This project's adapter layer keeps its existing dependency policy.
- Independence is demonstrated by the same provenance chain the vectorbt review used, and every
  adopted item must be able to show every link of it:

```text
observed mechanism
      |
our own requirement, stated in our own terms
      |
our own interface
      |
our own mathematical specification
      |
our own implementation
      |
our own tests
```

Whether any particular use of an Apache-2.0 work is permissible is a licensing question, not a design
question, and this document does not answer it.

### 1.3 Preconditions

Two placement decisions are larger than any item here and are owned elsewhere. If either is answered
differently, this document changes rather than the answer:

1. **The research layer may hold evidence contracts and decision artifacts, but has no decision
   authority and no live execution authority.** Items L5, L7 and L13 create artifact shapes; none of
   them may gate, size or emit an order.
2. **The data layer's authority is unchanged.** Items L11 and L12 add declarations and audits about
   adapters; they do not add venues, do not add a vendor abstraction over the catalog, and do not
   change who owns an instrument's data.

## 2. Method and evidence

The probe was read-only, performed against the local checkout at revision `a7fb99a` on branch
`main`, in four slices: the data-provider layer, the agent and graph layer, the evaluation and
backtest layer, and the engineering discipline. Claims about the source are cited by path and line
against that checkout. Claims about this repository were verified against the working tree at code
revision `838ae29203` by reading the cited files, and then verified a second time by an independent
pass that re-opened every citation; that pass produced the corrections recorded as revision 2.

Coverage:

- read in full: `tradingagents/dataflows/interface.py`, `errors.py`, `registry.py`,
  `vendor_cache.py`, `vendor_breaker.py`, `market_data_validator.py`, `pit_registry.py`,
  `schema.py`, `agents/utils/evidence_gather.py`, `independent_vote.py`, `risk_tool_loop.py`,
  `prompt_metrics.py`, `strategies/score_engine.py`, `strategies/backtest_engine.py`,
  `backtest_models.py`, `market_tradability.py`, `scripts/backtest_strategy.py`,
  `contracts/research_decision.v1.schema.json`, and the source's `docs/AGENT_ONBOARDING.md`;
- read in part, by mechanism: the vendor modules named in section 4, the graph setup and conditional
  logic, the decision-packet and position-contract code, the score producers, the evaluation
  modules, the test-layout document, and the unused-surface audits;
- not reviewed: the source's `reports/` run outputs, `cli/`, `_papers26/`, the batch transcripts, and
  the upstream project it forks, which is a different codebase at a different revision.

The source's own documentation is treated as evidence of intent, never as evidence of behaviour. It
is materially stale in places: `docs/design_decision_context.md:3` still says "DESIGN - not built"
while later sections of the same document record the phases as built, and
`docs/design_multi_agent_debate.md:3` says "no code changed" while the structured debate modules
exist. Every behavioural claim below was therefore re-derived from code.

## 3. What TradingAgents is, mechanically

Four properties explain almost every learning, and the rest of the description exists only to
establish why particular mechanisms do or do not transfer.

- **A function-level vendor router with typed failures.** There is one primary entry point,
  `route_to_vendor(method, \*args, \*\*kwargs)` (`tradingagents/dataflows/interface.py:741`), with a
  second typed variant, `route_to_vendor_typed` (`interface.py:949`), and a dispatch table mapping a
  method name to a per-vendor callable (`interface.py:445-701`). A configuration string *is* the
  fallback chain; there is no silent fallback to an unlisted vendor (`interface.py:765-773`), and an
  explicit `none`/`off` returns a `DATA_DISABLED` sentinel (`interface.py:756`). Failure is surfaced
  as a typed exception whose class corresponds to a **router reaction** rather than to a
  human-readable cause (`errors.py:1-19`): rate-limited means try the next vendor, unconfigured means
  try the next vendor, no-data means try the next vendor.
- **An LLM graph whose decisions are gated by deterministic code.** Analysts with bounded tool
  loops feed researchers, a research manager, a trader, a three-persona risk debate and a portfolio
  manager (`tradingagents/graph/setup.py:207-283`; the source's `README.md:1056-1100`). The
  distinctive part is not the roster but the gating: the decision number is computed by
  `strategies/contract.py:1-6`, the debate's own deterministic verification runs before any judge
  (`strategies/debate_score.py:145,176-186`), and the JSON decision document is a schema with a
  recomputable hash (`contracts/research_decision.v1.schema.json`,
  `tradingagents/execution_contract.py:87-89,461-464`).
- **A deterministic score kernel under the model.** `strategies/score_engine.py` maps a raw value to
  a favourable 0-100 score (`score_engine.py:67-116`) and combines present components with
  renormalised weights, returning the score *with* its coverage, the components present, the
  components withheld and the reason (`score_engine.py:141-243`). A missing component never becomes a
  zero or a neutral fifty: below the coverage floor the composite is withheld.
- **A daily-bar backtest with a documented honesty budget.** `strategies/backtest_engine.py` and
  `scripts/backtest_strategy.py` replay daily OHLCV with fees, slippage and a tradability gate, and
  the source states its own limits plainly: "daily-bar model, no queue-position realism", advisory
  and analysis-only (`scripts/backtest_strategy.py:1-15`).

## 4. Capability taxonomy

The source's data layer covers about thirty vendors. Grouped by what they serve:

| Group                    | Vendors                                                                                                           | Notes                                                                      |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Equity market data       | yfinance, EODHD, tiingo, twelve_data, stockdata, massive, Alpaca                                                  | Free and keyed REST and library clients; EODHD is the primary daily source |
| Fundamentals and filings | SEC EDGAR (filings, XBRL history, full-text search), moomoo, FMP, Alpha Vantage, yfinance                         | Statement parsing with identity and scale guards                           |
| News and sentiment       | Benzinga, NewsAPI, GDELT, Seeking Alpha RSS, Reddit, StockTwits, massive, EODHD                                   | Passed through a coalescing cache and a staleness gate                     |
| Flow and positioning     | FINRA (Reg SHO short volume, ATS dark-pool), CBOE options, congress trades, Polymarket, float shares, PatentsView | Keyless official sources where they exist                                  |
| Macro and rates          | FRED, Federal Reserve (SOFR and par-yield curves), Treasury Fiscal (TGA)                                          | Look-ahead guarded on the observation end                                  |
| FX                       | fx (a thin yfinance adapter)                                                                                      | DXY and major pairs                                                        |

The abstractions over that set are:

| Layer                  | What it is                                                                                                                        | Citation                                              |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Router                 | One primary entry point and a typed variant, a per-method dispatch table, a configured chain                                      | `interface.py:445-701,741,949`                        |
| Error taxonomy         | One exception class per router reaction                                                                                           | `errors.py:1-19`                                      |
| Capability catalogue   | A machine-queryable registry of coverage, credentials and requirements                                                            | `registry.py:18-33,41-51`                             |
| Absence vocabulary     | `NO_DATA_AVAILABLE`, `DATA_UNAVAILABLE`, `DATA_DISABLED` with the chain-end reason                                                | `interface.py:756,900-945`                            |
| Result envelope        | Provider, `fallback_from`, `is_stale`, `stale_seconds`, `data_quality`, `missing_fields`, `price_caliber`, `volume_unit`, absence | `schema.py:26-90`; `market_router.py:97-110`          |
| Negative cache         | Refusals remembered per endpoint, not per symbol                                                                                  | `vendor_breaker.py:119,166-173`                       |
| Cache                  | Versioned key, forming-bar exclusion, sentinels never cached                                                                      | `vendor_cache.py:33-68,167-181`                       |
| Validator              | Ground-truth snapshot separate from the model, short-history flags, live-price sanity                                             | `market_data_validator.py:33-45,48-51,76-110,114-190` |
| Point-in-time registry | Masks later-dated rows absolutely                                                                                                 | `pit_registry.py:112-120`                             |

## 5. Where NautilusTrader stands today

Every line of this table was verified in this repository, with the file cited.

| Area                                  | TradingAgents                                                                                                                                                                                                                   | NautilusTrader today                                                                                                                                                                                                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Vendor integration                    | About thirty vendors behind one router, with typed fallback and a coverage registry                                                                                                                                             | Eighteen integrations, listed one per adapter in [docs/integrations/index.md](../integrations/index.md); each adapter's data client handles its own venue's subscriptions and requests (`docs/concepts/adapters.md`), so there is no chain to configure                 |
| Failure vocabulary                    | Typed errors keyed to router reactions; explicit absence strings; refusals cached negatively                                                                                                                                    | Typed denials and capability codes exist; a per-adapter capability *declaration* does not                                                                                                                                                                               |
| Missing data                          | A missing component is withheld with its reason; never scored as zero or fifty (`score_engine.py:141-243`)                                                                                                                      | A calculation reports a `MetricResult` - a status from a closed vocabulary plus a reason code - rather than an optional number (`crates/analysis/src/metric.rs:25-32,379`)                                                                                              |
| Staleness                             | Forming bars not cached; a >10-day-old latest row rejected (`vendor_cache.py:167-181`, `stockstats_utils.py:21,98-104,126-132`)                                                                                                 | A portfolio snapshot carries `is_stale`, `stale_instruments` and `unpriced_instruments` (`crates/model/src/events/portfolio/snapshot.rs:73-78`), but an individual market-data item has no staleness field, so a strategy cannot ask whether the tick it holds is stale |
| Point-in-time discipline              | A registry that masks later-dated rows absolutely (`pit_registry.py:112-120`)                                                                                                                                                   | Point-in-time membership and dataset contracts are implemented in `crates/research`                                                                                                                                                                                     |
| Validation separate from the producer | A deterministic snapshot builder that flags short-history indicators because the library silently substitutes a truncated window (`market_data_validator.py:33-45,114-190`)                                                     | Fill, fee, slippage and impact models are pluggable and independently configured (`crates/execution/src/models/`)                                                                                                                                                       |
| Number provenance                     | A per-run card hashing the data snapshot, the engine output and the model parameters (`prompt_metrics.py:258-330`)                                                                                                              | Identity contracts and digests exist for a study, trial, dataset, universe and result (`python/nautilus_trader/optimization/identity.py`)                                                                                                                               |
| Decision artifact                     | A versioned JSON decision schema with a recomputable self-excluding hash (`execution_contract.py:87-89,461-464`)                                                                                                                | No decision artifact exists, by design (invariant 7.1)                                                                                                                                                                                                                  |
| Bar fill honesty                      | Six pinned guards over seven recorded defects (`tests/test_backtest_fill_semantics.py:44-266`, `tests/test_next_bar_fill.py:34-64`; `CHANGELOG.md:45-60`)                                                                       | Gap-aware fills, resting stop-limits, fixed-point prices, a status gate and single-point slippage are present (section 8, L8 and L9); **the same-bar hazard is present and documented**                                                                                 |
| Tradability                           | A `market_tradability` gate, including a locked-limit rule                                                                                                                                                                      | `InstrumentStatus` and `MarketStatusAction` drive a matching gate; exchange price limits are a recorded non-goal; unmapped actions are dropped silently                                                                                                                 |
| Engine discipline gates               | Fifteen repo-wide gate tests that fail the build on a rule breach, plus gate-off byte-identity checks (`docs/developer/10-tests-layout.md:24-32`, `tests/test_decision_packet.py:70`, `tests/test_engine_ownership_map.py:110`) | Conversion hooks (docs, formatting, error, pyo3, testing, cargo conventions) and module-name parity tests exist; claim-binding gates do not                                                                                                                             |
| Pre-trade caps                        | Per-role output-token ceilings enforced by a wiring gate                                                                                                                                                                        | Rust risk caps in the send path, `count_caps` (`crates/risk/src/engine/config.rs:52,71`); reachable only from Rust                                                                                                                                                      |
| Reproducibility                       | A written statement that LLM output is never bit-identical and that backtests are a scaffold (`README.md:1269-1301`)                                                                                                            | Determinism is a design property: seeded fill and slippage models, no wall clock in simulation (`docs/concepts/backtesting/fill-models.md`)                                                                                                                             |

Three rows deserve comment.

**The vendor models are not comparable.** TradingAgents needs many vendors because one source is
often wrong, rate-limited or unentitled, and its answer is a chain with typed fallback. This
repository needs one authoritative source per instrument because a fill must be reproducible; its
answer is a typed client per venue. The lesson that transfers is not the chain but the *declaration*
of what a source can and cannot serve, and the *memory* of a refusal.

**The decision artifact has no home here.** A JSON decision document with a hash is meaningful in a
system that produces decisions; in this repository the closest analogues are an experiment digest and
a research result identity, both of which already exist. Any item in section 8 that creates a
decision artifact is therefore scoped to the research layer and forbidden from carrying authority.

**The source is strong exactly where this repository is thin.** Its bar-level honesty guards exist
because defects in that backtest produced and emitted flattering numbers - the source's own record
says the metrics were "computed on a flattered series" (`CHANGELOG.md:56`). This repository's
matching engine is more rigorous than that backtest in five of the six guards and *weaker in one*: an
order submitted from `on_bar(N)` with no latency model settles against the book left at bar N's
close, so a signal computed from that close can fill on that same close
(`crates/backtest/src/engine.rs:938-956`; `docs/concepts/backtesting/bar-execution.md`, Order
submission timing). The behaviour is documented, not hidden, and a latency model is the stated
mitigation; what the source adds is the *regression test* that makes the hazard impossible to
reintroduce silently.

## 6. Comparison and the structural difference

| Dimension        | TradingAgents                                           | NautilusTrader                                                     |
| ---------------- | ------------------------------------------------------- | ------------------------------------------------------------------ |
| Purpose          | Produce a researched, written decision                  | Execute and simulate orders, and support research on the same code |
| Unit of work     | A ticker-day, a graph run, a report directory           | An event, an order, a fill, a position                             |
| Data authority   | Many sources per series, ranked by a configured chain   | One source per instrument, typed per venue                         |
| Failure          | Degrade to the next vendor, then to an explicit absence | Refuse with a typed reason; a venue outage halts that venue        |
| Decision         | Produced by a graph, gated by deterministic code        | Produced by a strategy, gated by the risk engine                   |
| Determinism      | Non-deterministic by nature; recorded, not prevented    | Required; seeded and clock-independent                             |
| Money arithmetic | Floats (`float`)                                        | Fixed-point decimal (`Price`, `Quantity`, `Money`)                 |
| Verification     | Gate tests over wiring, prompts, contracts and docs     | Golden regression scenarios, digests, property tests, benches      |
| Language         | Python throughout                                       | Rust core, Python control plane                                    |

**The structural difference** is where each system places the checks. TradingAgents is a pipeline of
probabilistic steps, so it must place deterministic verification *around* each probabilistic step:
before the evidence, before the judge, before the artifact, before the report. This repository is
deterministic end to end, so it places its checks *in* the types: a non-finite price cannot exist, a
missing statistic is a typed absence, a denied order names its field. Both are correct for their
substrate. The transfer is therefore mostly in one direction - from a system that had to invent
external guards, to a system that can state the same rules as invariants and test them once.

## 7. Architectural invariants

### 7.1 No model, and no decision, enters the kernel

Nothing in section 8 may add an LLM call, a prompt, a model client or a network service the kernel
waits on, and nothing may give the research layer authority over an order, a size or a risk limit. A
research artifact may be recorded beside an order, exactly as an experiment digest is, and may not be
required by the execution path. The existing authority rule applies unchanged: a component that
touches PnL consumes the portfolio or states why it does not.

### 7.2 Mechanisms that must not enter the architecture

Recorded so that the boundary is explicit and testable:

- A vendor router with a silent fallback chain, for market data. An instrument's price has one
  authority; a chain that silently switches source mid-run makes a backtest unreproducible.
- Any relaxation of the fixed-point money types to accommodate a float-valued vendor.
- A model-generated number in a fill, a fee, a size, a limit or a statistic.
- Any blocking call to a model or a vendor inside the matching engine or the risk engine.
- The source's per-role output-token ceilings as a *risk* mechanism; they bound a prompt's length,
  not a position.
- Copied prompt text, role names or JSON field names from the source.

### 7.3 Licence, dependency and provenance

As section 1.2. In addition, every adopted item must record, in the implementation document, the
observed mechanism it came from, the source citation, and the words in which this project states the
requirement afresh. An item that cannot show that chain is not adopted.

### 7.4 Verification invariants

- A new guard arrives with a test that fails before it and passes after it, and the test states the
  *value* it protects, not merely that no exception was raised.
- A rule that can be checked mechanically becomes a hook or a test that fails the build; a rule that
  cannot is stated in this document and in the implementation record, and is not pretended to be
  enforced.
- Every claim in this document is cited by path and line, and was re-derived by an independent pass
  before revision 2 was recorded.

### 7.5 Provider neutrality and the data layer's authority

The data layer keeps one authority per instrument. Items L11 and L12 add a declaration and an audit
*about* adapters; they must not introduce a second price for the same instrument, must not introduce
a runtime dependency on a vendor SDK, and must not change which client owns an instrument's
subscription.

## 8. Candidate learnings

Each item states the observed mechanism with its citation, the requirement in this project's own
words, the home, and the failure mode if it is not adopted. The classification of each item appears
in the table below the list; the list itself is ordered by the work order.

### L1 Forced evidence gathering with recorded leaves

**Observed.** Before an analyst is invoked, a fixed tool set is run for it, so the composition of the
evidence is deterministic even though the prose is not. Each call becomes a recorded leaf with a
status of `ok`, `error`, `no_data` or `timeout`; a failure is a leaf, never an exception; the
arguments are hashed for identity; and a repeated request is served by a short-circuit node
(`tradingagents/agents/utils/evidence_gather.py:1-60,62,725-890`).

**Requirement.** When a research run assembles inputs, the set of inputs it intended to read must be
declared before the run, and each intended input must end as a recorded leaf with a status and a
reason. The record must be machine-comparable, so that two runs of the same declaration can be
diffed by the inputs each actually had.

**Home.** The research layer. This is the same shape as the capability type this project already
owns: availability, a canonical code, a human detail and the requirements not met
(`crates/core/src/capability.rs:58,129`). The item is to apply that shape to evidence, not to invent
a second one.

**Failure mode without it.** A study cannot distinguish "the input was read and was neutral" from
"the input was never read", which is the same defect class as a missing statistic scored as zero.

### L2 Uncertainty is not adverse evidence

**Observed.** The source states the rule and tests it rather than prompting for it: "An uncertainty
is not a bearish fact. NA / unavailable / unmeasured entries must never be counted as evidence
against a thesis. Only measured facts with an adverse sign are bearish"
(`docs/design_decision_context.md:579-581`, asserted by test per `:1820`). Structurally, the score
kernel renormalises over the components that are present and withholds the composite below a coverage
floor instead of substituting a neutral value (`strategies/score_engine.py:141-243`).

**Requirement.** Wherever this project aggregates, a missing measurement is excluded from the
denominator and reported as missing. It is never a zero, never a neutral midpoint, and never a
negative. A genuine zero stays a real zero, and the distinction is visible in the result.

**Boundary with L6.** This item governs the *membership of a denominator*: which observations are
allowed to influence an aggregate. L6 governs the *return value of a computation*: what a producer
does when it cannot produce a value at all. An aggregate over a partially missing input set is L2's
case; a computation that fails outright is L6's.

**Home.** Already the rule for statistics (`crates/analysis/src/metric.rs:25-32`) and execution
analytics (`crates/trading/src/analytics/`); the item is to make it a stated invariant of the
research layer's aggregates and to test it there, where a factor or a score may be aggregated.

**Failure mode without it.** A coverage-blind mean punishes an instrument for the vendor's outage,
which in a ranking is indistinguishable from a weak signal.

### L3 A computed number carries its binding budget

**Observed.** "The LLM argues the thesis; this computes the number. Size = min over independent
budgets (Kelly, risk-per-trade) scaled by volatility targeting, order-flow distribution and agreement
- all clamped to config caps and returned with an audit trail of which budget bound it."
(`tradingagents/strategies/contract.py:1-6`, wired at `graph/trading_graph.py:1083-1199` behind
`enable_position_contract`, default off.)

**Requirement.** Any computed size, limit or target that results from taking the minimum or the
maximum of several constraints must report which constraint bound it, and the reported value must be
reproducible from the recorded inputs without re-running the producer.

**Home.** The risk and portfolio layer (`crates/risk/src/python/sizing.rs`, the portfolio's target
pipeline). The record belongs beside the existing order-denied reason codes, which already name the
field that refused an order.

**Failure mode without it.** An unexplained size cannot be reviewed, and a change in the binding
constraint cannot be detected by a regression test - only the number would change.

### L4 A downgrade names its ground

**Observed.** A challenge pass "may downgrade only on one of the three closed grounds; it may not
create a new caution rationale ... it must never turn BUY -> HOLD because 'there is uncertainty'",
and the sole permitted mutation is one named field
(`docs/design_decision_context.md:784-790`, `strategies/decision_guardrail.py:88-91`), with the four
rules "enforced by code, not by prompt wording" (`:803-811`).

**Requirement.** Any mechanism that weakens or refuses a result must do so on a closed, named set of
grounds, and the name must travel with the result. A weakening for an unnamed reason is a defect in
the same class as a denial without a reason code.

**Home.** The risk engine and the research layer. This is already this project's convention for
denials (a denial names the field: `docs/concepts/execution/algorithms.md`, TWAP refusing the policy
keys it cannot honour); the item is to state it as an invariant of every weakening path and to test
that no unnamed ground exists.

**Failure mode without it.** A silent downgrade is unauditable, and its cause cannot be fixed
because it was never named.

### L5 A bounded decision packet is an information boundary

**Observed.** A packet is defined as "the bounded, deterministic decision context - research evidence
and decision-domain constraints, for adjudication. Not an order, not a recommendation and not a
gate", with a hard character budget (`packet_max_chars=12_000`, `engine_row_max_chars=160`) and a
truncation marker (`tradingagents/strategies/decision_packet.py:79-109`). The stated invariant is:
"Research can be large. Decision context cannot be large by accident"
(`docs/design_decision_context.md:76`).

**Requirement.** Where this project hands a research result to another component, the hand-off is a
bounded, deterministic document with a declared budget, and any truncation is recorded with what was
dropped and how much.

**Home.** The research layer, if and when a research result is handed to a consumer. This project has
no such consumer today, so the item is a *specification only* and is not to be built for its own
sake.

**Failure mode without it.** An unbounded hand-off is a hidden dependency: the reader's behaviour
changes with the size of an unrelated input.

### L6 A degenerate computation is refused, never substituted

**Observed.** The structured-output layer detects a monologue, a degenerate loop and a self-halt, and
the protocol states that a "free-text fallback must never ship a model's *private* drafting
monologue as the plan"; instead it "emits the explicit 'Decision: unavailable' notice"
(`docs/design_multi_agent_debate.md:143-148`, with the detectors and the notice in
`tradingagents/agents/utils/structured.py:385-405,603-605`). The same discipline appears in the
vendor layer, where a failure becomes an explicit absence string with a reason and sentinels are
never cached as data (`tradingagents/dataflows/interface.py:756,900-945`,
`vendor_cache.py:167-181`).

**Requirement.** A computation that cannot produce a trustworthy value returns a not-available state
carrying a reason; it never returns a degraded substitute, and a not-available result is never
recorded as if it were a value.

**Boundary with L2.** See L2: this item governs what a producer returns when it cannot compute at
all; L2 governs which observations may enter an aggregate.

**Home.** Everywhere this project already behaves this way (a statistic's status and reason, a
capability's unmet requirements); the item is to state the prohibition explicitly - *no substituted
values* - and to test the boundary cases, including that a sentinel or an absence is never persisted
as data.

**Failure mode without it.** A substituted value is worse than a missing one, because it is
indistinguishable from a measured value afterwards.

### L7 A self-excluding content hash, recomputed and flagged

**Observed.** A decision artifact carries `artifact_sha256` and `decision_hash` computed over its own
sorted-keys body **excluding the hash fields** (documented at
`tradingagents/execution_contract.py:87-89`), an integrity function recomputes it (`:104`), and a
mismatch is raised as `artifact_hash_mismatch` (`:461-464`). The check is deliberately non-fatal:
"Read-only and non-fatal by design: the report tree must still render when an artifact is malformed"
(`execution_contract.py:350-351`).

**Requirement.** A persisted research artifact carries a digest over its own content excluding the
digest, and a reader can recompute it and be told, explicitly, whether it matches.

**Home.** The research layer, beside the existing identity contracts
(`python/nautilus_trader/optimization/identity.py`) and the parity protocol
([parity_protocol.md](../developer_guide/parity_protocol.md)). The non-fatal reading rule is the part
worth copying: a malformed artifact is reported, not silently repaired, and not allowed to render as
valid.

**Failure mode without it.** An artifact edited after the fact is indistinguishable from the one that
was produced, and a digest that includes itself can never be verified.

### L8 A bar-level fill-honesty checklist

**Observed.** The source records seven bar-fill defects across two changelog entries
(`CHANGELOG.md:45-60`) and pins the resulting guards with tests that state the exact price and PnL
they protect (`tests/test_backtest_fill_semantics.py:44-266`, `tests/test_next_bar_fill.py:34-64`).
Six guards cover them:

| Guard                                                              | The defect it fixed                                                                                    |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| A stop gapped through fills at the gap, not the trigger            | A stop 95 on a bar opening 85 reported -600 instead of -1600 (`CHANGELOG.md:58`)                       |
| The exit scan starts after the entry bar for a next-bar-close fill | The entry bar's own low stopped the trade out (`CHANGELOG.md:59`)                                      |
| A stop-limit whose limit is unreachable does not fill              | A buy stop-limit at 100 filled on a bar that traded 106-110 (`CHANGELOG.md:60`)                        |
| A non-finite price cannot be committed as a fill                   | A NaN exit price propagated into `net_pnl`, the CSV row and the printed statistics (`CHANGELOG.md:49`) |
| Every exit leg obeys the tradability gate                          | A stop was booked on a locked limit-down bar (`CHANGELOG.md:47`)                                       |
| Every leg pays slippage                                            | Entry-only slippage understated the round trip by one leg, always favourably (`CHANGELOG.md:48`)       |

**Requirement.** This project states its own bar-level fill rules as a checklist, and every line of
the checklist is pinned by a test that asserts the price, not merely the absence of an error. The
checklist, with the state of this repository verified in code:

| Rule                                                                       | State here                                      | Evidence                                                                                                                       |
| -------------------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| A gap through a stop fills at the gap, not the trigger                     | Present                                         | `crates/execution/src/matching_engine/mod.rs:2022-2024,4564-4573`; the trigger-reached predicate is `matching_core.rs:414-427` |
| An unreachable stop-limit rests rather than filling at an impossible price | Present                                         | `matching_engine/mod.rs:6561-6622`                                                                                             |
| A non-finite price cannot exist at all                                     | Present, structurally                           | `crates/model/src/types/price.rs:86-90,195-196`; `crates/core/src/correctness.rs:630-634`                                      |
| Slippage applies at one point to every fill                                | Present                                         | `matching_engine/mod.rs:5137-5160`                                                                                             |
| A bar the instrument cannot trade on blocks the fill, not only the marking | Present for status; silent for unmapped actions | `matching_engine/mod.rs:4090`; see L9                                                                                          |
| A signal from bar N's close cannot fill on bar N                           | **Absent by default**                           | `crates/backtest/src/engine.rs:938-956`                                                                                        |

**Home.** `crates/execution` and `crates/backtest`, with the checklist recorded in
[bar-execution.md](../concepts/backtesting/bar-execution.md) and
[fill-prices-and-matching.md](../concepts/backtesting/fill-prices-and-matching.md).

**Failure mode without it.** The five present rules are not currently pinned by tests that state
their prices, so a future change to the matching engine could regress one of them without any test
failing. That is precisely how the source acquired its defects.

### L9 A tradability gate must guard every leg, and an unmapped status is not silent

**Observed.** The source's tradability gate is applied to the *exit* checks as well as to the
mark-to-market update, after a locked limit-down bar produced a booked stop
(`scripts/backtest_strategy.py:222,228-233`), and its `market_tradability` module models a suspended
market and a locked limit separately (`tradingagents/strategies/market_tradability.py:43-70,76`).

**Requirement, part one - every leg.** A bar on which the instrument cannot trade must block order
matching entirely, for resting orders and for new ones, and for every leg of a strategy's logic.

**Requirement, part two - no silent drop.** A trading-state input that this project does not act on
must be recorded as unhandled rather than discarded. Concretely, this repository maps six of the
sixteen `MarketStatusAction` variants - `Trading`, `PreOpen`, `Pause`, `Suspend`, `Halt` and
`Close`, in four arms - and drops the remaining ten with a catch-all
(`crates/execution/src/matching_engine/mod.rs:2569-2592`; the enum is
`crates/model/src/enums.rs:978-1011`). The dropped set includes `Quoting`, documented as "the
instrument is quoting but not trading", and `NotAvailableForTrading`, documented as "not available
for trading, either trading has closed or been halted". Neither changes the gate, which is
`market_status == MarketStatus::Open` (`matching_engine/mod.rs:3039,4090`), so after a venue has sent
`Trading` a later `Quoting` or `NotAvailableForTrading` leaves the simulated market matching. The
status type even carries `is_trading` and `is_quoting` as venue-provided optionals
(`crates/model/src/data/status.rs:40,54,56`; [instrument_status.md](../concepts/data/instrument_status.md)),
and those are not consulted by the gate either - the backtest exchange forwards only the action
(`crates/backtest/src/exchange.rs:1139`).

**Home.** `crates/execution` (the mapping and the gate), `crates/model` (if a variant needs a
state), and [instrument_status.md](../concepts/data/instrument_status.md), which currently does not
state which actions are honoured.

**Note on scope.** A full exchange price-limit lock is an explicit non-goal of this project and stays
refused (see [vnpy_lessons_implementation.md](vnpy_lessons_implementation.md), Explicit non-goals).
This item does not request it. It requests that a status the venue *did* send, meaning "not
tradable", stops being ignored, and that the unhandled set is visible.

**Failure mode without it.** A bar backtest that loads instrument status data can simulate fills on a
market the venue had declared unavailable, and no test or log records that the status was dropped.

### L10 Deterministic verification precedes any model judgement

**Observed.** The debate protocol runs a deterministic verification layer before any judge: a hard
breach (a violated claim or a malformed schema) triggers regeneration and then aborts to the baseline
rather than reaching the jury (`strategies/debate_score.py:145,176-186`), and "L2 never runs unless
L1 fully passes" (`docs/design_multi_agent_debate.md:43-44`).

**Requirement.** Where this project ever consults a non-deterministic component - a model, a
heuristic, a third-party score - the deterministic checks run first, and a failed check terminates
the path instead of being passed to the non-deterministic component for a verdict.

**Home.** A rule for the research layer, to be recorded now and applied if such a component is ever
added. Nothing in this repository currently consults a model, so there is nothing to change.

**Failure mode without it.** A model asked to adjudicate a malformed input produces a confident
answer about nothing, and the failure is laundered into a result.

### L11 A declared provider capability surface, with typed refusals remembered

**Observed.** Three mechanisms together: a machine-queryable registry of what each vendor can serve
and what credential it needs (`tradingagents/dataflows/registry.py:18-33,41-51`); an error taxonomy
where each class corresponds to a router reaction (`errors.py:1-19`); and a negative capability cache
keyed on the **endpoint, never the symbol** - the rule is stated as "``capability`` is the ENDPOINT
identity, never the symbol" (`vendor_breaker.py:166-173`, key built at `:119`) - which eliminated
measured duplicate refusals: 575 FMP 429s, 60 Massive 403s and 26 Finnhub 403s
(`docs/developer/03-dataflow-vendors.md:60-105`).

**Requirement.** Each adapter declares, in machine-readable form, what it can serve for which
instrument classes, what it requires to be configured, and which of its requests are known to be
refused; and a refusal is remembered against the request shape so it is not repeated within a run.

**Home.** `crates/adapters` plus the integration guides under `docs/integrations/`. This project
already owns the result shape - `Capability` with availability, a canonical code, a detail and the
requirements not met, rejecting a prose code at construction (`crates/core/src/capability.rs:58,129,212`),
with a source test that no caller classifies an answer by its detail
(`python/tests/unit/optimization/test_capability.py:49-53,194`). The item is a per-adapter
declaration built from that type, and a negative cache whose key is the request shape.

**Failure mode without it.** A user discovers an adapter's limits by failing in production, and a
rate-limited request is retried until the venue bans the client.

### L12 A per-provider unused-surface audit

**Observed.** A repeatable read-only study per vendor with a uniform method: enumerate the vendor's
full public surface by introspecting the installed package, extract the used set by grepping every
call site in the tree, then live-probe the unused set and record each failure's exact message,
because the audit's own conclusion is that neither code is proof on its own: "A 404 is not evidence
of absence, and a 403 is not evidence of absence either"
(`docs/design_eodhd_unused_surface.md:49`; the method and results are in
`docs/design_eodhd_unused_surface.md`, `docs/design_moomoo_unused_api_surface.md` and
`docs/design_finnhub_yfinance_unused_surface.md`).

**Requirement.** For each adapter, a documented audit of the vendor surface this project does and
does not use, produced by introspection of the client and a tree-wide call-site search, with the
unused set probed and each result recorded, and with wrong claims about a vendor corrected in the
same document.

**Home.** `docs/integrations/<venue>.md`, one audit section or a companion note per adapter.

**Failure mode without it.** An adapter silently lacks a data type a strategy needs, and nobody can
tell whether the vendor does not offer it or this project never implemented it.

### L13 A run provenance card

**Observed.** A per-run record emits `snapshot_id`, `data_snapshot_hash` (including the price
caliber), `engine_output_hash` and `model_parameters_hash`, all produced by `snapshot_identity`
(`tradingagents/agents/utils/prompt_metrics.py:258-317`), feeding a `run_card.json` (`:321-330`); a
prompt edit is made attributable by writing a per-run prompt condition to the prediction ledger, and
"a run with no recorded condition is `unavailable`, never pooled" (`CHANGELOG.md:726-730`).

**Requirement.** A result file carries the identity of everything that produced it, and a result
whose producer identity is unknown is excluded from any aggregate rather than pooled with the rest.

**Home.** The research layer, beside the existing experiment digests and result identities
(`python/nautilus_trader/optimization/identity.py`). The transferable rule is the exclusion rule, not
the LLM fields: an unidentified run is not evidence.

**Failure mode without it.** Two runs produced by different inputs are averaged as if they were the
same study.

### L14 A discipline gate that fails the build on a rule breach

**Observed.** Fifteen repo-wide tests are described as contracts rather than feature tests, and "a
change that ignores one of these fails the suite": wiring, dead-state dedupe, doc binding, prompt
signature, prompt trigger, test quality, execution contract, config isolation, gate toggles, API
reference environment table, tool binding, vendor signature, window integrity, report hygiene and
engine ownership, including a ban on un-failable test shapes
(`docs/developer/10-tests-layout.md:24-32`). New consumers land behind a default-false gate, and a
gate-off run must be byte-identical, checked by a dedicated test rather than by a general toggle test
(`tests/test_decision_packet.py:70`, `tests/test_engine_ownership_map.py:110`).

**Requirement.** A rule that a contributor must follow is checked by a mechanism that fails the build
when it is broken - a hook or a test - rather than by prose in a guide.

**Home.** The existing hook set (`.pre-commit-hooks/`, `scripts/check-*.py`) and the test suite. This
project already does this for formatting, docs conventions, error conventions, pyo3 conventions,
testing conventions, cargo conventions and public module names; the items above extend the same habit
to a fill-honesty checklist (L8), a status-mapping completeness check (L9) and an adapter capability
declaration (L11).

**Failure mode without it.** A convention that is only written down decays; a convention with a gate
cannot.

### Classification

| Item | Classification           | Why                                                                                          |
| ---- | ------------------------ | -------------------------------------------------------------------------------------------- |
| L1   | Capability               | The evidence contract does not exist here                                                    |
| L2   | Discipline               | Statistics already behave this way; the research layer's aggregates are not stated or tested |
| L3   | Capability               | No mechanism reports which constraint bound a computed size                                  |
| L4   | Discipline               | Denials already name a field; every weakening path is not yet covered                        |
| L5   | Specification only       | No consumer exists for a bounded research hand-off                                           |
| L6   | Discipline               | The not-available rule exists; the prohibition on substitutes is not stated as an invariant  |
| L7   | Capability               | The identity contracts exist; a self-excluding content hash on an artifact does not          |
| L8   | Discipline, plus one gap | Five of six rules are already true and unpinned; the sixth is absent                         |
| L9   | Defect class             | An unmapped "not tradable" status silently permits simulated fills                           |
| L10  | Specification only       | Nothing here consults a model today                                                          |
| L11  | Capability               | No per-adapter capability declaration or negative cache exists                               |
| L12  | Capability               | The integration guides describe features, not the unused surface                             |
| L13  | Specification only       | The exclusion rule is the transferable part; the artifact has no consumer                    |
| L14  | Discipline               | The hook habit exists; these three checks do not                                             |

## 9. Gaps the source does not close

Recorded so the harvest is not mistaken for a complete inventory.

- **Execution realism.** The source's own backtest is a daily-bar model with no queue position, no
  order-book depth, no partial fills and one intrabar path. This project's matching engine,
  fill-model set and queue-position option are strictly ahead; nothing transfers here.
- **Live execution.** The source routes no orders. Everything it knows about venue behaviour is
  second-hand, so nothing in it informs this project's execution client contracts, reconciliation or
  order-state machine.
- **Determinism.** The source records non-determinism instead of removing it, which is the right
  choice for an LLM pipeline and the wrong choice for a backtest. This project's seeded models and
  clock-free simulation are already the stronger arrangement.
- **Money arithmetic.** Floats throughout. This project's fixed-point types make an entire defect
  class (L8's non-finite price) structurally impossible, which the source has to guard against
  at runtime.
- **Adapters as products.** The source's vendors are library calls and REST seams; it has no client
  lifecycle, no subscription ownership, no reconnect, no instrument provider and no venue semantics.
  L11 and L12 borrow the governance, not the client.
- **Statistical machinery.** The source's evaluation modules (deflated Sharpe, CPCV, reality check,
  PBO) are its own; this project acquired the equivalent discipline from the vectorbt review, and the
  source adds nothing there.
- **Reproducibility of the model.** Nothing the source does makes an LLM run reproducible, and it
  says so (`README.md:1269-1301`). No item above asks this project to try.

## 10. Dependency graph

Contracts that must exist before the items that consume them.

```text
Capability shape (exists: crates/core) ------> L11 adapter declaration ------> L12 audit
                                           \--> L1 evidence leaf (research)

Absence/not-available rule (exists) ---------> L2 aggregation invariant ------> L13 provenance card
                                           \--> L6 no-substitution rule

Denial reason codes (exists) ----------------> L3 binding-budget record
                                           \--> L4 named downgrade ground

Identity contracts (exists) -----------------> L7 content hash ---------------> L13 provenance card

Fill-honesty checklist (docs) ---------------> L8 pinned tests
Status mapping (crates/execution) -----------> L9 gate completeness

Discipline gates (exists) -------------------> L14 extension to L8, L9, L11
```

Work order and independence:

1. **L9** is independent and smallest; it is a defect class with a citation and no new contract.
2. **L8** is documentation plus tests over behaviour that already exists, except its last row.
3. **L1, L2, L6, L7** are research-layer contracts and share the capability and identity shapes.
4. **L3, L4** extend the risk and denial vocabulary; they do not depend on 3.
5. **L11, L12** are adapter governance; L12 needs L11's declaration.
6. **L5, L10, L13** are specifications that should not be built until a consumer exists.
7. **L14** lands last, because it gates the items above.

## 11. Recommended order

| Phase | Items             | Why first                                                                                |
| ----- | ----------------- | ---------------------------------------------------------------------------------------- |
| 1     | L9, L8            | A cited defect class and a checklist over behaviour that already exists; no new contract |
| 2     | L1, L2, L6        | The evidence and absence contracts of the research layer                                 |
| 3     | L3, L4            | The computed-number and named-ground discipline in the risk vocabulary                   |
| 4     | L11, L12          | Adapter capability declarations and the per-adapter audits                               |
| 5     | L7, L13           | Artifact identity and the exclusion rule for unidentified results                        |
| 6     | L14, then L5, L10 | Gates, then the specifications that wait for a consumer                                  |

## 12. Decisions

| Id  | Decision                                                                                                     | Rationale                                                                                                                                                                         |
| --- | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T1  | The harvest is confined to evidence integrity, provider governance, fill honesty and verification discipline | The source's LLM architecture and market semantics are out of scope (section 1.1)                                                                                                 |
| T2  | No source code, test, prompt text or documentation enters this repository, and no source dependency is added | Section 1.2                                                                                                                                                                       |
| T3  | Nothing adopted may hold decision or execution authority                                                     | Invariant 7.1                                                                                                                                                                     |
| T4  | L9 is treated as a defect class to be fixed, not merely documented                                           | An unmapped "not tradable" status silently permits simulated fills (L9)                                                                                                           |
| T5  | The same-bar fill hazard is recorded as a known weakness with a stated mitigation, not as a defect           | The behaviour is documented in [bar-execution.md](../concepts/backtesting/bar-execution.md) and a latency model is the stated mitigation; the item is the missing regression test |
| T6  | An exchange price-limit lock remains refused                                                                 | It is an existing non-goal; L9 does not reopen it                                                                                                                                 |
| T7  | L5, L10 and L13 are specifications only until a consumer exists                                              | Building an artifact with no reader is the abstraction this project avoids                                                                                                        |
| T8  | The capability shape in `crates/core` is reused rather than extended with a second result type               | One shape, domain-scoped codes, already tested                                                                                                                                    |

### 12.1 Minimum acceptance per decision

| Item | Minimum acceptance                                                                                                                                           |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| L1   | A declared input set; one leaf per input with a status and a reason; two runs diffable by inputs                                                             |
| L2   | An aggregate over a partial input set excludes the missing member from the denominator and reports it; a test asserts the score is withheld, not substituted |
| L3   | A computed size reports its binding constraint; the value is reproducible from the record                                                                    |
| L4   | Every weakening path names a ground from a closed set; a test asserts no unnamed ground exists                                                               |
| L5   | A bounded document with a declared budget and counted truncation (specification only)                                                                        |
| L6   | A not-available result carries a reason; a test asserts no substitute value is produced or persisted                                                         |
| L7   | A digest over content excluding the digest; a reader recomputes and reports a mismatch without repairing                                                     |
| L8   | Each of the six rules pinned by a test asserting the exact price, including the same-bar rule                                                                |
| L9   | Every `MarketStatusAction` is either mapped or explicitly recorded as unhandled; a test asserts the mapping is total or the unhandled set is declared        |
| L10  | Deterministic verification precedes any non-deterministic judgement (specification only)                                                                     |
| L11  | A per-adapter declaration built from the capability shape, plus a negative cache keyed on the request shape                                                  |
| L12  | A documented audit per adapter with the used surface, the unused surface, the probe result and the corrections                                               |
| L13  | A result carries its producer identity; an unidentified result is excluded from aggregates (the exclusion rule is the part to adopt)                         |
| L14  | Every rule above that can be checked mechanically has a gate that fails the build                                                                            |

## 13. Remaining open questions

1. **Where does the evidence contract live** - in `crates/research` beside the panel and dataset
   contracts, or as a smaller type in `crates/core` beside `Capability`? L1 needs a home before it
   can be specified further.
2. **Is a decision artifact ever wanted here?** If the answer is no, L5, L7 and L13 reduce to the
   identity-contract rules this project already has, and only the exclusion rule survives.
3. **How far should an adapter declaration go** at a fine granularity - per data type, per instrument
   class, or per venue? The machine-readable form depends on the answer, and so does L12's audit
   template.
4. **Should the negative cache (L11) be process-local or persisted?** A persisted refusal is a
   configuration-shaped artifact and needs an invalidation rule.
5. **Does the same-bar fill hazard deserve a native next-bar fill mode**, or is a latency model plus
   a regression test the right permanent answer? This is a public API question and belongs to the
   owner.
6. **Which of L8's five present rules should be pinned first?** They are equal in principle; the
   cheapest is probably the gap-through-stop, whose behaviour is a single condition.
