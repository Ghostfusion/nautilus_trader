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
The pre-registered policy: the frozen half of the candidate language.

Every bound a candidate specification is checked against, every declared set a field's value is
resolved from, and every reference the compiler resolves rather than accepts from the candidate
lives here. A candidate never states a significance threshold, a benchmark return or a cost
assumption, because those are policy: fixed before a campaign and outside the search's reach.

The policy is what makes the language closed rather than merely narrow. The schema refuses a field
it does not declare, and this module decides which values that field may hold, so admitting a value
and declaring it are the same act. A field whose bound the policy does not declare is therefore not
a value the schema can accept, which is why `bound` returns nothing rather than a permissive
default.

This module owns the policy's bound surface and its identity. The campaign lifecycle, the freeze and
change rules and the confirmation boundary arrive with the governance milestone.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal
from enum import Enum
from typing import TYPE_CHECKING

from nautilus_trader.discovery.schema import Bound
from nautilus_trader.discovery.schema import BoundKind
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Iterable
    from collections.abc import Mapping


# The stable identity of the pre-registered policy, and its version. A change to any declared set,
# bound or reference changes the version, so a historical result names the policy it ran under.
POLICY_ID = "discovery.policy"
POLICY_VERSION = 1

# The length of a `sha256:` digest: the scheme, the separator and the 64 hex digits.
_SHA256_DIGEST_LENGTH = 71

# The declared sets a specification's field values are resolved from, which are also the names a
# caller may pass to `DiscoveryPolicy.declared`.
DECLARED_SET_NAMES: tuple[str, ...] = (
    "families",
    "asset_classes",
    "membership_policies",
    "signal_frequencies",
    "decision_times",
    "execution_eligibilities",
    "corporate_action_treatments",
    "entry_directions",
    "operators",
    "selection_modes",
    "tie_breaks",
    "weighting_methods",
    "cash_policies",
    "rebalance_frequencies",
    "rebalance_timings",
)


class ReferenceKind(Enum):
    """
    The kind of frozen object a declared reference names.
    """

    BENCHMARK = "benchmark"
    COST_MODEL = "cost_model"
    VALIDATION_POLICY = "validation_policy"


@dataclass(frozen=True)
class ComplexityLimits:
    """
    The caps that stop a candidate from spending a trial budget on a larger expression.

    Parameters
    ----------
    max_conditions : int, default 4
        The maximum number of leaf conditions an entry block may state.
    max_condition_depth : int, default 2
        The maximum nesting depth of the condition tree.
    max_features : int, default 8
        The maximum number of features a candidate may declare.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If a limit is not positive.

    """

    max_conditions: int = 4
    max_condition_depth: int = 2
    max_features: int = 8

    def __post_init__(self) -> None:
        """
        Validate the declared limits.
        """
        for field, value in (
            ("max_conditions", self.max_conditions),
            ("max_condition_depth", self.max_condition_depth),
            ("max_features", self.max_features),
        ):
            if isinstance(value, bool) or not isinstance(value, int):
                raise TypeError(f"{field} must be an int, was {type(value).__name__}")
            if value < 1:
                raise ValueError(f"{field} must be positive, was {value}")

    def to_dict(self) -> dict[str, object]:
        """
        Return the limits as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "max_conditions": self.max_conditions,
            "max_condition_depth": self.max_condition_depth,
            "max_features": self.max_features,
        }


@dataclass(frozen=True)
class Reference:
    """
    One frozen object a specification may refer to, named by its digest.

    The object itself is not held here. A reference is an identity, and the compiler resolves it to
    the digest that names the frozen artifact, so a specification citing a benchmark cannot cite a
    different one later under the same name.

    Parameters
    ----------
    name : str
        The reference's name, which is the name a specification's evaluation block cites.
    kind : ReferenceKind
        The kind of frozen object the reference names.
    digest : str
        The `sha256:<hex>` digest of the frozen artifact.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If the name or the digest is not usable.

    """

    name: str
    kind: ReferenceKind
    digest: str

    def __post_init__(self) -> None:
        """
        Validate the declared reference.
        """
        if not isinstance(self.name, str):
            raise TypeError(f"name must be a str, was {type(self.name).__name__}")
        if not isinstance(self.kind, ReferenceKind):
            raise TypeError(f"kind must be a ReferenceKind, was {type(self.kind).__name__}")
        if not isinstance(self.digest, str):
            raise TypeError(f"digest must be a str, was {type(self.digest).__name__}")
        if not self.name:
            raise ValueError("name must not be empty")
        if not self.digest.startswith("sha256:") or len(self.digest) != _SHA256_DIGEST_LENGTH:
            raise ValueError(f"digest must be a sha256 digest, was {self.digest}")

    def to_dict(self) -> dict[str, object]:
        """
        Return the reference as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {"name": self.name, "kind": self.kind.value, "digest": self.digest}


