# Agents that propose candidates

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing in a market: it is the five roles that turn a written brief into a hypothesis, a candidate specification, an explanation of the evidence, a review, or the next experiment                                                                                                                                                                                                                             |
| How often it trades       | Once per invocation, whenever the campaign asks a role for a proposal or a review; a live call is made once and every later reading replays the response that was recorded                                                                                                                                                                                                                                     |
| What you need             | Nothing but this page                                                                                                                                                                                                                                                                                                                                                                                          |
| Where the rules come from | [The agent contract](../../../python/nautilus_trader/discovery/agents.py) and [the prompt type](../../../python/nautilus_trader/discovery/prompts/__init__.py)                                                                                                                                                                                                                                                 |
| The underlying research   | The prior-art table in [the design](../../../docs/design/strategy_discovery_design.md), which records that a language model proposes while deterministic code evaluates, and that a model's training window creates a look-ahead risk a design cannot detect from the returns                                                                                                                                  |
| How well it held up       | Mixed: the role declarations, the prompt identity, the refusal path and the replay are unit-tested and the invocation record is the evidence a campaign keeps, but the plan records that four of the five roles are not candidate producers, so their responses are only checked for being a JSON object, and that a live model call cannot be reproduced, which is why a campaign replays a recorded response |
| Also appears in           | [What a candidate is](../what-a-candidate-is/README.md), [Search by evolution](../search-by-evolution/README.md) and [The boundary](../the-boundary/README.md)                                                                                                                                                                                                                                                 |

## The idea in one paragraph

A campaign that searches for strategies needs ideas, and this part of the machinery gets them from a
language model instead of from a person writing each one by hand. It uses five roles, each allowed to
produce exactly one kind of thing: a testable hypothesis, a candidate written in the campaign's own
language, an explanation of evidence, a review, or the next experiment. Each role is also told what it
may not do, and the surprising part is how those limits are kept. They are not kept by the wording of
the instruction; they are kept by what each role is handed, so the analyst is given a record and cannot
write one, the reviewer is given an evidence bundle and cannot change it, and no role is given a tool
that could compute a number, reach a broker, or open the held-back data. The brief is treated as data
with its own identity, so a record can say which instructions produced an answer, and every call is
written down whether the answer was usable or not.

## Why anyone believed it

Writing candidate rules by hand is slow, and a search needs many candidates. A model can draft them
quickly, which is why the design borrows the loop that FunSearch and AlphaEvolve use: the model
proposes, and something else with no imagination and no stake decides whether the proposal is good.

The counterparty here is not another trader but the evaluator: a piece of ordinary code that runs the
backtest and the gates. For its verdict to mean anything, the proposer must not reach the evaluator's
instruments. If a role could compute the number that grades it, set the threshold it is graded against,
or peek at the held-back period, its proposal would stop being a proposal and become a claim it has
quietly made true. The design's wager is that authority can be removed from the objects a role is
given, and that this is stronger than asking the role politely not to use it.

## An everyday comparison

A hospital runs trials. One doctor writes a hypothesis: this drug should help, and here is why. A
pharmacist turns it into a prescription inside the hospital's approved drug list and dose limits. A
reviewer reads the plan and lists what could go wrong. None of these three may decide whether the drug
worked, and none holds the key to the sealed results room; that belongs to the statisticians, who are
told the plan in advance and then run it on patients. The rule keeping them apart is not a sentence on
the form but that each is handed a different set of things, and the doctor's set contains no results.

## The rules, step by step

The five roles, what each produces, and what each may not do, in the design's own words.

1. The research scientist produces a testable hypothesis with its mechanism, the features it needs and
   the strongest rival explanation; it may not name a metric value, a threshold, or a result.
2. The strategy engineer produces a candidate specification written in the campaign's declared
   language; it may not invent a feature, exceed a bound, or claim the candidate has passed anything.
3. The experiment analyst produces an explanation of the recorded evidence and a recommendation; it may
   not compute an authoritative metric, change a threshold, or decide eligibility.
4. The adversarial reviewer produces findings, each with a severity, an evidence reference and a
   reproducible test; it may not waive a gate, or authorise anything.
5. The search strategist produces the next experiment, as a bounded change or a mix of two, with its
   reason and its cost; it may not read the held-back period, change the policy, or duplicate a
   candidate.

