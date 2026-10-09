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
Signal intents, arbitration between them, and the scoring of an entry decision.

Three declarations that belong to a strategy but are usually written as code inside it: what an
intention is (`SignalSide`, `SignalIntent`), how competing intentions on one instrument are
arbitrated (`ConflictPolicy`), and how a set of conditions becomes a scored entry decision
(`ConfluenceCard`). All three are pure: they read no cache and no clock, submit nothing, and record
every refusal with the reason it was made, so a decision can be audited after the fact and tested
without an engine.

The response vocabulary is closed. `ConflictReason` names every case in which an intention is not
admitted, and `ConfluenceResult` distinguishes a refusal for want of data (a rule whose input was
missing) from a refusal on the data (a rule that was evaluated and did not match).
"""

from nautilus_trader.signal_policy.conflict import DEFAULT_PRECEDENCE as DEFAULT_PRECEDENCE
from nautilus_trader.signal_policy.conflict import ConflictMode as ConflictMode
from nautilus_trader.signal_policy.conflict import ConflictPolicy as ConflictPolicy
from nautilus_trader.signal_policy.conflict import ConflictReason as ConflictReason
from nautilus_trader.signal_policy.conflict import ConflictRefusal as ConflictRefusal
from nautilus_trader.signal_policy.conflict import ConflictResolution as ConflictResolution
from nautilus_trader.signal_policy.conflict import InstrumentResolution as InstrumentResolution
from nautilus_trader.signal_policy.confluence import AllOf as AllOf
from nautilus_trader.signal_policy.confluence import AnyOf as AnyOf
from nautilus_trader.signal_policy.confluence import AtLeast as AtLeast
from nautilus_trader.signal_policy.confluence import Comparator as Comparator
from nautilus_trader.signal_policy.confluence import Condition as Condition
from nautilus_trader.signal_policy.confluence import ConfluenceCard as ConfluenceCard
from nautilus_trader.signal_policy.confluence import ConfluenceResult as ConfluenceResult
from nautilus_trader.signal_policy.confluence import GroupOutcome as GroupOutcome
from nautilus_trader.signal_policy.confluence import Not as Not
from nautilus_trader.signal_policy.confluence import RuleGroup as RuleGroup
from nautilus_trader.signal_policy.confluence import ScoreKind as ScoreKind
from nautilus_trader.signal_policy.confluence import ScoreRule as ScoreRule
from nautilus_trader.signal_policy.confluence import confluence_card as confluence_card
from nautilus_trader.signal_policy.confluence import evaluate as evaluate
from nautilus_trader.signal_policy.intent import PositionRequirement as PositionRequirement
from nautilus_trader.signal_policy.intent import PositionSide as PositionSide
from nautilus_trader.signal_policy.intent import SignalIntent as SignalIntent
from nautilus_trader.signal_policy.intent import SignalSide as SignalSide


__all__ = [
    "DEFAULT_PRECEDENCE",
    "AllOf",
    "AnyOf",
    "AtLeast",
    "Comparator",
    "Condition",
    "ConflictMode",
    "ConflictPolicy",
    "ConflictReason",
    "ConflictRefusal",
    "ConflictResolution",
    "ConfluenceCard",
    "ConfluenceResult",
    "GroupOutcome",
    "InstrumentResolution",
    "Not",
    "PositionRequirement",
    "PositionSide",
    "RuleGroup",
    "ScoreKind",
    "ScoreRule",
    "SignalIntent",
    "SignalSide",
    "confluence_card",
    "evaluate",
]
