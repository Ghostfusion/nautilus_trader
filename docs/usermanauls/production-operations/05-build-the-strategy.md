# 05 - Build the operational harness

Lecture 03 configured the risk engine from Python. This lecture goes deeper on the strongest
count-based limits, the count caps, and then runs the sandbox client. It does two things.

1. It reads the Rust configuration behind the caps, shows how to set one from Python, and proves the
   behavior by running the risk crate's own tests.
2. It runs the sandbox execution client end to end against a live public data feed, and states what
   the sandbox does not simulate.

## Part 1: the count caps

### What Python can set

`nautilus_trader.risk.RiskEngineConfig` accepts `bypass`, `max_order_submit_rate`,
`max_order_modify_rate`, `max_notional_per_order`, `count_caps`, `full_position_exit_venues`, and
`debug`. That is the whole constructor, verified in `crates/risk/src/python/config.rs`. Rates are
written `limit/HH:MM:SS`, and a cap is a `RiskCap` naming a metric and a scope from
`nautilus_trader.risk`, a limit, and a window in nanoseconds.

### The cap field in the Rust configuration

Open `crates/risk/src/engine/config.rs`. The struct has one more field:

```rust
    /// Count caps, each a predicate over a scope, a metric and a window.
    ///
    /// A cap refuses the actions that increase exposure when the count it observes reaches its
    /// limit, and it never refuses a cancellation. Caps are evaluated in configuration order, so
    /// the first reached cap names the refusal.
    #[builder(default)]
    pub count_caps: Vec<RiskCap>,
```

The Python constructor argument maps to this field: `RiskEngineConfig(count_caps=[RiskCap(...)])`
sets the same `Vec<RiskCap>`, and a serialized live configuration carries it as
`METRIC/SCOPE/LIMIT[/WINDOW_NS]` strings. The dimension names below are the same on every surface.

### The four dimensions of a cap

A `RiskCap` (defined in `crates/risk/src/engine/cap.rs`) is a predicate with four parts.

| Dimension | What it selects                              | Values                                                                        |
| --------- | -------------------------------------------- | ----------------------------------------------------------------------------- |
| Scope     | Which orders are counted                     | `GLOBAL`, `STRATEGY`, `ACCOUNT`, `INSTRUMENT`, `VENUE`, `STRATEGY_INSTRUMENT` |
| Metric    | What is counted                              | `ACTIVE`, `SUBMIT`, `MODIFY`, `CANCEL`, `FILL`, `REPEATED_REQUEST`            |
| Limit     | How many are allowed                         | A positive integer                                                            |
| Window    | The rolling duration the count is taken over | A duration in nanoseconds, or none for `ACTIVE`                               |

The scope and metric names above are the serialized forms of the `RiskCapScope` and `RiskCapMetric`
enums in `crates/model/src/risk.rs`.

Three rules from `docs/concepts/execution/index.md` decide what a cap does:

1. Counters are keyed per rule and per concrete scope, so two rules over one scope with different
   windows are two independent rules and neither redefines the other's window.
2. A window is rolling and half-open at its start. There is no reset boundary, so capacity returns
   as occurrences age out rather than at a session change.
3. A cancel or fill cap gates **submits**, not the cancellation or the fill itself. A cancellation is
   never refused by a cap, because refusing to cancel is the behavior a risk limit must not have.

An `ACTIVE` cap counts the open order set as it stands and takes no window. Every other metric
requires a window. Configuration validation rejects a zero limit, a missing window on a windowed
metric, a window on an `ACTIVE` cap, and a duplicate rule.

The `REPEATED_REQUEST` metric counts by request shape: instrument, side, order type, quantity, and
price, excluding the client order id. A strategy that repeats the same request necessarily issues a
new client order id each time, so a counter keyed on that id would never observe the repeat. The
identity type is `RiskRequestKey` in `crates/model/src/risk.rs`.

A refusal is reported as an `OrderDenied` whose reason mentions the observed count, the limit, the
scope, and the window. `crates/risk/src/engine/cap.rs` also retains the most recent 64 refusals as
structured decision records, so a refusal is observable as data and not only as a log line.