What each role is given is the other half of its definition: a role's brief is limited to the fields
its prompt declares, and nothing else. The strategy engineer, for example, is given exactly five: the
candidate families, the features it may select by name, the parameter bounds, the declared value sets,
and a statement of its information set. The prompt refuses to render if a field the template does not
name is supplied, or if a declared field is missing, which stops a metric, a threshold or a gate
condition from being smuggled into a brief that may not contain one.

One invocation then runs in a fixed order.

1. The brief is rendered from the fields the role's prompt declares. A missing declared field or an
   extra undeclared one is refused here, before any model is called.
2. The model is reached through an injected callable. The repository ships no model dependency, and no
   credential is read by this part of the code.
3. The response is read as JSON. Anything that is not a JSON object is recorded as unstructured.
4. If the response declares `outcome` equal to `insufficient_data`, it is recorded as insufficient
   data, with the reason it gives, and the campaign continues.
5. A role that proposes no candidate is accepted once its response is a JSON object.
6. The strategy engineer is the one candidate producer: its proposal is read from the key
   `specification` and passed to the language's own validator, and a missing key or a refused proposal
   is a schema violation carrying the validator's reasons, while an accepted one is accepted.
7. The invocation is recorded in every case. A refusal is a result, not an error.

## The maths, with every symbol named

There is little arithmetic here. What there is, is identity: short strings that stand for other
things, so a record can be checked without storing the whole brief or the whole answer.

```text
prompt_digest   = hash of ( prompt_id, version, fields, template )
input_digest    = hash of ( role, prompt_id, prompt_version, prompt_digest, instructions )
response_digest = hash of ( response )
```

- `hash` is a deterministic function that turns a description into a short fixed string. The same
  description always gives the same string, and any change gives a different one.
- `prompt_id`, `version`, `fields` and `template` are the prompt's name, its version, the field names
  it declares, and its instruction text. Digesting them together means a change to the wording or the
  version is a different prompt, not the same one edited.
- `role` is which of the five roles is being invoked; `instructions` is the template with its declared
  fields filled in, which is what a replay is keyed by, so two briefs differing by one sentence are two
  different invocations.
- `response` is the text the model returned, digested as it arrived.

A second small rule labels the period a result covers against the model's training window.

```text
if knowledge_cutoff is not a well-formed date, or period_start is not: UNKNOWN
else if period_start > knowledge_cutoff: POST_CUTOFF
else: VULNERABLE
```

- `knowledge_cutoff` is the date the model's training data is taken to end, when one is known, and
  `period_start` is the earliest date of the market period the resulting candidate is measured over.
- The comparison is made on the text of the two dates, which is only sound because both are checked to
  be the same length and zero-padded; a value that is not is labelled unknown rather than compared.

The label is a standing classification: a result over a period the model could plausibly already know
is marked as more vulnerable to the model having seen the answer in advance, and nothing turns a
vulnerable period into a clean one.

## A worked example

One brief is sent to the strategy engineer four times, so its `input_digest` is the same and a replay
would key on it. The four responses are the four things a single answer can be, and each is recorded as
its own outcome. The values are made up but of the kind the campaign would send. The brief names five
fields: families `cross_sectional_rank, absolute_threshold`; features `ema_trend`; bounds
`exit.holding_period_cap in [1, 252]`; declared sets `selection_modes: rank, threshold`; information set
`the policy's bounds and declared sets; no results`.

| Response sent                                                   | How it was read                             | Recorded outcome    |
| --------------------------------------------------------------- | ------------------------------------------- | ------------------- |
| `The trend should continue, so buy the strongest names.`        | not a JSON object                           | `unstructured`      |
| `{"specification": {...}}` with family `invented_family`        | the language's validator refused the family | `schema_violation`  |
| `{"outcome": "insufficient_data", "reason": "no bound fits"}`   | declared its information was not enough     | `insufficient_data` |
| `{"specification": {...}}` with a declared family and in-bounds | the language's validator accepted it        | `accepted`          |

The accepted invocation leaves this record, which is what the campaign keeps. The digests are shown as
`...` rather than as their real values.

