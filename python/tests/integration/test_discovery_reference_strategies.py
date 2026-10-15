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
FX-M3-3: the interpreter matches independently specified reference strategies.

Design criterion 19 requires that, for each reference strategy of the design's Section 9.3 - a
ranking strategy, a threshold strategy, a holding-period exit, a rebalance schedule, a target
allocation, a strategy with no eligible instruments, and one encountering an opening gap - the
interpreted strategy's signals, targets, timing and portfolio outcomes match a hand-written
fixture. A test showing that a candidate compiled and completed is not evidence about the
interpreter, and neither is the compiler's own error vocabulary, so every expectation below is
written down from the declared strategy rather than read back from the run, and each is asserted
against a record the run produced: `strategy.decisions` (its `rebalance`, `cross_section`,
`selection`, `targets`, `signals` and `client_order_ids`), `strategy.fills`, and the engine's own
`cache.orders()`, `cache.positions_open()` and `cache.positions_closed()`.

The market-data fixture. Four NASDAQ equities, one daily bar per instrument per period, all four
sharing the period's event timestamp, and a quote one second before each bar so the target pipeline
resolves an entry price from the cache. The registered feature is the repository's moving average
of the close with a lookback of three, and each instrument's closes are constant, so the feature
value equals that constant at every servable period and the ranking is hand-computable and
independent of exactly where the warm-up ends. The first servable decision - the first decision
whose cross-section carries any value - arrives at period 3, one period per lookback observation,
and the ranking scenario asserts that against the declared lookback.

The declared portfolio ceiling. The reference strategies of the design declare a full-unit ceiling,
which the first-release policy does not admit: `portfolio.max_position_weight` is bounded to `0.005`
through `0.25`, and a payload declaring `1.0` is refused with `value_out_of_bound`. The fixture
therefore declares the ceiling the policy admits, `0.25`, and hand-computes every weight after the
position cap has clamped it. That is where the reference strategies' stated pre-cap weights stop
being the weights the allocation emits, and the scenarios state both.

Two interpreter conventions this fixture observes rather than asserts, because the design fixes the
holding-period unit and says nothing about either:

- A period contains one decision per instrument, so a decision taken on the first bar of a period
  reads that instrument's value at the current period and the others' at the previous one. Constant
  closes keep the ranking and the selection invisible to that asynchrony; the fixture asserts the
  first servable decision and its one-instrument cross-section, and nothing further about it.
- With `rebalance.frequency = daily` every decision bar rebalances, so a holding-period exit at a
  period can be followed by a fresh entry for the same name later in that same period. The
  holding-period scenario asserts the exit periods the declared rule implies - a flatten two
  decision periods after the entry that opened the holding, and never at the period in between - and
  not the intra-period re-entry itself, which is an implementation choice rather than a declared
  rule.

A split is deliberately not used for the seventh scenario. A split changes a position's price
without changing the position, so a stop measured against the position's average entry price would
fire on it, and that hazard belongs with the corporate-action accounting of criterion 6, which this
repository does not have yet. The reference strategy that exercises the corporate-action hazard
therefore waits for that accounting; the opening gap, which the declared execution model can already
state, is what this fixture exercises instead.

Tolerances, each with the quantity it bounds and why it is that tolerance:

- A target weight against a hand-computed share: absolute 1e-12. The allocation computes in
  `Decimal` from decimal text (the declared cap and the declared shares), and the only lossy step is
  the conversion to the `float` a `Target` value carries, where a double holds a share to within
  about 1e-16 relative.
- A signal strength against `min(weight / cap, 1.0)`: absolute 1e-12. The strength is a Python float
  computed from the target's float weight and the declared cap, so it carries that same single
  conversion and no other loss.
- A fill price against the venue's declared close, and against a stop threshold: absolute 1e-9.
  Prices are two-decimal text at the instrument's declared precision and both sides of the
  comparison are `Decimal`; the tolerance covers only a representation difference. The inequality
  that carries the finding, a gap fill strictly below the stop, is asserted strictly and is 20.00
  wide, far outside any tolerance.
