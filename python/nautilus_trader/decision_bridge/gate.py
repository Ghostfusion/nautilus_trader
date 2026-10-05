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
Deterministic gate aggregation, and the disposition it produces.

The artifact carries one field that claims something about permission, and it can contradict the
engine's own eligibility. Two implementations reading the same artifact must not reach different
answers, so the aggregation is a declared rule rather than an implementation's judgement:

1. the claim's value is mapped onto a severity by a declared, closed mapping;
2. a value outside that mapping resolves to `RESTRICT`, because the safe direction of an unknown
   value is closed, and the resolution reports that it was unmapped so the event is visible;
3. the most restrictive severity of the claim, the regime reading and the engine's eligibility wins,
   so a permissive value never overrides a stricter one and the result is order-independent;
4. engine eligibility is evaluated independently and cannot be raised by anything research-side, so
a
   research `PERMIT` beside an engine refusal still produces no signal.

A disagreement between the claim and the engine's eligibility is recorded as `GATE_CONFLICT` while
the effective gate remains the most restrictive resolution. A conflict is a diagnostic attribute
rather than a terminal state, because the restrictive resolution is already a correct answer and the
conflict is information about the producer rather than a reason to stop. The disagreement recorded
here is a *permission mismatch* -- one side permits an action the other refuses -- and not the
ordinary case of the claim reducing risk beside an eligible engine.

The disposition is three-valued and only one branch closes the gate. `UNCERTAIN` admits, provided
every independent engine gate passes, and reduces risk by a configured factor: the research half
saying "I cannot establish a directional regime" is not the same statement as "this trade should not
exist", and rejecting on it would hand the research half the authority to close a gate. The
reduction
factor is configuration and is measured as a hypothesis rather than assumed, and it reaches the
signal's strength -- the model's confidence never does.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import field
from typing import Any

from nautilus_trader.decision_bridge.contract import REGIME_SEVERITY_MAPPING
from nautilus_trader.decision_bridge.contract import Diagnostic
from nautilus_trader.decision_bridge.contract import Severity
from nautilus_trader.decision_bridge.contract import gate_severity
from nautilus_trader.decision_bridge.contract import most_restrictive


@dataclass(frozen=True)
class DispositionConfig:
    """
    The bridge's own policy magnitude.

    Parameters
    ----------
    uncertain_reduction : float
        The strength an `UNCERTAIN` resolution contributes, in `(0, 1]`. A reduction never increases
        risk, so a factor above one is refused rather than clamped.

    Raises
    ------
    ValueError
        If the factor is outside `(0, 1]`.

    """

    uncertain_reduction: float = 0.5

    def __post_init__(self) -> None:
        """
        Validate that the reduction is a usable magnitude.
        """
        factor = self.uncertain_reduction
        if isinstance(factor, bool) or not isinstance(factor, (int, float)):
            raise ValueError(  # noqa: TRY004 - configuration, not a type contract
                f"uncertain_reduction must be a number, was {type(factor).__name__}",
            )
        if not 0.0 < float(factor) <= 1.0:
            raise ValueError(
                f"uncertain_reduction {factor} must be in (0, 1]; a reduction never increases risk",
            )


@dataclass(frozen=True)
class GateResolution:
    """
    The result of aggregating the artifact's claim, the regime reading and engine eligibility.

    Parameters
    ----------
    claim : Any
        The claim's raw value, as read from the artifact.
    claim_severity : Severity
        The claim's severity. `RESTRICT` when the value was unmapped.
    claim_mapped : bool
        Whether the claim's value was inside the declared mapping.
    regime : Any
        The regime reading's raw value, or `None` when none was supplied.
    disposition : Severity
        The regime reading's severity.
    engine_eligible : bool
        Whether the engine's own eligibility permits an order.
    effective : Severity
        The most restrictive of the three. This is the gate.
    diagnostics : tuple[Diagnostic, ...]
        What was recorded while resolving.

    """

    claim: Any
    claim_severity: Severity
    claim_mapped: bool
    regime: Any
    disposition: Severity
    engine_eligible: bool
    effective: Severity
    diagnostics: tuple[Diagnostic, ...] = field(default=())

    @property
    def permits(self) -> bool:
        """
        Return whether an order may exist at all under this resolution.
        """
        return self.effective is not Severity.RESTRICT

    @property
    def reduces_risk(self) -> bool:
        """
        Return whether the resolution admits at a reduced risk.
        """
        return self.effective is Severity.UNCERTAIN

    @property
    def engine_severity(self) -> Severity:
        """
        Return the engine's eligibility as a severity.
        """
        return Severity.PERMIT if self.engine_eligible else Severity.RESTRICT


