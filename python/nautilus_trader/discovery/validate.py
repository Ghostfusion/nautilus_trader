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
The compiler's first four stages, and the structured result they produce.

Stage 1 is the schema: `parse_specification` decides whether the payload is well formed and inside
the bounds the pre-registered policy declares. Stage 2 resolves every declared alias against the
feature registry, so the definitions a candidate selects are named and their digests are known.
Stage 3 is semantic validation: the interactions between blocks that each have a plausible silent
precedence, refused rather than resolved by whichever code path happens to run first. Stage 4 is
temporal validation: every input must be knowable at the declared decision instant, and every fill
must be eligible strictly after it.

A stage that fails stops the pipeline, because each stage reads what the previous one produced: an
unresolved feature leaves the temporal check without a definition to measure against. Within a stage
every refusal is collected rather than stopping at the first, so a refused candidate is one
auditable outcome rather than a sequence of retries, and `ValidationResult` carries the errors, the
definitions that resolved and the specification that parsed.

Warnings are a separate channel from refusals. A warning says a specification is legal and depends
on something an operator should see, so warnings are reported only for a specification that passes
every stage; a refusal is never softened by a warning beside it.

Two interactions the language resolves by declaration rather than by refusal, recorded because they
were considered. The holding-period cap is counted in decision bars and an exit is evaluated at
every decision bar, so a cap shorter than the rebalance interval is not a contradiction: the
position leaves between rebalances, which is what the cap asks for. And the position cap is applied
before the sector cap, which is the binding order the allocation function also declares, so the two
caps interact by a declared order rather than by whichever happens to bind first.

The tables this module orders against are closed for the same reason the schema's are: an instant or
a frequency the table does not hold cannot be ordered, and the compiler refuses it rather than
guessing an ordering, because a guessed ordering is a silent look-ahead.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal
from enum import Enum
from typing import TYPE_CHECKING

from nautilus_trader.discovery.schema import ConditionLeaf
from nautilus_trader.discovery.schema import Refusal
from nautilus_trader.discovery.schema import RefusalCode
from nautilus_trader.discovery.schema import parse_specification


if TYPE_CHECKING:
    from collections.abc import Mapping

    from nautilus_trader.discovery.features import FeatureDefinition
    from nautilus_trader.discovery.features import FeatureRegistry
    from nautilus_trader.discovery.policy import DiscoveryPolicy
    from nautilus_trader.discovery.schema import ConditionNode
    from nautilus_trader.discovery.schema import DataBlock
    from nautilus_trader.discovery.schema import ExitBlock
    from nautilus_trader.discovery.schema import Migration
    from nautilus_trader.discovery.schema import PortfolioBlock
    from nautilus_trader.discovery.schema import RebalanceBlock
    from nautilus_trader.discovery.schema import RiskPostRunBlock
    from nautilus_trader.discovery.schema import SelectionBlock
    from nautilus_trader.discovery.schema import Specification


# One trading session expressed in nanoseconds, and the interval of each declared frequency in
# sessions. A signal interval and a rebalance interval are both read from this table, so the two
# cannot disagree about how long a week is.
SESSION_NS = 86_400_000_000_000
FREQUENCY_SESSIONS: dict[str, int] = {"daily": 1, "weekly": 5, "monthly": 21}

# The instants the compiler can order, from a decision at the open to the latest eligible fill.
# `session_close` and `same_session_close` share an order because deciding at the close and filling
# at it is the undeterred case the temporal stage refuses, not a later instant.
EXECUTION_INSTANT_ORDER: dict[str, int] = {
    "session_open": 0,
    "session_close": 1,
    "same_session_close": 1,
    "next_session_open": 2,
}

# The selection modes this stage knows how to check. A policy may declare another mode, and a block
# this stage cannot check is refused rather than skipped, as the tables above are closed.
RANK_MODE = "rank"
THRESHOLD_MODE = "threshold"
NONE_MODE = "none"

# The multiples and thresholds the warning rules compare against, named so a change to either is a
# visible change rather than a literal to be found in a condition.
WARMUP_DOMINANCE_MULTIPLE = 4
FULLY_INVESTED = Decimal(1)

