# 04 - The sample data

The repository commits two CSV files for this manual in
[sample_data/](sample_data/). They are the smallest data that still behaves like a real
option chain: two expiries, five strikes, two option kinds, and a matching underlying
quote series. This lecture opens them, explains every column, and shows the output of a
real inspection run.

Full provenance, units, row counts, licence, and the exact generator commands are in
[sample_data/README.md](sample_data/README.md). Read that file alongside this one.

## The option chain file

The file is `sample_data/deribit_btc_option_chain.csv`. It has 20 data rows plus the
header. Here is the first part of the file, quoted exactly, with 8 rows elided at the end:

```csv
instrument_id,underlying,expiry_utc,expiry_ns,option_kind,strike_price,bid_price,ask_price,open_interest,underlying_price,ts_event_ns
BTC-25DEC26-55000-C.DERIBIT,BTC,2026-12-25,1798156800000000000,CALL,55000.00,13425.68,13973.66,360.0,65000.00,1790000000000000000
BTC-25DEC26-55000-P.DERIBIT,BTC,2026-12-25,1798156800000000000,PUT,55000.00,3071.27,3196.63,360.0,65000.00,1790000000000000000
BTC-25DEC26-60000-C.DERIBIT,BTC,2026-12-25,1798156800000000000,CALL,60000.00,10459.89,10886.83,380.0,65000.00,1790000000000000000
BTC-25DEC26-60000-P.DERIBIT,BTC,2026-12-25,1798156800000000000,PUT,60000.00,4955.10,5157.34,380.0,65000.00,1790000000000000000
BTC-25DEC26-65000-C.DERIBIT,BTC,2026-12-25,1798156800000000000,CALL,65000.00,8014.22,8341.34,400.0,65000.00,1790000000000000000
BTC-25DEC26-65000-P.DERIBIT,BTC,2026-12-25,1798156800000000000,PUT,65000.00,7359.03,7659.39,400.0,65000.00,1790000000000000000
BTC-25DEC26-70000-C.DERIBIT,BTC,2026-12-25,1798156800000000000,CALL,70000.00,6051.23,6298.21,380.0,65000.00,1790000000000000000
BTC-25DEC26-70000-P.DERIBIT,BTC,2026-12-25,1798156800000000000,PUT,70000.00,10245.64,10663.82,380.0,65000.00,1790000000000000000
BTC-25DEC26-75000-C.DERIBIT,BTC,2026-12-25,1798156800000000000,CALL,75000.00,4511.91,4696.07,360.0,65000.00,1790000000000000000
BTC-25DEC26-75000-P.DERIBIT,BTC,2026-12-25,1798156800000000000,PUT,75000.00,13555.91,14109.21,360.0,65000.00,1790000000000000000
BTC-26MAR27-55000-C.DERIBIT,BTC,2027-03-26,1806019200000000000,CALL,55000.00,16216.73,16878.63,360.0,65000.00,1790000000000000000
BTC-26MAR27-55000-P.DERIBIT,BTC,2027-03-26,1806019200000000000,PUT,55000.00,5333.35,5551.03,360.0,65000.00,1790000000000000000
```

## Every chain column

| Column             | Type and unit              | What it means                                                                  |
| ------------------ | -------------------------- | ------------------------------------------------------------------------------ |
| `instrument_id`    | text                       | The contract's identity in the engine, `SYMBOL.VENUE`. The venue is `DERIBIT`. |
| `underlying`       | text                       | The asset the contract is written on, `BTC`.                                   |
| `expiry_utc`       | date, `YYYY-MM-DD`         | The expiry calendar date, for a human reader.                                  |
| `expiry_ns`        | integer nanoseconds        | The expiry instant, the value the engine stores.                               |
| `option_kind`      | `CALL` or `PUT`            | The right the contract carries.                                                |
| `strike_price`     | quote currency, 2 decimals | The fixed price in the contract.                                               |
| `bid_price`        | quote currency, 2 decimals | The highest price a buyer is showing.                                          |
| `ask_price`        | quote currency, 2 decimals | The lowest price a seller is asking.                                           |
| `open_interest`    | contracts, 1 decimal       | How many contracts are still open at this strike.                              |
| `underlying_price` | quote currency, 2 decimals | The underlying price in force at the snapshot.                                 |
| `ts_event_ns`      | integer nanoseconds        | The instant the snapshot was taken.                                            |

