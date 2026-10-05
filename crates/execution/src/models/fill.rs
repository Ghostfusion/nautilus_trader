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

use std::{
    cell::RefCell,
    fmt::{Debug, Display},
    rc::Rc,
};

#[cfg(all(feature = "simulation", madsim))]
use madsim::rand::RngCore;
use nautilus_core::{
    UnixNanos,
    correctness::{check_in_range_inclusive_f64, check_non_negative_f64},
};
use nautilus_model::{
    data::order::BookOrder,
    enums::{BookType, OrderSide},
    identifiers::InstrumentId,
    instruments::{Instrument, InstrumentAny},
    orderbook::OrderBook,
    orders::{Order, OrderAny},
    types::{Price, Quantity},
};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// Sentinel size used as "unlimited" liquidity in the synthetic fill book.
const UNLIMITED_LIQUIDITY_UNITS: u64 = 10_000_000_000;

fn unlimited_liquidity(precision: u8) -> Quantity {
    Quantity::from_mantissa_exponent(UNLIMITED_LIQUIDITY_UNITS, 0, precision)
}

/// Context for a passive (maker) fill decision.
///
/// A passive fill decision is made when a resting limit order is at the touch and the matching
/// engine asks its fill model whether the order fills. The context carries only the signals the
/// engine already tracks at that point: the order's own side and quantity, the quantity resting
/// ahead of it at its price when queue position tracking is enabled, and a signed measure of
/// recent trade-flow toxicity against the order's side.
///
/// A model that does not condition on the context implements nothing new: the default
/// [`FillModel::is_limit_filled_with_context`] delegates to [`FillModel::is_limit_filled`], and
/// the engine's decision is exactly what it was before the context existed.
#[derive(Clone, Copy, Debug)]
pub struct PassiveFillContext {
    /// The side of the resting order seeking a passive fill.
    pub order_side: OrderSide,
    /// The quantity the order is seeking to fill.
    pub order_quantity: Quantity,
    /// The quantity resting ahead of the order at its price.
    ///
    /// Zero when queue position tracking is disabled or the order is not tracked.
    pub queue_ahead: Quantity,
    /// A signed measure of recent trade-flow toxicity, in `[-1, 1]`.
    ///
    /// Positive is adverse to the resting order (recent aggressive flow was on the opposite
    /// side), zero is no signal, and negative is favourable flow on the order's own side. The
    /// magnitude is the last trade's size as a fraction of the order's own quantity, capped at
    /// one; the sign is the direction of that flow relative to the resting side.
    pub toxicity: f64,
}

pub trait FillModel {
    /// Returns `true` if a limit order should be filled based on the model.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot determine whether the order should fill.
    fn is_limit_filled(&mut self) -> anyhow::Result<bool>;

    /// Returns `true` if a limit order should be filled, given passive fill `context`.
    ///
    /// The default delegates to [`FillModel::is_limit_filled`], so a model that does not
    /// condition on the book or recent flow is unaffected and receives the same decision it made
    /// before this method existed.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot determine whether the order should fill.
    fn is_limit_filled_with_context(
        &mut self,
        _context: &PassiveFillContext,
    ) -> anyhow::Result<bool> {
        self.is_limit_filled()
    }

    /// Returns `true` if an order fill should slip by one tick.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot determine whether the order should slip.
    fn is_slipped(&mut self) -> anyhow::Result<bool>;

    /// Returns whether limit orders at or inside the spread are fillable.
    ///
    /// When true, the matching core treats a limit order as fillable if its
    /// price is at or better than the current best quote on its own side
    /// (BUY >= bid, SELL <= ask), not just when it crosses the spread.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot determine its spread-fill behavior.
    fn fill_limit_inside_spread(&self) -> anyhow::Result<bool> {
        Ok(false)
    }

    /// Returns the random seed this model declares, or `None` if unseeded.
    ///
    /// Models that make no random draws, or that rely on an unseeded source,
    /// return `None`.
    fn random_seed(&self) -> Option<u64> {
        None
    }

    /// Seeds this model if it declares no seed of its own.
    ///
    /// Called before the venue is constructed so that a run is reproducible. An
    /// implementation that cannot be seeded simply leaves the model unchanged;
    /// the default does nothing.
    fn seed_if_unset(&mut self, seed: u64) {
        let _ = seed;
    }

    /// Returns a simulated `OrderBook` for fill simulation.
    ///
    /// Custom fill models provide their own liquidity simulation by returning an
    /// `OrderBook` that represents expected market liquidity. The matching engine
    /// uses this to determine fills.
    ///
    /// Returns `None` to use the matching engine's standard fill logic.
    /// A returned book supplies the available liquidity, including when it yields no fills.
    /// Missing historical bid or ask prices are passed as `None`.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot provide simulated liquidity.
    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>>;
}

/// Shared runtime handle for a fill model.
#[derive(Clone)]
pub struct FillModelHandle(Rc<RefCell<dyn FillModel>>);

impl FillModelHandle {
    /// Creates a new [`FillModelHandle`] from a fill model.
    #[must_use]
    pub fn new<T>(model: T) -> Self
    where
        T: FillModel + 'static,
    {
        Self(Rc::new(RefCell::new(model)))
    }

    /// Creates a new [`FillModelHandle`] from an existing reference-counted model.
    #[must_use]
    pub fn from_rc(model: Rc<RefCell<dyn FillModel>>) -> Self {
        Self(model)
    }

    /// Returns the random seed declared by the wrapped fill model, if any.
    #[must_use]
    pub fn random_seed(&self) -> Option<u64> {
        self.0.borrow().random_seed()
    }

    /// Seeds the wrapped fill model if it declares no seed of its own.
    ///
    /// The handle may be shared, so the model is updated through its interior
    /// mutability. Seeding is expected before the venue is constructed.
    pub fn seed_if_unset(&self, seed: u64) {
        self.0.borrow_mut().seed_if_unset(seed);
    }
}

impl Debug for FillModelHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(FillModelHandle))
            .field(&"<dyn FillModel>")
            .finish()
    }
}

impl FillModel for FillModelHandle {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        self.0.borrow_mut().is_limit_filled()
    }

    fn is_limit_filled_with_context(
        &mut self,
        context: &PassiveFillContext,
    ) -> anyhow::Result<bool> {
        self.0.borrow_mut().is_limit_filled_with_context(context)
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        self.0.borrow_mut().is_slipped()
    }

    fn fill_limit_inside_spread(&self) -> anyhow::Result<bool> {
        self.0.borrow().fill_limit_inside_spread()
    }

    fn random_seed(&self) -> Option<u64> {
        self.0.borrow().random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        self.0.borrow_mut().seed_if_unset(seed);
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        self.0
            .borrow_mut()
            .get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask)
    }
}

impl Default for FillModelHandle {
    fn default() -> Self {
        FillModelAny::default().into()
    }
}

impl From<FillModelAny> for FillModelHandle {
    fn from(model: FillModelAny) -> Self {
        Self::new(model)
    }
}

#[derive(Debug)]
pub struct ProbabilisticFillState {
    prob_fill_on_limit: f64,
    prob_slippage: f64,
    random_seed: Option<u64>,
    rng: StdRng,
}

impl ProbabilisticFillState {
    /// Creates a new [`ProbabilisticFillState`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        check_in_range_inclusive_f64(prob_fill_on_limit, 0.0, 1.0, "prob_fill_on_limit")?;
        check_in_range_inclusive_f64(prob_slippage, 0.0, 1.0, "prob_slippage")?;
        let rng = match random_seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => default_std_rng(),
        };
        Ok(Self {
            prob_fill_on_limit,
            prob_slippage,
            random_seed,
            rng,
        })
    }

    /// Returns the declared random seed, or `None` if the state is unseeded.
    #[must_use]
    pub fn random_seed(&self) -> Option<u64> {
        self.random_seed
    }

    /// Reseeds the state with the given `seed`.
    ///
    /// Intended for a component that declared no seed of its own: it records
    /// `seed` and rebuilds the random source from it. The draw probabilities are
    /// unchanged.
    pub fn reseed(&mut self, seed: u64) {
        self.random_seed = Some(seed);
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn is_limit_filled(&mut self) -> bool {
        self.event_success(self.prob_fill_on_limit)
    }

    pub fn is_slipped(&mut self) -> bool {
        self.event_success(self.prob_slippage)
    }

    pub fn random_bool(&mut self, probability: f64) -> bool {
        self.event_success(probability)
    }

    fn event_success(&mut self, probability: f64) -> bool {
        match probability {
            0.0 => false,
            1.0 => true,
            _ => self.rng.random_bool(probability),
        }
    }
}

impl Clone for ProbabilisticFillState {
    fn clone(&self) -> Self {
        Self::new(
            self.prob_fill_on_limit,
            self.prob_slippage,
            self.random_seed,
        )
        .expect("ProbabilisticFillState clone should not fail with valid parameters")
    }
}

fn default_std_rng() -> StdRng {
    #[cfg(all(feature = "simulation", madsim))]
    {
        // Deterministic RNG when running inside a madsim runtime; otherwise
        // (e.g. plain `#[rstest]` tests under `cfg(madsim)`) fall back to the
        // host RNG. Production paths under simulation always run inside a
        // runtime, so they continue to consume seeded bytes.
        if madsim::runtime::Handle::try_current().is_ok() {
            let mut seed = [0u8; 32];
            madsim::rand::thread_rng().fill_bytes(&mut seed);
            return StdRng::from_seed(seed);
        }
    }

    StdRng::from_rng(&mut rand::rng()) // dst-ok: outside madsim runtime
}

