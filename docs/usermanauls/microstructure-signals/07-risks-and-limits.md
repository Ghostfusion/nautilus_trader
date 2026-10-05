# Risks and limits

Lecture 05 ran a strategy that bought because a thin ask made the book lean up. In production, four
kinds of failure eat that strategy: the signal decays, the edge was fitted to one venue, the feed
does not behave like a file, and the clock lies. This lecture names each one, then covers position
sizing and what the engine actually enforces.

## 1. Signal decay

The imbalance signal is not a constant property of a market. It appears where a venue is slow or
under-competes, and it disappears as participants speed up. Three concrete stages of decay:

- **Within seconds.** The market maker who was resting the thin side cancels and reposts, exactly
  as lecture 01 described. The imbalance you measured is gone.
- **Within a session.** A venue's message rate changes through the day. A trigger threshold tuned
  to the open will fire constantly at the close, when the book is thin, or never, when it is deep.
- **Across months.** New participants, new order types, fee changes and venue changes alter the
  book's behaviour. A backtest from last year is a hypothesis about last year, not about now.

The practical defence is not a better threshold. It is to measure the signal's response time and
half-life on data the fitting never saw, and to keep the strategy's holding period shorter than
that half-life. The engine gives you the fills and positions to do this; it cannot do the reasoning
for you.

## 2. Overfitting to one venue

Book imbalance is the most venue-specific signal in this manual. Two venues listing the same
instrument have different books, different tick sizes, different fee schedules and different
participants. A ratio threshold of 0.20 that fires beautifully on Binance may be the median on
another venue, so it fires on everything or nothing.

Symptoms that you have fitted to one venue:

- The strategy is profitable on one file and flat or negative on every other.
- Changing the venue changes the number of triggers by an order of magnitude.
- The chosen threshold sits at the tail of the venue's own ratio distribution. In the repository's
  own gold tutorial (`docs/tutorials/gold_book_imbalance_ax.md`) the distribution's mass sits left
  of the 0.10 threshold, so that threshold is addressable; a threshold in the dense middle would be
  not.

The defence is to hold out a venue, not only a date. Fit on one venue, test on another that trades
the same instrument, and only then believe the threshold.

## 3. Snapshot versus delta stream

The two sample files contain snapshots and incremental deltas in the same file, which is convenient
but unusual. In a live feed they arrive differently, and treating one as the other corrupts the
book.

|                         | Snapshot                                 | Delta stream                           |
| ----------------------- | ---------------------------------------- | -------------------------------------- |
| Contains                | Complete book state                      | Changes since the last message         |
| If you miss one         | The next snapshot repairs the book       | The book is permanently wrong          |
| If you double-apply one | Usually harmless if it clears first      | Size is added twice; the book is wrong |
| Cost                    | Rebuilds the whole book                  | Cheap per message                      |
| Correct handling        | Emit `CLEAR` then adds, set `F_SNAPSHOT` | Apply in order, keep `sequence`        |

The engine's rules for this are explicit in `docs/concepts/order_book.md`:

- Out-of-order and snapshot deltas are **applied, not rejected**, so a replay still reaches the
  state the events describe. Only the metadata is protected: `ts_last` never regresses, and
  `sequence` never regresses except across a full clear.
- A stale update logs one warning per regressed field, `sequence` and `ts_event` independently.
  Incremental deltas log once per stale delta; snapshot deltas once per snapshot; depth snapshots
  once, because an `OrderBookDepth` replaces the book in one update.
- Some feeds restart their sequence counter when they clear the book. A full clear **without**
  `F_SNAPSHOT` is checked against the old high-water and then becomes the new high-water.
  Snapshot-flagged clears preserve the current high-water.
- `depth` in a subscription is not a wildcard: `depth=None` selects the adapter default and does
  not match an explicit depth. Two consumers sharing a source must agree on client, book type,
  depth and parameters.

The dangerous part is the silent one: a missed delta is not rejected. The book becomes wrong and
the signal trades on the wrong book. Run `book.check_integrity()` yourself after rebuilding, and
watch the stale-update warnings in the log.

A related trap is the interval subscription. It publishes the cached book on a timer, and during a
feed outage it can keep publishing the last cached book, so the strategy sees a live-looking but
frozen book. Depth callbacks marked `managed=False` do not update the cached book at all; if you
read the cached book expecting them to, you read stale state.

## 4. Clock and sequencing hazards

Time is not decoration. The engine stores every timestamp as integer nanoseconds since the Unix
epoch, and it uses `ts_event` (when the venue produced the event) and `ts_init` (when the engine
initialised it) for different purposes. Four hazards follow.

- **Unit confusion.** The sample fixture's `timestamp` is in **microseconds**, not nanoseconds. A
  loader that forgets to multiply by 1000 produces events in the year 1970 or the far future, and
  the book is empty or nonsensical. Lecture 04 showed the conversion.
