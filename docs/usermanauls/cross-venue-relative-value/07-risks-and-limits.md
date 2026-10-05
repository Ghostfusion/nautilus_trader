# 07 - Risks and limits

This lecture is about what breaks when a cross-venue relative-value strategy runs for real, how to
size it, what limits to set, and which of those limits the engine enforces for you.

## 1. Leg risk

**Leg risk** is the risk that one leg of the pair exists and the other does not. It is the defining
risk of this style, because a pair trade is two orders at two venues and nothing makes them
atomic.

Lecture 05 measured it. With `NOTIONAL_CAP = {"BTCUSDT-PERP.BINANCE": "1000"}`, the output was:

```text
[1] ENTER spread_bps=12.04
DENIED BTCUSDT-PERP.BINANCE O-20231114-221343-001-000-1 reason=NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=1000.00000000 USDT, notional=64028.20000000 USDT
[1] EXIT spread_bps=4.94
DENIED BTCUSDT-PERP.BINANCE O-20231114-221606-001-000-3 reason=NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=1000.00000000 USDT, notional=63972.20000000 USDT
```

The Binance leg was denied and the Bybit leg filled anyway. The strategy held a naked short of
1 BTC between entry and exit, and the run finished `+22.03` instead of `-85.17` purely because the
price moved in the direction that happened to suit an unhedged short. Three consequences:

1. **The engine does not roll back a pair.** It has no concept of the pair. Each order is
   risk-checked and routed on its own; see `crates/risk/src/engine/mod.rs` and
   [`../../concepts/execution/index.md`](../../concepts/execution/index.md).
2. **An order list cannot fix it across venues.** An order list requires every order in the list to
   use the same venue (`docs/concepts/orders/advanced.md`). Two venues means two submissions.
3. **Your `_open` flag is a statement of intent.** The strategy in lecture 05 set `_open = True`
   before either order was submitted. A denial afterwards left the flag lying. The fix is to count
   fills and reconcile the two legs to their positions, not to a boolean.

How to reduce leg risk, in the order of how much it helps:

- **Submit the illiquid leg first.** If the thin leg fails, nothing else was sent. This inverts the
  problem: now a fill on the thin leg leaves you naked until the hedge leg arrives.
- **Reconcile on position events.** In `on_position_opened` and `on_position_closed`, check both
  instruments; if one leg has a position and the other does not, flatten the one that does. That is
  the strategy owning the hedge, which is what the engine expects.
- **Cap the size so that a naked leg is survivable.** The size of one leg is your unhedged
  exposure, in the worst case, for as long as the repair takes.
- **Use a limit order on the leg you are willing to miss.** A market order always trades and always
  pays the taker fee; a limit order may not fill at all, which leaves you flat instead of naked.

## 2. Partial fills on one leg

A partial fill is the same problem in smaller size, and it is harder to see. The matching engine
fills what the book can support; `DefaultFillModel` fills fully and never slips, which is a
convenient fiction. The other shipped models are probabilistic: `ProbabilisticFillModel` fills a
limit order with probability `prob_fill_on_limit` and slips with probability `prob_slippage`, and
`ProbabilisticSlippageModel` is the slippage half alone. They are bound in
`crates/execution/src/python/fill.rs` and implemented in `crates/execution/src/models/fill.rs`.

With size 1.000 BTC and a top of book holding 1.000 BTC in the fixture, everything fills. That is a
property of the fixture, not of the market. On a real venue a 1.000 BTC market order against 0.300
BTC of displayed liquidity fills 0.300 and walks the book for the rest, so the two legs end up with
different quantities. The engine then has two correctly-sized positions with different sizes, and
your PnL is a hedged core plus a directional residual. The only defence is to test the fill
quantity on every fill event and to send a corrective order for the difference.

## 3. Venue outages

Live venues halt. A matching engine goes into a non-trading state, an API returns errors, a
WebSocket disconnects, or the venue freezes withdrawals while remaining open for trading. Each of
those has a different effect on a pair:

| Failure                     | Effect on the pair                                                      |
| --------------------------- | ----------------------------------------------------------------------- |
| Trading halted on one venue | You cannot open or close that leg. The other leg is naked.              |
| Data feed down on one venue | You are trading on a stale price. The basis you see is fiction.         |
| Withdrawals frozen          | Your hedge is trapped on the wrong venue. Convergence does not pay you. |
| API errors on one venue     | An order may or may not exist. You must reconcile before acting.        |
| Both venues down            | You hold whatever you hold until they return.                           |

The engine does not model any of this in a backtest. What it does give you is the machinery to
handle it live: per-client startup reconciliation of cached order and position state against venue
reports ([`../../concepts/execution/reconciliation.md`](../../concepts/execution/reconciliation.md)
and [`../../concepts/live.md`](../../concepts/live.md)), and per-instrument trading-state checks in
the risk engine, which deny an order when the instrument is not `ACTIVE`. A live relative-value
strategy must additionally treat "the other venue is not responding" as a reason to flatten rather
than to wait.

## 4. Transfer and settlement time

This is the cost that kills small cross-venue bases. Moving an asset between venues means a
withdrawal on one side, a transfer, and a deposit on the other. During that window:

- You pay a withdrawal fee.
- You are exposed: one leg exists, the other does not, for the whole transfer.
- The basis can close without you.

The backtest does not model transfers at all. There is no balance movement between two venues in
the engine, because the two venues are two separate accounts. If your strategy needs the asset on
the second venue to open the short, the transfer is a real-world prerequisite that no backtest
will show you. The way to avoid it is to keep inventory on both venues and rebalance rarely, and to
size so that the rebalance is worth its cost.

Settlement time also applies to anything that is not the same instrument on both sides. A derivative
settles in its settlement currency, spot settles in the base asset, and a bank transfer settles
when the bank says so.

## 5. Position sizing and loss limits

Size the trade from the worst case, not the hoped-for case. Four numbers, and the smallest wins:

| Limit                     | How to set it                                                                                                                                                          |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Per-leg notional cap      | The most you are willing to have naked on one venue. Set it with `RiskEngineConfig(max_notional_per_order={...})`, per instrument.                                     |
| Total pair notional       | Per-leg cap times the number of venues, checked against total account equity.                                                                                          |
| Margin headroom per venue | Each leg has its own margin account. Keep enough free margin on each venue that a large adverse move does not liquidate one leg before the other leg's profit arrives. |
| Maximum loss per pair     | In bps of the entry basis. If the basis widens past it, close both legs.                                                                                               |

The backtest in lecture 05 used a trade size of `1.000` BTC, which is about 64,000 USDT per leg.
The account report showed `160.07` locked on BINANCE and `160.26` locked on BYBIT while the pair
was open, and zero on both venues afterwards. Read the locked figure from your own run rather than
assuming it: it is derived from the venue's margin model, which you configure per venue with
`default_leverage`, `leverages`, or `margin_model` in `BacktestEngine.add_venue`.

Three limits worth setting in your own code, because the engine will not set them:

1. **A maximum holding time.** A basis that has not converged in your recorded half-life is not the
   trade you thought it was. The design record for relative-value screens makes the same point
   about a persistence estimate: `docs/design/relative_value_screening.md`, section 2.2, shows that
   a fitted half-life is biased in the direction that admits the pairs the gate is meant to
   exclude.
2. **A maximum number of pairs.** Each new pair multiplies the number of ways two legs can
   disagree.
3. **A stop on the pair's mark-to-market**, measured on the sum of the two legs, not on either leg.

## 6. What the engine enforces, and what it does not

The risk engine is a component of every NautilusTrader system, backtest included. Unless
`RiskEngineConfig(bypass=True)` is set, it validates, per order
([`../../concepts/execution/index.md`](../../concepts/execution/index.md), "Risk engine"):

