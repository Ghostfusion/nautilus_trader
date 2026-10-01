# Sample data

## `btcusdt-1d-bars.csv`

- **What it is:** a deterministic, hand-made fixture of 240 one-day BTC/USDT bars, used by every
  program in the lectures.
- **Columns:** `ts_event_ns`, `open_price`, `high_price`, `low_price`, `close_price`, `volume_size`.
- **Units:** the timestamp is integer nanoseconds since the Unix epoch; the four prices are USDT per
  BTC to two decimal places; the volume is BTC to six decimal places.
- **Rows:** 240 data rows plus one header row, 241 lines in total. The series spans
  2021-12-31T00:00:00Z to 2022-08-27T00:00:00Z.
- **Provenance:** synthetic. The prices are a deterministic combination of sine waves and a linear
  drift, chosen to give realistic daily movements. They are not real market data and carry no
  licence obligation, because no venue data was copied.
- **Determinism:** the generator below uses no random numbers and no clock, so running it reproduces
  these bytes exactly.

## The generator

Save as `gen_sample.py` in the repository root and run:

```bash
uv run --project python --no-sync python gen_sample.py
```

```python
import math
from pathlib import Path

OUT = Path("docs/usermanauls/ai-training/sample_data/btcusdt-1d-bars.csv")

ROWS = 240
TS0 = 1_640_908_800_000_000_000  # 2021-12-31T00:00:00Z in nanoseconds
DAY = 86_400_000_000_000

lines = ["ts_event_ns,open_price,high_price,low_price,close_price,volume_size"]
for i in range(ROWS):
    ts = TS0 + i * DAY
    mid = 47_000.0 + 6_000.0 * math.sin(i / 30.0) + 2_000.0 * math.sin(i / 7.0) + 30.0 * i
    open_p = mid + 250.0 * math.sin(i / 9.0)
    close_p = mid + 300.0 * math.sin(i / 6.0)
    high_p = max(open_p, close_p) + abs(200.0 * math.sin(i / 5.0)) + 50.0
    low_p = min(open_p, close_p) - abs(200.0 * math.cos(i / 5.0)) - 50.0
    volume = 1.5 + 0.5 * abs(math.sin(i / 11.0))
    lines.append(f"{ts},{open_p:.2f},{high_p:.2f},{low_p:.2f},{close_p:.2f},{volume:.6f}")

OUT.write_text("\n".join(lines) + "\n", encoding="ascii", newline="\n")
print(f"wrote {OUT} rows={ROWS}")
```

## What the file does not contain

- No order book, so the venue in the lectures runs on an L1 book and the default fill model, with no
  probabilistic slippage by default (`docs/concepts/backtesting/fill-models.md`).
- No trades or quotes, so a strategy that needs tick data cannot be exercised on this fixture.
- No corporate actions or instrument metadata beyond the symbol; the instrument object itself comes
  from `TestInstrumentProvider.btcusdt_binance()` in the programs.
