# The research-to-execution bridge: projecting a research decision onto a trading signal

**Status:** design only. No code was changed by this record.

The companion implementation plan is
[tradingagents_decision_bridge_implementation.md](tradingagents_decision_bridge_implementation.md).
The earlier review of the same research project is
[tradingagents_lessons_design.md](tradingagents_lessons_design.md), which harvested governance
mechanisms from it; this record asks a different question, and is the first to propose a join.
Provenance, revision history and the narrative of how each decision changed are in Appendix A; the
body is written as a specification of the current design.

The request that produced this record drew the chain explicitly. On the research side: raw evidence,
analysts, score engines, an LLM debate and adjudication, a decision context, a research decision. On
the engine side: decision context, strategy eligibility, position target, portfolio construction,
risk, execution, orders, fills.

**That chain already exists twice.** The research half runs in the TradingAgents checkout next door
and emits a versioned decision artifact. The engine half exists in this repository as
`TradingSignal`, `Target` and the optional target pipeline, which resolve a statement of view into a
desired exposure and then into the minimal order set. What does not exist is the join: nothing reads
one and produces the other. This record specifies that join, and nothing else.

| Revision | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1        | Initial record. Read-only against both trees; the engine-side stages were found already implemented and the design was reduced to the projection itself                                                                                                                                                                                                                                                                                                       |
| 2        | Owner disposition of all eight decisions recorded, with D3, D5 and D7 modified and D6, I2 and the allocation rule strengthened; the policies, the lifecycle and the refusal vocabulary added; `actionable_at` resolved from the bundled `TradingCalendar`                                                                                                                                                                                                     |
| 3        | Boundary semantics closed: deterministic gate precedence (I15), rating-to-signal semantics made total including `HOLD` and `SELL`, receipt time added and retroactive action forbidden, expiry made exclusive, the allocation domain validated, order identity made immutable after a venue rejection, producer authorization separated from identity, bridge attribution added, and the acceptance matrix expanded. Provenance narrative moved to Appendix A |

## 1. Purpose and scope

### 1.1 Two halves, two authorities

The two systems are complementary because they answer different questions and are authoritative for
different things.

| Question                           | Research half (TradingAgents)                 | Engine half (this repository)                  |
| ---------------------------------- | --------------------------------------------- | ---------------------------------------------- |
| What is true about the instrument? | Reads about thirty vendors, scores the result | One authoritative source per instrument        |
| What do we think should happen?    | A written thesis, a rating, a debate verdict  | Not a question this repository asks            |
| What is a price?                   | A read, with a stated caliber and staleness   | The instrument definition and the venue        |
| What exposure do we want?          | An advisory allocation percentage             | `Target`, resolved against portfolio state     |
| May this order exist?              | An advisory gate verdict in a document        | The risk engine, which can refuse              |
| What actually happened?            | Nothing: it routes no order                   | Fills, commissions, slippage, venue rejections |

The asymmetry that matters: the research half produces claims and opinions, and the engine half owns
authoritative market, portfolio, risk, order and fill state. A fill is a fact in a way that a gate
verdict is not, and an engine-computed target is authoritative in a way that a research allocation is
not. The whole design follows from keeping those apart.

The governing principle, adopted from the owner's review of this record, states the same rule in one
line:

> Research may constrain or inform the engine, but it must never become a second execution or risk
> authority.

### 1.2 The seam is one artifact

The research half writes a decision document to disk. Its own contract file calls it "the artifact
the research layer drops at the boundary", names the intended consumer, and states that the consumer
"fails closed on anything it cannot validate" (`tradingagents/reporting.py:591` in that project). The
document is produced in production today and declares schema version 1.2.0
(`execution_contract.py:41`), with the required set extending at 1.1.0 to `expires_at`,
`idempotency_key`, `producer` and `artifact_sha256`
(`contracts/research_decision.v1.schema.json`, the `x-required-when` block).

So the seam is not a design choice to be made here. It is an existing, versioned, hashed,
expiring, identifiable artifact with a stated consumer that does not yet exist. This record proposes
that this repository be that consumer, and that the artifact's own fields be used rather than a
re-projection of them.

### 1.3 The licence, dependency and provenance boundary

The boundary recorded in the earlier review still applies and is not weakened by this one
(`tradingagents_lessons_design.md` section 1.2): the research project is Apache-2.0, this repository
is LGPL-3.0-only, and no source file, test, fixture, prompt text or copied documentation from it may
enter this repository, and no artifact may be derived by mechanical transformation of any of them.

This record therefore describes a *published wire contract* by its field names, which is interface
fact, and copies nothing. It adds no dependency of that project to this one, and it introduces no
second price for any instrument.

### 1.4 Method

Read-only against both trees. Files were read and text was searched; no process was run against
either tree, no engine was instantiated, and nothing was measured. Every claim about the research
half is a claim about its code and artifacts as they are written; every claim about this repository
is a claim about a type, a signature or a doc comment that was read, cited by path. Where a
statement is reasoning rather than a citation, it is marked as inference at the point it is made.

## 2. What already exists

### 2.1 The research half

- A graph of LLM roles over about thirty vendors, ending in a written decision and a JSON document.
- A decision contract with a recomputable hash, an idempotency key, a producer identity, an
  `effective_date`, a `produced_at`, an `expires_at`, and a `risk_gate` carrying a verdict and its
  reasons. The risk gate's own vocabulary is PASS, WARN and REJECT, per the research project's own
  tool documentation; its permission fields carry a two-value allowed or blocked form.
- Its own deterministic guard, which can only weaken a decision and never strengthen it. Its own
  README describes `strategies/decision_guardrail.py` capping a Buy or Overweight at Hold and capping
  confidence on non-fresh data, "downgrade-only by construction"; the earlier review records the same
  property as item L4 (`tradingagents_lessons_design.md` section 8).
- No order routing of any kind. Its own review states it plainly: "The source routes no orders"
  (`tradingagents_lessons_design.md` section 9).
- A backtest that is a daily-bar model with one intrabar path, no queue position and no partial
  fills, which the same section records as strictly behind this repository's matching engine and
  therefore not comparable to it.

### 2.2 The engine half

The three-layer vocabulary is already implemented and already states the discipline this bridge
needs.

