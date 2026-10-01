# Sample data

This directory holds two small, deterministic comma-separated files used by the options
manual. Both are ASCII with LF line endings, a header row, and a trailing newline. They are
synthetic fixtures written for teaching: no venue or vendor data is redistributed, and no
real account or order information is present. The licence is the repository licence,
GNU Lesser General Public License v3.0.

Neither file is a full market-data feed. They are a single frozen snapshot that makes the
columns, the units, and the arithmetic inspectable by hand.

## `deribit_btc_option_chain.csv`

What it is: one point-in-time snapshot of a Deribit-style BTC option chain, two expiries
by five strikes by two option kinds, 20 data rows.

| Column             | Unit                       | Meaning                                                   |
| ------------------ | -------------------------- | --------------------------------------------------------- |
| `instrument_id`    | text                       | Contract identifier, `SYMBOL.VENUE`.                      |
| `underlying`       | text                       | Underlying asset symbol, `BTC`.                           |
| `expiry_utc`       | date `YYYY-MM-DD`          | Expiry calendar date, for reading.                        |
| `expiry_ns`        | nanoseconds since epoch    | Expiry instant in the engine's integer nanosecond unit.   |
| `option_kind`      | `CALL` or `PUT`            | The right the contract gives.                             |
| `strike_price`     | quote currency, 2 decimals | The price the contract fixes.                             |
| `bid_price`        | quote currency, 2 decimals | Highest price a buyer is showing.                         |
| `ask_price`        | quote currency, 2 decimals | Lowest price a seller is showing.                         |
| `open_interest`    | contracts, 1 decimal       | Number of contracts still open at this strike.            |
| `underlying_price` | quote currency, 2 decimals | Underlying price in force at the snapshot.                |
| `ts_event_ns`      | nanoseconds since epoch    | Snapshot instant in the engine's integer nanosecond unit. |

Row count: 20 data rows plus the header.

How it was produced: the mids come from the plain Black-Scholes-Merton closed form for a
European option (no dividend) with spot 65000, risk-free rate 4 percent, volatility 60
percent, and the two expiries shown. The bid and ask are the mid minus and plus a spread
of at least 1.00 quote units. Open interest is a synthetic wing-decaying value. Because the
calls and puts share one model, put-call parity holds to rounding, which the manual's
implied-forward check relies on.

Generator command, run from the repository root:

```bash
python - <<'PY'
import csv, math, calendar
from datetime import datetime, timezone

OUT = "docs/usermanauls/options/sample_data/deribit_btc_option_chain.csv"
SPOT, RATE, VOL, TS = 65000.0, 0.04, 0.60, 1790000000000000000
STRIKES = [55000.0, 60000.0, 65000.0, 70000.0, 75000.0]
EXPIRIES = ["2026-12-25", "2027-03-26"]

def ncdf(x):
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))

def bs(kind, s, k, r, vol, t):
    d1 = (math.log(s / k) + (r + 0.5 * vol * vol) * t) / (vol * math.sqrt(t))
    d2 = d1 - vol * math.sqrt(t)
    if kind == "C":
        return s * ncdf(d1) - k * math.exp(-r * t) * ncdf(d2)
    return k * math.exp(-r * t) * ncdf(-d2) - s * ncdf(-d1)

def exp_ns(date_str):
    dt = datetime.strptime(date_str, "%Y-%m-%d").replace(tzinfo=timezone.utc)
    return calendar.timegm(dt.utctimetuple()) * 1_000_000_000

rows = []
for date_str in EXPIRIES:
    e = exp_ns(date_str)
    t = (e - TS) / (365.25 * 24 * 3600 * 1_000_000_000)
    for k in STRIKES:
        for kind in ("C", "P"):
            mid = bs(kind, SPOT, k, RATE, VOL, t)
            hs = max(1.0, round(0.01 * mid * 2, 2))
            sym = "BTC-" + datetime.strptime(date_str, "%Y-%m-%d").strftime("%d%b%y").upper() + f"-{int(k)}-{kind}"
            rows.append({
                "instrument_id": sym + ".DERIBIT", "underlying": "BTC",
                "expiry_utc": date_str, "expiry_ns": e,
                "option_kind": "CALL" if kind == "C" else "PUT",
                "strike_price": f"{k:.2f}", "bid_price": f"{round(mid - hs, 2):.2f}",
                "ask_price": f"{round(mid + hs, 2):.2f}",
                "open_interest": f"{round(400.0 - 4.0 * abs(k - SPOT) / 1000.0, 1):.1f}",
                "underlying_price": f"{SPOT:.2f}", "ts_event_ns": TS,
            })

fields = ["instrument_id", "underlying", "expiry_utc", "expiry_ns", "option_kind",
          "strike_price", "bid_price", "ask_price", "open_interest",
          "underlying_price", "ts_event_ns"]
with open(OUT, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=fields, lineterminator="\n")
    w.writeheader()
    w.writerows(rows)
PY
```

## `deribit_btc_underlying_quotes.csv`

What it is: ten one-second best-bid-and-offer snapshots of the underlying reference
instrument `BTC-PERPETUAL.DERIBIT` around the same fixed snapshot time, 10 data rows.

| Column          | Unit                       | Meaning                                           |
| --------------- | -------------------------- | ------------------------------------------------- |
| `instrument_id` | text                       | Underlying instrument identifier, `SYMBOL.VENUE`. |
| `bid_price`     | quote currency, 2 decimals | Highest price a buyer is showing.                 |
| `ask_price`     | quote currency, 2 decimals | Lowest price a seller is showing.                 |
| `bid_size`      | contracts, 3 decimals      | Size available at the bid.                        |
| `ask_size`      | contracts, 3 decimals      | Size available at the ask.                        |
| `ts_event_ns`   | nanoseconds since epoch    | Snapshot instant in integer nanoseconds.          |

Row count: 10 data rows plus the header.

How it was produced: the mid walks from 65000.00 upward by 5.00 per second, with a fixed
0.50 wide spread and 10.000 size on each side.

Generator command, run from the repository root:

```bash
python - <<'PY'
import csv

OUT = "docs/usermanauls/options/sample_data/deribit_btc_underlying_quotes.csv"
SPOT, TS = 65000.0, 1790000000000000000
rows = []
for i in range(10):
    px = SPOT + i * 5.0
    rows.append({
        "instrument_id": "BTC-PERPETUAL.DERIBIT",
        "bid_price": f"{px - 0.5:.2f}", "ask_price": f"{px + 0.5:.2f}",
        "bid_size": "10.000", "ask_size": "10.000",
        "ts_event_ns": TS + i * 1_000_000_000,
    })
fields = ["instrument_id", "bid_price", "ask_price", "bid_size", "ask_size", "ts_event_ns"]
with open(OUT, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=fields, lineterminator="\n")
    w.writeheader()
    w.writerows(rows)
PY
```

## How these relate to a live venue

A real Deribit option chain arrives as one ticker update per contract. The adapter turns
each ticker into a `QuoteTick` plus an `OptionGreeks` message; an underlying reference price
arrives on the same feed. These CSVs keep only the columns a beginner needs to see, so they
do not include implied volatilities, timestamps per contract, or the venue's own greeks.
The manual shows how the engine fills those in from the Rust surface and greeks code.
