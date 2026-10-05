# TradingAgents lesson review: implementation record

Companion to [`tradingagents_lessons_design.md`](tradingagents_lessons_design.md), and to the two
earlier records it follows, [`vnpy_lessons_implementation.md`](vnpy_lessons_implementation.md) and
[`vectorbt_lessons_implementation.md`](vectorbt_lessons_implementation.md). Section references of
the form `design 8 L1` point at the design document, `T1` to `T8` are its decisions, `L1` to `L14`
are its candidate learnings, and `design 13.x` is its open-question list.

Nothing in this document is a delivery record. The design document is a plan; this is the matching
specification, and section 1.3 records every item as not implemented.

## 1. Status and scope

### 1.1 The defect-only mandate

**No production code was changed by this probe, and no production code has been changed since.**
The instruction that authorised the workstream permits a code change only for a confirmed defect.
The probe was read-only against both trees: it read files and ran text searches, and it wrote
nothing outside this document.

One item in the design document is classified as a defect rather than as a learning, and it is the
only path by which this workstream may change code: **L9's unmapped trading status**, recorded in
section 5.1 with its citations. The defect is *recorded* here, not repaired. Whether it is repaired,
and by whom, is a decision the owner of `crates/execution` makes; this document states the defect,
its exact location, its consequence and the acceptance test that would close it.

Everything else in section 4 is a specification. Each of the fourteen items is **not implemented**;
section 1.3 says so per item, and section 6 states the test or measurement that would prove each one
if it were built. Nothing here should be read as a claim that any of it works.

Three items - L5, L10 and L13 - are worse than unimplemented: they are specified in the design
document as items that **must not be built yet**, because this repository has no consumer for them
(`design 12 T7`). Building an artifact with no reader is exactly the abstraction this project
avoids, so those three are recorded as specifications and nothing more. Their subsections say so.

### 1.2 Revision history

This record was written after the design document was finished, from a second pass over the same two
trees, with every citation re-derived rather than copied. The design document was not edited.

| Revision | Change                                                                                                                                                                                                                                 |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1        | Initial record. All fourteen items retained from `design 8`. Fourteen mechanism specifications, one defect and thirteen design warnings, the acceptance and verification plan, the refused set and the carried-forward open questions. |

Two citations in revision 1 of the design document did not resolve to the code as written. Both were
re-derived here, reported, and then corrected in place in revision 2 of the design document, which is
the record of authority:

| Design revision 1 claim                                                  | Verified position and the correction applied                                                                                                              |
| ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `is_trading` and `is_quoting` at `crates/model/src/enums.rs:836,929`     | The fields are on the status type, at `crates/model/src/data/status.rs:40,54,56`; the cited lines were the `InstrumentCloseType` and `MarketStatus` enums |
| "maps five of the sixteen `MarketStatusAction` variants" (`design 8 L9`) | The match at `crates/execution/src/matching_engine/mod.rs:2569-2592` has four arms covering **six** variants; ten reach the catch-all (section 5.1)       |

Neither drift changes the defect, which is that the catch-all drops `Quoting` and
`NotAvailableForTrading` (section 5.1). Both are recorded here because a citation that lands on the
wrong file is itself a defect of the same class this workstream exists to prevent.

### 1.3 Implementation log

One row per item. **Every item is not implemented.** The last column says where its specification
lives. An item marked *specification only* is one the design document forbids building until a
consumer exists (`design 12 T7`); its row is still specified in full, because a specification is
what the design document asked for.

| Item | Mechanism adopted                                                                                                                                      | Status                                   | Specification |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------- | ------------- |
| L1   | A declared input set for a research run, and one recorded leaf per intended input carrying a status and a reason, built on the domain capability shape | Not implemented                          | 4.1           |
| L2   | An aggregation invariant: a missing measurement leaves the denominator and is reported, never substituted by zero, a midpoint or a negative            | Not implemented                          | 4.2           |
| L3   | A computed size, limit or target reports the constraint that bound it, reproducibly from the record                                                    | Not implemented                          | 4.3           |
| L4   | Every weakening or refusal names a ground from a closed set, and the name travels with the result                                                      | Not implemented                          | 4.4           |
| L5   | A bounded, deterministic hand-off document with a declared budget and counted truncation                                                               | Not implemented (specification only)     | 4.5           |
| L6   | A computation that cannot produce a trustworthy value returns not-available with a reason; no substituted value is produced or persisted               | Not implemented                          | 4.6           |
| L7   | A persisted research artifact carries a digest over its content excluding the digest; a reader recomputes and reports a mismatch without repairing it  | Not implemented                          | 4.7           |
| L8   | The bar-level fill-honesty checklist, each of its six rules pinned by a test that asserts the price                                                    | Not implemented                          | 4.8           |
| L9   | The tradability gate guards every leg, and an unhandled status action is recorded rather than dropped                                                  | Not implemented (defect recorded in 5.1) | 4.9           |
| L10  | Deterministic verification precedes any non-deterministic judgement, and a failed check terminates the path                                            | Not implemented (specification only)     | 4.10          |
| L11  | A per-adapter capability declaration built from the capability shape, with typed refusals remembered by request shape                                  | Not implemented                          | 4.11          |
| L12  | A per-adapter unused-surface audit: used set, unused set, probe result and corrections                                                                 | Not implemented                          | 4.12          |
| L13  | A result carries its producer identity, and an unidentified result is excluded from aggregates rather than pooled                                      | Not implemented (specification only)     | 4.13          |
| L14  | Every mechanically checkable rule above has a gate that fails the build, and a gate that cannot fail is not a gate                                     | Not implemented                          | 4.14          |

## 2. Probe provenance and reproduction

| Field                                  | Value                                                                                                                                                                                                                           |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source project                         | TradingAgents, a local LLM research framework checked out outside this repository                                                                                                                                               |
| Source revision probed                 | `a7fb99ab8c7f265a29967f9cbe24c68823f4f0ac` (`a7fb99a`), branch `main`                                                                                                                                                           |
| Source revision at the end of the pass | `ded6f73`, a direct child of `a7fb99a` that changes `CHANGELOG.md`, `docs/AGENT_ONBOARDING.md` and `docs/implementation_plan_value_screen_score.md` and nothing this document cites except one bullet body at `CHANGELOG.md:16` |
| This repository                        | `838ae292031c0e22c904f40557910631b99da16d` (`838ae29203`), branch `develop`, read from the working tree, which carried uncommitted modifications to unrelated files by concurrent work                                          |
| Method                                 | Read-only. Files read, text searched, no process run against either tree, no code changed                                                                                                                                       |
| Licence boundary                       | Source is Apache-2.0 (`LICENSE:1-3`); this repository is LGPL-3.0-only. No source file, test, fixture or prompt text entered this repository (`design 1.2`)                                                                     |

The source's HEAD advanced from `a7fb99a` to `ded6f73` while the pass was running. The diff between
the two is three files and twelve lines, and every source citation below was re-checked against both
revisions where the file is `CHANGELOG.md`, and against the working tree otherwise. The advance is
recorded because the design document names `a7fb99a` and a reader re-deriving a citation should know
which tree answered.

### 2.1 The four slices

The probe was performed in four slices. Each claim about the source is cited by path and line
against the checkout above; each claim about this repository is cited by path and line and was
re-derived from the working tree.

| Slice                         | What it covered                                                                                                                        | How its claims were verified                                                                                                                                                                                         | Where it lands               |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| Data-provider layer           | The router, the error taxonomy, the capability registry, the result envelope, the caches, the validator and the point-in-time registry | Read the modules named in section 4 in full; confirmed the absence vocabulary and the sentinel prefixes by search; confirmed the router entry point and the registry's credential table by reading their definitions | L1, L2, L6, L11, L12         |
| Agent and graph layer         | The evidence gatherer, the score kernel, the position contract, the decision guardrail, the decision packet and the run card           | Read each module's docstring and its public entry points; confirmed the closed grounds rule and the coverage floor by reading the enforcement sites, not the prose                                                   | L2, L3, L4, L5, L7, L10, L13 |
| Evaluation and backtest layer | The daily-bar backtest and its fill semantics, the tradability module, the test suite that pins the six fill guards                    | Read `CHANGELOG.md:45-60` and enumerated the tests by name in `tests/test_backtest_fill_semantics.py` and `tests/test_next_bar_fill.py`                                                                              | L8, L9                       |
| Engineering discipline        | The repo-wide gate tests, the default-off hardening toggles, the non-fatal integrity check                                             | Read `docs/developer/10-tests-layout.md:25-37`, the gate call sites in the graph, and the integrity function's docstring and call site                                                                               | L14, and the warnings in 5.2 |

### 2.2 What was not reviewed

- The source's `reports/` run outputs, its `cli/`, its `_papers26/` directory and its batch
  transcripts. They are outputs, not mechanisms.
- The upstream project the source forks, which is a different codebase at a different revision.
- The source's LLM integrations and prompt texts, which are out of scope by `design 1.1` and are
  refused by section 7 of this document.
- The source's `reports/.kilo/worktrees/` copies, which shadow `tradingagents/strategies/`; every
  source citation in this document resolves to the canonical `tradingagents/` tree, not to a
  worktree copy.
- Any source behaviour that would require running the source. Nothing was executed, so every source
  claim is a claim about code as written, and this document says code, not behaviour, throughout.

## 3. Verified state of this repository

Each row is a claim the design document relies on. The evidence is what establishes it, with the
line numbers as read from the working tree at `838ae29203`. The "kind" column is what the item in
section 4 is: a **new contract** (this repository has no such mechanism), a **discipline** (the
behaviour exists and the item is to state and test it), or a **defect** (the behaviour is wrong).

