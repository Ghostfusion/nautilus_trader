# 03 - The first run

This lecture gets you from nothing to a printed factor ranking and a set of target weights. The
program is small on purpose. It reads the committed price panel, computes one factor, sorts the
instruments, and prints an equal-weight book. Once you can run it, every later lecture is the same
three moves with more discipline.

## Setup

You need a working NautilusTrader development environment. It is normally created with `uv`; see
[environment setup](../../developer_guide/environment_setup.md). The commands below assume the
repository is checked out and its `python/.venv` exists.

Open a terminal in the repository root. Set the environment, then confirm the package imports:

```bash
export PATH="$HOME/.cargo/bin;$HOME/.local/uv012;$HOME/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
cd "path/to/nautilus_trader"
uv run --no-sync --project python python -c "import nautilus_trader; print(nautilus_trader.__version__)"
```

```text
2.0.0rc6
```

The version string is what the manual was written against. A different version is fine; a missing
import is not.

## The program

Save this as `lecture03.py` somewhere outside the repository (for example in your home directory).
Run it from the repository root, so the relative path to the sample data resolves.

```python
from pathlib import Path

PANEL = Path("docs/usermanauls/factor-portfolio/sample_data/price_panel.csv")
LOOKBACK = 5


def load_panel(path):
    closes = {}
    for line in path.read_text().splitlines()[1:]:
        ts_ns, instrument_id, close = line.split(",")
        closes.setdefault(instrument_id, []).append((int(ts_ns), float(close)))
    return closes


def momentum(closes, lookback):
    latest = closes[-1][1]
    older = closes[-1 - lookback][1]
    return latest / older - 1.0


panel = load_panel(PANEL)
factor = {iid: momentum(series, LOOKBACK) for iid, series in panel.items()}

ranked = sorted(factor, key=factor.get, reverse=True)
print("instrument  factor")
for iid in ranked:
    print(f"{iid:<11} {factor[iid]:+.6f}")

top = ranked[:2]
weight = 1.0 / len(top)
print("\nequal-weight longs:", {iid: round(weight, 4) for iid in top})
```

Run it:

```bash
uv run --no-sync --project python python lecture03.py
```

```text
instrument  factor
DDD.SIM     +0.045455
AAA.SIM     +0.017857
BBB.SIM     +0.004854
CCC.SIM     -0.021127

equal-weight longs: {'DDD.SIM': 0.5, 'AAA.SIM': 0.5}
```

Those are real numbers from the committed panel. `DDD.SIM` rose the most over the last five days and
`CCC.SIM` fell, so the factor ranks `DDD.SIM` first and `CCC.SIM` last.

## Line by line

- `from pathlib import Path` imports the standard library's file-path type. It is the only import;
  this program uses no engine code, because a factor is arithmetic over data you already have.
- `PANEL = Path(...)` is the committed sample file, described in [04](04-sample-data.md). The path
  is relative to the repository root, which is why the program must be run from there.
- `LOOKBACK = 5` is the factor's window: "momentum over the last five days".
- `load_panel` reads the CSV. `.splitlines()[1:]` drops the header row. `line.split(",")` gives the
  three columns: an integer timestamp in nanoseconds, an instrument identifier, and the close price
  as text. `int(ts_ns)` and `float(close)` convert them to numbers. `setdefault(...).append(...)`
  groups the rows by instrument, preserving the file's order.
- `momentum` takes one instrument's list of `(timestamp, close)` pairs and returns the five-day
  simple return: the latest close divided by the close five rows earlier, minus one. `closes[-1]` is
  the last element, `closes[-1 - lookback]` is the one `lookback` positions before it.
- The dict comprehension `{iid: momentum(...) for ...}` computes one factor value per instrument.
- `sorted(factor, key=factor.get, reverse=True)` sorts the instrument identifiers by their factor,
  best first. Sorting keys by a mapping's values is the single most reuseable line in the whole
  manual.
- The print loop walks the ranking. `f"{iid:<11} {factor[iid]:+.6f}"` left-aligns the identifier in
  11 columns and prints the factor with a forced sign and six decimal places, so the columns line
  up and the sign is always visible.
- `top = ranked[:2]` takes the two best. `weight = 1.0 / len(top)` gives each 0.5, so the weights
  sum to 1.0. That is example 1 from lecture 01, applied to real data.

## What you just did, and what you did not

You computed a factor, ranked a universe of four instruments, and produced a long-only target book.
That is the entire skeleton of the style.

You did **not** check that the data was known at the decision instant, you did **not** exclude
instruments that were not yet in the universe, you did **not** measure whether the ranking predicted
anything, and you did **not** test it on data it had not seen. Every one of those is a lecture:

- point-in-time membership and look-ahead: [05](05-build-the-strategy.md);
- measuring whether the ranking predicts, and correcting for how many things you tried:
  [06](06-measure-and-evaluate.md);
- the ways this study can lie to you: [07](07-risks-and-limits.md).

Next: [04 - The sample data](04-sample-data.md).
