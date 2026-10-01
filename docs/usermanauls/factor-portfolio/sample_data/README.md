# Sample data

This folder holds the only data the factor-portfolio manual needs. It is committed to the
repository, so you can follow every lecture with no network access and no market-data subscription.

## Files

### `price_panel.csv`

A **panel** of daily closing prices: one row per instrument per day.

- Rows: 144 data rows plus one header row.
- Instruments: four, all on the simulated venue `SIM`.
- Days: 36, one observation per instrument per day.
- Line endings: LF. Encoding: ASCII. One trailing newline.

| Column          | Type    | Unit                         | Meaning                                               |
| --------------- | ------- | ---------------------------- | ----------------------------------------------------- |
| `ts_event_ns`   | integer | nanoseconds since Unix epoch | The instant of the close, at 00:00:00 UTC each day.   |
| `instrument_id` | string  | none                         | The instrument, as `SYMBOL.VENUE`, such as `AAA.SIM`. |
| `close`         | decimal | money units                  | The closing price of that instrument at that instant. |

The timestamps run from `1735689600000000000` (2025-01-01T00:00:00Z) to `1738713600000000000`
(2025-02-05T00:00:00Z), inclusive, one per day.

First eight rows, verbatim (the remaining 136 data rows are elided):

```csv
ts_event_ns,instrument_id,close
1735689600000000000,AAA.SIM,100.00
1735689600000000000,BBB.SIM,50.00
1735689600000000000,CCC.SIM,80.00
1735689600000000000,DDD.SIM,20.00
1735776000000000000,AAA.SIM,101.90
1735776000000000000,BBB.SIM,52.05
1735776000000000000,CCC.SIM,80.70
1735776000000000000,DDD.SIM,20.75
```

## How it was produced

The file is **synthetic**: a deterministic fixture, made by the script below. It was not taken from
any real venue, and it is not derived from any repository test fixture. There is no licence issue,
because there is no external source.

The generator has no clock and no randomness, so anyone who runs it obtains byte-identical output.
Save it as `gen_price_panel.py` and run `python gen_price_panel.py`; it writes `price_panel.csv`
beside itself.

```python
"""Generate sample_data/price_panel.csv for the factor-portfolio manual."""

from pathlib import Path

START_NS = 1_735_689_600_000_000_000  # 2025-01-01T00:00:00Z in nanoseconds
DAY_NS = 86_400_000_000_000
DAYS = 36
WAVE = (0.0, 1.0, 0.0, -1.0, 0.0)

INSTRUMENTS = (
    ("AAA.SIM", 100.00, 0.40, 1.5),
    ("BBB.SIM", 50.00, 0.05, 2.0),
    ("CCC.SIM", 80.00, -0.30, 1.0),
    ("DDD.SIM", 20.00, 0.25, 0.5),
)


def main() -> None:
    out = Path(__file__).with_name("price_panel.csv")
    lines = ["ts_event_ns,instrument_id,close"]
    for day in range(DAYS):
        ts = START_NS + day * DAY_NS
        for instrument_id, base, slope, amplitude in INSTRUMENTS:
            close = base + slope * day + amplitude * WAVE[day % len(WAVE)]
            lines.append(f"{ts},{instrument_id},{close:.2f}")
    out.write_text("\n".join(lines) + "\n", encoding="ascii", newline="\n")
    print(f"wrote {out} with {len(lines) - 1} rows")


if __name__ == "__main__":
    main()
```

Each instrument's price is `base + slope * day + amplitude * wave`, where `wave` cycles through
`[0, 1, 0, -1, 0]` so the paths are not straight lines. `AAA.SIM` trends up fastest in price,
`CCC.SIM` trends down, and `DDD.SIM` trends up fastest in percentage terms. Those differences are
what the momentum factor of lecture 05 ranks.

## How this would come from a real venue

A real daily close would arrive from an adapter as a bar, and the engine stores bars in a
`ParquetDataCatalog`. The panel here is the same idea in the simplest possible form: a timestamp,
an instrument, and a price. In production you would read the catalog, filter by the point-in-time
universe (not by today's membership), and assemble the same three columns. Lecture 02 names the
objects involved.

## Regenerating and verifying

- To regenerate the file: run the generator above from any directory; it writes `price_panel.csv`
  next to itself. Copy it over the committed file only if you want to reproduce the exact bytes.
- To verify the committed file: `wc -l price_panel.csv` reports 145 lines (144 data rows plus the
  header), and `file price_panel.csv` reports `CSV ASCII text`.
