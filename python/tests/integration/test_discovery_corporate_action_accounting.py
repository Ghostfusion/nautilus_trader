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
Corporate-action accounting in value and in cash, as design criterion 6 requires it.

The criterion reconciles cash, quantity, position value and account equity across a dividend, a
split, a position held through an ex-date, and positions opened and closed around an action
boundary. The fixture records what the engine does, and does not adjust the expectation to fit it:
a dividend reaches the module as data and is applied to prices, and nothing credits an account.

The reconciliation is stated as arithmetic rather than as a comparison of two adjusted price
series. The value leg reconciles, because the adjustment series carries both actions. The cash leg
does not, because there is no cash line to reconcile it against, and the shortfall is exactly the
distribution the position was entitled to.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.execution import BestPriceFillModel
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import AdjustmentSeries
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import CorporateAction
from nautilus_trader.model import CorporateActionType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestInstrumentProvider


USD = Currency.from_str("USD")
INSTRUMENT = TestInstrumentProvider.aapl_equity()
INSTRUMENT_ID = INSTRUMENT.id
VENUE = INSTRUMENT.venue
BAR_TYPE = BarType.from_str(f"{INSTRUMENT_ID}-1-MINUTE-LAST-EXTERNAL")

MINUTE_NS = 60_000_000_000
TS_START = 1_704_067_200_000_000_000
SPLIT_NS = TS_START + MINUTE_NS + 30_000_000_000
DIVIDEND_NS = TS_START + 2 * MINUTE_NS + 30_000_000_000
ANNOUNCE_NS = TS_START - 10 * MINUTE_NS

# Two bars before the split and two after it, at the raw series the venue published.
RAW_PRICES = ("400.00", "400.00", "200.00", "200.00")
SHARES = 100
STARTING_BALANCE = "1_000_000 USD"
# The distribution the 100 shares were entitled to: 2.50 per share.
DISTRIBUTION = Decimal("250.00")
SPLIT_RATIO = Decimal(4)


@dataclass(frozen=True)
class Accounting:
    """
    The ledger lines the criterion reconciles, read after a run.
    """

    cash: Decimal
    cash_delta: Decimal
    quantity: Decimal
    actions: int
    order_prices: tuple[str, ...]


class AccountingConfig(StrategyConfig):
    """
    Configure the accounting strategy.
    """

    def __init__(self, *, bar_type: str, exit_on_last_bar: bool) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.bar_type = bar_type
        self.exit_on_last_bar = exit_on_last_bar


class AccountingStrategy(Strategy):
    """
    Hold one position through the action boundary, and optionally close it on the last bar.
    """

    def __init__(self, config: AccountingConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self._exit_on_last_bar = config.exit_on_last_bar
        self._bars_seen = 0
        self._entered = False
        self._exited = False
        self.actions: list[CorporateAction] = []

    def on_start(self) -> None:
        """
        Subscribe to the bar stream and to the instrument's corporate actions.
        """
        self.subscribe_bars(self._bar_type)
        self.subscribe_corporate_actions(INSTRUMENT_ID)

    def on_bar(self, _bar: Bar) -> None:
        """
        Enter on the first bar, and exit on the last bar when the scenario closes.
        """
        self._bars_seen += 1
        if not self._entered:
            self._entered = True
            self._submit(OrderSide.BUY)
            return
        if self._exit_on_last_bar and self._bars_seen == len(RAW_PRICES) and not self._exited:
            self._exited = True
            self._submit(OrderSide.SELL)

    def on_corporate_action(self, action: CorporateAction) -> None:
        """
        Record the action the module received.
        """
        self.actions.append(action)

    def _submit(self, side: OrderSide) -> None:
        self.submit_order(
            self.order_factory.market(
                instrument_id=INSTRUMENT_ID,
                order_side=side,
                quantity=Quantity.from_int(SHARES),
            ),
        )


def bars() -> list[Bar]:
    """
    Build the raw bar series, with the split's price effect already visible in it.
    """
    result: list[Bar] = []
    for index, value in enumerate(RAW_PRICES):
        price = Price.from_str(value)
        ts = TS_START + index * MINUTE_NS
        result.append(
            Bar(
                BAR_TYPE,
                price,
                price,
                price,
                price,
                Quantity.from_int(100),
                ts,
                ts,
            ),
        )
    return result


def actions() -> list[CorporateAction]:
    """
    Build the split and the dividend, announced before they take effect.
    """
    return [
        CorporateAction(
            INSTRUMENT_ID,
            CorporateActionType.SPLIT,
            SPLIT_RATIO,
            None,
            SPLIT_NS,
            ANNOUNCE_NS,
            ANNOUNCE_NS,
        ),
        CorporateAction(
            INSTRUMENT_ID,
            CorporateActionType.DIVIDEND,
            Decimal("2.50"),
            None,
            DIVIDEND_NS,
            ANNOUNCE_NS,
            ANNOUNCE_NS,
        ),
    ]


def run(*, exit_on_last_bar: bool = False, with_actions: bool = True) -> Accounting:
    """
    Run the accounting scenario and return the ledger lines it produced.
    """
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=VENUE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money(1_000_000.0, USD)],
        base_currency=USD,
        # A zero-slippage fill model, so every reconciled figure is the bar's own price.
        fill_model=BestPriceFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0),
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    engine.add_instrument(INSTRUMENT)
    engine.add_data(bars())
    if with_actions:
        engine.add_data(actions())
    strategy = AccountingStrategy(
        AccountingConfig(bar_type=str(BAR_TYPE), exit_on_last_bar=exit_on_last_bar),
    )
    engine.add_strategy(strategy)

    try:
        engine.run()
        account = engine.cache.account_for_venue(VENUE)
        cash = Decimal(0) if account is None else account.balances()[USD].total.as_decimal()
        orders = engine.cache.orders()
        return Accounting(
            cash=cash,
            cash_delta=cash - Decimal(1_000_000),
            quantity=Decimal(engine.portfolio.net_position(INSTRUMENT_ID)),
            actions=len(strategy.actions),
            order_prices=tuple(str(order.avg_px) for order in orders),
        )
    finally:
        engine.dispose()


