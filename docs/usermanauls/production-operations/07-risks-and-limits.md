# 07 - Risks and limits

This lecture is about the two hardest moments in a live run: coming back after the process stopped,
and repairing the books after the connection broke. It also covers position sizing and loss limits,
and it ends with an honest list of why a live run never matches a backtest exactly.

## Part 1: restart recovery and the state contract

### The engine's own state and your state

Two different things survive a restart.

- The engine's **cache** holds orders, positions, accounts, and instruments. With a database
  backing, the node can restore it before reconciliation runs
  (`docs/how_to/configure_live_trading.md`).
- The **event store** holds the ordered record of the messages that changed state. It is the durable
  authority; the cache is a write-through projection (`docs/concepts/event_sourcing.md`).

Your strategy's own variables are neither. They live in the Python process and disappear when it
exits, unless you hand them over explicitly.

### The explicit `on_save` and `on_load` contract

A strategy implements two hooks. The type annotations from `docs/concepts/strategies.md` are:

```python
def on_save(self) -> dict[str, bytes]:
def on_load(self, state: dict[str, bytes]) -> None:
```

The contract has four parts.

1. `on_save` returns a dictionary whose keys are strings and whose values are **bytes**. You choose
   the keys. The engine stores the bytes; it does not interpret them.
2. `on_load` receives that same dictionary back, at startup, before the strategy trades.
3. The hooks fire only when a cache database backing is attached. Without one, `load_state` and
   `save_state` have no effect and the kernel says so in a warning.
4. Because you choose the encoding, you also own the compatibility. A changed key or a changed
   encoding is a state format change.

The third point is the one beginners miss. Prove it to yourself with this script, saved outside the
repository. It runs a backtest with both options enabled and no cache backing, then exercises the
contract directly.

```python
"""The on_save/on_load state contract, and what the engine does without a backing."""

from __future__ import annotations

from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LogLevel
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy

SIM = Venue("SIM")
USD = Currency.from_str("USD")
AUDUSD_SIM = TestInstrumentProvider.audusd_sim()


class CounterConfig(StrategyConfig):
    def __init__(self, instrument_id, **_kwargs) -> None:
        super().__init__()
        self.instrument_id = instrument_id


class Counter(Strategy):
    def __init__(self, config: CounterConfig) -> None:
        super().__init__(config)
        self.quote_count = 0
        self.high = Decimal("0")

    def on_start(self) -> None:
        self.subscribe_quotes(self.config.instrument_id)

    def on_quote(self, tick: QuoteTick) -> None:
        self.quote_count += 1
        if tick.bid_price.as_decimal() > self.high:
            self.high = tick.bid_price.as_decimal()

    def on_save(self) -> dict:
        state = {
            "quote_count": str(self.quote_count).encode(),
            "high": str(self.high).encode(),
        }
        print(f"on_save    -> keys={sorted(state)}")
        return state

    def on_load(self, state: dict) -> None:
        self.quote_count = int(state["quote_count"].decode())
        self.high = Decimal(state["high"].decode())
        print(f"on_load    -> quote_count={self.quote_count} high={self.high}")

    def on_stop(self) -> None:
        print(f"on_stop    -> quote_count={self.quote_count} high={self.high}")


def build(save: bool, load: bool) -> BacktestEngine:
    engine = BacktestEngine(
        BacktestEngineConfig(
            trader_id=TraderId.from_str("OPS-001"),
            logging=LoggerConfig(stdout_level=LogLevel.WARNING, print_config=False),
            save_state=save,
            load_state=load,
        ),
    )
    engine.add_venue(
        venue=SIM,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(1_000_000, USD)],
        fee_model=MakerTakerFeeModel(maker_rate=Decimal("0"), taker_rate=Decimal("0")),
    )
    engine.add_instrument(AUDUSD_SIM)
    return engine


print("RUN 1: cache has no database backing, save_state=True, load_state=True")
engine = build(save=True, load=True)
engine.add_data(TestDataProvider.audusd_quotes(count=5))
engine.add_strategy(Counter(CounterConfig(instrument_id=AUDUSD_SIM.id)))
engine.run()
engine.reset()
engine.dispose()

print()
print("CONTRACT ROUND TRIP (direct calls)")
source = Counter(CounterConfig(instrument_id=AUDUSD_SIM.id))
source.quote_count = 7
source.high = Decimal("0.71005")
state = source.on_save()
print("value types:", {key: type(value).__name__ for key, value in state.items()})
restored = Counter(CounterConfig(instrument_id=AUDUSD_SIM.id))
restored.on_load(state)
print("restored   :", restored.quote_count, restored.high)
```

Run it:

```bash
cd <repo-root>/python
uv run --no-sync python <temp-dir>/po_manual/state_demo.py
```

