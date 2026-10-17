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
The discovery campaign configuration: one declared campaign, loadable from JSON.

This module owns the configuration document, its schema, and the two checks the design places on
it. It composes the existing typed surfaces rather than replacing them: a document is a view of
`DiscoveryPolicy`, `ConcurrencyPolicy`, `SearchPolicy` and `Evolution`, and a caller builds those
objects from the declarations a parse produces.

The document is JSON rather than YAML because it follows the convention of `optimization/config.py`
and, through it, `crates/system/src/config_file.rs`: typed constructors remain the canonical API, a
file is a view of a typed configuration, and an unknown key is refused rather than ignored.

The document declares:

- `campaign`: the label, the directory the evidence and the inventory live under, the one writer
  that claims the inventory, the research family the campaign descends from, and its seed.
- `policy`: the path of the pre-registered policy document, and optionally the digest it must have.
- `search`: the declared search policy and the population the evolution starts from.
- `workers`: the capability allowlist a worker environment is measured against, and the
  memory-driven concurrency policy.

Two sentences of the design are discharged here rather than asserted. Section 12's environment
assertion asks that "the configuration loader refuses a credential in a policy file", so every
string in the policy document and in the configuration document itself is scanned for a credential,
and a finding refuses the load naming the location and the shape rather than the value. The
boundary's stronger control is that "Discovery workers run in an environment with an explicit
capability allowlist", so the allowlist is declared here, an entry naming a live adapter is refused
outright, and `check_worker_capabilities` measures the packages a worker holds against it.

