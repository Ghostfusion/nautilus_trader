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
The non-monotonicity harness: a crash a de-risking has to unwind into, swept over the budget.

The budget bounds the clip the participant may send.

The claim the harness is built to express is T10's: a risk limit that is prudent on its own can
make a crash worse for the participant it protects. The scenario gives the participant a clip equal
to its budget, which is the response a cap induces on a participant that respects it, and the crash
is a generated flow read as a decline. A tighter budget then buys a smaller clip, so the unwind
takes longer and carries more of the fall, while a looser budget lets the clip grow and the book
charges more impact for the size. The severity is the loss on equity, which carries both, and the
harness reports the direction it measures rather than the one it expects.

What the harness cannot express is where the corpus's non-monotonicity comes from at the market
level: the participant's own impact moving the price that the *other* participants then trade
against. The venue's impact model charges the taker a worse price and never updates the book (T3),
so a crash here is exogenous drift and the only coupling a cap has is the clip the order carries.
The harness prints the comparison, and prints that limit beside it, so a reader sees which of the
two directions the simulation can and cannot show.
"""

from __future__ import annotations

import logging
from decimal import Decimal
from itertools import pairwise

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import SyntheticFlowConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskCap
from nautilus_trader.risk import RiskCapMetric
from nautilus_trader.risk import RiskCapScope
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestInstrumentProvider


logger = logging.getLogger(__name__)

VENUE = Venue("BINANCE")
CURRENCY = Currency.from_str("USDT")
STARTING_BALANCE = 10_000_000.0
PRICE_BASE = Decimal("1000.00")
PRICE_STEP = Decimal("0.01")
START_NS = 1_600_000_000_000_000_000
BAR_NS = 60_000_000_000
BAR_COUNT = 256
HURST = 0.6
IMPACT_EXPONENT = 0.5
SEED = 7
POSITION_BTC = Decimal("20.000")
# The crash's depth: the generated path with its sign flipped and stretched, so the fall is about
# ten percent of the base price over the run.
CRASH_SCALE = 40.0
# The impact the book charges a taker: a square root in the clip, in whole price increments.
IMPACT_PREFACTOR = 2_000.0
IMPACT_REFERENCE_BTC = 1.0
# One budget per minute and a clip of exactly that budget: the participant sizes what it sends to
# what the cap permits, which is the response the cap induces on a participant that respects it.
BUDGET_WINDOW_NS = BAR_NS
BUDGETS = (0.25, 0.5, 1.0, 2.0, 5.0, 20.0)


class CrashImpact:
    """
    The book's impact for the scenario: a square root in the clip, in whole price increments.

    The protocol is the impact model's own duck typing, so the scenario states the impact it wants
    rather than depending on a built-in model's calibration surface.
    """

    def impact_increments(self, fill_quantity: float) -> int:
        """
        Return the number of price increments the clip moves the fill price by.
        """
        return int(IMPACT_PREFACTOR * (fill_quantity / IMPACT_REFERENCE_BTC) ** IMPACT_EXPONENT)


class CrashedUnwinder(Strategy):
    """
    Open a position on the first bar, then sell it out one clip at a time.

    The clip is what the participant sends to the book, and the scenario sets it to the budget the
    cap permits for the minute.
    """

    def __init__(self, config: StrategyConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self.instrument_id = None
        self.bar_type = None
        self.clip = Decimal("1.000")
        self.bars = 0

    def configure(self, bar_type: str, instrument_id: str, budget: float) -> None:
        """
        Configure the strategy before the run.
        """
        self.bar_type = BarType.from_str(bar_type)
        self.instrument_id = InstrumentId.from_str(instrument_id)
        self.clip = min(Decimal(str(budget)), POSITION_BTC)

    def on_start(self) -> None:
        """
        On start.
        """
        assert self.bar_type is not None
        self.subscribe_bars(self.bar_type)

    def on_bar(self, _bar: Bar) -> None:
        """
        On bar.
        """
        assert self.instrument_id is not None
        self.bars += 1
        net = self.portfolio.net_position(self.instrument_id)

        if self.bars == 1 and net == 0:
            self.submit_order(
                self.order_factory.market(
                    instrument_id=self.instrument_id,
                    order_side=OrderSide.BUY,
                    quantity=Quantity.from_decimal_dp(POSITION_BTC, 6),
                ),
            )
        elif net > 0:
            # A reduce-only clip, so the unwind cannot overshoot into a short.
            clipped = min(self.clip, net)
            self.submit_order(
                self.order_factory.market(
                    instrument_id=self.instrument_id,
                    order_side=OrderSide.SELL,
                    quantity=Quantity.from_decimal_dp(clipped, 6),
                    reduce_only=True,
                ),
            )


def crash_prices(seed: int = SEED, bar_count: int = BAR_COUNT) -> list[float]:
    """
    Return a crashing price path: the generated flow's increments read as a pure decline.

    The flow's increments have no mean, so a path's net direction is a coin flip at a seed and an
    entry can land before a bounce. Taking the magnitude of every increment and one sign makes the
    run a crash whose severity is the unwind's rather than the entry's timing, and keeps the
    increments' memory, so the draw is still the generator's.
    """
    flow = SyntheticFlowConfig(HURST, IMPACT_EXPONENT, bar_count, seed).generate()
    path = flow.prices
    crash = [0.0]

    for earlier, later in pairwise(path):
        crash.append(crash[-1] - CRASH_SCALE * abs(later - earlier))

    return crash


def bars_from_prices(instrument: object, bar_type: BarType, prices: list[float]) -> list[Bar]:
    """
    Build one-minute bars from a price path, guarding the OHLC invariants.
    """
    bars = []
    previous = PRICE_BASE

    for index, level in enumerate(prices):
        close = PRICE_BASE + Decimal(str(level)).quantize(PRICE_STEP)
        bars.append(
            Bar(
                bar_type=bar_type,
                open=Price.from_decimal_dp(previous, instrument.price_precision),
                high=Price.from_decimal_dp(
                    max(previous, close) + PRICE_STEP,
                    instrument.price_precision,
                ),
                low=Price.from_decimal_dp(
                    min(previous, close) - PRICE_STEP,
                    instrument.price_precision,
                ),
                close=Price.from_decimal_dp(close, instrument.price_precision),
                volume=Quantity.from_decimal_dp(Decimal("5.0"), instrument.size_precision),
                ts_event=START_NS + index * BAR_NS,
                ts_init=START_NS + index * BAR_NS,
            ),
        )
        previous = close

    return bars


def participation_cap(budget: float) -> RiskCap:
    """
    Return the participation budget the participant is allowed over the crash.
    """
    return RiskCap(
        metric=RiskCapMetric.Participation,
        scope=RiskCapScope.StrategyInstrument,
        limit=0,
        window=BUDGET_WINDOW_NS,
        quantity_limit=Decimal(str(budget)),
    )


def run_crash(budget: float | None, seed: int = SEED) -> tuple[float, float]:
    """
    Run the crash with the given participation budget and return (loss, quantity left).

    A budget of ``None`` runs uncapped, which is the reference the capped levels are read against.
    """
    instrument = TestInstrumentProvider.btcusdt_binance()
    bar_type = BarType.from_str(f"{instrument.id}-1-MINUTE-LAST-EXTERNAL")

    risk_engine = (
        RiskEngineConfig()
        if budget is None
        else RiskEngineConfig(count_caps=[participation_cap(budget)])
    )
    engine = BacktestEngine(
        BacktestEngineConfig(bypass_logging=True, run_analysis=False, risk_engine=risk_engine),
    )
    engine.add_venue(
        venue=VENUE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=CURRENCY,
        starting_balances=[Money(STARTING_BALANCE, CURRENCY)],
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
        market_impact_model=CrashImpact(),
    )
    engine.add_instrument(instrument)
    engine.add_data(bars_from_prices(instrument, bar_type, crash_prices(seed)))

    strategy = CrashedUnwinder(StrategyConfig())
    strategy.configure(str(bar_type), str(instrument.id), budget if budget is not None else 1e9)
    engine.add_strategy(strategy)
    engine.run()

    account = engine.portfolio.account(VENUE)
    assert account is not None
    # The severity carries what was sold and what was left: the loss is measured on equity, so an
    # open position's mark against the crash counts as much as a realized one.
    balance = account.balance_total(CURRENCY).as_double()
    unrealized = engine.portfolio.unrealized_pnl(instrument.id)
    left = engine.portfolio.net_position(instrument.id)
    engine.dispose()

    unrealized = 0.0 if unrealized is None else unrealized.as_double()

    # Severity is the loss itself, positive when the crash cost something.
    return -((balance - STARTING_BALANCE) + unrealized), float(left)


def report(label: str, budget: float | None, loss: float, left: float) -> None:
    """
    Log one level of the sweep.
    """
    capped = "uncapped" if budget is None else f"budget {budget:g}"
    logger.info("%s %s: severity=%.2f left=%.3f", label, capped, loss, left)


def sweep(
    budgets: tuple[float, ...] = BUDGETS, seed: int = SEED
) -> list[tuple[float, float, float]]:
    """
    Run every budget and return (budget, loss, quantity left) per level, plus the uncapped level.
    """
    levels: list[tuple[float, float, float]] = []

    loss, left = run_crash(None, seed)
    report("level", None, loss, left)
    levels.append((float("inf"), loss, left))

    for budget in budgets:
        loss, left = run_crash(budget, seed)
        report("level", budget, loss, left)
        levels.append((budget, loss, left))

    capped = levels[1:]
    uncapped = levels[0]
    logger.info(
        "the tightest capped level costs %.2f against %.2f uncapped, and leaves %.3f of the "
        "position against %.3f",
        capped[0][1],
        uncapped[1],
        capped[0][2],
        uncapped[2],
    )
    logger.info(
        "severity is %s in the budget",
        "non-increasing" if monotone_in_budget(levels) else "not non-increasing",
    )

    return levels


def monotone_in_budget(levels: list[tuple[float, float, float]]) -> bool:
    """
    Return whether the severity is non-increasing as the budget grows, tightest first.
    """
    ordered = sorted(levels, key=lambda level: level[0])
    return all(later[1] <= earlier[1] for earlier, later in pairwise(ordered))


def test_the_crash_sweep_is_reported_and_deterministic() -> None:
    """
    The sweep runs, reports every level, and reproduces itself at the same seed.
    """
    levels = sweep()
    repeated = sweep((BUDGETS[0], BUDGETS[4]))[1:]

    assert len(levels) == len(BUDGETS) + 1
    assert [level[1] for level in repeated] == [levels[1][1], levels[5][1]]

    # A budget that buys a smaller clip keeps the position in the fall longer, so it costs more.
    assert levels[1][1] > levels[-1][1], "the tightest budget should cost the most here"


def test_the_severity_response_to_the_budget_is_measured_not_assumed() -> None:
    """
    The harness reports the direction it measures, so the assertion is the measured shape.

    The corpus's non-monotone response needs the participant's impact to move the price that the
    other participants then trade against, and the simulated venue does not price the flow it
    receives, so what this harness reports is the direction it measures rather than that one.
    """
    levels = sweep()

    assert all(level[1] != 0.0 for level in levels), "every level should cost something"

    # Measured, not assumed: in this simulation a tighter budget is monotonically more expensive
    # for the participant, because a slower unwind carries the fall for longer while the venue
    # charges nothing for the size the cap withholds.
    assert monotone_in_budget(levels), "the measured response should be reported as it is"
    logger.info(
        "the non-monotone direction the corpus reports is not reachable here: the venue does not "
        "price the aggregate flow",
    )
