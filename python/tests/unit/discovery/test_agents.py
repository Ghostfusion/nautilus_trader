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
Tests for the agent contract: the five roles, their prompts and the structured-output path.

The invocations are recorded and replayed, so a response is reproducible without recalling the
model, and it is never accepted without the validator the language itself uses.
"""

from __future__ import annotations

import json
from dataclasses import replace

import pytest

from nautilus_trader.discovery.agents import AgentContext
from nautilus_trader.discovery.agents import AgentOutcome
from nautilus_trader.discovery.agents import AgentRequest
from nautilus_trader.discovery.agents import Contamination
from nautilus_trader.discovery.agents import Language
from nautilus_trader.discovery.agents import ReplayModel
from nautilus_trader.discovery.agents import Role
from nautilus_trader.discovery.agents import classify
from nautilus_trader.discovery.agents import invoke
from nautilus_trader.discovery.prompts import Prompt
from nautilus_trader.discovery.schema import RefusalCode
from nautilus_trader.optimization.space import digest_of
from tests.unit.discovery.fixtures import REMOVE
from tests.unit.discovery.fixtures import changed
from tests.unit.discovery.fixtures import fixture_policy
from tests.unit.discovery.fixtures import fixture_registry
from tests.unit.discovery.fixtures import legal_payload


# One value per field each role declares, so a prompt can be rendered from its own field set and no
# other.
ROLE_VALUES: dict[Role, dict[str, str]] = {
    Role.RESEARCH_SCIENTIST: {
        "families": "trend, mean_reversion",
        "features": "ema_close",
        "mechanism_rules": "a mechanism must name its horizon",
        "information_set": "the campaign scope and the registered features",
    },
    Role.STRATEGY_ENGINEER: {
        "families": "trend, mean_reversion",
        "features": "ema_close",
        "bounds": "lookback in [5, 60]",
        "declared_sets": "direction in {long, short}",
        "information_set": "the hypothesis and the declared bounds",
    },
    Role.EXPERIMENT_ANALYST: {
        "ledger_record": "candidate abc, fold one, return 0.01",
        "information_set": "the ledger record, read-only",
    },
    Role.ADVERSARIAL_REVIEWER: {
        "evidence_package": "the evidence package for candidate abc",
        "information_set": "the evidence package, read-only",
    },
    Role.SEARCH_STRATEGIST: {
        "population": "candidate abc, candidate def",
        "objectives": "sharpe, turnover",
        "policy": "search policy one",
        "information_set": "the population and the development objectives",
    },
}


def request_of(role: Role) -> AgentRequest:
    """Return the invocation the role's own declared fields render."""
    return AgentRequest.of(role, values=ROLE_VALUES[role])


def language() -> Language:
    """Return the fixture language a candidate proposal is validated against."""
    return Language(policy=fixture_policy(), registry=fixture_registry())


def replay_for(request: AgentRequest, response: str) -> ReplayModel:
    """Return a replay model holding one recorded response for one invocation."""
    return ReplayModel().record(request, response)


def context() -> AgentContext:
    """Return invocation metadata that names the model."""
    return AgentContext(model_identifier="model.fixture/v1")


def test_the_five_roles_each_declare_a_prompt_fields_and_their_authority() -> None:
    """Each role names a prompt, the fields it is given, and what it produces and may not do."""
    assert len(Role) == 5
    assert {role.prompt.prompt_id for role in Role} == {role.value for role in Role}
    assert len({role.produces for role in Role}) == 5
    assert len({role.may_not for role in Role}) == 5

    for role in Role:
        assert role.fields
        assert len(set(role.fields)) == len(role.fields)
        assert role.produces
        assert role.may_not

        serialised = role.to_dict()
        assert serialised["role"] == role.value
        assert serialised["fields"] == list(role.fields)
        assert serialised["produces"] == role.produces
        assert serialised["may_not"] == role.may_not
        assert serialised["prompt_version"] == role.prompt.version
        assert serialised["prompt_digest"] == role.prompt.digest()


def test_a_role_prompt_renders_its_declared_fields_and_refuses_any_other() -> None:
    """Every role renders from exactly its field set, and a missing or undeclared field raises."""
    for role in Role:
        prompt = role.prompt
        values = ROLE_VALUES[role]

        assert prompt.render(**values) == prompt.template.format(**values)

        absent = prompt.fields[0]
        with pytest.raises(ValueError, match=absent):
            prompt.render(**{name: value for name, value in values.items() if name != absent})

        with pytest.raises(ValueError, match="metric_value"):
            prompt.render(**values, metric_value="0.5")