| Layer             | Type and home                                                            | What its own documentation says                                                                                                                          |
| ----------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Statement of view | `TradingSignal`, `crates/model/src/signal.rs`                            | "a statement of opinion, not a trading command: it carries no quantity, no price, and no execution instruction"                                          |
| Desired exposure  | `Target` / `TargetValue`, `crates/model/src/target.rs`                   | "A target is not an order and is not a trading command" and "a signal is a statement of view, a target is the exposure that view resolves to"            |
| Minimal order set | `TargetOrder`, from `TargetReconciler` in `crates/trading/src/target.rs` | "a pure function ... returns the minimal order set"; "the cache and the portfolio stay authoritative and a target never becomes a second position store" |

The stages are landed, not aspirational:

- `TargetConstruction` resolves signals into targets, sizing with the one sizing implementation in
  `nautilus_risk::sizing` and deriving the stop price from `TargetConstructionConfig::stop_loss_bps`
  by the signal's direction (`crates/trading/src/target.rs`).
- `TargetReconciler` nets a target against the caller's position and resting orders and emits one
  order per target with a delta worth submitting (`crates/trading/src/target.rs`).
- `TargetPipeline` is the two stages as one value, and it is **opt-in**: "A strategy holds none until
  it enables one, so a strategy that never enables it produces byte-identical behaviour to one that
  predates the pipeline" (`crates/trading/src/target_pipeline.rs`).
- A strategy reaches it through `Strategy.submit_signals(signals)`, which returns the client order
  ids in the order the orders were emitted (`crates/trading/src/strategy/mod.rs`,
  `python/nautilus_trader/trading/__init__.pyi:569`).
- Both stages "read no clock" and mutate neither input, so they are deterministic and unit-testable
  without an engine.
- Parity between the Rust and Python paths is already tested
  (`python/tests/integration/test_target_pipeline_parity.py`, with a regression case at
  `python/tests/regression/cases/target_pipeline_parity.py`).

Beneath that: the risk engine with per-instrument notional caps, count caps and order rate limits
(`crates/risk/src/engine/config.rs`), the execution engine and matching engine, the moomoo adapter
for live routing, `TradingCalendar` for session boundaries (`crates/model/src/calendars/mod.rs`), and
`nautilus-research` as the point-in-time panel owner with a label, a feature compiler and
cross-sectional operators.

### 2.3 The two halves already agree on the invariant

The research project's review in this repository records that its bounded decision packet is defined
as "Not an order, not a recommendation" (`tradingagents_lessons_design.md` section 8, L5). This
repository's `TradingSignal` documentation says the same thing in different words. Neither project
learned it from the other.

That agreement is the load-bearing fact of this design. The bridge does not have to invent the
discipline or impose it on a reluctant producer; it has to *mechanically enforce* an invariant both
sides already state and neither side currently checks across the boundary.

## 3. The projection, stage by stage

The requested chain, mapped onto what exists. "Projection" means a pure function with a typed error;
no stage here reads a clock or a global.

| Stage                  | Research artifact                               | Engine side                           | Status                                                  |
| ---------------------- | ----------------------------------------------- | ------------------------------------- | ------------------------------------------------------- |
| Decision context       | the decision document                           | a `TradingSignal`                     | to project                                              |
| Strategy eligibility   | `risk_gate`, `trade_permission`, `binding_gate` | admission gate plus instrument status | to build, plus one prerequisite                         |
| Position target        | `recommended_allocation_pct`                    | `TargetConstruction`                  | exists; the artifact may only cap                       |
| Portfolio construction | (none)                                          | `TargetReconciler`                    | exists; per instrument, not a cross-sectional optimiser |
| Risk                   | advisory gate verdict                           | `RiskEngineConfig` limits             | exists; to configure                                    |
| Execution and orders   | (none)                                          | `Strategy.submit_signals`             | exists; to make idempotent                              |
| Fills                  | (none)                                          | execution and matching engines        | exists                                                  |

Sections 3.6 to 3.8 are the policies the projection needs; they are not extra stages.

The order of resolution, which is the load-bearing part of the whole design:

```text
                         research_decision.json
                                  |
                                  v
                      schema and hash validation ----- no -----> refused
                                  |
                                 yes
                                  v
                        availability and expiry (3.6)
                                  |
                                  v
                    tradability, positively established (3.2)
                                  |
                                  v
                     gate resolution, deterministic (3.2, I15)
                                  |
                                  v
                    rating and position interpretation (3.6)
                                  |
                                  v
                          projection (3.1, 3.7, 3.8)
                                  |
                                  v
                            TradingSignal
                                  |
                                  v
                          TargetConstruction
                                  |
                                  v
                     allocation ceiling applied (3.7, I13)
                                  |
                                  v
                        TargetReconciler --> RiskEngine --> order --> fill
```

**The first failing stage wins, and later stages are not evaluated.** An artifact that fails two
checks is refused with the reason of the earlier stage in this order, so the refusal reason is a
deterministic function of the artifact rather than of an implementation's evaluation order (I6).

### 3.1 Decision context to signal

A `TradingSignal` carries exactly a statement of view: instrument, direction, horizon, magnitude,
source, expiry and provenance
(`crates/model/src/signal.rs`). The decision document carries a ticker, a direction or rating, a
confidence, a horizon, a producer, an expiry and a run identity. The projection is close to total,
and each field that does not map directly is settled in section 7 rather than defaulted here.

One property of the projection is a rule rather than a mapping: the research decision is **necessary
in this strategy but never sufficient by itself**. A signal exists only when the artifact is valid,
actionable, positively tradable and confluent under the gate rule, and when the engine's own
eligibility passes. The artifact is what makes the flow worth running; it is never what authorises it.
That is invariant I2 in section 4, and the policies that implement it are sections 3.6 to 3.8.

### 3.2 Eligibility is the only place permission can be granted, and it cannot be

Eligibility asks whether this instrument, now, may be acted on at all. It has three inputs:

1. **Engine facts, authoritative.** Instrument status and tradability (`MarketStatusAction` and
   `InstrumentStatus`, `crates/model/src/enums.rs:978`, `crates/model/src/data/status.rs`), the
   instrument definition, and portfolio state.
2. **The artifact's own claims.** `risk_gate.verdict`, `trade_permission`, `binding_gate`, which are
   advisory by the producer's own statement.
3. **The regime reading.** Either the research half's own regime score, or the measured regime from
   `implementation/sector-regime-engine/`, which carries a pre-registered decision rule and
   therefore a falsifiable null.

#### Tradability is positively established, never merely not-denied

