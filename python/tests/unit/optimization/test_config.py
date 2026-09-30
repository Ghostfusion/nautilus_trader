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
Tests for the optimization configuration file entry point.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from nautilus_trader.analysis import ConstraintComparison
from nautilus_trader.analysis import ObjectiveDirection
from nautilus_trader.optimization.config import CONFIG_SCHEMA
from nautilus_trader.optimization.config import ConfigError
from nautilus_trader.optimization.config import load_config
from nautilus_trader.optimization.config import main
from nautilus_trader.optimization.config import parse_config
from nautilus_trader.optimization.config import run_config


STRATEGY = "strategies.ema_cross:EMACross"
STRATEGY_CONFIG = "strategies.ema_cross:EMACrossConfig"
FACTORY = "tests.integration.test_optimization:_config_parts"


def document() -> dict[str, Any]:
    """
    Build a valid configuration document.
    """
    return {
        "schema": CONFIG_SCHEMA,
        "strategy": {
            "strategy_path": STRATEGY,
            "config_path": STRATEGY_CONFIG,
            "config_factory": FACTORY,
        },
        "space": {
            "base": {"instrument_id": "BTCUSDT.BINANCE"},
            "parameters": [
                {"name": "fast_ema_period", "choices": [5, 10]},
                {"name": "slow_ema_period", "choices": [20, 30]},
            ],
        },
        "window": {"start": 100, "end": 200},
        "objective": {
            "terms": [
                {"metric": "Sharpe Ratio (252 days)", "weight": 1.0, "direction": "maximize"},
            ],
        },
        "constraints": [{"metric": "Max Drawdown", "comparison": "at_least", "bound": -0.013}],
        "stage": {"kind": "optimize"},
        "concurrency": {"max_workers": 2, "memory_fraction": 0.5},
        "store": {"directory": "store"},
    }


def write_document(tmp_path: Path, payload: str) -> Path:
    """
    Write a raw configuration document to a file.
    """
    path = tmp_path / "config.json"
    path.write_text(payload, encoding="utf-8")
    return path


def test_valid_document_parses_into_the_declared_types(tmp_path: Path) -> None:
    """
    Test a valid document builds the declared optimizer inputs.
    """
    config = load_config(write_document(tmp_path, json.dumps(document())))

    assert config.strategy.strategy_path == STRATEGY
    assert config.strategy.config_factory == FACTORY
    assert config.start == 100
    assert config.end == 200
    assert len(list(config.space.expand())) == 4
    assert config.objective.terms[0].metric == "Sharpe Ratio (252 days)"
    assert config.objective.terms[0].direction == ObjectiveDirection.MAXIMIZE
    assert config.constraints[0].comparison == ConstraintComparison.AT_LEAST
    assert config.constraints[0].bound == -0.013
    assert config.stage.kind == "optimize"
    assert config.concurrency.max_workers == 2
    assert config.concurrency.memory_fraction == 0.5
    assert config.store_directory == "store"


def test_unknown_top_level_key_is_rejected() -> None:
    """
    Test an unknown top-level key is rejected rather than ignored.
    """
    payload = document()
    payload["unknown"] = 1

    with pytest.raises(ConfigError, match="unknown key"):
        parse_config(payload)


@pytest.mark.parametrize(
    "key",
    ["strategy", "space", "objective", "stage", "concurrency", "store"],
)
def test_unknown_nested_key_is_rejected(key: str) -> None:
    """
    Test an unknown key in each nested mapping is rejected.
    """
    payload = document()
    payload[key]["unknown"] = 1

    with pytest.raises(ConfigError, match="unknown key"):
        parse_config(payload)


def test_unknown_term_key_is_rejected() -> None:
    """
    Test an unknown key in an objective term is rejected.
    """
    payload = document()
    payload["objective"]["terms"][0]["unknown"] = 1

    with pytest.raises(ConfigError, match="unknown key"):
        parse_config(payload)


def test_missing_required_key_is_rejected() -> None:
    """
    Test a missing required mapping is rejected.
    """
    payload = document()
    del payload["objective"]

    with pytest.raises(ConfigError, match="objective is required"):
        parse_config(payload)


def test_unsupported_stage_kind_is_rejected() -> None:
    """
    Test an unsupported stage kind is rejected.
    """
    payload = document()
    payload["stage"] = {"kind": "unknown"}

    with pytest.raises(ConfigError, match=r"stage\.kind"):
        parse_config(payload)


def test_non_finite_number_is_rejected(tmp_path: Path) -> None:
    """
    Test a non-finite JSON number is rejected.
    """
    text = json.dumps(document()).replace('"weight": 1.0', '"weight": Infinity')

    with pytest.raises(ConfigError, match="non-finite"):
        load_config(write_document(tmp_path, text))


def test_invalid_json_is_rejected(tmp_path: Path) -> None:
    """
    Test invalid JSON is rejected.
    """
    with pytest.raises(ConfigError, match="not valid JSON"):
        load_config(write_document(tmp_path, "{"))


def test_walk_forward_requires_the_window_and_lengths() -> None:
    """
    Test a walk-forward stage without its inputs is rejected before any run.
    """
    payload = document()
    payload["stage"] = {"kind": "walk_forward"}
    payload["window"] = {"start": None, "end": None}
    del payload["store"]

    with pytest.raises(ConfigError, match=r"stage\.in_sample_ns"):
        run_config(parse_config(payload))


def test_out_of_sample_requires_the_selected_parameters() -> None:
    """
    Test an out-of-sample stage without a parameter set is rejected before any run.
    """
    payload = document()
    payload["stage"] = {"kind": "out_of_sample"}
    del payload["store"]

    with pytest.raises(ConfigError, match=r"stage\.parameters is required"):
        run_config(parse_config(payload))


def test_validate_parses_the_selected_parameters() -> None:
    """
    Test a validate stage parses its parameter set.
    """
    payload = document()
    payload["stage"] = {"kind": "validate", "parameters": {"fast_ema_period": 10}}
    parsed = parse_config(payload)

    assert parsed.stage.parameters == {"fast_ema_period": 10}


def test_main_reports_an_unknown_key_with_a_nonzero_exit_code(tmp_path: Path, capsys: Any) -> None:
    """
    Test the entry point reports an unknown key in the document and the exit code.
    """
    payload = document()
    payload["unknown"] = 1
    path = write_document(tmp_path, json.dumps(payload))

    exit_code = main([str(path)])

    output = json.loads(capsys.readouterr().out)
    assert exit_code == 1
    assert output["status"] == "error"
    assert output["error_type"] == "ConfigError"
    assert "unknown key" in output["error_message"]


def test_main_reports_usage_with_exit_code_two(capsys: Any) -> None:
    """
    Test the entry point reports a usage error with exit code two.
    """
    exit_code = main([])

    output = json.loads(capsys.readouterr().out)
    assert exit_code == 2
    assert output["status"] == "error"