| Claim                                                                                                                                       | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Kind                                             |
| ------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| The capability answer type exists and is shared, with domain-scoped code sets                                                               | `crates/core/src/capability.rs:19-27` (module doc: the shape is shared and the code sets are not; only the code may be compared); `:38-47` (`is_canonical_code`); `:58-63` (fields `available`, `code`, `detail`, `requirements`); `:68` (`available`), `:88` (`unavailable`, with the canonical-code debug assertion), `:104` (`requiring`), `:117-131` (accessors)                                                                                       | new contract for L1 and L11, built on this       |
| The capability type has three independent producers, so the shape is proven rather than proposed                                            | `crates/analysis/src/metric.rs:707-712` (a metric's capability from its reason); `crates/model/src/events/order/denied_reason.rs:478-491` (a denial's capability, the code being the leading token); `crates/persistence/src/common/coverage.rs:615-642` (catalog coverage, folding gaps into requirements)                                                                                                                                                | evidence for `design 12 T8`                      |
| A statistic returns a status with a reason from a closed vocabulary, never a zero                                                           | `crates/analysis/src/metric.rs:379-...` (`MetricReason` with seven variants and an `ALL` slice); `crates/analysis/src/python/metric.rs:146` (the reason exposed to Python)                                                                                                                                                                                                                                                                                 | discipline over existing behaviour: L2, L6       |
| Execution analytics already reports an undefined metric as not available with a reason                                                      | `crates/trading/src/analytics/metrics.rs:73` (`UnavailableReason`); `:100-128` (`MetricValue`, with `NotAvailable(UnavailableReason)` and `value()` returning `None`); `:137-151` (`Metric` pairs a declaration with a value)                                                                                                                                                                                                                              | discipline: L2, L6                               |
| Identity contracts exist for a universe, a dataset, a computation, a study and a trial, each with a digest                                  | `python/nautilus_trader/optimization/identity.py:84-92` (`_digest`), `:146` (`UniverseIdentity`), `:204` (`DatasetIdentity`), `:277` (`ComputationIdentity`), `:349` (`StudyIdentity`), `:490` (`TrialIdentity`), with `digest()` at `:191`, `:264`, `:336`, `:433`                                                                                                                                                                                        | exists: L7, L13 build beside it                  |
| A parity protocol exists for a second implementation of one kernel                                                                          | `docs/developer_guide/parity_protocol.md:1-16`                                                                                                                                                                                                                                                                                                                                                                                                             | exists: L7's reading rule sits beside it         |
| The risk configuration is rate limits, a per-instrument notional cap, count caps, full-position-exit venues and debug                       | `crates/risk/src/engine/config.rs:55` (`bypass`), `:58` (`max_order_submit: RateLimit`), `:61` (`max_order_modify: RateLimit`), `:64` (`max_notional_per_order: AHashMap<InstrumentId, Decimal>`), `:71` (`count_caps: Vec<RiskCap>`), `:76` (`full_position_exit_venues: AHashSet<Venue>`), `:79` (`debug`)                                                                                                                                               | discipline: L3's record belongs here             |
| A denial is a typed reason, and the reason can render a capability                                                                          | `crates/model/src/events/order/denied_reason.rs:71` (`OrderDeniedReason`); `:478-491` (`capability()`, the code being the leading token)                                                                                                                                                                                                                                                                                                                   | discipline: L3, L4                               |
| Position sizers exist as a Python-visible surface with one built-in                                                                         | `crates/risk/src/python/sizing.rs:43` (`PositionSizer`), `:103` (the overridable `calculate`), `:125` (`FixedRiskSizer`), `:186-217` (its `calculate` delegating to `calculate_fixed_risk_position_size`)                                                                                                                                                                                                                                                  | discipline: L3                                   |
| Four execution algorithms exist, with a policy-part declaration and a refusal path                                                          | `crates/trading/src/algorithm/` contains `iceberg.rs`, `quote_pegged.rs`, `sniper.rs`, `twap.rs`; `policy.rs:60` (`PolicyPart`), `:292-308` (`declared_parts`), `:315-316` (`unsupported`); `mod.rs:129` (`supported_policy_parts`)                                                                                                                                                                                                                        | discipline: L4's convention already exists here  |
| The matching engine maps a status action to a market status, and the map has a catch-all                                                    | `crates/execution/src/matching_engine/mod.rs:2569` (`fn process_status`), `:2573-2592` (four arms, six variants, and `_ => {}`)                                                                                                                                                                                                                                                                                                                            | defect: L9                                       |
| The market status gates both order acceptance and matching                                                                                  | `crates/execution/src/matching_engine/mod.rs:3039` (`if self.market_status != MarketStatus::Open` on the validate path, refusing with "Market {} is {}, cannot accept order {}"); `:4090` (`if self.market_status == MarketStatus::Open` around the match loop)                                                                                                                                                                                            | defect context: L9                               |
| The status action vocabulary is sixteen variants, of which the last two are not tradability states at all                                   | `crates/model/src/enums.rs:978` (`pub enum MarketStatusAction`), `:986` (`Quoting = 3`, documented "the instrument is quoting but not trading"), `:1010` (`NotAvailableForTrading = 15`, documented "not available for trading, either trading has closed or been halted"); `:929` (`pub enum MarketStatus`), `:931` (`Open = 1`)                                                                                                                          | defect: L9                                       |
| The status type carries venue-provided trading and quoting state, and nothing in the execution crate reads it                               | `crates/model/src/data/status.rs:54` (`is_trading: Option<bool>`), `:56` (`is_quoting: Option<bool>`); a search of `crates/execution/src/` for either name returns nothing                                                                                                                                                                                                                                                                                 | defect: L9                                       |
| The status documentation describes the fields but not which actions the engine honours                                                      | `docs/concepts/data/instrument_status.md:9-18` (the field table, including `is_trading` and `is_quoting`), `:20-25` (behaviour: the optionals preserve venue state, the action is the normalised status, strategies may handle status updates)                                                                                                                                                                                                             | defect: L9                                       |
| A non-finite price cannot exist: prices are fixed-point integers                                                                            | `crates/model/src/types/price.rs:85-90` (`PriceRaw` is `i128` or `i64` by feature); `:177` (`pub struct Price`); `:195` (`new_checked`, returning a `CorrectnessResult`); `crates/core/src/correctness.rs:621-639` (`check_in_range_inclusive_f64` rejects NaN and infinity before the range test)                                                                                                                                                         | exists: L8's third rule                          |
| A gap through a stop fills at the gap, not the trigger                                                                                      | `crates/execution/src/matching_engine/mod.rs:2022-2032` (the gap-open branch sets `fill_at_market = true` before the open trade tick), `:4560-4575` (during bar H/L/C the fill is pulled back to the trigger price only when `fill_at_market` is false); `crates/execution/src/matching_core.rs:405-427` (`is_stop_matched_with_trigger_type`, the trigger predicate)                                                                                      | exists: L8's first rule                          |
| A triggered stop-limit whose limit is unreachable rests rather than filling                                                                 | `crates/execution/src/matching_engine/mod.rs:6568-6622`: after the trigger, the order fills only if the maker-inside test or `is_limit_matched` passes; otherwise its liquidity side is set and it is resynced into the book as a resting order                                                                                                                                                                                                            | exists: L8's second rule                         |
| Slippage is applied at one composition point, to every fill                                                                                 | `crates/execution/src/matching_engine/mod.rs:5137-5160` (the fill model decides eligibility, quantity and base price; the configured slippage model then decides, and an independent model is the single source, the fill model's own slippage not being consulted)                                                                                                                                                                                        | exists: L8's fourth rule                         |
| A bar the instrument cannot trade on blocks matching for resting and new orders                                                             | `crates/execution/src/matching_engine/mod.rs:4090` (the match loop is entered only when the market status is `Open`); `:3039` (new orders are refused when it is not)                                                                                                                                                                                                                                                                                      | exists: L8's fifth rule, partially               |
| A signal from bar N's close can settle against bar N's close                                                                                | `crates/backtest/src/engine.rs:941-956` (the bar is routed to the exchange and the venues settle at `ts_init` before the data engine dispatches the bar to modules at `:966`); documented at `docs/concepts/backtesting/bar-execution.md:99-110`                                                                                                                                                                                                           | known weakness: L8's sixth rule (`design 12 T5`) |
| No latency model is required for that ordering, so it is the default                                                                        | `docs/concepts/backtesting/bar-execution.md:101-102` ("Without a latency model, an order submitted from `on_bar` settles immediately against the book left at bar N's close"), and the next-bar warning at `:121-140`                                                                                                                                                                                                                                      | known weakness: L8                               |
| The matching engine's status behaviour is tested for the mapped transitions only                                                            | `crates/execution/tests/integration/matching_engine.rs:772-776` (cases `Pause`, `Suspend`, `Close` on `process_status`), `:834-845` (cases `Halt`, `Pause`, `Suspend`, `Close` on reset), `:810`, `:897` (`Trading` reopens); no test names any unmapped variant                                                                                                                                                                                           | defect evidence: L9                              |
| The venue path reaches `process_status` from replayed status data                                                                           | `crates/backtest/src/exchange.rs:1139` (`matching_engine.process_status(status.action)`)                                                                                                                                                                                                                                                                                                                                                                   | defect context: L9                               |
| The repository already has repo-wide gates, including a public-module parity gate                                                           | `.pre-commit-hooks/check_nautilus_conventions.sh:357-426` (the exposed Python submodule set is checked as a literal table and an unexpected registration fails the hook); the sibling hooks `check_error_conventions.sh`, `check_pyo3_conventions.sh`, `check_testing_conventions.sh`, `check_cargo_conventions.sh`, `check_unicode_typography.sh`, `check_non_latin_text.sh`, `check_hidden_chars.sh`; the script gate `scripts/check-markdown-tables.py` | exists: L14                                      |
| The adapter layer is eighteen crates with a documented integration per venue                                                                | `crates/adapters/` contains eighteen entries (`architect_ax`, `betfair`, `binance`, `blockchain`, `bybit`, `coinbase`, `databento`, `deribit`, `derive`, `dydx`, `hyperliquid`, `interactive_brokers`, `kraken`, `lighter`, `okx`, `polymarket`, `sandbox`, `tardis`); `docs/integrations/index.md:5-30` lists the integrations with an ID, a type, a status and a guide link                                                                              | exists: L11, L12                                 |
| The research crate exists as the point-in-time owner                                                                                        | `crates/research/Cargo.toml:2` (`name = "nautilus-research"`); `crates/research/src/lib.rs:15-38` (the crate doc naming `membership`, `dataset`, `panel`, `operators`, `feature` and `label`); `crates/research/src/panel.rs:150` (`Panel`) with `:176` (`check`)                                                                                                                                                                                          | exists: L1's first candidate home                |
| There is no evidence leaf type, no aggregation invariant, no bound-constraint record, no downgrade-ground record and no run provenance card | Searches of `crates/research/src/`, `crates/risk/src/` and `crates/model/src/events/order/` for such types return nothing; the closest neighbours are the capability type, `OrderDeniedReason` and the identity contracts named above                                                                                                                                                                                                                      | new contract: L1 to L4, L7, L13                  |

## 4. Mechanism specifications

Each subsection states the observed mechanism with its source citation, the requirement restated in
this project's own words, the interface or type involved, the data the mechanism needs, the refusal
behaviour, and the minimum acceptance. No code, no Rust and no Python appears: these are
specifications, and section 6 is the verification plan for them.

The provenance chain `design 1.2` requires - observed mechanism, our requirement, our interface, our
mathematical or logical specification, our implementation, our tests - is stated per item as the
first two blocks plus the interface block. The last two links do not exist yet, for any item.

### 4.1 L1: forced evidence gathering with recorded leaves

**Observed mechanism.** Before an analyst is invoked, a fixed tool set is run for it, so the
composition of the evidence is deterministic even though the prose is not. Each call becomes a
recorded leaf whose status is one of `ok`, `error`, `no_data` or `timeout`; a failure is a leaf and
never an exception; repeated requests are short-circuited; the arguments are hashed for identity
(`tradingagents/agents/utils/evidence_gather.py:16-18`, `:56`, `:163`, `:216-220`).

**Requirement, in this project's own words.** When a research run assembles inputs, the set of
inputs it intends to read is declared before the run starts, and each intended input ends as a
recorded leaf carrying a status and, when it is not `ok`, a reason. The record is machine-comparable,
so two runs of the same declaration can be diffed by which inputs each actually had.

**Interface or type involved.** The per-leaf answer reuses the capability shape at
`crates/core/src/capability.rs:58-129`: availability, a canonical code from a closed set the research
domain declares, a human detail, and the requirements that were not met. The leaf itself carries the
input's identity (the request's canonical argument digest, in the style of
`python/nautilus_trader/optimization/identity.py:84-92`), that answer, and the timestamps. The
declaration is a separate value: the ordered set of intended inputs, one entry per leaf. The
location of the declaration is `design 13.1`'s open question; the two candidates are
`crates/research` beside `Dataset` and `Panel` (`crates/research/src/lib.rs:15-38`) and
`crates/core` beside `Capability`.

