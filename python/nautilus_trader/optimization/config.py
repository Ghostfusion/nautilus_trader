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
The optimization configuration file: one declared sweep, loadable from JSON.

This module is the single entry point shared by the `nautilus optimize` command and notebook
workflows, so both drive exactly the same optimizer. It owns no optimization logic: it parses a
strict JSON document into the existing `ParameterSpace`, `Objective`, `Constraint`, `Optimizer`,
`ConcurrencyPolicy`, `ExperimentStore`, and stage types, and runs them through the one runner.

The config is JSON rather than YAML because it follows the convention of the configuration file
loader in `crates/system/src/config_file.rs`: typed constructors remain the canonical API and a
file is a view of a typed configuration, with unknown keys rejected rather than ignored.

The document records the whole sweep:

- `strategy`: the importable strategy and config paths, and the importable configuration factory
  that returns the venue, data, and engine configurations for a run window.
- `space`: the base strategy values and the named parameters with their ordered choices.
- `window`: the run window in Unix nanoseconds (or null for the data's own bounds).
- `objective` and `constraints`: the `nautilus_trader.analysis` terms and bounds.
- `stage`: the methodology stage to run.
- `concurrency`: the memory-driven worker policy.
- `store`: an optional persistence directory.
"""

from __future__ import annotations

import json
import math
import os
import sys
from dataclasses import dataclass
from dataclasses import field
from importlib import import_module
from pathlib import Path
from typing import TYPE_CHECKING

from nautilus_trader.analysis import Constraint
from nautilus_trader.analysis import ConstraintComparison
from nautilus_trader.analysis import Objective
from nautilus_trader.analysis import ObjectiveDirection
from nautilus_trader.analysis import ObjectiveTerm
from nautilus_trader.common import LogLevel
from nautilus_trader.common import init_logging
from nautilus_trader.core import UUID4
from nautilus_trader.model import TraderId
from nautilus_trader.optimization.assumptions import BarAmbiguityPolicy
from nautilus_trader.optimization.concurrency import ConcurrencyPolicy
from nautilus_trader.optimization.optimizer import Optimizer
from nautilus_trader.optimization.persistence import ExperimentStore
from nautilus_trader.optimization.report import ExperimentResult
from nautilus_trader.optimization.report import SearchReport
from nautilus_trader.optimization.runner import BacktestRunner
from nautilus_trader.optimization.runner import FailedExperiment
from nautilus_trader.optimization.search import GridSearch
from nautilus_trader.optimization.space import Experiment
from nautilus_trader.optimization.space import Parameter
from nautilus_trader.optimization.space import ParameterSpace
from nautilus_trader.optimization.splits import LabelOverlapRule
from nautilus_trader.optimization.splits import LeakagePolicy
from nautilus_trader.optimization.stages import OutOfSampleStage
from nautilus_trader.optimization.stages import TrainStage
from nautilus_trader.optimization.stages import ValidateStage
from nautilus_trader.optimization.stages import ValidationResult
from nautilus_trader.optimization.stages import WalkForwardReport
from nautilus_trader.optimization.stages import WalkForwardStage
from nautilus_trader.optimization.stages import walk_forward_windows


if TYPE_CHECKING:
    from collections.abc import Callable
    from collections.abc import Mapping

    from nautilus_trader.optimization.space import JsonValue


# The schema identifier of the configuration document.
CONFIG_SCHEMA = "nautilus.optimization.config/v1"

# The schema identifier of the emitted machine-readable result document.
OUTPUT_SCHEMA = "nautilus.optimization.cli/v1"

# The supported methodology stage kinds.
STAGE_KINDS = ("optimize", "train", "validate", "out_of_sample", "walk_forward")

# The keys of the stage leakage policy, and the label rules it accepts.
_LEAKAGE_KEYS = frozenset(
    {
        "purge_before_ns",
        "purge_after_ns",
        "embargo_after_ns",
        "label_overlap_rule",
        "label_horizon_ns",
        "zero_interval_justification",
    },
)
_LABEL_RULES = {rule.value: rule for rule in LabelOverlapRule}

_DIRECTIONS: dict[str, ObjectiveDirection] = {
    "maximize": ObjectiveDirection.MAXIMIZE,
    "minimize": ObjectiveDirection.MINIMIZE,
}

_COMPARISONS: dict[str, ConstraintComparison] = {
    "at_least": ConstraintComparison.AT_LEAST,
    "at_most": ConstraintComparison.AT_MOST,
}

_TOP_LEVEL_KEYS = frozenset(
    {
        "schema",
        "strategy",
        "space",
        "window",
        "objective",
        "constraints",
        "stage",
        "concurrency",
        "store",
        "assumptions",
    },
)

# The keys of the declared execution assumptions, spelled as the venue configuration spells them.
_ASSUMPTION_KEYS = frozenset({"bar_execution", "adaptive_high_low_ordering"})


class ConfigError(ValueError):
    """
    A configuration file is invalid: a missing key, a wrong type, or an unknown key.
    """


@dataclass(frozen=True)
class StrategySpec:
    """
    The strategy under test and the factory that builds a run's configurations.

    Parameters
    ----------
    strategy_path : str
        The importable strategy path, as `module:Class`.
    config_path : str
        The importable strategy config path, as `module:Class`.
    config_factory : str
        The importable configuration factory path, as `module:function`. The function takes the
        run window bounds and returns the venue configurations, data configurations, and engine
        configuration for one run.

    """

    strategy_path: str
    config_path: str
    config_factory: str


@dataclass(frozen=True)
class StageSpec:
    """
    The methodology stage to run and its stage-specific inputs.

    Parameters
    ----------
    kind : str
        The stage kind: `optimize`, `train`, `validate`, `out_of_sample`, or `walk_forward`.
    parameters : Mapping[str, JsonValue] | None
        The parameter set for `validate` and `out_of_sample`, or None.
    in_sample_ns : int | None
        The walk-forward in-sample length in nanoseconds, or None.
    out_of_sample_ns : int | None
        The walk-forward out-of-sample length in nanoseconds, or None.
    leakage : LeakagePolicy | None
        The walk-forward leakage policy, or None for the declared default of the stage.

    """

    kind: str
    parameters: Mapping[str, JsonValue] | None = None
    in_sample_ns: int | None = None
    out_of_sample_ns: int | None = None
    leakage: LeakagePolicy | None = None


@dataclass(frozen=True)
class OptimizationConfig:
    """
    A complete declared optimization: a strategy, a space, a window, an objective, and a stage.

    Parameters
    ----------
    strategy : StrategySpec
        The strategy under test and its configuration factory.
    space : ParameterSpace
        The parameter space to sweep.
    objective : Objective
        The objective over the runs' statistics.
    constraints : tuple[Constraint, ...]
        The constraints over the same values.
    stage : StageSpec
        The methodology stage to run.
    start : int | None
        The run window start in Unix nanoseconds, or None.
    end : int | None
        The run window end in Unix nanoseconds, or None.
    concurrency : ConcurrencyPolicy
        The memory-driven concurrency policy.
    store_directory : str | None
        The persistence directory, or None when results are not persisted.
    assumptions : BarAmbiguityPolicy
        The declared bar-derived execution assumptions, defaulting to the declared default.

    """

    strategy: StrategySpec
    space: ParameterSpace
    objective: Objective
    constraints: tuple[Constraint, ...] = ()
    stage: StageSpec = field(default_factory=lambda: StageSpec("optimize"))
    start: int | None = None
    end: int | None = None
    concurrency: ConcurrencyPolicy = field(default_factory=ConcurrencyPolicy)
    store_directory: str | None = None
    assumptions: BarAmbiguityPolicy = field(default_factory=BarAmbiguityPolicy.declared_default)


def load_config(path: str | Path) -> OptimizationConfig:
    """
    Load a configuration document from the JSON file at `path`.

    The document must be a strict JSON object with no non-finite numbers, and every mapping is
    validated against its allowed keys so an unknown key is rejected rather than ignored.

    Parameters
    ----------
    path : str | Path
        The path to the configuration file.

    Returns
    -------
    OptimizationConfig

    """
    text = Path(path).read_text(encoding="utf-8")
    try:
        payload: object = json.loads(text, parse_constant=_reject_constant)
    except json.JSONDecodeError as exc:
        raise ConfigError(f"Configuration is not valid JSON: {exc}") from exc
    return parse_config(payload)


def parse_config(payload: object) -> OptimizationConfig:
    """
    Parse an already-decoded configuration document.

    Parameters
    ----------
    payload : object
        The decoded JSON document.

    Returns
    -------
    OptimizationConfig

    """
    document = _require_mapping(payload, "config")
    _reject_unknown(document, _TOP_LEVEL_KEYS, "config")

    schema = document.get("schema")
    if schema is not None and schema != CONFIG_SCHEMA:
        raise ConfigError(f"config.schema must be '{CONFIG_SCHEMA}', was {schema!r}")

    start, end = _parse_window(document.get("window"))
    return OptimizationConfig(
        strategy=_parse_strategy(_require_key(document, "strategy", "config")),
        space=_parse_space(_require_key(document, "space", "config")),
        objective=_parse_objective(_require_key(document, "objective", "config")),
        constraints=_parse_constraints(document.get("constraints", [])),
        stage=_parse_stage(_require_key(document, "stage", "config")),
        start=start,
        end=end,
        concurrency=_parse_concurrency(document.get("concurrency")),
        store_directory=_parse_store(document.get("store")),
        assumptions=_parse_assumptions(document.get("assumptions")),
    )


def _parse_assumptions(payload: object) -> BarAmbiguityPolicy:
    """
    Parse the declared execution assumptions.

    An absent declaration is the declared default, which is named rather than inferred from a venue
    setting, and an ambiguous declaration is a configuration error.
    """
    if payload is None:
        return BarAmbiguityPolicy.declared_default()

    mapping = _require_mapping(payload, "config.assumptions")
    _reject_unknown(mapping, _ASSUMPTION_KEYS, "config.assumptions")

    bar_execution = _optional_bool(mapping.get("bar_execution"), "config.assumptions.bar_execution")
    adaptive = _optional_bool(
        mapping.get("adaptive_high_low_ordering"),
        "config.assumptions.adaptive_high_low_ordering",
    )

    try:
        return BarAmbiguityPolicy.from_venue_flags(
            bar_execution=True if bar_execution is None else bar_execution,
            adaptive_high_low_ordering=(False if adaptive is None else adaptive),
        )
    except ValueError as exc:
        raise ConfigError(f"config.assumptions: {exc}") from exc


def run_config(config: OptimizationConfig) -> dict[str, object]:
    """
    Run the declared optimization and return its machine-readable result document.

    The objects the package exports are built here and run through the one runner, so the CLI and
    a notebook call exactly the same code path. The objective, the space, and the stage are the
    existing types; this function only wires them together and encodes the outcome.

    Parameters
    ----------
    config : OptimizationConfig
        The declared optimization.

    Returns
    -------
    dict[str, object]
        The result document.

    """
    factory = _resolve_importable(config.strategy.config_factory)
    runner = BacktestRunner(
        config_factory=factory,
        strategy_path=config.strategy.strategy_path,
        config_path=config.strategy.config_path,
        start=config.start,
        end=config.end,
    )
    optimizer = Optimizer(
        runner=runner,
        objective=config.objective,
        constraints=config.constraints,
        concurrency=config.concurrency,
        search=GridSearch(),
    )
    store = ExperimentStore(config.store_directory) if config.store_directory else None

    document = _run_stage(config, optimizer, store)
    _attach_context(document, config)
    return document


def main(argv: list[str] | None = None) -> int:
    """
    Run a configuration file and print one machine-readable JSON document.

    Failures are reported both in the document and through the return code, matching the catalog
    data subcommands, so a caller can distinguish a failure from an empty result.

    Parameters
    ----------
    argv : list[str] | None, default None
        The command-line arguments, or None to use `sys.argv[1:]`.

    Returns
    -------
    int
        The process exit code.

    """
    arguments = list(sys.argv[1:]) if argv is None else list(argv)
    if len(arguments) != 1:
        usage = "Usage: python -m nautilus_trader.optimization.config CONFIG.json"
        _emit(_error_document(ConfigError(usage)))
        return 2
    # Bypass console logging so the JSON document is the only standard output. The guard stays
    # alive for the duration of the run.
    _logging_guard = _silence_console_logging()
    try:
        config = load_config(arguments[0])
        document = run_config(config)
    except Exception as exc:  # noqa: BLE001 (a config or run failure is reported, never raised)
        _emit(_error_document(exc))
        return 1
    _emit(document)
    return 0


def _silence_console_logging() -> object:
    """
    Bypass console logging unless `NAUTILUS_LOG` configures it explicitly.

    The emitted document is machine-readable, so console logging is off by default and failures
    stay visible through the document and the exit code, matching the catalog data subcommands.
    """
    if os.environ.get("NAUTILUS_LOG"):
        return None
    return init_logging(
        trader_id=TraderId("OPTIMIZE-000"),
        instance_id=UUID4(),
        level_stdout=LogLevel.INFO,
        is_bypassed=True,
        print_config=False,
    )


def _run_stage(
    config: OptimizationConfig,
    optimizer: Optimizer,
    store: ExperimentStore | None,
) -> dict[str, object]:
    """
    Dispatch to the declared methodology stage and encode its outcome.
    """
    kind = config.stage.kind
    if kind == "optimize":
        return _report_document(kind, optimizer.optimize(config.space, store=store))
    if kind == "train":
        return _train_document(optimizer, config, store)
    if kind == "validate":
        return _validation_document(kind, ValidateStage(optimizer).run(_stage_experiment(config)))
    if kind == "out_of_sample":
        return _out_of_sample_document(kind, optimizer, config, store)
    if kind == "walk_forward":
        return _walk_forward_document(config, optimizer)
    raise ConfigError(f"Unsupported stage kind {kind!r}")


def _train_document(
    optimizer: Optimizer,
    config: OptimizationConfig,
    store: ExperimentStore | None,
) -> dict[str, object]:
    """
    Encode a train-stage selection.
    """
    selected = TrainStage(optimizer).run(config.space)
    report = SearchReport((selected,) if selected is not None else (), ())
    if store is not None and selected is not None:
        store.write_report(report)
    return _report_document("train", report)


def _out_of_sample_document(
    kind: str,
    optimizer: Optimizer,
    config: OptimizationConfig,
    store: ExperimentStore | None,
) -> dict[str, object]:
    """
    Encode an out-of-sample evaluation of one experiment.
    """
    outcome = OutOfSampleStage(optimizer).run(_stage_experiment(config))
    results = (outcome,) if isinstance(outcome, ExperimentResult) else ()
    failures = () if isinstance(outcome, ExperimentResult) else (outcome,)
    report = SearchReport(results, failures)
    if store is not None:
        store.write_report(report)
    return _report_document(kind, report)


def _report_document(kind: str, report: SearchReport) -> dict[str, object]:
    """
    Encode a search report as the result document.
    """
    return {
        "schema": OUTPUT_SCHEMA,
        "command": "optimize",
        "stage": kind,
        "status": "ok",
        "results": [_result_record(result) for result in report.results],
        "failures": [_failure_record(failure) for failure in report.failures],
        "best": _record_or_none(report.best()),
        "best_feasible": _record_or_none(report.best_feasible()),
    }


def _validation_document(
    kind: str,
    outcome: ValidationResult | FailedExperiment,
) -> dict[str, object]:
    """
    Encode a validation outcome as the result document.
    """
    if isinstance(outcome, FailedExperiment):
        return {
            "schema": OUTPUT_SCHEMA,
            "command": "optimize",
            "stage": kind,
            "status": "ok",
            "results": [],
            "failures": [_failure_record(outcome)],
            "best": None,
            "best_feasible": None,
            "validation": None,
        }
    return {
        "schema": OUTPUT_SCHEMA,
        "command": "optimize",
        "stage": kind,
        "status": "ok",
        "results": [],
        "failures": [],
        "best": None,
        "best_feasible": None,
        "validation": {
            "experiment_digest": outcome.experiment.digest,
            "parameters": dict(outcome.experiment.parameters),
            "score": _encode_number(outcome.score),
            "constraints_satisfied": outcome.constraints_satisfied,
            "violations": list(outcome.violations),
            "metric_values": {
                name: _encode_number(value) for name, value in outcome.metric_values.items()
            },
        },
    }


def _walk_forward_document(
    config: OptimizationConfig,
    optimizer: Optimizer,
) -> dict[str, object]:
    """
    Encode a walk-forward run as the result document.
    """
    stage = config.stage
    in_sample = _require_int(stage.in_sample_ns, "stage.in_sample_ns")
    out_of_sample = _require_int(stage.out_of_sample_ns, "stage.out_of_sample_ns")
    start = _require_int(config.start, "window.start")
    end = _require_int(config.end, "window.end")
    windows = walk_forward_windows(
        start,
        end,
        in_sample=in_sample,
        out_of_sample=out_of_sample,
        leakage=stage.leakage,
    )
    report = WalkForwardStage(in_sample=optimizer, out_of_sample=optimizer).run(
        config.space,
        windows,
    )
    return _walk_forward_records(report)


def _walk_forward_records(report: WalkForwardReport) -> dict[str, object]:
    """
    Encode the per-window walk-forward outcomes and pool the completed evaluations.
    """
    results: list[dict[str, object]] = []
    failures: list[dict[str, object]] = []
    windows: list[dict[str, object]] = []
    for window in report.windows:
        selected = window.selected
        windows.append(
            {
                "in_sample_start": window.window.in_sample_start,
                "in_sample_end": window.window.in_sample_end,
                "out_of_sample_start": window.window.out_of_sample_start,
                "out_of_sample_end": window.window.out_of_sample_end,
                "selected_experiment_digest": selected.digest if selected is not None else None,
            },
        )
        outcome = window.out_of_sample
        if isinstance(outcome, ExperimentResult):
            results.append(_result_record(outcome))
        elif isinstance(outcome, FailedExperiment):
            failures.append(_failure_record(outcome))

    ranked = SearchReport(tuple(report.out_of_sample_results()), ())
    return {
        "schema": OUTPUT_SCHEMA,
        "command": "optimize",
        "stage": "walk_forward",
        "status": "ok",
        "results": results,
        "failures": failures,
        "best": _record_or_none(ranked.best()),
        "best_feasible": _record_or_none(ranked.best_feasible()),
        "windows": windows,
    }


def _attach_context(document: dict[str, object], config: OptimizationConfig) -> None:
    """
    Attach the non-stage context to a result document.

    The declared assumption policy is part of the record, so a result names the execution
    assumptions it was produced under rather than leaving them to be inferred from a version.
    """
    document["assumptions"] = config.assumptions.to_dict()
    document["window"] = {"start": config.start, "end": config.end}
    document["persistence_directory"] = config.store_directory
    document["result_count"] = len(_sequence(document["results"]))
    document["failure_count"] = len(_sequence(document["failures"]))


def _result_record(result: ExperimentResult) -> dict[str, object]:
    """
    Encode one scored experiment result.
    """
    return {
        "experiment_digest": result.experiment.digest,
        "parameters": dict(result.experiment.parameters),
        "canonical_digest": result.run.canonical_digest,
        "score": _encode_number(result.score),
        "constraints_satisfied": result.constraints_satisfied,
        "specification": (None if result.specification is None else result.specification.to_dict()),
        "metric_values": {
            name: _encode_number(value) for name, value in result.run.metric_values.items()
        },
    }


def _failure_record(failure: FailedExperiment) -> dict[str, object]:
    """
    Encode one failed experiment.
    """
    return {
        "experiment_digest": failure.experiment.digest,
        "parameters": dict(failure.experiment.parameters),
        "error_type": failure.error_type,
        "error_message": failure.error_message,
    }


def _record_or_none(result: ExperimentResult | None) -> dict[str, object] | None:
    """
    Encode an optional best result.
    """
    return None if result is None else _result_record(result)


def _stage_experiment(config: OptimizationConfig) -> Experiment:
    """
    Build the single experiment a validate or out-of-sample stage evaluates.
    """
    parameters = config.stage.parameters
    if not parameters:
        raise ConfigError(f"stage.parameters is required for the '{config.stage.kind}' stage")
    return Experiment(dict(parameters))


def _error_document(exc: Exception) -> dict[str, object]:
    """
    Encode a failure as a machine-readable result document.
    """
    return {
        "schema": OUTPUT_SCHEMA,
        "command": "optimize",
        "status": "error",
        "error_type": type(exc).__name__,
        "error_message": str(exc),
    }


def _emit(document: dict[str, object]) -> None:
    """
    Write one strict JSON document to standard output.
    """
    sys.stdout.write(json.dumps(document, sort_keys=True, ensure_ascii=True, allow_nan=False))
    sys.stdout.write("\n")


def _parse_strategy(payload: object) -> StrategySpec:
    """
    Parse the strategy mapping.
    """
    mapping = _require_mapping(payload, "strategy")
    _reject_unknown(mapping, {"strategy_path", "config_path", "config_factory"}, "strategy")
    return StrategySpec(
        strategy_path=_require_str(mapping, "strategy_path", "strategy"),
        config_path=_require_str(mapping, "config_path", "strategy"),
        config_factory=_require_str(mapping, "config_factory", "strategy"),
    )


def _parse_space(payload: object) -> ParameterSpace:
    """
    Parse the parameter space.
    """
    mapping = _require_mapping(payload, "space")
    _reject_unknown(mapping, {"base", "parameters"}, "space")
    base = _require_json_object(mapping.get("base", {}), "space.base")

    raw_parameters = mapping.get("parameters", [])
    if not isinstance(raw_parameters, list):
        raise ConfigError("space.parameters must be an array")
    parameters = tuple(_parse_parameter(item) for item in raw_parameters)
    return ParameterSpace(base=base, parameters=parameters)


def _parse_parameter(payload: object) -> Parameter:
    """
    Parse one named parameter.
    """
    mapping = _require_mapping(payload, "space.parameters[]")
    _reject_unknown(mapping, {"name", "choices"}, "space.parameters[]")
    name = _require_str(mapping, "name", "space.parameters[]")
    choices = mapping.get("choices")
    if not isinstance(choices, list):
        raise ConfigError(f"space parameter '{name}' choices must be an array")
    values = tuple(_json_scalar(choice, f"space parameter {name!r} choice") for choice in choices)
    return Parameter(name, values)


def _parse_objective(payload: object) -> Objective:
    """
    Parse the objective.
    """
    mapping = _require_mapping(payload, "objective")
    _reject_unknown(mapping, {"terms"}, "objective")
    raw_terms = mapping.get("terms")
    if not isinstance(raw_terms, list) or not raw_terms:
        raise ConfigError("objective.terms must be a non-empty array")
    return Objective([_parse_term(term) for term in raw_terms])


def _parse_term(payload: object) -> ObjectiveTerm:
    """
    Parse one objective term.
    """
    mapping = _require_mapping(payload, "objective.terms[]")
    _reject_unknown(mapping, {"metric", "weight", "direction"}, "objective.terms[]")
    return ObjectiveTerm(
        _require_str(mapping, "metric", "objective.terms[]"),
        _require_number(mapping, "weight", "objective.terms[]"),
        _require_option(mapping, "direction", "objective.terms[]", _DIRECTIONS),
    )


def _parse_constraints(payload: object) -> tuple[Constraint, ...]:
    """
    Parse the constraints.
    """
    if not isinstance(payload, list):
        raise ConfigError("constraints must be an array")
    return tuple(_parse_constraint(item) for item in payload)


def _parse_constraint(payload: object) -> Constraint:
    """
    Parse one constraint.
    """
    mapping = _require_mapping(payload, "constraints[]")
    _reject_unknown(mapping, {"metric", "comparison", "bound"}, "constraints[]")
    return Constraint(
        _require_str(mapping, "metric", "constraints[]"),
        _require_option(mapping, "comparison", "constraints[]", _COMPARISONS),
        _require_number(mapping, "bound", "constraints[]"),
    )


def _parse_stage(payload: object) -> StageSpec:
    """
    Parse the methodology stage.
    """
    mapping = _require_mapping(payload, "stage")
    _reject_unknown(
        mapping,
        {"kind", "parameters", "in_sample_ns", "out_of_sample_ns", "leakage"},
        "stage",
    )
    kind = _require_str(mapping, "kind", "stage")
    if kind not in STAGE_KINDS:
        raise ConfigError(f"stage.kind must be one of {list(STAGE_KINDS)}, was {kind!r}")
    if mapping.get("leakage") is not None and kind != "walk_forward":
        raise ConfigError(f"stage.leakage requires the walk_forward stage, was {kind!r}")

    parameters = mapping.get("parameters")
    return StageSpec(
        kind=kind,
        parameters=(
            None if parameters is None else _require_json_object(parameters, "stage.parameters")
        ),
        in_sample_ns=_optional_int(mapping.get("in_sample_ns"), "stage.in_sample_ns"),
        out_of_sample_ns=_optional_int(mapping.get("out_of_sample_ns"), "stage.out_of_sample_ns"),
        leakage=_parse_leakage(mapping.get("leakage")),
    )


def _parse_leakage(payload: object) -> LeakagePolicy | None:
    """
    Parse the walk-forward leakage policy, which must justify any zero interval.
    """
    if payload is None:
        return None

    mapping = _require_mapping(payload, "stage.leakage")
    _reject_unknown(mapping, _LEAKAGE_KEYS, "stage.leakage")

    rule = mapping.get("label_overlap_rule")
    if rule is None:
        label_overlap_rule = LabelOverlapRule.NONE
    elif not isinstance(rule, str) or rule not in _LABEL_RULES:
        raise ConfigError(
            f"stage.leakage.label_overlap_rule must be one of {sorted(_LABEL_RULES)}, was {rule!r}",
        )
    else:
        label_overlap_rule = _LABEL_RULES[rule]

    try:
        return LeakagePolicy(
            purge_before=_optional_int(
                mapping.get("purge_before_ns"),
                "stage.leakage.purge_before_ns",
            ),
            purge_after=_optional_int(
                mapping.get("purge_after_ns"),
                "stage.leakage.purge_after_ns",
            ),
            embargo_after=_optional_int(
                mapping.get("embargo_after_ns"),
                "stage.leakage.embargo_after_ns",
            ),
            label_overlap_rule=label_overlap_rule,
            label_horizon=_optional_int(
                mapping.get("label_horizon_ns"),
                "stage.leakage.label_horizon_ns",
            ),
            zero_interval_justification=_optional_str(
                mapping.get("zero_interval_justification"),
                "stage.leakage.zero_interval_justification",
            ),
        )
    except (TypeError, ValueError) as exc:
        raise ConfigError(f"stage.leakage: {exc}") from exc


def _parse_concurrency(payload: object) -> ConcurrencyPolicy:
    """
    Parse the concurrency policy.
    """
    if payload is None:
        return ConcurrencyPolicy()
    mapping = _require_mapping(payload, "concurrency")
    _reject_unknown(mapping, {"max_workers", "per_run_bytes", "memory_fraction"}, "concurrency")

    default = ConcurrencyPolicy()
    per_run_bytes = _optional_int(mapping.get("per_run_bytes"), "concurrency.per_run_bytes")
    max_workers = _optional_int(mapping.get("max_workers"), "concurrency.max_workers")
    memory_fraction = mapping.get("memory_fraction")
    if memory_fraction is not None and not isinstance(memory_fraction, int | float):
        raise ConfigError("concurrency.memory_fraction must be a number")

    return ConcurrencyPolicy(
        per_run_bytes=default.per_run_bytes if per_run_bytes is None else per_run_bytes,
        memory_fraction=(
            default.memory_fraction if memory_fraction is None else float(memory_fraction)
        ),
        max_workers=max_workers,
    )


def _parse_store(payload: object) -> str | None:
    """
    Parse the optional persistence store.
    """
    if payload is None:
        return None
    mapping = _require_mapping(payload, "store")
    _reject_unknown(mapping, {"directory"}, "store")
    return _require_str(mapping, "directory", "store")


def _parse_window(payload: object) -> tuple[int | None, int | None]:
    """
    Parse the optional run window.
    """
    if payload is None:
        return None, None
    mapping = _require_mapping(payload, "window")
    _reject_unknown(mapping, {"start", "end"}, "window")
    return (
        _optional_int(mapping.get("start"), "window.start"),
        _optional_int(mapping.get("end"), "window.end"),
    )


def _resolve_importable(reference: str) -> Callable[..., object]:
    """
    Resolve an importable `module:attribute` reference.
    """
    module_name, separator, attribute = reference.partition(":")
    if not separator or not module_name or not attribute:
        raise ConfigError(f"Importable reference must be 'module:attribute', was {reference!r}")
    try:
        module = import_module(module_name)
    except ImportError as exc:
        raise ConfigError(f"Cannot import module {module_name!r}: {exc}") from exc
    try:
        resolved: object = getattr(module, attribute)
    except AttributeError as exc:
        raise ConfigError(f"Module {module_name!r} has no attribute {attribute!r}") from exc
    if not callable(resolved):
        raise ConfigError(f"Importable reference {reference!r} is not callable")
    return resolved


def _require_mapping(value: object, where: str) -> dict[str, object]:
    """
    Require a JSON object.
    """
    if not isinstance(value, dict):
        raise ConfigError(f"{where} must be a JSON object")
    mapping: dict[str, object] = {}
    for key, item in value.items():
        if not isinstance(key, str):
            raise ConfigError(f"{where} keys must be strings")
        mapping[key] = item
    return mapping


def _require_json_object(value: object, where: str) -> dict[str, JsonValue]:
    """
    Require a JSON object whose values are all scalars.
    """
    mapping = _require_mapping(value, where)
    return {key: _json_scalar(item, f"{where}.{key}") for key, item in mapping.items()}


def _json_scalar(value: object, where: str) -> JsonValue:
    """
    Require a JSON scalar.
    """
    if value is None or isinstance(value, str | bool | int):
        return value
    if isinstance(value, float) and math.isfinite(value):
        return value
    raise ConfigError(f"{where} must be a finite JSON scalar")


def _require_key(mapping: Mapping[str, object], key: str, where: str) -> object:
    """
    Require a key.
    """
    if key not in mapping:
        raise ConfigError(f"{where}.{key} is required")
    return mapping[key]


def _require_str(mapping: Mapping[str, object], key: str, where: str) -> str:
    """
    Require a string-valued key.
    """
    value = _require_key(mapping, key, where)
    if not isinstance(value, str) or not value:
        raise ConfigError(f"{where}.{key} must be a non-empty string")
    return value


def _require_number(mapping: Mapping[str, object], key: str, where: str) -> float:
    """
    Require a number-valued key.
    """
    value = _require_key(mapping, key, where)
    if isinstance(value, bool) or not isinstance(value, int | float):
        raise ConfigError(f"{where}.{key} must be a number")
    return float(value)


def _require_option[T](
    mapping: Mapping[str, object],
    key: str,
    where: str,
    options: Mapping[str, T],
) -> T:
    """
    Require a string-valued key that names one of the given options.
    """
    value = _require_str(mapping, key, where)
    if value not in options:
        raise ConfigError(f"{where}.{key} must be one of {sorted(options)}, was {value!r}")
    return options[value]


def _require_int(value: int | None, where: str) -> int:
    """
    Require a present integer.
    """
    if value is None:
        raise ConfigError(f"{where} is required")
    return value


def _optional_int(value: object, where: str) -> int | None:
    """
    Parse an optional integer.
    """
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int):
        raise ConfigError(f"{where} must be an integer")
    return value


def _optional_str(value: object, where: str) -> str | None:
    """
    Parse an optional string.
    """
    if value is None:
        return None
    if not isinstance(value, str):
        raise ConfigError(f"{where} must be a string")
    return value


def _optional_bool(value: object, where: str) -> bool | None:
    """
    Parse an optional boolean.
    """
    if value is None:
        return None
    if not isinstance(value, bool):
        raise ConfigError(f"{where} must be a boolean")
    return value


def _sequence(value: object) -> tuple[object, ...]:
    """
    Require a JSON array.
    """
    if not isinstance(value, list):
        raise ConfigError("Expected a JSON array")
    return tuple(value)


def _reject_unknown(mapping: Mapping[str, object], allowed: frozenset[str], where: str) -> None:
    """
    Reject keys not present in the allowed set.
    """
    unknown = sorted(set(mapping) - allowed)
    if unknown:
        raise ConfigError(f"{where} contains unknown key(s): {unknown}")


def _reject_constant(name: str) -> float:
    """
    Reject the non-finite JSON constants that Python accepts by default.
    """
    raise ConfigError(f"Configuration contains a non-finite number: {name}")


def _encode_number(value: float) -> float | str:
    """
    Encode a number, mapping non-finite floats to a string so the document is strict JSON.
    """
    return value if math.isfinite(value) else str(value)


if __name__ == "__main__":
    sys.exit(main())
