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
The cases the discovery command front end is held to.

The front end writes one document, so the cases are about what that document says: the campaign, the
resolved policy's identity and the declarations on the success path, the refusal code and the path a
refusal reached on the failure path, a credential that must reach neither, and the three exit codes.
The document is read here the way a caller reads it, from the captured standard output, because that
is the front end's whole contract.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

import pytest

from nautilus_trader.discovery import cli
from nautilus_trader.discovery.cli import main
from nautilus_trader.discovery.config import CONFIG_SCHEMA
from nautilus_trader.discovery.config import ConfigRefusalCode
from nautilus_trader.discovery.config import parse_config
from tests.unit.discovery.fixtures import fixture_policy


POLICY_NAME = "policy.json"

CONFIG_NAME = "config.json"

# A credential the canonical list names, and one whose value carries a vendor prefix rather than
# naming a variable at all
CANONICAL_CREDENTIAL = "BINANCE_API_KEY"
PREFIXED_SECRET = "sk-live-abcdefghijklmnop1234"

# The usage text, written out rather than read from the module, so an invocation answered with the
# wrong module's usage is a failure rather than a tautology
USAGE = "Usage: python -m nautilus_trader.discovery.cli CONFIG.json"

# The keys a report holds, which are the same keys whatever the document declared
SEARCH_KEYS = {
    "population_size",
    "elite",
    "family_floor",
    "mutation_rate",
    "exploration_share",
    "family_allocation",
}
WORKER_KEYS = {"capability_allowlist", "concurrency"}
CONCURRENCY_KEYS = {"per_run_bytes", "memory_fraction", "max_workers"}


def document() -> dict[str, Any]:
    """
    Build a valid configuration document.
    """
    return {
        "schema": CONFIG_SCHEMA,
        "campaign": {
            "label": "campaign-1",
            "directory": "evidence",
            "writer": "orchestrator",
            "family_id": "family-1",
            "seed": 20261017,
        },
        "policy": {"path": POLICY_NAME},
        "search": {"population_size": 16, "elite": 3, "mutation_rate": 0.25},
        "workers": {
            "capability_allowlist": ["nautilus_trader.discovery", "nautilus_trader.core"],
            "concurrency": {"per_run_bytes": 268_435_456, "memory_fraction": 0.5, "max_workers": 4},
        },
    }


def write_document(tmp_path: Path, payload: Any, name: str = CONFIG_NAME) -> Path:
    """
    Write a configuration or policy document to a file.
    """
    text = payload if isinstance(payload, str) else json.dumps(payload)
    path = tmp_path / name
    path.write_text(text, encoding="utf-8")
    return path


def write_campaign(tmp_path: Path, config: Any = None, policy: Any = None) -> Path:
    """
    Write a configuration document beside the policy document it names.
    """
    policy_payload = fixture_policy().to_dict() if policy is None else policy
    write_document(tmp_path, policy_payload, POLICY_NAME)
    return write_document(tmp_path, document() if config is None else config)


def emitted(capsys: Any) -> dict[str, Any]:
    """
    Return the one document the front end wrote, and assert it wrote nothing else.
    """
    captured = capsys.readouterr()

    assert captured.err == ""
    assert captured.out.count("\n") == 1
    return json.loads(captured.out)