The engine's answer is three-valued, not two-valued: tradable, not tradable, or unknown. Unknown is
not tradable. A requirement of the form "status is not a rejection" is insufficient, because it
admits the unknown case, and an instrument whose venue or instrument class the reader does not
recognise is exactly the case that must not reach construction. The requirement is per leg, not per
strategy: a multi-leg instrument's every leg is established separately, because a spread whose second
leg cannot trade is not tradable (I14).

#### The three research-side gates resolve deterministically

The artifact carries three fields that each claim something about permission, and they can
contradict each other. Two implementations reading the same artifact must not reach different
answers, so the aggregation is a declared rule rather than an implementation's judgement.

Each field is mapped to one of three severities by a declared, closed mapping:

| Severity    | Meaning for the bridge                       |
| ----------- | -------------------------------------------- |
| `PERMIT`    | the field raises no objection                |
| `UNCERTAIN` | the field permits the action at reduced risk |
| `RESTRICT`  | the field refuses the action                 |

The rule is then:

```text
research_severity  = the most restrictive severity of the three fields
effective_gate     = min(research_severity, engine_eligibility)      # engine wins
```

Three properties make it deterministic. A permissive field never overrides a stricter field, because
the aggregation is a maximum over severity rather than a first-match or a caller's choice. A value
that is not in the declared mapping resolves to `RESTRICT`, because the safe direction of an unknown
value is closed. And engine eligibility is evaluated separately and cannot be raised by anything
research-side, so a research `PERMIT` beside an engine refusal still produces no signal.

A disagreement among the three fields is recorded as the diagnostic `GATE_CONFLICT`, with the fields
and their values, while the effective gate remains the most restrictive resolution. A conflict is a
diagnostic attribute rather than a terminal state, because the restrictive resolution is already a
correct answer and the conflict is information about the producer rather than a reason to stop. The
same diagnostic is recorded when two artifact fields disagree in any other way that the policies of
section 3.6 resolve restrictively.

#### The regime disposition is three-valued, and only one branch closes the gate

| Disposition | Effect on eligibility | Effect on risk              |
| ----------- | --------------------- | --------------------------- |
| `PASS`      | unchanged             | the configured risk applies |
| `UNCERTAIN` | unchanged             | a configured reduction      |
| `REJECT`    | closes                | no order exists             |

`UNCERTAIN` admits, provided every independent engine gate passes, and reduces risk by a configured
factor. It is not treated as a rejection: the research half saying "I cannot establish a directional
regime" is not the same statement as "this trade should not exist", and rejecting on it would hand
the research half the authority to close a gate (I2). The reduction factor is configuration, and
section 6 makes measuring it part of the work rather than an assumption.

### 3.3 Position target

`TargetConstruction` already does this stage, and its behaviour is documented and pinned: a
directional signal is sized by fixed-risk sizing from a context entry price and a configured stop
distance, the signal's strength scales the risk and is clamped to the unit interval, and the
resulting weight is capped by `max_weight`. The artifact's `recommended_allocation_pct` is not an
input to that calculation. Under I2 and I13 it may only cap the result (section 3.7).

**The research stop stays advisory in revision 1.** The construction stage derives its stop from
`TargetConstructionConfig::stop_loss_bps`; the artifact carries advisory entry, stop and target
levels. Adopting the artifact's stop would be adopting a size, not a level: fixed-risk sizing is the
risk budget divided by the stop distance, so a research-supplied stop distance *is* a
research-supplied position size, which is precisely what I2 forbids. The levels therefore travel as
advisory metadata in the signal's provenance. A bounded tightening policy, in which a validated
research stop may only tighten the engine's stop and never loosen it, is recorded as future work in
section 8 and is not part of this design.

One property of the existing stage is worth stating because it removes a whole class of bridge
defect: degenerate inputs produce a typed error rather than a silent omission, for the documented
reason that "an omission would also turn a directional signal whose size floors to zero into a flat
target, which instructs the opposite of the signal"
(`crates/trading/src/target.rs`, section "Degenerate inputs"). A bridge that drops an unsizeable
decision would therefore be reversing the decision, not ignoring it.

### 3.4 Portfolio construction: what exists, and what does not

`TargetReconciler` reconciles targets against an authoritative snapshot and returns the minimal
order set. That is per-instrument reconciliation with portfolio-aware netting, not a cross-sectional
portfolio optimiser: there is no component in this repository that takes N desired exposures and
returns N weights that sum to a gross target under a correlation model, and this record does not
propose one. The honest boundary is:

- Construction is per signal. Cross-asset effects enter only through context equity and through the
  risk engine's caps.
- Gross and net exposure limits therefore belong in `RiskEngineConfig`, not in a constructor.
- Any cross-sectional scaling must not be built from `nautilus-research` operators: that crate's own
  documentation states the factor pipeline "is authoritative in the research pipeline and nowhere
  else, and execution stays on the normal strategy and execution path"
  (`crates/research/src/lib.rs`).

### 3.5 Risk, execution, orders, fills

These are the stages this repository already owns and the reasons this half of the chain is worth
having. The bridge's only obligations are to configure the risk engine so that a decision cannot
exceed a limit, to make submission idempotent, and to keep the audit chain from a decision to a fill.

### 3.6 The actionability policy: a rating is not a direction, and a date is not an instant

Two distinct policies live here, and both exist because the artifact and the engine answer different
questions.

#### A rating is not a direction, and the resolution is total

`BUY`, `HOLD` and `SELL` are not a one-to-one encoding of `LONG`, `FLAT` and `SHORT`, because the
interpretation of a view depends on the position that already exists. The policy is total over the
rating vocabulary, closed on an unknown rating, and it states its output for every row rather than
leaving an implementer to invent one:

| Research view | Position | Engine result                                                                                       |
| ------------- | -------- | --------------------------------------------------------------------------------------------------- |
| `BUY`         | flat     | a `LONG` signal, with the artifact's allocation as the ceiling                                      |
| `BUY`         | long     | a `LONG` signal, with the same ceiling; the reconciler emits the delta only if exposure is below it |
| `HOLD`        | flat     | no signal, recorded as `NO_SIGNAL`: the artifact states no view and there is nothing to maintain    |
| `HOLD`        | long     | no signal, recorded as `NO_SIGNAL`: the engine's own reconciliation already maintains an exposure   |
| `SELL`        | long     | a flat signal, meaning a target of zero exposure; the reconciler emits the minimal reduction        |
| `SELL`        | flat     | no signal, recorded as `NO_SIGNAL`: revision 1 is long-only                                         |

