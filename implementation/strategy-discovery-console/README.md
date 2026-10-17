# Strategy Discovery Console

A browser console for the strategy discovery layer of this repository, built from
[the design](../../docs/design/strategy_discovery_design.md),
[the implementation plan](../../docs/design/strategy_discovery_implementation.md) and the twelve
tutorials in [the tutorial group](../../tutorials/strategy-discovery/README.md).

It does not search for strategies and it does not claim that any strategy works. It shows, at every
layer those documents specify, what the layer accepts, what it refuses, what it counts and what it
records: the arithmetic the design states, the refusal codes the code raises, the identities the
ledger keeps, and the four acceptance criteria that do not pass.

## Run it

```bash
bun install          # nothing to install; there are no dependencies
bun run build        # bundles src/ into app.js
bun test             # the engine, data and transcription tests
bun run verify       # every transcription, against the file it names
```

Then open `index.html` in a browser. The bundle is an IIFE rather than an ES module, so the page
works from the filesystem without a server. If you prefer a server:

```bash
bun run serve        # http://127.0.0.1:8791
```

There is no framework, no CDN, no install step beyond bun itself, and no network access at runtime.

`src/data/layers.js` is generated. When the tutorials, the acceptance bundle or the credential list
change, regenerate it and re-run the checks:

```bash
bun run extract      # rewrites src/data/layers.js from the repository
bun test && bun run verify
```

## One campaign, at every layer

The console is organised the way the work happens, and one campaign is shown at every step:

| Step | Layer              | What it decides                                                                    |
| ---- | ------------------ | ---------------------------------------------------------------------------------- |
| 1    | The rules          | Which thresholds, bounds and references the search may not choose                  |
| 2    | The candidate      | Which specifications are admissible at all, as data rather than as code            |
| 3    | The record         | Which identities are computed, what the ledger appends and what the chain protects |
| 4    | The executor       | How one compiled candidate becomes a ranking, weights and instructions             |
| 5    | The counting       | The four counts a campaign keeps, and the correction that reads them               |
| 6    | The held-back data | Which data may be looked at, by which lineage, and once                            |
| 7    | The gates          | Six questions asked in one order, stopping at the first no                         |
| 8    | The search         | How candidates are bred from a seed, and what the kept set does not correct for    |
| 9    | The agents         | What a proposal may be, and what a response is classified as                       |
| 10   | The shadow         | What a frozen candidate is watched against, and what parity with a run means       |
| 11   | The audit          | What the campaign can be shown to have contained, from the ledger alone            |
| 12   | The boundary       | Why execution authority is absent rather than switched off                         |

Three more views sit beside the layers: **One campaign** runs the whole pipeline for the settings you
choose, **The acceptance record** reports all twenty-two criteria from the repository's own bundle,
and **What this console is** states what is modelled and what is not.

## How this relates to the code that does the work

The artifact of record is `python/nautilus_trader/discovery/` and the Rust engine it calls. This
console is a model of it: the rules are implemented in JavaScript, in modules whose only input is
what you type into the page. Two consequences are worth stating plainly.

- **The digests are not the engine's.** The engine's canonical digest is blake3 over a canonical
  serialisation. A page opened from the filesystem cannot rely on WebCrypto, and a pure-JavaScript
  blake3 is a package rather than part of this app, so the console reproduces the discipline - a
  canonical serialisation, a digest over it, a chain over the digests - with a SHA-256 it carries
  itself, and labels every value `sha256:`. The SHA-256 is checked against the published NIST
  vectors and against an independent implementation for every input length through two blocks.
- **Every transcription is checked.** The console quotes the tutorials, transcribes the bounds, the
  refusal codes, the gate ladder and the acceptance record, and lists ninety-two credential names.
  Each of those values is stored inside a `fact(value, path, line, text)` wrapper that records the
  file, the line and the verbatim supporting text, and `tools/check-facts.mjs` fails - by hand and
  inside `bun test` - when that text is no longer in the file. A `Mixed` grade or a failed criterion
  therefore cannot quietly become a passing one.

## What the console refuses to smooth over

The repository's acceptance bundle has twenty-two entries: eighteen pass, four do not, none is
blocked. The four are reported in the interface, on the layer they touch and in their own view:

| Criterion | What it needs                                           | What is missing                                                        |
| --------- | ------------------------------------------------------- | ---------------------------------------------------------------------- |
| 2         | A run identity that joins seed, build and configuration | The engine-side run manifest has no binding in Python                  |
| 4         | The settlement bridge to defer rather than settle       | The bridge resolves eligibility and defers nothing                     |
| 6         | Corporate actions to credit an entitlement              | The stage adjusts prices and does not credit what a holder was owed    |
| 10        | A rerun shown identical from the record alone           | The engine's canonical digest cannot be recomputed from retained bytes |

Two modules the plan names in its layout are unbuilt: the configuration module and the command-line
entry point. Where a layer would read a configuration file, the console says the control is
procedural today, and the same is true of the worker capability allowlist and the half of the
environment assertion that needs a policy-file loader.

## Validation

Two independent checks:

- `bun test` asserts the engines' behaviour headlessly: the refusal codes, the orderings, the
  stopping rule, the arithmetic against hand-computed values from the tutorials, determinism from a
  seed, the chain's behaviour under tampering, and agreement between each engine and the
  transcription it reads. It also runs the fact checker, so a document that moves without the
  console fails the suite.
- `bun run verify` reports the same check over every transcription in `src/data/`, with the file and
  line each value was read from.

Where the console shows a number the design states, the number is asserted in a test. Where the
design states a limit - a procedural control, an unbuilt module, a criterion that fails - the limit
is a value in the code that the interface displays, not a comment.

## Layout

```
index.html                the page
app.js                    built bundle, generated by "bun run build", not tracked
src/main.js               navigation and boot
src/store.js              the one campaign the views share
src/engine/digest.js      SHA-256, the canonical serialisation, the chain rule
src/engine/rng.js         the seeded generator and the summary statistics
src/engine/campaign.js    one campaign, composed through every layer
src/engine/<layer>.js     one module per layer: pure functions, no DOM, no I/O
src/data/source.js        fact(): the provenance wrapper
src/data/layers.js        generated: the twelve layer rows, the criteria, the credentials
src/data/<layer>.js       the transcriptions that layer's engine reads
src/ui/shell.js           the DOM helpers every view uses
src/ui/views/<id>.js      one view per layer, plus the campaign and the acceptance record
tests/<name>.test.mjs     the headless test suite
tools/serve.mjs           optional static server
tools/extract-layers.mjs  regenerates src/data/layers.js from the repository
tools/check-facts.mjs     verifies every transcription against its source
```

`src/engine/` is the whole surface that matters. It is pure functions with no DOM and no I/O, which
is why it can be tested headlessly and why the numbers on screen are the numbers the tests cover.

## What this is not

Not investment advice, not an execution system, not a search that finds strategies, and not a claim
that any candidate is any good. It has no order-submission path and nothing to authenticate one
with; the boundary view is about that absence. Its shadow view is fed synthetic observations rather
than a live adapter, its agent view replays recorded responses rather than calling a model, and its
campaign is one deterministic path through the rules rather than a measurement of anything.