- Selections, instrument sets, exit tuples, rebalance flags, order sides and identifiers, counts and
  position quantities: exact. These are discrete values the run either produced or did not, so there
  is no bounded quantity to compare and a tolerance would hide a difference rather than bound it.
"""

from __future__ import annotations

from collections.abc import Iterator
from contextlib import contextmanager
from decimal import Decimal
from typing import Any

import pytest

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.discovery.compile import CompiledPlan
from nautilus_trader.discovery.compile import compile_specification
from nautilus_trader.discovery.features import FeatureDefinition
from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.features import build_memberships
from nautilus_trader.discovery.policy import DiscoveryPolicy
from nautilus_trader.discovery.policy import Reference
from nautilus_trader.discovery.policy import ReferenceKind
from nautilus_trader.discovery.schema import SPECIFICATION_SCHEMA_VERSION
from nautilus_trader.discovery.strategy import BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY
from nautilus_trader.discovery.strategy import DecisionRecord
from nautilus_trader.discovery.strategy import DiscoveryInterpreter
from nautilus_trader.discovery.strategy import DiscoveryInterpreterConfig
from nautilus_trader.discovery.strategy import InterpreterRunInputs
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Equity
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Symbol
from nautilus_trader.model import Venue


# The fixture's universe, named so a scenario reads as the strategy it describes.
AAPL = "AAPL.NASDAQ"
MSFT = "MSFT.NASDAQ"
NVDA = "NVDA.NASDAQ"
AMZN = "AMZN.NASDAQ"
INSTRUMENTS = (AAPL, MSFT, NVDA, AMZN)

# The declared feature: the repository's simple moving average of the close, and the alias the
# specifications refer to it by.
TREND_ID = "sma_close_3"
TREND_ALIAS = "trend"
LOOKBACK = 3

# The portfolio ceiling the first-release policy admits. A full-unit ceiling, which the reference
# strategies of the design declare, is outside the policy's bound and is refused at compilation.
CAP = 0.25
CAP_TEXT = "0.25"

# The tolerances of the table in the module docstring, named here so a comparison states which one
# it uses.
WEIGHT_TOLERANCE = 1e-12
STRENGTH_TOLERANCE = 1e-12
PRICE_TOLERANCE = Decimal("1e-9")

USD = Currency.from_str("USD")
VENUE = Venue("NASDAQ")
BAR_SPECIFICATION = BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY["daily"]

SECOND_NS = 1_000_000_000
DAY_NS = 86_400 * SECOND_NS

# Twelve consecutive daily periods starting Monday 2022-01-03T00:00:00Z, which is the Monday of ISO
# week 2022-W01; period 8 is Monday 2022-01-10, the first day of ISO week 2022-W02.
PERIODS = 12
TS_START = 1_641_168_000_000_000_000
WEEK_PERIODS = (1, 8)

STARTING_BALANCE = 1_000_000.0
VOLUME = 1_000_000
QUOTE_SIZE = 1_000
STOP_LOSS_BPS = 100
BENCHMARK_DIGEST = f"sha256:{'0' * 64}"

# The gap scenario's data: AAPL is flat at 100.00 for four periods, then gaps to 70.00, which is the
# first close at or below the 10 per cent stop measured from the 100.00 entry the platform recorded.
GAP_PERIOD = 5
GAP_CLOSES: dict[str, tuple[str, ...]] = {
    AAPL: ("100.00",) * 4 + ("70.00", "60.00", "60.00", "60.00"),
    MSFT: ("95.00",) * 8,
    NVDA: ("90.00",) * 8,
    AMZN: ("85.00",) * 8,
}

# The four-instrument constant closes of the ranking, threshold and rebalance scenarios.
CONSTANT_CLOSES: dict[str, tuple[str, ...]] = {
    AAPL: ("100.00",) * PERIODS,
    MSFT: ("95.00",) * PERIODS,
    NVDA: ("90.00",) * PERIODS,
    AMZN: ("85.00",) * PERIODS,
}

# The three-instrument constant closes of the target-allocation scenario.
THREE_CLOSES: dict[str, tuple[str, ...]] = {
    AAPL: ("100.00",) * 8,
    MSFT: ("95.00",) * 8,
    NVDA: ("90.00",) * 8,
}


def equity(instrument_id: str) -> Equity:
    """
    Return one fixture equity on the fixture venue, at the declared precision.
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