Observed output:

```text
RUN 1: cache has no database backing, save_state=True, load_state=True
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_system::kernel: Cache has no database backing, load_state=true and save_state=true will have no effect
on_stop    -> quote_count=5 high=0.71007

CONTRACT ROUND TRIP (direct calls)
on_save    -> keys=['high', 'quote_count']
value types: {'quote_count': 'bytes', 'high': 'bytes'}
on_load    -> quote_count=7 high=0.71005
restored   : 7 0.71005
```

Read the three facts off that output.

1. The kernel warned that with no cache backing both options have no effect. Note that `on_save` was
   never printed in RUN 1: the hook did not fire. A strategy that assumes its state is being saved
   is wrong unless a backing exists.
2. The round trip works: `on_save` produced a dictionary of `bytes`, and `on_load` reconstructed the
   two values exactly.
3. The keys are your own choice. Rename `quote_count` and an old state file no longer loads.

### Wiring a real restart

A live node that should survive a restart is built with a cache backing and both state options, for
example:

```python
node = (
    LiveNode.builder("LiveNode", TraderId.from_str("TRADER-001"), Environment.LIVE)
    .with_cache_database_factory(RedisCacheConfig(host="localhost", port=6379))
    .with_exec_engine_config(
        LiveExecutionEngineConfig(
            snapshot_orders=True,
            snapshot_positions=True,
        ),
    )
    .with_load_state(True)
    .with_save_state(True)
    .build()
)
```

`RedisCacheConfig` and `PostgresCacheConfig` are the two Python-visible backings
(`nautilus_trader.infrastructure`). Any other object raises `NotImplementedError` from
`with_cache_database_factory`, and a failed database connection fails `run()`. Database-backed nodes
must use `run()`: `run_async()` rejects a backing that would block the host event loop. Set
`CacheConfig.flush_on_start = true` to clear the backing instead of restoring it.

This sample is illustrative. It needs a running Redis or Postgres server, so this manual does not
run it.

### What happens after a hard kill

A process that dies without finishing its lifecycle leaves its run file marked `Running`. On the
next boot, recovery scans each such predecessor and seals it from its durable tail
(`docs/concepts/event_sourcing.md`):

| Durable tail                              | Sealed status      | Eligible as a parent |
| ----------------------------------------- | ------------------ | -------------------- |
| No entries                                | `CrashedRecovered` | Yes                  |
| Clean, without `RunEnded`                 | `CrashedRecovered` | Yes                  |
| Clean, ending in `RunEnded`               | `Ended`            | No                   |
| Hash mismatch, gap, or structural failure | `Quarantined`      | No                   |

Only `CrashedRecovered` predecessors become a parent run, so a crash is recoverable but a corrupted
run is not chained. Kernel-managed replay uses `EventStoreConfig::replay_from_run_id` and **requires
`load_state=true`**; without it the kernel logs an error and returns without restoring the cache or
opening a child run. Quarantined runs are rejected. Replay is state only: it does not publish to the
live bus, run strategy code, query venues, run reconciliation, or re-arm clocks.

## Part 2: reconciliation after a disconnect

A disconnect leaves an order's outcome unknown. The transport is gone, the acknowledgement never
arrived, and the strategy cannot tell a rejection from a fill. Guessing in either direction creates
either a duplicate order or an invisible one.

The engine does not guess. It reconciles.

### Startup

Unless reconciliation is disabled, a live node runs startup reconciliation for each execution client
before trader components start. `reconciliation_lookback_mins` controls how far back it requests
history; leaving it unset uses the adapter's documented default. Reconciliation compares local order
and position state with authoritative venue reports and emits the missing events.

The invariants it preserves, from `docs/concepts/execution/reconciliation.md`:

1. Authoritative reports recover the exact order status and filled quantity.
2. An explicit open or flat position report is the quantity target. A missing report is not flat.
3. A bounded historical fill with no in-scope position report updates order state only, without
   changing positions or economics.
4. Reconciled positions match venue reports within the applicable quantity tolerance.
5. Reported entry averages match within a relative tolerance of 0.0001 (0.01%) before startup
   proceeds.
6. Synthetic identifiers are deterministic functions of the logical event, so a replay deduplicates
   them across restarts.

### The scenarios that matter operationally