Nothing here runs a campaign, holds an engine, reads the environment or reaches the network. The
module imports without side effects, so the package stays collectable by the test-collection hook,
and the command front end is `cli.py` in the same layout.
"""

from __future__ import annotations

import json
import re
from collections.abc import Mapping
from dataclasses import dataclass
from dataclasses import field
from dataclasses import replace
from enum import Enum
from pathlib import Path
from typing import TYPE_CHECKING

from nautilus_trader.discovery.policy import DECLARED_SET_NAMES
from nautilus_trader.discovery.policy import ComplexityLimits
from nautilus_trader.discovery.policy import DiscoveryPolicy
from nautilus_trader.discovery.policy import Reference
from nautilus_trader.discovery.policy import ReferenceKind
from nautilus_trader.discovery.schema import Bound
from nautilus_trader.discovery.schema import BoundKind
from nautilus_trader.discovery.schema import decimal_of
from nautilus_trader.optimization.concurrency import ConcurrencyPolicy


if TYPE_CHECKING:
    from collections.abc import Iterable
    from collections.abc import Sequence
    from decimal import Decimal


# The schema identifier of the configuration document.
CONFIG_SCHEMA = "nautilus.discovery.config/v1"

# The package prefix a capability allowlist may never name, because the boundary removes it: a
# worker whose environment imports an adapter is not the environment the design describes
ADAPTER_PREFIX = "nautilus_trader.adapters"

# The number of entries one policy sector pair holds, which is an instrument id and a sector
SECTOR_PAIR_LENGTH = 2

# The last segment a name carries when it names a credential rather than a declared value
_CREDENTIAL_ENDINGS = frozenset(
    {
        "ADDRESS",
        "API_KEY",
        "CREDENTIAL",
        "CREDENTIALS",
        "ID",
        "INDEX",
        "KEY",
        "PASSPHRASE",
        "PASSWORD",
        "PK",
        "SECRET",
        "TOKEN",
        "USERNAME",
    },
)

# The shapes a credential value takes, as the vendors issue them rather than as a list names them,
# and a bare hex string long enough to be a key with the algorithm prefix stripped from it
_CREDENTIAL_VALUES = (
    re.compile(r"^(sk|pk)_(live|test)_[A-Za-z0-9]{16,}$"),
    re.compile(r"^sk-[A-Za-z0-9_-]{16,}$"),
    re.compile(r"^(AKIA|ASIA)[A-Z0-9]{12,}$"),
    re.compile(r"^gh[pousr]_[A-Za-z0-9]{20,}$"),
    re.compile(r"^xox[baprs]-[A-Za-z0-9-]{10,}$"),
    re.compile(r"^eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\."),
    re.compile(r"^-----BEGIN [A-Z ]+-----"),
    re.compile(r"^[0-9a-fA-F]{32,}$"),
)

# The shape a credential variable name takes, which is what the canonical list's entries are
_CREDENTIAL_NAME = re.compile(r"^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+$")

# The shape a package name takes, which is what a capability allowlist entry must be
_PACKAGE_NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$")

_TOP_LEVEL_KEYS = frozenset({"schema", "campaign", "policy", "search", "workers"})

_CAMPAIGN_KEYS = frozenset({"label", "directory", "writer", "family_id", "seed"})

_POLICY_DECLARATION_KEYS = frozenset({"path", "digest"})

_SEARCH_KEYS = frozenset(
    {
        "population_size",
        "elite",
        "family_floor",
        "mutation_rate",
        "exploration_share",
        "family_allocation",
    },
)

_WORKER_KEYS = frozenset({"capability_allowlist", "concurrency"})

_CONCURRENCY_KEYS = frozenset({"per_run_bytes", "memory_fraction", "max_workers"})

# The keys of the policy document, which are exactly the keys `DiscoveryPolicy.to_dict()` emits: the
# fifteen declared sets come from the policy's own declaration rather than a second copy of them
_POLICY_KEYS = frozenset(
    {
        "policy_id",
        "version",
        "feature_allowlist",
        "bounds",
        "limits",
        "references",
        "sectors",
    },
) | frozenset(DECLARED_SET_NAMES)

_BOUND_KEYS = frozenset({"name", "kind", "lower", "upper", "choices"})

_LIMIT_KEYS = frozenset({"max_conditions", "max_condition_depth", "max_features"})

_REFERENCE_KEYS = frozenset({"name", "kind", "digest"})


class ConfigRefusalCode(Enum):
    """
    The closed vocabulary a configuration document is refused with.
    """

    DOCUMENT_NOT_READABLE = "document_not_readable"
    DOCUMENT_NOT_JSON = "document_not_json"
    NOT_A_MAPPING = "not_a_mapping"
    SCHEMA_MISMATCH = "schema_mismatch"
    UNKNOWN_FIELD = "unknown_field"
    FIELD_MISSING = "field_missing"
    FIELD_TYPE = "field_type"
    FIELD_EMPTY = "field_empty"
    VALUE_REPEATED = "value_repeated"
    VALUE_OUT_OF_RANGE = "value_out_of_range"
    NUMBER_NOT_FINITE = "number_not_finite"
    CREDENTIAL_REFUSED = "credential_refused"
    POLICY_REFUSED = "policy_refused"
    POLICY_DIGEST_MISMATCH = "policy_digest_mismatch"
    ALLOWLIST_ENTRY_INVALID = "allowlist_entry_invalid"
    ALLOWLIST_ENTRY_NAMES_ADAPTER = "allowlist_entry_names_adapter"


class ConfigError(ValueError):
    """
    A configuration document is invalid, or names something it must not.

    Parameters
    ----------
    code : ConfigRefusalCode
        The refusal's code, from the closed vocabulary.
    message : str
        What was refused, in the vocabulary the coding standards require.
    path : str, default ""
        The dotted location in the document the refusal is about, empty when it is about the
        document as a whole.

    """

    def __init__(self, code: ConfigRefusalCode, message: str, *, path: str = "") -> None:
        """
        Initialize the refusal.
        """
        super().__init__(f"{code.value}: {message}")
        self.code = code
        self.path = path


class CredentialKind(Enum):
    """
    The shape a credential was found in, which is reported instead of the credential.
    """

    NAMED = "named"
    VARIABLE_NAME = "variable_name"
    VALUE = "value"


@dataclass(frozen=True)
class CredentialFinding:
    """
    One credential found in a document, named by where it is rather than by what it is.

    The value is never carried here. A refusal that echoed a credential would put it in a log, a
    terminal and a test failure, which is the outcome the scan exists to prevent.

    Parameters
    ----------
    path : str
        The dotted location in the document the string was found at.
    kind : CredentialKind
        The shape the credential was recognised by.

    """

    path: str
    kind: CredentialKind

    def to_dict(self) -> dict[str, object]:
        """
        Return the finding as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {"path": self.path, "kind": self.kind.value}


