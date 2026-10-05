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
The join between a research decision artifact and this repository's trading types.

The research half writes a versioned decision document to disk; the engine half owns authoritative
market, portfolio, risk, order and fill state. Nothing read one and produced the other. This package
is that join, and nothing else: it reads the artifact, refuses it by name when it cannot be used,
projects it onto a `TradingSignal`, and records on an append-only ledger what became of it.

The governing rule is that research may constrain or inform the engine but must never become a
second
execution or risk authority. Mechanically that means research-derived fields may only reduce what
the
engine independently decided: they may cap a size, reject an action or reduce risk, and they may not
invent a direction, enlarge a position or authorise an otherwise invalid decision.

The projection is a pure function of the artifact, the configuration, the calendar and the engine's
own state. It reads no clock: the receipt instant is supplied by the caller, because a producer
cannot
know when its output will be read and a bridge that read the clock could not be replayed.

The stages run in one declared order, and the first failing stage names the refusal:
admission (`admit`), availability (`resolve_actionability`), tradability (`TradabilityTracker` and
`require`), eligibility (`resolve_gate`), the rating policy (`decide_rating`), and the projection
itself (`project`). `submit_once` then submits the resulting order set at most once, and the
ledger is
what makes that true.
"""

from __future__ import annotations

from nautilus_trader.decision_bridge.artifact import KNOWN_FIELDS as KNOWN_FIELDS
from nautilus_trader.decision_bridge.artifact import ResearchDecision as ResearchDecision
from nautilus_trader.decision_bridge.artifact import admit as admit
from nautilus_trader.decision_bridge.artifact import artifact_digest as artifact_digest
from nautilus_trader.decision_bridge.artifact import iso_to_unix_nanos as iso_to_unix_nanos
from nautilus_trader.decision_bridge.carrier import (
    ResearchDecisionCarrier as ResearchDecisionCarrier,
)
from nautilus_trader.decision_bridge.contract import GATE_SEVERITY_MAPPING as GATE_SEVERITY_MAPPING
from nautilus_trader.decision_bridge.contract import (
    REGIME_SEVERITY_MAPPING as REGIME_SEVERITY_MAPPING,
)
from nautilus_trader.decision_bridge.contract import STAGE_ORDER as STAGE_ORDER
from nautilus_trader.decision_bridge.contract import Diagnostic as Diagnostic
from nautilus_trader.decision_bridge.contract import Outcome as Outcome
from nautilus_trader.decision_bridge.contract import Refusal as Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode as RefusalCode
from nautilus_trader.decision_bridge.contract import RefusalLog as RefusalLog
from nautilus_trader.decision_bridge.contract import RefusalRecord as RefusalRecord
from nautilus_trader.decision_bridge.contract import Severity as Severity
from nautilus_trader.decision_bridge.contract import Stage as Stage
from nautilus_trader.decision_bridge.contract import gate_severity as gate_severity
from nautilus_trader.decision_bridge.contract import most_restrictive as most_restrictive
from nautilus_trader.decision_bridge.contract import stage_of as stage_of
from nautilus_trader.decision_bridge.execution import DENIAL_CODES as DENIAL_CODES
from nautilus_trader.decision_bridge.execution import (
    RETRYABLE_DENIAL_CODES as RETRYABLE_DENIAL_CODES,
)
from nautilus_trader.decision_bridge.execution import RetryDecision as RetryDecision
from nautilus_trader.decision_bridge.execution import RetryPolicy as RetryPolicy
from nautilus_trader.decision_bridge.execution import SubmissionResult as SubmissionResult
from nautilus_trader.decision_bridge.execution import (
    classify_engine_message as classify_engine_message,
)
from nautilus_trader.decision_bridge.execution import leading_denial_code as leading_denial_code
from nautilus_trader.decision_bridge.execution import pipeline_config_for as pipeline_config_for
from nautilus_trader.decision_bridge.execution import position_state as position_state
from nautilus_trader.decision_bridge.execution import record_decision as record_decision
from nautilus_trader.decision_bridge.execution import record_engine_event as record_engine_event
from nautilus_trader.decision_bridge.execution import record_fill_once as record_fill_once
from nautilus_trader.decision_bridge.execution import retry_decision as retry_decision
from nautilus_trader.decision_bridge.execution import submit_once as submit_once
from nautilus_trader.decision_bridge.gate import DispositionConfig as DispositionConfig
from nautilus_trader.decision_bridge.gate import GateResolution as GateResolution
from nautilus_trader.decision_bridge.gate import resolve_gate as resolve_gate
from nautilus_trader.decision_bridge.gate import strength_for as strength_for
from nautilus_trader.decision_bridge.identity import ORDER_ID_MAX_LENGTH as ORDER_ID_MAX_LENGTH
from nautilus_trader.decision_bridge.identity import ORDER_SCHEME as ORDER_SCHEME
from nautilus_trader.decision_bridge.identity import OrderIdentity as OrderIdentity
from nautilus_trader.decision_bridge.identity import OrderRole as OrderRole
from nautilus_trader.decision_bridge.identity import encode_order_id as encode_order_id
from nautilus_trader.decision_bridge.identity import is_bridge_order_id as is_bridge_order_id
from nautilus_trader.decision_bridge.identity import order_identity as order_identity
from nautilus_trader.decision_bridge.ledger import ADMISSION_RESULTS as ADMISSION_RESULTS
from nautilus_trader.decision_bridge.ledger import EXPOSURE_STAGES as EXPOSURE_STAGES
from nautilus_trader.decision_bridge.ledger import UNATTRIBUTED as UNATTRIBUTED
from nautilus_trader.decision_bridge.ledger import AdmissionLedger as AdmissionLedger
from nautilus_trader.decision_bridge.ledger import DecisionRecord as DecisionRecord
from nautilus_trader.decision_bridge.ledger import DuplicateArrival as DuplicateArrival
from nautilus_trader.decision_bridge.numeric import DEFAULT_SCALES as DEFAULT_SCALES
from nautilus_trader.decision_bridge.numeric import DeclaredScales as DeclaredScales
from nautilus_trader.decision_bridge.numeric import allocation_fraction as allocation_fraction
from nautilus_trader.decision_bridge.numeric import (
    is_allocation_in_domain as is_allocation_in_domain,
)
from nautilus_trader.decision_bridge.numeric import to_decimal as to_decimal
from nautilus_trader.decision_bridge.projection import ProjectionConfig as ProjectionConfig
from nautilus_trader.decision_bridge.projection import ProjectionContext as ProjectionContext
from nautilus_trader.decision_bridge.projection import ProjectionResult as ProjectionResult
from nautilus_trader.decision_bridge.projection import project as project
from nautilus_trader.decision_bridge.ratings import POLICY as POLICY
from nautilus_trader.decision_bridge.ratings import VIEW_MAPPING as VIEW_MAPPING
from nautilus_trader.decision_bridge.ratings import PositionState as PositionState
from nautilus_trader.decision_bridge.ratings import RatingDecision as RatingDecision
from nautilus_trader.decision_bridge.ratings import RatingResolution as RatingResolution
from nautilus_trader.decision_bridge.ratings import ResearchView as ResearchView
from nautilus_trader.decision_bridge.ratings import decide_rating as decide_rating
from nautilus_trader.decision_bridge.ratings import is_policy_total as is_policy_total
from nautilus_trader.decision_bridge.ratings import view_of as view_of
from nautilus_trader.decision_bridge.risk import UNREACHABLE_FROM_PYTHON as UNREACHABLE_FROM_PYTHON
from nautilus_trader.decision_bridge.risk import RiskLimits as RiskLimits
from nautilus_trader.decision_bridge.risk import (
    build_risk_engine_config as build_risk_engine_config,
)
from nautilus_trader.decision_bridge.temporal import Actionability as Actionability
from nautilus_trader.decision_bridge.temporal import JsonCalendarView as JsonCalendarView
from nautilus_trader.decision_bridge.temporal import resolve_actionability as resolve_actionability
from nautilus_trader.decision_bridge.tradability import Tradability as Tradability
from nautilus_trader.decision_bridge.tradability import TradabilityTracker as TradabilityTracker
from nautilus_trader.decision_bridge.tradability import combine as combine
from nautilus_trader.decision_bridge.tradability import from_status as from_status
from nautilus_trader.decision_bridge.tradability import require as require


__all__ = [
    "ADMISSION_RESULTS",
    "DEFAULT_SCALES",
    "DENIAL_CODES",
    "EXPOSURE_STAGES",
    "GATE_SEVERITY_MAPPING",
    "KNOWN_FIELDS",
    "ORDER_ID_MAX_LENGTH",
    "ORDER_SCHEME",
    "POLICY",
    "REGIME_SEVERITY_MAPPING",
    "RETRYABLE_DENIAL_CODES",
    "STAGE_ORDER",
    "UNATTRIBUTED",
    "UNREACHABLE_FROM_PYTHON",
    "VIEW_MAPPING",
    "Actionability",
    "AdmissionLedger",
    "DecisionRecord",
    "DeclaredScales",
    "Diagnostic",
    "DispositionConfig",
    "DuplicateArrival",
    "GateResolution",
    "JsonCalendarView",
    "OrderIdentity",
    "OrderRole",
    "Outcome",
    "PositionState",
    "ProjectionConfig",
    "ProjectionContext",
    "ProjectionResult",
    "RatingDecision",
    "RatingResolution",
    "Refusal",
    "RefusalCode",
    "RefusalLog",
    "RefusalRecord",
    "ResearchDecision",
    "ResearchDecisionCarrier",
    "ResearchView",
    "RetryDecision",
    "RetryPolicy",
    "RiskLimits",
    "Severity",
    "Stage",
    "SubmissionResult",
    "Tradability",
    "TradabilityTracker",
    "admit",
    "allocation_fraction",
    "artifact_digest",
    "build_risk_engine_config",
    "classify_engine_message",
    "combine",
    "decide_rating",
    "from_status",
    "gate_severity",
    "is_allocation_in_domain",
    "is_bridge_order_id",
    "is_policy_total",
    "iso_to_unix_nanos",
    "leading_denial_code",
    "most_restrictive",
    "order_identity",
    "pipeline_config_for",
    "position_state",
    "project",
    "record_decision",
    "record_engine_event",
    "record_fill_once",
    "require",
    "resolve_actionability",
    "resolve_gate",
    "retry_decision",
    "stage_of",
    "strength_for",
    "submit_once",
    "to_decimal",
    "view_of",
]
