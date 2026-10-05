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
The projection: one artifact becomes one statement of view, or a named refusal.

A decision document projects to exactly one statement of view, or to a named refusal. The projection
reads no clock, consults no model, mutates nothing, and for equal inputs returns an equal signal.
The
artifact is necessary to the flow and never sufficient for it: a signal exists only when the
artifact
is valid, actionable, positively tradable and confluent under the gate rule, and when the engine's
own
eligibility passes.

The stages run in the design's declared order, and the first failing stage names the refusal:
admission, availability, tradability, eligibility, the rating policy, and finally the projection
itself. Later stages are not evaluated after a refusal, so the reason a record was refused is a
function of the artifact rather than of an implementation's evaluation order.

Three fields do not map directly, and each is settled here rather than defaulted. *Horizon* is not
in
the wire contract at all, so it is configuration: a horizon is semantic, the artifact carries none,
and deriving one from the expiry would let an operational instant decide a semantic quantity in
reverse. *Direction* is derived from the rating and never from the artifact's `direction` field,
which
the contract declares as the consumer's own slot ("null allowed when rating resolves the action"),
so
a producer-set direction is recorded as evidence rather than honoured as an instruction. And
*strength*
is written only by the disposition mapping, never read from the artifact: the model's confidence is
a
probability, strength is a magnitude, and equating them would silently convert one into a position
size.

The advisory allocation caps rather than instructs: it is normalised once, from a percentage to a
fraction, and exposed as `ceiling` for the caller to impose on the engine's own construction. An
allocation of zero beside a directional view resolves to no signal rather than to a refusal, because
zero is inside the domain and the artifact sanctions no exposure; the contradiction is recorded
under
the `GATE_CONFLICT` diagnostic. A non-zero allocation beside a view that resolves to a zero target
is
not consumed and is recorded the same way, with the target remaining zero as the restrictive
resolution.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import field
from typing import TYPE_CHECKING
from typing import Any

from nautilus_trader.decision_bridge.artifact import KNOWN_FIELDS
from nautilus_trader.decision_bridge.artifact import ResearchDecision
from nautilus_trader.decision_bridge.artifact import admit
from nautilus_trader.decision_bridge.contract import Diagnostic
from nautilus_trader.decision_bridge.contract import Outcome
from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.gate import DispositionConfig
from nautilus_trader.decision_bridge.gate import GateResolution
from nautilus_trader.decision_bridge.gate import resolve_gate
from nautilus_trader.decision_bridge.gate import strength_for
from nautilus_trader.decision_bridge.numeric import DEFAULT_SCALES
from nautilus_trader.decision_bridge.numeric import DeclaredScales
from nautilus_trader.decision_bridge.numeric import allocation_fraction
from nautilus_trader.decision_bridge.ratings import PositionState
from nautilus_trader.decision_bridge.ratings import RatingDecision
from nautilus_trader.decision_bridge.ratings import RatingResolution
from nautilus_trader.decision_bridge.ratings import decide_rating
from nautilus_trader.decision_bridge.temporal import Actionability
from nautilus_trader.decision_bridge.temporal import resolve_actionability
from nautilus_trader.decision_bridge.tradability import Tradability
from nautilus_trader.decision_bridge.tradability import require
from nautilus_trader.model import SignalDirection
from nautilus_trader.model import TradingSignal


if TYPE_CHECKING:
    from collections.abc import Callable
    from decimal import Decimal

    from nautilus_trader.model import InstrumentId
    from nautilus_trader.model import TradingCalendar


@dataclass(frozen=True)
class ProjectionConfig:
    """
    The bridge's own configuration.

    Parameters
    ----------
    horizon_ns : int
        The horizon the bridge's signals state, in nanoseconds. Required and positive: the wire
        contract declares no horizon, a horizon is semantic, and deriving one from the operational
        expiry would let an instant decide a semantic quantity in reverse.
    authorized_producers : frozenset[str]
        The producer services this bridge accepts. Identity and authorisation are different
        properties: a producer that is identified but not authorised is refused.
    scales : DeclaredScales, optional
        The declared decimal scales. Defaults to `DEFAULT_SCALES`.
    disposition : DispositionConfig, optional
        The disposition's policy magnitude. Defaults to `DispositionConfig()`.

    Raises
    ------
    ValueError
        If the horizon is not a positive duration.

    """

    horizon_ns: int
    authorized_producers: frozenset[str] = frozenset()
    scales: DeclaredScales = DEFAULT_SCALES
    disposition: DispositionConfig = field(default_factory=DispositionConfig)

    def __post_init__(self) -> None:
        """
        Validate the configured horizon.
        """
        if isinstance(self.horizon_ns, bool) or not isinstance(self.horizon_ns, int):
            raise ValueError(  # noqa: TRY004 - configuration, not a type contract
                f"horizon_ns must be an int, was {type(self.horizon_ns).__name__}",
            )
        if self.horizon_ns <= 0:
            raise ValueError(f"horizon_ns {self.horizon_ns} is not a positive duration")


