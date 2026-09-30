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
Integration tests for a universe driven by a backtest run.

The universe component is registered with the engine's trader, so the run drives its lifecycle:
its selection timer starts with the run, its membership changes reach a subscribed strategy, and
its member subscriptions are released when the run stops.
"""

from __future__ import annotations

from datetime import UTC
from datetime import datetime
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.core import UUID4
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import ClientOrderId
from nautilus_trader.model import Currency
from nautilus_trader.model import Equity
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import MarketOrder
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Symbol
from nautilus_trader.model import TimeInForce
from nautilus_trader.model import UniverseMembershipState
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.trading import ScheduledUniverseRule
from nautilus_trader.trading import StaticUniverseRule
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from nautilus_trader.trading import Universe
from nautilus_trader.trading import UniverseDefinition
from nautilus_trader.trading import UniverseRemovalPolicy
from nautilus_trader.trading import UniverseSubscription


def _equity(symbol: str, isin: str) -> Equity:
    return Equity(
        instrument_id=InstrumentId.from_str(f"{symbol}.XNYS"),
        raw_symbol=Symbol(symbol),
        isin=isin,
        currency=Currency.from_str("USD"),
        price_precision=2,
        price_increment=Price.from_str("0.01"),
        lot_size=Quantity.from_int(100),
        ts_event=0,
        ts_init=0,
    )


AAPL = _equity("AAPL", "US0378331005")
MSFT = _equity("MSFT", "US5949181045")

TS_START = int(datetime(2024, 12, 2, tzinfo=UTC).timestamp() * 1_000_000_000)
INTERVAL_NS = 60 * 60 * 1_000_000_000
QUOTE_COUNT = 8
UNIVERSE = "equities"


class MembershipConfig(StrategyConfig):
    """
    Record the membership changes of a universe and trade each instrument once.
    """

    def __init__(self, *, universe: str, trade_size: str, **_kwargs: object) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.universe = universe
        self.trade_size = trade_size


class MembershipStrategy(Strategy):
    """
    Subscribe to a universe, record its changes, and trade each instrument on becoming active.
    """

    RECEIVED: list[tuple[str, str]] = []
    ORDERED: list[str] = []

    def __init__(self, config: MembershipConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._universe = config.universe
        self._qty = Quantity.from_str(config.trade_size)

    def on_start(self) -> None:
        """
        On start.
        """
        self.subscribe_universe_changes(self._universe)

    def on_universe_changed(self, change) -> None:
        """
        On universe membership change.
        """
        type(self).RECEIVED.append((str(change.state), str(change.instrument_id)))

        if change.state != UniverseMembershipState.ACTIVE:
            return

        type(self).ORDERED.append(str(change.instrument_id))

        trader_id = self.trader_id
        assert trader_id is not None

        order = MarketOrder(
            trader_id=trader_id,
            strategy_id=self.strategy_id,
            instrument_id=change.instrument_id,
            client_order_id=ClientOrderId(f"{self.strategy_id}-{UUID4()}"),
            order_side=OrderSide.BUY,
            quantity=self._qty,
            init_id=UUID4(),
            ts_init=self.clock.timestamp_ns(),
            time_in_force=TimeInForce.GTC,
            reduce_only=False,
            quote_quantity=False,
            contingency_type=None,
        )
        self.submit_order(order)


def _quotes() -> list[QuoteTick]:
    quotes: list[QuoteTick] = []
    for instrument in (AAPL, MSFT):
        for index in range(QUOTE_COUNT):
            ts = TS_START + index * INTERVAL_NS
            quotes.append(
                QuoteTick(
                    instrument_id=instrument.id,
                    bid_price=Price.from_str("200.00"),
                    ask_price=Price.from_str("200.01"),
                    bid_size=Quantity(10_000, precision=0),
                    ask_size=Quantity(10_000, precision=0),
                    ts_event=ts,
                    ts_init=ts,
                ),
            )
    return quotes


def _definition(policy: UniverseRemovalPolicy | None = None) -> UniverseDefinition:
    definition = UniverseDefinition(
        UNIVERSE,
        Venue("XNYS"),
        ScheduledUniverseRule(
            UNIVERSE,
            [
                (TS_START + INTERVAL_NS, [AAPL.id]),
                (TS_START + 3 * INTERVAL_NS, [AAPL.id, MSFT.id]),
                (TS_START + 6 * INTERVAL_NS, [AAPL.id]),
            ],
        ),
    )
    definition.set_selection_interval_ns(INTERVAL_NS)

    if policy is not None:
        definition.set_removal_policy(policy)

    return definition


def _run(policy: UniverseRemovalPolicy | None = None) -> tuple[Universe, list[tuple[str, str]], list[str]]:
    MembershipStrategy.RECEIVED = []
    MembershipStrategy.ORDERED = []
    universe = Universe(_definition(policy))

    config = BacktestEngineConfig(
        bypass_logging=True,
        run_analysis=False,
        risk_engine=RiskEngineConfig(bypass=True),
    )

    xnys = Venue("XNYS")
    usd = Currency.from_str("USD")

    engine = BacktestEngine(config)

    try:
        engine.add_venue(
            venue=xnys,
            oms_type=OmsType.NETTING,
            account_type=AccountType.CASH,
            base_currency=usd,
            starting_balances=[Money(1_000_000.0, usd)],
            fee_model=MakerTakerFeeModel(
                maker_rate=Decimal("0"),
                taker_rate=Decimal("0"),
            ),
        )
        engine.add_instrument(AAPL)
        engine.add_instrument(MSFT)
        engine.add_data(_quotes())
        engine.add_universe(universe)
        engine.add_strategy(
            MembershipStrategy(
                MembershipConfig(universe=UNIVERSE, trade_size="100"),
            ),
        )
        engine.run(start=TS_START, end=TS_START + QUOTE_COUNT * INTERVAL_NS)
    finally:
        engine.dispose()

    return universe, list(MembershipStrategy.RECEIVED), list(MembershipStrategy.ORDERED)


def test_membership_changes_are_delivered_and_traded() -> None:
    """
    A scheduled universe trades each instrument as it becomes active.
    """
    universe, received, ordered = _run()

    assert received == [
        ("ADDED", str(AAPL.id)),
        ("ACTIVE", str(AAPL.id)),
        ("ADDED", str(MSFT.id)),
        ("ACTIVE", str(MSFT.id)),
        ("REMOVING", str(MSFT.id)),
        ("REMOVING", str(AAPL.id)),
        ("REMOVED", str(AAPL.id)),
        ("REMOVED", str(MSFT.id)),
    ]
    assert ordered == [str(AAPL.id), str(MSFT.id)]

    # Stopping the run released every claim, including the held removal of MSFT
    assert universe.state(AAPL.id) == UniverseMembershipState.REMOVED
    assert universe.state(MSFT.id) == UniverseMembershipState.REMOVED
    assert universe.member_count == 2


def test_removal_of_a_member_with_an_open_position_is_held() -> None:
    """
    A departing member with an open position keeps its subscriptions until the run stops.

    The universe reports the condition when it evaluates the removal, and the state stays REMOVING
    rather than completing on the selection step.
    """
    _, received, _ = _run()

    held = received.index(("REMOVING", str(MSFT.id)))

    assert ("ACTIVE", str(MSFT.id)) in received[:held]
    # The only release of MSFT is the last change of the run, so its removal was held throughout
    assert received.index(("REMOVED", str(MSFT.id))) == len(received) - 1


def test_release_regardless_policy_completes_the_removal() -> None:
    """
    A policy that ignores open positions completes the removal.
    """
    universe, received, _ = _run(UniverseRemovalPolicy.RELEASE_REGARDLESS)

    # The removal completed on the selection step itself, before the run stopped
    assert received.index(("REMOVED", str(MSFT.id))) < received.index(
        ("REMOVING", str(AAPL.id)),
    )
    assert universe.state(MSFT.id) == UniverseMembershipState.REMOVED
    assert not universe.is_member(MSFT.id)


def test_scheduled_rule_selects_the_effective_set() -> None:
    """
    The declarative rule is a pure function of the instant it is asked about.
    """
    rule = ScheduledUniverseRule(
        UNIVERSE,
        [
            (TS_START, [AAPL.id]),
            (TS_START + INTERVAL_NS, [AAPL.id, MSFT.id]),
        ],
    )

    assert rule.select(TS_START) == [AAPL.id]
    assert rule.select(TS_START + INTERVAL_NS) == [AAPL.id, MSFT.id]
    assert rule.select(TS_START + 10 * INTERVAL_NS) == [AAPL.id, MSFT.id]


def test_static_rule_and_definition_configuration() -> None:
    """
    A definition carries the rule, the subscriptions, and the settings a run consumes.
    """
    rule = StaticUniverseRule(UNIVERSE, [MSFT.id, AAPL.id])

    assert rule.select(0) == [AAPL.id, MSFT.id]

    definition = UniverseDefinition(UNIVERSE, Venue("XNYS"), rule)

    assert definition.name == UNIVERSE
    assert definition.venue == Venue("XNYS")
    assert definition.selection_interval_ns is None
    assert definition.removal_policy == UniverseRemovalPolicy.REQUIRE_FLAT
    assert definition.subscriptions == [
        UniverseSubscription.INSTRUMENT,
        UniverseSubscription.QUOTES,
        UniverseSubscription.TRADES,
    ]

    definition.set_selection_interval_ns(INTERVAL_NS)
    definition.set_subscriptions([UniverseSubscription.QUOTES])
    definition.set_removal_policy(UniverseRemovalPolicy.RELEASE_REGARDLESS)

    assert definition.selection_interval_ns == INTERVAL_NS
    assert definition.subscriptions == [UniverseSubscription.QUOTES]
    assert definition.removal_policy == UniverseRemovalPolicy.RELEASE_REGARDLESS
