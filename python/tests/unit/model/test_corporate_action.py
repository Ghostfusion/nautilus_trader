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
Tests for the corporate action data type.
"""

from __future__ import annotations

from decimal import Decimal

from nautilus_trader.model import CorporateAction
from nautilus_trader.model import CorporateActionType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Symbol


def test_corporate_action_exposes_the_split_fields() -> None:
    """
    Test a split action exposes its ratio and effective time.
    """
    action = CorporateAction(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        action=CorporateActionType.SPLIT,
        value=Decimal("4"),
        new_symbol=None,
        effective_ns=1_700_000_000_000_000_000,
        ts_event=1_700_000_000_000_000_001,
        ts_init=1_700_000_000_000_000_002,
    )

    assert action.instrument_id == InstrumentId.from_str("AAPL.XNYS")
    assert action.action == CorporateActionType.SPLIT
    assert action.value == Decimal("4")
    assert action.new_symbol is None
    assert action.effective_ns == 1_700_000_000_000_000_000
    assert action.ts_event == 1_700_000_000_000_000_001
    assert action.ts_init == 1_700_000_000_000_000_002


def test_corporate_action_exposes_a_symbol_change() -> None:
    """
    Test a symbol change carries the new venue symbol.
    """
    action = CorporateAction(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        action=CorporateActionType.SYMBOL_CHANGE,
        value=Decimal("0"),
        new_symbol=Symbol("AAPL.NEW"),
        effective_ns=1,
        ts_event=2,
        ts_init=3,
    )

    assert action.action == CorporateActionType.SYMBOL_CHANGE
    assert action.new_symbol == Symbol("AAPL.NEW")


def test_corporate_action_type_round_trips_through_strings() -> None:
    """
    Test the action kind parses from its canonical string.
    """
    assert CorporateActionType.from_str("SPLIT") == CorporateActionType.SPLIT
    assert CorporateActionType.from_str("symbol_change") == CorporateActionType.SYMBOL_CHANGE
    assert str(CorporateActionType.DELISTING) == "DELISTING"
    assert CorporateActionType("DIVIDEND") == CorporateActionType.DIVIDEND


def test_corporate_action_metadata_names_the_instrument() -> None:
    """
    Test the action reports the instrument metadata used by the catalog.
    """
    metadata = CorporateAction.get_metadata(InstrumentId.from_str("AAPL.XNYS"))

    assert metadata["instrument_id"] == "AAPL.XNYS"