### No cap is shipped by default

`count_caps` defaults to an empty list. An engine whose configuration declares no cap denies nothing
on this path. The concept guide is explicit that no default value or window duration is shipped,
because a value that is too low denies legitimate strategies and a value that is too high is not a
limit.

The one measured configuration in the repository is the input the send-path benchmark used. It is a
benchmark input, **not** a recommendation:

| Scope      | Metric | Limit  |
| ---------- | ------ | ------ |
| Global     | Submit | 20,000 |
| Global     | Cancel | 10,000 |
| Global     | Fill   | 10,000 |
| Instrument | Submit | 2,000  |
| Global     | Active | 50     |

Derive your own values from your venue's message limits and your observed order flow, and record the
window you chose rather than inheriting a duration.

### Prove it by running the crate's tests

The caps live in a Rust-only subsystem, so demonstrate them with the crate's own test target. Run:

```bash
export PATH="C:/Users/vince/.cargo/bin;$PATH"
export CARGO_TARGET_DIR='D:/Users/vince/PycharmProjects/nautilus_trader/target'
cd D:/Users/vince/PycharmProjects/nautilus_trader
cargo nextest run --locked -p nautilus-risk --features python -E 'test(cap::tests)'
```

Observed output, first build took about 25 minutes, the test run itself 0.4 seconds. The nextest
separator rule is rendered here with ASCII hyphens.

```text
        PASS [   0.032s] ( 1/36) nautilus-risk engine::cap::tests::test_an_account_cap_does_not_apply_without_an_account
        PASS [   0.035s] ( 2/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_03
        PASS [   0.126s] ( 3/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_02
        PASS [   0.127s] ( 4/36) nautilus-risk engine::cap::tests::test_evaluate_ignores_a_cap_that_does_not_gate_the_action
        PASS [   0.163s] ( 5/36) nautilus-risk engine::cap::tests::test_a_voided_fill_releases_its_occurrence
        PASS [   0.163s] ( 6/36) nautilus-risk engine::cap::tests::test_occurrences_expire_at_the_window_boundary
        PASS [   0.163s] ( 7/36) nautilus-risk engine::cap::tests::test_counters_are_per_rule_and_per_scope
        PASS [   0.182s] ( 8/36) nautilus-risk engine::cap::tests::test_evaluate_refuses_at_the_limit_and_records_the_rule
        PASS [   0.182s] ( 9/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_04
        PASS [   0.182s] (10/36) nautilus-risk engine::cap::tests::test_evaluate_skips_an_account_cap_without_an_account
        PASS [   0.175s] (11/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_06
        PASS [   0.209s] (12/36) nautilus-risk engine::cap::tests::test_a_repeated_request_cap_counts_per_request_identity
        PASS [   0.209s] (13/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_01
        PASS [   0.094s] (14/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_07
        PASS [   0.196s] (15/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_05
        PASS [   0.103s] (16/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_08
        PASS [   0.095s] (17/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_09
        PASS [   0.095s] (18/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_10
        PASS [   0.076s] (19/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_12
        PASS [   0.103s] (20/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_11
        PASS [   0.089s] (21/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_13
        PASS [   0.088s] (22/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_14
        PASS [   0.089s] (23/36) nautilus-risk engine::cap::tests::test_only_increasing_actions_are_gated::case_15
        PASS [   0.089s] (24/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_02
        PASS [   0.089s] (25/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_01
        PASS [   0.101s] (26/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_03
        PASS [   0.094s] (27/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_05
        PASS [   0.097s] (28/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_04
        PASS [   0.083s] (29/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_06
        PASS [   0.083s] (30/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_08
        PASS [   0.086s] (31/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_07
        PASS [   0.091s] (32/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_09
        PASS [   0.087s] (33/36) nautilus-risk engine::cap::tests::test_only_the_counted_action_is_recorded::case_10
        PASS [   0.111s] (34/36) nautilus-risk engine::cap::tests::test_request_key_excludes_the_client_order_id
        PASS [   0.082s] (35/36) nautilus-risk engine::cap::tests::test_subject_keys_cover_every_scope
        PASS [   0.087s] (36/36) nautilus-risk engine::cap::tests::test_scope_key_round_trips_its_scope
------------
     Summary [   0.392s] 36 tests run: 36 passed, 1258 skipped
```