def bar(instrument_id: str, close: str, ts_event: int) -> Bar:
    """
    Return one daily bar with a one-unit spread around the given close.
    """
    close_price = Price.from_str(close)

    return Bar(
        BarType.from_str(f"{instrument_id}-{BAR_SPECIFICATION}"),
        close_price,
        Price.from_str(f"{close_price.as_double() + 1.0:.2f}"),
        Price.from_str(f"{close_price.as_double() - 1.0:.2f}"),
        close_price,
        Quantity.from_int(VOLUME),
        ts_event,
        ts_event,
    )


def quote(instrument_id: str, close: str, ts_event: int) -> QuoteTick:
    """
    Return one quote at the given close, so the pipeline can resolve an entry price.
    """
    price = Price.from_str(close)

    return QuoteTick(
        InstrumentId.from_str(instrument_id),
        price,
        price,
        Quantity.from_int(QUOTE_SIZE),
        Quantity.from_int(QUOTE_SIZE),
        ts_event,
        ts_event,
    )


def bars(closes: dict[str, tuple[str, ...]]) -> list[Bar]:
    """
    Return the fixture's bars, instrument by instrument and period by period.

    The bars of one period share the period's event timestamp, so a decision period is one calendar
    day however many instruments the cross-section holds.
    """
    result: list[Bar] = []
    for index in range(max(len(series) for series in closes.values())):
        for instrument_id, series in closes.items():
            ts_event = TS_START + index * DAY_NS
            result.append(bar(instrument_id, series[min(index, len(series) - 1)], ts_event))

    return result


def quotes(closes: dict[str, tuple[str, ...]]) -> list[QuoteTick]:
    """
    Return one quote per fixture bar, stamped one second before the bar it belongs to.

    The stamp is a second early because a quote at the decision instant can be processed after the
    order that instant submitted, and the pipeline resolves an entry price from the cache.
    """
    result: list[QuoteTick] = []
    for index in range(max(len(series) for series in closes.values())):
        for instrument_id, series in closes.items():
            ts_event = TS_START + index * DAY_NS - SECOND_NS
            result.append(quote(instrument_id, series[min(index, len(series) - 1)], ts_event))

    return result


def policy() -> DiscoveryPolicy:
    """
    Return the fixture campaign's pre-registered policy, with its frozen references and sector map.
    """
    return DiscoveryPolicy.first_release(
        feature_allowlist=(TREND_ID,),
        references=(
            Reference("fixture-benchmark", ReferenceKind.BENCHMARK, BENCHMARK_DIGEST),
            Reference("fixture-cost-model", ReferenceKind.COST_MODEL, BENCHMARK_DIGEST),
            Reference("fixture-validation", ReferenceKind.VALIDATION_POLICY, BENCHMARK_DIGEST),
        ),
        sectors=tuple((instrument_id, "mega") for instrument_id in INSTRUMENTS),
    )


def definition() -> FeatureDefinition:
    """
    Return the fixture's registered trend definition.
    """
    return FeatureDefinition(
        feature_id=TREND_ID,
        operator="sma",
        input_field="close",
        lookback=LOOKBACK,
    )


def selection_block(**overrides: Any) -> dict[str, Any]:
    """
    Return the ranking selection block, with the given fields overridden.
    """
    block: dict[str, Any] = {
        "mode": "rank",
        "feature": TREND_ALIAS,
        "count": "2",
        "tie_break": "feature_desc",
    }

    return {**block, **overrides}


def threshold_selection(comparison: str, threshold: str) -> dict[str, Any]:
    """
    Return the threshold selection block: one comparison and one threshold, and no tie rule.
    """
    return {
        "mode": "threshold",
        "feature": TREND_ALIAS,
        "comparison": comparison,
        "threshold": threshold,
    }


def exit_block(**overrides: Any) -> dict[str, Any]:
    """
    Return the exit block, with the given fields declared over the flatten rule.
    """
    return {"flatten_ineligible": True, **overrides}