# The paths the refusals and warnings of this module are reported at. A caller groups by path as
# well as by code, so every path a rule names is stated once here.
_SELECTION_PATH = "selection"
_EXIT_PATH = "exit"
_RANK_EXIT_PATH = "exit.rank_exit_count"
_PORTFOLIO_PATH = "portfolio"
_SECTOR_WEIGHT_PATH = "portfolio.max_sector_weight"
_TIMING_PATH = "rebalance.timing"
_FREQUENCY_PATH = "rebalance.frequency"
_ELIGIBILITY_PATH = "data.execution_eligibility"
_DECISION_TIME_PATH = "data.decision_time"
_POST_RUN_PATH = "risk.post_run"

# The subjects the field-level rules are phrased by, so a refusal says which block it is about.
_RANK_SUBJECT = "rank selection"
_THRESHOLD_SUBJECT = "threshold selection"
_NONE_SUBJECT = "selection that ranks nothing"


class WarningCode(Enum):
    """
    The closed vocabulary a warning is reported with.

    A warning is not a refusal: it names a legal specification that depends on something an operator
    should see before the run is read. A code is part of the auditable contract, so a warning is
    grouped and counted by code rather than by reading its message.
    """

    FEATURE_WARMUP_DOMINATES = "feature_warmup_dominates"
    UNUSED_DECLARED_FEATURE = "unused_declared_feature"
    POSITION_CAP_BELOW_FULLY_INVESTED = "position_cap_below_fully_invested"
    POST_RUN_LIMITS_ARE_GATE_CONDITIONS = "post_run_limits_are_gate_conditions"


@dataclass(frozen=True)
class ValidationWarning:
    """
    One reason a legal specification deserves an operator's attention.

    Parameters
    ----------
    code : WarningCode
        The warning's code, from the closed vocabulary.
    path : str
        The dotted location in the payload the warning is about.
    message : str
        What was noticed, in the vocabulary the coding standards require.

    """

    code: WarningCode
    path: str
    message: str


@dataclass(frozen=True)
class ValidationResult:
    """
    The outcome of the compiler's first four stages.

    The specification is carried whenever a payload parsed, including when a later stage refused it,
    so a caller can record what was submitted beside the reasons it was refused. The definitions are
    carried in declaration order and only when every declaration resolved, because a partial tuple
    would not line up with the aliases it is read by.

    Parameters
    ----------
    specification : Specification, optional
        The parsed specification, present whenever the payload parsed.
    errors : tuple[Refusal, ...]
        Every refusal the stages recorded, empty when none did.
    warnings : tuple[ValidationWarning, ...]
        The warnings of an accepted specification, empty otherwise.
    definitions : tuple[FeatureDefinition, ...]
        The resolved definitions, in the order the specification declared them.
    migrations : tuple[str, ...]
        The schema versions the payload passed through before it was parsed.

    """

    specification: Specification | None
    errors: tuple[Refusal, ...]
    warnings: tuple[ValidationWarning, ...]
    definitions: tuple[FeatureDefinition, ...]
    migrations: tuple[str, ...] = ()

    @property
    def accepted(self) -> bool:
        """
        Whether the specification parsed and no stage refused it.
        """
        return self.specification is not None and not self.errors


