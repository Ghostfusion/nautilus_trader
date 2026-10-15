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
Test the closed specification schema, its refusals, its bounds and its versioning.

The schema is the only gate into the language, so these tests are about what cannot get in: a field a
block does not declare, a value outside the policy's bound, a bound the policy does not declare, a
value outside a declared set, a feature a candidate is not allowed to reference, a condition tree
that is too deep or too large, and a version this build does not accept. Each asserts the refusal's
code and path rather than its wording, because the code is the auditable contract.
"""

from __future__ import annotations

from decimal import Decimal
from typing import TYPE_CHECKING
from typing import Any

from nautilus_trader.discovery.policy import Bound
from nautilus_trader.discovery.policy import BoundKind
from nautilus_trader.discovery.schema import SPECIFICATION_SCHEMA_VERSION
from nautilus_trader.discovery.schema import RefusalCode
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.discovery.schema import count_condition_leaves
from nautilus_trader.discovery.schema import parse_specification
from nautilus_trader.discovery.schema import scan_for_code_references
from tests.unit.discovery import fixtures


if TYPE_CHECKING:
    from collections.abc import Mapping

    from nautilus_trader.discovery.schema import Refusal


BLOCKS = (
    "identity",
    "universe",
    "data",
    "features",
    "selection",
    "entry",
    "exit",
    "portfolio",
    "rebalance",
    "risk",
    "evaluation",
    "provenance",
)


def refusals(payload: object) -> tuple[Refusal, ...]:
    """
    Parse the payload against the fixture policy and return its refusals.
    """
    return parse_specification(payload, fixtures.fixture_policy()).refusals


def codes(payload: object) -> set[tuple[RefusalCode, str]]:
    """
    Return the refusals as `(code, path)` pairs.
    """
    return {(refusal.code, refusal.path) for refusal in refusals(payload)}


def test_a_legal_payload_is_accepted_and_round_trips_through_its_canonical_form() -> None:
    """
    Test that the specification's own canonical form parses again to the same values.
    """
    parsed = parse_specification(fixtures.legal_payload(), fixtures.fixture_policy())

    assert parsed.accepted
    assert parsed.specification is not None

    specification = parsed.specification

    assert specification.version == SPECIFICATION_SCHEMA_VERSION
    reparsed = parse_specification(specification.to_dict(), fixtures.fixture_policy())

    assert reparsed.accepted
    assert reparsed.specification is not None
    assert reparsed.specification.to_dict() == specification.to_dict()


def test_field_order_does_not_change_the_canonical_form() -> None:
    """
    Test that two payloads differing only in field order have one identity.

    A canonical form that depended on field order would give one candidate two identities, and the
    ledger keys a candidate by its digest.
    """
    payload = fixtures.legal_payload()
    reordered: dict[str, Any] = {}
    for block in reversed(list(payload)):
        value = payload[block]
        reordered[block] = (
            {key: value[key] for key in reversed(list(value))} if isinstance(value, dict) else value
        )

    first = parse_specification(payload, fixtures.fixture_policy())
    second = parse_specification(reordered, fixtures.fixture_policy())

    assert first.specification is not None
    assert second.specification is not None
    assert first.specification.to_dict() == second.specification.to_dict()
    assert canonical_mapping(first.specification.to_dict()) == canonical_mapping(
        second.specification.to_dict(),
    )


def test_the_canonical_form_includes_a_bound_the_parser_does_not_type() -> None:
    """
    Test that the canonical mapping is a faithful flattening of nested plain data.
    """
    payload = {"a": {"b": [1, "two"]}, "c": None, "d": Decimal("0.10")}
    flat = canonical_mapping(payload)

    assert flat == {"a.b.0": 1, "a.b.1": "two", "c": None, "d": "0.10"}


def test_an_unknown_field_is_refused_at_every_block() -> None:
    """
    Test that no block accepts a field it does not declare.

    The schema is closed, so a value cannot be carried into a place no validator reads. The test
    walks every block rather than one, because a single undeclared field is a single block's bug.
    """
    for block in BLOCKS:
        payload = fixtures.changed(fixtures.legal_payload(), {f"{block}.junk": 1})
        refused = codes(payload)

        assert (RefusalCode.BLOCK_UNKNOWN_FIELD, f"{block}.junk") in refused, block


def test_an_unknown_top_level_field_is_refused() -> None:
    """
    Test that the specification itself is closed.
    """
    payload = fixtures.legal_payload()
    payload["strategy_path"] = "strategies.ema_cross:EMACross"

    assert (RefusalCode.CODE_REFERENCE_REFUSED, "strategy_path") in codes(payload)


def test_a_value_outside_the_policys_bound_is_refused_with_the_bound_stated() -> None:
    """
    Test that a bound comes from the policy rather than from the candidate.

    The fixture policy is rebuilt with the position weight cap's own bound narrowed, and a payload
    that is legal under the shipped bound is refused, which is the property that makes the bound the
    policy's rather than the schema's constant.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"portfolio.max_position_weight": "0.24"},
    )
    assert codes(payload) == set()

    narrow = tuple(
        Bound(bound.name, BoundKind.RANGE, lower=Decimal("0.005"), upper=Decimal("0.2"))
        if bound.name == "portfolio.max_position_weight"
        else bound
        for bound in fixtures.fixture_policy().bounds
    )
    policy = fixtures.fixture_policy(bounds=narrow)

    assert {
        (refusal.code, refusal.path) for refusal in parse_specification(payload, policy).refusals
    } == {(RefusalCode.VALUE_OUT_OF_BOUND, "portfolio.max_position_weight")}


