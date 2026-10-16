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
The agent contract: five roles, their recorded invocations, and the structured-output path.

The separation of authority between the roles is not enforced by prompt wording. It is enforced by
the objects each role is given: the analyst receives a ledger record and cannot write one, the
reviewer receives an evidence package read-only, and no role receives an engine handle, a metric
implementation, a credential, or a holdout path. A prompt therefore declares exactly the fields the
role is given, and `Prompt.render` refuses a call that supplies a field the template does not name,
which is what keeps a metric out of a brief that is not allowed to contain one.

Structured output is validated by the same validator the language uses, so an agent cannot smuggle a
key the schema does not have. A refusal is a first-class result: a response that cannot produce a
testable hypothesis returns an explicit insufficient-data outcome, which is recorded, and the
campaign continues.

The model is reached through an injected callable, and this module holds no model dependency and
reads no credential. Determinism has a caveat that belongs in the code rather than in a comment:
model output is not deterministic across a provider version or a sampling configuration, so a run's
reproducibility claim covers the compiled candidate and its backtest, and an invocation is
reproduced by replaying its recorded response rather than by calling the model again. `ReplayModel`
is that mechanism, and it is part of the module rather than of the tests because a campaign has to
stay reproducible when a provider disappears or its sampling behaviour changes.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from dataclasses import field
from enum import Enum
from typing import TYPE_CHECKING
from typing import Protocol
from typing import runtime_checkable

from nautilus_trader.discovery.prompts import Prompt
from nautilus_trader.discovery.prompts import adversarial_reviewer
from nautilus_trader.discovery.prompts import experiment_analyst
from nautilus_trader.discovery.prompts import research_scientist
from nautilus_trader.discovery.prompts import search_strategist
from nautilus_trader.discovery.prompts import strategy_engineer
from nautilus_trader.discovery.schema import Refusal
from nautilus_trader.discovery.schema import RefusalCode
from nautilus_trader.discovery.validate import ValidationResult
from nautilus_trader.discovery.validate import validate_specification
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping

    from nautilus_trader.discovery.policy import DiscoveryPolicy
    from nautilus_trader.discovery.validate import FeatureRegistry


AGENT_PROTOCOL_VERSION = "discovery.agents/v1"

# The one key a response declares to say that the information it was given is not enough. It is
# checked before the schema, because an insufficient-data reply is a result rather than a malformed
# candidate.
INSUFFICIENT_DATA_KEY = "outcome"
INSUFFICIENT_DATA_VALUE = "insufficient_data"

# The key a candidate proposal is read from.
SPECIFICATION_KEY = "specification"

# An ISO calendar date, whose shape is checked before two dates are compared as strings.
_ISO_DATE_LENGTH = 10
_ISO_DATE_PART_WIDTHS: tuple[int, ...] = (4, 2, 2)