def validate_specification(
    payload: object,
    policy: DiscoveryPolicy,
    registry: FeatureRegistry,
    *,
    migrations: Mapping[str, Migration] | None = None,
) -> ValidationResult:
    """
    Run the compiler's first four stages over a payload and return the structured result.

    The stages run in order, each reading what the previous one produced, and the first stage that
    refuses stops the pipeline. Every refusal a stage records is returned, so a candidate refused
    for four reasons is one outcome carrying four reasons rather than four attempts.

    Parameters
    ----------
    payload : object
        The specification as data, rather than a parsed view of it.
    policy : DiscoveryPolicy
        The pre-registered policy the payload is checked against.
    registry : FeatureRegistry
        The registered definitions the declared features are resolved against.
    migrations : Mapping[str, Migration], optional
        The migration table to use, defaulting to the schema module's.

    Returns
    -------
    ValidationResult

    """
    parsed = parse_specification(payload, policy, migrations=migrations)
    specification = parsed.specification

    if specification is None:
        return ValidationResult(
            specification=None,
            errors=parsed.refusals,
            warnings=(),
            definitions=(),
            migrations=parsed.migrations,
        )

    definitions, unresolved = _resolve_features(specification, registry)
    if unresolved:
        return ValidationResult(
            specification=specification,
            errors=unresolved,
            warnings=(),
            definitions=(),
            migrations=parsed.migrations,
        )

    semantic = _validate_semantics(specification, policy)
    if semantic:
        return ValidationResult(
            specification=specification,
            errors=semantic,
            warnings=(),
            definitions=definitions,
            migrations=parsed.migrations,
        )

    temporal = _validate_temporal(specification, definitions)
    if temporal:
        return ValidationResult(
            specification=specification,
            errors=temporal,
            warnings=(),
            definitions=definitions,
            migrations=parsed.migrations,
        )

    return ValidationResult(
        specification=specification,
        errors=(),
        warnings=_collect_warnings(specification),
        definitions=definitions,
        migrations=parsed.migrations,
    )


def _resolve_features(
    specification: Specification,
    registry: FeatureRegistry,
) -> tuple[tuple[FeatureDefinition, ...], tuple[Refusal, ...]]:
    """
    Resolve every declared alias against the registry.

    Parameters
    ----------
    specification : Specification
        The parsed specification.
    registry : FeatureRegistry
        The registered definitions the identifiers are resolved against.

    Returns
    -------
    tuple[tuple[FeatureDefinition, ...], tuple[Refusal, ...]]
        The resolved definitions in declaration order, and a refusal per alias that did not resolve.

    """
    definitions: list[FeatureDefinition] = []
    refusals: list[Refusal] = []

    for declaration in specification.features.declarations:
        definition = registry.resolve(declaration.feature_id)
        if definition is None:
            refusals.append(
                Refusal(
                    RefusalCode.FEATURE_UNRESOLVED,
                    _feature_path(declaration.alias),
                    f"the registry holds no definition for {declaration.feature_id}, so the alias "
                    f"{declaration.alias} selects a feature the run cannot serve",
                ),
            )
            continue
        definitions.append(definition)

    return tuple(definitions), tuple(refusals)


def _validate_semantics(
    specification: Specification,
    policy: DiscoveryPolicy,
) -> tuple[Refusal, ...]:
    """
    Refuse every interaction between blocks that the language resolves by declaration.

    Parameters
    ----------
    specification : Specification
        The parsed specification.
    policy : DiscoveryPolicy
        The pre-registered policy whose declarations the blocks are checked against.

    Returns
    -------
    tuple[Refusal, ...]

    """
    refusals: list[Refusal] = []
    refusals.extend(_selection_refusals(specification.selection))
    refusals.extend(_exit_refusals(specification.exit, specification.selection.mode))
    refusals.extend(_portfolio_refusals(specification.portfolio, policy))
    refusals.extend(_rebalance_refusals(specification.data, specification.rebalance))

    return tuple(refusals)


def _selection_refusals(selection: SelectionBlock) -> list[Refusal]:
    """
    Refuse a selection block whose mode and fields do not agree.

    Parameters
    ----------
    selection : SelectionBlock
        The parsed selection block.

    Returns
    -------
    list[Refusal]

    """
    if selection.mode == RANK_MODE:
        return _rank_selection_refusals(selection)
    if selection.mode == THRESHOLD_MODE:
        return _threshold_selection_refusals(selection)
    if selection.mode == NONE_MODE:
        return _none_selection_refusals(selection)

    return [
        Refusal(
            RefusalCode.SELECTION_INCOMPLETE,
            _SELECTION_PATH,
            f"the compiler checks the selection modes {RANK_MODE}, {THRESHOLD_MODE} and "
            f"{NONE_MODE}, and cannot check {selection.mode}",
        ),
    ]


