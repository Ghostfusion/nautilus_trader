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
The float boundary, scoped to execution-bearing fields.

The research half computes in floating point throughout; this repository's prices, quantities and
money are fixed point. Every execution-bearing numeric field crossing the boundary is converted at a
declared scale, and a value that does not fit is refused rather than rounded to fit -- a bound met
by
cutting a cell in half is not a bound.

Execution-bearing means a price, a quantity, a money amount, a stop distance, or an allocation that
will bound exposure. Research metadata -- scores, coverage, sentiment and the model's confidence --
is *not* converted: it has no execution purpose, it is consumed by measurement rather than by the
order layer, and transforming it into an execution type would be a numerical change with no
consumer. That is why every conversion here takes the declared scale explicitly and no helper in
this module accepts a metadata field.

The declared scale is configuration rather than an implicit default of a parser, so the place a
number's precision is decided is visible in one object instead of scattered through call sites.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import ROUND_HALF_EVEN
from decimal import Decimal
from decimal import DecimalException
from typing import TYPE_CHECKING

from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode


if TYPE_CHECKING:
    from decimal import Context


# The allocation's declared domain, as a percentage. The producer declares the same domain on its
# own field, and it is validated rather than clamped: a negative allocation must not be allowed to
# interact with exit logic, and a value above the maximum must not silently become the maximum,
# because clamping would mask a contract violation as a legitimate instruction.
ALLOCATION_MIN_PCT = Decimal(0)
ALLOCATION_MAX_PCT = Decimal(100)


@dataclass(frozen=True)
class DeclaredScales:
    """
    The declared decimal scale of each execution-bearing field class.

    Parameters
    ----------
    price : int
        Decimal places for a price.
    quantity : int
        Decimal places for a quantity.
    money : int
        Decimal places for a money amount.
    stop_distance : int
        Decimal places for a stop distance.
    allocation : int
        Decimal places for an allocation that bounds exposure.

    Raises
    ------
    ValueError
        If any scale is negative or exceeds the working precision of the decimal context.

    """

    price: int = 2
    quantity: int = 8
    money: int = 2
    stop_distance: int = 4
    allocation: int = 6

    def __post_init__(self) -> None:
        """
        Validate that every declared scale is usable.
        """
        for name in ("price", "quantity", "money", "stop_distance", "allocation"):
            places = getattr(self, name)
            if not isinstance(places, int) or isinstance(places, bool):
                raise ValueError(  # noqa: TRY004 - a declared scale is configuration
                    f"DeclaredScales.{name} must be an int, was {type(places).__name__}",
                )
            if places < 0:
                raise ValueError(f"DeclaredScales.{name} must not be negative, was {places}")
            if places > 20:  # noqa: PLR2004 - the working precision is the contract
                raise ValueError(
                    f"DeclaredScales.{name} {places} exceeds the supported precision (20)",
                )


# The declared default, stated once. A caller that needs a different scale supplies its own object
# rather than a keyword default appearing at each call site.
DEFAULT_SCALES = DeclaredScales()


def quantizer(places: int) -> Decimal:
    """
    Return the quantiser for a number of decimal places.

    Parameters
    ----------
    places : int
        The number of decimal places.

    Returns
    -------
    Decimal

    """
    return Decimal(1).scaleb(-places)


def is_finite_number(value: object) -> bool:
    """
    Return whether a value is a finite number rather than a bool, string or non-finite float.

    Parameters
    ----------
    value : object
        The value to test.

    Returns
    -------
    bool

    """
    if isinstance(value, bool):
        return False
    if isinstance(value, int):
        return True
    if isinstance(value, float):
        return value == value and value not in (float("inf"), float("-inf"))  # noqa: PLR0124 - NaN test
    if isinstance(value, Decimal):
        return value.is_finite()

    return False


def as_decimal(value: float | Decimal) -> Decimal:
    """
    Return the exact decimal reading of a number.

    A float is read through its own shortest round-tripping representation rather than through
    binary
    expansion, so `0.1` reads as `Decimal("0.1")` and not as its 55-digit binary value. The equality
    test in `to_decimal` is then a statement about the number the producer wrote.

    Parameters
    ----------
    value : int | float | Decimal
        The number to read.

    Returns
    -------
    Decimal

    """
    if isinstance(value, Decimal):
        return value
    if isinstance(value, float):
        return Decimal(repr(value))

    return Decimal(value)


def to_decimal(
    value: object,
    *,
    places: int,
    field: str,
    context: Context | None = None,
) -> Decimal | Refusal:
    """
    Convert an execution-bearing value at its declared scale.

    Parameters
    ----------
    value : object
        The value as read from the artifact.
    places : int
        The declared decimal places for the field.
    field : str
        The field's name, for the refusal's detail.
    context : Context, optional
        The decimal context to work in. Defaults to the thread's context.

    Returns
    -------
    Decimal | Refusal
        The value at its declared scale, or a refusal naming the field. A value that needs more
        precision than declared is refused rather than rounded to fit.

    """
    if not is_finite_number(value):
        return Refusal(
            RefusalCode.FLOAT_CONVERSION_OVERFLOW,
            f"{field}={value!r} is not a finite number",
        )

    exact = as_decimal(value)  # type: ignore[arg-type]
    try:
        quantised = exact.quantize(quantizer(places), rounding=ROUND_HALF_EVEN, context=context)
    except DecimalException as exc:
        return Refusal(
            RefusalCode.FLOAT_CONVERSION_OVERFLOW,
            f"{field}={exact} does not fit the declared scale of {places} places: {exc}",
        )

    if quantised != exact:
        return Refusal(
            RefusalCode.FLOAT_CONVERSION_OVERFLOW,
            f"{field}={exact} needs more precision than the declared {places} places",
        )

    return quantised


def is_allocation_in_domain(value: object) -> bool:
    """
    Return whether an advisory allocation is inside its declared domain.

    Parameters
    ----------
    value : object
        The allocation as read from the artifact, or `None` when it is absent.

    Returns
    -------
    bool
        `True` when the value is absent, or a finite number within zero to one hundred inclusive.

    """
    if value is None:
        return True
    if not is_finite_number(value):
        return False

    decimal = as_decimal(value)  # type: ignore[arg-type]
    return ALLOCATION_MIN_PCT <= decimal <= ALLOCATION_MAX_PCT


def allocation_fraction(
    value: object,
    *,
    scales: DeclaredScales = DEFAULT_SCALES,
) -> Decimal | Refusal:
    """
    Convert an advisory allocation to a fraction of exposure.

    The normalisation happens exactly once, here, from the artifact's percentage to the fraction the
    engine's configuration speaks. The domain check belongs to admission, so this function reads a
    value already inside its domain.

    Parameters
    ----------
    value : object
        The allocation as read from the artifact.
    scales : DeclaredScales, optional
        The declared scales. Defaults to `DEFAULT_SCALES`.

    Returns
    -------
    Decimal | Refusal
        The allocation as a fraction, or a refusal when the value does not fit its declared scale.

    """
    converted = to_decimal(
        value,
        places=scales.allocation,
        field="recommended_allocation_pct",
    )
    if isinstance(converted, Refusal):
        return converted

    return converted / ALLOCATION_MAX_PCT