fn build_l2_book(instrument_id: InstrumentId) -> OrderBook {
    OrderBook::new(instrument_id, BookType::L2_MBP)
}

fn add_order(book: &mut OrderBook, side: OrderSide, price: Price, size: Quantity, order_id: u64) {
    let order = BookOrder::new(side, price, size, order_id);
    book.add(order, 0, 0, UnixNanos::default());
}

#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct DefaultFillModel {
    state: ProbabilisticFillState,
}

impl DefaultFillModel {
    /// Creates a new [`DefaultFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
        })
    }
}

impl Clone for DefaultFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl Default for DefaultFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl Display for DefaultFillModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DefaultFillModel(prob_fill_on_limit={}, prob_slippage={})",
            self.state.prob_fill_on_limit, self.state.prob_slippage
        )
    }
}

impl FillModel for DefaultFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        _instrument: &InstrumentAny,
        _order: &OrderAny,
        _best_bid: Option<Price>,
        _best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        Ok(None)
    }
}

/// Fill model that executes all orders at the best available price with unlimited liquidity.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct BestPriceFillModel {
    state: ProbabilisticFillState,
}

impl BestPriceFillModel {
    /// Creates a new [`BestPriceFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
        })
    }
}

impl Clone for BestPriceFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl Default for BestPriceFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl FillModel for BestPriceFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn fill_limit_inside_spread(&self) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        _order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let mut book = build_l2_book(instrument.id());
        let size_prec = instrument.size_precision();
        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid,
            unlimited_liquidity(size_prec),
            1,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask,
            unlimited_liquidity(size_prec),
            2,
        );
        Ok(Some(book))
    }
}

/// Fill model that forces exactly one tick of slippage for all orders.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct OneTickSlippageFillModel {
    state: ProbabilisticFillState,
}

impl OneTickSlippageFillModel {
    /// Creates a new [`OneTickSlippageFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
        })
    }
}

impl Clone for OneTickSlippageFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl Default for OneTickSlippageFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl FillModel for OneTickSlippageFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        _order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let tick = instrument.price_increment();
        let size_prec = instrument.size_precision();
        let mut book = build_l2_book(instrument.id());

        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid - tick,
            unlimited_liquidity(size_prec),
            1,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask + tick,
            unlimited_liquidity(size_prec),
            2,
        );
        Ok(Some(book))
    }
}

/// Fill model with 50/50 chance of best price fill or one tick slippage.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct ProbabilisticFillModel {
    state: ProbabilisticFillState,
}

impl ProbabilisticFillModel {
    /// Creates a new [`ProbabilisticFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
        })
    }
}

impl Clone for ProbabilisticFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl Default for ProbabilisticFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl FillModel for ProbabilisticFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        _order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let tick = instrument.price_increment();
        let size_prec = instrument.size_precision();
        let mut book = build_l2_book(instrument.id());

        if self.state.random_bool(0.5) {
            add_order(
                &mut book,
                OrderSide::Buy,
                best_bid,
                unlimited_liquidity(size_prec),
                1,
            );
            add_order(
                &mut book,
                OrderSide::Sell,
                best_ask,
                unlimited_liquidity(size_prec),
                2,
            );
        } else {
            add_order(
                &mut book,
                OrderSide::Buy,
                best_bid - tick,
                unlimited_liquidity(size_prec),
                1,
            );
            add_order(
                &mut book,
                OrderSide::Sell,
                best_ask + tick,
                unlimited_liquidity(size_prec),
                2,
            );
        }
        Ok(Some(book))
    }
}

/// Fill model with two tiers: first 10 contracts at best price, remainder one tick worse.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct TwoTierFillModel {
    state: ProbabilisticFillState,
}

impl TwoTierFillModel {
    /// Creates a new [`TwoTierFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
        })
    }
}

impl Clone for TwoTierFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl Default for TwoTierFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl FillModel for TwoTierFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        _order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let tick = instrument.price_increment();
        let size_prec = instrument.size_precision();
        let mut book = build_l2_book(instrument.id());

        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid,
            Quantity::new(10.0, size_prec),
            1,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask,
            Quantity::new(10.0, size_prec),
            2,
        );
        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid - tick,
            unlimited_liquidity(size_prec),
            3,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask + tick,
            unlimited_liquidity(size_prec),
            4,
        );
        Ok(Some(book))
    }
}

/// Fill model with three tiers: 50 at best, 30 at +1 tick, 20 at +2 ticks.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct ThreeTierFillModel {
    state: ProbabilisticFillState,
}

impl ThreeTierFillModel {
    /// Creates a new [`ThreeTierFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
        })
    }
}

impl Clone for ThreeTierFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl Default for ThreeTierFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl FillModel for ThreeTierFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        _order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let tick = instrument.price_increment();
        let two_ticks = tick + tick;
        let size_prec = instrument.size_precision();
        let mut book = build_l2_book(instrument.id());

        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid,
            Quantity::new(50.0, size_prec),
            1,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask,
            Quantity::new(50.0, size_prec),
            2,
        );
        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid - tick,
            Quantity::new(30.0, size_prec),
            3,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask + tick,
            Quantity::new(30.0, size_prec),
            4,
        );
        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid - two_ticks,
            Quantity::new(20.0, size_prec),
            5,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask + two_ticks,
            Quantity::new(20.0, size_prec),
            6,
        );
        Ok(Some(book))
    }
}

/// Fill model that simulates partial fills: max 5 contracts at best, unlimited one tick worse.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct LimitOrderPartialFillModel {
    state: ProbabilisticFillState,
}

impl LimitOrderPartialFillModel {
    /// Creates a new [`LimitOrderPartialFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
        })
    }
}

impl Clone for LimitOrderPartialFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl Default for LimitOrderPartialFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl FillModel for LimitOrderPartialFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        _order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let tick = instrument.price_increment();
        let size_prec = instrument.size_precision();
        let mut book = build_l2_book(instrument.id());

        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid,
            Quantity::new(5.0, size_prec),
            1,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask,
            Quantity::new(5.0, size_prec),
            2,
        );
        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid - tick,
            unlimited_liquidity(size_prec),
            3,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask + tick,
            unlimited_liquidity(size_prec),
            4,
        );
        Ok(Some(book))
    }
}

/// Fill model that applies different execution based on order size.
/// Small orders (<=10) get 50 contracts at best. Large orders get 10 at best, remainder at +1 tick.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct SizeAwareFillModel {
    state: ProbabilisticFillState,
}

impl SizeAwareFillModel {
    /// Creates a new [`SizeAwareFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
        })
    }
}

impl Clone for SizeAwareFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl Default for SizeAwareFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl FillModel for SizeAwareFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let tick = instrument.price_increment();
        let size_prec = instrument.size_precision();
        let mut book = build_l2_book(instrument.id());

        let threshold = Quantity::new(10.0, size_prec);
        if order.quantity() <= threshold {
            // Small orders: good liquidity at best
            add_order(
                &mut book,
                OrderSide::Buy,
                best_bid,
                Quantity::new(50.0, size_prec),
                1,
            );
            add_order(
                &mut book,
                OrderSide::Sell,
                best_ask,
                Quantity::new(50.0, size_prec),
                2,
            );
        } else {
            // Large orders: price impact
            let remaining = order.quantity() - threshold;
            add_order(&mut book, OrderSide::Buy, best_bid, threshold, 1);
            add_order(&mut book, OrderSide::Sell, best_ask, threshold, 2);
            add_order(&mut book, OrderSide::Buy, best_bid - tick, remaining, 3);
            add_order(&mut book, OrderSide::Sell, best_ask + tick, remaining, 4);
        }
        Ok(Some(book))
    }
}

/// Fill model that reduces available liquidity by a factor to simulate market competition.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct CompetitionAwareFillModel {
    state: ProbabilisticFillState,
    liquidity_factor: Decimal,
}

impl CompetitionAwareFillModel {
    /// Creates a new [`CompetitionAwareFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters or `liquidity_factor` are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
        liquidity_factor: f64,
    ) -> anyhow::Result<Self> {
        let state = ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?;
        check_in_range_inclusive_f64(liquidity_factor, 0.0, 1.0, "liquidity_factor")?;
        let liquidity_factor = Decimal::try_from(liquidity_factor)?;

        Ok(Self {
            state,
            liquidity_factor,
        })
    }
}

impl Clone for CompetitionAwareFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            liquidity_factor: self.liquidity_factor,
        }
    }
}

impl Default for CompetitionAwareFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None, DEFAULT_LIQUIDITY_FACTOR).unwrap()
    }
}

impl FillModel for CompetitionAwareFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        _order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let size_prec = instrument.size_precision();
        let mut book = build_l2_book(instrument.id());

        // Minimum 1 to avoid zero-size orders
        let available = Quantity::from_decimal_dp(
            (dec!(1000) * self.liquidity_factor).max(Decimal::ONE),
            size_prec,
        )?;

        add_order(&mut book, OrderSide::Buy, best_bid, available, 1);
        add_order(&mut book, OrderSide::Sell, best_ask, available, 2);
        Ok(Some(book))
    }
}

/// Fill model that adjusts liquidity based on recent trading volume.
/// Uses 25% of recent volume at best price, unlimited one tick worse.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct VolumeSensitiveFillModel {
    state: ProbabilisticFillState,
    recent_volume: f64,
}

impl VolumeSensitiveFillModel {
    /// Creates a new [`VolumeSensitiveFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
            recent_volume: 1000.0,
        })
    }

    pub fn set_recent_volume(&mut self, volume: f64) {
        self.recent_volume = volume;
    }
}

