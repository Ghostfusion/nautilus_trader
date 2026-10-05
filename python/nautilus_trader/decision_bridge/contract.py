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
The closed vocabularies the research-to-execution bridge speaks.

A refusal is an outcome rather than a discarded record: the question "what fraction of research
decisions never became executable, and why" is answerable only if the failures are kept, so a code
here is recorded on the admission ledger rather than raised as an error.

Refusal codes are grouped by the stage that produces them, and that grouping is also the order the
stages can occur in. A record that fails several stages is refused with the reason of the earliest
stage, so the reason is a function of the artifact rather than of an implementation's evaluation
order. Diagnostics are recorded beside a record that continued, because in every diagnostic case the
restrictive answer is already correct and the diagnostic is information about the producer or the
pipeline rather than a reason to stop.

Two codes are additions to the owner's set, and both are marked here as such. `CALENDAR_UNCOVERED`
and `RATING_UNKNOWN` were the design's own additions; `PRODUCED_AT_ABSENT` and `CALENDAR_MISSING`
are this implementation's, and each answers a case the closed set otherwise cannot name: the
producer's contract declares `produced_at` nullable, so an artifact can be schema-valid while
carrying no instant to resolve availability from, and a key with no bundled calendar is not the same
case as a calendar that does not cover the instant.

`PUBLISHED_AFTER_DECISION` and `MISSING_KNOWLEDGE_DATE` are the data path's own additions (W8.2),
marked as such below: they name a datum that cannot be shown to have been knowable at the decision
time, because its publication instant is later than the decision or is not carried at all.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import field
from enum import Enum
from enum import unique


@unique
class Stage(Enum):
    """
    The stage that produces a code, in the order the stages occur.
    """

    ADMISSION = "ADMISSION"
    AVAILABILITY = "AVAILABILITY"
    TRADABILITY = "TRADABILITY"
    ELIGIBILITY = "ELIGIBILITY"
    PROJECTION = "PROJECTION"
    EXECUTION = "EXECUTION"


# The declared stage order. The first failing stage names the refusal, so this tuple is the reason a
# record failing two checks cannot be refused with the later one's code.
STAGE_ORDER: tuple[Stage, ...] = (
    Stage.ADMISSION,
    Stage.AVAILABILITY,
    Stage.TRADABILITY,
    Stage.ELIGIBILITY,
    Stage.PROJECTION,
    Stage.EXECUTION,
)


@unique
class RefusalCode(Enum):
    """
    The closed set of reasons an artifact stops at a stage.

    The set is closed deliberately: a new failure mode requires a declared code rather than a
    message, so a reader can classify an outcome without reading prose.
    """

    # Admission
    SCHEMA_INVALID = "SCHEMA_INVALID"
    HASH_INVALID = "HASH_INVALID"
    PRODUCER_UNKNOWN = "PRODUCER_UNKNOWN"
    PRODUCER_UNAUTHORIZED = "PRODUCER_UNAUTHORIZED"
    INSTRUMENT_UNKNOWN = "INSTRUMENT_UNKNOWN"
    ALLOCATION_INVALID = "ALLOCATION_INVALID"
    DUPLICATE = "DUPLICATE"

    # Availability
    MISSING_EXPIRY = "MISSING_EXPIRY"
    # Addition: the contract leaves `produced_at` nullable
    PRODUCED_AT_ABSENT = "PRODUCED_AT_ABSENT"
    CALENDAR_MISSING = "CALENDAR_MISSING"  # Addition: no bundled calendar for the instrument's key
    CALENDAR_UNCOVERED = "CALENDAR_UNCOVERED"
    ARTIFACT_EXPIRED = "ARTIFACT_EXPIRED"
    ACTIONABILITY_INVALID = "ACTIONABILITY_INVALID"
    # Additions: the data path's knowledge date (W8.2). A record whose knowledge date is absent, or
    # later than the decision time, cannot be shown to have been knowable when the decision was
    # made, so it is excluded rather than assumed fresh.
    PUBLISHED_AFTER_DECISION = "PUBLISHED_AFTER_DECISION"
    MISSING_KNOWLEDGE_DATE = "MISSING_KNOWLEDGE_DATE"

    # Tradability
    TRADABILITY_UNKNOWN = "TRADABILITY_UNKNOWN"
    TRADABILITY_REJECTED = "TRADABILITY_REJECTED"

    # Eligibility
    RISK_GATE_REJECT = "RISK_GATE_REJECT"

    # Projection
    RATING_UNKNOWN = "RATING_UNKNOWN"
    UNSIZEABLE = "UNSIZEABLE"
    FLOAT_CONVERSION_OVERFLOW = "FLOAT_CONVERSION_OVERFLOW"

    # Execution
    ENGINE_RISK_LIMIT = "ENGINE_RISK_LIMIT"


