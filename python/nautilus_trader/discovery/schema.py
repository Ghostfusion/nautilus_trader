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
The closed specification schema: what a candidate may say, and what is refused.

A candidate is data. This module owns its shape: the twelve blocks of the language, the value each
field may hold, the declared set a field's value is resolved from, and the bound a value is checked
against. It owns no behaviour, and it composes nothing but the standard library, so a payload is
checked before anything in the repository is reached.

Three rules make the schema closed rather than merely narrow. Every block refuses a field it does
not declare, so a specification cannot carry a value into a place no validator reads. Every declared
set and bound is resolved from the pre-registered policy, never compared against a constant here, so
what a field may hold is decided outside both the candidate and this module. And a value that
reaches this module as code - a module path, a callable, an expression, a field named for one - is
refused by `scan_for_code_references`, because a specification that could execute something would
not be data.

Versioning is explicit. A specification declares the schema version it was written against, and a
version this build does not accept is migrated by a registered migration or refused, so a field
never silently acquires a different meaning than the one it was written for.

A `Specification` is the view a parsed payload produces, and `parse_specification` is its only gate.
The compiler is handed the payload rather than the view for that reason: the artifact that runs is
the artifact that was hashed, so there is no second path into the language.
"""

from __future__ import annotations

import re
from collections.abc import Mapping
from collections.abc import Sequence
from dataclasses import dataclass
from decimal import Decimal
from decimal import InvalidOperation
from enum import Enum
from typing import TYPE_CHECKING
from typing import cast


if TYPE_CHECKING:
    from collections.abc import Callable

    from nautilus_trader.discovery.policy import DiscoveryPolicy
    from nautilus_trader.optimization.space import JsonValue


# The identity of the schema and the versions this build accepts. A specification carries the
# version it was written against, so a later change to a field's meaning is a migration rather than
# a silent reinterpretation of a stored candidate.
SPECIFICATION_SCHEMA_ID = "discovery.specification"
SPECIFICATION_SCHEMA_VERSION = "discovery.specification/v1"
SPECIFICATION_SCHEMA_VERSIONS: tuple[str, ...] = (SPECIFICATION_SCHEMA_VERSION,)

# The field names that ask for code rather than data, at any depth of a payload. A specification
# naming one of these is refused by name, so the refusal says what was asked for.
_CODE_FIELD_NAMES: frozenset[str] = frozenset(
    {
        "callable",
        "class",
        "class_path",
        "code",
        "code_path",
        "entry_point",
        "eval",
        "exec",
        "expression",
        "hook",
        "import",
        "imports",
        "lambda",
        "module",
        "module_path",
        "script",
        "source_code",
        "strategy_path",
    },
)

# The value shapes that are code rather than data: an import, an evaluation, a shell call, or the
# `module:attribute` form this repository uses to name an importable strategy.
_CODE_VALUE_PATTERNS: tuple[re.Pattern[str], ...] = (
    re.compile(r"\b__import__\b"),
    re.compile(r"\b(?:eval|exec)\s*\("),
    re.compile(r"\b(?:os\.system|subprocess|importlib)\b"),
    re.compile(r"^[A-Za-z_][\w]*(?:\.[A-Za-z_][\w]*)*:[A-Za-z_][\w]*$"),
)

# The maximum number of migrations a single payload may pass through, so a cycle in the migration
# table is a refusal rather than a loop.
_MAX_MIGRATIONS = 8


class BoundKind(Enum):
    """
    The form a declared bound takes: a contiguous range, or an enumerated choice set.
    """

    RANGE = "range"
    CHOICE = "choice"


@dataclass(frozen=True)
class Bound:
    """
    One declared bound, as an inclusive range or an enumerated choice set.

    The two forms are exclusive, and the ambiguous declarations are refused rather than resolved by
    precedence: a bound that states both, or neither, does not say which values are admissible.

    Parameters
    ----------
    name : str
        The bound's name, which is the name a field's bound column refers to.
    kind : BoundKind
        Whether the bound is an inclusive range or an enumerated choice set.
    lower : Decimal, optional
        The inclusive lower limit, for a range bound.
    upper : Decimal, optional
        The inclusive upper limit, for a range bound.
    choices : tuple[Decimal, ...], default ()
        The admissible values, for a choice bound.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If the declaration is ambiguous or degenerate.

    """

    name: str
    kind: BoundKind
    lower: Decimal | None = None
    upper: Decimal | None = None
    choices: tuple[Decimal, ...] = ()

    def __post_init__(self) -> None:
        """
        Validate the declared bound and refuse an ambiguous one.
        """
        if not isinstance(self.name, str):
            raise TypeError(f"name must be a str, was {type(self.name).__name__}")
        if not isinstance(self.kind, BoundKind):
            raise TypeError(f"kind must be a BoundKind, was {type(self.kind).__name__}")
        if not isinstance(self.choices, tuple):
            raise TypeError(f"choices must be a tuple, was {type(self.choices).__name__}")
        if not self.name:
            raise ValueError("name must not be empty")

        if self.kind is BoundKind.RANGE:
            self._check_range()
        else:
            self._check_choices()

    def _check_range(self) -> None:
        """
        Validate the limits of a range bound.
        """
        if self.lower is None or self.upper is None:
            raise ValueError(
                f"a range bound requires both limits, {self.name} states "
                f"{self.lower} and {self.upper}",
            )
        if self.choices:
            raise ValueError(f"a range bound states no choices, {self.name} states choices")
        if self.lower > self.upper:
            raise ValueError(
                f"a range bound requires lower <= upper, {self.name} states "
                f"{self.lower} > {self.upper}",
            )

    def _check_choices(self) -> None:
        """
        Validate the choices of a choice bound.
        """
        if self.lower is not None or self.upper is not None:
            raise ValueError(f"a choice bound states no limits, {self.name} states limits")
        if not self.choices:
            raise ValueError(f"a choice bound requires choices, {self.name} states none")
        if len(set(self.choices)) != len(self.choices):
            raise ValueError(f"a choice bound requires distinct choices, {self.name} repeats one")

    def admits(self, value: Decimal) -> bool:
        """
        Return whether the given value is inside the declared bound.

        Parameters
        ----------
        value : Decimal
            The value to check.

        Returns
        -------
        bool

        """
        if self.kind is BoundKind.CHOICE:
            return value in self.choices

        # A validated range bound carries both limits, which `__post_init__` is what enforces.
        lower = cast("Decimal", self.lower)
        upper = cast("Decimal", self.upper)

        return lower <= value <= upper

    def describe(self) -> str:
        """
        Return the bound in the form an error message states it.

        Returns
        -------
        str

        """
        if self.kind is BoundKind.CHOICE:
            return "one of " + ", ".join(str(choice) for choice in self.choices)

        return f"{self.lower} to {self.upper} inclusive"

    def to_dict(self) -> dict[str, object]:
        """
        Return the bound as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "name": self.name,
            "kind": self.kind.value,
            "lower": None if self.lower is None else str(self.lower),
            "upper": None if self.upper is None else str(self.upper),
            "choices": [str(choice) for choice in self.choices],
        }


