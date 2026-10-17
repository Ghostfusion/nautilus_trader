# The boundary: execution authority absent rather than switched off

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing is bought or sold: the part acts on the reach of the discovery process itself, checking that no order path and no credential can leave it                                                                                                                                                                                                                                                   |
| How often it trades       | Once per test run, at the end of the discovery work: the four assertions are made in one pass and nothing is bought or sold                                                                                                                                                                                                                                                                         |
| What you need             | Nothing but this page; the exercise needs only a written list and a pencil                                                                                                                                                                                                                                                                                                                          |
| Where the rules come from | [The execution boundary in the design](../../../docs/design/strategy_discovery_design.md), Section 12, and the four assertions in [the boundary fixture](../../../python/tests/integration/test_discovery_boundary.py)                                                                                                                                                                              |
| The underlying research   | No paper behind it: the boundary is this repository's own design, [strategy_discovery_design.md](../../../docs/design/strategy_discovery_design.md), Section 4 invariant 9 and Section 12                                                                                                                                                                                                           |
| How well it held up       | Mixed: the four assertions run and the boundary criterion passes, with the import graph resolved in a clean interpreter rather than over source text, but the dependency check is evidence rather than proof, and the worker allowlist is declared and checked in `discovery/config.py` while nothing provisions the environment it describes, so the shape of that environment is still procedural |
| Also appears in           | [What a candidate is](../what-a-candidate-is/README.md), where the refusals this page records are produced, and the [group index](../README.md)                                                                                                                                                                                                                                                     |

## The idea in one paragraph

A boundary in a computer system is usually a rule: something is possible, and a check is supposed to
stop it. The boundary described here is different, and the difference is the whole point. The
discovery layer, the part of this repository that proposes strategy ideas and tests them, has no
broker connection installed, no credential in the environment it runs in, and no code path that can
send an order; nothing was switched off, because the pieces were never put in. Four assertions make
that absence observable rather than asserted: the import graph reaches no adapter module, the only
execution client a worker is given is the sandbox one, no credential from the repository's canonical
list is in the environment the boundary defines, and a candidate that tries to name an execution
venue is refused and the refusal is recorded. The rest of this page says what each assertion
observes, and what none of them can show.

## Why anyone believed it

The argument begins with what a setup switch is worth. A setting that turns live trading off is a
line in a file: it can be typed the wrong way, a later change can leave it on by default, and a path
around it can be built without anyone noticing at the time. The design records the reviewers'
position that a configuration flag is not a boundary, and answers with removal instead of
restriction. The environment holds no adapter and no credential, so a submission path that somehow
survived would have nothing to reach and nothing to authenticate with.

The second belief is that a claim of absence should be checkable. This repository already kept one
canonical list of adapter credential variables for its own pre-flight checks, which turns "the
discovery process reads none of them" from a promise into a statement a test can read. The third is
that the unit of safety is the shape of the environment rather than an inspection of code: what can
be reached from here is a fact about a running interpreter, not an opinion about source text.

## An everyday comparison

A workshop that does metalwork wants no chance of an oxy-gas torch being lit by mistake. One way is
to buy the torch and keep it in a locked cabinet. The other is not to have the torch at all: the
bench has no gas line, no regulator and no hoses, and the cupboard that would hold the cylinders was
never built, so a visitor searching the room all day finds nothing that could light. The second
workshop is this boundary, the locked cabinet is what the design calls a configuration flag, and the
four assertions are the visitor's walk through the room written down so it can be repeated.

## The rules, step by step

1. Resolve the import graph in a clean interpreter. A separate interpreter is started, the discovery
   package is imported in it, and it prints every module it loaded whose name begins with
   `nautilus_trader.adapters`, together with the names the package exports. A clean interpreter is
   used because a test process has already loaded the venue adapters its own tests exercise, and what
   the boundary is about is what the discovery package reaches by itself.
2. Assert on the modules and on the exports. The dependency assertion passes when the adapter module
   list is empty, the export list holds more than two hundred names, and no exported name contains
   `ExecutionClient`, `OrderFactory` or `submit_order`. The export half matters because a package can
   reach no adapter and still hand out a way to submit.
3. Name the one execution client a worker is allowed. The sandbox package must export
   `SandboxExecutionClientFactory` and `SandboxExecutionClientConfig`, and must export nothing whose
   name contains `DataClient`. That asymmetry is the capability: a worker can turn an observation into
   a simulated fill inside the process, and no order leaves it.
