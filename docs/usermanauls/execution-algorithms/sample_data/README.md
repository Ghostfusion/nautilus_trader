# Execution-algorithms sample data

Two small, ASCII, deterministic CSV files. Both are committed to the repository and need no network.

## usdjpy_quotes_sample.csv

- **What it is**: a best-bid/best-ask quote stream for USD/JPY on the simulated venue `SIM`.
- **Rows**: 120 data rows plus one header row (121 lines including the header).
- **Header**: `ts_event_ns,bid_price,ask_price,bid_size,ask_size`
- **Columns**:
  - `ts_event_ns`: event time in integer nanoseconds since the Unix epoch.
  - `bid_price`: best bid, JPY per USD, 3 decimal places.
  - `ask_price`: best ask, JPY per USD, 3 decimal places.
  - `bid_size`: size quoted at the best bid, in USD, integer.
  - `ask_size`: size quoted at the best ask, in USD, integer.
- **Shape**: one tick per second, starting at `1546383600000000000` (2019-01-02 00:00:00 UTC) and
  ending at `1546383719000000000`. The bid follows a sine wave from 109.500 to 109.618; the ask is
  always 0.010 above the bid, so the spread is one price increment throughout.
- **Provenance**: synthetic. Derived from `TestDataProvider.usdjpy_quotes(count=120)` in
  `python/nautilus_trader/testkit/providers.py:680`, which generates the same sine wave in memory.
  Hand-made fixture, no venue licence, safe to commit.
- **Generator command**:

  ```python
  import csv
  from nautilus_trader.testkit.providers import TestDataProvider

  quotes = TestDataProvider.usdjpy_quotes(count=120)
  with open("usdjpy_quotes_sample.csv", "w", newline="") as f:
      writer = csv.writer(f, lineterminator="\n")
      writer.writerow(["ts_event_ns", "bid_price", "ask_price", "bid_size", "ask_size"])
      for q in quotes:
          writer.writerow([int(q.ts_event), str(q.bid_price), str(q.ask_price),
                           str(q.bid_size), str(q.ask_size)])
  ```

  Run it from the repository `python/` directory with `uv run --no-sync python <script>`. The fixed
  `lineterminator="\n"` is required to reproduce the committed LF bytes; the Python `csv` module
  otherwise writes CRLF.

## twap_child_fills_sample.csv

- **What it is**: the six fills of the lecture 03 TWAP run, one row per spawned order plus the
  parent's final slice.
- **Rows**: 6 data rows plus one header row (7 lines including the header).
- **Header**: `client_order_id,exec_spawn_id,spawn_sequence,ts_event_ns,qty,price`
- **Columns**:
  - `client_order_id`: the order's identifier; children end in `-E1`, `-E2`, and so on.
  - `exec_spawn_id`: the parent's identifier. On the parent's own row this equals the row's
    `client_order_id`.
  - `spawn_sequence`: `0` for the parent's final slice, `1` to `5` for the children.
  - `ts_event_ns`: fill time in integer nanoseconds since the Unix epoch.
  - `qty`: filled quantity, in USD, integer.
  - `price`: average fill price, JPY per USD, 3 decimal places.
- **Shape**: six rows of 1000 units each, prices 109.511 to 109.561, filling one per 10 seconds.
  Parent id `O-20190101-230000-001-000-1`.
- **Provenance**: produced by running the lecture 03/05 program against
  `usdjpy_quotes_sample.csv` with the fixed `ProbabilisticFillModel(random_seed=42)`, a parent
  quantity of 6000, `horizon_secs=60`, and `interval_secs=10`. It is a recorded run output, not
  market data.
- **Generator command**: run the lecture 03 program, then dump each cached order's
  `client_order_id`, `exec_spawn_id`, spawn sequence, `ts_last`, `filled_qty`, and `avg_px` with
  `csv.writer(f, lineterminator="\n")`.

## Byte rules

Both files are ASCII, use LF line endings, have a header row and a trailing newline, and contain no
trailing whitespace. Regenerating them with the commands above yields identical bytes.
