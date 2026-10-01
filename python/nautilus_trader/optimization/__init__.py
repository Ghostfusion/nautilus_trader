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
Parameter optimization over backtest runs, composed in Python over Rust execution.

The pipeline is a parameter space, a search strategy, a run through `BacktestNode`, an objective
and constraints from `nautilus_trader.analysis`, and a ranked result. The optimizer composes runs
and never alters one, so every backtest still goes through the single execution path. Runs fan out
to processes with a memory-driven concurrency limit, and the methodology stages (train, optimize,
validate, out-of-sample, walk-forward) are explicit rather than one loop over a grid. Validation
windows come from a split contract that states which observations the evaluation information
excludes from training, rather than from an implicit time distance.
"""

from nautilus_trader.optimization.assumptions import BarAmbiguityPolicy as BarAmbiguityPolicy
from nautilus_trader.optimization.assumptions import GapHandling as GapHandling
from nautilus_trader.optimization.assumptions import IntrabarPath as IntrabarPath
from nautilus_trader.optimization.assumptions import TriggerFill as TriggerFill
from nautilus_trader.optimization.assumptions import TriggerPrecedence as TriggerPrecedence
from nautilus_trader.optimization.capability import ResearchCapabilityCode as ResearchCapabilityCode
from nautilus_trader.optimization.capability import (
    gapped_range_capability as gapped_range_capability,
)
from nautilus_trader.optimization.capability import history_capability as history_capability
from nautilus_trader.optimization.capability import leakage_capability as leakage_capability
from nautilus_trader.optimization.capability import (
    pair_distinguishability_capability as pair_distinguishability_capability,
)
from nautilus_trader.optimization.capability import persistence_capability as persistence_capability
from nautilus_trader.optimization.capability import (
    screen_family_capability as screen_family_capability,
)
from nautilus_trader.optimization.capability import (
    significance_capability as significance_capability,
)
from nautilus_trader.optimization.capability import split_capability as split_capability
from nautilus_trader.optimization.concurrency import ConcurrencyPolicy as ConcurrencyPolicy
from nautilus_trader.optimization.config import ConfigError as ConfigError
from nautilus_trader.optimization.config import OptimizationConfig as OptimizationConfig
from nautilus_trader.optimization.config import load_config as load_config
from nautilus_trader.optimization.config import run_config as run_config
from nautilus_trader.optimization.identity import ComputationIdentity as ComputationIdentity
from nautilus_trader.optimization.identity import DatasetIdentity as DatasetIdentity
from nautilus_trader.optimization.identity import ExecutionStatus as ExecutionStatus
from nautilus_trader.optimization.identity import ResearchResult as ResearchResult
from nautilus_trader.optimization.identity import SelectionRule as SelectionRule
from nautilus_trader.optimization.identity import StudyIdentity as StudyIdentity
from nautilus_trader.optimization.identity import TrialIdentity as TrialIdentity
from nautilus_trader.optimization.identity import TrialProvenance as TrialProvenance
from nautilus_trader.optimization.identity import UniverseIdentity as UniverseIdentity
from nautilus_trader.optimization.identity import (
    objective_definition_from_terms as objective_definition_from_terms,
)
from nautilus_trader.optimization.identity import split_contract_digest as split_contract_digest
from nautilus_trader.optimization.identity import trial_identity as trial_identity
from nautilus_trader.optimization.labels import AlignmentConvention as AlignmentConvention
from nautilus_trader.optimization.labels import ForwardAggregate as ForwardAggregate
from nautilus_trader.optimization.labels import LabelDefinition as LabelDefinition
from nautilus_trader.optimization.labels import LabelKind as LabelKind
from nautilus_trader.optimization.labels import LabelSeries as LabelSeries
from nautilus_trader.optimization.labels import MissingDataPolicy as MissingDataPolicy
from nautilus_trader.optimization.labels import label_series as label_series
from nautilus_trader.optimization.metrics import statistic_values as statistic_values
from nautilus_trader.optimization.optimizer import Optimizer as Optimizer
from nautilus_trader.optimization.persistence import Evaluation as Evaluation
from nautilus_trader.optimization.persistence import EvaluationCache as EvaluationCache
from nautilus_trader.optimization.persistence import ExperimentStore as ExperimentStore
from nautilus_trader.optimization.relative_value import (
    PersistenceConvention as PersistenceConvention,
)
from nautilus_trader.optimization.relative_value import PersistenceEstimate as PersistenceEstimate
from nautilus_trader.optimization.relative_value import RecursiveMemory as RecursiveMemory
from nautilus_trader.optimization.relative_value import ScreenFamily as ScreenFamily
from nautilus_trader.optimization.relative_value import ScreenOutcome as ScreenOutcome
from nautilus_trader.optimization.relative_value import SignalWindow as SignalWindow
from nautilus_trader.optimization.report import ExperimentResult as ExperimentResult
from nautilus_trader.optimization.report import SearchReport as SearchReport
from nautilus_trader.optimization.run import RunDescription as RunDescription
from nautilus_trader.optimization.run import ValidationMode as ValidationMode
from nautilus_trader.optimization.run import ValidationScheme as ValidationScheme
from nautilus_trader.optimization.runner import BacktestRunner as BacktestRunner
from nautilus_trader.optimization.runner import CanonicalRun as CanonicalRun
from nautilus_trader.optimization.runner import FailedExperiment as FailedExperiment
from nautilus_trader.optimization.search import EvolutionaryOperators as EvolutionaryOperators
from nautilus_trader.optimization.search import EvolutionarySearch as EvolutionarySearch
from nautilus_trader.optimization.search import GridSearch as GridSearch
from nautilus_trader.optimization.search import RandomSearch as RandomSearch
from nautilus_trader.optimization.search import SearchStrategy as SearchStrategy
from nautilus_trader.optimization.significance import DivisorConvention as DivisorConvention
from nautilus_trader.optimization.significance import ReturnCompounding as ReturnCompounding
from nautilus_trader.optimization.significance import ReturnMoments as ReturnMoments
from nautilus_trader.optimization.significance import SharpeEstimate as SharpeEstimate
from nautilus_trader.optimization.significance import SharpeFrequency as SharpeFrequency
from nautilus_trader.optimization.significance import SharpeSample as SharpeSample
from nautilus_trader.optimization.significance import SignificanceReport as SignificanceReport
from nautilus_trader.optimization.significance import SignificanceResult as SignificanceResult
from nautilus_trader.optimization.significance import StatisticalContract as StatisticalContract
from nautilus_trader.optimization.significance import TrialDependence as TrialDependence
from nautilus_trader.optimization.significance import deflated_sharpe_ratio as deflated_sharpe_ratio
from nautilus_trader.optimization.significance import per_period_sharpe as per_period_sharpe
from nautilus_trader.optimization.significance import return_moments as return_moments
from nautilus_trader.optimization.significance import significance_report as significance_report
from nautilus_trader.optimization.significance import (
    trial_provenance_from_runs as trial_provenance_from_runs,
)
from nautilus_trader.optimization.space import Experiment as Experiment
from nautilus_trader.optimization.space import Parameter as Parameter
from nautilus_trader.optimization.space import ParameterSpace as ParameterSpace
from nautilus_trader.optimization.splits import LabelOverlapRule as LabelOverlapRule
from nautilus_trader.optimization.splits import LeakagePolicy as LeakagePolicy
from nautilus_trader.optimization.splits import Split as Split
from nautilus_trader.optimization.splits import SplitContract as SplitContract
from nautilus_trader.optimization.splits import SplitDirection as SplitDirection
from nautilus_trader.optimization.stages import OptimizeStage as OptimizeStage
from nautilus_trader.optimization.stages import OutOfSampleStage as OutOfSampleStage
from nautilus_trader.optimization.stages import TrainStage as TrainStage
from nautilus_trader.optimization.stages import ValidateStage as ValidateStage
from nautilus_trader.optimization.stages import ValidationResult as ValidationResult
from nautilus_trader.optimization.stages import WalkForwardReport as WalkForwardReport
from nautilus_trader.optimization.stages import WalkForwardResult as WalkForwardResult
from nautilus_trader.optimization.stages import WalkForwardStage as WalkForwardStage
from nautilus_trader.optimization.stages import WalkForwardWindow as WalkForwardWindow
from nautilus_trader.optimization.stages import walk_forward_windows as walk_forward_windows


__all__ = [
    "AlignmentConvention",
    "BacktestRunner",
    "BarAmbiguityPolicy",
    "CanonicalRun",
    "ComputationIdentity",
    "ConcurrencyPolicy",
    "ConfigError",
    "DatasetIdentity",
    "DivisorConvention",
    "Evaluation",
    "EvaluationCache",
    "EvolutionaryOperators",
    "EvolutionarySearch",
    "ExecutionStatus",
    "Experiment",
    "ExperimentResult",
    "ExperimentStore",
    "FailedExperiment",
    "ForwardAggregate",
    "GapHandling",
    "GridSearch",
    "IntrabarPath",
    "LabelDefinition",
    "LabelKind",
    "LabelOverlapRule",
    "LabelSeries",
    "LeakagePolicy",
    "MissingDataPolicy",
    "OptimizationConfig",
    "OptimizeStage",
    "Optimizer",
    "OutOfSampleStage",
    "Parameter",
    "ParameterSpace",
    "PersistenceConvention",
    "PersistenceEstimate",
    "RandomSearch",
    "RecursiveMemory",
    "ResearchCapabilityCode",
    "ResearchResult",
    "ReturnCompounding",
    "ReturnMoments",
    "RunDescription",
    "ScreenFamily",
    "ScreenOutcome",
    "SearchReport",
    "SearchStrategy",
    "SelectionRule",
    "SharpeEstimate",
    "SharpeFrequency",
    "SharpeSample",
    "SignalWindow",
    "SignificanceReport",
    "SignificanceResult",
    "Split",
    "SplitContract",
    "SplitDirection",
    "StatisticalContract",
    "StudyIdentity",
    "TrainStage",
    "TrialDependence",
    "TrialIdentity",
    "TrialProvenance",
    "TriggerFill",
    "TriggerPrecedence",
    "UniverseIdentity",
    "ValidateStage",
    "ValidationMode",
    "ValidationResult",
    "ValidationScheme",
    "WalkForwardReport",
    "WalkForwardResult",
    "WalkForwardStage",
    "WalkForwardWindow",
    "deflated_sharpe_ratio",
    "gapped_range_capability",
    "history_capability",
    "label_series",
    "leakage_capability",
    "load_config",
    "objective_definition_from_terms",
    "pair_distinguishability_capability",
    "per_period_sharpe",
    "persistence_capability",
    "return_moments",
    "run_config",
    "screen_family_capability",
    "significance_capability",
    "significance_report",
    "split_capability",
    "split_contract_digest",
    "statistic_values",
    "trial_identity",
    "trial_provenance_from_runs",
    "walk_forward_windows",
]
