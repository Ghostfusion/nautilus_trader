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

from nautilus_trader.discovery.agents import _ISO_DATE_LENGTH as _ISO_DATE_LENGTH
from nautilus_trader.discovery.agents import _ISO_DATE_PART_WIDTHS as _ISO_DATE_PART_WIDTHS
from nautilus_trader.discovery.agents import _MAY_NOT as _MAY_NOT
from nautilus_trader.discovery.agents import _PRODUCES as _PRODUCES
from nautilus_trader.discovery.agents import _PROMPTS as _PROMPTS
from nautilus_trader.discovery.agents import AGENT_PROTOCOL_VERSION as AGENT_PROTOCOL_VERSION
from nautilus_trader.discovery.agents import INSUFFICIENT_DATA_KEY as INSUFFICIENT_DATA_KEY
from nautilus_trader.discovery.agents import INSUFFICIENT_DATA_VALUE as INSUFFICIENT_DATA_VALUE
from nautilus_trader.discovery.agents import SPECIFICATION_KEY as SPECIFICATION_KEY
from nautilus_trader.discovery.agents import AgentContext as AgentContext
from nautilus_trader.discovery.agents import AgentModel as AgentModel
from nautilus_trader.discovery.agents import AgentOutcome as AgentOutcome
from nautilus_trader.discovery.agents import AgentRequest as AgentRequest
from nautilus_trader.discovery.agents import AgentResult as AgentResult
from nautilus_trader.discovery.agents import Contamination as Contamination
from nautilus_trader.discovery.agents import InvocationRecord as InvocationRecord
from nautilus_trader.discovery.agents import Language as Language
from nautilus_trader.discovery.agents import ReplayModel as ReplayModel
from nautilus_trader.discovery.agents import Role as Role
from nautilus_trader.discovery.agents import classify as classify
from nautilus_trader.discovery.agents import invoke as invoke
from nautilus_trader.discovery.compile import CompilationResult as CompilationResult
from nautilus_trader.discovery.compile import CompiledPlan as CompiledPlan
from nautilus_trader.discovery.compile import compile_specification as compile_specification
from nautilus_trader.discovery.compile import compose_plan as compose_plan
from nautilus_trader.discovery.compile import engine_version as engine_version
from nautilus_trader.discovery.compile import specification_digest_of as specification_digest_of
from nautilus_trader.discovery.config import ADAPTER_PREFIX as ADAPTER_PREFIX
from nautilus_trader.discovery.config import CONFIG_SCHEMA as CONFIG_SCHEMA
from nautilus_trader.discovery.config import SECTOR_PAIR_LENGTH as SECTOR_PAIR_LENGTH
from nautilus_trader.discovery.config import CampaignDeclaration as CampaignDeclaration
from nautilus_trader.discovery.config import ConfigError as ConfigError
from nautilus_trader.discovery.config import ConfigRefusalCode as ConfigRefusalCode
from nautilus_trader.discovery.config import CredentialFinding as CredentialFinding
from nautilus_trader.discovery.config import CredentialKind as CredentialKind
from nautilus_trader.discovery.config import DiscoveryConfig as DiscoveryConfig
from nautilus_trader.discovery.config import PolicyDeclaration as PolicyDeclaration
from nautilus_trader.discovery.config import SearchDeclaration as SearchDeclaration
from nautilus_trader.discovery.config import WorkerCapabilityCheck as WorkerCapabilityCheck
from nautilus_trader.discovery.config import WorkerDeclaration as WorkerDeclaration
from nautilus_trader.discovery.config import check_worker_capabilities as check_worker_capabilities
from nautilus_trader.discovery.config import load_config as load_config
from nautilus_trader.discovery.config import load_policy as load_policy
from nautilus_trader.discovery.config import parse_config as parse_config
from nautilus_trader.discovery.config import parse_policy as parse_policy
from nautilus_trader.discovery.config import scan_for_credentials as scan_for_credentials
from nautilus_trader.discovery.custody import ConfirmationCustody as ConfirmationCustody
from nautilus_trader.discovery.custody import ConfirmationVerdict as ConfirmationVerdict
from nautilus_trader.discovery.custody import CustodyError as CustodyError
from nautilus_trader.discovery.custody import CustodyRefusalCode as CustodyRefusalCode
from nautilus_trader.discovery.custody import CustodyState as CustodyState
from nautilus_trader.discovery.custody import Reservation as Reservation
from nautilus_trader.discovery.custody import WorkerEnvironmentCheck as WorkerEnvironmentCheck
from nautilus_trader.discovery.custody import inherited_reservations as inherited_reservations
from nautilus_trader.discovery.evolve import _CONVENTIONS as _CONVENTIONS
from nautilus_trader.discovery.evolve import _EDIT_ATTEMPTS as _EDIT_ATTEMPTS
from nautilus_trader.discovery.evolve import _EXPLORATION_SHARE as _EXPLORATION_SHARE
from nautilus_trader.discovery.evolve import _GENERATION_STAMP as _GENERATION_STAMP
from nautilus_trader.discovery.evolve import (
    _PARAMETER_GENES_PER_MUTATION as _PARAMETER_GENES_PER_MUTATION,
)
from nautilus_trader.discovery.evolve import _PREFERS_HIGH as _PREFERS_HIGH
from nautilus_trader.discovery.evolve import (
    _STRUCTURAL_GENES_PER_MUTATION as _STRUCTURAL_GENES_PER_MUTATION,
)
from nautilus_trader.discovery.evolve import EQUIVALENCE_IGNORED_PATHS as EQUIVALENCE_IGNORED_PATHS
from nautilus_trader.discovery.evolve import EQUIVALENCE_RULE_ID as EQUIVALENCE_RULE_ID
from nautilus_trader.discovery.evolve import EVOLUTION_VERSION as EVOLUTION_VERSION
from nautilus_trader.discovery.evolve import FROZEN_FIELDS as FROZEN_FIELDS
from nautilus_trader.discovery.evolve import SEARCH_POLICY_ID as SEARCH_POLICY_ID
from nautilus_trader.discovery.evolve import SEARCH_POLICY_VERSION as SEARCH_POLICY_VERSION
from nautilus_trader.discovery.evolve import Amendment as Amendment
from nautilus_trader.discovery.evolve import Deduplication as Deduplication
from nautilus_trader.discovery.evolve import DevelopmentResult as DevelopmentResult
from nautilus_trader.discovery.evolve import EquivalenceMerge as EquivalenceMerge
from nautilus_trader.discovery.evolve import Evolution as Evolution
from nautilus_trader.discovery.evolve import GenerationPlan as GenerationPlan
from nautilus_trader.discovery.evolve import Objective as Objective
from nautilus_trader.discovery.evolve import PartitionRole as PartitionRole
from nautilus_trader.discovery.evolve import PartitionRoles as PartitionRoles
from nautilus_trader.discovery.evolve import RoleAssignment as RoleAssignment
from nautilus_trader.discovery.evolve import SearchDiagnosis as SearchDiagnosis
from nautilus_trader.discovery.evolve import SearchPolicy as SearchPolicy
from nautilus_trader.discovery.evolve import SearchRefusal as SearchRefusal
from nautilus_trader.discovery.evolve import Selection as Selection
from nautilus_trader.discovery.evolve import deduplicate as deduplicate
from nautilus_trader.discovery.evolve import dominates as dominates
from nautilus_trader.discovery.evolve import equivalence_key as equivalence_key
from nautilus_trader.discovery.evolve import pareto_front as pareto_front
from nautilus_trader.discovery.evolve import role_audit as role_audit
from nautilus_trader.discovery.evolve import selectable as selectable
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
from nautilus_trader.discovery.genome import _CONDITION_BOUND as _CONDITION_BOUND
from nautilus_trader.discovery.genome import _MECHANISM_PATHS as _MECHANISM_PATHS
from nautilus_trader.discovery.genome import _MUTATION_ATTEMPTS as _MUTATION_ATTEMPTS
from nautilus_trader.discovery.genome import _MUTATION_PLACES as _MUTATION_PLACES
from nautilus_trader.discovery.genome import _PARAMETER_GENES as _PARAMETER_GENES
from nautilus_trader.discovery.genome import _STATEMENT_FIELDS as _STATEMENT_FIELDS
from nautilus_trader.discovery.genome import _STRUCTURAL_GENES as _STRUCTURAL_GENES
from nautilus_trader.discovery.genome import GENOME_VERSION as GENOME_VERSION
from nautilus_trader.discovery.genome import Change as Change
from nautilus_trader.discovery.genome import Edit as Edit
from nautilus_trader.discovery.genome import Gene as Gene
from nautilus_trader.discovery.genome import GeneticDegree as GeneticDegree
from nautilus_trader.discovery.genome import Genome as Genome
from nautilus_trader.discovery.genome import GenomeRefusal as GenomeRefusal
from nautilus_trader.discovery.genome import Mechanism as Mechanism
from nautilus_trader.discovery.genome import genome_of as genome_of
from nautilus_trader.discovery.genome import mutate as mutate
from nautilus_trader.discovery.genome import recombine as recombine
from nautilus_trader.discovery.genome import with_mechanism as with_mechanism
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
from nautilus_trader.discovery.ledger import CampaignAudit as CampaignAudit
from nautilus_trader.discovery.ledger import CampaignInventory as CampaignInventory
from nautilus_trader.discovery.ledger import CandidateJudgement as CandidateJudgement
from nautilus_trader.discovery.ledger import ChainVerification as ChainVerification
from nautilus_trader.discovery.ledger import CountName as CountName
from nautilus_trader.discovery.ledger import Counts as Counts
from nautilus_trader.discovery.ledger import DataRole as DataRole
from nautilus_trader.discovery.ledger import DiscoveryLedger as DiscoveryLedger
from nautilus_trader.discovery.ledger import EnvelopeKind as EnvelopeKind
from nautilus_trader.discovery.ledger import EnvelopeReference as EnvelopeReference
from nautilus_trader.discovery.ledger import EventKind as EventKind
from nautilus_trader.discovery.ledger import EvidencePackage as EvidencePackage
from nautilus_trader.discovery.ledger import GateVerdictEntry as GateVerdictEntry
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
from nautilus_trader.discovery.ledger import audit_campaign as audit_campaign
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
from nautilus_trader.discovery.prompts import PROMPTS_ID as PROMPTS_ID
from nautilus_trader.discovery.prompts import Prompt as Prompt
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
from nautilus_trader.discovery.shadow import SHADOW_SCHEMA as SHADOW_SCHEMA
from nautilus_trader.discovery.shadow import SIMULATED_LABEL as SIMULATED_LABEL
from nautilus_trader.discovery.shadow import NonFill as NonFill
from nautilus_trader.discovery.shadow import ShadowError as ShadowError
from nautilus_trader.discovery.shadow import ShadowObservation as ShadowObservation
from nautilus_trader.discovery.shadow import ShadowPlan as ShadowPlan
from nautilus_trader.discovery.shadow import ShadowRefusalCode as ShadowRefusalCode
from nautilus_trader.discovery.shadow import ShadowReport as ShadowReport
from nautilus_trader.discovery.shadow import SignalDiscrepancy as SignalDiscrepancy
from nautilus_trader.discovery.shadow import TemporalContract as TemporalContract
from nautilus_trader.discovery.shadow import TemporalParity as TemporalParity
from nautilus_trader.discovery.shadow import TemporalReading as TemporalReading
from nautilus_trader.discovery.shadow import shadow_period as shadow_period
from nautilus_trader.discovery.shadow import temporal_parity as temporal_parity
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
    "ADAPTER_PREFIX",
    "AGENT_PROTOCOL_VERSION",
    "BUNDLE_SCHEMA",
    "CONFIG_SCHEMA",
    "COUNTS_MAPPING",
    "ENVELOPE_SCHEMA",
    "EQUIVALENCE_IGNORED_PATHS",
    "EQUIVALENCE_RULE_ID",
    "EVOLUTION_VERSION",
    "FROZEN_FIELDS",
    "GENOME_VERSION",
    "INSUFFICIENT_DATA_KEY",
    "INSUFFICIENT_DATA_VALUE",
    "INVENTORY_SCHEMA",
    "LEDGER_SCHEMA",
    "MISSING_UNAVAILABLE",
    "NON_DETERMINISTIC_COMPONENTS",
    "OPERATORS",
    "POLICY_ID",
    "POLICY_VERSION",
    "PROMPTS_ID",
    "REQUIRED_REFERENCE_KINDS",
    "SEARCH_POLICY_ID",
    "SEARCH_POLICY_VERSION",
    "SECTOR_PAIR_LENGTH",
    "SHADOW_SCHEMA",
    "SIMULATED_LABEL",
    "SPECIFICATION_KEY",
    "SPECIFICATION_SCHEMA_ID",
    "SPECIFICATION_SCHEMA_VERSION",
    "WARMUP_UNAVAILABLE",
    "_CONDITION_BOUND",
    "_CONVENTIONS",
    "_EDIT_ATTEMPTS",
    "_EXPLORATION_SHARE",
    "_GENERATION_STAMP",
    "_ISO_DATE_LENGTH",
    "_ISO_DATE_PART_WIDTHS",
    "_MAY_NOT",
    "_MECHANISM_PATHS",
    "_MUTATION_ATTEMPTS",
    "_MUTATION_PLACES",
    "_PARAMETER_GENES",
    "_PARAMETER_GENES_PER_MUTATION",
    "_PREFERS_HIGH",
    "_PRODUCES",
    "_PROMPTS",
    "_STATEMENT_FIELDS",
    "_STRUCTURAL_GENES",
    "_STRUCTURAL_GENES_PER_MUTATION",
    "Actor",
    "AdmissionAssessment",
    "AdmissionInputs",
    "AdmissionOutcome",
    "AgentContext",
    "AgentModel",
    "AgentOutcome",
    "AgentRequest",
    "AgentResult",
    "AllocationRequest",
    "Amendment",
    "AttemptOutcome",
    "AttemptRecord",
    "Bound",
    "BoundKind",
    "BundleFinding",
    "BundleVerification",
    "CalibrationCase",
    "CalibrationOutcome",
    "CalibrationReport",
    "CampaignAudit",
    "CampaignDeclaration",
    "CampaignInventory",
    "CandidateJudgement",
    "CandidateState",
    "ChainVerification",
    "Change",
    "CompilationResult",
    "CompiledPlan",
    "ComplexityLimits",
    "ConditionAll",
    "ConditionAny",
    "ConditionLeaf",
    "ConditionNode",
    "ConfigError",
    "ConfigRefusalCode",
    "ConfirmationCustody",
    "ConfirmationVerdict",
    "Contamination",
    "ControlClass",
    "CountName",
    "Counts",
    "CredentialFinding",
    "CredentialKind",
    "CrossSectionEntry",
    "CrossSectionRow",
    "CustodyError",
    "CustodyRefusalCode",
    "CustodyState",
    "DataBlock",
    "DataRole",
    "DecisionRecord",
    "Deduplication",
    "DevelopmentResult",
    "DiscoveryConfig",
    "DiscoveryInterpreter",
    "DiscoveryInterpreterConfig",
    "DiscoveryLedger",
    "DiscoveryPolicy",
    "Edit",
    "EntryBlock",
    "EnvelopeKind",
    "EnvelopeReference",
    "EquivalenceMerge",
    "EvaluationBlock",
    "EventKind",
    "EvidencePackage",
    "Evolution",
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
    "GateVerdictEntry",
    "GateVerdictRecord",
    "Gene",
    "GenerationPlan",
    "GeneticDegree",
    "Genome",
    "GenomeRefusal",
    "IdempotencyKey",
    "IdentityBlock",
    "InterpreterRunInputs",
    "InvocationRecord",
    "Language",
    "LedgerEvent",
    "Lifecycle",
    "LineageRecord",
    "Mechanism",
    "NonFill",
    "Objective",
    "OutcomeRecord",
    "ParsedSpecification",
    "PartitionRole",
    "PartitionRoles",
    "PboEvaluation",
    "PboExclusion",
    "PboRow",
    "PboState",
    "PolicyDeclaration",
    "PortfolioBlock",
    "Prompt",
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
    "ReplayModel",
    "ReproducibilityEnvelope",
    "Reservation",
    "RiskBlock",
    "RiskPostRunBlock",
    "RiskPreTradeBlock",
    "RiskRuntimeBlock",
    "Role",
    "RoleAssignment",
    "RunIdentity",
    "RunReceipt",
    "RunRequest",
    "SearchDeclaration",
    "SearchDiagnosis",
    "SearchPolicy",
    "SearchRefusal",
    "Selection",
    "SelectionBlock",
    "ShadowError",
    "ShadowObservation",
    "ShadowPlan",
    "ShadowRefusalCode",
    "ShadowReport",
    "SignalDiscrepancy",
    "SignalRecord",
    "Specification",
    "TemporalContract",
    "TemporalParity",
    "TemporalReading",
    "TrackingAssumptions",
    "TransitionRecord",
    "TransitionRule",
    "TrialMapping",
    "UniverseBlock",
    "ValidationResult",
    "ValidationWarning",
    "WarningCode",
    "WorkerCapabilityCheck",
    "WorkerDeclaration",
    "WorkerEnvironmentCheck",
    "allocate_targets",
    "assess_admission",
    "audit_campaign",
    "build_memberships",
    "build_panel",
    "bundle_identity",
    "canonical_mapping",
    "check_worker_capabilities",
    "classify",
    "compile_specification",
    "compose_plan",
    "compute_observations",
    "count_condition_leaves",
    "counts_from_events",
    "decimal_of",
    "deduplicate",
    "dominates",
    "engine_version",
    "equivalence_key",
    "evaluate_pbo_matrix",
    "experiment_for",
    "genome_of",
    "inherited_reservations",
    "invoke",
    "load_config",
    "load_policy",
    "minimum_track_record_length",
    "mutate",
    "non_deterministic_digest",
    "pareto_front",
    "parse_config",
    "parse_policy",
    "parse_specification",
    "rebalance_period_key",
    "recombine",
    "recompute",
    "reconcile_receipt",
    "required_observations",
    "research_family_id",
    "role_audit",
    "run_calibration",
    "run_experiment",
    "scan_for_code_references",
    "scan_for_credentials",
    "select_cross_section",
    "selectable",
    "shadow_period",
    "specification_digest_of",
    "temporal_parity",
    "transition_rule",
    "trial_mapping",
    "validate_specification",
    "verify_bundle",
    "with_mechanism",
]
