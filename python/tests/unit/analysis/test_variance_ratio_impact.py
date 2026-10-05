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
The linear-impact failure mode, made observable.

A persistent flow priced with a *linear* impact response produces a price path whose variance ratio
is well above one: the trend is an artifact of the impact model rather than a property of the flow,
and this test is what makes it observable instead of mistakable for a result.

The concave response the generator validates inflates the same flow less, and it does not remove
the inflation: the flow is persistent by construction, so every monotone impact response leaves the
induced price positively autocorrelated. The durable statement is therefore that the linear response
reports a variance ratio above one and above the concave response's, not that the concave response
is diffusive; the measured values are recorded in each test and in the plan's T3 acceptance.
"""

from nautilus_trader.analysis import VarianceRatio
from nautilus_trader.backtest import SyntheticFlowConfig


NANOS_PER_DAY = 86_400_000_000_000

# A Hurst exponent above one half is what makes the flow persistent, which is the condition the
# artifact needs. The generator refuses an exponent at or below one half.
TARGET_HURST = 0.6
IMPACT_EXPONENT = 0.5
COUNT = 4096
SEED = 42

# The variance ratio is read at a scale large enough for a persistent process to show itself.
PERIOD = 20

# Returns are ratios of consecutive prices, so the path is based away from zero to keep them sane.
PRICE_BASE = 1000.0


def returns_from_prices(prices: list[float]) -> dict[int, float]:
    """
    Build the simple returns of a price path, one observation per day.
    """
    returns: dict[int, float] = {}
    for index in range(1, len(prices)):
        previous = prices[index - 1]
        if previous == 0.0:
            continue
        returns[index * NANOS_PER_DAY] = prices[index] / previous - 1.0
    return returns


def linear_impact_prices(quantities: list[float]) -> list[float]:
    """
    Price a flow with a linear impact response: each period's impact is the flow itself.

    The generator's own path applies ``sign(flow) * |flow| ** impact_exponent`` per period. An
    exponent of one is what a linear model implies, and the generator refuses it because it breaks
    the diffusiveness condition, so the path is built here to show what a linear model would have
    produced from the same flow.
    """
    prices = [PRICE_BASE]
    for quantity in quantities:
        prices.append(prices[-1] + quantity)
    return prices


def test_linear_impact_on_a_persistent_flow_reports_a_variance_ratio_above_one() -> None:
    """
    A linear impact response turns a persistent flow into a trend the ratio can see.
    """
    flow = SyntheticFlowConfig(TARGET_HURST, IMPACT_EXPONENT, COUNT, SEED).generate()

    ratio = VarianceRatio(PERIOD).calculate_from_returns(
        returns_from_prices(linear_impact_prices(flow.quantities))
    )

    # Measured at these parameters: 1.533 (and 1.476 at seed 7), so the claim is well clear of one.
    assert ratio is not None
    assert ratio > 1.0, f"linear impact on a persistent flow reported {ratio}"


def test_the_concave_response_inflates_the_same_flow_less_than_the_linear_one() -> None:
    """
    The concave response reduces the inflation without removing it.

    The flow is persistent, so the concave path is above one as well; what separates the two is the
    size of the inflation, which is what a reader has to compare before attributing a trend to the
    flow.
    """
    config = SyntheticFlowConfig(TARGET_HURST, IMPACT_EXPONENT, COUNT, SEED)
    flow = config.generate()

    concave = VarianceRatio(PERIOD).calculate_from_returns(returns_from_prices(flow.prices))
    linear = VarianceRatio(PERIOD).calculate_from_returns(
        returns_from_prices(linear_impact_prices(flow.quantities))
    )

    # Measured at these parameters: 1.391 concave against 1.533 linear, on the same flow and seed.
    assert concave is not None
    assert linear is not None
    assert concave < linear, f"concave {concave} did not stay below linear {linear}"
