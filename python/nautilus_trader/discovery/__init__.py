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
from nautilus_trader.discovery.custody import ConfirmationCustody as ConfirmationCustody
from nautilus_trader.discovery.custody import ConfirmationVerdict as ConfirmationVerdict
from nautilus_trader.discovery.custody import CustodyError as CustodyError
from nautilus_trader.discovery.custody import CustodyRefusalCode as CustodyRefusalCode
from nautilus_trader.discovery.custody import CustodyState as CustodyState
from nautilus_trader.discovery.custody import Reservation as Reservation
from nautilus_trader.discovery.custody import WorkerEnvironmentCheck as WorkerEnvironmentCheck
from nautilus_trader.discovery.custody import inherited_reservations as inherited_reservations
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
from nautilus_trader.discovery.gates import Actor as Actor
from nautilus_trader.discovery.gates import AdmissionAssessment as AdmissionAssessment
from nautilus_trader.discovery.gates import AdmissionInputs as AdmissionInputs
from nautilus_trader.discovery.gates import AdmissionOutcome as AdmissionOutcome
from nautilus_trader.discovery.gates import CalibrationCase as CalibrationCase
from nautilus_trader.discovery.gates import CalibrationOutcome as CalibrationOutcome
from nautilus_trader.discovery.gates import CalibrationReport as CalibrationReport
from nautilus_trader.discovery.gates import CandidateState as CandidateState
from nautilus_trader.discovery.gates import ControlClass as ControlClass
from nautilus_trader.discovery.gates import GateEvidence as GateEvidence
from nautilus_trader.discovery.gates import GateName as GateName
from nautilus_trader.discovery.gates import GatePolicy as GatePolicy
from nautilus_trader.discovery.gates import GateReport as GateReport
from nautilus_trader.discovery.gates import GateService as GateService
from nautilus_trader.discovery.gates import GateVerdict as GateVerdict
from nautilus_trader.discovery.gates import GateVerdictRecord as GateVerdictRecord
from nautilus_trader.discovery.gates import Lifecycle as Lifecycle
from nautilus_trader.discovery.gates import OutcomeRecord as OutcomeRecord
from nautilus_trader.discovery.gates import PboEvaluation as PboEvaluation
from nautilus_trader.discovery.gates import PboExclusion as PboExclusion
from nautilus_trader.discovery.gates import PboRow as PboRow
from nautilus_trader.discovery.gates import PboState as PboState
from nautilus_trader.discovery.gates import TrackingAssumptions as TrackingAssumptions
from nautilus_trader.discovery.gates import TransitionRecord as TransitionRecord
from nautilus_trader.discovery.gates import TransitionRule as TransitionRule
from nautilus_trader.discovery.gates import assess_admission as assess_admission
from nautilus_trader.discovery.gates import evaluate_pbo_matrix as evaluate_pbo_matrix
from nautilus_trader.discovery.gates import (
    minimum_track_record_length as minimum_track_record_length,
)
from nautilus_trader.discovery.gates import required_observations as required_observations
from nautilus_trader.discovery.gates import run_calibration as run_calibration
from nautilus_trader.discovery.gates import transition_rule as transition_rule
from nautilus_trader.discovery.ledger import BUNDLE_SCHEMA as BUNDLE_SCHEMA
from nautilus_trader.discovery.ledger import COUNTS_MAPPING as COUNTS_MAPPING
from nautilus_trader.discovery.ledger import ENVELOPE_SCHEMA as ENVELOPE_SCHEMA
from nautilus_trader.discovery.ledger import INVENTORY_SCHEMA as INVENTORY_SCHEMA
from nautilus_trader.discovery.ledger import LEDGER_SCHEMA as LEDGER_SCHEMA
from nautilus_trader.discovery.ledger import (
    NON_DETERMINISTIC_COMPONENTS as NON_DETERMINISTIC_COMPONENTS,
)
from nautilus_trader.discovery.ledger import REQUIRED_REFERENCE_KINDS as REQUIRED_REFERENCE_KINDS
from nautilus_trader.discovery.ledger import AttemptOutcome as AttemptOutcome
from nautilus_trader.discovery.ledger import AttemptRecord as AttemptRecord
from nautilus_trader.discovery.ledger import BundleFinding as BundleFinding
from nautilus_trader.discovery.ledger import BundleVerification as BundleVerification
from nautilus_trader.discovery.ledger import CampaignInventory as CampaignInventory
from nautilus_trader.discovery.ledger import ChainVerification as ChainVerification
from nautilus_trader.discovery.ledger import CountName as CountName
from nautilus_trader.discovery.ledger import Counts as Counts
from nautilus_trader.discovery.ledger import DataRole as DataRole
from nautilus_trader.discovery.ledger import DiscoveryLedger as DiscoveryLedger
from nautilus_trader.discovery.ledger import EnvelopeKind as EnvelopeKind
from nautilus_trader.discovery.ledger import EnvelopeReference as EnvelopeReference
from nautilus_trader.discovery.ledger import EventKind as EventKind
from nautilus_trader.discovery.ledger import EvidencePackage as EvidencePackage
from nautilus_trader.discovery.ledger import IdempotencyKey as IdempotencyKey
from nautilus_trader.discovery.ledger import LedgerEvent as LedgerEvent
from nautilus_trader.discovery.ledger import LineageRecord as LineageRecord
from nautilus_trader.discovery.ledger import Recomputation as Recomputation
from nautilus_trader.discovery.ledger import RecomputedQuantity as RecomputedQuantity
from nautilus_trader.discovery.ledger import Reconciliation as Reconciliation
from nautilus_trader.discovery.ledger import RegistrationOutcome as RegistrationOutcome
from nautilus_trader.discovery.ledger import RegistrationState as RegistrationState
from nautilus_trader.discovery.ledger import ReproducibilityEnvelope as ReproducibilityEnvelope
from nautilus_trader.discovery.ledger import RunIdentity as RunIdentity
from nautilus_trader.discovery.ledger import RunReceipt as RunReceipt
from nautilus_trader.discovery.ledger import TrialMapping as TrialMapping
from nautilus_trader.discovery.ledger import bundle_identity as bundle_identity
from nautilus_trader.discovery.ledger import counts_from_events as counts_from_events
from nautilus_trader.discovery.ledger import non_deterministic_digest as non_deterministic_digest
from nautilus_trader.discovery.ledger import recompute as recompute
from nautilus_trader.discovery.ledger import reconcile_receipt as reconcile_receipt
from nautilus_trader.discovery.ledger import research_family_id as research_family_id
from nautilus_trader.discovery.ledger import trial_mapping as trial_mapping
from nautilus_trader.discovery.ledger import verify_bundle as verify_bundle
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
from nautilus_trader.discovery.strategy import AllocationRequest as AllocationRequest
from nautilus_trader.discovery.strategy import CrossSectionEntry as CrossSectionEntry
from nautilus_trader.discovery.strategy import CrossSectionRow as CrossSectionRow
from nautilus_trader.discovery.strategy import DecisionRecord as DecisionRecord
from nautilus_trader.discovery.strategy import DiscoveryInterpreter as DiscoveryInterpreter
from nautilus_trader.discovery.strategy import (
    DiscoveryInterpreterConfig as DiscoveryInterpreterConfig,
)
from nautilus_trader.discovery.strategy import FillRecord as FillRecord
from nautilus_trader.discovery.strategy import InterpreterRunInputs as InterpreterRunInputs
from nautilus_trader.discovery.strategy import SignalRecord as SignalRecord
from nautilus_trader.discovery.strategy import allocate_targets as allocate_targets
from nautilus_trader.discovery.strategy import rebalance_period_key as rebalance_period_key
from nautilus_trader.discovery.strategy import select_cross_section as select_cross_section
from nautilus_trader.discovery.validate import ValidationResult as ValidationResult
from nautilus_trader.discovery.validate import ValidationWarning as ValidationWarning
from nautilus_trader.discovery.validate import WarningCode as WarningCode
from nautilus_trader.discovery.validate import validate_specification as validate_specification