impl Clone for VolumeSensitiveFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            recent_volume: self.recent_volume,
        }
    }
}

impl Default for VolumeSensitiveFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl FillModel for VolumeSensitiveFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        _order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let tick = instrument.price_increment();
        let size_prec = instrument.size_precision();
        let mut book = build_l2_book(instrument.id());

        check_non_negative_f64(self.recent_volume, "recent_volume")?;
        let recent_volume = Decimal::try_from(self.recent_volume)?;

        // Minimum 1 to avoid zero-size orders
        let available =
            Quantity::from_decimal_dp((recent_volume * dec!(0.25)).max(Decimal::ONE), size_prec)?;

        add_order(&mut book, OrderSide::Buy, best_bid, available, 1);
        add_order(&mut book, OrderSide::Sell, best_ask, available, 2);
        add_order(
            &mut book,
            OrderSide::Buy,
            best_bid - tick,
            unlimited_liquidity(size_prec),
            3,
        );
        add_order(
            &mut book,
            OrderSide::Sell,
            best_ask + tick,
            unlimited_liquidity(size_prec),
            4,
        );
        Ok(Some(book))
    }
}

/// Fill model that simulates varying conditions based on market hours.
/// During low liquidity: wider spreads (one tick worse). Normal hours: standard liquidity.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct MarketHoursFillModel {
    state: ProbabilisticFillState,
    is_low_liquidity: bool,
}

impl MarketHoursFillModel {
    /// Creates a new [`MarketHoursFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if probability parameters are not in range [0, 1].
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?,
            is_low_liquidity: false,
        })
    }

    pub fn set_low_liquidity_period(&mut self, is_low_liquidity: bool) {
        self.is_low_liquidity = is_low_liquidity;
    }

    pub fn is_low_liquidity_period(&self) -> bool {
        self.is_low_liquidity
    }
}

impl Clone for MarketHoursFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            is_low_liquidity: self.is_low_liquidity,
        }
    }
}

impl Default for MarketHoursFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None).unwrap()
    }
}

impl FillModel for MarketHoursFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        _order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        let (Some(best_bid), Some(best_ask)) = (best_bid, best_ask) else {
            return Ok(None);
        };

        let tick = instrument.price_increment();
        let size_prec = instrument.size_precision();
        let mut book = build_l2_book(instrument.id());
        let normal_volume = 500.0;

        if self.is_low_liquidity {
            add_order(
                &mut book,
                OrderSide::Buy,
                best_bid - tick,
                Quantity::new(normal_volume, size_prec),
                1,
            );
            add_order(
                &mut book,
                OrderSide::Sell,
                best_ask + tick,
                Quantity::new(normal_volume, size_prec),
                2,
            );
        } else {
            add_order(
                &mut book,
                OrderSide::Buy,
                best_bid,
                Quantity::new(normal_volume, size_prec),
                1,
            );
            add_order(
                &mut book,
                OrderSide::Sell,
                best_ask,
                Quantity::new(normal_volume, size_prec),
                2,
            );
        }
        Ok(Some(book))
    }
}

/// Fill model whose passive fill probability falls with queue ahead and adverse flow.
///
/// A resting order fills less often when more quantity sits ahead of it at its price and when
/// recent aggressive trade flow is against its side. The fill probability is
///
/// ```text
/// p = prob_fill_on_limit
///     * exp(-(queue_sensitivity * queue_ahead / (queue_ahead + order_quantity)
///             + toxicity_sensitivity * max(toxicity, 0)))
/// ```
///
/// so it equals `prob_fill_on_limit` with no queue ahead and no adverse flow, and falls
/// monotonically as either signal turns against the order. The `queue_ahead / (queue_ahead +
/// order_quantity)` ratio is in `[0, 1)` and measures the order's place in the queue relative to
/// its own size; only adverse (positive) toxicity shortens the fill, while favourable flow leaves
/// the base probability in place. Slippage draws at `prob_slippage`, as for the other
/// probabilistic models.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct AdverseSelectionFillModel {
    state: ProbabilisticFillState,
    queue_sensitivity: f64,
    toxicity_sensitivity: f64,
}

impl AdverseSelectionFillModel {
    /// Creates a new [`AdverseSelectionFillModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `prob_fill_on_limit` or `prob_slippage` is not in range `[0, 1]`, or
    /// if `queue_sensitivity` or `toxicity_sensitivity` is not finite and non-negative.
    pub fn new(
        prob_fill_on_limit: f64,
        prob_slippage: f64,
        random_seed: Option<u64>,
        queue_sensitivity: f64,
        toxicity_sensitivity: f64,
    ) -> anyhow::Result<Self> {
        let state = ProbabilisticFillState::new(prob_fill_on_limit, prob_slippage, random_seed)?;
        check_non_negative_f64(queue_sensitivity, "queue_sensitivity")?;
        check_non_negative_f64(toxicity_sensitivity, "toxicity_sensitivity")?;

        Ok(Self {
            state,
            queue_sensitivity,
            toxicity_sensitivity,
        })
    }

    /// Returns the passive fill probability for the given `context`.
    ///
    /// The value is `prob_fill_on_limit` when there is no queue ahead and no adverse flow, and
    /// falls monotonically as either the queue ahead grows relative to the order's own quantity
    /// or as the toxicity turns against the order's side. Non-finite sensitivity parameters are
    /// rejected by the constructor, so the result is always finite and within `[0, 1]`.
    #[must_use]
    pub fn fill_probability(&self, context: &PassiveFillContext) -> f64 {
        let queue_ahead = context.queue_ahead.as_f64();
        let order_quantity = context.order_quantity.as_f64();
        let total = queue_ahead + order_quantity;
        let queue_ratio = if total > 0.0 {
            queue_ahead / total
        } else {
            0.0
        };
        let adverse_toxicity = context.toxicity.max(0.0);
        let decay = (-(self.queue_sensitivity * queue_ratio
            + self.toxicity_sensitivity * adverse_toxicity))
            .exp();

        (self.state.prob_fill_on_limit * decay).clamp(0.0, 1.0)
    }
}

impl Clone for AdverseSelectionFillModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            queue_sensitivity: self.queue_sensitivity,
            toxicity_sensitivity: self.toxicity_sensitivity,
        }
    }
}

impl Default for AdverseSelectionFillModel {
    fn default() -> Self {
        Self::new(1.0, 0.0, None, 1.0, 1.0).unwrap()
    }
}

impl FillModel for AdverseSelectionFillModel {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_limit_filled())
    }

    fn is_limit_filled_with_context(
        &mut self,
        context: &PassiveFillContext,
    ) -> anyhow::Result<bool> {
        let probability = self.fill_probability(context);
        Ok(self.state.random_bool(probability))
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }

    fn random_seed(&self) -> Option<u64> {
        self.state.random_seed()
    }

    fn seed_if_unset(&mut self, seed: u64) {
        if self.state.random_seed().is_none() {
            self.state.reseed(seed);
        }
    }

    fn get_orderbook_for_fill_simulation(
        &mut self,
        _instrument: &InstrumentAny,
        _order: &OrderAny,
        _best_bid: Option<Price>,
        _best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        Ok(None)
    }
}

#[derive(Clone, Debug)]
pub enum FillModelAny {
    Default(DefaultFillModel),
    BestPrice(BestPriceFillModel),
    OneTickSlippage(OneTickSlippageFillModel),
    Probabilistic(ProbabilisticFillModel),
    TwoTier(TwoTierFillModel),
    ThreeTier(ThreeTierFillModel),
    LimitOrderPartialFill(LimitOrderPartialFillModel),
    SizeAware(SizeAwareFillModel),
    CompetitionAware(CompetitionAwareFillModel),
    VolumeSensitive(VolumeSensitiveFillModel),
    MarketHours(MarketHoursFillModel),
    AdverseSelection(AdverseSelectionFillModel),
}

impl FillModel for FillModelAny {
    fn is_limit_filled(&mut self) -> anyhow::Result<bool> {
        match self {
            Self::Default(m) => m.is_limit_filled(),
            Self::BestPrice(m) => m.is_limit_filled(),
            Self::OneTickSlippage(m) => m.is_limit_filled(),
            Self::Probabilistic(m) => m.is_limit_filled(),
            Self::TwoTier(m) => m.is_limit_filled(),
            Self::ThreeTier(m) => m.is_limit_filled(),
            Self::LimitOrderPartialFill(m) => m.is_limit_filled(),
            Self::SizeAware(m) => m.is_limit_filled(),
            Self::CompetitionAware(m) => m.is_limit_filled(),
            Self::VolumeSensitive(m) => m.is_limit_filled(),
            Self::MarketHours(m) => m.is_limit_filled(),
            Self::AdverseSelection(m) => m.is_limit_filled(),
        }
    }

    fn is_limit_filled_with_context(
        &mut self,
        context: &PassiveFillContext,
    ) -> anyhow::Result<bool> {
        match self {
            Self::Default(m) => m.is_limit_filled_with_context(context),
            Self::BestPrice(m) => m.is_limit_filled_with_context(context),
            Self::OneTickSlippage(m) => m.is_limit_filled_with_context(context),
            Self::Probabilistic(m) => m.is_limit_filled_with_context(context),
            Self::TwoTier(m) => m.is_limit_filled_with_context(context),
            Self::ThreeTier(m) => m.is_limit_filled_with_context(context),
            Self::LimitOrderPartialFill(m) => m.is_limit_filled_with_context(context),
            Self::SizeAware(m) => m.is_limit_filled_with_context(context),
            Self::CompetitionAware(m) => m.is_limit_filled_with_context(context),
            Self::VolumeSensitive(m) => m.is_limit_filled_with_context(context),
            Self::MarketHours(m) => m.is_limit_filled_with_context(context),
            Self::AdverseSelection(m) => m.is_limit_filled_with_context(context),
        }
    }

