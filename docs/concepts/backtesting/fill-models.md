# Fill Models

Historical data cannot show how a simulated order would have interacted with other market
participants. A **fill model** controls the assumptions NautilusTrader makes about limit-order
eligibility, one-tick slippage, and optional synthetic liquidity.

## Behavior by book type

With L2 or L3 data, the recorded book supplies price levels and sizes. The matching engine walks
those levels, and `prob_fill_on_limit` can model whether a touched limit order fills.
`prob_slippage` does not apply because the book itself determines price impact.

With an L1 book, including books updated from quotes, trades, or bars:

- `prob_fill_on_limit` controls whether a limit order fills when its price is touched.
- `prob_slippage` is evaluated for every fill, regardless of order type or liquidity side.
- A successful slippage draw moves the fill one tick against the order direction.
- A model may provide a synthetic L2 book to represent liquidity beyond the best bid and ask.

For example, with `prob_slippage=0.5`, each BUY fill has a 50% chance of moving one tick higher.
Set `random_seed` when a run must reproduce the model's random draws.

If a venue does not specify a fill model, it uses `DefaultFillModel` with
`prob_fill_on_limit=1.0` and `prob_slippage=0.0`. The model therefore considers a touched limit
fill-eligible, and L1 fills do not receive probabilistic one-tick slippage by default. This does
not disable the matching engine's separate residual-fill rule for eligible market-style orders.

