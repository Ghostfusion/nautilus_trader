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
Tests for the trading signal value type.
"""

from __future__ import annotations

import pytest

from nautilus_trader.model import InstrumentId
from nautilus_trader.model import SignalDirection
from nautilus_trader.model import TradingSignal


def test_trading_signal_exposes_the_statement_of_view() -> None:
    """
    Test a signal exposes its direction, horizon, magnitude, source, and expiry.
    """
    signal = TradingSignal(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        direction=SignalDirection.LONG,
        horizon_ns=3_600_000_000_000,
        strength=0.75,
        source="ema_cross",
        expiry_ns=5_000,
        provenance={"model": "ema"},
        ts_event=1_000,
        ts_init=2_000,
    )

    assert signal.instrument_id == InstrumentId.from_str("AAPL.XNYS")
    assert signal.direction == SignalDirection.LONG
    assert signal.horizon_ns == 3_600_000_000_000
    assert signal.strength == 0.75
    assert signal.source == "ema_cross"
    assert signal.expiry_ns == 5_000
    assert signal.provenance == {"model": "ema"}
    assert signal.ts_event == 1_000
    assert signal.ts_init == 2_000


def test_trading_signal_optional_fields_default_to_absent() -> None:
    """
    Test a signal built from direction alone reports absent optional fields.
    """
    signal = TradingSignal(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        direction=SignalDirection.FLAT,
    )

    assert signal.horizon_ns is None
    assert signal.strength is None
    assert signal.source is None
    assert signal.expiry_ns is None
    assert signal.provenance is None
    assert signal.ts_event == 0
    assert signal.ts_init == 0


def test_trading_signal_rejects_a_negative_strength() -> None:
    """
    Test a strength is a magnitude and cannot be negative.
    """
    with pytest.raises(ValueError):
        TradingSignal(
            instrument_id=InstrumentId.from_str("AAPL.XNYS"),
            direction=SignalDirection.LONG,
            strength=-0.1,
        )


def test_trading_signal_rejects_a_non_finite_strength() -> None:
    """
    Test a strength must be finite.
    """
    with pytest.raises(ValueError):
        TradingSignal(
            instrument_id=InstrumentId.from_str("AAPL.XNYS"),
            direction=SignalDirection.LONG,
            strength=float("nan"),
        )


def test_trading_signal_rejects_a_zero_horizon() -> None:
    """
    Test a horizon must be a positive duration.
    """
    with pytest.raises(ValueError):
        TradingSignal(
            instrument_id=InstrumentId.from_str("AAPL.XNYS"),
            direction=SignalDirection.LONG,
            horizon_ns=0,
        )


def test_trading_signal_rejects_an_expiry_before_the_event_time() -> None:
    """
    Test a view cannot lapse before it is stated.
    """
    with pytest.raises(ValueError):
        TradingSignal(
            instrument_id=InstrumentId.from_str("AAPL.XNYS"),
            direction=SignalDirection.LONG,
            expiry_ns=999,
            ts_event=1_000,
        )


def test_trading_signal_expires_at_and_after_the_expiry() -> None:
    """
    Test a signal has not lapsed before its expiry and has at and after it.
    """
    signal = TradingSignal(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        direction=SignalDirection.SHORT,
        expiry_ns=5_000,
        ts_event=1_000,
    )

    assert signal.is_expired(4_999) is False
    assert signal.is_expired(5_000) is True
    assert signal.is_expired(5_001) is True


def test_trading_signal_without_an_expiry_never_expires() -> None:
    """
    Test a signal with no expiry does not lapse.
    """
    signal = TradingSignal(
        instrument_id=InstrumentId.from_str("AAPL.XNYS"),
        direction=SignalDirection.LONG,
    )

    assert signal.is_expired(0) is False
    assert signal.is_expired(2**63) is False


def test_signal_direction_round_trips_through_strings() -> None:
    """
    Test the direction parses from its canonical string.
    """
    assert SignalDirection.from_str("long") == SignalDirection.LONG
    assert str(SignalDirection.SHORT) == "SHORT"
    assert SignalDirection("FLAT") == SignalDirection.FLAT