    fn fill_limit_inside_spread(&self) -> anyhow::Result<bool> {
        match self {
            Self::Default(m) => m.fill_limit_inside_spread(),
            Self::BestPrice(m) => m.fill_limit_inside_spread(),
            Self::OneTickSlippage(m) => m.fill_limit_inside_spread(),
            Self::Probabilistic(m) => m.fill_limit_inside_spread(),
            Self::TwoTier(m) => m.fill_limit_inside_spread(),
            Self::ThreeTier(m) => m.fill_limit_inside_spread(),
            Self::LimitOrderPartialFill(m) => m.fill_limit_inside_spread(),
            Self::SizeAware(m) => m.fill_limit_inside_spread(),
            Self::CompetitionAware(m) => m.fill_limit_inside_spread(),
            Self::VolumeSensitive(m) => m.fill_limit_inside_spread(),
            Self::MarketHours(m) => m.fill_limit_inside_spread(),
            Self::AdverseSelection(m) => m.fill_limit_inside_spread(),
        }
    }

    fn random_seed(&self) -> Option<u64> {
        match self {
            Self::Default(m) => m.random_seed(),
            Self::BestPrice(m) => m.random_seed(),
            Self::OneTickSlippage(m) => m.random_seed(),
            Self::Probabilistic(m) => m.random_seed(),
            Self::TwoTier(m) => m.random_seed(),
            Self::ThreeTier(m) => m.random_seed(),
            Self::LimitOrderPartialFill(m) => m.random_seed(),
            Self::SizeAware(m) => m.random_seed(),
            Self::CompetitionAware(m) => m.random_seed(),
            Self::VolumeSensitive(m) => m.random_seed(),
            Self::MarketHours(m) => m.random_seed(),
            Self::AdverseSelection(m) => m.random_seed(),
        }
    }

    fn seed_if_unset(&mut self, seed: u64) {
        match self {
            Self::Default(m) => m.seed_if_unset(seed),
            Self::BestPrice(m) => m.seed_if_unset(seed),
            Self::OneTickSlippage(m) => m.seed_if_unset(seed),
            Self::Probabilistic(m) => m.seed_if_unset(seed),
            Self::TwoTier(m) => m.seed_if_unset(seed),
            Self::ThreeTier(m) => m.seed_if_unset(seed),
            Self::LimitOrderPartialFill(m) => m.seed_if_unset(seed),
            Self::SizeAware(m) => m.seed_if_unset(seed),
            Self::CompetitionAware(m) => m.seed_if_unset(seed),
            Self::VolumeSensitive(m) => m.seed_if_unset(seed),
            Self::MarketHours(m) => m.seed_if_unset(seed),
            Self::AdverseSelection(m) => m.seed_if_unset(seed),
        }
    }

    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        match self {
            Self::Default(m) => m.is_slipped(),
            Self::BestPrice(m) => m.is_slipped(),
            Self::OneTickSlippage(m) => m.is_slipped(),
            Self::Probabilistic(m) => m.is_slipped(),
            Self::TwoTier(m) => m.is_slipped(),
            Self::ThreeTier(m) => m.is_slipped(),
            Self::LimitOrderPartialFill(m) => m.is_slipped(),
            Self::SizeAware(m) => m.is_slipped(),
            Self::CompetitionAware(m) => m.is_slipped(),
            Self::VolumeSensitive(m) => m.is_slipped(),
            Self::MarketHours(m) => m.is_slipped(),
            Self::AdverseSelection(m) => m.is_slipped(),
        }
    }

    #[rustfmt::skip]
    fn get_orderbook_for_fill_simulation(
        &mut self,
        instrument: &InstrumentAny,
        order: &OrderAny,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
    ) -> anyhow::Result<Option<OrderBook>> {
        match self {
            Self::Default(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::BestPrice(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::OneTickSlippage(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::Probabilistic(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::TwoTier(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::ThreeTier(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::LimitOrderPartialFill(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::SizeAware(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::CompetitionAware(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::VolumeSensitive(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::MarketHours(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
            Self::AdverseSelection(m) => m.get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask),
        }
    }
}

impl Default for FillModelAny {
    fn default() -> Self {
        Self::Default(DefaultFillModel::default())
    }
}

impl Display for FillModelAny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default(m) => write!(f, "{m}"),
            Self::BestPrice(_) => write!(f, "BestPriceFillModel"),
            Self::OneTickSlippage(_) => write!(f, "OneTickSlippageFillModel"),
            Self::Probabilistic(_) => write!(f, "ProbabilisticFillModel"),
            Self::TwoTier(_) => write!(f, "TwoTierFillModel"),
            Self::ThreeTier(_) => write!(f, "ThreeTierFillModel"),
            Self::LimitOrderPartialFill(_) => write!(f, "LimitOrderPartialFillModel"),
            Self::SizeAware(_) => write!(f, "SizeAwareFillModel"),
            Self::CompetitionAware(_) => write!(f, "CompetitionAwareFillModel"),
            Self::VolumeSensitive(_) => write!(f, "VolumeSensitiveFillModel"),
            Self::MarketHours(_) => write!(f, "MarketHoursFillModel"),
            Self::AdverseSelection(_) => write!(f, "AdverseSelectionFillModel"),
        }
    }
}

// The `CompetitionAwareFillModel` default liquidity factor, applied when a fill model
// configuration does not supply one.
const DEFAULT_LIQUIDITY_FACTOR: f64 = 0.3;

// The `AdverseSelectionFillModel` default sensitivities, applied when a fill model
// configuration does not supply the sensitivities its constructor takes.
const DEFAULT_QUEUE_SENSITIVITY: f64 = 1.0;
const DEFAULT_TOXICITY_SENSITIVITY: f64 = 1.0;

/// The built-in fill models selectable by configuration.
///
/// The variants correspond one-for-one with the [`FillModelAny`] runtime variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        eq_int,
        module = "nautilus_trader.execution",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.execution")
)]
pub enum FillModelKind {
    /// The default model: fills against the matching engine's recorded book.
    Default,
    /// Fills at the best bid or ask with unlimited size.
    BestPrice,
    /// Fills one tick beyond the best bid or ask with unlimited size.
    OneTickSlippage,
    /// Chooses the best price or one tick worse.
    Probabilistic,
    /// Fills 10 units at best, then the remainder one tick worse.
    TwoTier,
    /// Fills 50, 30, and 20 units across three levels.
    ThreeTier,
    /// Fills 5 units at best, then the remainder one tick worse.
    LimitOrderPartialFill,
    /// Changes the synthetic book shape at an order size of 10 units.
    SizeAware,
    /// Exposes a configurable fraction of 1,000 units at best.
    CompetitionAware,
    /// Exposes 25% of its recent volume at best.
    VolumeSensitive,
    /// Uses a normal or one-tick-wider synthetic spread.
    MarketHours,
    /// Reduces the passive fill probability with queue ahead and adverse trade flow.
    ///
    /// Resolves with the model's default queue and toxicity sensitivities.
    AdverseSelection,
}

/// A configuration description of a built-in fill model.
///
/// This is the description side of the fill model set. [`FillModelKind`] names one of the
/// built-in models and the remaining fields carry the parameters that model's constructor
/// takes, so a fill model can be described as data, compared, and resolved once with
/// [`FillModelConfig::resolve`].
///
/// Resolution delegates to the existing model constructors: no fill behaviour lives here, and
/// the resolved model is exactly the one its constructor builds. A model object passed
/// directly (a [`FillModelAny`], or a Python model object) remains accepted wherever a fill
/// model is configured, so existing configurations are unaffected.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct FillModelConfig {
    /// Which built-in fill model to resolve to.
    pub kind: FillModelKind,
    /// The probability a limit order fills when the market touches, but does not cross, its price.
    pub prob_fill_on_limit: f64,
    /// The probability a fill is slipped by one tick on an L1 book.
    pub prob_slippage: f64,
    /// The optional seed for the model's random draws.
    pub random_seed: Option<u64>,
    /// The liquidity factor for [`FillModelKind::CompetitionAware`].
    ///
    /// `None` uses the model default. Supplying a factor for any other kind is an error
    /// rather than a silently ignored setting.
    pub liquidity_factor: Option<f64>,
}

impl Default for FillModelConfig {
    fn default() -> Self {
        Self {
            kind: FillModelKind::Default,
            prob_fill_on_limit: 1.0,
            prob_slippage: 0.0,
            random_seed: None,
            liquidity_factor: None,
        }
    }
}

impl FillModelConfig {
    /// Resolves the configuration into the built-in fill model implementation.
    ///
    /// Each kind is constructed with that model's own constructor, so parameter validation and
    /// the resulting model are exactly those of the corresponding model type. The default
    /// configuration resolves to the default fill model.
    ///
    /// # Errors
    ///
    /// Returns an error if `prob_fill_on_limit`, `prob_slippage`, or `liquidity_factor` is
    /// outside `[0, 1]`, or if `liquidity_factor` is supplied for a kind that does not
    /// consume it.
    pub fn resolve(&self) -> anyhow::Result<FillModelAny> {
        if self.liquidity_factor.is_some() && self.kind != FillModelKind::CompetitionAware {
            anyhow::bail!(
                "liquidity_factor is only consumed by the CompetitionAware fill model, was supplied for {}",
                self.kind
            );
        }

        let prob_fill_on_limit = self.prob_fill_on_limit;
        let prob_slippage = self.prob_slippage;
        let random_seed = self.random_seed;

        Ok(match self.kind {
            FillModelKind::Default => FillModelAny::Default(DefaultFillModel::new(
                prob_fill_on_limit,
                prob_slippage,
                random_seed,
            )?),
            FillModelKind::BestPrice => FillModelAny::BestPrice(BestPriceFillModel::new(
                prob_fill_on_limit,
                prob_slippage,
                random_seed,
            )?),
            FillModelKind::OneTickSlippage => FillModelAny::OneTickSlippage(
                OneTickSlippageFillModel::new(prob_fill_on_limit, prob_slippage, random_seed)?,
            ),
            FillModelKind::Probabilistic => FillModelAny::Probabilistic(
                ProbabilisticFillModel::new(prob_fill_on_limit, prob_slippage, random_seed)?,
            ),
            FillModelKind::TwoTier => FillModelAny::TwoTier(TwoTierFillModel::new(
                prob_fill_on_limit,
                prob_slippage,
                random_seed,
            )?),
            FillModelKind::ThreeTier => FillModelAny::ThreeTier(ThreeTierFillModel::new(
                prob_fill_on_limit,
                prob_slippage,
                random_seed,
            )?),
            FillModelKind::LimitOrderPartialFill => FillModelAny::LimitOrderPartialFill(
                LimitOrderPartialFillModel::new(prob_fill_on_limit, prob_slippage, random_seed)?,
            ),
            FillModelKind::SizeAware => FillModelAny::SizeAware(SizeAwareFillModel::new(
                prob_fill_on_limit,
                prob_slippage,
                random_seed,
            )?),
            FillModelKind::CompetitionAware => {
                let liquidity_factor = self.liquidity_factor.unwrap_or(DEFAULT_LIQUIDITY_FACTOR);

                FillModelAny::CompetitionAware(CompetitionAwareFillModel::new(
                    prob_fill_on_limit,
                    prob_slippage,
                    random_seed,
                    liquidity_factor,
                )?)
            }
            FillModelKind::VolumeSensitive => FillModelAny::VolumeSensitive(
                VolumeSensitiveFillModel::new(prob_fill_on_limit, prob_slippage, random_seed)?,
            ),
            FillModelKind::MarketHours => FillModelAny::MarketHours(MarketHoursFillModel::new(
                prob_fill_on_limit,
                prob_slippage,
                random_seed,
            )?),
            FillModelKind::AdverseSelection => {
                let model = AdverseSelectionFillModel::new(
                    prob_fill_on_limit,
                    prob_slippage,
                    random_seed,
                    DEFAULT_QUEUE_SENSITIVITY,
                    DEFAULT_TOXICITY_SENSITIVITY,
                )?;

                FillModelAny::AdverseSelection(model)
            }
        })
    }
}

impl Display for FillModelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "DefaultFillModel"),
            Self::BestPrice => write!(f, "BestPriceFillModel"),
            Self::OneTickSlippage => write!(f, "OneTickSlippageFillModel"),
            Self::Probabilistic => write!(f, "ProbabilisticFillModel"),
            Self::TwoTier => write!(f, "TwoTierFillModel"),
            Self::ThreeTier => write!(f, "ThreeTierFillModel"),
            Self::LimitOrderPartialFill => write!(f, "LimitOrderPartialFillModel"),
            Self::SizeAware => write!(f, "SizeAwareFillModel"),
            Self::CompetitionAware => write!(f, "CompetitionAwareFillModel"),
            Self::VolumeSensitive => write!(f, "VolumeSensitiveFillModel"),
            Self::MarketHours => write!(f, "MarketHoursFillModel"),
            Self::AdverseSelection => write!(f, "AdverseSelectionFillModel"),
        }
    }
}