**Data it needs.** The declaration (input identity, and what it intends to read), the per-leaf
outcome, the argument digest, the status from the domain's closed set, the reason when the status is
not `ok`, and the run identity the leaves belong to.

**Refusal behaviour.** A leaf that could not be read is a leaf whose capability answer is
unavailable with a code and a reason; it is never an empty value, never an exception that escapes the
gatherer, and never a silently omitted entry. A run whose declaration is not fully accounted for -
fewer leaves than declared inputs, or a duplicate identity - is itself unavailable with a code,
rather than partially evidenced.

**Minimum acceptance.** One leaf per declared input, in declaration order; every status drawn from
the domain's closed set; a test asserting that a failing input produces a leaf with its reason and no
escape; a test asserting that two runs of one declaration diff by leaf status; a test asserting that
a declaration with an unaccounted input is refused rather than partially recorded.

**Home and status.** `crates/research`, subject to `design 13.1`. Not implemented.

### 4.2 L2: uncertainty is not adverse evidence

**Observed mechanism.** The source states the rule and tests it rather than prompting for it: "An
uncertainty is not a bearish fact. `NA` / `unavailable` / `unmeasured` entries must never be counted
as evidence against a thesis. Only measured facts with an adverse sign are bearish"
(`docs/design_decision_context.md:579-581`, with the rule asserted by test per
`docs/design_decision_context.md:1820`). Structurally, the score kernel renormalises over the
components that are present and withholds the composite below a coverage floor instead of
substituting a neutral value (`tradingagents/strategies/score_engine.py:141-243`, the combine
function, with the coverage arithmetic inside it).

**Requirement, in this project's own words.** Wherever a research aggregate is computed, a missing
measurement is excluded from the denominator and reported as missing. It is never a zero, never a
neutral midpoint, and never a negative. A genuine zero remains a real zero, and the difference
between an absent component and a measured zero is visible in the result.

**Interface or type involved.** The aggregate is a value paired with its coverage and its withheld
set, in the shape this project already uses: `MetricValue::NotAvailable(UnavailableReason)` at
`crates/trading/src/analytics/metrics.rs:100-128` and `MetricReason` at `crates/analysis/src/metric.rs:379`
are the existing instances of the same rule. The aggregate's own new part is the coverage record: the
components present, the components withheld with their reason, and the resolved floor.

**Data it needs.** The declared component set, each component's presence, each component's weight (or
an explicitly declared equal weighting), and the coverage floor, which is either an absolute count or
a fraction resolved against the declared set.

**Refusal behaviour.** Below the floor the composite is withheld: the value is not available, with a
reason that names the coverage shortfall, and it is never returned as a substituted number. A
component that is present but non-finite is itself unavailable and does not enter the sum.

**Minimum acceptance.** An aggregate over a partial input set excludes the missing member from the
denominator and reports it; a test asserts that the composite is withheld rather than substituted;
a test distinguishes a measured zero from an absence, in both directions; a test asserts that the
reported coverage equals the fraction of declared weight present.

**Home and status.** The research layer's aggregates, following the convention already enforced for
statistics and execution analytics. Not implemented as a stated and tested research invariant.

### 4.3 L3: a computed number carries its binding budget

**Observed mechanism.** "The LLM argues the thesis; this computes the number. Size = min over
independent budgets (Kelly, risk-per-trade) scaled by volatility targeting, order-flow distribution
and agreement - all clamped to config caps and returned with an audit trail of which budget bound
it." (`tradingagents/strategies/contract.py:1-6`).

**Requirement, in this project's own words.** Any computed size, limit or target that results from
taking a minimum or a maximum over several constraints reports which constraint bound it, and the
reported value can be reproduced from the recorded inputs without re-running the producer.

**Interface or type involved.** The record belongs beside the existing denial vocabulary:
`OrderDeniedReason` at `crates/model/src/events/order/denied_reason.rs:71`, whose `capability()`
at `:478-491` already renders a canonical code, and which already names the field that refused an
order. The producer side is the sizer surface at `crates/risk/src/python/sizing.rs:43` (`PositionSizer`)
with its one built-in `FixedRiskSizer` at `:125`, and the portfolio's target pipeline. The record
itself is a small value: the binding constraint's identity, the value it produced, the other
candidates that lost, and the inputs each was computed from.

**Data it needs.** The candidate values, the constraint each came from, the arithmetic that combined
them, the winner's identity, and the input record the winner's value is reproducible from.

**Refusal behaviour.** A computed number whose binding constraint cannot be named is not emitted: it
is unavailable with a code, rather than emitted unexplained. A constraint set that is empty or
degenerate produces an unavailable answer, not a default size.

**Minimum acceptance.** A computed size reports its binding constraint; the value is reproducible
from the record alone; a test asserts that changing the binding constraint changes the reported
constraint and not only the number; a test asserts that an unnamed or empty constraint set is
refused.

**Home and status.** The risk and portfolio layer, reusing the denial vocabulary rather than
inventing a second result type (`design 12 T8`). Not implemented.

### 4.4 L4: a downgrade names its ground

**Observed mechanism.** A challenge pass "may downgrade only on one of the three closed grounds; it
may not create a new caution rationale ... it must never turn BUY -> HOLD because 'there is
uncertainty'", and the sole permitted mutation is one named field
(`docs/design_decision_context.md:784-790`), with the four rules "enforced by code, not by prompt
wording" and a table naming the enforcement site of each (`docs/design_decision_context.md:803-811`,
the sole mutation being `decision_guardrail.downgrade_toward_hold`, whose implementation is
`tradingagents/strategies/decision_guardrail.py:84-95`).

**Requirement, in this project's own words.** Any mechanism that weakens or refuses a result does so
on a closed, named set of grounds, and the name travels with the result. A weakening for an unnamed
reason is a defect of the same class as a denial without a reason code.

