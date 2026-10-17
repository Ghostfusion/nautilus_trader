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
Tests for the discovery configuration document and its loader.

The cases are the ones a reader would notice breaking: a key that is ignored rather than refused, a
policy document that does not round trip through its written form, a credential that reaches a
policy file, an allowlist that names a live adapter, and a pinned policy digest that is not checked.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from nautilus_trader.discovery.config import CONFIG_SCHEMA
from nautilus_trader.discovery.config import ConfigError
from nautilus_trader.discovery.config import ConfigRefusalCode
from nautilus_trader.discovery.config import CredentialKind
from nautilus_trader.discovery.config import check_worker_capabilities
from nautilus_trader.discovery.config import load_config
from nautilus_trader.discovery.config import load_policy
from nautilus_trader.discovery.config import parse_config
from nautilus_trader.discovery.config import parse_policy
from nautilus_trader.discovery.config import scan_for_credentials
from nautilus_trader.discovery.policy import DiscoveryPolicy
from nautilus_trader.optimization.concurrency import ConcurrencyPolicy
from tests.unit.discovery.fixtures import fixture_policy


REPO_ROOT = Path(__file__).resolve().parents[4]

POLICY_NAME = "policy.json"

CONFIG_NAME = "config.json"

# A credential the canonical list names, one whose value carries a vendor prefix, and a bare hex
# string long enough to be a key with the algorithm prefix stripped off
CANONICAL_CREDENTIAL = "BINANCE_API_KEY"
PREFIXED_SECRET = "sk-live-abcdefghijklmnop1234"
HEX_SECRET = "9f" * 24


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
        "search": {"population_size": 8, "elite": 2, "mutation_rate": 0.4},
        "workers": {
            "capability_allowlist": ["nautilus_trader.discovery", "nautilus_trader.core"],
            "concurrency": {"max_workers": 4},
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


def adapter_env_vars() -> list[str]:
    """
    Read the repository's canonical list of adapter credential variables.
    """
    text = (REPO_ROOT / "scripts" / "strip-adapter-env.bash").read_text(encoding="utf-8")
    body = text.partition("ADAPTER_ENV_VARS=(")[2].partition("\n)")[0]
    return [line.strip() for line in body.splitlines() if line.strip()]


def refused(code: ConfigRefusalCode, payload: Any) -> ConfigError:
    """
    Assert that a document is refused with the given code, and return the refusal.
    """
    with pytest.raises(ConfigError) as raised:
        parse_config(payload)

    assert raised.value.code is code
    return raised.value


def test_a_valid_document_parses_into_the_declared_types(tmp_path: Path) -> None:
    """
    Test a valid document builds the declared campaign inputs.
    """
    config = load_config(write_campaign(tmp_path))

    assert config.campaign.label == "campaign-1"
    assert config.campaign.directory == "evidence"
    assert config.campaign.writer == "orchestrator"
    assert config.campaign.family_id == "family-1"
    assert config.campaign.seed == 20261017
    assert config.policy.path == POLICY_NAME
    assert config.policy.digest == ""
    assert config.search.population_size == 8
    assert config.search.elite == 2
    assert config.search.family_floor == 2
    assert config.search.mutation_rate == 0.4
    assert config.search.exploration_share == 0.25
    assert config.search.family_allocation == ()
    assert config.workers.capability_allowlist == (
        "nautilus_trader.discovery",
        "nautilus_trader.core",
    )
    assert config.workers.concurrency == ConcurrencyPolicy(per_run_bytes=536870912, max_workers=4)
    assert config.resolved_policy is not None
    assert config.resolved_policy.digest() == fixture_policy().digest()


def test_a_parsed_document_does_not_resolve_the_policy_it_names() -> None:
    """
    Test a pure parse leaves the policy unresolved, because it reads no file.
    """
    config = parse_config(document())

    assert config.resolved_policy is None
    assert config.policy.path == POLICY_NAME


def test_the_search_and_worker_blocks_default_when_they_are_absent() -> None:
    """
    Test an absent block is the declared default rather than an error.
    """
    payload = document()
    del payload["search"]
    del payload["workers"]

    config = parse_config(payload)

    assert config.search.population_size == 12
    assert config.search.elite == 2
    assert config.search.family_floor == 2
    assert config.search.mutation_rate == 0.5
    assert config.search.exploration_share == 0.25
    assert config.workers.capability_allowlist == ()
    assert config.workers.concurrency == ConcurrencyPolicy()


def test_a_partial_search_block_merges_with_the_declared_defaults() -> None:
    """
    Test a declared field moves while the undeclared ones stay at their defaults.
    """
    payload = document()
    payload["search"] = {"mutation_rate": 0.1}

    config = parse_config(payload)

    assert config.search.mutation_rate == 0.1
    assert config.search.population_size == 12
    assert config.search.elite == 2


def test_the_family_allocation_is_declared_in_a_stable_order() -> None:
    """
    Test the allocation is ordered by family rather than by document order.
    """
    payload = document()
    payload["search"] = {"family_allocation": {"trend": 0.6, "carry": 0.4}}

    config = parse_config(payload)

    assert config.search.family_allocation == (("carry", 0.4), ("trend", 0.6))


def test_a_policy_document_round_trips_through_its_written_form() -> None:
    """
    Test the written form of a policy parses back into the policy it was written from.
    """
    policy = fixture_policy()

    parsed = parse_policy(policy.to_dict())

    assert parsed.to_dict() == policy.to_dict()
    assert parsed.digest() == policy.digest()


def test_a_policy_document_that_omits_a_declared_set_is_refused() -> None:
    """
    Test a policy document missing a declared set is refused rather than defaulted.
    """
    payload = fixture_policy().to_dict()
    del payload["families"]

    with pytest.raises(ConfigError) as raised:
        parse_policy(payload)

    assert raised.value.code is ConfigRefusalCode.FIELD_MISSING
    assert raised.value.path == "policy.families"


def test_a_policy_document_that_states_an_unknown_key_is_refused() -> None:
    """
    Test an unknown policy key is refused rather than ignored.
    """
    payload = fixture_policy().to_dict()
    payload["sectors_by_name"] = []

    with pytest.raises(ConfigError) as raised:
        parse_policy(payload)

    assert raised.value.code is ConfigRefusalCode.UNKNOWN_FIELD


def test_a_policy_the_object_refuses_is_reported_with_its_own_message() -> None:
    """
    Test a policy document the policy object refuses is a single refusal, not a traceback.
    """
    payload = fixture_policy().to_dict()
    payload["families"] = []

    with pytest.raises(ConfigError) as raised:
        parse_policy(payload)

    assert raised.value.code is ConfigRefusalCode.POLICY_REFUSED
    assert "families must not be empty" in str(raised.value)


def test_a_policy_digest_that_does_not_match_the_pin_is_refused(tmp_path: Path) -> None:
    """
    Test a declared policy digest is checked against the document it names.
    """
    payload = document()
    payload["policy"] = {"path": POLICY_NAME, "digest": "sha256:" + "c" * 64}
    path = write_campaign(tmp_path, config=payload)

    with pytest.raises(ConfigError) as raised:
        load_config(path)

    assert raised.value.code is ConfigRefusalCode.POLICY_DIGEST_MISMATCH
    assert raised.value.path == "policy.digest"
    assert fixture_policy().digest() in str(raised.value)


def test_a_policy_digest_that_matches_the_pin_is_loaded(tmp_path: Path) -> None:
    """
    Test a matching pin loads the policy it was written for.
    """
    payload = document()
    payload["policy"] = {"path": POLICY_NAME, "digest": fixture_policy().digest()}
    path = write_campaign(tmp_path, config=payload)

    config = load_config(path)

    assert config.resolved_policy is not None
    assert config.resolved_policy.digest() == fixture_policy().digest()


@pytest.mark.parametrize(
    ("where", "value"),
    [
        ("families", CANONICAL_CREDENTIAL),
        ("feature_allowlist", PREFIXED_SECRET),
        ("asset_classes", HEX_SECRET),
        ("sectors", (("AAPL.NASDAQ", "TWS_PASSWORD"),)),
    ],
)
def test_a_credential_in_a_policy_file_is_refused(
    tmp_path: Path,
    where: str,
    value: Any,
) -> None:
    """
    Test the loader refuses a credential wherever in the policy document it sits.
    """
    policy = fixture_policy().to_dict()
    if where == "sectors":
        policy[where] = [[instrument_id, sector] for instrument_id, sector in value]
    else:
        policy[where] = [*policy[where], value]
    path = write_campaign(tmp_path, policy=policy)

    with pytest.raises(ConfigError) as raised:
        load_config(path)

    assert raised.value.code is ConfigRefusalCode.CREDENTIAL_REFUSED
    assert raised.value.path.startswith(f"policy.{where}")
    assert CANONICAL_CREDENTIAL not in str(raised.value)
    assert PREFIXED_SECRET not in str(raised.value)
    assert HEX_SECRET not in str(raised.value)


def test_a_credential_in_the_configuration_document_is_refused() -> None:
    """
    Test a credential in the configuration document itself is refused, not only in a policy file.
    """
    payload = document()
    payload["campaign"]["label"] = CANONICAL_CREDENTIAL

    error = refused(ConfigRefusalCode.CREDENTIAL_REFUSED, payload)

    assert error.path == "config.campaign.label"
    assert CANONICAL_CREDENTIAL not in str(error)


def test_a_credential_a_caller_names_is_refused_without_matching_a_shape() -> None:
    """
    Test a caller can extend the scan with the names only that caller knows.
    """
    payload = document()
    payload["campaign"]["family_id"] = "HYPERLIQUID_TESTNET"

    assert scan_for_credentials(payload) == ()

    with pytest.raises(ConfigError) as raised:
        parse_config(payload, credentials=["HYPERLIQUID_TESTNET"])

    assert raised.value.code is ConfigRefusalCode.CREDENTIAL_REFUSED


def test_every_name_in_the_canonical_list_is_refused_when_it_is_supplied() -> None:
    """
    Test the repository's canonical credential list is refused when a caller supplies it.
    """
    names = adapter_env_vars()
    assert len(names) > 80
    assert CANONICAL_CREDENTIAL in names

    for name in names:
        payload = fixture_policy().to_dict()
        payload["families"] = [name]
        findings = scan_for_credentials(payload, path="policy", names=names)
        assert findings, name
        assert findings[0].path == "policy.families[0]"


def test_the_secret_shaped_names_in_the_canonical_list_are_refused_without_being_named() -> None:
    """
    Test the shapes refuse the credential names without a caller supplying the list.
    """
    for name in (
        "BINANCE_API_KEY",
        "BINANCE_FUTURES_TESTNET_ED25519_API_SECRET",
        "TWS_PASSWORD",
        "POLYMARKET_PASSPHRASE",
        "BLOCKCHAIN_PRIVATE_KEY",
        "BETFAIR_USERNAME",
    ):
        payload = fixture_policy().to_dict()
        payload["families"] = [name]
        findings = scan_for_credentials(payload, path="policy")
        assert [finding.kind for finding in findings] == [CredentialKind.VARIABLE_NAME], name


def test_a_clean_document_reports_no_finding() -> None:
    """
    Test a policy document that carries no credential reports nothing.
    """
    findings = scan_for_credentials(fixture_policy().to_dict(), path="policy")

    assert findings == ()


def test_a_finding_names_where_the_credential_is_rather_than_what_it_is() -> None:
    """
    Test a finding carries a location and a shape, so no log or failure repeats the value.
    """
    payload = fixture_policy().to_dict()
    payload["families"] = ["cross_sectional_rank", CANONICAL_CREDENTIAL]

    findings = scan_for_credentials(payload, path="policy")

    assert [finding.to_dict() for finding in findings] == [
        {"path": "policy.families[1]", "kind": "variable_name"},
    ]


@pytest.mark.parametrize(
    "entry",
    [
        "nautilus_trader.adapters",
        "nautilus_trader.adapters.binance",
        "nautilus_trader.adapters.sandbox",
    ],
)
def test_an_allowlist_entry_naming_a_live_adapter_is_refused(entry: str) -> None:
    """
    Test a capability allowlist may not name an adapter, because the boundary removes them.
    """
    payload = document()
    payload["workers"]["capability_allowlist"] = ["nautilus_trader.core", entry]

    error = refused(ConfigRefusalCode.ALLOWLIST_ENTRY_NAMES_ADAPTER, payload)

    assert error.path == "workers.capability_allowlist[1]"


@pytest.mark.parametrize(
    "entry", ["nautilus_trader discovery", "not a package", "discovery/", "discovery:"]
)
def test_an_allowlist_entry_that_is_not_a_package_name_is_refused(entry: str) -> None:
    """
    Test an allowlist entry that is not a package name is refused.
    """
    payload = document()
    payload["workers"]["capability_allowlist"] = [entry]

    assert refused(ConfigRefusalCode.ALLOWLIST_ENTRY_INVALID, payload).path == (
        "workers.capability_allowlist[0]"
    )


def test_an_empty_capability_allowlist_admits_no_package() -> None:
    """
    Test an empty allowlist is the strictest reading rather than a permissive default.
    """
    check = check_worker_capabilities("worker-1", ["nautilus_trader.core"], [])

    assert check.unexpected == ("nautilus_trader.core",)
    assert not check.holds


def test_a_package_under_an_allowlisted_package_is_admitted() -> None:
    """
    Test a module below an allowlisted package is admitted and a stranger is not.
    """
    check = check_worker_capabilities(
        "worker-1",
        ["nautilus_trader.discovery.config", "nautilus_trader.core", "requests"],
        ["nautilus_trader.discovery", "nautilus_trader.core"],
    )

    assert check.unexpected == ("requests",)
    assert not check.holds
    assert check.to_dict() == {
        "worker": "worker-1",
        "allowlist": ["nautilus_trader.core", "nautilus_trader.discovery"],
        "unexpected": ["requests"],
        "holds": False,
    }


def test_a_worker_environment_holding_only_allowlisted_packages_holds() -> None:
    """
    Test a clean worker environment passes the capability check.
    """
    check = check_worker_capabilities(
        "worker-1",
        ["nautilus_trader.discovery", "nautilus_trader.core"],
        ["nautilus_trader"],
    )

    assert check.holds
    assert check.unexpected == ()


@pytest.mark.parametrize(
    ("where", "document_key"),
    [
        ("config", "unknown"),
        ("campaign", "timezone"),
        ("search", "generations"),
        ("workers", "timeout_seconds"),
        ("policy", "digest_algorithm"),
    ],
)
def test_an_unknown_key_is_refused_at_every_level(where: str, document_key: str) -> None:
    """
    Test an unknown key is refused rather than ignored, at every level of the document.
    """
    payload = document()
    if where == "config":
        payload[document_key] = 1
    else:
        payload[where][document_key] = 1

    error = refused(ConfigRefusalCode.UNKNOWN_FIELD, payload)

    assert document_key in str(error)
    assert error.path == where


@pytest.mark.parametrize(
    ("block", "key"),
    [
        ("campaign", "seed"),
        ("campaign", "label"),
        ("campaign", "directory"),
        ("workers", "capability_allowlist"),
    ],
)
def test_a_declared_key_that_is_absent_is_refused(block: str, key: str) -> None:
    """
    Test a key the document must state is refused when it is absent.
    """
    payload = document()
    del payload[block][key]

    error = refused(ConfigRefusalCode.FIELD_MISSING, payload)

    assert error.path == f"{block}.{key}"


def test_the_policy_declaration_must_name_a_path() -> None:
    """
    Test a policy block without a path is refused, because there is nothing to load.
    """
    payload = document()
    payload["policy"] = {"digest": "sha256:" + "c" * 64}

    error = refused(ConfigRefusalCode.FIELD_MISSING, payload)

    assert error.path == "policy.path"


@pytest.mark.parametrize(
    ("block", "key", "value"),
    [
        ("campaign", "seed", True),
        ("campaign", "seed", "20261017"),
        ("campaign", "label", 1),
        ("search", "population_size", 1.5),
        ("workers", "capability_allowlist", "nautilus_trader"),
        ("workers", "concurrency", {"memory_fraction": "half"}),
    ],
)
def test_a_value_of_the_wrong_type_is_refused(block: str, key: str, value: Any) -> None:
    """
    Test a value of the wrong type is refused with the type it had.
    """
    payload = document()
    payload[block][key] = value

    error = refused(ConfigRefusalCode.FIELD_TYPE, payload)

    assert f"{block}.{key}" in error.path
    assert ", was " in str(error)


@pytest.mark.parametrize(
    ("block", "key"),
    [("campaign", "label"), ("campaign", "writer"), ("workers", "capability_allowlist")],
)
def test_an_empty_string_is_refused(block: str, key: str) -> None:
    """
    Test an empty string is refused as its own defect rather than as a type error.
    """
    payload = document()
    payload[block][key] = [""] if key == "capability_allowlist" else ""

    assert refused(ConfigRefusalCode.FIELD_EMPTY, payload)


@pytest.mark.parametrize(
    ("block", "key", "value"),
    [
        ("search", "population_size", 0),
        ("search", "elite", 20),
        ("search", "mutation_rate", 1.5),
        ("search", "family_allocation", {"trend": 1.5}),
        ("workers", "concurrency", {"max_workers": 0}),
    ],
)
def test_a_value_outside_the_declared_range_is_refused(block: str, key: str, value: Any) -> None:
    """
    Test a value the declaration does not admit is refused rather than clamped.
    """
    payload = document()
    payload[block][key] = value

    assert refused(ConfigRefusalCode.VALUE_OUT_OF_RANGE, payload)


def test_a_repeated_member_is_refused() -> None:
    """
    Test a declared set that states a member twice is refused rather than deduplicated.
    """
    payload = fixture_policy().to_dict()
    payload["families"] = [*payload["families"], payload["families"][0]]

    with pytest.raises(ConfigError) as raised:
        parse_policy(payload)

    assert raised.value.code is ConfigRefusalCode.VALUE_REPEATED
    assert raised.value.path == "policy.families"


def test_a_repeated_allowlist_entry_is_refused() -> None:
    """
    Test an allowlist that states an entry twice is refused rather than deduplicated.
    """
    payload = document()
    payload["workers"]["capability_allowlist"] = ["nautilus_trader.core", "nautilus_trader.core"]

    assert refused(ConfigRefusalCode.VALUE_REPEATED, payload).path == "workers.capability_allowlist"


def test_a_schema_that_is_not_this_one_is_refused() -> None:
    """
    Test a document declaring another schema is refused.
    """
    payload = document()
    payload["schema"] = "nautilus.discovery.config/v2"

    error = refused(ConfigRefusalCode.SCHEMA_MISMATCH, payload)

    assert CONFIG_SCHEMA in str(error)


def test_a_document_that_declares_no_schema_is_accepted() -> None:
    """
    Test an absent schema is accepted, as the optimization configuration accepts one.
    """
    payload = document()
    del payload["schema"]

    config = parse_config(payload)

    assert config.campaign.label == "campaign-1"


def test_a_document_that_is_not_an_object_is_refused() -> None:
    """
    Test a document that is not a JSON object is refused.
    """
    assert refused(ConfigRefusalCode.NOT_A_MAPPING, ["campaign"]).path == "config"


def test_a_non_finite_number_is_refused(tmp_path: Path) -> None:
    """
    Test the non-finite JSON constants Python accepts by default are refused.
    """
    text = json.dumps(document()).replace('"mutation_rate": 0.4', '"mutation_rate": NaN')
    path = write_document(tmp_path, text)

    with pytest.raises(ConfigError) as raised:
        load_config(path)

    assert raised.value.code is ConfigRefusalCode.NUMBER_NOT_FINITE


def test_a_document_that_is_not_json_is_refused(tmp_path: Path) -> None:
    """
    Test a document that is not valid JSON is refused with the decoder's own words.
    """
    path = write_document(tmp_path, "{not json")

    with pytest.raises(ConfigError) as raised:
        load_config(path)

    assert raised.value.code is ConfigRefusalCode.DOCUMENT_NOT_JSON


def test_a_document_that_is_not_there_is_refused(tmp_path: Path) -> None:
    """
    Test a document that cannot be read is refused rather than raised as an OSError.
    """
    with pytest.raises(ConfigError) as raised:
        load_config(tmp_path / "absent.json")

    assert raised.value.code is ConfigRefusalCode.DOCUMENT_NOT_READABLE


def test_a_policy_document_that_is_not_there_names_the_declaration(tmp_path: Path) -> None:
    """
    Test a policy file that cannot be read is located by the key that named it.
    """
    payload = document()
    payload["policy"] = {"path": "absent.json"}
    path = write_document(tmp_path, payload)

    with pytest.raises(ConfigError) as raised:
        load_config(path)

    assert raised.value.code is ConfigRefusalCode.DOCUMENT_NOT_READABLE
    assert raised.value.path == "policy.path"


def test_a_relative_policy_path_resolves_against_the_configuration_directory(
    tmp_path: Path,
) -> None:
    """
    Test a relative policy path is resolved against the directory the document was read from.
    """
    nested = tmp_path / "campaigns" / "campaign-1"
    nested.mkdir(parents=True)
    write_campaign(nested)

    config = load_config(nested / CONFIG_NAME)

    assert config.resolved_policy is not None
    assert config.resolved_policy.digest() == fixture_policy().digest()


def test_the_loader_reads_a_policy_through_a_declaration(tmp_path: Path) -> None:
    """
    Test a declaration loads on its own, so a caller with only the declaration can resolve it.
    """
    write_campaign(tmp_path)
    config = parse_config(document())

    policy: DiscoveryPolicy = load_policy(config.policy, directory=tmp_path)

    assert policy.digest() == fixture_policy().digest()


def test_a_declaration_without_a_digest_pins_nothing(tmp_path: Path) -> None:
    """
    Test a declaration that pins no digest loads whatever the file holds.
    """
    write_campaign(
        tmp_path, policy=fixture_policy(feature_allowlist=frozenset({"close"})).to_dict()
    )
    config = parse_config(document())

    policy = load_policy(config.policy, directory=tmp_path)

    assert policy.feature_allowlist == frozenset({"close"})