/// The fill model selection for a venue, resolved per instrument as an inheritance chain.
///
/// The selection levels, from least to most specific, are the global default, the venue
/// default, and the per-instrument override:
///
/// 1. The global default is the built-in fill model. It is what a venue inherits when the
///    venue configuration sets no fill model.
/// 2. The venue default is set by the venue configuration, or replaced at runtime for a
///    backtest venue.
/// 3. The per-instrument override is set by the venue configuration for a named instrument.
///
/// A level that sets a model overrides every less specific level, and a level that sets
/// nothing inherits, so an instrument without an override resolves to the venue default and a
/// venue without a default resolves to the built-in model. Resolution happens once, when the
/// matching engine for an instrument is created; the engine then holds that model, which is
/// the matching-engine level of the chain and can be replaced at runtime.
///
/// Order-specific overrides are not part of the selection: an order carries no fill model
/// affiliation, so there is no level to declare and nothing to inherit.
#[derive(Clone, Debug, Default)]
pub struct FillModelSelection {
    venue_default: FillModelHandle,
    instrument_overrides: ahash::AHashMap<InstrumentId, FillModelHandle>,
}

impl FillModelSelection {
    /// Creates a new [`FillModelSelection`] from a venue default and per-instrument overrides.
    #[must_use]
    pub fn new(
        venue_default: FillModelHandle,
        instrument_overrides: ahash::AHashMap<InstrumentId, FillModelHandle>,
    ) -> Self {
        Self {
            venue_default,
            instrument_overrides,
        }
    }

    /// Returns the venue default, the level below the per-instrument overrides.
    #[must_use]
    pub fn venue_default(&self) -> &FillModelHandle {
        &self.venue_default
    }

    /// Replaces the venue default, leaving the per-instrument overrides in place.
    pub fn set_venue_default(&mut self, fill_model: FillModelHandle) {
        self.venue_default = fill_model;
    }

    /// Sets the per-instrument override, replacing any existing override for the instrument.
    pub fn set_instrument_override(
        &mut self,
        instrument_id: InstrumentId,
        fill_model: FillModelHandle,
    ) {
        self.instrument_overrides.insert(instrument_id, fill_model);
    }

    /// Returns `true` if the instrument has an override instead of inheriting the venue default.
    #[must_use]
    pub fn has_instrument_override(&self, instrument_id: &InstrumentId) -> bool {
        self.instrument_overrides.contains_key(instrument_id)
    }

    /// Resolves the fill model for an instrument: its override if set, else the venue default.
    #[must_use]
    pub fn resolve(&self, instrument_id: &InstrumentId) -> FillModelHandle {
        self.instrument_overrides
            .get(instrument_id)
            .cloned()
            .unwrap_or_else(|| self.venue_default.clone())
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::correctness::CorrectnessError;
    use nautilus_model::{
        enums::OrderType,
        instruments::stubs::{audusd_sim, crypto_perpetual_ethusdt},
        orders::builder::OrderTestBuilder,
    };
    use rstest::{fixture, rstest};

    use super::*;

    #[fixture]
    fn fill_model() -> DefaultFillModel {
        let seed = 42;
        DefaultFillModel::new(0.5, 0.1, Some(seed)).unwrap()
    }

    #[rstest]
    fn test_fill_model_display(fill_model: DefaultFillModel) {
        assert_eq!(
            format!("{fill_model}"),
            "DefaultFillModel(prob_fill_on_limit=0.5, prob_slippage=0.1)"
        );
    }

    #[rstest]
    fn test_fill_model_param_prob_fill_on_limit_error() {
        let error = DefaultFillModel::new(1.1, 0.1, None).unwrap_err();

        assert_eq!(
            error.downcast_ref::<CorrectnessError>(),
            Some(&CorrectnessError::OutOfRange {
                param: "prob_fill_on_limit".to_string(),
                min: "0".to_string(),
                max: "1".to_string(),
                value: "1.1".to_string(),
                type_name: "f64",
            })
        );
        assert_eq!(
            error.to_string(),
            "invalid f64 for 'prob_fill_on_limit' not in range [0, 1], was 1.1"
        );
    }

    #[rstest]
    fn test_fill_model_param_prob_slippage_error() {
        let error = DefaultFillModel::new(0.5, 1.1, None).unwrap_err();

        assert_eq!(
            error.downcast_ref::<CorrectnessError>(),
            Some(&CorrectnessError::OutOfRange {
                param: "prob_slippage".to_string(),
                min: "0".to_string(),
                max: "1".to_string(),
                value: "1.1".to_string(),
                type_name: "f64",
            })
        );
        assert_eq!(
            error.to_string(),
            "invalid f64 for 'prob_slippage' not in range [0, 1], was 1.1"
        );
    }

    #[rstest]
    #[case(f64::NAN, "NaN")]
    #[case(f64::INFINITY, "inf")]
    #[case(f64::NEG_INFINITY, "-inf")]
    fn test_competition_aware_fill_model_rejects_non_finite_liquidity_factor(
        #[case] value: f64,
        #[case] expected_value: &str,
    ) {
        let error = CompetitionAwareFillModel::new(1.0, 0.0, None, value).unwrap_err();

        assert_eq!(
            error.downcast_ref::<CorrectnessError>(),
            Some(&CorrectnessError::InvalidValue {
                param: "liquidity_factor".to_string(),
                value: expected_value.to_string(),
                type_name: "f64",
            })
        );
    }

    #[rstest]
    #[case(-0.1, "-0.1")]
    #[case(1.1, "1.1")]
    fn test_competition_aware_fill_model_rejects_out_of_range_liquidity_factor(
        #[case] value: f64,
        #[case] expected_value: &str,
    ) {
        let error = CompetitionAwareFillModel::new(1.0, 0.0, None, value).unwrap_err();

        assert_eq!(
            error.downcast_ref::<CorrectnessError>(),
            Some(&CorrectnessError::OutOfRange {
                param: "liquidity_factor".to_string(),
                min: "0".to_string(),
                max: "1".to_string(),
                value: expected_value.to_string(),
                type_name: "f64",
            })
        );
    }

    #[rstest]
    #[case(f64::NAN, "NaN")]
    #[case(f64::INFINITY, "inf")]
    #[case(f64::NEG_INFINITY, "-inf")]
    fn test_volume_sensitive_fill_model_rejects_non_finite_volume(
        #[case] volume: f64,
        #[case] expected_value: &str,
    ) {
        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(100_000))
            .build();
        let mut model = VolumeSensitiveFillModel::default();
        model.set_recent_volume(volume);

        let error = model
            .get_orderbook_for_fill_simulation(
                &instrument,
                &order,
                Some(Price::from("0.80000")),
                Some(Price::from("0.80010")),
            )
            .unwrap_err();

        assert_eq!(
            error.downcast_ref::<CorrectnessError>(),
            Some(&CorrectnessError::InvalidValue {
                param: "recent_volume".to_string(),
                value: expected_value.to_string(),
                type_name: "f64",
            })
        );
    }

