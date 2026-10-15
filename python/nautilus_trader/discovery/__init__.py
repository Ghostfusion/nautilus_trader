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
Autonomous strategy discovery: a candidate language, a compiler, and one interpreter.

A candidate is data. The language is closed, the policy that bounds it is frozen outside the search,
and the only executor is a single interpreter strategy configured from a compiled specification, so
the artifact that ran is the artifact that was hashed. Nothing here generates code, imports a module
named by a candidate, or computes a statistic the repository already computes.

This package is research and tooling. No kernel crate depends on it, it depends on no kernel crate's
internals, and it composes the public Python surfaces of the platform rather than reaching past
them.

Importing this package has no side effects and needs no network.
"""

from nautilus_trader.discovery.compile import CompilationResult as CompilationResult
from nautilus_trader.discovery.compile import CompiledPlan as CompiledPlan
from nautilus_trader.discovery.compile import compile_specification as compile_specification
from nautilus_trader.discovery.compile import compose_plan as compose_plan
from nautilus_trader.discovery.compile import engine_version as engine_version
from nautilus_trader.discovery.compile import specification_digest_of as specification_digest_of
from nautilus_trader.discovery.features import MISSING_UNAVAILABLE as MISSING_UNAVAILABLE
from nautilus_trader.discovery.features import OPERATORS as OPERATORS
from nautilus_trader.discovery.features import WARMUP_UNAVAILABLE as WARMUP_UNAVAILABLE
from nautilus_trader.discovery.features import FeatureDefinition as FeatureDefinition
from nautilus_trader.discovery.features import FeatureObservation as FeatureObservation
from nautilus_trader.discovery.features import FeatureRegistry as FeatureRegistry
from nautilus_trader.discovery.features import FeatureSnapshot as FeatureSnapshot
from nautilus_trader.discovery.features import FeatureStream as FeatureStream
from nautilus_trader.discovery.features import FeatureValueAt as FeatureValueAt
from nautilus_trader.discovery.features import build_memberships as build_memberships
from nautilus_trader.discovery.features import build_panel as build_panel
from nautilus_trader.discovery.features import compute_observations as compute_observations
from nautilus_trader.discovery.ledger import LEDGER_SCHEMA as LEDGER_SCHEMA
from nautilus_trader.discovery.ledger import DiscoveryLedger as DiscoveryLedger
from nautilus_trader.discovery.ledger import EvidencePackage as EvidencePackage
from nautilus_trader.discovery.ledger import RunIdentity as RunIdentity
from nautilus_trader.discovery.policy import POLICY_ID as POLICY_ID
from nautilus_trader.discovery.policy import POLICY_VERSION as POLICY_VERSION
from nautilus_trader.discovery.policy import ComplexityLimits as ComplexityLimits
from nautilus_trader.discovery.policy import DiscoveryPolicy as DiscoveryPolicy
from nautilus_trader.discovery.policy import Reference as Reference
from nautilus_trader.discovery.policy import ReferenceKind as ReferenceKind
from nautilus_trader.discovery.run import ExperimentOutcome as ExperimentOutcome
from nautilus_trader.discovery.run import RunRequest as RunRequest
from nautilus_trader.discovery.run import experiment_for as experiment_for
from nautilus_trader.discovery.run import run_experiment as run_experiment
from nautilus_trader.discovery.schema import SPECIFICATION_SCHEMA_ID as SPECIFICATION_SCHEMA_ID
from nautilus_trader.discovery.schema import (
    SPECIFICATION_SCHEMA_VERSION as SPECIFICATION_SCHEMA_VERSION,
)
from nautilus_trader.discovery.schema import Bound as Bound
from nautilus_trader.discovery.schema import BoundKind as BoundKind
from nautilus_trader.discovery.schema import ConditionAll as ConditionAll
from nautilus_trader.discovery.schema import ConditionAny as ConditionAny
from nautilus_trader.discovery.schema import ConditionLeaf as ConditionLeaf
from nautilus_trader.discovery.schema import ConditionNode as ConditionNode
from nautilus_trader.discovery.schema import DataBlock as DataBlock
from nautilus_trader.discovery.schema import EntryBlock as EntryBlock
from nautilus_trader.discovery.schema import EvaluationBlock as EvaluationBlock
from nautilus_trader.discovery.schema import ExitBlock as ExitBlock
from nautilus_trader.discovery.schema import FeatureDeclaration as FeatureDeclaration
from nautilus_trader.discovery.schema import FeaturesBlock as FeaturesBlock
from nautilus_trader.discovery.schema import IdentityBlock as IdentityBlock
from nautilus_trader.discovery.schema import ParsedSpecification as ParsedSpecification
from nautilus_trader.discovery.schema import PortfolioBlock as PortfolioBlock
from nautilus_trader.discovery.schema import ProvenanceBlock as ProvenanceBlock
from nautilus_trader.discovery.schema import RebalanceBlock as RebalanceBlock
from nautilus_trader.discovery.schema import Refusal as Refusal
from nautilus_trader.discovery.schema import RefusalCode as RefusalCode
from nautilus_trader.discovery.schema import RiskBlock as RiskBlock
from nautilus_trader.discovery.schema import RiskPostRunBlock as RiskPostRunBlock
from nautilus_trader.discovery.schema import RiskPreTradeBlock as RiskPreTradeBlock
from nautilus_trader.discovery.schema import RiskRuntimeBlock as RiskRuntimeBlock
from nautilus_trader.discovery.schema import SelectionBlock as SelectionBlock
from nautilus_trader.discovery.schema import Specification as Specification
from nautilus_trader.discovery.schema import UniverseBlock as UniverseBlock
from nautilus_trader.discovery.schema import canonical_mapping as canonical_mapping
from nautilus_trader.discovery.schema import count_condition_leaves as count_condition_leaves
from nautilus_trader.discovery.schema import decimal_of as decimal_of
from nautilus_trader.discovery.schema import parse_specification as parse_specification
from nautilus_trader.discovery.schema import scan_for_code_references as scan_for_code_references
from nautilus_trader.discovery.validate import ValidationResult as ValidationResult
from nautilus_trader.discovery.validate import ValidationWarning as ValidationWarning
from nautilus_trader.discovery.validate import WarningCode as WarningCode
from nautilus_trader.discovery.validate import validate_specification as validate_specification


__all__ = [
    "LEDGER_SCHEMA",
    "MISSING_UNAVAILABLE",
    "OPERATORS",
    "POLICY_ID",
    "POLICY_VERSION",
    "SPECIFICATION_SCHEMA_ID",
    "SPECIFICATION_SCHEMA_VERSION",
    "WARMUP_UNAVAILABLE",
    "Bound",
    "BoundKind",
    "CompilationResult",
    "CompiledPlan",
    "ComplexityLimits",
    "ConditionAll",
    "ConditionAny",
    "ConditionLeaf",
    "ConditionNode",
    "DataBlock",
    "DiscoveryLedger",
    "DiscoveryPolicy",
    "EntryBlock",
    "EvaluationBlock",
    "EvidencePackage",
    "ExitBlock",
    "ExperimentOutcome",
    "FeatureDeclaration",
    "FeatureDefinition",
    "FeatureObservation",
    "FeatureRegistry",
    "FeatureSnapshot",
    "FeatureStream",
    "FeatureValueAt",
    "FeaturesBlock",
    "IdentityBlock",
    "ParsedSpecification",
    "PortfolioBlock",
    "ProvenanceBlock",
    "RebalanceBlock",
    "Reference",
    "ReferenceKind",
    "Refusal",
    "RefusalCode",
    "RiskBlock",
    "RiskPostRunBlock",
    "RiskPreTradeBlock",
    "RiskRuntimeBlock",
    "RunIdentity",
    "RunRequest",
    "SelectionBlock",
    "Specification",
    "UniverseBlock",
    "ValidationResult",
    "ValidationWarning",
    "WarningCode",
    "build_memberships",
    "build_panel",
    "canonical_mapping",
    "compile_specification",
    "compose_plan",
    "compute_observations",
    "count_condition_leaves",
    "decimal_of",
    "engine_version",
    "experiment_for",
    "parse_specification",
    "run_experiment",
    "scan_for_code_references",
    "specification_digest_of",
    "validate_specification",
]