**Interface or type involved.** The closed set is declared by the domain that answers, exactly as
`crates/core/src/capability.rs:19-27` requires: the denial domain's set is `OrderDeniedReason`
(`crates/model/src/events/order/denied_reason.rs:71`), and this project already refuses a declared
policy part it cannot honour, naming the field
(`crates/trading/src/algorithm/policy.rs:60`, `:292-316`; `docs/concepts/execution/algorithms.md:37-41`).
The weakening path therefore carries the same shape: the original result, the weakened result, and
the ground's identity from the closed set.

**Data it needs.** The original result, the weakened result, the ground name, and the evidence for
the ground (the falsifier or contradiction the source's rule requires), with the identity of
whatever produced that evidence.

**Refusal behaviour.** A weakening that cannot name a ground from the set is refused as a defect: the
path returns an unavailable answer with a code rather than a weakened result. A ground outside the
set is refused, not coerced to the nearest member. Creating a new ground at the point of weakening
is itself the failure this item exists to prevent.

**Minimum acceptance.** Every weakening path names a ground from a closed set; a test asserts that no
unnamed ground can be produced; a test asserts the ground set is exhaustive over the weakening paths
that exist, so a new path cannot be added without extending the set deliberately.

**Home and status.** The risk engine's refusal vocabulary and the research layer's weakening paths.
Not implemented.

### 4.5 L5: a bounded decision packet as an information boundary (specification only)

**Observed mechanism.** A packet is "the bounded, deterministic decision context - research evidence
and decision-domain constraints, for adjudication. Not an order, not a recommendation and not a
gate", with a hard budget (`packet_max_chars` of `12_000` and `engine_row_max_chars` of `160` at
`tradingagents/strategies/decision_packet.py:80-86`) and truncation that is visible and counted
(`:365`, `:391`, `:530` all reading the declared row bound). The stated invariant is: "Research can
be large. Decision context cannot be large by accident."

**Requirement, in this project's own words.** Where this project hands a research result to another
component, the hand-off is a bounded, deterministic document with a declared budget, and any
truncation is recorded with what was dropped and how much.

**Interface or type involved.** A packet type, a declared budget, and a truncation record. The
document is a pure function of its inputs, in the same sense as the source's packet, which the source
says performs "no vendor call, no model call, no state mutation"
(`docs/design_decision_context.md:1818`).

**Data it needs.** The source document, the budget, the rows or sections included, and the
truncation record per bounded unit.

**Refusal behaviour.** A hand-off without a declared budget is not produced; a truncation is recorded
rather than silent; a packet whose required section is truncated below usefulness is refused rather
than emitted as a partial answer, following the source's rule that a bound met by cutting a cell in
half is not a bound (`tradingagents/strategies/decision_packet.py:530`).

**Minimum acceptance.** A bounded document with a declared budget and counted truncation, and a test
that a document exceeding the budget reports exactly what was dropped. This is a specification only.

**Home and status.** The research layer, if and when a research result is handed to a consumer.
**This item must not be built until that consumer exists** (`design 12 T7`, `design 13.2`); building
a packet with no reader is the abstraction this project avoids. Not implemented.

### 4.6 L6: a degenerate computation is refused, never substituted

**Observed mechanism.** The source's structured-output layer detects a monologue, a degenerate loop
and a self-halt, and emits an explicit "Decision: unavailable" rather than shipping a private draft
as the plan (`docs/design_multi_agent_debate.md`). The same discipline appears in its vendor layer,
where a failure becomes an explicit absence string with a reason
(`tradingagents/dataflows/interface.py:756,900-945`) and sentinels are never cached as data (the
`_SENTINEL_PREFIXES` tuple at `tradingagents/dataflows/vendor_cache.py:37-38`, with the cache key
carrying a version at `:63` so a semantic change invalidates stored entries).

**Requirement, in this project's own words.** A computation that cannot produce a trustworthy value
returns a not-available state carrying a reason; it never returns a degraded substitute, and a
not-available result is never recorded as if it were a value.

**Interface or type involved.** Three existing instances of the rule, which the item generalises:
`MetricValue::NotAvailable(UnavailableReason)` at `crates/trading/src/analytics/metrics.rs:100-128`,
`MetricReason` at `crates/analysis/src/metric.rs:379-...`, and `Capability` at
`crates/core/src/capability.rs:58-129`. The rule needs no new type; what it needs is the prohibition
stated once and its boundary cases tested where a substitution is tempting.

**Data it needs.** The reason, from the domain's closed set, and the inputs that made the value
undecidable, so the reason is checkable rather than a narrative.

**Refusal behaviour.** Substitution is the failure: a zero, a midpoint, a last-known value or a
default is never returned in place of an undefined result. A sentinel or an absence is never
persisted as data, and a read path that finds one reports it rather than decoding it as a value.

**Minimum acceptance.** A not-available result carries a reason; a test asserts that no substitute
value is produced for each boundary case (empty series, non-finite input, undefined operation,
absent currency); a test asserts that a not-available result is not persisted through the path that
stores values, and that a stored sentinel is reported rather than decoded.

**Home and status.** Everywhere this project already behaves this way; the item is the stated
prohibition and its boundary tests. Not implemented as a stated rule.

### 4.7 L7: a self-excluding content hash, recomputed and flagged

**Observed mechanism.** A decision artifact carries `artifact_sha256` and `decision_hash` computed
over its own sorted-keys body excluding the hash fields, and an integrity function recomputes and
flags `artifact_hash_mismatch` (`tradingagents/execution_contract.py:455-462`, reached from the
problem enumeration that treats a present hash field as a reason to re-seal). The check is
deliberately non-fatal: "Read-only and non-fatal by design: the report tree must still render when
an artifact is malformed" (`tradingagents/execution_contract.py:352-356`).

**Requirement, in this project's own words.** A persisted research artifact carries a digest over its
own content excluding the digest, and a reader can recompute it and be told, explicitly, whether it
matches. The reading rule is the part worth adopting: a mismatch is reported, never repaired, and a
malformed artifact is not rendered as if it were valid.

**Interface or type involved.** The digest style already exists in the identity contracts:
`digest_of` at `python/nautilus_trader/optimization/space.py:75` and the composition helpers at
`python/nautilus_trader/optimization/identity.py:84-92`, with `digest()` on each identity at `:191`,
`:264`, `:336`, `:433`. The companion reading rule sits beside the parity protocol at
`docs/developer_guide/parity_protocol.md:1-16`. The artifact side needs a canonical serialisation
with the digest fields excluded by construction, and a verdict type that distinguishes match,
mismatch and malformed.

**Data it needs.** The artifact body in a canonical form (key order fixed, excluded fields declared
by name), the digest algorithm and its version, and the recomputation verdict.

**Refusal behaviour.** A mismatch is reported and nothing is rewritten; a malformed artifact is
reported as malformed and is not repaired, and its digest is not recomputed and written back. A
reader that cannot tell a match from an unverifiable artifact reports the latter.

**Minimum acceptance.** A digest over content excluding the digest; a reader that recomputes and
reports a mismatch without repairing; a test that flips one non-digest field and asserts the verdict
changes to mismatch while the stored bytes are untouched; a test that removes a digest field and
asserts the artifact is reported as unverifiable rather than valid.

**Home and status.** The research layer, beside the identity contracts and the parity protocol
(`design 8 L7`). Not implemented. `design 13.2` asks whether this repository ever wants a decision
artifact at all; if it does not, this item reduces to the digest rule the identity contracts already
apply, and only the reading rule is new.

### 4.8 L8: a bar-level fill-honesty checklist

**Observed mechanism.** Six guards, each pinned by a test that states the exact price and PnL it
protects (`tests/test_backtest_fill_semantics.py`, 268 lines, with the guard tests named at `:57`,
`:71`, `:84`, `:102`, `:125`, `:138`, `:157`, `:169`, `:194`, `:205`, `:219`, `:227`, `:236`, `:243`,
`:252`, `:261`; and `tests/test_next_bar_fill.py:34-64`), each traceable to a defect that had
flattered the results (`CHANGELOG.md:45`, `:56-60`, which records that each defect was reproduced
before the fix and that the six fail before it while two controls pass throughout).

**Requirement, in this project's own words.** This project states its own bar-level fill rules as a
checklist, and every line of the checklist is pinned by a test that asserts the price, not merely the
absence of an error. A rule that is true today and untested is a rule the next change breaks.

**The checklist, with the state of this repository re-derived from code.** The design document's
statement is retained and each row re-checked; the evidence column is what establishes the state.

| Rule                                                                                               | State here                                                          | Evidence                                                                                                                                                                                    |
| -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A gap through a stop fills at the gap, not the trigger                                             | Present                                                             | `crates/execution/src/matching_engine/mod.rs:2022-2032` (the gap-open branch sets `fill_at_market`); `:4560-4575` (the trigger-price pull-back applies only when `fill_at_market` is false) |
| A triggered stop-limit whose limit is unreachable rests rather than filling at an impossible price | Present                                                             | `crates/execution/src/matching_engine/mod.rs:6568-6622` (fill only on the maker-inside test or `is_limit_matched`, otherwise resync as resting)                                             |
| A non-finite price cannot exist at all                                                             | Present, structurally                                               | `crates/model/src/types/price.rs:85-90` (`PriceRaw` is an integer), `:195` (`new_checked`); `crates/core/src/correctness.rs:621-639` (NaN and infinity rejected)                            |
| Slippage applies at one point to every fill                                                        | Present                                                             | `crates/execution/src/matching_engine/mod.rs:5137-5160` (one composition point; an independent slippage model is the single source)                                                         |
| A bar the instrument cannot trade on blocks the fill, not only the marking                         | Present for the mapped status actions; silent for the unmapped ones | `crates/execution/src/matching_engine/mod.rs:4090` and `:3039`; the unmapped set is section 5.1                                                                                             |
| A signal from bar N's close cannot fill on bar N                                                   | **Absent by default**                                               | `crates/backtest/src/engine.rs:941-956` (settle before dispatch); documented at `docs/concepts/backtesting/bar-execution.md:99-110`                                                         |