# The stage each code is produced by. A code is refused with the stage it belongs to, so a refusal's
# stage is derived from its code rather than passed beside it.
_CODE_STAGES: dict[RefusalCode, Stage] = {
    RefusalCode.SCHEMA_INVALID: Stage.ADMISSION,
    RefusalCode.HASH_INVALID: Stage.ADMISSION,
    RefusalCode.PRODUCER_UNKNOWN: Stage.ADMISSION,
    RefusalCode.PRODUCER_UNAUTHORIZED: Stage.ADMISSION,
    RefusalCode.INSTRUMENT_UNKNOWN: Stage.ADMISSION,
    RefusalCode.ALLOCATION_INVALID: Stage.ADMISSION,
    RefusalCode.DUPLICATE: Stage.ADMISSION,
    RefusalCode.MISSING_EXPIRY: Stage.AVAILABILITY,
    RefusalCode.PRODUCED_AT_ABSENT: Stage.AVAILABILITY,
    RefusalCode.CALENDAR_MISSING: Stage.AVAILABILITY,
    RefusalCode.CALENDAR_UNCOVERED: Stage.AVAILABILITY,
    RefusalCode.ARTIFACT_EXPIRED: Stage.AVAILABILITY,
    RefusalCode.ACTIONABILITY_INVALID: Stage.AVAILABILITY,
    RefusalCode.PUBLISHED_AFTER_DECISION: Stage.AVAILABILITY,
    RefusalCode.MISSING_KNOWLEDGE_DATE: Stage.AVAILABILITY,
    RefusalCode.TRADABILITY_UNKNOWN: Stage.TRADABILITY,
    RefusalCode.TRADABILITY_REJECTED: Stage.TRADABILITY,
    RefusalCode.RISK_GATE_REJECT: Stage.ELIGIBILITY,
    RefusalCode.RATING_UNKNOWN: Stage.PROJECTION,
    RefusalCode.UNSIZEABLE: Stage.PROJECTION,
    RefusalCode.FLOAT_CONVERSION_OVERFLOW: Stage.PROJECTION,
    RefusalCode.ENGINE_RISK_LIMIT: Stage.EXECUTION,
}


def stage_of(code: RefusalCode) -> Stage:
    """
    Return the stage a refusal code is produced by.

    Parameters
    ----------
    code : RefusalCode
        The refusal code.

    Returns
    -------
    Stage

    """
    return _CODE_STAGES[code]


@unique
class Diagnostic(Enum):
    """
    Something worth knowing about an artifact that continued.

    A diagnostic is an attribute rather than a refusal: the restrictive resolution is already the
    correct answer, and the diagnostic is information about the producer or the pipeline.
    """

    GATE_CONFLICT = "GATE_CONFLICT"
    ACTIONABILITY_PAST = "ACTIONABILITY_PAST"


@unique
class Outcome(Enum):
    """
    What became of an admitted artifact.

    The lifecycle branches are kept apart because pooling any two of them is how a desk loses the
    ability to tell a broken pipeline from a cautious one: a decision that yielded no order because
    the policy declined it is not the same record as one that never arrived.
    """

    REFUSED = "REFUSED"
    DUPLICATE = "DUPLICATE"
    NO_SIGNAL = "NO_SIGNAL"
    SIGNAL = "SIGNAL"
    DENIED = "DENIED"
    FILLED = "FILLED"


@unique
class Severity(Enum):
    """
    How restrictive a permission claim, a regime reading or the engine's eligibility is.

    The members are ordered from least to most restrictive, and every aggregation in this package is
    a maximum under that order: the most restrictive input wins, so a permissive value can never
    override a stricter one and the aggregation is order-independent.
    """

    PERMIT = "PERMIT"
    UNCERTAIN = "UNCERTAIN"
    RESTRICT = "RESTRICT"

    @property
    def rank(self) -> int:
        """
        Return the member's position in the restrictive order.
        """
        return _SEVERITY_RANK[self]


_SEVERITY_RANK: dict[Severity, int] = {
    Severity.PERMIT: 0,
    Severity.UNCERTAIN: 1,
    Severity.RESTRICT: 2,
}


def most_restrictive(*severities: Severity) -> Severity:
    """
    Return the most restrictive of the given severities.

    The rule is the design's "the most restrictive value wins", stated once and used everywhere. The
    design writes the same rule as `min(research_severity, engine_eligibility)`, which is this
    maximum under the opposite member ordering; naming it here keeps one implementation of it.

    Parameters
    ----------
    *severities : Severity
        The severities to aggregate. At least one is required.

    Returns
    -------
    Severity

    Raises
    ------
    ValueError
        If no severity is given.

    """
    if not severities:
        raise ValueError("at least one severity is required")

    return max(severities, key=lambda severity: severity.rank)


