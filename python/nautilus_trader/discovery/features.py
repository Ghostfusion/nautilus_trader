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
The feature-value contract: what a feature is, and when its value may be used.

Candidates select features; something has to deliver each value to the running strategy at the
moment it became knowable. This module owns both sides of that contract so they can be compared
rather than assumed to agree.

The offline side is `compute_observations`, which drives the repository's own indicator over a bar
series through the research path and pairs each reading with the time it describes and the time it
may be used. The decision-time side is `FeatureStream`, which drives the same indicator classes from
a running strategy and serves the reading the declaration asked for. Both answer the same questions
- the value, its observation instant, its availability instant, its units, and whether the
instrument was a member at the decision instant - because a precomputed panel and a streaming
strategy disagreeing about warm-up, missing values, units or membership is a look-ahead no
downstream gate can see.

Two rules make the contract closed. A definition naming an operator this build cannot drive is
refused rather than skipped, so a feature is never silently absent from a run. And a value is never
fabricated: before the declared lookback is satisfied, no value is served at all, because a moving
average's warm-up reading and a value of zero are not the same statement.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING
from typing import Protocol

from nautilus_trader.analysis.research import compute_indicator
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.indicators import AverageTrueRange
from nautilus_trader.indicators import ExponentialMovingAverage
from nautilus_trader.indicators import RelativeStrengthIndex
from nautilus_trader.indicators import SimpleMovingAverage
from nautilus_trader.model import InstrumentId
from nautilus_trader.optimization.space import digest_of
from nautilus_trader.research import MembershipInterval
from nautilus_trader.research import MembershipSeries
from nautilus_trader.research import Panel
from nautilus_trader.research import PanelRow


if TYPE_CHECKING:
    from collections.abc import Iterable
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.discovery.schema import FeatureDeclaration
    from nautilus_trader.model import Bar


# The operators a definition may name, and the repository's indicator each is driven through. A
# definition naming an operator this table does not hold is refused, so the set is closed.
OPERATOR_EMA = "ema"
OPERATOR_SMA = "sma"
OPERATOR_RSI = "rsi"
OPERATOR_ATR = "atr"

OPERATORS: tuple[str, ...] = (OPERATOR_EMA, OPERATOR_SMA, OPERATOR_RSI, OPERATOR_ATR)

# What happens before the lookback is satisfied, and what a run does with an absent value. Both are
# declared per definition, and both name the same behaviour: the value is not available.
WARMUP_UNAVAILABLE = "unavailable"
MISSING_UNAVAILABLE = "unavailable"

WARMUP_POLICIES: tuple[str, ...] = (WARMUP_UNAVAILABLE,)
MISSING_POLICIES: tuple[str, ...] = (MISSING_UNAVAILABLE,)


class DrivenIndicator(Protocol):
    """
    The part of an indicator both the research path and a running strategy drive.
    """

    @property
    def value(self) -> float:
        """
        The indicator's current reading.
        """
        ...

    def handle_bar(self, bar: Bar) -> None:
        """
        Update the indicator with a bar.
        """
        ...


def _build_indicator(operator: str, lookback: int) -> DrivenIndicator:
    """
    Return the repository's indicator for an operator and a lookback.

    Parameters
    ----------
    operator : str
        The registered operator.
    lookback : int
        The number of observations the value is computed over.

    Returns
    -------
    DrivenIndicator

    Raises
    ------
    ValueError
        If the operator is not one this build can drive.

    """
    if operator == OPERATOR_EMA:
        return ExponentialMovingAverage(lookback)
    if operator == OPERATOR_SMA:
        return SimpleMovingAverage(lookback)
    if operator == OPERATOR_RSI:
        return RelativeStrengthIndex(lookback)
    if operator == OPERATOR_ATR:
        return AverageTrueRange(lookback)

    raise ValueError(f"operator must be one of {', '.join(OPERATORS)}, was {operator}")


