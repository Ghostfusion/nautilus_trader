# 09 - Go further

This lecture is honest about what the manual did not cover, points at the concept pages that go
deeper, and says what to learn next.

## 1. What this manual did not do

Each item below is a real limitation of the example, not a defect to be hidden.

| Gap                     | Why it matters                                                                                                                                                                                                                                                    |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| No real adapter         | The two venues are simulated. A live pair needs two data clients and two execution clients, with credentials, rate limits and venue-specific order types.                                                                                                         |
| No transfer modelling   | The engine has no notion of moving an asset between two venues. Withdrawal fees and transfer time must be handled outside the backtest.                                                                                                                           |
| No borrow cost          | A real short has a lender and a fee. The simulated short does not.                                                                                                                                                                                                |
| No cointegration screen | The pair here is constructed to be cointegrated. A real screen must estimate a hedge ratio, test the relationship, and count its trial family. `docs/design/relative_value_screening.md` is the repository's own design for what such a declaration must contain. |
| No partial-fill repair  | The fill model fills fully. A live strategy needs a repair path.                                                                                                                                                                                                  |
| One pair, four minutes  | No statistics computed over this sample carry any weight. The engine's own annualised statistics correctly report `NaN`.                                                                                                                                          |
| No execution algorithms | Both legs are single market orders. Slicing, TWAP and the other execution algorithms are covered by the `execution-algorithms` manual and by [`../../concepts/execution/algorithms.md`](../../concepts/execution/algorithms.md).                                  |

## 2. The concept pages to read next

In the order that follows this manual:

1. [`../../concepts/instruments/index.md`](../../concepts/instruments/index.md) for the instrument
   model, including `crypto_perpetual.md` and `currency_pair.md`.
2. [`../../concepts/data/index.md`](../../concepts/data/index.md), then
   [`../../concepts/data/funding_rate_update.md`](../../concepts/data/funding_rate_update.md),
   [`../../concepts/data/index_price_update.md`](../../concepts/data/index_price_update.md) and
   [`../../concepts/data/mark_price_update.md`](../../concepts/data/mark_price_update.md).
3. [`../../concepts/positions.md`](../../concepts/positions.md) for netting, hedging and the
   position lifecycle.
4. [`../../concepts/accounting.md`](../../concepts/accounting.md) for the balance invariant, margin
   scopes and the portfolio query surface.
5. [`../../concepts/execution/index.md`](../../concepts/execution/index.md) and
   [`../../concepts/execution/reconciliation.md`](../../concepts/execution/reconciliation.md) for
   the paths an order takes and how live state is recovered.
6. [`../../concepts/synthetics.md`](../../concepts/synthetics.md) if you want the engine to derive
   the spread as a synthetic price. Remember that a synthetic cannot be traded.
7. [`../../concepts/universes.md`](../../concepts/universes.md) if your pair set changes during a
   run, and [`../../concepts/continuous_futures.md`](../../concepts/continuous_futures.md) if your
   legs are expiring contracts rather than perpetuals.
8. [`../../concepts/orders/advanced.md`](../../concepts/orders/advanced.md) for what an order list
   can and cannot do, including the same-venue restriction that stops a cross-venue pair from being
   one list.

## 3. A worked relative-value tutorial in the repository

Two existing tutorials run a delta-neutral, three-leg relative-value strategy live, in Rust:
[`../../tutorials/delta_neutral_options_bybit.md`](../../tutorials/delta_neutral_options_bybit.md)
and [`../../tutorials/delta_neutral_options_derive.md`](../../tutorials/delta_neutral_options_derive.md).
They sell an out-of-the-money call and put and hedge the resulting delta with a perpetual, which is
the same idea as this manual with a different hedge instrument:

- The hedge instrument is a perpetual, exactly as here.
- The hedge trigger is a venue-provided delta rather than a price difference, so the relationship
  between the legs comes from the venue's greeks rather than from a spread you compute.