def _rank_selection_refusals(selection: SelectionBlock) -> list[Refusal]:
    """
    Refuse a rank selection that does not state exactly one bound and its ranking.

    Parameters
    ----------
    selection : SelectionBlock
        The parsed selection block, which declares the rank mode.

    Returns
    -------
    list[Refusal]

    """
    refusals: list[Refusal] = []

    if (selection.percentile is None) == (selection.count is None):
        refusals.append(
            Refusal(
                RefusalCode.SELECTION_INCOMPLETE,
                _SELECTION_PATH,
                "a rank selection states exactly one of percentile and count",
            ),
        )

    refusals.extend(
        _required_refusals(
            RefusalCode.SELECTION_INCOMPLETE,
            _SELECTION_PATH,
            _RANK_SUBJECT,
            {"feature": selection.feature, "tie_break": selection.tie_break},
        ),
    )

    return refusals


def _threshold_selection_refusals(selection: SelectionBlock) -> list[Refusal]:
    """
    Refuse a threshold selection that does not state its comparison and no ranking.

    Parameters
    ----------
    selection : SelectionBlock
        The parsed selection block, which declares the threshold mode.

    Returns
    -------
    list[Refusal]

    """
    refusals = _required_refusals(
        RefusalCode.SELECTION_INCOMPLETE,
        _SELECTION_PATH,
        _THRESHOLD_SUBJECT,
        {
            "feature": selection.feature,
            "comparison": selection.comparison,
            "threshold": selection.threshold,
        },
    )
    refusals.extend(
        _forbidden_refusals(
            RefusalCode.SELECTION_INCOMPLETE,
            _SELECTION_PATH,
            _THRESHOLD_SUBJECT,
            {
                "percentile": selection.percentile,
                "count": selection.count,
                "tie_break": selection.tie_break,
            },
        ),
    )

    return refusals


def _none_selection_refusals(selection: SelectionBlock) -> list[Refusal]:
    """
    Refuse a selection that ranks nothing and states a ranking field anyway.

    Parameters
    ----------
    selection : SelectionBlock
        The parsed selection block, which declares the none mode.

    Returns
    -------
    list[Refusal]

    """
    return _forbidden_refusals(
        RefusalCode.SELECTION_INCOMPLETE,
        _SELECTION_PATH,
        _NONE_SUBJECT,
        {
            "feature": selection.feature,
            "percentile": selection.percentile,
            "count": selection.count,
            "threshold": selection.threshold,
            "comparison": selection.comparison,
            "tie_break": selection.tie_break,
        },
    )


def _exit_refusals(exit_block: ExitBlock, selection_mode: str) -> list[Refusal]:
    """
    Refuse an exit whose threshold fields disagree with each other or with the selection.

    Parameters
    ----------
    exit_block : ExitBlock
        The parsed exit block.
    selection_mode : str
        The mode the selection block declared.

    Returns
    -------
    list[Refusal]

    """
    refusals: list[Refusal] = []
    complete = exit_block.comparison is not None and exit_block.threshold is not None

    if (exit_block.feature is not None) != complete:
        refusals.append(
            Refusal(
                RefusalCode.EXIT_INCOMPLETE,
                _EXIT_PATH,
                "an exit states a feature with both comparison and threshold, or states no feature",
            ),
        )

    if exit_block.rank_exit_count is not None and selection_mode != RANK_MODE:
        refusals.append(
            Refusal(
                RefusalCode.EXIT_INCOMPLETE,
                _RANK_EXIT_PATH,
                f"rank_exit_count is resolved against a ranking, and the selection mode is "
                f"{selection_mode}",
            ),
        )

    return refusals


def _portfolio_refusals(portfolio: PortfolioBlock, policy: DiscoveryPolicy) -> list[Refusal]:
    """
    Refuse a sector cap the policy declares no sector for.

    Parameters
    ----------
    portfolio : PortfolioBlock
        The parsed portfolio block.
    policy : DiscoveryPolicy
        The pre-registered policy whose sector map the cap is applied through.

    Returns
    -------
    list[Refusal]

    """
    if portfolio.max_sector_weight is not None and not policy.has_sectors:
        return [
            Refusal(
                RefusalCode.SECTOR_INPUT_UNDECLARED,
                _SECTOR_WEIGHT_PATH,
                "max_sector_weight is applied through the instrument-to-sector mapping, and the "
                "policy declares no sector for any instrument",
            ),
        ]

    return []


