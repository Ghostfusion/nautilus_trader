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
FX-M3-4: feature values have parity between research and decision time.

Design criterion 20 requires one fixture that computes a feature offline through the research path
and evaluates the same feature at decision time through the interpreter, with the two agreeing on
value, observation and availability timestamps, warm-up behaviour, missing-value treatment, units
and universe membership. `python/tests/integration/test_research_api.py` establishes the parity
pattern this fixture follows: the same indicator over the same bars, once through the research
path and once inside a running backtest.

The research side is `compute_observations`, which drives the repository's indicator outside the
engine. The decision-time side is a `FeatureStream` driven by a live strategy inside a real
`BacktestEngine` over the same bars, which is the mechanism the compiled interpreter uses.

Four things this fixture establishes beyond agreement on the numbers:

- the research path reports `0.0` for a bar before the lookback is satisfied, because the
  repository's indicators initialise their reading to zero, and the decision-time contract serves
  nothing there, so a warm-up reading can never be mistaken for a value;
- a value whose availability lags its observation is not served at the bar it describes, which is
  what stops a publication or restatement lag from becoming a look-ahead;
- membership is read at the decision instant rather than at the time of analysis, so an instrument
  that left the universe is not selectable afterwards even though its history holds a value;
- a value that is absent is absent rather than zero, so an ineligible instrument cannot be selected
  on the strength of a fabricated reading.
