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
Engine-side composition points of Section 14, the integration verification matrix.

Each test names the Section 14 row it proves. The matrix pairs a capability to compose with the
public entry point that would supply it, so a row is proved by exercising that entry point, not by
restating the design. Where the entry point does not exist from Python, the test pins the absence
rather than assuming the capability, because an empty cell is `blocked` rather than assumed.

All fixtures are deterministic and offline: the bundled minute bars under the repository's
`test_data/` directory are replayed through the same engine the other integration tests use, and no
adapter environment variable is read.
"""

from __future__ import annotations

import importlib
import inspect
from decimal import Decimal
from functools import partial
from pathlib import Path

import pytest

import nautilus_trader
from nautilus_trader import execution
from nautilus_trader import model
from nautilus_trader.backtest import BacktestDataConfig
from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestNode
from nautilus_trader.backtest import BacktestVenueConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import BarType
from nautilus_trader.model import BookType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import NautilusDataType
from nautilus_trader.model import OmsType
from nautilus_trader.model import Price
from nautilus_trader.model import Venue
from nautilus_trader.optimization import BacktestRunner
from nautilus_trader.optimization import CanonicalRun
from nautilus_trader.optimization import Experiment
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from nautilus_trader.trading import TargetPipelineConfig
from tests.providers import TestDataProvider
from tests.providers import TestInstrumentProvider


INSTRUMENT = TestInstrumentProvider.btcusdt_binance()
INSTRUMENT_ID = INSTRUMENT.id
BAR_TYPE = "BTCUSDT.BINANCE-1-MINUTE-LAST-EXTERNAL"
CSV_NAME = "btc-perp-20211231-20220201_1m.csv"
MAX_ROWS = 120
TRADE_SIZE = "0.010000"
STRATEGY = "strategies.ema_cross:EMACross"
STRATEGY_CONFIG = "strategies.ema_cross:EMACrossConfig"
SHARPE = "Sharpe Ratio (simple, sample, 252 days)"

# The Section 14 row verifies that a live data path exists for the shadow period. These adapter
# modules expose a `DataClientFactory` and a `DataClientConfig`; the row is proved by their
# existence, not by constructing a client, so nothing here needs credentials.
VENUE_ADAPTERS = (
    "architect_ax",
    "betfair",
    "binance",
    "blockchain",
    "bybit",
    "coinbase",
    "databento",
    "deribit",
    "derive",
    "dydx",
    "eodhd",
    "hyperliquid",
    "interactive_brokers",
    "kraken",
    "lighter",
    "moomoo",
    "okx",
    "polymarket",
    "tardis",
)
# Data clients that a shadow run cannot construct without credentials (an API key or secret) are
# named here rather than instantiated: architect_ax, betfair, binance, blockchain, bybit, coinbase,
# databento, deribit, derive, dydx, eodhd, hyperliquid, interactive_brokers, kraken, lighter,
# moomoo, okx, polymarket, tardis.


class NoopConfig(StrategyConfig):
    """
    Configure the no-op strategy used to inject a live strategy object.
    """

    def __init__(self, *, bar_type: str) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.bar_type = bar_type


class NoopStrategy(Strategy):
    """
    Subscribe to bars and do nothing else.

    This class is defined in the test module and has no import path, which is the point of the
    live-object injection row: the engine accepts the instance itself.
    """

    def __init__(self, config: NoopConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)

    def on_start(self) -> None:
        """
        On start.
        """
        self.subscribe_bars(self._bar_type)

    def on_bar(self, bar: object) -> None:
        """
        On bar.
        """


def _bars(max_rows: int) -> list:
    """
    Read a slice of the bundled Binance minute bars.
    """
    return TestDataProvider.bars_from_binance_csv(
        INSTRUMENT,
        bar_type=BarType.from_str(BAR_TYPE),
        csv_name=CSV_NAME,
        max_rows=max_rows,
    )


def _build_catalog(directory: Path) -> Path:
    """
    Write one instrument and a slice of minute bars into a fresh Parquet catalog.
    """
    directory.mkdir(parents=True, exist_ok=True)
    catalog = ParquetDataCatalog(str(directory))
    catalog.write_instruments([INSTRUMENT])
    catalog.write_bars(_bars(MAX_ROWS))
    return directory


def _config_parts(start, end, *, catalog_path, instrument_id, bar_type):
    """
    Build the venue, data, and engine configurations for a run window.
    """
    venue = BacktestVenueConfig(
        name="BINANCE",
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=["10 BTC", "10_000_000 USDT"],
        book_type="L1_MBP",
        fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.001"), taker_rate=Decimal("0.001")),
    )
    data = BacktestDataConfig(
        data_type=NautilusDataType.Bar,
        catalog_path=str(catalog_path),
        instrument_id=InstrumentId.from_str(instrument_id),
        bar_types=[bar_type],
        start_time=start,
        end_time=end,
    )
    engine = BacktestEngineConfig(bypass_logging=True, run_analysis=False)
    return [venue], [data], engine


def _new_engine() -> BacktestEngine:
    """
    Build an engine that logs nothing and runs no analysis.
    """
    return BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))


def _add_venue(engine: BacktestEngine, balance: str = "10 BTC") -> None:
    """
    Add the matching BINANCE venue with an explicit zero-fee-equivalent model.
    """
    engine.add_venue(
        venue=Venue("BINANCE"),
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money.from_str(balance), Money.from_str("10000000 USDT")],
        book_type=BookType.L1_MBP,
        fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.001"), taker_rate=Decimal("0.001")),
    )


def _engine_canonical(balance: str, max_rows: int = 60):
    """
    Run one no-op backtest over the bundled bars and return its canonical result.
    """
    engine = _new_engine()
    try:
        _add_venue(engine, balance)
        engine.add_instrument(INSTRUMENT)
        engine.add_data(_bars(max_rows))
        engine.add_strategy(NoopStrategy(NoopConfig(bar_type=BAR_TYPE)))
        engine.run()
        return engine.get_canonical_result()
    finally:
        engine.dispose()


def test_one_backtest_per_experiment_with_identity(tmp_path: Path) -> None:
    """
    Row: one backtest per experiment with identity (`BacktestRunner`, `CanonicalRun`).

    One experiment runs exactly one backtest through `BacktestRunner`, and the outcome is a
    `CanonicalRun` carrying a canonical digest and the canonical document bytes. The digest is the
    comparability key, so a second identical experiment must produce the same key and the same
    document.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    factory = partial(
        _config_parts,
        catalog_path=str(catalog_path),
        instrument_id=str(INSTRUMENT_ID),
        bar_type=BAR_TYPE,
    )
    runner = BacktestRunner(
        config_factory=factory,
        strategy_path=STRATEGY,
        config_path=STRATEGY_CONFIG,
    )
    experiment = Experiment(
        {
            "instrument_id": str(INSTRUMENT_ID),
            "bar_type": BAR_TYPE,
            "trade_size": TRADE_SIZE,
            "fast_ema_period": 10,
            "slow_ema_period": 30,
        },
    )

    outcome = runner.run(experiment, [SHARPE])

    assert isinstance(outcome, CanonicalRun)
    assert outcome.canonical_digest.startswith("blake3:")
    assert len(outcome.canonical_digest) == len("blake3:") + 64
    assert outcome.canonical_result.startswith(b"{")
    assert SHARPE in outcome.metric_values

    again = runner.run(experiment, [SHARPE])
    assert isinstance(again, CanonicalRun)
    assert again.canonical_digest == outcome.canonical_digest
    assert again.canonical_result == outcome.canonical_result

    # The `CanonicalRun` frozen dataclass composes the same identity by hand.
    composed = CanonicalRun(
        experiment,
        outcome.canonical_digest,
        outcome.metric_values,
        outcome.canonical_result,
    )
    assert composed.canonical_digest == outcome.canonical_digest


