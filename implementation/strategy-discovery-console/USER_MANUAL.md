# Strategy Discovery Console: user manual

For a reader with no technical or financial background. Every term is explained where it first
appears. If you only read one section, read section 11, the cheat sheet.

---

## 1. What this app does, in plain words

A trading system can test an idea about the market: "buy the strongest shares and hold them for a
week, then look again". Testing thousands of such ideas is cheap on a computer, and that is the
trouble. If you test enough ideas, one of them will look brilliant by luck alone, and if you only
write down the winner you will never know whether you found something or misled yourself.

This repository contains a layer built to prevent exactly that. It proposes candidate ideas, tests
them through a simulation, and - this is the unusual part - keeps a written record of the whole
search rather than of the winner. Whoever reads the record later can see how many ideas were tried,
which ones were turned away and why, and whether the numbers were corrected for the fact that so many
were tried.

This app shows you that layer, layer by layer, in a web browser. You can change a number, plant a
deliberate mistake, and watch the layer refuse, count, record or read back what it did.

Think of it as a guided tour behind the counter of a research process, not as a tool that picks
shares.

### What you need

- A web browser (Chrome, Edge, Firefox or Safari).
- Nothing else, to start. There is no data to download and no account to open.

## 2. What this app does not do

| It does not...                            | Because...                                                                                                                         |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Buy or sell anything                      | There is no broker connection and no credential anywhere in it                                                                     |
| Predict the market                        | It models rules about proposing, checking and recording ideas, not prices                                                          |
| Tell you whether any strategy is good     | It shows what the layer accepts and refuses; judging an idea is not its job                                                        |
| Measure a real search's performance       | Its campaign is one made-up path through the rules, run on numbers you can see and change                                          |
| Connect to anything over the network      | Every number is computed in the page, and nothing is downloaded                                                                    |
| Call an artificial-intelligence model     | The page about proposing agents replays answers that were recorded earlier, and says so                                            |
| Produce the repository's own fingerprints | The repository uses a different fingerprint method (see section 9); this app uses its own and labels it clearly                    |
| Hide the parts that do not work           | Four of the repository's twenty-two acceptance checks do not pass, and every one of them is shown as failing, with what is missing |

If you are looking for a tool that tells you what to trade, this is the wrong app. If you want to see
how a careful research process protects itself from its own enthusiasm, this is exactly the right one.

## 3. The one idea you need

Everything in this app follows from a single arrangement: **one campaign, with a record of everything
it did.**

A campaign is a single search. It declares - in advance - the thresholds it will be judged by: how
much data it may look at, which parts are kept back for a final check, how many ideas it is allowed
to try before the statistics stop being believable. It then proposes candidates, tests some of them,
turns some away, and writes down every one of those events.

Three consequences run through the whole app:

1. **A candidate is data, not code.** An idea is written as a block of names and numbers. The machine
   can read it, copy it, fingerprint it and refuse it without ever running it. That is what makes a
   search safe to run unattended.
2. **The search may propose anything and decide nothing.** The thresholds, the kept-back data and the
   risk limits are inputs to the campaign, set outside it. The search cannot loosen them.
3. **Every claim has a fingerprint.** What was written down, what ran, what the run produced and what
   the record says each have their own identity, computed from their content. Two readers compare
   fingerprints instead of trusting a number.

A fourth idea is the reason the app exists at all: **the measurement is built to refuse.** A candidate
that cannot be expressed in the language is refused and recorded as a refusal, not run and reported
as a bad experiment.

## 4. How to run it

The app is a folder with no dependencies. From the folder
`implementation/strategy-discovery-console`:

```bash
bun install      # nothing to install; there are no dependencies
bun run build    # builds the page's script
bun test         # checks the machinery
bun run verify   # checks that every borrowed fact is still where it says it is
```

Then open `index.html` in your browser, or run `bun run serve` and open `http://127.0.0.1:8791`.

If the page says the script is missing, run `bun run build` and reload.

## 5. How to read any page

Every one of the twelve layer pages has the same five parts, in the same order:

