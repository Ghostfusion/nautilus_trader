# Notifications

Notifications deliver a small, named set of operational events to outbound channels. The subsystem
is another consumer of the event architecture rather than another cross-cutting dependency: a
subsystem publishes the events it already publishes, and the notification layer translates them. It
lives in Rust, in `nautilus_common::notification`.

## The event set

The event set is closed, not open-ended. A `NotificationEvent` carries exactly one of nine
`NotificationClass` variants, so a sink cannot be registered for an event type it has never heard
of:

| Class                      | Raised from                                       |
| -------------------------- | ------------------------------------------------- |
| `RiskLimitBreach`          | The risk engine halting trading.                  |
| `OrderRejection`           | A rejected order.                                 |
| `StrategyStop`             | A strategy stopping.                              |
| `ExecutionCompletion`      | An order fill completing an execution.            |
| `DrawdownThresholdBreach`  | A position's realized return breaching a threshold. |
| `DataFeedDisconnect`       | A data feed disconnecting.                        |
| `BrokerDisconnect`         | A broker or venue disconnecting.                  |
| `BacktestCompletion`       | A backtest run completing.                        |
| `OptimizationCompletion`   | An optimization run completing.                   |

Each event carries the identity a sink needs to be useful (strategy, instrument, account and run
where applicable), a `NotificationSeverity` (`Info`, `Warning` or `Critical`) and a message.

## The router

`NotificationRouter::subscribe` subscribes on the existing bus points and translates what they
carry: order events (`events.order.*`), position events (`events.position.*`) and the risk engine's
trading state (`events.risk`). Classes with no existing bus event - a strategy stop, a data feed or
broker disconnect, a backtest or optimization completion - are raised by publishing a
`NotificationEvent` to `NOTIFICATION_TOPIC`, or by calling `NotificationRouter::publish` directly.

The router dispatches an event to every sink registered for its class. Publishing never blocks and
never runs transport I/O on the caller's thread: the message goes to the sink's bounded queue and
the call returns. A publish with no sink registered is a no-op, so a subsystem never depends on a
sink being configured. A drawdown threshold is declared on the router
(`set_drawdown_threshold`); without one, position events raise nothing.

## Sinks, queues and coalescing

A `NotificationSink` has a minimal contract: send a titled `NotificationMessage` with a severity.
Each registered sink owns a worker thread fed by a bounded queue with a declared bound and a
coalescing interval (`NotificationSinkConfig`, defaulting to 1024 messages and 250 ms):

- a burst of messages arriving within one interval is coalesced into a single send, which bounds
  the send rate a burst can produce;
- a full queue drops the message and increments a counter, observable through
  `NotificationRouter::drop_count`; the caller is never blocked and the queue never grows without
  limit;
- a delivery failure is reported through the ordinary logging path (`log_error!`) and never
  re-enters the router, so a broken sink cannot loop;
- `NotificationRouter::sent_count` reports the deliveries a sink achieved.

## Transports

Two sinks ship: `EmailSink` over SMTP and `WebhookSink` over an HTTP POST of a JSON body. Each is
built over an injectable `NotificationTransport`, so a caller can substitute its own transport, and
tests drive the default transports against a local server stub.

The default transports (`SmtpTransport`, `HttpTransport`) speak plaintext over a blocking
`std::net::TcpStream` and do not negotiate TLS: `HttpTransport` accepts `http://` URLs only, and
`SmtpTransport` performs no authentication. They suit a private network, a local relay or an
authenticated tunnel, but credentials and notification bodies travel unencrypted on the wire. Use a
custom `NotificationTransport` implementation where TLS is required. No vendor SDK is used.

## Where it lives

The subsystem is `crates/common/src/notification/`: the event set in `event.rs`, the sink trait and
its bounded queue and coalescing in `sink.rs`, the transport seam in `transport.rs`, the SMTP and
HTTP implementations in `email.rs` and `webhook.rs`, and the router in `router.rs`.
