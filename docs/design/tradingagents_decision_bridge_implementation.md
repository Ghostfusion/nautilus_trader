# The research-to-execution bridge: implementation record

Companion to [`tradingagents_decision_bridge_design.md`](tradingagents_decision_bridge_design.md),
and to [`tradingagents_lessons_implementation.md`](tradingagents_lessons_implementation.md), which
records the earlier review of the same research project. Section references of the form `design 4 I2`
point at the design document and its invariants, `design 5.2` at its refusal and diagnostic
vocabulary, `design 7` at its owner decisions, `design 10` at its implementation gates, and `B1` to
`B15` are this document's items.

Nothing in this document is a delivery record. The design document is a plan; this is the matching
specification, and section 1.3 records each item against what now exists.

## 1. Status and scope

### 1.1 What exists, and what this plan adds

**The bridge now exists as a hand-written, pure-Python package at
`python/nautilus_trader/decision_bridge/`.** Revision 5's change was additive: the package is new; it
added no crate, no PyO3 binding and no generated stub; and no existing Python file was changed by
that work. The Rust measurement of B11 and B12 is a new module inside the existing `nautilus-research`
crate, `crates/research/src/measurement.rs`, reached from the crate's own `lib.rs` by a module
declaration and re-exports. Revision 6 is additive in a different sense: it adds two PyO3 surfaces
and one live-configuration field to existing crates, because the two Python surfaces the bridge
needed were exactly the ones revision 5 recorded as absent, and it extends the bundled equity
calendar. Each is recorded where its item is specified, and section `7` carries the resolutions.

Most of the chain the request describes is already built. That is the most important fact in this
plan, because it shrinks the work to a join:

| Chain stage            | Where it lives today                                               | This plan's involvement                                     |
| ---------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------- |
| Decision context       | The research project's decision contract, schema 1.2.0             | Read it; project it (B1, B2)                                |
| Gate resolution        | Nothing; the verdict is advisory and independent                   | Aggregate deterministically (B14)                           |
| Strategy eligibility   | Nothing; `TradingSignal` validity and expiry exist                 | Build the admission gate and the policies (B2, B5, B7, B13) |
| Position target        | `TargetConstruction`, `crates/trading/src/target.rs`               | Configure; apply the ceiling only (B5)                      |
| Portfolio construction | `TargetReconciler`, same file; per instrument, not cross-sectional | Nothing (design 3.4)                                        |
| Risk                   | `RiskEngineConfig`, `crates/risk/src/engine/config.rs`             | Configure (B8)                                              |
| Session boundaries     | `TradingCalendar`, `crates/model/src/calendars/mod.rs`             | Resolve actionability from it (B4)                          |
| Execution and orders   | `Strategy.submit_signals`, `crates/trading/src/strategy/mod.rs`    | Make submission idempotent (B9)                             |
| Fills                  | Execution engine, matching engine, catalog                         | Trace them and attribute the exposure (B10, B15)            |
| Measurement            | `nautilus-research`: `Panel`, `Label`, `operators`                 | Wire the evaluation (B11, B12)                              |

Three facts make the join small rather than large. First, `TargetConstruction` and `TargetReconciler`
are pure functions that read no clock and mutate neither input, so the whole engine-side chain is
unit-testable without an engine and the bridge inherits that testability. Second, the pipeline is
opt-in by construction: a strategy that does not enable one behaves exactly as it did before the
pipeline existed (`crates/trading/src/target_pipeline.rs`), so this workstream cannot change any
existing strategy's behaviour. Third, the session question the actionability rule needs is already
answered as data: `TradingCalendar` states that it "answers whether a given instant is tradeable, and
what the next or previous session boundary is", is keyed by venue, asset class and optional symbol,
and is loaded once and never mutated during a run.

### 1.2 Revision history

| Revision | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1        | Initial record. Twelve items, four phases, one prerequisite recorded as open, and the verification plan; written after the engine-side stages were found already implemented                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 2        | Owner disposition of all eight decisions applied. The expiry requirement, the calendar-resolved `actionable_at`, the three-state disposition, per-leg tradability, the domain-separated order identity and the closed refusal vocabulary specified; the ledger became a first-class audit record                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 3        | Boundary semantics closed and two items added. Gate aggregation is specified as B14 and attribution as B15; `received_at` and the exclusive expiry are specified in B4; the rating policy becomes total in B13; order identity becomes immutable in B9; producer authorization joins B12; the acceptance matrix is the design's section 9 table                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 4        | The implementation gates of `design 10` speak. Gate 1 is negative: the status mapping does not positively identify tradability, so B7 now waits on a route choice rather than on the establishment (`design 3.2`, appendix A). Gate 2 is negative about its domain: the artifact carries one permission claim, not three, so B14's aggregation is over `risk_gate.verdict` and the engine's eligibility (`design 2.1`, `3.2`, appendix A). Gate 3 is negative on totality: the rating table states rows for three of the producer's five ratings (`design 3.6`). Gate 4 is negative on inputs: the temporal rule reads a nullable `produced_at` with no declared outcome (`design 3.6`, `5.2`). Four owner decisions follow from the four findings; no item's mechanism changed                                                                                                                                                                                                |
| 5        | The four findings of revision 4 resolved and every item implemented. Route A chosen for B7 (`design 3.2`); `Overweight` takes `BUY`'s row and `Underweight` takes `SELL`'s, with a short position closing to no signal (`design 3.6`); `PRODUCED_AT_ABSENT` and `CALENDAR_MISSING` added for the two temporal absences (`design 5.2`); I15 restated as one permission claim. Five reachability facts recorded: the wire contract declares no horizon; the producer leaves the advisory allocation null in production; the bundled calendar's coverage ends 2025-12-31; `RiskEngineConfig.count_caps` is unreachable from Python; and two of the five attribution quantities cross no boundary back to the bridge                                                                                                                                                                                                                                                               |
| 6        | The three reachability limits revision 5 recorded are closed, and one expectation in the design is corrected. `count_caps` is a Python surface over `RiskCap`, `RiskCapMetric` and `RiskCapScope`, and the live configuration carries the same caps instead of hardcoding them away (`design 3.5`); the constructed target is exposed as `Strategy.targets`, so the engine-constructed exposure is recorded (`design 6.5`); the risk stage's answer is recorded from the engine's own order events, so a denial is a measured zero (`design 6.5`); the bundled XNYS equity calendar now covers 2026 (`design 3.6`); a directory of calendar documents serves a key the bundle does not carry (`design 3.6`); and the producer publishes the allocation the ceiling reads, in the domain the design said was the producer's to declare (`design 3.7`). The correction: this engine denies rather than resizes, so `risk_approved` is never an intermediate value (`design 6.5`) |

### 1.3 Implementation log

One row per item. **Every item is implemented.** The four findings of revision 4 are resolved, so no
row is gated any longer, and the one implementation gate that tests new code, gate 5, passes
(`design 10`). A row's Status cell names where its item lives and any reachability limit that does
not change the implementation.