__all__ = [
    "BUNDLE_SCHEMA",
    "COUNTS_MAPPING",
    "ENVELOPE_SCHEMA",
    "INVENTORY_SCHEMA",
    "LEDGER_SCHEMA",
    "MISSING_UNAVAILABLE",
    "NON_DETERMINISTIC_COMPONENTS",
    "OPERATORS",
    "POLICY_ID",
    "POLICY_VERSION",
    "REQUIRED_REFERENCE_KINDS",
    "SPECIFICATION_SCHEMA_ID",
    "SPECIFICATION_SCHEMA_VERSION",
    "WARMUP_UNAVAILABLE",
    "Actor",
    "AdmissionAssessment",
    "AdmissionInputs",
    "AdmissionOutcome",
    "AllocationRequest",
    "AttemptOutcome",
    "AttemptRecord",
    "Bound",
    "BoundKind",
    "BundleFinding",
    "BundleVerification",
    "CalibrationCase",
    "CalibrationOutcome",
    "CalibrationReport",
    "CampaignInventory",
    "CandidateState",
    "ChainVerification",
    "CompilationResult",
    "CompiledPlan",
    "ComplexityLimits",
    "ConditionAll",
    "ConditionAny",
    "ConditionLeaf",
    "ConditionNode",
    "ConfirmationCustody",
    "ConfirmationVerdict",
    "ControlClass",
    "CountName",
    "Counts",
    "CrossSectionEntry",
    "CrossSectionRow",
    "CustodyError",
    "CustodyRefusalCode",
    "CustodyState",
    "DataBlock",
    "DataRole",
    "DecisionRecord",
    "DiscoveryInterpreter",
    "DiscoveryInterpreterConfig",
    "DiscoveryLedger",
    "DiscoveryPolicy",
    "EntryBlock",
    "EnvelopeKind",
    "EnvelopeReference",
    "EvaluationBlock",
    "EventKind",
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
    "FillRecord",
    "GateEvidence",
    "GateName",
    "GatePolicy",
    "GateReport",
    "GateService",
    "GateVerdict",
    "GateVerdictRecord",
    "IdempotencyKey",
    "IdentityBlock",
    "InterpreterRunInputs",
    "LedgerEvent",
    "Lifecycle",
    "LineageRecord",
    "OutcomeRecord",
    "ParsedSpecification",
    "PboEvaluation",
    "PboExclusion",
    "PboRow",
    "PboState",
    "PortfolioBlock",
    "ProvenanceBlock",
    "RebalanceBlock",
    "Recomputation",
    "RecomputedQuantity",
    "Reconciliation",
    "Reference",
    "Refusal",
    "RefusalCode",
    "RegistrationOutcome",
    "RegistrationState",
    "ReproducibilityEnvelope",
    "Reservation",
    "RiskBlock",
    "RiskPostRunBlock",
    "RiskPreTradeBlock",
    "RiskRuntimeBlock",
    "RunIdentity",
    "RunReceipt",
    "RunRequest",
    "SelectionBlock",
    "SignalRecord",
    "Specification",
    "TrackingAssumptions",
    "TransitionRecord",
    "TransitionRule",
    "TrialMapping",
    "UniverseBlock",
    "ValidationResult",
    "ValidationWarning",
    "WarningCode",
    "WorkerEnvironmentCheck",
    "allocate_targets",
    "assess_admission",
    "build_memberships",
    "build_panel",
    "bundle_identity",
    "canonical_mapping",
    "compile_specification",
    "compose_plan",
    "compute_observations",
    "count_condition_leaves",
    "counts_from_events",
    "decimal_of",
    "engine_version",
    "evaluate_pbo_matrix",
    "experiment_for",
    "inherited_reservations",
    "minimum_track_record_length",
    "non_deterministic_digest",
    "parse_specification",
    "rebalance_period_key",
    "recompute",
    "reconcile_receipt",
    "required_observations",
    "research_family_id",
    "run_calibration",
    "run_experiment",
    "scan_for_code_references",
    "select_cross_section",
    "specification_digest_of",
    "transition_rule",
    "trial_mapping",
    "validate_specification",
    "verify_bundle",
]
