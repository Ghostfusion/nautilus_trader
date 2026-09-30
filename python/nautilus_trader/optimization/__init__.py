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
validate, out-of-sample, walk-forward) are explicit rather than one loop over a grid.
"""

from nautilus_trader.optimization.concurrency import ConcurrencyPolicy as ConcurrencyPolicy
from nautilus_trader.optimization.config import ConfigError as ConfigError
from nautilus_trader.optimization.config import OptimizationConfig as OptimizationConfig
from nautilus_trader.optimization.config import load_config as load_config
from nautilus_trader.optimization.config import run_config as run_config
from nautilus_trader.optimization.metrics import statistic_values as statistic_values
from nautilus_trader.optimization.optimizer import Optimizer as Optimizer
from nautilus_trader.optimization.persistence import ExperimentStore as ExperimentStore
from nautilus_trader.optimization.report import ExperimentResult as ExperimentResult
from nautilus_trader.optimization.report import SearchReport as SearchReport
from nautilus_trader.optimization.runner import BacktestRunner as BacktestRunner
from nautilus_trader.optimization.runner import CanonicalRun as CanonicalRun
from nautilus_trader.optimization.runner import FailedExperiment as FailedExperiment
from nautilus_trader.optimization.search import GridSearch as GridSearch
from nautilus_trader.optimization.search import SearchStrategy as SearchStrategy
from nautilus_trader.optimization.space import Experiment as Experiment
from nautilus_trader.optimization.space import Parameter as Parameter
from nautilus_trader.optimization.space import ParameterSpace as ParameterSpace
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
    "BacktestRunner",
    "CanonicalRun",
    "ConcurrencyPolicy",
    "ConfigError",
    "Experiment",
    "ExperimentResult",
    "ExperimentStore",
    "FailedExperiment",
    "GridSearch",
    "OptimizationConfig",
    "OptimizeStage",
    "Optimizer",
    "OutOfSampleStage",
    "Parameter",
    "ParameterSpace",
    "SearchReport",
    "SearchStrategy",
    "TrainStage",
    "ValidateStage",
    "ValidationResult",
    "WalkForwardReport",
    "WalkForwardResult",
    "WalkForwardStage",
    "WalkForwardWindow",
    "load_config",
    "run_config",
    "statistic_values",
    "walk_forward_windows",
]