The two timestamp columns are the same instant expressed twice: `expiry_utc` is for you,
`expiry_ns` is for the engine. The engine uses integer nanoseconds since the Unix epoch
everywhere, so a date string is never parsed at run time.

## The underlying file

The file is `sample_data/deribit_btc_underlying_quotes.csv`. It has 10 data rows plus the
header. All 10 rows are quoted here; none are elided.

```csv
instrument_id,bid_price,ask_price,bid_size,ask_size,ts_event_ns
BTC-PERPETUAL.DERIBIT,64999.50,65000.50,10.000,10.000,1790000000000000000
BTC-PERPETUAL.DERIBIT,65004.50,65005.50,10.000,10.000,1790000001000000000
BTC-PERPETUAL.DERIBIT,65009.50,65010.50,10.000,10.000,1790000002000000000
BTC-PERPETUAL.DERIBIT,65014.50,65015.50,10.000,10.000,1790000003000000000
BTC-PERPETUAL.DERIBIT,65019.50,65020.50,10.000,10.000,1790000004000000000
BTC-PERPETUAL.DERIBIT,65024.50,65025.50,10.000,10.000,1790000005000000000
BTC-PERPETUAL.DERIBIT,65029.50,65030.50,10.000,10.000,1790000006000000000
BTC-PERPETUAL.DERIBIT,65034.50,65035.50,10.000,10.000,1790000007000000000
BTC-PERPETUAL.DERIBIT,65039.50,65040.50,10.000,10.000,1790000008000000000
BTC-PERPETUAL.DERIBIT,65044.50,65045.50,10.000,10.000,1790000009000000000
```

| Column          | Type and unit              | What it means                                                 |
| --------------- | -------------------------- | ------------------------------------------------------------- |
| `instrument_id` | text                       | The underlying reference instrument, `BTC-PERPETUAL.DERIBIT`. |
| `bid_price`     | quote currency, 2 decimals | The best bid for the underlying.                              |
| `ask_price`     | quote currency, 2 decimals | The best ask for the underlying.                              |
| `bid_size`      | contracts, 3 decimals      | Size showing at the bid.                                      |
| `ask_size`      | contracts, 3 decimals      | Size showing at the ask.                                      |
| `ts_event_ns`   | integer nanoseconds        | The instant of the quote.                                     |

The two files are not the same shape on purpose. The chain is a cross-section at one
instant; the underlying is a short time series. A real feed works the same way: option
quotes and greeks arrive per contract, and the underlying reference price arrives on its
own stream at its own cadence.

## How to read the chain by hand

Pick the near expiry, 2026-12-25, and the strike 65000.00, which is at the money because the
underlying price is 65000.00.

- The call has a mid price of `(8014.22 + 8341.34) / 2 = 8177.78`.
- The put has a mid price of `(7359.03 + 7659.39) / 2 = 7509.21`.
- The call is worth more than the put by `668.57`.

That difference is not an accident and it is not free money. It reflects the fact that the
forward price of BTC for this expiry is above the spot price, because it costs something to
hold the position to expiry. The next lecture turns this observation into the implied
forward check.

Read the calls down the strikes: at strike 55000 the call mid is about 13699, at 65000 it
is about 8177, and at 75000 it is about 4604. Calls get cheaper as the strike rises. Read
the puts the other way: the put mid is about 3133 at 55000 and about 13832 at 75000. Puts
get more expensive as the strike rises. If you ever see this order reversed with a wide
gap, a strike is mispriced or the data is stale, and the surface filters in lecture
[06](06-measure-and-evaluate.md) are what reject it.

