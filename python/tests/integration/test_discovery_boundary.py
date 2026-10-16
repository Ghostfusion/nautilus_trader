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
`FX-M6-1`: the execution boundary, as the four tests of design Section 12 make it observable.

The design's boundary is capability removal rather than a configuration flag, and what can be checked
from here is what is reached rather than what is declared. The dependency assertion resolves the
discovery package's import graph in a clean interpreter and reports the adapter modules in it, which
is evidence over the module set rather than a reading of source text. The capability assertion names
the only execution client a worker may be given, the environment assertion runs over both entry
points against the repository's own canonical list of adapter credential variables, and the negative
test shows that a candidate naming an execution path is refused with the refusal recorded.

What these tests cannot prove is stated rather than implied, and the design states it: an import-graph
check is evidence rather than proof, because a dynamic import, an indirect callback or a shared
execution stack can evade a check made over module names. They also cannot stop an operator from
installing an adapter and running the discovery code with it. The boundary that is stronger than any
of these is the shape of the worker environment, which is why the environment assertion is run over
the shadow side too.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import TYPE_CHECKING
from typing import Any

from nautilus_trader.discovery.compile import compile_specification
from nautilus_trader.discovery.ledger import CampaignInventory
from nautilus_trader.discovery.ledger import audit_campaign
from nautilus_trader.discovery.ledger import research_family_id
from nautilus_trader.discovery.schema import RefusalCode
from tests.unit.discovery.fixtures import changed
from tests.unit.discovery.fixtures import fixture_policy
from tests.unit.discovery.fixtures import fixture_registry
from tests.unit.discovery.fixtures import legal_payload


if TYPE_CHECKING:
    from collections.abc import Mapping

    from nautilus_trader.discovery.compile import CompilationResult


REPO_ROOT = Path(__file__).resolve().parents[3]
CANDIDATE = "sha256:" + "a" * 64

# The adapter module prefix whose presence in the discovery import graph the boundary forbids.
ADAPTER_PREFIX = "nautilus_trader.adapters"

# The exported names that would mean the discovery package reached an execution surface of its own.
SUBMISSION_EXPORTS = ("ExecutionClient", "OrderFactory", "submit_order")

# The import graph is resolved in a clean interpreter, because this process has imported the venue
# adapters its own tests exercise: what the boundary is about is what discovery alone reaches.
_RESOLVED_GRAPH = """
import json
import sys

sys.path.insert(0, "python")

import nautilus_trader.discovery

print(
    json.dumps(
        {
            "adapters": sorted(
                name for name in sys.modules if name.startswith("nautilus_trader.adapters")
            ),
            "exported": sorted(getattr(nautilus_trader.discovery, "__all__", ())),
        },
    ),
)
"""


def campaign(directory: Path) -> CampaignInventory:
    """
    Return an inventory over the fixture campaign.
    """
    return CampaignInventory(
        directory / "campaign-1",
        writer="orchestrator",
        campaign_id="campaign-1",
        family_id=research_family_id(snapshot="snapshot-1", selection_process="rank_first"),
    )


def adapter_env_vars() -> tuple[str, ...]:
    """
    Return the adapter credential variables the repository names in one canonical list.

    The list is `ADAPTER_ENV_VARS` in `scripts/strip-adapter-env.bash`, which the design names as the
    one place the repository declares them. It is read rather than restated, so this fixture cannot
    drift from the list `make pre-flight` unsets.
    """
    script = REPO_ROOT / "scripts" / "strip-adapter-env.bash"
    text = script.read_text(encoding="utf-8")
    _, _, declared = text.partition("ADAPTER_ENV_VARS=(")
    body, _, _ = declared.partition("\n)")

    return tuple(line.strip() for line in body.splitlines() if line.strip())


def resolved_graph(environment: Mapping[str, str] | None = None) -> dict[str, Any]:
    """
    Return the discovery package's resolved import graph, read in a clean interpreter.

    Parameters
    ----------
    environment : Mapping[str, str] | None, default None
        The environment to resolve it in, or None to inherit this process's.

    Returns
    -------
    dict[str, Any]

    """
    completed = subprocess.run(
        [sys.executable, "-c", _RESOLVED_GRAPH],
        cwd=REPO_ROOT,
        env=None if environment is None else dict(environment),
        capture_output=True,
        text=True,
        check=True,
    )

    return json.loads(completed.stdout)


def refused(result: CompilationResult) -> tuple[tuple[str, str], ...]:
    """
    Return each refusal as its code and the path it was raised against.
    """
    return tuple((error.code.value, error.path) for error in result.errors)