```text
{
  "schema": "discovery.agents/v1",
  "invocation": {"role": "strategy_engineer", "prompt_id": "strategy_engineer",
                 "prompt_version": "1", "input_digest": "..."},
  "context": {"model_identifier": "model.fixture/v1", "knowledge_cutoff": "2023-06-01",
              "period_start": "2024-03-01", "contamination": "post_cutoff"},
  "response_digest": "...",
  "validation_outcome": "accepted"
}
```

Read the record plainly. It names the model and version, the prompt and its version, the digest of the
brief, the digest of the answer, and what validating the answer produced. The context carries the
sampling configuration, the date the call ran, and the period the candidate is measured over, whose
start is later than the cutoff, so the period is labelled post-cutoff. The other three rows differ only
in their outcome and, for two of them, in the reasons the language gave; each still produces a record.
A fifth invocation with the same brief and a recorded response returns it byte for byte, which is what
keeps the campaign reproducible after the provider moves on.

## What the research actually found

This is machinery built from a design and a plan, so "the research" is the prior art the design cites
and the findings its own tests recorded. The design's prior-art table judges four claims. That a
language model proposes while deterministic code evaluates, with an external layer deciding, is
established, because FunSearch and AlphaEvolve implement the loop around an evaluator. That such a loop
works only where an automatic evaluator exists is established and stated by those authors themselves,
which is why this design leans on the engine and the gates rather than on an agent's judgement. That a
constrained language beats free code generation is unproven, and the design keeps the constraint for
safety and auditability rather than because anyone measured a gain. That published agent systems
demonstrate live profitability is not supported: the cited preprint reports numbers with no costs and
no multiple-testing control, one independent re-run fails to beat buy-and-hold without cherry-picked
settings, and a later position paper argues that reported gains are not evidence of deployment
readiness.

The design also records, as established, that a model's training window creates a look-ahead risk in a
historical study: independent work measures the leak, shows that asking the model to forget is only a
partial mitigation, and shows apparent returns falling away once the period passes beyond what the
model knows. That is why the contamination label exists and why it is a label rather than a fix.

What the tests found is what the grade rests on. The five roles declare five prompts, five field sets
and five distinct statements of what each produces and may not do, and rendering refuses any field
outside the declared set. A prompt's digest changes with its template and its version, and a replayed
invocation with no recorded response raises rather than inventing one. The integration test sends one
brief four times and gets an unstructured response, a schema violation, an insufficient-data reply and
an accepted proposal, with the campaign continuing after each. Against that, the plan names two limits
it carries rather than repairs: four of the five roles are not candidate producers, so their answers
are checked only for being a JSON object, and agents cannot be reproduced by recall, so a campaign
replays a recorded response, which is why the invocation record is evidence and a live call is not.

## How this project relates to it

The whole contract is one module, [the agent contract](../../../python/nautilus_trader/discovery/agents.py).
It holds the five roles and their authority tables, the outcome set, the contamination classifier, the
context and request objects, the replay model, the invocation record, and the one function that ties
them together. A reader who does read code can run a single invocation by hand:

```py
invoke(model, request, context=context, language=Language(policy=policy, registry=registry))
```

