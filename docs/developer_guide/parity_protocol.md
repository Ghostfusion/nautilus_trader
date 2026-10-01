# Secondary Implementation Parity Protocol

Use this protocol when a second implementation of one kernel is introduced beside the one already in
service. It is engineering policy rather than a research capability: it changes how the second
implementation is written and validated, not what a result means, and it is not a prerequisite for
running research. It becomes actionable only when a second implementation exists.

The transition it was written for is the Cython-to-PyO3 one, where the v1 (Cython) engine is the
reference and the v2 (Rust with PyO3 bindings) engine is the second implementation. The rules below
apply to any other pair in the same way.

## 1. One implementation is the reference

The reference is the implementation published results already rest on. Where two exist, the older one
is the reference until the newer one has passed every case in the checklist below, and no result may
be published from the second implementation before then.

For the Cython-to-PyO3 transition the reference is v1. The out-of-process harness
[`scripts/benchmark-backtest-versions.py`](../../scripts/benchmark-backtest-versions.py) already
treats the two generations that way: it builds an isolated environment per runtime
(`Runtime(version="1.231.0", backend="cython")` and `Runtime(version="2.0.0rc6", backend="pyo3")`),
runs the same scenarios against both, and asserts that the two agree on the Python version and the
precision mode before comparing anything (`The v1 and v2 runtimes do not use the same Python
version`, `... the same precision mode`).

## 2. The second implementation mirrors the reference

The second implementation must preserve:

- argument order, so a positional call means the same thing in both;
- return shape, including what is `None` or empty;
- dtype and memory layout for anything crossing the boundary, because a layout that differs only in
  stride-then-shape order is invisible in a value comparison;
- documented error behavior, so a caller's handling of a failure does not change.

It must be validated on parity, fallback, explicit-error and layout-sensitive cases before it is used
for anything.

## 3. Refusal beats degradation

Where a capability cannot be preserved, the second implementation refuses and the reference runs. A
forced request for the absent implementation raises a typed error rather than silently degrading to
the reference: a call-time choice that changes numerics without saying so would make every published
result ambiguous.

## 4. Benchmarks follow parity

Benchmarks are never the justification for the second implementation. They may be recorded once
parity is stable, and the recording has to state the runtime identity it was taken under, which the
harness does above.

## 5. Import direction

The reference implementation never imports the second one, and public callers never import either
implementation directly. A caller goes through the owning crate or module, which is what makes rule
3 enforceable at all.

## 6. Version agreement is asserted at build time

The two halves of the distribution must agree on their version, and the build asserts it rather than
leaving it to review. `crates/core/build.rs` compares the version declared in `python/pyproject.toml`
against the version the Rust build carries, and its failure message names both:

```text
Version mismatch: pyproject.toml=<version>, hardcoded=<version>
```

The same value reaches the runtime through `nautilus_trader.core.NAUTILUS_VERSION`, which is the
compile-time `NAUTILUS_VERSION` propagated by that build script, while the Python package derives
`__version__` from the installed distribution metadata. A version disagreement therefore fails the
build rather than surfacing as a mismatch in a report.

## 7. Status

- **Not a prerequisite for research**: nothing in the research surface depends on this protocol.
- **Actionable when a second implementation appears**: the checklist below is then applied to it, and
  the parity evidence is recorded with the runtime identities it was taken under.

## Checklist for a second implementation

| Step | What has to be true                                                                                                              |
| ---- | -------------------------------------------------------------------------------------------------------------------------------- |
| 1    | The reference is named, with the revision its published results came from.                                                       |
| 2    | Argument order, return shape, dtype and memory layout are mirrored and asserted case by case.                                    |
| 3    | Parity cases pass, and the comparison is made outside the implementation being tested.                                           |
| 4    | Fallback cases pass: the second implementation refuses, the reference runs, and the refusal is typed.                            |
| 5    | Explicit-error cases pass: a forced request for the absent implementation raises rather than degrades.                           |
| 6    | Layout-sensitive cases pass: a value comparison alone is not accepted as proof.                                                  |
| 7    | Benchmarks are recorded only now, with the runtime identity of each side.                                                        |
| 8    | The import direction holds: the reference does not import the second implementation, and callers use the owning crate or module. |
| 9    | The build-time version agreement covers the pair, and its message names both versions.                                           |