**Interface or type involved.** The matching engine and the backtest venue exchange. The existing
test homes are the in-module suite in `crates/execution/src/matching_engine/mod.rs` (55 `fn test`
items) and the integration files `crates/execution/tests/integration/matching_engine.rs` and
`crates/backtest/tests/integration/exchange.rs`. The checklist itself is
documentation, and its stated home is
[`bar-execution.md`](../concepts/backtesting/bar-execution.md) and
[`fill-prices-and-matching.md`](../concepts/backtesting/fill-prices-and-matching.md) per `design 8 L8`.

**Data it needs.** The bar's OHLC sequence, the resting and new order set, the instrument's price
precision, the configured fill model (and, when set, the independent slippage model), and the bar's
tradability state.

**Refusal behaviour.** The guard itself is the refusal: a fill at a price the bar never offered is
not committed. For the sixth rule the behaviour is not refused but deferred: the documented
mitigation is a latency model, which delays the order's arrival so its settlement sees a later book
state (`docs/concepts/backtesting/bar-execution.md:104-110`).

**Minimum acceptance.** Each of the six rules pinned by a test that asserts the exact price, with the
sixth pinned either as a native next-bar mode or as the regression test that makes the hazard
impossible to reintroduce silently (`design 12 T5`).

**A measurement this pass took.** A search of `crates/execution/tests/` and `crates/backtest/tests/`
for a test that pins the gap-through-stop fill price found none: the matches for "gap" in the
matching-engine integration file are two doc comments at `:8252` and `:8316` about a gap scenario
with no book liquidity, and the backtest matches are about batch gaps and an FX rollover at
`crates/backtest/tests/integration/exchange.rs:4659`. Fill prices are asserted in general, for
example at `crates/backtest/tests/integration/exchange.rs:1029` (`assert_eq!(fill.last_px,
expected_fill_price)`), but not for this rule. The search is the evidence, not a proof of absence,
and writing the first pinned test is L8's first act (`design 13.6`).

**Home and status.** `crates/execution` and `crates/backtest`, with the checklist recorded in the
two concept documents named above. Not implemented.

### 4.9 L9: a tradability gate must guard every leg, and an unmapped status is not silent

**Observed mechanism.** The source's tradability gate is applied to the exit checks as well as to
the mark-to-market update, after a locked limit-down bar produced a booked stop
(`scripts/backtest_strategy.py:225-236`, where the loop now skips a blocked bar for the stop and
target checks and the code says the trade rides through it), and its tradability module models a
locked market explicitly, deriving suspension from the absence of a tradable close
(`tradingagents/strategies/market_tradability.py:43-70,76`).

**Requirement, part one - every leg.** A bar on which the instrument cannot trade blocks order
matching entirely, for resting orders and for new ones, and for every leg of a strategy's logic.

**Requirement, part two - no silent drop.** A trading-state input this project does not act on is
recorded as unhandled rather than discarded.

**Interface or type involved.** `MarketStatusAction` at `crates/model/src/enums.rs:978-1011` (sixteen
variants), `MarketStatus` at `:929-...`, `InstrumentStatus` at `crates/model/src/data/status.rs:40-57`
(with the venue-provided `is_trading` at `:54` and `is_quoting` at `:56`), the mapping at
`crates/execution/src/matching_engine/mod.rs:2569-2592`, and the gate at `:3039` and `:4090`. The
transfer path from replayed status data into the engine is `crates/backtest/src/exchange.rs:1139`.

**Data it needs.** The status action, the current market status, the instrument, and (if the item
chooses to consult them) the venue-provided `is_trading` and `is_quoting` optionals, which are
currently unread anywhere in `crates/execution/src/`.

**Refusal behaviour.** Matching is refused when the market status is not `Open`, from both the
new-order path and the resting-order path; and an action that the mapping does not act on is recorded
with its action, instrument and timestamp as unhandled, so the drop is visible in a log or a
counter rather than inferred from silence.

**Minimum acceptance.** Every `MarketStatusAction` is either mapped or explicitly recorded as
unhandled, and a test asserts that the mapping is total or that the unhandled set is declared and
enumerated. The defect recorded in section 5.1 is the reason this item is first in the work order
(`design 11`).

**Home and status.** `crates/execution` for the mapping and the gate, `crates/model` if a variant
needs a state, and [`instrument_status.md`](../concepts/data/instrument_status.md), which currently
does not state which actions are honoured. Not implemented.

**Scope note.** A full exchange price-limit lock remains an explicit non-goal and is refused by
section 7 (`design 12 T6`). This item does not request it. It requests that a status the venue
*did* send, meaning "not tradable", stops being ignored, and that the unhandled set is visible.

### 4.10 L10: deterministic verification precedes any model judgement (specification only)

**Observed mechanism.** The source's debate protocol runs a deterministic verification layer before
any judge: a hard breach - a violated claim or a malformed schema - triggers regeneration and then
aborts to the baseline rather than reaching the jury, and the blind judge never runs unless the
deterministic layer fully passes (`tradingagents/strategies/debate_score.py:39-71`, `:104-145`, and
the standing rule that the judge runs only after the deterministic verdict).

**Requirement, in this project's own words.** Where this project ever consults a non-deterministic
component - a model, a heuristic, or a third-party score - the deterministic checks run first, and a
failed check terminates the path instead of being passed to the non-deterministic component for a
verdict.

**Interface or type involved.** None today. This repository consults no model, so there is no call
site to change (`design 8 L10`). The rule is stated now so that a future component inherits it; the
deterministic check's verdict is the only interface the rule needs, and a verdict is either a pass or
a typed failure.

**Data it needs.** The input being checked, the deterministic verdict, and the typed reason for a
failure.

**Refusal behaviour.** A failed deterministic check terminates the path: the non-deterministic
component is not invoked, or its result is discarded unread. A malformed input is never adjudicated.

**Minimum acceptance.** The rule is recorded, and if such a component is ever added, a test asserts
that a failing deterministic check prevents the non-deterministic call rather than preceding it in
prose. This is a specification only, and there is nothing to build today.

**Home and status.** A stated rule for the research layer. **Not to be built until such a component
exists** (`design 12 T7`). Not implemented, and correctly so.

### 4.11 L11: a declared provider capability surface, with typed refusals remembered

**Observed mechanism.** Three mechanisms together: a machine-queryable registry of what each vendor
can serve and what credential it needs (`tradingagents/dataflows/registry.py:18-33` for the
provider-to-config-key table, `:41-51` for the coverage and credential surface); an error taxonomy
where each class corresponds to a router reaction
rather than to a human-readable cause (`tradingagents/dataflows/errors.py:1-19`, whose docstring
states that the number of types is the number of distinct router reactions and that empty and stale
data deliberately share one class); and a negative capability cache keyed on the endpoint, never on
the symbol, which eliminated measured duplicate refusals - 575 FMP 429s, 60 Massive 403s and 26
Finnhub 403s in one batch (`tradingagents/dataflows/vendor_breaker.py:175-213`, with the measured
counts at `docs/developer/03-dataflow-vendors.md:77-78`).

**Requirement, in this project's own words.** Each adapter declares, in machine-readable form, what it
can serve for which instrument classes, what it requires to be configured, and which of its requests
are known to be refused; and a refusal is remembered against the request shape, so it is not repeated
within a run.

**Interface or type involved.** A per-adapter declaration built from the existing capability type
(`crates/core/src/capability.rs:58-129`), which already carries availability, a canonical code from a
domain-declared closed set, a detail and the requirements not met, and which already has three
independent producers (`crates/analysis/src/metric.rs:707-712`,
`crates/model/src/events/order/denied_reason.rs:478-491`,
`crates/persistence/src/common/coverage.rs:615-642`). The declaration belongs beside each adapter crate
under `crates/adapters/` and is documented in the venue's guide under `docs/integrations/index.md:5-30`.
The negative cache is a per-run memory keyed on the request shape: the endpoint or request kind, the
instrument class, and the credential the request needs - never the individual symbol.

**Data it needs.** The declaration (instrument classes servable, credential requirements, known
refusals), the request shape used as the cache key, the refusal's typed class, and the run or process
scope the memory lives in (`design 13.4`).

**Refusal behaviour.** An unavailable answer names a canonical code from the adapter's closed set
plus the requirements that were not met; a remembered refusal is not retried within the run; and a
new refusal is recorded with its class so the next identical request short-circuits. A refusal is
never converted into an empty result.

**Minimum acceptance.** A per-adapter declaration built from the capability shape; a negative cache
keyed on the request shape; a test asserting the declaration's codes are canonical and that the
declared set is closed; a test asserting a remembered refusal is not repeated and that a different
symbol hitting the same endpoint is served by the memory.

**Home and status.** `crates/adapters` plus the integration guides. The data layer keeps one
authority per instrument: this item adds a declaration and a memory about adapters, and introduces no
second price and no vendor SDK dependency (`design 7.5`). Not implemented. `design 13.3` asks how fine
the declaration's granularity should be, and `design 13.4` asks whether the memory is process-local or
persisted.

### 4.12 L12: a per-provider unused-surface audit

**Observed mechanism.** A repeatable read-only study per vendor with a uniform method: enumerate the
vendor's full public surface by introspecting the installed package, extract the used set by
searching every call site in the tree, then probe the unused set and record each failure's exact
message, on the rule that a 404 is not evidence of absence and a 403 is
(`docs/design_eodhd_unused_surface.md`, `docs/design_moomoo_unused_api_surface.md`,
`docs/design_finnhub_yfinance_unused_surface.md`).