@dataclass(frozen=True)
class ProjectionContext:
    """
    The engine facts one projection reads.

    Every value here is supplied by the caller, so the projection itself reads no clock and no
    global
    and can be replayed exactly.

    Parameters
    ----------
    received_at : int
        The moment this engine received the artifact, as UNIX nanoseconds.
    tradability : Tradability
        The instrument's tradability, established per leg.
    position : PositionState
        The instrument's position, read from the cache and portfolio, which stay authoritative.
    engine_eligible : bool
        Whether the engine's own eligibility permits an order. Evaluated independently of the
        artifact and never raised by it.
    calendar : TradingCalendar | None
        The instrument's trading calendar, or `None` when none is available for its key.
    regime : Any
        The regime reading's own value, in its own vocabulary. `None` when no reading was supplied.

    """

    received_at: int
    tradability: Tradability
    position: PositionState
    engine_eligible: bool
    calendar: TradingCalendar | None = None
    regime: Any = None


@dataclass(frozen=True)
class ProjectionResult:
    """
    What became of one artifact.

    Parameters
    ----------
    outcome : Outcome
        The lifecycle branch the artifact reached.
    detail : str
        Why it reached it, in prose. The refusal's code is the classification; this is the evidence.
    refusal : Refusal | None
        The named refusal, when the artifact was refused.
    decision : ResearchDecision | None
        The admitted artifact, when admission succeeded.
    signal : TradingSignal | None
        The projected statement of view, when one exists.
    actionable_at : int | None
        The resolved availability instant, when availability was resolved.
    ceiling : Decimal | None
        The advisory allocation as a fraction, to be imposed on the engine's own construction.
        `None`
        when the artifact carries no allocation.
    gate : GateResolution | None
        The gate resolution, when the gate was evaluated.
    rating : RatingDecision | None
        The rating decision, when the rating policy was evaluated.
    diagnostics : tuple[Diagnostic, ...]
        Diagnostics recorded on the way.

    """

    outcome: Outcome
    detail: str
    refusal: Refusal | None = None
    decision: ResearchDecision | None = None
    signal: TradingSignal | None = None
    actionable_at: int | None = None
    ceiling: Decimal | None = None
    gate: GateResolution | None = None
    rating: RatingDecision | None = None
    diagnostics: tuple[Diagnostic, ...] = ()

    @property
    def refused(self) -> bool:
        """
        Return whether the artifact was refused.
        """
        return self.refusal is not None


def _refused(
    refusal: Refusal,
    *,
    decision: ResearchDecision | None = None,
    diagnostics: tuple[Diagnostic, ...] = (),
) -> ProjectionResult:
    return ProjectionResult(
        outcome=Outcome.REFUSED,
        detail=refusal.detail,
        refusal=refusal,
        decision=decision,
        diagnostics=diagnostics,
    )