- **Clock skew.** `local_timestamp` is the recorder's receive time, not the venue's. The gap between
  `timestamp` and `local_timestamp` is network delay, and it is exactly the delay your signal has
  already lost.
- **Reordering.** Real adapters buffer and reorder. If you sort by the wrong field, a snapshot can
  arrive after an update that depends on it and wipe state that should have survived. Sort by
  `ts_init` before replay, as `examples/backtest/crypto_orderbook_imbalance.py` does.
- **Sequence gaps.** A jump in `sequence` means a message was lost. The engine compares sequences
  and warns, but it does not stop your strategy. You must decide: rebuild from a snapshot, or halt.

If your timestamps are wrong, every other measurement is wrong, and no amount of tuning repairs it.

## 5. Position sizing and loss limits

The strategy in lecture 05 caps each order at `max_trade_size`, and sizes it to the available size
at the level, whichever is smaller. That is a per-order cap, not a position cap. Repeated triggers
in one direction accumulate a position, exactly as the repository's own gold run accumulated a
short over a day (`docs/tutorials/gold_book_imbalance_ax.md`). Three layers control this.

**Layer 1: your strategy.** Add a position limit. Before submitting, check
`self.portfolio.net_position(self._instrument_id)` (or `is_net_long` / `is_net_short`, used in
`examples/live/architect_ax/strategies.py`) and refuse to add beyond a bound. The engine does not
do this for you.

**Layer 2: the Python risk engine.** `RiskEngineConfig` exposes three limits to Python
(`python/nautilus_trader/risk/__init__.pyi`, implemented in `crates/risk/src/python/config.rs`):

| Field                    | What it limits                                                      |
| ------------------------ | ------------------------------------------------------------------- |
| `max_order_submit_rate`  | How many orders may be submitted per time window, as a rate string. |
| `max_order_modify_rate`  | How many order modifications per time window.                       |
| `max_notional_per_order` | Maximum notional value per order, per instrument.                   |

These are real pre-trade checks: an order that violates one is denied before it reaches the venue.

**Layer 3: the count caps.** The pre-trade send, cancel and fill count caps (decision D1) are
`count_caps: Vec<RiskCap>` in `crates/risk/src/engine/config.rs`, and the Python configuration takes
them: a cap is a `RiskCap` naming a metric and a scope from `nautilus_trader.risk`, a limit, and a
window in nanoseconds, which an `Active` cap omits. That is the tool for a hard cap on how many
messages a strategy may send in a window; `max_order_submit_rate` and `max_order_modify_rate` remain
the coarser rate limits beside it.

Set fees explicitly. A backtest venue refuses to be added without a `fee_model`; pass an explicit
zero-fee model if you truly want zero, rather than leaving it out. Fees are the term that turns a
small positive edge into a loss.

## 6. What the engine does and does not enforce

| Question                                     | Does the engine enforce it?                      |
| -------------------------------------------- | ------------------------------------------------ |
| Delta instrument matches the book            | Yes, `apply_delta` returns `InstrumentMismatch`. |
| Best bid does not exceed best ask            | Only when you call `book.check_integrity()`.     |
| No more than one order per level in `L2_MBP` | Only when you call `book.check_integrity()`.     |
| `ts_last` and `sequence` never regress       | Yes.                                             |
| A missed delta is detected                   | It is warned about; the strategy is not stopped. |
| A stale book is detected                     | No.                                              |
| A position limit is respected                | No; that is your strategy's job.                 |
| Supported instrument ID and price precision  | Yes, on `apply_delta` and order submission.      |
| Zero-size levels are removed                 | Only if your loader emits `DELETE`.              |
| Fees are charged                             | Only if you pass a non-zero `fee_model`.         |

The last row of each half is the one beginners get wrong. The engine gives you the mechanism; it
does not infer your intent.

## 7. A failure checklist

Before you trust any imbalance run:

1. Did the loader convert microseconds to nanoseconds and zero size to `DELETE`?
2. Did the loader emit `CLEAR` before every snapshot after the first?
3. Are prices and sizes quantised to the instrument's precision?
4. Was the venue created with `book_type=BookType.L2_MBP`?
5. Did `book.check_integrity()` pass on the rebuilt book?
6. Were there stale-update or sequence-gap warnings in the log?
7. Are fees non-zero and realistic for the venue?
8. Is there a position limit in the strategy, and a risk engine limit behind it?
9. Was the threshold fitted on one venue and tested on another?
10. Are there enough fills to mean anything, and a no-signal baseline to compare against?

If any answer is no, the numbers from lecture 06 are not yet evidence.

## Next

[08](08-exercises.md) gives you six exercises, including one that breaks the book on purpose.
