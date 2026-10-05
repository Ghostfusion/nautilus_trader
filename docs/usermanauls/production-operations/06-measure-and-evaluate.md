# 06 - Measure and evaluate

A live strategy produces two kinds of measurement: reports that describe what happened to your
orders and money, and alerts that tell a human when something needs attention. This lecture covers
both, and the one rule that must never be broken: an alert body must not carry a credential.

## Part 1: the reports

Every engine can produce tables after a run. In a live node you read the same information from the
cache and the portfolio instead, but the columns and the meaning are the same.

Save this script outside the repository.

```python
"""The operational reports a live run produces, and how to read the numbers."""

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
from nautilus_trader.model import OrderSide
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy

SIM = Venue("SIM")
USD = Currency.from_str("USD")
AUDUSD_SIM = TestInstrumentProvider.audusd_sim()


class OneTradeConfig(StrategyConfig):
    def __init__(self, instrument_id, **_kwargs) -> None:
        super().__init__()
        self.instrument_id = instrument_id


class OneTrade(Strategy):
    def __init__(self, config: OneTradeConfig) -> None:
        super().__init__(config)
        self.done = False

    def on_start(self) -> None:
        self.subscribe_quotes(self.config.instrument_id)

    def on_quote(self, _tick: QuoteTick) -> None:
        if self.done:
            return
        self.done = True
        instrument = self.cache.instrument(self.config.instrument_id)
        self.submit_order(
            self.order_factory.market(
                self.config.instrument_id,
                OrderSide.BUY,
                instrument.make_qty(Decimal("100000")),
            ),
        )


engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("OPS-001"),
        logging=LoggerConfig(stdout_level=LogLevel.ERROR, print_config=False),
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
engine.add_data(TestDataProvider.audusd_quotes(count=20))
engine.add_strategy(OneTrade(OneTradeConfig(instrument_id=AUDUSD_SIM.id)))
engine.run()

print("--- POSITIONS REPORT ---")
positions = engine.generate_positions_report()
print("columns:", list(positions.columns))
pos_cols = [
    "instrument_id",
    "side",
    "quantity",
    "avg_px_open",
    "realized_pnl",
    "commissions",
    "ts_opened",
    "duration_ns",
]
print(positions[pos_cols].to_string(index=False))
print()
print("--- FILLS REPORT ---")
fills = engine.generate_order_fills_report()
fill_cols = [
    "instrument_id",
    "side",
    "type",
    "quantity",
    "status",
    "filled_qty",
    "avg_px",
    "liquidity_side",
    "commissions",
    "last_trade_id",
]
print(fills[fill_cols].to_string(index=False))
print()
print("--- ACCOUNT REPORT ---")
account = engine.generate_account_report(SIM)
print(account.to_string(index=False))
print()
print("--- ORDER FILL COUNT ---")
print("closed orders:", engine.cache.orders_closed_count())
print("open orders  :", engine.cache.orders_open_count())

engine.reset()
engine.dispose()
```

Run it:

```bash
cd <repo-root>/python
uv run --no-sync python <temp-dir>/po_manual/reports_demo.py
```

Observed output:

```text
--- POSITIONS REPORT ---
columns: ['type', 'events', 'adjustments', 'trader_id', 'strategy_id', 'instrument_id', 'account_id', 'opening_order_id', 'closing_order_id', 'entry', 'side', 'quantity', 'peak_qty', 'price_precision', 'size_precision', 'multiplier', 'is_inverse', 'ts_init', 'ts_opened', 'ts_last', 'ts_closed', 'duration_ns', 'avg_px_open', 'avg_px_close', 'realized_return', 'realized_pnl', 'venue_order_ids', 'trade_ids', 'buy_qty', 'sell_qty', 'commissions', 'is_snapshot']
instrument_id side quantity  avg_px_open realized_pnl commissions                 ts_opened  duration_ns
  AUD/USD.SIM LONG   100000       0.7101     0.00 USD  [0.00 USD] 2019-01-01 23:00:00+00:00            0

--- FILLS REPORT ---
instrument_id side   type quantity status filled_qty  avg_px liquidity_side commissions          last_trade_id
  AUD/USD.SIM  BUY MARKET   100000 FILLED     100000 0.71010          TAKER  [0.00 USD] T-65ff55bb95075e97-001

--- ACCOUNT REPORT ---
     total locked       free currency account_id account_type                                                                                                                    margins  reported info base_currency
1000000.00   0.00 1000000.00      USD    SIM-001       MARGIN                                                                                                                         []      True   {}           USD
1000000.00 213.03  999786.97      USD    SIM-001       MARGIN [{'type': 'MarginBalance', 'initial': '0.00', 'maintenance': '213.03', 'currency': 'USD', 'instrument_id': 'AUD/USD.SIM'}]     False   {}           USD

--- ORDER FILL COUNT ---
closed orders: 1
open orders  : 0
```

### What each column means

| Column           | Plain meaning                                                                |
| ---------------- | ---------------------------------------------------------------------------- |
| `side`           | Which way the position points. `LONG` means you own the instrument.          |
| `quantity`       | How many units the position holds.                                           |
| `avg_px_open`    | The average price you paid to open it.                                       |
| `realized_pnl`   | Money actually banked. It stays zero while the position is open.             |
| `commissions`    | Fees charged. Listed per currency.                                           |
| `ts_opened`      | When the position opened, in the engine's clock.                             |
| `duration_ns`    | How long it has been open, in nanoseconds.                                   |
| `status`         | One of `FILLED`, `PARTIALLY_FILLED`, `DENIED`, `CANCELED`, `EXPIRED`.        |
| `filled_qty`     | How much of the order actually traded.                                       |
| `avg_px`         | The average price of the fills on that order.                                |
| `liquidity_side` | `TAKER` if the order removed liquidity, `MAKER` if it added it.              |
| `total`          | The account's whole balance in that currency.                                |
| `locked`         | Money reserved against open orders and positions.                            |
| `free`           | Money available for new orders. It always equals `total - locked`.           |
| `reported`       | `True` for a balance the venue reported, `False` for one the engine derived. |

The balance invariant `total == locked + free` holds in every account row
(`docs/concepts/accounting.md`). In the output above the second row locks 213.03 USD as maintenance
margin for the open position, so free falls to 999,786.97.

### What good and bad look like

These are rules of thumb, not the engine's opinions. The engine reports the number; you decide.

| Number                       | Unhealthy                 | Healthy                        | Why                                                      |
| ---------------------------- | ------------------------- | ------------------------------ | -------------------------------------------------------- |
| `filled_qty` vs `quantity`   | Often zero                | Usually the full quantity      | A strategy whose orders mostly do not fill is mispriced. |
| `liquidity_side` mix         | All `TAKER`               | A planned mix                  | Taker fills pay the spread and usually a higher fee.     |
| `commissions`                | Rising faster than profit | Small relative to gross profit | Fees are the one cost you can measure exactly.           |
| `realized_pnl`               | Negative over many trades | Positive after costs           | One lucky trade is not evidence.                         |
| `locked` relative to `total` | Above about half          | Low                            | A high locked share means little room for new orders.    |

### Three common beginner misreadings

1. **Reading a report row as a trade.** The orders report includes denied orders, with `status
   DENIED` and `filled_qty 0`. A row count is a count of orders you attempted, not of trades you
   made.
2. **Reading `NaN` as zero.** The statistics block prints `NaN` when the sample is too small or a
   denominator is zero. `NaN` means "cannot be computed", not "no risk". In the run below most
   statistics are `NaN` because the position never closed.
3. **Reading `total` as spendable money.** Only `free` is spendable. The gap is `locked`, and it is
   money the engine has promised to open orders and positions.

## Part 2: the statistics block

With logging at `INFO`, the backtest engine prints a statistics block when a run finishes. These are
the real lines from the one-trade run above, with the `INFO` prefix removed:

```text
 PnL Statistics (USD)
Avg Winner:                     NaN
Max Winner:                     NaN
Min Winner:                     NaN
PnL (total):                    0.00
PnL% (total):                   0.00
Win Rate:                       0.00
 Returns Statistics
Average Win (Return):           0.00
Profit Factor:                  NaN
Returns Kurtosis:               NaN
Returns Skewness:               NaN
Returns Volatility (252 days):  NaN
Risk Return Ratio:              NaN
Sharpe Ratio (252 days):        NaN
Sortino Ratio (252 days):       NaN
Tail Ratio:                     NaN
 General Statistics
Long Ratio:                     1
```

| Statistic                       | Plain meaning                                 | Rough reading                                      |
| ------------------------------- | --------------------------------------------- | -------------------------------------------------- |
| `Win Rate`                      | Share of closed trades that made money.       | 0.4 to 0.6 is common; it says nothing alone.       |
| `PnL (total)`                   | Money made or lost over the run.              | Must be positive after costs.                      |
| `Profit Factor`                 | Gross profit divided by gross loss.           | Above 1.0 breaks even; 1.5 or more is comfortable. |
| `Sharpe Ratio (252 days)`       | Return per unit of wobble, annualized.        | Above 1.0 is good; below 0 is bad.                 |
| `Sortino Ratio (252 days)`      | Like Sharpe, but only counts downward wobble. | Above 1.5 is good.                                 |
| `Returns Volatility (252 days)` | How much the return swings.                   | Lower is calmer, not automatically better.         |
| `Max Drawdown`                  | The worst fall from a peak to a later low.    | Compare it against your pain limit.                |
| `Long Ratio`                    | Share of positions that were long.            | Near 1 or 0 means the strategy is one-sided.       |

## Part 3: alerts

### The subsystem is Rust only

The notification subsystem lives in `crates/common/src/notification/` and has no Python binding. You
cannot register a sink or read the counters from Python. It is driven by a Rust-native node. State
that plainly before relying on it.

### The event set is closed

A `NotificationEvent` carries exactly one of nine classes. A sink cannot be registered for a class
that does not exist.

| Class                     | Raised when                                        |
| ------------------------- | -------------------------------------------------- |
| `RiskLimitBreach`         | The risk engine halts trading.                     |
| `OrderRejection`          | An order is rejected.                              |
| `StrategyStop`            | A strategy stops.                                  |
| `ExecutionCompletion`     | An order fill completes an execution.              |
| `DrawdownThresholdBreach` | A position's realized return breaches a threshold. |
| `DataFeedDisconnect`      | A data feed disconnects.                           |
| `BrokerDisconnect`        | A broker or venue disconnects.                     |
| `BacktestCompletion`      | A backtest run completes.                          |
| `OptimizationCompletion`  | An optimization run completes.                     |

Each event carries the identity a sink needs (strategy, instrument, account, run where applicable),
a severity of `Info`, `Warning`, or `Critical`, and a message.

### The router, the queue, and the drop policy

`NotificationRouter::subscribe` subscribes to existing bus points and translates what they carry:
order events, position events, and the risk engine's trading state. Classes with no existing bus
event are raised by publishing a `NotificationEvent` to `NOTIFICATION_TOPIC` or by calling
`NotificationRouter::publish` directly.

Publishing never blocks and never runs transport I/O on the caller's thread. The message goes to the
sink's bounded queue and the call returns. Each sink owns a worker thread fed by that queue, with a
declared bound and a coalescing interval, defaulting to 1024 messages and 250 milliseconds.

- A burst arriving within one interval is coalesced into a single send, which bounds the send rate a
  burst can produce.
- A full queue drops the message and increments a counter, readable through
  `NotificationRouter::drop_count`. The caller is never blocked and the queue never grows.
- A delivery failure is reported through the ordinary logging path and never re-enters the router.
- `NotificationRouter::sent_count` reports the deliveries a sink achieved.

Those two counters are operational signals in their own right. `sent_count` rising proves the path
works. `drop_count` rising proves the path is saturated and alerts are being lost, which is a
different failure from "nothing happened".

