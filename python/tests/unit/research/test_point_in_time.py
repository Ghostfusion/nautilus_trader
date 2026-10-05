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
Tests for the Python binding of the research crate's point-in-time minimum.

The Rust-side contract is tested in `crates/research/tests/leakage.rs`; these tests prove the
same rules are reachable and enforced from Python: a feature value carries the instant of the
latest input it reads, a feature observed after its row timestamp is rejected, the row timestamp
is an inclusive boundary, and membership is resolved from the stored series rather than from
today's universe.
"""

from __future__ import annotations

import pytest

from nautilus_trader.model import InstrumentId
from nautilus_trader.research import FeatureValue
from nautilus_trader.research import MembershipInterval
from nautilus_trader.research import MembershipSeries
from nautilus_trader.research import Panel
from nautilus_trader.research import PanelRow


UNIVERSE = "research"
SOURCE = "rule/v1"
INSTRUMENT = InstrumentId.from_str("A.X")


def series() -> MembershipSeries:
    """
    Build the stored membership series the tests resolve against.
    """
    stored = MembershipSeries(UNIVERSE, SOURCE)
    stored.push(MembershipInterval(UNIVERSE, SOURCE, INSTRUMENT, 100, None))
    return stored


def test_a_feature_value_carries_the_instant_of_its_latest_input() -> None:
    """
    Test that a feature value reports the point in time of the data it reads.
    """
    value = FeatureValue(0.5, 101)

    assert value.value == 0.5
    assert value.as_of == 101


def test_a_feature_reading_its_own_future_is_rejected() -> None:
    """
    Test that a feature observed after the row timestamp is rejected.
    """
    row = PanelRow(INSTRUMENT, 100).with_feature("future_return", 0.5, 101).with_member(True)

    with pytest.raises(ValueError, match="future_return"):
        Panel(series(), [row])


def test_a_feature_observed_at_the_row_timestamp_is_accepted() -> None:
    """
    Test that the row timestamp is an inclusive boundary for a feature's observations.
    """
    row = PanelRow(INSTRUMENT, 100).with_feature("mid", 42.0, 100).with_label(1.5).with_member(True)
    panel = Panel(series(), [row])

    panel.check()

    assert panel.len() == 1
    assert panel.universe() == UNIVERSE
    assert not panel.is_empty()
    assert panel.rows()[0].features["mid"].as_of == 100
    assert panel.rows()[0].label == 1.5
    assert panel.rows()[0].member


def test_membership_is_resolved_from_the_stored_series_at_the_row_timestamp() -> None:
    """
    Test that membership is read point-in-time, and that claiming it without one fails.
    """
    stored = series()

    assert stored.universe() == UNIVERSE
    assert stored.source() == SOURCE
    assert stored.is_member_at(INSTRUMENT, 100)
    assert stored.members_at(100) == [INSTRUMENT]
    assert stored.instruments() == [INSTRUMENT]

    # The interval is half open: the entry instant is covered, the exit instant is not
    interval = MembershipInterval(UNIVERSE, SOURCE, INSTRUMENT, 100, 200)
    assert interval.instrument_id == INSTRUMENT
    assert interval.ts_event == 100
    assert interval.exited_at == 200
    assert interval.covers(100)
    assert not interval.covers(200)

    early = PanelRow(INSTRUMENT, 99).with_member(True)

    with pytest.raises(ValueError, match="membership"):
        Panel(stored, [early])