Three consequences are deliberate. `HOLD` produces no signal at all rather than a directional signal
carrying a "maintain" flag, because this repository's `TradingSignal` has no such flag and inventing
one would change a platform type for a bridge policy; a `HOLD` therefore cannot be used to *cancel*
or enlarge an existing target, which is the correct reading of a view that expresses nothing. A
`SELL` on a long position resolves to a **zero target**, not to a partial reduction, because a
partial exit needs a specification the artifact does not carry, and letting `SELL` imply an arbitrary
percentage would be the bridge inventing a position size. A non-zero allocation beside a `SELL` is
therefore not consumed, and that disagreement between two artifact fields is recorded under the
`GATE_CONFLICT` diagnostic while the target remains zero, which is the restrictive resolution.

**Revision 1 is long-only.** A `SELL` on a flat instrument produces no signal, and the branch that
would permit a short is closed rather than left to a configuration flag. A long-short policy is
future work (section 8), and when it exists it must be an explicit policy, not an artifact field
whose value happens to be unrecognised.

**The bridge never invents a direction.** A `BUY` is never resolved to a flat or a short target by
any policy, a `SELL` is never resolved to a long, and the only way a view changes direction is by the
rating itself. That is invariant I2's directional form.

#### A date is not an instant, and an expiry is required

The artifact's `effective_date` is a date, its `produced_at` is an instant, and its `expires_at` is an
instant. A fourth instant is bridge-side rather than artifact-side: `received_at`, the moment this
engine admitted the artifact. It is not trusted from the artifact, because a producer cannot know when
its output will be read, and it is what stops the bridge from creating a signal whose actionability
already lies in the past.

The bridge resolves an `actionable_at` from the engine's own session data rather than from a bespoke
rule. `TradingCalendar` already answers exactly this question
(`crates/model/src/calendars/mod.rs`):

- `is_tradeable(ts)` answers whether a given instant is tradeable.
- `next_open(ts)` and `prev_close(ts)` answer what the next and previous session boundaries are.
- `sessions_on(date)`, `is_trading_day(date)`, `is_holiday(date)` and `early_close(date)` answer the
  session shape for a date.
- `covers(ts)` answers whether the calendar's data covers the instant at all, and
  `warn_if_coverage_ends_before` exists because coverage is finite.
- Calendars are keyed by `CalendarKey { venue, asset_class, symbol }`, which is what makes an equity
  calendar resolvable per listing, and they are bundled and immutable: the module states they are
  "loaded once, validated on load, and never mutated while the run is in progress".

The resolution rule, in order:

```text
1. calendar does not cover produced_at --------> refuse (CALENDAR_UNCOVERED)
2. expires_at absent --------------------------> refuse (MISSING_EXPIRY)
3. produced_at >= expires_at ------------------> refuse (ARTIFACT_EXPIRED)
4. calendar_actionable =
       produced_at                     when is_tradeable(produced_at)
       next_open(produced_at)          otherwise
   and no next open exists --------------------> refuse (ACTIONABILITY_INVALID)
5. calendar_actionable < received_at ----------> record ACTIONABILITY_PAST,
                                                 actionable_at = received_at
6. actionable_at >= expires_at ----------------> refuse (ARTIFACT_EXPIRED)
```

Four consequences are worth stating plainly.

**No retroactive action.** Step 5 is the rule that the owner's review called for: the effective
actionability instant is never earlier than the instant the engine received the artifact. The clamp
is recorded as a diagnostic rather than a refusal, because the artifact is not at fault; the *engine*
arrived late, and that is information about the pipeline.

**Expiry is exclusive.** An artifact is valid while `t < expires_at`, so an instant equal to the
expiry is refused (step 6 uses `>=`). The producer's field is documented as the instant at which the
artifact ceases to be actionable, and a comparison of the other sense would let an artifact survive
exactly at the instant it stops being usable.

**An expiry is never derived from the horizon.** A horizon is semantic and an expiry is operational,
and silently converting one into the other would let a vocabulary word decide when an artifact dies.
The producer's own contract already notes that a session's last hour is a window an artifact must not
be actionable after, which is the same rule seen from the producer's side.

**Availability is resolved before reference time, never after it.** That is invariant I3. The
calendar module's own division of responsibility is preserved: it states that instrument lifetime
belongs to the instrument and that "a calendar only answers whether an instant is tradeable, and when
the sessions are".

### 3.7 The advisory allocation is a ceiling, not an instruction, and it is validated

The artifact's `recommended_allocation_pct` is an opinion about how much exposure a view deserves. It
has a declared domain: zero to one hundred inclusive, which is also the domain the producer declares
on its own field, and it is normalised to a fraction exactly once at admission. Every value outside
that domain is refused with `ALLOCATION_INVALID` rather than clamped: a negative allocation must not
be allowed to interact with exit logic, and a value above the maximum must not silently become the
maximum, because clamping would mask a producer contract violation as a legitimate instruction. Not a
number, infinite, and not finite are the same refusal.

Within the domain, the engine's exposure is the result of sizing from engine prices against engine
equity, bounded by that opinion and by the risk engine. The direction of resolution is therefore:

```text
research allocation
        |
        v
advisory ceiling
        |
        v
engine target construction
        |
        v
portfolio-aware reconciliation --> minimal order set
```

and never:

```text
research allocation ----------> order quantity
```

The worked case is the one the owner's review raises. The artifact says `BUY` at an allocation of
five percent while the engine already holds four percent of the instrument's cap. The answer is not a
five percent order and not a target of five percent of equity; it is the minimal delta from the
current exposure to the engine's constructed target, with the research allocation acting as a ceiling
on that target. This is invariant I13, and it is the reason the artifact cannot be read as a
portfolio instruction even when it looks like one.

An allocation of zero beside a directional rating resolves to no signal rather than to a refusal,
because zero is inside the domain: the artifact sanctions no exposure, and there is nothing to
construct. The contradiction between a direction and a zero ceiling is recorded under the
`GATE_CONFLICT` diagnostic.

### 3.8 Strength carries the bridge's policy magnitude, never the model's confidence

Two settled decisions meet in one field, and the meeting point is worth writing down because it is
not obvious from either decision alone. `TargetConstruction` scales risk by the signal's `strength`,
clamped to the unit interval, with absent meaning the configured per-trade risk. Confidence must not
be projected onto strength (D2). The regime disposition needs a magnitude (D5, section 3.2).

The consequence, derived here rather than cited: `strength` is the field that carries the bridge's
*own policy multiplier*, and the artifact is never read for it. A `PERMIT` disposition contributes a
factor of one, which is equivalent to leaving strength absent, and an `UNCERTAIN` disposition
contributes the configured reduction. The model's `confidence` is never converted, never clamped and
never used for sizing; it travels as research metadata and is evaluated through calibration in
section 6.

