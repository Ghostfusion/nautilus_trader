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
boundary. The fixture reads the venue account, the position the cache holds and the prices the
orders filled at, and it states the reconciliation as arithmetic rather than as a comparison of two
adjusted price series.

The scenario is one coherent total-return reading. The position is opened at 400.00 before both
actions, the ex-date pays 2.50 on the shares held then, the four-for-one split restates the quantity
at 400 and divides the price basis by four, and the raw series falls from 400.00 to 100.00 where the
split takes effect. The value leg is the adjustment series: 400.00 times the split factor of one
quarter, less the 2.50 per share dividend, is 97.50. The cash leg is the account: the purchase, the
distribution credited at the ex-date, and the sale of the shares the split left.
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
SECOND_NS = 1_000_000_000
TS_START = 1_704_067_200_000_000_000
# The ex-date falls between the first and the second bar, and the split takes effect between the
# second and the third, where the raw series' own change is.
DIVIDEND_NS = TS_START + 30 * SECOND_NS
SPLIT_NS = TS_START + MINUTE_NS + 30 * SECOND_NS
ANNOUNCE_NS = TS_START - 10 * MINUTE_NS

# Two bars before the split and two after it, at the raw series the venue published. A four-for-one
# split divides the published price by four, so the raw series moves from 400.00 to 100.00.
RAW_PRICES = ("400.00", "400.00", "100.00", "100.00")
SHARES = 100
STARTING_BALANCE = "1_000_000 USD"
# The distribution the 100 shares were entitled to at the ex-date: 2.50 per share. The ex-date
# precedes the split, so the entitlement is the quantity held before it.
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
            self._submit(OrderSide.BUY, Quantity.from_int(SHARES))
            return
        if self._exit_on_last_bar and self._bars_seen == len(RAW_PRICES) and not self._exited:
            self._exited = True
            # The exit sells the position the run holds, which is the opened quantity restated by
            # any split that took effect while it was open.
            held = self.portfolio.net_position(INSTRUMENT_ID)
            self._submit(OrderSide.SELL, Quantity.from_int(int(held or 0)))

    def on_corporate_action(self, action: CorporateAction) -> None:
        """
        Record the action the module received.
        """
        self.actions.append(action)

    def _submit(self, side: OrderSide, quantity: Quantity) -> None:
        self.submit_order(
            self.order_factory.market(
                instrument_id=INSTRUMENT_ID,
                order_side=side,
                quantity=quantity,
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
    Build the dividend and the split, announced before they take effect.

    The ex-date comes first, so the entitlement is paid on the shares the position held before the
    split restated it, and the raw series' own change follows at the split's effective instant.
    """
    return [
        CorporateAction(
            INSTRUMENT_ID,
            CorporateActionType.DIVIDEND,
            Decimal("2.50"),
            None,
            DIVIDEND_NS,
            ANNOUNCE_NS,
            ANNOUNCE_NS,
        ),
        CorporateAction(
            INSTRUMENT_ID,
            CorporateActionType.SPLIT,
            SPLIT_RATIO,
            None,
            SPLIT_NS,
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
    # Once both actions have taken effect the series is the raw series again.
    after_both = TS_START + len(RAW_PRICES) * MINUTE_NS
    assert series.split_factor(after_both) == Decimal(1)
    assert series.dividends(after_both) == Decimal(0)


def test_both_action_records_reach_the_module() -> None:
    """
    Test that the action data is delivered to the strategy.

    Both records arrive at their effective instants, so the accounting the other cases read is the
    engine's rather than an absence of the data the engine was given.
    """
    accounting = run()

    assert accounting.actions == 2


def test_a_held_dividend_credits_its_entitlement() -> None:
    """
    Test that the cash line records the purchase and the distribution.

    The 100 shares cost 40,000, and the position was entitled to 250 at the ex-date, so the cash the
    criterion reconciles against is the purchase less the distribution the shares were paid.
    """
    accounting = run()

    assert accounting.order_prices == (RAW_PRICES[0],)
    assert accounting.cash_delta == Decimal("-40000.00") + DISTRIBUTION


def test_the_split_restates_the_position_quantity() -> None:
    """
    Test that the split restates the quantity rather than the price alone.

    A four-for-one split leaves the same 100 shares as 400, so the position carries the notional it
    held at the pre-split price into the post-split series.
    """
    accounting = run()

    assert accounting.quantity == Decimal(SHARES) * SPLIT_RATIO


def test_an_entry_and_exit_around_the_boundary_carries_the_distribution() -> None:
    """
    Test the round trip: the price movement realises nothing and the distribution is carried.

    The position is opened before both actions and closed after them, at 400.00 on 100 shares and
    then at 100.00 on the 400 the split left, so the price movement realises nothing and the 250 the
    shares were paid at the ex-date is the whole of what a total-return account of the same position
    carries. The quantity is zero at the end because the exit sells what the split left.
    """
    accounting = run(exit_on_last_bar=True)

    assert accounting.order_prices == (RAW_PRICES[0], RAW_PRICES[-1])
    assert accounting.cash_delta == DISTRIBUTION
    assert accounting.quantity == Decimal(0)


def test_the_action_data_changes_the_cash_the_run_reports() -> None:
    """
    Test that the action data changes the numbers, which is what makes the accounting the engine's.

    The same scenario without the action records is never restated and never credited, so the exit
    sells the 100 shares that were opened and the fall in the raw series is realised in place of the
    distribution.
    """
    with_actions = run(exit_on_last_bar=True)
    without_actions = run(exit_on_last_bar=True, with_actions=False)
    realised = Decimal(SHARES) * (Decimal(RAW_PRICES[-1]) - Decimal(RAW_PRICES[0]))

    assert with_actions.cash_delta == DISTRIBUTION
    assert without_actions.cash_delta == realised
    assert with_actions.quantity == Decimal(0)
    assert without_actions.quantity == Decimal(0)
    assert without_actions.actions == 0
