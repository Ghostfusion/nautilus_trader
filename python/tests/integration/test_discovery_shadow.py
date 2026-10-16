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
`FX-M6-2` and `FX-M6-3`: one temporal contract for replay and shadow, and the divergence report.

Criterion 13 asks for something stronger than two runs agreeing, which is why the fixture is built so
that the two halves are produced independently. The replay's decision and eligibility instants come
from the engine: the interpreter records the decision at the bar it decided at, and the fill records
the instant the venue filled it. The shadow half comes from the feed: an observed bar carries the
instant the feed delivered it, and the eligible instant is the next observed bar's, which is the
declared `next_session_open` eligibility read off the observed data rather than off the run. The two
agree only if the interpreter and the feed implement the same declaration, and the negative cases
below show that the comparison can fail.

Criterion 15 is the report rather than the period: a late signal, a missing observation, a simulated
non-fill with its reason and a cost that moved are each a field of the report, so the divergence
between the backtest's assumption and the market's behaviour is diagnosable instead of summarised. A
shadow result is labelled simulated wherever it is shown, the observation count is the one declared
before the period started, and nothing here is a claim about profitability: the report is evidence
about integration and operational behaviour.

The live-data side of a shadow period is a platform decision, which is why the fixture feeds the
monitor observed bars rather than binding an adapter. The bars are the same bars the replay ran over,
so the timing question is asked exactly: a live feed that delivered them later, or not at all, is what
the second fixture shows.
"""

from __future__ import annotations

from collections.abc import Iterator
from contextlib import contextmanager
from decimal import Decimal
from typing import TYPE_CHECKING

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.discovery.compile import compile_specification
from nautilus_trader.discovery.features import FeatureDefinition
from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.features import build_memberships
from nautilus_trader.discovery.policy import DiscoveryPolicy
from nautilus_trader.discovery.policy import Reference
from nautilus_trader.discovery.policy import ReferenceKind
from nautilus_trader.discovery.schema import SPECIFICATION_SCHEMA_VERSION
from nautilus_trader.discovery.shadow import ShadowObservation
from nautilus_trader.discovery.shadow import ShadowPlan
from nautilus_trader.discovery.shadow import TemporalContract
from nautilus_trader.discovery.shadow import TemporalReading
from nautilus_trader.discovery.shadow import shadow_period
from nautilus_trader.discovery.shadow import temporal_parity
from nautilus_trader.discovery.strategy import BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY
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
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Symbol
from nautilus_trader.model import Venue


if TYPE_CHECKING:
    from nautilus_trader.discovery.compile import CompiledPlan


# One instrument is a cross-section, so the ranking selection is expressed without a scenario that
# depends on the order of several names.
INSTRUMENT = "AAPL.NASDAQ"
INSTRUMENTS = (INSTRUMENT,)

TREND_ID = "sma_close_2"
TREND_ALIAS = "trend"
LOOKBACK = 2

CAP_TEXT = "0.25"

USD = Currency.from_str("USD")
VENUE = Venue("NASDAQ")
BAR_SPECIFICATION = BAR_SPECIFICATION_BY_SIGNAL_FREQUENCY["daily"]

SECOND_NS = 1_000_000_000
DAY_NS = 86_400 * SECOND_NS
TS_START = 1_641_168_000_000_000_000
PERIODS = 6

STARTING_BALANCE = 1_000_000.0
VOLUME = 1_000_000
QUOTE_SIZE = 1_000
STOP_LOSS_BPS = 100
REFERENCE_DIGEST = f"sha256:{'0' * 64}"

CLOSES: tuple[str, ...] = ("100.00", "101.00", "102.00", "103.00", "104.00", "105.00")


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


def observed_instants() -> tuple[int, ...]:
    """
    Return each period's observed instant, which is what the feed delivers and the replay decided at.

    A period is one calendar day and its instant is the bar's own event timestamp, so the observed
    series is the same one the replay ran over and the timing question is asked rather than assumed.
    """
    return tuple(TS_START + index * DAY_NS for index in range(PERIODS))


def bars() -> list[Bar]:
    """
    Return the fixture's observed bars.
    """
    return [
        bar(INSTRUMENT, CLOSES[index], instant) for index, instant in enumerate(observed_instants())
    ]


def quotes() -> list[QuoteTick]:
    """
    Return one quote per fixture bar, stamped one second before the bar it belongs to.

    A quote at the decision instant can be processed after the order that instant submitted, so the
    quote is stamped a second early and the target pipeline resolves its entry price from the cache.
    """
    return [
        quote(INSTRUMENT, CLOSES[index], instant - SECOND_NS)
        for index, instant in enumerate(observed_instants())
    ]


def quote(instrument_id: str, close: str, ts_event: int) -> QuoteTick:
    """
    Return one quote at the given close.
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