def resolve_gate(
    *,
    claim: Any,  # noqa: ANN401 - the claim is read before validation
    engine_eligible: bool,
    regime: Any = None,  # noqa: ANN401 - the regime reading is read before validation
    claim_mapping: dict[str, Severity] | None = None,
    regime_mapping: dict[str, Severity] | None = None,
) -> GateResolution:
    """
    Aggregate the artifact's claim, the regime reading and the engine's eligibility.

    Parameters
    ----------
    claim : Any
        The artifact's permission claim, as read from `risk_gate.verdict`.
    engine_eligible : bool
        Whether the engine's own eligibility permits an order. Evaluated independently of the claim
        and never raised by it.
    regime : Any, optional
        The regime reading's own value, in its own vocabulary. `None` when no reading was supplied,
        which contributes no restriction.
    claim_mapping : dict[str, Severity], optional
        The declared mapping for the claim's vocabulary. Defaults to `GATE_SEVERITY_MAPPING`.
    regime_mapping : dict[str, Severity], optional
        The declared mapping for the regime vocabulary. Defaults to `REGIME_SEVERITY_MAPPING`.

    Returns
    -------
    GateResolution

    """
    claim_severity, claim_mapped = gate_severity(claim, claim_mapping)

    if regime is None:
        disposition = Severity.PERMIT
    else:
        disposition, _ = gate_severity(
            regime,
            REGIME_SEVERITY_MAPPING if regime_mapping is None else regime_mapping,
        )

    engine = Severity.PERMIT if engine_eligible else Severity.RESTRICT
    effective = most_restrictive(claim_severity, disposition, engine)

    diagnostics: tuple[Diagnostic, ...] = ()
    # A permission mismatch is one side permitting an action the other refuses: the claim permits
    # while the engine refuses, or the claim refuses while the engine permits. A reduction beside an
    # eligible engine is not a conflict -- it is the reduction working as declared.
    mismatch = (engine_eligible and claim_severity is Severity.RESTRICT) or (
        not engine_eligible and claim_severity is Severity.PERMIT
    )
    if mismatch:
        diagnostics = (Diagnostic.GATE_CONFLICT,)

    return GateResolution(
        claim=claim,
        claim_severity=claim_severity,
        claim_mapped=claim_mapped,
        regime=regime,
        disposition=disposition,
        engine_eligible=engine_eligible,
        effective=effective,
        diagnostics=diagnostics,
    )


def strength_for(severity: Severity, config: DispositionConfig) -> float | None:
    """
    Return the signal strength the disposition contributes.

    A `PERMIT` disposition contributes a factor of one, which is equivalent to leaving strength
    absent; an `UNCERTAIN` disposition contributes the configured reduction. Neither reads the
    artifact: the model's confidence is a probability, strength is a magnitude, and equating them
    would silently convert one into a position size.

    Parameters
    ----------
    severity : Severity
        The effective severity.
    config : DispositionConfig
        The configured reduction.

    Returns
    -------
    float | None

    Raises
    ------
    ValueError
        If the severity is `RESTRICT`, under which no signal exists to carry a strength.

    """
    if severity is Severity.RESTRICT:
        raise ValueError("a restrictive resolution produces no signal, so it has no strength")
    if severity is Severity.UNCERTAIN:
        return float(config.uncertain_reduction)

    return None