"""

from __future__ import annotations

from decimal import Decimal

import pytest

from nautilus_trader.analysis.research import compute_indicator
from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.discovery import FeatureDeclaration
from nautilus_trader.discovery.features import FeatureDefinition
from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.features import FeatureSnapshot
from nautilus_trader.discovery.features import FeatureStream
from nautilus_trader.discovery.features import build_memberships
from nautilus_trader.discovery.features import build_panel
from nautilus_trader.discovery.features import compute_observations
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.indicators import ExponentialMovingAverage
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import Venue
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestInstrumentProvider


SECOND_NS = 1_000_000_000
DAY_NS = 86_400 * SECOND_NS
TS_START = 1_704_067_200_000_000_000

USD = Currency.from_str("USD")
ALIAS = "momentum"
FEATURE_ID = "close_ema_5"
LOOKBACK = 5
LEAVE_AT_BAR = 8

CLOSES = (
    "100.00",
    "101.00",
    "102.00",
    "103.00",
    "104.00",
    "105.00",
    "104.00",
    "103.00",
    "102.00",
    "101.00",
    "100.00",
    "101.00",
)

INSTRUMENT = TestInstrumentProvider.aapl_equity()
INSTRUMENT_ID = INSTRUMENT.id
VENUE: Venue = INSTRUMENT.venue
BAR_TYPE = BarType.from_str(f"{INSTRUMENT_ID}-1-DAY-LAST-EXTERNAL")


def _bar(bar_type: BarType, close: str, ts: int) -> Bar:
    price = Price.from_str(close)

    return Bar(
        bar_type=bar_type,
        open=price,
        high=price,
        low=price,
        close=price,
        volume=Quantity.from_int(1_000),
        ts_event=ts,
        ts_init=ts,
    )


def bars(closes: tuple[str, ...] = CLOSES) -> list[Bar]:
    """
    Build one daily bar per close, starting at the fixture's first timestamp.
    """
    return [_bar(BAR_TYPE, close, TS_START + index * DAY_NS) for index, close in enumerate(closes)]


def definition(*, lookback: int = LOOKBACK, availability_lag_ns: int = 0) -> FeatureDefinition:
    """
    Build the ema definition the fixture declares and resolves.
    """
    return FeatureDefinition(
        feature_id=FEATURE_ID,
        operator="ema",
        input_field="close",
        lookback=lookback,
        availability_lag_ns=availability_lag_ns,
        units="price",
    )


def declarations(*, skip: int = 0, lookback: int = LOOKBACK) -> tuple[FeatureDeclaration, ...]:
    """
    Build the single declaration the fixture's streams hold.
    """
    return (FeatureDeclaration(alias=ALIAS, feature_id=FEATURE_ID, lookback=lookback, skip=skip),)


class ParityConfig(StrategyConfig):
    """
    Configure the strategy that drives one stream inside the engine.
    """

    def __init__(
        self,
        *,
        bar_type: str,
        declarations: tuple[FeatureDeclaration, ...],
        registry: FeatureRegistry,
        memberships: object,
    ) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.bar_type = bar_type
        self.declarations = declarations
        self.registry = registry
        self.memberships = memberships


class ParityStrategy(Strategy):
    """
    Drive one `FeatureStream` on every bar and record the snapshot each bar produced.
    """

    def __init__(self, config: ParityConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self.stream = FeatureStream(
            config.declarations,
            config.registry,
            config.memberships,
            INSTRUMENT_ID,
        )
        self.snapshots: list[tuple[int, FeatureSnapshot]] = []

    def on_start(self) -> None:
        """
        Subscribe to the instrument's bars.
        """
        self.subscribe_bars(self._bar_type)

    def on_bar(self, bar: Bar) -> None:
        """
        Update the stream and record the snapshot the bar produced.
        """
        self.stream.update(bar)
        self.snapshots.append((bar.ts_event, self.stream.snapshot(bar.ts_event)))


def run_in_engine(
    *,
    bar_data: list[Bar] | None = None,
    registry: FeatureRegistry,
    feature_declarations: tuple[FeatureDeclaration, ...],
    memberships: object,
) -> ParityStrategy:
    """
    Run the strategy over the fixture's bars and return it.
    """
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=VENUE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money(1_000_000.0, USD)],
        base_currency=USD,
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    engine.add_instrument(INSTRUMENT)
    engine.add_data(bars() if bar_data is None else bar_data)
    strategy = ParityStrategy(
        ParityConfig(
            bar_type=str(BAR_TYPE),
            declarations=feature_declarations,
            registry=registry,
            memberships=memberships,
        ),
    )
    engine.add_strategy(strategy)

    try:
        engine.run()
    finally:
        engine.dispose()

    return strategy


def served_values(strategy: ParityStrategy) -> list[tuple[int, float]]:
    """
    Return the values the strategy was served, as `(decision instant, value)` pairs.
    """
    served: list[tuple[int, float]] = []
    for decision_ts, snapshot in strategy.snapshots:
        value = snapshot.get(ALIAS)
        if value is not None:
            served.append((decision_ts, value.value))

    return served


def test_a_decision_time_value_agrees_with_the_research_path() -> None:
    """
    Test the parity criterion: value, both timestamps, units and identity all agree.

    The research side computes the feature outside the engine; the decision-time side reads it from
    a stream driven by a live strategy over the same bars. Agreement here is what makes a research
    finding and a strategy result the same statement.
    """
    feature_definition = definition()
    registry = FeatureRegistry([feature_definition])
    memberships = build_memberships(
        "TEST",
        "fixture",
        [(str(INSTRUMENT_ID), TS_START, None)],
    )
    strategy = run_in_engine(
        registry=registry,
        feature_declarations=declarations(),
        memberships=memberships,
    )
    offline = compute_observations(feature_definition, bars())

    read = [
        snapshot.get(ALIAS) for _, snapshot in strategy.snapshots if snapshot.get(ALIAS) is not None
    ]

    assert len(read) == len(offline)
    assert [value.value for value in read] == [observation.value for observation in offline]
    assert [value.observed_at for value in read] == [
        observation.observed_at for observation in offline
    ]
    assert [value.available_at for value in read] == [
        observation.available_at for observation in offline
    ]
    assert {value.units for value in read} == {"price"}
    assert {value.definition_digest for value in read} == {feature_definition.digest()}
    assert {value.member for value in read} == {True}
    assert {value.feature_id for value in read} == {FEATURE_ID}


def test_a_warmup_bar_serves_nothing_where_the_research_path_reports_zero() -> None:
    """
    Test the warm-up divergence the contract exists to remove.

    A moving average reports `0.0` until its period is satisfied, which is not a value. The research
    path passes that reading through, and the decision-time contract serves nothing there, so a
    strategy cannot act on a price of zero.
    """
    feature_definition = definition()
    registry = FeatureRegistry([feature_definition])
    memberships = build_memberships(
        "TEST",
        "fixture",
        [(str(INSTRUMENT_ID), TS_START, None)],
    )
    strategy = run_in_engine(
        registry=registry,
        feature_declarations=declarations(),
        memberships=memberships,
    )

    raw = compute_indicator(ExponentialMovingAverage(LOOKBACK), bars())
    warmup_bars = [snapshot for _, snapshot in strategy.snapshots[: LOOKBACK - 1]]

    assert len(warmup_bars) == LOOKBACK - 1
    assert all(snapshot.get(ALIAS) is None for snapshot in warmup_bars)
    assert all(ALIAS not in snapshot for snapshot in warmup_bars)
    # The repository's indicator reports zero for those bars, which is the divergence itself.
    assert raw[: LOOKBACK - 1] == [0.0] * (LOOKBACK - 1)
    assert raw[LOOKBACK - 1] == compute_observations(feature_definition, bars())[0].value


def test_an_availability_lag_defers_a_value_beyond_the_bar_it_describes() -> None:
    """
    Test availability as a distinct instant from observation.

    A value observed at one bar and available a day later is not served at its own bar. The served
    value carries the observation instant it was produced at, so a released intent can be priced
    from the instant it became knowable rather than from the instant it was computed.
    """
    feature_definition = definition(availability_lag_ns=DAY_NS)
    registry = FeatureRegistry([feature_definition])
    memberships = build_memberships(
        "TEST",
        "fixture",
        [(str(INSTRUMENT_ID), TS_START, None)],
    )
    strategy = run_in_engine(
        registry=registry,
        feature_declarations=declarations(),
        memberships=memberships,
    )
    offline = compute_observations(feature_definition, bars())
    read = [(decision_ts, snapshot.get(ALIAS)) for decision_ts, snapshot in strategy.snapshots]

    first_available = next(item for item in read if item[1] is not None)
    assert first_available[0] == offline[0].observed_at + DAY_NS
    assert first_available[1] is not None
    assert first_available[1].observed_at == offline[0].observed_at
    assert first_available[1].available_at == offline[0].available_at
    # The bar that produced the value served nothing, because the value was not yet available.
    at_observation = next(item[1] for item in read if item[0] == offline[0].observed_at)
    assert at_observation is None


def test_membership_is_read_at_the_decision_instant() -> None:
    """
    Test membership as the state at the decision instant rather than at analysis time.

    The instrument is a member for the first bars and leaves afterwards. Its value keeps arriving,
    because the feature is a property of the price series rather than of the universe, and every
    snapshot after the exit says the instrument is not a member, so it cannot be selected.
    """
    feature_definition = definition()
    registry = FeatureRegistry([feature_definition])
    leave_at = TS_START + LEAVE_AT_BAR * DAY_NS
    memberships = build_memberships("TEST", "fixture", [(str(INSTRUMENT_ID), TS_START, leave_at)])
    strategy = run_in_engine(
        registry=registry,
        feature_declarations=declarations(),
        memberships=memberships,
    )

    after_exit = [
        snapshot.get(ALIAS)
        for decision_ts, snapshot in strategy.snapshots
        if decision_ts >= leave_at
    ]

    assert after_exit
    assert all(value is not None for value in after_exit)
    assert all(value.member is False for value in after_exit)
    before_exit = [
        snapshot.get(ALIAS)
        for decision_ts, snapshot in strategy.snapshots
        if decision_ts < leave_at
    ]
    assert all(value.member is True for value in before_exit if value is not None)


def test_the_offline_panel_carries_the_same_observations_as_the_stream() -> None:
    """
    Test the precomputed panel against the decision-time stream.

    The panel is the offline artifact a candidate's features are read from, and the stream is what a
    running strategy reads. One row exists exactly where a value was served, at the same observation
    instant and with the same value.
    """
    feature_definition = definition()
    registry = FeatureRegistry([feature_definition])
    bar_data = bars()
    memberships = build_memberships(
        "TEST",
        "fixture",
        [(str(INSTRUMENT_ID), TS_START, None)],
    )
    strategy = run_in_engine(
        bar_data=bar_data,
        registry=registry,
        feature_declarations=declarations(),
        memberships=memberships,
    )
    panel = build_panel(declarations(), registry, {str(INSTRUMENT_ID): bar_data}, memberships)

    rows = panel.rows()
    served = [
        (snapshot.get(ALIAS).observed_at, snapshot.get(ALIAS).value)
        for _, snapshot in strategy.snapshots
        if snapshot.get(ALIAS) is not None
    ]

    assert panel.len() == len(rows) == len(served)
    assert [(row.ts_event, row.features[ALIAS].value) for row in rows] == served
    assert [row.features[ALIAS].as_of for row in rows] == [row.ts_event for row in rows]
    # No row is written for a bar that produced no reading, so the panel holds no warm-up value.
    assert rows[0].ts_event == bar_data[LOOKBACK - 1].ts_event
    assert panel.rows()[0].member is True


def test_a_feature_shorter_than_its_lookback_is_absent_rather_than_zero() -> None:
    """
    Test missing-value treatment: an instrument that cannot supply the lookback has no value.

    The stream serves nothing for the alias and the panel writes no row, so nothing downstream can
    read a zero where a value should have been and select an instrument on the strength of it.
    """
    feature_definition = definition()
    registry = FeatureRegistry([feature_definition])
    short_bars = bars(CLOSES[: LOOKBACK - 1])
    memberships = build_memberships(
        "TEST",
        "fixture",
        [(str(INSTRUMENT_ID), TS_START, None)],
    )
    stream = FeatureStream(declarations(), registry, memberships, INSTRUMENT_ID)

    for bar in short_bars:
        stream.update(bar)

    snapshot = stream.snapshot(short_bars[-1].ts_event)
    panel = build_panel(declarations(), registry, {str(INSTRUMENT_ID): short_bars}, memberships)

    assert snapshot.values == ()
    assert ALIAS not in snapshot
    assert snapshot.get(ALIAS) is None
    assert panel.is_empty()
    assert compute_observations(feature_definition, short_bars) == []


def test_a_declared_skip_serves_the_reading_the_declaration_asked_for() -> None:
    """
    Test the skip rule: a decision made on an older reading carries that reading's own instant.
    """
    feature_definition = definition()
    registry = FeatureRegistry([feature_definition])
    bar_data = bars()
    memberships = build_memberships(
        "TEST",
        "fixture",
        [(str(INSTRUMENT_ID), TS_START, None)],
    )
    skip = 2
    strategy = run_in_engine(
        bar_data=bar_data,
        registry=registry,
        feature_declarations=declarations(skip=skip),
        memberships=memberships,
    )
    offline = compute_observations(feature_definition, bar_data)
    read = [
        (decision_ts, snapshot.get(ALIAS))
        for decision_ts, snapshot in strategy.snapshots
        if snapshot.get(ALIAS) is not None
    ]
    first_decision, first_value = read[0]

    assert first_decision == bar_data[LOOKBACK - 1 + skip].ts_event
    assert first_value is not None
    assert first_value.observed_at == bar_data[LOOKBACK - 1].ts_event
    assert first_value.value == offline[0].value
    # Every served reading is two bars behind its decision instant.
    assert all(value.observed_at <= decision_ts - skip * DAY_NS for decision_ts, value in read)


def test_a_definition_this_build_cannot_drive_is_refused() -> None:
    """
    Test that a feature cannot be silently absent from a run.

    A definition naming an operator the build cannot drive is refused when it is constructed, and a
    declaration naming a feature the registry does not hold is refused when the stream is built.
    """
    with pytest.raises(ValueError, match="operator must be one of"):
        FeatureDefinition(
            feature_id="close_hull_9",
            operator="hull",
            input_field="close",
            lookback=9,
        )

    registry = FeatureRegistry([definition()])
    memberships = build_memberships("TEST", "fixture", [(str(INSTRUMENT_ID), TS_START, None)])

    with pytest.raises(ValueError, match="is not registered"):
        FeatureStream(
            (FeatureDeclaration(alias="other", feature_id="close_sma_50", lookback=50, skip=0),),
            registry,
            memberships,
            INSTRUMENT_ID,
        )
