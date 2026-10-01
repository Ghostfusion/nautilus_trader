# 08 - Exercises

Each exercise starts from the lecture 05 program (`search.py`) unless it says otherwise. A solution
is given as the changed lines. Run each one and look at the report, not just at the source.

## Exercise 1: shrink the budget

Task: set the random search to `budget=3` and run it. What are `evaluated`, `space_size` and
`evaluated_fraction` in the report, and why is `executions` still 3 on the first run?

Solution:

```python
-        search = RandomSearch(seed=7, budget=6)
+        search = RandomSearch(seed=7, budget=3)
```

`evaluated` is 3, `space_size` is 9 and `evaluated_fraction` is one third. `executions` is 3 because
the first run executes every evaluation it has not seen. A budget smaller than the space reports the
best of what it evaluated, which is why the winner is not necessarily the winner of the space.

## Exercise 2: add a third parameter

Task: add `trade_size` as a parameter with choices `("0.500000", "1.000000")`. What is the size of the
space now, and what does a `budget=6` random search report as `evaluated_fraction`?

Solution:

```python
         space = ParameterSpace(
             base={
                 "instrument_id": INSTRUMENT_ID,
                 "bar_type": BAR_TYPE,
-                "trade_size": TRADE_SIZE,
             },
             parameters=(
                 Parameter("fast_ema_period", (5, 10, 15)),
                 Parameter("slow_ema_period", (20, 30, 40)),
+                Parameter("trade_size", ("0.500000", "1.000000")),
             ),
         )
```

The space is `3 * 3 * 2 = 18` experiments. With `budget=6` the evaluated fraction is
`6 / 18 = 0.3333`. Note that `trade_size` moved out of `base` into `parameters`: a key cannot be both
a base value and a parameter name, and the space refuses the collision.

## Exercise 3: a different seed is a different selection

Task: change the seed to 99 and confirm the digest order differs from seed 7's. Then change only the
seed back to 7 and confirm the order returns.

Solution:

```python
-        other = make_optimizer(RandomSearch(seed=99, budget=6)).optimize(
+        other = make_optimizer(RandomSearch(seed=1234, budget=6)).optimize(
             space,
             store=ExperimentStore(root / "store3"),
         )
```

With seed 7 the observed order was
`['462d738d', '6ea5e04b', 'b6146026', 'c666b92d', '55d700f6', 'fd9da49e']`; with seed 99 it was
`['f60e860e', 'b6146026', '8cecaeb5', '6de5edfe', '55d700f6', 'fd9da49e']`. Changing the seed
changes which experiments are selected, and restoring it restores the selection exactly.

## Exercise 4: make every candidate infeasible

Task: change the drawdown constraint so that no experiment can satisfy it, run the sweep, and observe
`best_feasible`.

Solution:

```python
-            constraints=(Constraint(DRAWDOWN, ConstraintComparison.AT_LEAST, -0.05),),
+            constraints=(Constraint(DRAWDOWN, ConstraintComparison.AT_LEAST, 0.0),),
```

The runs finish, but each is recorded with `constraints_satisfied=False`. `report.best()` still
returns the best-ranked result, and `report.best_feasible()` returns `None`, because no candidate is
feasible. An infeasible candidate is not given a penalty score, so the ranking is unchanged; only the
feasibility flag differs. In lecture 03's program the printed line `best_feasible=` would read
`False`.

## Exercise 5: dependence must be declared

Task: build a `SharpeSample` with `dependence=TrialDependence.DEPENDENT` and no `effective_trials`,
then with an `effective_trials` above the nominal count. What happens?

Solution:

```python
SharpeSample(
    sharpe=0.15,
    trial_sharpes=[0.15, 0.12, 0.10],
    observations=250,
    skew=0.0,
    kurtosis=3.0,
    dependence=TrialDependence.DEPENDENT,
)
```

The first call raises `ValueError: a study with dependent trials must declare its effective trial
count`. Adding `effective_trials=5` raises `ValueError: effective_trials 5 exceeds the nominal count
3`. Dependence is declared, never inferred, because a sweep over adjacent parameters is not an
independent sample and the correction cannot tell from the values alone
(`python/nautilus_trader/optimization/significance.py`).

## Exercise 6: prove the cache from the file system

Task: give the store a permanent directory instead of a temporary one, run the sweep twice in two
separate processes, and inspect the store between the runs.

Solution:

```python
-    with tempfile.TemporaryDirectory() as tmp:
-        root = Path(tmp)
+    with tempfile.TemporaryDirectory() as tmp:
+        root = Path("runs")
+        root.mkdir(exist_ok=True)
```

After the first run, `runs/store/report.json` exists and lists the canonical digests, the failures,
`evaluated`, `space_size`, `executions` and the seed. The second process prints
`resumed: executions 0`: the return series is not re-read because every experiment digest is already
in `results/`. The manifest is the machine-readable record of the sweep.

## Exercise 7: break it on purpose

Task: delete `concurrency=ConcurrencyPolicy(max_workers=1)` from the optimizer so the default
memory-driven worker count applies, and run the program.

Solution:

```python
-            concurrency=ConcurrencyPolicy(max_workers=1),
             search=search,
```

The sweep no longer runs in this process. A worker process is spawned, and the strategy defined in the
`__main__` module cannot be imported there. The observed output was:

```text
best is None: True
evaluated=9 space_size=9 executions=9
evaluated_fraction=1.0
failures=9
FAIL AssertionError
FAIL AssertionError
FAIL AssertionError
```

Every experiment is recorded as a failed experiment with an empty `AssertionError`, so no result
survives and `report.best()` is `None`. The lesson is that the config factory and the strategy must be
importable by module path for a worker to use them. The fix is either to keep one worker, as the
manual does, or to move the strategy into an importable module and reference it as
`module:Class` in `BacktestRunner`.
