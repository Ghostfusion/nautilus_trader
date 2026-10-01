# 01 - What is production operations

Production operations is the work you do so that a strategy you trust in a test does not hurt you
when it runs with real money. The strategy decides *what* to trade. Production operations decides
*how much* it is allowed to trade, *who* is told when something goes wrong, *what* is remembered
across a restart, and *how* the engine repairs its own books after the network breaks.

This lecture tells four short stories. Each one is a normal way a live run loses money. Each one
ends with the control that would have caught it, and names the file in this repository that
implements that control. The rest of the manual builds those controls one at a time.

## Story 1: the runaway order loop

A beginner connects a strategy to a live venue and leaves it running. A variable that should hold a
price holds zero instead, so the strategy's condition "the price is below my entry" is always true.
The strategy submits a market order on every market tick. The feed delivers 200 ticks per second, so
in one second the strategy sends 200 orders. Each order buys, so the position grows without limit,
and each fill pays the taker fee.

**The control:** a pre-trade rate limit and a per-order notional cap in the risk engine. The
`RiskEngine` sits on the submit path and can refuse a command before it reaches a venue
(`docs/concepts/execution/index.md`). In lecture 03 you configure both from Python and watch a
six-order burst get cut down to two.

## Story 2: the stale position

A strategy opens a small long position, then the process is stopped for maintenance and started
again. The strategy's variables are fresh: it believes it is flat. Its first action is "if flat,
buy". It buys again, and the account now holds twice the intended exposure. Nothing in the strategy
raised an error, because from the strategy's point of view nothing was wrong.

**The control:** saved state plus startup reconciliation. A strategy can declare exactly what must
survive a restart with `on_save` and `on_load`, the cache can persist that state to a database, and
the live node reconciles cached and venue state before any strategy trades
(`docs/concepts/live.md`, `docs/concepts/execution/reconciliation.md`). Lecture 07 covers the
contract and its one hard requirement: a database backing.

## Story 3: the disconnect

The connection to the venue drops for ninety seconds. The strategy submits an order during the gap.
The submit command left the process, but the acknowledgement never arrived. When the connection
returns, the strategy does not know whether that order exists at the venue, was rejected, or filled
partially. Guessing "it probably failed" and sending a replacement creates two orders. Guessing "it
probably worked" leaves an order you cannot see.

**The control:** transport state events plus execution reconciliation. The live node publishes a
`SocketStateChanged` event when a transport becomes available or is lost, and startup reconciliation
compares local order and position state against authority venue reports, generating the missing
events (`docs/concepts/live.md`, `docs/concepts/execution/reconciliation.md`). Lecture 07 walks the
scenarios.

## Story 4: the silent fill

Everything looks healthy. The strategy shows no errors and the alert channel is quiet. In fact a
fill arrived with an unusual commission, and the engine recorded it against the wrong position leg,
so the displayed profit is wrong. Because nobody watches a number that has always looked fine, the
error is found days later.

**The control:** a small, named set of operational notifications, plus the counters that prove the
alert path itself is working. The notification subsystem raises `ExecutionCompletion` when a fill
completes an execution, `OrderRejection` when an order is rejected, and `RiskLimitBreach` when the
risk engine halts trading (`docs/concepts/notifications.md`). It also counts dropped alerts, so a
quiet channel is distinguishable from a broken one. Lecture 06 covers this, including the fact that
the subsystem is Rust only and that its default transports are plaintext.

## Vocabulary

Read this table once. Every term is used from lecture 02 onward.