def eligible_instant(signal_at: int) -> int:
    """
    Return the instant the feed considers a decision at `signal_at` eligible to fill at.

    The declared eligibility is `next_session_open`, which is the first session open at or after the
    decision. The fixture's periods carry one instant each and the candidate decides at the session
    open, so that is the decision's own period; a candidate deciding at the session close would be
    eligible at the following one, and the rule below returns that without being told which it is.
    This is the feed's own reading of the declaration, computed from the declared rule and the
    observed series rather than read back from the run, and it is what the replay's recorded fill
    instant is compared against.
    """
    return next(instant for instant in observed_instants() if instant >= signal_at)


def policy() -> DiscoveryPolicy:
    """
    Return the fixture campaign's pre-registered policy, with its frozen references and sector map.
    """
    return DiscoveryPolicy.first_release(
        feature_allowlist=(TREND_ID,),
        references=(
            Reference("fixture-benchmark", ReferenceKind.BENCHMARK, REFERENCE_DIGEST),
            Reference("fixture-cost-model", ReferenceKind.COST_MODEL, REFERENCE_DIGEST),
            Reference("fixture-validation", ReferenceKind.VALIDATION_POLICY, REFERENCE_DIGEST),
        ),
        sectors=((INSTRUMENT, "mega"),),
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


def payload() -> dict[str, object]:
    """
    Return the frozen candidate's hand-written specification, as the compiler's own payload.
    """
    return {
        "version": SPECIFICATION_SCHEMA_VERSION,
        "identity": {
            "family": "cross_sectional_rank",
            "hypothesis": "the strongest trend in the cross-section persists over the next session",
            "mechanism": "rank the cross-section by a short moving average and hold the leader",
            "falsification": "the ranked leader earns no more than the cross-section median",
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
            "decision_time": "session_open",
            "execution_eligibility": "next_session_open",
            "corporate_action_treatment": "raw",
        },
        "features": {
            "declared": {
                TREND_ALIAS: {"feature": TREND_ID, "lookback": LOOKBACK, "skip": 0},
            },
        },
        "selection": {
            "mode": "rank",
            "feature": TREND_ALIAS,
            "count": "1",
            "tie_break": "feature_desc",
        },
        "entry": {"direction": "long"},
        "exit": {"flatten_ineligible": True},
        "portfolio": {
            "position_count": 1,
            "weighting": "equal",
            "max_position_weight": CAP_TEXT,
            "cash_policy": "leave_idle",
            "max_sector_weight": "1",
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
        "provenance": {"generator": "discovery.fixture", "seed": 1, "parents": []},
    }


def compile_plan() -> CompiledPlan:
    """
    Return the compiled plan of the frozen candidate, through the repository's own compiler.

    The assertion carries the compiler's refusals, so a payload the compiler refuses is a visible
    failure rather than a scenario that silently asserts nothing.
    """
    compiled = compile_specification(payload(), policy(), FeatureRegistry((definition(),)))

    assert compiled.plan is not None, [refusal.message for refusal in compiled.errors]

    return compiled.plan


@contextmanager
def replay(plan: CompiledPlan) -> Iterator[DiscoveryInterpreter]:
    """
    Run the frozen candidate's specification through the compiled interpreter.

    This is the replay half: the interpreter records the decision at the bar it decided at, and the
    venue records the instant it filled at, so the pair the shadow run is compared against is
    produced by the engine rather than by the fixture.
    """
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=VENUE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money(STARTING_BALANCE, USD)],
        base_currency=USD,
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    engine.add_instrument(equity(INSTRUMENT))
    engine.add_data(bars())
    engine.add_data(quotes())
    strategy = DiscoveryInterpreter(
        DiscoveryInterpreterConfig(
            inputs=InterpreterRunInputs(
                plan=plan,
                memberships=build_memberships(
                    "fixture-universe",
                    "discovery.fixture",
                    [(INSTRUMENT, 0, None)],
                ),
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
        yield strategy
    finally:
        engine.dispose()


def plan_for(plan: CompiledPlan, *, minimum_observations: int) -> ShadowPlan:
    """
    Return the shadow declaration of the frozen candidate.
    """
    return ShadowPlan(
        candidate_id=plan.specification_digest,
        specification_digest=plan.specification_digest,
        period="2022-01-03 to 2022-01-13",
        minimum_observations=minimum_observations,
    )


def test_fx_m6_2_a_shadow_run_records_the_field_set_and_timestamps_of_its_backtest() -> None:
    """
    `FX-M6-2`: criterion 13, over one frozen candidate's replay and its shadow.

    The two readings are produced independently: the replay's instants are the engine's records, and
    the shadow's are the feed's. They agree because both implement the declared contract, and the two
    negative cases show the comparison is capable of failing - once for a declaration, once for an
    instant - so a passing comparison is evidence rather than a tautology.
    """
    plan = compile_plan()
    contract = TemporalContract.of(plan.specification.data)

    with replay(plan) as strategy:
        decisions = tuple(strategy.decisions)
        fills = tuple(strategy.fills)

    assert decisions, "the fixture's candidate decided at no period"
    assert fills, "the fixture's candidate filled nothing, so there is no replay half"

    fill = fills[0]

    assert fill.decision_at is not None, "the fixture's fill records no decision it came from"

    decision = next(record for record in decisions if record.decision_at == fill.decision_at)

    replay_reading = TemporalReading(
        contract=contract,
        decision_at=decision.decision_at,
        eligible_fill_at=fill.ts_event,
    )
    shadow_reading = TemporalReading(
        contract=TemporalContract.of(plan.specification.data),
        decision_at=decision.decision_at,
        eligible_fill_at=eligible_instant(decision.decision_at),
    )

    parity = temporal_parity(replay_reading, shadow_reading)

    assert parity.holds is True, parity.describe()
    assert parity.same_fields is True
    assert parity.same_pair is True
    assert parity.differing_fields == ()

    # The comparison fails when a declaration differs, and names the field it differed in.
    elsewhere = TemporalReading(
        contract=TemporalContract(
            signal_frequency=contract.signal_frequency,
            decision_time=contract.decision_time,
            execution_eligibility="same_session_close",
            corporate_action_treatment=contract.corporate_action_treatment,
        ),
        decision_at=decision.decision_at,
        eligible_fill_at=eligible_instant(decision.decision_at),
    )
    declarations = temporal_parity(replay_reading, elsewhere)

    assert declarations.holds is False
    assert declarations.differing_fields == ("execution_eligibility",)

    # And when the feed delivers the decision at another instant.
    late = TemporalReading(
        contract=contract,
        decision_at=decision.decision_at + SECOND_NS,
        eligible_fill_at=eligible_instant(decision.decision_at) + SECOND_NS,
    )
    timing = temporal_parity(replay_reading, late)

    assert timing.holds is False
    assert timing.same_pair is False
    assert "the timestamps differ" in timing.describe()


def test_fx_m6_3_a_shadow_period_reports_its_divergence_rather_than_summarising_it() -> None:
    """
    `FX-M6-3`: criterion 15, over a period whose feed diverged in four different ways.

    One decision arrived a second late, one intended entry did not fill and says why, the decisions
    after it were never delivered, and the observed cost moved away from the modelled one. Each is a
    field of the report, and the report is labelled simulated: a period that reached its declared
    count is readable, and being readable is not evidence of an edge.
    """
    plan = compile_plan()
    contract = TemporalContract.of(plan.specification.data)

    with replay(plan) as strategy:
        expected = tuple(record.decision_at for record in strategy.decisions)

    assert len(expected) >= 3, "the fixture's candidate decided at fewer periods than the scenario"

    late_at = expected[1] + SECOND_NS
    observations = (
        ShadowObservation(
            signal_at=expected[0],
            intended_entry=True,
            eligible_fill_at=eligible_instant(expected[0]),
            fill_price=Decimal(CLOSES[1]),
            modelled_cost=Decimal("1.00"),
            observed_cost=Decimal("1.25"),
        ),
        ShadowObservation(
            signal_at=late_at,
            intended_entry=True,
            eligible_fill_at=eligible_instant(late_at),
            fill_price=Decimal(CLOSES[2]),
            modelled_cost=Decimal("1.00"),
            observed_cost=Decimal("1.00"),
        ),
        ShadowObservation(
            signal_at=expected[2],
            intended_entry=True,
            eligible_fill_at=eligible_instant(expected[2]),
            non_fill_reason="no quote was observed at the eligible instant",
        ),
    )

    report = shadow_period(
        plan_for(plan, minimum_observations=3),
        contract,
        expected_signal_times=expected,
        observations=observations,
    )

    assert report.label == "simulated"
    assert report.eligible_decisions == len(expected)
    assert report.observed_decisions == 3

    # The late signal carries an instant of its own, so two eligible decisions went undelivered.
    assert report.missing_observations == len(expected) - 2

    discrepancies = report.signal_discrepancies

    assert len(discrepancies) == 1
    assert discrepancies[0].observed_at == late_at
    assert discrepancies[0].magnitude == SECOND_NS

    assert [non_fill.reason for non_fill in report.non_fills] == [
        "no quote was observed at the eligible instant",
    ]
    assert report.modelled_cost == Decimal("2.00")
    assert report.observed_cost == Decimal("2.25")
    assert report.cost_difference == Decimal("0.25")
    assert report.diverged is True

    # Reading a report needs the count declared before the period started, and nothing more.
    assert report.readable is True
    assert report.unreadable_reason == ""
    assert report.digest().startswith("sha256:")

    undeclared = shadow_period(
        plan_for(plan, minimum_observations=len(expected) + 1),
        contract,
        expected_signal_times=expected,
        observations=observations,
    )

    assert undeclared.readable is False
    assert "declared before it started" in undeclared.unreadable_reason
