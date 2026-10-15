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
The policy, registry and payload builders the discovery compiler tests share.

The legal payload is built once, and every negative case is a mutation of it, so a rule is tested by
changing one thing in a payload that otherwise passes every stage rather than by writing a second
fixture that can drift from the first. The payload is deliberately plain data: it is what a candidate
submits, not a parsed specification.

The fixture policy is the first-release scope with a feature allowlist, three frozen references and
no sector map, so the sector rule has to be exercised by a policy that declares one.
"""

from __future__ import annotations

from copy import deepcopy
from dataclasses import replace
from typing import Any

from nautilus_trader.discovery.features import OPERATOR_EMA
from nautilus_trader.discovery.features import FeatureDefinition
from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.policy import DiscoveryPolicy
from nautilus_trader.discovery.policy import Reference
from nautilus_trader.discovery.policy import ReferenceKind
from nautilus_trader.discovery.schema import SPECIFICATION_SCHEMA_VERSION


# The registered feature the legal payload selects, under an alias that is not its identifier, so a
# test that confuses the two is visible rather than harmless.
FEATURE_ID = "ema_close"
ALIAS = "trend"

# An identifier the fixture policy allows and the fixture registry does not hold, so the resolution
# stage has a feature it can fail to resolve.
UNREGISTERED_FEATURE_ID = "rsi_close"

# A frozen reference digest the policy only checks the shape of, and the names the legal payload
# cites. The digest is fixed rather than computed, because a reference is an identity.
REFERENCE_DIGEST = "sha256:" + "0" * 64
BENCHMARK = "benchmark.us_equity"
COST_MODEL = "cost_model.first_release"
VALIDATION_POLICY = "validation.first_release"

# The lookback the legal payload declares. It is exactly four weekly intervals, which is the largest
# lookback that does not dominate the rebalance interval, so the boundary is a test rather than an
# accident.
LEGAL_LOOKBACK = 20

# The instrument the fixture policy declares a sector for, when a test asks for one.
SECTOR_INSTRUMENT = "AAPL.NASDAQ"


class _Remove:
    """
    A sentinel that tells `changed` to delete a field rather than set it.
    """


REMOVE = _Remove()


def fixture_policy(**overrides: Any) -> DiscoveryPolicy:
    """
    Return the fixture policy, with any declared field replaced.

    Parameters
    ----------
    **overrides : Any
        The declared fields to replace, which the dataclass revalidates.

    Returns
    -------
    DiscoveryPolicy

    """
    policy = DiscoveryPolicy.first_release(
        feature_allowlist=(FEATURE_ID, UNREGISTERED_FEATURE_ID),
        references=(
            Reference(BENCHMARK, ReferenceKind.BENCHMARK, REFERENCE_DIGEST),
            Reference(COST_MODEL, ReferenceKind.COST_MODEL, REFERENCE_DIGEST),
            Reference(VALIDATION_POLICY, ReferenceKind.VALIDATION_POLICY, REFERENCE_DIGEST),
        ),
    )

    if not overrides:
        return policy

    return replace(policy, **overrides)


def fixture_registry(
    lookback: int = LEGAL_LOOKBACK,
    availability_lag_ns: int = 0,
    *,
    extra: bool = False,
) -> FeatureRegistry:
    """
    Return the registry the legal payload's feature resolves against.

    Parameters
    ----------
    lookback : int, default LEGAL_LOOKBACK
        The lookback the registered definition computes over.
    availability_lag_ns : int, default 0
        The lag between the observation and the earliest time the value may be used.
    extra : bool, default False
        Whether to add a second definition the legal payload does not select.

    Returns
    -------
    FeatureRegistry

    """
    definitions = [
        FeatureDefinition(
            feature_id=FEATURE_ID,
            operator=OPERATOR_EMA,
            input_field="close",
            lookback=lookback,
            availability_lag_ns=availability_lag_ns,
        ),
    ]

    if extra:
        definitions.append(
            FeatureDefinition(
                feature_id=UNREGISTERED_FEATURE_ID,
                operator=OPERATOR_EMA,
                input_field="close",
                lookback=lookback,
            ),
        )

    return FeatureRegistry(definitions)


def legal_payload() -> dict[str, Any]:
    """
    Return a payload no stage refuses, which every negative case mutates.

    Returns
    -------
    dict[str, Any]

    """
    return {
        "version": SPECIFICATION_SCHEMA_VERSION,
        "identity": {
            "family": "cross_sectional_rank",
            "hypothesis": "The strongest twenty-session trend continues over the next week.",
            "mechanism": "Trend persistence across a rebalanced cross-section.",
            "falsification": "Refuted when the top quintile does not beat the universe mean.",
        },
        "universe": {
            "asset_classes": ["EQUITY"],
            "membership_policy": "point_in_time",
            "source": "catalog.us_equities",
            "min_price": "5",
            "min_dollar_volume": "1000000",
            "exclusions": [],
        },
        "data": {
            "signal_frequency": "daily",
            "decision_time": "session_close",
            "execution_eligibility": "next_session_open",
            "corporate_action_treatment": "adjusted",
        },
        "features": {
            "declared": {
                ALIAS: {"feature": FEATURE_ID, "lookback": LEGAL_LOOKBACK, "skip": 0},
            },
        },
        "selection": {
            "mode": "rank",
            "feature": ALIAS,
            "percentile": "0.2",
            "tie_break": "feature_desc",
        },
        "entry": {
            "direction": "long",
            "conditions": {"all": [{"feature": ALIAS, "comparison": "gt", "value": "0"}]},
        },
        "exit": {
            "holding_period_cap": 10,
            "flatten_ineligible": True,
        },
        "portfolio": {
            "position_count": 5,
            "weighting": "equal",
            "max_position_weight": "0.2",
            "cash_policy": "leave_idle",
        },
        "rebalance": {"frequency": "weekly", "timing": "next_session_open"},
        "risk": {
            "pre_trade": {"max_gross_exposure": "1"},
            "runtime": {},
            "post_run": {},
        },
        "evaluation": {
            "benchmark": BENCHMARK,
            "cost_model": COST_MODEL,
            "validation_policy": VALIDATION_POLICY,
        },
        "provenance": {"generator": "hand_written", "seed": 7, "parents": []},
    }


def changed(payload: dict[str, Any], changes: dict[str, Any]) -> dict[str, Any]:
    """
    Return a copy of the payload with each dotted path set to its value, or removed.

    Parameters
    ----------
    payload : dict[str, Any]
        The payload to copy.
    changes : dict[str, Any]
        The dotted paths to set, where a value of `REMOVE` deletes the field instead.

    Returns
    -------
    dict[str, Any]

    """
    mutated = deepcopy(payload)

    for path, value in changes.items():
        _assign(mutated, path, value)

    return mutated


def _assign(mutated: dict[str, Any], path: str, value: Any) -> None:
    """
    Set or delete one dotted path inside an already-copied payload.

    Parameters
    ----------
    mutated : dict[str, Any]
        The copied payload, updated in place.
    path : str
        The dotted path of the field.
    value : Any
        The value to set, or `REMOVE` to delete the field.

    """
    segments = path.split(".")
    block = mutated

    for segment in segments[:-1]:
        block = block[segment]

    if isinstance(value, _Remove):
        block.pop(segments[-1], None)
    else:
        block[segments[-1]] = value