@dataclass(frozen=True)
class WorkerCapabilityCheck:
    """
    What a worker environment holds that its capability allowlist does not admit.

    The check is the design's "capability check that a worker environment contains only allowlisted
    packages". It is not `custody.py`'s `WorkerEnvironmentCheck`, which asks whether an environment
    names the confirmation set's location; this one asks what packages an environment holds.

    Parameters
    ----------
    worker : str
        The worker the environment was measured for.
    allowlist : tuple[str, ...]
        The capability allowlist the environment was measured against, sorted.
    unexpected : tuple[str, ...]
        The packages held that the allowlist does not admit, sorted, empty when it admits them all.

    """

    worker: str
    allowlist: tuple[str, ...]
    unexpected: tuple[str, ...] = ()

    @property
    def holds(self) -> bool:
        """
        Whether the environment holds only allowlisted packages.
        """
        return not self.unexpected

    def to_dict(self) -> dict[str, object]:
        """
        Return the check as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "worker": self.worker,
            "allowlist": list(self.allowlist),
            "unexpected": list(self.unexpected),
            "holds": self.holds,
        }


@dataclass(frozen=True)
class CampaignDeclaration:
    """
    The campaign a configuration opens: where its evidence lives and what identifies it.

    Parameters
    ----------
    label : str
        The campaign label, which is the inventory's campaign id.
    directory : str
        The directory the inventory and the evidence live under. It is resolved against the
        directory the configuration document was read from.
    writer : str
        The one writer that claims the inventory, which a second writer is refused by.
    family_id : str, default ""
        The research family the campaign descends from, empty when it descends from none.
    seed : int, default 0
        The campaign's declared seed, which fixes the numbers a run draws.

    """

    label: str
    directory: str
    writer: str
    family_id: str = ""
    seed: int = 0


@dataclass(frozen=True)
class PolicyDeclaration:
    """
    The pre-registered policy document a campaign runs under.

    Parameters
    ----------
    path : str
        The path of the policy document, resolved against the directory the configuration document
        was read from.
    digest : str, default ""
        The digest the policy document must have, or empty when the declaration pins none.

    """

    path: str
    digest: str = ""


@dataclass(frozen=True)
class SearchDeclaration:
    """
    The search policy and the population the evolution starts from.

    These are the declared inputs of `evolve.Evolution`, which is what a caller builds from them.
    The prompt texts are not declared here: they are code in `discovery/prompts/`, and a document
    that could restate them would be a second copy of them.

    Parameters
    ----------
    population_size : int, default 12
        The number of genomes a generation holds.
    elite : int, default 2
        The number of genomes carried over unchanged.
    family_floor : int, default 2
        The smallest number of genomes a family keeps.
    mutation_rate : float, default 0.5
        The share of a generation produced by mutation.
    exploration_share : float, default 0.25
        The share of the generation drawn from outside the current front.
    family_allocation : tuple[tuple[str, float], ...], default ()
        The share of the population each family is allocated, in declared order.

    """

    population_size: int = 12
    elite: int = 2
    family_floor: int = 2
    mutation_rate: float = 0.5
    exploration_share: float = 0.25
    family_allocation: tuple[tuple[str, float], ...] = ()


@dataclass(frozen=True)
class WorkerDeclaration:
    """
    The shape of a worker environment and the memory-driven concurrency behind it.

    Parameters
    ----------
    capability_allowlist : tuple[str, ...], default ()
        The packages a worker environment may hold. An empty allowlist admits no package, which is
        the strictest reading of an explicit allowlist rather than a permissive default.
    concurrency : ConcurrencyPolicy
        The concurrency policy the workers are sized by.

    """

    capability_allowlist: tuple[str, ...] = ()
    concurrency: ConcurrencyPolicy = field(default_factory=ConcurrencyPolicy)


@dataclass(frozen=True)
class DiscoveryConfig:
    """
    The declarations one discovery configuration document holds.

    Parameters
    ----------
    campaign : CampaignDeclaration
        The campaign the configuration opens.
    policy : PolicyDeclaration
        The policy document the campaign runs under.
    search : SearchDeclaration
        The declared search policy and population.
    workers : WorkerDeclaration
        The worker environment's capability allowlist and the concurrency policy.
    resolved_policy : DiscoveryPolicy, optional
        The policy the loader read from the document the declaration names, or None when the
        document was parsed without reading it.

    """

    campaign: CampaignDeclaration
    policy: PolicyDeclaration
    search: SearchDeclaration = field(default_factory=SearchDeclaration)
    workers: WorkerDeclaration = field(default_factory=WorkerDeclaration)
    resolved_policy: DiscoveryPolicy | None = None


def scan_for_credentials(
    payload: object,
    *,
    path: str = "",
    names: Iterable[str] = (),
) -> tuple[CredentialFinding, ...]:
    """
    Scan a decoded document for a credential, wherever in it the credential sits.

    A string is a credential when it names one of the variables the caller passes, when it takes the
    shape of a credential variable name, or when it takes the shape of a credential value. Keys and
    values are both scanned, at every depth, because a credential in a key and a credential in a
    value are the same defect.

    Parameters
    ----------
    payload : object
        The decoded document.
    path : str, default ""
        The dotted path the document sits at, used to qualify every finding.
    names : Iterable[str], default ()
        The credential variable names to refuse in addition to the shapes, such as the entries of
        `scripts/strip-adapter-env.bash`.

    Returns
    -------
    tuple[CredentialFinding, ...]
        Every finding, in document order.

    """
    known = {(name if isinstance(name, str) else str(name)).upper() for name in names}
    return tuple(_scan(payload, path, known))


def check_worker_capabilities(
    worker: str,
    packages: Iterable[str],
    allowlist: Iterable[str],
) -> WorkerCapabilityCheck:
    """
    Measure the packages a worker environment holds against a capability allowlist.

    A package is admitted when it is an allowlist entry or a module below one, so an allowlist
    naming `nautilus_trader.discovery` admits `nautilus_trader.discovery.config`. Nothing else is
    admitted, and an empty allowlist admits nothing.

    Parameters
    ----------
    worker : str
        The worker the environment belongs to.
    packages : Iterable[str]
        The packages the environment holds.
    allowlist : Iterable[str]
        The declared capability allowlist.

    Returns
    -------
    WorkerCapabilityCheck

    """
    admitted = tuple(sorted({str(entry) for entry in allowlist}))
    unexpected = tuple(
        sorted(
            {str(package) for package in packages if not _admitted(str(package), admitted)},
        ),
    )
    return WorkerCapabilityCheck(worker=worker, allowlist=admitted, unexpected=unexpected)


def parse_config(payload: object, *, credentials: Iterable[str] = ()) -> DiscoveryConfig:
    """
    Parse an already-decoded configuration document.

    Parameters
    ----------
    payload : object
        The decoded JSON document.
    credentials : Iterable[str], default ()
        The credential variable names to refuse in addition to the shapes.

    Returns
    -------
    DiscoveryConfig

    Raises
    ------
    ConfigError
        If the document is not a strict object, is missing a key, states an unknown key, carries a
        credential, or declares a value the schema does not admit.

    """
    document = _require_mapping(payload, "config")
    _reject_unknown(document, _TOP_LEVEL_KEYS, "config")
    _reject_schema(document)
    _refuse_credentials(document, "config", credentials)
    return DiscoveryConfig(
        campaign=_parse_campaign(_require_key(document, "campaign", "config")),
        policy=_parse_policy_declaration(_require_key(document, "policy", "config")),
        search=_parse_search(document.get("search")),
        workers=_parse_workers(document.get("workers")),
    )


def load_config(path: str | Path, *, credentials: Iterable[str] = ()) -> DiscoveryConfig:
    """
    Load the configuration document at `path`, and the policy document it names.

    The directory the document was read from is the directory its declared paths are resolved
    against, so a configuration and the policy beside it are one unit rather than two paths that
    happen to agree.

    Parameters
    ----------
    path : str | Path
        The path of the configuration document.
    credentials : Iterable[str], default ()
        The credential variable names to refuse in addition to the shapes.

    Returns
    -------
    DiscoveryConfig
        The declarations, with `resolved_policy` holding the policy the loader read.

    Raises
    ------
    ConfigError
        If either document cannot be read, is not JSON, carries a credential, or declares something
        the schema does not admit.

    """
    source = Path(path)
    config = parse_config(_read_json(source, "config"), credentials=credentials)
    resolved = load_policy(config.policy, directory=source.parent, credentials=credentials)
    return replace(config, resolved_policy=resolved)


def load_policy(
    declaration: PolicyDeclaration,
    *,
    directory: str | Path = ".",
    credentials: Iterable[str] = (),
) -> DiscoveryPolicy:
    """
    Load the policy document a declaration names and check it against the declaration's pin.

    Parameters
    ----------
    declaration : PolicyDeclaration
        The declared policy document.
    directory : str | Path, default "."
        The directory a relative path is resolved against.
    credentials : Iterable[str], default ()
        The credential variable names to refuse in addition to the shapes.

    Returns
    -------
    DiscoveryPolicy

    Raises
    ------
    ConfigError
        If the document cannot be read, is not JSON, carries a credential, declares something the
        policy refuses, or does not have the digest the declaration pins.

    """
    path = Path(declaration.path)
    source = path if path.is_absolute() else Path(directory) / path
    payload = _read_json(source, "policy", "policy.path")
    _refuse_credentials(payload, "policy", credentials)
    policy = parse_policy(payload)
    if declaration.digest and declaration.digest != policy.digest():
        raise ConfigError(
            ConfigRefusalCode.POLICY_DIGEST_MISMATCH,
            f"the declaration pins {declaration.digest} and the document has {policy.digest()}",
            path="policy.digest",
        )
    return policy


def parse_policy(payload: object) -> DiscoveryPolicy:
    """
    Parse an already-decoded policy document into the policy object.

    The document is exactly what `DiscoveryPolicy.to_dict()` emits, and every key it emits is
    required, so a parse of a written policy is the policy it was written from. Nothing is
    defaulted here: a policy whose `policy_id` were defaulted would digest differently from the
    policy it was written from, and the digest is what names the declarations a result ran under.

    Parameters
    ----------
    payload : object
        The decoded JSON document.

    Returns
    -------
    DiscoveryPolicy

    Raises
    ------
    ConfigError
        If the document is missing a key, states an unknown key, or declares a value the policy
        object refuses.

    """
    document = _require_mapping(payload, "policy")
    _reject_unknown(document, _POLICY_KEYS, "policy")
    declared = {name: _parse_string_set(document, name, "policy") for name in DECLARED_SET_NAMES}
    try:
        return DiscoveryPolicy(
            **declared,
            policy_id=_require_str(document, "policy_id", "policy"),
            version=_require_int(document, "version", "policy"),
            feature_allowlist=_parse_string_set(document, "feature_allowlist", "policy"),
            bounds=_parse_bounds(_require_key(document, "bounds", "policy")),
            limits=_parse_limits(_require_key(document, "limits", "policy")),
            references=_parse_references(_require_key(document, "references", "policy")),
            sectors=_parse_sectors(_require_key(document, "sectors", "policy")),
        )
    except (TypeError, ValueError) as e:
        raise ConfigError(
            ConfigRefusalCode.POLICY_REFUSED,
            f"the policy refused it: {e}",
        ) from e


def _scan(payload: object, path: str, names: frozenset[str]) -> list[CredentialFinding]:
    # Report a credential at the location it was found at rather than by its value
    findings: list[CredentialFinding] = []
    if isinstance(payload, Mapping):
        for key, value in payload.items():
            key_path = f"{path}.{key}" if path else str(key)
            if isinstance(key, str):
                kind = _credential_kind(key, names)
                if kind is not None:
                    findings.append(CredentialFinding(path=key_path, kind=kind))
            findings.extend(_scan(value, key_path, names))
        return findings
    if isinstance(payload, list | tuple):
        for index, item in enumerate(payload):
            findings.extend(_scan(item, f"{path}[{index}]", names))
        return findings
    if isinstance(payload, str):
        kind = _credential_kind(payload, names)
        if kind is not None:
            findings.append(CredentialFinding(path=path, kind=kind))
    return findings


def _credential_kind(value: str, names: frozenset[str]) -> CredentialKind | None:
    # Recognise a credential by name first, because a name is the stronger statement of the two
    if value.upper() in names:
        return CredentialKind.NAMED
    if _CREDENTIAL_NAME.match(value) and value.rsplit("_", maxsplit=1)[-1] in _CREDENTIAL_ENDINGS:
        return CredentialKind.VARIABLE_NAME
    if any(pattern.match(value) for pattern in _CREDENTIAL_VALUES):
        return CredentialKind.VALUE
    return None


def _refuse_credentials(payload: object, where: str, names: Iterable[str]) -> None:
    findings = scan_for_credentials(payload, path=where, names=names)
    if not findings:
        return
    first = findings[0]
    total = len(findings)
    raises = "finding" if total == 1 else "findings"
    raise ConfigError(
        ConfigRefusalCode.CREDENTIAL_REFUSED,
        f"{first.path} carries a credential ({first.kind.value}), and the value is not repeated "
        f"here; {total} {raises} in the {where} document",
        path=first.path,
    )


def _admitted(package: str, allowlist: tuple[str, ...]) -> bool:
    return package in allowlist or any(package.startswith(f"{entry}.") for entry in allowlist)


def _read_json(path: Path, where: str, location: str = "") -> object:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as e:
        raise ConfigError(
            ConfigRefusalCode.DOCUMENT_NOT_READABLE,
            f"the {where} document at {path} could not be read: {e}",
            path=location,
        ) from e
    try:
        return json.loads(text, parse_constant=_reject_constant)
    except json.JSONDecodeError as e:
        raise ConfigError(
            ConfigRefusalCode.DOCUMENT_NOT_JSON,
            f"the {where} document at {path} is not valid JSON: {e}",
            path=location,
        ) from e


def _reject_constant(name: str) -> float:
    # Reject the non-finite JSON constants that Python accepts by default
    raise ConfigError(
        ConfigRefusalCode.NUMBER_NOT_FINITE,
        f"the document states the non-finite number {name}",
    )


def _reject_schema(document: Mapping[str, object]) -> None:
    schema = document.get("schema")
    if schema is None or schema == CONFIG_SCHEMA:
        return
    raise ConfigError(
        ConfigRefusalCode.SCHEMA_MISMATCH,
        f"config.schema must be {CONFIG_SCHEMA!r}, was {schema!r}",
        path="schema",
    )


def _parse_campaign(payload: object) -> CampaignDeclaration:
    mapping = _require_mapping(payload, "campaign")
    _reject_unknown(mapping, _CAMPAIGN_KEYS, "campaign")
    return CampaignDeclaration(
        label=_require_str(mapping, "label", "campaign"),
        directory=_require_str(mapping, "directory", "campaign"),
        writer=_require_str(mapping, "writer", "campaign"),
        family_id=_optional_str(mapping.get("family_id"), "campaign.family_id"),
        seed=_require_int(mapping, "seed", "campaign"),
    )


def _parse_policy_declaration(payload: object) -> PolicyDeclaration:
    mapping = _require_mapping(payload, "policy")
    _reject_unknown(mapping, _POLICY_DECLARATION_KEYS, "policy")
    return PolicyDeclaration(
        path=_require_str(mapping, "path", "policy"),
        digest=_optional_str(mapping.get("digest"), "policy.digest"),
    )


def _parse_search(payload: object) -> SearchDeclaration:
    if payload is None:
        return SearchDeclaration()
    mapping = _require_mapping(payload, "search")
    _reject_unknown(mapping, _SEARCH_KEYS, "search")

    population_size = _optional_int(mapping.get("population_size"), "search.population_size")
    elite = _optional_int(mapping.get("elite"), "search.elite")
    family_floor = _optional_int(mapping.get("family_floor"), "search.family_floor")
    default = SearchDeclaration()
    size = default.population_size if population_size is None else population_size
    elites = default.elite if elite is None else elite
    floor_count = default.family_floor if family_floor is None else family_floor

    if size < 1:
        raise ConfigError(
            ConfigRefusalCode.VALUE_OUT_OF_RANGE,
            f"search.population_size must be positive, was {size}",
            path="search.population_size",
        )
    if elites < 0 or elites > size:
        raise ConfigError(
            ConfigRefusalCode.VALUE_OUT_OF_RANGE,
            f"search.elite must be between 0 and the population size {size}, was {elites}",
            path="search.elite",
        )
    if floor_count < 0:
        raise ConfigError(
            ConfigRefusalCode.VALUE_OUT_OF_RANGE,
            f"search.family_floor must not be negative, was {floor_count}",
            path="search.family_floor",
        )

    return SearchDeclaration(
        population_size=size,
        elite=elites,
        family_floor=floor_count,
        mutation_rate=_optional_rate(
            mapping.get("mutation_rate"),
            "search.mutation_rate",
            default.mutation_rate,
        ),
        exploration_share=_optional_rate(
            mapping.get("exploration_share"),
            "search.exploration_share",
            default.exploration_share,
        ),
        family_allocation=_parse_allocation(mapping.get("family_allocation")),
    )


def _parse_allocation(payload: object) -> tuple[tuple[str, float], ...]:
    if payload is None:
        return ()
    mapping = _require_mapping(payload, "search.family_allocation")
    allocation = []
    for family in sorted(mapping):
        share = mapping[family]
        where = f"search.family_allocation.{family}"
        if not family:
            raise ConfigError(
                ConfigRefusalCode.FIELD_EMPTY,
                f"{where} must name a family",
                path=where,
            )
        if isinstance(share, bool) or not isinstance(share, int | float):
            raise ConfigError(
                ConfigRefusalCode.FIELD_TYPE,
                f"{where} must be a number, was {_type_name(share)}",
                path=where,
            )
        if share < 0 or share > 1:
            raise ConfigError(
                ConfigRefusalCode.VALUE_OUT_OF_RANGE,
                f"{where} must be between 0 and 1, was {share}",
                path=where,
            )
        allocation.append((family, float(share)))
    return tuple(allocation)


def _parse_workers(payload: object) -> WorkerDeclaration:
    if payload is None:
        return WorkerDeclaration()
    mapping = _require_mapping(payload, "workers")
    _reject_unknown(mapping, _WORKER_KEYS, "workers")
    return WorkerDeclaration(
        capability_allowlist=_parse_allowlist(
            _require_key(mapping, "capability_allowlist", "workers"),
        ),
        concurrency=_parse_concurrency(mapping.get("concurrency")),
    )


def _parse_allowlist(payload: object) -> tuple[str, ...]:
    entries = _require_array(payload, "workers.capability_allowlist")
    packages = []
    for index, entry in enumerate(entries):
        where = f"workers.capability_allowlist[{index}]"
        if not isinstance(entry, str):
            raise ConfigError(
                ConfigRefusalCode.FIELD_TYPE,
                f"{where} must be a string, was {_type_name(entry)}",
                path=where,
            )
        if not entry:
            raise ConfigError(
                ConfigRefusalCode.FIELD_EMPTY,
                f"{where} must not be empty",
                path=where,
            )
        if not _PACKAGE_NAME.match(entry):
            raise ConfigError(
                ConfigRefusalCode.ALLOWLIST_ENTRY_INVALID,
                f"{where} must be a package name, was {entry!r}",
                path=where,
            )
        if entry == ADAPTER_PREFIX or entry.startswith(f"{ADAPTER_PREFIX}."):
            raise ConfigError(
                ConfigRefusalCode.ALLOWLIST_ENTRY_NAMES_ADAPTER,
                f"{where} names a live adapter, and the boundary removes every adapter from a "
                f"worker environment",
                path=where,
            )
        packages.append(entry)
    if len(set(packages)) != len(packages):
        raise ConfigError(
            ConfigRefusalCode.VALUE_REPEATED,
            "workers.capability_allowlist states an entry twice",
            path="workers.capability_allowlist",
        )
    return tuple(packages)


def _parse_concurrency(payload: object) -> ConcurrencyPolicy:
    if payload is None:
        return ConcurrencyPolicy()
    mapping = _require_mapping(payload, "workers.concurrency")
    _reject_unknown(mapping, _CONCURRENCY_KEYS, "workers.concurrency")

    default = ConcurrencyPolicy()
    per_run_bytes = _optional_int(
        mapping.get("per_run_bytes"),
        "workers.concurrency.per_run_bytes",
    )
    max_workers = _optional_int(mapping.get("max_workers"), "workers.concurrency.max_workers")
    memory_fraction = mapping.get("memory_fraction")
    if memory_fraction is not None and (
        isinstance(memory_fraction, bool) or not isinstance(memory_fraction, int | float)
    ):
        raise ConfigError(
            ConfigRefusalCode.FIELD_TYPE,
            f"workers.concurrency.memory_fraction must be a number, was "
            f"{type(memory_fraction).__name__}",
            path="workers.concurrency.memory_fraction",
        )
    if max_workers is not None and max_workers < 1:
        raise ConfigError(
            ConfigRefusalCode.VALUE_OUT_OF_RANGE,
            f"workers.concurrency.max_workers must be positive, was {max_workers}",
            path="workers.concurrency.max_workers",
        )
    return ConcurrencyPolicy(
        per_run_bytes=default.per_run_bytes if per_run_bytes is None else per_run_bytes,
        memory_fraction=(
            default.memory_fraction if memory_fraction is None else float(memory_fraction)
        ),
        max_workers=max_workers,
    )


def _parse_bounds(payload: object) -> tuple[Bound, ...]:
    entries = _require_array(payload, "policy.bounds")
    bounds = []
    for index, entry in enumerate(entries):
        where = f"policy.bounds[{index}]"
        mapping = _require_mapping(entry, where)
        _reject_unknown(mapping, _BOUND_KEYS, where)
        kind = _require_one_of(mapping, "kind", where, {k.value for k in BoundKind})
        bounds.append(
            Bound(
                name=_require_str(mapping, "name", where),
                kind=BoundKind(kind),
                lower=_optional_decimal(_require_key(mapping, "lower", where), f"{where}.lower"),
                upper=_optional_decimal(_require_key(mapping, "upper", where), f"{where}.upper"),
                choices=tuple(
                    _parse_decimals(_require_key(mapping, "choices", where), f"{where}.choices"),
                ),
            ),
        )
    if len(bounds) != len({bound.name for bound in bounds}):
        raise ConfigError(
            ConfigRefusalCode.VALUE_REPEATED,
            "policy.bounds names a bound twice",
            path="policy.bounds",
        )
    return tuple(bounds)


def _parse_limits(payload: object) -> ComplexityLimits:
    mapping = _require_mapping(payload, "policy.limits")
    _reject_unknown(mapping, _LIMIT_KEYS, "policy.limits")
    return ComplexityLimits(
        max_conditions=_require_int(mapping, "max_conditions", "policy.limits"),
        max_condition_depth=_require_int(mapping, "max_condition_depth", "policy.limits"),
        max_features=_require_int(mapping, "max_features", "policy.limits"),
    )


def _parse_references(payload: object) -> tuple[Reference, ...]:
    entries = _require_array(payload, "policy.references")
    references = []
    for index, entry in enumerate(entries):
        where = f"policy.references[{index}]"
        mapping = _require_mapping(entry, where)
        _reject_unknown(mapping, _REFERENCE_KEYS, where)
        kind = _require_one_of(mapping, "kind", where, {k.value for k in ReferenceKind})
        references.append(
            Reference(
                name=_require_str(mapping, "name", where),
                kind=ReferenceKind(kind),
                digest=_require_str(mapping, "digest", where),
            ),
        )
    return tuple(references)


def _parse_sectors(payload: object) -> tuple[tuple[str, str], ...]:
    entries = _require_array(payload, "policy.sectors")
    sectors = []
    for index, entry in enumerate(entries):
        where = f"policy.sectors[{index}]"
        pair = _require_array(entry, where)
        if len(pair) != SECTOR_PAIR_LENGTH:
            raise ConfigError(
                ConfigRefusalCode.FIELD_TYPE,
                f"{where} must be a pair of an instrument id and a sector, was {len(pair)} items",
                path=where,
            )
        instrument_id = _require_non_empty(pair[0], f"{where}[0]")
        sector = _require_non_empty(pair[1], f"{where}[1]")
        sectors.append((instrument_id, sector))
    return tuple(sectors)


def _parse_string_set(document: Mapping[str, object], name: str, where: str) -> frozenset[str]:
    members = _require_array(_require_key(document, name, where), f"{where}.{name}")
    names = []
    for index, member in enumerate(members):
        names.append(_require_non_empty(member, f"{where}.{name}[{index}]"))
    if len(set(names)) != len(names):
        raise ConfigError(
            ConfigRefusalCode.VALUE_REPEATED,
            f"{where}.{name} states a member twice",
            path=f"{where}.{name}",
        )
    return frozenset(names)


def _parse_decimals(payload: object, where: str) -> list[Decimal]:
    entries = _require_array(payload, where)
    decimals = []
    for index, entry in enumerate(entries):
        decimals.append(_require_decimal(entry, f"{where}[{index}]"))
    return decimals


def _require_mapping(payload: object, where: str) -> Mapping[str, object]:
    if not isinstance(payload, Mapping):
        raise ConfigError(
            ConfigRefusalCode.NOT_A_MAPPING,
            f"{where} must be a JSON object, was {_type_name(payload)}",
            path=where,
        )
    for key in payload:
        if not isinstance(key, str):
            raise ConfigError(
                ConfigRefusalCode.FIELD_TYPE,
                f"{where} states a key that is not a string, was {_type_name(key)}",
                path=where,
            )
    return payload


def _require_key(document: Mapping[str, object], key: str, where: str) -> object:
    if key not in document:
        raise ConfigError(
            ConfigRefusalCode.FIELD_MISSING,
            f"{where}.{key} is required",
            path=f"{where}.{key}",
        )
    return document[key]


def _require_array(payload: object, where: str) -> Sequence[object]:
    if not isinstance(payload, list | tuple):
        raise ConfigError(
            ConfigRefusalCode.FIELD_TYPE,
            f"{where} must be a JSON array, was {_type_name(payload)}",
            path=where,
        )
    return payload


def _require_str(document: Mapping[str, object], key: str, where: str) -> str:
    return _require_non_empty(_require_key(document, key, where), f"{where}.{key}")


def _require_non_empty(payload: object, where: str) -> str:
    if not isinstance(payload, str):
        raise ConfigError(
            ConfigRefusalCode.FIELD_TYPE,
            f"{where} must be a string, was {_type_name(payload)}",
            path=where,
        )
    if not payload:
        raise ConfigError(
            ConfigRefusalCode.FIELD_EMPTY,
            f"{where} must not be empty",
            path=where,
        )
    return payload


def _optional_str(payload: object, where: str) -> str:
    if payload is None:
        return ""
    return _require_non_empty(payload, where)


def _require_int(document: Mapping[str, object], key: str, where: str) -> int:
    value = _require_key(document, key, where)
    return _require_integer(value, f"{where}.{key}")


def _optional_int(payload: object, where: str) -> int | None:
    if payload is None:
        return None
    return _require_integer(payload, where)


def _require_integer(payload: object, where: str) -> int:
    if isinstance(payload, bool) or not isinstance(payload, int):
        raise ConfigError(
            ConfigRefusalCode.FIELD_TYPE,
            f"{where} must be an integer, was {_type_name(payload)}",
            path=where,
        )
    return payload


def _optional_rate(payload: object, where: str, default: float) -> float:
    if payload is None:
        return default
    if isinstance(payload, bool) or not isinstance(payload, int | float):
        raise ConfigError(
            ConfigRefusalCode.FIELD_TYPE,
            f"{where} must be a number, was {_type_name(payload)}",
            path=where,
        )
    if payload < 0 or payload > 1:
        raise ConfigError(
            ConfigRefusalCode.VALUE_OUT_OF_RANGE,
            f"{where} must be between 0 and 1, was {payload}",
            path=where,
        )
    return float(payload)


def _require_decimal(payload: object, where: str) -> Decimal:
    if not isinstance(payload, str):
        raise ConfigError(
            ConfigRefusalCode.FIELD_TYPE,
            f"{where} must be a decimal string, was {_type_name(payload)}",
            path=where,
        )
    value = decimal_of(payload)
    if value is None:
        raise ConfigError(
            ConfigRefusalCode.FIELD_TYPE,
            f"{where} is not a decimal, was {payload!r}",
            path=where,
        )
    return value


def _optional_decimal(payload: object, where: str) -> Decimal | None:
    if payload is None:
        return None
    return _require_decimal(payload, where)


def _require_one_of(
    document: Mapping[str, object],
    key: str,
    where: str,
    options: set[str],
) -> str:
    value = _require_non_empty(_require_key(document, key, where), f"{where}.{key}")
    if value not in options:
        raise ConfigError(
            ConfigRefusalCode.FIELD_TYPE,
            f"{where}.{key} must be one of {sorted(options)}, was {value!r}",
            path=f"{where}.{key}",
        )
    return value


def _reject_unknown(document: Mapping[str, object], allowed: frozenset[str], where: str) -> None:
    unknown = sorted(key for key in document if key not in allowed)
    if unknown:
        raise ConfigError(
            ConfigRefusalCode.UNKNOWN_FIELD,
            f"{where} contains unknown key(s): {unknown}",
            path=where,
        )


def _type_name(payload: object) -> str:
    return "null" if payload is None else type(payload).__name__