4. Check the environment rather than the shell. The fixture reads the canonical list of adapter
   credential variables out of `scripts/strip-adapter-env.bash` instead of restating it, so the test
   cannot drift from the list the repository's own pre-flight unsets. It then removes every one of
   those names from the current process's variables and resolves the import graph again in that
   stripped environment. The assertion is that discovery still imports there and still reaches no
   adapter.
5. Refuse a candidate that names an execution path. A candidate specification is data, so it has no
   field for a venue or an adapter; a field named like one is refused as an unknown field, and a value
   shaped like an import is refused as a code reference. The refusal carries the code and the path it
   was raised against, not a message to be interpreted.
6. Read the refusals back from the record. Both refusals are written into a campaign inventory, and
   the audit reconstructs the campaign from the ledger: the candidate is reported as refused, with no
   attempts, with both refusal codes present, and with no experiment executed.

## The maths, with every symbol named

A language means a set of things, and absence is an empty set. The first assertion is written:

```text
reached  = the modules loaded by importing nautilus_trader.discovery
adapters = { module in reached : module begins with "nautilus_trader.adapters" }
```

- `reached` is the whole loaded set, read from a clean interpreter rather than from source text.
- `adapters` is the part of it that is a venue adapter, live or otherwise.
- The assertion is that `adapters` is the empty set, so adding a live adapter to the graph fails it,
  which is what makes it a regression check rather than a one-off measurement.

The export surface is checked the same way:

```text
exports    = the names in nautilus_trader.discovery.__all__
forbidden  = { name in exports : name contains "ExecutionClient", "OrderFactory" or "submit_order" }
```

- `exports` is what the package offers to a reader, and its count is asserted to be above two hundred;
  the count observed is 268, grown from 231 when this work was added.
- `forbidden` is the set that would mean the package reached an execution surface of its own.
- The assertion is that `forbidden` is empty, so the check has teeth on what the package hands out and
  not only on what it loads.

The environment assertion is set subtraction, and its size is stated:

```text
named    = the names declared in ADAPTER_ENV_VARS of scripts/strip-adapter-env.bash
stripped = { name: value in the process environment : name is not in named }
```

- `named` is the canonical list, read out of the script rather than copied into the test; it holds 92
  names, the first `AX_API_KEY` and the last `TWS_USERNAME`.
- `stripped` is the environment the boundary defines, which is the operator's environment with those
  names removed.
- The assertion is that `named` holds more than 80 names, that three of them are present by name, and
  that in `stripped` discovery imports and reaches no adapter. The count of 92 is the arithmetic, and
  the three named variables are `BINANCE_API_KEY`, `DATABENTO_API_KEY` and `EODHD_API_KEY`.

The capability assertion is one more empty set:

```text
sandbox   = the names in nautilus_trader.adapters.sandbox.__all__
data_path = { name in sandbox : name contains "DataClient" }
```

- `sandbox` holds exactly `SandboxExecutionClientConfig` and `SandboxExecutionClientFactory`.
- `data_path` is the data-client factory that a worker is deliberately not given.
- The assertion is that both execution names are present and `data_path` is empty, so a worker can
  simulate an execution and cannot fetch its own data from the same place.

## A worked example

The negative case is worked from a candidate to a recorded refusal, using the candidate the fixture
already has and changing one field at a time. Every code below is the one the schema really returns.

| Step | What is done                                                                       | What is observed                                                                           |
| ---- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| 1    | The legal candidate the fixture already uses, taken unchanged                      | it is accepted, and the accepted result carries a plan rather than a refusal list          |
| 2    | One field is changed: `data.venue` is set to the value `BINANCE`                   | refused, with exactly one pair, code `block_unknown_field`, path `data.venue`              |
| 3    | A different field is changed: `universe.module_path` is set to a venue module path | refused, with exactly one pair, code `code_reference_refused`, path `universe.module_path` |
| 4    | The unchanged candidate is compiled once more                                      | accepted, so what was refused is the names of the fields and not the shape of the form     |
| 5    | Both refusals are recorded into a campaign inventory                               | the inventory holds two refusals against one candidate identity                            |
| 6    | The audit reconstructs the campaign from the ledger                                | outcome `refused`, no attempts, both codes listed, executed experiments zero               |

The arithmetic of the refusals is a set with one member each:

```text
refused(result) = { (error.code, error.path) for error in result.errors }
refused(submission) = (("block_unknown_field",    "data.venue"),)
refused(code)       = (("code_reference_refused", "universe.module_path"),)
```

