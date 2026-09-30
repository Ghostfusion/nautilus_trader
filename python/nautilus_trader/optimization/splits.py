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
Split contracts for validation, and the leakage exclusion relation.

A split contract cuts a period into consecutive windows and divides each window between named sets,
yielding the bounds of every set. It owns the two things that must not be improvised per study:
where each window's sets begin and end, and which observations the evaluation information excludes
from training.

Leakage is expressed as an exclusion relation rather than a time distance:

- `purge_before` trims the end of a set that precedes the evaluation set, so a training observation
  whose information reaches into the evaluation window is removed.
- `purge_after` moves the start of a set that follows the evaluation set, for the same reason in the
  other direction.
- `embargo_after` is the minimum gap between one split and the next, so consecutive splits do not
  share information through serial dependence.
- `label_overlap_rule` decides whether a declared label horizon is folded into the purge. Labels
  look forward, so a training observation within one horizon of the evaluation start carries
  evaluation information whatever the distance between the sets; with `LabelOverlapRule.ENFORCE` the
  purge is derived from the horizon rather than guessed.

The concept is mandatory and the values are not. A zero interval is permitted when the policy says
why it is zero: an interval left unset (`None`) and an interval decided to be zero (`0`) are
distinguishable, and any zero interval requires a justification, so a zero exclusion can never be
silent.