def portfolio_block(**overrides: Any) -> dict[str, Any]:
    """
    Return the portfolio block, with the given fields overridden.
    """
    block: dict[str, Any] = {
        "position_count": 2,
        "weighting": "rank_proportional",
        "max_position_weight": CAP_TEXT,
        "cash_policy": "leave_idle",
    }

    return {**block, **overrides}


def rebalance_block(**overrides: Any) -> dict[str, Any]:
    """
    Return the rebalance block, with the given fields overridden.
    """
    return {"frequency": "daily", "timing": "next_session_open", **overrides}


def specification(
    *,
    family: str = "cross_sectional_rank",
    selection: dict[str, Any] | None = None,
    exits: dict[str, Any] | None = None,
    portfolio: dict[str, Any] | None = None,
    rebalance: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """
    Return one reference strategy's hand-written specification, as the compiler's own payload.

    The defaults are the ranking strategy: rank the cross-section by the moving average and hold the
    two leaders at the declared ceiling, with the residual left idle.
    """
    return {
        "version": SPECIFICATION_SCHEMA_VERSION,
        "identity": {
            "family": family,
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
                TREND_ALIAS: {"feature": TREND_ID, "lookback": LOOKBACK, "skip": 0},
            },
        },
        "selection": selection if selection is not None else selection_block(),
        "entry": {"direction": "long"},
        "exit": exits if exits is not None else exit_block(),
        "portfolio": portfolio if portfolio is not None else portfolio_block(),
        "rebalance": rebalance if rebalance is not None else rebalance_block(),
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
        "provenance": {"generator": "discovery.fixture", "seed": 1, "parents": []},
    }


def compile_plan(payload: dict[str, Any]) -> CompiledPlan:
    """
    Return the compiled plan of one reference strategy, through the repository's own compiler.

    The assertion carries the compiler's refusals, so a payload the compiler refuses is a visible
    failure rather than a scenario that silently asserts nothing. A refusal here means the reference
    strategy cannot be expressed in the language, not that the interpreter disagrees with it.
    """
    compiled = compile_specification(payload, policy(), FeatureRegistry((definition(),)))

    assert compiled.plan is not None, [refusal.message for refusal in compiled.errors]

    return compiled.plan


@contextmanager
def interpreter_run(
    payload: dict[str, Any],
    closes: dict[str, tuple[str, ...]],
) -> Iterator[tuple[BacktestEngine, DiscoveryInterpreter]]:
    """
    Run one reference strategy through the compiled interpreter, yielding the engine and it.
    """
    plan = compile_plan(payload)
    memberships = build_memberships(
        "fixture-universe",
        "discovery.fixture",
        [(instrument_id, 0, None) for instrument_id in INSTRUMENTS],
    )
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=VENUE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money(STARTING_BALANCE, USD)],
        base_currency=USD,
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    for instrument_id in INSTRUMENTS:
        engine.add_instrument(equity(instrument_id))
    engine.add_data(bars(closes))
    engine.add_data(quotes(closes))
    strategy = DiscoveryInterpreter(
        DiscoveryInterpreterConfig(
            inputs=InterpreterRunInputs(
                plan=plan,
                memberships=memberships,
                instrument_ids=INSTRUMENTS,
                venue=VENUE,
                commission_rate=Decimal(0),
                stop_loss_bps=STOP_LOSS_BPS,
                sectors=policy().sectors,
            ),
        ),
    )
    engine.add_strategy(strategy)

    try:
        engine.run()
        yield engine, strategy
    finally:
        engine.dispose()


def period_of(ts_event: int) -> int:
    """
    Return the fixture period a timestamp belongs to, counting from one.
    """
    return (ts_event - TS_START) // DAY_NS + 1


def decisions_by_period(strategy: DiscoveryInterpreter) -> dict[int, list[DecisionRecord]]:
    """
    Return the decision records grouped by period, in the order the run produced them.
    """
    grouped: dict[int, list[DecisionRecord]] = {}
    for decision in strategy.decisions:
        grouped.setdefault(period_of(decision.decision_at), []).append(decision)

    return grouped


