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
Tests for the declared-scale conversion.
"""

from __future__ import annotations

from decimal import Decimal

import pytest

from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.numeric import DeclaredScales
from nautilus_trader.decision_bridge.numeric import allocation_fraction
from nautilus_trader.decision_bridge.numeric import is_allocation_in_domain
from nautilus_trader.decision_bridge.numeric import to_decimal


def test_a_value_that_fits_its_declared_scale_converts_exactly() -> None:
    """
    Test that a value fitting its declared scale converts exactly to a Decimal.
    """
    result = to_decimal(191.34, places=2, field="entry_price")

    assert result == Decimal("191.34")


def test_a_value_needing_more_precision_is_refused_rather_than_rounded() -> None:
    """
    Test that a value needing more precision is refused rather than rounded.
    """
    result = to_decimal(1.005, places=2, field="entry_price")

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.FLOAT_CONVERSION_OVERFLOW
    assert "entry_price" in result.detail


def test_a_large_magnitude_that_exceeds_the_working_precision_is_refused() -> None:
    """
    Test that a value exceeding the working precision is refused.
    """
    result = to_decimal(1e30, places=2, field="target_notional")

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.FLOAT_CONVERSION_OVERFLOW


def test_non_finite_and_non_numeric_values_are_refused() -> None:
    """
    Test that non-finite and non-numeric values are refused.
    """
    for value in (float("nan"), float("inf"), float("-inf"), "5.0", None, True):
        result = to_decimal(value, places=2, field="stop_loss")
        assert isinstance(result, Refusal), value
        assert result.code is RefusalCode.FLOAT_CONVERSION_OVERFLOW, value


def test_a_value_already_at_its_scale_is_not_truncated() -> None:
    """
    Test that a value already at its declared scale is converted without truncation.
    """
    assert to_decimal(0.1, places=1, field="quantity") == Decimal("0.1")
    assert isinstance(to_decimal(0.15, places=1, field="quantity"), Refusal)


def test_the_declared_scales_refuse_a_negative_or_excessive_precision() -> None:
    """
    Test that declared scales reject a negative precision, an excessive one, and a non-int.
    """
    with pytest.raises(ValueError, match="must not be negative"):
        DeclaredScales(price=-1)
    with pytest.raises(ValueError, match="exceeds the supported precision"):
        DeclaredScales(quantity=21)
    with pytest.raises(ValueError, match="must be an int"):
        DeclaredScales(money=2.0)  # type: ignore[arg-type]


def test_the_allocation_domain_admits_zero_to_one_hundred_inclusive() -> None:
    """
    Test that the allocation domain admits None and values from zero to one hundred inclusive.
    """
    assert is_allocation_in_domain(None)
    assert is_allocation_in_domain(0)
    assert is_allocation_in_domain(100)
    assert is_allocation_in_domain(5.0)


def test_the_allocation_domain_refuses_negative_oversized_and_non_finite() -> None:
    """
    Test that the allocation domain refuses negative, oversized, and non-finite values.
    """
    assert not is_allocation_in_domain(-0.01)
    assert not is_allocation_in_domain(100.01)
    assert not is_allocation_in_domain(float("nan"))
    assert not is_allocation_in_domain(float("inf"))
    assert not is_allocation_in_domain("5")
    assert not is_allocation_in_domain(True)


def test_the_allocation_is_normalised_from_a_percentage_to_a_fraction_once() -> None:
    """
    Test that a percentage allocation is normalised to a fraction exactly once.
    """
    assert allocation_fraction(5.0) == Decimal("0.05")
    assert allocation_fraction(0) == Decimal(0)
    assert allocation_fraction(100) == Decimal(1)


def test_an_allocation_that_does_not_fit_its_declared_scale_is_refused() -> None:
    """
    Test that an allocation not fitting its declared scale is refused.
    """
    result = allocation_fraction(5.0000001, scales=DeclaredScales(allocation=6))

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.FLOAT_CONVERSION_OVERFLOW