| Item | Mechanism                                                                                                                                 | Status                                      | Specification |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ------------- |
| B1   | A pure projection from the decision document to a `TradingSignal`, with the fields that do not map directly decided rather than defaulted | Implemented                                 | 4.1           |
| B2   | An admission gate that validates the envelope and refuses with a reason from the closed vocabulary                                        | Implemented                                 | 4.2           |
| B3   | A first-class admission ledger that makes the chain from decision to fill one query                                                       | Implemented                                 | 4.3           |
| B4   | `actionable_at` resolved from the instrument's trading calendar, bounded below by receipt, with an exclusive required expiry              | Implemented; coverage extended through 2026 | 4.4           |
| B5   | The monotone disposition mapping, with the disposition, the ceiling and the reduction factor as configuration                             | Implemented                                 | 4.5           |
| B6   | A declared-scale conversion for execution-bearing numerics only, refusing a value that does not fit                                       | Implemented                                 | 4.6           |
| B7   | Positive, three-valued, per-leg tradability, where unknown is not tradable                                                                | Implemented (route A)                       | 4.7           |
| B8   | The risk engine configuration the bridge requires: per-instrument notional caps, count caps and rate limits                               | Implemented                                 | 4.8           |
| B9   | Idempotent submission, with a versioned domain-separated order identity that is immutable after a venue rejection                         | Implemented (gate 5 passes)                 | 4.9           |
| B10  | The audit chain from a decision to an order to a fill, queryable in both directions                                                       | Implemented                                 | 4.10          |
| B11  | The measurement: three experiments kept apart, confidence calibration, the reduction factor's effect, redundancy, regime conditioning     | Implemented in `crates/research`            | 4.11          |
| B12  | Producer identity and authorization as separate properties, with an unidentified record excluded from aggregates                          | Implemented in `crates/research`            | 4.12          |
| B13  | The rating policy: total over the rating and position cross product, long-only, with no invented direction                                | Implemented                                 | 4.13          |
| B14  | Deterministic gate aggregation: a declared closed severity mapping, the most restrictive value wins, engine eligibility independent       | Implemented                                 | 4.14          |
| B15  | Bridge attribution: five exposure quantities recorded per decision, so the loss localises itself                                          | Implemented                                 | 4.15          |

## 2. Provenance and reproduction

| Field             | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Research project  | TradingAgents, a local LLM research framework checked out outside this repository                                                                                                                                                                                                                                                                                                                                                                                          |
| Research revision | Read from the working tree without pinning a revision. The earlier review pinned `a7fb99a`, and every research-side claim here is about the code and artifacts as they are written                                                                                                                                                                                                                                                                                         |
| This repository   | Branch `develop`, read from the working tree. The working tree carried uncommitted modifications to unrelated files by concurrent work                                                                                                                                                                                                                                                                                                                                     |
| Method            | Read-only in revisions 1 to 4: files read, text searched, no process run against either tree, no engine instantiated, nothing measured. Revision 5 wrote the bridge package and the Rust measurement module. Revision 6 added the two Python surfaces and the live configuration field, extended the bundled calendar, and changed the producer's own artifact, which is the first research-side change in this record. Both revisions ran the tests recorded in section 5 |
| Revision 2 basis  | The owner's disposition of all eight decisions, received as a review of revision 1 of the design document                                                                                                                                                                                                                                                                                                                                                                  |
| Revision 3 basis  | The owner's second review, which raised boundary semantics rather than architecture; its five corrections and five recommendations are recorded in `design 7.1`                                                                                                                                                                                                                                                                                                            |
| Licence boundary  | The research project is Apache-2.0; this repository is LGPL-3.0-only. No source file, test, fixture, prompt text or copied documentation from it entered this repository, and no artifact here is derived by mechanical transformation of any of them (`design 1.3`)                                                                                                                                                                                                       |

The research-side citations in this document name files and line numbers in that project, which is
interface fact about a published contract. Nothing was copied from it.

## 3. Phases and ordering

Four phases, preceded by the implementation gates of `design 10`. Each phase's acceptance is
observable behaviour, not a compiled artifact.

### 3.1 Phase 1: carriage, admission and availability (B1, B2, B3, B4, B6)

The artifact reaches the engine as custom data, is validated, and either becomes an active admitted
record or is refused with a reason from the closed vocabulary.

**Acceptance.** A re-pushed duplicate produces no second signal and no second order, and its
`DUPLICATE` outcome is recorded. An artifact with no expiry is refused with `MISSING_EXPIRY`. An
artifact whose expiry equals or precedes its resolved actionability is refused with
`ARTIFACT_EXPIRED`, as is an artifact whose production time or receipt time is at or after its
expiry. An artifact whose instant the calendar does not cover is refused with `CALENDAR_UNCOVERED`.
An artifact whose numeric field does not fit its declared scale, or whose allocation is outside its
domain, is refused rather than rounded or clamped. A canonical artifact produced at a known instant
projects to a signal whose `expiry_ns` and event time are both derivable from the artifact, the
receipt time and the calendar alone.

**Evidence.** `python/.venv/Scripts/python.exe -m pytest tests/unit/decision_bridge/ -q`, 136 passed.

### 3.2 Phase 2: policy (B5, B7, B13, B14)

Advisory fields become bounds, tradability becomes a positive engine answer, a view becomes a
direction only through a policy, and the research-side permission claim resolves deterministically
against the engine's independent eligibility.

**Acceptance.** A record whose resolved gate is restrictive cannot yield a long or short signal, by
construction rather than by convention, and the refusal names the gate. Artifact fields in
disagreement still produce one deterministic restrictive result with `GATE_CONFLICT` recorded, and an
unmapped verdict value resolves to restriction rather than to permission. An instrument whose
tradability is unknown produces no signal and names `TRADABILITY_UNKNOWN`; a two-leg instrument whose
second leg is unknown
does the same. Every row of the `design 3.6` rating table produces its stated output, `HOLD` produces
no signal, and `SELL` on a flat instrument produces none. An `UNCERTAIN` disposition admits only when
every independent engine gate passes, and the resulting signal's strength is the configured reduction
rather than any value read from the artifact.

**Evidence.** `python/.venv/Scripts/python.exe -m pytest tests/unit/decision_bridge/ -q`, 136 passed.

### 3.3 Phase 3: enforcement, audit and attribution (B8, B9, B10, B15)

**Acceptance.** A signal that would breach a configured per-instrument notional cap is refused by the
risk engine with a typed denial naming the cap, not by the bridge. A restart between the projection
and the submission does not produce a second order, and a replay after a venue rejection does not
either, because the identity is immutable. Two orders from one decision that differ only by
instrument, leg or role have different client order ids, and the scheme version is recorded. Every
fill in a replay resolves to exactly one decision id. Five exposure quantities are recorded per
decision, and their differences localise a shortfall to sizing, risk or execution.

**Evidence.** `python/.venv/Scripts/python.exe -m pytest tests/integration/test_decision_bridge_execution.py -q`, 5 passed.

### 3.4 Phase 4: measurement (B11, B12)

**Acceptance.** A panel over the admitted stream cannot be constructed if any feature reads the
future, and cannot be constructed with a row whose membership is wrong. The out-of-sample information
coefficient of every score is reported with its uncertainty; `confidence` is reported against
realised hit rates by bucket; the reduction factor's effect is reported across expectancy, Sharpe,
hit rate, drawdown and CVaR rather than as one number; and no aggregate pools a record whose producer
identity is unknown.

**Evidence.** `cargo test -p nautilus-research`, whose suites all pass (lib, factors, feature
leakage, leakage, membership and reproducibility).

## 4. Item specifications

### 4.1 B1 A pure projection to `TradingSignal`

**Observed.** `TradingSignal` carries instrument, direction, horizon, strength, source, expiry and
provenance, and validates that strength is finite and non-negative, that a horizon is positive, and
that an expiry is not before the event time (`crates/model/src/signal.rs`). The decision document
carries a ticker, a direction or rating, a confidence, a horizon, a producer, an expiry, and a run
identity.

**Requirement, in this project's own words.** A decision document projects to exactly one statement of
view, or to a named refusal. The projection reads no clock, consults no model, mutates nothing, and
for equal inputs returns an equal signal. The artifact is necessary to the flow and never sufficient
for it (`design 4 I2`).