def first_servable_period(strategy: DiscoveryInterpreter) -> int:
    """
    Return the first period whose decisions read any feature value.
    """
    return next(
        period_of(decision.decision_at) for decision in strategy.decisions if decision.cross_section
    )


def complete_decisions(
    strategy: DiscoveryInterpreter, instrument_count: int
) -> list[DecisionRecord]:
    """
    Return the decisions whose cross-section holds every instrument of the scenario's universe.
    """
    return [
        decision
        for decision in strategy.decisions
        if len(decision.cross_section) == instrument_count
    ]


def target_weights(decision: DecisionRecord) -> dict[str, float]:
    """
    Return the weights the allocation emitted, by instrument identifier.
    """
    return {
        str(target.instrument_id): float(target.value.weight or 0.0) for target in decision.targets
    }


def long_strengths(decision: DecisionRecord) -> dict[str, float]:
    """
    Return the strength of every long signal the decision submitted, by instrument identifier.
    """
    return {
        signal.instrument_id: signal.strength
        for signal in decision.signals
        if signal.direction == "LONG"
    }


def flat_instruments(decision: DecisionRecord) -> tuple[str, ...]:
    """
    Return the instruments the decision submitted a reduce-to-zero signal for.
    """
    return tuple(signal.instrument_id for signal in decision.signals if signal.direction == "FLAT")


def submitted_order_ids(strategy: DiscoveryInterpreter) -> set[str]:
    """
    Return every client order id a decision of the run said it submitted.
    """
    return {order_id for decision in strategy.decisions for order_id in decision.client_order_ids}


def open_position_ids(engine: BacktestEngine) -> set[str]:
    """
    Return the instruments the platform holds an open position in after the run.
    """
    return {str(position.instrument_id) for position in engine.cache.positions_open()}


def closed_position_ids(engine: BacktestEngine) -> set[str]:
    """
    Return the instruments whose position the platform recorded as closed by the run.
    """
    return {str(position.instrument_id) for position in engine.cache.positions_closed()}


def test_the_ranking_reference_strategy_selects_and_holds_the_two_leaders() -> None:
    """
    Scenario 1: a ranking strategy.

    Every decision whose cross-section holds all four instruments selects AAPL and then MSFT, in
    that order, submits exactly those two long signals, and allocates each the declared ceiling: the
    rank-proportional shares over two names are 2/3 for the first and 1/3 for the second, and the
    position cap clamps both to 0.25, so each strength is min(0.25 / 0.25, 1.0). NVDA and AMZN are
    never selected and never signalled, and the only open positions after the run are AAPL and MSFT.
    """
    with interpreter_run(specification(), CONSTANT_CLOSES) as (engine, strategy):
        # The data contract: the first decision that reads any value arrives at the lookback period,
        # and it reads one instrument, because the other three bars of that period have not arrived.
        assert first_servable_period(strategy) == LOOKBACK
        first = next(decision for decision in strategy.decisions if decision.cross_section)
        assert tuple(entry.instrument_id for entry in first.cross_section) == (AAPL,)
        assert first.selection == (AAPL,)

        complete = complete_decisions(strategy, len(INSTRUMENTS))
        assert complete
        for decision in complete:
            assert decision.rebalance
            assert decision.selection == (AAPL, MSFT)
            assert target_weights(decision) == {
                AAPL: pytest.approx(CAP, abs=WEIGHT_TOLERANCE),
                MSFT: pytest.approx(CAP, abs=WEIGHT_TOLERANCE),
            }
            assert long_strengths(decision) == {
                AAPL: pytest.approx(min(CAP / CAP, 1.0), abs=STRENGTH_TOLERANCE),
                MSFT: pytest.approx(min(CAP / CAP, 1.0), abs=STRENGTH_TOLERANCE),
            }
            assert sum(target_weights(decision).values()) <= 1.0

        assert {instrument for d in strategy.decisions for instrument in d.selection} == {
            AAPL,
            MSFT,
        }
        assert {signal.instrument_id for d in strategy.decisions for signal in d.signals} == {
            AAPL,
            MSFT,
        }
        assert all(
            signal.direction == "LONG"
            for decision in strategy.decisions
            for signal in decision.signals
        )

        orders = engine.cache.orders()
        assert orders
        assert all(order.side == OrderSide.BUY for order in orders)
        assert {str(order.instrument_id) for order in orders} == {AAPL, MSFT}
        assert {str(order.client_order_id) for order in orders} <= submitted_order_ids(strategy)
        assert open_position_ids(engine) == {AAPL, MSFT}
        assert {fill.instrument_id for fill in strategy.fills} == {AAPL, MSFT}