def _rebalance_refusals(data: DataBlock, rebalance: RebalanceBlock) -> list[Refusal]:
    """
    Refuse a rebalance whose timing or frequency disagrees with the declared signal.

    Parameters
    ----------
    data : DataBlock
        The parsed data block, which declares when the candidate decides and fills.
    rebalance : RebalanceBlock
        The parsed rebalance block.

    Returns
    -------
    list[Refusal]

    """
    refusals: list[Refusal] = []

    if rebalance.timing != data.execution_eligibility:
        refusals.append(
            Refusal(
                RefusalCode.REBALANCE_TIMING_MISMATCH,
                _TIMING_PATH,
                f"the rebalance timing {rebalance.timing} must equal the execution eligibility "
                f"{data.execution_eligibility}, so a rebalance is eligible when the data "
                "block says its intent is",
            ),
        )

    signal_sessions = FREQUENCY_SESSIONS.get(data.signal_frequency)
    rebalance_sessions = FREQUENCY_SESSIONS.get(rebalance.frequency)

    if signal_sessions is None or rebalance_sessions is None:
        refusals.append(
            Refusal(
                RefusalCode.REBALANCE_FREQUENCY_MISMATCH,
                _FREQUENCY_PATH,
                f"the compiler orders the frequencies {', '.join(FREQUENCY_SESSIONS)}, and cannot "
                f"order {rebalance.frequency} against {data.signal_frequency}",
            ),
        )
    elif rebalance_sessions < signal_sessions:
        refusals.append(
            Refusal(
                RefusalCode.REBALANCE_FREQUENCY_MISMATCH,
                _FREQUENCY_PATH,
                f"a rebalance at {rebalance.frequency} is finer than the signal at "
                f"{data.signal_frequency}, so the candidate would rebalance between decisions",
            ),
        )

    return refusals


def _validate_temporal(
    specification: Specification,
    definitions: tuple[FeatureDefinition, ...],
) -> tuple[Refusal, ...]:
    """
    Refuse a specification whose inputs cannot be known or whose fill is not deferred.

    Parameters
    ----------
    specification : Specification
        The parsed specification, whose every declaration resolved.
    definitions : tuple[FeatureDefinition, ...]
        The resolved definitions, in declaration order.

    Returns
    -------
    tuple[Refusal, ...]

    """
    refusals: list[Refusal] = []
    undeferred = _undeferred_execution_refusal(specification.data)

    if undeferred is not None:
        refusals.append(undeferred)

    refusals.extend(_unavailable_feature_refusals(specification, definitions))

    return tuple(refusals)


def _undeferred_execution_refusal(data: DataBlock) -> Refusal | None:
    """
    Return the refusal of an execution that is not strictly later than the decision, if any.

    Parameters
    ----------
    data : DataBlock
        The parsed data block, which declares both instants.

    Returns
    -------
    Refusal or None

    """
    decision = EXECUTION_INSTANT_ORDER.get(data.decision_time)
    eligibility = EXECUTION_INSTANT_ORDER.get(data.execution_eligibility)
    ordered = ", ".join(EXECUTION_INSTANT_ORDER)

    if decision is None:
        return Refusal(
            RefusalCode.UNDEFERRED_EXECUTION,
            _DECISION_TIME_PATH,
            f"the compiler orders the instants {ordered}, which do not hold {data.decision_time}",
        )

    if eligibility is None:
        return Refusal(
            RefusalCode.UNDEFERRED_EXECUTION,
            _ELIGIBILITY_PATH,
            f"the compiler orders the instants {ordered}, which do not hold "
            f"{data.execution_eligibility}",
        )

    if eligibility <= decision:
        return Refusal(
            RefusalCode.UNDEFERRED_EXECUTION,
            _ELIGIBILITY_PATH,
            f"the execution eligibility {data.execution_eligibility} must be strictly later than "
            f"the decision time {data.decision_time}, or the candidate decides at one instant and "
            "fills at it",
        )

    return None


