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
The recorded execution timing of a bar-driven backtest, as the discovery design declares it.

Design Section 8.5 separates the decision time from the execution eligibility and asserts that a
deferral exists that the compiler can emit. This module records what the engine actually does, and
keeps the unsafe case as the pre-enforcement half of the settlement pair: an intent submitted from
`on_bar` with no deferral settles at the decision bar's own close, at the decision bar's own
timestamp.

Every assertion here observes recorded event ordering, fill timestamps and fill prices, and the
account balance. None of them observes a configuration value, because a configured latency is not
evidence that the matching engine honoured the timing.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import date
from decimal import Decimal
from pathlib import Path
from typing import Any

import pytest

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.config import DataEngineConfig
from nautilus_trader.decision_bridge.artifact import decision_id_for
from nautilus_trader.decision_bridge.artifact import revision_id_for
from nautilus_trader.decision_bridge.carrier import ResearchDecisionCarrier
from nautilus_trader.decision_bridge.contract import Outcome
from nautilus_trader.decision_bridge.execution import position_state
from nautilus_trader.decision_bridge.execution import record_decision
from nautilus_trader.decision_bridge.execution import submit_once
from nautilus_trader.decision_bridge.ledger import AdmissionLedger
from nautilus_trader.decision_bridge.projection import ProjectionConfig
from nautilus_trader.decision_bridge.projection import ProjectionContext
from nautilus_trader.decision_bridge.projection import project
from nautilus_trader.decision_bridge.tradability import Tradability
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.execution import StaticLatencyModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
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
from nautilus_trader.model import register_custom_data_class
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from nautilus_trader.trading import TargetPipelineConfig
from tests.providers import TestInstrumentProvider
from tests.unit.decision_bridge import fixtures


USD = Currency.from_str("USD")
INSTRUMENT = TestInstrumentProvider.aapl_equity()
INSTRUMENT_ID = INSTRUMENT.id
VENUE = INSTRUMENT.venue
BAR_TYPE = BarType.from_str(f"{INSTRUMENT_ID}-1-MINUTE-LAST-EXTERNAL")

SECOND_NS = 1_000_000_000
MINUTE_NS = 60 * SECOND_NS
TS_START = 1_704_067_200_000_000_000
# Strictly between the timestamp of the first bar and the timestamp of the second.
BETWEEN_BARS_NS = 30 * SECOND_NS
TRADE_SIZE = "100"
CLOSES = ("100.25", "101.25", "102.25", "103.25")
OPENS = ("100.00", "101.00", "102.00", "103.00")
SUBSCRIBE_BARS = "subscribe_bars"
SUBMIT_ORDER = "submit_order"
LOW = 0.50


@dataclass(frozen=True)
class Lifecycle:
    """
    The recorded lifecycle of one intent.

    `fill_ts` and `fill_px` come from the strategy's fill callback when it received one, and from
    the order otherwise, which is what distinguishes a fill the strategy observed from one the
    engine recorded without notifying it.
    """

    callbacks: tuple[str, ...]
    decision_ts: int | None
    submission_ts: int | None
    fill_ts: int | None
    fill_px: str | None
    fill_observed_by_strategy: bool
    position_ts: int | None
    timer_ts: int | None
    order_status: str
    order_ts_last: int | None
    account_total: str | None


class TimingConfig(StrategyConfig):
    """
    Configure the recording strategy.
    """

    def __init__(
        self,
        *,
        bar_type: str,
        trade_size: str,
        submit_from: str,
        timer_ns: int | None = None,
        quotes: bool = False,
    ) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.bar_type = bar_type
        self.trade_size = trade_size
        self.submit_from = submit_from
        self.timer_ns = timer_ns
        self.quotes = quotes