The reason this matters is not pedantry. The two quantities are documented as different kinds of
thing: the signal states that strength "is a magnitude, not a probability: it is not bounded by one
and does not sum to one", while the artifact's confidence is the model's self-assessed probability of
its own rating. Equating them would make a confident flat view and a timid strong view
indistinguishable, and would silently convert a probability into a position size. If
confidence-informed sizing is wanted later, the defensible form is a calibrated function mapping a
confidence, a horizon and a regime onto an empirical probability, and then onto a bounded modifier,
which is recorded as future work in section 8.

## 4. Invariants

**I1. One authority per instrument.** No price, quantity or money value is ever constructed from the
decision document. The artifact's prices are annotations; the engine's prices come from instruments,
quotes and fills. This repository's rule is unchanged: one authoritative source per instrument.

**I2. Advisory inputs are monotone-reducing, and never sufficient.** Research-derived fields may only
constrain an independently valid engine decision. They may not create, enlarge or authorise an
otherwise invalid decision. Stated as properties rather than as prose, so that a property test can
assert them: for a long exposure the effective target is never greater than the engine's constructed
target, and the effective risk is never greater than the engine's configured risk budget; the
directional equivalent is that the resolved direction is the rating's own, never one the bridge chose.
Consequences: research may **cap** a size, may **reject** an action, and may **reduce** risk; it may
not invent a direction, and it may not flatten a view it did not itself state as flat. In this
strategy the artifact is necessary, because it is what makes a decision to act arise at all; it is
never sufficient, because a signal exists only when the artifact is valid, actionable, positively
tradable and confluent, and the engine's own eligibility passes.