def test_deterministic_result_comparison() -> None:
    """
    Row: deterministic result comparison (`CanonicalBacktestResult`).

    `BacktestEngine.get_canonical_result()` returns a canonical document; `.digest()` is its
    `blake3:` identity, `.to_bytes()` the exact encoding, and `.first_divergence(expected_bytes)`
    the first field-level difference against another document, reported as an RFC 6901 JSON
    Pointer. Two identical runs have equal digests and no divergence; a run whose starting balance
    differs diverges, and the pointer path is reported. There is no Python constructor and no
    `__eq__`, so comparison is over serialised bytes; `from_slice` is Rust-only.
    """
    first = _engine_canonical("10 BTC")
    second = _engine_canonical("10 BTC")

    assert first.digest() == second.digest()
    assert first.to_bytes() == second.to_bytes()
    assert first.first_divergence(second.to_bytes()) is None

    different = _engine_canonical("5 BTC")
    assert different.digest() != first.digest()

    divergence = first.first_divergence(different.to_bytes())
    assert divergence is not None
    path, expected, actual = divergence
    assert path == "/accounts/0/Cash/base/balances/BTC/free"
    assert expected is not None
    assert actual is not None
    assert "5.0" in expected
    assert "10.0" in actual


def test_strategy_injection_as_live_object() -> None:
    """
    Row: strategy injection as a live object (`BacktestNode` and `BacktestEngine`).

    The engine accepts a locally defined `Strategy` subclass instance with no import path, the
    instance carries its `.config`, and the strategy id appears in the canonical document. A
    non-`Strategy` object is rejected with a `RuntimeError`. `BacktestNode.add_strategy` exists as
    the node-level counterpart; its live-object use and `get_engine_canonical_result` are already
    exercised in `python/tests/integration/test_backtest_node_bar_fills.py`, which is why this
    test uses the engine directly. The node reaches the canonical result through
    `get_engine_canonical_result(run_config_id)`, not `get_canonical_result`.
    """
    engine = _new_engine()
    try:
        _add_venue(engine)
        engine.add_instrument(INSTRUMENT)
        engine.add_data(_bars(30))
        strategy = NoopStrategy(NoopConfig(bar_type=BAR_TYPE))
        engine.add_strategy(strategy)
        engine.run()
        canonical = engine.get_canonical_result()
    finally:
        engine.dispose()

    assert isinstance(strategy.config, NoopConfig)
    assert str(strategy.strategy_id).encode() in canonical.to_bytes()

    rejecting = _new_engine()
    try:
        with pytest.raises(RuntimeError, match="Strategy"):
            rejecting.add_strategy(object())
    finally:
        rejecting.dispose()

    assert hasattr(BacktestNode, "add_strategy")
    assert hasattr(BacktestNode, "get_engine_canonical_result")


