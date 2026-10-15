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
The compiler's fifth stage: an accepted specification becomes a compiled plan with both identities.

The plan is data. It holds no generated code, no module path and no callable, and nothing in this
module imports a name a candidate supplied, so there is no second execution path into the language
and no import side effect to trust. The interpreter is the only executor, the specification is
configured into it, and the artifact that runs is the artifact that was hashed.

Two identities are kept and neither substitutes for the other. The *specification identity* is the
canonical digest of what was submitted, so a candidate is found and deduplicated by the thing it
said. The *behavioural identity* is a digest over the specification digest together with the
resolved feature definitions, the policy, the registry, the schema version, the declared
bar-ambiguity assumptions and the engine version, so a candidate whose feature definition, policy or
engine changed underneath it is a different candidate even though its text is unchanged.

The bar-ambiguity assumptions are part of the behavioural identity because a specification's
behaviour depends on the assumptions it ran under: whether a stop is compared against the bar's path
or its close, and what happens to a threshold the market gaps past, are declared there rather than
invented here.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

import nautilus_trader
from nautilus_trader.discovery.schema import SPECIFICATION_SCHEMA_VERSION
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.discovery.validate import ValidationWarning
from nautilus_trader.discovery.validate import validate_specification
from nautilus_trader.optimization.assumptions import BarAmbiguityPolicy
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping

    from nautilus_trader.discovery.features import FeatureDefinition
    from nautilus_trader.discovery.features import FeatureRegistry
    from nautilus_trader.discovery.policy import DiscoveryPolicy
    from nautilus_trader.discovery.schema import Migration
    from nautilus_trader.discovery.schema import Refusal
    from nautilus_trader.discovery.schema import Specification


# The version reported when the installed package exposes none. The value is read from the package
# rather than from a file, so a plan names the engine that built it or says it does not know.
ENGINE_VERSION_UNKNOWN = "unknown"


