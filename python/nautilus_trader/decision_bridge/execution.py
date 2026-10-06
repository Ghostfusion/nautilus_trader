# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------
"""
The submission path, with the advisory allocation imposed as a ceiling.

A decision produces at most one order set, ever, across restarts, re-reads and retries, and the
ledger
is what makes that true. The engine's own submission path mints the client order ids and carries no
caller metadata, so the bridge cannot make submission idempotent by identifier on that path: it
makes
the *decision* idempotent instead. The decision's orders are recorded on the ledger as soon as they
exist, and a second delivery of the same decision -- after a restart, a re-read, or a venue
rejection --
finds them there and submits nothing. A rejection does not free the identity, because from the
engine's
side a lost acknowledgement after venue acceptance is indistinguishable from a rejection.

The advisory allocation is imposed where the engine actually applies a ceiling, rather than recorded
as an intention. `TargetConstruction` caps the constructed weight by its configured `max_weight`,
and
the pipeline holding that configuration can be replaced per decision, so the bridge configures the
pipeline at this decision's ceiling immediately before submitting this decision's signal. The
ceiling
therefore only ever lowers the engine's own construction: the bridge writes no size, reads no price,
and cannot enlarge a position.

Nothing here pre-checks a limit the risk engine owns. A second opinion on a limit is a second
authority, so the bridge submits and lets the engine refuse, recording the typed denial with the cap
the engine named. An engine refusal whose message names no class of failure the bridge knows is
recorded with the engine's own words and no bridge code, because a wrong code is worse than no code.

Two of the five exposure quantities of the attribution chain are the bridge's own configuration, and
the other three are read from the engine rather than estimated. The engine's `targets` call returns
the constructed target, whose weight is the allocation the pipeline resolved against the engine's
own sizing; the engine's order events report whether the risk engine approved or refused the order;
and the fill reports what was realised. `submit_signals` itself returns the client order ids of the
orders the engine emitted and nothing else. The ledger reports an absent quantity as *not reached*,
which is the distinction the attribution exists to make, and a quantity invented here from an
order's notional would be a different measurement wearing the same name.

The five quantities are stored in the artifact's percent units, not as fractions: a constructed
weight of `0.005` is recorded as `0.5`. `research_requested` and `bridge_capped` are already in
percent, and `engine_constructed` and `risk_approved` are converted or copied to match.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from nautilus_trader.decision_bridge.contract import Outcome
from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.ledger import ADMITTED
from nautilus_trader.decision_bridge.ledger import DUPLICATE
from nautilus_trader.decision_bridge.ledger import REFUSED
from nautilus_trader.decision_bridge.ledger import UNATTRIBUTED
from nautilus_trader.decision_bridge.ratings import PositionState
from nautilus_trader.trading import TargetPipelineConfig


if TYPE_CHECKING:
    from collections.abc import Iterable
    from decimal import Decimal

    from nautilus_trader.decision_bridge.artifact import ResearchDecision
    from nautilus_trader.decision_bridge.ledger import AdmissionLedger
    from nautilus_trader.decision_bridge.projection import ProjectionResult
    from nautilus_trader.model import Target
    from nautilus_trader.trading import Strategy


# The engine's own words for the failures the bridge can classify without guessing. Each marker is a
# statement the engine makes about a typed error; a message that matches none leaves the code
# absent, because a wrong classification is worse evidence than an unclassified message.
UNSIZEABLE_MARKERS: tuple[str, ...] = ("unsizeable",)
RISK_LIMIT_MARKERS: tuple[str, ...] = ("notional", "risk limit", "denied")

# The canonical denial codes of `crates/model/src/events/order/denied_reason.rs`, transcribed so a
# rendered denial's leading token can be recognised as a code rather than as prose. That module's
# doc states the leading token is the canonical code and that a consumer must not recover
# classification from the diagnostic suffix, so a leading token outside this set yields no code at
# all rather than a guess.
#
# `MISSING_KNOWLEDGE_DATE` and `PUBLISHED_AFTER_DECISION` are the bridge's own additions (W8.2), not
# the engine's: the knowledge-date gate names its exclusions in the same closed vocabulary so a
# leading token it renders is recognisable as a code.
DENIAL_CODES: frozenset[str] = frozenset(
    {
        "ACTIVE_ORDER_LIMIT_REACHED",
        "BETTING_BALANCE_LOCKED_CALCULATION_FAILED",
        "CLIENT_VENUE_MISMATCH",
        "CUMULATIVE_INITIAL_MARGIN_CALCULATION_FAILED",
        "CUMULATIVE_INITIAL_MARGIN_EXCEEDS_FREE_BALANCE",
        "CUMULATIVE_NOTIONAL_EXCEEDS_FREE_BALANCE",
        "EXPIRE_TIME_IN_PAST",
        "INITIAL_MARGIN_CALCULATION_FAILED",
        "INITIAL_MARGIN_EXCEEDS_FREE_BALANCE",
        "INSTRUMENT_NOT_FOUND",
        "INVALID_CLIENT_ORDER_ID",
        "INVALID_MAX_NOTIONAL_PER_ORDER",
        "INVALID_POSITION_ID",
        "MARKET_PRICE_UNAVAILABLE",
        "MISSING_EXPIRE_TIME",
        "MISSING_KNOWLEDGE_DATE",
        "MISSING_TRAILING_OFFSET",
        "MISSING_TRAILING_OFFSET_TYPE",
        "MISSING_TRIGGER_TYPE",
        "NOTIONAL_BELOW_MINIMUM",
        "NOTIONAL_CALCULATION_FAILED",
        "NOTIONAL_EXCEEDS_FREE_BALANCE",
        "NOTIONAL_EXCEEDS_MAXIMUM",
        "NOTIONAL_EXCEEDS_MAX_PER_ORDER",
        "NO_EXECUTION_CLIENT",
        "ORDER_COUNT_LIMIT_REACHED",
        "ORDER_LIST_DENIED",
        "ORDER_LIST_INCOMPLETE",
        "POSITION_NOT_FOUND",
        "PRICE_NOT_ALIGNED_TO_TICK",
        "PRICE_NOT_POSITIVE",
        "PRICE_PRECISION_EXCEEDS_MAXIMUM",
        "PUBLISHED_AFTER_DECISION",
        "QUANTITY_BELOW_MINIMUM",
        "QUANTITY_CONVERSION_FAILED",
        "QUANTITY_EXCEEDS_MAXIMUM",
        "QUANTITY_PRECISION_EXCEEDS_MAXIMUM",
        "RATE_LIMIT_EXCEEDED",
        "REDUCE_ONLY_WOULD_INCREASE_POSITION",
        "REPEATED_REQUEST_LIMIT_REACHED",
        "PARTICIPATION_LIMIT_REACHED",
        "INVENTORY_LIMIT_REACHED",
        "STREAM_RECONCILING",
        "SUBMIT_FAILED",
        "TRADING_HALTED",
        "TRADING_STATE_REDUCING",
        "TRAILING_STOP_CALCULATION_FAILED",
        "UNSUPPORTED_ORDER_LIST",
        "UNSUPPORTED_ORDER_TYPE",
        "UNSUPPORTED_REDUCE_ONLY",
        "UNSUPPORTED_TIME_IN_FORCE",
        "UNSUPPORTED_TP_SL",
        "UNSUPPORTED_TRAILING_OFFSET_TYPE",
        "VALIDATION_FAILED",
    },
)

# The denials a deliberate retry may follow. A denial is retryable iff the engine states the
# condition clears (`STREAM_RECONCILING`'s own message ends "retry after recovery") or the refusal
# is a counter whose limit is defined over a window or over the open-order set, so the condition
# clears on its own. Everything else - including an unrecognised or absent code - is terminal: the
# engine does not state that it clears, and this bridge does not guess.
RETRYABLE_DENIAL_CODES: frozenset[str] = frozenset(
    {
        "STREAM_RECONCILING",
        "RATE_LIMIT_EXCEEDED",
        "ORDER_COUNT_LIMIT_REACHED",
        "ACTIVE_ORDER_LIMIT_REACHED",
        "REPEATED_REQUEST_LIMIT_REACHED",
    },
)


@dataclass(frozen=True)
class SubmissionResult:
    """
    What the submission path did with one projection.

    Parameters
    ----------
    outcome : Outcome
        The lifecycle branch.
    client_order_ids : tuple[str, ...]
        The identifiers the engine returned for the orders it emitted. Empty when none was emitted.
    detail : str
        Why the branch was reached.
    refusal : Refusal | None
        The typed refusal, when the failure had a class the bridge knows.
    engine_message : str | None
        The engine's own message, recorded verbatim when it refused.

    """

    outcome: Outcome
    client_order_ids: tuple[str, ...] = ()
    detail: str = ""
    refusal: Refusal | None = None
    engine_message: str | None = None


@dataclass(frozen=True)
class RetryPolicy:
    """
    How many deliberate retries a decision may make after a retryable engine denial.

    Parameters
    ----------
    max_attempts : int
        The greatest number of retries this policy permits. `max_attempts=1` permits exactly one
        retry after the first submission, and `max_attempts=0` is rejected: a policy that permits
        nothing is the absence of a retry, expressed by passing no `RetryDecision` to `submit_once`
        at all.

    Raises
    ------
    ValueError
        If `max_attempts` is not a positive integer.

    """

    max_attempts: int

    def __post_init__(self) -> None:
        """
        Validate the retry budget.
        """
        if isinstance(self.max_attempts, bool) or not isinstance(self.max_attempts, int):
            raise ValueError(  # noqa: TRY004 - configuration, not a type contract
                f"max_attempts must be an int, was {type(self.max_attempts).__name__}",
            )
        if self.max_attempts < 1:
            raise ValueError(f"max_attempts must be at least 1, was {self.max_attempts}")


@dataclass(frozen=True)
class RetryDecision:
    """
    The question "may this decision be submitted again", answered against recorded state.

    Parameters
    ----------
    permitted : bool
        Whether a deliberate retry is permitted.
    attempts : int
        The attempt count the decision was computed against. `submit_once` refuses a permission
        whose count no longer matches the ledger, so a permission read before another attempt
        cannot resubmit.
    denial_code : str | None
        The canonical denial code the decision was computed from, or `None` when the engine's
        leading token was not a code the bridge recognises. `None` is never retryable: an
        unrecognised code fails closed rather than being treated as transient.
    reason : str
        Why the retry was permitted or refused, in prose.
    attempt : str
        The caller-supplied label recorded for the retry this decision authorises, or empty when
        the caller supplied none, in which case `submit_once` records a count-derived label. The
        bridge mints no engine identity and reads no clock; the caller owns the retry schedule and
        any waiting between attempts, and this label is only the audit record's name for the
        attempt.

    """

    permitted: bool
    attempts: int
    denial_code: str | None
    reason: str
    attempt: str = ""


def pipeline_config_for(
    base: TargetPipelineConfig,
    ceiling: Decimal | None,
) -> TargetPipelineConfig:
    """
    Return the pipeline configuration for one decision's ceiling.

    Parameters
    ----------
    base : TargetPipelineConfig
        The desk's own pipeline configuration.
    ceiling : Decimal | None
        The advisory allocation as a fraction of exposure, or `None` when the artifact carries none.

    Returns
    -------
    TargetPipelineConfig
        The base configuration with `max_weight` lowered to the ceiling when the ceiling is
        stricter,
        and otherwise the base configuration itself. The ceiling can only lower it: a research
        allocation is a bound, not an instruction.

    """
    if ceiling is None or ceiling >= base.max_weight:
        return base

    return TargetPipelineConfig(
        risk_per_trade=base.risk_per_trade,
        stop_loss_bps=base.stop_loss_bps,
        max_weight=ceiling,
        commission_rate=base.commission_rate,
        min_order_quantity=base.min_order_quantity,
    )


def classify_engine_message(message: str) -> Refusal | None:
    """
    Classify an engine refusal by the engine's own words, or decline to.

    Parameters
    ----------
    message : str
        The message the engine raised.

    Returns
    -------
    Refusal | None
        The typed refusal, or `None` when the message names no class of failure this bridge knows.
        An unclassified failure is recorded with the engine's own message, which is evidence, rather
        than
        given a code that would misstate it.

    """
    lowered = message.lower()
    if any(marker in lowered for marker in UNSIZEABLE_MARKERS):
        return Refusal(RefusalCode.UNSIZEABLE, message)
    if any(marker in lowered for marker in RISK_LIMIT_MARKERS):
        return Refusal(RefusalCode.ENGINE_RISK_LIMIT, message)

    return None


def leading_denial_code(message: str | None) -> str | None:
    """
    Return the canonical denial code a rendered engine message leads with, or `None`.

    The denial module's own doc states that each rendered message's leading token is the stable
    code and that a consumer must not recover classification from the diagnostic suffix, so only
    the token before the first colon is read, and it is returned only when it names a code in the
    engine's closed vocabulary. Anything else yields no code: a wrong classification is worse
    evidence than none.

    Parameters
    ----------
    message : str, optional
        The rendered engine message, or `None`.

    Returns
    -------
    str | None

    """
    if not message:
        return None
    head = message.strip().split(":", 1)[0].strip()
    token = head.split()[0] if head else ""
    return token if token in DENIAL_CODES else None


def retry_decision(
    *,
    policy: RetryPolicy,
    attempts: int,
    denial_code: str | None,
    attempt: str = "",
) -> RetryDecision:
    """
    Decide whether one denial may be followed by a deliberate retry.

    The decision is computed against recorded state: `attempts` is the ledger's current attempt
    count for the decision, and the result carries it so `submit_once` can refuse a permission
    computed against a count that has since moved. A retry is permitted only when the engine's own
    leading token is a retryable code and the budget is not exhausted; nothing else is retryable,
    including an absent or unrecognised code.

    Parameters
    ----------
    policy : RetryPolicy
        The retry budget.
    attempts : int
        The attempt count the permission is computed against.
    denial_code : str | None
        The engine's denial message or its leading code, or `None` when the order was not denied.
    attempt : str, optional
        The caller-supplied label for the retry this decision authorises. Defaults to empty, in
        which case `submit_once` records a count-derived label.

    Returns
    -------
    RetryDecision

    """
    code = leading_denial_code(denial_code)
    if code is None:
        return RetryDecision(
            permitted=False,
            attempts=attempts,
            denial_code=None,
            attempt=attempt,
            reason=(
                "the engine's leading token is not a denial code this bridge recognises, "
                "so the retry is refused rather than guessed"
            ),
        )
    if code not in RETRYABLE_DENIAL_CODES:
        return RetryDecision(
            permitted=False,
            attempts=attempts,
            denial_code=code,
            attempt=attempt,
            reason=f"denial {code} is terminal: the engine does not state that it clears",
        )
    if attempts >= policy.max_attempts:
        return RetryDecision(
            permitted=False,
            attempts=attempts,
            denial_code=code,
            attempt=attempt,
            reason=(
                f"the retry budget of {policy.max_attempts} is exhausted after {attempts} attempts"
            ),
        )

    return RetryDecision(
        permitted=True,
        attempts=attempts,
        denial_code=code,
        attempt=attempt,
        reason=f"denial {code} states that it clears and the retry budget is not exhausted",
    )


def position_state(net_position: Decimal) -> PositionState:
    """
    Map the engine's net position onto the state the rating policy is total over.

    Parameters
    ----------
    net_position : Decimal
        The instrument's net position, read from the portfolio, which stays authoritative.

    Returns
    -------
    PositionState
        `LONG` above zero, `SHORT` below it, and `FLAT` at zero. A negative position is reported
        rather than refused: the bridge must answer for the state the engine reports, and every row
        for it resolves to no signal.

    """
    if net_position > 0:
        return PositionState.LONG
    if net_position < 0:
        return PositionState.SHORT

    return PositionState.FLAT


def _record_exposure_once(
    ledger: AdmissionLedger,
    decision_id: str,
    stage: str,
    quantity: float,
) -> None:
    """
    Write one exposure stage once, leaving an already-recorded answer untouched.

    Attribution is the bridge reading the engine's own answer, never a reason to refuse an order, so
    a stage that already has a value is left as it is rather than raising: a retry must not rewrite
    an earlier measurement, and attribution must never fail the submission it describes.

    Parameters
    ----------
    ledger : AdmissionLedger
        The ledger.
    decision_id : str
        The identity of the decision.
    stage : str
        The exposure stage, one of `EXPOSURE_STAGES`.
    quantity : float
        The exposure the stage holds, in the artifact's allocation units.

    """
    record = ledger.record(decision_id)
    if record is None or getattr(record, stage) is not None:
        return
    ledger.record_exposure(decision_id, stage, quantity)


def _record_attribution(
    ledger: AdmissionLedger,
    decision: ResearchDecision,
    ceiling: Decimal | None,
) -> None:
    """
    Record the exposure quantities the bridge owns.

    A quantity whose stage did not run is not written at all, so the ledger reports it as not
    reached rather than as a zero that would claim the exposure was considered and refused. Both
    quantities are percent-valued, as the ledger's allocation units require.
    """
    if decision.allocation_pct is not None:
        _record_exposure_once(
            ledger,
            decision.decision_id,
            "research_requested",
            decision.allocation_pct,
        )
    if ceiling is not None:
        _record_exposure_once(
            ledger,
            decision.decision_id,
            "bridge_capped",
            float(ceiling) * 100.0,
        )


def _record_engine_constructed(
    ledger: AdmissionLedger,
    decision_id: str,
    targets: Iterable[Target],
) -> None:
    """
    Record the allocation the engine constructed, in the artifact's percent units.

    The pipeline's construction stage emits a weight target in the ceiling path, and that weight is
    the allocation as a fraction of equity, so `weight * 100` is the percent value the ledger
    stores. A target whose kind is not `WEIGHT` is refused rather than converted: deriving an
    allocation from a quantity or a notional would repeat the engine's own sizing and price
    resolution, which this bridge must never do, so the stage is left absent and reported as not
    reached.

    No target identity is written. The engine's `Target` carries no identity - its fields are
    `instrument_id`, `value`, `ts_event` and `ts_init` - so there is nothing to record, and
    `target_id` stays a named absence rather than a re-derived surrogate.

    Parameters
    ----------
    ledger : AdmissionLedger
        The ledger.
    decision_id : str
        The identity of the decision.
    targets : Iterable[Target]
        The targets the engine constructed for the decision's signal.

    """
    record = ledger.record(decision_id)
    if record is None or record.engine_constructed is not None:
        return
    for target in targets:
        if target.value.kind != "WEIGHT":
            continue
        weight = target.value.weight
        if weight is None:
            continue
        ledger.record_exposure(decision_id, "engine_constructed", float(weight) * 100.0)
        return


def _retry_refusal(
    ledger: AdmissionLedger,
    decision_id: str,
    retry: RetryDecision,
) -> str | None:
    """
    Return why a retry permission is refused now, or `None` when it still holds.

    A permission that was not permitted is refused, and so is one computed against an attempt count
    the ledger has since moved past. The stored count is what makes a stale permission safe: only a
    caller that read the current count may retry.
    """
    if not retry.permitted:
        return retry.reason
    current = ledger.attempt_count(decision_id)
    if retry.attempts != current:
        return (
            f"the retry was computed against {retry.attempts} attempts but the ledger now holds "
            f"{current}; a stale permission submits nothing"
        )
    return None


def submit_once(  # noqa: PLR0913 - the submission's declared inputs
    *,
    ledger: AdmissionLedger,
    projection: ProjectionResult,
    strategy: Strategy,
    base_pipeline: TargetPipelineConfig,
    ceiling: Decimal | None,
    retry: RetryDecision | None = None,
) -> SubmissionResult:
    """
    Submit one projected decision's signal at most once.

    Parameters
    ----------
    ledger : AdmissionLedger
        The ledger the decision's orders are recorded on, and the memory that makes a replay a
        no-op.
    projection : ProjectionResult
        The projection to submit.
    strategy : Strategy
        The strategy whose pipeline and submission path are used. Its client order ids are the
        engine's own, minted on the engine's path.
    base_pipeline : TargetPipelineConfig
        The desk's own pipeline configuration.
    ceiling : Decimal | None
        The decision's advisory ceiling as a fraction, or `None`.
    retry : RetryDecision, optional
        A permission, computed against the ledger's recorded attempt count, to submit a decision
        that already produced an order set once more after the engine refused it. `None` keeps the
        duplicate rule: a decision that already produced its order set submits nothing. A refused
        permission returns `Outcome.DUPLICATE`, which already means "this call submits nothing",
        rather than a new outcome. The bridge holds no clock, timer or backoff and is a non-goal
        there: the caller labels the attempt and owns any waiting between attempts, and this
        function only decides whether a labelled attempt is permitted.

    Returns
    -------
    SubmissionResult

    """
    decision = projection.decision
    if decision is None:
        return SubmissionResult(outcome=projection.outcome, detail=projection.detail)

    record = ledger.record(decision.decision_id)
    if record is not None and record.order_ids:
        if retry is None:
            return SubmissionResult(
                outcome=Outcome.DUPLICATE,
                client_order_ids=tuple(record.order_ids),
                detail="the decision already produced its order set; a replay submits nothing",
            )
        refusal = _retry_refusal(ledger, decision.decision_id, retry)
        if refusal is not None:
            return SubmissionResult(
                outcome=Outcome.DUPLICATE,
                client_order_ids=tuple(record.order_ids),
                detail=refusal,
            )

    if projection.signal is None:
        return SubmissionResult(outcome=projection.outcome, detail=projection.detail)

    strategy.enable_target_pipeline(pipeline_config_for(base_pipeline, ceiling))
    _record_attribution(ledger, decision, ceiling)

    try:
        targets = strategy.targets([projection.signal])
        _record_engine_constructed(ledger, decision.decision_id, targets)
        client_order_ids = strategy.submit_signals([projection.signal])
    except Exception as exc:  # noqa: BLE001 - the engine's refusal is a recordable outcome
        message = str(exc)
        return SubmissionResult(
            outcome=Outcome.DENIED,
            detail=f"the engine refused the order set: {message}",
            refusal=classify_engine_message(message),
            engine_message=message,
        )

    for client_order_id in client_order_ids:
        ledger.record_order(decision.decision_id, str(client_order_id))
    if retry is not None:
        attempt = retry.attempt or f"attempt-{ledger.attempt_count(decision.decision_id) + 1}"
        ledger.record_attempt(decision.decision_id, attempt)

    return SubmissionResult(
        outcome=Outcome.SIGNAL,
        client_order_ids=tuple(str(client_order_id) for client_order_id in client_order_ids),
        detail="the order set was submitted once",
    )


def record_fill_once(
    *,
    ledger: AdmissionLedger,
    client_order_id: str,
    fill_id: str,
    filled_pct: float | None,
) -> str:
    """
    Attribute one fill to the decision that produced its order.

    Parameters
    ----------
    ledger : AdmissionLedger
        The ledger.
    client_order_id : str
        The client order id on the fill.
    fill_id : str
        The identity of the fill.
    filled_pct : float | None
        The filled exposure in the artifact's allocation units, or `None` when it cannot be read.

    Returns
    -------
    str
        The decision the fill was attributed to, or `UNATTRIBUTED` when the order carries no
        decision
        identity.

    """
    decision_id = ledger.decision_for_order(client_order_id)
    if decision_id == UNATTRIBUTED:
        return UNATTRIBUTED

    ledger.record_fill(decision_id, fill_id)
    if filled_pct is not None:
        record = ledger.record(decision_id)
        if record is not None and record.actually_filled is None:
            ledger.record_exposure(decision_id, "actually_filled", filled_pct)

    return decision_id


# Ordering evidence for the guard below. In `crates/risk/src/engine/mod.rs`, `handle_submit_order`
# runs every risk check (reduce-only, instrument presence, caps, `check_order`, `check_orders_risk`)
# and only then calls `self.execution_gateway(TradingCommand::SubmitOrder(command))`; every denial
# path returns before that call, and the submit throttler's failure handler emits `OrderDenied`
# without forwarding while its success handler routes to `exec_engine_queue_execute`. The execution
# engine emits `OrderSubmitted` (`crates/execution/src/matching_engine/mod.rs:6720`) only for a
# command it received, and a denied command never reaches it. A denial is therefore emitted instead
# of `OrderSubmitted`, never beside it: for one order the two observations are mutually exclusive.
# The risk stage may still revise its own earlier answer across retries of one decision, which is
# what `AdmissionLedger.record_risk_answer` exists for.
def record_engine_event(
    *,
    ledger: AdmissionLedger,
    client_order_id: str,
    reason: str | None = None,
) -> str:
    """
    Attribute one engine order event to its decision and record the risk stage.

    Whether an order was denied is decided by the event, never by classifying its message: `reason`
    is the engine's rendered denial message for a denial and `None` for a non-denial, so a denial
    whose leading token the bridge does not recognise is still recorded as a denial rather than
    silently misread as an approval.

    A denial is the risk engine's own answer, so `risk_approved` is recorded as zero: a measured
    zero, not an absent stage. A non-denial is an `OrderSubmitted` event, which the engine's
    ordering guarantees can only follow a passed risk gate, so the risk stage records the
    constructed allocation the gate approved. Because this engine's risk constraints *deny* an
    order rather than sizing it down, `risk_approved` is either the constructed allocation or zero
    and never an intermediate value.

    An order with no decision identity writes nothing and is reported as `UNATTRIBUTED`, so an
    order placed outside the bridge cannot be assigned to the nearest decision.

    Parameters
    ----------
    ledger : AdmissionLedger
        The ledger.
    client_order_id : str
        The client order id on the engine's order event.
    reason : str, optional
        The engine's rendered denial message when the order was denied, or `None` when it was
        submitted. The message's leading token is not consulted here: it is evidence beside the
        event, not the reason the event is a denial.

    Returns
    -------
    str
        The decision the event was attributed to, or `UNATTRIBUTED` when the order carries no
        decision identity.

    """
    decision_id = ledger.decision_for_order(client_order_id)
    if decision_id == UNATTRIBUTED:
        return UNATTRIBUTED

    if reason is not None:
        ledger.record_risk_answer(decision_id, 0.0)
        return decision_id

    record = ledger.record(decision_id)
    if record is not None and record.engine_constructed is not None:
        ledger.record_risk_answer(decision_id, record.engine_constructed)

    return decision_id


def admission_result_of(projection: ProjectionResult) -> str:
    """
    Return the ledger's admission result for a projection.

    Parameters
    ----------
    projection : ProjectionResult
        The projection.

    Returns
    -------
    str
        One of the ledger's `ADMISSION_RESULTS`.

    """
    if projection.refusal is not None:
        return REFUSED
    if projection.outcome is Outcome.DUPLICATE:
        return DUPLICATE

    return ADMITTED


def record_decision(
    *,
    ledger: AdmissionLedger,
    projection: ProjectionResult,
    received_at: int,
    admission_timestamp: int,
) -> str:
    """
    Record one projection's arrival and outcome on the ledger.

    Parameters
    ----------
    ledger : AdmissionLedger
        The ledger.
    projection : ProjectionResult
        The projection.
    received_at : int
        The instant the bridge received the artifact, in UNIX nanoseconds.
    admission_timestamp : int
        The instant the decision stage evaluated the artifact, in UNIX nanoseconds.

    Returns
    -------
    str
        The admission result recorded, which is `DUPLICATE` for a repeated delivery.

    """
    decision = projection.decision
    if decision is None:
        return REFUSED

    return ledger.admit(
        decision.decision_id,
        artifact_sha256=decision.artifact_sha256,
        idempotency_key=decision.idempotency_key or decision.revision_id,
        instrument_id=str(decision.instrument_id),
        produced_at=decision.produced_at,
        received_at=received_at,
        effective_date=decision.effective_date.isoformat(),
        expires_at=decision.expires_at,
        actionable_at=projection.actionable_at,
        admission_timestamp=admission_timestamp,
        admission_result=admission_result_of(projection),
        refusal_reason=projection.refusal.code if projection.refusal else None,
        diagnostics=projection.diagnostics,
    )
