# Universes

An instrument universe is a runtime membership over instruments. Instruments themselves are static
in a run: they are added up front and their data subscriptions are declared by the component that
consumes the data. A universe makes membership a property of the run instead, so instruments can
enter and leave play while the system is running.

A universe is three separate concepts with distinct ownership:

- **Definition**: the rule and settings that describe what is eligible, and what a member holds.
- **Selection**: the scheduled evaluation that decides membership, driven by the clock.
- **Membership**: explicit per-instrument state, with removal as a process rather than an
  immediate unsubscribe.

The component lives in `crates/trading/src/universe/`, and its membership values live in
`nautilus_model::universe` because a membership change crosses component boundaries.

## The definition

A definition carries:

| Setting              | Meaning                                                                  |
| -------------------- | ------------------------------------------------------------------------ |
| `name`               | Identifies the component, its membership topic, and its selection timer. |
| `venue`              | The venue the universe is defined over.                                  |
| `rule`               | Decides which instruments are eligible at an instant.                    |
| `subscriptions`      | What each member holds (instrument, status, quotes, trades, bars).       |
| `selection_interval` | The clock interval for periodic selection, if any.                       |
| `removal_policy`     | What must be true before a departing member releases its subscriptions.  |

A rule answers one question: which instruments are eligible at an instant. It is consulted only
from the selection step, so it must be a function of the instant it is given and of state it
already holds. A rule never reads a wall clock and never issues a data request: the universe owns
the subscriptions and the metadata requests.

Two rules are provided. `StaticUniverseRule` holds the same set at every instant, and
`ScheduledUniverseRule` selects the set with the greatest effective instant at or before the
instant asked about. A schedule is input data to a run, so the same schedule always produces the
same membership at the same instant.

## Selection

Selection is a clock-driven step, and it is canonical:

- The rule is evaluated once per step, at the instant the step is evaluated at.
- Instruments are processed in instrument ID order, so the same rule, definition, and instant
  produce the same changes in the same order.
- Every state change of a step is applied before any of them is reported, so a subscriber never
  observes a half-applied step.
- No wall clock is read. A step is evaluated when the component starts, on the selection interval
  if the definition configures one, and whenever the owning component calls it.

A member is `ADDED` when selection first includes it: its subscriptions are requested and its
instrument definition is requested. It becomes `ACTIVE` once the run knows its definition, which
is how a member becomes tradable rather than merely selected.

## Removal is a process

A member that selection no longer includes moves to `REMOVING`, and it keeps its subscriptions
while it is there. The universe then reports what it can see for that instrument:

- Open orders and open positions are reported once per blocking condition, with the instrument and
  both counts, so a held removal is visible without flooding the log.
- The removal is re-evaluated on every subsequent selection step, and on request. It completes on
  the step after the owning component has closed what the policy requires.

`UniverseRemovalPolicy::REQUIRE_FLAT` (the default) completes a removal only when the member has
no open orders and no open position. `UniverseRemovalPolicy::RELEASE_REGARDLESS` completes it
regardless, leaving any open order or position behind.

A universe never submits or cancels orders. It reports the condition and the component that owns
the orders decides what to do about them. Selection re-including an instrument whose removal began
cancels the removal, and a removed instrument can rejoin later, which is a new membership.

Stopping a universe releases every claim it holds, including the claims of members whose removal
was held, because a stopped component must not leave a subscription behind.

## Subscriptions and metadata

Member subscriptions go through the existing data command path, so each claim the universe holds is
released when the member leaves, and a departing instrument releases only the universe's claim:
another component subscribed to the same instrument keeps its own.

Instrument definitions are requested through the existing request flow rather than a new provider
interface: the universe requests the instruments of its venue once when it starts, and the
definition of each instrument it adds.

## Receiving membership changes

Each change is published on the topic `events.universe.{name}`, and actors and strategies receive
it through `on_universe_changed`:

```python
from nautilus_trader.model import UniverseMembershipState
from nautilus_trader.trading import Strategy


class UniverseTrader(Strategy):
    def on_start(self) -> None:
        self.subscribe_universe_changes("equities")

    def on_universe_changed(self, change) -> None:
        if change.state == UniverseMembershipState.ACTIVE:
            self.subscribe_quotes(change.instrument_id)
```

The subscription is released with the component's other subscriptions, and
`unsubscribe_universe_changes` releases it explicitly. A membership change is not data: it reports
an instrument entering or leaving a universe, not a market event, so it is not routed through the
data pipeline.

## Defining a universe in Python

```python
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Venue
from nautilus_trader.trading import ScheduledUniverseRule
from nautilus_trader.trading import Universe
from nautilus_trader.trading import UniverseDefinition

rule = ScheduledUniverseRule(
    "equities",
    [
        (start_ns + hour_ns, [InstrumentId.from_str("AAPL.XNYS")]),
        (start_ns + 3 * hour_ns, [InstrumentId.from_str("AAPL.XNYS"), InstrumentId.from_str("MSFT.XNYS")]),
    ],
)
definition = UniverseDefinition("equities", Venue("XNYS"), rule)
definition.set_selection_interval_ns(hour_ns)
definition.set_removal_policy(UniverseRemovalPolicy.REQUIRE_FLAT)

engine.add_universe(Universe(definition))
```

A rule may also be any object implementing `select(ts_ns)`, so a universe can be driven by a
research screen rather than by a declared schedule.

A universe is registered with a run through `add_universe`, which the backtest engine, the backtest
node, and the live node all provide. The run then drives the component's lifecycle: its selection
step and its subscription claims start and stop with the run.

## What a universe is not

- It is not a data subscription for the strategy. The universe holds the claims its definition
  declares; a component that trades a member subscribes for itself and releases its own claim.
- It is not order management. It never submits, modifies, or cancels an order.
- It is not a second position store. Open orders and positions are read from the cache, which stays
  authoritative.
