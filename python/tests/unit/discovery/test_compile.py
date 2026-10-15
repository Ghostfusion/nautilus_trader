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
Tests for the compiler's fifth stage and the two identities a compiled plan carries.

The behavioural identity is restated here in full rather than compared against the implementation's
own computation of it, so a key added to or removed from the plan's inputs is a visible change to a
test rather than a silent change to an identity.
"""

from __future__ import annotations

from typing import Any
from typing import cast

import nautilus_trader
from nautilus_trader.discovery.compile import ENGINE_VERSION_UNKNOWN
from nautilus_trader.discovery.compile import CompilationResult
from nautilus_trader.discovery.compile import CompiledPlan
from nautilus_trader.discovery.compile import compile_specification
from nautilus_trader.discovery.compile import compose_plan
from nautilus_trader.discovery.compile import engine_version
from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.policy import DiscoveryPolicy
from nautilus_trader.discovery.schema import SPECIFICATION_SCHEMA_VERSION
from nautilus_trader.discovery.schema import RefusalCode
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.discovery.validate import SESSION_NS
from nautilus_trader.discovery.validate import WarningCode
from nautilus_trader.optimization.assumptions import BarAmbiguityPolicy
from nautilus_trader.optimization.space import digest_of
from tests.unit.discovery.fixtures import ALIAS
from tests.unit.discovery.fixtures import FEATURE_ID
from tests.unit.discovery.fixtures import LEGAL_LOOKBACK
from tests.unit.discovery.fixtures import REMOVE
from tests.unit.discovery.fixtures import UNREGISTERED_FEATURE_ID
from tests.unit.discovery.fixtures import changed
from tests.unit.discovery.fixtures import fixture_policy
from tests.unit.discovery.fixtures import fixture_registry
from tests.unit.discovery.fixtures import legal_payload


def _compile(
    changes: dict[str, Any] | None = None,
    policy: DiscoveryPolicy | None = None,
    registry: FeatureRegistry | None = None,
) -> CompilationResult:
    """
    Return the compilation of the fixture payload with the given changes applied.
    """
    return compile_specification(
        changed(legal_payload(), {} if changes is None else changes),
        fixture_policy() if policy is None else policy,
        fixture_registry() if registry is None else registry,
    )


def _plan(
    changes: dict[str, Any] | None = None,
    policy: DiscoveryPolicy | None = None,
    registry: FeatureRegistry | None = None,
) -> CompiledPlan:
    """
    Return the plan of the fixture payload, which every accepted case compiles to.
    """
    compiled = _compile(changes, policy, registry)

    assert compiled.plan is not None

    return compiled.plan


def test_a_legal_specification_compiles_with_both_identities() -> None:
    """
    Test both identities of a legal plan digest the inputs the design names.
    """
    policy = fixture_policy()
    registry = fixture_registry()
    plan = _plan(policy=policy, registry=registry)

    assert plan.specification_digest == digest_of(canonical_mapping(plan.specification.to_dict()))
    assert plan.specification_identity == plan.specification_digest
    assert plan.policy_digest == policy.digest()
    assert plan.registry_digest == registry.digest()
    assert plan.assumptions_digest == BarAmbiguityPolicy.declared_default().digest
    assert plan.engine_version == engine_version()

    expected = digest_of(
        {
            "schema": SPECIFICATION_SCHEMA_VERSION,
            "specification": plan.specification_digest,
            "policy": plan.policy_digest,
            "registry": plan.registry_digest,
            "features": "|".join(definition.digest() for definition in plan.definitions),
            "assumptions": plan.assumptions_digest,
            "engine": plan.engine_version,
        },
    )

    assert plan.behavioural_digest == expected
    assert plan.behavioural_digest != plan.specification_digest


def test_two_compilations_of_one_payload_agree() -> None:
    """
    Test both identities are stable across two compilations of the same inputs.
    """
    first = _plan()
    second = _plan()

    assert first.specification_digest == second.specification_digest
    assert first.behavioural_digest == second.behavioural_digest
    assert first == second


def test_a_changed_policy_changes_only_the_behavioural_identity() -> None:
    """
    Test a policy the candidate never names still moves the behavioural identity.
    """
    base = _plan()
    revised = _plan(policy=fixture_policy(version=2))

    assert revised.specification_digest == base.specification_digest
    assert revised.behavioural_digest != base.behavioural_digest
    assert revised.policy_digest != base.policy_digest


def test_a_changed_definition_changes_only_the_behavioural_identity() -> None:
    """
    Test a feature definition that changed underneath a candidate is a different candidate.
    """
    base = _plan()
    revised = _plan(registry=fixture_registry(lookback=LEGAL_LOOKBACK + 1))

    assert revised.specification_digest == base.specification_digest
    assert revised.behavioural_digest != base.behavioural_digest
    assert revised.registry_digest != base.registry_digest


def test_a_widened_registry_changes_only_the_behavioural_identity() -> None:
    """
    Test a definition the candidate did not select is still part of the identity it ran against.
    """
    base = _plan()
    widened = _plan(registry=fixture_registry(extra=True))

    assert widened.specification_digest == base.specification_digest
    assert widened.behavioural_digest != base.behavioural_digest


def test_a_refused_payload_returns_errors_rather_than_raising() -> None:
    """
    Test every stage that refuses compiles to errors with no plan.
    """
    unparsed = compile_specification(None, fixture_policy(), fixture_registry())
    unresolved = _compile(
        {f"features.declared.{ALIAS}.feature": UNREGISTERED_FEATURE_ID},
    )
    semantic = _compile({"selection.feature": REMOVE})
    temporal = _compile(registry=fixture_registry(availability_lag_ns=SESSION_NS))

    for compiled in (unparsed, unresolved, semantic, temporal):
        assert compiled.accepted is False
        assert compiled.plan is None
        assert compiled.errors
        assert compiled.warnings == ()

    assert semantic.errors[0].code is RefusalCode.SELECTION_INCOMPLETE
    assert temporal.errors[0].code is RefusalCode.FEATURE_UNAVAILABLE_AT_DECISION


def test_definition_for_resolves_a_declared_alias() -> None:
    """
    Test the plan resolves an alias to the definition it selected, and nothing else.
    """
    registry = fixture_registry()
    plan = _plan(registry=registry)

    assert plan.definition_for(ALIAS) == registry.resolve(FEATURE_ID)
    assert plan.definition_for("absent") is None


def test_to_dict_carries_the_identities_the_specification_and_the_definitions() -> None:
    """
    Test the plain-data form holds every identity and each definition with its digest.
    """
    plan = _plan()
    payload = plan.to_dict()
    definitions = cast("list[dict[str, Any]]", payload["definitions"])

    assert payload["specification_digest"] == plan.specification_digest
    assert payload["behavioural_digest"] == plan.behavioural_digest
    assert payload["policy_digest"] == plan.policy_digest
    assert payload["registry_digest"] == plan.registry_digest
    assert payload["assumptions_digest"] == plan.assumptions_digest
    assert payload["engine_version"] == plan.engine_version
    assert payload["migrations"] == []
    assert payload["specification"] == plan.specification.to_dict()

    assert len(definitions) == 1
    assert definitions[0]["alias"] == ALIAS
    assert definitions[0]["feature_id"] == plan.definitions[0].feature_id
    assert definitions[0]["digest"] == plan.definitions[0].digest()


def test_warnings_are_reported_beside_an_accepted_plan() -> None:
    """
    Test a warning does not stop a plan being compiled.
    """
    compiled = _compile({f"features.declared.{ALIAS}.lookback": LEGAL_LOOKBACK + 1})

    assert compiled.accepted is True
    assert compiled.plan is not None
    assert [(warning.code, warning.path) for warning in compiled.warnings] == [
        (WarningCode.FEATURE_WARMUP_DOMINATES, f"features.declared.{ALIAS}.lookback"),
    ]


def test_the_engine_version_is_read_from_the_installed_package() -> None:
    """
    Test the plan names the engine that built it rather than inventing a version.
    """
    assert ENGINE_VERSION_UNKNOWN == "unknown"
    assert engine_version() == getattr(nautilus_trader, "__version__", ENGINE_VERSION_UNKNOWN)
    assert _plan().engine_version == engine_version()


def test_compose_plan_is_the_stage_compilation_runs() -> None:
    """
    Test composing a plan from an accepted specification agrees with compiling the payload.
    """
    policy = fixture_policy()
    registry = fixture_registry()
    compiled = _compile(policy=policy, registry=registry)

    assert compiled.plan is not None

    composed = compose_plan(
        compiled.plan.specification,
        policy,
        registry,
        compiled.plan.definitions,
        compiled.plan.migrations,
    )

    assert composed.specification_digest == compiled.plan.specification_digest
    assert composed.behavioural_digest == compiled.plan.behavioural_digest
    assert composed == compiled.plan
