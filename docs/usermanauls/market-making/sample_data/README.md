# Sample data

These files are committed to the repository so the market making course runs with no network
access. Every file is deterministic: running the documented generator produces byte-identical
output.

All prices are decimal, all sizes are integer base units, and all timestamps are integer
nanoseconds since the Unix epoch (`1970-01-01T00:00:00Z`), which is the unit the engine uses. Each
file is ASCII, LF line endings, comma separated, with a header row and a trailing newline.

## `usdjpy_quotes_sample.csv`

- What it is: 300 USD/JPY quote updates.
- Rows: 300 data rows plus the header.
- Columns:
  - `ts_event_ns`: quote timestamp, integer nanoseconds.
  - `bid_price`: best buy price, 3 decimal places.
  - `ask_price`: best sell price, 3 decimal places.
  - `bid_size`: size at the bid, base units.
  - `ask_size`: size at the ask, base units.
- Units: price in JPY per USD; sizes in USD.
- Provenance: derived from the deterministic in-memory generator
  `TestDataProvider.usdjpy_quotes` in `python/nautilus_trader/testkit/providers.py:653`. The file is
  exactly the first 300 generated ticks; lecture `04` verifies this by recomputation.
- Generator command:

  ```bash
  cd python
  uv run --no-sync python -c "
  from pathlib import Path
  from nautilus_trader.testkit.providers import TestDataProvider

  lines = ['ts_event_ns,bid_price,ask_price,bid_size,ask_size']
  for q in TestDataProvider.usdjpy_quotes(count=300):
      lines.append(f'{int(q.ts_event)},{q.bid_price},{q.ask_price},{int(q.bid_size)},{int(q.ask_size)}')
  Path('usdjpy_quotes_sample.csv').write_text('\n'.join(lines) + '\n', encoding='ascii', newline='')
  "
  ```

  The `newline=""` argument is required on Windows; without it Python rewrites the line endings and
  the file no longer matches this committed copy.

## `usdjpy_trades_sample.csv`

- What it is: 300 USD/JPY trade prints at the same timestamps as the quote file.
- Rows: 300 data rows plus the header.
- Columns:
  - `ts_event_ns`: trade timestamp, integer nanoseconds.
  - `price`: trade price, 3 decimal places.
  - `size`: traded size, base units.
  - `aggressor_side`: `BUY` or `SELL`.
- Units: price in JPY per USD; size in USD.
- Provenance: synthetic, generated from the quote file. Even-indexed rows print at the ask with a
  `BUY` aggressor; odd-indexed rows print at the bid with a `SELL` aggressor. Sizes cycle through
  `250000`, `260000`, `270000`.
- Generator command (run after the quote file exists):

  ```bash
  cd python
  uv run --no-sync python -c "
  import csv
  from pathlib import Path

  rows = list(csv.DictReader(Path('usdjpy_quotes_sample.csv').open(newline='', encoding='ascii')))
  lines = ['ts_event_ns,price,size,aggressor_side']
  for i, row in enumerate(rows):
      if i % 2 == 0:
          price, side = row['ask_price'], 'BUY'
      else:
          price, side = row['bid_price'], 'SELL'
      size = 250_000 + 10_000 * (i % 3)
      lines.append(f'{row[\"ts_event_ns\"]},{price},{size},{side}')
  Path('usdjpy_trades_sample.csv').write_text('\n'.join(lines) + '\n', encoding='ascii', newline='')
  "
  ```

## `gbpusd_quotes_sample.csv`

- What it is: 40 GBP/USD quote updates around the worked example market in lecture `01`.
- Rows: 40 data rows plus the header.
- Columns: the same five columns as the USD/JPY quote file, with 5 decimal places.
- Units: price in USD per GBP; sizes in GBP.
- Provenance: synthetic, hand-made for this manual. The first bid is `1.20000` and the first ask is
  `1.20020`, matching the worked example. Each subsequent row adds `0.00001` to both.
- Generator command:

  ```bash
  cd python
  uv run --no-sync python -c "
  from decimal import Decimal
  from pathlib import Path

  base_ns = 1_546_383_600_000_000_000
  lines = ['ts_event_ns,bid_price,ask_price,bid_size,ask_size']
  for i in range(40):
      drift = Decimal(i) * Decimal('0.00001')
      bid = Decimal('1.20000') + drift
      ask = Decimal('1.20020') + drift
      lines.append(f'{base_ns + i * 1_000_000_000},{bid:.5f},{ask:.5f},1000000,1000000')
  Path('gbpusd_quotes_sample.csv').write_text('\n'.join(lines) + '\n', encoding='ascii', newline='')
  "
  ```

## `book_snapshot_deltas.csv`

- What it is: a two-row order book snapshot.
- Rows: 2 data rows plus the header.
- Columns:
  - `exchange`, `symbol`: venue and instrument, here `deribit` and `BTC-PERPETUAL`.
  - `timestamp`: venue timestamp, integer nanoseconds.
  - `local_timestamp`: local capture timestamp, integer nanoseconds.
  - `is_snapshot`: `True` for a full book image.
  - `side`: `bid` or `ask`.
  - `price`: level price.
  - `amount`: level size.
- Units: price in USD; size in contracts.
- Provenance: copied byte for byte from the committed test fixture
  `crates/adapters/tardis/test_data/csv/deltas_1.csv`, which is part of the Tardis adapter test
  data. The original fixture is a three-line file (header plus two rows); this copy is identical.
- How it was produced: a plain file copy, not a generator. To reproduce it from a checkout:

  ```bash
  cp crates/adapters/tardis/test_data/csv/deltas_1.csv \
     docs/usermanauls/market-making/sample_data/book_snapshot_deltas.csv
  ```

## Licence and provenance status

- `usdjpy_quotes_sample.csv` and `usdjpy_trades_sample.csv` are derived from this repository's own
  test generators and carry the repository's licence (LGPL-3.0).
- `gbpusd_quotes_sample.csv` is synthetic and created for this manual; it carries the repository's
  licence.
- `book_snapshot_deltas.csv` is copied from this repository's Tardis test fixture and carries the
  same licence as the rest of the repository. The values are test data, not licensed market data.

No file contains real, licensed market data.