def test_a_bound_the_policy_does_not_declare_refuses_every_value_of_the_field() -> None:
    """
    Test that an undeclared bound is refused rather than treated as permissive.

    A field with no declared bound is a field the policy does not cover, so no value for it can be
    admitted: an unstated bound is not an unbounded one.
    """
    stripped = tuple(
        bound
        for bound in fixtures.fixture_policy().bounds
        if bound.name != "portfolio.position_count"
    )
    policy = fixtures.fixture_policy(bounds=stripped)
    result = parse_specification(fixtures.legal_payload(), policy)

    assert not result.accepted
    assert any(refusal.code is RefusalCode.BOUND_NOT_DECLARED for refusal in result.refusals)


def test_a_value_outside_a_declared_set_is_refused() -> None:
    """
    Test that a value resolved from the policy's declared sets is refused when it is not in one.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"rebalance.frequency": "hourly"},
    )

    assert (RefusalCode.VALUE_NOT_DECLARED, "rebalance.frequency") in codes(payload)


def test_a_feature_outside_the_allowlist_is_refused() -> None:
    """
    Test that a candidate may reference only the features a campaign declared.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {f"features.declared.{fixtures.ALIAS}.feature": "close_hull_9"},
    )

    assert (
        RefusalCode.FEATURE_NOT_ALLOWED,
        f"features.declared.{fixtures.ALIAS}.feature",
    ) in codes(
        payload,
    )


def test_a_feature_reference_that_is_not_declared_is_refused() -> None:
    """
    Test that the selection must name one of the aliases the specification declares.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"selection.feature": "unlisted"},
    )

    assert (RefusalCode.FEATURE_NOT_DECLARED, "selection.feature") in codes(payload)


def test_a_reference_the_policy_does_not_declare_is_refused() -> None:
    """
    Test that an evaluation reference resolves to a frozen object rather than to a name.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"evaluation.benchmark": "benchmark.invented"},
    )

    assert (RefusalCode.REFERENCE_UNRESOLVED, "evaluation.benchmark") in codes(payload)


def test_a_condition_tree_deeper_than_the_declared_limit_is_refused() -> None:
    """
    Test the nesting limit, and that the leaf count of a tree is what the limit counts.
    """
    nested = {
        "all": [{"any": [{"all": [{"feature": fixtures.ALIAS, "comparison": "gt", "value": 1}]}]}]
    }
    payload = fixtures.changed(fixtures.legal_payload(), {"entry.conditions": nested})

    # The limit is on the nesting of the tree, so the group reaching the third level is refused
    # rather than the comparison inside it, which is one level deeper still.
    assert (RefusalCode.CONDITION_DEPTH_EXCEEDED, "entry.conditions.all.0.any.0") in codes(payload)


def test_a_condition_tree_with_too_many_leaves_is_refused() -> None:
    """
    Test the condition cap against the number of comparisons rather than of groups.
    """
    leaf = {"feature": fixtures.ALIAS, "comparison": "gt", "value": 1}
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"entry.conditions": {"all": [leaf] * 5}},
    )

    assert (RefusalCode.CONDITION_COUNT_EXCEEDED, "entry.conditions") in codes(payload)


def test_a_condition_group_stating_both_all_and_any_is_refused() -> None:
    """
    Test that an ambiguous group is refused rather than resolved by precedence.
    """
    leaf = {"feature": fixtures.ALIAS, "comparison": "gt", "value": 1}
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"entry.conditions": {"all": [leaf], "any": [leaf]}},
    )

    assert (RefusalCode.CONDITION_NOT_RESOLVED, "entry.conditions") in codes(payload)