class Role(Enum):
    """
    The five roles, with what each one produces and what each one is not given.

    The roles are the design's, and the authority column does the work: a role is defined as much by
    the fields it never sees as by the fields it produces.
    """

    RESEARCH_SCIENTIST = "research_scientist"
    STRATEGY_ENGINEER = "strategy_engineer"
    EXPERIMENT_ANALYST = "experiment_analyst"
    ADVERSARIAL_REVIEWER = "adversarial_reviewer"
    SEARCH_STRATEGIST = "search_strategist"

    @property
    def prompt(self) -> Prompt:
        """
        The prompt version the role is invoked with.

        Returns
        -------
        Prompt

        """
        return _PROMPTS[self]

    @property
    def fields(self) -> tuple[str, ...]:
        """
        The fields the role is given, and the whole of what it is given.

        Returns
        -------
        tuple[str, ...]

        """
        return self.prompt.fields

    @property
    def produces(self) -> str:
        """
        What the role produces.

        Returns
        -------
        str

        """
        return _PRODUCES[self]

    @property
    def may_not(self) -> str:
        """
        What the role may not do, which is the other half of its definition.

        Returns
        -------
        str

        """
        return _MAY_NOT[self]

    def to_dict(self) -> dict[str, object]:
        """
        Return the role as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "role": self.value,
            "produces": self.produces,
            "may_not": self.may_not,
            "fields": list(self.fields),
            "prompt_id": self.prompt.prompt_id,
            "prompt_version": self.prompt.version,
            "prompt_digest": self.prompt.digest(),
        }


_PRODUCES: Mapping[Role, str] = {
    Role.RESEARCH_SCIENTIST: (
        "a falsifiable hypothesis with its mechanism, its required features, and its strongest "
        "alternative explanation"
    ),
    Role.STRATEGY_ENGINEER: "a candidate specification in the declared language",
    Role.EXPERIMENT_ANALYST: "an explanation of the recorded evidence and a recommendation",
    Role.ADVERSARIAL_REVIEWER: (
        "findings with severity, an evidence reference, and a reproducible test"
    ),
    Role.SEARCH_STRATEGIST: (
        "the next experiment, as a bounded mutation or recombination with its rationale and cost"
    ),
}

_MAY_NOT: Mapping[Role, str] = {
    Role.RESEARCH_SCIENTIST: "name a metric value, a threshold, or a result",
    Role.STRATEGY_ENGINEER: (
        "invent a feature, exceed a bound, or claim the candidate has passed anything"
    ),
    Role.EXPERIMENT_ANALYST: (
        "compute an authoritative metric, alter a threshold, or decide eligibility"
    ),
    Role.ADVERSARIAL_REVIEWER: "waive a gate, or authorise anything",
    Role.SEARCH_STRATEGIST: "read the holdout, change the policy, or duplicate a candidate",
}

_PROMPTS: Mapping[Role, Prompt] = {
    Role.RESEARCH_SCIENTIST: research_scientist.RESEARCH_SCIENTIST,
    Role.STRATEGY_ENGINEER: strategy_engineer.STRATEGY_ENGINEER,
    Role.EXPERIMENT_ANALYST: experiment_analyst.EXPERIMENT_ANALYST,
    Role.ADVERSARIAL_REVIEWER: adversarial_reviewer.ADVERSARIAL_REVIEWER,
    Role.SEARCH_STRATEGIST: search_strategist.SEARCH_STRATEGIST,
}


class AgentOutcome(Enum):
    """
    What validating an agent's response produced.

    A refusal is a first-class result rather than an error: an unstructured response, a response
    that does not satisfy the schema, and an explicit insufficient-data reply are three different
    outcomes, and each is recorded.
    """

    ACCEPTED = "accepted"
    UNSTRUCTURED = "unstructured"
    SCHEMA_VIOLATION = "schema_violation"
    INSUFFICIENT_DATA = "insufficient_data"

    @property
    def is_refusal(self) -> bool:
        """
        Whether the outcome is a refusal rather than an accepted proposal.

        Returns
        -------
        bool

        """
        return self in (AgentOutcome.UNSTRUCTURED, AgentOutcome.SCHEMA_VIOLATION)


class Contamination(Enum):
    """
    How vulnerable a result is to hypotheses the model may already know.

    The classification is standing rather than computed per result: a candidate evaluated over a
    period the model could plausibly know is marked as more vulnerable than one evaluated after its
    cutoff, and no gate in this design can convert the first into the second.
    """

    POST_CUTOFF = "post_cutoff"
    VULNERABLE = "vulnerable"
    UNKNOWN = "unknown"


def classify(*, knowledge_cutoff: str, period_start: str) -> Contamination:
    """
    Classify a period against a declared knowledge cutoff.

    Both values are ISO calendar dates, and a value that is not one is classified as unknown rather
    than compared: a lexicographic comparison of dates is only sound when they are zero-padded and
    the same length, so the shape is checked rather than assumed.

    Parameters
    ----------
    knowledge_cutoff : str
        The model's declared or known cutoff, as an ISO calendar date.
    period_start : str
        The earliest date of the period the result covers, as an ISO calendar date.

    Returns
    -------
    Contamination

    """
    if not _is_iso_date(knowledge_cutoff) or not _is_iso_date(period_start):
        return Contamination.UNKNOWN
    if period_start > knowledge_cutoff:
        return Contamination.POST_CUTOFF
    return Contamination.VULNERABLE


def _is_iso_date(value: str) -> bool:
    if len(value) != _ISO_DATE_LENGTH:
        return False
    parts = value.split("-")
    if len(parts) != len(_ISO_DATE_PART_WIDTHS):
        return False
    return all(
        part.isdigit() and len(part) == width
        for part, width in zip(parts, _ISO_DATE_PART_WIDTHS, strict=True)
    )


@dataclass(frozen=True)
class AgentContext:
    """
    The invocation metadata a caller supplies, which is recorded rather than inferred.

    Parameters
    ----------
    model_identifier : str
        The model and version, as the operator declares it.
    sampling : Mapping[str, object]
        The sampling configuration the response was produced under.
    knowledge_cutoff : str
        The model's declared or known knowledge cutoff, empty when none is available.
    generated_on : str
        The date the invocation ran.
    period_start : str
        The earliest date of the market period the resulting candidate is evaluated over.

    """

    model_identifier: str
    sampling: Mapping[str, object] = field(default_factory=dict)
    knowledge_cutoff: str = ""
    generated_on: str = ""
    period_start: str = ""

    def __post_init__(self) -> None:
        """
        Refuse a context that does not name the model, because a record without one is not evidence.
        """
        if not self.model_identifier.strip():
            raise ValueError("model_identifier must not be empty")

    @property
    def contamination(self) -> Contamination:
        """
        The standing vulnerability classification for the period.

        Returns
        -------
        Contamination

        """
        return classify(
            knowledge_cutoff=self.knowledge_cutoff,
            period_start=self.period_start,
        )

    def to_dict(self) -> dict[str, object]:
        """
        Return the context as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "model_identifier": self.model_identifier,
            "sampling": dict(self.sampling),
            "knowledge_cutoff": self.knowledge_cutoff,
            "generated_on": self.generated_on,
            "period_start": self.period_start,
            "contamination": self.contamination.value,
        }


