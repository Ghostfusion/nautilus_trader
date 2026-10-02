# Sector Regime Engine

A regime-gated sector allocation tool implementing
[`strategies/sector_rotation_strategies.md`](../../strategies/sector_rotation_strategies.md).

It does not rank sectors and predict leadership. It measures whether leadership contains exploitable
temporal dependence, gates every directional overlay on that measurement, and attributes the result so
that a mechanism and a cost are never confused for one another.

## Run it

```bash
bun install          # nothing to install; there are no dependencies
bun run build        # bundles src/ into app.js (about 60 KB)
bun test             # 32 engine tests, no network, no data files
```

Then open `index.html` in a browser. The bundle is an IIFE rather than an ES module, so the page
works from the filesystem without a server. If you prefer a server:

```bash
bun run serve        # http://127.0.0.1:8787
```

There is no framework, no CDN, no install step beyond bun itself, and no network access at runtime.

## What it does, mapped to the document

| Document section                            | Implementation                                                                                                                       |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Section 2, four regimes including UNCERTAIN | `classifyRegime` in `src/engine.js`                                                                                                  |
| Section 2.1, two independent axes           | Section 4 of the interface: temporal dependence and cross-sectional structure                                                        |
| Section 4.1, comparison types A and B       | `attribution`, reported as separate labelled lines                                                                                   |
| Section 8.1, evidence hierarchy             | Every metric carries a `primary`, `secondary` or `diagnostic` tier; only primary tests can decide the regime                         |
| Section 8.2, the tests                      | Rank autocorrelation, return autocorrelation, variance ratio, conditional persistence, rank turnover, dispersion, correlation, Hurst |
| Section 9, eligibility gating               | `eligibility`, rendered as a table of admissible and blocked strategies                                                              |
| Section 10, two-layer portfolio             | Layer A drift-band rebalancing, Layer B overlay that is off unless admitted                                                          |
| Section 10.5, split cost model              | Trading cost per trade, holding cost as a continuous drag, never mixed                                                               |
| Section 10.6, attribution                   | Five additive log-return lines with the residual printed                                                                             |
| Section 10.7, pre-registration              | Frozen configuration fingerprint, observation and held-out windows, held-out results hidden until revealed                           |

## The decision rule

A directional overlay is admitted only when a **primary** test is both statistically significant and
economically material:

```
significant?          p < alpha, Bonferroni-corrected across the three primary tests
material?             |effect| outside the deadband
                      rank autocorrelation  0.05
                      return autocorrelation 0.05
                      variance ratio deviation from 1  0.2

admitted   = significant AND material   -> POSITIVE_DEPENDENCE or NEGATIVE_DEPENDENCE
conflicting directions                  -> UNCERTAIN
significant but inside the deadband     -> UNCERTAIN, with the reason shown
outside the deadband but not significant-> UNCERTAIN
neither, all inside the deadband        -> MEMORYLESS
too few observations                    -> UNCERTAIN
```

The magnitude requirement is not decoration. On thousands of daily observations an economically
trivial effect is easy to detect; admitting an overlay on one would trade a rounding error against a
spread. Adding this gate took the classifier from 27 of 30 correct on a seeded sweep to 60 of 60.

MEMORYLESS and UNCERTAIN both mean the same thing operationally: **no directional overlay**. The
difference is only what is known about the data.

## Validation

Two independent checks:

- `bun test` asserts the engine's behaviour, including the classifier against synthetic universes
  whose regime is known by construction. Accuracy is asserted as a rate across seeds rather than on a
  single lucky draw, because a statistical classifier has a false positive rate and pretending
  otherwise would be dishonest.
- The interface's **validation sweep** runs the same check in the browser against the configuration
  you have frozen, and reports a confusion matrix. Changing alpha or the deadband changes the
  instrument, so the sweep must be re-run after any change to those.

## Data

**Synthetic** (default). Returns are generated as correlated noise plus a per-sector latent component
whose autocorrelation is set by the chosen regime. `memoryless` uses zero persistence, `persistent`
uses +0.8, `reverting` uses -0.6, and `uncertain` uses a 200-observation sample that is below the
minimum observation count by construction. Ground truth is displayed next to the measurement so
disagreement is visible rather than hidden.

**CSV.** A header row of sector names and a first column of dates, either prices or returns. The
loader detects nothing and assumes nothing: you declare which one it is.

**EODHD.** Optional, and the only part of this directory that touches the network:

```bash
bun run fetch          # 8 years by default
bun run fetch -- 12    # 12 years
```

It reads `EODHD_API_KEY` from the repository `.env` or the environment, the same variable the Rust
adapter uses, and writes `data/sector-prices.csv`. It fetches in one-year windows so no single
response is truncated by a row limit. Two caveats worth knowing: `adjusted_close` is split and
dividend adjusted rather than a licensed total-return index, and the intersection of inception dates
shortens the sample further back than about 2019, because XLC listed in 2018 and XLRE in 2015.

## Configuration notes

**Drift band is relative to the target weight.** A 5 percent band means a weight may drift 5 percent
away from its target, so 0.0909 becomes 0.0955. This matters: with eleven sectors, an *absolute* 5
percentage point deviation from a 9.09 percent target is a 55 percent relative move, so the higher
bands in the frequency grid would never trigger and the grid would say nothing.

**Expense ratios are placeholders.** `PLACEHOLDER_EXPENSE_BPS` is 10 basis points for every fund and
the same number is the default in the interface. Replace it with prospectus figures before drawing any
conclusion from a net-of-cost number.

**Results are single-path.** The backtest is one realised path through one sample. It is not an
estimate of expected performance, and the held-out window is short by construction. Sharpe ratios with
a handful of held-out years are noise.

## Layout

```
index.html            the page
app.js                built bundle, generated by "bun run build"
src/engine.js         all computation: statistics, metrics, regime, portfolio, attribution
src/data.js           universe definition, synthetic generator, CSV parsing
src/ui.js             rendering and wiring only
src/styles.css        styling
tests/engine.test.mjs 32 tests
tools/serve.mjs       optional static server
tools/fetch-eodhd.mjs optional EODHD downloader
```

`src/engine.js` is the whole surface that matters. It is pure functions with no DOM and no I/O, which
is why it can be tested headlessly and why the numbers on screen are the numbers the tests cover.

## What this is not

Not investment advice, not an execution system, and not a claim that any overlay works. Every
directional strategy in the document failed either the significance test or the cost test on the data
available here. The tool exists to make that measurement reproducible, and to make the honest outcome
of it visible: in the default memoryless universe the rebalancing contribution measured against the
component average is positive while the contribution measured against drifting buy and hold is
negative, which is precisely the distinction the document draws in Section 4.1.