### What each test proves

| Test                                                        | The operational fact it pins down                               |
| ----------------------------------------------------------- | --------------------------------------------------------------- |
| `test_evaluate_refuses_at_the_limit_and_records_the_rule`   | A cap refuses at the limit and the refusal names its rule.      |
| `test_evaluate_ignores_a_cap_that_does_not_gate_the_action` | A rule only affects the actions it was configured to gate.      |
| `test_only_increasing_actions_are_gated`                    | A cap never refuses a cancellation.                             |
| `test_occurrences_expire_at_the_window_boundary`            | The window is rolling; capacity returns as events age out.      |
| `test_a_voided_fill_releases_its_occurrence`                | A voided fill gives its capacity back.                          |
| `test_counters_are_per_rule_and_per_scope`                  | Two rules over one scope stay independent.                      |
| `test_a_repeated_request_cap_counts_per_request_identity`   | A repeated request is counted by shape, not by client order id. |
| `test_request_key_excludes_the_client_order_id`             | The request identity ignores the client order id.               |
| `test_subject_keys_cover_every_scope`                       | Every configured scope maps to a concrete counter key.          |
| `test_scope_key_round_trips_its_scope`                      | A counter key resolves back to its scope.                       |

These are the properties lecture 03's Python rate limit cannot express: a rate limit counts actions
per interval globally, while a cap counts a metric over a chosen scope and window.

## Part 2: the sandbox client end to end

Paper trading means live data and simulated execution. In NautilusTrader that is the sandbox
execution client: a live node in the `SANDBOX` environment with a real data client and a simulated
matching engine in place of a venue connection.

The script below uses a public Coinbase data feed, which needs no credentials, and a simulated
execution client with an explicit starting balance. Save it outside the repository.