def test_a_prompt_digest_tracks_its_template_and_its_version() -> None:
    """A version or a template change is a different identity, and the same prompt is stable."""
    prompt = Prompt(prompt_id="fixture", version="1", template="a {x} b", fields=("x",))
    again = Prompt(prompt_id="fixture", version="1", template="a {x} b", fields=("x",))

    assert prompt.digest() == again.digest()
    assert prompt.digest() != replace(prompt, version="2").digest()
    assert prompt.digest() != replace(prompt, template="a {x} c").digest()


def test_a_request_renders_only_its_roles_fields_and_digests_them() -> None:
    """The instructions are what a replay is keyed by, and an undeclared field is refused."""
    role = Role.RESEARCH_SCIENTIST
    request = request_of(role)

    assert request.role is role
    assert request.information_set == ROLE_VALUES[role]["information_set"]
    assert ROLE_VALUES[role]["information_set"] in request.instructions

    same = request_of(role)
    assert request.input_digest == same.input_digest
    assert request.to_dict()["input_digest"] == request.input_digest

    other = AgentRequest(
        role=role,
        instructions=request.instructions + " one more sentence",
        information_set="another information set",
    )
    assert other.input_digest != request.input_digest

    with pytest.raises(ValueError, match="was given"):
        AgentRequest.of(role, values={**ROLE_VALUES[role], "metric_value": "0.5"})


def test_an_agent_context_needs_a_model_identifier() -> None:
    """A record without a model identifier is not evidence, so the context refuses to exist."""
    assert AgentContext(model_identifier="model/v1").model_identifier == "model/v1"

    for identifier in ("", "   ", "\t"):
        with pytest.raises(ValueError, match="model_identifier"):
            AgentContext(model_identifier=identifier)


def test_a_context_classifies_its_period_against_the_cutoff() -> None:
    """The context reports the standing vulnerability of the period it covers."""
    vulnerable = AgentContext(
        model_identifier="model/v1",
        knowledge_cutoff="2024-01-01",
        period_start="2023-12-31",
    )
    unknown = AgentContext(model_identifier="model/v1")

    assert vulnerable.contamination is Contamination.VULNERABLE
    assert vulnerable.to_dict()["contamination"] == "vulnerable"
    assert unknown.contamination is Contamination.UNKNOWN


def test_classify_reads_the_period_start_against_the_cutoff() -> None:
    """An absent date is unknown, a later period is post-cutoff, and a boundary is vulnerable."""
    assert classify(knowledge_cutoff="", period_start="2024-01-01") is Contamination.UNKNOWN
    assert classify(knowledge_cutoff="2024-01-01", period_start="") is Contamination.UNKNOWN

    later = classify(knowledge_cutoff="2023-01-01", period_start="2024-01-01")
    assert later is Contamination.POST_CUTOFF

    assert classify(knowledge_cutoff="2024-01-01", period_start="2024-01-01") is (
        Contamination.VULNERABLE
    )
    assert classify(knowledge_cutoff="2024-06-01", period_start="2024-01-01") is (
        Contamination.VULNERABLE
    )


def test_a_replay_model_replays_only_what_it_recorded() -> None:
    """An unrecorded invocation raises, recording accumulates, and the evidence is sorted."""
    first = request_of(Role.EXPERIMENT_ANALYST)
    second = request_of(Role.ADVERSARIAL_REVIEWER)
    empty = ReplayModel()

    with pytest.raises(KeyError, match="no recorded response"):
        empty(first)

    model = empty.record(first, "first response")
    assert model(first) == "first response"

    with pytest.raises(KeyError, match="no recorded response"):
        model(second)

    accumulated = model.record(second, "second response")
    assert accumulated(first) == "first response"
    assert accumulated(second) == "second response"
    assert model.as_evidence() == {first.input_digest: "first response"}

    evidence = accumulated.as_evidence()
    assert list(evidence) == sorted(evidence)
    assert evidence == {
        first.input_digest: "first response",
        second.input_digest: "second response",
    }


def test_an_unstructured_response_is_recorded_as_a_refusal() -> None:
    """A response that is not JSON is refused and still recorded rather than lost."""
    role = Role.EXPERIMENT_ANALYST
    request = request_of(role)
    response = "not json at all"

    result = invoke(replay_for(request, response), request, context=context())

    assert result.outcome is AgentOutcome.UNSTRUCTURED
    assert result.outcome.is_refusal is True
    assert result.accepted is False
    assert result.insufficient is False
    assert result.payload is None
    assert result.validation is None
    assert result.refusals == ()
    assert result.reason
    assert result.record.outcome is AgentOutcome.UNSTRUCTURED
    assert result.record.request == request
    assert result.record.response_digest == digest_of({"response": response})
    assert result.record.context == context()
    assert result.record.to_dict()["validation_outcome"] == "unstructured"