**Interface or type involved.** `TradingSignal::new`, with `SignalDirection` from the artifact through
the policy of B13, `horizon_ns` from the configured horizon, `expiry_ns` from its expiry as resolved
in B4, `source` from its producer, and `provenance` from its run identity and its advisory levels.
`strength` is written only by the disposition mapping of B5 and is never read from the artifact
(`design 3.8`).

**Data it needs.** The artifact, the instrument definition for its ticker, the configuration that
names the conventions, the receipt time, and the calendar of B4.

**Refusal behaviour.** An unknown ticker, an unmappable rating, an absent direction, or a horizon
that cannot be expressed as a positive duration each produce a named refusal. A record is never
partially projected: either a signal exists or a refusal does.

**Minimum acceptance.** A unit test over a canonical artifact asserting the projected signal's every
field, and a test per unmappable input asserting the refusal names the field. A test that two
projections of the same artifact are equal, and a test that no artifact field reaches `strength`.

**Revision 5.** The wire contract declares no horizon, so the horizon is the bridge's own
configuration rather than a value read from the artifact, and it is refused at construction when it
is not a positive duration. Direction is derived from the rating alone; a producer-set direction is
recorded as evidence rather than honoured as an instruction.

**Home and status.** A pure function beside its consumer. The alternative, a new crate, is more
surface for one function (`design 7` D1, settled). Implemented in
`python/nautilus_trader/decision_bridge/projection.py`.

### 4.2 B2 The admission gate

**Observed.** The artifact declares a schema version, and at 1.1.0 and later requires an expiry, an
idempotency key, a producer identity and an artifact hash
(`contracts/research_decision.v1.schema.json`, the `x-required-when` block). The contract ignores
unknown fields, so a drifted reader loses a field in silence. The producer declares the allocation's
domain on its own field, and a declared domain is not a validated one at the reader.

**Requirement, in this project's own words.** A record is admitted only if it validates against the
declared schema version, carries every field that version requires, its artifact hash recomputes, its
producer is both identified and authorised, and its numeric fields are inside their declared domains.
Anything else is refused with a reason drawn from the closed vocabulary in `design 5.2`, and a field
the reader does not know is recorded rather than ignored.

**Interface or type involved.** A fallible conversion returning either an admitted value or a typed
refusal whose code is a member of the closed set, in the shape the domain already uses for this
(`MetricResult` in `crates/analysis/src/metric.rs` is a status from a closed vocabulary plus a reason
code, and is the nearest existing precedent).

**Data it needs.** The artifact, the set of schema versions the reader knows, and the authorised
producer set.

**Refusal behaviour.** `SCHEMA_INVALID`, `HASH_INVALID`, `PRODUCER_UNKNOWN`, `PRODUCER_UNAUTHORIZED`,
`INSTRUMENT_UNKNOWN` and `ALLOCATION_INVALID` each name their failure, and the reason is the first
failing stage in the `design 3` order rather than an arbitrary one (`design 4 I6`). A refusal is never
converted into an empty result, and the vocabulary is closed, so a new failure mode requires a
declared code rather than a message.

**Minimum acceptance.** A test per admission code in `design 5.2`. A test that an artifact with an
extra unknown field is admitted and the field is recorded. A test that a hash mismatch is refused
rather than repaired. A test that a known but unauthorised producer is refused with
`PRODUCER_UNAUTHORIZED` and not with `PRODUCER_UNKNOWN`. A test that the declared code set is closed:
an undeclared code cannot be constructed. A test that an artifact failing two stages reports the
earlier stage's reason.

**Home and status.** The boundary reader (B1). Implemented in
`python/nautilus_trader/decision_bridge/artifact.py`.

### 4.3 B3 The admission ledger

**Observed.** Retry artifacts for the same ticker exist beside the originals in the research checkout,
so a repeated analysis of one name on one day is a normal outcome, not an anomaly. The artifact
carries the key that distinguishes the two cases.

**Requirement, in this project's own words.** A second arrival of the same artifact is a no-op
recorded as `DUPLICATE`. A new artifact for the same ticker and reference date is admitted as a
revision, and the two are never confused. The ledger answers both questions from the record's own
identity, not from the file name or the arrival order, and it is the single place the chain from a
decision to a fill can be read.

**Interface or type involved.** A first-class audit record, not incidental logging, carrying at least:
`decision_id`, `artifact_sha256`, `idempotency_key`, `instrument_id`, `produced_at`, `received_at`,
`effective_date`, `expires_at`, `actionable_at`, `admission_timestamp`, `admission_result`,
`refusal_reason`, `diagnostics[]`, `projected_signal_id`, `target_id`, `order_ids[]`, `fill_ids[]` and
`attempts[]`, plus the five exposure quantities of B15. Persistence follows the catalog's existing
conventions, and the record is append-only so that a later stage fills its own fields rather than
rewriting an earlier stage's answer.

**Data it needs.** The artifact's identity fields, the receipt time, and the identifiers and
quantities produced by each later stage.

**Refusal behaviour.** A duplicate is admitted as a no-op with its reason recorded, not refused as an
error and not silently dropped. A revision is admitted and marked as superseding. Diagnostics such as
`GATE_CONFLICT` and `ACTIONABILITY_PAST` are recorded beside a record that continued.

**Minimum acceptance.** A test that the same artifact twice yields one admitted record and one
`DUPLICATE`. A test that two artifacts differing only in their production time yield one revision
chain. A test that a decision, its signal, its target, its orders and its fills are readable from the
ledger alone, with no replay. A test that diagnostics are readable on a record that was admitted.

**Revision 6.** One field is added to the list above: `attempts[]`, the caller-supplied labels of a
decision's deliberate retries, and the one stage permitted to revise its own earlier answer is the
risk stage, because a decision may make several attempts and a later one can pass a gate an earlier
one was denied by. The addition is compatible in the stored form - a document written before the
field existed loads with it empty - and it exists so that a deliberate attempt is never confused
with a duplicate arrival, which stays recorded as a `DuplicateArrival` and never as an attempt.

**Home and status.** Beside the catalog's other stores. Implemented in
`python/nautilus_trader/decision_bridge/ledger.py`.

### 4.4 B4 Actionability resolved from the calendar, bounded by receipt

**Observed.** The artifact separates the reference date of the analysis from the instant the artifact
existed, and bounds how long it stays actionable. `TradingCalendar` answers the engine-side question:
`is_tradeable(ts)`, `next_open(ts)`, `prev_close(ts)`, `sessions_on(date)`, `is_trading_day(date)`,
`is_holiday(date)`, `early_close(date)` and `covers(ts)`, keyed by
`CalendarKey { venue, asset_class, symbol }` and loaded once and never mutated
(`crates/model/src/calendars/mod.rs`).

**Requirement, in this project's own words.** Availability is resolved before reference time, always,
and the resolution is a pure function of the artifact, the receipt time and the instrument's calendar.
An artifact is invisible to a replay until it existed and until this engine received it, and it is not
actionable at or after it expires. The effective instant is never earlier than receipt, because a
retroactively resolved instant would be an instruction to act in the past. An expiry is required and is
never derived from the horizon, because a horizon is semantic and an expiry is operational.

**Interface or type involved.** The six-step resolution rule of `design 3.6`, implemented over the
calendar API above, with the receipt time supplied by the bridge rather than read from the artifact.
The signal's event time is the resolved instant and `expiry_ns` is the artifact's expiry.

**Data it needs.** `produced_at`, `expires_at`, `effective_date`, the bridge's `received_at`, the
instrument's venue and asset class, and the bundled calendar for that key (`calendars::bundled`,
`calendars::bundled_keys`). Session events (`calendars::events`) are the natural trigger for acting on
a resolution that lands at a session boundary.