**Requirement, in this project's own words.** For each adapter, a documented audit of the vendor
surface this project does and does not use, produced by introspection of the client and a tree-wide
call-site search, with the unused set probed and each result recorded, and with wrong claims about a
vendor corrected in the same document.

**Interface or type involved.** `docs/integrations/<venue>.md`, one audit section or a companion note
per adapter, listed from `docs/integrations/index.md:5-30`. The method needs no new code: it is a
documentation obligation with a reproducible recipe, and the recipe is the interface.

**Data it needs.** The client library's public surface, the tree-wide call sites (a search over the
adapter crate and the workspace), the probe results with each failure's exact message, and the
corrections to earlier claims.

**Refusal behaviour.** A 403 or an equivalent entitlement failure is recorded as evidence that the
surface is unavailable; a 404 alone is recorded as inconclusive and never as absence. A claim the
probe contradicts is corrected in the document rather than left standing beside the new evidence.

**Minimum acceptance.** A documented audit per adapter with the used surface, the unused surface, the
probe result and the corrections; a test is not the verification here, since the deliverable is a
document, and the acceptance is that a reader can tell whether the vendor does not offer a data type
or this project never implemented it.

**Home and status.** `docs/integrations/<venue>.md`. Depends on L11's declaration for its used-surface
baseline (`design 10`). Not implemented.

### 4.13 L13: a run provenance card (specification only)

**Observed mechanism.** A per-run record emits `snapshot_id`, `data_snapshot_hash` (over the ticker,
trade date and price caliber identity), `engine_output_hash` and `model_parameters_hash` over the
sampling-relevant configuration (`tradingagents/agents/utils/prompt_metrics.py:258-276`, with the
model-parameter hash at `:317`), feeding a run card (`:330`); a prompt edit is made attributable by
writing a per-run condition to the prediction ledger, and a run with no recorded condition is
`unavailable`, never pooled.

**Requirement, in this project's own words.** A result file carries the identity of everything that
produced it, and a result whose producer identity is unknown is excluded from any aggregate rather
than pooled with the rest.

**Interface or type involved.** The identity contracts at
`python/nautilus_trader/optimization/identity.py:146` (`UniverseIdentity`), `:204`
(`DatasetIdentity`), `:277` (`ComputationIdentity`), `:349` (`StudyIdentity`) and `:490`
(`TrialIdentity`), each with a `digest()`, plus `digest_of` at
`python/nautilus_trader/optimization/space.py:75`. The transferable part is not the source's
model-shaped fields - this project has no model - but the **exclusion rule**: an unidentified run is
not evidence.

**Data it needs.** The dataset, universe, computation and study digests, the seed, the parameter
digest, the execution status, and the recorded condition under which the run was made.

**Refusal behaviour.** A result whose producer identity is unknown is excluded from an aggregate and
reported as excluded, rather than pooled. A run with no recorded condition is unavailable.

**Minimum acceptance.** A result carries its producer identity, and a test asserts that an
unidentified result is excluded from an aggregate rather than averaged into it. This is the part to
adopt; the card's presentation is not. This is a specification only.

**Home and status.** The research layer, beside the identity contracts. **This item must not be built
until an aggregate exists that would pool its results** (`design 12 T7`, `design 13.2`). Not
implemented.

### 4.14 L14: a discipline gate that fails the build on a rule breach

**Observed mechanism.** The source describes fifteen repo-wide tests as contracts rather than
feature tests, and states that a change which ignores one of them fails the suite - wiring,
dead-state dedupe, doc binding, prompt signature, prompt trigger, test quality, execution contract,
config isolation, gate toggles, API reference environment table, tool binding, vendor signature,
window integrity, report hygiene and engine ownership, including a ban on un-failable test shapes
(`docs/developer/10-tests-layout.md:24-32`, which enumerates `test_calc_agent_wiring`,
`test_prompt_signature_contract`, `test_test_quality_gate`, `test_gate_env_toggles` and the rest).
Every new consumer lands behind a default-false gate, and the gate-off byte-identity rule is checked
by a dedicated test rather than by the toggle test itself
(`tests/test_decision_packet.py:70`, `tests/test_engine_ownership_map.py:110`).

**Requirement, in this project's own words.** A rule a contributor must follow is checked by a
mechanism that fails the build when it is broken - a hook or a test - rather than by prose in a
guide. A rule that can be checked mechanically becomes a gate; a rule that cannot is stated and is
not pretended to be enforced (`design 7.4`).

**Interface or type involved.** The existing hook set `.pre-commit-hooks/check_*.sh` and the script
gates under `scripts/check-*.py`, with the public-module parity gate at
`.pre-commit-hooks/check_nautilus_conventions.sh:357-426` as the model: it checks the exposed Python
submodule set against a literal table and fails on an unexpected registration. The rules this item
gates are L8's checklist, L9's mapping completeness and L11's declaration.

**Data it needs.** The rule set, the gate that checks each rule, and the failure message that names
the rule and the offending site.

**Refusal behaviour.** A breach fails the build. A gate that cannot fail is not a gate: a check that
passes on a deliberately broken tree is itself a defect, which is why the source bans un-failable
test shapes.

**Minimum acceptance.** Every rule above that can be checked mechanically has a gate that fails the
build; each such gate is itself exercised against a broken input in a test, so a gate that has
decayed into a no-op is detected. This item lands last, because it gates the others
(`design 10`, `design 11`).

**Home and status.** The hook set and the test suite. Not implemented.

## 5. Defects

### 5.1 In this repository

One defect was confirmed in the areas the probe touched. **It is recorded, not repaired** (section
1.1): the code that owns it is `crates/execution`, and the repair is not this workstream's.

**The defect: a status action meaning "not tradable" is silently discarded, and the simulated market
keeps matching.**

**The mapping.** `process_status` at `crates/execution/src/matching_engine/mod.rs:2569` matches the
action in four arms:

| Arm       | Variants                                                                           | Transition           |
| --------- | ---------------------------------------------------------------------------------- | -------------------- |
| 1         | `Trading`, `PreOpen`, when the current status is `Closed`, `Paused` or `Suspended` | to `Open`            |
| 2         | `Pause`, when the current status is `Open`                                         | to `Paused`          |
| 3         | `Suspend`, when the current status is `Open`                                       | to `Suspended`       |
| 4         | `Halt`, `Close`, when the current status is `Open`                                 | to `Closed`          |
| catch-all | everything else                                                                    | no change, no record |

The arms are at `:2573-2590` and the catch-all `_ => {}` at `:2591-2592`. Six variants are acted on:
`Trading` (7), `PreOpen` (1), `Pause` (9), `Suspend` (10), `Halt` (8) and `Close` (12). Ten reach the
catch-all: `None` (0), `PreCross` (2), `Quoting` (3), `Cross` (4), `Rotation` (5),
`NewPriceIndication` (6), `PreClose` (11), `PostClose` (13), `ShortSellRestrictionChange` (14) and
`NotAvailableForTrading` (15).

Two of those ten are tradability states, not informational ones, and the enum says so in its own
documentation at `crates/model/src/enums.rs:978-1011`:

- `Quoting = 3`, at `:986`: "The instrument is quoting but not trading."
- `NotAvailableForTrading = 15`, at `:1010`: "The instrument is not available for trading, either
  trading has closed or been halted."

**The gate.** The market status is consulted in two places, and it is the *only* input:

- `crates/execution/src/matching_engine/mod.rs:3039`: `if self.market_status != MarketStatus::Open`
  on the validate path, refusing a new order with "Market {} is {}, cannot accept order {}".
- `crates/execution/src/matching_engine/mod.rs:4090`: `if self.market_status == MarketStatus::Open`
  around the match loop, so resting orders are matched only under the same condition.

**The consequence.** After a venue sends `Trading`, `market_status` is `Open` (arm 1). A later
`Quoting` or `NotAvailableForTrading` falls to the catch-all, so `market_status` remains `Open`, and
both gates continue to permit matching. A bar replay that loads instrument status data
(`crates/backtest/src/exchange.rs:1139` feeds replayed statuses to `process_status`) can therefore
simulate fills on a market the venue had declared unavailable, and nothing logs that the status was
dropped.

**The unread optionals.** `InstrumentStatus` carries `is_trading` at `crates/model/src/data/status.rs:54`
and `is_quoting` at `:56`, documented as venue-provided and optional. A search of
`crates/execution/src/` for either name returns nothing, so neither is consulted by the gate. The
adapters that produce them include Binance (`crates/adapters/binance/src/common/status.rs:89-97`),
Bybit (`crates/adapters/bybit/src/common/status.rs:99-107`) and Betfair
(`crates/adapters/betfair/src/stream/parse.rs:229-237`), and four adapters map a vendor state onto
`NotAvailableForTrading` (`crates/adapters/binance/src/common/status.rs:35`,
`crates/adapters/bybit/src/common/status.rs:34`, `crates/adapters/kraken/src/http/futures/client.rs:1336`,
`crates/adapters/lighter/src/data/mod.rs:910`). The statuses are produced in production and dropped
in simulation.

**The documentation.** `docs/concepts/data/instrument_status.md` documents the type and its fields,
including both optionals (`:9-25`), and says strategies can handle status updates through
`on_instrument_status(...)`, but it does not state which actions the matching engine honours and
which it drops. A reader of the type cannot tell that two variants are inert.

**What was checked, and the result.**