def test_a_json_array_is_not_read_as_an_object() -> None:
    """Only a JSON object is a response; an array, a scalar and null are unstructured."""
    request = request_of(Role.ADVERSARIAL_REVIEWER)

    for response in ("[1, 2]", '[{"findings": []}]', '"text"', "42", "null", "true"):
        result = invoke(replay_for(request, response), request, context=context())

        assert result.outcome is AgentOutcome.UNSTRUCTURED
        assert result.payload is None
        assert result.accepted is False


def test_an_explicit_insufficient_data_reply_is_a_complete_result() -> None:
    """A role that declares its information is not enough is recorded, not refused."""
    request = request_of(Role.STRATEGY_ENGINEER)
    reply = {"outcome": "insufficient_data", "reason": "no declared bound fits the hypothesis"}
    response = json.dumps(reply)

    result = invoke(replay_for(request, response), request, context=context())

    assert result.outcome is AgentOutcome.INSUFFICIENT_DATA
    assert result.outcome.is_refusal is False
    assert result.insufficient is True
    assert result.accepted is False
    assert result.reason == "no declared bound fits the hypothesis"
    assert result.payload == reply
    assert result.validation is None
    assert result.refusals == ()


def test_an_analyst_report_is_accepted_without_a_language() -> None:
    """A role that proposes no candidate is accepted, because there is no candidate to validate."""
    request = request_of(Role.EXPERIMENT_ANALYST)
    report = {
        "explanation": "the record shows a positive fold",
        "unsupported_claims": [],
        "recommendation": "replicate over the next fold",
    }

    result = invoke(replay_for(request, json.dumps(report)), request, context=context())

    assert result.outcome is AgentOutcome.ACCEPTED
    assert result.outcome.is_refusal is False
    assert result.accepted is True
    assert result.insufficient is False
    assert result.payload == report
    assert result.validation is None
    assert result.refusals == ()
    assert result.reason == ""


def test_a_candidate_proposal_is_accepted_only_through_the_languages_validator() -> None:
    """The engineer's proposal is accepted because the language's own validation accepted it."""
    request = request_of(Role.STRATEGY_ENGINEER)
    payload = legal_payload()
    response = json.dumps({"specification": payload})
    lang = language()

    result = invoke(replay_for(request, response), request, context=context(), language=lang)

    assert result.outcome is AgentOutcome.ACCEPTED
    assert result.accepted is True
    assert result.refusals == ()
    assert result.reason == ""
    assert result.payload == {"specification": payload}
    assert result.validation is not None
    assert result.validation.accepted is True
    assert result.validation == lang.validate(payload)


def test_a_proposal_the_language_refuses_carries_the_languages_refusals() -> None:
    """A specification the language refuses is refused with the reason the language gave."""
    request = request_of(Role.STRATEGY_ENGINEER)
    specification = changed(legal_payload(), {"identity.mechanism": REMOVE})
    lang = language()
    expected = lang.validate(specification)
    response = json.dumps({"specification": specification})

    result = invoke(replay_for(request, response), request, context=context(), language=lang)

    assert expected.accepted is False
    assert expected.errors
    assert result.outcome is AgentOutcome.SCHEMA_VIOLATION
    assert result.outcome.is_refusal is True
    assert result.accepted is False
    assert result.validation == expected
    assert result.refusals == expected.errors
    assert result.reason == expected.errors[0].message
    assert result.payload == {"specification": specification}
    assert (result.refusals[0].code, result.refusals[0].path) == (
        RefusalCode.FIELD_MISSING,
        "identity.mechanism",
    )


def test_a_proposal_without_a_specification_block_is_refused() -> None:
    """An engineer response without the specification key is a violation, not a proposal."""
    request = request_of(Role.STRATEGY_ENGINEER)
    reply = {"hypothesis": "a candidate without its specification"}
    response = json.dumps(reply)

    result = invoke(replay_for(request, response), request, context=context())

    assert result.outcome is AgentOutcome.SCHEMA_VIOLATION
    assert result.accepted is False
    assert result.validation is None
    assert len(result.refusals) == 1

    refusal = result.refusals[0]
    assert refusal.code is RefusalCode.BLOCK_MISSING
    assert refusal.path == "specification"
    assert refusal.message
    assert result.reason == refusal.message
    assert result.payload == reply


def test_a_candidate_proposal_without_a_language_is_refused_not_accepted() -> None:
    """A proposal offered without its validator raises rather than being accepted unvalidated."""
    request = request_of(Role.STRATEGY_ENGINEER)
    response = json.dumps({"specification": legal_payload()})

    with pytest.raises(ValueError, match="needs the language"):
        invoke(replay_for(request, response), request, context=context())