def test_targets_sizing_and_reconciliation() -> None:
    """
    Row: targets, sizing and reconciliation.

    `Target.from_quantity`, `from_weight` and `from_notional` construct the three sizing kinds;
    `TargetValue` is a read-only view with no constructor; `TargetPipelineConfig` constructs and
    `Strategy.enable_target_pipeline(config)` exists. `TargetPipeline`, `TargetConstruction` and
    `TargetReconciler` have no Python binding, so this row composes only through a strategy's
    `enable_target_pipeline`.
    """
    instrument_id = InstrumentId.from_str("AAPL.XNAS")

    by_quantity = model.Target.from_quantity(
        instrument_id,
        model.Quantity.from_str("100"),
    )
    assert by_quantity.kind == "QUANTITY"
    assert by_quantity.value.kind == "QUANTITY"
    assert str(by_quantity.value.quantity) == "100"

    by_weight = model.Target.from_weight(instrument_id, 0.25)
    assert by_weight.kind == "WEIGHT"
    assert by_weight.value.kind == "WEIGHT"
    assert by_weight.value.weight == 0.25

    by_notional = model.Target.from_notional(instrument_id, Money.from_str("1000 USD"))
    assert by_notional.kind == "NOTIONAL"
    assert by_notional.value.kind == "NOTIONAL"
    assert by_notional.value.notional == Money.from_str("1000 USD")

    with pytest.raises(TypeError, match="TargetValue"):
        model.TargetValue()

    config = TargetPipelineConfig(
        risk_per_trade=Decimal("0.01"),
        stop_loss_bps=50,
        max_weight=Decimal("0.2"),
        commission_rate=Decimal("0.001"),
    )
    assert config.risk_per_trade == Decimal("0.01")
    assert config.stop_loss_bps == 50
    assert config.max_weight == Decimal("0.2")
    assert config.commission_rate == Decimal("0.001")

    assert hasattr(Strategy, "enable_target_pipeline")
    assert callable(Strategy.enable_target_pipeline)

    for unbound in ("TargetPipeline", "TargetConstruction", "TargetReconciler"):
        assert not hasattr(model, unbound)
        assert not hasattr(nautilus_trader.trading, unbound)


def test_corporate_action_records_and_adjustment_arithmetic() -> None:
    """
    Row: corporate-action records and adjustment arithmetic.

    A dividend and a split construct as `CorporateAction` records, an `AdjustmentSeries` composes
    them, and the adjustment arithmetic matches the documented convention
    (`adjusted = raw * split_factor - dividends`). The `PriceRepresentation` members are pinned.
    `ParquetDataCatalog.write_corporate_actions` exists. No dividend cash-flow path exists
    anywhere, so this test asserts only the price arithmetic and never a cash credit.
    """
    instrument_id = InstrumentId.from_str("AAPL.XNAS")
    split_ns = 1_000_000_000_000_000_000
    dividend_ns = 2_000_000_000_000_000_000

    split = model.CorporateAction(
        instrument_id,
        model.CorporateActionType.SPLIT,
        Decimal(4),
        None,
        split_ns,
        split_ns,
        split_ns,
    )
    dividend = model.CorporateAction(
        instrument_id,
        model.CorporateActionType.DIVIDEND,
        Decimal("2.50"),
        None,
        dividend_ns,
        dividend_ns,
        dividend_ns,
    )
    assert split.action == model.CorporateActionType.SPLIT
    assert dividend.action == model.CorporateActionType.DIVIDEND
    assert split.value == Decimal(4)
    assert dividend.value == Decimal("2.50")

    series = model.AdjustmentSeries(instrument_id, [split, dividend])
    assert series.instrument_id == instrument_id
    assert len(series.actions) == 2

    before = split_ns - 1
    assert series.split_factor(before) == Decimal("0.25")
    assert series.dividends(before) == Decimal("2.50")

    adjusted = series.adjust(Price.from_str("400.00"), before)
    assert str(adjusted) == "97.5000"
    assert str(series.unadjust(adjusted, before)) == "400.00"

    # A price at or after the split is already adjusted, so the factor there is one.
    assert series.split_factor(split_ns) == Decimal(1)
    assert series.dividends(split_ns) == Decimal("2.50")

    assert {member.name for member in model.PriceRepresentation.variants()} == {"RAW", "ADJUSTED"}
    assert model.PriceRepresentation.RAW.name == "RAW"
    assert model.PriceRepresentation.ADJUSTED.name == "ADJUSTED"
    assert hasattr(ParquetDataCatalog, "write_corporate_actions")


