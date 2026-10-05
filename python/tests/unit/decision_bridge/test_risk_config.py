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
Tests for the decision-bridge risk configuration.

The proof is against the real engine, not against the configuration object alone: a backtest runs
with the built configuration, an order that breaches the per-instrument notional cap is refused with
the engine's own denial naming the cap, and a cancellation of a resting order in the same run is not
refused. A round trip checks that the built configuration carries exactly the declared limits.

The count caps are covered too: the built configuration carries the declared cap into the engine,
the engine's own validation refuses an unsupported cap shape rather than the bridge guessing, and a
backtest with a tight count cap has its over-limit submission refused with the engine's own
`ORDER_COUNT_LIMIT_REACHED`. Revision 6 removed the binding gap these tests used to pin, so the
assertion that the keyword was rejected is gone rather than weakened.
"""

from __future__ import annotations

from decimal import Decimal

import pytest

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.decision_bridge.risk import UNREACHABLE_FROM_PYTHON
from nautilus_trader.decision_bridge.risk import RiskLimits
from nautilus_trader.decision_bridge.risk import build_risk_engine_config
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import OrderStatus
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskCap
from nautilus_trader.risk import RiskCapMetric
from nautilus_trader.risk import RiskCapScope
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.trading import Strategy
from tests.providers import TestInstrumentProvider


USDT = Currency.from_str("USDT")
ETHUSDT = TestInstrumentProvider.ethusdt_binance()

# 2020-09-13T12:26:40Z, so generated client order IDs are deterministic.
BASE_NS = 1_600_000_000_000_000_000
QUOTE_COUNT = 6

CAP = "100"
RESTING_PRICE = "1999.00"
RESTING_QUANTITY = "0.04000"

# A count cap loose enough that the notional probe never reaches it, and a second, tight one that a
# two-submission probe does; both are declared in the configuration's own units.
COUNT_CAP_LIMIT = 1000
COUNT_CAP_WINDOW_NS = 60_000_000_000
TIGHT_COUNT_CAP_LIMIT = 1
# 0.01 ETH at the probe's prices is about 20 USDT: comfortably above the instrument's 10 USDT
# minimum notional and comfortably below the 100 USDT notional cap, so a refusal in the count-cap
# test can only be the count cap's.
UNDER_CAP_MARKET_QUANTITY = "0.01000"
OVERSIZED_QUANTITY = "0.10000"


def _quotes(count: int = QUOTE_COUNT) -> list[QuoteTick]:
    """
    Build a deterministic quote stream around 2000.00 USDT.
    """
    ticks = []

    for i in range(count):
        mid = Decimal("2000.00") + Decimal("1.00") * i
        ticks.append(
            QuoteTick(
                instrument_id=ETHUSDT.id,
                bid_price=Price.from_decimal_dp(mid - Decimal("0.05"), ETHUSDT.price_precision),
                ask_price=Price.from_decimal_dp(mid + Decimal("0.05"), ETHUSDT.price_precision),
                bid_size=Quantity.from_decimal_dp(Decimal(10), ETHUSDT.size_precision),
                ask_size=Quantity.from_decimal_dp(Decimal(10), ETHUSDT.size_precision),
                ts_event=BASE_NS + (i * 1_000_000_000),
                ts_init=BASE_NS + (i * 1_000_000_000),
            ),
        )

    return ticks


def _engine(risk_engine: RiskEngineConfig) -> BacktestEngine:
    """
    Build a backtest engine with the given risk configuration.
    """
    engine = BacktestEngine(
        BacktestEngineConfig(
            bypass_logging=True,
            run_analysis=False,
            risk_engine=risk_engine,
        ),
    )
    engine.add_venue(
        venue=Venue("BINANCE"),
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        starting_balances=[Money(1_000_000.0, USDT)],
        base_currency=USDT,
        fee_model=MakerTakerFeeModel(
            maker_rate=Decimal(0),
            taker_rate=Decimal(0),
        ),
    )
    engine.add_instrument(ETHUSDT)

    return engine


class NotionalCapProbe(Strategy):
    """
    Submit one over-cap market order and one under-cap resting order, then cancel the resting order.

    The over-cap order must be refused; the under-cap order must be accepted and remain cancellable,
    so one run shows both that the cap binds and that a cancellation is not refused by it.
    """

    def __init__(self, config: object = None) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self.denied = []
        self.canceled = []
        self.oversized_id = None
        self.resting_id = None
        self._quote_count = 0

    def on_start(self) -> None:
        """
        Subscribe to quotes.
        """
        self.subscribe_quotes(ETHUSDT.id)

    def on_quote(self, _quote: QuoteTick) -> None:
        """
        Submit both orders on the first quote and cancel on the second.
        """
        self._quote_count += 1

        if self._quote_count == 1:
            oversized = self.order_factory.market(
                instrument_id=ETHUSDT.id,
                order_side=OrderSide.BUY,
                quantity=Quantity.from_str(OVERSIZED_QUANTITY),
            )
            self.oversized_id = oversized.client_order_id
            self.submit_order(oversized)

            resting = self.order_factory.limit(
                instrument_id=ETHUSDT.id,
                order_side=OrderSide.BUY,
                quantity=Quantity.from_str(RESTING_QUANTITY),
                price=Price.from_str(RESTING_PRICE),
            )
            self.resting_id = resting.client_order_id
            self.submit_order(resting)
        elif self._quote_count == 2:
            self.cancel_order(self.resting_id)

    def on_order_denied(self, event: object) -> None:
        """
        Record an order denial.
        """
        self.denied.append(event)

    def on_order_canceled(self, event: object) -> None:
        """
        Record an order cancellation.
        """
        self.canceled.append(event)


class CountCapProbe(Strategy):
    """
    Submit two under-notional market orders, one quote apart.

    The first is admitted and feeds the count cap; the second reaches the limit and must be denied by
    the count cap rather than by the notional cap, which both orders are far below.
    """

    def __init__(self, config: object = None) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self.submitted = []
        self.denied = []
        self.first_id = None
        self.second_id = None
        self._quote_count = 0

    def on_start(self) -> None:
        """
        Subscribe to quotes.
        """
        self.subscribe_quotes(ETHUSDT.id)

    def on_quote(self, _quote: QuoteTick) -> None:
        """
        Submit one under-notional order per quote, twice.
        """
        self._quote_count += 1
        if self._quote_count > 2:
            return

        order = self.order_factory.market(
            instrument_id=ETHUSDT.id,
            order_side=OrderSide.BUY,
            quantity=Quantity.from_str(UNDER_CAP_MARKET_QUANTITY),
        )
        if self._quote_count == 1:
            self.first_id = order.client_order_id
        else:
            self.second_id = order.client_order_id
        self.submitted.append(order.client_order_id)
        self.submit_order(order)

    def on_order_denied(self, event: object) -> None:
        """
        Record an order denial.
        """
        self.denied.append(event)


def _declared_limits() -> RiskLimits:
    """
    Return the desk's declared limits used by the engine-backed tests.
    """
    return RiskLimits(
        max_notional_per_order={str(ETHUSDT.id): CAP},
        count_caps=[
            RiskCap(
                RiskCapMetric.Submit,
                RiskCapScope.Global,
                COUNT_CAP_LIMIT,
                COUNT_CAP_WINDOW_NS,
            ),
        ],
        max_order_submit_rate="250/00:00:05",
        max_order_modify_rate="50/00:01:00",
        full_position_exit_venues=[Venue("BINANCE")],
    )


def _run_probe() -> tuple[BacktestEngine, NotionalCapProbe]:
    """
    Run the probe against a real engine configured from the declared limits.
    """
    engine = _engine(build_risk_engine_config(_declared_limits()))
    strategy = NotionalCapProbe()
    engine.add_strategy(strategy)
    engine.add_data(_quotes())
    engine.run()

    return engine, strategy


def test_built_config_round_trips_declared_limits() -> None:
    """
    Test the built configuration carries exactly the declared limits.
    """
    limits = _declared_limits()

    config = build_risk_engine_config(limits)

    assert config.max_notional_per_order == {str(ETHUSDT.id): CAP}
    assert len(config.count_caps) == 1
    assert config.count_caps[0].limit == COUNT_CAP_LIMIT
    assert config.count_caps[0].window == COUNT_CAP_WINDOW_NS
    assert config.max_order_submit_rate == "250/00:00:05"
    assert config.max_order_modify_rate == "50/00:01:00"
    assert config.full_position_exit_venues == [Venue("BINANCE")]
    assert config.bypass is False


def test_limits_require_at_least_one_notional_cap() -> None:
    """
    Test a declared limit set without a per-instrument cap is refused.
    """
    with pytest.raises(ValueError, match="max_notional_per_order"):
        RiskLimits(
            max_notional_per_order={},
            count_caps=[],
            max_order_submit_rate="250/00:00:05",
            max_order_modify_rate="50/00:01:00",
        )


def test_nothing_the_bridge_needs_is_unreachable_from_python() -> None:
    """
    Test the recorded reachability list is empty, the count cap having been its last entry.
    """
    assert UNREACHABLE_FROM_PYTHON == ()


def test_the_engine_validates_a_count_cap_rather_than_the_bridge() -> None:
    """
    Test an unsupported cap shape is refused by the engine's own validation.

    An `Active` cap counts the open order set and takes no window, so declaring one with a window
    must be refused by the same validation a backtest configuration goes through: the bridge
    forwards the cap and does not second-guess it.
    """
    with pytest.raises(ValueError, match="count_caps"):
        RiskEngineConfig(
            count_caps=[
                RiskCap(RiskCapMetric.Active, RiskCapScope.Global, 5, COUNT_CAP_WINDOW_NS),
            ],
        )


def test_a_count_cap_refuses_the_submission_that_reaches_it() -> None:
    """
    Test a count cap binds: the submission over the declared limit is denied naming the cap.

    Both submissions are far below the notional cap, so the refusal can only be the count cap's.
    """
    limits = RiskLimits(
        max_notional_per_order={str(ETHUSDT.id): CAP},
        count_caps=[
            RiskCap(
                RiskCapMetric.Submit,
                RiskCapScope.Instrument,
                TIGHT_COUNT_CAP_LIMIT,
                COUNT_CAP_WINDOW_NS,
            ),
        ],
        max_order_submit_rate="250/00:00:05",
        max_order_modify_rate="50/00:01:00",
    )
    engine = _engine(build_risk_engine_config(limits))
    strategy = CountCapProbe()
    engine.add_strategy(strategy)
    engine.add_data(_quotes())
    engine.run()

    try:
        assert len(strategy.submitted) == 2
        assert len(strategy.denied) == 1
        assert strategy.denied[0].client_order_id == strategy.second_id
        assert strategy.denied[0].reason.startswith("ORDER_COUNT_LIMIT_REACHED")
    finally:
        engine.dispose()


def test_notional_cap_breach_is_refused_naming_the_cap() -> None:
    """
    Test an order breaching the per-instrument notional cap is refused with the cap named.
    """
    _engine_used, strategy = _run_probe()

    try:
        assert len(strategy.denied) == 1

        reason = strategy.denied[0].reason

        assert reason.startswith("NOTIONAL_EXCEEDS_MAX_PER_ORDER")
        assert "max=100.00000000 USDT" in reason
    finally:
        _engine_used.dispose()


def test_cancellation_is_not_refused_by_a_notional_cap() -> None:
    """
    Test a cancellation is not refused while the same cap is refusing a submission.
    """
    engine, strategy = _run_probe()

    try:
        assert len(strategy.denied) == 1
        assert strategy.denied[0].reason.startswith("NOTIONAL_EXCEEDS_MAX_PER_ORDER")
        assert len(strategy.canceled) == 1
        assert strategy.canceled[0].client_order_id == strategy.resting_id
        assert engine.cache.order(strategy.resting_id).status == OrderStatus.CANCELED
    finally:
        engine.dispose()
