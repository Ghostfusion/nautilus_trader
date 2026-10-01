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
Label policies for the target path, with the alignment convention as part of the definition.

A label is a future outcome, so every construction in this module reads observations the feature
path cannot see. The stratification is therefore part of the contract rather than a coding
convention: a label series is a distinct type, it is refused where market data is expected, and the
module is not reachable from the live path, so a label value cannot quietly become a feature.

A definition states the outcome, the forward window, and how the window is aligned to the feature
rows. The alignment convention is first class because pairing a feature row with the label anchored
at that row and pairing it with the label anchored at the next row produce datasets that look
equivalent, since the second is the first shifted by one row, while differing in what the outcome
covers and in how far into the future the label reaches. The reach is the quantity a leakage policy
has to cover: it is measured from the produced series rather than derived from the definition, so it
stays correct when missing observations make a window cover more time than its bar count suggests.

The first tranche is a fixed-horizon forward return, the forward aggregates of the window's per-bar
returns, and a first-hit label reporting which of two independent thresholds the cumulative return
from the entry breached first. The extrema and trend-state labels of the second tranche need the
dataset contract that the earlier review specified, and are not reachable here.

The mechanisms are re-derived from our own requirements: the tranche is specified in
`docs/design/vectorbt_lessons_design.md` section 9 L6, and no code, fixture or documentation of the
source library is used.
"""

from __future__ import annotations

from bisect import bisect_left
from dataclasses import dataclass
from enum import Enum
from enum import unique
from math import isfinite
from math import isnan
from statistics import fmean
from statistics import stdev
from typing import TYPE_CHECKING
from typing import cast

from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Sequence

    from nautilus_trader.optimization.space import JsonValue
    from nautilus_trader.optimization.splits import LeakagePolicy

# A standard deviation needs two observations to be defined, so a horizon below it could never
# produce a finite value and is refused at the definition rather than answered with a missing label.
_MINIMUM_DISPERSION_OBSERVATIONS = 2

# The first-hit label reports which threshold the cumulative return from the entry breached first.
_FIRST_HIT_POSITIVE = 1.0
_FIRST_HIT_NEGATIVE = -1.0
_FIRST_HIT_NONE = 0.0

# A bar series row carries a timestamp and a close.
_ROW_FIELDS = 2


@unique
class AlignmentConvention(Enum):
    """
    Which label row a feature row is paired with.

    `SIGNAL_BAR` pairs a feature row with the label anchored at the same bar, so the decision is
    entered at that bar's close and the outcome is measured from there. `NEXT_BAR` pairs it with the
    label anchored at the following bar, so the entry is one bar later.

    The two are distinguishable in two ways: the label values are the same array shifted by one row,
    and the second reaches one bar further beyond its feature row, so the exclusion interval a split
    must apply is larger.

    """

    SIGNAL_BAR = "signal_bar"
    NEXT_BAR = "next_bar"


@unique
class MissingDataPolicy(Enum):
    """
    How a label window treats an observation whose close is missing.

    `PROPAGATE` requires every observation of the window to be present, so a window containing a
    missing close has no label at all. `SKIP` measures the window over the present observations, so
    the horizon counts observations rather than bars, which makes the window cover more time.

    """

    PROPAGATE = "propagate"
    SKIP = "skip"


@unique
class LabelKind(Enum):
    """
    The outcome a label definition measures over its forward window.
    """

    FORWARD_RETURN = "forward_return"
    FORWARD_AGGREGATE = "forward_aggregate"
    FIRST_HIT_THRESHOLD = "first_hit_threshold"


@unique
class ForwardAggregate(Enum):
    """
    The statistic a `LabelKind.FORWARD_AGGREGATE` label reduces its window with.

    The window is reduced as per-bar returns rather than as endpoint returns, so the mean is the
    average of one-bar returns, the standard deviation is their dispersion with the sample divisor,
    and the minimum and maximum are the worst and best single bars, not the worst and best holding
    period.
    """

    MEAN = "mean"
    STD = "std"
    MIN = "min"
    MAX = "max"


def _optional_number(value: object, where: str) -> float | None:
    """
    Require a finite number, or None when it is unset.
    """
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError(f"{where} must be a number or None, was {value!r}")
    number = float(value)
    if not isfinite(number):
        raise ValueError(f"{where} must be finite, was {value!r}")

    return number


@dataclass(frozen=True)
class LabelDefinition:
    """
    A label policy: the outcome it measures, the window it measures it over, and its alignment.

    The window of a label anchored at a bar is the `horizon` observations that follow the `wait`
    observations after that bar, and its entry reference is the close of the `wait`-th observation
    after the anchor. A zero wait therefore enters at the anchor's own close while still measuring
    the outcome over the bars after it, because the wait excludes the anchor bar from the window
    without excluding its close from the entry.

    Parameters
    ----------
    label_id : str
        The identifier of the policy, unique within a study.
    kind : LabelKind
        The outcome the policy measures.
    horizon : int
        The forward window length in observations. Positive.
    wait : int, default 0
        The observations to wait before the entry. Non-negative.
    alignment : AlignmentConvention, default AlignmentConvention.SIGNAL_BAR
        Which label row a feature row is paired with.
    missing_data_policy : MissingDataPolicy, default MissingDataPolicy.PROPAGATE
        How the window treats an observation whose close is missing.
    aggregate : ForwardAggregate | None, default None
        The reduction for `LabelKind.FORWARD_AGGREGATE`. Required by that kind and refused by the
        others, because a field set but never applied is not a policy.
    positive_threshold : float | None, default None
        The positive cumulative return that `LabelKind.FIRST_HIT_THRESHOLD` reports when it is
        breached first. Required by that kind, strictly positive, and refused by the others.
    negative_threshold : float | None, default None
        The negative cumulative return for the same kind. Required by that kind, strictly negative,
        and refused by the others. It is independent of the positive threshold rather than its
        negation, because the two barriers are separate decisions.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If a field is out of range, or is set for a kind that does not use it.

    """

    label_id: str
    kind: LabelKind
    horizon: int
    wait: int = 0
    alignment: AlignmentConvention = AlignmentConvention.SIGNAL_BAR
    missing_data_policy: MissingDataPolicy = MissingDataPolicy.PROPAGATE
    aggregate: ForwardAggregate | None = None
    positive_threshold: float | None = None
    negative_threshold: float | None = None

    def __post_init__(self) -> None:
        """
        Validate the definition.
        """
        self._validate_common()
        self._validate_kind_fields()

    def _validate_common(self) -> None:
        """
        Validate the fields every kind carries.
        """
        if not isinstance(self.label_id, str) or not self.label_id.strip():
            raise ValueError(f"label_id must be a non-empty string, was {self.label_id!r}")
        if not isinstance(self.kind, LabelKind):
            raise TypeError(f"kind must be a LabelKind, was {self.kind!r}")
        if isinstance(self.horizon, bool) or not isinstance(self.horizon, int):
            raise TypeError(f"horizon must be an int, was {self.horizon!r}")
        if self.horizon < 1:
            raise ValueError(f"horizon must be at least 1 observation, was {self.horizon}")
        if isinstance(self.wait, bool) or not isinstance(self.wait, int):
            raise TypeError(f"wait must be an int, was {self.wait!r}")
        if self.wait < 0:
            raise ValueError(f"wait must not be negative, was {self.wait}")
        if not isinstance(self.alignment, AlignmentConvention):
            raise TypeError(f"alignment must be an AlignmentConvention, was {self.alignment!r}")
        if not isinstance(self.missing_data_policy, MissingDataPolicy):
            raise TypeError(
                "missing_data_policy must be a MissingDataPolicy, "
                f"was {self.missing_data_policy!r}",
            )

    def _validate_kind_fields(self) -> None:
        """
        Validate the fields that only one kind uses.
        """
        aggregate = self.aggregate
        if aggregate is not None and not isinstance(aggregate, ForwardAggregate):
            raise TypeError(f"aggregate must be a ForwardAggregate or None, was {aggregate!r}")

        positive = _optional_number(self.positive_threshold, "positive_threshold")
        negative = _optional_number(self.negative_threshold, "negative_threshold")

        if self.kind is LabelKind.FORWARD_AGGREGATE:
            self._validate_aggregate(aggregate)
        elif aggregate is not None:
            raise ValueError(
                f"aggregate is only applied by forward_aggregate, not {self.kind.value}",
            )

        if self.kind is LabelKind.FIRST_HIT_THRESHOLD:
            self._validate_thresholds(positive, negative)
        elif positive is not None or negative is not None:
            raise ValueError(
                f"thresholds are only applied by first_hit_threshold, not {self.kind.value}",
            )

    def _validate_aggregate(self, aggregate: ForwardAggregate | None) -> None:
        """
        Validate the reduction of a forward-aggregate label.
        """
        if aggregate is None:
            raise ValueError("forward_aggregate requires an aggregate")
        if aggregate is ForwardAggregate.STD and self.horizon < _MINIMUM_DISPERSION_OBSERVATIONS:
            raise ValueError(
                f"aggregate 'std' requires a horizon of at least "
                f"{_MINIMUM_DISPERSION_OBSERVATIONS} observations, was {self.horizon}",
            )

    def _validate_thresholds(self, positive: float | None, negative: float | None) -> None:
        """
        Validate the two thresholds of a first-hit label.
        """
        if positive is None or negative is None:
            raise ValueError("first_hit_threshold requires both thresholds")
        if positive <= 0.0:
            raise ValueError(f"positive_threshold must be strictly positive, was {positive}")
        if negative >= 0.0:
            raise ValueError(f"negative_threshold must be strictly negative, was {negative}")

    def __str__(self) -> str:
        """
        Return a human-readable representation.
        """
        return f"LabelDefinition({self.label_id}, {self.kind.value}, horizon={self.horizon})"

    @property
    def anchor_offset(self) -> int:
        """
        Return the offset in observations from a feature row to its label's anchor.
        """
        return 1 if self.alignment is AlignmentConvention.NEXT_BAR else 0

    @property
    def entry_offset(self) -> int:
        """
        Return the offset in observations from a feature row to its entry reference bar.
        """
        return self.anchor_offset + self.wait

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the definition over every field.
        """
        return digest_of(self.to_dict())

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the definition as a canonical-friendly mapping.
        """
        return {
            "label_id": self.label_id,
            "kind": self.kind.value,
            "horizon": self.horizon,
            "wait": self.wait,
            "alignment": self.alignment.value,
            "missing_data_policy": self.missing_data_policy.value,
            "aggregate": self.aggregate.value if self.aggregate is not None else None,
            "positive_threshold": self.positive_threshold,
            "negative_threshold": self.negative_threshold,
        }


@dataclass(frozen=True)
class LabelSeries:
    """
    The label values of one definition over one bar series, aligned to the feature rows.

    There is one entry per row of the input series, `None` where the window is not complete, so the
    labels share a row grid with the features. `forward_reach_ns` is the largest distance from a
    feature row to the last observation its label reads, over the computed rows; it is the quantity
    a leakage policy has to cover, and it is measured rather than derived so it accounts for the
    alignment convention, for the wait and for a missing-data policy that extends a window.

    Parameters
    ----------
    definition : LabelDefinition
        The policy the values were computed with.
    timestamps : tuple[int, ...]
        The feature rows in nanoseconds, strictly increasing.
    values : tuple[float | None, ...]
        The label of each row, or `None` where the window is not complete.
    forward_reach_ns : int
        The largest forward reach in nanoseconds of any computed label, zero when none was.

    """

    definition: LabelDefinition
    timestamps: tuple[int, ...]
    values: tuple[float | None, ...]
    forward_reach_ns: int

    def __len__(self) -> int:
        """
        Return the number of feature rows.
        """
        return len(self.timestamps)

    @property
    def computed(self) -> int:
        """
        Return the number of rows that carry a label.
        """
        return sum(1 for value in self.values if value is not None)

    def value_at(self, timestamp: int) -> float | None:
        """
        Return the label of the given feature row.

        Raises
        ------
        KeyError
            If the timestamp is not a feature row of this series.

        """
        index = bisect_left(self.timestamps, timestamp)
        if index == len(self.timestamps) or self.timestamps[index] != timestamp:
            raise KeyError(f"{timestamp} is not a feature row of this label series")

        return self.values[index]

    def leakage_shortfall_ns(self, policy: LeakagePolicy) -> int:
        """
        Return how much of the label's forward reach the leakage policy does not cover.

        The shortfall is the number a study has to act on, so it is returned rather than only
        reported: a positive value is the amount the effective purge has to grow by. When the
        leakage policy enforces a label horizon, the horizon is already folded into its purge.

        """
        return max(0, self.forward_reach_ns - policy.purge)

    def validate_leakage(self, policy: LeakagePolicy) -> None:
        """
        Refuse a leakage policy that does not cover the label's forward reach.

        A training row whose label reaches into the evaluation set carries evaluation information,
        so the policy's effective purge before the evaluation set has to cover the reach rather
        than some chosen distance. The intervals themselves remain a study decision: this checks the
        decision, it does not supply one.

        Raises
        ------
        ValueError
            If the effective purge is shorter than the label's forward reach.

        """
        shortfall = self.leakage_shortfall_ns(policy)
        if shortfall <= 0:
            return

        raise ValueError(
            f"label {self.definition.label_id!r} reaches {self.forward_reach_ns} ns beyond its "
            f"feature row while the leakage policy purges {policy.purge} ns, so a training row "
            f"within {shortfall} ns of the evaluation set would carry evaluation information; "
            "cover the reach with purge_before or with an enforced label horizon",
        )


def _reject_label_input(series: object) -> None:
    """
    Refuse a label series offered as market data.

    A label is an outcome of the future, so reading one as an input to a feature reads the future
    into the feature path. The two paths are separate types and the mistake is refused by name.

    Raises
    ------
    TypeError
        If the value is a `LabelSeries`.

    """
    if isinstance(series, LabelSeries):
        raise TypeError(
            "a LabelSeries cannot be read as market data: a label is a future outcome, so feeding "
            "it into a feature reads the future; labels and features are separate paths",
        )


def _close(value: object) -> float | None:
    """
    Return a close as a float, None when it is missing.

    A `None` or a NaN close is a missing observation. An infinity is neither missing nor usable, so
    it is refused rather than reinterpreted as one of the two.
    """
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError(f"a close must be a number or None, was {value!r}")
    close = float(value)
    if isnan(close):
        return None
    if not isfinite(close):
        raise ValueError(
            f"a non-finite close is refused rather than treated as missing, was {value!r}",
        )

    return close


def _prepare(
    series: Sequence[tuple[int, float | None]],
) -> tuple[tuple[int, ...], tuple[float | None, ...]]:
    """
    Return the timestamps and closes of a bar series, validated.

    Raises
    ------
    TypeError
        If a row is not a pair, or a timestamp or close has the wrong type.
    ValueError
        If timestamps do not strictly increase, or a close is infinite.

    """
    timestamps: list[int] = []
    closes: list[float | None] = []

    for row in series:
        if not isinstance(row, tuple) or len(row) != _ROW_FIELDS:
            raise TypeError(f"a bar series row must be a (timestamp, close) pair, was {row!r}")
        timestamp, raw = row
        if isinstance(timestamp, bool) or not isinstance(timestamp, int):
            raise TypeError(f"a bar timestamp must be an int, was {timestamp!r}")
        if timestamps and timestamp <= timestamps[-1]:
            raise ValueError(
                f"bar timestamps must strictly increase, was {timestamp} after {timestamps[-1]}",
            )
        timestamps.append(timestamp)
        closes.append(_close(raw))

    return tuple(timestamps), tuple(closes)


def _window(
    closes: tuple[float | None, ...],
    anchor: int,
    definition: LabelDefinition,
) -> tuple[float, tuple[float, ...], tuple[int, ...]] | None:
    """
    Return the entry close and the window closes and indices, or None when the window is incomplete.
    """
    count = len(closes)
    start = anchor + definition.wait
    if definition.missing_data_policy is MissingDataPolicy.PROPAGATE:
        return _contiguous_window(closes, start, count, definition.horizon)

    return _present_window(closes, start, count, definition.horizon)


def _contiguous_window(
    closes: tuple[float | None, ...],
    start: int,
    count: int,
    horizon: int,
) -> tuple[float, tuple[float, ...], tuple[int, ...]] | None:
    """
    Return a window of consecutive observations, or None when one is missing or out of range.
    """
    end = start + horizon
    if start < 0 or end >= count:
        return None
    if any(closes[index] is None for index in range(start, end + 1)):
        return None

    indices = tuple(range(start + 1, end + 1))
    values = tuple(cast("float", closes[index]) for index in indices)

    return cast("float", closes[start]), values, indices


def _present_window(
    closes: tuple[float | None, ...],
    start: int,
    count: int,
    horizon: int,
) -> tuple[float, tuple[float, ...], tuple[int, ...]] | None:
    """
    Return a window of present observations, or None when fewer than the horizon remain.
    """
    entry = -1
    for index in range(max(start, 0), count):
        if closes[index] is not None:
            entry = index
            break
    if entry < 0:
        return None

    indices: list[int] = []
    index = entry + 1
    while index < count and len(indices) < horizon:
        if closes[index] is not None:
            indices.append(index)
        index += 1
    if len(indices) < horizon:
        return None

    return (
        cast("float", closes[entry]),
        tuple(cast("float", closes[index]) for index in indices),
        tuple(indices),
    )


def _ratio(numerator: float, denominator: float) -> float:
    """
    Return the simple return of a close against another.
    """
    return numerator / denominator - 1.0


def _reduce(returns: tuple[float, ...], aggregate: ForwardAggregate) -> float:
    """
    Reduce the per-bar returns of a window.

    The mean uses a compensated accumulation, and the standard deviation is the sample one, so the
    divisor matches the dispersion kernel of the analysis crate rather than the population.
    """
    if aggregate is ForwardAggregate.MEAN:
        return fmean(returns)
    if aggregate is ForwardAggregate.STD:
        return stdev(returns)
    if aggregate is ForwardAggregate.MIN:
        return min(returns)

    return max(returns)


def _per_bar_returns(entry_close: float, window: tuple[float, ...]) -> tuple[float, ...]:
    """
    Return the return of every window observation against the one before it.
    """
    returns: list[float] = []
    previous = entry_close
    for close in window:
        returns.append(_ratio(close, previous))
        previous = close

    return tuple(returns)


def _first_hit(
    entry_close: float,
    window: tuple[float, ...],
    indices: tuple[int, ...],
    definition: LabelDefinition,
) -> tuple[float, int]:
    """
    Return the first breached threshold of a window, and the index where it was breached.

    The barriers are on the cumulative return from the entry, as the barriers of an open trade are,
    so the label reports which one an entry would have reached first. The scan is over closes rather
    than over highs and lows, so one observation cannot breach both barriers and the first hit is
    total; an intrabar comparison would need the bar ordering policy that this layer does not own.
    """
    positive = cast("float", definition.positive_threshold)
    negative = cast("float", definition.negative_threshold)

    for offset, close in enumerate(window):
        cumulative = _ratio(close, entry_close)
        if cumulative >= positive:
            return _FIRST_HIT_POSITIVE, indices[offset]
        if cumulative <= negative:
            return _FIRST_HIT_NEGATIVE, indices[offset]

    return _FIRST_HIT_NONE, indices[-1]


def _evaluate(
    entry_close: float,
    window: tuple[float, ...],
    indices: tuple[int, ...],
    definition: LabelDefinition,
) -> tuple[float, int]:
    """
    Return the label value of a window and the index of the last observation it reads.
    """
    if definition.kind is LabelKind.FIRST_HIT_THRESHOLD:
        return _first_hit(entry_close, window, indices, definition)
    if definition.kind is LabelKind.FORWARD_RETURN:
        return _ratio(window[-1], entry_close), indices[-1]

    returns = _per_bar_returns(entry_close, window)
    aggregate = cast("ForwardAggregate", definition.aggregate)

    return _reduce(returns, aggregate), indices[-1]


def label_series(
    definition: LabelDefinition,
    series: Sequence[tuple[int, float | None]],
) -> LabelSeries:
    """
    Compute the label values of the definition over a bar series, aligned to its rows.

    The series is a sequence of `(timestamp_ns, close)` pairs in strictly increasing order. A close
    of `None` or NaN is a missing observation; an infinity is refused, so a missing value is never
    read as data.

    Parameters
    ----------
    definition : LabelDefinition
        The policy to compute.
    series : Sequence[tuple[int, float | None]]
        The bar closes, in nanoseconds and price.

    Returns
    -------
    LabelSeries
        One value per row, `None` where the window is not complete.

    Raises
    ------
    TypeError
        If the series is a `LabelSeries`, a row is not a pair, or a value has the wrong type.
    ValueError
        If timestamps do not strictly increase, or a close is infinite.

    """
    _reject_label_input(series)
    timestamps, closes = _prepare(series)
    anchor_offset = definition.anchor_offset
    values: list[float | None] = []
    reach = 0

    for row, timestamp in enumerate(timestamps):
        window = _window(closes, row + anchor_offset, definition)
        if window is None:
            values.append(None)
            continue

        entry_close, closes_window, indices = window
        value, last = _evaluate(entry_close, closes_window, indices, definition)
        values.append(value)
        reach = max(reach, timestamps[last] - timestamp)

    return LabelSeries(
        definition=definition,
        timestamps=timestamps,
        values=tuple(values),
        forward_reach_ns=reach,
    )
