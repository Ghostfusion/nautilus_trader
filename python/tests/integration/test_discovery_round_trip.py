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
The discovery round trip: one hand-written candidate specification becomes one ledger entry.

The candidate is one dict of payload, and each artefact after it is produced by the step before it:
`compile_specification` accepts the payload and compiles it, the runner identifies the experiment
from the compiled plan, the run returns the engine's canonical result and the evidence package, and
the ledger writes one entry for the package and refuses to rewrite it. Nothing here writes an
artefact the pipeline is supposed to produce, so the identities a reader sees are the ones the
pipeline computed.

The interpreter is handed to the runner as a live object and is the run's only executor, so the
run's behaviour is evidence rather than assertion: the fixture reads the decisions the interpreter
made, the signals it submitted, and the orders and fills the engine produced from them. The data
the run replays is written into a temporary `ParquetDataCatalog` and described by
`BacktestDataConfig`, because a run replays its data through the catalog like every other backtest.
The quotes are part of that data and not of the interpreter's subscriptions: the target pipeline
resolves an entry price from the cache at the decision instant and the interpreter subscribes to
bars alone, exactly as a campaign that runs a daily candidate supplies them.

Four instruments with eight daily closes each make the ranking rotate: AMZN is third on its own flat
moving average until NVDA's rising one overtakes it, which is the case where a held position leaves
the selected set.
"""

from __future__ import annotations

from decimal import Decimal
from typing import TYPE_CHECKING
from typing import Any

import pytest

from nautilus_trader.backtest import BacktestDataConfig
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestVenueConfig
from nautilus_trader.discovery.compile import compile_specification
from nautilus_trader.discovery.features import FeatureDefinition
from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.features import build_memberships
from nautilus_trader.discovery.ledger import DiscoveryLedger
from nautilus_trader.discovery.policy import DiscoveryPolicy
from nautilus_trader.discovery.policy import Reference
from nautilus_trader.discovery.policy import ReferenceKind
from nautilus_trader.discovery.run import RunRequest
from nautilus_trader.discovery.run import experiment_for
from nautilus_trader.discovery.run import run_experiment
from nautilus_trader.discovery.schema import SPECIFICATION_SCHEMA_VERSION
from nautilus_trader.discovery.strategy import BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY
from nautilus_trader.discovery.strategy import DiscoveryInterpreter
from nautilus_trader.discovery.strategy import DiscoveryInterpreterConfig
from nautilus_trader.discovery.strategy import InterpreterRunInputs
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Equity
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import MarketOrder
from nautilus_trader.model import NautilusDataType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Symbol
from nautilus_trader.persistence import ParquetDataCatalog
from tests.providers import TestInstrumentProvider


if TYPE_CHECKING:
    from pathlib import Path

    from nautilus_trader.discovery.compile import CompiledPlan
    from nautilus_trader.discovery.run import ExperimentOutcome


# The fixture's instruments, named so a case reads as the decision it exercises.
A = "AAPL.XNAS"
B = "MSFT.XNAS"
C = "NVDA.XNAS"
D = "AMZN.XNAS"

# The candidate's declared values and the metrics the run's evidence package records.
TREND_ID = "sma_close_3"
TREND_LOOKBACK = 3
RANK_PROPORTIONAL = "rank_proportional"
MODE_RANK = "rank"
TIE_FEATURE_DESC = "feature_desc"
CASH_POLICY_LEAVE_IDLE = "leave_idle"
SHARPE = "Sharpe Ratio (simple, sample, 252 days)"
DRAWDOWN = "Max Drawdown (simple)"
SNAPSHOT = "discovery.fixture-2024-01"
SEED = 1

USD = Currency.from_str("USD")
INTEGRATOR = TestInstrumentProvider.aapl_equity()
INSTRUMENT_ID = str(INTEGRATOR.id)
VENUE = INTEGRATOR.venue
BAR_SPECIFICATION = BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY["daily"]
SECOND_NS = 1_000_000_000
DAY_NS = 86_400 * SECOND_NS
TS_START = 1_704_067_200_000_000_000
STARTING_BALANCE = 1_000_000.0
VOLUME = 1_000_000
QUOTE_SIZE = 1_000
MAX_POSITION_WEIGHT = Decimal("0.25")
STOP_LOSS_BPS = 100
BENCHMARK_DIGEST = f"sha256:{'0' * 64}"
CLOSES: dict[str, tuple[str, ...]] = {
    INSTRUMENT_ID: ("100.00", "101.00", "102.00", "103.00", "104.00", "105.00", "106.00", "107.00"),
    B: ("200.00", "199.00", "198.00", "197.00", "196.00", "195.00", "194.00", "193.00"),
    C: ("50.00", "60.00", "70.00", "80.00", "90.00", "100.00", "110.00", "120.00"),
    D: ("80.00", "80.00", "80.00", "80.00", "80.00", "80.00", "80.00", "80.00"),
}


class RecordingInterpreter(DiscoveryInterpreter):
    """
    The interpreter, keeping the orders the engine produced while it can still read them.

    The interpreter is the run's only executor and the engine disposes its cache when the run ends,
    which is after `run_experiment` returns. The orders are the evidence for the signals, so they are
    read at the stop the engine calls while the cache still holds them, and they are the engine's own
    orders rather than anything this fixture assembled.
    """

    def __init__(self, config: DiscoveryInterpreterConfig) -> None:
        """
        Initialize the interpreter and an empty record of the engine's orders.
        """
        super().__init__(config)
        self.engine_orders: list[Any] = []

    def on_stop(self) -> None:
        """
        Keep the orders the engine produced, before it disposes them.
        """
        self.engine_orders = list(self.cache.orders())
        super().on_stop()


def instrument(instrument_id: str) -> Equity:
    """
    Return one fixture equity on the AAPL instrument's venue.
    """
    return Equity(
        instrument_id=InstrumentId.from_str(instrument_id),
        raw_symbol=Symbol(instrument_id.split(".", maxsplit=1)[0]),
        isin=None,
        currency=USD,
        price_precision=2,
        price_increment=Price.from_str("0.01"),
        lot_size=Quantity.from_int(100),
        ts_event=0,
        ts_init=0,
    )


def instruments() -> tuple[Equity, ...]:
    """
    Return the fixture's instruments: the provider's own AAPL equity and three of its peers.
    """
    return (INTEGRATOR, instrument(B), instrument(C), instrument(D))


def bar(instrument_id: str, close: str, ts_event: int) -> Bar:
    """
    Return one daily bar with a spread around the given close.
    """
    close_price = Price.from_str(close)
    bar_type = BarType.from_str(f"{instrument_id}-{BAR_SPECIFICATION}")

    return Bar(
        bar_type,
        close_price,
        Price.from_str(f"{close_price.as_double() + 1.0:.2f}"),
        Price.from_str(f"{close_price.as_double() - 1.0:.2f}"),
        close_price,
        Quantity.from_int(VOLUME),
        ts_event,
        ts_event,
    )


def bars(instrument_id: str) -> list[Bar]:
    """
    Return one instrument's fixture bars, in ascending time order.
    """
    return [
        bar(instrument_id, close, TS_START + index * DAY_NS)
        for index, close in enumerate(CLOSES[instrument_id])
    ]


def quotes(instrument_id: str) -> list[QuoteTick]:
    """
    Return one quote per fixture bar, so the pipeline can resolve an entry price at a decision.

    The target pipeline resolves the entry price from the cache, and the interpreter subscribes to
    bars rather than to quotes: the quotes are part of the run's data, not of the interpreter's
    subscriptions, exactly as a campaign that runs a daily candidate supplies them.
    """
    identifier = InstrumentId.from_str(instrument_id)

    return [
        QuoteTick(
            identifier,
            Price.from_str(close),
            Price.from_str(close),
            Quantity.from_int(QUOTE_SIZE),
            Quantity.from_int(QUOTE_SIZE),
            TS_START + index * DAY_NS - SECOND_NS,
            TS_START + index * DAY_NS - SECOND_NS,
        )
        for index, close in enumerate(CLOSES[instrument_id])
    ]


def definition() -> FeatureDefinition:
    """
    Return the fixture's registered trend definition.
    """
    return FeatureDefinition(
        feature_id=TREND_ID,
        operator="sma",
        input_field="close",
        lookback=TREND_LOOKBACK,
    )


def policy() -> DiscoveryPolicy:
    """
    Return the fixture's pre-registered policy, with its frozen references and sector map.
    """
    return DiscoveryPolicy.first_release(
        feature_allowlist=(TREND_ID,),
        references=(
            Reference("fixture-benchmark", ReferenceKind.BENCHMARK, BENCHMARK_DIGEST),
            Reference("fixture-cost-model", ReferenceKind.COST_MODEL, BENCHMARK_DIGEST),
            Reference("fixture-validation", ReferenceKind.VALIDATION_POLICY, BENCHMARK_DIGEST),
        ),
        sectors=((INSTRUMENT_ID, "mega"), (B, "mega"), (C, "chips")),
    )


def payload() -> dict[str, Any]:
    """
    Return a hand-written candidate specification: rank the trend and hold the three leaders.
    """
    return {
        "version": SPECIFICATION_SCHEMA_VERSION,
        "identity": {
            "family": "cross_sectional_rank",
            "hypothesis": "the strongest trend in the cross-section persists over the next session",
            "mechanism": "rank the cross-section by a short moving average and hold the leaders",
            "falsification": "the ranked leaders earn no more than the cross-section median",
        },
        "universe": {
            "asset_classes": ["EQUITY"],
            "membership_policy": "point_in_time",
            "source": "discovery.fixture",
            "min_price": "1",
            "min_dollar_volume": "1000000",
            "exclusions": [],
        },
        "data": {
            "signal_frequency": "daily",
            "decision_time": "session_close",
            "execution_eligibility": "next_session_open",
            "corporate_action_treatment": "raw",
        },
        "features": {
            "declared": {
                "trend": {
                    "feature": TREND_ID,
                    "lookback": TREND_LOOKBACK,
                    "skip": 0,
                },
            },
        },
        "selection": {
            "mode": MODE_RANK,
            "feature": "trend",
            "count": "3",
            "tie_break": TIE_FEATURE_DESC,
        },
        "entry": {"direction": "long"},
        "exit": {"flatten_ineligible": True},
        "portfolio": {
            "position_count": 3,
            "weighting": RANK_PROPORTIONAL,
            "max_position_weight": str(MAX_POSITION_WEIGHT),
            "cash_policy": CASH_POLICY_LEAVE_IDLE,
        },
        "rebalance": {"frequency": "daily", "timing": "next_session_open"},
        "risk": {
            "pre_trade": {},
            "runtime": {"halt_new_entries": False, "flatten_ineligible": False},
            "post_run": {},
        },
        "evaluation": {
            "benchmark": "fixture-benchmark",
            "cost_model": "fixture-cost-model",
            "validation_policy": "fixture-validation",
        },
        "provenance": {"generator": "discovery.fixture", "seed": SEED, "parents": []},
    }


def catalog(directory: Path) -> Path:
    """
    Write the fixture's instruments, bars and quotes into a fresh Parquet catalog.

    The writer takes one instrument's data per call, so each instrument is written on its own: a
    chunk of a bar type or an instrument carries one identity.
    """
    written = ParquetDataCatalog(str(directory))
    written.write_instruments(list(instruments()))
    for instrument_id in CLOSES:
        written.write_bars(bars(instrument_id))
        written.write_quote_ticks(quotes(instrument_id))

    return directory


def venue() -> BacktestVenueConfig:
    """
    Return the single venue the run trades through, with a market-taker fee model.
    """
    return BacktestVenueConfig(
        name=str(VENUE),
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=[f"{STARTING_BALANCE} USD"],
        base_currency=USD,
        book_type="L1_MBP",
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )


def data_configs(catalog_path: Path) -> list[BacktestDataConfig]:
    """
    Return the bar and quote configurations the run replays the catalog through.
    """
    ids = [item.id for item in instruments()]

    return [
        BacktestDataConfig(
            data_type=NautilusDataType.Bar,
            catalog_path=str(catalog_path),
            instrument_ids=ids,
            bar_types=[f"{item_id}-{BAR_SPECIFICATION}" for item_id in ids],
        ),
        BacktestDataConfig(
            data_type=NautilusDataType.QuoteTick,
            catalog_path=str(catalog_path),
            instrument_ids=ids,
        ),
    ]


def run_inputs(plan: CompiledPlan) -> InterpreterRunInputs:
    """
    Return the run inputs the interpreter is configured from, over the compiled fixture plan.
    """
    memberships = build_memberships(
        "fixture-universe",
        "discovery.fixture",
        [(str(item.id), 0, None) for item in instruments()],
    )

    return InterpreterRunInputs(
        plan=plan,
        memberships=memberships,
        instrument_ids=tuple(str(item.id) for item in instruments()),
        venue=VENUE,
        commission_rate=Decimal(0),
        stop_loss_bps=STOP_LOSS_BPS,
        sectors=policy().sectors,
    )


def run_candidate(
    plan: CompiledPlan,
    catalog_path: Path,
) -> tuple[ExperimentOutcome, RecordingInterpreter]:
    """
    Run one experiment with the interpreter injected as a live object.

    Parameters
    ----------
    plan : CompiledPlan
        The compiled plan the run executes.
    catalog_path : Path
        The directory the run's instruments, bars and quotes were written to.

    Returns
    -------
    tuple[ExperimentOutcome, RecordingInterpreter]

    """
    strategy = RecordingInterpreter(DiscoveryInterpreterConfig(inputs=run_inputs(plan)))
    outcome = run_experiment(
        RunRequest(
            plan=plan,
            strategy=strategy,
            venues=[venue()],
            data=data_configs(catalog_path),
            engine=BacktestEngineConfig(bypass_logging=True, run_analysis=False),
            metrics=[SHARPE, DRAWDOWN],
            seed=SEED,
            snapshot=SNAPSHOT,
        ),
    )

    return outcome, strategy


def test_a_hand_written_candidate_round_trips_from_specification_to_a_ledger_entry(
    tmp_path: Path,
) -> None:
    """
    Compile one payload, run it, package the run, append the package, and refuse the rewrite.

    The interpreter's decisions, signals, targets, orders and fills are asserted in the run's own
    step, because they are what the run produced rather than what the chain asserts about it.
    """
    # The payload compiles into the plan that runs, and nothing was refused.
    compiled = compile_specification(payload(), policy(), FeatureRegistry((definition(),)))
    assert compiled.accepted, compiled.errors

    plan = compiled.plan
    assert plan is not None, compiled.errors

    # The plan runs once, over the catalog's bars and quotes, with the interpreter live in it.
    outcome, strategy = run_candidate(plan, catalog(tmp_path))

    # The experiment is the compiled plan's own behavioural identity, recomputable from the plan.
    assert outcome.experiment.digest == experiment_for(plan).digest

    # The run recorded the engine's canonical result, which is the artefact a later recomputation
    # compares against rather than a claim.
    assert outcome.run.canonical_digest.startswith("blake3:")
    assert outcome.run.canonical_result

    # The evidence package carries the run's own identities, bytes and metrics.
    package = outcome.package
    assert package.candidate_id == plan.behavioural_digest
    assert package.behavioural_digest == plan.behavioural_digest
    assert package.specification_digest == plan.specification_digest
    assert package.experiment_digest == outcome.experiment.digest
    assert package.canonical_digest == outcome.run.canonical_digest
    assert package.canonical_bytes == outcome.run.canonical_result
    assert package.metric_values == outcome.run.metric_values
    assert set(package.metric_values) == {SHARPE, DRAWDOWN}

    # Every fixture bar made one decision, because the interpreter subscribes to the run's bars.
    assert len(strategy.decisions) == sum(len(closes) for closes in CLOSES.values())
    assert strategy.decisions
    assert strategy.decisions[0].selection == ()
    assert strategy.decisions[0].cross_section == ()
    assert any(len(decision.selection) == 3 for decision in strategy.decisions)

    for decision in strategy.decisions:
        assert decision.rebalance
        assert all(entry.available_at >= entry.observed_at for entry in decision.cross_section)
        assert all(entry.member for entry in decision.cross_section)

        allocated = {
            str(target.instrument_id): float(target.value.weight or 0.0)
            for target in decision.targets
        }
        long_strengths: dict[str, float] = {}
        for signal in decision.signals:
            if signal.direction == "LONG":
                long_strengths[signal.instrument_id] = signal.strength
                weight = allocated[signal.instrument_id]
                assert weight > 0.0
                assert signal.strength == pytest.approx(
                    min(weight / float(MAX_POSITION_WEIGHT), 1.0),
                )
            elif signal.direction == "FLAT" and signal.instrument_id in allocated:
                assert allocated[signal.instrument_id] == 0.0

        ordered = [
            long_strengths[instrument_id]
            for instrument_id in decision.selection
            if instrument_id in long_strengths
        ]
        assert ordered == sorted(ordered, reverse=True)

    # A position leaving the selected set is the case the allocation flattens, so at least one
    # decision must submit the reduce-to-zero signal the reconciler turns into the closing order.
    flats = [
        signal.instrument_id
        for decision in strategy.decisions
        for signal in decision.signals
        if signal.direction == "FLAT"
    ]
    assert D in flats

    # The name that leaves the selection is the case the allocation flattens: it is allocated a
    # zero weight, which is what makes that flat signal a reduce-to-zero rather than an exit rule.
    zero_weight_targets = [
        str(target.instrument_id)
        for decision in strategy.decisions
        for target in decision.targets
        if float(target.value.weight or 0.0) == 0.0
    ]
    assert D in zero_weight_targets

    submitted = {
        client_order_id
        for decision in strategy.decisions
        for client_order_id in decision.client_order_ids
    }
    orders = strategy.engine_orders
    assert orders
    assert all(isinstance(order, MarketOrder) for order in orders)
    assert {str(order.client_order_id) for order in orders} <= submitted

    assert strategy.fills
    for fill in strategy.fills:
        # Every fill is attributed to the decision that produced its order, and the engine's fill
        # instant is never earlier than that decision: it is the platform's fill, not a target the
        # interpreter invented.
        assert fill.decision_at is not None
        assert fill.ts_event >= fill.decision_at
        assert fill.client_order_id in submitted
    assert {fill.client_order_id for fill in strategy.fills} == submitted

    # The ledger writes one entry, keyed and named for the plan's behavioural digest.
    ledger = DiscoveryLedger(tmp_path / "ledger")
    path = ledger.append(package)
    assert path.stem == plan.behavioural_digest.replace(":", "_")
    assert ledger.candidate_ids() == (plan.behavioural_digest,)
    assert len(ledger) == 1

    # The entry the reader gets back records the identity and the canonical digest of this run.
    entry = ledger.entries()[0]
    assert entry["candidate_id"] == plan.behavioural_digest
    assert entry["behavioural_digest"] == plan.behavioural_digest
    assert entry["specification_digest"] == plan.specification_digest
    assert entry["policy_digest"] == plan.policy_digest
    assert entry["registry_digest"] == plan.registry_digest
    assert entry["experiment_digest"] == outcome.experiment.digest
    assert entry["canonical_digest"] == outcome.run.canonical_digest
    assert entry["engine_version"] == plan.engine_version
    assert entry["snapshot"] == SNAPSHOT
    assert entry["seed"] == SEED
    assert entry["status"] == "completed"

    # An append-only ledger refuses to rewrite what was already learned.
    with pytest.raises(FileExistsError, match=plan.behavioural_digest):
        ledger.append(package)
    assert len(ledger) == 1
