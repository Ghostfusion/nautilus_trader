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
The opt-in target pipeline path, from a signal to a submitted market order.

A strategy enables the target pipeline and submits one long signal from `on_quote`. The pipeline
constructs a target with the fixed-risk sizing calculation, reconciles it against the cache and the
portfolio, and submits one market order on the existing path. The checkpoints are that order and its
fill, so the committed canonical digest pins the whole signal-to-order path.

The mid of the quotes is `100.000` and the account holds `1,000,000 USD`, so with a 1 per cent stop
(100 basis points) and `0.0001` of equity risked the fixed-risk sizing resolves to exactly 100 units,
a multiple of the instrument's 100 unit lot size, and the reconciled order states that size.
"""

from __future__ import annotations

from decimal import Decimal
from typing import Any

from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import Equity
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import SignalDirection
from nautilus_trader.model import Symbol
from nautilus_trader.model import TradingSignal
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from nautilus_trader.trading import TargetPipelineConfig
from tests.regression.scenario import Checkpoint
from tests.regression.scenario import Scenario
from tests.regression.scenario import backtest_engine


NAME = "target_pipeline_parity"

INSTRUMENT = Equity(
    instrument_id=InstrumentId.from_str("AAPL.XNAS"),
    raw_symbol=Symbol("AAPL"),
    isin="US0378331005",
    currency=Currency.from_str("USD"),
    price_precision=2,
    price_increment=Price.from_str("0.01"),
    lot_size=Quantity.from_int(100),
    ts_event=0,
    ts_init=0,
)
INSTRUMENT_ID = INSTRUMENT.id
INSTRUMENT_ID_STR = str(INSTRUMENT_ID)

XNAS = Venue("XNAS")
USD = Currency.from_str("USD")

TS_START = 1_704_067_200_000_000_000
INTERVAL_NS = 60_000_000_000
QUOTE_COUNT = 5

# The construction settings that resolve the signal to 100 units. See the module docstring.
PIPELINE_CONFIG = TargetPipelineConfig(
    risk_per_trade=Decimal("0.0001"),
    stop_loss_bps=100,
    max_weight=Decimal("1"),
    commission_rate=Decimal("0"),
    min_order_quantity=Quantity.from_int(0),
)

CHECKPOINTS = (
    Checkpoint("order", instrument_id=INSTRUMENT_ID_STR, ordinal=0),
    Checkpoint("fill", instrument_id=INSTRUMENT_ID_STR, ordinal=0),
)


class PipelineParityConfig(StrategyConfig):
    """
    Configure the pipeline parity strategy.
    """

    def __init__(self, *, instrument_id: str) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.instrument_id = instrument_id


class PipelineParityStrategy(Strategy):
    """
    Submit one long signal through the target pipeline on the first quote.
    """

    def __init__(self, config: PipelineParityConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._instrument_id = InstrumentId.from_str(config.instrument_id)
        self.submitted = False

    def on_start(self) -> None:
        """
        On start.
        """
        self.enable_target_pipeline(PIPELINE_CONFIG)
        self.subscribe_quotes(self._instrument_id)

    def on_quote(self, _quote: QuoteTick) -> None:
        """
        On quote.
        """
        if self.submitted:
            return
        self.submitted = True
        self.submit_signals(
            [
                TradingSignal(
                    instrument_id=self._instrument_id,
                    direction=SignalDirection.LONG,
                ),
            ],
        )


def _engine_config() -> BacktestEngineConfig:
    return BacktestEngineConfig(
        bypass_logging=True,
        run_analysis=False,
        risk_engine=RiskEngineConfig(bypass=True),
    )


def _quotes() -> list[QuoteTick]:
    quotes: list[QuoteTick] = []
    for index in range(QUOTE_COUNT):
        ts = TS_START + index * INTERVAL_NS
        quotes.append(
            QuoteTick(
                instrument_id=INSTRUMENT_ID,
                bid_price=Price.from_str("99.99"),
                ask_price=Price.from_str("100.01"),
                bid_size=Quantity(10_000, precision=0),
                ask_size=Quantity(10_000, precision=0),
                ts_event=ts,
                ts_init=ts,
            ),
        )
    return quotes


def execute() -> Any:
    """
    Run the scenario and return its canonical result.
    """
    with backtest_engine(_engine_config()) as engine:
        engine.add_venue(
            venue=XNAS,
            oms_type=OmsType.NETTING,
            account_type=AccountType.CASH,
            base_currency=USD,
            starting_balances=[Money(1_000_000.0, USD)],
            fee_model=MakerTakerFeeModel(
                maker_rate=Decimal("0"),
                taker_rate=Decimal("0"),
            ),
        )
        engine.add_instrument(INSTRUMENT)
        engine.add_data(_quotes())
        engine.add_strategy(
            PipelineParityStrategy(PipelineParityConfig(instrument_id=INSTRUMENT_ID_STR)),
        )
        engine.run(start=TS_START, end=TS_START + QUOTE_COUNT * INTERVAL_NS)
        return engine.get_canonical_result()


SCENARIO = Scenario(
    name=NAME,
    execute=execute,
    checkpoints=CHECKPOINTS,
)