| Scenario                           | What the engine does                                                                |
| ---------------------------------- | ----------------------------------------------------------------------------------- |
| Order state discrepancy            | Updates local state to match the venue and emits the missing events.                |
| Missed fills                       | Generates the missing `OrderFilled` and applies its economics.                      |
| External orders                    | Creates unclaimed orders with strategy ID `EXTERNAL` and tag `VENUE`.               |
| Partially filled then canceled     | Sets state to `CANCELED` and preserves the fill history.                            |
| Different fill data                | Preserves cached data and logs the discrepancy.                                     |
| Long position quantity mismatch    | Generates a BUY LIMIT for the difference when `generate_missing_orders` is enabled. |
| Short position quantity mismatch   | Generates a SELL LIMIT for the difference when generation is enabled.               |
| Position reduction                 | Generates an opposite-side LIMIT for the difference.                                |
| Position side flip                 | Generates a LIMIT to close the internal side and open the external side.            |
| In-flight submit timeout           | Resolves to `REJECTED` with `INFLIGHT_TIMEOUT` after the retries are exhausted.     |
| In-flight cancel or update timeout | Resolves to `CANCELED` through reconciliation.                                      |

Orders the engine generates to align a position discrepancy are tagged `RECONCILIATION`. To detect
unclaimed external orders in your own code, check `order.strategy_id.value == "EXTERNAL"`.

`submission_recovery_policy` defaults to `ResolveLocally`. Selecting `RetainUnresolved` enables
submission identity tracking and publishes a `SubmissionRecoveryExhausted` diagnostic when an
unacknowledged submission reaches a recovery limit. It is not an order event and does not establish a
venue outcome.

The sample file `sample_data/venue_fills_report.csv` is a small picture of the evidence this path
consumes: two buys and a sell that net to 200,000 units. If your cache said 150,000, invariant 2 would
drive a reconciliation order for the missing 50,000.

## Part 3: position sizing and loss limits

### Size from risk, not from hope

A position size should come from two numbers you choose in advance:

1. The most money you are willing to lose on this trade if your stop is hit.
2. The distance from your entry to your stop, in price units.

The size is then `risk_money / stop_distance`, rounded down to the instrument's size precision. Test
that arithmetic before the trade, not after.

`nautilus_trader.risk` exposes `FixedRiskSizer` and `PositionSizer` for this. Its `calculate` method
takes an entry price, a stop-loss price, the account equity, and a risk amount, and returns a
`Quantity`.

### What the engine enforces, and what it does not

The engine is a mechanical guard, not a risk manager. Know the difference.

| The engine does enforce                                       | The engine does not enforce                         |
| ------------------------------------------------------------- | --------------------------------------------------- |
| Price and trigger-price precision                             | Whether a price is sensible.                        |
| Positive prices, except where the instrument allows negatives | A maximum loss per day.                             |
| Quantity precision and instrument minimum and maximum bounds  | A maximum position size for you.                    |
| GTD orders have not already expired                           | A maximum drawdown.                                 |
| `reduce_only` orders do not increase the referenced position  | That you set a stop at all.                         |
| `max_notional_per_order` and instrument notional bounds       | Daily loss limits.                                  |
| Cash balance impact for non-margin accounts                   | That your account type matches your venue contract. |
| Submit and modify rate limits                                 | Anything about strategy logic correctness.          |
| Trading-state restrictions (`ACTIVE`, `HALTED`, `REDUCING`)   | Recovery from a worse position than you planned.    |
| Count caps, when you configure them                           | A default cap value; none is shipped.               |

If a submit-time check fails, the engine emits `OrderDenied`. If a modify-time check fails, it emits
`OrderModifyRejected`. The engine never refuses a cancellation.

The drawdown threshold used by the notification router is declared on the router with
`set_drawdown_threshold`. Without one, position events raise nothing. That is a separate control from
the risk engine: set it if you want a `DrawdownThresholdBreach` alert.

## Part 4: why a live run never matches a backtest exactly

This is not a defect to be engineered away. It is a list of things a simulation cannot know
(`docs/concepts/live.md`).

| Difference        | Why it exists                                                                   |
| ----------------- | ------------------------------------------------------------------------------- |
| Venue             | Venue rules and adapter capabilities decide which order types and events exist. |
| Transport         | A network failure can leave a command's outcome unknown.                        |
| Timing            | Independent inputs interleave; there is no single global FIFO order.            |
| Persistence       | Cache writes and event capture are not gated on durable commit.                 |
| External activity | Venue reports include orders created outside your node.                         |
| Reconciliation    | Startup and runtime checks change state that a backtest never changes.          |

Two more, from the runner itself:

- The seven internal channels are unbounded and dispatched in a fixed priority, with execution
  events before execution commands and data events after both. A slow handler delays every channel.
- The runner does not throttle a feed, halt trading, or shut down when a queue threshold is crossed.
  Your application must decide.

The practical rule: use the backtest to learn the shape of the strategy, and the sandbox to learn the
shape of the operations. Expect the live PnL to differ from both, and instrument the difference
rather than hoping it away.

Continue to [08 - Exercises](08-exercises.md).