def test_the_threshold_reference_strategy_selects_by_the_declared_threshold() -> None:
    """
    Scenario 2: a threshold strategy.

    The selection is the instruments whose constant moving average is strictly greater than 92,
    which is AAPL at 100.00 and MSFT at 95.00 by a different rule than the ranking scenario's count.
    NVDA at 90.00 and AMZN at 85.00 are never selected and never signalled, and the only open
    positions after the run are AAPL and MSFT.
    """
    payload = specification(
        family="absolute_threshold",
        selection=threshold_selection("gt", "92"),
        portfolio=portfolio_block(weighting="equal"),
    )

    with interpreter_run(payload, CONSTANT_CLOSES) as (engine, strategy):
        complete = complete_decisions(strategy, len(INSTRUMENTS))
        assert complete
        for decision in complete:
            assert decision.rebalance
            assert decision.selection == (AAPL, MSFT)
            assert target_weights(decision) == {
                AAPL: pytest.approx(CAP, abs=WEIGHT_TOLERANCE),
                MSFT: pytest.approx(CAP, abs=WEIGHT_TOLERANCE),
            }

        assert {instrument for d in strategy.decisions for instrument in d.selection} == {
            AAPL,
            MSFT,
        }
        assert {signal.instrument_id for d in strategy.decisions for signal in d.signals} == {
            AAPL,
            MSFT,
        }
        assert open_position_ids(engine) == {AAPL, MSFT}
        assert {fill.instrument_id for fill in strategy.fills} == {AAPL, MSFT}


def test_the_holding_period_exit_is_counted_in_decision_periods() -> None:
    """
    Scenario 3: a holding-period exit.

    AAPL alone is selected at every rebalance, so it is entered on the first servable decision, at
    period `start`. A cap of two decision periods expires the holding at `start + 2`, and a
    bar-event counter would have expired it part-way through `start + 1` instead. The declared
    holding period is reset by the entry that opens a holding, which is what leaves `start + 3`
    without an exit and `start + 4` with one.
    """
    payload = specification(
        selection=selection_block(count="1"),
        exits=exit_block(holding_period_cap=2),
        portfolio=portfolio_block(position_count=1, weighting="equal"),
    )

    with interpreter_run(payload, CONSTANT_CLOSES) as (engine, strategy):
        start = first_servable_period(strategy)
        assert start == LOOKBACK

        long_periods = tuple(
            sorted(
                {
                    period_of(decision.decision_at)
                    for decision in strategy.decisions
                    if AAPL in long_strengths(decision)
                },
            ),
        )
        exit_periods = tuple(
            sorted(
                {
                    period_of(decision.decision_at)
                    for decision in strategy.decisions
                    if AAPL in flat_instruments(decision)
                },
            ),
        )

        assert long_periods[0] == start
        assert {decision.selection for decision in strategy.decisions} == {(), (AAPL,)}
        assert exit_periods == (start + 2, start + 4, start + 6, start + 8)
        assert start + 1 not in exit_periods
        assert start + 3 not in exit_periods

        # The exit selects the holding once, on the period's first decision, and the entry that
        # follows it in the same period is the implementation's own choice: the fixture asserts the
        # exit signal, not the re-entry's instant inside the period.
        by_period = decisions_by_period(strategy)
        exiting = [d for d in by_period[start + 2] if AAPL in flat_instruments(d)]
        assert len(exiting) == 1
        assert flat_instruments(by_period[start + 2][0]) == (AAPL,)
        assert all(decision.exits == (AAPL,) for decision in exiting)

        # The platform holds one netting position per instrument, so the flatten at an exit period
        # is the sell order and the position the run reopens in the same period; the entry that
        # opens a holding and the one that reopens it are the same size, which is the cap-clamped
        # weight resolved against the account's equity.
        orders = engine.cache.orders()
        sides = [order.side for order in orders]
        assert all(str(order.instrument_id) == AAPL for order in orders)
        assert {str(order.client_order_id) for order in orders} <= submitted_order_ids(strategy)
        assert sides[0] == OrderSide.BUY
        assert sides[-1] == OrderSide.BUY
        sells = [order for order in orders if order.side == OrderSide.SELL]
        assert len(sells) == len(exit_periods)
        open_positions = engine.cache.positions_open()
        assert len(open_positions) == 1
        assert str(open_positions[0].instrument_id) == AAPL
        assert Decimal(str(open_positions[0].signed_qty)) == Decimal(str(sells[0].quantity))