@dataclass(frozen=True)
class AgentRequest:
    """
    One invocation: the role, the rendered instructions, and the identity of both.

    Parameters
    ----------
    role : Role
        The role being invoked.
    instructions : str
        The rendered prompt.
    information_set : str
        The statement of what the role was given, which travels with the record.

    """

    role: Role
    instructions: str
    information_set: str

    @property
    def prompt(self) -> Prompt:
        """
        The prompt version that produced the instructions.

        Returns
        -------
        Prompt

        """
        return self.role.prompt

    @property
    def input_digest(self) -> str:
        """
        The digest of the instructions, which is what a replay is keyed by.

        Returns
        -------
        str

        """
        return digest_of(
            {
                "role": self.role.value,
                "prompt_id": self.prompt.prompt_id,
                "prompt_version": self.prompt.version,
                "prompt_digest": self.prompt.digest(),
                "instructions": self.instructions,
            },
        )

    def to_dict(self) -> dict[str, object]:
        """
        Return the request as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "role": self.role.value,
            "prompt_id": self.prompt.prompt_id,
            "prompt_version": self.prompt.version,
            "prompt_digest": self.prompt.digest(),
            "information_set": self.information_set,
            "input_digest": self.input_digest,
        }

    @classmethod
    def of(cls, role: Role, *, values: Mapping[str, object]) -> AgentRequest:
        """
        Render a role's prompt from the fields the role is given, and no others.

        Parameters
        ----------
        role : Role
            The role to invoke.
        values : Mapping[str, object]
            The prompt's field values, one per declared field.

        Returns
        -------
        AgentRequest

        """
        return cls(
            role=role,
            instructions=role.prompt.render(**values),
            information_set=str(values.get("information_set", "")),
        )


@dataclass(frozen=True)
class Language:
    """
    The policy and registry a candidate proposal is validated against.

    A proposal is data until the language's own validator has read it, so the validator travels with
    the policy and the registry it resolves features against rather than being optional at a call
    site.

    Parameters
    ----------
    policy : DiscoveryPolicy
        The policy the proposal is read against.
    registry : FeatureRegistry
        The feature registry the proposal's declarations resolve against.

    """

    policy: DiscoveryPolicy
    registry: FeatureRegistry

    def validate(self, payload: object) -> ValidationResult:
        """
        Validate a candidate specification with the language's own validator.

        Parameters
        ----------
        payload : object
            The candidate specification.

        Returns
        -------
        ValidationResult

        """
        return validate_specification(payload, self.policy, self.registry)


@runtime_checkable
class AgentModel(Protocol):
    """
    The injected callable a role is reached through.

    The repository has no model dependency, so the provider is the operator's choice: an adapter
    implementing this protocol may reach any provider, and the credential it needs is read from the
    environment by the operator rather than by this module.
    """

    def __call__(self, request: AgentRequest) -> str:
        """
        Return the model's response to an invocation.

        Parameters
        ----------
        request : AgentRequest
            The invocation.

        Returns
        -------
        str

        """
        ...


@dataclass(frozen=True)
class ReplayModel:
    """
    The recorded responses a campaign replays instead of recalling a model.

    Parameters
    ----------
    responses : Mapping[str, str]
        Input digest to the response that was recorded for it.

    """

    responses: Mapping[str, str] = field(default_factory=dict)

    def __call__(self, request: AgentRequest) -> str:
        """
        Return the recorded response for an invocation.

        Parameters
        ----------
        request : AgentRequest
            The invocation.

        Returns
        -------
        str

        Raises
        ------
        KeyError
            If no response was recorded for this invocation, because a replay that invents a
            response is not a replay.

        """
        digest = request.input_digest
        if digest not in self.responses:
            raise KeyError(f"no recorded response for invocation {digest}")
        return self.responses[digest]

    def record(self, request: AgentRequest, response: str) -> ReplayModel:
        """
        Return a replay model with one more recorded response.

        Parameters
        ----------
        request : AgentRequest
            The invocation the response belongs to.
        response : str
            The response as it was received.

        Returns
        -------
        ReplayModel

        """
        return ReplayModel(responses={**self.responses, request.input_digest: response})

    def as_evidence(self) -> dict[str, str]:
        """
        Return the recorded responses as the evidence a campaign keeps.

        Returns
        -------
        dict[str, str]

        """
        return dict(sorted(self.responses.items()))


@dataclass(frozen=True)
class InvocationRecord:
    """
    The record of one agent invocation, which is evidence and not a transcript.

    Parameters
    ----------
    request : AgentRequest
        The invocation.
    context : AgentContext
        The metadata the invocation ran under.
    response_digest : str
        The digest of the response as it was received.
    outcome : AgentOutcome
        What validating the response produced.

    """

    request: AgentRequest
    context: AgentContext
    response_digest: str
    outcome: AgentOutcome

    def to_dict(self) -> dict[str, object]:
        """
        Return the record as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "schema": AGENT_PROTOCOL_VERSION,
            "invocation": self.request.to_dict(),
            "context": self.context.to_dict(),
            "response_digest": self.response_digest,
            "validation_outcome": self.outcome.value,
        }