- `result.errors` is everything a compilation refused, one entry per field it turned down.
- Each pair is the closed code for the reason and the path the reason was raised against.
- The assertion is that each set holds exactly that one pair, so a field named like a venue is refused
  for being an unknown field while a value shaped like an import is refused for being a code
  reference: two different reasons, both visible in the record. The refusal is per field, which is
  why step 4's unchanged candidate still compiles and why the refusals are evidence about the fields
  rather than about the idea behind them.

## What the research actually found

This page is about machinery, so what is measured is the repository's own tests rather than a market.
Criterion 12 of the design's acceptance list reads: adding a live adapter to the discovery import graph
fails the dependency assertion, a candidate naming an execution venue is refused, and no broker
credential, live adapter or order-submission path is present in the discovery environment or the
shadow environment. Its bundle entry is `FX-M6-1` and its status is passed, carried by four tests in
the boundary fixture.

The recorded result of those four tests is what the assertions above describe: the graph resolved in a
clean interpreter holds no adapter module and no export names an execution surface; the sandbox
exports its execution client factory and config and no data client; the 92 variables of the canonical
list reach neither the stripped environment discovery imports in nor a worker's; and the two
candidates that name a venue or a module path are refused and recorded, and the refusals are read back
from the audit. The milestone that produced them also reports 271 passing tests across the discovery
unit and acceptance trees and 82 across the discovery integration tests.

The bundle as a whole holds twenty-two entries: all twenty-two pass with identities whose digests
recompute, none is failed and none is blocked. Four of them were failed until this change, and they
are recorded here as what closed them rather than as what is still absent, because the record that
reports a failure is the same record that has to report the pass replacing it:

| Criterion | What it requires                                                                                | What closed it                                                                                                                                                                                                                                                       |
| --------- | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2         | A rerun is identical, and what is not deterministic is named                                    | the engine's canonical digest recomputes from retained bytes through `CanonicalBacktestResult.digest_of_bytes`, and the run manifest is the engine's own through `BacktestNode.get_run_manifest`, retained beside the receipt so a recomputation reconciles against it |
| 4         | A signal cannot fill before its declared eligibility time, and the test that proves it can fail | the shutdown drain reaches each deferred arrival and settles everything there except a single market order that would increase the position at a price the run observed before that order existed, which stays unexecuted; the bridge path remains the one case the table names |
| 6         | Corporate-action accounting reconciles in value and in cash                                     | the corporate-action stage settles each action at its own effective instant, credits a dividend's entitlement to the positions that hold the instrument in their settlement currency, and restates a split's quantity and price basis, so the cash leg reconciles      |
| 10        | Every number recomputes                                                                         | the canonical digest recomputes from the bytes the store retains, and each receipt is reconciled against the manifest its registration retained, so no input is unavailable                                                                                            |

Criteria 2 and 10 shared the absent join, the run identity that would tie a seed, a build and a
configuration together at the engine's own boundary, and that join now exists. What the passing tests
do not show is what the design says itself: they are necessary and not sufficient, and an import-graph
check is evidence rather than proof.

## How this project relates to it

The four assertions live as tests in
[test_discovery_boundary.py](../../../python/tests/integration/test_discovery_boundary.py), because
the design states them as four tests and a production module would put test surface into production
code. The canonical credential list is `ADAPTER_ENV_VARS` in
[scripts/strip-adapter-env.bash](../../../scripts/strip-adapter-env.bash), whose own comment says to
add a new adapter variable there when one is introduced. The execution client a worker is allowed is
in [the sandbox package](../../../python/nautilus_trader/adapters/sandbox), whose
export list holds the sandbox config and factory and nothing else. The refusals are produced by the
schema's closed vocabulary in
[schema.py](../../../python/nautilus_trader/discovery/schema.py) and read back through the ledger's
audit in [ledger.py](../../../python/nautilus_trader/discovery/ledger.py).

A reader who does read code can run the four assertions with one command:

```bash
python -m pytest python/tests/integration/test_discovery_boundary.py
```

The [acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json) is where
their result is recorded against criterion 12, along with the four entries that fail.

## Where it goes wrong

- The dependency assertion is evidence, not proof, and the design says so itself: a dynamic import, an
  indirect callback or a shared execution stack can each evade a check made over module names. The
  design keeps it as a regression check and does not present it as proof that order submission is
  impossible.
