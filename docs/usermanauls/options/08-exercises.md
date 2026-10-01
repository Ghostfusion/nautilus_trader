# 08 - Exercises

Work each exercise before reading its solution. The solutions are given as the changed
lines or the arithmetic, not as a fresh program. Exercise 7 is the break-it-on-purpose
task: you will delete one field and watch the whole chain go empty.

## Exercise 1: Payoff by hand

You sell one put with strike 60.00 and receive a premium of 3.00. The underlying at expiry
is 54.00. What is your profit or loss, and what is your break-even underlying price?

**Solution.** The put pays `max(60 - 54, 0) = 6.00` to the buyer, so you lose 6.00 on the
contract and keep the 3.00 premium. Your result is `3.00 - 6.00 = -3.00`. The break-even is
`60 - 3 = 57`: above 57 you keep some or all of the premium, below 57 you lose.

## Exercise 2: Read the committed chain

In `sample_data/deribit_btc_option_chain.csv`, find the contract with the widest absolute
bid-ask spread and the two expiries' implied forwards at the strike 65000.00.

**Solution.** The widest spread is on `BTC-26MAR27-55000-C.DERIBIT` at `16878.63 -
16216.73 = 661.90`.

For the near expiry (2026-12-25): `C = 8177.78`, `P = 7509.21`,
`T = 0.258473`, so `F = 65000 + (C - P) * exp(0.04 * T) = 65675.52`.

For the far expiry (2027-03-26): `C = 11556.14`, `P = 10249.64`,
`T = 0.507618`, so `F = 66333.30`.

The far forward is higher, as it should be: the further the expiry, the more the cost of
carrying the position to that date accumulates. If the two forwards were inverted, the
longer-dated contract would be mispriced or the data stale.

## Exercise 3: Pin the strikes

Change the `03` program's subscription line so the chain only ever contains the strikes
64000.00 and 66000.00, regardless of where the underlying is.

**Solution.** Replace the strike range:

```python
# before
strike_range=StrikeRange.atm_relative(1, 1),
# after
strike_range=StrikeRange.fixed([Price.from_str("64000.00"), Price.from_str("66000.00")]),
```

The real output becomes:

```text
2026-09-21T14:13:22.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 1: strikes=2 atm=65000.00
2026-09-21T14:13:23.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 2: strikes=2 atm=65000.00
2026-09-21T14:13:24.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 3: strikes=2 atm=66000.00
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 4: strikes=2 atm=66000.00
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.Probe-000: TOTAL snapshots=4
```

The strike count is two at every snapshot, because a fixed range does not follow the
underlying. Compare the relative range, which reported three strikes near the start and
dropped to two as the active window shifted:

```text
2026-09-21T14:13:22.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 1: strikes=3 atm=65000.00
2026-09-21T14:13:23.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 2: strikes=3 atm=65000.00
2026-09-21T14:13:24.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 3: strikes=2 atm=66000.00
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 4: strikes=2 atm=66000.00
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.Probe-000: TOTAL snapshots=4
```

A fixed range is right when you care about specific strikes. A relative range is right when
you want to stay near the money.

## Exercise 4: Raw mode

Change the subscription so every quote and greeks update publishes a slice immediately,
then count the snapshots.

**Solution.** Set the interval to nothing:

```python
# before
snapshot_interval_ms=1000,
# after
snapshot_interval_ms=None,
```

The real output becomes:

```text
2026-09-21T14:13:20.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 1: strikes=1 atm=65000.00
...
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 64: strikes=2 atm=66000.00
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.Probe-000: TOTAL snapshots=64
```

Sixty-four snapshots instead of four, because the data has six steps by six contracts by two
data types, and each update publishes. The early snapshots have only one strike because the
chain is still filling in. Raw mode is for latency-sensitive strategies that react to
individual updates; snapshot mode is for periodic work. This is the trade-off described in
[options.md](../../concepts/options.md#snapshot-vs-raw-mode).

## Exercise 5: Move the fee model

Run the `03` program with a capped option fee model at 3 basis points per side instead of
the zero-fee model. Confirm the venue accepts it.

**Solution.** Replace the fee model:

```python
# before
from nautilus_trader.execution import MakerTakerFeeModel
...
fee_model=MakerTakerFeeModel(maker_rate=Decimal("0"), taker_rate=Decimal("0")))
# after
from nautilus_trader.execution import CappedOptionFeeModel
...
fee_model=CappedOptionFeeModel(maker_rate=Decimal("0.0003"), taker_rate=Decimal("0.0003")))
```

The real output keeps the chain intact:

```text
2026-09-21T14:13:22.000000000Z [INFO] BACKTESTER-001.Probe-000: snapshot 1: strikes=3 atm=65000.00
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.Probe-000: TOTAL snapshots=4
```

The fee model must be chosen explicitly on the simulated venue; it is not inferred from the
venue name ([options.md](../../concepts/options.md#backtesting-option-chains)).

## Exercise 6: A different chain length

Add a fourth strike of 67000.00 to the `03` program and predict what changes before you run
it.

**Solution.** Change the strike list and let all the loops follow it:

```python
# before
for i, strike in enumerate(["64000.00", "65000.00", "66000.00"]):
# after
for i, strike in enumerate(["64000.00", "65000.00", "66000.00", "67000.00"]):
```

Because the engine now holds eight contracts, the bootstrap line reports eight active
instruments instead of six, and the initial relative window can hold more strikes. The
strategy code does not change: it reads whatever strikes the slice reports. This is the
benefit of subscribing to a chain instead of to individual contracts.

## Exercise 7: Break it on purpose

Delete the `underlying_price` from every `OptionGreeks` event and run the `03` program.
Predict the result first, then run it.

**Solution.** Change the greeks construction:

```python
# before
g = OptionGreeks(instrument_id=inst.id, delta=delta, gamma=0.0002, vega=12.5, theta=-3.2,
                 mark_iv=0.6, bid_iv=0.59, ask_iv=0.61,
                 underlying_price=spot, open_interest=100.0, ts_event=ts, ts_init=ts)
# after
g = OptionGreeks(instrument_id=inst.id, delta=delta, gamma=0.0002, vega=12.5, theta=-3.2,
                 mark_iv=0.6, bid_iv=0.59, ask_iv=0.61,
                 underlying_price=None, open_interest=100.0, ts_event=ts, ts_init=ts)
```

The real output is:

```text
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.Probe-000: TOTAL snapshots=0
```

Zero snapshots. The dynamic strike range cannot choose its active strikes without an
at-the-money price, so the chain never bootstraps and the strategy never receives a slice.
There is no error and no warning from the strategy; the data simply never arrives. This is
the failure mode described in
[options.md](../../concepts/options.md#bootstrap-and-rebalancing): for `atm_relative`,
`atm_percent`, and `delta` ranges, the subscription is deferred until a reference price is
known. A strategy that assumes data will arrive will sit silent forever. Always check that
the at-the-money source is present before trusting an empty chain.

## What to take from these exercises

- The strike range decides how much of the chain you see, and whether it follows the money.
- Snapshot interval trades update volume against reactivity.
- Fees must be declared on the venue.
- The chain subscription hides per-contract plumbing, but it depends on one field,
  `underlying_price`, to find the money.
- The engine will not warn you when a deferred subscription never bootstraps. Check the
  data, not only the code.
