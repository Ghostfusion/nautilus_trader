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
The discovery command front end: one configuration path in, one JSON document out.

`main` is the command-line row of the implementation plan's layout. It reads the configuration
document `config.py` owns, resolves the policy document that configuration declares, and writes one
machine-readable JSON document to standard output. On success the document reports the campaign, the
identity of the resolved policy, the search declaration and the worker declaration, which is what a
campaign is bound to before anything runs. On failure it reports `status` as `error` with the type
of the failure and, when the loader refused the document, the code from `ConfigRefusalCode` it
refused with and the path it reached that code at. The code travels in the document rather than only
inside the message, because a code is what a caller groups, counts and records refusals by.

The front end loads and reports. It runs no campaign, holds no engine, measures no environment and
reaches no network, so reading the document tells a caller the configuration is admissible without a
run having happened. Three exit codes match `optimize`: 2 for an invocation that is not one
configuration path, 1 for a document the loader refused, and 0 for a configuration that was
reported.

The credential scan here is the loader's own, so it refuses a credential by the shape of a secret
rather than by a list of variable names: the canonical list lives in the repository and a front end
has no document that carries it, and a caller that has the names passes them to
`config.load_config` directly. The ambient environment is likewise not read, because the design
keeps that assertion in the boundary fixture rather than in the front end.

Console logging is not bypassed the way `optimization/config.py` bypasses it, because importing the
discovery package installs no handler: the document is the only thing written to standard output
without a guard.

The front end is reached as `python -m nautilus_trader.discovery.cli CONFIG.json`, and it is
deliberately outside the package facade, as `optimize`'s `main` is outside `optimization`'s: a
command entry point is reached as a command, and the facade is the library surface.
"""

from __future__ import annotations

import json
import sys

from nautilus_trader.discovery.config import ConfigError
from nautilus_trader.discovery.config import DiscoveryConfig
from nautilus_trader.discovery.config import load_config


# The schema identifier of the document the front end writes.
OUTPUT_SCHEMA = "nautilus.discovery.cli/v1"

# The command every document names, which is the layer the front end is the front of.
COMMAND = "discovery"

# The one invocation the front end accepts.
_USAGE = "Usage: python -m nautilus_trader.discovery.cli CONFIG.json"


class UsageError(ValueError):
    """
    A command line that is not exactly one configuration path.

    The message is the usage text. The type is named in the document's `error_type` field, so a
    caller reads a wrong invocation from a refused document without parsing prose.
    """


def main(argv: list[str] | None = None) -> int:
    """
    Report the configuration document a command line names.

    Failures are reported in the document and through the return code, as `optimize` reports them,
    so a caller distinguishes a refused invocation, a refused document and a report.

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
        _emit(_error_document(UsageError(_USAGE)))
        return 2
    try:
        config = load_config(arguments[0])
    except ConfigError as e:
        _emit(_error_document(e))
        return 1
    except Exception as e:  # noqa: BLE001 (a load failure is reported, never raised)
        _emit(_error_document(e))
        return 1
    _emit(_configuration_document(config))
    return 0


def _configuration_document(config: DiscoveryConfig) -> dict[str, object]:
    """
    Encode a loaded configuration as the report document.

    The policy block carries the identity of the policy the loader read. Those three fields are null
    when the configuration was parsed without reading the document it declares, which is what
    `config.parse_config` produces and `config.load_config`, which the front end calls, does not.
    """
    policy = config.resolved_policy
    search = config.search
    workers = config.workers
    concurrency = workers.concurrency
    return {
        "schema": OUTPUT_SCHEMA,
        "command": COMMAND,
        "status": "ok",
        "campaign": {
            "label": config.campaign.label,
            "directory": config.campaign.directory,
            "writer": config.campaign.writer,
            "family_id": config.campaign.family_id,
            "seed": config.campaign.seed,
        },
        "policy": {
            "path": config.policy.path,
            "declared_digest": config.policy.digest,
            "policy_id": None if policy is None else policy.policy_id,
            "version": None if policy is None else policy.version,
            "digest": None if policy is None else policy.digest(),
        },
        "search": {
            "population_size": search.population_size,
            "elite": search.elite,
            "family_floor": search.family_floor,
            "mutation_rate": search.mutation_rate,
            "exploration_share": search.exploration_share,
            "family_allocation": [list(pair) for pair in search.family_allocation],
        },
        "workers": {
            "capability_allowlist": list(workers.capability_allowlist),
            "concurrency": {
                "per_run_bytes": concurrency.per_run_bytes,
                "memory_fraction": concurrency.memory_fraction,
                "max_workers": concurrency.max_workers,
            },
        },
    }


def _error_document(exc: Exception) -> dict[str, object]:
    """
    Encode a failure as a report document.

    A refusal carries the code and the path the loader reached them at, and the code is the value of
    `ConfigRefusalCode` rather than a second spelling of it.
    """
    document: dict[str, object] = {
        "schema": OUTPUT_SCHEMA,
        "command": COMMAND,
        "status": "error",
        "error_type": type(exc).__name__,
        "error_message": str(exc),
    }
    if isinstance(exc, ConfigError):
        document["error_code"] = exc.code.value
        if exc.path:
            document["error_path"] = exc.path
    return document


def _emit(document: dict[str, object]) -> None:
    """
    Write one strict JSON document to standard output.
    """
    sys.stdout.write(json.dumps(document, sort_keys=True, ensure_ascii=True, allow_nan=False))
    sys.stdout.write("\n")


if __name__ == "__main__":
    sys.exit(main())
