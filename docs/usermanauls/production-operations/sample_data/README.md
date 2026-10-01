# Sample data

This folder holds three small CSV files used by the production-operations lectures. They are ASCII,
comma separated, LF line endings, with a header row and one trailing newline. Every file is
deterministic: the recipe below regenerates the identical bytes.

Provenance for all three files: **synthetic**, written by hand for this manual. None is copied from a
venue, and none is copied from a repository test fixture. The licence status is the same as the
repository: see `LICENSE` at the repository root.

| File                   | Rows | Bytes | SHA-256                                                          |
| ---------------------- | ---- | ----- | ---------------------------------------------------------------- |
| quotes_audusd.csv      | 120  | 7744  | b1c6156e3e3ccc192b971530f9ee1621eef7a50052a45eb0524a3e8010091998 |
| venue_fills_report.csv | 3    | 401   | 04d5ec1b50b2f0d61179c941fa05d8187863364160df4fa6f38712f0c448618c |
| state_snapshots.csv    | 3    | 283   | 6361b86a51c9a4a7bb34c5cb13c9100ac5b1ac4a6eeff801077daa4bfb65458b |

## quotes_audusd.csv

**What it is:** 120 simulated quote ticks for the AUD/USD currency pair, one tick per second, shaped
like the in-memory generator `TestDataProvider.audusd_quotes(count)` in
`python/nautilus_trader/testkit/providers.py`. Lecture 04 reads it and feeds it to the backtest
engine.

**Columns:**

| Column        | Unit                          | Meaning                                          |
| ------------- | ----------------------------- | ------------------------------------------------ |
| ts_event_ns   | nanoseconds since Unix epoch  | When the quote happened, in the venue's clock.   |
| instrument_id | text                          | Venue and symbol together, here `AUD/USD.SIM`.   |
| bid_price     | USD per AUD, 5 decimal places | The highest price a buyer is currently offering. |
| ask_price     | USD per AUD, 5 decimal places | The lowest price a seller is currently asking.   |
| bid_size      | units of AUD                  | How many units are offered at the bid price.     |
| ask_size      | units of AUD                  | How many units are offered at the ask price.     |

**Recipe:** start at `ts_event_ns = 1546383600000000000` and add `1000000000` nanoseconds (one
second) per row. For row `i` starting at zero, compute
`bid = 0.71000 + 0.00500 * sin(i / 300.0)` and `ask = bid + 0.00010`, formatting both to five decimal
places with round-half-even. Set `bid_size = ask_size = 1000000` and
`instrument_id = AUD/USD.SIM`. The generator that produced the committed file is a Python script that
applies exactly this loop with `csv.DictWriter(..., lineterminator="\n")`.

**How a real venue produces this:** a market data feed pushes quote updates over a WebSocket. Each
message carries a symbol, a bid price, a bid size, an ask price, an ask size, and a venue timestamp.
An adapter converts those fields into the engine's `QuoteTick` type. A recorded feed written to disk
by such an adapter is the realistic version of this file.

## venue_fills_report.csv

**What it is:** a venue execution report for three fills, used by lecture 07 to show what
reconciliation compares against. The report covers the moment a connection returned after a 90
second gap.

**Columns:**

| Column              | Unit                         | Meaning                                     |
| ------------------- | ---------------------------- | ------------------------------------------- |
| ts_event_ns         | nanoseconds since Unix epoch | When the venue executed the fill.           |
| venue_order_id      | text                         | The venue's identifier for the order.       |
| client_order_id     | text                         | Your own identifier for the order.          |
| instrument_id       | text                         | Venue and symbol together.                  |
| side                | text                         | `BUY` or `SELL`.                            |
| last_qty            | units of the base asset      | How many units traded in this fill.         |
| last_px             | quote currency per unit      | The price of this fill.                     |
| commission          | quote currency               | The fee the venue charged for the fill.     |
| commission_currency | text                         | The currency of the commission, here `USD`. |

**Recipe:** hand-written. The three timestamps are the base `1546383730000000000` plus 0, 1, and 3
seconds. The first two fills are buys of 100,000 and 150,000 units, so the net long position is
250,000 units. The third is a sell of 50,000 units, leaving 200,000 units. The sizes and prices are
chosen so that a reader can check the arithmetic by hand.

**How a real venue produces this:** after a reconnect, an adapter calls the venue's execution history
endpoint and converts each returned fill into an engine `OrderFilled` event during reconciliation. The
venue returns the same fields shown here; the exact names differ per venue.

## state_snapshots.csv

**What it is:** three rows recording what a strategy's `on_save` returned at two points in one
process, and what `on_load` restored at the start of the next process. Lecture 07 uses it as a
concrete picture of the state contract.

**Columns:**

| Column           | Unit                         | Meaning                                        |
| ---------------- | ---------------------------- | ---------------------------------------------- |
| ts_event_ns      | nanoseconds since Unix epoch | When the state hook ran.                       |
| run_id           | text                         | Which process run the row belongs to.          |
| strategy_id      | text                         | The strategy the state belongs to.             |
| event            | text                         | `on_save` or `on_load`.                        |
| quote_count      | count                        | How many quotes the strategy had processed.    |
| orders_submitted | count                        | How many orders the strategy had submitted.    |
| orders_denied    | count                        | How many of those the risk engine had refused. |
| net_position_qty | units of the base asset      | The strategy's net position at that moment.    |

**Recipe:** hand-written. The values follow lecture 03's CASE 1 run: a 120-tick data set, a burst of
six orders of which two were admitted and four refused, and a resulting net long position of 200,000
units (two fills of 100,000). `RUN-001` saves twice, at the start and near the end; `RUN-002` loads at
its start.

**How a real system produces this:** the live node calls each strategy's `on_save` at shutdown when
`save_state` is enabled, and the cache writes the returned bytes to its database backing. On the next
start, `on_load` receives them back when `load_state` is enabled. This CSV is a readable summary of
that exchange, not the engine's own storage format.

## Regenerating

The committed bytes were produced by running a generator script outside the repository that writes
these three files with `csv.DictWriter(f, fieldnames=..., lineterminator="\n")` and ASCII encoding.
The recipe paragraphs above are the complete specification; a reader who applies them row by row
obtains the SHA-256 values in the table at the top of this file.