| Area checked                               | Method                                                                                          | Result                                                                                                                                                                                           |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The status mapping                         | Read `process_status` in full and enumerated the arms and variants against `MarketStatusAction` | Ten variants reach the catch-all; two of them mean "not tradable"                                                                                                                                |
| The gate                                   | Read both `market_status` comparisons and confirmed no other input participates                 | The mapping's output is the only input to the gate; `is_trading`/`is_quoting` are unread                                                                                                         |
| Test coverage                              | Enumerated every `process_status` call in the test tree                                         | Cases cover `Pause`, `Suspend`, `Close`, `Halt` and `Trading`; no test names an unmapped variant, at `crates/execution/tests/integration/matching_engine.rs:772-776`, `:834-845`, `:810`, `:897` |
| Adapter production of the dropped variants | Searched `crates/adapters/` for `NotAvailableForTrading` and `Quoting`                          | Four adapters produce `NotAvailableForTrading`; the variants are live, not theoretical                                                                                                           |
| The other L8 rules                         | Read the gap, stop-limit, price, slippage and gate sites                                        | No second defect found; the same-bar hazard is a documented weakness, not a defect (`design 12 T5`)                                                                                              |

**A tally to confirm.** Re-derived from the code, the match in `process_status` has four arms
covering six variants (`Trading`, `PreOpen`, `Pause`, `Suspend`, `Halt`, `Close`) and ten reach the
catch-all. Revision 1 of the design document said five and revision 2 records six, corrected from
this re-derivation. The count matters because L9's acceptance test is written against it: a test that
asserted a total mapping over the wrong set would pass for the wrong reason.

**Acceptance that would close it.** Every `MarketStatusAction` is either mapped or explicitly
recorded as unhandled; a test asserts that the mapping is total, or that the unhandled set is declared
and enumerated, and a test asserts that a later `Quoting` or `NotAvailableForTrading` after `Trading`
either changes the gate's verdict or produces a recorded unhandled verdict. Whether the fix is a
model change (a state for "quoting, not trading") or a recorded drop is an open decision (section 8).

### 5.2 In the source, recorded as design warnings

These are recorded because each one is a mistake that the items in section 4 could repeat. They are
not reported upstream; this workstream does not interact with that project's issue tracker. The
citations were re-derived from the checkout at the revisions named in section 2.

| #   | Observation                                                                                                                                                                                                                                                                                                                                                                                              | Evidence                                                                                                                                                                                                                         | Why it matters here                                                                                                                                                                                             |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Six bar-fill defects, all in the strategy's favour.** A stop gapped through filled at its trigger; the exit scan read the entry bar's own past range; a stop-limit filled when its limit was never reachable; a limit-locked bar executed an exit; the exit leg paid no slippage; a suspended bar's NaN leaked into the fill report                                                                    | `CHANGELOG.md:45`, `:56-60`; `tests/test_backtest_fill_semantics.py` (268 lines, the guard tests at `:57`, `:71`, `:84`, `:102`, `:125`, `:138`, `:157`, `:169`, `:194`, `:205`, `:219`, `:227`, `:236`, `:243`, `:252`, `:261`) | L8 exists because each defect had already flattered a published metric. The repair is the regression test that states the price, not the fix alone                                                              |
| 2   | **The same class was fixed twice.** The first pass fixed the entry and stop paths; a second pass found the exit leg exempt from both the tradability gate and slippage in the same function                                                                                                                                                                                                              | `CHANGELOG.md:45` (the second-pass entry, naming two more instances of the same class)                                                                                                                                           | A guard applied at one leg is not a guard. This is why L9's part one says *every leg*, and why L8's checklist is a checklist and not a single test                                                              |
| 3   | **Heartbeat lines in its own design documents are stale.** One document's status line says "DESIGN - not built. No code changes accompany this document" while later sections of the same document record the phases as built; another says "no code changed" while the structured debate modules exist                                                                                                  | `docs/design_decision_context.md:3`; `docs/design_multi_agent_debate.md:3`                                                                                                                                                       | The source's documentation is evidence of intent, never of behaviour. Every behavioural claim in this record was re-derived from code, and this document's own status lines are therefore written from the code |
| 4   | **The most consequential protections are opt-in and default off.** The deterministic position contract runs only behind `enable_position_contract`; the accuracy advisory behind `enable_accuracy_ceiling`; the debate, risk governor, quant scorecard, computed context, decision packet, prediction ledger, strategy overlays, orderflow and events blocks each sit behind their own default-false key | `tradingagents/graph/trading_graph.py:1083` (`enable_position_contract`), `:544` and `:558` (`enable_accuracy_ceiling`), `:241`, `:594`, `:748`, `:758`, `:808`, `:830`, `:872`, `:1002`, `:1026`, `:1046`                       | A correctness mechanism that is off by default protects nobody by default. L3's binding-budget record and L4's named ground must not be gated behind a flag that a deployment forgets                           |
| 5   | **The artifact integrity check is deliberately non-fatal.** A hash mismatch is reported as a problem and the report still renders                                                                                                                                                                                                                                                                        | `tradingagents/execution_contract.py:352-356` (the docstring stating the rule), `:455-462` (the mismatch check)                                                                                                                  | The reading rule is worth copying - L7 adopts it - but the pairing is the warning: a non-fatal report is only safe if something consumes it. A verdict nobody reads is prose, not a gate (L14)                  |
| 6   | **A negative cache removed measured duplicate refusals only because it was keyed on the endpoint rather than the symbol.** 575 FMP 429s, 60 Massive 403s and 26 Finnhub 403s were eliminated by the change of key                                                                                                                                                                                        | `tradingagents/dataflows/vendor_breaker.py:175-213`; `docs/developer/03-dataflow-vendors.md:77-78`                                                                                                                               | L11's key is the request shape, and the failure mode of a per-symbol key is already measured in the source: it does not deduplicate anything, because a rate limit is per endpoint, not per instrument          |
| 7   | **The vendor error taxonomy has one class per router reaction, not one per human-readable cause**, and the docstring says so, so a new vendor needs no new handler                                                                                                                                                                                                                                       | `tradingagents/dataflows/errors.py:1-19`                                                                                                                                                                                         | L11 copies the principle: a refusal's code belongs to the reaction the caller takes, and a code that is prose defeats the checkable half of the answer (`crates/core/src/capability.rs:19-27`)                  |
| 8   | **A sentinel is never cached as data, and the cache's schema is versioned so a semantic change invalidates stored entries**                                                                                                                                                                                                                                                                              | `tradingagents/dataflows/vendor_cache.py:37-38` (the sentinel prefixes), `:63` (the cache version, whose stated purpose is that a caliber or unit change can never resurface under a new schema)                                 | L6's "an absence is never persisted as a value", and the same rule this project applies to catalog data                                                                                                         |
| 9   | **The point-in-time read masks later-dated rows absolutely**                                                                                                                                                                                                                                                                                                                                             | `tradingagents/dataflows/pit_registry.py:108-122` (`read_as_of`, documented as never showing a record dated after the requested as-of)                                                                                           | This repository's point-in-time discipline is already at least as strong (`crates/research/src/lib.rs:15-38`, `crates/research/src/panel.rs:176`); the source adds nothing here, which is why no item adopts it |
| 10  | **Non-determinism is recorded rather than removed, and the project says so**                                                                                                                                                                                                                                                                                                                             | `README.md:1273` (sampling is non-deterministic and providers do not guarantee byte-identical output)                                                                                                                            | The right choice for an LLM pipeline and the wrong choice for a backtest (section 7). No item asks this project to try to reproduce a model                                                                     |

## 6. Acceptance criteria and verification, by item

This table is the detailed verification plan. The minimum acceptance contract per item, which is what
makes the specification independently reviewable, is stated in `design 12.1`; the column below states
the specific test or measurement that would prove it, and the status is `Not implemented` for every
row.