@dataclass(frozen=True)
class FeatureDefinition:
    """
    One registered feature: its identity, how its value is computed, and when it may be used.

    Parameters
    ----------
    feature_id : str
        The identifier a specification references.
    operator : str
        The registered operator the value is computed with.
    input_field : str
        The bar field the value is computed from, which the declaration's lookback indexes.
    lookback : int
        The number of observations the value is computed over.
    availability_lag_ns : int, default 0
        The lag between the observation and the earliest time the value may be used.
    units : str, default "price"
        The unit convention of the value, so a scaling error fails at the boundary.
    lower : float, optional
        The inclusive lower end of the value's declared range.
    upper : float, optional
        The inclusive upper end of the value's declared range.
    warmup : str, default WARMUP_UNAVAILABLE
        What happens before the lookback is satisfied.
    missing : str, default MISSING_UNAVAILABLE
        What a run does with an absent value.
    version : int, default 1
        The definition's version, which changes when the computation changes.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If a declaration is unusable.

    """

    feature_id: str
    operator: str
    input_field: str
    lookback: int
    availability_lag_ns: int = 0
    units: str = "price"
    lower: float | None = None
    upper: float | None = None
    warmup: str = WARMUP_UNAVAILABLE
    missing: str = MISSING_UNAVAILABLE
    version: int = 1

    def __post_init__(self) -> None:
        """
        Validate the definition and refuse an unusable one.
        """
        self._check_text()
        self._check_declared()
        self._check_numbers()

    def _check_text(self) -> None:
        """
        Validate the definition's text fields.
        """
        for name, value in (
            ("feature_id", self.feature_id),
            ("input_field", self.input_field),
            ("units", self.units),
        ):
            if not isinstance(value, str):
                raise TypeError(f"{name} must be a str, was {type(value).__name__}")
            if not value.strip():
                raise ValueError(f"{name} must not be empty")

    def _check_declared(self) -> None:
        """
        Validate the values resolved from the declared sets.
        """
        if self.operator not in OPERATORS:
            raise ValueError(
                f"operator must be one of {', '.join(OPERATORS)}, was {self.operator}",
            )
        if self.warmup not in WARMUP_POLICIES:
            raise ValueError(
                f"warmup must be one of {', '.join(WARMUP_POLICIES)}, was {self.warmup}",
            )
        if self.missing not in MISSING_POLICIES:
            raise ValueError(
                f"missing must be one of {', '.join(MISSING_POLICIES)}, was {self.missing}",
            )

    def _check_numbers(self) -> None:
        """
        Validate the definition's numeric fields.
        """
        for name, value in (
            ("lookback", self.lookback),
            ("availability_lag_ns", self.availability_lag_ns),
            ("version", self.version),
        ):
            if isinstance(value, bool) or not isinstance(value, int):
                raise TypeError(f"{name} must be an int, was {type(value).__name__}")
        if self.lookback < 1:
            raise ValueError(f"lookback must be positive, was {self.lookback}")
        if self.availability_lag_ns < 0:
            raise ValueError(
                f"availability_lag_ns must not be negative, was {self.availability_lag_ns}",
            )
        if self.version < 1:
            raise ValueError(f"version must be positive, was {self.version}")
        if self.lower is not None and self.upper is not None and self.lower > self.upper:
            raise ValueError(
                f"a declared range requires lower <= upper, was {self.lower} > {self.upper}",
            )

    def to_dict(self) -> dict[str, object]:
        """
        Return the definition as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "feature_id": self.feature_id,
            "version": self.version,
            "operator": self.operator,
            "input_field": self.input_field,
            "lookback": self.lookback,
            "availability_lag_ns": self.availability_lag_ns,
            "units": self.units,
            "lower": self.lower,
            "upper": self.upper,
            "warmup": self.warmup,
            "missing": self.missing,
        }

    def digest(self) -> str:
        """
        Return the digest of the definition, which is its identity rather than its name.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


