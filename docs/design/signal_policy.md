# Signal intents, arbitration and scored entry decisions: implementation record

## 1. Scope

A strategy component rarely speaks alone. A trend rule wants to open, a mean-reversion rule wants to
close, and both fire on the same bar for the same instrument; without a declared policy the winner is
whatever the submission code evaluated first. The engine's own signal model states a view with three
directions - long, short and flat - and has no name for adding to a position in tranches, trimming one
in tranches, or saying that a close is a close.

This record covers a pure-Python package that fills the second gap without touching the first: three
declarations that belong to a strategy and are usually written as code inside it.

| Module                                               | What it declares                                                                      |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `python/nautilus_trader/signal_policy/intent.py`     | `SignalSide` (six transitions), `PositionSide`, `PositionRequirement`, `SignalIntent` |
| `python/nautilus_trader/signal_policy/conflict.py`   | `ConflictPolicy`, `ConflictMode`, `ConflictReason`, the resolution records            |
| `python/nautilus_trader/signal_policy/confluence.py` | `Condition` and its compositions, `ScoreRule`, `RuleGroup`, `ConfluenceCard`          |

Everything is pure: no cache, no clock, no submission. The package adds no crate, no PyO3 binding and
no generated stub, and it changes no existing Python or Rust file.

## 2. What was deliberately not done

**`SignalDirection` was not extended.** Adding variants to it breaks two exhaustive matches in
`crates/trading/src/target.rs` (`construct_one` and `stop_price`), changes a Python-visible enum and
would need stub regeneration. The six transitions live in the policy layer, and the mapping onto the
engine's three directions is explicit and partial: an open maps to `LONG` or `SHORT`, a close maps to
`FLAT`, and the two scaling sides raise.

**The tranche arithmetic is not in the engine.** `SignalSide.SCALE_IN` and `SCALE_OUT` have no target
of their own. The target path sizes a position from risk, so a tranche expressed as a target would
replace the position rather than add to or trim it; a partial trim has no target representation at
all. A caller who wants tranches keeps that arithmetic in their own strategy, and the policy records
the intention as an intention.

**No cooldown, capacity or exposure cap.** Each needs state the package does not own - bars elapsed,
position size, account exposure - and each already has a home: the risk engine for caps, the strategy
for cooldowns over its own state. A rule that needs both a declaration and live state belongs where
the state is.

## 3. The four gates, in the order they run

`ConflictPolicy.resolve` decides one instrument at a time, against a position state the caller
supplies, and records every decision it did not take:

1. **Disabled and expired.** A side the policy was told not to admit is refused with
   `disabled_side`; an intention past its declared expiry is refused with `expired`, and only when
   the caller declares the instant it is resolving at.
2. **Position state.** An open requires no position, a close requires the side it closes, and a scale
   requires some position to scale. The rest are refused with `position_state`, naming the state
   required and the state found.
3. **The direction mutex.** A long intention and a short intention on one instrument are a
   contradiction. `REFUSE` (the default) refuses both rather than guessing; `PRIORITY` and `STRENGTH`
   resolve it and say which rule decided, including the tie-break to precedence when two strengths
   are equal.
4. **One slot per instrument.** The target path builds one target per instrument per submission, so
   the best-ranked intention is admitted by default and the losers are refused with
   `lower_precedence`, naming the rank and the strength that beat them.

The default precedence is exits before entries before scaling -
`CLOSE_LONG, CLOSE_SHORT, SCALE_OUT, OPEN_LONG, OPEN_SHORT, SCALE_IN` - so a policy given no
configuration reduces exposure rather than adding to it.

## 4. The scored entry decision

`ConfluenceCard` states what a chain of `if` statements states implicitly: named conditions, weighted
rules, how many must match, what score must be reached, what must hold, and what vetoes. Four
properties are enforced rather than documented.

- **A missing input is not a false one.** A condition over an input that was not supplied evaluates to
  unknown, unknown is never satisfied, and the rules whose inputs were missing are listed in the
  result, so an entry refused for want of data is distinguishable from one refused on the data.
- **A veto is not a requirement.** A mandatory condition must hold for a qualified entry; a veto
  refuses the entry when it holds at all. Both are read before the score can qualify anything.
- **A rule's points carry its own sign.** A positive rule cannot carry negative points, so a card
  cannot be read as though every rule added to the score.
- **The primary group's floor is a floor.** A high total score cannot buy past a primary group that
  did not match enough rules or reach its own score floor.

## 5. Verification

| Check      | Command                                                                                     | Result                    |
| ---------- | ------------------------------------------------------------------------------------------- | ------------------------- |
| Unit tests | `python/.venv/Scripts/python.exe -m pytest tests/unit/signal_policy -q`                     | 47 passed                 |
| Lint       | `ruff check --config pyproject.toml nautilus_trader/signal_policy tests/unit/signal_policy` | clean                     |
| Format     | `ruff format --check nautilus_trader/signal_policy tests/unit/signal_policy`                | 7 files already formatted |

The tests check the boundaries rather than the happy path: the requirement truth table, the refusal of
a strength outside the unit interval, the refusal of a scaling side to become a signal, each of the
four gates on its own, the three modes of the mutex including the tie-break, the three-valued logic of
the composed conditions, and that a refusal for want of data is recorded separately from one on the
data.