def test_the_rebalance_schedule_rebalances_once_per_iso_week() -> None:
    """
    Scenario 4: a rebalance schedule.

    The twelve periods run from Monday 2022-01-03 to 2022-01-14, so their ISO weeks are 2022-W01
    (periods 1 through 7) and 2022-W02 (periods 8 through 12). A weekly rebalance rebalances on the
    first decision of each week and on no other decision: periods 1 and 8, whose first decisions are
    2022-01-03T00:00:00Z and 2022-01-10T00:00:00Z. No long signal is submitted on a decision that is
    not a rebalance bar.
    """
    payload = specification(rebalance=rebalance_block(frequency="weekly"))

    with interpreter_run(payload, CONSTANT_CLOSES) as (engine, strategy):
        by_period = decisions_by_period(strategy)
        assert tuple(sorted(by_period)) == tuple(range(1, PERIODS + 1))

        for period, decisions in by_period.items():
            flags = tuple(decision.rebalance for decision in decisions)
            assert flags.count(True) <= 1
            if period in WEEK_PERIODS:
                assert flags[0] is True
            else:
                assert flags.count(True) == 0

        rebalancing = [decision for decision in strategy.decisions if decision.rebalance]
        assert [decision.decision_at for decision in rebalancing] == [
            TS_START,
            TS_START + 7 * DAY_NS,
        ]
        assert all(
            decision.rebalance
            for decision in strategy.decisions
            for signal in decision.signals
            if signal.direction == "LONG"
        )
        # The first rebalance bar of the run is still inside the lookback's warm-up, so the entry it
        # would have made does not exist; the second one enters both leaders.
        assert rebalancing[0].selection == ()
        assert rebalancing[1].selection == (AAPL, MSFT)
        assert open_position_ids(engine) == {AAPL, MSFT}
        assert {fill.instrument_id for fill in strategy.fills} == {AAPL, MSFT}


def test_the_target_allocation_emits_the_declared_shares_under_the_position_cap() -> None:
    """
    Scenario 5: a target allocation.

    Three names are ranked by the constant moving average, so the selection order is AAPL, MSFT,
    NVDA. The rank-proportional shares over three names are 3/6, 2/6 and 1/6 of the investable
    ceiling; the position cap of 0.25 clamps the first two and leaves the third, so the emitted
    weights are 0.25, 0.25 and 1/6, their sum is below the ceiling, and the strengths are
    min(weight / cap, 1.0).
    """
    payload = specification(
        selection=selection_block(count="3"),
        portfolio=portfolio_block(position_count=3, weighting="rank_proportional"),
    )

    with interpreter_run(payload, THREE_CLOSES) as (engine, strategy):
        complete = complete_decisions(strategy, len(THREE_CLOSES))
        assert complete
        decision = complete[0]

        assert decision.selection == (AAPL, MSFT, NVDA)
        weights = target_weights(decision)
        assert tuple(weights) == (AAPL, MSFT, NVDA)
        assert weights[AAPL] == pytest.approx(CAP, abs=WEIGHT_TOLERANCE)
        assert weights[MSFT] == pytest.approx(CAP, abs=WEIGHT_TOLERANCE)
        assert weights[NVDA] == pytest.approx(min(1 / 6, CAP), abs=WEIGHT_TOLERANCE)
        assert sum(weights.values()) <= 1.0

        strengths = long_strengths(decision)
        assert strengths[AAPL] == pytest.approx(min(CAP / CAP, 1.0), abs=STRENGTH_TOLERANCE)
        assert strengths[MSFT] == pytest.approx(min(CAP / CAP, 1.0), abs=STRENGTH_TOLERANCE)
        assert strengths[NVDA] == pytest.approx(min((1 / 6) / CAP, 1.0), abs=STRENGTH_TOLERANCE)

        for other in complete[1:]:
            assert target_weights(other) == weights

        assert open_position_ids(engine) == {AAPL, MSFT, NVDA}