class FeatureRegistry:
    """
    The registered definitions a specification's features are resolved against.

    Parameters
    ----------
    definitions : Iterable[FeatureDefinition]
        The definitions this build can serve.

    Raises
    ------
    TypeError
        If a definition has the wrong type.
    ValueError
        If two definitions carry the same identifier.

    """

    def __init__(self, definitions: Iterable[FeatureDefinition]) -> None:
        """
        Initialize the registry over the given definitions.
        """
        resolved: list[FeatureDefinition] = []
        seen: set[str] = set()

        for definition in definitions:
            if not isinstance(definition, FeatureDefinition):
                raise TypeError(
                    f"definitions must hold FeatureDefinition, was {type(definition).__name__}",
                )
            if definition.feature_id in seen:
                raise ValueError(f"feature_id must be distinct, {definition.feature_id} repeats")
            seen.add(definition.feature_id)
            resolved.append(definition)

        self._definitions: tuple[FeatureDefinition, ...] = tuple(resolved)
        self._by_id: dict[str, FeatureDefinition] = {item.feature_id: item for item in resolved}

    def resolve(self, feature_id: str) -> FeatureDefinition | None:
        """
        Return the definition registered under the given identifier, if any.

        Parameters
        ----------
        feature_id : str
            The identifier a specification references.

        Returns
        -------
        FeatureDefinition or None

        """
        return self._by_id.get(feature_id)

    def definitions(self) -> tuple[FeatureDefinition, ...]:
        """
        Return every registered definition, in registration order.

        Returns
        -------
        tuple[FeatureDefinition, ...]

        """
        return self._definitions

    def digest(self) -> str:
        """
        Return the digest of the registry, which names the definitions a result ran against.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of({"definitions": "|".join(item.digest() for item in self._definitions)})

    def __len__(self) -> int:
        """
        Return the number of registered definitions.
        """
        return len(self._definitions)

    def __contains__(self, feature_id: object) -> bool:
        """
        Return whether an identifier is registered.
        """
        return isinstance(feature_id, str) and feature_id in self._by_id


@dataclass(frozen=True)
class FeatureObservation:
    """
    One reading of a feature, with the time it describes and the time it may be used.

    Parameters
    ----------
    observed_at : int
        The time the value describes, as the bar's event timestamp.
    available_at : int
        The earliest time the value may be used, including the declared lag.
    value : float
        The reading.

    """

    observed_at: int
    available_at: int
    value: float


def compute_observations(
    definition: FeatureDefinition,
    bars: Sequence[Bar],
) -> list[FeatureObservation]:
    """
    Compute a feature over a bar series through the research path.

    Each reading is paired with the bar's event timestamp, and no reading is produced before the
    declared lookback is satisfied, because a warm-up reading is not a value.

    Parameters
    ----------
    definition : FeatureDefinition
        The definition to compute.
    bars : Sequence[Bar]
        The bars to compute over, in the order they occurred.

    Returns
    -------
    list[FeatureObservation]

    """
    indicator = _build_indicator(definition.operator, definition.lookback)
    values = compute_indicator(indicator, list(bars))
    first = definition.lookback - 1

    return [
        FeatureObservation(
            observed_at=bars[index].ts_event,
            available_at=bars[index].ts_event + definition.availability_lag_ns,
            value=value,
        )
        for index, value in enumerate(values)
        if index >= first
    ]


@dataclass(frozen=True)
class _StreamedReading:
    """
    One reading a stream produced, with the bar it was produced on.
    """

    bar_index: int
    observed_at: int
    available_at: int
    value: float


@dataclass(frozen=True)
class FeatureValueAt:
    """
    One feature value as a decision instant sees it.
    """

    alias: str
    feature_id: str
    definition_digest: str
    value: float
    observed_at: int
    available_at: int
    units: str
    member: bool


@dataclass(frozen=True)
class FeatureSnapshot:
    """
    The feature values available to one instrument at one decision instant.

    An alias that is absent was not available, which is a different statement from a value of zero.

    Parameters
    ----------
    ts_event : int
        The decision instant the snapshot was taken at.
    values : tuple[FeatureValueAt, ...]
        The values available, in the order the specification declared them.

    """

    ts_event: int
    values: tuple[FeatureValueAt, ...]

    def get(self, alias: str) -> FeatureValueAt | None:
        """
        Return the value for an alias, or nothing when it was not available.

        Parameters
        ----------
        alias : str
            The declared alias.

        Returns
        -------
        FeatureValueAt or None

        """
        for value in self.values:
            if value.alias == alias:
                return value

        return None

    def aliases(self) -> tuple[str, ...]:
        """
        Return the aliases the snapshot carries.

        Returns
        -------
        tuple[str, ...]

        """
        return tuple(value.alias for value in self.values)

    def __contains__(self, alias: object) -> bool:
        """
        Return whether the snapshot carries an alias.
        """
        return isinstance(alias, str) and self.get(alias) is not None


class FeatureStream:
    """
    The decision-time contract for one instrument: the values a running strategy may read.

    The stream drives one indicator per declared feature, and it serves the reading the declaration
    asked for rather than whichever reading happens to be newest: a declaration's skip selects how
    many observations back the decision is made. A reading is served only once the lookback is
    satisfied and its availability has passed, so the value a strategy reads is one it could have
    known.

    Parameters
    ----------
    declarations : Iterable[FeatureDeclaration]
        The features the specification declared.
    registry : FeatureRegistry
        The registry the identifiers are resolved against.
    memberships : MembershipSeries
        The point-in-time membership series the instrument's membership is read from.
    instrument_id : InstrumentId
        The instrument the stream belongs to.

    Raises
    ------
    ValueError
        If a declared feature is not registered.

    """

    def __init__(
        self,
        declarations: Iterable[FeatureDeclaration],
        registry: FeatureRegistry,
        memberships: MembershipSeries,
        instrument_id: InstrumentId,
    ) -> None:
        """
        Initialize the stream with one indicator per declared feature.
        """
        self._instrument_id = instrument_id
        self._memberships = memberships
        self._declarations: list[FeatureDeclaration] = []
        self._definitions: list[FeatureDefinition] = []
        self._indicators: list[DrivenIndicator] = []
        self._readings: list[list[_StreamedReading]] = []
        self._bar_index = -1

        for declaration in declarations:
            definition = registry.resolve(declaration.feature_id)
            if definition is None:
                raise ValueError(
                    f"declared feature {declaration.feature_id} is not registered, "
                    f"the registry holds {len(registry)} definitions",
                )
            self._declarations.append(declaration)
            self._definitions.append(definition)
            self._indicators.append(_build_indicator(definition.operator, definition.lookback))
            self._readings.append([])

    def update(self, bar: Bar) -> None:
        """
        Update every indicator with one bar.

        Parameters
        ----------
        bar : Bar
            The bar that arrived.

        """
        self._bar_index += 1

        for index, indicator in enumerate(self._indicators):
            indicator.handle_bar(bar)
            definition = self._definitions[index]
            self._readings[index].append(
                _StreamedReading(
                    bar_index=self._bar_index,
                    observed_at=bar.ts_event,
                    available_at=bar.ts_event + definition.availability_lag_ns,
                    value=indicator.value,
                ),
            )

    def snapshot(self, decision_at: int) -> FeatureSnapshot:
        """
        Return the values available at a decision instant.

        Parameters
        ----------
        decision_at : int
            The event timestamp of the bar the decision is made on.

        Returns
        -------
        FeatureSnapshot

        """
        member = bool(self._memberships.is_member_at(self._instrument_id, decision_at))
        values: list[FeatureValueAt] = []

        for index, declaration in enumerate(self._declarations):
            definition = self._definitions[index]
            first_valid = definition.lookback - 1
            valid = [
                reading for reading in self._readings[index] if reading.bar_index >= first_valid
            ]
            available = [reading for reading in valid if reading.available_at <= decision_at]

            # Readings this decision cannot use are kept only while they can still be used later:
            # one that is already available and no longer within the declared skip is dropped, and
            # one whose availability has not passed is retained.
            self._readings[index] = [
                reading for reading in valid if reading.available_at > decision_at
            ] + available[-(declaration.skip + 1) :]

            if len(available) < declaration.skip + 1:
                continue

            reading = available[-(declaration.skip + 1)]
            values.append(
                FeatureValueAt(
                    alias=declaration.alias,
                    feature_id=definition.feature_id,
                    definition_digest=definition.digest(),
                    value=reading.value,
                    observed_at=reading.observed_at,
                    available_at=reading.available_at,
                    units=definition.units,
                    member=member,
                ),
            )

        return FeatureSnapshot(ts_event=decision_at, values=tuple(values))


def build_memberships(
    universe: str,
    source: str,
    intervals: Iterable[tuple[str, int, int | None]],
) -> MembershipSeries:
    """
    Build a point-in-time membership series from declared intervals.

    Parameters
    ----------
    universe : str
        The universe the series describes.
    source : str
        The provenance of the membership declaration.
    intervals : Iterable[tuple[str, int, int or None]]
        The `(instrument_id, entered_at, exited_at)` intervals to declare.

    Returns
    -------
    MembershipSeries

    """
    series = MembershipSeries(universe, source)

    for instrument_id, entered_at, exited_at in intervals:
        series.push(
            MembershipInterval(
                universe,
                source,
                InstrumentId.from_str(instrument_id),
                entered_at=entered_at,
                exited_at=exited_at,
            ),
        )

    return series


def build_panel(
    declarations: Iterable[FeatureDeclaration],
    registry: FeatureRegistry,
    bars_by_instrument: Mapping[str, Sequence[Bar]],
    memberships: MembershipSeries,
) -> Panel:
    """
    Build the point-in-time panel the offline side of the parity fixture reads.

    One row is written per instrument and per bar that produced a reading, carrying every declared
    alias's value at that observation and the instrument's membership at that instant. A row is
    never written for a bar that produced no reading, so the panel cannot manufacture a warm-up
    value.

    Parameters
    ----------
    declarations : Iterable[FeatureDeclaration]
        The features the specification declared.
    registry : FeatureRegistry
        The registry the identifiers are resolved against.
    bars_by_instrument : Mapping[str, Sequence[Bar]]
        The bars to compute over, by instrument identifier.
    memberships : MembershipSeries
        The point-in-time membership series.

    Returns
    -------
    Panel

    Raises
    ------
    ValueError
        If a declared feature is not registered.

    """
    resolved: list[tuple[FeatureDeclaration, list[FeatureObservation], str]] = []

    for declaration in declarations:
        definition = registry.resolve(declaration.feature_id)
        if definition is None:
            raise ValueError(f"declared feature {declaration.feature_id} is not registered")
        for instrument_id, bars in bars_by_instrument.items():
            observations = compute_observations(definition, list(bars))
            resolved.append((declaration, observations, instrument_id))

    rows: list[PanelRow] = []

    for declaration, observations, instrument_id in resolved:
        instrument = InstrumentId.from_str(instrument_id)
        for observation in observations:
            row = PanelRow(instrument, observation.observed_at)
            row = row.with_feature(declaration.alias, observation.value, observation.observed_at)
            row = row.with_member(
                bool(memberships.is_member_at(instrument, observation.observed_at)),
            )
            rows.append(row)

    return Panel(memberships, rows)