| Item | Acceptance                                                                                                                   | Verification method                                                           | Status          | The test or measurement that would prove it                                                                                                                                                                            |
| ---- | ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| L1   | A declared input set; one leaf per input with a status and a reason; two runs diffable by inputs                             | Unit tests over the declaration and the leaves, plus a two-run diff           | Not implemented | A test that a failing input yields a leaf with its code and reason and no escape; a test that two runs of one declaration differ exactly by leaf status; a test that an unaccounted declared input refuses the run     |
| L2   | An aggregate over a partial input set excludes the missing member and reports it; the composite is withheld, not substituted | Unit tests over the coverage arithmetic, with a measured zero as a control    | Not implemented | A test with one component absent asserting the withheld verdict and the reported coverage; a control with the same component measured at zero asserting it enters the denominator                                      |
| L3   | A computed size reports its binding constraint; the value is reproducible from the record                                    | Unit test per constraint set, plus a reproduction check from the record alone | Not implemented | A test asserting the reported binding constraint changes when the winning constraint changes by one unit; a test reproducing the value from the record with the producer removed                                       |
| L4   | Every weakening path names a ground from a closed set; no unnamed ground exists                                              | Exhaustive test over the weakening paths and the declared grounds             | Not implemented | A test enumerating the declared grounds against the paths, failing when a path is uncovered; a test that an unlisted ground is refused rather than coerced                                                             |
| L5   | A bounded document with a declared budget and counted truncation                                                             | Specification only; no test until a consumer exists                           | Not implemented | A test that an over-budget document reports exactly the dropped units and their count, if and when a consumer appears                                                                                                  |
| L6   | A not-available result carries a reason; no substitute value is produced or persisted                                        | Boundary tests per undefined case, plus a persistence round trip              | Not implemented | A test per boundary case (empty, non-finite, undefined, unresolved currency) asserting the reason; a test that storing a not-available result does not write a value, and that a stored sentinel is reported           |
| L7   | A digest over content excluding the digest; a reader recomputes and reports a mismatch without repairing                     | Tamper test plus a malformed-artifact test                                    | Not implemented | A test flipping one non-digest field asserting the mismatch verdict and untouched bytes; a test removing the digest asserting "unverifiable" rather than "valid"                                                       |
| L8   | Each of the six rules pinned by a test asserting the exact price, including the same-bar rule                                | Six tests, one per rule, each naming the price it protects                    | Not implemented | A gap-through-stop test asserting the fill at the gap price; a stop-limit test asserting no fill on an unreachable limit; a NaN-price test; a two-leg slippage test; a gate test per leg; the same-bar regression test |
| L9   | Every `MarketStatusAction` is either mapped or explicitly recorded as unhandled                                              | A totality test over the variant set, plus a status transition test           | Not implemented | A test iterating every variant asserting a mapped transition or a recorded unhandled verdict; a test that `Quoting` and `NotAvailableForTrading` after `Trading` change the gate or are recorded                       |
| L10  | Deterministic verification precedes any non-deterministic judgement                                                          | Specification only; no test until such a component exists                     | Not implemented | A test asserting the non-deterministic component is not invoked after a failed deterministic check, if and when one is added                                                                                           |
| L11  | A per-adapter declaration built from the capability shape, plus a refusal memory keyed on the request shape                  | Declaration tests per adapter, plus cache-key tests                           | Not implemented | A test asserting each adapter's declared codes are canonical and its set closed; a test that a remembered refusal short-circuits a second identical request and a different symbol on the same endpoint                |
| L12  | A documented audit per adapter: used surface, unused surface, probe result, corrections                                      | Document review against the recipe; no runtime test                           | Not implemented | A reader can determine, for each vendor capability, whether the vendor lacks it or this project never implemented it; the deliverable is the document                                                                  |
| L13  | A result carries its producer identity; an unidentified result is excluded from aggregates                                   | Specification only; no test until an aggregate exists                         | Not implemented | A test asserting an unidentifiable result is reported as excluded rather than averaged in, if and when an aggregate appears                                                                                            |
| L14  | Every mechanically checkable rule above has a gate that fails the build, and a gate that cannot fail is not a gate           | One gate per rule, plus a broken-input test per gate                          | Not implemented | A test that runs each gate against a deliberately breached tree and asserts a non-zero exit; a test asserting the gate set covers the rule set                                                                         |

## 7. Explicit non-goals

Not to be implemented as part of this harvest, whatever else changes. This list is what makes the
harvest checkable: none of the items in section 4 is built on any of these, and nothing in this
workstream implemented any of them.

- **A vendor router with a silent fallback chain for market data.** An instrument's price has one
  authority; a chain that silently switches source mid-run makes a backtest unreproducible
  (`design 7.2`, `design 7.5`). L11 borrows the declaration and the refusal memory, never the chain.
- **Any second authoritative price for the same instrument, or any runtime dependency on a vendor
  SDK** introduced by L11 or L12 (`design 7.5`).
- **Any relaxation of the fixed-point money types to accommodate a float-valued vendor**
  (`design 7.2`). `Price`, `Quantity` and `Money` keep their integer representation
  (`crates/model/src/types/price.rs:85-90`); no item accepts a float where a price is required.
- **Any model-generated number in a fill, a fee, a size, a limit or a statistic** (`design 7.2`), and
  **any LLM call, prompt, prompt text, role name, model client, or JSON field name copied from the
  source** (`design 7.1`, `design 1.1`). This repository consults no model, and no item adds one.
- **Any blocking call to a model or a vendor inside the matching engine or the risk engine**
  (`design 7.2`).
- **Any new third-party dependency.** Not `langgraph`, not `langchain-core`, not `yfinance`, not
  `backtrader`, not the `moomoo` SDK, and not any LLM client library, all of which appear in the
  source's own dependency list (`pyproject.toml:4-42` in that project). This project's dependency
  policy is unchanged (`design 1.2`, `design T2`).
- **No source code, test, fixture, prompt text, or copied documentation enters this repository**, and
  no artifact is derived by mechanical transformation of any of them (`design 1.2`, `design T2`).
- **The exchange price-limit lock and any clamping to exchange limits.** This is an existing non-goal
  of this project, restated in the VeighNa record, and L9 does not reopen it (`design 12 T6`,
  [`vnpy_lessons_implementation.md`](vnpy_lessons_implementation.md), Explicit non-goals). L9 is
  narrower: a status the venue did send, meaning "not tradable", stops being ignored.
- **The source's per-role output-token ceilings as a risk mechanism** (`design 7.2`). They bound a
  prompt's length, not a position.
- **The source's statistical machinery** - deflated Sharpe, CPCV, the reality check, PBO. This
  project acquired the equivalent discipline from the vectorbt review, and the source adds nothing
  there (`design 9`).
- **Reproducibility of a model run.** Nothing the source does makes an LLM run reproducible, and it
  says so (`README.md:1273`); no item asks this project to try (`design 9`).
- **Any decision authority for the research layer.** Item L5, L7 and L13 create artifact shapes and
  none of them may gate, size or emit an order (`design 7.1`, `design T3`).

## 8. Outstanding measurements and open items

The design document's open questions are carried forward first, unchanged and unresolved, then the
measurements and questions this pass added. Nothing here is answered by invention.

1. **Where does the evidence contract live** - in `crates/research` beside the panel and dataset
   contracts (`crates/research/src/lib.rs:15-38`), or as a smaller type in `crates/core` beside
   `Capability` (`crates/core/src/capability.rs:58-129`)? L1 needs a home before it can be specified
   further (`design 13.1`).
2. **Is a decision artifact ever wanted here?** If the answer is no, L5, L7 and L13 reduce to the
   identity-contract rules this project already has, and only L7's reading rule and L13's exclusion
   rule survive (`design 13.2`).
3. **How far should an adapter declaration go** - per data type, per instrument class, or per venue?
   The machine-readable form depends on the answer, and so does L12's audit template
   (`design 13.3`).
4. **Should the negative cache of L11 be process-local or persisted?** A persisted refusal is a
   configuration-shaped artifact and needs an invalidation rule (`design 13.4`).
5. **Does the same-bar fill hazard deserve a native next-bar fill mode**, or is a latency model plus
   a regression test the right permanent answer? This is a public API question and belongs to the
   owner (`design 13.5`, `design 12 T5`).
6. **Which of L8's rules should be pinned first?** They are equal in principle; the cheapest is
   probably the gap-through-stop, whose behaviour is a single condition (`design 13.6`). This pass
   adds the measurement that no such test currently exists: the search of
   `crates/execution/tests/` and `crates/backtest/tests/` found no test asserting that fill price
   (section 4.8), so the gap-through-stop test is writing-from-scratch rather than a tightening.
7. **The mapped-variant count needs confirming before L9's test is written.** The design document
   states five of sixteen (`design 8 L9`); the match at
   `crates/execution/src/matching_engine/mod.rs:2573-2592` has four arms covering six variants
   (`Trading`, `PreOpen`, `Pause`, `Suspend`, `Halt`, `Close`), with ten reaching the catch-all. A
   totality test written against the wrong number would pass for the wrong reason.
8. **Should `MarketStatusAction::Quoting` map to a state, or be recorded as unhandled?** The enum
   documents it as "quoting but not trading" (`crates/model/src/enums.rs:986`), which is a distinct
   condition from both `Open` and `Paused`; a state for it is a model change, an unhandled verdict is
   not, and the choice is the owner's.
9. **Two design-document citations do not resolve as written** (section 1.2): the location of
   `is_trading` and `is_quoting` (verified at `crates/model/src/data/status.rs:54,56`, not in
   `crates/model/src/enums.rs`), and the mapped-variant tally in item 7. They are recorded rather
   than edited, because the design document is not this workstream's file.
10. **What the adapters actually emit in practice is unmeasured.** Four adapter crates map a vendor
    state onto `NotAvailableForTrading` (`crates/adapters/binance/src/common/status.rs:35`,
    `crates/adapters/bybit/src/common/status.rs:34`,
    `crates/adapters/kraken/src/http/futures/client.rs:1336`,
    `crates/adapters/lighter/src/data/mod.rs:910`), and three set `is_trading`
    (`crates/adapters/binance/src/common/status.rs:89`, `crates/adapters/bybit/src/common/status.rs:99`,
    `crates/adapters/betfair/src/stream/parse.rs:229`). No measurement exists of how often those
    actions occur in recorded data, so the frequency of the L9 defect in a real replay is unknown.
    The measurement would be a count of the dropped variants over a recorded status dataset, and it
    has not been taken.
11. **The cost of a per-adapter declaration and a refusal memory on the data path is unmeasured.**
    L11 adds a lookup to a request path that is not currently instrumented, and no benchmark exists
    for it. This is a measurement to take before the declaration is designed, not after.
12. **The gate set of L14 has no membership decision yet.** Which of L8's six rules, L9's mapping and
    L11's declaration become hooks rather than tests is undecided; the repository's precedent is a
    shell hook for a tree-wide text rule
    (`.pre-commit-hooks/check_nautilus_conventions.sh:357-426`) and a test for a behavioural rule.
13. **Nothing in section 4 has been built, measured or benchmarked**, and no code path was exercised
    by this pass. Every item's verification column in section 6 is a plan, not a result. The single
    exception in kind is the defect in section 5.1, which was confirmed by reading the mapping, the
    gate, the enum, the test cases and the adapter producers, and was not reproduced by running a
    replay.
