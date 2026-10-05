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
Test execution analytics behavior.
"""

from __future__ import annotations

import pytest

from nautilus_trader.model import ClientOrderId
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.trading import METRIC_ARRIVAL_SLIPPAGE_BPS
from nautilus_trader.trading import METRIC_IMPLEMENTATION_SHORTFALL_BPS
from nautilus_trader.trading import METRIC_VWAP_SLIPPAGE_BPS
from nautilus_trader.trading import BenchmarkInterval
from nautilus_trader.trading import ExecutionObserver
from nautilus_trader.trading import ExecutionTerms
from nautilus_trader.trading import MetricUnits
from nautilus_trader.trading import QuoteObservation
from nautilus_trader.trading import TradeObservation
from nautilus_trader.trading import UnavailableReason


INSTRUMENT_ID = InstrumentId.from_str("EUR/USD.PARITY")
HORIZON_NS = 60_000_000


def _pinned_observer() -> ExecutionObserver:
    """
    Build the scenario the Rust observer tests pin.

    A BUY 100 parent over a 60 ms horizon, decision 100.00, arrival 101.00, parent submitted at
    1.8s, benchmark interval [2.0s, 2.3s], and fills of 50 at 102.00 and 50 at 102.02.
    """
    terms = ExecutionTerms(
        instrument_id=INSTRUMENT_ID,
        order_side=OrderSide.BUY,
        quantity=Quantity.from_int(100),
        limit_price=None,
        horizon_ns=HORIZON_NS,
    )
    observer = ExecutionObserver(terms).with_benchmark(
        BenchmarkInterval(2_000_000_000, 2_300_000_000),
    )
    observer.set_parent_submitted(1_800_000_000)
    observer.set_decision(1_850_000_000, Price.from_str("100.00"))
    observer.set_arrival(1_900_000_000, Price.from_str("101.00"))

    observer.observe_quote(
        QuoteObservation(1_900_000_000, Price.from_str("100.50"), Price.from_str("101.50"))
    )
    observer.observe_quote(
        QuoteObservation(2_000_000_000, Price.from_str("100.00"), Price.from_str("101.00"))
    )
    observer.observe_quote(
        QuoteObservation(2_300_000_000, Price.from_str("101.00"), Price.from_str("102.00"))
    )
    observer.observe_trade(
        TradeObservation(2_000_000_000, Price.from_str("100.00"), Quantity.from_int(1))
    )
    observer.observe_trade(
        TradeObservation(2_300_000_000, Price.from_str("102.00"), Quantity.from_int(1))
    )

    c1 = ClientOrderId("C1")
    c2 = ClientOrderId("C2")
    observer.observe_child_submitted(c1, 2_350_000_000, Quantity.from_int(50))
    observer.observe_fill(c1, 2_400_000_000, Quantity.from_int(50), Price.from_str("102.00"))
    observer.observe_child_submitted(c2, 2_550_000_000, Quantity.from_int(50))
    observer.observe_fill(c2, 2_600_000_000, Quantity.from_int(50), Price.from_str("102.02"))

    # Quotes for adverse selection: fill 1 sees mid 100.50 then 100.00, fill 2 sees 101.50 then 101.00.
    observer.observe_quote(
        QuoteObservation(2_400_000_000, Price.from_str("100.00"), Price.from_str("101.00"))
    )
    observer.observe_quote(
        QuoteObservation(2_460_000_000, Price.from_str("99.50"), Price.from_str("100.50"))
    )
    observer.observe_quote(
        QuoteObservation(2_600_000_000, Price.from_str("101.00"), Price.from_str("102.00"))
    )
    observer.observe_quote(
        QuoteObservation(2_660_000_000, Price.from_str("100.50"), Price.from_str("101.50"))
    )

    return observer


def test_pinned_slippage_metrics_match_hand_computed_values() -> None:
    """
    Test the exposed slippage metrics carry the hand-computed basis points.
    """
    metrics = _pinned_observer().metrics()

    # fill VWAP 102.01: ((102.01 - 100.00) / 100.00) * 10_000 against the decision price,
    # then ((102.01 - 101.00) / 101.00) * 10_000 against the arrival price and the interval VWAP.
    assert metrics.implementation_shortfall_bps.value == pytest.approx(201.0, abs=1e-9)
    assert metrics.arrival_slippage_bps.value == pytest.approx(100.0, abs=1e-9)
    assert metrics.vwap_slippage_bps.value == pytest.approx(100.0, abs=1e-9)
    assert metrics.twap_slippage_bps.value == pytest.approx(100.0, abs=1e-9)
    assert metrics.spread_capture.value == pytest.approx(-2.02, abs=1e-9)
    assert metrics.fill_ratio.value == pytest.approx(1.0, abs=1e-9)
    assert metrics.adverse_selection.value == pytest.approx(0.5, abs=1e-9)

    # Every value travels with the declaration that defines it.
    shortfall = metrics.implementation_shortfall_bps
    assert shortfall.declaration.metric_id == METRIC_IMPLEMENTATION_SHORTFALL_BPS
    assert shortfall.declaration.units == MetricUnits.BASIS_POINTS
    assert shortfall.declaration.reference_price is not None
    assert metrics.arrival_slippage_bps.declaration.metric_id == METRIC_ARRIVAL_SLIPPAGE_BPS
    assert metrics.vwap_slippage_bps.declaration.metric_id == METRIC_VWAP_SLIPPAGE_BPS
    assert metrics.adverse_selection.declaration.horizon_ns == HORIZON_NS


def test_undefined_metric_reports_none_with_its_reason() -> None:
    """
    Test an undefined metric is `None` with a reason, never `0.0`, while a genuine zero stays zero.
    """
    metrics = _pinned_observer().metrics()

    # No parent limit price was declared, so price improvement is undefined.
    assert metrics.price_improvement.value is None
    assert metrics.price_improvement.reason == UnavailableReason.NO_REFERENCE_PRICE
    assert metrics.price_improvement.is_available() is False

    # No child was cancelled, which is a genuine zero rather than an absence.
    assert metrics.cancel_ratio.value == 0.0


def test_observer_takes_model_ticks() -> None:
    """
    Test the observer accepts the model's quote tick and values a passive fill against it.
    """
    terms = ExecutionTerms(
        instrument_id=INSTRUMENT_ID,
        order_side=OrderSide.BUY,
        quantity=Quantity.from_int(10),
        limit_price=None,
        horizon_ns=None,
    )
    observer = ExecutionObserver(terms)
    observer.observe_quote_tick(
        QuoteTick(
            INSTRUMENT_ID,
            Price.from_str("100.50"),
            Price.from_str("101.50"),
            Quantity.from_int(1),
            Quantity.from_int(1),
            1_000_000_000,
            1_000_000_000,
        ),
    )
    observer.set_arrival(1_000_000_000, Price.from_str("101.00"))
    observer.observe_fill(
        ClientOrderId("P1"),
        1_100_000_000,
        Quantity.from_int(10),
        Price.from_str("100.50"),
    )

    # A passive BUY filling at the bid 100.50 against an arrival mid of 101.00 and half-spread 0.50.
    assert observer.metrics().spread_capture.value == pytest.approx(1.0, abs=1e-9)


def test_nanosecond_fields_round_trip_in_both_directions() -> None:
    """
    Test the timestamp and horizon fields survive the boundary as nanoseconds.
    """
    terms = ExecutionTerms(
        instrument_id=INSTRUMENT_ID,
        order_side=OrderSide.BUY,
        quantity=Quantity.from_int(10),
        limit_price=Price.from_str("99.00"),
        horizon_ns=HORIZON_NS,
    )
    observer = ExecutionObserver(terms)

    assert observer.terms.horizon_ns == HORIZON_NS
    assert observer.terms.limit_price == Price.from_str("99.00")
    assert observer.benchmark is None

    quote = QuoteObservation(1_900_000_000, Price.from_str("100.50"), Price.from_str("101.50"))
    assert quote.timestamp == 1_900_000_000
    assert quote.mid() == pytest.approx(101.0, abs=1e-9)
    assert quote.half_spread() == pytest.approx(0.5, abs=1e-9)

    interval = BenchmarkInterval(2_000_000_000, 2_300_000_000)
    assert interval.contains(2_000_000_000) is True
    assert interval.contains(2_300_000_000) is True
    assert interval.contains(2_300_000_001) is False