The prompts are separate small modules under
[python/nautilus_trader/discovery/prompts](../../../python/nautilus_trader/discovery/prompts/), one per
role, such as
[the strategy engineer's](../../../python/nautilus_trader/discovery/prompts/strategy_engineer.py),
which carries the field list, the bounds it repeats into the brief, and the instruction to reply with
one JSON object or an insufficient-data reply. The prompt type itself lives in
[the prompts package](../../../python/nautilus_trader/discovery/prompts).

The tests in [test_agents.py](../../../python/tests/unit/discovery/test_agents.py) check the role
declarations, the render refusals, the digest tracking, the classifier, the replay and each outcome;
[test_discovery_search.py](../../../python/tests/integration/test_discovery_search.py) sends one brief
through all four outcomes and records each against the campaign.

The environment claim behind the role limits is checked elsewhere: the repository names its adapter
credential variables in one list, `scripts/strip-adapter-env.bash`, and
[the boundary page](../the-boundary/README.md) states the assertion that discovery reads none of them.
Here it matters only as a limit: see below.

## Where it goes wrong

- Four of the five roles are checked only for producing a JSON object. Their limit is the field set
  they are given, not the content of what they write, so a scientist or an analyst can still write
  something unhelpful inside a well-formed reply. That is a named limit the plan carries.
- A live model call cannot be reproduced. Sampling, version and provider all move, so the only
  reproducible object is the recorded response; a campaign that loses its replay store loses the
  ability to re-run its proposals, even though it can still re-run the candidates it compiled.
- The contamination label does not repair anything. A candidate measured over a period the model may
  already know is marked vulnerable and stays vulnerable, and no gate converts the first into the second.
- The environment assertion is procedural today, not enforced by configuration: the plan records that
  the stronger control, the shape of the worker environment, is not built because the configuration
  module is unbuilt, so a machine with live credentials is protected by procedure alone.
- A brief that leaked a field would defeat the separation. The prompts refuse an undeclared field for
  exactly this reason, so the render refusal is load-bearing rather than tidy; a reader who edits a
  template to accept one more field has changed the role's authority, not just its wording.
- The roles decide nothing. They propose a hypothesis, a candidate, an explanation, a review or the
  next experiment, and the campaign's gates and thresholds sit outside all of them. What the roles
  cannot do to a number is anything at all: no role computes a performance number, sets a threshold,
  reads the confirmation set, or holds a credential, so a proposal never carries its own score.

## Try it yourself

This is a paper exercise, and it needs nothing but the five roles above.

1. Write down five candidate tasks: invent a reason a price might drift; write a rule in a fixed
   shape; explain why a past result appeared; find a flaw in a plan; choose what to try next.
2. For each task, name the role that could own it and one thing that role may not do.
3. Look at the field list the strategy engineer is given and ask, for each task, whether the brief would
   need a field that is not on the list. If it would, the role as defined cannot do the task, and that
   is the point.
4. Take the sentence `the candidate should return about twenty percent a year, so buy it` and ask
   which of the five roles could have written it. Work through the list; none may, and noticing which
   rule blocks each one is the exercise.

What to notice: the limits are not polite requests a clever answer can talk its way around. They are
the shape of the information each role is handed, so a role never given a metric, a threshold or the
held-back data has nothing to route around. The separation is made of missing objects, not wording.

## Where this came from

- [The agent contract](../../../python/nautilus_trader/discovery/agents.py): the five roles and their
  authority tables, the four outcomes, the classifier, the request, context, replay and record objects.
- [The prompt type](../../../python/nautilus_trader/discovery/prompts/__init__.py): the prompt as data with a digest, and the render refusal for a missing or undeclared field.
- [The strategy engineer's prompt](../../../python/nautilus_trader/discovery/prompts/strategy_engineer.py): the declared field set and the bounds and value sets it repeats into the brief.
- The design, [strategy_discovery_design.md](../../../docs/design/strategy_discovery_design.md),
  Section 10.1 (roles and authorities, and the sentence that the separation is enforced by the objects
  each role is given rather than by prompt wording), Section 10.2 (the agent contract, the recorded
  fields, the replay justification and the contamination classification), and Section 12 (capability
  removal and the four boundary tests).
- The plan, [strategy_discovery_implementation.md](../../../docs/design/strategy_discovery_implementation.md),
  Section 11, which names the two carried limits (four of the five roles are not candidate producers,
  and agents cannot be reproduced by recall) and Section 12.1, which records that the worker allowlist
  is procedural today.
- [The agent unit tests](../../../python/tests/unit/discovery/test_agents.py) and
  [the search integration test](../../../python/tests/integration/test_discovery_search.py), which
  exercise the role declarations, the render refusals, the digest tracking, the classifier, the replay
  and each of the four recorded outcomes.
- [The canonical credential list](../../../scripts/strip-adapter-env.bash), the one place the
  repository names the adapter environment variables whose absence the boundary asserts.

## Words used in this tutorial

- invocation: one call of one role with one rendered brief, recorded whether or not the answer was
  usable.
- prompt: the versioned instructions a role is asked with, together with the fields it declares,
  digested into a single identity.
- insufficient data: the recorded outcome of a role that states the information it was given is not
  enough to answer, which is a complete result rather than a refusal.
- refusal: a recorded outcome that is not an accepted proposal; here either a response that was not a
  JSON object or one the schema did not accept.
- replay: answering an invocation from a previously recorded response rather than by calling the model
  again.
- contamination: the standing label on a result whose period the model may plausibly have seen, where
  the knowledge cutoff is the date its training data is taken to end.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