### Prove it with the crate's tests

The notification subsystem is Rust only, so demonstrate it with the crate's own test target. The lib
test target for `nautilus-common` does not build on Windows in this checkout because of unused
imports treated as errors, so run the integration test binary directly.

```bash
export PATH="$HOME/.cargo/bin;$PATH"
export CARGO_TARGET_DIR='<repo-root>/target'
cd <repo-root>
cargo nextest run --locked -p nautilus-common --features python --test notifications
```

Observed output. The nextest separator rule is rendered here with ASCII hyphens.

```text
        PASS [   0.028s] (1/5) nautilus-common::notifications publishing_without_sinks_is_a_no_op
        PASS [   0.036s] (2/5) nautilus-common::notifications email_sink_delivers_over_smtp
        PASS [   0.054s] (3/5) nautilus-common::notifications router_queue_drops_and_counts_under_saturation
        PASS [   0.058s] (4/5) nautilus-common::notifications webhook_sink_posts_json_over_http
        PASS [   1.272s] (5/5) nautilus-common::notifications router_coalesces_burst_into_single_send
------------
     Summary [   1.273s] 5 tests run: 5 passed, 0 skipped
```

`router_coalesces_burst_into_single_send` publishes 100 events inside one 500 millisecond interval
and asserts that the sink sends once, with `sent_count` 1 and `drop_count` 0.
`router_queue_drops_and_counts_under_saturation` uses a queue bound of one and a sink that blocks;
it publishes 500 events, asserts the publishing loop returns in under a second, asserts
`drop_count` is at least 400, and asserts `sent_count` is 0 while the sink is blocked. That is the
drop policy: the caller is never blocked, and the loss is counted.

### The transports are plaintext

Two sinks ship: `EmailSink` over SMTP and `WebhookSink` over an HTTP POST of a JSON body. The
default transports, `SmtpTransport` and `HttpTransport`, speak plaintext over a blocking TCP stream
and do not negotiate TLS. `HttpTransport` accepts `http://` URLs only, and `SmtpTransport` performs
no authentication.

Therefore: **never put a credential, an API key, a token, or an account number in a notification
body, subject, or URL.** A notification body travels unencrypted on the wire with the default
transports. If a sink needs a secret to authenticate, obtain it at the transport, and implement a
custom `NotificationTransport` that negotiates TLS.

Sinks are built over an injectable `NotificationTransport`, so a deployment that needs TLS supplies
its own implementation rather than editing the built-in ones.

## Part 4: two more operational signals

### Queue pressure

The live runner's internal channels are unbounded. They do not apply backpressure, coalesce
messages, or impose a maximum depth, so sustained input above dispatch capacity increases queue
depth, latency, and memory use, and the runner does not automatically throttle a feed or halt
trading (`docs/concepts/live.md`).

The queue monitor is disabled by default. When `LiveNodeConfig.queue_monitor` is set, the node
converts queue samples into typed state transitions and publishes a `QueueStateChanged` value on
`events.system.QueueStateChanged.<channel>`. Each channel tracks `Backlogged` (point-in-time depth)
and `Slow` (mean dispatch time) independently. Each clear threshold must be lower than its trigger
threshold. Actors subscribe with `subscribe_queue_state` and receive events through
`on_queue_state`.

Treat these as operational signals, not service-time guarantees. The application decides how to
alert, reduce input, halt new exposure, or stop the node.

### Transport state

An adapter that opts into socket state reporting causes the node to publish `SocketStateChanged` on
`events.system.SocketStateChanged.<client_id>.<endpoint>`. A lost transport publishes
`Disconnected`; a recovered transport publishes `Connected`. `Connected` means transport
availability, not that authentication and subscription replay finished.

A disconnect is not an order outcome by itself. It does not reject, cancel, or resolve an in-flight
command; only stream updates, queries, or reconciliation do that. That is the bridge to lecture 07.

Continue to [07 - Risks and limits](07-risks-and-limits.md).