## Inspect it yourself

Run this from the repository root. It reads both files and prints their shape and one
put-call parity check.

```python
import csv
import math

base = "docs/usermanauls/options/sample_data"
chain = list(csv.DictReader(open(base + "/deribit_btc_option_chain.csv", newline="")))
under = list(csv.DictReader(open(base + "/deribit_btc_underlying_quotes.csv", newline="")))

print("chain columns:", list(chain[0].keys()))
print("chain rows:", len(chain))
print("underlying columns:", list(under[0].keys()))
print("underlying rows:", len(under))

expiries = sorted({r["expiry_utc"] for r in chain})
strikes = sorted({r["strike_price"] for r in chain})
kinds = sorted({r["option_kind"] for r in chain})
print("expiries:", expiries)
print("strikes:", strikes)
print("kinds:", kinds)

near = "2026-12-25"
K = "65000.00"
c = next(r for r in chain if r["expiry_utc"] == near and r["strike_price"] == K and r["option_kind"] == "CALL")
p = next(r for r in chain if r["expiry_utc"] == near and r["strike_price"] == K and r["option_kind"] == "PUT")
C = (float(c["bid_price"]) + float(c["ask_price"])) / 2
P = (float(p["bid_price"]) + float(p["ask_price"])) / 2
T = (int(c["expiry_ns"]) - int(c["ts_event_ns"])) / (365.25 * 24 * 3600 * 1_000_000_000)
forward = float(K) + (C - P) * math.exp(0.04 * T)
print(f"near call mid={C:.2f} near put mid={P:.2f} T={T:.6f}")
print(f"implied forward={forward:.2f} spot*exp(rT)={65000.0 * math.exp(0.04 * T):.2f}")

spread = max(float(r["ask_price"]) - float(r["bid_price"]) for r in chain)
print(f"widest bid-ask spread={spread:.2f}")
```

The real output is:

```text
chain columns: ['instrument_id', 'underlying', 'expiry_utc', 'expiry_ns', 'option_kind', 'strike_price', 'bid_price', 'ask_price', 'open_interest', 'underlying_price', 'ts_event_ns']
chain rows: 20
underlying columns: ['instrument_id', 'bid_price', 'ask_price', 'bid_size', 'ask_size', 'ts_event_ns']
underlying rows: 10
expiries: ['2026-12-25', '2027-03-26']
strikes: ['55000.00', '60000.00', '65000.00', '70000.00', '75000.00']
kinds: ['CALL', 'PUT']
near call mid=8177.78 near put mid=7509.21 T=0.258473
implied forward=65675.52 spot*exp(rT)=65675.52
widest bid-ask spread=661.90
```

The last line is a useful warning. The widest spread in this file is 661.90, on a contract
whose price is around 13800. A beginner who crosses that spread immediately pays about 4.8
percent of the contract value just to enter. Spread is a real cost and it is why a chain
snapshot is not a list of fair values.

## How this data would come from a real venue

On Deribit or Bybit, the chain is not delivered as a file. Each contract streams a ticker
update. The adapter turns each update into a `QuoteTick` for the best bid and offer and an
`OptionGreeks` message carrying the venue's delta, gamma, vega, theta, implied volatilities,
underlying price, and open interest. An underlying reference price arrives on a separate
stream. NautechSystems replays recorded `QuoteTick` and `OptionGreeks` from a Nautilus
Parquet catalog in a backtest, which is what the
[Tardis option chain example](../../../examples/backtest/tardis_option_chain.py) does. That
example requires a catalog containing option instruments, quotes, and greeks; without such
a catalog it cannot run. The CSVs in this folder are the manual's stand-in so you can see
the same values without a catalog.

Continue to [05](05-build-the-strategy.md) to build the strategy and check the forward.