def test_realism_models() -> None:
    """
    Row: realism models: fill, fee, latency, slippage, impact.

    One fill model, one fee model, one latency model, the one slippage model and a market impact
    model all construct from `nautilus_trader.execution`. The asymmetry is real: fee, fill and
    latency have Python base classes; `SlippageModel` and `MarketImpactModel` base classes do not
    exist, so the concrete slippage and impact models stand alone.
    """
    fill_model = execution.DefaultFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0)
    fee_model = execution.FixedFeeModel(Money.from_str("1.00 USD"))
    latency_model = execution.StaticLatencyModel(base_latency_nanos=1_000_000)
    slippage_model = execution.ProbabilisticSlippageModel(prob_slippage=0.1, random_seed=7)
    impact_model = execution.LinearMarketImpactModel(model.Quantity.from_str("1"), 10)

    assert isinstance(fill_model, execution.DefaultFillModel)
    assert isinstance(fee_model, execution.FixedFeeModel)
    assert isinstance(latency_model, execution.StaticLatencyModel)
    assert isinstance(slippage_model, execution.ProbabilisticSlippageModel)
    assert isinstance(impact_model, execution.LinearMarketImpactModel)

    assert inspect.isclass(execution.FillModel)
    assert inspect.isclass(execution.FeeModel)
    assert inspect.isclass(execution.LatencyModel)
    assert issubclass(execution.FixedFeeModel, execution.FeeModel)

    assert not hasattr(execution, "SlippageModel")
    assert not hasattr(execution, "MarketImpactModel")


def test_engine_run_manifest_and_entry_hashes_absent() -> None:
    """
    Row: engine run manifest and entry hashes (`crates/event_store/`).

    There is no Python binding for the event store at all: importing
    `nautilus_trader.event_store` raises `ModuleNotFoundError` and no event-store symbol is present
    on the package, so the manifest, entry, hash and verifier surfaces are unreachable from
    Python. For this row to pass, a PyO3 module would have to be added for `crates/event_store`
    (its manifest and entry types, the entry hashing, and the verifier) and exported on the
    `nautilus_trader` package.
    """
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("nautilus_trader.event_store")

    assert not hasattr(nautilus_trader, "event_store")
    assert not hasattr(nautilus_trader, "EventStore")
    assert not any(name in {"event_store", "EventStore"} for name in dir(nautilus_trader))


def test_live_data_path_for_shadow() -> None:
    """
    Row: live data path for shadow (a live data adapter, as design Section 11.5 assumes).

    Every venue adapter module imports offline and exports a `DataClientFactory` and a
    `DataClientConfig`, so a shadow run can name a live data source without starting a client or
    reading credentials. The sandbox venue exports an execution client factory and config and no
    data-client factory, which is the asymmetry the shadow path relies on.
    """
    for adapter in VENUE_ADAPTERS:
        module = importlib.import_module(f"nautilus_trader.adapters.{adapter}")
        factories = [name for name in dir(module) if name.endswith("DataClientFactory")]
        configs = [name for name in dir(module) if name.endswith("DataClientConfig")]
        assert factories, f"{adapter} has no DataClientFactory"
        assert configs, f"{adapter} has no DataClientConfig"
        for name in factories + configs:
            assert inspect.isclass(getattr(module, name))

    sandbox = importlib.import_module("nautilus_trader.adapters.sandbox")
    assert hasattr(sandbox, "SandboxExecutionClientFactory")
    assert hasattr(sandbox, "SandboxExecutionClientConfig")
    assert not any(name.endswith("DataClientFactory") for name in dir(sandbox))
    assert not any(name.endswith("DataClientConfig") for name in dir(sandbox))