```python
"""Sandbox execution client end to end: live Coinbase data, simulated matching engine."""

from __future__ import annotations

import asyncio
from decimal import Decimal

from nautilus_trader.adapters.coinbase import COINBASE
from nautilus_trader.adapters.coinbase import COINBASE_VENUE
from nautilus_trader.adapters.coinbase import CoinbaseDataClientConfig
from nautilus_trader.adapters.coinbase import CoinbaseDataClientFactory
from nautilus_trader.adapters.coinbase import CoinbaseEnvironment
from nautilus_trader.adapters.sandbox import SandboxExecutionClientConfig
from nautilus_trader.adapters.sandbox import SandboxExecutionClientFactory
from nautilus_trader.common import Environment
from nautilus_trader.config import LiveRiskEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.live import LiveNode
from nautilus_trader.model import AccountId
from nautilus_trader.model import AccountType
from nautilus_trader.model import ClientId
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Quantity
from nautilus_trader.model import StrategyId
from nautilus_trader.model import TimeInForce
from nautilus_trader.model import TraderId
from nautilus_trader.testkit import ExecTesterConfig

TRADER_ID = TraderId.from_str("TESTER-001")
ACCOUNT_ID = AccountId.from_str("COINBASE-SANDBOX-001")
STRATEGY_ID = StrategyId.from_str("EXEC_TESTER-001")
INSTRUMENT_ID = InstrumentId.from_str(f"BTC-USDC.{COINBASE}")
USDC = Currency.from_str("USDC")
RUN_SECONDS = 25


async def main() -> None:
    node = (
        LiveNode.builder("SANDBOX-EXEC-TESTER-001", TRADER_ID, Environment.SANDBOX)
        .with_reconciliation(reconciliation=False)
        .with_risk_engine_config(LiveRiskEngineConfig(bypass=True))
        .add_data_client(
            None,
            CoinbaseDataClientFactory(),
            CoinbaseDataClientConfig(environment=CoinbaseEnvironment.LIVE),
        )
        .add_simulated_exec_client(
            COINBASE,
            SandboxExecutionClientFactory(),
            SandboxExecutionClientConfig(
                venue=COINBASE_VENUE,
                oms_type=OmsType.NETTING,
                account_type=AccountType.CASH,
                starting_balances=[Money(float("100000"), USDC)],
                account_id=ACCOUNT_ID,
                fee_model=MakerTakerFeeModel(
                    maker_rate=Decimal("0.001"),
                    taker_rate=Decimal("0.001"),
                ),
            ),
        )
        .build()
    )
    node.add_builtin_strategy(
        "ExecTester",
        ExecTesterConfig(
            strategy_id=STRATEGY_ID,
            instrument_id=INSTRUMENT_ID,
            client_id=ClientId.from_str(COINBASE),
            external_order_instrument_ids=[INSTRUMENT_ID],
            order_qty=Quantity.from_str("0.0001"),
            subscribe_quotes=True,
            subscribe_trades=True,
            open_position_on_start_qty=Decimal("0.0001"),
            open_position_on_first_quote=True,
            open_position_time_in_force=TimeInForce.IOC,
            enable_limit_buys=True,
            enable_limit_sells=False,
            tob_offset_ticks=500,
            use_post_only=True,
            cancel_orders_on_stop=True,
            close_positions_on_stop=True,
            reduce_only_on_stop=False,
            dry_run=False,
            log_data=False,
        ),
    )

    handle = node.handle()
    cache = node.cache
    run_task = asyncio.create_task(node.run_async())
    for _ in range(600):
        if handle.is_running:
            break
        if run_task.done():
            await run_task
            raise RuntimeError("LiveNode stopped during startup")
        await asyncio.sleep(0.1)
    print("NODE RUNNING:", handle.is_running)

    await asyncio.sleep(RUN_SECONDS)

    print("OPEN ORDERS BEFORE STOP:", cache.orders_open_count())
    print(
        "POSITIONS BEFORE STOP :",
        [
            (str(p.instrument_id), str(p.side), str(p.quantity), str(p.avg_px_open))
            for p in cache.positions_open()
        ],
    )
    handle.stop()
    await run_task
    print("NODE STOPPED")
    node.dispose()


if __name__ == "__main__":
    asyncio.run(main())
```

Run it:

```bash
cd D:/Users/vince/PycharmProjects/nautilus_trader/python
uv run --no-sync python C:/Users/vince/AppData/Local/Temp/po_manual/sandbox_demo.py
```

Observed output, trimmed to the lines that matter, from a 43 second run on 2026-10-01:

```text
2026-10-01T19:22:08.260201500Z [INFO] TESTER-001.nautilus_sandbox::execution: Sandbox execution client started: venue=COINBASE, account_id=COINBASE-SANDBOX-001, oms_type=Netting, account_type=Cash
2026-10-01T19:22:09.756545000Z [INFO] TESTER-001.nautilus_sandbox::execution: Sandbox execution client connected: venue=COINBASE
2026-10-01T19:22:09.814866600Z [INFO] TESTER-001.nautilus_trading::strategy: EXEC_TESTER-001 <--[EVT] OrderFilled(instrument_id=BTC-USDC.COINBASE, client_order_id=O-20261001-192209-001-001-1, venue_order_id=COINBASE-0-1, account_id=COINBASE-SANDBOX-001, trade_id=T-7c05784237527833-001, position_id=BTC-USDC.COINBASE-EXEC_TESTER-001, order_side=BUY, order_type=MARKET, last_qty=0.00010000, last_px=84_680.46 USDC, commission=0.00846805 USDC, liquidity_side=TAKER, ts_event=1790882529813615000)
NODE RUNNING: True
2026-10-01T19:22:18.772839900Z [INFO] TESTER-001.nautilus_trading::strategy: EXEC_TESTER-001 <--[EVT] OrderFilled(instrument_id=BTC-USDC.COINBASE, client_order_id=O-20261001-192209-001-001-2, venue_order_id=COINBASE-0-2, account_id=COINBASE-SANDBOX-001, trade_id=T-b3215bb248465023-002, position_id=BTC-USDC.COINBASE-EXEC_TESTER-001, order_side=BUY, order_type=LIMIT, last_qty=0.00010000, last_px=84_675.45 USDC, commission=0.00846754 USDC, liquidity_side=MAKER, ts_event=1790882538772130200)
OPEN ORDERS BEFORE STOP: 1
POSITIONS BEFORE STOP : [('BTC-USDC.COINBASE', 'LONG', '0.00050000', '84664.99000000002')]
2026-10-01T19:22:34.864276500Z [INFO] TESTER-001.nautilus_trading::strategy: EXEC_TESTER-001 <--[EVT] OrderFilled(instrument_id=BTC-USDC.COINBASE, client_order_id=O-20261001-192234-001-001-7, venue_order_id=COINBASE-0-7, account_id=COINBASE-SANDBOX-001, trade_id=T-47a44904eada3443-008, position_id=BTC-USDC.COINBASE-EXEC_TESTER-001, order_side=SELL, order_type=MARKET, last_qty=0.00050000, last_px=84_666.53 USDC, commission=0.04233326 USDC, liquidity_side=TAKER, ts_event=1790882554863853600)
NODE STOPPED
```

Read it in order. The sandbox execution client starts and connects. The built-in `ExecTester`
strategy buys 0.0001 BTC with an immediate-or-cancel market order and the simulated matching engine
fills it as a taker at 84,680.46 USDC. Then it places post-only limit orders; one of them fills as a
maker at 84,675.45 USDC. Twenty-five seconds in, the cache holds one open order and one long position
of 0.00050000 BTC at an average price of about 84,664.99 USDC. On stop, the strategy closes the
position with a market sell that fills as a taker, and the node shuts down cleanly.

### The capture contract that this script demonstrates

`run_async()` lends the node to the coroutine for the duration of the run. Reading state through the
node itself raises while the run is in progress. The first attempt at this script did exactly that:

```text
RuntimeError: LiveNode is being run by `run_async`; use the handle returned by `handle()` to stop
it, and the `cache` and `portfolio` captured before the run to read state
```

The fix, visible in the listing, is to capture `handle` and `cache` before creating the run task.
`docs/concepts/live.md` states the same contract: capture `cache`, `portfolio`, and `handle()` before
starting, call `dispose()` after the run task finishes, and run one live node per process.

### What the sandbox does not simulate

Sandbox execution is honest about being a simulation. It does not reproduce:

| Not simulated                               | Consequence for you                                                                                                                                   |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Venue acceptance rules                      | A venue that rejects an order type or instruction never rejects it here.                                                                              |
| Venue-side order status reports             | The client generates none; on stop it rejects in-flight commands instead of leaving them `SUBMITTED` (`docs/concepts/backtesting/execution-flow.md`). |
| Network latency, by default                 | Commands reach the matching engine immediately unless you configure `latency_model`.                                                                  |
| The return leg of a configured latency      | `StaticLatencyModel` delays submit, modify, and cancel only; venue-generated events are not delayed.                                                  |
| Real venue liquidity                        | Fills come from the simulated matching engine and the models you pass.                                                                                |
| Queue position, unless enabled              | `queue_position` and `liquidity_consumption` default to off.                                                                                          |
| External orders and venue history           | Reconciliation has no venue-side history to compare against, so nothing is repaired.                                                                  |
| Venue margin rules and account restrictions | The account is the local one you configured with `starting_balances`.                                                                                 |

Use the sandbox to rehearse order flow, limits, and shutdown behavior against real prices. Do not
read a sandbox profit as evidence about live profit, because the fills are the ones your
configuration produced.

Continue to [06 - Measure and evaluate](06-measure-and-evaluate.md).