def _unavailable_feature_refusals(
    specification: Specification,
    definitions: tuple[FeatureDefinition, ...],
) -> list[Refusal]:
    """
    Refuse every declared feature that cannot be known at the decision instant.

    The signal frequency is read from the same table the rebalance ordering uses, and stage 3
    refuses a frequency that table does not hold, so every frequency reaching this point is ordered.

    Parameters
    ----------
    specification : Specification
        The parsed specification.
    definitions : tuple[FeatureDefinition, ...]
        The resolved definitions, in declaration order.

    Returns
    -------
    list[Refusal]

    """
    frequency = specification.data.signal_frequency
    interval_ns = FREQUENCY_SESSIONS[frequency] * SESSION_NS
    refusals: list[Refusal] = []

    for declaration, definition in zip(
        specification.features.declarations,
        definitions,
        strict=True,
    ):
        if definition.availability_lag_ns < interval_ns:
            continue
        refusals.append(
            Refusal(
                RefusalCode.FEATURE_UNAVAILABLE_AT_DECISION,
                _feature_path(declaration.alias),
                f"the definition of {declaration.feature_id} is available "
                f"{definition.availability_lag_ns} nanoseconds after it is observed, at or beyond "
                f"the {frequency} signal interval of {interval_ns} nanoseconds, so the value is "
                "not knowable at the decision instant",
            ),
        )

    return refusals


def _collect_warnings(specification: Specification) -> tuple[ValidationWarning, ...]:
    """
    Return the warnings of an accepted specification.

    Parameters
    ----------
    specification : Specification
        The parsed specification, which no stage refused.

    Returns
    -------
    tuple[ValidationWarning, ...]

    """
    warnings: list[ValidationWarning] = []
    warnings.extend(_warmup_warnings(specification))
    warnings.extend(_unused_feature_warnings(specification))
    warnings.extend(_position_cap_warnings(specification.portfolio))
    warnings.extend(_post_run_warnings(specification.risk.post_run))

    return tuple(warnings)


def _warmup_warnings(specification: Specification) -> list[ValidationWarning]:
    """
    Warn about a declared lookback that dominates the rebalance interval.

    The declaration's lookback is the window the candidate asked for, and the rebalance interval is
    read from the same table stage 3 orders frequencies with, so a frequency reaching this point is
    one the table holds.

    Parameters
    ----------
    specification : Specification
        The parsed specification.

    Returns
    -------
    list[ValidationWarning]

    """
    sessions = FREQUENCY_SESSIONS[specification.rebalance.frequency]
    ceiling = sessions * WARMUP_DOMINANCE_MULTIPLE

    return [
        ValidationWarning(
            WarningCode.FEATURE_WARMUP_DOMINATES,
            _lookback_path(declaration.alias),
            f"a lookback of {declaration.lookback} exceeds {WARMUP_DOMINANCE_MULTIPLE} "
            f"{specification.rebalance.frequency} intervals, which is {ceiling} sessions, so the "
            "feature's warm-up consumes most of the window it is read in",
        )
        for declaration in specification.features.declarations
        if declaration.lookback > ceiling
    ]


def _unused_feature_warnings(specification: Specification) -> list[ValidationWarning]:
    """
    Warn about a declared alias nothing reads.

    Parameters
    ----------
    specification : Specification
        The parsed specification.

    Returns
    -------
    list[ValidationWarning]

    """
    referenced = _referenced_aliases(specification)

    return [
        ValidationWarning(
            WarningCode.UNUSED_DECLARED_FEATURE,
            _declaration_path(declaration.alias),
            f"the alias {declaration.alias} is referenced by neither the selection, the entry tree "
            "nor the exit block, so its value is never read",
        )
        for declaration in specification.features.declarations
        if declaration.alias not in referenced
    ]


def _referenced_aliases(specification: Specification) -> set[str]:
    """
    Return every alias the selection, the entry tree and the exit block reference.

    Parameters
    ----------
    specification : Specification
        The parsed specification.

    Returns
    -------
    set[str]

    """
    referenced: set[str] = set()

    if specification.selection.feature is not None:
        referenced.add(specification.selection.feature)
    if specification.exit.feature is not None:
        referenced.add(specification.exit.feature)

    _collect_condition_features(specification.entry.conditions, referenced)

    return referenced