def decimal_of(value: object) -> Decimal | None:
    """
    Return the given value as an exact `Decimal`, or nothing when it cannot be one.

    Values arrive from JSON and from fixtures, so an integer, a string and a float are all
    admissible, and each is converted through its own written form rather than through binary
    floating point, which keeps a declared `0.1` exactly one tenth.

    Parameters
    ----------
    value : object
        The value to convert.

    Returns
    -------
    Decimal or None

    """
    if isinstance(value, bool):
        return None
    if isinstance(value, int | float | str):
        try:
            return Decimal(str(value))
        except InvalidOperation:
            return None

    return None


class RefusalCode(Enum):
    """
    The closed vocabulary a refusal is reported with.

    A code is part of the auditable contract: a caller groups, counts and records refusals by code
    rather than by reading a message, so a message may be improved without changing an outcome.
    """

    NOT_A_MAPPING = "not_a_mapping"
    VERSION_MISSING = "version_missing"
    VERSION_NOT_A_STRING = "version_not_a_string"
    VERSION_UNSUPPORTED = "version_unsupported"
    VERSION_MIGRATION_CYCLE = "version_migration_cycle"
    BLOCK_MISSING = "block_missing"
    BLOCK_NOT_A_MAPPING = "block_not_a_mapping"
    BLOCK_UNKNOWN_FIELD = "block_unknown_field"
    FIELD_MISSING = "field_missing"
    FIELD_TYPE = "field_type"
    FIELD_EMPTY = "field_empty"
    FIELD_REPEATED = "field_repeated"
    VALUE_NOT_DECLARED = "value_not_declared"
    BOUND_NOT_DECLARED = "bound_not_declared"
    VALUE_OUT_OF_BOUND = "value_out_of_bound"
    FEATURE_NOT_ALLOWED = "feature_not_allowed"
    FEATURE_NOT_DECLARED = "feature_not_declared"
    FEATURE_COUNT_EXCEEDED = "feature_count_exceeded"
    CONDITION_COUNT_EXCEEDED = "condition_count_exceeded"
    CONDITION_DEPTH_EXCEEDED = "condition_depth_exceeded"
    CONDITION_NOT_RESOLVED = "condition_not_resolved"
    REFERENCE_UNRESOLVED = "reference_unresolved"
    RISK_FIELD_MISPLACED = "risk_field_misplaced"
    CODE_REFERENCE_REFUSED = "code_reference_refused"
    FEATURE_UNRESOLVED = "feature_unresolved"
    SELECTION_INCOMPLETE = "selection_incomplete"
    EXIT_INCOMPLETE = "exit_incomplete"
    REBALANCE_TIMING_MISMATCH = "rebalance_timing_mismatch"
    REBALANCE_FREQUENCY_MISMATCH = "rebalance_frequency_mismatch"
    HOLDING_PERIOD_BELOW_REBALANCE = "holding_period_below_rebalance"
    SECTOR_INPUT_UNDECLARED = "sector_input_undeclared"
    FEATURE_UNAVAILABLE_AT_DECISION = "feature_unavailable_at_decision"
    UNDEFERRED_EXECUTION = "undeferred_execution"