**Refusal behaviour.** `CALENDAR_UNCOVERED`, `MISSING_EXPIRY`, `ARTIFACT_EXPIRED` and
`ACTIONABILITY_INVALID` each have their own code, and `ACTIONABILITY_PAST` is recorded as a diagnostic
when the clamp of step 5 applies. A guess is never substituted for a resolution, and a calendar whose
coverage ends before the instant is reported rather than extended. Gate 4 found the rule is not total
over its inputs: `produced_at` is nullable in the producer's contract while steps 1, 3 and 4 read it,
and an absent calendar key is not the same case as an uncovered instant; both were given their
declared codes in revision 5 (`design 3.6`, `5.2`).

**Minimum acceptance.** One test per branch of the six-step rule. A test that an artifact produced
after a session's close resolves to the next session's open. A test that an artifact produced during a
session resolves to its own instant. A test that an instant earlier than receipt is clamped and
diagnosed. A test that `expires_at == actionable_at` refuses. A test that `produced_at >= expires_at`
and `received_at >= expires_at` each refuse. A worked example stating the hours of foresight a
reference-date timestamp would have granted.

**Revision 5.** The two absences the rule reads are now named: `PRODUCED_AT_ABSENT` for an artifact
that is schema-valid but carries no `produced_at`, and `CALENDAR_MISSING` for a key with no calendar
at all, which is not the same absence as a calendar that does not cover the instant (`design 3.6`,
`5.2`).

**Revision 6.** The data this item refused to extend is refreshed, and a second source is added. The
bundled `XNYS.EQUITY` document carries the 2026 holidays and the two 2026 early closes and declares
coverage to 2026-12-31, taken from the exchange's published calendar and cross-checked against a
second source, so step 1 stops refusing an artifact produced after 2025-12-31. `JsonCalendarView`
resolves a calendar from a directory of `nautilus-trading-calendar/v1` documents, loaded once at
construction and keyed by the calendar's own key, so an instrument the bundle does not carry, or an
instant past the bundle's window, is served by an operator's own document; the view never consults the
bundle, two documents declaring one key raise, and a document that cannot be parsed is an absent
calendar that the rule already reports as `CALENDAR_MISSING`.

**Home and status.** The projection, reading `crates/model/src/calendars`. Implemented in
`python/nautilus_trader/decision_bridge/temporal.py`, with the calendar sources in `JsonCalendarView`
and `crates/model/resources/calendars/xnys-equity.json`.

### 4.5 B5 The monotone disposition mapping

**Observed.** `TargetConstruction` maps a direction to a signed weight, treats a flat direction as
reduce-to-zero resolved without reading the context, and scales risk by strength clamped to the unit
interval.

**Requirement, in this project's own words.** Every advisory field is a bound. The disposition is
three-valued: `PERMIT` leaves eligibility unchanged and applies the configured risk, `UNCERTAIN`
leaves eligibility unchanged and applies a configured reduction, and a restrictive resolution closes
eligibility so that no order exists. Stated as assertable properties: the effective target is never
greater than the engine's constructed target, and the effective risk is never greater than the
engine's configured budget (`design 4 I2`). The artifact's allocation acts as a ceiling on the
constructed target rather than as a target of its own (`design 4 I13`).

**Interface or type involved.** The projection, expressed as a mapping that is total over the advisory
vocabulary and monotone in exposure by construction. The reduction factor and the ceiling are
configuration, not constants: the factor is written into the signal's strength by the disposition
mapping (`design 3.8`), and the ceiling bounds the constructed weight after construction, per the
`design 3` order. An allocation of zero beside a directional rating resolves to no signal, and the
disagreement is recorded as a diagnostic.

**Data it needs.** The resolved gate from B14, the advisory fields, the direction from B13, the
allocation, and the engine's facts from B7.

**Refusal behaviour.** A restrictive resolution refuses with `RISK_GATE_REJECT` rather than proceeding
at reduced risk. The safe direction of an unknown value is closed, never open.

**Minimum acceptance.** A table-driven test over every (disposition, permission, direction)
combination asserting that the resulting exposure is never greater than the exposure with an absent
advisory block. A test that the reduction factor is read from configuration and appears in the
signal's strength, and a test that no artifact field does. A property test over generated allocations
asserting the effective target never exceeds the engine's constructed target.

**Revision 5.** The advisory ceiling is imposed through `TargetPipelineConfig.max_weight`, which is
replaced per decision: the engine's own construction is strictly lowered, and a decision with no
allocation leaves the engine's own `max_weight` in place.

**Revision 6.** The ceiling is active rather than dormant. The producer publishes its measured book
size as `recommended_allocation_pct`, in the field's declared 0..100 domain, and its contract declares
that domain, so a real artifact now reaches this item's replacement of `max_weight`; the value is
published only when it is measurable and inside the domain, and an absent or out-of-domain size still
publishes null, because a mis-scaled ceiling is worse than none. No bridge rule changed for this: the
domain check, the single normalisation to a fraction and the per-decision replacement were all in
place while the field was null.

**Home and status.** The projection. Implemented in
`python/nautilus_trader/decision_bridge/projection.py` and `gate.py`, with the ceiling applied on the
execution path in `execution.py`.

### 4.6 B6 The float boundary, scoped to execution-bearing fields

**Observed.** The research half computes in floating point throughout; this repository's prices,
quantities and money are fixed point. Its own review records the difference as a structural advantage
(`tradingagents_lessons_design.md` section 9).

**Requirement, in this project's own words.** Every execution-bearing numeric field crossing the
boundary is converted at a declared scale, and a value that does not fit is refused rather than
rounded to fit. Execution-bearing means a price, a quantity, a money amount, a stop distance, or an
allocation that will bound exposure. Research metadata, meaning scores, coverage, sentiment and
confidence, is preserved in its research-native representation, because it has no execution purpose
and is consumed by measurement rather than by the order layer (`design 4 I7`). The declared scale is
part of the configuration, not an implicit default of the parser.

**Interface or type involved.** The existing decimal conversion seam used by the risk crate's Python
bindings (`crates/risk/src/python/sizing.rs` extracts a `Decimal` from a Python value), and the
domain types `Price`, `Quantity` and `Money`. The allocation's domain check belongs to B2, and its
normalisation to a fraction happens exactly once.

**Data it needs.** The declared scale per execution-bearing field class, and the declared domain per
bounded field.

**Refusal behaviour.** A value that is not representable at its declared scale is refused with
`FLOAT_CONVERSION_OVERFLOW`, matching the construction stage's own `StopPrice` refusal for the same
condition at the price layer. An allocation outside its domain is refused with `ALLOCATION_INVALID`
and is never clamped.

**Minimum acceptance.** A test that a value needing more precision than declared is refused and not
rounded. A test that an allocation of negative, above the maximum, infinite and not-a-number is
refused. A test that a research score is passed through unchanged rather than converted.

**Home and status.** The projection. Implemented in
`python/nautilus_trader/decision_bridge/numeric.py`.

### 4.7 B7 Positive, three-valued, per-leg tradability

**Observed.** The engine models instrument status explicitly, including a state whose documented
meaning is that the instrument is not available for trading either because trading has closed or
because it has been halted (`crates/model/src/enums.rs:978`; `InstrumentStatus` in
`crates/model/src/data/status.rs`). The earlier review's item L9 asks that the tradability gate guard
every leg and that an unhandled status action be recorded rather than dropped; it is recorded there as
unimplemented, and a later read of that item's cited match block did not reproduce the claim
(`design 7` D6). Revision 4 resolved the prerequisite negatively, and revision 5 chose route A
(`design 3.2`, appendix A).