# The producer's own vocabulary for the risk gate's verdict. The mapping onto the three severities
# is configuration and is pinned by a test, so a producer's vocabulary change is a visible
# configuration change rather than a silent behaviour change. A value outside the mapping resolves
# to RESTRICT, because the safe direction of an unknown value is closed.
GATE_SEVERITY_MAPPING: dict[str, Severity] = {
    "PASS": Severity.PERMIT,
    "WARN": Severity.UNCERTAIN,
    "REJECT": Severity.RESTRICT,
}

# The regime reading's own vocabulary (design 3.2), mapped onto the same three severities. A regime
# reading that cannot establish a directional regime reduces risk rather than closing the gate.
REGIME_SEVERITY_MAPPING: dict[str, Severity] = {
    "PASS": Severity.PERMIT,
    "UNCERTAIN": Severity.UNCERTAIN,
    "REJECT": Severity.RESTRICT,
}

# The names the artifact's own consumer reserves. A producer is refused permission to set them by
# its own validator, so this reader never reads them as claims: a value under one of these names is
# an unknown field, recorded rather than honoured.
RESERVED_PRODUCER_NAMES: tuple[str, ...] = (
    "trade_permission",
    "binding_gate",
    "permission_reason",
)


def gate_severity(
    verdict: object,
    mapping: dict[str, Severity] | None = None,
) -> tuple[Severity, bool]:
    """
    Map a permission claim's value onto a severity.

    Parameters
    ----------
    verdict : object
        The claim's value, as read from the artifact. Any type is accepted, because the artifact
        declares the block as an open object.
    mapping : dict[str, Severity], optional
        The declared mapping. Defaults to `GATE_SEVERITY_MAPPING`.

    Returns
    -------
    tuple[Severity, bool]
        The severity, and whether the value was mapped. An unmapped value resolves to `RESTRICT` and
        reports `False`, so the caller can record the unmapped value while still resolving closed.

    """
    declared = GATE_SEVERITY_MAPPING if mapping is None else mapping
    if isinstance(verdict, str):
        severity = declared.get(verdict.strip().upper())
        if severity is not None:
            return severity, True

    return Severity.RESTRICT, False


@dataclass(frozen=True)
class Refusal:
    """
    A named refusal that travels with the result.

    Parameters
    ----------
    code : RefusalCode
        The reason, from the closed set.
    detail : str
        What was observed, in prose. Never a substitute for the code.

    """

    code: RefusalCode
    detail: str

    @property
    def stage(self) -> Stage:
        """
        Return the stage the refusal was produced by.
        """
        return stage_of(self.code)

    def __str__(self) -> str:
        """
        Return the refusal as its code and detail.
        """
        return f"{self.code.value}: {self.detail}"


@dataclass(frozen=True)
class RefusalRecord:
    """
    Every refusal of one artifact, with the earliest stage's refusal first.

    A stage that would run only after an earlier refusal is not evaluated, so this record is the
    evidence that a refusal reason is deterministic: `reason` is the code of the earliest stage
    present.

    Parameters
    ----------
    refusals : tuple[Refusal, ...]
        The refusals observed, in the order they were produced.
    diagnostics : tuple[Diagnostic, ...]
        Diagnostics recorded before the refusal.

    """

    refusals: tuple[Refusal, ...] = ()
    diagnostics: tuple[Diagnostic, ...] = ()

    @property
    def reason(self) -> RefusalCode | None:
        """
        Return the code of the earliest failing stage, or `None` when nothing was refused.
        """
        if not self.refusals:
            return None

        order = {stage: index for index, stage in enumerate(STAGE_ORDER)}
        return min(self.refusals, key=lambda refusal: order[refusal.stage]).code

    def __bool__(self) -> bool:
        """
        Return whether the record holds at least one refusal.
        """
        return bool(self.refusals)


@dataclass(frozen=True)
class RefusalLog:
    """
    An accumulator for refusals and diagnostics produced while evaluating one artifact.

    The log exists so a caller can observe several failures and still be told the earliest stage's
    reason, which is what makes the refusal a function of the artifact rather than of an evaluation
    order.
    """

    refusals: list[Refusal] = field(default_factory=list)
    diagnostics: list[Diagnostic] = field(default_factory=list)

    def refuse(self, code: RefusalCode, detail: str) -> Refusal:
        """
        Record a refusal and return it.
        """
        refusal = Refusal(code=code, detail=detail)
        self.refusals.append(refusal)
        return refusal

    def diagnose(self, diagnostic: Diagnostic) -> None:
        """
        Record a diagnostic.
        """
        if diagnostic not in self.diagnostics:
            self.diagnostics.append(diagnostic)

    @property
    def reason(self) -> RefusalCode | None:
        """
        Return the code of the earliest failing stage, or `None` when nothing was refused.
        """
        return self.record.reason

    @property
    def record(self) -> RefusalRecord:
        """
        Return an immutable view of what the log holds.
        """
        return RefusalRecord(
            refusals=tuple(self.refusals),
            diagnostics=tuple(self.diagnostics),
        )