| Part                 | What it is for                                                                                     |
| -------------------- | -------------------------------------------------------------------------------------------------- |
| The idea             | The tutorial's own paragraph, quoted, with the grade that page carries and what the grade rests on |
| The model            | The part you can drive: change something, see what the layer decides                               |
| What it refuses      | The exact words the layer uses when it says no, and the condition that produces each one           |
| Where it goes wrong  | The tutorial's own list of the limits, the unbuilt parts and the known gaps                        |
| Where this came from | File paths into the repository, so you can check any claim yourself                                |

A **grade** is the tutorial collection's own verdict on how well that part of the work held up. The
grades in this app are "Strong as a definition" (the rules are unit-tested against known answers) and
"Mixed" (something about it is procedural, unbuilt, or does not pass).

## 6. The twelve layer pages

The pages follow the order the work happens in.

### 1. The rules nobody may choose

The thresholds, the allowed words, the numeric bounds and the references a candidate may cite, all
declared before any search begins.

- **Fingerprint**: the identity of the frozen declaration, so two results can be compared.
- **Bound**: change a numeric range and watch the declaration refuse a bad one.
- **Declared set**: add or remove a member of a closed list of allowed words.
- **Field**: ask how one value in a candidate resolves against the policy, and see where the answer
  comes from.
- Buttons: *Reset the declarations*, *Add the member*, *Remove the member*, *Show it*.

### 2. What a candidate is

The twelve blocks a candidate must have, the words each block may use, and the two caps that count
complexity.

- **the one thing broken**: plant one defect and see which stage of the reading refuses it.
- The other controls describe the candidate block by block (family, minimum price, declared features,
  selection mode, entry conditions, exit, portfolio, rebalance, benchmark).
- You will see the pipeline run in order and stop where the candidate is invalid.

### 3. What ran against what was written down

The written record: each event in order, each with its own fingerprint, and each carrying the
fingerprint of the event before it.

- **Kind / Candidate / Job key**: append an event of your choosing.
- **Checkpoint, held outside the writer**: the earlier fingerprint you are checking against.
- **Tamper with entry number**: change an earlier event and watch every fingerprint after it change,
  which is how a removed or reordered record becomes visible.
- Buttons: *Append*, *Verify*, *Tamper*, *Restore the opening chain*, and three *Demonstrate* buttons
  for the three refusals the write path has.

### 4. The one executor

The single object the engine drives. It decides once per price bar, and only on the bars that begin a
new holding period does it choose what to hold and how much.

- **Ranking length, Portfolio size, Position cap, Weighting, Cash policy**: change the allocation and
  watch the weights, the capped weights and the instruction strengths move.
- The page states the known timing gap: the bridge stamps an instruction and submits it immediately,
  so a fill can arrive earlier than the stamp.

### 5. Counting the search

The four numbers a campaign keeps - proposals, distinct configurations, machine runs, and attempts
admitted into a comparison - and the correction that needs them.

- Buttons: *+ proposal*, *+ new configuration*, *+ equivalent configuration*, *+ execution*,
  *+ trial (new key)*, *+ trial (replay the last key)* and *reset*. Add one event at a time and
  watch the four numbers move independently.
- **dependence** and **declared effective count**: say whether the attempts are independent; a
  dependent campaign that declares nothing is refused rather than given a default.

### 6. Two kinds of held-back data

Data is kept back in roles: some may be looked at freely, one role may be read only for a final check,
and one role is rationed to a campaign.

- **Lineage**: the line of descent a reservation belongs to.
- **Data role**: which role is being asked for.
- Button: *Reserve holdout data*. Ask twice for the same lineage and the second ask is refused; the
  page lists the reservation's states and the exact refusals.

### 7. The pre-registered gate chain and the pre-campaign admission test

Six questions asked in one order. The reading stops at the first "no", and a later question that would
have passed never excuses an earlier failure; a question never reached is recorded as never asked.

- **planted defect**: break exactly one condition and watch where the chain stops.
- The admission controls are the pre-campaign arithmetic: observations available, kept-back share,
  configurations planned, the effect you hope to detect, and how much the trial results vary. The
  answer is *admit*, *narrow the search to N* or *refuse*, shown with its arithmetic.

### 8. Searching for strategies by breeding candidates

The search treats a candidate as a set of named positions, each holding one value, and breeds new
candidates by changing values and exchanging blocks between two parents of the same family.