@dataclass(frozen=True)
class CompiledPlan:
    """
    An accepted specification and everything its behaviour depends on, under two identities.

    The definitions are held in the order the specification declared them, which is what lets
    `definition_for` resolve an alias without a second mapping that could drift from the first.

    Parameters
    ----------
    specification : Specification
        The accepted specification, which is the artifact that runs.
    definitions : tuple[FeatureDefinition, ...]
        The resolved feature definitions, in declaration order.
    specification_digest : str
        The canonical digest of the specification, which is its specification identity.
    behavioural_digest : str
        The digest of the compiled plan, which is the identity behaviour is compared by.
    policy_digest : str
        The digest of the pre-registered policy the specification was compiled against.
    registry_digest : str
        The digest of the feature registry the definitions were resolved from.
    assumptions_digest : str
        The digest of the declared bar-ambiguity assumptions the run is read under.
    engine_version : str
        The engine version the plan was compiled by.
    migrations : tuple[str, ...], default ()
        The schema versions the payload passed through before it was parsed, recorded because a
        migrated candidate's identity includes the path it took.

    """

    specification: Specification
    definitions: tuple[FeatureDefinition, ...]
    specification_digest: str
    behavioural_digest: str
    policy_digest: str
    registry_digest: str
    assumptions_digest: str
    engine_version: str
    migrations: tuple[str, ...] = ()

    @property
    def specification_identity(self) -> str:
        """
        Return the specification identity, which is the same digest as `specification_digest`.

        The two names are both kept: `specification_digest` is the field a serialized plan carries,
        and `specification_identity` is the name the design uses when it distinguishes the two
        identities a candidate is tracked by. They are one value.
        """
        return self.specification_digest

    def definition_for(self, alias: str) -> FeatureDefinition | None:
        """
        Return the definition a declared alias resolved to, or nothing when it declared none.

        Parameters
        ----------
        alias : str
            The alias the specification declared.

        Returns
        -------
        FeatureDefinition or None

        """
        for index, declaration in enumerate(self.specification.features.declarations):
            if declaration.alias == alias:
                return self.definitions[index] if index < len(self.definitions) else None

        return None

    def to_dict(self) -> dict[str, object]:
        """
        Return the plan as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "specification_digest": self.specification_digest,
            "behavioural_digest": self.behavioural_digest,
            "policy_digest": self.policy_digest,
            "registry_digest": self.registry_digest,
            "assumptions_digest": self.assumptions_digest,
            "engine_version": self.engine_version,
            "migrations": list(self.migrations),
            "specification": self.specification.to_dict(),
            "definitions": [
                {
                    "alias": declaration.alias,
                    **definition.to_dict(),
                    "digest": definition.digest(),
                }
                for declaration, definition in zip(
                    self.specification.features.declarations,
                    self.definitions,
                    strict=True,
                )
            ],
        }


@dataclass(frozen=True)
class CompilationResult:
    """
    The outcome of compiling a payload: a plan, or the refusals that stopped it.

    A refusal is an auditable outcome rather than an error to be retried in a different shape, so a
    refused payload returns its errors here and never raises.

    Parameters
    ----------
    plan : CompiledPlan, optional
        The compiled plan, present only when the payload was accepted.
    errors : tuple[Refusal, ...]
        Every refusal the validation stages recorded.
    warnings : tuple[ValidationWarning, ...]
        The warnings of an accepted specification, which are reported beside the plan.

    """

    plan: CompiledPlan | None
    errors: tuple[Refusal, ...]
    warnings: tuple[ValidationWarning, ...]

    @property
    def accepted(self) -> bool:
        """
        Whether a plan was compiled and nothing refused it.
        """
        return self.plan is not None and not self.errors


def compile_specification(
    payload: object,
    policy: DiscoveryPolicy,
    registry: FeatureRegistry,
    *,
    migrations: Mapping[str, Migration] | None = None,
) -> CompilationResult:
    """
    Validate a payload and compile it into a plan, or return the refusals that stopped it.

    Validation runs first, and a payload a stage refused is never compiled: the plan exists only for
    a specification that passed every stage, so a refused payload returns its errors rather than
    raising.

    Parameters
    ----------
    payload : object
        The specification as data.
    policy : DiscoveryPolicy
        The pre-registered policy the payload is checked against, whose digest the plan carries.
    registry : FeatureRegistry
        The registered definitions the declared features are resolved against.
    migrations : Mapping[str, Migration], optional
        The migration table to use, defaulting to the schema module's.

    Returns
    -------
    CompilationResult

    """
    validated = validate_specification(payload, policy, registry, migrations=migrations)
    specification = validated.specification

    if specification is None or validated.errors:
        return CompilationResult(
            plan=None,
            errors=validated.errors,
            warnings=validated.warnings,
        )

    return CompilationResult(
        plan=compose_plan(
            specification,
            policy,
            registry,
            validated.definitions,
            validated.migrations,
        ),
        errors=(),
        warnings=validated.warnings,
    )


def compose_plan(
    specification: Specification,
    policy: DiscoveryPolicy,
    registry: FeatureRegistry,
    definitions: tuple[FeatureDefinition, ...],
    migrations: tuple[str, ...] = (),
) -> CompiledPlan:
    """
    Compose the compiled plan of an accepted specification, with both of its identities.

    Parameters
    ----------
    specification : Specification
        The accepted specification, which is the artifact that runs.
    policy : DiscoveryPolicy
        The pre-registered policy the specification was compiled against.
    registry : FeatureRegistry
        The registry the definitions were resolved from.
    definitions : tuple[FeatureDefinition, ...]
        The resolved definitions, in declaration order.
    migrations : tuple[str, ...], default ()
        The schema versions the payload passed through before it was parsed.

    Returns
    -------
    CompiledPlan

    """
    assumptions_digest = BarAmbiguityPolicy.declared_default().digest
    engine = engine_version()
    specification_digest = specification_digest_of(specification)
    policy_digest = policy.digest()
    registry_digest = registry.digest()

    behavioural_digest = digest_of(
        {
            "schema": SPECIFICATION_SCHEMA_VERSION,
            "specification": specification_digest,
            "policy": policy_digest,
            "registry": registry_digest,
            "features": "|".join(definition.digest() for definition in definitions),
            "assumptions": assumptions_digest,
            "engine": engine,
        },
    )

    return CompiledPlan(
        specification=specification,
        definitions=definitions,
        specification_digest=specification_digest,
        behavioural_digest=behavioural_digest,
        policy_digest=policy_digest,
        registry_digest=registry_digest,
        assumptions_digest=assumptions_digest,
        engine_version=engine,
        migrations=migrations,
    )


def specification_digest_of(specification: Specification) -> str:
    """
    Return the canonical digest of a specification, which is its specification identity.

    Parameters
    ----------
    specification : Specification
        The specification to digest.

    Returns
    -------
    str
        The `sha256:<hex>` digest.

    """
    return digest_of(canonical_mapping(specification.to_dict()))


def engine_version() -> str:
    """
    Return the version of the installed engine, or `unknown` when the package exposes none.

    Returns
    -------
    str

    """
    version = getattr(nautilus_trader, "__version__", None)
    if isinstance(version, str) and version:
        return version

    return ENGINE_VERSION_UNKNOWN
