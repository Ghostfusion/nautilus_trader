# 07 - Risks and limits

Options can lose money faster than most beginners expect, and the engine enforces some
limits but not others. This lecture names the three ways an option position bleeds, the
rules the engine does and does not apply, and the one serialization limitation that will
surprise you when you persist a European contract.

## The three fast ways to lose

### 1. Time decay

An option is a wasting asset. Every day it moves closer to expiry, it loses the part of its
price that was paying for the possibility of a large move. This is theta, and it is negative
for a bought option. The decay is not linear: it accelerates in the last weeks. A bought
option can be right about direction and still lose money because the move did not happen
quickly enough.

To see the size of the effect, take the American versus European price from
[05](05-build-the-strategy.md). A European put with one year to expiry was worth 5.573526,
and the American put 6.088851. That difference is a claim on time and choice; when the time
is gone, the difference is gone. Holding a long option costs you every day you wait.

What to do: size the position so the total premium you can lose is a planned amount, and
prefer expiries that give your thesis time to play out.

### 2. Implied volatility moves

Implied volatility is the market's payment for movement. When it falls, every option price
falls, even if the underlying has not moved. Vega measures the sensitivity: an option with
vega 0.1967 loses about 0.1967 for each percentage point of implied volatility that
disappears.

This is the trap of buying options after a spike in fear. You pay a high implied volatility,
the underlying does nothing, the fear subsides, and the option falls from the volatility
change alone. A bought option can lose on time decay and on implied volatility at the same
time, with the underlying exactly where you expected.

What to do: compare the implied volatility you are paying with recent realized movement, and
be suspicious of buying when implied is far above realized.

### 3. The wrong exercise assumption

If you price an American option as if it were European, you undervalue it, and if you sell
it at the European value you have sold something worth more than you received. The engine
does not let you make this mistake silently, because the style is declared on the instrument
and the pricer reads it: `price_option` routes a European contract to the closed form and an
American contract to the tree
(`crates/model/src/data/pricing.rs`, `crates/model/src/data/binomial.rs`). The test
`price_option_routes_on_declared_style` confirms the two prices differ when only the style
changes.

The trap appears when the declaration is wrong or missing. `OptionContract` defaults to
American and `CryptoOption` to European. A crypto contract modeled as the wrong class is
priced by the wrong model. In the local Python calculator, American options are priced as
European for the greeks computation
([greeks.md](../../concepts/greeks.md)), so greeks from the calculator on an American contract
are an approximation.

## The capnp serialization limitation

Persisting an instrument through the Cap'n Proto path loses the exercise style. The schema
for an option contract carries no exercise-style field, and the decoder fills the default
instead. The source states it directly in `crates/serialization/src/capnp/instruments.rs`:
the schema carries no exercise style, so a decoded option contract takes the instrument
class default, American, the listed equity convention, and a non-default style does not
survive a capnp round trip.

The practical consequences are:

- An `OptionContract` declared European becomes American after a capnp encode and decode.
- An `OptionContract` declared American round-trips unchanged, because American is the
  default it lands on anyway.
- A `CryptoOption` is unaffected in practice, because the decoder also applies the class
  default, which is European for that class.

What to do: if your pipeline persists option instruments and relies on a non-default style,
do not trust a capnp round trip to preserve it. Re-declare the style on load from a source
you control, or keep the style in a field the schema does carry, such as `info`, and
re-apply it after decoding. This is a real gap, not a bug in your code.

## Position sizing

Options have two very different risk profiles and the sizing rules differ.

**A bought option** can lose at most the premium. Size it so the sum of premiums on all
bought options is a fixed fraction of your account, for example one or two percent, and
treat that sum as money already spent. Do not average down by buying more of the same
expiring option; that concentrates the same decay in one place.

**A sold option** can lose far more than the premium. The loss is not capped, and the margin
required grows as the position moves against you. Size a short option by the worst plausible
move, not by the premium collected. A short strangle that collects a small credit can lose a
multiple of it in a single session.

**A delta-hedged book** adds a third risk: the hedge is only neutral at the moment you place
it. Gamma and vega drift, and a book that looks flat can become directional quickly. The
repository's delta-neutral example rehedges on every greeks update and on a timer
([delta_neutral_options_bybit.md](../../tutorials/delta_neutral_options_bybit.md)); the hedge
threshold exists because doing nothing is itself a bet.

## What the engine enforces

The risk engine can cap the rate at which a strategy sends orders and the notional of any
single order, configured in Python (`crates/risk/src/python/config.rs`):

- `max_order_submit_rate`: the maximum rate of order submissions.
- `max_order_modify_rate`: the maximum rate of order modifications.
- `max_notional_per_order`: a per-instrument ceiling on order notional.

The pre-trade send, cancel, and fill count caps are a different mechanism and are Rust only:
they live in `crates/risk/src/config.rs` and a Python user cannot set one. If you need those
caps you must configure them at the Rust layer.

Live clients add reconciliation. The delta-neutral example runs with
`with_reconciliation(true)`, which queries the venue at startup and hydrates the cache with
existing orders and positions before the strategy starts. The tutorial warns that this
hydration does not filter by strategy: an unrelated open position on the hedge instrument is
treated as part of the hedge, and a large enough one triggers a hedge order that flattens
it. Run such a strategy on an account with no unrelated positions on the hedge instrument.

## What the engine does not enforce

- It does not cap your loss. A sold option can lose more than the account; the engine will
  submit the order.
- It does not know your intent. A hedge order that flattens an unrelated position is a valid
  order to the engine.
- It does not prevent a wrong exercise-style declaration. It routes on the declaration and
  trusts it.
- It does not repair a bad surface. The surface refuses data that violates its checks; it
  never invents a fix.
- It does not price options in Python. A Python strategy must use venue-provided greeks or
  the local calculator; the exact American price lives in Rust.

## Failure modes to expect in production

| Failure                 | What you observe                                                  | First response                                                               |
| ----------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Greeks stop arriving    | `on_option_greeks` goes quiet, chain snapshots thin out.          | Check the feed and the subscription; the strategy may need a timer fallback. |
| Underlying price absent | Dynamic strike ranges never bootstrap, the chain stays empty.     | Ensure the greeks carry `underlying_price` or a reference price is fetched.  |
| Crossed or stale quotes | Surface filters count rejections by reason.                       | Inspect the rejection counts before blaming the fit.                         |
| Expiry passes           | Contracts stop being active and the manager tears down.           | Unsubscribe or let the manager release the series.                           |
| Wrong style persisted   | A European contract returns as American after a capnp round trip. | Re-declare the style on load.                                                |

## A pre-trade checklist

1. Is the exercise style declared correctly on every option instrument?
2. Is the total premium at risk a planned fraction of the account?
3. For short options, is the worst plausible move survivable?
4. Is implied volatility cheap or expensive relative to recent realized movement?
5. Are the risk engine's order-rate and notional caps set?
6. If positions are persisted, is the style preserved or re-applied on load?
7. Is the account free of unrelated positions that a hedge could flatten?

If you cannot answer all seven, do not size the position yet. The exercises in
[08](08-exercises.md) include one that breaks the style declaration on purpose so you can see
the routing change a price.