def _collect_condition_features(node: ConditionNode | None, referenced: set[str]) -> None:
    """
    Add the feature of every condition leaf in a tree to the referenced aliases.

    Parameters
    ----------
    node : ConditionNode, optional
        The condition tree, or nothing when the entry states no conditions.
    referenced : set[str]
        The aliases collected so far, updated in place.

    """
    if node is None:
        return
    if isinstance(node, ConditionLeaf):
        referenced.add(node.feature)
        return

    for child in node.conditions:
        _collect_condition_features(child, referenced)


def _position_cap_warnings(portfolio: PortfolioBlock) -> list[ValidationWarning]:
    """
    Warn about a position cap that cannot invest the whole portfolio.

    Parameters
    ----------
    portfolio : PortfolioBlock
        The parsed portfolio block.

    Returns
    -------
    list[ValidationWarning]

    """
    allocated = portfolio.position_count * portfolio.max_position_weight
    if allocated >= FULLY_INVESTED:
        return []

    return [
        ValidationWarning(
            WarningCode.POSITION_CAP_BELOW_FULLY_INVESTED,
            _PORTFOLIO_PATH,
            f"{portfolio.position_count} positions at max_position_weight "
            f"{portfolio.max_position_weight} allocate {allocated}, below fully invested, so the "
            "remainder is decided by the cash policy rather than by the selection",
        ),
    ]


def _post_run_warnings(post_run: RiskPostRunBlock) -> list[ValidationWarning]:
    """
    Warn about every post-run limit, which the run it is recorded beside cannot obey.

    Parameters
    ----------
    post_run : RiskPostRunBlock
        The parsed post-run block.

    Returns
    -------
    list[ValidationWarning]

    """
    limits = {
        "max_drawdown": post_run.max_drawdown,
        "max_turnover": post_run.max_turnover,
        "max_cost_ratio": post_run.max_cost_ratio,
    }

    return [
        ValidationWarning(
            WarningCode.POST_RUN_LIMITS_ARE_GATE_CONDITIONS,
            f"{_POST_RUN_PATH}.{name}",
            f"{name} is measured once the observations exist, so it cannot control the run it is "
            "recorded beside and is a candidate acceptance criterion rather than a runtime promise",
        )
        for name, value in limits.items()
        if value is not None
    ]


def _required_refusals(
    code: RefusalCode,
    path: str,
    subject: str,
    fields: Mapping[str, object | None],
) -> list[Refusal]:
    """
    Return a refusal for every field the subject requires and does not state.

    Parameters
    ----------
    code : RefusalCode
        The code every refusal is reported with.
    path : str
        The dotted location every refusal is reported at.
    subject : str
        The block the refusal is phrased about.
    fields : Mapping[str, object or None]
        The field names and the values the block stated.

    Returns
    -------
    list[Refusal]

    """
    return [
        Refusal(code, path, f"a {subject} requires {name}")
        for name, value in fields.items()
        if value is None
    ]


def _forbidden_refusals(
    code: RefusalCode,
    path: str,
    subject: str,
    fields: Mapping[str, object | None],
) -> list[Refusal]:
    """
    Return a refusal for every field the subject states and must not.

    Parameters
    ----------
    code : RefusalCode
        The code every refusal is reported with.
    path : str
        The dotted location every refusal is reported at.
    subject : str
        The block the refusal is phrased about.
    fields : Mapping[str, object or None]
        The field names and the values the block stated.

    Returns
    -------
    list[Refusal]

    """
    return [
        Refusal(code, path, f"a {subject} states no {name}")
        for name, value in fields.items()
        if value is not None
    ]


def _declaration_path(alias: str) -> str:
    """
    Return the path of a feature declaration.

    Parameters
    ----------
    alias : str
        The alias the specification declared.

    Returns
    -------
    str

    """
    return f"features.declared.{alias}"


def _feature_path(alias: str) -> str:
    """
    Return the path of the registry reference a declaration states.

    Parameters
    ----------
    alias : str
        The alias the specification declared.

    Returns
    -------
    str

    """
    return f"{_declaration_path(alias)}.feature"


def _lookback_path(alias: str) -> str:
    """
    Return the path of the lookback a declaration states.

    Parameters
    ----------
    alias : str
        The alias the specification declared.

    Returns
    -------
    str

    """
    return f"{_declaration_path(alias)}.lookback"
