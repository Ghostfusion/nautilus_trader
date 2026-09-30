# Target pipeline

The target pipeline is an optional path from a statement of view to the orders that reach a venue.
It sits between the decision a strategy makes and the order submission path that already exists, and
it changes nothing about that path.

The pipeline is three separate layers with distinct ownership:

- **Signal**: a statement of view about one instrument. It carries a direction, an optional
  horizon, strength, source, expiry, and provenance. It is not a trading command and it carries no
  order quantity.
- **Target**: a desired exposure for one instrument, stated as a quantity, a weight, or a notional.
  It is the exposure a view resolves to after portfolio and risk context, not the change to the
  current exposure.
- **Order**: the existing order types. An order is the only one of the three that reaches a venue.

A signal is not a target and a target is not an order. A signal states what a component believes; a
target states the exposure that belief resolves to; an order states how the exposure is reached. None
of the three is a trading command: the pipeline returns order values as data and never submits,
cancels, or modifies anything.

## Ownership

The cache and the portfolio remain authoritative. The pipeline builds a snapshot of their state,
reconciles targets against it, and returns the minimal order set as values. It stores no position
state of its own and never becomes a second source of position truth, so targets cannot drift from
the account they describe. The caller submits the returned values on the existing strategy-to
`ExecutionAlgorithm` path.

The two stages hold configuration and plain data only:

- **Construction** resolves each signal to a target using a portfolio context (equity, instrument
  definitions, entry prices, and position state). A directional signal is sized by the fixed-risk
  sizing calculation in `nautilus_risk::sizing`, and the resulting exposure is expressed as a weight
  capped by the configuration. A flat signal states reduce-to-zero and needs no context.
- **Reconciliation** resolves each target to a signed quantity, nets it against the position and the
  resting orders in the snapshot, and emits one order per target whose difference is worth
  submitting. A delta that is zero, below the minimum order quantity, or that rounds to zero at the
  instrument's size increment produces no order.

Neither stage reads a clock, and neither mutates its inputs. For equal inputs they return equal
results in the same order.

## Opt-in

The pipeline is opt-in per strategy and disabled by default. A strategy holds no pipeline until it
enables one, so a strategy that never uses it is byte-identical to one written before the pipeline
existed. The direct path, where a strategy builds an order and submits it, is unchanged and remains
first-class: both paths converge on `ExecutionAlgorithm`.

From Python, a strategy controls the pipeline with:

- `enable_target_pipeline(config)`: enables the pipeline from a `TargetPipelineConfig`, which bundles
  the fixed-risk construction settings and the minimum order quantity. Configuration is validated
  here, so an invalid value raises at construction.
- `disable_target_pipeline()`: returns the strategy to the direct path.
- `target_pipeline_enabled()`: reports whether the pipeline is enabled.
- `submit_signals(signals)`: constructs a target per signal, reconciles them against the cache and
  the portfolio, and submits each resulting market order on the existing path, returning the client
  order IDs in the order the orders were emitted.

## Degenerate inputs

The pipeline reports an input it cannot resolve rather than silently omitting it, because a dropped
reduce-to-zero instruction would leave an unwanted position open. When a signal names an instrument
the cache holds no definition for, or no price for, construction reports it and the submission does
not happen, so a missing input surfaces as an error rather than a partial batch.

An entry price is resolved from the cache for each signal instrument, preferring the last trade
price, then the mid quote price, then the mark price. A price that is absent from all three is a
missing price, and construction reports it.

## Where it lives

The construction and reconciliation stages live in `crates/trading/src/target.rs`, and the pipeline
that binds them lives in `crates/trading/src/target_pipeline.rs`. The signal and target values live
in `nautilus_model` because they cross component boundaries. The pipeline is not registered on an
engine or a strategy; a strategy holds one only after it opts in.