def test_count_condition_leaves_counts_comparisons() -> None:
    """
    Test the leaf counter the condition cap is applied through.
    """
    leaf = {"feature": fixtures.ALIAS, "comparison": "gt", "value": 1}
    other = {"feature": fixtures.ALIAS, "comparison": "lt", "value": 2}
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"entry.conditions": {"all": [leaf, leaf, other]}},
    )
    parsed = parse_specification(payload, fixtures.fixture_policy())

    assert parsed.accepted
    assert parsed.specification is not None
    assert parsed.specification.entry.conditions is not None
    assert count_condition_leaves(parsed.specification.entry.conditions) == 3


def test_a_version_this_build_does_not_accept_is_migrated_or_refused() -> None:
    """
    Test that a stored version is never silently reinterpreted.

    A build with no migration registered refuses the version it does not know; a build that registers
    one applies it, records that it did, and parses the migrated payload.
    """
    refused = fixtures.changed(
        fixtures.legal_payload(),
        {"version": "discovery.specification/v0"},
    )

    assert (RefusalCode.VERSION_UNSUPPORTED, "version") in codes(refused)

    migrated_payload = fixtures.legal_payload()
    migrated_payload["version"] = "discovery.specification/v0"

    def _migrate(payload: Mapping[str, object]) -> Mapping[str, object]:
        rewritten = dict(payload)
        rewritten["version"] = SPECIFICATION_SCHEMA_VERSION
        return rewritten

    result = parse_specification(
        migrated_payload,
        fixtures.fixture_policy(),
        migrations={"discovery.specification/v0": _migrate},
    )

    assert result.accepted
    assert result.migrations == ("discovery.specification/v0",)


def test_a_migration_cycle_is_refused_rather_than_looped() -> None:
    """
    Test that migrations which do not reach an accepted version are refused.
    """
    payload = fixtures.legal_payload()
    payload["version"] = "discovery.specification/v0"

    def _endless(state: Mapping[str, object]) -> Mapping[str, object]:
        return dict(state)

    result = parse_specification(
        payload,
        fixtures.fixture_policy(),
        migrations={"discovery.specification/v0": _endless},
    )

    assert (RefusalCode.VERSION_MIGRATION_CYCLE, "version") in {
        (refusal.code, refusal.path) for refusal in result.refusals
    }


def test_a_missing_version_is_refused() -> None:
    """
    Test that a specification declares the version it was written against.
    """
    payload = fixtures.legal_payload()
    del payload["version"]

    assert (RefusalCode.VERSION_MISSING, "version") in codes(payload)


def test_a_payload_that_is_not_a_mapping_is_refused() -> None:
    """
    Test that the parser refuses anything that is not a mapping.
    """
    result = parse_specification(["not", "a", "mapping"], fixtures.fixture_policy())

    assert not result.accepted
    assert result.refusals[0].code is RefusalCode.NOT_A_MAPPING


def test_a_missing_block_is_refused_by_name() -> None:
    """
    Test that an incomplete payload names the block it is missing.
    """
    payload = fixtures.legal_payload()
    del payload["risk"]

    assert (RefusalCode.BLOCK_MISSING, "risk") in codes(payload)


def test_a_block_that_is_not_a_mapping_is_refused() -> None:
    """
    Test that a block is a mapping rather than a value.
    """
    payload = fixtures.changed(fixtures.legal_payload(), {"features": ["ema_close"]})

    assert (RefusalCode.BLOCK_NOT_A_MAPPING, "features") in codes(payload)


def test_a_misplaced_risk_field_is_refused_by_category() -> None:
    """
    Test that a limit measured after a run is refused where it would promise control during one.
    """
    payload = fixtures.changed(
        fixtures.legal_payload(),
        {"risk.runtime.max_drawdown": "0.2"},
    )

    assert (RefusalCode.RISK_FIELD_MISPLACED, "risk.runtime.max_drawdown") in codes(payload)


def test_code_is_refused_wherever_a_payload_carries_it() -> None:
    """
    Test the code-reference scan directly, including nested positions a block does not declare.
    """
    assert scan_for_code_references({"a": {"b": "eval(1)"}})[0].path == "a.b"
    assert scan_for_code_references({"module": "x"})[0].path == "module"
    assert scan_for_code_references({"a": [lambda: None]})[0].path == "a.0"
    assert scan_for_code_references({"a": 1}) == []
    assert scan_for_code_references({"a": "the close above its average"}) == []
