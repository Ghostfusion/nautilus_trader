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
Tests for the portfolio target value type.
"""

from __future__ import annotations

import pytest

from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import Quantity
from nautilus_trader.model import Target


def test_target_from_quantity_exposes_the_exposure() -> None:
    """
    Test a quantity target exposes its kind and value.
    """
    target = Target.from_quantity(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        quantity=Quantity.from_str("1.5"),
        ts_event=1_000,
        ts_init=2_000,
    )

    assert target.instrument_id == InstrumentId.from_str("AAPL.XNYS")
    assert target.kind == "QUANTITY"
    assert target.value.kind == "QUANTITY"
    assert target.value.quantity == Quantity.from_str("1.5")
    assert target.value.weight is None
    assert target.value.notional is None
    assert target.ts_event == 1_000
    assert target.ts_init == 2_000
    assert target.is_flat() is False


def test_target_from_weight_exposes_the_exposure() -> None:
    """
    Test a weight target exposes its kind and value.
    """
    target = Target.from_weight(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        weight=0.25,
    )

    assert target.kind == "WEIGHT"
    assert target.value.kind == "WEIGHT"
    assert target.value.weight == 0.25
    assert target.value.quantity is None
    assert target.value.notional is None
    assert target.is_flat() is False


def test_target_from_notional_exposes_the_exposure() -> None:
    """
    Test a notional target exposes its kind and value.
    """
    target = Target.from_notional(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        notional=Money.from_str("1000 USD"),
    )

    assert target.kind == "NOTIONAL"
    assert target.value.kind == "NOTIONAL"
    assert target.value.notional == Money.from_str("1000 USD")
    assert target.value.quantity is None
    assert target.value.weight is None
    assert target.is_flat() is False


def test_target_rejects_a_non_finite_weight() -> None:
    """
    Test a weight must be finite.
    """
    with pytest.raises(ValueError):
        Target.from_weight(
            instrument_id=InstrumentId.from_str("AAPL.XNYS"),
            weight=float("nan"),
        )


def test_target_is_flat_for_a_zero_quantity() -> None:
    """
    Test a zero quantity target is flat.
    """
    flat = Target.from_quantity(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        quantity=Quantity.from_str("0"),
    )
    open_target = Target.from_quantity(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        quantity=Quantity.from_str("0.01"),
    )

    assert flat.is_flat() is True
    assert open_target.is_flat() is False


def test_target_is_flat_for_a_zero_weight() -> None:
    """
    Test a zero weight target is flat.
    """
    flat = Target.from_weight(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        weight=0.0,
    )
    open_target = Target.from_weight(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        weight=-0.25,
    )

    assert flat.is_flat() is True
    assert open_target.is_flat() is False


def test_target_is_flat_for_a_zero_notional() -> None:
    """
    Test a zero notional target is flat.
    """
    flat = Target.from_notional(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        notional=Money.from_str("0 USD"),
    )
    open_target = Target.from_notional(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        notional=Money.from_str("-100 USD"),
    )

    assert flat.is_flat() is True
    assert open_target.is_flat() is False