@dataclass(frozen=True)
class AgentResult:
    """
    The outcome of one invocation: the record, the proposal, and any refusal.

    Parameters
    ----------
    record : InvocationRecord
        The invocation record, which exists whether or not the response was accepted.
    outcome : AgentOutcome
        What validating the response produced.
    payload : Mapping[str, object] | None
        The parsed proposal, when the response was structured JSON at all.
    validation : ValidationResult | None
        The language's own validation result, when the response reached it.
    reason : str
        Why the response was refused, empty when it was accepted.
    refusals : tuple[Refusal, ...]
        The language's refusals, empty when the response never reached the language's validator.

    """

    record: InvocationRecord
    outcome: AgentOutcome
    payload: Mapping[str, object] | None = None
    validation: ValidationResult | None = None
    reason: str = ""
    refusals: tuple[Refusal, ...] = ()

    @property
    def accepted(self) -> bool:
        """
        Whether the response produced an accepted proposal.

        Returns
        -------
        bool

        """
        return self.outcome is AgentOutcome.ACCEPTED

    @property
    def insufficient(self) -> bool:
        """
        Whether the role declared that its information was not enough.

        Returns
        -------
        bool

        """
        return self.outcome is AgentOutcome.INSUFFICIENT_DATA

    def to_dict(self) -> dict[str, object]:
        """
        Return the result as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "record": self.record.to_dict(),
            "outcome": self.outcome.value,
            "payload": None if self.payload is None else dict(self.payload),
            "reason": self.reason,
            "refusals": [refusal.to_dict() for refusal in self.refusals],
        }


def invoke(
    model: AgentModel,
    request: AgentRequest,
    *,
    context: AgentContext,
    language: Language | None = None,
) -> AgentResult:
    """
    Invoke a role, validate its response, and record the invocation either way.

    The prompt's declared fields are the whole of what the role is given, so a request carrying a
    field the template does not name is refused by `Prompt.render` rather than passed through. A
    response is read as JSON, then as an insufficient-data result, then by the language's own
    validator, and the invocation record is produced in every case.

    Parameters
    ----------
    model : AgentModel
        The injected callable the role is reached through.
    request : AgentRequest
        The rendered invocation.
    context : AgentContext
        The invocation metadata.
    language : Language | None
        The policy and registry a candidate proposal is validated against.

    Returns
    -------
    AgentResult

    Raises
    ------
    ValueError
        If a candidate proposal is offered without the language to validate it against, because
        accepting an unvalidated proposal is what the contract exists to prevent.

    """
    response = model(request)
    response_digest = digest_of({"response": response})
    outcome, payload, validation, reason, refusals = _validate(
        response,
        role=request.role,
        language=language,
    )
    record = InvocationRecord(
        request=request,
        context=context,
        response_digest=response_digest,
        outcome=outcome,
    )
    return AgentResult(
        record=record,
        outcome=outcome,
        payload=payload,
        validation=validation,
        reason=reason,
        refusals=refusals,
    )


def _validate(
    response: str,
    *,
    role: Role,
    language: Language | None,
) -> tuple[
    AgentOutcome,
    Mapping[str, object] | None,
    ValidationResult | None,
    str,
    tuple[Refusal, ...],
]:
    loaded = _load(response)
    if loaded is None:
        return (
            AgentOutcome.UNSTRUCTURED,
            None,
            None,
            "the response is not a JSON object",
            (),
        )

    declared = loaded.get(INSUFFICIENT_DATA_KEY)
    if isinstance(declared, str) and declared == INSUFFICIENT_DATA_VALUE:
        return (
            AgentOutcome.INSUFFICIENT_DATA,
            loaded,
            None,
            str(loaded.get("reason", "")),
            (),
        )

    if role is not Role.STRATEGY_ENGINEER:
        # The other four roles produce prose that no candidate schema governs: the analyst explains
        # a record, the reviewer reports findings, the scientist states a hypothesis and the
        # strategist proposes a change. What is checked for them is that the response is a JSON
        # object, and the authority that limits them is the field set they were given.
        return (AgentOutcome.ACCEPTED, loaded, None, "", ())

    candidate = loaded.get(SPECIFICATION_KEY)
    if not isinstance(candidate, dict):
        refusal = Refusal(
            code=RefusalCode.BLOCK_MISSING,
            path=SPECIFICATION_KEY,
            message=f"a candidate proposal carries its specification under {SPECIFICATION_KEY}",
        )
        return (
            AgentOutcome.SCHEMA_VIOLATION,
            loaded,
            None,
            refusal.message,
            (refusal,),
        )

    if language is None:
        raise ValueError("a candidate proposal needs the language to be validated against")

    validation = language.validate(candidate)
    if validation.accepted:
        return (AgentOutcome.ACCEPTED, loaded, validation, "", ())

    first = validation.errors[0] if validation.errors else None
    return (
        AgentOutcome.SCHEMA_VIOLATION,
        loaded,
        validation,
        "" if first is None else first.message,
        validation.errors,
    )


def _load(response: str) -> Mapping[str, object] | None:
    try:
        loaded = json.loads(response)
    except json.JSONDecodeError:
        return None
    if not isinstance(loaded, dict):
        return None
    return loaded
