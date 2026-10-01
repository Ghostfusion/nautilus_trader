# 05 - Building the option pipeline

This is the core lecture. You will do four things in order: understand how the exercise
style is declared and selected, see a real price from the Rust binomial tree printed by a
cargo test run, check the implied forward from a call and a put, and subscribe to Greeks
from Python. Then you will assemble the pieces into a strategy that follows the
at-the-money strike as the underlying moves.

Everything here is runnable offline. The Rust tree has no Python binding, so it is shown
through its own crate's tests, and a small scratch crate is used to print one number.

## Step 1: Exercise style selection

An option's early-exercise right changes its value, so the engine declares the style on the
instrument and the pricer reads that declaration. The Python-visible enum has two members.

```python
from nautilus_trader.model import ExerciseStyle

print([str(s) for s in ExerciseStyle.variants()])
print("american value:", ExerciseStyle.AMERICAN.value)
print("european value:", ExerciseStyle.EUROPEAN.value)
print("from_str european:", ExerciseStyle.from_str("european"))
```

```text
['AMERICAN', 'EUROPEAN']
american value: 1
european value: 2
from_str european: EUROPEAN
```

The rule, from [options.md](../../concepts/options.md#exercise-style-and-pricing), is:

- `OptionContract` declares American by default, the listed equity convention.
- `CryptoOption` declares European.
- `Instrument::exercise_style` returns the declaration for an option and nothing for any
  other instrument.

In Rust, `price_option` in `crates/model/src/data/pricing.rs` selects the model from that
declaration: European exercise uses the closed-form Black-Scholes price, and American
exercise uses a Cox-Ross-Rubinstein tree with a default of 512 steps
(`crates/model/src/data/binomial.rs`). The Python surface exposes the enum, but not the
per-instrument getter and not the pricer; that asymmetry is the point of the next two
steps.

## Step 2: A price from the Rust tree

First run the crate's own pricing tests. From the repository root:

```bash
export CARGO_TARGET_DIR='D:/Users/vince/PycharmProjects/nautilus_trader/target'
cargo nextest run --locked -p nautilus-model --features python -E 'binary(option_pricing)'
```

The real output is:

```text
    Starting 8 tests across 1 binary (5 binaries skipped)
        PASS [   0.066s] (1/8) nautilus-model::option_pricing crr_converges_to_black_scholes
        PASS [   0.074s] (2/8) nautilus-model::option_pricing zero_volatility_gives_discounted_forward_intrinsic
        PASS [   0.084s] (3/8) nautilus-model::option_pricing at_expiry_price_equals_intrinsic
        PASS [   0.084s] (4/8) nautilus-model::option_pricing reference_value_table
        PASS [   0.095s] (5/8) nautilus-model::option_pricing implied_forward_from_parity_recovers_forward
        PASS [   0.105s] (6/8) nautilus-model::option_pricing american_put_dominates_european_and_intrinsic
        PASS [   0.115s] (7/8) nautilus-model::option_pricing price_option_routes_on_declared_style
        PASS [   0.120s] (8/8) nautilus-model::option_pricing american_call_without_dividend_equals_european
     Summary [   0.121s] 8 tests run: 8 passed, 0 skipped
```

What these eight tests prove, in plain words:

- `crr_converges_to_black_scholes`: as the tree's step count rises, the European tree price
  approaches the closed-form price. The error is bounded, not asserted to one number,
  because the tree's error oscillates with the parity of the step count.
- `american_put_dominates_european_and_intrinsic`: the American put is never cheaper than
  the European put, and never cheaper than immediate exercise. It is an identity every
  American price must satisfy.
- `american_call_without_dividend_equals_european`: an American call on an underlying that
  pays no dividend is worth exactly the European call, because early exercise would only
  throw away the remaining time value.
- `at_expiry_price_equals_intrinsic` and
  `zero_volatility_gives_discounted_forward_intrinsic`: the two edge cases, expiry and zero
  movement.
- `price_option_routes_on_declared_style`: changing only the declared style changes the
  price materially, which is the check that the routing exists.
- `reference_value_table`: a table of European and American prices, with the European values
  checked against the repository's closed form and the American values checked against the
  identities rather than an unsourced number.
- `implied_forward_from_parity_recovers_forward`: the parity helper in Step 3.

The tests pass or fail; they do not print a price. To see one number, create a tiny crate
outside the repository. This is the smallest program that calls the Rust pricer:

```rust
// Cargo.toml
// [package]
// name = "rustprobe"
// version = "0.0.0"
// edition = "2021"
//
// [dependencies]
// nautilus-model = { path = "D:/Users/vince/PycharmProjects/nautilus_trader/crates/model", default-features = false }

use nautilus_model::data::{price_option, implied_forward_from_parity, OptionPricingParams};
use nautilus_model::enums::{ExerciseStyle, OptionKind};

#[test]
fn display_tree_price() {
    let params = |style| OptionPricingParams {
        spot: 100.0, strike: 100.0, risk_free_rate: 0.05, cost_of_carry: 0.05,
        volatility: 0.20, time_to_expiry: 1.0, option_kind: OptionKind::Put,
        exercise_style: style,
    };
    let euro = price_option(&params(ExerciseStyle::European)).unwrap();
    let amer = price_option(&params(ExerciseStyle::American)).unwrap();
    println!("European put = {euro:.6}");
    println!("American put = {amer:.6}");
    println!("American / European = {:.6}", amer / euro);
    println!(
        "implied forward from chain = {:.2}",
        implied_forward_from_parity(8177.78, 7509.21, 65000.0, 0.04, 0.258473)
    );
}
```

Run it with `cargo test -- --nocapture` from that crate. The real output is:

```text
running 1 test
European put = 5.573526
American put = 6.088851
American / European = 1.092459
implied forward from chain = 65675.52
test display_tree_price ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

Read the first two lines. A European put with one year to expiry, spot 100, strike 100,
rate 5 percent, and volatility 20 percent is worth 5.573526. The American put, identical in
every other respect, is worth 6.088851: about 9.2 percent more. That difference is the
early-exercise premium. It exists because the American holder can act before expiry and the
European holder cannot. This is exactly why the style is declared on the instrument rather
than assumed.

## Step 3: The implied forward check

The Rust module also exposes `implied_forward_from_parity`. For a European call and put at
one strike it recovers the forward price of the underlying with
`F = K + (C - P) * exp(r * T)`. It assumes European exercise, a single expiry, no
dividends beyond the carry in the prices, no transaction costs, and one risk-free rate for
borrowing and lending. It is not suitable for American options, where early exercise breaks
parity.

The crate test `implied_forward_from_parity_recovers_forward` checks it against a known
forward. The scratch program above also prints the number for the sample chain:
`65675.52`.

You can check the same value by hand from the committed CSV. Run this from the repository
root:

```python
import csv
import math

rows = list(csv.DictReader(open("docs/usermanauls/options/sample_data/deribit_btc_option_chain.csv", newline="")))
near = [r for r in rows if r["expiry_utc"] == "2026-12-25" and r["strike_price"] == "65000.00"]
call = next(r for r in near if r["option_kind"] == "CALL")
put = next(r for r in near if r["option_kind"] == "PUT")
C = (float(call["bid_price"]) + float(call["ask_price"])) / 2
P = (float(put["bid_price"]) + float(put["ask_price"])) / 2
T = (int(call["expiry_ns"]) - int(call["ts_event_ns"])) / (365.25 * 24 * 3600 * 1_000_000_000)
forward = 65000.0 + (C - P) * math.exp(0.04 * T)
print(f"call mid={C:.2f} put mid={P:.2f} T={T:.6f}")
print(f"implied forward={forward:.2f}")
```

```text
call mid=8177.78 put mid=7509.21 T=0.258473
implied forward=65675.52
```

The implied forward, 65675.52, matches the Rust helper to the cent. This is a real check you
should run on any chain before trusting it: if the forward implied by one strike disagrees
with the forward implied by another strike at the same expiry, one of the two contracts is
stale or mispriced, and the surface filters in [06](06-measure-and-evaluate.md) will reject
it.

## Step 4: A Greeks subscription in Python

Greeks arrive two ways: per instrument and per chain slice. This step shows the per
instrument path, and the handler that the engine calls.

This program constructs one `OptionGreeks` event and formats it the way a strategy's
`on_option_greeks` handler does. It also computes greeks from a known volatility and
implies a volatility from an observed price. Run it from the `python` directory with
`uv run --no-sync python /path/to/file.py`:

```python
from nautilus_trader.model import (
    GreeksConvention, InstrumentId, OptionGreeks, black_scholes_greeks, imply_vol_and_greeks,
)

ID = InstrumentId.from_str("BTC-20261225-65000-C.DERIBIT")


def format_greeks(greeks: OptionGreeks) -> str:
    return (
        f"{greeks.instrument_id}: delta={greeks.delta:.4f} gamma={greeks.gamma:.6f} "
        f"vega={greeks.vega:.4f} theta={greeks.theta:.4f} "
        f"mark_iv={greeks.mark_iv} underlying={greeks.underlying_price}"
    )


event = OptionGreeks(
    instrument_id=ID,
    delta=0.5135, gamma=0.0393, vega=0.1967, theta=-0.0210, rho=0.0,
    mark_iv=0.60, bid_iv=0.59, ask_iv=0.61,
    underlying_price=65000.0, open_interest=400.0,
    ts_event=1_790_000_000_000_000_000, ts_init=1_790_000_000_000_000_000,
    convention=GreeksConvention.BLACK_SCHOLES,
)
print(format_greeks(event))

known = black_scholes_greeks(s=100.0, r=0.05, b=0.05, vol=0.20, is_call=True, k=100.0, t=1.0)
print(f"known vol 0.20 -> price={known.price:.6f} delta={known.delta:.6f} "
      f"gamma={known.gamma:.6f} vega={known.vega:.6f} theta={known.theta:.6f}")

implied = imply_vol_and_greeks(s=100.0, r=0.05, b=0.05, is_call=True, k=100.0, t=1.0, price=10.45)
print(f"observed price 10.45 -> implied vol={implied.vol:.6f} delta={implied.delta:.6f}")
```

```text
BTC-20261225-65000-C.DERIBIT: delta=0.5135 gamma=0.039300 vega=0.1967 theta=-0.0210 mark_iv=0.6 underlying=65000.0
known vol 0.20 -> price=10.450577 delta=0.636831 gamma=0.018762 vega=0.375240 theta=-0.017561
observed price 10.45 -> implied vol=0.199984 delta=0.636835
```

Read the last two lines. With a known volatility of 0.20 and a cost of carry equal to the
rate, the call is worth 10.450577. Feeding the observed price 10.45 back into the implied
solver returns a volatility of 0.199984, essentially 0.20. That round trip is the
definition of implied volatility: it is the volatility that makes the model price equal the
market price.

`black_scholes_greeks` scales vega by 0.01 and theta by 1/365.25, so vega is the change per
one percentage point of volatility and theta is the daily decay. American options are priced
as European for the Python greeks computation; the American tree is the Rust path you ran in
Step 2.

The actual subscription is a strategy or actor method. This block is the live-only shape; it
requires a connected client and is not runnable offline, so no output is shown for it.

```python
from nautilus_trader.model import ClientId

client_id = ClientId("DERIBIT")
self.subscribe_option_greeks(instrument_id, client_id=client_id)
```

The handler you implement is exactly the formatting function above:

```python
def on_option_greeks(self, greeks) -> None:
    self.log.info(
        f"{greeks.instrument_id}: "
        f"delta={greeks.delta:.4f} gamma={greeks.gamma:.6f} "
        f"vega={greeks.vega:.4f} theta={greeks.theta:.4f} "
        f"mark_iv={greeks.mark_iv} underlying={greeks.underlying_price}"
    )
```

To stop, call `self.unsubscribe_option_greeks(instrument_id, client_id=client_id)`. The
complete live examples are
[bybit_option_greeks.py](../../../examples/live/bybit/bybit_option_greeks.py) and
[deribit_option_greeks.py](../../../examples/live/deribit/deribit_option_greeks.py), and the
chain example is
[deribit_option_chain.py](../../../examples/live/deribit/deribit_option_chain.py). All three
connect to a venue and print live data, so they are described rather than faked here.

## Step 5: Assemble the pipeline

Now extend the program from [03](03-first-run.md). Keep the same imports, the same
`make_option` helper, and the same engine setup. Replace the strategy class with this one,
which follows the at-the-money strike, reads the call and the put at that strike, and prints
the difference between them.

```python
class SkewConfig(StrategyConfig):
    def __init__(self, series_id) -> None:
        super().__init__()
        self.series_id = series_id


class SkewStrategy(Strategy):
    def __init__(self, config: SkewConfig) -> None:
        super().__init__(config)
        self.rows = []

    def on_start(self) -> None:
        self.subscribe_option_chain(
            self.config.series_id,
            strike_range=StrikeRange.atm_relative(1, 1),
            snapshot_interval_ms=1000,
        )

    def on_option_chain(self, chain) -> None:
        atm = chain.atm_strike
        if atm is None:
            return
        call = chain.get_call(atm)
        put = chain.get_put(atm)
        if call is None or put is None:
            return
        call_mid = (call.quote.bid_price.as_double() + call.quote.ask_price.as_double()) / 2
        put_mid = (put.quote.bid_price.as_double() + put.quote.ask_price.as_double()) / 2
        call_delta = call.greeks.delta if call.greeks else 0.0
        self.rows.append((str(atm), call_mid, put_mid, call_delta))

    def on_stop(self) -> None:
        self.log.info(f"snapshots with a full ATM leg: {len(self.rows)}")
        for strike, call_mid, put_mid, call_delta in self.rows:
            self.log.info(
                f"atm={strike} call_mid={call_mid:.2f} put_mid={put_mid:.2f} "
                f"call_delta={call_delta:.4f} call-put={call_mid - put_mid:.2f}"
            )
```

Build the data with the underlying price stepping by 150 per second so the at-the-money
strike shifts during the run:

```python
for step in range(6):
    ts = TS + step * 1_000_000_000
    spot = 65000.0 + step * 150.0
    for inst, i, kind in contracts:
        offset = abs(i - 1)
        mid = (8000.0 - i * 500.0) if kind == OptionKind.CALL else (3000.0 + i * 500.0)
        q = QuoteTick(instrument_id=inst.id,
                      bid_price=Price.from_str(f"{mid - 10:.2f}"),
                      ask_price=Price.from_str(f"{mid + 10:.2f}"),
                      bid_size=Quantity.from_int(5), ask_size=Quantity.from_int(5),
                      ts_event=ts, ts_init=ts)
        delta = (0.6 - i * 0.1) if kind == OptionKind.CALL else -(0.4 - i * 0.1)
        g = OptionGreeks(instrument_id=inst.id, delta=delta, gamma=0.0002, vega=12.5, theta=-3.2,
                         mark_iv=0.6 + offset * 0.01, bid_iv=0.59, ask_iv=0.61,
                         underlying_price=spot, open_interest=100.0, ts_event=ts, ts_init=ts)
        data.extend([q, g])

engine.add_data(data)
engine.add_strategy(SkewStrategy(SkewConfig(series_id=series)))
engine.run()
engine.reset()
engine.dispose()
```

The real output, with only the strategy lines kept:

```text
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.SkewStrategy-000: snapshots with a full ATM leg: 4
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.SkewStrategy-000: atm=65000.00 call_mid=7500.00 put_mid=3500.00 call_delta=0.5000 call-put=4000.00
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.SkewStrategy-000: atm=65000.00 call_mid=7500.00 put_mid=3500.00 call_delta=0.5000 call-put=4000.00
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.SkewStrategy-000: atm=66000.00 call_mid=7000.00 put_mid=4000.00 call_delta=0.4000 call-put=3000.00
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.SkewStrategy-000: atm=66000.00 call_mid=7000.00 put_mid=4000.00 call_delta=0.4000 call-put=3000.00
```

The first two snapshots report the at-the-money strike at 65000, matching the underlying
price near 65000. As the underlying price rises, the tracker moves the at-the-money strike
to 66000 and the strategy reads the 66000 call and put instead. The `call-put` column falls
from 4000 to 3000 because the strategy is now reading a different pair of contracts. This is
the pipeline that a real option strategy is built on: follow the money, read the contracts
at the active strikes, and act on the numbers.

## What you have built

You now have four working pieces: exercise-style selection, a Rust tree price with its
tests, the implied forward check, and a Python Greeks subscription. The pipeline follows
the chain as the underlying moves. The next lecture explains the numbers on those contracts,
especially the greeks and the surface checks, and what a good and a bad value look like.