@dataclass(frozen=True)
class Refusal:
    """
    One reason a specification was refused.

    Parameters
    ----------
    code : RefusalCode
        The refusal's code, from the closed vocabulary.
    path : str
        The dotted location in the payload the refusal is about.
    message : str
        What was refused, in the vocabulary the coding standards require.

    """

    code: RefusalCode
    path: str
    message: str

    def to_dict(self) -> dict[str, object]:
        """
        Return the refusal as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {"code": self.code.value, "path": self.path, "message": self.message}


def canonical_mapping(payload: Mapping[str, object]) -> dict[str, JsonValue]:
    """
    Flatten nested plain data into the key-value form the repository's canonical serializer accepts.

    Dictionary keys become dotted paths, sequence elements take their index as a path segment, and a
    leaf keeps its own value, so two payloads that differ anywhere differ in the flat mapping. The
    repository's `digest_of` serializes a flat mapping with sorted keys, so flattening here is what
    lets a nested specification, a policy and a compiled plan share one digest contract.

    Parameters
    ----------
    payload : Mapping[str, object]
        The nested plain data to flatten.

    Returns
    -------
    dict[str, JsonValue]

    Raises
    ------
    TypeError
        If a value is neither a mapping, a sequence of plain data, nor a scalar.

    """
    flat: dict[str, JsonValue] = {}
    _flatten_into(flat, "", payload)

    return flat


def _flatten_into(flat: dict[str, JsonValue], prefix: str, value: object) -> None:
    if isinstance(value, str | int | float | bool) or value is None:
        if prefix:
            flat[prefix] = value
        return
    if isinstance(value, Decimal):
        if prefix:
            flat[prefix] = str(value)
        return
    if isinstance(value, dict):
        for key, item in value.items():
            if not isinstance(key, str):
                raise TypeError(f"a plain-data key must be a string, was {type(key).__name__}")
            _flatten_into(flat, f"{prefix}.{key}" if prefix else key, item)
        return
    if isinstance(value, list | tuple):
        for index, item in enumerate(value):
            _flatten_into(flat, f"{prefix}.{index}", item)
        return

    raise TypeError(
        f"a plain-data value must be a mapping, a sequence or a scalar, was {type(value).__name__}",
    )


# A migration rewrites a payload written against one schema version into the next version. The table
# is empty for the first version, and a migration is added rather than a field being reinterpreted.
type Migration = Callable[[Mapping[str, object]], Mapping[str, object]]

MIGRATIONS: dict[str, Migration] = {}


def migrate(
    payload: Mapping[str, object],
    *,
    migrations: Mapping[str, Migration] | None = None,
) -> tuple[Mapping[str, object], tuple[str, ...], Refusal | None]:
    """
    Apply the registered migrations to a payload until its version is one this build accepts.

    Parameters
    ----------
    payload : Mapping[str, object]
        The payload as it was stored.
    migrations : Mapping[str, Migration], optional
        The migration table to use, defaulting to the module's.

    Returns
    -------
    tuple[Mapping[str, object], tuple[str, ...], Refusal or None]
        The migrated payload, the versions it passed through, and the refusal that stopped it.

    """
    table = MIGRATIONS if migrations is None else migrations
    applied: list[str] = []

    for _ in range(_MAX_MIGRATIONS):
        version = payload.get("version")
        if version in SPECIFICATION_SCHEMA_VERSIONS:
            return payload, tuple(applied), None
        if not isinstance(version, str) or not version:
            code = (
                RefusalCode.VERSION_MISSING if version is None else RefusalCode.VERSION_NOT_A_STRING
            )
            return (
                payload,
                tuple(applied),
                Refusal(
                    code,
                    "version",
                    f"a specification must declare its schema version, one of "
                    f"{', '.join(SPECIFICATION_SCHEMA_VERSIONS)}",
                ),
            )
        migration = table.get(version)
        if migration is None:
            return (
                payload,
                tuple(applied),
                Refusal(
                    RefusalCode.VERSION_UNSUPPORTED,
                    "version",
                    f"schema version {version} is not accepted and no migration from it is "
                    f"registered, this build accepts {', '.join(SPECIFICATION_SCHEMA_VERSIONS)}",
                ),
            )
        payload = migration(payload)
        applied.append(version)

    return (
        payload,
        tuple(applied),
        Refusal(
            RefusalCode.VERSION_MIGRATION_CYCLE,
            "version",
            f"the migrations applied to schema version {payload.get('version')} do not reach an "
            "accepted version",
        ),
    )


def scan_for_code_references(payload: object, path: str = "") -> list[Refusal]:
    """
    Return a refusal for every place in the payload that names code rather than data.

    The scan is the reason the language cannot express an import, a callable or an expression. It
    runs over keys and values at every depth before any block is read, so a value can be refused
    wherever it is put, including in a place the schema does not declare.

    Parameters
    ----------
    payload : object
        The payload to scan.
    path : str, default ""
        The dotted location the scan has reached.

    Returns
    -------
    list[Refusal]

    """
    refusals: list[Refusal] = []

    if isinstance(payload, dict):
        for key, value in payload.items():
            if isinstance(key, str) and key.lower() in _CODE_FIELD_NAMES:
                refusals.append(
                    Refusal(
                        RefusalCode.CODE_REFERENCE_REFUSED,
                        f"{path}.{key}" if path else key,
                        f"a specification is data and may not declare {key}",
                    ),
                )
                continue
            refusals.extend(
                scan_for_code_references(value, f"{path}.{key}" if path else str(key)),
            )
        return refusals
    if isinstance(payload, list | tuple):
        for index, item in enumerate(payload):
            refusals.extend(scan_for_code_references(item, f"{path}.{index}"))
        return refusals
    plain = isinstance(payload, str | int | float | bool | Decimal | type(None))
    if callable(payload) or not plain:
        refusals.append(
            Refusal(
                RefusalCode.CODE_REFERENCE_REFUSED,
                path,
                f"a specification is data and may not carry a {type(payload).__name__}",
            ),
        )
        return refusals
    if isinstance(payload, str):
        for pattern in _CODE_VALUE_PATTERNS:
            if pattern.search(payload):
                refusals.append(
                    Refusal(
                        RefusalCode.CODE_REFERENCE_REFUSED,
                        path,
                        f"a specification value may not be executable code, {payload!r} is",
                    ),
                )
                break

    return refusals


@dataclass(frozen=True)
class IdentityBlock:
    """
    What the candidate claims, in the words it will be audited by.
    """

    family: str
    hypothesis: str
    mechanism: str
    falsification: str


@dataclass(frozen=True)
class UniverseBlock:
    """
    Which instruments the candidate may hold, and the floors that decide membership.
    """

    asset_classes: tuple[str, ...]
    membership_policy: str
    source: str
    min_price: Decimal | None = None
    min_dollar_volume: Decimal | None = None
    exclusions: tuple[str, ...] = ()


@dataclass(frozen=True)
class DataBlock:
    """
    When the candidate decides, when its intent may fill, and how actions are treated.
    """

    signal_frequency: str
    decision_time: str
    execution_eligibility: str
    corporate_action_treatment: str


@dataclass(frozen=True)
class FeatureDeclaration:
    """
    One reference to a registered feature, under the alias the rest of the specification uses.

    Parameters
    ----------
    alias : str
        The name the selection, entry and exit blocks refer to.
    feature_id : str
        The identifier of the registered feature.
    lookback : int
        The lookback length the value is read at.
    skip : int
        The number of observations skipped before the lookback window.

    """

    alias: str
    feature_id: str
    lookback: int
    skip: int


@dataclass(frozen=True)
class FeaturesBlock:
    """
    The features the candidate selects, and the aliases it refers to them by.
    """

    declarations: tuple[FeatureDeclaration, ...]


@dataclass(frozen=True)
class SelectionBlock:
    """
    How the candidate's eligible set is chosen, and how a tie in it is broken.
    """

    mode: str
    feature: str | None = None
    percentile: Decimal | None = None
    count: Decimal | None = None
    threshold: Decimal | None = None
    comparison: str | None = None
    tie_break: str | None = None


@dataclass(frozen=True)
class ConditionLeaf:
    """
    One comparison between a declared feature and a bounded value.
    """

    feature: str
    comparison: str
    value: Decimal


@dataclass(frozen=True)
class ConditionAll:
    """
    A conjunction of conditions.
    """

    conditions: tuple[ConditionNode, ...]


@dataclass(frozen=True)
class ConditionAny:
    """
    A disjunction of conditions.
    """

    conditions: tuple[ConditionNode, ...]


type ConditionNode = ConditionLeaf | ConditionAll | ConditionAny


@dataclass(frozen=True)
class EntryBlock:
    """
    Whether and when the candidate enters an eligible instrument.
    """

    direction: str
    conditions: ConditionNode | None = None


@dataclass(frozen=True)
class ExitBlock:
    """
    The declared ways a position leaves the portfolio.
    """

    holding_period_cap: int | None = None
    rank_exit_count: int | None = None
    feature: str | None = None
    comparison: str | None = None
    threshold: Decimal | None = None
    stop_fraction: Decimal | None = None
    target_fraction: Decimal | None = None
    flatten_ineligible: bool = True


@dataclass(frozen=True)
class PortfolioBlock:
    """
    The candidate's own exposure decisions, bounded by the risk policy.
    """

    position_count: int
    weighting: str
    max_position_weight: Decimal
    cash_policy: str
    max_sector_weight: Decimal | None = None


@dataclass(frozen=True)
class RebalanceBlock:
    """
    How often the candidate rebalances, and when its resulting intent is eligible.
    """

    frequency: str
    timing: str


@dataclass(frozen=True)
class RiskPreTradeBlock:
    """
    The constraint the allocation function enforces before targets are emitted.
    """

    max_gross_exposure: Decimal | None = None


@dataclass(frozen=True)
class RiskRuntimeBlock:
    """
    The controls the interpreter applies while the run is live.
    """

    halt_new_entries: bool = False
    flatten_ineligible: bool = False


@dataclass(frozen=True)
class RiskPostRunBlock:
    """
    The acceptance limits the gate evaluates once the observations exist.
    """

    max_drawdown: Decimal | None = None
    max_turnover: Decimal | None = None
    max_cost_ratio: Decimal | None = None


@dataclass(frozen=True)
class RiskBlock:
    """
    The three categories of risk field, kept apart by when a control acts.
    """

    pre_trade: RiskPreTradeBlock
    runtime: RiskRuntimeBlock
    post_run: RiskPostRunBlock


@dataclass(frozen=True)
class EvaluationBlock:
    """
    The frozen objects the candidate is evaluated against, named rather than stated.
    """

    benchmark: str
    cost_model: str
    validation_policy: str


@dataclass(frozen=True)
class ProvenanceBlock:
    """
    Where the candidate came from, recorded rather than trusted.
    """

    generator: str
    seed: int
    parents: tuple[str, ...] = ()


@dataclass(frozen=True)
class Specification:
    """
    A parsed, normalised specification, which is the thing that is hashed and the thing that runs.
    """

    version: str
    identity: IdentityBlock
    universe: UniverseBlock
    data: DataBlock
    features: FeaturesBlock
    selection: SelectionBlock
    entry: EntryBlock
    exit: ExitBlock
    portfolio: PortfolioBlock
    rebalance: RebalanceBlock
    risk: RiskBlock
    evaluation: EvaluationBlock
    provenance: ProvenanceBlock

    def to_dict(self) -> dict[str, object]:
        """
        Return the specification in its canonical, normalised form.

        Returns
        -------
        dict[str, object]

        """
        return {
            "version": self.version,
            "identity": {
                "family": self.identity.family,
                "hypothesis": self.identity.hypothesis,
                "mechanism": self.identity.mechanism,
                "falsification": self.identity.falsification,
            },
            "universe": {
                "asset_classes": list(self.universe.asset_classes),
                "membership_policy": self.universe.membership_policy,
                "source": self.universe.source,
                "min_price": _decimal_text(self.universe.min_price),
                "min_dollar_volume": _decimal_text(self.universe.min_dollar_volume),
                "exclusions": list(self.universe.exclusions),
            },
            "data": {
                "signal_frequency": self.data.signal_frequency,
                "decision_time": self.data.decision_time,
                "execution_eligibility": self.data.execution_eligibility,
                "corporate_action_treatment": self.data.corporate_action_treatment,
            },
            "features": {
                "declared": {
                    declaration.alias: {
                        "feature": declaration.feature_id,
                        "lookback": declaration.lookback,
                        "skip": declaration.skip,
                    }
                    for declaration in self.features.declarations
                },
            },
            "selection": {
                "mode": self.selection.mode,
                "feature": self.selection.feature,
                "percentile": _decimal_text(self.selection.percentile),
                "count": _decimal_text(self.selection.count),
                "threshold": _decimal_text(self.selection.threshold),
                "comparison": self.selection.comparison,
                "tie_break": self.selection.tie_break,
            },
            "entry": {
                "direction": self.entry.direction,
                "conditions": None
                if self.entry.conditions is None
                else _condition_to_dict(self.entry.conditions),
            },
            "exit": {
                "holding_period_cap": self.exit.holding_period_cap,
                "rank_exit_count": self.exit.rank_exit_count,
                "feature": self.exit.feature,
                "comparison": self.exit.comparison,
                "threshold": _decimal_text(self.exit.threshold),
                "stop_fraction": _decimal_text(self.exit.stop_fraction),
                "target_fraction": _decimal_text(self.exit.target_fraction),
                "flatten_ineligible": self.exit.flatten_ineligible,
            },
            "portfolio": {
                "position_count": self.portfolio.position_count,
                "weighting": self.portfolio.weighting,
                "max_position_weight": _decimal_text(self.portfolio.max_position_weight),
                "max_sector_weight": _decimal_text(self.portfolio.max_sector_weight),
                "cash_policy": self.portfolio.cash_policy,
            },
            "rebalance": {
                "frequency": self.rebalance.frequency,
                "timing": self.rebalance.timing,
            },
            "risk": {
                "pre_trade": {
                    "max_gross_exposure": _decimal_text(self.risk.pre_trade.max_gross_exposure),
                },
                "runtime": {
                    "halt_new_entries": self.risk.runtime.halt_new_entries,
                    "flatten_ineligible": self.risk.runtime.flatten_ineligible,
                },
                "post_run": {
                    "max_drawdown": _decimal_text(self.risk.post_run.max_drawdown),
                    "max_turnover": _decimal_text(self.risk.post_run.max_turnover),
                    "max_cost_ratio": _decimal_text(self.risk.post_run.max_cost_ratio),
                },
            },
            "evaluation": {
                "benchmark": self.evaluation.benchmark,
                "cost_model": self.evaluation.cost_model,
                "validation_policy": self.evaluation.validation_policy,
            },
            "provenance": {
                "parents": list(self.provenance.parents),
                "generator": self.provenance.generator,
                "seed": self.provenance.seed,
            },
        }


@dataclass(frozen=True)
class ParsedSpecification:
    """
    The outcome of parsing a payload: the specification, or the refusals that stopped it.

    Parameters
    ----------
    specification : Specification, optional
        The parsed specification, present only when no refusal was recorded.
    refusals : tuple[Refusal, ...]
        Every refusal the parse recorded.
    migrations : tuple[str, ...]
        The schema versions the payload passed through before it was parsed.

    """

    specification: Specification | None
    refusals: tuple[Refusal, ...]
    migrations: tuple[str, ...] = ()

    @property
    def accepted(self) -> bool:
        """
        Whether the payload was accepted.
        """
        return self.specification is not None and not self.refusals


def _decimal_text(value: Decimal | None) -> str | None:
    return None if value is None else str(value)


def _condition_to_dict(node: ConditionNode) -> dict[str, object]:
    if isinstance(node, ConditionLeaf):
        return {"feature": node.feature, "comparison": node.comparison, "value": str(node.value)}
    if isinstance(node, ConditionAll):
        return {"all": [_condition_to_dict(item) for item in node.conditions]}

    # The union has three members and two are handled above, so this is the disjunction.
    return {"any": [_condition_to_dict(item) for item in node.conditions]}


class _Reader:
    """
    Read one block's fields, recording a refusal rather than raising on the first one.

    The read methods are the schema: a field that is not read is a field the block does not declare,
    and `finish` refuses it. The allowed set is therefore never a second table that can drift from
    the reads.
    """

    def __init__(
        self,
        path: str,
        payload: object,
        policy: DiscoveryPolicy,
        refusals: list[Refusal],
    ) -> None:
        self.path = path
        self.policy = policy
        self.refusals = refusals
        self._read: set[str] = set()
        self._payload: Mapping[str, object] = {}

        if not isinstance(payload, Mapping):
            refusals.append(
                Refusal(
                    RefusalCode.BLOCK_NOT_A_MAPPING,
                    path,
                    f"a block must be a mapping, was {type(payload).__name__}",
                ),
            )
            return

        self._payload = payload

        refusals.extend(
            Refusal(
                RefusalCode.BLOCK_UNKNOWN_FIELD,
                path,
                f"a field name must be a string, was {type(key).__name__}",
            )
            for key in payload
            if not isinstance(key, str)
        )

    def refuse(self, code: RefusalCode, field_path: str, message: str) -> None:
        self.refusals.append(Refusal(code, field_path, message))

    def field_path(self, name: str) -> str:
        return f"{self.path}.{name}" if self.path else name

    def take(self, name: str) -> tuple[bool, object]:
        self._read.add(name)

        return name in self._payload, self._payload.get(name)

    def finish(self) -> None:
        """
        Refuse every field the block does not declare.
        """
        for key in self._payload:
            if isinstance(key, str) and key not in self._read:
                self.refuse(
                    RefusalCode.BLOCK_UNKNOWN_FIELD,
                    self.field_path(key),
                    f"the {self.path} block does not declare {key}",
                )

    def text(self, name: str, *, required: bool = True) -> str | None:
        """
        Read a non-empty text field.
        """
        present, value = self.take(name)
        if not present or value is None:
            if required:
                self.refuse(
                    RefusalCode.FIELD_MISSING,
                    self.field_path(name),
                    f"{name} is required",
                )
            return None
        if not isinstance(value, str):
            self.refuse(
                RefusalCode.FIELD_TYPE,
                self.field_path(name),
                f"{name} must be a string, was {type(value).__name__}",
            )
            return None
        if not value.strip():
            self.refuse(RefusalCode.FIELD_EMPTY, self.field_path(name), f"{name} must not be empty")
            return None

        return value

    def boolean(self, name: str, *, default: bool | None = None) -> bool | None:
        """
        Read a boolean field.
        """
        present, value = self.take(name)
        if not present or value is None:
            if default is None and not present:
                self.refuse(
                    RefusalCode.FIELD_MISSING,
                    self.field_path(name),
                    f"{name} is required",
                )
            return default
        if not isinstance(value, bool):
            self.refuse(
                RefusalCode.FIELD_TYPE,
                self.field_path(name),
                f"{name} must be a bool, was {type(value).__name__}",
            )
            return default

        return value

    def integer(self, name: str, *, required: bool = True) -> int | None:
        """
        Read an integer field.
        """
        present, value = self.take(name)
        if not present or value is None:
            if required:
                self.refuse(
                    RefusalCode.FIELD_MISSING,
                    self.field_path(name),
                    f"{name} is required",
                )
            return None
        if isinstance(value, bool) or not isinstance(value, int):
            self.refuse(
                RefusalCode.FIELD_TYPE,
                self.field_path(name),
                f"{name} must be an int, was {type(value).__name__}",
            )
            return None

        return value

    def bounded(self, name: str, bound_name: str, *, required: bool = True) -> Decimal | None:
        """
        Read a decimal field and check it against the policy's bound of that name.
        """
        present, value = self.take(name)
        if not present or value is None:
            if required:
                self.refuse(
                    RefusalCode.FIELD_MISSING,
                    self.field_path(name),
                    f"{name} is required",
                )
            return None

        number = decimal_of(value)
        if number is None:
            self.refuse(
                RefusalCode.FIELD_TYPE,
                self.field_path(name),
                f"{name} must be a number, was {type(value).__name__}",
            )
            return None

        bound = self.bound(bound_name)
        if bound is None:
            return None
        if not bound.admits(number):
            self.refuse(
                RefusalCode.VALUE_OUT_OF_BOUND,
                self.field_path(name),
                f"{name} must be {bound.describe()}, was {number}",
            )
            return None

        return number

    def bounded_int(self, name: str, bound_name: str, *, required: bool = True) -> int | None:
        """
        Read an integer field and check it against the policy's bound of that name.
        """
        number = self.bounded(name, bound_name, required=required)
        if number is None:
            return None
        if number != number.to_integral_value():
            self.refuse(
                RefusalCode.FIELD_TYPE,
                self.field_path(name),
                f"{name} must be an int, was {number}",
            )
            return None

        return int(number)

    def declared(self, name: str, set_name: str, *, required: bool = True) -> str | None:
        """
        Read a field whose value is one of the policy's declared names.
        """
        value = self.text(name, required=required)
        if value is None:
            return None

        declared = self.policy.declared(set_name)
        if value not in declared:
            self.refuse(
                RefusalCode.VALUE_NOT_DECLARED,
                self.field_path(name),
                f"{name} must be one of {', '.join(sorted(declared))}, was {value}",
            )
            return None

        return value

    def declared_list(self, name: str, set_name: str, *, required: bool = True) -> tuple[str, ...]:
        """
        Read a sequence whose every element is one of the policy's declared names.
        """
        values = self.identifiers(name, required=required)
        declared = self.policy.declared(set_name)
        admitted: list[str] = []

        for value in values:
            if value not in declared:
                self.refuse(
                    RefusalCode.VALUE_NOT_DECLARED,
                    self.field_path(name),
                    f"{name} entries must be one of {', '.join(sorted(declared))}, was {value}",
                )
                continue
            admitted.append(value)

        return tuple(admitted)

    def identifiers(self, name: str, *, required: bool = False) -> tuple[str, ...]:
        """
        Read a sequence of non-empty, distinct identifiers.
        """
        present, value = self.take(name)
        if not present or value is None:
            if required:
                self.refuse(
                    RefusalCode.FIELD_MISSING,
                    self.field_path(name),
                    f"{name} is required",
                )
            return ()
        if isinstance(value, str) or not isinstance(value, Sequence):
            self.refuse(
                RefusalCode.FIELD_TYPE,
                self.field_path(name),
                f"{name} must be a sequence, was {type(value).__name__}",
            )
            return ()

        seen: list[str] = []
        for index, item in enumerate(value):
            if not isinstance(item, str) or not item.strip():
                self.refuse(
                    RefusalCode.FIELD_TYPE,
                    f"{self.field_path(name)}.{index}",
                    f"{name} entries must be non-empty strings, was {type(item).__name__}",
                )
                continue
            if item in seen:
                self.refuse(
                    RefusalCode.FIELD_REPEATED,
                    f"{self.field_path(name)}.{index}",
                    f"{name} states {item} more than once",
                )
                continue
            seen.append(item)

        return tuple(seen)

    def feature_ref(
        self,
        name: str,
        declared: frozenset[str],
        *,
        required: bool = True,
    ) -> str | None:
        """
        Read a field that must name one of the features the specification declares.
        """
        value = self.text(name, required=required)
        if value is None:
            return None
        if value not in declared:
            self.refuse(
                RefusalCode.FEATURE_NOT_DECLARED,
                self.field_path(name),
                f"{name} must name a declared feature, {value} is not declared",
            )
            return None

        return value

    def reference(self, name: str, *, required: bool = True) -> str | None:
        """
        Read a field that must name one of the policy's frozen references.
        """
        value = self.text(name, required=required)
        if value is None:
            return None
        if self.policy.reference(value) is None:
            declared = ", ".join(reference.name for reference in self.policy.references)
            self.refuse(
                RefusalCode.REFERENCE_UNRESOLVED,
                self.field_path(name),
                f"{name} must name a frozen reference, {value} is not declared"
                + (f", the policy declares {declared}" if declared else ""),
            )
            return None

        return value

    def bound(self, bound_name: str) -> Bound | None:
        bound = self.policy.bound(bound_name)
        if bound is None:
            self.refusals.append(
                Refusal(
                    RefusalCode.BOUND_NOT_DECLARED,
                    self.path,
                    f"the policy declares no bound named {bound_name}, so the {self.path} block "
                    "cannot admit a value",
                ),
            )

        return bound


def _read_identity(reader: _Reader) -> IdentityBlock:
    return IdentityBlock(
        family=reader.declared("family", "families") or "",
        hypothesis=reader.text("hypothesis") or "",
        mechanism=reader.text("mechanism") or "",
        falsification=reader.text("falsification") or "",
    )


def _read_universe(reader: _Reader) -> UniverseBlock:
    return UniverseBlock(
        asset_classes=reader.declared_list("asset_classes", "asset_classes"),
        membership_policy=reader.declared("membership_policy", "membership_policies") or "",
        source=reader.text("source") or "",
        min_price=reader.bounded("min_price", "universe.min_price", required=False),
        min_dollar_volume=reader.bounded(
            "min_dollar_volume",
            "universe.min_dollar_volume",
            required=False,
        ),
        exclusions=reader.identifiers("exclusions"),
    )


def _read_data(reader: _Reader) -> DataBlock:
    return DataBlock(
        signal_frequency=reader.declared("signal_frequency", "signal_frequencies") or "",
        decision_time=reader.declared("decision_time", "decision_times") or "",
        execution_eligibility=reader.declared(
            "execution_eligibility",
            "execution_eligibilities",
        )
        or "",
        corporate_action_treatment=reader.declared(
            "corporate_action_treatment",
            "corporate_action_treatments",
        )
        or "",
    )


def _read_features(reader: _Reader) -> FeaturesBlock:
    present, payload = reader.take("declared")

    if not present or payload is None:
        reader.refuse(
            RefusalCode.FIELD_MISSING,
            reader.field_path("declared"),
            "declared is required",
        )
        return FeaturesBlock(())

    if not isinstance(payload, Mapping):
        reader.refuse(
            RefusalCode.FIELD_TYPE,
            reader.field_path("declared"),
            f"declared must be a mapping of aliases, was {type(payload).__name__}",
        )
        return FeaturesBlock(())

    limit = reader.policy.limits.max_features
    if len(payload) > limit:
        reader.refuse(
            RefusalCode.FEATURE_COUNT_EXCEEDED,
            reader.field_path("declared"),
            f"a specification declares at most {limit} features, was {len(payload)}",
        )

    declarations: list[FeatureDeclaration] = []
    for alias, body in payload.items():
        path = f"{reader.field_path('declared')}.{alias}"
        if not isinstance(alias, str) or not alias.strip():
            reader.refuse(
                RefusalCode.FIELD_TYPE,
                path,
                f"a feature alias must be a non-empty string, was {type(alias).__name__}",
            )
            continue

        entry = _Reader(path, body, reader.policy, reader.refusals)
        feature_id = entry.text("feature")
        if feature_id is not None and feature_id not in reader.policy.feature_allowlist:
            entry.refuse(
                RefusalCode.FEATURE_NOT_ALLOWED,
                f"{path}.feature",
                f"feature must be one of the policy's registered features, {feature_id} is not",
            )
            feature_id = None

        lookback = entry.bounded_int("lookback", "feature.lookback")
        skip = entry.bounded_int("skip", "feature.skip", required=False)
        entry.finish()

        declarations.append(
            FeatureDeclaration(
                alias=alias,
                feature_id=feature_id or "",
                lookback=lookback or 0,
                skip=0 if skip is None else skip,
            ),
        )

    return FeaturesBlock(tuple(declarations))


def _read_selection(reader: _Reader, declared: frozenset[str]) -> SelectionBlock:
    return SelectionBlock(
        mode=reader.declared("mode", "selection_modes") or "",
        feature=reader.feature_ref("feature", declared, required=False),
        percentile=reader.bounded("percentile", "selection.percentile", required=False),
        count=reader.bounded("count", "selection.count", required=False),
        threshold=reader.bounded("threshold", "selection.threshold", required=False),
        comparison=reader.declared("comparison", "operators", required=False),
        tie_break=reader.declared("tie_break", "tie_breaks", required=False),
    )


@dataclass(frozen=True)
class _ConditionWalk:
    """
    Where a condition tree walk has reached: the reader, the declared aliases and the depth.
    """

    reader: _Reader
    declared: frozenset[str]
    path: str
    depth: int = 1

    def descend(self, path: str) -> _ConditionWalk:
        """
        Return the walk state of a nested condition.

        Parameters
        ----------
        path : str
            The dotted location of the nested condition.

        Returns
        -------
        _ConditionWalk

        """
        return _ConditionWalk(
            reader=self.reader,
            declared=self.declared,
            path=path,
            depth=self.depth + 1,
        )


def count_condition_leaves(node: ConditionNode) -> int:
    """
    Return the number of leaf comparisons in a condition tree.

    Parameters
    ----------
    node : ConditionNode
        The tree, or any node of it.

    Returns
    -------
    int

    """
    if isinstance(node, ConditionLeaf):
        return 1

    return sum(count_condition_leaves(child) for child in node.conditions)


def _read_condition(payload: object, walk: _ConditionWalk) -> ConditionNode | None:
    limit = walk.reader.policy.limits

    if walk.depth > limit.max_condition_depth:
        walk.reader.refuse(
            RefusalCode.CONDITION_DEPTH_EXCEEDED,
            walk.path,
            f"a condition tree nests at most {limit.max_condition_depth} levels, was {walk.depth}",
        )
        return None

    if not isinstance(payload, Mapping):
        walk.reader.refuse(
            RefusalCode.CONDITION_NOT_RESOLVED,
            walk.path,
            f"a condition must be a comparison, an all group or an any group, was "
            f"{type(payload).__name__}",
        )
        return None

    groups = [group for group in ("all", "any") if group in payload]
    if groups:
        return _read_condition_group(payload, groups, walk)

    return _read_condition_leaf(payload, walk)


def _read_condition_group(
    payload: Mapping[str, object],
    groups: list[str],
    walk: _ConditionWalk,
) -> ConditionNode | None:
    for key in payload:
        if isinstance(key, str) and key not in {"all", "any"}:
            walk.reader.refuse(
                RefusalCode.BLOCK_UNKNOWN_FIELD,
                f"{walk.path}.{key}",
                f"a condition group does not declare {key}",
            )
    if len(groups) > 1:
        walk.reader.refuse(
            RefusalCode.CONDITION_NOT_RESOLVED,
            walk.path,
            "a condition states all or any, not both",
        )
        return None

    group = groups[0]
    body = payload.get(group)
    if not isinstance(body, Sequence) or isinstance(body, str) or not body:
        walk.reader.refuse(
            RefusalCode.CONDITION_NOT_RESOLVED,
            f"{walk.path}.{group}",
            f"the {group} group must hold at least one condition",
        )
        return None

    children: list[ConditionNode] = []
    for index, item in enumerate(body):
        child = _read_condition(item, walk.descend(f"{walk.path}.{group}.{index}"))
        if child is not None:
            children.append(child)

    if group == "all":
        return ConditionAll(tuple(children))

    return ConditionAny(tuple(children))


def _read_condition_leaf(
    payload: Mapping[str, object], walk: _ConditionWalk
) -> ConditionLeaf | None:
    leaf = _Reader(walk.path, payload, walk.reader.policy, walk.reader.refusals)
    feature = leaf.feature_ref("feature", walk.declared)
    comparison = leaf.declared("comparison", "operators")
    value = leaf.bounded("value", "condition.value")
    leaf.finish()

    if feature is None or comparison is None or value is None:
        return None

    return ConditionLeaf(feature=feature, comparison=comparison, value=value)


def _read_entry(reader: _Reader, declared: frozenset[str]) -> EntryBlock:
    direction = reader.declared("direction", "entry_directions") or ""
    present, payload = reader.take("conditions")

    if not present or payload is None:
        return EntryBlock(direction=direction)

    path = reader.field_path("conditions")
    walk = _ConditionWalk(reader=reader, declared=declared, path=path)
    conditions = _read_condition(payload, walk)

    if conditions is None:
        return EntryBlock(direction=direction)

    leaves = count_condition_leaves(conditions)
    limit = reader.policy.limits.max_conditions
    if leaves > limit:
        reader.refuse(
            RefusalCode.CONDITION_COUNT_EXCEEDED,
            path,
            f"an entry block states at most {limit} conditions, was {leaves}",
        )
        return EntryBlock(direction=direction)

    return EntryBlock(direction=direction, conditions=conditions)


def _read_exit(reader: _Reader, declared: frozenset[str]) -> ExitBlock:
    return ExitBlock(
        holding_period_cap=reader.bounded_int(
            "holding_period_cap",
            "exit.holding_period_cap",
            required=False,
        ),
        rank_exit_count=reader.bounded_int(
            "rank_exit_count",
            "exit.rank_exit_count",
            required=False,
        ),
        feature=reader.feature_ref("feature", declared, required=False),
        comparison=reader.declared("comparison", "operators", required=False),
        threshold=reader.bounded("threshold", "condition.value", required=False),
        stop_fraction=reader.bounded("stop_fraction", "exit.stop_fraction", required=False),
        target_fraction=reader.bounded("target_fraction", "exit.target_fraction", required=False),
        flatten_ineligible=reader.boolean("flatten_ineligible", default=True) or False,
    )


def _read_portfolio(reader: _Reader) -> PortfolioBlock:
    return PortfolioBlock(
        position_count=reader.bounded_int("position_count", "portfolio.position_count") or 0,
        weighting=reader.declared("weighting", "weighting_methods") or "",
        max_position_weight=reader.bounded("max_position_weight", "portfolio.max_position_weight")
        or Decimal(0),
        max_sector_weight=reader.bounded(
            "max_sector_weight",
            "portfolio.max_sector_weight",
            required=False,
        ),
        cash_policy=reader.declared("cash_policy", "cash_policies") or "",
    )


def _read_rebalance(reader: _Reader) -> RebalanceBlock:
    return RebalanceBlock(
        frequency=reader.declared("frequency", "rebalance_frequencies") or "",
        timing=reader.declared("timing", "rebalance_timings") or "",
    )


def _read_risk(reader: _Reader) -> RiskBlock:
    _refuse_misplaced_risk_fields(reader)

    pre_trade_reader = _nested(reader, "pre_trade")
    pre_trade = RiskPreTradeBlock(
        max_gross_exposure=pre_trade_reader.bounded(
            "max_gross_exposure",
            "risk.max_gross_exposure",
            required=False,
        ),
    )
    pre_trade_reader.finish()

    runtime_reader = _nested(reader, "runtime")
    runtime = RiskRuntimeBlock(
        halt_new_entries=runtime_reader.boolean("halt_new_entries", default=False) or False,
        flatten_ineligible=runtime_reader.boolean("flatten_ineligible", default=False) or False,
    )
    runtime_reader.finish()

    post_run_reader = _nested(reader, "post_run")
    post_run = RiskPostRunBlock(
        max_drawdown=post_run_reader.bounded("max_drawdown", "risk.max_drawdown", required=False),
        max_turnover=post_run_reader.bounded("max_turnover", "risk.max_turnover", required=False),
        max_cost_ratio=post_run_reader.bounded(
            "max_cost_ratio",
            "risk.max_cost_ratio",
            required=False,
        ),
    )
    post_run_reader.finish()

    return RiskBlock(pre_trade=pre_trade, runtime=runtime, post_run=post_run)


def _refuse_misplaced_risk_fields(reader: _Reader) -> None:
    for category, fields in _RISK_CATEGORY_FIELDS.items():
        _, payload = reader.take(category)
        if not isinstance(payload, Mapping):
            continue
        for name in payload:
            if not isinstance(name, str) or name in fields:
                continue
            for other, other_fields in _RISK_CATEGORY_FIELDS.items():
                if other != category and name in other_fields:
                    reader.refuse(
                        RefusalCode.RISK_FIELD_MISPLACED,
                        f"{reader.field_path(category)}.{name}",
                        f"{name} is a {other.replace('_', '-')} field and does not control "
                        f"behaviour, so it belongs under {other}",
                    )


def _nested(reader: _Reader, name: str) -> _Reader:
    _, payload = reader.take(name)
    if payload is None:
        return _Reader(reader.field_path(name), {}, reader.policy, reader.refusals)

    return _Reader(reader.field_path(name), payload, reader.policy, reader.refusals)


def _read_evaluation(reader: _Reader) -> EvaluationBlock:
    return EvaluationBlock(
        benchmark=reader.reference("benchmark") or "",
        cost_model=reader.reference("cost_model") or "",
        validation_policy=reader.reference("validation_policy") or "",
    )


def _read_provenance(reader: _Reader) -> ProvenanceBlock:
    return ProvenanceBlock(
        generator=reader.text("generator") or "",
        seed=reader.integer("seed") or 0,
        parents=reader.identifiers("parents"),
    )


# The risk fields by category, which is what makes a misplaced one refusable rather than ignored. A
# limit measured after a run cannot control it, so the category is part of the field's meaning.
_RISK_CATEGORY_FIELDS: dict[str, tuple[str, ...]] = {
    "pre_trade": ("max_gross_exposure",),
    "runtime": ("halt_new_entries", "flatten_ineligible"),
    "post_run": ("max_drawdown", "max_turnover", "max_cost_ratio"),
}

_BLOCK_NAMES: tuple[str, ...] = (
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


def parse_specification(
    payload: object,
    policy: DiscoveryPolicy,
    *,
    migrations: Mapping[str, Migration] | None = None,
) -> ParsedSpecification:
    """
    Parse and validate a candidate specification against the closed schema and the policy's bounds.

    Parsing collects every refusal rather than stopping at the first, because a failed compilation
    is an auditable outcome rather than an error to be retried until the validator relents.

    Parameters
    ----------
    payload : object
        The specification as data.
    policy : DiscoveryPolicy
        The pre-registered policy whose declared sets and bounds the payload is checked against.
    migrations : Mapping[str, Migration], optional
        The migration table to use, defaulting to the module's.

    Returns
    -------
    ParsedSpecification

    """
    if not isinstance(payload, Mapping):
        return ParsedSpecification(
            specification=None,
            refusals=(
                Refusal(
                    RefusalCode.NOT_A_MAPPING,
                    "",
                    f"a specification must be a mapping, was {type(payload).__name__}",
                ),
            ),
        )

    refusals = scan_for_code_references(payload)
    if refusals:
        return ParsedSpecification(specification=None, refusals=tuple(refusals))

    payload, applied, refusal = migrate(payload, migrations=migrations)
    if refusal is not None:
        return ParsedSpecification(specification=None, refusals=(refusal,), migrations=applied)

    reader = _Reader("", payload, policy, refusals)

    for name in _BLOCK_NAMES:
        present, value = reader.take(name)
        if not present or value is None:
            reader.refuse(RefusalCode.BLOCK_MISSING, name, f"the {name} block is required")

    version = reader.text("version")
    reader.finish()

    if refusals:
        return ParsedSpecification(specification=None, refusals=tuple(refusals), migrations=applied)

    # The features block is read first because the selection, entry and exit blocks refer to the
    # aliases it declares, so a reference can be refused as undeclared rather than resolved later.
    features = _read_block(payload, "features", policy, refusals, _read_features)
    declared: frozenset[str] = frozenset(declaration.alias for declaration in features.declarations)

    identity = _read_block(payload, "identity", policy, refusals, _read_identity)
    universe = _read_block(payload, "universe", policy, refusals, _read_universe)
    data = _read_block(payload, "data", policy, refusals, _read_data)
    selection = _read_block(
        payload,
        "selection",
        policy,
        refusals,
        lambda block: _read_selection(block, declared),
    )
    entry = _read_block(
        payload,
        "entry",
        policy,
        refusals,
        lambda block: _read_entry(block, declared),
    )
    exit_block = _read_block(
        payload,
        "exit",
        policy,
        refusals,
        lambda block: _read_exit(block, declared),
    )
    portfolio = _read_block(payload, "portfolio", policy, refusals, _read_portfolio)
    rebalance = _read_block(payload, "rebalance", policy, refusals, _read_rebalance)
    risk = _read_block(payload, "risk", policy, refusals, _read_risk)
    evaluation = _read_block(payload, "evaluation", policy, refusals, _read_evaluation)
    provenance = _read_block(payload, "provenance", policy, refusals, _read_provenance)

    if refusals:
        return ParsedSpecification(specification=None, refusals=tuple(refusals), migrations=applied)

    specification = Specification(
        version=version or SPECIFICATION_SCHEMA_VERSION,
        identity=identity,
        universe=universe,
        data=data,
        features=features,
        selection=selection,
        entry=entry,
        exit=exit_block,
        portfolio=portfolio,
        rebalance=rebalance,
        risk=risk,
        evaluation=evaluation,
        provenance=provenance,
    )

    return ParsedSpecification(
        specification=specification,
        refusals=tuple(refusals),
        migrations=applied,
    )


def _read_block[T](
    payload: Mapping[str, object],
    name: str,
    policy: DiscoveryPolicy,
    refusals: list[Refusal],
    reader: Callable[[_Reader], T],
) -> T:
    block = _Reader(name, payload.get(name), policy, refusals)
    parsed = reader(block)
    block.finish()

    return parsed
