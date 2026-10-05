# 07 - Risks and limits

This lecture is about failure. It explains what happens when a child order is refused, what happens
when the horizon expires, why churn is dangerous, and the rule that keeps the parent's quantity
consistent. It ends with the limits the engine enforces and the ones it does not.

## What happens when a child is denied

A spawned order passes through the same risk checks as any order. If a check fails, the engine emits
an `OrderDenied` event with a standardized reason
([Order denied reasons](../../concepts/execution/index.md#order-denied-reasons)). In
[05-build-the-strategy.md](05-build-the-strategy.md) the remainder slice of 1 unit was denied with:

```text
QUANTITY_BELOW_MINIMUM: effective=1, min=1000
```

The execution algorithm sees that denial. The core publishes the denial to the algorithm, and
restores the spawn's unfilled quantity to the primary while the primary is still local
(`crates/trading/src/algorithm/mod.rs:1524-1533`). The restore itself is
`restore_primary_order_quantity` (`mod.rs:748`), and the doc summary is at `mod.rs:100-105`:

> When a reduced spawned order terminates with unfilled quantity, its reduction is restored in
> primary quantity units while the primary remains locally mutable.

The two child-driven algorithms take a stronger action. Iceberg and sniper call
`abort_on_child_refusal`, which denies the primary with the child's reason and completes the sequence
instead of retrying (`crates/trading/src/algorithm/iceberg.rs:573-589`,
`crates/trading/src/algorithm/sniper.rs:603-619`). The source comment is explicit: "A refused child is
not retried: the sequence completes and the primary carries the refusal, so the algorithm is never
left neither quoting nor completing." The sniper header says the same at `sniper.rs:27-28`.

TWAP is different. It does not abort on a child denial; the core restores the quantity, and TWAP
continues on its timer. This is why the 6001 example ended with the parent `DENIED` only after every
child had been attempted.

## What happens when the horizon expires

The horizon is the deadline. Its handling differs per algorithm:

- TWAP uses the horizon only to compute the slice count. Its last slice is the parent order itself,
  and it submits that last slice on the final timer (`crates/trading/src/algorithm/twap.rs:445-449`).
  If the parent is already closed when a timer fires, TWAP completes the sequence
  (`twap.rs:431-434`).
- Iceberg and sniper set a timer for the horizon when they start. When it fires they cancel the
  working child and complete the sequence, leaving whatever quantity did not execute
  (`iceberg.rs:498-520`, `sniper.rs:517-540`). The sniper header notes that without a horizon the
  sequence is open indefinitely, which "leaves the sequence open" by design (`sniper.rs:29-34`).

Expiry does not mean the algorithm tries harder. There is no remaining-quantity panic submission; a
partially executed parent is a normal, reportable outcome.

## Churn

**Churn** is the repeated cancelling and re-sending of child orders. It costs in three ways:

1. Each cancel and re-send is a message to the venue, and venues rate-limit or reject bursts.
2. A re-quoted resting order loses its place in the queue, so it is less likely to fill passively.
3. It makes the execution harder to interpret, because the number of fills stops matching the number
   of slices.

Iceberg and sniper protect against fast churn with `requote_secs` or the touch-change rule. In the
iceberg run you saw exactly one cancellation (`E1 CANCELED`) followed by re-quotes
(`E2` ... `E7`). The schedules keep counters for the churn evidence: `submitted` and `cancelled` at
`iceberg.rs:625-628` and `sniper.rs:651-654`, which feed the `child_churn` metric in the Rust
analytics crate.

As a beginner, watch the ratio of children to slices. Six slices and six children is clean. Six
slices and forty children means the re-quote rule is firing constantly; raise `requote_secs` or use a
wider price.

## The parent quantity conservation rule

The rule is documented on the concept page and is worth restating because it is the one invariant
that keeps a sliced execution coherent:

> If a spawned order is denied, rejected, canceled, expired, or refused before submission, its
> unfilled proportion is restored in the primary order's quantity units while the primary remains
> local. Once primary submission is handed off, its quantity is committed and is not changed by a
> later spawn outcome.

Source: [Execution algorithms](../../concepts/execution/algorithms.md). The implementation restores
on the event in `crates/trading/src/algorithm/mod.rs:1524-1572`, and re-deducts a late fill that
arrives after a restore (`mod.rs:899`).

In the lecture 03 run this rule reads as an arithmetic identity:

```text
parent original quantity      = 6000
children spawned (E1..E5)      = 5 * 1000 = 5000
parent final slice submitted   = 1000
5000 + 1000                    = 6000        # nothing created, nothing lost
```

The parent's quantity field shows `1000` at the end, not `6000`, because the field tracks the
remaining local quantity, and the final slice was the parent's own order.

The rule also has a subtle edge. A **late fill** on a spawn that was already cancelled arrives after
its quantity was restored. The core re-deducts the late fill's quantity from the primary while the
primary is still locally mutable; if that quantity was already reused by a later spawn, the excess is
netted from that spawn's own restoration (`mod.rs:899-960`, and the concept page).

## Position sizing

None of the four algorithms sizes a position for you. You choose the parent quantity, and the
algorithm divides it. A common beginner mistake is to size from the account and let the algorithm
spread it; that is fine, but the whole parent is a commitment. If the market runs while you work,
every unfilled slice is a slice you still owe.

A sane workflow:

1. Decide the maximum position you want, and the loss you can tolerate.
2. Size the parent so that a bad fill on the whole parent is still inside the loss budget.
3. Choose a horizon that the available liquidity can absorb. A 6,000-unit order in 60 seconds is
   aggressive; the same order over an hour is easier for the market.
4. Backtest with the same horizon and size you intend to trade.

## What the engine enforces

For every order, the risk engine checks the things listed in
[Risk engine](../../concepts/execution/index.md#risk-engine): instrument price and quantity
precision, positive prices where the instrument allows them, quantity minimum and maximum, GTD
expiry, `reduce_only`, notional limits, cash balances for non-margin accounts, submit and modify
rates, and the trading state. A failure produces a denial, not a silent skip.

In Python you can set `max_order_submit_rate`, `max_order_modify_rate`, `max_notional_per_order` and
`count_caps` on the risk configuration (`crates/risk/src/python/config.rs`). A cap is a
`RiskCap(metric, scope, limit, window)`, with the metric and scope vocabularies in
`nautilus_trader.risk`.

## What the engine does not enforce

- **No loss limit.** The engine does not stop a strategy that is losing money. Risk checks are about
  validity and exposure, not about profit and loss.
- **No participation rate.** None of the four shipped algorithms honours `participation_rate`; every
  one of them refuses it by name. If you want to trade a fraction of volume, you must write a custom
  algorithm or slice manually.
- **No slippage cap.** `max_slippage_bps` is likewise refused by all four.
- **No urgency.** `urgency` is refused by all four.
- **The horizon is not a liquidation guarantee.** For TWAP the horizon is a schedule; for iceberg
  and sniper it is a deadline that cancels and stops. Neither guarantees the parent is fully filled
  by the horizon.
- **A backtest venue requires an explicit fee model.** If you omit it, `add_venue` raises
  `ValueError: Backtest venue requires an explicit fee_model, including an explicit zero-fee model`.
  A backtest with no fee model is not a real result.

## Production failures to expect

| Failure                                | What you see                                               | Response                                               |
| -------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------ |
| A child is refused                     | `OrderDenied` with a code such as `QUANTITY_BELOW_MINIMUM` | Fix the schedule; the primary quantity is restored.    |
| The horizon expires with quantity left | Working child cancelled; parent still open                 | Report the unfilled quantity; it is a real cost.       |
| Venue rejects the child                | `OrderRejected` from the venue                             | Iceberg and sniper abort the sequence; TWAP restores.  |
| Churn                                  | Far more children than slices                              | Raise `requote_secs`, widen the price, or reduce size. |
| A late fill on a cancelled child       | Quantity re-deducted from the primary                      | None if the engine handled it; verify the position.    |
| Duplicate late fills                   | Repeated fills after a cancel                              | The engine rejects a repeated `trade_id`.              |

Read [08-exercises.md](08-exercises.md) to practise all of this.