**I3. Four instants, and availability decides first.** The artifact carries an `effective_date` (the
reference date of the analysis), a `produced_at` (when the artifact existed) and an `expires_at` (when
it ceases to be actionable); the bridge holds a fourth, `received_at`, which it does not trust from
the artifact. Availability is resolved before reference time, always. `expires_at` is required, is
exclusive (valid while `t < expires_at`), and is never derived from a semantic horizon.
`actionable_at` is resolved from the instrument's own trading calendar and is never earlier than
`received_at`, so no action can be retroactive (section 3.6). A backtest that timestamps a record at
its `effective_date` grants the strategy every hour between midnight and production; the distinction
is not a local convention. Separating a datum's availability from its reference time is the
disciplining move that makes look-ahead freedom checkable at all
([arXiv 2607.04958](https://arxiv.org/abs/2607.04958)), and `TradingSignal` already carries the field
for the result of that resolution, `expiry_ns`.

**I4. No lookahead by construction, not by convention.** `Panel::new` runs `Panel::check`, which
refuses a feature whose `as_of` is after its row's event time with `PanelError::Lookahead`
(`crates/research/src/panel.rs:150,176`), and refuses a row whose membership disagrees with the
stored series. A decision record used as a research feature must be projected into that aperture; a
panel built from records timestamped at their reference date rather than their availability cannot
be constructed at all.

**I5. The gate is enforced, not read.** A gate verdict in a JSON document is a claim. The enforceable
form is a risk-engine limit and a typed denial. Where the resolved gate says `RESTRICT`, the
requirement is that no order exists, not that the document said so.

**I6. Typed refusals, never silent omission, with a deterministic reason.** A record that cannot be
validated, sized or admitted produces a named refusal that travels with the result, drawn from the
closed vocabulary in section 5.2. The precedent is already in the code this bridge calls:
`TargetConstructionError`'s variants, and `OrderDeniedReason` in the order layer. A dropped record
must be distinguishable from a record that carried no view, and an artifact that fails several stages
is refused with the reason of the first failing stage in the declared resolution order, so the reason
is a function of the artifact and not of an implementation's evaluation order.

**I7. The float boundary is declared and checked, and only where execution needs it.** The research
half works in floating point throughout; this repository's prices, quantities and money are fixed
point. Every **execution-bearing** numeric field that crosses must be converted at a declared scale,
and a value that does not fit must be refused rather than rounded to fit, by the producer's own
standard that a bound met by cutting a cell in half is not a bound. Execution-bearing means a price, a
quantity, a money amount, a stop distance, or an allocation that will bound exposure. Research
metadata, meaning scores, coverage, sentiment and the model's confidence, is **not** converted: it has
no execution purpose, it is consumed by measurement rather than by the order layer, and transforming
it into an execution type would be a numerical change with no consumer.

**I8. Determinism is preserved where it exists and recorded where it does not.** The construction and
reconciliation stages read no clock and are pure. The research half is not reproducible and says so.
The bridge must not import non-determinism into a deterministic layer: the projection is a pure
function of the artifact, the configuration, the calendar and the engine's own state.

**I9. Order identity is idempotent, per decision, per role, and immutable.** A decision produces at
most one order set, ever, across restarts, re-reads and retries. The identity of an order is derived
from the artifact's own key plus what distinguishes one order from another within it: the instrument,
the leg and the order's role. The derivation is a versioned, domain-separated hash, and the scheme
version is recorded with the order (section 7, D7). The identity is immutable for the lifetime of the
decision: a venue rejection, a cancellation, or a lost acknowledgement does not free it, so a replay
cannot produce a second order, and a deliberate retry requires a new execution-attempt identity
derived from an explicit retry policy. The reason for the strictness is that a lost acknowledgement
after venue acceptance is indistinguishable, from the engine's side, from a rejection.

**I10. Every fill traces to the decision that caused it.** The audit chain is decision to order to
fill, queryable in both directions. This is what makes the measurement in section 6 possible at all,
and it is what the producer cannot build because it routes no orders.

**I11. Coverage travels with a score.** The research half's score kernel withholds a composite below
a coverage floor rather than substituting a neutral value (`tradingagents_lessons_design.md` section
3). A score that arrives without its coverage is a different object from the same score with it, and
the bridge must not make them indistinguishable.

**I12. Producer identity is not authorization, and an unidentified result is not evidence.** A
producer that is identified but not authorised is refused with `PRODUCER_UNAUTHORIZED`, which is a
different outcome from `PRODUCER_UNKNOWN`: the first means the bridge knows who spoke and does not
trust them, the second means the bridge cannot tell who spoke. A record whose producer identity is
unknown is excluded from any aggregate rather than pooled, which is the rule this repository already
accepted as L13 and has not implemented.

**I13. A research allocation is a ceiling, not an instruction.** The artifact's allocation is an
advisory bound on the engine's constructed target, inside a validated domain, normalised once and
never clamped, and an order is always the minimal delta from current exposure to that target, resolved
against portfolio state (section 3.7).

**I14. Tradability is positively established, per leg.** No instrument reaches construction unless
the engine has positively answered that it is tradeable. The answer is three-valued, unknown is not
tradable, and every leg of a multi-leg instrument is established separately. An unknown or unmapped
status produces a typed refusal (section 3.2).

**I15. Gate aggregation is deterministic and restrictive.** The artifact's permission fields resolve
to one severity through a declared, closed mapping; the most restrictive value wins; a permissive
field never overrides a stricter field; a value outside the mapping resolves to rejection; and engine
eligibility is evaluated independently and cannot be raised by anything research-side (section 3.2).

## 5. Failure modes, and which invariant answers each

| Failure                                                           | Evidence in the two trees                                                                                                                        | Answered by     |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | --------------- |
| The same ticker is analysed twice and both records are admitted   | Retry artifacts for the same symbol exist beside the originals in the research checkout                                                          | I9              |
| A backtest reads a decision before it existed                     | `effective_date` is a date; production is a timestamp; the two differ by hours                                                                   | I3, I4          |
| A signal is actioned before the engine received it                | A retroactively resolved instant is executable unless receipt bounds it                                                                          | I3, step 5      |
| Two permission fields disagree and two readers differ             | Three advisory fields with independent vocabularies                                                                                              | I15             |
| A gate verdict is quoted instead of enforced                      | PASS can coexist with a block on new risk in the same record                                                                                     | I2, I5          |
| An unsizeable view becomes a flat target                          | Documented as the reason the construction stage reports typed errors                                                                             | I6              |
| An unknown market status reads as permission                      | The status vocabulary is wider than any reader's match arms                                                                                      | I14             |
| A research allocation is executed as absolute size                | An allocation is a proportion of a book the engine owns, not an order quantity                                                                   | I13             |
| A negative or oversized allocation is clamped into validity       | Clamping a contract violation produces a plausible instruction from an invalid one                                                               | I13             |
| A `SELL` becomes a short in a long-biased book                    | The rating vocabulary is not a direction vocabulary                                                                                              | I2, section 3.6 |
| A `SELL` implies an arbitrary partial reduction                   | The artifact carries no partial-exit field                                                                                                       | section 3.6     |
| A `HOLD` is read as an instruction to change exposure             | The artifact states no view, so there is nothing to resolve                                                                                      | section 3.6     |
| A self-reported confidence becomes a risk multiplier              | Confidence is a probability and strength is a magnitude; the two are documented as different                                                     | I2, section 3.8 |
| A venue rejection frees an order identity and a replay re-submits | A lost acknowledgement is indistinguishable from a rejection                                                                                     | I9              |
| A known but unauthorised producer is treated as trusted           | Identity and authorization are different properties                                                                                              | I12             |
| A score without coverage reads as a neutral score                 | The producer withholds below a coverage floor and records coverage beside the score                                                              | I11             |
| Nine scores are tested as nine independent hypotheses             | A scorecard carries nine scores per name; a panel multiplies them by symbols and days                                                            | section 6       |
| An LLM thesis encodes the future                                  | Measured externally: back-tested returns evaporate once the model's knowledge window ends ([arXiv 2510.07920](https://arxiv.org/abs/2510.07920)) | section 6       |
| Currency drift between schema versions loses fields silently      | The contract ignores unknown fields and adds required ones at a version boundary                                                                 | I6, I12         |

The last two are not defects in the bridge and the bridge cannot fix them. They are the reason the
measurement in section 6 is part of the design rather than an afterthought: an artifact stream from a
model window has to be *measured* out of sample, because it cannot be *made* clean.

### 5.1 The lifecycle, and why a refusal is an outcome

A refusal is not a discarded record. It is the most informative part of the stream, because the
question "what fraction of research decisions never became executable, and why" is answerable only if
the failures are kept. Every branch below stays queryable, and the outcomes that are not a fill are
distinguished from each other rather than collapsed:

```text
RESEARCH_PRODUCED
       |
       v
   VALIDATED -------- no -------> REFUSED (with a reason, kept)
       |
      yes
       v
   ADMITTED -------- no-op -----> DUPLICATE (recorded, no signal)
       |
       v
  ACTIONABLE ------- no --------> NO_SIGNAL (policy declined, kept)
       |
       v
   SIGNAL --------- no ---------> NO_SIGNAL (rating resolves to none, kept)
       |
       v
   TARGET
       |
       v
   RISK ------------ denied -----> DENIED (typed, kept)
       |
       v
   ORDER -----------------------> partial, rejected, cancelled, filled
       |
       v
   FILL
```

The distinctions that matter, kept apart by construction: no decision produced at all; a decision
produced and refused by the engine; a decision admitted that yielded no order because the policy
declined it; a decision admitted that yielded no order because the rating resolved to none; an order
denied by risk; and an order that reached the venue. Pooling any two of those is how a desk loses the
ability to tell a broken pipeline from a cautious one.

### 5.2 Refusal codes and recorded diagnostics

One closed vocabulary, declared once and shared by the ledger and the execution path, so that a reader
can classify an outcome without reading a message. It has two parts, and the distinction is
deliberate: a **refusal** stops the artifact, and a **diagnostic** records something worth knowing
about an artifact that continued. The codes are grouped by the stage that produces them, which is also
the order they can occur in.

| Stage        | Refusal code                    |
| ------------ | ------------------------------- |
| Admission    | `SCHEMA_INVALID`                |
| Admission    | `HASH_INVALID`                  |
| Admission    | `PRODUCER_UNKNOWN`              |
| Admission    | `PRODUCER_UNAUTHORIZED`         |
| Admission    | `INSTRUMENT_UNKNOWN`            |
| Admission    | `ALLOCATION_INVALID`            |
| Admission    | `DUPLICATE`                     |
| Availability | `MISSING_EXPIRY`                |
| Availability | `ARTIFACT_EXPIRED`              |
| Availability | `ACTIONABILITY_INVALID`         |
| Availability | `CALENDAR_UNCOVERED` (addition) |
| Tradability  | `TRADABILITY_UNKNOWN`           |
| Tradability  | `TRADABILITY_REJECTED`          |
| Eligibility  | `RISK_GATE_REJECT`              |
| Projection   | `RATING_UNKNOWN` (addition)     |
| Projection   | `UNSIZEABLE`                    |
| Projection   | `FLOAT_CONVERSION_OVERFLOW`     |
| Execution    | `ENGINE_RISK_LIMIT`             |

| Stage        | Diagnostic attribute |
| ------------ | -------------------- |
| Eligibility  | `GATE_CONFLICT`      |
| Availability | `ACTIONABILITY_PAST` |

Four notes on the set. `DUPLICATE` is an outcome rather than a failure: it is recorded with its
reason and produces no signal, which is what makes a repeated analysis a no-op rather than an error.
`ENGINE_RISK_LIMIT` is produced after admission, by the risk engine, and is included because the
lifecycle is one vocabulary even though two components speak it. `GATE_CONFLICT` and
`ACTIONABILITY_PAST` are attributes rather than refusals, because in both cases there is a correct
restrictive resolution and the diagnostic is information about the producer or the pipeline rather
than a reason to stop. `CALENDAR_UNCOVERED` and `RATING_UNKNOWN` are additions to the owner's set,
each marked, and each required by a resolution rule that can fail on its own: a finite calendar can
fail to cover an instant, and a closed rating vocabulary can receive a value outside it.

## 6. Measurement is the other half of the join

The engine half is worth having for two things: execution, and knowing what the decisions were worth.
The second is only available here, because the research half has no fills.

### 6.1 Three experiments, kept apart

Poor performance has several different causes, and attributing it to the wrong one is how a desk
either abandons a working signal or keeps a broken policy. The three experiments are therefore
separate, and the architecture already makes them separable:

| Experiment                 | The transformation measured                                           |
| -------------------------- | --------------------------------------------------------------------- |
| A. Research signal quality | the research decision against a forward return, with no bridge        |
| B. Bridge policy effect    | the research decision against the eligible signals the policy yields  |
| C. Execution realization   | an eligible signal against its target, its risk decision and its fill |

A strong information coefficient with weak realised profit is then a finding about B or C rather than
about A, and each is measured where it happens.

### 6.2 Signal quality

A `Panel` over (instrument, day) rows with the score features and a forward `Label`, then the
cross-sectional operators for an information coefficient series per score. The `label` module already
carries a horizon and a stated terminal convention, and `Panel` already refuses the two mistakes that
flatter this measurement: a feature that reads the future and a row whose membership is wrong.

### 6.3 Confidence calibration, and the reduction factor as a hypothesis

`confidence` bucketed against realised hit rate, per rating and per horizon. This is where confidence
is *used*, under D2: not converted into a size, but measured.

The `UNCERTAIN` reduction factor is a hypothesis, not a fact, and it is measured across the metric set
rather than on one number: expectancy, Sharpe, hit rate, maximum drawdown and CVaR, for the admitted
population under `PASS` and under `UNCERTAIN`, with the differences between them reported. A
reduction is justified only if the difference is favourable in risk-adjusted terms; three outcomes
are all legitimate findings, including "no meaningful difference", and the design does not presume
which one will hold.

### 6.4 Redundancy and regime conditioning

The correlation structure of the score set, to see how many independent hypotheses are really being
tested, because nine scores invite nine tests. And the same measurements conditioned on a regime
verdict, with `implementation/sector-regime-engine/` supplying a null model: if the measured regime is
memoryless, a large out-of-sample information coefficient is evidence of leakage rather than evidence
of skill.

### 6.5 Bridge attribution: where the exposure disappeared

For every decision, five exposure quantities are recorded, and the differences between them localise
the loss:

```text
research requested allocation
        |  bridge ceiling (I13)
        v
bridge capped allocation
        |  engine construction (3.3)
        v
engine constructed allocation
        |  risk engine (I5)
        v
risk approved allocation
        |  execution
        v
actually filled allocation
```

Worked example:

```text
research requested   8.0%
bridge capped        8.0%
engine constructed   5.0%     engine sizing
risk approved        3.0%     risk constraint
actually filled      2.7%     execution realization
```

Each step is a measurement, not a log line: the sequence answers "where did the exposure disappear"
without re-deriving the path, and it is what distinguishes a sizing problem from a risk constraint
from a fill shortfall.

An `Label` and a `Panel` are enough for the signal-quality measurements; the attribution quantities
are the audit record's own fields. Nothing in this section requires a new mechanism, which is why it
is in the design and not in the implementation plan beyond the wiring.

## 7. Owner decisions

All eight decisions are settled. The table is the disposition, and each note records what it changes
in this record and what remains open about it.

| Decision | Disposition                                                                                                                                                                         |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1       | Custom-data carriage, pure projection at the consumer boundary, admission ledger persisted. No new bridge crate in revision 1                                                       |
| D2       | Confidence is never mapped to strength; it remains research metadata and is evaluated through calibration                                                                           |
| D3       | `actionable_at` resolved from production and market-calendar availability, bounded below by receipt; explicit `expires_at` required and exclusive; no expiry derived from horizon   |
| D4       | Research stop, entry and exit levels remain advisory; the engine-configured stop stays authoritative                                                                                |
| D5       | `UNCERTAIN` admits only when independent engine eligibility passes, with a configured risk reduction; `REJECT` closes eligibility                                                   |
| D6       | Positive, three-valued, per-leg tradability; unknown or unmapped status produces a typed refusal                                                                                    |
| D7       | Deterministic, versioned, domain-separated client order ids from the idempotency key plus instrument, leg and order role; the scheme version is recorded; the identity is immutable |
| D8       | Every refused artifact is persisted with a structured reason and full provenance                                                                                                    |

### 7.1 Corrections applied in revision 3

The owner's second review raised boundary semantics rather than architecture, and asked for five
corrections before implementation and five recommendations. None of them changed a decision; all of
them closed an ambiguity. Each is recorded with where it landed.

| Correction                                                                                       | Where it landed                      |
| ------------------------------------------------------------------------------------------------ | ------------------------------------ |
| Gate precedence among the three research-side fields                                             | `3.2` and **I15**                    |
| `HOLD` and `SELL` semantics made total and explicit                                              | `3.6`, `5.1`                         |
| Receipt time added, retroactive action forbidden                                                 | `3.6` step 5, **I3**                 |
| Expiry made exclusive (`actionable_at >= expires_at` refuses)                                    | `3.6` step 6, **I3**                 |
| Allocation domain validated, never clamped                                                       | `3.7`, **I13**, `ALLOCATION_INVALID` |
| `SELL` on a long position resolves to a zero target                                              | `3.6`                                |
| Order identity immutable after a venue rejection                                                 | **I9**                               |
| Producer authorization separated from producer identity                                          | **I12**, `PRODUCER_UNAUTHORIZED`     |
| Bridge attribution quantities recorded per decision                                              | `6.5`                                |
| Acceptance matrix expanded with the temporal, gate, `HOLD`, allocation-boundary and replay cases | `9`                                  |

Three further tightenings from the same review are also applied: the monotonicity rule is stated as
assertable properties rather than as prose (I2), the float boundary is scoped to execution-bearing
fields only so research metadata is not needlessly transformed (I7), and revision 1 is stated as
long-only with the shorting branch closed rather than left to a flag (`3.6`, section 8).

## 8. Non-goals

- No change to either half's internals. The research project is not asked to change its contract, and
  this repository's signal, target, pipeline, calendar and risk types are not asked to change their
  meaning.
- No cross-sectional portfolio optimiser. Section 3.4 records why.
- **Long-only in revision 1.** A shorting policy is future work, and until it exists a `SELL` on a
  flat instrument produces no signal rather than a short.
- No partial exit from a `SELL`. The artifact carries no partial-exit field, so a `SELL` resolves to a
  zero target and the reconciler emits the minimal reduction.
- No second price, no second position store, no new order path.
- No LLM in the engine half. The bridge reads a document; it consults no model.
- No attempt to make the research half reproducible. It says it cannot be, and the measurement in
  section 6 is the answer to that rather than a fix for it.
- No claim that any number produced by the research half's own backtest is comparable to a number
  produced here.
- No bounded stop-tightening policy in revision 1 (section 3.3), and no calibrated
  confidence-to-size function (section 3.8). Both are recorded as future work with the condition for
  admitting them: their own configuration, their own measurement, and no capacity to loosen what the
  engine owns.

## 9. How you would know it worked

Each test corresponds to an invariant rather than to a feature.

| Test                                                    | Expected                                                                                                    |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Re-pushed duplicate                                     | no second order, `DUPLICATE` recorded                                                                       |
| `expires_at == actionable_at`                           | refused, `ARTIFACT_EXPIRED`                                                                                 |
| `produced_at > expires_at`                              | refused, `ARTIFACT_EXPIRED`                                                                                 |
| `received_at > expires_at`                              | refused, `ARTIFACT_EXPIRED`                                                                                 |
| `actionable_at < received_at`                           | clamped to receipt, `ACTIONABILITY_PAST` recorded                                                           |
| Calendar does not cover the instant                     | refused, `CALENDAR_UNCOVERED`                                                                               |
| Missing expiry                                          | refused, `MISSING_EXPIRY`                                                                                   |
| Gate fields disagree (`PASS` beside a restriction)      | deterministic restrictive result, `GATE_CONFLICT` recorded                                                  |
| Unmapped gate value                                     | refused, restrictive resolution                                                                             |
| Engine eligibility refuses while research permits       | no order                                                                                                    |
| `BUY` with allocation `0`                               | no signal, recorded                                                                                         |
| Allocation negative, above the maximum, or not a number | refused, `ALLOCATION_INVALID`                                                                               |
| `HOLD` on a long position                               | no signal, `NO_SIGNAL` recorded, exposure unchanged                                                         |
| `SELL` on a long position                               | target zero, reconciler emits the minimal reduction                                                         |
| `SELL` on a flat position                               | no signal, recorded                                                                                         |
| Unrecognised rating                                     | refused, `RATING_UNKNOWN`                                                                                   |
| Unsizeable view                                         | named refusal, not a flat target                                                                            |
| Tradability unknown, or a second leg unknown            | no order, typed refusal                                                                                     |
| Venue rejects an order, then the artifact is replayed   | no second order                                                                                             |
| Two orders differing only by role, or by instrument     | different ids, same scheme version                                                                          |
| `BUY` above the current exposure                        | minimal delta to the engine target, never the advised amount                                                |
| Attribution chain                                       | five exposures recorded per decision, differences localised                                                 |
| Fill resolution                                         | every fill resolves to one decision id                                                                      |
| Panel with a future-dated feature                       | cannot be constructed                                                                                       |
| Measurement                                             | information coefficient with uncertainty, calibration by bucket, and the reduction factor's measured effect |

## 10. Implementation gates

The design is closed, but the record must not be read as "ready to code without further decisions".
Five gates come first, in this order, and each is a test rather than a statement:

```text
design closed
    |
    v
D6 current-code verification      - does the status mapping satisfy I14?
    |
    v
gate precedence test              - the aggregation of I15 over its vocabulary
    |
    v
HOLD and SELL mapping test        - every row of the 3.6 table
    |
    v
temporal edge-case tests          - the six cases of the 3.6 resolution rule
    |
    v
idempotency replay test           - including the post-rejection replay
    |
    v
implementation
```

The first gate is the only one that is not a test of new code: whether the current execution status
mapping positively identifies tradability, per leg, is an open prerequisite recorded in Appendix A,
and an implementer must establish it before relying on I14.

## Appendix A. Provenance of this record

Revision 1 was written read-only against both trees, and reduced the design to the projection after
finding the engine-side stages already implemented. Revision 2 applied the owner's disposition of
all eight decisions, which modified D3, D5 and D7, strengthened D6 with the three-valued tradability
requirement, added I13 and I14, and produced the lifecycle, the refusal vocabulary and the
attribution of the allocation as a ceiling. Revision 3 applied the owner's second review, which raised
boundary semantics rather than architecture: the corrections are listed in section 7.1.

Two provenance notes are kept here rather than in the body. The owner's second review cites 641 lines
for revision 1 of revision 2, which is 402 lines as written in revision 1; the ratio is 1.59, which is
consistent with a rendered or wrapped view, and every section and invariant the review names resolves
in the file. It is recorded because a correction's provenance should say what was read.

The D6 prerequisite is contested between two records in this repository. The earlier review of the
research project records an item, its L9, in which an unmapped market status is not silent; a later
read of that item's cited match block did not reproduce the claim. Revision 3's D6 disposition settles
the *requirement* and does not adjudicate the *prerequisite*, which is why section 10 makes it the
first implementation gate rather than an assumption.

Three items are deferred rather than forgotten, each with the condition for admitting it: the bounded
stop-tightening policy (section 3.3), the calibrated confidence-to-size function (section 3.8), and
any cross-sectional portfolio construction (section 3.4). Each would need its own configuration, its
own measurement, and no capacity to loosen what the engine owns.
