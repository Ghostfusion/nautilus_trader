# Training and tuning a strategy

This manual teaches you how to use NautilusTrader as a fast, repeatable environment in which you
search for good strategy parameters and, if you want, feed a learning agent. Training here means
running a strategy many times over a past period with different settings and scoring each run;
tuning means choosing the settings that scored best. It is a search over parameters by default, and
reinforcement learning is an option rather than a requirement.

The one idea the whole manual defends is that a good score found by searching is not evidence of
skill. The more settings you try, the better the best of them looks even when none of them has any
real edge. Every lecture returns to that point with numbers.

## Who should read this

You should read this if you can run a program in a terminal and you want to know how the engine
supports parameter search, why a search needs a validation split, and how to report a search
honestly. You do not need a finance background and you do not need Python experience: every program
is given whole, and each line is explained.

If you have never used the engine at all, read the [intraday and systematic
manual](../intraday-systematic/README.md) first, because it builds a bar-driven strategy without a
search, and this manual assumes you have seen that shape.

## The lecture list

| Lecture                                                  | What it covers                                                              | Time   |
| -------------------------------------------------------- | --------------------------------------------------------------------------- | ------ |
| [01-what-is-ai-training.md](01-what-is-ai-training.md)   | The vocabulary, where the profit comes from, and the overfitting arithmetic | 25 min |
| [02-the-engine-view.md](02-the-engine-view.md)           | How the repository models a search, before any code                         | 20 min |
| [03-first-run.md](03-first-run.md)                       | A complete two-parameter sweep with its exact output                        | 45 min |
| [04-sample-data.md](04-sample-data.md)                   | The committed sample bars, their columns and units                          | 20 min |
| [05-build-the-strategy.md](05-build-the-strategy.md)     | A seeded search, a validation scheme, and a resume from a cache             | 60 min |
| [06-measure-and-evaluate.md](06-measure-and-evaluate.md) | The deflated Sharpe ratio and trial provenance                              | 35 min |
| [07-risks-and-limits.md](07-risks-and-limits.md)         | Overfitting, leakage, cost, and what the engine does not enforce            | 30 min |
| [08-exercises.md](08-exercises.md)                       | Six exercises with solutions and one deliberate breakage                    | 45 min |
| [09-go-further.md](09-go-further.md)                     | Honest gaps and what to read next                                           | 15 min |

## Prerequisites

- A working NautilusTrader development environment, with the `nautilus_trader` package importable
  from `python/`. See the developer guide.
- `uv` available on the path described in the repository's environment setup, or any Python 3.12+
  interpreter with the package installed.
- The sample data in `sample_data/`, which is committed with this manual. You need no market data
  subscription and no network access to follow along.

## Sample-data inventory

| File                                                               | Rows | What it is                                            |
| ------------------------------------------------------------------ | ---- | ----------------------------------------------------- |
| [sample_data/btcusdt-1d-bars.csv](sample_data/btcusdt-1d-bars.csv) | 240  | Daily BTC/USDT bars, for the search in lectures 03-06 |

See [sample_data/README.md](sample_data/README.md) for the columns, units, generator and provenance.

## The engine contract in one paragraph

Parameter optimization is a Python orchestration layer over the Rust execution engine
(`docs/concepts/optimization.md`). It builds a parameter space, enumerates or samples experiments,
runs each experiment through `BacktestNode`, scores each result with an objective and constraints
from `nautilus_trader.analysis`, and ranks the survivors. It composes runs and never alters one, so
every backtest still goes through the single execution path. The layer is Python-only: the matching
engine, the metrics and the risk checks underneath it are the same ones a single backtest uses.