**Requirement, in this project's own words.** No instrument reaches construction unless the engine has
positively answered that it is tradeable. The answer is three-valued, unknown is not tradable, and
every leg of a multi-leg instrument is established separately, because a spread whose second leg
cannot trade is not tradable. A requirement of the form "the status is not a rejection" is not
sufficient, because it admits the unknown case.

**Interface or type involved.** The engine's status types, read at the moment of admission and again
at the moment of submission, because a status can change between the two. The instrument's own
lifetime (`activation_ns`, `expiration_ns`) is consulted separately, since the calendar module states
that lifetime belongs to the instrument and not to the calendar.

**Data it needs.** Instrument status, the instrument definition, and the legs of a multi-leg
instrument.

**Refusal behaviour.** `TRADABILITY_UNKNOWN` for an unrecognised or unavailable status, and
`TRADABILITY_REJECTED` for a status that positively refuses. Gate 1 established that the current
status mapping does not satisfy I14; this item is built on route A of `design 3.2`, and does not
re-adjudicate the mapping.

**Minimum acceptance.** A test that a bullish record for an instrument in the not-available state
produces no order. A test that an unrecognised status produces `TRADABILITY_UNKNOWN` rather than
admitting. A test that a two-leg instrument with one unknown leg produces no order. A test that a
status change between admission and submission is honoured.

**Revision 5.** Route A is implemented: the bridge keeps its own positive, three-valued, per-leg
state from `InstrumentStatus` events, reads `is_trading`, treats an absent reading as unknown, and
refuses the whole instrument when any leg refuses. Nothing in the execution engine, the risk engine
or any existing strategy changes.

**Home and status.** The eligibility layer, taking the prerequisite from `crates/execution`.
Implemented in `python/nautilus_trader/decision_bridge/tradability.py`, by route A of `design 3.2`.

### 4.8 B8 The risk configuration the bridge requires

**Observed.** `RiskEngineConfig` already carries per-instrument notional caps, count caps each a
predicate over a scope, a metric and a window, and order-submit and order-modify rate limits
(`crates/risk/src/engine/config.rs`). Its own documentation states that a cap refuses the actions that
increase exposure when the count it observes reaches its limit, that it never refuses a cancellation,
and that caps are evaluated in configuration order so the first reached cap names the refusal. The
configuration denies unknown fields.

**Requirement, in this project's own words.** The exposure limits the bridge exists to enforce are
configured and pinned by a test, and the refusal a breach produces names the cap that bound it. Gross
and net exposure limits live here rather than in a constructor (`design 3.4`).

**Interface or type involved.** `RiskEngineConfig`, in the engine's own configuration surface. The
`UNCERTAIN` reduction factor is **not** here: it is projection configuration (`design 3.8`), because
it shapes the signal rather than judging the order.

**Data it needs.** The desk's limits, which are policy and not derivable here.

**Refusal behaviour.** The risk engine refuses with `ENGINE_RISK_LIMIT`, and the denial is typed. The
bridge does not pre-check what the risk engine owns, because a second opinion on a limit is a second
authority.

**Minimum acceptance.** A test that an order breaching a configured notional cap is refused with the
cap named. A test that a cancellation is never refused by a cap.

**Revision 5.** Per-instrument notional caps and both order rate limits are configured and proven.
Count caps are unreachable from Python: `RiskEngineConfig.count_caps` has no Python binding, and the
live runtime hardcodes an empty list (`LiveRiskEngineConfig` in `crates/live/src/node/config.rs`), so
the bridge records them as unreachable rather than emulating them.

**Revision 6.** The gap is closed on both sides. `RiskCap`, `RiskCapMetric` and `RiskCapScope` are
Python classes over the engine's own vocabulary, with a class attribute per variant, so a cap names
its metric and scope exactly as the Rust configuration does; `RiskEngineConfig` takes `count_caps`;
and the live configuration carries the same caps in its own `METRIC/SCOPE/LIMIT[/WINDOW_NS]` string
form, parsed into the same `RiskCap` values and validated by the same builder, so a cap declared for a
live node is no longer dropped on the way to the engine and backtest and live share one validation
path. `RiskLimits` declares the caps beside the notional limits, `UNREACHABLE_FROM_PYTHON` is empty,
and the test that pinned the gap is replaced by three that prove the capability: the empty
reachability list, the engine's own validation refusing an `Active` cap declared with a window, and a
backtest in which a `SUBMIT/INSTRUMENT/1` cap denies the second submission with
`ORDER_COUNT_LIMIT_REACHED`.

**Home and status.** Engine configuration. Implemented in
`python/nautilus_trader/decision_bridge/risk.py`, with the bindings in
`crates/risk/src/python/config.rs` and `crates/live/src/python/config.rs`.

### 4.9 B9 Idempotent submission, with an immutable identity

**Observed.** `Strategy.submit_signals` returns the client order ids in the order the orders were
emitted, and the identifier accepts an ASCII string
(`crates/trading/src/strategy/mod.rs`, `crates/model/src/identifiers/client_order_id.rs:53`). The
artifact carries the key that makes this possible.

**Requirement, in this project's own words.** A decision produces at most one order set, ever, across
restarts, re-reads and retries. The identity of an order is derived from the identity of the decision
plus what distinguishes one order from another within it: the instrument, the leg and the order's
role. The derivation is domain-separated and versioned, so a scheme change is visible in the order
rather than inferred from history, and hashing the raw key alone is not sufficient because one
decision may produce several orders. The identity is **immutable for the lifetime of the decision**: a
venue rejection, a cancellation, or a lost acknowledgement does not free it, so a replay produces no
second order, and a deliberate retry requires a new execution-attempt identity derived from an
explicit retry policy. The strictness is not caution: from the engine's side a lost acknowledgement
after venue acceptance is indistinguishable from a rejection, so treating a rejection as permission to
resubmit risks duplicate exposure.

**Interface or type involved.** A deterministic client order id over a domain-separated digest, in the
shape `hash("ta-bridge-v1" || idempotency_key || instrument_id || leg_id || order_role)`, encoded to
the identifier's permitted ASCII shape, with the scheme identifier and the encoding recorded alongside
the order. Three identifiers are deliberately distinct: the decision identity, the artifact's
`idempotency_key`, and the engine's client order id. A retry policy, if one is added, produces a new
attempt identity and therefore a new order id, and it is a separate configuration rather than a change
to this scheme.

**Data it needs.** The idempotency key, the instrument, the leg and the role, and the encoding's
length budget.

**Refusal behaviour.** A collision with an existing order is treated as an already-submitted decision,
not as an error to retry. A replay after a rejection is a collision like any other.

**Minimum acceptance.** A test that submitting the same decision twice yields one order set. A test
that two orders differing only by role, and two differing only by instrument, have different ids. A
test that the scheme version is recorded with the order. A test that a replay after a venue rejection
produces no second order. A test that a retry under an explicit attempt policy produces a different
id and is therefore distinguishable from the original.

**Revision 5.** The engine mints the client order ids on the submission path, so the decision-level
guard is what makes a replay a no-op: the bridge derives the identity from the decision, the
instrument, the leg and the role, and the same decision never produces a second order set.

**Revision 6.** The retry policy this item said would be separate is implemented as its own
configuration, and it is the design's own rule rather than a new one. A denial is classified from the
canonical leading token of the engine's rendered message (`DENIAL_CODES` transcribes the engine's
49-code vocabulary; `RETRYABLE_DENIAL_CODES` is the five whose condition the engine states clears or
whose limit is a window that closes), and an unrecognised token is terminal. `RetryPolicy` bounds the
budget, `RetryDecision` carries the attempt count it was computed against, and `submit_once` submits
the same decision again only when the permission is permitted and its count still matches the
ledger's, so a stale permission submits nothing. A permitted retry reaches the engine again, appends
the engine's new order ids and records an attempt label, which is why a deliberate attempt is never
confused with a duplicate arrival; the bridge holds no clock, so the caller labels the attempt and
owns any waiting.

