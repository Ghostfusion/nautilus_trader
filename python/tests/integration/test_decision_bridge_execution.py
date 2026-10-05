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
End-to-end proof that a research decision artifact becomes exactly one engine order and fill.

One strategy carries a `ResearchDecisionCarrier` across the platform's custom-data path, projects
the artifact it carries, records the arrival on an `AdmissionLedger`, submits the projected signal
through the target pipeline, and attributes the resulting fill back to the decision. The engine is
real: venue, instrument, quotes, matching, risk and portfolio are all the running backtest's.

What is proven here, and nowhere else, is the join itself. A canonical artifact produces exactly one
order set and one fill. The artifact's advisory allocation lowers the engine's own construction, so
research may only cap and never enlarge. A replay of the same artifact submits nothing and adds no
order or fill, because the decision's orders are already on the ledger. A replay after the risk engine
has positively refused the order set also adds no order, so a refusal does not free the identity.
Every fill resolves to exactly one decision id, and an order the bridge never placed is reported as
`UNATTRIBUTED` rather than assigned to the nearest decision.

The venue is XNAS, the instrument AAPL.XNAS, the account holds 1,000,000 USD, the mid of the quotes
is 100.000, and the pipeline risks 0.0001 of equity against a 1 per cent stop, so the uncapped
construction resolves to 100 units. The artifact's instants fall inside the bundled XNYS.EQUITY
calendar's coverage (valid 2024-01-01 to 2026-12-31).
"""

from __future__ import annotations

import json
from datetime import date
from decimal import Decimal
from pathlib import Path
from typing import Any

import pytest

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.decision_bridge.artifact import decision_id_for
from nautilus_trader.decision_bridge.artifact import revision_id_for
from nautilus_trader.decision_bridge.carrier import ResearchDecisionCarrier
from nautilus_trader.decision_bridge.contract import Outcome
from nautilus_trader.decision_bridge.execution import position_state
from nautilus_trader.decision_bridge.execution import record_decision
from nautilus_trader.decision_bridge.execution import record_engine_event
from nautilus_trader.decision_bridge.execution import record_fill_once
from nautilus_trader.decision_bridge.execution import submit_once
from nautilus_trader.decision_bridge.ledger import ADMITTED
from nautilus_trader.decision_bridge.ledger import UNATTRIBUTED
from nautilus_trader.decision_bridge.ledger import AdmissionLedger
from nautilus_trader.decision_bridge.projection import ProjectionConfig
from nautilus_trader.decision_bridge.projection import ProjectionContext
from nautilus_trader.decision_bridge.projection import project
from nautilus_trader.decision_bridge.risk import RiskLimits
from nautilus_trader.decision_bridge.risk import build_risk_engine_config
from nautilus_trader.decision_bridge.tradability import Tradability
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import CustomData
from nautilus_trader.model import DataType
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import TradingCalendar
from nautilus_trader.model import Venue
from nautilus_trader.model import register_custom_data_class
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from nautilus_trader.trading import TargetPipelineConfig
from tests.providers import TestInstrumentProvider
from tests.unit.decision_bridge import fixtures


INSTRUMENT = TestInstrumentProvider.aapl_equity()
INSTRUMENT_ID = INSTRUMENT.id
XNAS = Venue("XNAS")
USD = Currency.from_str("USD")

START_NS = fixtures.unix_ns(2025, 6, 2, 16, 50)
INTERVAL_NS = 60_000_000_000
QUOTE_COUNT = 40
ARTIFACT_NS = fixtures.unix_ns(2025, 6, 2, 17, 0)

BASE_PIPELINE = TargetPipelineConfig(
    risk_per_trade=Decimal("0.0001"),
    stop_loss_bps=100,
    max_weight=Decimal("1"),
    commission_rate=Decimal("0"),
    min_order_quantity=Quantity.from_int(0),
)

PROJECTION_CONFIG = ProjectionConfig(
    horizon_ns=86_400_000_000_000,
    authorized_producers=fixtures.AUTHORIZED_PRODUCERS,
)

register_custom_data_class(ResearchDecisionCarrier)
DATA_TYPE = DataType("ResearchDecisionCarrier", {"source": "tradingagents"}, str(INSTRUMENT_ID))


class BridgeStrategyConfig(StrategyConfig):
    """
    Configure the end-to-end bridge strategy.
    """

    def __init__(
        self,
        *,
        instrument_id: Any,
        ledger_directory: Path,
        calendar: TradingCalendar,
        data_type: DataType,
        place_direct_order: bool,
    ) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.instrument_id = instrument_id
        self.ledger_directory = ledger_directory
        self.calendar = calendar
        self.data_type = data_type
        self.place_direct_order = place_direct_order


class BridgeExecutionStrategy(Strategy):
    """
    Drive the bridge chain from custom data, using only the package's public API.
    """

    def __init__(self, config: BridgeStrategyConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self.ledger = AdmissionLedger(config.ledger_directory)
        self.projections: list[Any] = []
        self.admissions: list[str] = []
        self.submissions: list[Any] = []
        self.attributions: list[str] = []
        self.fill_ids: list[str] = []
        self.denials: list[str] = []
        self.event_attributions: list[str] = []
        self.direct_order_id: str | None = None
        self.direct_fill_id: str | None = None
        self.direct_attribution: str | None = None
        self._direct_placed = False

    def on_start(self) -> None:
        """
        On start.
        """
        self.subscribe_data(self.config.data_type)
        self.subscribe_quotes(self.config.instrument_id)

    def on_quote(self, _quote: QuoteTick) -> None:
        """
        Place one order on the strategy's own path, to be reported as unattributed.
        """
        if not self.config.place_direct_order or self._direct_placed:
            return
        self._direct_placed = True
        order = self.order_factory.market(
            instrument_id=self.config.instrument_id,
            order_side=OrderSide.BUY,
            quantity=Quantity.from_int(10),
        )
        self.direct_order_id = str(order.client_order_id)
        self.submit_order(order)

    def on_data(self, data: CustomData) -> None:
        """
        Project the carried artifact, record it, and submit its signal at most once.
        """
        raw = json.loads(data.data.document)
        instrument_id = self.config.instrument_id
        now = self.clock.timestamp_ns()
        projection = project(
            raw,
            context=ProjectionContext(
                received_at=now,
                tradability=Tradability.TRADABLE,
                position=position_state(self.portfolio.net_position(instrument_id)),
                engine_eligible=True,
                calendar=self.config.calendar,
            ),
            config=PROJECTION_CONFIG,
            resolve_instrument=fixtures.resolve_instrument,
        )
        self.projections.append(projection)
        self.admissions.append(
            record_decision(
                ledger=self.ledger,
                projection=projection,
                received_at=now,
                admission_timestamp=now,
            ),
        )
        self.submissions.append(
            submit_once(
                ledger=self.ledger,
                projection=projection,
                strategy=self,
                base_pipeline=BASE_PIPELINE,
                ceiling=projection.ceiling,
            ),
        )

    def on_order_filled(self, event: Any) -> None:
        """
        Attribute the fill to the decision that produced its order, if any.
        """
        client_order_id = str(event.client_order_id)
        fill_id = str(event.trade_id)
        attribution = record_fill_once(
            ledger=self.ledger,
            client_order_id=client_order_id,
            fill_id=fill_id,
            filled_pct=None,
        )
        if client_order_id == self.direct_order_id:
            self.direct_fill_id = fill_id
            self.direct_attribution = attribution
            return
        self.fill_ids.append(fill_id)
        self.attributions.append(attribution)

    def on_order_denied(self, event: Any) -> None:
        """
        Record that the engine positively refused an order set, and the risk stage's answer.
        """
        client_order_id = str(event.client_order_id)
        self.denials.append(client_order_id)
        self.event_attributions.append(
            record_engine_event(
                ledger=self.ledger,
                client_order_id=client_order_id,
                reason=str(event.reason),
            ),
        )

    def on_order_submitted(self, event: Any) -> None:
        """
        Record the risk stage's answer for an order the engine put on its submission path.
        """
        self.event_attributions.append(
            record_engine_event(
                ledger=self.ledger,
                client_order_id=str(event.client_order_id),
                reason=None,
            ),
        )


def _calendar() -> TradingCalendar:
    """
    Return the bundled calendar, or skip when this build carries none.
    """
    calendar = TradingCalendar.bundled("XNYS", "EQUITY")
    if calendar is None:
        pytest.skip("the bundled XNYS.EQUITY calendar is not available in this build")
    return calendar


def _carrier(raw: dict[str, Any], ts_ns: int) -> ResearchDecisionCarrier:
    """
    Wrap a raw artifact as the custom data the platform carries unchanged.
    """
    revision_id = revision_id_for(raw["idempotency_key"], raw["artifact_sha256"])
    decision_id = decision_id_for(
        INSTRUMENT_ID,
        date.fromisoformat(raw["effective_date"]),
        revision_id,
    )
    return ResearchDecisionCarrier(
        ts_event=ts_ns,
        ts_init=ts_ns,
        decision_id=decision_id,
        artifact_sha256=raw["artifact_sha256"],
        idempotency_key=raw["idempotency_key"],
        produced_at=raw["produced_at"],
        effective_date=raw["effective_date"],
        document=json.dumps(raw, sort_keys=True),
    )


def _quotes() -> list[QuoteTick]:
    quotes: list[QuoteTick] = []
    for index in range(QUOTE_COUNT):
        ts = START_NS + index * INTERVAL_NS
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


def _run(
    tmp_path: Path,
    artifacts: list[dict[str, Any]],
    *,
    name: str,
    risk_engine: RiskEngineConfig | None = None,
    place_direct_order: bool = False,
) -> tuple[dict[str, Any], BridgeExecutionStrategy, AdmissionLedger]:
    """
    Run one backtest that delivers the artifacts and returns its canonical result.
    """
    ledger_directory = tmp_path / name
    engine = BacktestEngine(
        BacktestEngineConfig(
            bypass_logging=True,
            run_analysis=False,
            risk_engine=risk_engine if risk_engine is not None else RiskEngineConfig(bypass=True),
        ),
    )
    try:
        engine.add_venue(
            venue=XNAS,
            oms_type=OmsType.NETTING,
            account_type=AccountType.CASH,
            base_currency=USD,
            starting_balances=[Money(1_000_000.0, USD)],
            fee_model=MakerTakerFeeModel(maker_rate=Decimal("0"), taker_rate=Decimal("0")),
        )
        engine.add_instrument(INSTRUMENT)
        engine.add_data(_quotes())
        strategy = BridgeExecutionStrategy(
            BridgeStrategyConfig(
                instrument_id=INSTRUMENT_ID,
                ledger_directory=ledger_directory,
                calendar=_calendar(),
                data_type=DATA_TYPE,
                place_direct_order=place_direct_order,
            ),
        )
        engine.add_strategy(strategy)
        engine.add_data(
            [
                CustomData(DATA_TYPE, _carrier(raw, ARTIFACT_NS + index * INTERVAL_NS))
                for index, raw in enumerate(artifacts)
            ],
            sort=True,
        )
        engine.run(start=START_NS, end=START_NS + QUOTE_COUNT * INTERVAL_NS)
        document = json.loads(engine.get_canonical_result().to_bytes().decode("utf-8"))
        return document, strategy, AdmissionLedger(ledger_directory)
    finally:
        engine.dispose()


def _engine_orders(document: dict[str, Any]) -> list[str]:
    """
    Return the client order ids the engine's canonical result reports.
    """
    return [next(iter(record.values()))["core"]["client_order_id"] for record in document["orders"]]


def _engine_fills(document: dict[str, Any]) -> list[tuple[str, Decimal]]:
    """
    Return the (client order id, quantity) of every fill the engine reports.
    """
    fills: list[tuple[str, Decimal]] = []
    for record in document["fills"]:
        payload = next(iter(record["event"].values()))
        fills.append((record["client_order_id"], Decimal(payload["last_qty"])))
    return fills


def test_a_canonical_artifact_produces_exactly_one_order_and_one_fill(tmp_path: Path) -> None:
    """
    One canonical artifact yields exactly one order set and one engine fill.
    """
    document, strategy, ledger = _run(tmp_path, [fixtures.covered_artifact()], name="canonical")

    assert len(strategy.projections) == 1
    assert strategy.projections[0].outcome is Outcome.SIGNAL
    assert strategy.admissions == [ADMITTED]

    submission = strategy.submissions[0]
    assert submission.outcome is Outcome.SIGNAL
    assert len(submission.client_order_ids) == 1

    assert len(_engine_orders(document)) == 1
    assert len(_engine_fills(document)) == 1

    # The artifact requests 5 percent; the construction stage sizes by its own risk budget
    # (0.0001 of equity against a 1 percent stop, so 1 percent), which the ceiling does not
    # bind, and the risk engine approves what the construction produced. The fill quantity is
    # not measured by this test, so that stage is reported as not reached, never as a zero.
    record = ledger.record(strategy.projections[0].decision.decision_id)
    assert record is not None
    assert record.research_requested == pytest.approx(5.0)
    assert record.bridge_capped == pytest.approx(5.0)
    assert record.engine_constructed == pytest.approx(1.0)
    assert record.risk_approved == pytest.approx(record.engine_constructed)
    assert record.not_reached == ("actually_filled",)


def test_the_advisory_ceiling_lowers_the_engine_construction(tmp_path: Path) -> None:
    """
    The artifact's allocation caps the construction below the uncapped run's quantity.
    """
    lowered_document, lowered_strategy, lowered_ledger = _run(
        tmp_path,
        [fixtures.covered_artifact(recommended_allocation_pct=0.5)],
        name="lowered",
    )
    baseline_document, baseline_strategy, baseline_ledger = _run(
        tmp_path,
        [fixtures.covered_artifact(recommended_allocation_pct=None)],
        name="baseline",
    )

    lowered = _engine_fills(lowered_document)[0][1]
    baseline = _engine_fills(baseline_document)[0][1]

    assert lowered < baseline, (
        f"the ceiling did not lower the construction: "
        f"with the artifact allocation the filled quantity was {lowered}, "
        f"without any allocation it was {baseline}"
    )

    # The ceiling binds on the lowered run, so the construction is the capped allocation; the
    # baseline carries no ceiling and keeps the pipeline's own 1 percent. Both the construction
    # and the approval are recorded in the artifact's percent units, and the risk engine
    # approves exactly what the construction produced.
    lowered_record = lowered_ledger.record(lowered_strategy.projections[0].decision.decision_id)
    baseline_record = baseline_ledger.record(baseline_strategy.projections[0].decision.decision_id)
    assert lowered_record is not None
    assert baseline_record is not None
    assert lowered_record.bridge_capped == pytest.approx(0.5)
    assert lowered_record.engine_constructed == pytest.approx(0.5)
    assert lowered_record.risk_approved == pytest.approx(0.5)
    assert baseline_record.bridge_capped is None
    assert baseline_record.engine_constructed == pytest.approx(1.0)
    assert baseline_record.risk_approved == pytest.approx(1.0)


def test_a_replayed_artifact_produces_no_second_order_or_fill(tmp_path: Path) -> None:
    """
    A second delivery of the same artifact submits nothing and adds no order or fill.
    """
    raw = fixtures.covered_artifact()
    document, strategy, ledger = _run(tmp_path, [raw, raw], name="replay")

    assert [submission.outcome for submission in strategy.submissions] == [
        Outcome.SIGNAL,
        Outcome.DUPLICATE,
    ]
    assert strategy.submissions[1].client_order_ids == strategy.submissions[0].client_order_ids

    assert len(_engine_orders(document)) == 1
    assert len(_engine_fills(document)) == 1

    decision_id = strategy.projections[0].decision.decision_id
    record = ledger.record(decision_id)
    assert record is not None
    assert len(record.order_ids) == 1
    assert len(record.fill_ids) == 1
    assert len(ledger.duplicates(decision_id)) == 1
    # A replay is a repeated delivery, not a deliberate attempt, and the one order produced one
    # submission event: the risk stage approved what the construction produced.
    assert record.attempts == ()
    assert strategy.event_attributions == [decision_id]
    assert record.risk_approved == pytest.approx(record.engine_constructed)


def test_a_replay_after_an_engine_refusal_produces_no_second_order(tmp_path: Path) -> None:
    """
    A replay after the risk engine refuses the order set adds no order id.
    """
    raw = fixtures.covered_artifact()
    limits = RiskLimits(
        max_notional_per_order={str(INSTRUMENT_ID): "1"},
        count_caps=(),
        max_order_submit_rate="100/00:00:01",
        max_order_modify_rate="100/00:00:01",
    )
    document, strategy, ledger = _run(
        tmp_path,
        [raw, raw],
        name="refusal",
        risk_engine=build_risk_engine_config(limits),
    )

    assert len(strategy.denials) == 1
    assert [submission.outcome for submission in strategy.submissions] == [
        Outcome.SIGNAL,
        Outcome.DUPLICATE,
    ]

    decision_id = strategy.projections[0].decision.decision_id
    record = ledger.record(decision_id)
    assert record is not None
    assert len(record.order_ids) == 1
    assert record.fill_ids == ()
    assert len(_engine_orders(document)) == 1
    assert _engine_fills(document) == []

    # The denial is the risk stage's own answer, so the approved exposure is a measured zero
    # while the construction that preceded it is recorded as it was built. The denied order never
    # reached the submission path: one event for it, a denial and not a submission.
    assert strategy.event_attributions == [decision_id]
    assert record.engine_constructed == pytest.approx(1.0)
    assert record.risk_approved == 0.0


def test_every_fill_resolves_to_one_decision_and_a_direct_order_is_unattributed(
    tmp_path: Path,
) -> None:
    """
    Every fill resolves to its decision, and an order the bridge never placed is unattributed.
    """
    document, strategy, ledger = _run(
        tmp_path,
        [fixtures.covered_artifact()],
        name="attribution",
        place_direct_order=True,
    )

    decision_id = strategy.projections[0].decision.decision_id
    bridge_order_id = strategy.submissions[0].client_order_ids[0]
    assert strategy.fill_ids
    bridge_fill_id = strategy.fill_ids[0]

    assert strategy.attributions == [decision_id]
    assert ledger.decision_for_order(bridge_order_id) == decision_id
    assert ledger.decision_for_fill(bridge_fill_id) == decision_id

    assert len(strategy.fill_ids) == 1
    assert len(_engine_fills(document)) == 2
    assert strategy.direct_order_id is not None
    assert strategy.direct_attribution == UNATTRIBUTED
    assert ledger.decision_for_order(strategy.direct_order_id) == UNATTRIBUTED
    assert strategy.direct_fill_id is not None
    assert ledger.decision_for_fill(strategy.direct_fill_id) == UNATTRIBUTED

    # Both orders reached the engine's submission path; the bridge's resolves to its decision and
    # the strategy's own order is reported as unattributed rather than assigned to the nearest.
    assert set(strategy.event_attributions) == {decision_id, UNATTRIBUTED}