Bounds are half-open nanosecond intervals over the run period, matching the walk-forward windows of
`nautilus_trader.optimization.stages`.
"""

from __future__ import annotations

from bisect import bisect_left
from dataclasses import dataclass
from enum import Enum
from enum import unique
from typing import TYPE_CHECKING

from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.optimization.space import JsonValue


type SetLength = int | float
type SetBounds = tuple[int, int]

# The leakage intervals a policy declares, in the order they are reported.
_INTERVAL_FIELDS = ("purge_before", "purge_after", "embargo_after")

# Fractional lengths round to the nanosecond; this is the tolerance for fractions that account for a
# whole window, where the last set absorbs the rounding difference.
_FRACTION_TOLERANCE = 1e-9


def _require_ns(value: object, where: str) -> int | None:
    """
    Require a non-negative nanosecond interval, or None when it is unset.
    """
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"{where} must be nanoseconds as an int, was {value!r}")
    if value < 0:
        raise ValueError(f"{where} must not be negative, was {value}")

    return value


@unique
class LabelOverlapRule(Enum):
    """
    How a split contract accounts for labels that reach across a split boundary.

    `NONE` declares that the study's labels do not cross a boundary, which leaves the exclusion to
    the declared intervals. `ENFORCE` declares a label horizon and folds it into the purge before
    the evaluation set, so an overlapping training observation is excluded by construction.

    """

    NONE = "none"
    ENFORCE = "enforce"


@unique
class SplitDirection(Enum):
    """
    How a split contract lays out windows over the period, and where a leftover span goes.

    `EXACT` lays out as many whole windows as fit and drops any leftover span, `FORWARD` extends
    the last set of the last window by the leftover span, and `REVERSED` anchors the layout at the
    period end and extends the first set of the first window by the leftover span.

    """

    EXACT = "exact"
    FORWARD = "forward"
    REVERSED = "reversed"


@dataclass(frozen=True)
class LeakagePolicy:
    """
    The exclusion relation between a split's evaluation set and its training sets.

    An interval of `None` is unset and an interval of `0` is decided to be zero, so the two are
    distinguishable by value and by digest. Any zero interval requires
    `zero_interval_justification`; a justification without a zero interval is refused, because it
    records a decision that was not taken.

    Parameters
    ----------
    purge_before : int | None, default None
        The interval before the evaluation set, in nanoseconds, removed from a preceding set.
    purge_after : int | None, default None
        The interval after the evaluation set, in nanoseconds, removed from a following set.
    embargo_after : int | None, default None
        The minimum gap in nanoseconds between one split and the next.
    label_overlap_rule : LabelOverlapRule, default LabelOverlapRule.NONE
        Whether a declared label horizon is folded into `purge_before`.
    label_horizon : int | None, default None
        The label horizon in nanoseconds, required and only allowed with
        `LabelOverlapRule.ENFORCE`.
    zero_interval_justification : str | None, default None
        Why a zero interval is zero. Required when an interval is unset or zero.

    """

    purge_before: int | None = None
    purge_after: int | None = None
    embargo_after: int | None = None
    label_overlap_rule: LabelOverlapRule = LabelOverlapRule.NONE
    label_horizon: int | None = None
    zero_interval_justification: str | None = None

    def __post_init__(self) -> None:
        """
        Validate the policy, refusing an interval that is zero without a justification.
        """
        for field_name in _INTERVAL_FIELDS:
            _require_ns(getattr(self, field_name), field_name)
        self._validate_label_rule()
        self._validate_justification()

    def _validate_label_rule(self) -> None:
        """
        Validate that the label rule and the label horizon agree.
        """
        if not isinstance(self.label_overlap_rule, LabelOverlapRule):
            raise TypeError(
                f"label_overlap_rule must be a LabelOverlapRule, was {self.label_overlap_rule!r}",
            )
        if self.label_overlap_rule is LabelOverlapRule.ENFORCE:
            if _require_ns(self.label_horizon, "label_horizon") in (None, 0):
                raise ValueError("label_overlap_rule 'enforce' requires a positive label_horizon")

            return
        if self.label_horizon is not None:
            raise ValueError(
                "label_horizon requires label_overlap_rule 'enforce', because a horizon that is "
                "declared but never applied is not a policy",
            )

    def _validate_justification(self) -> None:
        """
        Validate that every zero interval is justified, and a justification has a zero to state.
        """
        zeros = [name for name in _INTERVAL_FIELDS if getattr(self, name) in (None, 0)]
        justified = bool((self.zero_interval_justification or "").strip())
        if zeros and not justified:
            raise ValueError(f"zero intervals {zeros} require a zero_interval_justification")
        if not zeros and justified:
            raise ValueError("zero_interval_justification is set while every interval is positive")

    @property
    def purge(self) -> int:
        """
        Return the effective purge before the evaluation set, with the label horizon folded in.
        """
        purge = self.purge_before or 0
        if self.label_overlap_rule is LabelOverlapRule.ENFORCE:
            purge = max(purge, self.label_horizon or 0)

        return purge

    @property
    def after(self) -> int:
        """
        Return the effective purge after the evaluation set.
        """
        return self.purge_after or 0

    @property
    def gap(self) -> int:
        """
        Return the effective minimum gap between one split and the next.
        """
        return self.embargo_after or 0

    @property
    def decided(self) -> frozenset[str]:
        """
        Return the intervals the policy decided, as opposed to left unset.
        """
        return frozenset(name for name in _INTERVAL_FIELDS if getattr(self, name) is not None)

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the policy, over its values and its decided intervals.
        """
        return digest_of(self.to_dict())

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the policy as a canonical-friendly mapping.
        """
        return {
            "purge_before": self.purge_before,
            "purge_after": self.purge_after,
            "embargo_after": self.embargo_after,
            "label_overlap_rule": self.label_overlap_rule.value,
            "label_horizon": self.label_horizon,
            "zero_interval_justification": self.zero_interval_justification,
        }


@dataclass(frozen=True)
class Split:
    """
    One split: the half-open nanosecond bounds of each of its sets.

    Parameters
    ----------
    bounds : Mapping[str, SetBounds]
        The `(start, stop)` bounds of every set, keyed by set name, in set order.

    """

    bounds: Mapping[str, SetBounds]

    def lengths(self) -> dict[str, int]:
        """
        Return the length of every set in nanoseconds.
        """
        return {name: stop - start for name, (start, stop) in self.bounds.items()}

    def indices(self, timestamps: Sequence[int]) -> dict[str, tuple[int, ...]]:
        """
        Return the observation positions of every set for a non-decreasing timestamp sequence.

        The positions are the indices of the timestamps inside each set's half-open bounds, so a
        caller can subset its own data without repeating the exclusion arithmetic.

        Parameters
        ----------
        timestamps : Sequence[int]
            The observation timestamps in Unix nanoseconds, in non-decreasing order.

        Returns
        -------
        dict[str, tuple[int, ...]]

        """
        return {
            name: tuple(range(bisect_left(timestamps, start), bisect_left(timestamps, stop)))
            for name, (start, stop) in self.bounds.items()
        }


@dataclass(frozen=True)
class SplitContract:
    """
    A reusable split contract: the named sets, their lengths, and the leakage exclusion relation.

    A set length is nanoseconds as an int, a fraction of the window in (0, 1) as a float, or `None`
    for exactly one set, which absorbs the rest of the window. Fractions and an omitted length
    require an explicit `window`; with every length absolute the window is their sum.

    Parameters
    ----------
    sets : tuple[str, ...]
        The ordered set names. Non-empty and unique.
    lengths : Mapping[str, SetLength | None]
        The length of every named set.
    leakage : LeakagePolicy
        The exclusion relation. Mandatory: a contract without one is not a contract.
    evaluation : str | None, default None
        The set whose information the other sets may not overlap. Defaults to the last named set.
    window : int | None, default None
        The window length in nanoseconds, required when a length is fractional or omitted.
    stride : int | None, default None
        The distance in nanoseconds between consecutive window starts. Defaults to the window
        length, so consecutive windows are adjacent; a smaller stride overlaps them.
    min_length : int | None, default None
        The minimum length in nanoseconds of every set in a split. A shorter split is dropped.
    n_splits : int | None, default None
        The number of splits to select. More whole splits than this selects this many evenly spaced
        splits; fewer raises.
    direction : SplitDirection, default SplitDirection.EXACT
        Where the leftover span of the period goes.

    """

    sets: tuple[str, ...]
    lengths: Mapping[str, SetLength | None]
    leakage: LeakagePolicy
    evaluation: str | None = None
    window: int | None = None
    stride: int | None = None
    min_length: int | None = None
    n_splits: int | None = None
    direction: SplitDirection = SplitDirection.EXACT

    def __post_init__(self) -> None:
        """
        Validate the contract.
        """
        self._validate_sets()
        self._validate_declared_lengths()
        self._validate_exclusions()
        self._validate_layout()

    def _validate_sets(self) -> None:
        """
        Validate that the set names are non-empty and unique.
        """
        if not self.sets:
            raise ValueError("a split contract requires at least one set")
        if any(not name for name in self.sets):
            raise ValueError("set names must be non-empty")
        if len(set(self.sets)) != len(self.sets):
            raise ValueError(f"set names must be unique, was {list(self.sets)}")

    def _validate_declared_lengths(self) -> None:
        """
        Validate that every set declares a length, and that at most one omits it.
        """
        unknown = sorted(set(self.lengths) - set(self.sets))
        if unknown:
            raise ValueError(f"lengths declared for unknown sets: {unknown}")
        missing = sorted(set(self.sets) - set(self.lengths))
        if missing:
            raise ValueError(f"lengths missing for sets: {missing}")
        omitted = [name for name in self.sets if self.lengths[name] is None]
        if len(omitted) > 1:
            raise ValueError(f"at most one set length may be omitted, was {omitted}")

    def _validate_exclusions(self) -> None:
        """
        Validate the leakage policy and the evaluation set.
        """
        if not isinstance(self.leakage, LeakagePolicy):
            raise TypeError(f"leakage must be a LeakagePolicy, was {self.leakage!r}")
        if self.evaluation is not None and self.evaluation not in self.sets:
            raise ValueError(f"evaluation set {self.evaluation!r} is not one of {list(self.sets)}")

    def _validate_layout(self) -> None:
        """
        Validate the window, the stride, the minimum length, the split count and the direction.
        """
        for field_name in ("window", "stride", "min_length"):
            value = getattr(self, field_name)
            if value is not None and value <= 0:
                raise ValueError(f"{field_name} must be positive, was {value}")
        if self.n_splits is not None and self.n_splits < 1:
            raise ValueError(f"n_splits must be at least 1, was {self.n_splits}")
        if not isinstance(self.direction, SplitDirection):
            raise TypeError(f"direction must be a SplitDirection, was {self.direction!r}")

    @property
    def evaluation_set(self) -> str:
        """
        Return the set whose information the other sets may not overlap.
        """
        return self.sets[-1] if self.evaluation is None else self.evaluation

    def resolved_lengths(self) -> dict[str, int]:
        """
        Return every set's length in nanoseconds, resolving fractions and the omitted set.

        Returns
        -------
        dict[str, int]

        """
        lengths, fraction_total = self._declared_lengths()
        omitted = [name for name in self.sets if self.lengths[name] is None]
        if omitted:
            lengths[omitted[0]] = self._omitted_length(lengths, omitted[0])
        elif self.window is not None:
            last = self.sets[-1]
            lengths[last] = self._window_remainder(lengths, fraction_total, self.window)

        return {name: lengths[name] for name in self.sets}

    def _declared_lengths(self) -> tuple[dict[str, int], float]:
        """
        Return the declared lengths in nanoseconds, with the total declared fraction.
        """
        lengths: dict[str, int] = {}
        fraction_total = 0.0

        for name in self.sets:
            value = self.lengths[name]
            if value is None:
                continue
            if isinstance(value, float):
                if self.window is None:
                    raise ValueError(
                        f"the fractional length of set {name!r} requires an explicit window",
                    )
                if not 0.0 < value < 1.0:
                    raise ValueError(
                        f"the fractional length of set {name!r} must be in (0, 1), was {value}",
                    )
                lengths[name] = round(value * self.window)
                fraction_total += value
            else:
                if isinstance(value, bool) or not isinstance(value, int):
                    raise TypeError(f"the length of set {name!r} must be an int or a float")
                if value <= 0:
                    raise ValueError(f"the length of set {name!r} must be positive, was {value}")
                lengths[name] = value

        return lengths, fraction_total

    def _omitted_length(self, lengths: Mapping[str, int], name: str) -> int:
        """
        Return the length of the omitted set, which absorbs the rest of the window.
        """
        if self.window is None:
            raise ValueError(f"the omitted length of set {name!r} requires an explicit window")
        room = self.window - sum(lengths.values())
        if room <= 0:
            raise ValueError(
                f"the declared lengths sum to {sum(lengths.values())}, which leaves no room "
                f"for set {name!r} in a window of {self.window}",
            )

        return room

    def _window_remainder(
        self,
        lengths: Mapping[str, int],
        fraction_total: float,
        window: int,
    ) -> int:
        """
        Return the length of the last set, which absorbs the window's rounding difference.

        Fractions round to the nanosecond, so when every declared length is fractional and the
        fractions account for the whole window the last set takes the difference instead of failing
        the contract. A total that falls short of the window for any other reason is refused.
        """
        total = sum(lengths.values())
        last = self.sets[-1]
        if total == window:
            return lengths[last]
        all_fractional = all(isinstance(value, float) for value in self.lengths.values())
        if not all_fractional or abs(fraction_total - 1.0) > _FRACTION_TOLERANCE:
            raise ValueError(f"the set lengths sum to {total}, not the declared window {window}")
        remainder = window - (total - lengths[last])
        if remainder <= 0:
            raise ValueError(f"the fractional lengths leave no room for set {last!r}")

        return remainder

    def split(self, start: int, end: int) -> tuple[Split, ...]:
        """
        Return the splits of the period, in time order.

        The exclusions are applied to every split in order: a preceding set ends no later than the
        purge before the evaluation set, a following set starts no earlier than the purge after it,
        and a split whose set becomes empty or shorter than `min_length` is dropped.

        Parameters
        ----------
        start : int
            The period start in Unix nanoseconds.
        end : int
            The period end in Unix nanoseconds.

        Returns
        -------
        tuple[Split, ...]

        """
        if end <= start:
            return ()

        lengths = self.resolved_lengths()
        window = sum(lengths.values()) if self.window is None else self.window
        stride = window if self.stride is None else self.stride
        stride = max(stride, window + self.leakage.gap)

        span = end - start
        if span < window:
            return ()

        count = 1 + (span - window) // stride
        tail = span - ((count - 1) * stride + window)
        splits: list[Split] = []

        for origin, extra in self._layout(start, count, stride, tail):
            bounds = self._bounds(origin, extra, lengths)
            bounds = self._excluded(bounds)
            if bounds is None:
                continue
            splits.append(Split(bounds))

        return self._select(splits)

    def _layout(self, start: int, count: int, stride: int, tail: int) -> list[tuple[int, int]]:
        """
        Return each window's start and the extra span the direction assigns to it.
        """
        leftovers = [0] * count
        origins = [start + index * stride for index in range(count)]

        if self.direction is SplitDirection.FORWARD:
            leftovers[-1] = tail
        elif self.direction is SplitDirection.REVERSED:
            origins = [origin + tail for origin in origins]
            leftovers[0] = tail

        return list(zip(origins, leftovers, strict=True))

    def _bounds(
        self,
        origin: int,
        extra: int,
        lengths: Mapping[str, int],
    ) -> dict[str, SetBounds]:
        """
        Return the set bounds of one window, with the extra span on the leading or trailing set.
        """
        bounds: dict[str, SetBounds] = {}
        cursor = origin
        for name in self.sets:
            bounds[name] = (cursor, cursor + lengths[name])
            cursor += lengths[name]

        if extra:
            if self.direction is SplitDirection.FORWARD:
                name = self.sets[-1]
                bounds[name] = (bounds[name][0], bounds[name][1] + extra)
            else:
                name = self.sets[0]
                bounds[name] = (bounds[name][0] - extra, bounds[name][1])

        return bounds

    def _excluded(self, bounds: dict[str, SetBounds]) -> dict[str, SetBounds] | None:
        """
        Return the bounds with the exclusions applied, or None when a set does not survive them.
        """
        evaluation = self.evaluation_set
        evaluation_start, evaluation_end = bounds[evaluation]
        excluded: dict[str, SetBounds] = {evaluation: (evaluation_start, evaluation_end)}
        for name in self.sets:
            if name == evaluation:
                continue
            low, high = bounds[name]
            if high <= evaluation_start:
                excluded[name] = (low, min(high, evaluation_start - self.leakage.purge))
            elif low >= evaluation_end:
                excluded[name] = (max(low, evaluation_end + self.leakage.after), high)
            else:
                raise ValueError(
                    f"set {name!r} overlaps the evaluation set {evaluation!r}, so the exclusion "
                    "relation cannot separate them",
                )

        for low, high in excluded.values():
            if high - low <= 0:
                return None
            if self.min_length is not None and high - low < self.min_length:
                return None

        return {name: excluded[name] for name in self.sets}

    def _select(self, splits: list[Split]) -> tuple[Split, ...]:
        """
        Return the requested number of evenly spaced splits, or the first when one is requested.
        """
        if self.n_splits is None or self.n_splits == len(splits):
            return tuple(splits)
        if len(splits) < self.n_splits:
            raise ValueError(
                f"n_splits requested {self.n_splits} splits, but only {len(splits)} fit the period "
                "under the declared lengths, minimum length and exclusions",
            )
        if self.n_splits == 1:
            return (splits[(len(splits) - 1) // 2],)

        indices = [
            index * (len(splits) - 1) // (self.n_splits - 1) for index in range(self.n_splits)
        ]

        return tuple(splits[index] for index in indices)