**Home and status.** The submission path. Implemented in
`python/nautilus_trader/decision_bridge/execution.py`, with the identity scheme in `identity.py`.

### 4.10 B10 The audit chain

**Observed.** Fills carry a client order id (`crates/model/src/reports/fill.rs`), and orders carry
provenance-carrying parameters, so both ends of the chain already exist. What does not exist is the
link from the order back to the decision that caused it.

**Requirement, in this project's own words.** Given a fill, the decision that caused it is
retrievable, and given a decision, its orders and fills are retrievable, without replaying the stream.

**Interface or type involved.** The decision identity carried on the order, in the position the order
layer already uses for caller-supplied metadata, plus the ledger of B3, whose `order_ids[]` and
`fill_ids[]` fields are what make the chain one query.

**Data it needs.** The decision identity and the order identity.

**Refusal behaviour.** An order with no decision identity is not attributed to a decision; it is
reported as unattributed rather than assigned to the nearest one.

**Minimum acceptance.** A test that a replay's fill resolves to the decision id. A test that an order
placed by the strategy's direct path is reported as unattributed.

**Home and status.** The order and persistence layers. Implemented in
`python/nautilus_trader/decision_bridge/ledger.py`.

### 4.11 B11 The measurement

**Observed.** `Panel::new` refuses a feature that reads past its row timestamp
(`PanelError::Lookahead`) and refuses a row whose membership disagrees with the stored series
(`crates/research/src/panel.rs:150,176`). `Label` carries a horizon and a stated terminal convention,
and the crate's operators include a cross-sectional rank (`crates/research/src/lib.rs`,
`crates/research/src/operators.rs`).

**Requirement, in this project's own words.** Three experiments are kept apart so that a poor result
is attributed to the right cause: the research decision against a forward return with no bridge, the
research decision against the eligible signals the policy yields, and an eligible signal against its
target, its risk decision and its fill. Within them: an information coefficient series per score with
its uncertainty, a calibration of `confidence` against realised hit rates by bucket, the reduction
factor's effect measured across expectancy, Sharpe, hit rate, maximum drawdown and CVaR with the
differences reported, and the correlation structure of the score set (`design 6`).

**Interface or type involved.** `Panel`, `Label`, and the existing cross-sectional operators. The
regime prior is read from `implementation/sector-regime-engine/`, which carries a pre-registered
decision rule and a null model; that application stays where it is and is not duplicated here. The
reduction factor is a hypothesis under measurement rather than a constant, and "no meaningful
difference" is an admissible result.

**Data it needs.** The admitted decisions with their scores and coverage, a forward label per horizon
over a declared universe, and a membership series.

**Refusal behaviour.** A feature whose availability is after its row's event time cannot be
constructed into a panel, by the existing check, which is the mechanism and not a convention.

**Minimum acceptance.** A test that a deliberately future-dated feature is refused at panel
construction. A test that the information coefficient of a known synthetic signal with a known
relationship to a known label recovers that relationship within its stated uncertainty. A test that a
score whose coverage is absent is reported as such rather than measured as neutral (`design 4 I11`). A
test that the three experiments report separately on the same admitted stream. A test that the
reduction factor's effect is reported across the metric set with its uncertainty.

**Revision 5.** The measurement is a new module in the existing `nautilus-research` crate,
`crates/research/src/measurement.rs`. The three experiments are three distinct types; the information
coefficient series uses the crate's own cross-sectional rank with jackknife standard errors; the
calibration buckets `confidence` by rating and horizon; and the reduction factor's effect, the
redundancy matrix and the regime conditioning are computed from the crate's operators.

**Home and status.** The research layer, using the existing mechanisms. Implemented in
`crates/research/src/measurement.rs`.

### 4.12 B12 Producer identity and authorization

**Observed.** The artifact carries a producer identity and an artifact hash. This repository already
accepted the rule that a result whose producer identity is unknown is excluded from an aggregate
rather than pooled with the rest (`tradingagents_lessons_implementation.md`, item L13, recorded there
as specification only because no consumer existed).

**Requirement, in this project's own words.** Producer identity and producer authorization are
different properties, and a producer that is identified but not authorised is refused rather than
trusted. Every admitted decision carries its producer identity, and no aggregate in B11 pools a record
whose identity is unknown (`design 4 I12`).

**Interface or type involved.** The existing identity machinery for research results
(`python/nautilus_trader/optimization/identity.py`), the artifact's own identity fields, and the
authorised producer set that B2 consults. This item supplies the consumer L13 lacked, and does not
build a second identity mechanism.

**Data it needs.** The producer identity, the artifact hash, the run identity, and the authorised
producer set.

**Refusal behaviour.** `PRODUCER_UNAUTHORIZED` at admission for a known but unauthorised producer, and
exclusion from every aggregate, counted and reported, for an admitted record whose identity is
unknown.

**Minimum acceptance.** A test that an unauthorised producer is refused and is distinguishable from an
unknown one. A test that an unidentified admitted record appears in no aggregate and in the exclusion
count.

**Revision 5.** The aggregate-side rule is implemented: a record whose producer identity is unknown is
excluded from every aggregate and counted, so no aggregate pools it.

**Home and status.** The research layer. Implemented: the aggregate-side rule in
`crates/research/src/measurement.rs`, with the producer refusal at admission in
`python/nautilus_trader/decision_bridge/artifact.py`.

### 4.13 B13 The rating policy

**Observed.** The artifact states a rating from a small vocabulary, and the engine's exposure depends
on the position that already exists. A rating is therefore not a direction: `HOLD` with a position is
not the same instruction as `HOLD` flat, and `SELL` flat is not the same as `SELL` long.

**Requirement, in this project's own words.** The pair of a research view and the current position
resolves to exactly one engine result through a policy that is total over the cross product and closed
on an unknown rating, and every row's output is stated rather than left to an implementer. `HOLD`
produces no signal at all rather than a directional signal with a maintain flag, because this
repository's `TradingSignal` has no such flag and inventing one would change a platform type for a
bridge policy. `SELL` on a long position resolves to a zero target rather than a partial reduction,
because a partial exit needs a specification the artifact does not carry. `SELL` on a flat position
produces no signal, because revision 1 is long-only. The bridge never invents a direction: a `BUY` is
never resolved to flat or short, and a `SELL` is never resolved to long (`design 4 I2`).

**Interface or type involved.** The policy table of `design 3.6`, evaluated inside the projection from
the artifact's rating and the position read from the cache and portfolio, which stay authoritative. A
non-zero allocation beside a `SELL` is not consumed and is recorded as a `GATE_CONFLICT` diagnostic,
with the target remaining zero as the restrictive resolution.

**Data it needs.** The rating, the current position for the instrument, and the long-only scope.

**Refusal behaviour.** An unrecognised rating refuses with `RATING_UNKNOWN`. A view that the policy
declines produces no signal and is recorded with its reason, so a declined policy is distinguishable
from a missing artifact (`design 5.1`).

**Minimum acceptance.** A table-driven test over the full rating and position cross product asserting
every row of `design 3.6`, including that `HOLD` yields no signal in both position states and that
`SELL` long yields a zero target. A test that an unrecognised rating refuses. A test that a declined
view is recorded as such rather than as an absent artifact. A property test that no policy row changes
a view's direction.

**Gate 3 (revision 4).** The table this item tests was not total: the producer's rating vocabulary is
five-valued and `design 3.6` then stated rows for three, so `Overweight` and `Underweight` needed
stated rows before this item could be tested against "every row" (`design 3.6`, appendix A).