def project(  # noqa: C901, PLR0911 - the first failing stage decides the refusal
    raw: Any,  # noqa: ANN401 - the artifact is read before validation
    *,
    context: ProjectionContext,
    config: ProjectionConfig,
    resolve_instrument: Callable[[str], InstrumentId | None],
) -> ProjectionResult:
    """
    Project one artifact onto a statement of view, or refuse it by name.

    Parameters
    ----------
    raw : Any
        The artifact as read from disk.
    context : ProjectionContext
        The engine facts.
    config : ProjectionConfig
        The bridge's configuration.
    resolve_instrument : Callable[[str], InstrumentId | None]
        Resolves the artifact's ticker to an instrument.

    Returns
    -------
    ProjectionResult
        The refusal, the declined view, or the signal. Never a partial projection.

    """
    # Stage: admission.
    decision = admit(
        raw,
        resolve_instrument=resolve_instrument,
        authorized_producers=config.authorized_producers,
        known_fields=KNOWN_FIELDS,
    )
    if isinstance(decision, Refusal):
        return _refused(decision)

    diagnostics: list[Diagnostic] = []

    # Stage: availability.
    actionability = resolve_actionability(
        produced_at=decision.produced_at,
        expires_at=decision.expires_at,
        received_at=context.received_at,
        calendar=context.calendar,
    )
    if isinstance(actionability, Refusal):
        return _refused(actionability, decision=decision)

    diagnostics.extend(actionability.diagnostics)

    # Stage: tradability, positively established per leg before this call.
    tradability_refusal = require(context.tradability, decision.instrument_id)
    if tradability_refusal is not None:
        return _refused(tradability_refusal, decision=decision, diagnostics=tuple(diagnostics))

    # Stage: eligibility, the artifact's claim aggregated with the engine's independent answer.
    gate = resolve_gate(
        claim=decision.gate_verdict,
        engine_eligible=context.engine_eligible,
        regime=context.regime,
    )
    diagnostics.extend(gate.diagnostics)
    if not gate.permits:
        return _refused(
            Refusal(
                RefusalCode.RISK_GATE_REJECT,
                f"effective gate is {gate.effective.value}: "
                f"claim={gate.claim!r} disposition={gate.disposition.value} "
                f"engine_eligible={gate.engine_eligible}",
            ),
            decision=decision,
            diagnostics=tuple(diagnostics),
        )

    # Stage: the rating policy, and the position it is resolved against.
    rating = decide_rating(decision.rating, context.position)
    if isinstance(rating, Refusal):
        return _refused(rating, decision=decision, diagnostics=tuple(diagnostics))

    # Stage: the advisory allocation, normalised exactly once.
    ceiling: Decimal | None = None
    if decision.allocation_pct is not None:
        converted = allocation_fraction(decision.allocation_pct, scales=config.scales)
        if isinstance(converted, Refusal):
            return _refused(converted, decision=decision, diagnostics=tuple(diagnostics))
        ceiling = converted

    # The two in-file disagreements the design resolves restrictively, each recorded.
    if rating.resolution is RatingResolution.LONG_SIGNAL and ceiling == 0:
        diagnostics.append(Diagnostic.GATE_CONFLICT)
        return ProjectionResult(
            outcome=Outcome.NO_SIGNAL,
            detail=(
                "the artifact sanctions no exposure: a directional view beside a zero allocation"
            ),
            decision=decision,
            actionable_at=actionability.actionable_at,
            ceiling=ceiling,
            gate=gate,
            rating=rating,
            diagnostics=tuple(diagnostics),
        )

    if rating.resolution is RatingResolution.FLAT_TARGET and ceiling not in (None, 0):
        # The allocation is not consumed: the target stays zero, and the disagreement is recorded.
        diagnostics.append(Diagnostic.GATE_CONFLICT)

    if rating.resolution is RatingResolution.NO_SIGNAL:
        return ProjectionResult(
            outcome=Outcome.NO_SIGNAL,
            detail=rating.rationale,
            decision=decision,
            actionable_at=actionability.actionable_at,
            ceiling=ceiling,
            gate=gate,
            rating=rating,
            diagnostics=tuple(diagnostics),
        )

    direction = (
        SignalDirection.LONG
        if rating.resolution is RatingResolution.LONG_SIGNAL
        else SignalDirection.FLAT
    )

    signal = TradingSignal(
        instrument_id=decision.instrument_id,
        direction=direction,
        horizon_ns=config.horizon_ns,
        strength=strength_for(gate.effective, config.disposition),
        source=decision.producer_service,
        expiry_ns=decision.expires_at,
        provenance=_provenance(decision, gate, rating, actionability, ceiling),
        ts_event=actionability.actionable_at,
        ts_init=actionability.actionable_at,
    )

    return ProjectionResult(
        outcome=Outcome.SIGNAL,
        detail=rating.rationale,
        decision=decision,
        signal=signal,
        actionable_at=actionability.actionable_at,
        ceiling=ceiling,
        gate=gate,
        rating=rating,
        diagnostics=tuple(diagnostics),
    )


def _provenance(
    decision: ResearchDecision,
    gate: GateResolution,
    rating: RatingDecision,
    actionability: Actionability,
    ceiling: Decimal | None,
) -> dict[str, Any]:
    """
    Return the signal's provenance.

    Every value is JSON-serialisable, because the signal's provenance is a JSON object on the engine
    side. The advisory levels travel here rather than into the engine: the artifact's stop would be
    a
    research-supplied position size, which is precisely what the bridge must not adopt.
    """
    advisory = decision.advisory
    return {
        "decision_id": decision.decision_id,
        "revision_id": decision.revision_id,
        "artifact_sha256": decision.artifact_sha256,
        "schema_version": decision.schema_version,
        "ticker": decision.ticker,
        "effective_date": decision.effective_date.isoformat(),
        "rating": decision.rating,
        "view": rating.view.value,
        "producer_service": decision.producer_service,
        "producer_run_id": decision.producer_run_id,
        "confidence": advisory.get("confidence"),
        "data_quality": advisory.get("data_quality"),
        "binding_constraint": advisory.get("binding_constraint"),
        "guardrail_reason": advisory.get("guardrail_reason"),
        "position": advisory.get("position"),
        "gate_claim": gate.claim,
        "gate_claim_mapped": gate.claim_mapped,
        "gate_effective": gate.effective.value,
        "gate_reasons": list(decision.gate_reasons),
        "allocation_pct": decision.allocation_pct,
        "ceiling_fraction": None if ceiling is None else str(ceiling),
        "actionable_at": actionability.actionable_at,
        "unknown_fields": list(decision.unknown_fields),
        "reserved_names_present": list(decision.reserved_names_present),
    }