- **seed** and **generation**: the same pair always draws the same children.
- **feature declarations** and **condition leaves**: how many of each a child may have.
- The page keeps a *front*: the candidates that nothing else beats on every measurement at once, with
  the columns you have chosen. A front is a set of options, not a winner.

### 9. Agents that propose candidates

Five roles - one asks a research question, one proposes a candidate, one explains evidence, one
reviews, one suggests the next experiment - and what happens to the answer each gives.

- **knowledge cutoff** and **period start**: whether an answer used knowledge it could not have had.
- The page sends four recorded answers through the classifier and shows the outcome for each:
  unstructured, schema violation, insufficient data, accepted.
- Buttons: *replay the accepted response*, *replay an invocation that was never recorded*.

### 10. The shadow period

A candidate that has passed everything is frozen and then watched against a live feed, with no
authority to trade, to see whether it behaves as its run said it would.

- **the shadow run declares**: which execution eligibility the shadow period declares; change it and
  the comparison with the run stops holding, because the rule compares values and not names.
- **declared minimum observations**: how many observations the period promised before it started.
- The page can show a fill being refused for being eligible before its own signal.

### 11. Auditing a campaign from the ledger alone

The reading that puts the whole search back: which candidates existed, which ran, which were judged,
under which policy, and why each was turned away. It writes nothing and trusts no counter.

- **the declared policy digest** and **the digest the inventory was opened with**: make the two differ
  and the reading says it describes a campaign under a different policy.
- **the engine version the reader holds**: the version the reading is being made by.
- **ask the reading to name a candidate**: name a candidate the compiled record holds and see it read
  back.

### 12. The boundary

Why the layer cannot send an order: not because a switch is off, but because the pieces were never
installed. Four assertions make that absence observable.

- **Reach a module from the graph**: add a module to the import graph and watch the check report it.
- **Offer a name from the package**: add an export and see whether it is part of the allowed surface.
- **The role**: which part of the system is asking for authority.
- **A variable in the environment discovery runs in**: add a credential name and watch the boundary
  report it rather than run.
- Two fields are venue-shaped: declare them and see the refusal the schema produces.

## 7. Three more pages

### One campaign, end to end

The only page that is not a layer. It runs the whole pipeline for the inputs you choose - campaign
name, seed, a planted defect, how many configurations you plan to try, the effect you hope to detect,
the kept-back share, the position cap and the declared result - and shows what each layer decided, in
order, with the numbers behind it. When the gate chain stops, the stages that depend on a judged
candidate are reported as **not reached** rather than run.

Two things on this page are declared rather than measured, and it says so: the evidence the gates read
is the worked example from the gate tutorial with this campaign's own identity in it, and the
development results the search reads are declared by the campaign rather than computed.

### The acceptance record

All twenty-two acceptance checks the repository keeps, with the status of each, the test or fixture
that exercises it, and what was observed. Eighteen pass, four do not, none is blocked. The four are
listed separately with what is missing.

### What this console is

The front page: what is modelled, what is not, the twelve layers with their grades, the four failing
checks, and how to read a page.

## 8. The four checks that do not pass

| Check | What it should establish                                              | What is missing                                                                                        |
| ----- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| 2     | That a run's identity joins the seed, the build and its configuration | The engine's own run record has no binding in the Python side yet                                      |
| 4     | That an instruction waits for its stamped instant                     | The bridge resolves eligibility and defers nothing, so the engine's latency model carries the contract |
| 6     | That a corporate action credits what a held position was owed         | The stage adjusts prices and does not credit the entitlement                                           |
| 10    | That a rerun can be shown identical from the record alone             | The engine's own fingerprint cannot be recomputed from the retained bytes                              |

Two of the four need the same missing piece, which is why they are usually discussed together. None of
them is a fault in the layer this app is about; each needs a change at the engine's boundary, and the
app reports the failure rather than a workaround.

Every module the plan's layout names now exists. The command front end is `discovery/cli.py`, and it
does four of the things these pages describe: it reads a configuration file, it names the campaign and
the rules it runs under, it refuses a credential found in a policy file, and it checks that the policy
file is the one that was pre-registered. It reports what it read as one JSON document and runs
nothing. What it does not do is build the worker environment the allowlist describes, so where a page
shows a control that is procedure rather than enforcement, the app says which half is which.