**Revision 5.** The table is now total over the producer's five-valued vocabulary: `Overweight` takes
`BUY`'s row and `Underweight` takes `SELL`'s, and a short position closes to no signal for every
rating.

**Home and status.** The projection. Implemented in
`python/nautilus_trader/decision_bridge/ratings.py`.

### 4.14 B14 Deterministic gate aggregation

**Observed.** The artifact carries one field that claims something about permission, and its
vocabulary is PASS, WARN or REJECT. Revision 3 recorded two further permission fields here; revision
4 found that both names are reserved by the artifact's own consumer and that a producer-set value is
refused by the producer's own validator, so the aggregation's artifact-side domain is that one
verdict (`design 3.2`, appendix A). Before this bridge nothing aggregated the verdict with the
engine's eligibility, so two readers could reach different answers from one artifact.

**Requirement, in this project's own words.** The artifact's permission claim resolves to one
severity through a declared, closed mapping; the most restrictive value wins; a permissive value
never overrides a stricter one; a value outside the mapping resolves to restriction; and engine
eligibility is evaluated independently and cannot be raised by anything research-side
(`design 4 I15`). A disagreement between the artifact's claim and the engine's eligibility, or
between two artifact fields that `design 3.6` resolves restrictively, is recorded as `GATE_CONFLICT`
while the effective gate remains the most restrictive resolution.

**Interface or type involved.** A severity type with three members (`PERMIT`, `UNCERTAIN`,
`RESTRICT`), a declared mapping from the claim's vocabulary onto those members, and a maximum
reduction over severity. The mapping is configuration and is pinned by a test, so that a producer's
vocabulary change is a visible configuration change rather than a silent behaviour change. The
aggregation is a pure function and is total: it is defined for every combination, including
unmapped values.

**Data it needs.** The artifact's permission claim, its declared mapping, and the engine's own
eligibility answer.

**Refusal behaviour.** An unmapped value resolves to `RESTRICT` rather than to an error, because the
artifact is still readable and the restrictive answer is correct; the unmapped value is recorded. A
restrictive resolution that stops the flow refuses with `RISK_GATE_REJECT`.

**Minimum acceptance.** A test over the full cross product of the verdict's vocabulary and the
engine's eligibility, asserting that the result is the most restrictive input and is
order-independent. A test that a permissive verdict beside a restrictive eligibility yields
restriction. A test that an unmapped verdict value yields restriction and is recorded. A test that a
research `PERMIT` beside an engine refusal yields no order. A commutativity test: permuting the
inputs does not change the result.

**Home and status.** The eligibility layer, beside B5. Implemented in
`python/nautilus_trader/decision_bridge/gate.py`, with gate 2's domain correction applied.

### 4.15 B15 Bridge attribution

**Observed.** The chain has four places where a desired exposure can shrink before a fill, and none of
them records the value it received, so a shortfall cannot be localised after the fact.

**Requirement, in this project's own words.** For every decision, five exposure quantities are
recorded: research requested, bridge capped, engine constructed, risk approved and actually filled.
The differences localise a shortfall to sizing, to a risk constraint or to execution, without
re-deriving the path (`design 6.5`).

**Interface or type involved.** Five fields on the B3 ledger record, each written by the stage that
owns it, so the sequence is an audit trail rather than a reconstruction. No new arithmetic is
required: the engine construction and the risk decision already produce the numbers, and the bridge
records them.

**Data it needs.** The requested allocation, the applied ceiling, the target the pipeline constructed,
the risk engine's answer as the engine itself states it, and the fill quantity.

**Refusal behaviour.** A stage that did not run leaves its quantity absent rather than zero, because a
zero would claim that the exposure was considered and refused when it was never reached. An absent
quantity is reported as not reached.

**Minimum acceptance.** A test that all five quantities are recorded for a decision that fills. A test
that a decision refused before construction records the stages it did not reach as absent. A test that
a constructed target below the requested allocation and a filled exposure below the approved one each
report their own difference.

**Revision 5.** Two of the five quantities do not cross back to the bridge: `submit_signals` returns
client order ids only, so the engine-constructed target and the risk engine's own decision are
recorded as absent and reported as not reached rather than as zero.

**Revision 6.** Both quantities revision 5 could not observe are recorded, and one expectation is
corrected. The constructed exposure is read from the target the pipeline's construction stage
produced (`Strategy.targets`), which is a weight target on the ceiling path, so the weight is the
allocation and is recorded in the artifact's percent units; a target that is not a weight records
nothing, because deriving an allocation from a quantity or a notional would repeat the engine's own
sizing and price resolution. The risk stage's answer is read from the engine's own order events: an
order on the submission path records the constructed allocation as approved, and a denial records a
measured zero, because a denial is this engine's form of a risk constraint. Whether an order was
denied is read from the event and never from classifying its message, so a denial whose code the
transcription does not carry is still a denial; the first implementation classified it, and that
defect was found in review and fixed before this revision was recorded. No intermediate value is
produced here, and no `target_id` is written either: the engine's `Target` carries no identity, so
that field stays a named absence.

**Home and status.** The ledger and the order layer. Implemented in
`python/nautilus_trader/decision_bridge/ledger.py`, with the exposure quantities recorded on the
execution path in `execution.py`.

## 5. Verification plan

The acceptance matrix is the design document's section 9 table, which is normative here, and it covers
the temporal cases, the gate cases, the `HOLD` and `SELL` cases, the allocation boundaries, the replay
cases and the attribution chain. This section records what was run to prove each phase and what cannot
be proven here.

| Phase | Proof                                                                                                                                                                                                                                                                                                  |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1     | `python/.venv/Scripts/python.exe -m pytest tests/unit/decision_bridge/ -q` -> 136 passed                                                                                                                                                                                                               |
| 2     | `python/.venv/Scripts/python.exe -m pytest tests/unit/decision_bridge/ -q` -> 136 passed                                                                                                                                                                                                               |
| 3     | `python/.venv/Scripts/python.exe -m pytest tests/integration/test_decision_bridge_execution.py -q` -> 5 passed, now asserting the exposure chain: the construction below the ceiling, an approval equal to the construction, a denial measured as zero, one event per order, and an unattributed order |
| 4     | `cargo test -p nautilus-research` -> all suites pass (lib 23, factors 11, feature_leakage 4, leakage 2, membership 3, reproducibility 3)                                                                                                                                                               |

Revision 6's own evidence, all run against the rebuilt extension:

| Item           | Proof                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The extension  | `maturin build --release --locked --out dist` builds the wheel, whose extension binary is installed into the source package. `maturin develop` is not usable on this host: it invokes a `uv` older than the `>=0.12,<0.13` pin in `python/pyproject.toml` and exits 2, so the wheel route is the recorded one                                                                                                                                                                         |
| Unit           | `pytest tests/unit/decision_bridge -q` -> 136 passed (105 at revision 5)                                                                                                                                                                                                                                                                                                                                                                                                              |
| Integration    | `pytest tests/integration/test_decision_bridge_execution.py -q` -> 5 passed, now asserting the exposure chain on a real engine                                                                                                                                                                                                                                                                                                                                                        |
| Rust           | `cargo nextest run -p nautilus-risk -p nautilus-trading -p nautilus-live` -> 3314 passed, 31 skipped                                                                                                                                                                                                                                                                                                                                                                                  |
| Lint and hooks | `ruff check` clean on every changed Python file; `cargo fmt --check` and the python-feature `cargo clippy` clean on the three crates; `check_pyo3_conventions.sh` exit 0                                                                                                                                                                                                                                                                                                              |
| Not run        | `python generate_stubs.py`. Regenerating the `.pyi` stubs rewrites Rust doc comments across the workspace and then builds the stub binary with the packaging feature set, so the checked-in stubs do not yet carry the new Python surfaces; a generated artifact may not be edited by hand, so this is recorded as an outstanding packaging step rather than closed here                                                                                                              |
| Documentation  | The count-cap sweep across the twenty-two `docs/usermanauls/` pages and `docs/concepts/execution/index.md`. Markdown tables pass 1 rc 1 (the padding normaliser) and pass 2 rc 0; `check_docs_conventions.sh` exit 0. Both examples were executed verbatim against the rebuilt extension: `RiskCap(RiskCapMetric.Submit, RiskCapScope.Instrument, 2_000, 60_000_000_000)` and `LiveRiskEngineConfig(count_caps=["SUBMIT/INSTRUMENT/2000/60000000000"])` both construct and round-trip |