def test_a_reported_configuration_carries_the_campaign_it_declares(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a reported configuration names the campaign it opens.
    """
    path = write_campaign(tmp_path)

    assert main([str(path)]) == 0
    assert emitted(capsys)["campaign"] == {
        "label": "campaign-1",
        "directory": "evidence",
        "writer": "orchestrator",
        "family_id": "family-1",
        "seed": 20261017,
    }


def test_a_reported_configuration_carries_the_resolved_policy_identity(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a reported configuration names the policy the loader read, not only the path it read.
    """
    path = write_campaign(tmp_path)

    assert main([str(path)]) == 0
    report = emitted(capsys)
    policy = report["policy"]

    assert policy["path"] == POLICY_NAME
    assert policy["declared_digest"] == ""
    assert policy["policy_id"] == fixture_policy().policy_id
    assert policy["version"] == fixture_policy().version
    assert policy["digest"] == fixture_policy().digest()


def test_a_reported_configuration_carries_the_declared_search(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test the search block reports what the document declared.
    """
    path = write_campaign(tmp_path)

    assert main([str(path)]) == 0
    search = emitted(capsys)["search"]

    assert search["population_size"] == 16
    assert search["elite"] == 3
    assert search["mutation_rate"] == 0.25
    assert search["family_allocation"] == []


def test_the_family_allocation_is_reported_in_declared_order(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test an allocated population is reported as pairs in the order it was declared in.

    The pairs are what keeps the declared order readable in the document: `_emit` sorts the keys of
    every object it writes, so a mapping would report the families in alphabetical order instead.
    """
    payload = document()
    payload["search"]["family_allocation"] = {"carry": 0.25, "trend": 0.5}
    path = write_campaign(tmp_path, config=payload)

    assert main([str(path)]) == 0
    assert emitted(capsys)["search"]["family_allocation"] == [["carry", 0.25], ["trend", 0.5]]


def test_a_reported_configuration_carries_the_declared_workers(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test the worker block reports the allowlist and the concurrency policy as declared.
    """
    path = write_campaign(tmp_path)

    assert main([str(path)]) == 0
    assert emitted(capsys)["workers"] == {
        "capability_allowlist": ["nautilus_trader.discovery", "nautilus_trader.core"],
        "concurrency": {
            "per_run_bytes": 268_435_456,
            "memory_fraction": 0.5,
            "max_workers": 4,
        },
    }


def test_the_report_holds_the_same_keys_whatever_the_document_declares(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a document without a search or worker block reports the same keys.

    The report's shape is the front end's, so a configuration that declares less is reported with
    the same keys and with an allowlist that admits nothing.
    """
    payload = document()
    del payload["search"]
    del payload["workers"]
    path = write_campaign(tmp_path, config=payload)

    assert main([str(path)]) == 0
    report = emitted(capsys)

    assert set(report) == {"campaign", "command", "policy", "schema", "search", "status", "workers"}
    assert set(report["search"]) == SEARCH_KEYS
    assert set(report["workers"]) == WORKER_KEYS
    assert set(report["workers"]["concurrency"]) == CONCURRENCY_KEYS
    assert report["workers"]["capability_allowlist"] == []


def test_the_document_names_its_schema_its_command_and_its_status(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a report is one strict document whose keys are sorted.
    """
    path = write_campaign(tmp_path)

    assert main([str(path)]) == 0
    captured = capsys.readouterr()
    report = json.loads(captured.out)

    assert report["schema"] == "nautilus.discovery.cli/v1"
    assert report["command"] == "discovery"
    assert report["status"] == "ok"
    assert list(report) == sorted(report)


def test_main_reads_the_process_arguments_when_it_is_given_none(
    tmp_path: Path,
    capsys: Any,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """
    Test the front end reads `sys.argv[1:]` when it is not given arguments.
    """
    path = write_campaign(tmp_path)
    monkeypatch.setattr(sys, "argv", ["nautilus_trader.discovery.cli", str(path)])

    assert main() == 0
    assert emitted(capsys)["status"] == "ok"


@pytest.mark.parametrize("arguments", [[], ["one.json", "two.json"]])
def test_an_invocation_that_is_not_one_path_is_a_usage_error(
    arguments: list[str],
    capsys: Any,
) -> None:
    """
    Test a wrong invocation is answered with the usage text and exit code two.
    """
    assert main(arguments) == 2
    report = emitted(capsys)

    assert report["status"] == "error"
    assert report["error_type"] == "UsageError"
    assert report["error_message"] == USAGE
    assert "error_code" not in report


def test_a_document_that_cannot_be_read_is_reported_with_its_code(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a path that holds no document is a refusal rather than a traceback.
    """
    assert main([str(tmp_path / "absent.json")]) == 1
    report = emitted(capsys)

    assert report["status"] == "error"
    assert report["error_type"] == "ConfigError"
    assert report["error_code"] == ConfigRefusalCode.DOCUMENT_NOT_READABLE.value
    assert "error_path" not in report


def test_a_document_that_is_not_json_is_reported_with_its_code(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a document that is not JSON is a refusal rather than a traceback.
    """
    path = write_document(tmp_path, "{not json")

    assert main([str(path)]) == 1
    assert emitted(capsys)["error_code"] == ConfigRefusalCode.DOCUMENT_NOT_JSON.value


@pytest.mark.parametrize(
    ("where", "path"),
    [
        ("config", "config"),
        ("campaign", "campaign"),
        ("search", "search"),
        ("workers", "workers"),
    ],
)
def test_an_unknown_key_is_reported_with_the_path_it_is_at(
    tmp_path: Path,
    capsys: Any,
    where: str,
    path: str,
) -> None:
    """
    Test an unknown key is reported with its code and the location it was found at.
    """
    payload = document()
    if where == "config":
        payload["unknown"] = 1
    else:
        payload[where]["unknown"] = 1
    source = write_campaign(tmp_path, config=payload)

    assert main([str(source)]) == 1
    report = emitted(capsys)

    assert report["error_type"] == "ConfigError"
    assert report["error_code"] == ConfigRefusalCode.UNKNOWN_FIELD.value
    assert report["error_path"] == path


def test_a_declared_key_that_is_absent_is_reported_with_its_path(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a missing key is reported with the location a reader has to visit.
    """
    payload = document()
    del payload["campaign"]["seed"]
    path = write_campaign(tmp_path, config=payload)

    assert main([str(path)]) == 1
    report = emitted(capsys)

    assert report["error_code"] == ConfigRefusalCode.FIELD_MISSING.value
    assert report["error_path"] == "campaign.seed"


def test_a_credential_named_in_the_configuration_is_refused_without_being_echoed(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a credential named in the configuration is refused, and does not reach the document.
    """
    payload = document()
    payload["campaign"]["writer"] = CANONICAL_CREDENTIAL
    path = write_campaign(tmp_path, config=payload)

    assert main([str(path)]) == 1
    captured = capsys.readouterr()
    report = json.loads(captured.out)

    assert report["error_code"] == ConfigRefusalCode.CREDENTIAL_REFUSED.value
    assert report["error_path"] == "config.campaign.writer"
    assert CANONICAL_CREDENTIAL not in captured.out


def test_a_credential_shaped_value_in_the_policy_is_refused_without_being_echoed(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a secret read from the policy document is refused by its shape and not repeated.
    """
    policy = fixture_policy().to_dict()
    policy["families"] = [PREFIXED_SECRET]
    path = write_campaign(tmp_path, policy=policy)

    assert main([str(path)]) == 1
    captured = capsys.readouterr()
    report = json.loads(captured.out)

    assert report["error_code"] == ConfigRefusalCode.CREDENTIAL_REFUSED.value
    assert report["error_path"] == "policy.families[0]"
    assert PREFIXED_SECRET not in captured.out


def test_a_digest_pin_that_does_not_match_is_reported(tmp_path: Path, capsys: Any) -> None:
    """
    Test a policy pin the document does not satisfy is a refusal with its own code.
    """
    payload = document()
    payload["policy"] = {"path": POLICY_NAME, "digest": "sha256:" + "c" * 64}
    path = write_campaign(tmp_path, config=payload)

    assert main([str(path)]) == 1
    report = emitted(capsys)

    assert report["error_code"] == ConfigRefusalCode.POLICY_DIGEST_MISMATCH.value
    assert report["error_path"] == "policy.digest"


def test_a_policy_the_object_refuses_is_reported_with_its_own_message(
    tmp_path: Path,
    capsys: Any,
) -> None:
    """
    Test a policy the policy object refuses is one document rather than a traceback.
    """
    policy = fixture_policy().to_dict()
    policy["families"] = []
    path = write_campaign(tmp_path, policy=policy)

    assert main([str(path)]) == 1
    report = emitted(capsys)

    assert report["error_code"] == ConfigRefusalCode.POLICY_REFUSED.value
    assert "families must not be empty" in report["error_message"]


@pytest.mark.parametrize(
    "entry",
    ["nautilus_trader.adapters", "nautilus_trader.adapters.binance"],
)
def test_an_allowlist_that_names_an_adapter_is_reported(
    tmp_path: Path,
    capsys: Any,
    entry: str,
) -> None:
    """
    Test the front end reports the allowlist refusal the loader makes.
    """
    payload = document()
    payload["workers"]["capability_allowlist"] = ["nautilus_trader.core", entry]
    path = write_campaign(tmp_path, config=payload)

    assert main([str(path)]) == 1
    report = emitted(capsys)

    assert report["error_code"] == ConfigRefusalCode.ALLOWLIST_ENTRY_NAMES_ADAPTER.value
    assert report["error_path"] == "workers.capability_allowlist[1]"


def test_a_configuration_that_was_parsed_rather_than_loaded_reports_no_identity() -> None:
    """
    Test a configuration without a resolved policy reports no identity to report.
    """
    report = cli._configuration_document(parse_config(document()))

    assert report["policy"] == {
        "path": POLICY_NAME,
        "declared_digest": "",
        "policy_id": None,
        "version": None,
        "digest": None,
    }
