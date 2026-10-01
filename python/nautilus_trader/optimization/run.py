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
The declared shape of a run: its validation scheme, its seed, its operators and its cache.

A search over a dataset is only interpretable when the run states which part of the dataset it
searched and which part it held out. A `ValidationScheme` names both, as half-open nanosecond
windows, and refuses a scheme whose search windows overlap a held-out window. Because the driver
only ever hands the search the experiment space, the held-out windows are never scored by the
search; the scheme is what makes that a declaration rather than a convention.

`RunDescription` is the summary of those declarations plus the search identity: the space digest,
the seed and the search strategy, the evolutionary operators when there are any, and the digest of
the evaluation cache. Its digest is stable for two runs that declare the same search.
"""

from __future__ import annotations

import itertools
from dataclasses import dataclass
from enum import Enum
from enum import unique
from typing import TYPE_CHECKING
from typing import cast

from nautilus_trader.optimization.search import EvolutionaryOperators
from nautilus_trader.optimization.search import EvolutionarySearch
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.optimization.search import SearchStrategy
    from nautilus_trader.optimization.space import JsonValue
    from nautilus_trader.optimization.space import ParameterSpace


# A window is a half-open `(start, end)` pair of UnixNanos bounds: each int is Unix nanoseconds,
# `start` is included and `end` is excluded.
type WindowBounds = tuple[int, int]

# A half-open interval is a (start, end) pair of UnixNanos bounds.
_WINDOW_WIDTH = 2


@unique
class ValidationMode(Enum):
    """
    How a run partitions the dataset between search and held-out evaluation.

    `SINGLE_SPLIT` searches one window and holds out at most one, and `WALK_FORWARD` searches each
    in-sample window and holds out the out-of-sample window that follows it.
    """

    SINGLE_SPLIT = "single_split"
    WALK_FORWARD = "walk_forward"


def _require_bounds(bounds: object, where: str) -> WindowBounds:
    """
    Require a positive half-open nanosecond interval.
    """
    if not isinstance(bounds, tuple) or len(bounds) != _WINDOW_WIDTH:
        raise TypeError(f"{where} must be a (start, end) tuple, was {bounds!r}")
    start, end = bounds
    if isinstance(start, bool) or not isinstance(start, int):
        raise TypeError(f"{where} start must be nanoseconds as an int, was {start!r}")
    if isinstance(end, bool) or not isinstance(end, int):
        raise TypeError(f"{where} end must be nanoseconds as an int, was {end!r}")
    if end <= start:
        raise ValueError(f"{where} must be a positive interval, was {bounds!r}")
    return (start, end)


def _check_windows(windows: Sequence[WindowBounds], where: str) -> None:
    """
    Require non-overlapping positive windows, sorted by start.
    """
    checked = tuple(_require_bounds(bounds, where) for bounds in windows)
    ordered = sorted(checked)
    for current, following in itertools.pairwise(ordered):
        if current[1] > following[0]:
            raise ValueError(f"{where} windows overlap: {current} and {following}")


def _overlaps(first: WindowBounds, second: WindowBounds) -> bool:
    """
    Return whether two half-open intervals overlap; touching intervals do not.
    """
    return first[0] < second[1] and second[0] < first[1]


@dataclass(frozen=True)
class ValidationScheme:
    """
    The declared partition of the dataset between search and held-out evaluation.

    Parameters
    ----------
    mode : ValidationMode
        Whether the run is a single split or a walk-forward sequence.
    search_windows : tuple[WindowBounds, ...]
        The half-open UnixNanos windows scored by the search. Non-empty and non-overlapping.
    held_out_windows : tuple[WindowBounds, ...], default ()
        The half-open UnixNanos windows never scored by the search, disjoint from every search
        window.

    """

    mode: ValidationMode
    search_windows: tuple[WindowBounds, ...]
    held_out_windows: tuple[WindowBounds, ...] = ()

    def __post_init__(self) -> None:
        """
        Validate the mode, the windows and their disjointness.
        """
        if not isinstance(self.mode, ValidationMode):
            raise TypeError(f"mode must be a ValidationMode, was {self.mode!r}")
        if not self.search_windows:
            raise ValueError("a validation scheme requires at least one search window")
        _check_windows(self.search_windows, "search_windows")
        _check_windows(self.held_out_windows, "held_out_windows")
        self._validate_disjoint()
        self._validate_mode()

    def _validate_disjoint(self) -> None:
        """
        Refuse a held-out window that overlaps a search window.
        """
        for search in self.search_windows:
            for held in self.held_out_windows:
                if _overlaps(search, held):
                    raise ValueError(
                        f"held-out window {held} overlaps search window {search}, so it would be "
                        "scored by the search",
                    )

    def _validate_mode(self) -> None:
        """
        Refuse a scheme whose windows do not match its mode.
        """
        if self.mode is ValidationMode.SINGLE_SPLIT and len(self.search_windows) != 1:
            raise ValueError("a single-split scheme has exactly one search window")
        if self.mode is ValidationMode.WALK_FORWARD and len(self.search_windows) != len(
            self.held_out_windows
        ):
            raise ValueError(
                "a walk-forward scheme pairs every search window with a held-out window",
            )

    @classmethod
    def single_split(
        cls,
        search: WindowBounds,
        held_out: WindowBounds | None = None,
    ) -> ValidationScheme:
        """
        Return a single-split scheme with one search window and an optional held-out window.

        Parameters
        ----------
        search : WindowBounds
            The search window.
        held_out : WindowBounds | None, default None
            The held-out window, or None when the run declares no hold-out.

        Returns
        -------
        ValidationScheme

        """
        held: tuple[WindowBounds, ...] = () if held_out is None else (held_out,)
        return cls(ValidationMode.SINGLE_SPLIT, (search,), held)

    @classmethod
    def walk_forward(
        cls,
        windows: Sequence[tuple[WindowBounds, WindowBounds]],
    ) -> ValidationScheme:
        """
        Return a walk-forward scheme from `(search, held_out)` window pairs.

        Parameters
        ----------
        windows : Sequence[tuple[WindowBounds, WindowBounds]]
            The search and held-out bounds of every window, in order.

        Returns
        -------
        ValidationScheme

        """
        return cls(
            ValidationMode.WALK_FORWARD,
            tuple(pair[0] for pair in windows),
            tuple(pair[1] for pair in windows),
        )

    @classmethod
    def from_walk_forward_windows(cls, windows: Sequence[object]) -> ValidationScheme:
        """
        Return a walk-forward scheme from `WalkForwardWindow`-shaped objects.

        Parameters
        ----------
        windows : Sequence[object]
            The windows, each exposing `in_sample_start`, `in_sample_end`, `out_of_sample_start`
            and `out_of_sample_end`.

        Returns
        -------
        ValidationScheme

        """
        return cls.walk_forward(
            tuple(
                (
                    (window.in_sample_start, window.in_sample_end),
                    (window.out_of_sample_start, window.out_of_sample_end),
                )
                for window in windows
            ),
        )

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the scheme as a canonical-friendly mapping.
        """
        return {
            "mode": self.mode.value,
            "search_windows": [list(bounds) for bounds in self.search_windows],
            "held_out_windows": [list(bounds) for bounds in self.held_out_windows],
        }

    @classmethod
    def from_dict(cls, payload: Mapping[str, object]) -> ValidationScheme:
        """
        Return the scheme decoded from a mapping.

        Parameters
        ----------
        payload : Mapping[str, object]
            A mapping as produced by `to_dict`.

        Returns
        -------
        ValidationScheme

        """
        mode = ValidationMode(payload["mode"])
        search = tuple(tuple(bounds) for bounds in payload["search_windows"])
        held = tuple(tuple(bounds) for bounds in payload["held_out_windows"])

        return cls(
            mode,
            cast("tuple[WindowBounds, ...]", search),
            cast("tuple[WindowBounds, ...]", held),
        )

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the declared scheme.
        """
        return digest_of(cast("Mapping[str, JsonValue]", self.to_dict()))


@dataclass(frozen=True)
class RunDescription:
    """
    The declared search identity of a run: its effects, not its outcome.

    Parameters
    ----------
    space_digest : str
        The digest of the parameter space the run searches.
    scheme : ValidationScheme
        The validation scheme the run searches under.
    seed : int | None, default None
        The seed the search is derived from, or None when it declares none.
    search_strategy : str | None, default None
        The name of the search strategy, or None when undeclared.
    operators : EvolutionaryOperators | None, default None
        The evolutionary operators, or None for a non-evolutionary search.
    cache_digest : str | None, default None
        The digest of the evaluation cache contents, or None when no cache is declared.

    """

    space_digest: str
    scheme: ValidationScheme
    seed: int | None = None
    search_strategy: str | None = None
    operators: EvolutionaryOperators | None = None
    cache_digest: str | None = None

    def __post_init__(self) -> None:
        """
        Validate the declared run.
        """
        if not self.space_digest:
            raise ValueError("space_digest must not be empty")
        if not isinstance(self.scheme, ValidationScheme):
            raise TypeError(f"scheme must be a ValidationScheme, was {self.scheme!r}")
        if self.operators is not None and not isinstance(self.operators, EvolutionaryOperators):
            raise TypeError(f"operators must be EvolutionaryOperators, was {self.operators!r}")

    @classmethod
    def of(
        cls,
        space: ParameterSpace,
        scheme: ValidationScheme,
        *,
        search: SearchStrategy | None = None,
        cache: object | None = None,
    ) -> RunDescription:
        """
        Return the run description for a space, a scheme, and an optional search and cache.

        Parameters
        ----------
        space : ParameterSpace
            The space the run searches.
        scheme : ValidationScheme
            The validation scheme the run searches under.
        search : SearchStrategy | None, default None
            The search strategy, or None when the run declares none.
        cache : object | None, default None
            An evaluation cache exposing a `digest` property, or None.

        Returns
        -------
        RunDescription

        """
        operators = search.operators if isinstance(search, EvolutionarySearch) else None
        return cls(
            space_digest=space.digest,
            scheme=scheme,
            seed=getattr(search, "seed", None),
            search_strategy=None if search is None else type(search).__name__,
            operators=operators,
            cache_digest=getattr(cache, "digest", None),
        )

    def to_dict(self) -> dict[str, object]:
        """
        Return the description as a canonical mapping.
        """
        return {
            "space_digest": self.space_digest,
            "scheme": self.scheme.to_dict(),
            "seed": self.seed,
            "search_strategy": self.search_strategy,
            "operators": None if self.operators is None else _operators_dict(self.operators),
            "cache_digest": self.cache_digest,
        }

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the declared run.
        """
        return digest_of(cast("Mapping[str, JsonValue]", self.to_dict()))


def _operators_dict(operators: EvolutionaryOperators) -> dict[str, object]:
    """
    Return the evolutionary operators as a canonical mapping.
    """
    return {
        "population_size": operators.population_size,
        "generations": operators.generations,
        "elite": operators.elite,
        "tournament_size": operators.tournament_size,
        "crossover_rate": operators.crossover_rate,
        "mutation_rate": operators.mutation_rate,
    }