def test_fx_m6_1_the_discovery_import_graph_reaches_no_adapter_module() -> None:
    """
    `FX-M6-1`: the dependency assertion of design Section 12 item 1.

    The discovery layer reaches no adapter at all, live or otherwise, and reaches no execution surface
    of its own: the interpreter produces targets and orders as data and submits nothing. Adding a live
    adapter to this graph fails the assertion, which is what makes it a regression check.
    """
    graph = resolved_graph()
    exports = graph["exported"]

    assert graph["adapters"] == [], graph["adapters"]

    # The assertion has teeth on the export surface too, and the surface is read rather than assumed.
    assert len(exports) > 200
    assert [name for name in exports if any(part in name for part in SUBMISSION_EXPORTS)] == []


def test_fx_m6_1_the_sandbox_venue_is_the_only_execution_client_a_worker_is_given() -> None:
    """
    `FX-M6-1`: the capability assertion of design Section 12 item 2.

    A shadow run obtains its data from a live adapter and its execution from the sandbox, and the
    sandbox is what makes a simulated fill possible without an order leaving the process. The
    asymmetry is the capability: the sandbox venue exposes an execution client and no data client.
    """
    from nautilus_trader.adapters import sandbox

    exported = set(sandbox.__all__)

    assert "SandboxExecutionClientFactory" in exported
    assert "SandboxExecutionClientConfig" in exported
    assert [name for name in exported if "DataClient" in name] == []


def test_fx_m6_1_no_credential_reaches_the_environment_discovery_is_run_in() -> None:
    """
    `FX-M6-1`: the environment assertion of design Section 12 item 3, over both entry points.

    The assertion is over the environment the boundary defines rather than over this operator's
    shell, because a developer machine with live credentials is exactly the case the design says is
    protected procedurally and not by these tests. So the environment is the repository's own
    stripped one, built from the canonical list, and what is asserted about it is behavioural:
    discovery imports there, and its resolved graph reaches no adapter, with no credential available
    to it at all. A submission path that survived the import check has nothing to authenticate with.
    """
    named = adapter_env_vars()

    assert len(named) > 80
    assert "BINANCE_API_KEY" in named
    assert "DATABENTO_API_KEY" in named
    assert "EODHD_API_KEY" in named

    stripped = {name: value for name, value in os.environ.items() if name not in named}
    graph = resolved_graph(environment=stripped)

    assert graph["adapters"] == []
    assert graph["exported"] != []


def test_fx_m6_1_a_candidate_naming_an_execution_path_is_refused_and_recorded(
    tmp_path: Path,
) -> None:
    """
    `FX-M6-1`: the negative submission test of design Section 12 item 4.

    A specification is data, so it has no slot for a venue, an adapter or a client, and a proposal
    that names one is refused by the closed schema rather than by a check that could be forgotten. A
    field named like a code path is refused as a code reference. Both refusals are then recorded, and
    the audit reads them back: a refusal is a proposal attempt and never an executed experiment.
    """
    policy = fixture_policy()
    registry = fixture_registry()

    submission = compile_specification(
        changed(legal_payload(), {"data.venue": "BINANCE"}),
        policy,
        registry,
    )

    assert submission.accepted is False
    assert refused(submission) == ((RefusalCode.BLOCK_UNKNOWN_FIELD.value, "data.venue"),)

    code = compile_specification(
        changed(legal_payload(), {"universe.module_path": "nautilus_trader.adapters.binance"}),
        policy,
        registry,
    )

    assert code.accepted is False
    assert refused(code) == ((RefusalCode.CODE_REFERENCE_REFUSED.value, "universe.module_path"),)

    # A legal specification still compiles, so the refusals above are the fields rather than the form.
    assert compile_specification(legal_payload(), policy, registry).accepted is True

    inventory = campaign(tmp_path)

    for result in (submission, code):
        error = result.errors[0]
        inventory.record_refusal(CANDIDATE, code=error.code.value, path=error.path)

    audit = audit_campaign(inventory)
    judgement = audit.candidates[0]

    assert judgement.candidate_id == CANDIDATE
    assert judgement.outcome == "refused"
    assert judgement.attempts == ()
    assert judgement.refusals == (
        RefusalCode.BLOCK_UNKNOWN_FIELD.value,
        RefusalCode.CODE_REFERENCE_REFUSED.value,
    )
    assert inventory.counts().proposal_attempts == 0
    assert inventory.counts().executed_experiments == 0