| Term            | Plain meaning                                                                             |
| --------------- | ----------------------------------------------------------------------------------------- |
| Venue           | The exchange or broker that receives your orders.                                         |
| Order           | Your instruction to buy or sell something.                                                |
| Fill            | The part of an order that actually traded.                                                |
| Quantity        | How many units you buy or sell.                                                           |
| Price           | The money paid per unit.                                                                  |
| Notional        | Price multiplied by quantity: the money value of an order.                                |
| Market order    | An order that trades immediately at whatever price is available.                          |
| Limit order     | An order that only trades at your price or better.                                        |
| Position        | What you currently hold in one instrument. A long position means you own it.              |
| Netting         | Combining all activity in one instrument into a single position.                          |
| Risk engine     | The component that checks an order before it is sent to a venue.                          |
| Pre-trade check | A risk check that runs before the order leaves the process.                               |
| Denial          | The risk engine's refusal of an order, reported as an `OrderDenied` event.                |
| Rate limit      | A cap on how many actions are allowed in a time interval.                                 |
| Rolling window  | A time interval that slides forward continuously, so old events leave it one by one.      |
| Count cap       | A limit on a counted thing, such as submits or open orders, over a scope and a window.    |
| Reconciliation  | Comparing your own records with the venue's records and repairing the difference.         |
| Cache           | The in-process store of current state: orders, positions, accounts.                       |
| Event store     | The durable, ordered record of the messages that changed state.                           |
| Backtest        | Running a strategy against saved prices in one process.                                   |
| Sandbox         | Running against a live data feed but with a simulated matching engine, so no real orders. |
| Paper trading   | Another name for sandbox trading: live prices, simulated execution.                       |
| Slippage        | The difference between the price you expected and the price you got.                      |
| Drawdown        | The fall from a peak account value to a later lower value.                                |
| Drop counter    | A counter of alerts that could not be delivered, readable from the notification router.   |

## Three worked examples by hand

### Example 1: does this order fit under the notional cap

You trade AUD/USD. The quote is bid 0.71000, ask 0.71010, so a market buy fills at the ask, 0.71010.
You configure a maximum order notional of 100,000 USD.

A market buy of 1,000,000 AUD has notional

```text
1,000,000 x 0.71010 = 710,100.00 USD
```

That is 710,100 USD against a 100,000 USD cap, so the order is refused. In lecture 03 the engine
prints exactly this comparison.

The largest quantity that fits under the cap is

```text
100,000 / 0.71010 = 140,825.23 units
```

AUD/USD is quoted with size precision 0 here, so the quantity must be a whole number. Rounding down
gives 140,825 units, whose notional is

```text
140,825 x 0.71010 = 99,999.83 USD
```

which passes. One more unit would be 100,000.54 USD and would be refused. This is why a notional cap
is safer than a quantity cap: the same quantity means different money as the price moves.

### Example 2: what a submit rate limit does to a burst

You configure `max_order_submit_rate="2/00:00:01"`, which means two order submissions per one second.
Your strategy submits six market orders in a single loop, all at the same timestamp.

```text
order 1 -> admitted
order 2 -> admitted
order 3 -> refused, RATE_LIMIT_EXCEEDED
order 4 -> refused, RATE_LIMIT_EXCEEDED
order 5 -> refused, RATE_LIMIT_EXCEEDED
order 6 -> refused, RATE_LIMIT_EXCEEDED
```

Two of six orders reach the venue. The same limit means that a loop submitting 250 orders per second
exhausts the budget after

```text
2 / 250 = 0.008 seconds
```

and then every further order is refused until the window allows another.

### Example 3: how long a rolling window lasts

A count cap of 2,000 submits over a window of one hour (3,600 seconds) allows, on average,

```text
2,000 / 3,600 = 0.556 submits per second
```

sustained. A strategy that submits 10 orders per second reaches the 2,000 limit after

```text
2,000 / 10 = 200 seconds
```

and is then refused until the earliest submits age out of the window. The window is rolling and
half-open: it has no reset boundary, and capacity returns one event at a time as old events leave it
(`docs/concepts/execution/index.md`).

## Where the money goes

All four stories cost money the same way: the strategy did a thing you did not intend, and nothing
in the system stopped it or told you. Production operations is therefore a list of narrow questions,
each answered by one control.

| Question                                     | Control in this manual                                   |
| -------------------------------------------- | -------------------------------------------------------- |
| Can the strategy send more than I allowed?   | Risk engine rate limits and count caps (lectures 03, 05) |
| Can one order be bigger than I allowed?      | `max_notional_per_order` (lecture 03)                    |
| Will anyone hear about a failure?            | Notifications, queue state, socket state (lecture 06)    |
| Do I know what happened while we were apart? | Event store and reconciliation (lecture 07)              |
| Does a restart lose the strategy's memory?   | `on_save`/`on_load` with a cache backing (lecture 07)    |
| Will the live run behave like my backtest?   | It will not, exactly; lecture 07 says why                |

Continue to [02 - The engine view](02-the-engine-view.md).