class RecordingStrategy(Strategy):
    """
    Record every lifecycle callback, and submit one market order from the bar or from a timer.
    """

    def __init__(self, config: TimingConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self._quantity = Quantity.from_str(config.trade_size)
        self._submit_from = config.submit_from
        self._timer_ns = config.timer_ns
        self._quotes = config.quotes
        self._bars_seen = 0
        self._sent = False
        self.callbacks: list[str] = []
        self.decision_ts: int | None = None
        self.submission_ts: int | None = None
        self.fill_ts: int | None = None
        self.fill_px: str | None = None
        self.position_ts: int | None = None
        self.timer_ts: int | None = None

    def _submit(self) -> None:
        self._sent = True
        self.callbacks.append(SUBMIT_ORDER)
        self.submit_order(
            self.order_factory.market(
                instrument_id=INSTRUMENT_ID,
                order_side=OrderSide.BUY,
                quantity=self._quantity,
            ),
        )

    def on_start(self) -> None:
        """
        Subscribe to the bar stream, to quotes when configured, and arm the timer.
        """
        self.subscribe_bars(self._bar_type)
        self.callbacks.append(SUBSCRIBE_BARS)
        if self._quotes:
            self.subscribe_quotes(INSTRUMENT_ID)
        if self._timer_ns is not None:
            self.clock.set_time_alert_ns("probe", self._timer_ns)

    def on_bar(self, bar: Bar) -> None:
        """
        Record the bar, and submit from the first one when the bar owns the submission.
        """
        self.callbacks.append("on_bar")
        self._bars_seen += 1
        if self._bars_seen == 1:
            self.decision_ts = bar.ts_init
        if self._bars_seen == 1 and self._submit_from == "bar" and not self._sent:
            self._submit()

    def on_time_event(self, event: Any) -> None:
        """
        Record the timer, and submit from it when the timer owns the submission.
        """
        self.callbacks.append("on_time_event")
        self.timer_ts = event.ts_init
        if self._submit_from == "timer" and not self._sent:
            self._submit()

    def on_order_submitted(self, event: Any) -> None:
        """
        Record the submission and its timestamp.
        """
        self.callbacks.append("on_order_submitted")
        self.submission_ts = event.ts_init

    def on_order_filled(self, event: Any) -> None:
        """
        Record the fill, its timestamp and its price.
        """
        self.callbacks.append("on_order_filled")
        self.fill_ts = event.ts_event
        self.fill_px = str(event.last_px)

    def on_position_opened(self, event: Any) -> None:
        """
        Record the position opening and its timestamp.
        """
        self.callbacks.append("on_position_opened")
        self.position_ts = event.ts_event


def _bar(
    bar_type: BarType,
    open_price: str,
    close: str,
    ts_event: int,
    ts_init: int,
) -> Bar:
    """
    Build one bar with distinct open, high, low and close.
    """
    open_px = Price.from_str(open_price)
    close_px = Price.from_str(close)
    high = max(open_px, close_px)
    low = min(open_px, close_px)
    return Bar(
        bar_type,
        open_px,
        Price.from_str(f"{high.as_double() + LOW:.2f}"),
        Price.from_str(f"{low.as_double() - LOW:.2f}"),
        close_px,
        Quantity.from_int(1_000_000),
        ts_event,
        ts_init,
    )


def bars(
    *,
    closes: tuple[str, ...] = CLOSES,
    offsets: tuple[int, ...] = (0, 1, 2, 3),
) -> list[Bar]:
    """
    Build one-minute bars at the given offsets, with the given closes.
    """
    result: list[Bar] = []
    for index, offset in enumerate(offsets):
        ts = TS_START + offset * MINUTE_NS
        result.append(
            _bar(
                BAR_TYPE,
                OPENS[index] if index < len(OPENS) else closes[index],
                closes[index],
                ts,
                ts,
            ),
        )
    return result


def quote(price: str, ts: int) -> QuoteTick:
    """
    Build a quote at the given price and timestamp.
    """
    px = Price.from_str(price)
    return QuoteTick(
        INSTRUMENT_ID,
        px,
        px,
        Quantity.from_int(1_000),
        Quantity.from_int(1_000),
        ts,
        ts,
    )


def run(
    *,
    bars_data: list[Bar] | None = None,
    latency_ns: int | None = None,
    quote_data: QuoteTick | None = None,
    timer_ns: int | None = None,
    submit_from: str = "bar",
    validate_sequence: bool = False,
) -> Lifecycle:
    """
    Run one timing scenario and return its recorded lifecycle.
    """
    engine = BacktestEngine(
        BacktestEngineConfig(
            bypass_logging=True,
            run_analysis=False,
            data_engine=DataEngineConfig(validate_data_sequence=validate_sequence),
        ),
    )
    engine.add_venue(
        venue=VENUE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money(1_000_000.0, USD)],
        base_currency=USD,
        latency_model=None
        if latency_ns is None
        else StaticLatencyModel(base_latency_nanos=latency_ns),
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    engine.add_instrument(INSTRUMENT)
    engine.add_data(bars() if bars_data is None else bars_data)
    if quote_data is not None:
        engine.add_data([quote_data])
    strategy = RecordingStrategy(
        TimingConfig(
            bar_type=str(BAR_TYPE),
            trade_size=TRADE_SIZE,
            submit_from=submit_from,
            timer_ns=timer_ns,
            quotes=quote_data is not None,
        ),
    )
    engine.add_strategy(strategy)

    try:
        engine.run()
        orders = engine.cache.orders()
        order = orders[0] if orders else None
        account = engine.cache.account_for_venue(VENUE)
        balance = None
        if account is not None:
            balance = account.balances()[USD]
        fill_ts = strategy.fill_ts
        fill_px = strategy.fill_px
        if order is not None:
            if fill_ts is None:
                fill_ts = order.ts_last
            if fill_px is None:
                fill_px = str(order.avg_px)
        return Lifecycle(
            callbacks=tuple(strategy.callbacks),
            decision_ts=strategy.decision_ts,
            submission_ts=strategy.submission_ts,
            fill_ts=fill_ts,
            fill_px=fill_px,
            fill_observed_by_strategy=strategy.fill_ts is not None,
            position_ts=strategy.position_ts,
            timer_ts=strategy.timer_ts,
            order_status="NONE" if order is None else order.status.name,
            order_ts_last=None if order is None else order.ts_last,
            account_total=None if balance is None else str(balance.total),
        )
    finally:
        engine.dispose()


def test_a_bar_callback_intent_settles_at_the_decision_bars_own_close() -> None:
    """
    Test the unsafe baseline: without a deferral the intent fills at the decision bar's close.

    This is the pre-enforcement half of the settlement pair. The fill carries the decision bar's
    own timestamp, so the strategy is filled at a price the bar it decided from had just published,
    and the account is debited at that same instant.
    """
    lifecycle = run()

    assert lifecycle.callbacks[:5] == (
        SUBSCRIBE_BARS,
        "on_bar",
        SUBMIT_ORDER,
        "on_order_submitted",
        "on_order_filled",
    )
    assert lifecycle.decision_ts == TS_START
    assert lifecycle.fill_ts == TS_START
    assert lifecycle.fill_px == CLOSES[0]
    # The position is mutated at the fill instant, with no settlement step between them.
    assert lifecycle.position_ts == lifecycle.fill_ts
    assert lifecycle.order_status == "FILLED"
    # 1_000_000 - (100 x 100.25).
    assert lifecycle.account_total == "989975.00 USD"


def test_a_latency_model_defers_the_fill_to_the_first_bar_at_or_after_arrival() -> None:
    """
    Test the deferral: with latency the intent settles with a later bar, at that bar's close.
    """
    lifecycle = run(latency_ns=SECOND_NS)

    assert lifecycle.submission_ts == TS_START
    assert lifecycle.decision_ts is not None
    assert lifecycle.fill_ts is not None
    assert lifecycle.fill_ts == TS_START + MINUTE_NS
    assert lifecycle.fill_ts > lifecycle.decision_ts
    assert lifecycle.fill_px == CLOSES[1]
    assert lifecycle.order_status == "FILLED"
    # The bar that released the intent was dispatched to the strategy before it settled.
    assert lifecycle.callbacks.index("on_order_filled") > lifecycle.callbacks.index("on_bar")


def test_a_deferred_fill_is_never_earlier_than_its_arrival_timestamp() -> None:
    """
    Test that the deferral is a lower bound: an arrival inside the third bar settles no earlier.

    The arrival here is 90 s after the decision, which falls between the second and third bars, so
    the first bar at or after the arrival is the third, whose timestamp is 120 s.
    """
    lifecycle = run(latency_ns=90 * SECOND_NS)

    assert lifecycle.fill_ts is not None
    assert lifecycle.fill_ts >= TS_START + 90 * SECOND_NS
    assert lifecycle.fill_ts == TS_START + 2 * MINUTE_NS
    assert lifecycle.fill_px == CLOSES[2]
    assert lifecycle.fill_observed_by_strategy is True


def test_an_unrestricted_settlement_point_settles_against_the_last_known_close() -> None:
    """
    Test that a timer releases the intent against the book left by the previous bar.

    A strategy timer is an unrestricted settlement point, so it settles every command that is due.
    What it prices against is the last bar's book, which is the previous session's close, and never
    the next session's open.
    """
    lifecycle = run(submit_from="timer", timer_ns=TS_START + BETWEEN_BARS_NS)

    assert lifecycle.timer_ts == TS_START + BETWEEN_BARS_NS
    assert lifecycle.fill_ts == TS_START + BETWEEN_BARS_NS
    assert lifecycle.fill_px == CLOSES[0]
    assert lifecycle.fill_px != OPENS[1]


def test_market_data_between_bars_releases_a_deferred_intent_at_its_own_price() -> None:
    """
    Test the only path that yields a price observed at the eligibility instant.

    A quote arriving after the arrival timestamp releases the deferred command and settles it at
    the quoted price, at the quote's timestamp. With quote data present at a session's open, this
    is how a next-session-open intent gets the opening price; with bar-only data it cannot.
    """
    lifecycle = run(
        latency_ns=SECOND_NS,
        quote_data=quote("101.00", TS_START + BETWEEN_BARS_NS),
    )

    assert lifecycle.fill_ts == TS_START + BETWEEN_BARS_NS
    assert lifecycle.fill_px == "101.00"
    assert lifecycle.fill_ts < TS_START + MINUTE_NS


def test_a_deferred_intent_with_no_subsequent_market_data_is_not_filled() -> None:
    """
    Test that no fill is fabricated: a deferred intent the data never reached stays unexecuted.

    No bar follows the only bar in the run, so no market data can release the command, and the run
    ends before the command's arrival. The run reports the order as it stands, priced by nothing,
    and leaves the account undebited, because a fill at a close observed before the order existed
    is not a fill the simulator can defend. This is the case table's missing-bar and holiday case,
    which the engine now meets rather than the opposite.
    """
    lifecycle = run(bars_data=bars(offsets=(0,)), latency_ns=SECOND_NS)

    # The order reached the venue's queue and nothing ever promoted it: the deferred arrival stayed
    # in the future for the whole run, so the venue never acknowledged the order at all.
    assert lifecycle.order_status == "SUBMITTED"
    assert lifecycle.fill_observed_by_strategy is False
    assert lifecycle.position_ts is None
    assert lifecycle.account_total == "1000000.00 USD"


def test_out_of_order_input_is_sorted_rather_than_refused() -> None:
    """
    Test the timestamp-order case: bars are replayed in ascending initialization order.

    The bars are added as first, third, second and fourth by timestamp. Nothing refuses the input
    and nothing reports a repair; the engine orders the stream by `ts_init` and replays it, so the
    third bar's close is delivered second. The declared expectation is that the run refuses the
    input rather than silently sorting it.
    """
    lifecycle = run(bars_data=bars(offsets=(0, 2, 1, 3)))

    assert lifecycle.callbacks.count("on_bar") == 4
    assert lifecycle.fill_ts == TS_START
    assert lifecycle.fill_px == CLOSES[0]


def test_sequence_validation_on_a_sorted_stream_drops_nothing() -> None:
    """
    Test that the data engine's sequence validation cannot reach the ordering case.

    With sequence validation enabled the same out-of-order input is still replayed in full, because
    the data iterator orders the stream by initialization timestamp before the engine sees it, so
    the validation never observes a violation. The declared expectation is that the run refuses
    out-of-order input, and no engine configuration reaches that behaviour.
    """
    lifecycle = run(bars_data=bars(offsets=(0, 2, 1, 3)), validate_sequence=True)

    assert lifecycle.callbacks.count("on_bar") == 4
    assert lifecycle.fill_ts == TS_START


def test_a_bar_is_dispatched_and_stamped_at_its_initialization_timestamp() -> None:
    """
    Test the delayed-availability case: availability, not the event timestamp, governs the clock.

    A bar whose `ts_init` is a minute after its `ts_event` is dispatched at `ts_init`, and an intent
    decided from it is stamped at `ts_init`, so a decision cannot be taken from it at its event
    time. The reverse case shows the same key: a bar initialized before its event timestamp moves
    the replay clock backwards and the fill is stamped there.
    """
    ts_init = TS_START + MINUTE_NS
    late = run(bars_data=[_bar(BAR_TYPE, OPENS[0], CLOSES[0], TS_START, ts_init)])

    assert late.decision_ts == ts_init
    assert late.fill_ts == ts_init
    assert late.fill_px == CLOSES[0]

    early_ts_init = TS_START - MINUTE_NS
    early = run(bars_data=[_bar(BAR_TYPE, OPENS[0], CLOSES[0], TS_START, early_ts_init)])

    assert early.decision_ts == early_ts_init
    assert early.fill_ts == early_ts_init


def test_a_deferred_fill_reports_the_gapped_bar_close_rather_than_the_prior_close() -> None:
    """
    Test the opening-gap case: the released intent is priced by the bar that released it.

    The second bar opens far above the first bar's close. The deferred fill is priced at the second
    bar's own close, so it is the gap's price and not the prior close, which is what the declared
    expectation requires of a gapped session.
    """
    gapped = bars(closes=(CLOSES[0], "110.25", CLOSES[2], CLOSES[3]))
    lifecycle = run(bars_data=gapped, latency_ns=SECOND_NS)

    assert lifecycle.fill_ts == TS_START + MINUTE_NS
    assert lifecycle.fill_px == "110.25"
    assert lifecycle.fill_px != CLOSES[0]


# The decision bridge's own eligibility resolution, observed against the same engine. The bridge is
# the second deferral mechanism the design names, and the observations below are the same ones: the
# recorded fill timestamp against the instant the artifact declared itself actionable at.


# The artifact is delivered after the XNYS regular session closes at 21:00 UTC, so the bridge's
# resolved eligibility is the next session's open rather than the delivery instant.
BRIDGE_QUOTE_START = fixtures.unix_ns(2025, 6, 2, 21, 20)
BRIDGE_ARTIFACT_TS = fixtures.unix_ns(2025, 6, 2, 21, 30)
BRIDGE_PRODUCED_AT = "2025-06-02T21:30:00+00:00"
BRIDGE_QUOTE_COUNT = 40
BRIDGE_BID = "99.99"
BRIDGE_ASK = "100.01"
BRIDGE_PIPELINE = TargetPipelineConfig(
    risk_per_trade=Decimal("0.0001"),
    stop_loss_bps=100,
    max_weight=Decimal(1),
    commission_rate=Decimal(0),
    min_order_quantity=Quantity.from_int(0),
)
BRIDGE_PROJECTION = ProjectionConfig(
    horizon_ns=86_400_000_000_000,
    authorized_producers=fixtures.AUTHORIZED_PRODUCERS,
)

register_custom_data_class(ResearchDecisionCarrier)
BRIDGE_DATA_TYPE = DataType(
    "ResearchDecisionCarrier",
    {"source": "tradingagents"},
    str(INSTRUMENT_ID),
)


@dataclass(frozen=True)
class BridgeLifecycle:
    """
    The recorded lifecycle of one projected artifact.
    """

    received_at: int | None
    actionable_at: int | None
    signal_ts: int | None
    submission_ts: int | None
    fill_ts: int | None
    fill_px: str | None
    submission_outcome: str | None


class BridgeTimingConfig(StrategyConfig):
    """
    Configure the bridge-backed strategy.
    """

    def __init__(
        self,
        *,
        instrument_id: Any,
        ledger_directory: Path,
        calendar: TradingCalendar,
        data_type: DataType,
    ) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.instrument_id = instrument_id
        self.ledger_directory = ledger_directory
        self.calendar = calendar
        self.data_type = data_type


class BridgeTimingStrategy(Strategy):
    """
    Project a delivered artifact, submit it once, and record every instant involved.
    """

    def __init__(self, config: BridgeTimingConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self.ledger = AdmissionLedger(config.ledger_directory)
        self.received_at: int | None = None
        self.actionable_at: int | None = None
        self.signal_ts: int | None = None
        self.submission_ts: int | None = None
        self.fill_ts: int | None = None
        self.fill_px: str | None = None
        self.submission_outcome: str | None = None

    def on_start(self) -> None:
        """
        Subscribe to the carried artifact and to the quote stream the venue's book is built from.
        """
        self.subscribe_data(self.config.data_type)
        self.subscribe_quotes(self.config.instrument_id)

    def on_data(self, data: CustomData) -> None:
        """
        Project the artifact and submit its signal once, at the instant it was delivered.
        """
        raw = json.loads(data.data.document)
        now = self.clock.timestamp_ns()
        self.received_at = now
        projection = project(
            raw,
            context=ProjectionContext(
                received_at=now,
                tradability=Tradability.TRADABLE,
                position=position_state(self.portfolio.net_position(self.config.instrument_id)),
                engine_eligible=True,
                calendar=self.config.calendar,
            ),
            config=BRIDGE_PROJECTION,
            resolve_instrument=fixtures.resolve_instrument,
        )
        self.actionable_at = projection.actionable_at
        if projection.signal is not None:
            self.signal_ts = projection.signal.ts_event
        record_decision(
            ledger=self.ledger,
            projection=projection,
            received_at=now,
            admission_timestamp=now,
        )
        result = submit_once(
            ledger=self.ledger,
            projection=projection,
            strategy=self,
            base_pipeline=BRIDGE_PIPELINE,
            ceiling=projection.ceiling,
        )
        self.submission_outcome = result.outcome.value
        self.submission_ts = self.clock.timestamp_ns()

    def on_order_filled(self, event: Any) -> None:
        """
        Record the fill, its timestamp and its price.
        """
        self.fill_ts = event.ts_event
        self.fill_px = str(event.last_px)


def bridge_quotes() -> list[QuoteTick]:
    """
    Build one-minute quotes spanning the artifact's arrival.
    """
    result: list[QuoteTick] = []
    for index in range(BRIDGE_QUOTE_COUNT):
        ts = BRIDGE_QUOTE_START + index * MINUTE_NS
        result.append(
            QuoteTick(
                INSTRUMENT_ID,
                Price.from_str(BRIDGE_BID),
                Price.from_str(BRIDGE_ASK),
                Quantity.from_int(10_000),
                Quantity.from_int(10_000),
                ts,
                ts,
            ),
        )
    return result


def bridge_carrier(raw: dict[str, Any], ts_ns: int) -> CustomData:
    """
    Wrap a raw artifact as the custom data the platform carries unchanged.
    """
    revision_id = revision_id_for(raw["idempotency_key"], raw["artifact_sha256"])
    decision_id = decision_id_for(
        INSTRUMENT_ID,
        date.fromisoformat(raw["effective_date"]),
        revision_id,
    )
    return CustomData(
        BRIDGE_DATA_TYPE,
        ResearchDecisionCarrier(
            ts_event=ts_ns,
            ts_init=ts_ns,
            decision_id=decision_id,
            artifact_sha256=raw["artifact_sha256"],
            idempotency_key=raw["idempotency_key"],
            produced_at=raw["produced_at"],
            effective_date=raw["effective_date"],
            document=json.dumps(raw, sort_keys=True),
        ),
    )


def run_bridge(tmp_path: Path, *, latency_ns: int | None = None) -> BridgeLifecycle:
    """
    Run one bridge-backed scenario and return its recorded lifecycle.
    """
    calendar = fixtures.bundled_calendar()
    if calendar is None:
        pytest.skip("the bundled XNYS.EQUITY calendar is not available in this build")

    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=VENUE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money(1_000_000.0, USD)],
        base_currency=USD,
        latency_model=None
        if latency_ns is None
        else StaticLatencyModel(base_latency_nanos=latency_ns),
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    engine.add_instrument(INSTRUMENT)
    engine.add_data(bridge_quotes())
    strategy = BridgeTimingStrategy(
        BridgeTimingConfig(
            instrument_id=INSTRUMENT_ID,
            ledger_directory=tmp_path / "bridge",
            calendar=calendar,
            data_type=BRIDGE_DATA_TYPE,
        ),
    )
    engine.add_strategy(strategy)
    engine.add_data(
        [
            bridge_carrier(
                fixtures.covered_artifact(produced_at=BRIDGE_PRODUCED_AT),
                BRIDGE_ARTIFACT_TS,
            ),
        ],
        sort=True,
    )

    try:
        engine.run(
            start=BRIDGE_QUOTE_START,
            end=BRIDGE_QUOTE_START + BRIDGE_QUOTE_COUNT * MINUTE_NS,
        )
        return BridgeLifecycle(
            received_at=strategy.received_at,
            actionable_at=strategy.actionable_at,
            signal_ts=strategy.signal_ts,
            submission_ts=strategy.submission_ts,
            fill_ts=strategy.fill_ts,
            fill_px=strategy.fill_px,
            submission_outcome=strategy.submission_outcome,
        )
    finally:
        engine.dispose()


def test_the_bridge_resolves_the_eligibility_to_the_next_session_open(tmp_path: Path) -> None:
    """
    Test that the bridge names an eligibility later than the artifact's arrival.

    The artifact is produced after a session close, so the resolution reads the calendar and returns
    the next session's open, and the projected signal carries that instant as its own event time.
    This is the resolution the design's deferral claim rests on, and it is derived from the
    artifact's production instant rather than from the instant the engine received it.
    """
    lifecycle = run_bridge(tmp_path)

    assert lifecycle.received_at == BRIDGE_ARTIFACT_TS
    assert lifecycle.actionable_at is not None
    assert lifecycle.actionable_at > lifecycle.received_at
    assert lifecycle.signal_ts == lifecycle.actionable_at
    assert lifecycle.submission_outcome == Outcome.SIGNAL.value


def test_the_bridge_submits_at_arrival_so_the_fill_precedes_the_declared_eligibility(
    tmp_path: Path,
) -> None:
    """
    Test that the resolved eligibility does not defer the fill: the submission is not scheduled.

    `submit_once` holds no clock and no timer, so the order set is submitted at the delivery
    instant, and with no latency model the fill is stamped there. The fill therefore precedes the
    artifact's own declared eligibility, which is why the compiler emits the latency deferral of the
    accepted mechanism and never this pattern alone: the bridge resolves availability, and its
    caller owns any waiting.
    """
    lifecycle = run_bridge(tmp_path)

    assert lifecycle.submission_ts == BRIDGE_ARTIFACT_TS
    assert lifecycle.fill_ts == BRIDGE_ARTIFACT_TS
    assert lifecycle.actionable_at is not None
    assert lifecycle.fill_ts < lifecycle.actionable_at
    assert lifecycle.fill_px in (BRIDGE_BID, BRIDGE_ASK)


def test_a_latency_model_moves_the_fill_to_the_next_observation_not_the_eligibility(
    tmp_path: Path,
) -> None:
    """
    Test that the deferral follows the data rather than the declaration.

    With a latency model the fill moves forward, but to the next quote the venue saw, so the
    deferred instant is set by the data snapshot rather than by the artifact's resolved eligibility.
    """
    lifecycle = run_bridge(tmp_path, latency_ns=SECOND_NS)

    assert lifecycle.fill_ts == BRIDGE_ARTIFACT_TS + MINUTE_NS
    assert lifecycle.actionable_at is not None
    assert lifecycle.fill_ts != lifecycle.actionable_at
    assert lifecycle.fill_px in (BRIDGE_BID, BRIDGE_ASK)