## 9. A note on the fingerprints

Every number, document or record can have a **fingerprint**: a short string computed from its content,
the same for the same content and different for any change. The repository's fingerprint method is
called blake3. A page opened from a file cannot use it without extra software, and this app has no
dependencies, so the app carries its own method - called SHA-256 - and every fingerprint it prints is
labelled `sha256:`.

Two consequences, stated on the pages that show a fingerprint:

- A fingerprint printed here is **not** comparable with one recorded in the repository.
- SHA-256 itself is checked in the app's tests against the published test vectors for that method, so
  the fingerprints are the real thing even though they are not the repository's.

## 10. The words this app uses

| Word                         | What it means here                                                                                                   |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Campaign                     | One search: its declared settings, its candidates, its runs and its record                                           |
| Candidate                    | A proposed strategy, written as data rather than as code                                                             |
| Block                        | One of the twelve sections a candidate is made of                                                                    |
| Specification                | The candidate as written down, before anything is built from it                                                      |
| Cell / gene                  | One named position in a candidate, holding one value that a search may change                                        |
| Policy                       | The declarations a campaign is judged under: allowed words, numeric bounds, references                               |
| Freeze                       | To fix a policy's content and compute its fingerprint, so later changes are visible as a different policy            |
| Ledger                       | The written record of everything a campaign did                                                                      |
| Chain                        | The rule that each record carries the fingerprint of the one before it, so a removal or reordering shows             |
| Digest / fingerprint         | A short string computed from content; the same content gives the same string                                         |
| Identity                     | Which of several fingerprints is meant: what was written, what ran, what the run produced, or what the record kept   |
| Refusal                      | The layer saying no, with a code and a reason; a refusal is recorded rather than thrown away                         |
| Gate                         | One of six questions asked before a candidate may be believed, in a fixed order                                      |
| Holdout / held-back data     | Data reserved from the search, read once for a final check                                                           |
| Lineage                      | The line of descent a holdout reservation belongs to                                                                 |
| Trial                        | An attempt that counts towards the statistics, as distinct from a proposal, a configuration or a machine run         |
| Deflated Sharpe ratio        | A Sharpe ratio (a reward-per-risk number) lowered to account for how many ideas were tried                           |
| Correction                   | The adjustment that accounts for trying many ideas at once                                                           |
| Overfitting probability      | A diagnostic that estimates how likely it is that the best result came from luck; it is reported and rejects nothing |
| Pareto front                 | The candidates that nothing else beats on every measurement at once: a set of options, not a winner                  |
| Seed                         | The starting number that makes a random-looking search reproducible                                                  |
| Shadow period                | Watching a frozen candidate against a live feed, with no authority to trade                                          |
| Interpreter                  | The one object the engine drives that turns a built candidate into decisions and instructions                        |
| Decision bar / rebalance bar | The two kinds of price bar the executor treats differently: it decides on both, and only rebalances on the second    |
| Audit                        | Reading the record back into what the campaign contained, without adding anything to it                              |
| Boundary                     | The set of things the layer cannot do because the pieces were never installed                                        |
| Acceptance criterion         | One of twenty-two checks the repository records as evidence that the work does what it claims                        |
| Grade                        | The tutorial collection's verdict on one layer, with a clause saying what the grade rests on                         |

## 11. The cheat sheet

- **Just want to see it work?** Open the page, leave everything alone, and look at **One campaign, end
  to end**. Thirteen stages, in order, with the numbers behind each.
- **Want to see it refuse?** On the same page, change **planted defect** to a gate. The chain stops at
  that gate, says which questions were never asked, and the shadow stage is reported as not reached.
- **Want to see the record defend itself?** On **What ran against what was written down**, press
  *Tamper* on an early entry and then *Verify*.
- **Want to see why a threshold cannot be chosen by the search?** **The rules nobody may choose**.
- **Want to see the arithmetic that stops an over-large search?** **The pre-registered gate chain**,
  the lower half.
- **Want the honest part?** **The acceptance record**, and the *Where it goes wrong* panel on every
  page.
- **Nothing is a recommendation.** No page of this app tells you what to trade, and no number in it is
  a measurement of a real market.
