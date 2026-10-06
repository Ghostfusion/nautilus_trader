// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// Configuration for `OrderMatchingEngine` instances.
///
/// With `defer_option_settlement`, automatic checks at the exact option expiry
/// timestamp cancel orders but defer settlement to `process_instrument_expiration`.
/// To settle at expiry, the caller schedules that call after processing the
/// timestamp's market data. Automatic checks after expiry can also settle.
/// Explicit contract-close events bypass this deferral.
#[derive(Debug, Clone, Deserialize, Serialize, bon::Builder)]
#[serde(default, deny_unknown_fields)]
pub struct OrderMatchingEngineConfig {
    #[builder(default = true)]
    pub bar_execution: bool,
    #[builder(default)]
    pub bar_adaptive_high_low_ordering: bool,
    #[builder(default = true)]
    pub trade_execution: bool,
    #[builder(default)]
    pub liquidity_consumption: bool,
    #[builder(default = true)]
    pub reject_stop_orders: bool,
    #[builder(default = true)]
    pub support_gtd_orders: bool,
    #[builder(default = true)]
    pub support_contingent_orders: bool,
    #[builder(default = true)]
    pub use_position_ids: bool,
    #[builder(default)]
    pub use_random_ids: bool,
    #[builder(default = true)]
    pub use_reduce_only: bool,
    #[builder(default)]
    pub use_market_order_acks: bool,
    #[builder(default)]
    pub queue_position: bool,
    /// Values a passive (maker) fill at the imbalance-adjusted microprice of the touch instead
    /// of the order's own limit price.
    ///
    /// The microprice is bounded by the maker's limit, so a fill is never valued worse than the
    /// limit. When the book has no two-sided size, or the flag is off, the limit price is kept
    /// and behaviour is unchanged. Defaults to false.
    #[builder(default)]
    pub passive_fill_microprice: bool,
    #[builder(default)]
    pub oto_full_trigger: bool,
    #[builder(default)]
    pub defer_option_settlement: bool,
    /// Cancels all open orders for the instrument when the market is halted.
    ///
    /// A halt blocks new submissions and matching either way, and leaves the resting book in
    /// place to be resolved by a reopen. With this flag the halt also empties the book, and each
    /// cancellation carries the venue reason `MARKET_HALTED`, so the emptied book is
    /// attributable in the resulting events. Defaults to false.
    #[builder(default)]
    pub cancel_on_halt: bool,
    /// The submission price band as a symmetric distance from the venue's reference price in
    /// basis points, or `None` for no band.
    ///
    /// A submission carrying a price or trigger price outside the band is rejected by the venue,
    /// naming the price, the band and the reference it was measured against. A band only rejects:
    /// trading continues, which is what distinguishes it from a circuit breaker and from the venue
    /// halt. Without a reference price the band cannot be evaluated, so a priced submission is
    /// rejected; a band is fail-closed rather than silently absent. Must be less than 10000, which
    /// the venue configuration enforces. Defaults to none.
    pub price_band_bps: Option<u32>,
    /// The circuit breaker, or `None` for none.
    ///
    /// A breaker is a halt window: it stops trading and then reopens on its own, where the venue
    /// halt stops trading until an operator reopens, and a price band only rejects an out-of-band
    /// submission. See [`CircuitBreakerConfig`]. Defaults to none.
    pub circuit_breaker: Option<CircuitBreakerConfig>,
    pub price_protection_points: Option<u32>,
}

/// Configuration for a matching engine's circuit breaker.
///
/// The breaker watches the venue's reference price, which is the same reference the price band is
/// measured against. It trips when that price moves by at least `move_bps` basis points from its
/// value at the start of the current window, where a window lasts `window_ns` from the event that
/// anchored it. A tripped breaker halts the market for `halt_ns` and reopens on the first matching
/// pass at or after the end of that halt, which anchors a fresh window.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
#[derive(Debug, Clone, Deserialize, Serialize, bon::Builder)]
#[serde(default, deny_unknown_fields)]
pub struct CircuitBreakerConfig {
    /// The move from the window's anchor price that trips the breaker, in basis points.
    #[builder(default = 200)]
    pub move_bps: u32,
    /// The duration of a window in nanoseconds.
    #[builder(default = 60_000_000_000)]
    pub window_ns: u64,
    /// The duration of the halt after a trip, in nanoseconds.
    #[builder(default = 60_000_000_000)]
    pub halt_ns: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl Default for OrderMatchingEngineConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_default() {
        let config = OrderMatchingEngineConfig::default();
        assert!(config.bar_execution);
        assert!(!config.bar_adaptive_high_low_ordering);
        assert!(config.trade_execution);
        assert!(!config.liquidity_consumption);
        assert!(config.reject_stop_orders);
        assert!(config.support_gtd_orders);
        assert!(config.support_contingent_orders);
        assert!(config.use_position_ids);
        assert!(!config.use_random_ids);
        assert!(config.use_reduce_only);
        assert!(!config.use_market_order_acks);
        assert!(!config.queue_position);
        assert!(!config.passive_fill_microprice);
        assert!(!config.oto_full_trigger);
        assert!(!config.defer_option_settlement);
        assert!(!config.cancel_on_halt);
        assert!(config.price_band_bps.is_none());
        assert!(config.circuit_breaker.is_none());
        assert_eq!(config.price_protection_points, None);
    }

    #[test]
    fn test_circuit_breaker_config_defaults() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.move_bps, 200);
        assert_eq!(config.window_ns, 60_000_000_000);
        assert_eq!(config.halt_ns, 60_000_000_000);
    }
}