:::warning
Historical order book data remains immutable after a fill. With
`liquidity_consumption=False`, the same displayed size can support more than one simulated order in
an iteration. Set `liquidity_consumption=True` to track consumed size per level until fresh data
arrives. See [order book immutability](fill-prices-and-matching.md#order-book-immutability).
:::

## Available models

| Model                        | Liquidity behavior                                      |
| ---------------------------- | ------------------------------------------------------- |
| `DefaultFillModel`           | Uses the matching engine's recorded book.               |
| `BestPriceFillModel`         | Provides unlimited size at the best bid and ask.        |
| `OneTickSlippageFillModel`   | Provides unlimited size one tick beyond the best price. |
| `ProbabilisticFillModel`     | Chooses the best price or one tick worse.               |
| `TwoTierFillModel`           | Places 10 units at best, then the rest one tick worse.  |
| `ThreeTierFillModel`         | Places 50, 30, and 20 units across three levels.        |
| `LimitOrderPartialFillModel` | Places 5 units at best, then the rest one tick worse.   |
| `SizeAwareFillModel`         | Changes the book shape at an order size of 10 units.    |
| `CompetitionAwareFillModel`  | Exposes a configurable fraction of 1,000 units at best. |
| `VolumeSensitiveFillModel`   | Places 25% of its internal volume at best.              |
| `MarketHoursFillModel`       | Uses a normal or one-tick-wider synthetic spread.       |

The tier sizes are model constants expressed in instrument quantity units. Confirm that they suit
the scale of the instrument before using a tiered model.

`CompetitionAwareFillModel` accepts `liquidity_factor` values in `[0.0, 1.0]`, defaults to `0.3`,
and clamps the calculated size to at least one instrument quantity unit.

The current Python bindings do not expose the state setters for `VolumeSensitiveFillModel` or
`MarketHoursFillModel`, and `FillModelConfig` does not carry them either. From Python they retain
their initial values of 1,000 recent-volume units and normal-liquidity mode.

## Configuration

Pass a built-in model object directly to `BacktestVenueConfig`:

```python
from decimal import Decimal

from nautilus_trader.config import BacktestVenueConfig
from nautilus_trader.execution import DefaultFillModel
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import BookType
from nautilus_trader.model import OmsType

venue = BacktestVenueConfig(
    name="SIM",
    oms_type=OmsType.NETTING,
    account_type=AccountType.CASH,
    book_type=BookType.L1_MBP,
    starting_balances=["100_000 USD"],
    fill_model=DefaultFillModel(
        prob_fill_on_limit=0.2,
        prob_slippage=0.5,
        random_seed=42,
    ),
    fee_model=MakerTakerFeeModel(
        maker_rate=Decimal("0"),
        taker_rate=Decimal("0"),
    ),
)
```

Synthetic book models use the same constructor parameters:

```python
from decimal import Decimal

from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.execution import ThreeTierFillModel

venue = BacktestVenueConfig(
    name="SIM",
    oms_type=OmsType.NETTING,
    account_type=AccountType.CASH,
    book_type=BookType.L1_MBP,
    starting_balances=["100_000 USD"],
    fill_model=ThreeTierFillModel(
        prob_fill_on_limit=1.0,
        prob_slippage=0.0,
        random_seed=42,
    ),
    fee_model=MakerTakerFeeModel(
        maker_rate=Decimal("0"),
        taker_rate=Decimal("0"),
    ),
)
```

The current high-level venue configuration accepts built-in fill models. It does not load fill
models from import-path configuration objects.

### Describing a model by configuration

Every place that accepts a fill model also accepts `FillModelConfig`, which describes one of the
built-in models as data:

```python
from decimal import Decimal

from nautilus_trader.config import BacktestVenueConfig
from nautilus_trader.execution import FillModelConfig
from nautilus_trader.execution import FillModelKind
from nautilus_trader.execution import MakerTakerFeeModel

venue = BacktestVenueConfig(
    name="SIM",
    oms_type=OmsType.NETTING,
    account_type=AccountType.CASH,
    book_type=BookType.L1_MBP,
    starting_balances=["100_000 USD"],
    fill_model=FillModelConfig(
        kind=FillModelKind.THREE_TIER,
        prob_fill_on_limit=1.0,
        prob_slippage=0.0,
        random_seed=42,
    ),
    fee_model=MakerTakerFeeModel(
        maker_rate=Decimal("0"),
        taker_rate=Decimal("0"),
    ),
)
```

`kind` selects the model, one of `FillModelKind.DEFAULT`, `BEST_PRICE`, `ONE_TICK_SLIPPAGE`,
`PROBABILISTIC`, `TWO_TIER`, `THREE_TIER`, `LIMIT_ORDER_PARTIAL_FILL`, `SIZE_AWARE`,
`COMPETITION_AWARE`, `VOLUME_SENSITIVE`, or `MARKET_HOURS`. The other fields carry the parameters
that model's constructor takes. `liquidity_factor` is consumed by `COMPETITION_AWARE` only, and
supplying it for any other kind is an error rather than a silently ignored setting. Omitted
parameters use the same defaults as the model classes, so `FillModelConfig()` describes the
default model.

The configuration resolves to the model it names when the configuration that carries it is
constructed, using that model's own constructor. It holds no behaviour of its own, so it is not a
second fill implementation: passing the model object directly remains supported and equivalent, and
`config.fill_model` reads back the resolved model object either way.

### Per-instrument overrides

A venue can set a different fill model for individual instruments:

```python
from decimal import Decimal

from nautilus_trader.config import BacktestVenueConfig
from nautilus_trader.execution import DefaultFillModel
from nautilus_trader.execution import FillModelConfig
from nautilus_trader.execution import FillModelKind
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import BookType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import OmsType

venue = BacktestVenueConfig(
    name="BINANCE",
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    book_type=BookType.L1_MBP,
    starting_balances=["1_000_000 USDT"],
    fill_model=DefaultFillModel(prob_fill_on_limit=0.0),
    instrument_fill_models={
        InstrumentId.from_str("ETHUSDT-PERP.BINANCE"): FillModelConfig(
            kind=FillModelKind.THREE_TIER,
            random_seed=7,
        ),
    },
    fee_model=MakerTakerFeeModel(
        maker_rate=Decimal("0"),
        taker_rate=Decimal("0"),
    ),
)
```

Fill model selection is an inheritance chain. The levels, from least to most specific, are the
global default (the built-in model above, used when a venue sets no fill model), the venue
`fill_model`, and the per-instrument override. A level that sets a model overrides the less
specific levels, and a level that sets nothing inherits them, so an instrument without an override
resolves to the venue fill model and a venue without one resolves to the default model.

The override is applied when the matching engine for the instrument is created. Calling
`BacktestEngine.change_fill_model` afterwards changes the venue level only, so existing instrument
overrides keep applying.

`instrument_fill_models` is accepted wherever `fill_model` is: `BacktestVenueConfig`,
`BacktestEngine.add_venue`, and `SandboxExecutionClientConfig`. Entries are either built-in model
objects or `FillModelConfig` descriptions, like `fill_model` itself. The sandbox field is
runtime-only in configuration serialization, as its other model fields are.

### Independent slippage

Fill, slippage, and fee are configured independently. A venue can set a slippage model on its own,
without adopting a composite fill model that folds slippage in:

```python
from decimal import Decimal

from nautilus_trader.config import BacktestVenueConfig
from nautilus_trader.execution import DefaultFillModel
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.execution import ProbabilisticSlippageModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import BookType
from nautilus_trader.model import OmsType

venue = BacktestVenueConfig(
    name="SIM",
    oms_type=OmsType.NETTING,
    account_type=AccountType.CASH,
    book_type=BookType.L1_MBP,
    starting_balances=["100_000 USD"],
    fill_model=DefaultFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0),
    slippage_model=ProbabilisticSlippageModel(prob_slippage=0.5, random_seed=42),
    fee_model=MakerTakerFeeModel(
        maker_rate=Decimal("0"),
        taker_rate=Decimal("0"),
    ),
)
```

`ProbabilisticSlippageModel` draws a one-tick adverse adjustment with probability `prob_slippage`
on each L1 fill and takes `random_seed` for reproducibility, exactly as the fill models' own
`prob_slippage` does. It is accepted wherever a venue is configured: `BacktestVenueConfig`,
`BacktestEngine.add_venue`, and `SandboxExecutionClientConfig`. The field is optional and defaults
to no independent model.

When a venue sets a slippage model, it is the single source of the slippage decision and the fill
model's own `prob_slippage` is not consulted. When a venue sets none, the fill model decides, which
is the default behavior. Do not set both on the same venue: configure the fill model with
`prob_slippage=0.0` when using an independent slippage model.

A decomposed configuration reproduces a composite one draw for draw when the fill decision is
deterministic (`prob_fill_on_limit` of `0.0` or `1.0`). A composite fill model draws its limit-fill
and slippage decisions from one random stream, while an independent slippage model has its own, so
with a stochastic fill decision the draws are statistically equivalent but not identical.

#### Ordering

The concerns compose in one order, and it is the order the matching engine applies them:

1. Fill eligibility: the fill model decides whether a limit order fills (`is_limit_filled`).
2. Fill quantity and base fill price: the fill model's book, or the synthetic book it returns,
   determines how much fills and at what base price.
3. Slippage adjustment: the slippage model decides whether the base fill price moves one price
   increment against the order direction (BUY up, SELL down) on an L1 book.
4. Market impact adjustment: the market impact model, when a venue sets one, moves the price of
   a liquidity-taking L1 fill by a size-dependent number of increments against the order
   direction.
5. Final fill price: the adjusted price is the price recorded on the fill event.
6. Fee: the fee model is charged on the final fill price and the fill quantity.

Slippage therefore changes the price the fee model sees, and never changes eligibility or
quantity. Market impact composes after slippage and never changes eligibility or quantity.

### Independent market impact

Fill and slippage are configured independently of market impact. A venue can set a market impact
model so that a large liquidity-taking order is filled further through the book than a small one:

```python
from decimal import Decimal

from nautilus_trader.config import BacktestVenueConfig
from nautilus_trader.execution import DefaultFillModel
from nautilus_trader.execution import LinearMarketImpactModel
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import BookType
from nautilus_trader.model import OmsType
from nautilus_trader.model import Quantity

venue = BacktestVenueConfig(
    name="SIM",
    oms_type=OmsType.NETTING,
    account_type=AccountType.CASH,
    book_type=BookType.L1_MBP,
    starting_balances=["100_000 USD"],
    fill_model=DefaultFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0),
    market_impact_model=LinearMarketImpactModel(
        quantity_per_increment=Quantity.from_str("10.000"),
        max_increments=5,
    ),
    fee_model=MakerTakerFeeModel(
        maker_rate=Decimal("0"),
        taker_rate=Decimal("0"),
    ),
)
```

`LinearMarketImpactModel` moves the fill price of a liquidity-taking (taker) L1 fill against the
order direction by one price increment for every `quantity_per_increment` units filled, capped at
`max_increments`. A fill smaller than one increment quantity is unchanged. The adjustment is
computed with exact decimal arithmetic on the fill quantity, so it is deterministic and takes no
random seed.

It is accepted wherever a venue is configured: `BacktestVenueConfig`,
`BacktestEngine.add_venue`, and `SandboxExecutionClientConfig`. The field is optional and defaults
to no model: a venue that sets none does not adjust a fill price for size, which is the default
behavior.

Market impact applies only to liquidity-taking fills on an L1 book. On L2 or L3 books the recorded
book already determines how far an order walks, and a resting (maker) fill does not move the price
against itself.

### Custom fill models

The low-level `BacktestEngine.add_venue()` method also accepts a custom Python object. It must
implement:

- `is_limit_filled() -> bool`
- `is_slipped() -> bool`

It may also implement:

- `fill_limit_inside_spread() -> bool`
- `get_orderbook_for_fill_simulation(instrument, order, best_bid, best_ask) -> OrderBook | None`

Subclassing `nautilus_trader.execution.FillModel` supplies default implementations for these
methods. This custom-object protocol applies to the low-level engine only.

The liquidity hook receives `None` for a missing historical bid or ask. Custom models must handle
these optional prices. Returning `None` uses the standard fill logic; returning an `OrderBook`
restricts fills to that book's eligible liquidity, even when no fills are available. Partial custom
fills are not topped up with historical liquidity or the L1 remainder-fill rule.

## Probabilistic parameters

### `prob_fill_on_limit` (default: `1.0`)

This value controls whether a limit order fills when the market touches, but does not cross, its
price:

- `0.0`: Never fill on touch.
- `0.5`: Fill on half of eligible touches on average.
- `1.0`: Always fill on touch.

Crossing the limit price is a separate matching condition. For explicit queue-volume tracking, see
[queue position tracking](trade-execution.md#queue-position-tracking).

### `prob_slippage` (default: `0.0`)

For L1 books, this value controls a one-tick adverse move on each fill:

- `0.0`: Never add model slippage.
- `0.5`: Add one tick on half of fills on average.
- `1.0`: Add one tick to every fill.

The draw applies to maker and taker fills. It does not apply to L2 or L3 books.

The same draw is available as its own model, `ProbabilisticSlippageModel`, configured through a
venue's `slippage_model` instead of the fill model's `prob_slippage`. See
[Independent slippage](#independent-slippage).

## Synthetic order books

Before determining a fill, the matching engine asks the model for an optional synthetic order book.
If the model returns a book, the engine fills against its levels. If it returns `None`, the engine
uses the recorded book.

:::warning[Synthetic book consumption]
Per-level `liquidity_consumption` tracking does not apply to a synthetic model book. A custom model
must represent any desired consumption behavior in the books it returns.
:::

## Execution realism inventory

This section describes the execution-realism code as it exists at the time of writing, based on a direct read of the Rust sources. The model traits and all built-in model implementations live in `crates/execution/src/models/` (fee.rs, fill.rs, latency.rs, slippage.rs, market_impact.rs); the matching engine that applies them lives in `crates/execution/src/matching_engine/` (mod.rs and config.rs); and the backtest wiring that lets a venue select and inject them lives in `crates/backtest/src/` (config.rs, node.rs, exchange.rs). The sandbox adapter mirrors the same wiring in `crates/adapters/sandbox/src/`. Every claim below is cited to the source line(s) that actually run.

### 1. Fee models

The `FeeModel` trait exposes `get_commission` and an optional `get_commission_with_context` that defaults to delegating to `get_commission` (`crates/execution/src/models/fee.rs:41-60`). `FeeModelAny` is the runtime enum (`fee.rs:124-133`); `FeeModelHandle` is a shared `Rc<dyn FeeModel>` (`fee.rs:65-121`).

| Model                          | Config / constructor                                                                              | Behaviour                                                                                                                                                                            | Citation         |
| ------------------------------ | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------- |
| `FixedFeeModel`                | `FixedFeeModel::new(commission, charge_commission_once)` (default `true`)                         | Returns the fixed commission unless `charge_commission_once` and the order already has fills, then returns zero.                                                                     | `fee.rs:191-232` |
| `MakerTakerFeeModel`           | `MakerTakerFeeModel::new(maker_rate, taker_rate)`; `zero()`; `set_override(instrument_id, rates)` | Resolves maker/taker rate from the schedule by `order.liquidity_side()`, then `notional * rate`. Errors if liquidity side is unset.                                                  | `fee.rs:335-386` |
| `PerContractFeeModel`          | `PerContractFeeModel::new(commission)`                                                            | `commission * fill_quantity * contracts`, where `contracts` is 1 except for generic spreads where it is the sum of leg ratios.                                                       | `fee.rs:245-330` |
| `ProbabilityPriceFeeModel`     | `ProbabilityPriceFeeModel::new(maker_rate, taker_rate)`; `set_override`                           | Requires a `BinaryOption` and fill price in `[0, 1]`; computes `qty * rate * p * (1 - p)` rounded to 5 dp, in the quote currency.                                                    | `fee.rs:414-483` |
| `CappedOptionFeeModel`         | `CappedOptionFeeModel::new(maker_rate, taker_rate, cap_rate)` (cap default `0.125`)               | Options only; `min(rate * underlying_px, cap * fill_px) * multiplier * qty` per contract (uses `rate` directly for inverse instruments, and requires an underlying price otherwise). | `fee.rs:488-592` |
| `TieredNotionalOptionFeeModel` | `TieredNotionalOptionFeeModel::new(maker_rate, taker_rate)`; `set_override`                       | Options only; `notional(fill_qty, fill_px) * rate`, in the notional currency.                                                                                                        | `fee.rs:596-660` |
| `PythonFeeModel`               | `FeeModelAny::Python` (feature `python`)                                                          | Dispatches to a user Python fee model.                                                                                                                                               | `fee.rs:131-132` |

Maker/taker rate resolution is deterministic: an exact `InstrumentId` override wins, otherwise the schedule default applies (`crates/model/src/fees.rs:83-137`). The shared notional arithmetic is `calculate_maker_taker_commission` (`crates/model/src/fees.rs:148-163`).

Where fees are actually applied:

- The commission is computed per fill inside `OrderMatchingEngine::fill_order`, immediately before the fill event is generated. It is passed a clone of the order carrying the pre-fill `filled_qty` (so `FixedFeeModel` charges once) and the resolved liquidity side; the fee currency is the instrument quote currency (`crates/execution/src/matching_engine/mod.rs:5433-5462`).
- For option instruments the underlying price for capped/tiered fees is resolved from cache (underlying `Last` then `Mark` then `Mid`, else option greeks) via `fee_underlying_price` (`mod.rs:5821-5854`).
- The venue's `FeeModelHandle` is stored on `SimulatedExchange` (`crates/backtest/src/exchange.rs:163,254`) and cloned into each `OrderMatchingEngine::new` call (`exchange.rs:506-507`). `SimulatedVenueConfig.fee_model` is required, not optional (`crates/backtest/src/config.rs:295-300`).
- `BacktestVenueConfig.fee_model` is `Option<FeeModelAny>` (`config.rs:540-543`) but `BacktestNode` errors if it is absent, requiring an explicit model including an explicit zero-fee model (`crates/backtest/src/node.rs:271-280`). The same requirement exists in the sandbox client (`crates/adapters/sandbox/src/execution.rs:127-150`).
- A venue-specific model outside the core crate is `PolymarketFeeModel`, which implements `FeeModel` directly (`crates/adapters/polymarket/src/models.rs:52`).

### 2. Fill models

The `FillModel` trait (`crates/execution/src/models/fill.rs:42-94`) has three decision methods - `is_limit_filled`, `is_slipped`, and `fill_limit_inside_spread` (default `false`, `fill.rs:72-74`) - plus `get_orderbook_for_fill_simulation`, which returns a synthetic L2 book or `None` to use the engine's standard book logic. `FillModelAny` enumerates all eleven variants (`fill.rs:1337-1437`).

| Variant                      | Synthetic book / fill price                                                                                                                   | Citation                  |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| `DefaultFillModel` (default) | Returns `None`; no synthetic prices. Limit-fill decision is probabilistic; default is `prob_fill_on_limit=1.0`, `prob_slippage=0.0`, no seed. | `fill.rs:265-297,319-327` |
| `BestPriceFillModel`         | Best bid and best ask, each with effectively unlimited size; `fill_limit_inside_spread()` returns `true`.                                     | `fill.rs:340-386,388-421` |
| `OneTickSlippageFillModel`   | `best_bid - tick` and `best_ask + tick`, unlimited size.                                                                                      | `fill.rs:429-503`         |
| `ProbabilisticFillModel`     | One random coin flip: half the time best bid/ask, otherwise one tick worse.                                                                   | `fill.rs:516-612`         |
| `TwoTierFillModel`           | 10 contracts at best bid/ask, then unlimited one tick worse.                                                                                  | `fill.rs:620-707`         |
| `ThreeTierFillModel`         | 50 at best, 30 at one tick, 20 at two ticks.                                                                                                  | `fill.rs:721-823`         |
| `LimitOrderPartialFillModel` | 5 at best bid/ask, then unlimited one tick worse (size-capped partial fill).                                                                  | `fill.rs:837-924`         |
| `SizeAwareFillModel`         | `qty <= 10`: 50 at best; larger: 10 at best and `qty - 10` one tick worse.                                                                    | `fill.rs:939-1024`        |
| `CompetitionAwareFillModel`  | `max(1000 * liquidity_factor, 1)` at best bid/ask; default `liquidity_factor=0.3`.                                                            | `fill.rs:1037-1110`       |
| `VolumeSensitiveFillModel`   | `max(recent_volume * 0.25, 1)` at best (default `recent_volume=1000`), then unlimited one tick worse.                                         | `fill.rs:1126-1214`       |
| `MarketHoursFillModel`       | Normal: 500 at best bid/ask; low-liquidity period: 500 one tick worse (toggle via `set_low_liquidity_period`).                                | `fill.rs:1230-1317`       |

Configuration layer:

- `FillModelKind` names the eleven variants (`crates/execution/src/models/fill.rs:1453-1494`) and `FillModelConfig` carries the kind plus `prob_fill_on_limit`, `prob_slippage`, `random_seed`, and the `CompetitionAware`-only `liquidity_factor` (`fill.rs:1496-1530`).
- `FillModelConfig::resolve` constructs the named model with that model's own constructor, so validation and behaviour are the constructor's, and an omitted `liquidity_factor` uses `DEFAULT_LIQUIDITY_FACTOR` (`fill.rs:1448-1450,1544-1623`). The default configuration resolves to the default model (`fill.rs:1532-1542`).
- `resolve` errors if `liquidity_factor` is supplied for any kind other than `CompetitionAware`, rather than ignoring it.
- The description is accepted wherever a fill model is accepted from Python, because `pyobject_to_fill_model_any` resolves it before the model bindings (`crates/execution/src/python/fill.rs:157-160`; the class methods are `crates/execution/src/python/fill.rs:321-392`). `BacktestVenueConfig`, `BacktestEngine.add_venue`, `BacktestEngine.change_fill_model`, and the sandbox client config all convert through that adapter, so a model object and a configuration are interchangeable inputs. Nothing downstream of the adapter changed.

Default and selection:

- The default is `FillModelAny::Default(DefaultFillModel::default())` (`fill.rs:1424-1427`), which is also what `FillModelHandle::default()` yields (`fill.rs:153-157`).
- Selection is an inheritance chain resolved by `FillModelSelection` in the execution crate (`fill.rs:1643-1716`): the global default, then the venue default, then the per-instrument override. `resolve` returns the instrument's override when it has one and the venue default otherwise (`fill.rs:1708-1715`), so a level that sets no model inherits.
- `BacktestVenueConfig.fill_model` is `Option<FillModelAny>` (`config.rs:535-536`) and `instrument_fill_models` is an optional `InstrumentId`-keyed map of the same description (`config.rs:543-547`); `BacktestNode` maps the venue model and the overrides to handles (`node.rs:265-276`).
- `SimulatedVenueConfig.fill_model` is a `FillModelHandle` (`config.rs:292-294`) and `instrument_fill_models` is an `InstrumentId`-keyed handle map (`config.rs:295-300`); the exchange builds the selection from both (`exchange.rs:164,255-259`) and resolves it per instrument when the matching engine is created, so the engine receives exactly the resolved model (`exchange.rs:527-531`). `BacktestEngine::change_fill_model` replaces the venue default at runtime and leaves instrument overrides in place (`exchange.rs:325-336`; `crates/backtest/src/engine.rs:351-360`).
- The sandbox client carries the same selection (`crates/adapters/sandbox/src/execution.rs:144-155`) and resolves it when it creates a matching engine (`crates/adapters/sandbox/src/execution.rs:1010`). Its `instrument_fill_models` field is runtime-only in serialization, like its other model fields (`crates/adapters/sandbox/src/config.rs:90-102`).
- When a model returns a synthetic book, its fills come from `OrderBook::simulate_fills` using a market sentinel price (`Price::max`/`Price::min`), and are marked `from_synthetic` (`mod.rs:4587-4623`). When it returns `None`, the engine uses the real book: market orders cross the book (`determine_market_price_and_volume`, `mod.rs:4532-4560`) and limit orders use crossed levels or `simulate_fills` (`determine_limit_price_and_volume`, `mod.rs:4369-4529`).
- `fill_limit_inside_spread` is applied once at engine construction: the matching core treats a limit at or better than the same-side best quote as fillable only when the model returns `true` (`mod.rs:186-188,563-567`).
- After a fill price is produced, `apply_fills` shifts it by one tick against the order direction when the book is L1 and the slippage decision is true (`mod.rs:5124-5142`). The decision is the venue's independent slippage model when one is configured, and the fill model's own `is_slipped()` otherwise, so the default path is the fill model. See [Slippage models](#8-slippage-models).
- Bar-driven fills: with `bar_execution` and an L1 book, `process_bar` synthesizes trade ticks from OHLC and fills the open at `bar.open`, the high at `bar.high`, the low at `bar.low`, and the close at `bar.close` (`mod.rs:1881-2060`), with high/low at bar prices (`mod.rs:2063-2097`).

### 3. Latency models

The `LatencyModel` trait returns insert/update/delete and base durations in nanoseconds (`crates/execution/src/models/latency.rs:31-51`). `LatencyModelAny` has exactly one variant, `Static` (`latency.rs:92-94`). `StaticLatencyModel::new` adds the base latency to each operation latency at construction, so the effective insert/update/delete latency is `base + operation` (`latency.rs:124-176`).

| Option               | Config field / path                                              | Effect                                                                                                                   | Citation                                           |
| -------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------- |
| Static latency       | `StaticLatencyModel::new(base, insert, update, delete)`          | Fixed per-operation delay; base is folded into each.                                                                     | `latency.rs:124-176`                               |
| Venue latency        | `BacktestVenueConfig.latency_model: Option<LatencyModelAny>`     | Optional; mapped to a handle in `BacktestNode`.                                                                          | `config.rs:537-538,778-781`; `node.rs:282,298`     |
| Runtime config       | `SimulatedVenueConfig.latency_model: Option<LatencyModelHandle>` | Stored on the exchange; `set_latency_model` raises it to `Some`.                                                         | `config.rs:307-308`; `exchange.rs:166,257,326-329` |
| Order submit latency | applied in `generate_inflight_command`                           | `SubmitOrder` -> `ts_init + insert_latency`; `Modify` -> `+ update_latency`; `Cancel` -> `+ delete_latency`.             | `exchange.rs:865-892`                              |
| Dispatch gate        | `Exchange::send`                                                 | With a latency model the command enters the inflight min-heap queue; without one it goes to the immediate message queue. | `exchange.rs:855-859`                              |

Data latency: no latency model is applied to market-data replay. The latency model is only consulted for `TradingCommand` dispatch (`exchange.rs:865-892`); nothing in `crates/backtest/src/` applies latency to data ticks. To be explicit, a data-latency model does not exist in this code.

### 4. Queue position

| Option               | Config field / path                                          | Effect                                                                                       | Citation                                               |
| -------------------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Queue position       | `OrderMatchingEngineConfig.queue_position` (default `false`) | Enables resting-order queue tracking so passive fills wait for depth ahead to trade through. | `crates/execution/src/matching_engine/config.rs:50-51` |
| Venue queue position | `BacktestVenueConfig.queue_position` (default `false`)       | Copied into the matching-engine config.                                                      | `config.rs:354-356,519-520`                            |

What it does to a fill:

- When an order is accepted/rested, `snapshot_queue_position` records the resting-side quantity ahead at that price via `get_quantity_at_level`; L1 orders behind the BBO are held pending until the BBO reaches the price (`mod.rs:570-613`).
- `determine_trade_fill_qty` returns `None` (blocking the fill) while tracked `ahead_raw > 0`, when the order is pending an L1 snapshot, or when the trade-excess budget is zero; otherwise it caps the fill by remaining trade volume and queue excess (`mod.rs:807-856`).
- In `fill_limit_order_with_snapshot`, when queue position is on, this allowed quantity caps (and can empty) the fills, cancelling FOK/IOC orders that cannot fill (`mod.rs:4865-4888`).
- On trade ticks, `decrement_queue_on_trade` front-consumes the tracked depth FIFO and allocates a shared trade-size budget by queue priority (`mod.rs:712-802`). Book snapshots/clears and depth updates rebase positions via `rebase_queue_positions` and `seed_tob_baseline` (`mod.rs:859-938,1647-1717`). Modify re-indexes an order's queue position when its price changes (`mod.rs:3442-3449`).

### 5. Liquidity consumption

| Option                | Config field / path                                                 | Effect                                                                  | Citation                    |
| --------------------- | ------------------------------------------------------------------- | ----------------------------------------------------------------------- | --------------------------- |
| Liquidity consumption | `OrderMatchingEngineConfig.liquidity_consumption` (default `false`) | Fills consume displayed book liquidity so later orders cannot reuse it. | `config.rs:34-35`           |
| Venue option          | `BacktestVenueConfig.liquidity_consumption` (default `false`)       | Copied into the matching-engine config.                                 | `config.rs:345-347,513-514` |

Effect in the code: `apply_liquidity_consumption` tracks per-level `(original_size, consumed)` for the side being hit, returns fills capped to `original_size - consumed`, and resets consumption when the book level size changes; with the option off it returns fills unchanged (`mod.rs:334-408`). When enabled, fill determination uses `get_all_crossed_levels` instead of `simulate_fills` so consumed levels are filtered out while valid deeper levels are still found (`mod.rs:4380,4402,4540`). Trade ticks seed a per-trade consumption budget (`mod.rs:2473-2489`). FOK attempts journal and revert consumption if unfillable (`mod.rs:417-446`). Synthetic fill-model books and L1 trigger-price fills deliberately skip consumption (`mod.rs:4715-4737`).

### 6. Market status and halts

- `OrderMatchingEngine::process_status` maps actions onto `market_status` (`mod.rs:2556-2580`): `Trading`/`PreOpen` from Closed/Paused/Suspended -> Open; `Pause` from Open -> Paused; `Suspend` from Open -> Suspended; and `Halt | Close` from Open -> Closed. There is no distinct Halt state: a halt is treated exactly as a close.
- `SimulatedExchange::process_instrument_status` forwards the action to the matching engine (`crates/backtest/src/exchange.rs:1091-1114`).
- Matching is gated on `market_status == MarketStatus::Open`: both `iterate_bids` and `iterate_asks` (fill and stop-trigger actions) run only inside that branch, so halts/closes stop new fills and triggers (`mod.rs:4077-4103`).
- New orders are rejected while the market is not Open with a `Market <id> is <status>, cannot accept order ...` rejection (`mod.rs:3026-3034`).
- Expiration forces the status to Closed and cancels open orders (`mod.rs:2661-2665`).
- Auction behaviour: none was found. `PreOpen` only transitions the status to Open (`mod.rs:2560-2566`); there is no auction/uncrossing routine in the matching engine.

### 7. Partial fills

Partial fills do occur. There is no purely random partial-fill model; partial fills are size-driven and quantity-capped:

- Every fill model with finite synthetic level size produces partial fills when the order exceeds that size: `TwoTierFillModel` (10, `fill.rs:682-707`), `ThreeTierFillModel` (50/30/20, `fill.rs:784-823`), `LimitOrderPartialFillModel` (5, `fill.rs:899-924`), `SizeAwareFillModel` (10 or 50, `fill.rs:998-1021`), `CompetitionAwareFillModel` (`fill.rs:1104-1110`), `VolumeSensitiveFillModel` (`fill.rs:1196-1214`), and `MarketHoursFillModel` (500, `fill.rs:1301-1317`). `BestPriceFillModel` uses effectively unlimited size.
- `DefaultFillModel` returns `None`, so partial fills come from the real book's available liquidity instead.
- With `queue_position`, allowed fill quantity is capped by leaves quantity, remaining trade volume, and queue excess (`mod.rs:807-856,4865-4888`).
- `apply_fills` caps each fill to the order's remaining quantity (`mod.rs:5173-5183`), and `fill_order` caps the fill quantity to leaves (`mod.rs:5424-5430`). Orders unfilled after an IOC are cancelled (`mod.rs:5225-5228`).

### 8. Slippage models

Slippage is its own concern, separate from the fill model and the fee model:

- The `SlippageModel` trait has one method, `is_slipped() -> bool`, and answers only whether a fill price moves one tick against the order direction (`crates/execution/src/models/slippage.rs:33-40`). `SlippageModelHandle` is the shared runtime handle (`slippage.rs:43-75`).
- `ProbabilisticSlippageModel` is the built-in implementation. It holds `prob_slippage` and `random_seed` in a `ProbabilisticFillState`, exactly as the fill models do, so a seeded model reproduces its draws and a decomposed configuration reproduces a composite one draw for draw (`slippage.rs:83-134`). `SlippageModelAny` is the runtime enum with that one variant (`slippage.rs:139-164`).
- The matching engine holds `Option<SlippageModelHandle>` next to the fill and fee models (`mod.rs:116`) and `set_slippage_model` replaces it (`mod.rs:554-562`). When it is `Some`, `apply_fills` consults it and does not consult the fill model's own slippage; when it is `None`, the fill model decides (`mod.rs:5124-5142`).
- `SimulatedVenueConfig.slippage_model` is `Option<SlippageModelHandle>` (`config.rs:301-306`) and `BacktestVenueConfig.slippage_model` is `Option<SlippageModelAny>` (`config.rs:549-554`, accessor `config.rs:794-798`); `BacktestNode` maps it to a handle (`node.rs:290-291,305`).
- `SimulatedExchange` stores it (`exchange.rs:165`) and passes it to each matching engine it creates (`exchange.rs:256,541-543`). The sandbox client does the same (`crates/adapters/sandbox/src/execution.rs:164,173,912,1033-1035`), and its config field is runtime-only in serialization (`crates/adapters/sandbox/src/config.rs:101-114`).
- `ProbabilisticSlippageModel` is exposed through `nautilus_trader.execution` (`crates/execution/src/python/mod.rs:57`), and `pyobject_to_slippage_model_any` converts it at the Python boundary (`crates/execution/src/python/slippage.rs:47-70`).

The ordering the engine applies is: fill eligibility, then fill quantity, then base fill price, then the slippage adjustment, then the market impact adjustment, then the final fill price, then the fee. Slippage adjusts the base fill price by one price increment against the order direction on an L1 book, and the fee model is charged last, on the adjusted price and the fill quantity. Market impact is a separate, later addition and is documented in [section 9](#9-market-impact-models).

### 9. Market impact models

Market impact is its own concern, separate from the fill model, the slippage model, and the fee model. It is a later addition than those concerns, so the section is numbered last.

- The `MarketImpactModel` trait has one method, `impact_increments(fill_quantity)`, which returns the number of price increments the fill price moves against the order direction (`crates/execution/src/models/market_impact.rs:45-54`). `MarketImpactModelHandle` is the shared runtime handle (`market_impact.rs:58-90`).
- `LinearMarketImpactModel` is the built-in implementation (`market_impact.rs:110-160`). It moves the price one increment for every `quantity_per_increment` units filled, capped at `max_increments`, using exact decimal division and a floor. It is deterministic and takes no random seed. `MarketImpactModelAny` is the runtime enum with that one variant (`market_impact.rs:171-183`).
- The matching engine holds `Option<MarketImpactModelHandle>` next to the fill and fee models (`crates/execution/src/matching_engine/mod.rs:118`) and `set_market_impact_model` replaces it (`mod.rs:566-574`). `apply_fills` applies it to a liquidity-taking L1 fill after the slippage adjustment (`mod.rs:5157-5190`): it computes `increments * price_increment` with checked arithmetic and adds the offset for a BUY or subtracts it for a SELL. A model that returns zero, a fill on an L2 or L3 book, or a resting (maker) fill leaves the price unchanged.
- `SimulatedVenueConfig.market_impact_model` is `Option<MarketImpactModelHandle>` (`crates/backtest/src/config.rs:313`) and `BacktestVenueConfig.market_impact_model` is `Option<MarketImpactModelAny>` (`config.rs:566`, accessor `config.rs:813-815`); `BacktestNode` maps it to a handle (`crates/backtest/src/node.rs:293-295,309`).
- `SimulatedExchange` stores it (`crates/backtest/src/exchange.rs:167`) and passes it to each matching engine it creates (`exchange.rs:547-549`). The sandbox client does the same (`crates/adapters/sandbox/src/execution.rs:166-168,179,919,1043-1045`), and its config field is runtime-only in serialization (`crates/adapters/sandbox/src/config.rs:118-127,357-385`).
- `LinearMarketImpactModel` is exposed through `nautilus_trader.execution` (`crates/execution/src/python/mod.rs:59`), and `pyobject_to_market_impact_model_any` converts it at the Python boundary (`crates/execution/src/python/market_impact.rs:48-61`).

Unlike the fill and slippage models, market impact involves no randomness, so it has no seed. The field is absent by default, so the default path adjusts no fill price for size.

### Determinism and seeds

All randomness is confined to the fill and slippage models; latency, fees, and the matching engine itself are deterministic given the same inputs. The random state lives in `ProbabilisticFillState`, which seeds `StdRng` with `random_seed` when provided and otherwise uses `default_std_rng` (`crates/execution/src/models/fill.rs:166-214,250-263`).

| Random / time-dependent path          | Where                                                                                                 | Seed                                                                                                                                    | Citation                                                                                           |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Limit-fill decision `is_limit_filled` | all eleven fill models, via `ProbabilisticFillState::is_limit_filled`                                 | `random_seed: Option<u64>` on each model constructor; fully deterministic when `prob_fill_on_limit` is `0.0`/`1.0` due to short-circuit | `fill.rs:179-214,294-296` (defaults)                                                               |
| Slippage decision `is_slipped`        | all eleven fill models, or `ProbabilisticSlippageModel` when one is configured                        | same `random_seed`; default `prob_slippage=0.0` is deterministic                                                                        | `fill.rs:202-208,294-296`; `slippage.rs:83-134`                                                    |
| Market impact adjustment              | `apply_fills` via the venue market impact model                                                       | none; exact function of the fill quantity                                                                                               | `mod.rs:5157-5190`; `market_impact.rs:110-160`                                                     |
| One-tick-vs-best coin flip            | `ProbabilisticFillModel::get_orderbook_for_fill_simulation` (`random_bool(0.5)`)                      | same `random_seed`                                                                                                                      | `fill.rs:575-612`                                                                                  |
| Seeded RNG                            | `StdRng::seed_from_u64(seed)` when a seed is supplied                                                 | explicit seed honored                                                                                                                   | `fill.rs:189-192`                                                                                  |
| Unseeded RNG                          | `default_std_rng`: madsim `thread_rng` under a madsim runtime, otherwise `rand::rng()` (host entropy) | none -> not reproducible across runs                                                                                                    | `fill.rs:250-263`                                                                                  |
| Venue/position IDs                    | `IdGen::generate*` uses `UUID4::new()` when `use_random_ids` is true                                  | `OrderMatchingEngineConfig.use_random_ids` (default `false`); trade IDs are always deterministic                                        | `config.rs:44-45`; `crates/execution/src/matching_engine/ids_generator.rs:194-197,242-243,267-268` |
| Order submit/update/delete latency    | `ts_init + fixed duration`                                                                            | deterministic; no randomness                                                                                                            | `exchange.rs:865-892`                                                                              |
| Fee and price arithmetic              | fee models, matching-engine price logic                                                               | deterministic; no randomness or wall-clock reads                                                                                        | `fee.rs:135-660`; `mod.rs:4369-4560`                                                               |

A `FillModelConfig` forwards `random_seed` to the model it resolves to (`fill.rs:1544-1623`), so a
seeded configuration reproduces its draws across resolutions.

No fill, fee, latency, or queue-position path reads the wall clock; the engine advances time only from supplied data timestamps and explicit latency durations.