def test_the_value_leg_adjusts_a_raw_price_for_both_actions() -> None:
    """
    Test that the adjustment series carries the split and the dividend.

    The arithmetic is the series' own: 400.00 times the split factor of one quarter, less the
    2.50 per share dividend, is 97.50. This is the leg that reconciles.
    """
    series = AdjustmentSeries(INSTRUMENT_ID, actions())

    assert series.split_factor(TS_START) == Decimal("0.25")
    assert series.dividends(TS_START) == Decimal("2.50")
    assert series.adjust(Price.from_str(RAW_PRICES[0]), TS_START).as_decimal() == Decimal("97.50")
    # After both actions the series is the raw series again.
    assert series.split_factor(DIVIDEND_NS) == Decimal(1)
    assert series.dividends(DIVIDEND_NS) == Decimal(0)


def test_both_action_records_reach_the_module() -> None:
    """
    Test that the action data is delivered while no cash line is created for it.

    Both records arrive at their effective instants, so the module is not blind to the action and
    the absence of a cash line is not an absence of data.
    """
    accounting = run()

    assert accounting.actions == 2


def test_a_held_dividend_credits_no_cash() -> None:
    """
    Test that the cash line records the purchase and nothing else.

    The 100 shares cost 40,000, which is the whole of the movement: the 250 the position was
    entitled to at the ex-date is not credited, so the cash reconciliation the criterion requires
    has nothing to reconcile the distribution against.
    """
    accounting = run()

    assert accounting.order_prices == (RAW_PRICES[0],)
    assert accounting.cash_delta == Decimal("-40000.00")
    assert accounting.cash_delta != Decimal("-40000.00") + DISTRIBUTION


def test_the_split_leaves_the_position_quantity_unchanged() -> None:
    """
    Test that the split is a price adjustment and not a quantity adjustment.

    A four-for-one split would leave the same 100 shares as 400 without changing their value. The
    position keeps its quantity and its price is the post-split one, so the quantity leg of the
    reconciliation is neither adjusted nor reconciled.
    """
    accounting = run()

    assert accounting.quantity == Decimal(SHARES)
    assert accounting.quantity != Decimal(SHARES) * SPLIT_RATIO


def test_an_entry_and_exit_around_the_boundary_omits_the_distribution() -> None:
    """
    Test the round trip: the price change is realised and the distribution is not.

    The position is opened before both actions and closed after them, at 400.00 and then 200.00, so
    the realised movement is 20,000 on 100 shares. A total-return account of the same position
    would carry 19,750, because the shares were held through an ex-date that paid 2.50 each. The
    difference is the omitted distribution, and it is the whole of the criterion's cash failure.
    """
    accounting = run(exit_on_last_bar=True)
    realised = Decimal(SHARES) * (Decimal(RAW_PRICES[2]) - Decimal(RAW_PRICES[0]))
    total_return = realised + DISTRIBUTION

    assert accounting.order_prices == (RAW_PRICES[0], RAW_PRICES[2])
    assert accounting.cash_delta == realised
    assert accounting.quantity == Decimal(0)
    assert accounting.cash_delta != total_return


def test_the_accounting_is_identical_without_the_action_data() -> None:
    """
    Test that the action data changes no number the run reports.

    The same scenario without the action records produces the same cash movement and the same
    quantity, which is what makes the omission a property of the accounting rather than of the
    fixture's data.
    """
    with_actions = run(exit_on_last_bar=True)
    without_actions = run(exit_on_last_bar=True, with_actions=False)

    assert with_actions.cash_delta == without_actions.cash_delta
    assert with_actions.quantity == without_actions.quantity
    assert without_actions.actions == 0
