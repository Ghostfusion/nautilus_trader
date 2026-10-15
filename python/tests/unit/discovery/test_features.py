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
Test the feature-value contract: definitions, the registry, and both sides of the delivery.

The parity fixture runs the contract inside a backtest. These tests pin the parts a backtest cannot
isolate: that a definition's digest is its identity rather than its name, that an operator or a
lookback this build cannot drive is refused, that the research path produces no reading before the
declared lookback, and that the decision-time side serves the reading the declaration asked for.
"""

from __future__ import annotations

from typing import cast

import pytest

from nautilus_trader.discovery.features import OPERATOR_EMA
from nautilus_trader.discovery.features import FeatureDefinition
from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.features import FeatureStream
from nautilus_trader.discovery.features import FeatureValueAt
from nautilus_trader.discovery.features import build_memberships
from nautilus_trader.discovery.features import build_panel
from nautilus_trader.discovery.features import compute_observations
from nautilus_trader.discovery.schema import FeatureDeclaration
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.research import MembershipSeries


DAY_NS = 86_400 * 1_000_000_000
START = 1_704_067_200_000_000_000

INSTRUMENT_ID = InstrumentId.from_str("AAPL.XNAS")
BAR_TYPE = BarType.from_str(f"{INSTRUMENT_ID}-1-DAY-LAST-EXTERNAL")
CLOSES = ("100.00", "101.00", "102.00", "103.00", "104.00", "105.00", "104.00", "103.00")
LOOKBACK = 5
ALIAS = "trend"
FEATURE_ID = "close_ema_5"
OTHER_FEATURE_ID = "close_sma_50"


def definition(
    *,
    feature_id: str = FEATURE_ID,
    operator: str = OPERATOR_EMA,
    lookback: object = LOOKBACK,
    availability_lag_ns: int = 0,
    lower: float | None = None,
    upper: float | None = None,
    warmup: str = "unavailable",
) -> FeatureDefinition:
    """
    Build the fixture definition, with any field overridden.

    The lookback is taken as an object so a test can hand the constructor a value of the wrong type
    and observe the refusal rather than a type checker's.
    """
    return FeatureDefinition(
        feature_id=feature_id,
        operator=operator,
        input_field="close",
        lookback=cast("int", lookback),
        availability_lag_ns=availability_lag_ns,
        lower=lower,
        upper=upper,
        warmup=warmup,
    )


def declaration(*, feature_id: str = FEATURE_ID, skip: int = 0) -> FeatureDeclaration:
    """
    Build the fixture declaration, with any field overridden.
    """
    return FeatureDeclaration(
        alias=ALIAS,
        feature_id=feature_id,
        lookback=LOOKBACK,
        skip=skip,
    )


def bars(closes: tuple[str, ...] = CLOSES) -> list[Bar]:
    """
    Build one daily bar per close.
    """
    built: list[Bar] = []
    for index, close in enumerate(closes):
        ts = START + index * DAY_NS
        price = Price.from_str(close)
        built.append(
            Bar(
                bar_type=BAR_TYPE,
                open=price,
                high=price,
                low=price,
                close=price,
                volume=Quantity.from_int(1_000),
                ts_event=ts,
                ts_init=ts,
            ),
        )

    return built


def memberships(*, exited_at: int | None = None) -> MembershipSeries:
    """
    Build the fixture membership series.
    """
    return build_memberships("TEST", "fixture", [(str(INSTRUMENT_ID), START, exited_at)])


def stream(
    *,
    declarations: tuple[FeatureDeclaration, ...] | None = None,
    feature_definition: FeatureDefinition | None = None,
) -> FeatureStream:
    """
    Build the fixture stream over one registry and one membership series.
    """
    resolved = definition() if feature_definition is None else feature_definition

    return FeatureStream(
        declarations or (declaration(),),
        FeatureRegistry([resolved]),
        memberships(),
        INSTRUMENT_ID,
    )


def served(
    feature_stream: FeatureStream, series: list[Bar]
) -> list[tuple[int, FeatureValueAt | None]]:
    """
    Drive the stream over the series and return what each decision instant was served.
    """
    read: list[tuple[int, FeatureValueAt | None]] = []
    for bar in series:
        feature_stream.update(bar)
        read.append((bar.ts_event, feature_stream.snapshot(bar.ts_event).get(ALIAS)))

    return read


def test_a_definition_digest_is_the_definitions_identity() -> None:
    """
    Test that the digest follows the computation rather than the name.

    Two definitions that compute the same thing agree, and a change to anything a value depends on
    differs, which is what stops a changed convention from masquerading as the same feature.
    """
    assert definition().digest() == definition().digest()
    assert definition().digest() != definition(lookback=6).digest()
    assert definition().digest() != definition(operator="sma").digest()
    assert definition().digest() != definition(feature_id="close_ema_20").digest()
    assert definition().digest().startswith("sha256:")


def test_a_definition_this_build_cannot_drive_is_refused() -> None:
    """
    Test that an unusable definition is refused rather than silently skipped.
    """
    with pytest.raises(ValueError, match="operator must be one of"):
        definition(operator="hull")

    with pytest.raises(ValueError, match="lookback must be positive"):
        definition(lookback=0)

    with pytest.raises(TypeError, match="lookback must be an int"):
        definition(lookback=1.5)

    with pytest.raises(ValueError, match="availability_lag_ns must not be negative"):
        definition(availability_lag_ns=-1)

    with pytest.raises(ValueError, match="lower <= upper"):
        definition(lower=2.0, upper=1.0)

    with pytest.raises(ValueError, match="warmup must be one of"):
        definition(warmup="zero")


def test_a_registry_resolves_only_the_definitions_it_holds() -> None:
    """
    Test the registry's lookups and its refusal of an ambiguous one.
    """
    registry = FeatureRegistry([definition()])

    assert len(registry) == 1
    assert FEATURE_ID in registry
    assert OTHER_FEATURE_ID not in registry
    assert registry.resolve(FEATURE_ID) is not None
    assert registry.resolve(OTHER_FEATURE_ID) is None
    assert [item.feature_id for item in registry.definitions()] == [FEATURE_ID]
    assert registry.digest() == FeatureRegistry([definition()]).digest()
    assert registry.digest() != FeatureRegistry([definition(lookback=6)]).digest()

    with pytest.raises(ValueError, match="distinct"):
        FeatureRegistry([definition(), definition()])

    with pytest.raises(TypeError, match="must hold FeatureDefinition"):
        FeatureRegistry([FEATURE_ID])  # type: ignore[list-item]


def test_the_research_path_produces_no_reading_before_the_lookback() -> None:
    """
    Test that a warm-up reading is not a value.

    The repository's indicator reports zero until its period is satisfied, so the research path
    omits those bars rather than publishing a price of zero as an observation.
    """
    series = bars()
    observations = compute_observations(definition(), series)

    assert len(observations) == len(series) - (LOOKBACK - 1)
    assert [item.observed_at for item in observations] == [
        bar.ts_event for bar in series[LOOKBACK - 1 :]
    ]
    assert all(item.value != 0.0 for item in observations)


def test_the_availability_lag_moves_availability_rather_than_observation() -> None:
    """
    Test that a lag is a separate instant from the time the value describes.
    """
    series = bars()
    lagged = compute_observations(definition(availability_lag_ns=DAY_NS), series)
    plain = compute_observations(definition(), series)

    assert [item.observed_at for item in lagged] == [item.observed_at for item in plain]
    assert [item.available_at for item in lagged] == [item.available_at + DAY_NS for item in plain]
    assert [item.value for item in lagged] == [item.value for item in plain]


def test_the_stream_serves_nothing_before_the_lookback_and_a_value_afterwards() -> None:
    """
    Test the warm-up boundary at the decision-time side.
    """
    series = bars()
    read = served(stream(), series)

    assert all(value is None for _, value in read[: LOOKBACK - 1])
    assert all(value is not None for _, value in read[LOOKBACK - 1 :])

    observations = compute_observations(definition(), series)
    values = [value for _, value in read[LOOKBACK - 1 :]]

    assert all(value is not None for value in values)
    assert [cast("FeatureValueAt", value).observed_at for value in values] == [
        item.observed_at for item in observations
    ]
    assert [cast("FeatureValueAt", value).value for value in values] == [
        item.value for item in observations
    ]


def test_the_stream_serves_the_reading_the_declaration_asked_for() -> None:
    """
    Test the skip rule: a declaration picks how far back its decision is made.
    """
    series = bars()
    skip = 2
    read = served(stream(declarations=(declaration(skip=skip),)), series)
    first_instant, first_value = next(item for item in read if item[1] is not None)

    assert first_instant == series[LOOKBACK - 1 + skip].ts_event
    assert first_value is not None
    assert first_value.observed_at == series[LOOKBACK - 1].ts_event
    assert first_value.value == compute_observations(definition(), series)[0].value


def test_the_stream_defers_a_value_whose_availability_has_not_passed() -> None:
    """
    Test that a value observed now and available later is not served now.
    """
    series = bars()
    feature_definition = definition(availability_lag_ns=DAY_NS)
    read = served(stream(feature_definition=feature_definition), series)
    first_instant, first_value = next(item for item in read if item[1] is not None)

    assert first_instant == series[LOOKBACK - 1].ts_event + DAY_NS
    assert first_value is not None
    assert first_value.observed_at == series[LOOKBACK - 1].ts_event
    assert first_value.available_at == first_instant
    # The bar that produced the value served nothing, because the value was not yet available.
    assert read[LOOKBACK - 1][1] is None


def test_the_snapshot_answers_for_an_alias_it_does_not_carry() -> None:
    """
    Test the snapshot's own lookups, which are what a selection reads.
    """
    series = bars()
    feature_stream = stream()

    for bar in series:
        feature_stream.update(bar)

    snapshot = feature_stream.snapshot(series[-1].ts_event)
    value = snapshot.get(ALIAS)

    assert snapshot.aliases() == (ALIAS,)
    assert ALIAS in snapshot
    assert "other" not in snapshot
    assert snapshot.get("other") is None
    assert snapshot.ts_event == series[-1].ts_event
    assert value is not None
    assert value.units == "price"
    assert value.member is True
    assert value.feature_id == FEATURE_ID


def test_the_stream_reads_membership_at_the_decision_instant() -> None:
    """
    Test that the universe state a value is delivered with is the decision instant's.
    """
    series = bars()
    exit_at = series[3].ts_event
    feature_stream = FeatureStream(
        (declaration(),),
        FeatureRegistry([definition()]),
        memberships(exited_at=exit_at),
        INSTRUMENT_ID,
    )
    read = served(feature_stream, series)
    after = [
        cast("FeatureValueAt", value)
        for instant, value in read
        if instant >= exit_at and value is not None
    ]

    assert after
    assert all(value.member is False for value in after)
    # The exit falls inside the warm-up for the first bars it could have applied to, so a value and
    # its membership are read independently rather than one standing in for the other.
    boundary = read[LOOKBACK - 1][1]
    assert boundary is not None
    assert boundary.member is False


def test_a_panel_carries_a_row_for_every_reading_and_no_warmup_row() -> None:
    """
    Test the offline artifact: one row per reading, stamped at the observation it holds.
    """
    series = bars()
    panel = build_panel(
        (declaration(),),
        FeatureRegistry([definition()]),
        {str(INSTRUMENT_ID): series},
        memberships(),
    )
    observations = compute_observations(definition(), series)
    rows = panel.rows()

    assert panel.len() == len(rows) == len(observations)
    assert rows[0].instrument_id == INSTRUMENT_ID
    assert rows[0].ts_event == series[LOOKBACK - 1].ts_event
    assert [row.features[ALIAS].value for row in rows] == [item.value for item in observations]
    assert [row.features[ALIAS].as_of for row in rows] == [
        item.observed_at for item in observations
    ]
    assert all(row.member is True for row in rows)
    assert panel.universe() == "TEST"


def test_a_panel_cannot_be_built_for_a_feature_the_registry_does_not_hold() -> None:
    """
    Test that the offline side refuses an unresolvable declaration rather than writing nothing.
    """
    with pytest.raises(ValueError, match="is not registered"):
        build_panel(
            (declaration(feature_id=OTHER_FEATURE_ID),),
            FeatureRegistry([definition()]),
            {str(INSTRUMENT_ID): bars()},
            memberships(),
        )


def test_a_stream_refuses_a_declaration_the_registry_does_not_hold() -> None:
    """
    Test that a candidate's feature is never silently absent from a run.
    """
    with pytest.raises(ValueError, match="is not registered"):
        stream(declarations=(declaration(feature_id=OTHER_FEATURE_ID),))