    #[rstest]
    fn test_volume_sensitive_fill_model_rejects_negative_volume() {
        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(100_000))
            .build();
        let mut model = VolumeSensitiveFillModel::default();
        model.set_recent_volume(-1.0);

        let error = model
            .get_orderbook_for_fill_simulation(
                &instrument,
                &order,
                Some(Price::from("0.80000")),
                Some(Price::from("0.80010")),
            )
            .unwrap_err();

        assert_eq!(
            error.downcast_ref::<CorrectnessError>(),
            Some(&CorrectnessError::NegativeValue {
                param: "recent_volume".to_string(),
                value: "-1".to_string(),
                type_name: "f64",
            })
        );
    }

    #[rstest]
    fn test_volume_sensitive_fill_model_rejects_volume_above_quantity_range() {
        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(100_000))
            .build();
        let mut model = VolumeSensitiveFillModel::default();
        model.set_recent_volume(100_000_000_000_000_000.0);

        let error = model
            .get_orderbook_for_fill_simulation(
                &instrument,
                &order,
                Some(Price::from("0.80000")),
                Some(Price::from("0.80010")),
            )
            .unwrap_err();

        assert!(matches!(
            error.downcast_ref::<CorrectnessError>(),
            Some(CorrectnessError::PredicateViolation { message })
                if message.contains("QuantityRaw") || message.contains("QUANTITY_RAW_MAX")
        ));
    }

    #[rstest]
    #[case(0.0, Quantity::from(1))]
    #[case(0.5, Quantity::from(500))]
    #[case(1.0, Quantity::from(1_000))]
    fn test_competition_aware_fill_model_builds_expected_liquidity(
        #[case] liquidity_factor: f64,
        #[case] expected_size: Quantity,
    ) {
        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(100_000))
            .build();
        let best_bid = Price::from("0.80000");
        let best_ask = Price::from("0.80010");
        let mut model = CompetitionAwareFillModel::new(1.0, 0.0, None, liquidity_factor).unwrap();

        let book = model
            .get_orderbook_for_fill_simulation(&instrument, &order, Some(best_bid), Some(best_ask))
            .unwrap()
            .unwrap();

        assert_eq!(book.best_bid_price(), Some(best_bid));
        assert_eq!(book.best_ask_price(), Some(best_ask));
        assert_eq!(book.best_bid_size(), Some(expected_size));
        assert_eq!(book.best_ask_size(), Some(expected_size));
    }

    #[rstest]
    fn test_competition_aware_fill_model_preserves_instrument_size_precision() {
        let instrument = InstrumentAny::CryptoPerpetual(crypto_perpetual_ethusdt());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(100_000))
            .build();
        let best_bid = Price::from("2000.00");
        let best_ask = Price::from("2000.01");
        let mut model = CompetitionAwareFillModel::new(1.0, 0.0, None, 0.001234).unwrap();

        let book = model
            .get_orderbook_for_fill_simulation(&instrument, &order, Some(best_bid), Some(best_ask))
            .unwrap()
            .unwrap();

        assert_eq!(book.best_bid_price(), Some(best_bid));
        assert_eq!(book.best_ask_price(), Some(best_ask));
        assert_eq!(book.best_bid_size(), Some(Quantity::from("1.234")));
        assert_eq!(book.best_ask_size(), Some(Quantity::from("1.234")));
    }

    #[rstest]
    fn test_volume_sensitive_fill_model_builds_expected_liquidity() {
        let instrument = InstrumentAny::CryptoPerpetual(crypto_perpetual_ethusdt());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(100_000))
            .build();
        let best_bid = Price::from("2000.00");
        let best_ask = Price::from("2000.01");
        let mut model = VolumeSensitiveFillModel::default();
        model.set_recent_volume(5.678);

        let book = model
            .get_orderbook_for_fill_simulation(&instrument, &order, Some(best_bid), Some(best_ask))
            .unwrap()
            .unwrap();

        assert_eq!(book.best_bid_price(), Some(best_bid));
        assert_eq!(book.best_ask_price(), Some(best_ask));
        assert_eq!(book.best_bid_size(), Some(Quantity::from("1.420")));
        assert_eq!(book.best_ask_size(), Some(Quantity::from("1.420")));
    }

    #[rstest]
    fn test_fill_model_is_limit_filled(mut fill_model: DefaultFillModel) {
        // Fixed seed makes this deterministic
        let result = fill_model.is_limit_filled().unwrap();
        assert!(!result);
    }

    #[rstest]
    fn test_fill_model_is_slipped(mut fill_model: DefaultFillModel) {
        // Fixed seed makes this deterministic
        let result = fill_model.is_slipped().unwrap();
        assert!(!result);
    }

    #[rstest]
    fn test_default_fill_model_returns_none() {
        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(100_000))
            .build();

        let mut model = DefaultFillModel::default();
        let result = model
            .get_orderbook_for_fill_simulation(
                &instrument,
                &order,
                Some(Price::from("0.80000")),
                Some(Price::from("0.80010")),
            )
            .unwrap();
        assert!(result.is_none());
    }

    #[rstest]
    fn test_best_price_fill_model_returns_book() {
        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(100_000))
            .build();

        let mut model = BestPriceFillModel::default();
        let result = model
            .get_orderbook_for_fill_simulation(
                &instrument,
                &order,
                Some(Price::from("0.80000")),
                Some(Price::from("0.80010")),
            )
            .unwrap();
        assert!(result.is_some());
        let book = result.unwrap();
        assert_eq!(book.best_bid_price().unwrap(), Price::from("0.80000"));
        assert_eq!(book.best_ask_price().unwrap(), Price::from("0.80010"));
    }

    #[rstest]
    fn test_one_tick_slippage_fill_model() {
        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(100_000))
            .build();

        let tick = instrument.price_increment();
        let best_bid = Price::from("0.80000");
        let best_ask = Price::from("0.80010");

        let mut model = OneTickSlippageFillModel::default();
        let result = model
            .get_orderbook_for_fill_simulation(&instrument, &order, Some(best_bid), Some(best_ask))
            .unwrap();
        assert!(result.is_some());
        let book = result.unwrap();

        assert_eq!(book.best_bid_price().unwrap(), best_bid - tick);
        assert_eq!(book.best_ask_price().unwrap(), best_ask + tick);
    }

    #[rstest]
    fn test_fill_model_any_dispatch() {
        let model = FillModelAny::default();
        assert!(matches!(model, FillModelAny::Default(_)));
    }

    #[rstest]
    fn test_fill_model_any_is_limit_filled() {
        let mut model = FillModelAny::Default(DefaultFillModel::new(0.5, 0.1, Some(42)).unwrap());
        let result = model.is_limit_filled().unwrap();
        assert!(!result);
    }

    #[rstest]
    fn test_fill_model_handle_from_any_owns_state_per_conversion() {
        let model = FillModelAny::Default(DefaultFillModel::new(0.5, 0.0, Some(42)).unwrap());
        let mut expected_model = model.clone();
        let mut first: FillModelHandle = model.clone().into();
        let mut second: FillModelHandle = model.into();

        let expected: Vec<_> = (0..16)
            .map(|_| expected_model.is_limit_filled().unwrap())
            .collect();
        let first_results: Vec<_> = (0..16).map(|_| first.is_limit_filled().unwrap()).collect();
        let second_results: Vec<_> = (0..16).map(|_| second.is_limit_filled().unwrap()).collect();
        let has_variation = expected.windows(2).any(|window| window[0] != window[1]);

        assert!(has_variation);
        assert_eq!(first_results, expected);
        assert_eq!(second_results, expected);
    }

    #[rstest]
    fn test_default_fill_model_fill_limit_inside_spread_is_false() {
        let model = DefaultFillModel::default();
        assert!(!model.fill_limit_inside_spread().unwrap());
    }

    #[rstest]
    #[case(None, None)]
    #[case(None, Some(Price::from("0.80010")))]
    #[case(Some(Price::from("0.80000")), None)]
    fn test_builtin_fill_models_delegate_when_quotes_are_missing(
        #[case] best_bid: Option<Price>,
        #[case] best_ask: Option<Price>,
    ) {
        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(10))
            .build();
        let models: Vec<Box<dyn FillModel>> = vec![
            Box::new(DefaultFillModel::default()),
            Box::new(BestPriceFillModel::default()),
            Box::new(OneTickSlippageFillModel::default()),
            Box::new(ProbabilisticFillModel::default()),
            Box::new(TwoTierFillModel::default()),
            Box::new(ThreeTierFillModel::default()),
            Box::new(LimitOrderPartialFillModel::default()),
            Box::new(SizeAwareFillModel::default()),
            Box::new(CompetitionAwareFillModel::default()),
            Box::new(VolumeSensitiveFillModel::default()),
            Box::new(MarketHoursFillModel::default()),
        ];

        for mut model in models {
            assert!(
                model
                    .get_orderbook_for_fill_simulation(&instrument, &order, best_bid, best_ask)
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[rstest]
    fn test_best_price_fill_model_fill_limit_inside_spread_is_true() {
        let model = BestPriceFillModel::default();
        assert!(model.fill_limit_inside_spread().unwrap());
    }

    #[rstest]
    fn test_one_tick_slippage_fill_model_fill_limit_inside_spread_is_false() {
        let model = OneTickSlippageFillModel::default();
        assert!(!model.fill_limit_inside_spread().unwrap());
    }

    #[rstest]
    fn test_fill_model_any_fill_limit_inside_spread_dispatch() {
        let default = FillModelAny::Default(DefaultFillModel::default());
        assert!(!default.fill_limit_inside_spread().unwrap());

        let best_price = FillModelAny::BestPrice(BestPriceFillModel::default());
        assert!(best_price.fill_limit_inside_spread().unwrap());

        let one_tick = FillModelAny::OneTickSlippage(OneTickSlippageFillModel::default());
        assert!(!one_tick.fill_limit_inside_spread().unwrap());
    }

    #[rstest]
    fn test_market_hours_fill_model_switches_liquidity_and_preserves_clone_state() {
        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let order = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from(17))
            .build();
        let mut model = MarketHoursFillModel::default();

        for (low_liquidity, bid, ask) in [
            (false, dec!(0.80000), dec!(0.80010)),
            (true, dec!(0.79999), dec!(0.80011)),
            (false, dec!(0.80000), dec!(0.80010)),
        ] {
            model.set_low_liquidity_period(low_liquidity);
            let mut cloned = model.clone();
            let book = cloned
                .get_orderbook_for_fill_simulation(
                    &instrument,
                    &order,
                    Some(Price::from("0.80000")),
                    Some(Price::from("0.80010")),
                )
                .unwrap()
                .unwrap();

            assert_eq!(model.is_low_liquidity_period(), low_liquidity);
            assert_eq!(cloned.is_low_liquidity_period(), low_liquidity);
            assert_eq!(
                book.bids_as_map(None).into_iter().collect::<Vec<_>>(),
                vec![(bid, dec!(500))]
            );
            assert_eq!(
                book.asks_as_map(None).into_iter().collect::<Vec<_>>(),
                vec![(ask, dec!(500))]
            );
        }
    }

    #[rstest]
    fn test_fill_model_config_default_resolves_to_default_model() {
        let config = FillModelConfig::default();
        let model = config.resolve().unwrap();

        assert_eq!(config, FillModelConfig::default());
        assert!(matches!(model, FillModelAny::Default(_)));
        assert_eq!(
            format!("{model}"),
            "DefaultFillModel(prob_fill_on_limit=1, prob_slippage=0)"
        );
    }

    #[rstest]
    #[case(FillModelKind::BestPrice)]
    #[case(FillModelKind::OneTickSlippage)]
    #[case(FillModelKind::Probabilistic)]
    #[case(FillModelKind::TwoTier)]
    #[case(FillModelKind::ThreeTier)]
    #[case(FillModelKind::LimitOrderPartialFill)]
    #[case(FillModelKind::SizeAware)]
    #[case(FillModelKind::CompetitionAware)]
    #[case(FillModelKind::VolumeSensitive)]
    #[case(FillModelKind::MarketHours)]
    fn test_fill_model_config_resolves_each_kind(#[case] kind: FillModelKind) {
        let config = FillModelConfig {
            kind,
            ..Default::default()
        };

        assert_eq!(format!("{}", config.resolve().unwrap()), kind.to_string());
    }

    #[rstest]
    fn test_fill_model_config_forwards_probabilities() {
        let config = FillModelConfig {
            prob_slippage: 1.0,
            ..Default::default()
        };
        let mut model = config.resolve().unwrap();

        assert!(model.is_limit_filled().unwrap());
        assert!(model.is_slipped().unwrap());

        let config = FillModelConfig {
            prob_fill_on_limit: 0.0,
            ..Default::default()
        };
        let mut model = config.resolve().unwrap();

        assert!(!model.is_limit_filled().unwrap());
        assert!(!model.is_slipped().unwrap());
    }

    #[rstest]
    fn test_fill_model_config_resolves_spread_fill_behavior() {
        let best_price = FillModelConfig {
            kind: FillModelKind::BestPrice,
            ..Default::default()
        };

        assert!(
            best_price
                .resolve()
                .unwrap()
                .fill_limit_inside_spread()
                .unwrap()
        );

        let default = FillModelConfig::default();

        assert!(
            !default
                .resolve()
                .unwrap()
                .fill_limit_inside_spread()
                .unwrap()
        );
    }

    #[rstest]
    fn test_fill_model_config_seed_reproduces_draws() {
        let config = FillModelConfig {
            prob_fill_on_limit: 0.5,
            prob_slippage: 0.5,
            random_seed: Some(42),
            ..Default::default()
        };
        let mut first = config.resolve().unwrap();
        let mut second = config.resolve().unwrap();
        let draws: Vec<bool> = (0..16).map(|_| first.is_slipped().unwrap()).collect();
        let repeated: Vec<bool> = (0..16).map(|_| second.is_slipped().unwrap()).collect();

        assert_eq!(draws, repeated);
    }

    #[rstest]
    fn test_fill_model_config_param_out_of_range_error() {
        let config = FillModelConfig {
            prob_slippage: 1.1,
            ..Default::default()
        };
        let error = config.resolve().unwrap_err();

        assert_eq!(
            error.downcast_ref::<CorrectnessError>(),
            Some(&CorrectnessError::OutOfRange {
                param: "prob_slippage".to_string(),
                min: "0".to_string(),
                max: "1".to_string(),
                value: "1.1".to_string(),
                type_name: "f64",
            })
        );
    }

    #[rstest]
    fn test_fill_model_config_competition_aware_liquidity_factor() {
        let default_factor = FillModelConfig {
            kind: FillModelKind::CompetitionAware,
            ..Default::default()
        };

        assert!(matches!(
            default_factor.resolve().unwrap(),
            FillModelAny::CompetitionAware(_)
        ));

        let explicit = FillModelConfig {
            kind: FillModelKind::CompetitionAware,
            liquidity_factor: Some(0.5),
            ..Default::default()
        };

        assert!(matches!(
            explicit.resolve().unwrap(),
            FillModelAny::CompetitionAware(_)
        ));

        let out_of_range = FillModelConfig {
            kind: FillModelKind::CompetitionAware,
            liquidity_factor: Some(1.5),
            ..Default::default()
        };

        assert!(out_of_range.resolve().is_err());
    }

    #[rstest]
    #[case(FillModelKind::Default)]
    #[case(FillModelKind::ThreeTier)]
    #[case(FillModelKind::VolumeSensitive)]
    fn test_fill_model_config_rejects_liquidity_factor_for_other_kinds(
        #[case] kind: FillModelKind,
    ) {
        let config = FillModelConfig {
            kind,
            liquidity_factor: Some(0.5),
            ..Default::default()
        };
        let error = config.resolve().unwrap_err().to_string();

        assert!(error.contains("liquidity_factor"));
        assert!(error.contains(&kind.to_string()));
    }

    fn best_price_handle() -> FillModelHandle {
        FillModelHandle::new(BestPriceFillModel::new(1.0, 0.0, None).unwrap())
    }

    #[rstest]
    fn test_fill_model_selection_inherits_the_global_default() {
        let selection = FillModelSelection::default();
        let instrument_id = InstrumentId::from("ETHUSDT.BINANCE");

        assert!(!selection.has_instrument_override(&instrument_id));
        assert!(
            !selection
                .resolve(&instrument_id)
                .fill_limit_inside_spread()
                .unwrap()
        );
    }

    #[rstest]
    fn test_fill_model_selection_instrument_override_wins_over_the_venue_default() {
        let venue_id = InstrumentId::from("BTCUSDT.BINANCE");
        let override_id = InstrumentId::from("ETHUSDT.BINANCE");
        let selection = FillModelSelection::new(
            FillModelHandle::new(DefaultFillModel::default()),
            ahash::AHashMap::from_iter([(override_id, best_price_handle())]),
        );

        assert!(selection.has_instrument_override(&override_id));
        assert!(!selection.has_instrument_override(&venue_id));
        assert!(
            selection
                .resolve(&override_id)
                .fill_limit_inside_spread()
                .unwrap()
        );
        assert!(
            !selection
                .resolve(&venue_id)
                .fill_limit_inside_spread()
                .unwrap()
        );
    }

    #[rstest]
    fn test_fill_model_selection_venue_default_change_keeps_instrument_overrides() {
        let venue_id = InstrumentId::from("BTCUSDT.BINANCE");
        let override_id = InstrumentId::from("ETHUSDT.BINANCE");
        let mut selection = FillModelSelection::new(
            FillModelHandle::new(DefaultFillModel::default()),
            ahash::AHashMap::new(),
        );
        selection.set_instrument_override(
            override_id,
            FillModelHandle::new(DefaultFillModel::default()),
        );
        selection.set_venue_default(best_price_handle());

        assert!(
            selection
                .venue_default()
                .fill_limit_inside_spread()
                .unwrap()
        );
        assert!(
            selection
                .resolve(&venue_id)
                .fill_limit_inside_spread()
                .unwrap()
        );
        assert!(
            !selection
                .resolve(&override_id)
                .fill_limit_inside_spread()
                .unwrap()
        );
    }

    #[rstest]
    fn test_seeding_two_unseeded_probabilistic_models_reproduces_their_draws() {
        let mut first =
            FillModelAny::Probabilistic(ProbabilisticFillModel::new(0.5, 0.5, None).unwrap());
        let mut second =
            FillModelAny::Probabilistic(ProbabilisticFillModel::new(0.5, 0.5, None).unwrap());

        assert_eq!(first.random_seed(), None);
        assert_eq!(second.random_seed(), None);

        first.seed_if_unset(11);
        second.seed_if_unset(11);

        assert_eq!(first.random_seed(), Some(11));
        assert_eq!(second.random_seed(), Some(11));

        for _ in 0..32 {
            assert_eq!(
                first.is_limit_filled().unwrap(),
                second.is_limit_filled().unwrap()
            );
            assert_eq!(first.is_slipped().unwrap(), second.is_slipped().unwrap());
        }
    }

    #[rstest]
    fn test_a_fill_model_declaring_no_seed_stays_unseeded_until_it_is_seeded() {
        let model =
            FillModelAny::Probabilistic(ProbabilisticFillModel::new(0.5, 0.5, None).unwrap());

        assert_eq!(model.random_seed(), None);
    }

    #[rstest]
    fn test_a_fill_model_declaring_its_own_seed_keeps_it_when_seeded() {
        let mut model =
            FillModelAny::Probabilistic(ProbabilisticFillModel::new(0.5, 0.5, Some(4)).unwrap());

        model.seed_if_unset(9);

        assert_eq!(model.random_seed(), Some(4));
    }

    fn passive_context(queue_ahead: Quantity, toxicity: f64) -> PassiveFillContext {
        PassiveFillContext {
            order_side: OrderSide::Buy,
            order_quantity: Quantity::from(100),
            queue_ahead,
            toxicity,
        }
    }

    #[rstest]
    fn test_adverse_selection_fill_probability_equals_base_without_signals() {
        let model = AdverseSelectionFillModel::new(0.5, 0.1, Some(42), 1.0, 1.0).unwrap();

        assert_eq!(
            model.fill_probability(&passive_context(Quantity::from(0), 0.0)),
            model.state.prob_fill_on_limit
        );
    }

    #[rstest]
    fn test_adverse_selection_fill_probability_falls_monotonically_with_queue_ahead() {
        let model = AdverseSelectionFillModel::new(1.0, 0.0, Some(42), 1.0, 4.0).unwrap();

        let probabilities: Vec<f64> = [
            Quantity::from(0),
            Quantity::from(25),
            Quantity::from(100),
            Quantity::from(300),
            Quantity::from(1_000),
        ]
        .iter()
        .map(|queue_ahead| model.fill_probability(&passive_context(*queue_ahead, 0.0)))
        .collect();

        assert!(probabilities.windows(2).all(|window| window[0] > window[1]));
    }

    #[rstest]
    fn test_adverse_selection_fill_probability_falls_monotonically_with_toxicity() {
        let model = AdverseSelectionFillModel::new(1.0, 0.0, Some(42), 4.0, 1.0).unwrap();

        let probabilities: Vec<f64> = [-1.0, 0.0, 0.25, 0.5, 1.0]
            .iter()
            .map(|toxicity| model.fill_probability(&passive_context(Quantity::from(0), *toxicity)))
            .collect();

        assert!(
            probabilities
                .windows(2)
                .all(|window| window[0] >= window[1])
        );
        assert!(probabilities[1] > probabilities[4]);
    }

    #[rstest]
    fn test_adverse_selection_favourable_flow_leaves_base_probability_in_place() {
        let model = AdverseSelectionFillModel::new(0.5, 0.0, Some(42), 2.0, 2.0).unwrap();

        assert_eq!(
            model.fill_probability(&passive_context(Quantity::from(0), -1.0)),
            model.fill_probability(&passive_context(Quantity::from(0), 0.0))
        );
    }

    #[rstest]
    fn test_a_model_that_ignores_the_context_falls_back_to_the_default_decision() {
        // The trait default delegates to `is_limit_filled`, so a model that does not
        // condition on the context makes the same draws as before.
        let mut legacy = DefaultFillModel::new(0.5, 0.1, Some(42)).unwrap();
        let mut contextual = DefaultFillModel::new(0.5, 0.1, Some(42)).unwrap();

        for _ in 0..32 {
            let expected = legacy.is_limit_filled().unwrap();
            let result = contextual
                .is_limit_filled_with_context(&passive_context(Quantity::from(250), 1.0))
                .unwrap();

            assert_eq!(result, expected);
        }
    }

    #[rstest]
    fn test_adverse_selection_context_decisions_reproduce_with_the_same_seed() {
        let mut first = AdverseSelectionFillModel::new(0.5, 0.1, Some(7), 1.0, 1.0).unwrap();
        let mut second = AdverseSelectionFillModel::new(0.5, 0.1, Some(7), 1.0, 1.0).unwrap();
        let context = passive_context(Quantity::from(150), 0.75);

        for _ in 0..64 {
            assert_eq!(
                first.is_limit_filled_with_context(&context).unwrap(),
                second.is_limit_filled_with_context(&context).unwrap()
            );
            assert_eq!(first.is_slipped().unwrap(), second.is_slipped().unwrap());
        }
    }

    #[rstest]
    fn test_adverse_selection_declaring_no_seed_stays_unseeded_until_seeded() {
        let mut model = FillModelAny::AdverseSelection(
            AdverseSelectionFillModel::new(0.5, 0.5, None, 1.0, 1.0).unwrap(),
        );

        assert_eq!(model.random_seed(), None);

        model.seed_if_unset(11);

        assert_eq!(model.random_seed(), Some(11));
    }

    #[rstest]
    fn test_adverse_selection_fill_probability_falls_against_the_plain_model() {
        let mut plain = ProbabilisticFillModel::new(0.5, 0.0, Some(42)).unwrap();
        let mut neutral = AdverseSelectionFillModel::new(0.5, 0.0, Some(42), 1.0, 1.0).unwrap();
        let mut adverse = AdverseSelectionFillModel::new(0.5, 0.0, Some(42), 1.0, 1.0).unwrap();
        let neutral_context = passive_context(Quantity::from(0), 0.0);
        let adverse_context = passive_context(Quantity::from(300), 1.0);

        let mut plain_fills = 0;
        let mut neutral_fills = 0;
        let mut adverse_fills = 0;

        for _ in 0..256 {
            plain_fills += usize::from(plain.is_limit_filled().unwrap());
            neutral_fills += usize::from(
                neutral
                    .is_limit_filled_with_context(&neutral_context)
                    .unwrap(),
            );
            adverse_fills += usize::from(
                adverse
                    .is_limit_filled_with_context(&adverse_context)
                    .unwrap(),
            );
        }

        assert_eq!(neutral_fills, plain_fills);
        assert!(adverse_fills < plain_fills);
    }

    #[rstest]
    fn test_fill_model_handle_forwards_the_passive_context() {
        let model = FillModelAny::AdverseSelection(
            AdverseSelectionFillModel::new(1.0, 0.0, Some(3), 8.0, 8.0).unwrap(),
        );
        let mut handle = FillModelHandle::from(model);

        assert!(
            handle
                .is_limit_filled_with_context(&passive_context(Quantity::from(0), 0.0))
                .unwrap()
        );
    }

    #[rstest]
    fn test_adverse_selection_rejects_out_of_range_probability() {
        let error = AdverseSelectionFillModel::new(1.1, 0.0, None, 1.0, 1.0)
            .unwrap_err()
            .to_string();

        assert_eq!(
            error,
            "invalid f64 for 'prob_fill_on_limit' not in range [0, 1], was 1.1"
        );
    }

    #[rstest]
    #[case(f64::NAN, "NaN")]
    #[case(f64::INFINITY, "inf")]
    #[case(f64::NEG_INFINITY, "-inf")]
    fn test_adverse_selection_rejects_non_finite_sensitivity(
        #[case] value: f64,
        #[case] expected_value: &str,
    ) {
        let error = AdverseSelectionFillModel::new(1.0, 0.0, None, value, 1.0).unwrap_err();

        assert_eq!(
            error.downcast_ref::<CorrectnessError>(),
            Some(&CorrectnessError::InvalidValue {
                param: "queue_sensitivity".to_string(),
                value: expected_value.to_string(),
                type_name: "f64",
            })
        );
    }

    #[rstest]
    fn test_adverse_selection_rejects_negative_sensitivity() {
        let error = AdverseSelectionFillModel::new(1.0, 0.0, None, 1.0, -0.5).unwrap_err();

        assert_eq!(
            error.downcast_ref::<CorrectnessError>(),
            Some(&CorrectnessError::NegativeValue {
                param: "toxicity_sensitivity".to_string(),
                value: "-0.5".to_string(),
                type_name: "f64",
            })
        );
    }

    #[rstest]
    fn test_fill_model_config_resolves_the_adverse_selection_kind() {
        let config = FillModelConfig {
            kind: FillModelKind::AdverseSelection,
            prob_fill_on_limit: 0.5,
            ..Default::default()
        };
        let mut model = config.resolve().unwrap();

        assert!(matches!(model, FillModelAny::AdverseSelection(_)));
        assert_eq!(
            format!("{model}"),
            FillModelKind::AdverseSelection.to_string()
        );
        assert!(model.random_seed().is_none());

        model.seed_if_unset(5);

        assert_eq!(model.random_seed(), Some(5));
    }
}
