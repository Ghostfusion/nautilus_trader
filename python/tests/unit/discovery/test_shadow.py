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
Tests for the shadow monitor: the shared temporal contract and the divergence report.

Each case is a period with one thing changed - a late signal, a non-fill, a missing observation, a
cost that moved - and the assertion is on the report's fields, because those are what a reader
diagnoses a divergence from rather than the wording of a finding.
"""

from __future__ import annotations

from decimal import Decimal

import pytest

from nautilus_trader.discovery.schema import DataBlock
from nautilus_trader.discovery.shadow import ShadowError
from nautilus_trader.discovery.shadow import ShadowObservation
from nautilus_trader.discovery.shadow import ShadowPlan
from nautilus_trader.discovery.shadow import ShadowRefusalCode
from nautilus_trader.discovery.shadow import TemporalContract
from nautilus_trader.discovery.shadow import TemporalReading
from nautilus_trader.discovery.shadow import shadow_period
from nautilus_trader.discovery.shadow import temporal_parity


SIGNAL_NS = 1_700_000_000_000_000_000
SECOND_NS = 1_000_000_000
MINUTE_NS = 60 * SECOND_NS


def data_block(*, eligibility: str = "next_session_open") -> DataBlock:
    """
    Return the data block a candidate declares.
    """
    return DataBlock(
        signal_frequency="daily",
        decision_time="session_close",
        execution_eligibility=eligibility,
        corporate_action_treatment="total_return",
    )


def contract(*, eligibility: str = "next_session_open") -> TemporalContract:
    """
    Return the temporal contract of the fixture candidate.
    """
    return TemporalContract.of(data_block(eligibility=eligibility))


def plan(*, minimum_observations: int = 3) -> ShadowPlan:
    """
    Return the plan a shadow period is declared under.
    """
    return ShadowPlan(
        candidate_id="sha256:" + "a" * 64,
        specification_digest="sha256:" + "c" * 64,
        period="2026-01-05 to 2026-02-02",
        minimum_observations=minimum_observations,
    )


def test_the_declared_field_set_is_the_specifications_data_block() -> None:
    """
    The contract is the data block's four declarations, and a changed declaration is a new contract.
    """
    declared = contract()

    assert declared.declared_fields() == (
        ("signal_frequency", "daily"),
        ("decision_time", "session_close"),
        ("execution_eligibility", "next_session_open"),
        ("corporate_action_treatment", "total_return"),
    )
    assert declared.to_dict() == {
        "signal_frequency": "daily",
        "decision_time": "session_close",
        "execution_eligibility": "next_session_open",
        "corporate_action_treatment": "total_return",
    }
    assert declared.digest() == contract().digest()
    assert declared.digest() != contract(eligibility="same_session_close").digest()


def test_parity_holds_only_when_the_declarations_and_the_pair_match() -> None:
    """
    A shadow run deciding at another instant, or under another declaration, is not sharing a contract.

    The two halves fail differently, which is why both are reported: a differing declaration means
    the shadow run is not shadowing this candidate at all, and a differing instant means the feed
    delivered the decision somewhere else.
    """
    replay = TemporalReading(
        contract=contract(),
        decision_at=SIGNAL_NS,
        eligible_fill_at=SIGNAL_NS + MINUTE_NS,
    )

    shared = temporal_parity(replay, replay)

    assert shared.holds is True
    assert shared.same_fields is True
    assert shared.same_pair is True
    assert shared.differing_fields == ()
    assert shared.describe() == "replay and shadow share one temporal contract"
    assert shared.to_dict()["replay"] == replay.to_dict()

    late = TemporalReading(
        contract=contract(),
        decision_at=SIGNAL_NS,
        eligible_fill_at=SIGNAL_NS + 2 * MINUTE_NS,
    )
    timing = temporal_parity(replay, late)

    assert timing.holds is False
    assert timing.same_fields is True
    assert timing.same_pair is False
    assert timing.differing_fields == ()
    assert "the timestamps differ" in timing.describe()

    other = TemporalReading(
        contract=contract(eligibility="same_session_close"),
        decision_at=SIGNAL_NS,
        eligible_fill_at=SIGNAL_NS + MINUTE_NS,
    )
    declarations = temporal_parity(replay, other)

    assert declarations.holds is False
    assert declarations.same_fields is False
    assert declarations.same_pair is True
    assert declarations.differing_fields == ("execution_eligibility",)
    assert "execution_eligibility" in declarations.describe()


def test_a_shadow_period_reports_each_divergence_it_observed() -> None:
    """
    Criterion 15: the counts, the discrepancies, the non-fills and the cost difference are fields.

    The period is read as divergence rather than as a result: one decision was delivered a second
    late, one produced no fill and says why, one was never delivered at all, and the observed cost
    differed from the modelled one.
    """
    declared = plan(minimum_observations=3)
    expected = (SIGNAL_NS, SIGNAL_NS + MINUTE_NS, SIGNAL_NS + 2 * MINUTE_NS)
    observations = (
        ShadowObservation(
            signal_at=SIGNAL_NS,
            intended_entry=True,
            eligible_fill_at=SIGNAL_NS + MINUTE_NS,
            fill_price=Decimal("100.00"),
            modelled_cost=Decimal("1.00"),
            observed_cost=Decimal("1.25"),
        ),
        ShadowObservation(
            signal_at=SIGNAL_NS + MINUTE_NS + SECOND_NS,
            intended_entry=True,
            eligible_fill_at=SIGNAL_NS + 2 * MINUTE_NS,
            modelled_cost=Decimal("1.00"),
            observed_cost=Decimal("1.00"),
            non_fill_reason="the security was halted",
        ),
    )

    report = shadow_period(
        declared,
        contract(),
        expected_signal_times=expected,
        observations=observations,
    )

    assert report.label == "simulated"
    assert report.schema == "discovery.shadow/v1"
    assert report.eligible_decisions == 3
    assert report.observed_decisions == 2

    # The late signal carries another instant, so both of those decisions are undelivered.
    assert report.missing_observations == 2

    assert report.readable is False
    assert "2 observations against the 3 declared" in report.unreadable_reason

    discrepancies = report.signal_discrepancies
    assert len(discrepancies) == 1
    assert discrepancies[0].expected_at == SIGNAL_NS + MINUTE_NS
    assert discrepancies[0].observed_at == SIGNAL_NS + MINUTE_NS + SECOND_NS
    assert discrepancies[0].magnitude == SECOND_NS

    assert [(non_fill.signal_at, non_fill.reason) for non_fill in report.non_fills] == [
        (SIGNAL_NS + MINUTE_NS + SECOND_NS, "the security was halted"),
    ]
    assert report.modelled_cost == Decimal("2.00")
    assert report.observed_cost == Decimal("2.25")
    assert report.cost_difference == Decimal("0.25")
    assert report.diverged is True

    # One line per class of divergence: the count of what never arrived, then each case.
    assert len(report.findings()) == 4
    assert report.findings()[0] == "2 observations were never delivered"

    as_data = report.to_dict()

    assert as_data["plan_digest"] == declared.digest()
    assert as_data["cost_difference"] == "0.25"
    assert as_data["readable"] is False
    assert report.digest().startswith("sha256:")

    # The declaration is part of the report, so two reports under two plans are two identities.
    assert (
        shadow_period(
            plan(minimum_observations=4),
            contract(),
            expected_signal_times=expected,
            observations=observations,
        ).digest()
        != report.digest()
    )


def test_a_period_that_reached_its_declared_count_is_readable() -> None:
    """
    Meeting the pre-registered count is the precondition for reading a report, and it is all it is.
    """
    declared = plan(minimum_observations=2)
    observation = ShadowObservation(
        signal_at=SIGNAL_NS,
        intended_entry=True,
        eligible_fill_at=SIGNAL_NS + MINUTE_NS,
        fill_price=Decimal("100.00"),
    )

    report = shadow_period(
        declared,
        contract(),
        expected_signal_times=(SIGNAL_NS,),
        observations=(observation, observation),
    )

    assert report.readable is True
    assert report.unreadable_reason == ""
    assert report.missing_observations == 0
    assert report.non_fills == ()

    # A repeated signal is visible rather than discarded, with a magnitude of zero.
    assert [found.magnitude for found in report.signal_discrepancies] == [0, 0]
    assert report.diverged is True


def test_a_period_that_cannot_be_read_as_declared_is_refused() -> None:
    """
    A plan with no declared count, and an observation whose fill precedes its own signal, are refused.

    Neither is a divergence to report: a period with no declared count cannot be read as declared at
    all, and a fill before its signal is a broken record rather than market behaviour.
    """
    with pytest.raises(ShadowError) as undeclared:
        shadow_period(
            plan(minimum_observations=0),
            contract(),
            expected_signal_times=(),
            observations=(),
        )

    assert undeclared.value.code is ShadowRefusalCode.MINIMUM_NOT_DECLARED
    assert "0 as its minimum observation count" in str(undeclared.value)

    broken = ShadowObservation(
        signal_at=SIGNAL_NS,
        intended_entry=True,
        eligible_fill_at=SIGNAL_NS - SECOND_NS,
    )
    with pytest.raises(ShadowError) as malformed:
        shadow_period(
            plan(minimum_observations=1),
            contract(),
            expected_signal_times=(SIGNAL_NS,),
            observations=(broken,),
        )

    assert malformed.value.code is ShadowRefusalCode.ELIGIBLE_BEFORE_SIGNAL
    assert "is eligible at" in str(malformed.value)

    unexplained = ShadowObservation(
        signal_at=SIGNAL_NS,
        intended_entry=True,
        eligible_fill_at=SIGNAL_NS + MINUTE_NS,
    )
    with pytest.raises(ShadowError) as unreasoned:
        shadow_period(
            plan(minimum_observations=1),
            contract(),
            expected_signal_times=(SIGNAL_NS,),
            observations=(unexplained,),
        )

    assert unreasoned.value.code is ShadowRefusalCode.NON_FILL_WITHOUT_REASON
    assert "did not fill and gives no reason" in str(unreasoned.value)

    # An observation that intended nothing is neither a fill nor a non-fill, so it is not refused.
    quiet = ShadowObservation(
        signal_at=SIGNAL_NS,
        intended_entry=False,
        eligible_fill_at=SIGNAL_NS,
    )
    quiet_report = shadow_period(
        plan(minimum_observations=1),
        contract(),
        expected_signal_times=(SIGNAL_NS,),
        observations=(quiet,),
    )

    assert quiet_report.non_fills == ()
    assert quiet_report.diverged is False