def test_the_strategy_with_no_eligible_instruments_does_nothing() -> None:
    """
    Scenario 6: a strategy with no eligible instruments.

    A threshold of 1000 is above every close, so the selection is empty at every decision, no signal
    is ever submitted, the engine holds no order, no fill is reported, and no position is opened or
    closed. The assertion is that nothing happened, not that the run completed.
    """
    payload = specification(
        family="absolute_threshold",
        selection=threshold_selection("gt", "1000"),
        portfolio=portfolio_block(weighting="equal"),
    )

    with interpreter_run(payload, CONSTANT_CLOSES) as (engine, strategy):
        assert strategy.decisions
        for decision in strategy.decisions:
            assert decision.selection == ()
            assert decision.targets == ()
            assert decision.signals == ()
            assert decision.client_order_ids == ()

        assert engine.cache.orders() == []
        assert strategy.fills == []
        assert open_position_ids(engine) == set()
        assert closed_position_ids(engine) == set()


def test_the_opening_gap_exits_on_the_close_and_fills_below_the_stop() -> None:
    """
    Scenario 7: an opening gap.

    The stop is evaluated against the decision bar's own close, measured from the position's average
    entry price as the platform recorded it. AAPL's closes are 100.00 through period 4 and 70.00 at
    period 5, so the stop of 10 per cent fires first at period 5: the close of 70.00 is at or below
    the threshold of 90.00, no earlier period exits, and the reduce-to-zero signal the exit submits
    is followed by a fill strictly below the threshold, which is the declared execution model
    handling the gap rather than a fill fabricated at the threshold.

    The gapped close also drops AAPL below MSFT in the ranking, so the allocation would flatten the
    holding as one that left the selection as well; the fixture asserts the stop's own record,
    `decision.exits`, and reads the entry the stop is measured from off the platform's position.
    """
    payload = specification(
        selection=selection_block(count="1"),
        exits=exit_block(stop_fraction="0.1"),
        portfolio=portfolio_block(position_count=1, weighting="equal"),
    )

    with interpreter_run(payload, GAP_CLOSES) as (engine, strategy):
        assert first_servable_period(strategy) == LOOKBACK

        # The entry is the platform's own record of the position the run opened, not an assumption.
        closed = [
            position
            for position in engine.cache.positions_closed()
            if str(position.instrument_id) == AAPL
        ]
        assert len(closed) == 1
        entry = Decimal(str(closed[0].avg_px_open))
        threshold = entry * (1 - Decimal("0.1"))
        assert entry == Decimal("100.00")
        assert threshold == Decimal("90.00")

        exit_decisions = [decision for decision in strategy.decisions if AAPL in decision.exits]
        assert [period_of(decision.decision_at) for decision in exit_decisions] == [GAP_PERIOD]
        exit_decision = exit_decisions[0]
        assert flat_instruments(exit_decision) == (AAPL,)

        exit_fills = [
            fill
            for fill in strategy.fills
            if fill.instrument_id == AAPL and fill.decision_at == exit_decision.decision_at
        ]
        assert len(exit_fills) == 1
        fill_price = Decimal(exit_fills[0].last_px)
        assert fill_price < threshold
        assert fill_price == pytest.approx(Decimal("70.00"), abs=PRICE_TOLERANCE)

        assert AAPL not in open_position_ids(engine)
        assert closed_position_ids(engine) == {AAPL}