- Price and trigger-price precision for the instrument.
- Positive prices, unless the instrument allows negative prices.
- Quantity precision and base-quantity minimum and maximum bounds.
- That a GTD order has not already expired.
- That `reduce_only` orders do not increase the referenced position.
- Engine-level `max_notional_per_order`, and the instrument's `min_notional` and `max_notional`.
- Cash-account balance impact for non-margin accounts.
- Submit and modify rate limits.
- Trading-state restrictions (`ACTIVE`, `HALTED`, `REDUCING`).

A failed submit-time check produces an `OrderDenied` event with a standardised reason code; the
lecture 05 output showed one, `NOTIONAL_EXCEEDS_MAX_PER_ORDER`, with the cap and the observed
notional in the message. A failed modify-time check produces `OrderModifyRejected`.

**What the engine does not enforce for this style:**

| Not enforced                      | Consequence                                                                                                                                                                                                                                   |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Two-leg atomicity                 | A denial or rejection on one leg leaves the other leg live. Section 1.                                                                                                                                                                        |
| Cross-venue netting               | Two venues means two positions, always. The engine never merges them.                                                                                                                                                                         |
| Transfer and withdrawal cost      | Never modelled. Section 4.                                                                                                                                                                                                                    |
| Funding unless you feed it        | A `FundingRateUpdate` in the data settles funding; without one there is no carry.                                                                                                                                                             |
| Borrow cost on a real short       | Not modelled. Spot shorts need a lender.                                                                                                                                                                                                      |
| Partial-fill repair               | The engine reports the fill; repairing the hedge is your code.                                                                                                                                                                                |
| A limit on how many pairs you run | Set `max_order_submit_rate` and `max_order_modify_rate`, or count pairs in your own code.                                                                                                                                                     |
| Pre-trade count caps from Python  | `RiskEngineConfig.count_caps` takes `RiskCap` values, `RiskCap(RiskCapMetric.Submit, RiskCapScope.Instrument, 2_000, 60_000_000_000)`, with the metric and scope vocabularies in `nautilus_trader.risk` (`crates/risk/src/python/config.rs`). |

A live node carries the same caps as `METRIC/SCOPE/LIMIT[/WINDOW_NS]` strings, so a count-based cap
is a configuration on either surface rather than a counter you write.

## 7. A paper spread is not a tradeable spread

The single most useful habit in this style is to compute, at the moment of the signal, three
numbers and print them:

1. **Paper spread**: the mid-to-mid basis, in bps. Lecture 05's entry signal was `12.04`.
2. **Touch spread**: sell the rich venue at its bid, buy the cheap venue at its ask. In the
   fixture, BYBIT's half-spread is 0.8 USDT and BINANCE's is 0.5 USDT, so the touch spread is
   exactly 1.3 USDT narrower than the paper spread, which at 64,000 USDT per BTC is 0.20 bps.
3. **Net of costs**: the touch spread minus `2 * (fee_A + fee_B)` in bps, and minus any carry you
   expect to pay or receive.

If number 3 is not comfortably positive, number 1 is decoration. In lecture 05, number 1 was 12.04
bps, number 3 was about `12.04 - 0.20 - 20 = -8.16` bps, and the realised result was a loss of
85.17 USDT. The paper spread was never a tradeable spread.

The fixture's half-spreads are small on purpose so that the fee difference between the two venues
is the visible effect. On a real pair the touch cost is frequently the larger of the two: a wide
half-spread on the leg you must cross can exceed both taker fees combined.

## 8. A production checklist

Before running this style with money:

1. Both venues' fee schedules are recorded as constants and used in the signal, not left implicit.
2. The entry threshold is above `2 * (fee_A + fee_B)` plus two half-spreads, with a margin for
   slippage.
3. Every leg has an executed-quantity check and a repair path for a partial fill.
4. The strategy reconciles positions on start and after every fill, and never trusts a boolean.
5. There is a maximum holding time and a maximum adverse basis.
6. Margin headroom on each venue is checked before sizing, not after a liquidation.
7. Transaction fees, withdrawal fees and funding are written down as costs, not discovered later.
8. The backtest's fill model is not believed. `DefaultFillModel` fills fully and never slips; the
   probabilistic models exist for a reason.

Continue to [08-exercises.md](08-exercises.md).