Three things cannot be established by any test in this repository, and the plan does not pretend
otherwise. The research half's own backtest numbers are not comparable to this engine's fills, because
its model is a daily-bar model with one intrabar path and no queue position (its own review records
this). The information leakage of a language model cannot be removed by the engine half; it can only
be measured out of sample, which is what B11 is for. And the reproducibility of a run of the research
half is that project's stated non-goal, not something this bridge can supply.

Two verification notes remain useful. The existing parity harness
(`python/tests/integration/test_target_pipeline_parity.py`) is the pattern a bridge parity test should
follow, and `python/tests/unit/backtest/test_backtest_engine_custom_data.py` is the pattern for
carriage: registration, catalog write, catalog query, `add_data` with `sort=True`, and `on_data`
delivery. Both were read, not run.

## 6. Dependency graph

```text
Artifact contract (exists) -------> B2 admission gate ----> B3 ledger ----> B1 projection
                                                                    |              |
Authorised producer set ------> B12 identity/authorization ---------+              |
Instrument definition (exists) -------------------------------------+              |
Engine status types (exists) --> B7 tradability (gate 1 of design 10) ------------->+
TradingCalendar (exists) ------> B4 actionability --------------------------------->+
                                                                                  |
B14 gate aggregation --> B13 rating policy --> B6 numeric conversion --> B5 disposition --
                                                                                  |
Signal + Target + pipeline (exists) <---------------------------------------------+
        |
        +--> B8 risk configuration (exists to configure) --> B9 idempotent submission --> B10 audit chain
        |                                                                                    |
        |                                                                                    v
        +--> B11 measurement <-- B12 producer identity                      B15 bridge attribution
```

Everything above the pipeline row is new; below it the engine's stages are existing, with the
bridge's configuration, idempotent submission, audit, measurement and attribution alongside them.

## 7. Prerequisites and deferred work

All eight of `design 7` are settled, and the four findings of revision 4 are resolved and applied.
Revision 5 implemented every item, and all five gates of `design 10` are closed: gates 1 to 4 were
executed read-only in revision 4 and returned negative, and gate 5, the only one that tests new code,
passes. This section states the four resolutions, the two reachability limits and the three
producer-side facts together, each also recorded where the item it affects is specified.

**Revision 6.** Three of the bullets below no longer state a limit, and one no longer states a
standing condition. The count caps are settable from Python as first-class `RiskCap` values and are
carried through the live configuration instead of being dropped; the two attribution quantities are
recorded, one from the target the construction stage produced and one from the engine's own order
events; the bundled calendar's coverage runs to 2026-12-31, with a directory of documents as a second
source; and the producer publishes the allocation the ceiling reads, so the ceiling is active rather
than inert. The bullets are left as revision 5 wrote them, because each was true then; the paragraphs
above and the per-item notes record what changed. Two items stand as stated: the wire contract still
declares no horizon, and the deferred policies of `design 3.3`, `3.4` and `3.8` are still deferred.
One packaging step is outstanding and is named rather than implied: the generated `.pyi` stubs do not
yet carry `RiskCap`, its two vocabularies, `count_caps` on either configuration, or
`Strategy.targets`, because regenerating them rewrites Rust doc comments across the workspace and then
builds the stub binary with the packaging feature set; hand-editing a generated artifact is not an
option, so `python generate_stubs.py` remains to be run once the tree is settled.

The documentation was swept with the change. Twenty-two pages across the `docs/usermanauls/` styles
asserted that the count caps were unreachable from Python, and one of them cited a
`crates/risk/src/python/config.rs` line range this change moved; the statements now describe the
Python surface and the live encoding, and the line range is replaced by the symbol. The canonical
concept page for the caps (`docs/concepts/execution/index.md`) gained the Python and live examples
beside its mechanism. Two records are deliberately left as history: the revision-5 rows and bullets
in this record and in the design record, which state the limit that revision 6 closed.

- **B7's prerequisite, resolved.** Whether the current execution status mapping positively identifies
  tradability, per leg, was contested between two records in this repository. Gate 1 established that
  it does not satisfy I14, so route A was chosen: the bridge keeps its own positive, three-valued,
  per-leg state from `InstrumentStatus` events, and nothing in the execution engine, the risk engine
  or any existing strategy changes (`design 3.2`).
- **B14's domain, corrected.** Gate 2 established that the artifact carries one permission claim, not
  three, because two of the names revision 3 read as artifact fields are reserved by the artifact's
  own consumer (`design 3.2`, appendix A). B14's rule is unchanged over the corrected domain and is
  implemented as such.
- **B13's rows, stated.** Gate 3 established that the rating table stated rows for three of the
  producer's five ratings. `Overweight` now takes `BUY`'s row and `Underweight` takes `SELL`'s, and a
  short position closes to no signal for every rating (`design 3.6`).
- **B4's input, decided.** Gate 4 established that the temporal rule consumes a nullable `produced_at`
  with no declared outcome, and that an absent calendar key is not the uncovered-instant case. Both
  absences are now named: `PRODUCED_AT_ABSENT` for the absent instant and `CALENDAR_MISSING` for the
  absent key (`design 3.6`, `5.2`).
- **Reachability limit: count caps.** `RiskEngineConfig.count_caps` has no Python binding, and the
  live runtime hardcodes an empty list (`LiveRiskEngineConfig` in `crates/live/src/node/config.rs`),
  so count caps cannot be configured from Python and are recorded as unreachable rather than
  emulated.
- **Reachability limit: two attribution quantities.** Two of B15's five quantities, the
  engine-constructed target and the risk engine's approval, never cross back through
  `submit_signals`, so the bridge leaves them absent and the ledger reports them as not reached.
- **Producer-side fact: no horizon.** The wire contract declares no horizon field, so the horizon is
  bridge configuration rather than a value read from the artifact.
- **Producer-side fact: the allocation is null.** The producer writes `recommended_allocation_pct` as
  null in production, so the ceiling is implemented and tested but inactive until the producer
  publishes a measured allocation; `position.size_pct_book` is not read because the contract declares
  no domain for it.
- **Producer-side fact: calendar coverage.** The bundled `XNYS.EQUITY` calendar covers 2024-01-01
  through 2025-12-31, so step 1 of the temporal rule refuses every artifact produced after it -- a
  data condition, not a code defect.
- **Deferred with a condition, not forgotten.** The bounded stop-tightening policy (`design 3.3`),
  the calibrated confidence-to-size function (`design 3.8`), any cross-sectional portfolio
  construction (`design 3.4`), and any retry policy after a venue rejection (B9). Each would need its
  own configuration, its own measurement, and, for the first three, no capacity to loosen what the
  engine owns.
- **The item numbering** here (B1 to B15) is local to this pair of documents and does not extend the
  L1 to L14 series of the earlier review.