@dataclass(frozen=True)
class DiscoveryPolicy:
    """
    The bounds, declared sets and frozen references a campaign's candidates are checked against.

    Every collection is held as a tuple or a frozenset so the policy is hashable and its canonical
    form is derivable, and the accessors build the mappings a caller wants rather than storing them.

    Parameters
    ----------
    families : frozenset[str]
        The candidate families a specification's identity block may declare.
    asset_classes : frozenset[str]
        The asset classes a universe block may cover.
    membership_policies : frozenset[str]
        The admissible universe membership policies.
    signal_frequencies : frozenset[str]
        The admissible signal frequencies.
    decision_times : frozenset[str]
        The admissible instants at which a candidate may decide.
    execution_eligibilities : frozenset[str]
        The admissible instants at which a candidate's intent may be eligible to fill.
    corporate_action_treatments : frozenset[str]
        The admissible corporate-action treatments the catalog can honour.
    entry_directions : frozenset[str]
        The admissible entry directions.
    operators : frozenset[str]
        The closed comparison operator set.
    selection_modes : frozenset[str]
        The admissible selection modes.
    tie_breaks : frozenset[str]
        The admissible tie-break rules.
    weighting_methods : frozenset[str]
        The admissible weighting methods.
    cash_policies : frozenset[str]
        The admissible cash policies.
    rebalance_frequencies : frozenset[str]
        The admissible rebalance frequencies.
    rebalance_timings : frozenset[str]
        The admissible rebalance timings.
    feature_allowlist : frozenset[str]
        The feature identifiers a candidate may reference.
    bounds : tuple[Bound, ...]
        The declared bounds, each named by the field that is checked against it.
    limits : ComplexityLimits
        The complexity caps.
    references : tuple[Reference, ...]
        The frozen objects an evaluation block may cite.
    sectors : tuple[tuple[str, str], ...], default ()
        The instrument-to-sector mapping a sector cap is applied through, or empty when undeclared.
    policy_id : str, default POLICY_ID
        The stable identity of the pre-registered policy.
    version : int, default POLICY_VERSION
        The policy version, which changes when any declaration above changes.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If a declared set is empty, or a name repeats.

    """

    families: frozenset[str]
    asset_classes: frozenset[str]
    membership_policies: frozenset[str]
    signal_frequencies: frozenset[str]
    decision_times: frozenset[str]
    execution_eligibilities: frozenset[str]
    corporate_action_treatments: frozenset[str]
    entry_directions: frozenset[str]
    operators: frozenset[str]
    selection_modes: frozenset[str]
    tie_breaks: frozenset[str]
    weighting_methods: frozenset[str]
    cash_policies: frozenset[str]
    rebalance_frequencies: frozenset[str]
    rebalance_timings: frozenset[str]
    feature_allowlist: frozenset[str]
    bounds: tuple[Bound, ...]
    limits: ComplexityLimits
    references: tuple[Reference, ...]
    sectors: tuple[tuple[str, str], ...] = ()
    policy_id: str = POLICY_ID
    version: int = POLICY_VERSION

    def __post_init__(self) -> None:
        """
        Validate the declared sets and refuse a policy that does not say enough to compile against.
        """
        self._check_declared_sets()
        self._check_collections()
        self._check_identity_fields()
        self._check_names_are_unique()

    def _check_declared_sets(self) -> None:
        """
        Validate the declared sets a field's value is resolved from.
        """
        for name in DECLARED_SET_NAMES:
            declared: object = getattr(self, name)
            if not isinstance(declared, frozenset):
                raise TypeError(f"{name} must be a frozenset, was {type(declared).__name__}")
            if not declared:
                raise ValueError(f"{name} must not be empty")
            if not all(isinstance(item, str) and item for item in declared):
                raise ValueError(f"{name} must hold non-empty names")
        if not isinstance(self.feature_allowlist, frozenset):
            raise TypeError(
                "feature_allowlist must be a frozenset, "
                f"was {type(self.feature_allowlist).__name__}",
            )
        if not all(isinstance(item, str) and item for item in self.feature_allowlist):
            raise ValueError("feature_allowlist must hold non-empty identifiers")

    def _check_collections(self) -> None:
        """
        Validate the collections a policy holds.
        """
        if not isinstance(self.bounds, tuple) or not all(
            isinstance(item, Bound) for item in self.bounds
        ):
            raise TypeError("bounds must be a tuple of Bound")
        if not isinstance(self.references, tuple) or not all(
            isinstance(item, Reference) for item in self.references
        ):
            raise TypeError("references must be a tuple of Reference")
        if not isinstance(self.limits, ComplexityLimits):
            raise TypeError(f"limits must be a ComplexityLimits, was {type(self.limits).__name__}")
        if not isinstance(self.sectors, tuple):
            raise TypeError(f"sectors must be a tuple, was {type(self.sectors).__name__}")

    def _check_identity_fields(self) -> None:
        """
        Validate the policy's own identity.
        """
        if not self.policy_id:
            raise ValueError("policy_id must not be empty")
        if isinstance(self.version, bool) or not isinstance(self.version, int):
            raise TypeError(f"version must be an int, was {type(self.version).__name__}")
        if self.version < 1:
            raise ValueError(f"version must be positive, was {self.version}")

    def _check_names_are_unique(self) -> None:
        """
        Refuse a repeated name, because a repeated name makes a lookup ambiguous.
        """
        bound_names = [bound.name for bound in self.bounds]
        if len(set(bound_names)) != len(bound_names):
            raise ValueError("bounds must have distinct names")
        reference_names = [reference.name for reference in self.references]
        if len(set(reference_names)) != len(reference_names):
            raise ValueError("references must have distinct names")
        sector_ids = [instrument_id for instrument_id, _ in self.sectors]
        if len(set(sector_ids)) != len(sector_ids):
            raise ValueError("sectors must name each instrument once")

    def declared(self, name: str) -> frozenset[str]:
        """
        Return the declared set with the given name.

        Parameters
        ----------
        name : str
            The attribute name of the declared set.

        Returns
        -------
        frozenset[str]

        Raises
        ------
        ValueError
            If no declared set carries that name.

        """
        if name not in DECLARED_SET_NAMES:
            raise ValueError(f"{name} is not a declared set of this policy")

        declared: object = getattr(self, name)
        if not isinstance(declared, frozenset):
            raise TypeError(f"{name} must be a frozenset, was {type(declared).__name__}")

        return declared

    def bound(self, name: str) -> Bound | None:
        """
        Return the bound with the given name, or nothing when the policy does not declare it.

        Parameters
        ----------
        name : str
            The bound's name.

        Returns
        -------
        Bound or None

        """
        for bound in self.bounds:
            if bound.name == name:
                return bound

        return None

    def reference(self, name: str) -> Reference | None:
        """
        Return the reference with the given name, or nothing when the policy does not declare it.

        Parameters
        ----------
        name : str
            The reference's name.

        Returns
        -------
        Reference or None

        """
        for reference in self.references:
            if reference.name == name:
                return reference

        return None

    def sector_map(self) -> Mapping[str, str]:
        """
        Return the declared instrument-to-sector mapping.

        Returns
        -------
        Mapping[str, str]

        """
        return dict(self.sectors)

    @property
    def has_sectors(self) -> bool:
        """
        Whether the policy declares a sector for any instrument.
        """
        return bool(self.sectors)

    def to_dict(self) -> dict[str, object]:
        """
        Return the policy as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "policy_id": self.policy_id,
            "version": self.version,
            "families": sorted(self.families),
            "asset_classes": sorted(self.asset_classes),
            "membership_policies": sorted(self.membership_policies),
            "signal_frequencies": sorted(self.signal_frequencies),
            "decision_times": sorted(self.decision_times),
            "execution_eligibilities": sorted(self.execution_eligibilities),
            "corporate_action_treatments": sorted(self.corporate_action_treatments),
            "entry_directions": sorted(self.entry_directions),
            "operators": sorted(self.operators),
            "selection_modes": sorted(self.selection_modes),
            "tie_breaks": sorted(self.tie_breaks),
            "weighting_methods": sorted(self.weighting_methods),
            "cash_policies": sorted(self.cash_policies),
            "rebalance_frequencies": sorted(self.rebalance_frequencies),
            "rebalance_timings": sorted(self.rebalance_timings),
            "feature_allowlist": sorted(self.feature_allowlist),
            "bounds": [bound.to_dict() for bound in self.bounds],
            "limits": self.limits.to_dict(),
            "references": [reference.to_dict() for reference in self.references],
            "sectors": [[instrument_id, sector] for instrument_id, sector in self.sectors],
        }

    def digest(self) -> str:
        """
        Return the digest of the policy, which names the declarations a result ran under.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))

    @classmethod
    def first_release(
        cls,
        *,
        feature_allowlist: Iterable[str],
        references: Iterable[Reference],
        sectors: Iterable[tuple[str, str]] = (),
    ) -> DiscoveryPolicy:
        """
        Return the policy of the declared first-release scope.

        The scope is decision SD12: long only, daily signals, weekly or slower rebalance, equities
        and ETFs, and no shorting, options, intraday logic or fundamental input. The feature
        allowlist, the frozen references and the sector map are campaign inputs rather than scope,
        so they are supplied by the caller and none is invented here.

        Parameters
        ----------
        feature_allowlist : Iterable[str]
            The feature identifiers the campaign declares.
        references : Iterable[Reference]
            The frozen objects the campaign's specifications may cite.
        sectors : Iterable[tuple[str, str]], default ()
            The instrument-to-sector mapping the campaign declares.

        Returns
        -------
        DiscoveryPolicy

        """
        return cls(
            families=frozenset({"cross_sectional_rank", "absolute_threshold", "static_hold"}),
            asset_classes=frozenset({"EQUITY", "ETF"}),
            membership_policies=frozenset({"point_in_time"}),
            signal_frequencies=frozenset({"daily"}),
            decision_times=frozenset({"session_close", "session_open"}),
            execution_eligibilities=frozenset({"same_session_close", "next_session_open"}),
            corporate_action_treatments=frozenset({"adjusted", "raw"}),
            entry_directions=frozenset({"long"}),
            operators=frozenset({"gt", "ge", "lt", "le"}),
            selection_modes=frozenset({"rank", "threshold", "none"}),
            tie_breaks=frozenset({"feature_desc", "instrument_id_asc"}),
            weighting_methods=frozenset({"equal", "rank_proportional"}),
            cash_policies=frozenset({"leave_idle", "reallocate"}),
            rebalance_frequencies=frozenset({"daily", "weekly", "monthly"}),
            rebalance_timings=frozenset({"next_session_open"}),
            feature_allowlist=frozenset(feature_allowlist),
            bounds=_FIRST_RELEASE_BOUNDS,
            limits=ComplexityLimits(),
            references=tuple(references),
            sectors=tuple(sectors),
        )


