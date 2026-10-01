# Production operations

**Production operations** is the work of keeping a trading strategy alive and honest once it trades
real money. A backtest runs in one process against a fixed file of prices; a live run talks to a
venue over the internet, keeps state across restarts, and can lose money while you are asleep. This
manual teaches the controls that stand between the two: risk limits that refuse bad orders, alerts
that reach a human, saved state that survives a crash, reconciliation that repairs the books after a
disconnect, and paper trading that rehearses the whole path without risking capital.

## Who should read this

Read this manual after you have run at least one backtest. You do not need a finance background or a
programming background. You do need to be able to copy text into a terminal and press Enter. If you
have never started a NautilusTrader backtest, work through the
[market-making manual](../market-making/README.md) or the
[EMA cross tutorial](../../tutorials/ema_cross.py) first, then come back.

## Lectures

| Lecture                                                                   | What it covers                                                                       | Time   |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ------ |
| [01 - What is production operations](01-what-is-production-operations.md) | Four ways a live run loses money, and the control that catches each one              | 25 min |
| [02 - The engine view](02-the-engine-view.md)                             | The objects, config, and files NautilusTrader uses to run a strategy for real        | 25 min |
| [03 - First run](03-first-run.md)                                         | Configure a risk engine from Python and watch it refuse a runaway order loop         | 45 min |
| [04 - Sample data](04-sample-data.md)                                     | The three committed CSV files: quotes, a venue fill report, and state snapshots      | 30 min |
| [05 - Build the operational harness](05-build-the-strategy.md)            | Rust-only count caps demonstrated by cargo test, then the sandbox client end to end  | 60 min |
| [06 - Measure and evaluate](06-measure-and-evaluate.md)                   | Reports, the alert event set, the drop counter, and why bodies must stay secret      | 45 min |
| [07 - Risks and limits](07-risks-and-limits.md)                           | Restart recovery, the `on_save`/`on_load` contract, reconciliation, live vs backtest | 50 min |
| [08 - Exercises](08-exercises.md)                                         | Six exercises with solutions and one "break it on purpose" exercise                  | 60 min |
| [09 - Go further](09-go-further.md)                                       | Honest gaps, the concept docs to read next, and where this manual stops              | 15 min |

## Prerequisites

- A working NautilusTrader checkout at `D:/Users/vince/PycharmProjects/nautilus_trader`, branch
  `develop`.
- The Python environment under `python/.venv`, run through `uv`. Lecture 03 shows the exact
  commands.
- For lecture 05 only: an internet connection, a public market data feed, and Rust tooling
  (`cargo` plus `cargo nextest`). The Rust count-cap test needs no network and no venue account.
- No venue credentials are needed anywhere in this manual. The sandbox lecture uses a public data
  feed and a simulated matching engine.

## Safety note

NautilusTrader can place real orders on real venues. Every code sample here is written to run in a
backtest or in the sandbox environment. Do not point any sample at a live execution client until you
have read lecture 07 and rehearsed the strategy in the sandbox.

## Sample data

The `sample_data/` folder holds three small, deterministic CSV files used by the lectures. Every
column name states its unit, every timestamp is an integer number of nanoseconds since the Unix
epoch (the unit the engine uses internally), and every file is ASCII with LF line endings.

| File                                                         | Rows | What it is                                           |
| ------------------------------------------------------------ | ---- | ---------------------------------------------------- |
| [quotes_audusd.csv](sample_data/quotes_audusd.csv)           | 120  | One second of simulated AUD/USD quote ticks          |
| [venue_fills_report.csv](sample_data/venue_fills_report.csv) | 3    | A venue fill report recovered after a connection gap |
| [state_snapshots.csv](sample_data/state_snapshots.csv)       | 3    | What `on_save` returned across two process restarts  |

See [sample_data/README.md](sample_data/README.md) for the full column-by-column description,
provenance, and the generator recipe for each file.