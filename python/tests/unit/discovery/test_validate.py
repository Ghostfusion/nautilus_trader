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
Tests for the compiler's first four stages.

Every case is the fixture payload with one thing changed, and a refusal is asserted by code and path
rather than by message, because the code and the path are the contract and the wording is not.
"""

from __future__ import annotations

from copy import deepcopy
from decimal import Decimal
from typing import Any

from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.policy import DiscoveryPolicy
from nautilus_trader.discovery.schema import SPECIFICATION_SCHEMA_VERSION
from nautilus_trader.discovery.schema import RefusalCode
from nautilus_trader.discovery.validate import SESSION_NS
from nautilus_trader.discovery.validate import ValidationResult
from nautilus_trader.discovery.validate import WarningCode
from nautilus_trader.discovery.validate import validate_specification
from tests.unit.discovery.fixtures import ALIAS
from tests.unit.discovery.fixtures import REMOVE
from tests.unit.discovery.fixtures import SECTOR_INSTRUMENT
from tests.unit.discovery.fixtures import UNREGISTERED_FEATURE_ID
from tests.unit.discovery.fixtures import changed
from tests.unit.discovery.fixtures import fixture_policy
from tests.unit.discovery.fixtures import fixture_registry
from tests.unit.discovery.fixtures import legal_payload


def _validate(
    changes: dict[str, Any],
    policy: DiscoveryPolicy | None = None,
    registry: FeatureRegistry | None = None,
) -> ValidationResult:
    """
    Return the result of validating the fixture payload with the given changes applied.
    """
    return validate_specification(
        changed(legal_payload(), changes),
        fixture_policy() if policy is None else policy,
        fixture_registry() if registry is None else registry,
    )


def _errors(result: ValidationResult) -> set[tuple[RefusalCode, str]]:
    """
    Return the code and path of every refusal, which is what a test asserts.
    """
    return {(error.code, error.path) for error in result.errors}


def _warnings(result: ValidationResult) -> set[tuple[WarningCode, str]]:
    """
    Return the code and path of every warning, which is what a test asserts.
    """
    return {(warning.code, warning.path) for warning in result.warnings}


def test_a_legal_specification_is_accepted() -> None:
    """
    Test a legal payload passes every stage and has nothing to report.
    """
    result = _validate({})

    assert result.accepted is True
    assert result.errors == ()
    assert result.warnings == ()
    assert result.specification is not None
    assert len(result.definitions) == 1
    assert result.migrations == ()


def test_a_payload_that_is_not_a_mapping_is_refused() -> None:
    """
    Test stage one refuses data that is not a specification at all.
    """
    result = validate_specification(None, fixture_policy(), fixture_registry())

    assert result.accepted is False
    assert result.specification is None
    assert _errors(result) == {(RefusalCode.NOT_A_MAPPING, "")}


def test_an_undeclared_block_field_is_refused_at_its_path() -> None:
    """
    Test stage one refuses a field no block declares.
    """
    result = _validate({"universe.currency": "USD"})

    assert _errors(result) == {(RefusalCode.BLOCK_UNKNOWN_FIELD, "universe.currency")}
    assert result.specification is None


def test_a_feature_the_registry_does_not_hold_is_refused() -> None:
    """
    Test stage two refuses an alias whose identifier does not resolve, at its reference.
    """
    result = _validate({f"features.declared.{ALIAS}.feature": UNREGISTERED_FEATURE_ID})

    assert result.accepted is False
    assert _errors(result) == {
        (RefusalCode.FEATURE_UNRESOLVED, f"features.declared.{ALIAS}.feature"),
    }
    assert result.definitions == ()


def test_a_refused_specification_still_carries_what_parsed() -> None:
    """
    Test a specification a later stage refused is carried beside the refusals.
    """
    result = _validate({f"features.declared.{ALIAS}.lookback": 21})

    assert result.accepted is True
    assert result.specification is not None

    refused = _validate({"selection.feature": REMOVE})

    assert refused.accepted is False
    assert refused.specification is not None
    assert len(refused.definitions) == 1
    assert refused.warnings == ()


def test_a_rank_selection_without_its_feature_is_refused() -> None:
    """
    Test stage three refuses a rank selection with no ranking feature.
    """
    result = _validate({"selection.feature": REMOVE})

    assert _errors(result) == {(RefusalCode.SELECTION_INCOMPLETE, "selection")}


def test_a_rank_selection_without_exactly_one_bound_is_refused() -> None:
    """
    Test stage three refuses a rank selection that states both bounds or neither.
    """
    both = _validate({"selection.count": 5})
    neither = _validate({"selection.percentile": REMOVE})

    assert _errors(both) == {(RefusalCode.SELECTION_INCOMPLETE, "selection")}
    assert _errors(neither) == {(RefusalCode.SELECTION_INCOMPLETE, "selection")}


def test_a_rank_selection_states_every_field_it_requires() -> None:
    """
    Test the refusals of one stage are collected rather than stopping at the first.
    """
    result = _validate({"selection.feature": REMOVE, "selection.tie_break": REMOVE})

    assert len(result.errors) == 2
    assert _errors(result) == {(RefusalCode.SELECTION_INCOMPLETE, "selection")}


def test_a_threshold_selection_without_a_threshold_is_refused() -> None:
    """
    Test stage three refuses a threshold selection missing one of its required fields.
    """
    result = _validate(
        {
            "selection.mode": "threshold",
            "selection.comparison": "gt",
            "selection.percentile": REMOVE,
            "selection.tie_break": REMOVE,
        },
    )

    assert _errors(result) == {(RefusalCode.SELECTION_INCOMPLETE, "selection")}


def test_a_threshold_selection_stating_a_ranking_is_refused() -> None:
    """
    Test stage three refuses a threshold selection that also states a ranking field.
    """
    result = _validate(
        {
            "selection.mode": "threshold",
            "selection.comparison": "gt",
            "selection.threshold": "0.5",
        },
    )

    assert _errors(result) == {(RefusalCode.SELECTION_INCOMPLETE, "selection")}


def test_a_complete_threshold_selection_is_accepted() -> None:
    """
    Test a threshold selection stating exactly its own fields is not refused.
    """
    result = _validate(
        {
            "selection.mode": "threshold",
            "selection.comparison": "gt",
            "selection.threshold": "0.5",
            "selection.percentile": REMOVE,
            "selection.tie_break": REMOVE,
        },
    )

    assert result.accepted is True
    assert result.errors == ()


def test_a_selection_that_ranks_nothing_and_states_a_feature_is_refused() -> None:
    """
    Test stage three refuses a selection declaring the none mode beside a ranking field.
    """
    result = _validate(
        {
            "selection.mode": "none",
            "selection.percentile": REMOVE,
            "selection.tie_break": REMOVE,
        },
    )

    assert _errors(result) == {(RefusalCode.SELECTION_INCOMPLETE, "selection")}


def test_a_selection_mode_the_stage_cannot_check_is_refused() -> None:
    """
    Test stage three refuses a mode the policy declared and this stage cannot order.
    """
    result = _validate(
        {
            "selection.mode": "top_decile",
            "selection.feature": REMOVE,
            "selection.percentile": REMOVE,
            "selection.tie_break": REMOVE,
        },
        policy=fixture_policy(
            selection_modes=frozenset({"rank", "threshold", "none", "top_decile"}),
        ),
    )

    assert _errors(result) == {(RefusalCode.SELECTION_INCOMPLETE, "selection")}


def test_an_exit_stating_a_feature_without_a_threshold_is_refused() -> None:
    """
    Test stage three refuses an exit whose feature has no comparison to be evaluated against.
    """
    result = _validate({"exit.feature": ALIAS})

    assert _errors(result) == {(RefusalCode.EXIT_INCOMPLETE, "exit")}


def test_an_exit_stating_a_threshold_without_a_feature_is_refused() -> None:
    """
    Test stage three refuses an exit whose threshold has no feature to be read from.
    """
    result = _validate({"exit.comparison": "gt", "exit.threshold": "0.5"})

    assert _errors(result) == {(RefusalCode.EXIT_INCOMPLETE, "exit")}


def test_a_rank_exit_without_a_rank_selection_is_refused() -> None:
    """
    Test stage three refuses a rank exit the selection cannot resolve.
    """
    result = _validate(
        {
            "exit.rank_exit_count": 5,
            "selection.mode": "none",
            "selection.feature": REMOVE,
            "selection.percentile": REMOVE,
            "selection.tie_break": REMOVE,
            "entry.conditions": REMOVE,
        },
    )

    assert _errors(result) == {(RefusalCode.EXIT_INCOMPLETE, "exit.rank_exit_count")}


def test_a_rank_exit_beside_a_rank_selection_is_accepted() -> None:
    """
    Test a rank exit beside a rank selection is not refused.
    """
    result = _validate({"exit.rank_exit_count": 5})

    assert result.accepted is True
    assert result.errors == ()


def test_a_sector_cap_without_a_sector_map_is_refused() -> None:
    """
    Test stage three refuses a sector cap the policy declares no sector to apply it through.
    """
    result = _validate({"portfolio.max_sector_weight": "0.5"})

    assert result.accepted is False
    assert _errors(result) == {
        (RefusalCode.SECTOR_INPUT_UNDECLARED, "portfolio.max_sector_weight"),
    }


def test_a_sector_cap_with_a_sector_map_is_accepted() -> None:
    """
    Test a sector cap beside a declared sector map is not refused.
    """
    result = _validate(
        {"portfolio.max_sector_weight": "0.5"},
        policy=fixture_policy(sectors=((SECTOR_INSTRUMENT, "technology"),)),
    )

    assert result.accepted is True
    assert result.errors == ()


def test_a_rebalance_timing_that_disagrees_with_the_eligibility_is_refused() -> None:
    """
    Test stage three refuses a rebalance timing that is not the declared eligibility.
    """
    result = _validate(
        {"rebalance.timing": "session_close"},
        policy=fixture_policy(
            rebalance_timings=frozenset({"next_session_open", "session_close"}),
        ),
    )

    assert _errors(result) == {(RefusalCode.REBALANCE_TIMING_MISMATCH, "rebalance.timing")}


def test_a_rebalance_finer_than_the_signal_is_refused() -> None:
    """
    Test stage three refuses a rebalance that would run between decisions.
    """
    result = _validate(
        {"data.signal_frequency": "weekly", "rebalance.frequency": "daily"},
        policy=fixture_policy(signal_frequencies=frozenset({"weekly"})),
    )

    assert _errors(result) == {(RefusalCode.REBALANCE_FREQUENCY_MISMATCH, "rebalance.frequency")}


def test_a_rebalance_at_the_signal_frequency_is_accepted() -> None:
    """
    Test a rebalance no finer than the signal is not refused.
    """
    result = _validate(
        {"data.signal_frequency": "weekly"},
        policy=fixture_policy(signal_frequencies=frozenset({"weekly"})),
    )

    assert result.accepted is True
    assert result.errors == ()


def test_an_execution_that_is_not_deferred_is_refused() -> None:
    """
    Test stage four refuses a specification that decides at one instant and fills at it.
    """
    result = _validate(
        {
            "data.execution_eligibility": "same_session_close",
            "rebalance.timing": "same_session_close",
        },
        policy=fixture_policy(
            rebalance_timings=frozenset({"next_session_open", "same_session_close"}),
        ),
    )

    assert result.accepted is False
    assert _errors(result) == {
        (RefusalCode.UNDEFERRED_EXECUTION, "data.execution_eligibility"),
    }


def test_an_execution_deferred_from_the_open_is_accepted() -> None:
    """
    Test a fill later than the decision instant is not refused.
    """
    result = _validate(
        {
            "data.decision_time": "session_open",
            "data.execution_eligibility": "same_session_close",
            "rebalance.timing": "same_session_close",
        },
        policy=fixture_policy(
            rebalance_timings=frozenset({"next_session_open", "same_session_close"}),
        ),
    )

    assert result.accepted is True
    assert result.errors == ()


def test_an_instant_the_table_does_not_hold_is_refused() -> None:
    """
    Test stage four refuses an instant it cannot order rather than guessing one.
    """
    result = _validate(
        {
            "data.execution_eligibility": "next_session_auction",
            "rebalance.timing": "next_session_auction",
        },
        policy=fixture_policy(
            execution_eligibilities=frozenset({"next_session_auction"}),
            rebalance_timings=frozenset({"next_session_auction"}),
        ),
    )

    assert result.accepted is False
    assert _errors(result) == {
        (RefusalCode.UNDEFERRED_EXECUTION, "data.execution_eligibility"),
    }


def test_a_feature_available_at_the_next_signal_is_refused() -> None:
    """
    Test stage four refuses a feature that cannot be known when the candidate decides.
    """
    result = _validate({}, registry=fixture_registry(availability_lag_ns=SESSION_NS))

    assert result.accepted is False
    assert _errors(result) == {
        (RefusalCode.FEATURE_UNAVAILABLE_AT_DECISION, f"features.declared.{ALIAS}.feature"),
    }


def test_a_feature_available_before_the_next_signal_is_accepted() -> None:
    """
    Test a lag just inside the signal interval is not refused.
    """
    result = _validate({}, registry=fixture_registry(availability_lag_ns=SESSION_NS - 1))

    assert result.accepted is True
    assert result.errors == ()


def test_a_lookback_dominating_the_rebalance_interval_warns() -> None:
    """
    Test a lookback beyond four rebalance intervals is reported rather than refused.

    The fixture payload declares the largest lookback that does not dominate, so the case above the
    boundary is one session more.
    """
    at_the_boundary = _validate({})
    above_it = _validate({f"features.declared.{ALIAS}.lookback": 21})

    assert at_the_boundary.warnings == ()
    assert above_it.accepted is True
    assert _warnings(above_it) == {
        (WarningCode.FEATURE_WARMUP_DOMINATES, f"features.declared.{ALIAS}.lookback"),
    }


def test_a_declared_feature_nothing_references_warns() -> None:
    """
    Test an alias no block reads is reported rather than refused.
    """
    result = _validate(
        {
            "selection.mode": "none",
            "selection.feature": REMOVE,
            "selection.percentile": REMOVE,
            "selection.tie_break": REMOVE,
            "entry.conditions": REMOVE,
        },
    )

    assert result.accepted is True
    assert _warnings(result) == {
        (WarningCode.UNUSED_DECLARED_FEATURE, f"features.declared.{ALIAS}"),
    }


def test_a_position_cap_below_fully_invested_warns() -> None:
    """
    Test a position cap that cannot invest the whole portfolio is reported.
    """
    result = _validate({"portfolio.max_position_weight": "0.1"})

    assert result.accepted is True
    assert _warnings(result) == {
        (WarningCode.POSITION_CAP_BELOW_FULLY_INVESTED, "portfolio"),
    }


def test_a_fully_invested_position_cap_does_not_warn() -> None:
    """
    Test the fixture payload's five positions at a fifth each do not warn.
    """
    result = _validate({})

    assert _warnings(result) == set()


def test_a_post_run_limit_warns_about_every_limit_it_names() -> None:
    """
    Test a limit measured after the run is reported as a gate condition.
    """
    result = _validate(
        {"risk.post_run.max_drawdown": "0.2", "risk.post_run.max_turnover": "1.5"},
    )

    assert result.accepted is True
    assert _warnings(result) == {
        (WarningCode.POST_RUN_LIMITS_ARE_GATE_CONDITIONS, "risk.post_run.max_drawdown"),
        (WarningCode.POST_RUN_LIMITS_ARE_GATE_CONDITIONS, "risk.post_run.max_turnover"),
    }


def test_a_payload_written_against_an_older_version_is_migrated() -> None:
    """
    Test the migration table reaches the schema stage rather than being ignored.
    """
    payload = changed(
        legal_payload(),
        {"version": "discovery.specification/v0", "universe.min_price": REMOVE},
    )
    migrations = {"discovery.specification/v0": _upgrade_universe}

    refused = validate_specification(payload, fixture_policy(), fixture_registry())

    assert _errors(refused) == {(RefusalCode.VERSION_UNSUPPORTED, "version")}

    migrated = validate_specification(
        payload,
        fixture_policy(),
        fixture_registry(),
        migrations=migrations,
    )

    assert migrated.accepted is True
    assert migrated.specification is not None
    assert migrated.specification.universe.min_price == Decimal(5)


def _upgrade_universe(payload: dict[str, Any]) -> dict[str, Any]:
    """
    Return a version-zero payload rewritten against the accepted schema.
    """
    migrated = deepcopy(payload)
    migrated["version"] = SPECIFICATION_SCHEMA_VERSION
    migrated["universe"]["min_price"] = "5"

    return migrated