# The bounds of the first-release scope. Each name is the field the schema checks against it, so a
# candidate's value is admitted only where this table, rather than the candidate, decided the range.
_FIRST_RELEASE_BOUNDS: tuple[Bound, ...] = (
    Bound("feature.lookback", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(252)),
    Bound("feature.skip", BoundKind.RANGE, lower=Decimal(0), upper=Decimal(5)),
    Bound("selection.percentile", BoundKind.RANGE, lower=Decimal("0.01"), upper=Decimal(1)),
    Bound("selection.count", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(50)),
    Bound(
        "selection.threshold",
        BoundKind.RANGE,
        lower=Decimal(-1_000_000),
        upper=Decimal(1_000_000),
    ),
    Bound("condition.value", BoundKind.RANGE, lower=Decimal(-1_000_000), upper=Decimal(1_000_000)),
    Bound("exit.holding_period_cap", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(252)),
    Bound("exit.rank_exit_count", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(50)),
    Bound("exit.stop_fraction", BoundKind.RANGE, lower=Decimal("0.001"), upper=Decimal("0.5")),
    Bound("exit.target_fraction", BoundKind.RANGE, lower=Decimal("0.001"), upper=Decimal(5)),
    Bound("portfolio.position_count", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(50)),
    Bound(
        "portfolio.max_position_weight",
        BoundKind.RANGE,
        lower=Decimal("0.005"),
        upper=Decimal("0.25"),
    ),
    Bound("portfolio.max_sector_weight", BoundKind.RANGE, lower=Decimal("0.1"), upper=Decimal(1)),
    Bound("risk.max_gross_exposure", BoundKind.RANGE, lower=Decimal("0.1"), upper=Decimal(2)),
    Bound("risk.max_drawdown", BoundKind.RANGE, lower=Decimal("0.01"), upper=Decimal("0.9")),
    Bound("risk.max_turnover", BoundKind.RANGE, lower=Decimal("0.05"), upper=Decimal(10)),
    Bound("risk.max_cost_ratio", BoundKind.RANGE, lower=Decimal("0.0001"), upper=Decimal("0.1")),
    Bound("universe.min_price", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(1000)),
    Bound(
        "universe.min_dollar_volume",
        BoundKind.RANGE,
        lower=Decimal(0),
        upper=Decimal(10_000_000_000),
    ),
)
