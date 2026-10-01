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
Parameter spaces, experiments, and their canonical digests.

A parameter space holds fixed base values and a set of named parameters, each with a discrete
ordered list of choices. Expansion is deterministic: the Cartesian product is taken in declaration
order with the last parameter varying fastest (the order of `itertools.product`), so the same
space always yields the same sweep in the same order.

An experiment is a complete parameter set. Its digest is derived from the set itself, canonically
serialized (sorted keys, compact separators, strict JSON), never from a clock or randomness. Equal
sets therefore always produce equal digests, which is what makes results comparable.
"""

from __future__ import annotations

import hashlib
import itertools
import json
from dataclasses import dataclass
from dataclasses import field
from typing import TYPE_CHECKING
from typing import cast


if TYPE_CHECKING:
    from collections.abc import Iterator
    from collections.abc import Mapping
    from collections.abc import Sequence


type JsonValue = str | int | float | bool | None


def canonical_json(parameters: Mapping[str, JsonValue]) -> str:
    """
    Serialize the given parameter set in a canonical, deterministic form.

    Keys are sorted, separators are compact, and non-finite numbers are rejected, so the same set
    always serializes to the same string on every platform and run.

    Parameters
    ----------
    parameters : Mapping[str, JsonValue]
        The parameter set to serialize.

    Returns
    -------
    str
        The canonical compact JSON text.

    """
    return json.dumps(
        dict(parameters),
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=True,
        allow_nan=False,
    )


def digest_of(parameters: Mapping[str, JsonValue]) -> str:
    """
    Return the digest of the given parameter set.

    The digest is `sha256:` followed by the 64 lowercase hex digits of the SHA-256 hash of the
    canonical serialization, so it depends only on the set itself.

    Parameters
    ----------
    parameters : Mapping[str, JsonValue]
        The parameter set to digest.

    Returns
    -------
    str
        The `sha256:<hex>` digest.

    """
    encoded = canonical_json(parameters).encode("utf-8")
    return f"sha256:{hashlib.sha256(encoded).hexdigest()}"


@dataclass(frozen=True)
class Experiment:
    """
    A complete parameter set, stable by digest.

    The parameters are the full strategy configuration the run carries: the space's base values
    merged with one choice for every swept parameter. The digest is derived from this set alone.

    Parameters
    ----------
    parameters : Mapping[str, JsonValue]
        The complete parameter set.

    """

    parameters: Mapping[str, JsonValue]

    @property
    def canonical(self) -> str:
        """
        The canonical JSON serialization of the parameter set.
        """
        return canonical_json(self.parameters)

    @property
    def digest(self) -> str:
        """
        The `sha256:<hex>` digest of the parameter set.
        """
        return digest_of(self.parameters)


@dataclass(frozen=True)
class Parameter:
    """
    One named parameter with a discrete, ordered list of choices.

    Parameters
    ----------
    name : str
        The strategy configuration key to sweep.
    choices : tuple[JsonValue, ...]
        The ordered choices. Must be non-empty and free of duplicates.

    """

    name: str
    choices: tuple[JsonValue, ...]

    def __post_init__(self) -> None:
        """
        Validate the parameter.
        """
        if not self.name:
            raise ValueError("Parameter name must be non-empty")
        if not self.choices:
            raise ValueError(f"Parameter '{self.name}' must have at least one choice")
        if len(set(self.choices)) != len(self.choices):
            raise ValueError(f"Parameter '{self.name}' choices must be unique")


@dataclass(frozen=True)
class ParameterSpace:
    """
    A base configuration plus the parameters to sweep, expanded deterministically.

    Expansion walks the Cartesian product of the parameters in declaration order with the last
    parameter varying fastest, merging each combination over the base values. Base keys must not
    collide with parameter names, and parameter names must be unique.

    Parameters
    ----------
    base : Mapping[str, JsonValue], default {}
        Fixed strategy configuration values carried by every experiment.
    parameters : tuple[Parameter, ...], default ()
        The parameters to sweep, in declaration order.

    """

    base: Mapping[str, JsonValue] = field(default_factory=dict)
    parameters: tuple[Parameter, ...] = ()

    def __post_init__(self) -> None:
        """
        Validate the space.
        """
        names = [parameter.name for parameter in self.parameters]
        if len(set(names)) != len(names):
            raise ValueError("Parameter names must be unique")
        collisions = sorted(set(names) & set(self.base))
        if collisions:
            raise ValueError(f"Parameters must not collide with base keys: {collisions}")

    def expand(self) -> Iterator[Experiment]:
        """
        Yield the experiments of this space in the documented deterministic order.

        Yields
        ------
        Experiment

        """
        names = [parameter.name for parameter in self.parameters]
        choices: Sequence[tuple[JsonValue, ...]] = [
            parameter.choices for parameter in self.parameters
        ]
        for combination in itertools.product(*choices):
            parameters = dict(self.base)
            parameters.update(dict(zip(names, combination, strict=True)))
            yield Experiment(parameters)

    @property
    def size(self) -> int:
        """
        Return the number of experiments the space expands to.

        A space with no swept parameters expands to exactly one experiment, the base set, matching
        the empty Cartesian product.

        Returns
        -------
        int

        """
        total = 1
        for parameter in self.parameters:
            total *= len(parameter.choices)
        return total

    def indices_at(self, index: int) -> tuple[int, ...]:
        """
        Return the choice indices of the experiment at a mixed-radix position.

        The position order matches `expand`: the last parameter varies fastest, so a sampled index
        maps to the same experiment the deterministic sweep would reach.

        Parameters
        ----------
        index : int
            The experiment position, in `[0, size)`.

        Returns
        -------
        tuple[int, ...]

        """
        if index < 0 or index >= self.size:
            raise IndexError(f"experiment index {index} is outside the space of size {self.size}")
        indices: list[int] = []
        remaining = index
        for parameter in reversed(self.parameters):
            width = len(parameter.choices)
            indices.append(remaining % width)
            remaining //= width
        indices.reverse()
        return tuple(indices)

    def experiment_from_indices(self, indices: Sequence[int]) -> Experiment:
        """
        Return the experiment for a tuple of per-parameter choice indices.

        Parameters
        ----------
        indices : Sequence[int]
            One choice index per parameter, in declaration order.

        Returns
        -------
        Experiment

        """
        if len(indices) != len(self.parameters):
            raise ValueError(f"expected {len(self.parameters)} indices, was {len(indices)}")
        parameters = dict(self.base)
        for parameter, index in zip(self.parameters, indices, strict=True):
            if index < 0 or index >= len(parameter.choices):
                raise ValueError(
                    f"index {index} is outside parameter '{parameter.name}' with "
                    f"{len(parameter.choices)} choices",
                )
            parameters[parameter.name] = parameter.choices[index]
        return Experiment(parameters)

    def experiment_at(self, index: int) -> Experiment:
        """
        Return the experiment at a mixed-radix position.

        Parameters
        ----------
        index : int
            The experiment position, in `[0, size)`.

        Returns
        -------
        Experiment

        """
        return self.experiment_from_indices(self.indices_at(index))

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the declared space.

        The digest is over the base values and the parameters with their ordered choices, so two
        studies declare the same space exactly when they declare the same sweep.

        Returns
        -------
        str

        """
        payload = {
            "base": dict(self.base),
            "parameters": [
                {"name": parameter.name, "choices": list(parameter.choices)}
                for parameter in self.parameters
            ],
        }
        return digest_of(cast("Mapping[str, JsonValue]", payload))