- The stronger control, the shape of the worker environment as an explicit capability allowlist, is
  declared and checked in `config.py` now, which refuses an entry naming a live adapter and measures a
  worker's packages against the allowlist; what no module builds is the environment itself, so the
  control is procedural wherever it is not enforced.
- Half of the environment assertion, that a credential be refused in a policy file, is testable now:
  `config.py` loads the policy document a campaign declares, refuses a credential anywhere in it by
  variable name or by the shape of a secret without repeating the value, and checks the digest the
  declaration pins against the document it read. What a loader cannot show is the process: whether an
  adapter credential variable is set in the environment discovery runs in remains the fixture's own
  assertion, and the policy object declares no credential field at all.
- The tests cannot stop an operator from installing an adapter and running the discovery code with
  it, and they cannot make a machine with the operator's own credentials safe. If the layer is run on
  such a machine the protection is procedural, which is why the environment assertion is over the
  environment the boundary defines rather than over the shell the operator sits at; `EODHD_API_KEY` is
  set on the machine this was written on, and the fixture reports that ambient case rather than
  failing on it.
- Where a stronger isolation is wanted, syscall-level sandboxing is the mechanism, and its own
  documentation states its limits: it is not a substitute for a secure architecture, it gives no
  protection against hardware side channels, and resource exhaustion is managed by the host rather
  than by the sandbox itself.
- A passing boundary says nothing about whether any candidate is any good. It shows that the layer
  cannot send an order and has nothing to authenticate one with; whether a tested idea earns anything
  after costs is a question the account-stage measurements answer, not this one.

## Try it yourself

You need a page and a pencil, and no money and no code. The exercise is to be the visitor walking the
room.

1. Write the four assertions as four questions in your own words, one line each: what does the graph
   reach; what does the package offer; what is in the environment; what happens to a candidate that
   names a venue.
2. Write five environment variable names that look like credentials to you. An honest list might be
   `BINANCE_API_KEY`, `DATABENTO_API_KEY`, `EODHD_API_KEY`, `PATH` and `TWS_USERNAME`.
3. Mark each one that the canonical list would name. The list holds 92 names, and three of them are
   `BINANCE_API_KEY`, `DATABENTO_API_KEY` and `EODHD_API_KEY`.
4. Now suppose `EODHD_API_KEY` is set in your own shell. Write one sentence saying what the
   environment assertion does and does not show about that.
5. Take the accepted candidate and add a field called `data.venue` holding the value `BINANCE`. Write
   the code and the path the refusal would carry.

What to notice: the assertion is about the environment the boundary defines, not about the shell you
sit at, so a machine holding your own credentials is protected by procedure rather than by this test.
The second thing to notice is that a refusal names the box and not the idea, so a candidate turned away
for a field it should not have named has learned nothing about whether its hypothesis was any good.

## Where this came from

- [The design's security and execution boundary, Section
  12](../../../docs/design/strategy_discovery_design.md), which states the four tests, the capability
  boundary and what the tests cannot prove; its Section 4 invariant 9 and its criterion 12 are cited.
- [The implementation plan](../../../docs/design/strategy_discovery_implementation.md), Section 12.1,
  whose M6.2 row is the boundary fixture and whose findings name the three limits, and Section 14,
  whose matrix records the sandbox row as partial and the live data path as a platform decision.
- [The boundary fixture](../../../python/tests/integration/test_discovery_boundary.py), the four
  assertions, the clean-interpreter graph reader, the credential-list reader and the two refusal cases.
- [The canonical credential list](../../../scripts/strip-adapter-env.bash), `ADAPTER_ENV_VARS` with its
  92 names and the comment that says when to add another.
- [The sandbox package](../../../python/nautilus_trader/adapters/sandbox/__init__.py), whose export
  list is the execution client a worker is given and holds no data-client factory.
- [The acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json), the
  criterion 12 entry that passes as `FX-M6-1` and the four entries that fail with their defects named.

## Words used in this tutorial

- adapter: the piece of code a program loads in order to talk to one particular venue or data source.
- assertion: a check written as a test that runs and reports a result, rather than a claim in prose.
- capability removal: making something impossible by never installing it rather than by a switch.
- credential: the name and secret a program supplies to prove to a venue who it is.
- import graph: every module a package loads when it is imported, found by importing it and reading
  what arrived.
- refusal: one recorded reason a candidate was not accepted, written as a code, a path and a message.
- sandbox: the simulated venue that fills an intended order inside the process, with no order leaving
  it.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