- The venue difference between the two tutorials is a single parameter name: Bybit takes `order_iv`,
  OKX takes `px_vol`, and Derive signs an explicit premium. That is a concrete example of how much
  of a cross-venue strategy is venue plumbing.

Both are marked Rust-only because they run the Rust `LiveNode`. Read them after you are comfortable
with the Python backtest in this manual.

## 4. Rust-only machinery you will meet

Not everything in the repository has a Python binding. One subsystem is relevant to this style:

**Execution analytics**, in `crates/trading/src/analytics`, has no Python bindings. It computes
implementation shortfall against a decision price (the metric constant
`METRIC_IMPLEMENTATION_SHORTFALL_BPS` in `crates/trading/src/analytics/metrics.rs`), which is the
natural measure of what a two-leg entry cost you relative to the price you decided on. This manual
does not use it; it computes the equivalent numbers by hand from the fills report, which is also the
best way to learn what the metric means. To see the crate's own tests:

```bash
export CARGO_TARGET_DIR='D:/Users/vince/PycharmProjects/nautilus_trader/target'
cargo nextest run --locked -p nautilus-trading --features python -E 'test(analytics)'
```

The seven analytics tests all pass:

```text
    Starting 7 tests across 1 binary (962 tests skipped)
        PASS [   0.028s] (1/7) nautilus-trading analytics::tests::pinned_metrics_match_hand_computed_values
        PASS [   0.035s] (2/7) nautilus-trading analytics::tests::adverse_selection_uses_fill_plus_horizon_reference
        PASS [   0.045s] (3/7) nautilus-trading analytics::tests::algorithm_child_counts_override_observed_counts
        PASS [   0.053s] (4/7) nautilus-trading analytics::tests::passive_fill_spread_capture_is_positive
        PASS [   0.060s] (5/7) nautilus-trading analytics::tests::observer_is_send_and_sync_friendly
        PASS [   0.086s] (6/7) nautilus-trading analytics::tests::adverse_selection_requires_a_declared_horizon
        PASS [   0.092s] (7/7) nautilus-trading analytics::tests::undefined_metric_is_not_available_never_zero
     Summary [   0.097s] 7 tests run: 7 passed, 962 skipped
```

If you need a capability that is Rust-only, the pattern is: confirm it in `crates/<crate>/src/`,
confirm the absence of a binding in `crates/<crate>/src/python/`, and then either configure it in
Rust or implement the equivalent in Python on top of the Python surface.

## 5. What to learn next, in order

1. **Run the `execution-algorithms` manual.** A two-leg entry is two parent orders; the same slicing
   machinery applies to each leg independently.
2. **Run the `microstructure-signals` manual** if you want to quote the spread rather than cross it.
   Crossing costs two taker fees; quoting converts them into a maker rebate and a fill probability.
3. **Read the repository's relative-value design record**, `docs/design/relative_value_screening.md`,
   in full. It is the project's own statement of what a screen must declare, what it must refuse,
   and how a fitted half-life, a hedge ratio and a window convention each go wrong.
4. **Measure one real pair.** Use two venues you can actually trade, record both books for a week,
   and compute the same four numbers from lecture 06, section 8: paper basis, executable basis,
   total cost, and ratio. The manual's numbers are illustrative; the discipline is not.
5. **Then, and not before, consider a live run.** Start in a sandbox or on testnet, with the
   smallest size the venue allows, and with the repair path of lecture 07 tested before the first
   real order.

## 6. Summary of the style in one place

- Relative value buys the cheap leg and sells the rich leg of the same economic thing.
- Cross-venue means two accounts, two fee schedules, two margin calculations and two positions,
  always.
- The profit is convergence and carry; the cost is two taker fees per round trip, two half-spreads,
  transfers and adverse funding.
- The engine executes each leg independently and does not roll back a pair. Leg risk is your code's
  responsibility.
- A paper spread that is not several times its cost is decoration.

Back to the [course index](README.md).
