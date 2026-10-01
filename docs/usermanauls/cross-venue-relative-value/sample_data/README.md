# Sample data for the cross-venue relative-value manual

Two files, one per venue, holding the same underlying over the same 240 one-second intervals. They
are small on purpose so that every number in the manual can be checked by hand.

## Files

| File                              | Rows (excluding the header) | Venue   | Instrument             |
| --------------------------------- | --------------------------- | ------- | ---------------------- |
| `btcusdt_perp_binance_quotes.csv` | 240                         | BINANCE | `BTCUSDT-PERP.BINANCE` |
| `btcusdt_perp_bybit_quotes.csv`   | 240                         | BYBIT   | `BTCUSDT-PERP.BYBIT`   |

Both files have the same header and the same timestamps, in the same order.

## Columns

| Column        | Unit                                                            | Meaning                                                                         |
| ------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `ts_event_ns` | integer nanoseconds since the Unix epoch (1970-01-01T00:00:00Z) | The instant the quote became valid at the venue. The engine uses the same unit. |
| `bid_price`   | USDT per BTC, one decimal                                       | The highest resting buy price. You can sell at this price.                      |
| `ask_price`   | USDT per BTC, one decimal                                       | The lowest resting sell price. You can buy at this price.                       |
| `bid_size`    | BTC, three decimals                                             | The quantity available at the bid in this fixture.                              |
| `ask_size`    | BTC, three decimals                                             | The quantity available at the ask in this fixture.                              |

The two files are comma separated, ASCII, LF line endings, one trailing newline, with a header row.

## Time range

- First row: `1700000000000000000` ns, which is `2023-11-14T22:13:20Z`.
- Step: `1000000000` ns, exactly one second.
- Last row: `1700000239000000000` ns, which is `2023-11-14T22:17:19Z`.

Timestamps are identical in both files, so row `i` of one file is contemporaneous with row `i` of
the other. That is the whole point of this fixture: a cross-venue basis needs two prices of the
same thing at the same instant.

## What the numbers represent

This is a **synthetic** fixture. It is not a recording of any venue. It exists to demonstrate the
mechanics of a basis without requiring a data subscription.

Each file carries the same underlying price plus a venue-specific offset:

```text
btc_mid(i)  = 64000.0 + 40.0 * sin(i / 30.0)
binance_mid = btc_mid(i)
bybit_mid   = btc_mid(i) + 55.0 + 45.0 * sin(i / 45.0)
```

The two venues therefore have the same direction but different levels, and their difference
oscillates between roughly 10 and 100 USDT per BTC. Measured over the file, the basis expressed in
basis points has a minimum of 1.56, a mean of 9.16 and a maximum of 15.62.

The half-spread is 0.5 USDT at BINANCE and 0.8 USDT at BYBIT, so the two venues do not have the
same trading cost. That difference is deliberate: it makes the executable spread differ from the
mid spread. Sizes cycle through 1.000, 1.500 and 2.000 BTC.

## How it was produced

Deterministically, from the standard library only, with no randomness and no network. Run this from
the repository root and it reproduces the committed bytes exactly:

```bash
python - <<'PY'
import csv
import math
from pathlib import Path

OUT = Path("docs/usermanauls/cross-venue-relative-value/sample_data")

ROWS = 240
STEP_NS = 1_000_000_000
START_NS = 1_700_000_000_000_000_000


def rows_for(venue: str) -> list[list]:
    rows = []
    for i in range(ROWS):
        ts = START_NS + i * STEP_NS
        btc_mid = 64000.0 + 40.0 * math.sin(i / 30.0)
        if venue == "binance":
            half = 0.5
            basis = 0.0
        else:
            half = 0.8
            basis = 55.0 + 45.0 * math.sin(i / 45.0)
        mid = btc_mid + basis
        bid = round(mid - half, 1)
        ask = round(mid + half, 1)
        size = 1.0 + (i % 3) * 0.5
        rows.append([ts, f"{bid:.1f}", f"{ask:.1f}", f"{size:.3f}", f"{size:.3f}"])
    return rows


HEADER = ["ts_event_ns", "bid_price", "ask_price", "bid_size", "ask_size"]

for venue in ("binance", "bybit"):
    path = OUT / f"btcusdt_perp_{venue}_quotes.csv"
    with path.open("w", encoding="ascii", newline="") as f:
        writer = csv.writer(f, lineterminator="\n")
        writer.writerow(HEADER)
        writer.writerows(rows_for(venue))
PY
```

`csv.writer` is given `lineterminator="\n"` so the files are LF on every platform.

## How it would come from a real venue

A real venue publishes a top-of-book stream over a WebSocket, and its history is available from the
venue's REST API or from a market-data vendor. To build these files from a real subscription you
would:

1. Subscribe to the book ticker of both instruments.
2. On each update, record the venue timestamp, the best bid and ask and their sizes.
3. Resample or filter so both files share one timestamp grid. This step is the one that needs a
   decision: two venues never publish at exactly the same nanosecond, so you either sample both
   books on a fixed grid (for example once per second), or you forward-fill the venue that did not
   update and record that you did.

The imported `QuoteTick` type is the same either way. For a perpetual future you would also record
`FundingRateUpdate`, and optionally `IndexPriceUpdate` and `MarkPriceUpdate`; the concept pages are
[`../../concepts/data/funding_rate_update.md`](../../../concepts/data/funding_rate_update.md),
[`../../concepts/data/index_price_update.md`](../../../concepts/data/index_price_update.md) and
[`../../concepts/data/mark_price_update.md`](../../../concepts/data/mark_price_update.md).

## Provenance and licence

- **Provenance**: synthetic, hand-designed fixture. Produced by the generator above from a fixed
  formula. No venue data, no vendor data, no third-party fixture was copied.
- **Units**: prices in USDT, sizes in BTC, time in integer nanoseconds.
- **Licence**: part of this repository, under the same licence as the repository. No external
  attribution is required and none is claimed.
