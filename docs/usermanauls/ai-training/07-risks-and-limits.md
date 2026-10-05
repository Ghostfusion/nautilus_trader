# 07 - Risks and limits

## What breaks in practice

### Overfitting

Overfitting is fitting the past so closely that the fit does not carry forward. In a search it is the
default outcome, not an accident: you try many settings and keep the one that looked best. Lecture 01
computed that the best of 100 noise trials with a 0.10 Sharpe spread shows an annualised Sharpe near
4. The engine gives you the correction for this (`deflated_sharpe_ratio`,
`python/nautilus_trader/optimization/significance.py`) but does not apply it for you, and a corrected
value is still only as good as its declared trial count.

### Leakage

Leakage is information from the evaluation window reaching the training window. It is not a matter of
distance in time. A label looks forward, so a training observation within one label horizon of the
evaluation start carries evaluation information however far apart the sets are
(`docs/concepts/optimization.md`, "Split contracts and leakage"). The defense is a
`LeakagePolicy` with a `purge_before`, an `embargo_after`, and a `label_overlap_rule` that folds a
declared `label_horizon` into the purge. `LabelSeries.forward_reach_ns` is measured from the produced
series rather than derived from the declaration, and `validate_leakage` refuses a policy whose purge
is shorter than that reach, reporting the shortfall in nanoseconds
(`python/nautilus_trader/optimization/labels.py`).

The walk-forward stages declare a zero leakage policy with an explicit justification, because they
compute statistics from results realised inside each window and declare no label horizon
(`python/nautilus_trader/optimization/stages.py`, `WALK_FORWARD_LEAKAGE`). A zero interval is
permitted, but not silently: it must be justified.

### The cost of a large search

Every experiment is a full backtest. Runs fan out to separate processes because execution is
single-threaded inside the kernel and Python holds the GIL, so memory, not CPU, is the binding
constraint. The measured per-run footprint is roughly 71 MiB before a run, peaking near 160 MiB of
working set and committing near 472 MiB of pagefile for the sample run; `DEFAULT_PER_RUN_BYTES` is 512
MiB, rounded up with headroom. The default policy uses 75 percent of available physical memory, so the
worker count is `available_memory * 0.75 // 512 MiB`, clamped to the platform process limit of 61 on
Windows. On a reference machine with about 57.9 GB available this derives 61 workers
(`docs/concepts/optimization.md`, "Concurrency"; `python/nautilus_trader/optimization/concurrency.py`).

Two consequences:

- A sweep budget is a memory budget. A large space with many workers can exhaust memory even though
  each run is small.
- The `config_factory` must be a module-level function so a worker process can import it. A strategy
  defined in a script's `__main__` module cannot be sent to a worker, which is why every program in
  this manual uses `ConcurrencyPolicy(max_workers=1)`. Use one worker for a scratch script and many
  workers only for an importable package.

### Non-reproducible randomness

All randomness is confined to the fill and slippage models; latency, fees and the matching engine are
deterministic given the same inputs. The random state seeds `StdRng` with `random_seed` when one is
provided and otherwise uses `default_std_rng`, which reads host entropy. So an unseeded fill model is
not reproducible across runs, while a seeded one is (`docs/concepts/backtesting/fill-models.md`,
"Determinism and seeds"). For a search this matters more than for a single backtest: if the fill
model draws randomness without a seed, two evaluations of the same experiment differ, and a
comparison between parameter sets mixes the parameter effect with the sampling noise.

The programs in this manual leave the venue on the default fill model, which has
`prob_fill_on_limit=1.0` and `prob_slippage=0.0`, so no draw is stochastic and the sweep is
reproducible. If you set `prob_slippage` above zero, set `random_seed` as well.

### Latency

A bar records four prices and no path between them, and a venue with a latency model delays order
insert, update and cancel by fixed durations; `StaticLatencyModel::new(base, insert, update, delete)`
adds the base to each operation latency (`docs/concepts/backtesting/fill-models.md`, "3. Latency
models"). A study that ignores latency is optimistic about any strategy that cancels or replaces
orders, because the delay is a real cost in live trading. The declared assumptions block records the
bar-execution rules a result ran under, but the venue's latency model is a separate configuration you
supply.

## Position sizing and loss limits

The strategy decides position size. The risk engine can then refuse orders that break a configured
limit. From Python you can set `max_order_submit_rate`, `max_order_modify_rate` and
`max_notional_per_order` on the risk engine configuration
(`crates/risk/src/python/config.rs`; `docs/concepts/optimization.md` lists the Python surface). The
pre-trade send, cancel and fill count caps are set the same way, as `RiskCap` values with the metric
and scope vocabularies in `nautilus_trader.risk`:
`RiskEngineConfig(count_caps=[RiskCap(RiskCapMetric.Submit, RiskCapScope.Instrument, 2_000, 60_000_000_000)])`.

A search does nothing about any of this: the parameters you sweep are strategy
parameters, and a parameter set that produces a large position is exactly as dangerous as the same
parameters run once.

The programs here size a fixed quantity per order and hold at most one position. That is a teaching
choice, not a risk policy. Before running anything like them with real capital, add a per-order
notional limit and a daily loss limit, and remember the risk engine is the authority for neither a
research decision nor a stop-loss.

## What the engine does and does not enforce

The engine enforces:

- Constraints before the objective. An infeasible candidate is recorded with `feasible=False` and no
  penalty in its score (`python/nautilus_trader/optimization/optimizer.py`).
- The validation-scheme boundary, when a scheme is given. The optimizer refuses a run whose window
  does not lie inside one of the scheme's search windows, and names a run inside a held-out window as
  such (`python/nautilus_trader/optimization/optimizer.py`, `_enforce_scheme`).
- Deterministic digests. Equal parameter sets produce equal experiment digests, and equal inputs
  produce equal result digests, so a cache and a comparison are meaningful.
- A digest-keyed cache. A resumed run executes none of the vectors it already holds
  (`python/nautilus_trader/optimization/persistence.py`).

The engine does not enforce:

- That you supply a validation scheme at all. Without a scheme, `optimize` scores whatever window the
  runner is given, including a window you meant to hold out.
- That you apply the deflated Sharpe ratio. It is reported, never a gate; nothing in the subsystem
  consults it before a strategy, an order or a risk check.
- A minimum number of trials or a maximum search budget. The correction reports `unavailable` below
  its declared minimums, but nothing stops a large search.
- A research discipline about what you do with the report. The counts are recorded for the correction
  that needs them; reading them is the human's job.

## The honest limit

A backtest optimised on one period tells you little about the next. The best of a sweep is a
selection drawn from a specific window, a specific space, a specific seed and a specific set of
execution assumptions. Change the period and the selection is no longer the same selection. The
walk-forward stages exist for this reason: they split a period into in-sample and out-of-sample
segments, search on the former and evaluate the selected model on the latter, per window, and never
pool the windows into one search (`python/nautilus_trader/optimization/stages.py`). Even then, a
handful of windows is a small sample, and a good out-of-sample number on one window is one
observation, not a promise about the future.
