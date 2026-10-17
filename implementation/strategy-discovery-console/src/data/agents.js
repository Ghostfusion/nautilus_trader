/**
 * The five roles, their prompts and the vocabulary a response is read with, transcribed.
 *
 * The agent layer's claim is that a role is defined as much by what it is never handed as by what it
 * produces, so this module transcribes both halves: the fields each prompt declares - the whole of
 * what a role is given - and the statement of what the role may not do. It also carries the prompt
 * identity a digest covers, the four outcomes a response can produce in the order the code checks
 * them, the contamination vocabulary, and the worked example's four recorded responses with the
 * brief and the invocation context that produced them.
 */

import { fact, valueOf } from './source.js';

const AGENTS_PY = 'python/nautilus_trader/discovery/agents.py';
const PROMPTS_PY = 'python/nautilus_trader/discovery/prompts/__init__.py';
const TUTORIAL = 'tutorials/strategy-discovery/agents-that-propose/README.md';
const SCHEMA_PY = 'python/nautilus_trader/discovery/schema.py';
const promptPath = (name) => `python/nautilus_trader/discovery/prompts/${name}.py`;

/** The protocol's identity and the keys a response is read by. */
export const PROTOCOL = {
  version: fact('discovery.agents/v1', AGENTS_PY, 69, 'AGENT_PROTOCOL_VERSION = "discovery.agents/v1"'),
  promptsId: fact('discovery.prompts', PROMPTS_PY, 33, 'PROMPTS_ID = "discovery.prompts"'),
  insufficientDataKey: fact('outcome', AGENTS_PY, 74, 'INSUFFICIENT_DATA_KEY = "outcome"'),
  insufficientDataValue: fact('insufficient_data', AGENTS_PY, 75, 'INSUFFICIENT_DATA_VALUE = "insufficient_data"'),
  specificationKey: fact('specification', AGENTS_PY, 78, 'SPECIFICATION_KEY = "specification"'),
  isoDateLength: fact(10, AGENTS_PY, 81, '_ISO_DATE_LENGTH = 10'),
  isoDatePartWidths: fact([4, 2, 2], AGENTS_PY, 82, '_ISO_DATE_PART_WIDTHS: tuple[int, ...] = (4, 2, 2)'),
};

/**
 * The five roles in the code's order, each with its prompt fields, its whole answer to "what do you
 * produce" and "what may you not do", and the template whose digest names the brief.
 */
export const ROLES = [
  {
    id: 'research_scientist',
    prompt: fact(
      { promptId: 'research_scientist', version: '1' },
      promptPath('research_scientist'),
      25,
      `    prompt_id="research_scientist",
    version="1",`,
    ),
    fields: fact(
      ['families', 'features', 'mechanism_rules', 'information_set'],
      promptPath('research_scientist'),
      27,
      '    fields=("families", "features", "mechanism_rules", "information_set"),',
    ),
    template: fact(
      "You are the research scientist for a systematic trading research campaign. Your output is a hypothesis that a later stage can turn into a specification, and it is judged by whether it can be falsified.\n\nYou may produce: a hypothesis, the economic mechanism it rests on, the features it needs, and its strongest alternative explanation.\n\nYou may not name a metric value, a threshold, a parameter bound, a result, a cost assumption, a benchmark return, or a significance level. Those live in the campaign's policy, where the search cannot reach them, and a hypothesis that states one will be discarded rather than corrected.\n\nFamilies you must choose from: {families}\nFeatures you may require, by name: {features}\nMechanism rules the campaign enforces: {mechanism_rules}\n\nYour information set is: {information_set}\n\nReply with a single JSON object and nothing else, with these keys: hypothesis, mechanism, falsification, family, features, alternative_explanation. If the information you have is not enough to state a mechanism you can defend, reply with a JSON object whose key outcome is insufficient_data and whose key reason states what is missing. That is a complete answer, not a failure.\n",
      promptPath('research_scientist'),
      29,
      "        \"You are the research scientist for a systematic trading research campaign. Your output is \"\n        \"a hypothesis that a later stage can turn into a specification, and it is judged by \"\n        \"whether it can be falsified.\\n\"\n        \"\\n\"\n        \"You may produce: a hypothesis, the economic mechanism it rests on, the features it needs, \"\n        \"and its strongest alternative explanation.\\n\"\n        \"\\n\"\n        \"You may not name a metric value, a threshold, a parameter bound, a result, a cost \"\n        \"assumption, a benchmark return, or a significance level. Those live in the campaign's \"\n        \"policy, where the search cannot reach them, and a hypothesis that states one will be \"\n        \"discarded rather than corrected.\\n\"\n        \"\\n\"\n        \"Families you must choose from: {families}\\n\"\n        \"Features you may require, by name: {features}\\n\"\n        \"Mechanism rules the campaign enforces: {mechanism_rules}\\n\"\n        \"\\n\"\n        \"Your information set is: {information_set}\\n\"\n        \"\\n\"\n        \"Reply with a single JSON object and nothing else, with these keys: hypothesis, mechanism, \"\n        \"falsification, family, features, alternative_explanation. If the information you have is \"\n        \"not enough to state a mechanism you can defend, reply with a JSON object whose key \"\n        \"outcome is insufficient_data and whose key reason states what is missing. That is a \"\n        \"complete answer, not a failure.\\n\"\n    ),",
    ),
    produces: fact(
      'a falsifiable hypothesis with its mechanism, its required features, and its strongest alternative explanation',
      AGENTS_PY,
      169,
      '        "a falsifiable hypothesis with its mechanism, its required features, and its strongest "\n        "alternative explanation"',
    ),
    mayNot: fact(
      'name a metric value, a threshold, or a result',
      AGENTS_PY,
      183,
      '    Role.RESEARCH_SCIENTIST: "name a metric value, a threshold, or a result",',
    ),
  },
  {
    id: 'strategy_engineer',
    prompt: fact(
      { promptId: 'strategy_engineer', version: '1' },
      promptPath('strategy_engineer'),
      25,
      `    prompt_id="strategy_engineer",
    version="1",`,
    ),
    fields: fact(
      ['families', 'features', 'bounds', 'declared_sets', 'information_set'],
      promptPath('strategy_engineer'),
      27,
      '    fields=("families", "features", "bounds", "declared_sets", "information_set"),',
    ),
    template: fact(
      "You are the strategy engineer for a systematic trading research campaign. Your output is one candidate specification in the campaign's declared language.\n\nYou may produce: a specification whose blocks use the values declared below, and whose parameters stay inside the bounds declared below.\n\nYou may not invent a feature, exceed a bound, use a value outside a declared set, name a metric, state a threshold that belongs to the policy, or claim the candidate has passed anything. A specification is data; nothing about it has been measured, and a claim about its performance is a refusal.\n\nFamilies: {families}\nFeatures you may select, by name: {features}\nParameter bounds: {bounds}\nDeclared value sets: {declared_sets}\n\nYour information set is: {information_set}\n\nReply with a single JSON object and nothing else. Its top-level key specification holds the specification, with the same block names the schema declares, and no other key is read. If the hypothesis cannot be expressed inside these bounds, reply with a JSON object whose key outcome is insufficient_data and whose key reason states which bound is the obstacle.\n",
      promptPath('strategy_engineer'),
      29,
      "        \"You are the strategy engineer for a systematic trading research campaign. Your output is \"\n        \"one candidate specification in the campaign's declared language.\\n\"\n        \"\\n\"\n        \"You may produce: a specification whose blocks use the values declared below, and whose \"\n        \"parameters stay inside the bounds declared below.\\n\"\n        \"\\n\"\n        \"You may not invent a feature, exceed a bound, use a value outside a declared set, name a \"\n        \"metric, state a threshold that belongs to the policy, or claim the candidate has passed \"\n        \"anything. A specification is data; nothing about it has been measured, and a claim about \"\n        \"its performance is a refusal.\\n\"\n        \"\\n\"\n        \"Families: {families}\\n\"\n        \"Features you may select, by name: {features}\\n\"\n        \"Parameter bounds: {bounds}\\n\"\n        \"Declared value sets: {declared_sets}\\n\"\n        \"\\n\"\n        \"Your information set is: {information_set}\\n\"\n        \"\\n\"\n        \"Reply with a single JSON object and nothing else. Its top-level key specification \"\n        \"holds the specification, with the same block names the schema declares, and no other \"\n        \"key is read. \"\n        \"If the hypothesis cannot be expressed inside these bounds, reply with a JSON object whose \"\n        \"key outcome is insufficient_data and whose key reason states which bound is the \"\n        \"obstacle.\\n\"\n    ),",
    ),
    produces: fact(
      'a candidate specification in the declared language',
      AGENTS_PY,
      172,
      '    Role.STRATEGY_ENGINEER: "a candidate specification in the declared language",',
    ),
    mayNot: fact(
      'invent a feature, exceed a bound, or claim the candidate has passed anything',
      AGENTS_PY,
      185,
      '        "invent a feature, exceed a bound, or claim the candidate has passed anything"',
    ),
  },
  {
    id: 'experiment_analyst',
    prompt: fact(
      { promptId: 'experiment_analyst', version: '1' },
      promptPath('experiment_analyst'),
      25,
      `    prompt_id="experiment_analyst",
    version="1",`,
    ),
    fields: fact(
      ['ledger_record', 'information_set'],
      promptPath('experiment_analyst'),
      27,
      '    fields=("ledger_record", "information_set"),',
    ),
    template: fact(
      "You are the experiment analyst for a systematic trading research campaign. You are given one ledger record read-only, and your output is an explanation of it with a recommendation.\n\nYou may produce: an explanation of what the record shows, the observations it does not support, and a recommendation for the next experiment.\n\nYou may not compute an authoritative metric, alter a threshold, decide eligibility, waive a gate, or add a number that the record does not contain. Any quantity you cite is a quotation of the record, and a quantity the record does not carry does not exist for you.\n\nThe ledger record is: {ledger_record}\n\nYour information set is: {information_set}\n\nReply with a single JSON object and nothing else, with these keys: explanation, unsupported_claims, recommendation. If the record is incomplete, reply with a JSON object whose key outcome is insufficient_data and whose key reason names the missing field.\n",
      promptPath('experiment_analyst'),
      29,
      "        \"You are the experiment analyst for a systematic trading research campaign. You are given \"\n        \"one ledger record read-only, and your output is an explanation of it with a \"\n        \"recommendation.\\n\"\n        \"\\n\"\n        \"You may produce: an explanation of what the record shows, the observations it does not \"\n        \"support, and a recommendation for the next experiment.\\n\"\n        \"\\n\"\n        \"You may not compute an authoritative metric, alter a threshold, decide eligibility, waive \"\n        \"a gate, or add a number that the record does not contain. Any quantity you cite is a \"\n        \"quotation of the record, and a quantity the record does not carry does not exist \"\n        \"for you.\\n\"\n        \"\\n\"\n        \"The ledger record is: {ledger_record}\\n\"\n        \"\\n\"\n        \"Your information set is: {information_set}\\n\"\n        \"\\n\"\n        \"Reply with a single JSON object and nothing else, with these keys: explanation, \"\n        \"unsupported_claims, recommendation. If the record is incomplete, reply with a JSON object \"\n        \"whose key outcome is insufficient_data and whose key reason names the missing field.\\n\"\n    ),",
    ),
    produces: fact(
      'an explanation of the recorded evidence and a recommendation',
      AGENTS_PY,
      173,
      '    Role.EXPERIMENT_ANALYST: "an explanation of the recorded evidence and a recommendation",',
    ),
    mayNot: fact(
      'compute an authoritative metric, alter a threshold, or decide eligibility',
      AGENTS_PY,
      188,
      '        "compute an authoritative metric, alter a threshold, or decide eligibility"',
    ),
  },
  {
    id: 'adversarial_reviewer',
    prompt: fact(
      { promptId: 'adversarial_reviewer', version: '1' },
      promptPath('adversarial_reviewer'),
      25,
      `    prompt_id="adversarial_reviewer",
    version="1",`,
    ),
    fields: fact(
      ['evidence_package', 'information_set'],
      promptPath('adversarial_reviewer'),
      27,
      '    fields=("evidence_package", "information_set"),',
    ),
    template: fact(
      "You are the adversarial reviewer for a systematic trading research campaign. You are given one evidence package read-only, and your job is to find what is wrong with the claim it supports.\n\nYou may produce: findings, each with a severity, a reference to the evidence that shows it, and a test that would reproduce it.\n\nYou may not waive a gate, authorise anything, decide that a candidate is acceptable, or restate a result as a conclusion. A finding is a claim about the evidence, and a claim without a reproducible test is not a finding.\n\nThe evidence package is: {evidence_package}\n\nYour information set is: {information_set}\n\nReply with a single JSON object and nothing else, with these keys: findings, each an object with severity, evidence_reference and reproducible_test. An empty findings list is a complete answer when the package supports nothing you can test.\n",
      promptPath('adversarial_reviewer'),
      29,
      "        \"You are the adversarial reviewer for a systematic trading research campaign. You are \"\n        \"given one evidence package read-only, and your job is to find what is wrong with the \"\n        \"claim it \"\n        \"supports.\\n\"\n        \"\\n\"\n        \"You may produce: findings, each with a severity, a reference to the evidence that shows \"\n        \"it, and a test that would reproduce it.\\n\"\n        \"\\n\"\n        \"You may not waive a gate, authorise anything, decide that a candidate is acceptable, or \"\n        \"restate a result as a conclusion. A finding is a claim about the evidence, and a claim \"\n        \"without a reproducible test is not a finding.\\n\"\n        \"\\n\"\n        \"The evidence package is: {evidence_package}\\n\"\n        \"\\n\"\n        \"Your information set is: {information_set}\\n\"\n        \"\\n\"\n        \"Reply with a single JSON object and nothing else, with these keys: findings, each \"\n        \"an object with severity, evidence_reference and reproducible_test. An empty findings \"\n        \"list is a \"\n        \"complete answer when the package supports nothing you can test.\\n\"\n    ),",
    ),
    produces: fact(
      'findings with severity, an evidence reference, and a reproducible test',
      AGENTS_PY,
      175,
      '        "findings with severity, an evidence reference, and a reproducible test"',
    ),
    mayNot: fact('waive a gate, or authorise anything', AGENTS_PY, 190, '    Role.ADVERSARIAL_REVIEWER: "waive a gate, or authorise anything",'),
  },
  {
    id: 'search_strategist',
    prompt: fact(
      { promptId: 'search_strategist', version: '1' },
      promptPath('search_strategist'),
      25,
      `    prompt_id="search_strategist",
    version="1",`,
    ),
    fields: fact(
      ['population', 'objectives', 'policy', 'information_set'],
      promptPath('search_strategist'),
      27,
      '    fields=("population", "objectives", "policy", "information_set"),',
    ),
    template: fact(
      "You are the search strategist for a systematic trading research campaign. You are given the current population's identities and its development objectives, and your output is the next experiment as a bounded mutation or a recombination.\n\nYou may produce: a change to one existing candidate, naming the genes to change, with its rationale and an estimate of its cost in experiments. The genes you may name and the bounds they hold are given below.\n\nYou may not read the holdout, read locked validation or confirmation results, change the search policy, change a frozen object, or propose a candidate that duplicates one already in the population. A proposal that does any of those is refused rather than corrected.\n\nThe population and its development objectives are: {population}\nThe objectives selection ranks by are: {objectives}\nThe search policy in force is: {policy}\n\nYour information set is: {information_set}\n\nReply with a single JSON object and nothing else, with these keys: genes, rationale, estimated_experiments. If the population gives you nothing worth changing, reply with a JSON object whose key outcome is insufficient_data and whose key reason states why.\n",
      promptPath('search_strategist'),
      29,
      "        \"You are the search strategist for a systematic trading research campaign. You are given \"\n        \"the current population's identities and its development objectives, and your output \"\n        \"is the next experiment as a bounded mutation or a recombination.\\n\"\n        \"\\n\"\n        \"You may produce: a change to one existing candidate, naming the genes to change, with its \"\n        \"rationale and an estimate of its cost in experiments. The genes you may name and the \"\n        \"bounds they hold are given below.\\n\"\n        \"\\n\"\n        \"You may not read the holdout, read locked validation or confirmation results, change the \"\n        \"search policy, change a frozen object, or propose a candidate that duplicates one already \"\n        \"in the population. A proposal that does any of those is refused rather than corrected.\\n\"\n        \"\\n\"\n        \"The population and its development objectives are: {population}\\n\"\n        \"The objectives selection ranks by are: {objectives}\\n\"\n        \"The search policy in force is: {policy}\\n\"\n        \"\\n\"\n        \"Your information set is: {information_set}\\n\"\n        \"\\n\"\n        \"Reply with a single JSON object and nothing else, with these keys: genes, rationale, \"\n        \"estimated_experiments. If the population gives you nothing worth changing, reply with a \"\n        \"JSON object whose key outcome is insufficient_data and whose key reason states why.\\n\"\n    ),",
    ),
    produces: fact(
      'the next experiment, as a bounded mutation or recombination with its rationale and cost',
      AGENTS_PY,
      178,
      '        "the next experiment, as a bounded mutation or recombination with its rationale and cost"',
    ),
    mayNot: fact(
      'read the holdout, change the policy, or duplicate a candidate',
      AGENTS_PY,
      191,
      '    Role.SEARCH_STRATEGIST: "read the holdout, change the policy, or duplicate a candidate",',
    ),
  },
];

/** The four outcomes a response produces, in the order the code tests them. */
export const OUTCOMES = [
  fact('accepted', AGENTS_PY, 212, 'ACCEPTED = "accepted"'),
  fact('unstructured', AGENTS_PY, 213, 'UNSTRUCTURED = "unstructured"'),
  fact('schema_violation', AGENTS_PY, 214, 'SCHEMA_VIOLATION = "schema_violation"'),
  fact('insufficient_data', AGENTS_PY, 215, 'INSUFFICIENT_DATA = "insufficient_data"'),
];

/** The two outcomes that are refusals rather than proposals. */
export const REFUSAL_OUTCOMES = fact(
  ['unstructured', 'schema_violation'],
  AGENTS_PY,
  227,
  'return self in (AgentOutcome.UNSTRUCTURED, AgentOutcome.SCHEMA_VIOLATION)',
);

/** How vulnerable a period is to hypotheses the model may already know, in the code's order. */
export const CONTAMINATION = [
  fact('post_cutoff', AGENTS_PY, 239, 'POST_CUTOFF = "post_cutoff"'),
  fact('vulnerable', AGENTS_PY, 240, 'VULNERABLE = "vulnerable"'),
  fact('unknown', AGENTS_PY, 241, 'UNKNOWN = "unknown"'),
];

/** The keys each digest covers: a prompt's identity, an invocation's, and a response's. */
export const DIGEST_INPUTS = {
  prompt: fact(
    ['prompt_id', 'version', 'fields', 'template'],
    PROMPTS_PY,
    98,
    `        return digest_of(
            {
                "prompt_id": self.prompt_id,
                "version": self.version,
                "fields": ",".join(self.fields),
                "template": self.template,
            },
        )`,
  ),
  invocation: fact(
    ['role', 'prompt_id', 'prompt_version', 'prompt_digest', 'instructions'],
    AGENTS_PY,
    392,
    `        return digest_of(
            {
                "role": self.role.value,
                "prompt_id": self.prompt.prompt_id,
                "prompt_version": self.prompt.version,
                "prompt_digest": self.prompt.digest(),
                "instructions": self.instructions,
            },
        )`,
  ),
  response: fact('response', AGENTS_PY, 728, 'response_digest = digest_of({"response": response})'),
};

/** Every message the agent layer reports, with the code or condition that produces it. */
export const AGENT_REFUSALS = {
  unstructured: fact(
    { code: 'unstructured', message: 'the response is not a JSON object' },
    AGENTS_PY,
    768,
    '"the response is not a JSON object",',
  ),
  blockMissing: fact(
    {
      code: 'block_missing',
      message: 'a candidate proposal carries its specification under {key}',
    },
    AGENTS_PY,
    794,
    'message=f"a candidate proposal carries its specification under {SPECIFICATION_KEY}",',
  ),
  languageMissing: fact(
    {
      code: 'language_missing',
      message: 'a candidate proposal needs the language to be validated against',
    },
    AGENTS_PY,
    805,
    'raise ValueError("a candidate proposal needs the language to be validated against")',
  ),
  unrecorded: fact(
    { code: 'unrecorded_invocation', message: 'no recorded response for invocation {digest}' },
    AGENTS_PY,
    545,
    'raise KeyError(f"no recorded response for invocation {digest}")',
  ),
  emptyModelIdentifier: fact(
    { code: 'model_identifier_empty', message: 'model_identifier must not be empty' },
    AGENTS_PY,
    314,
    'raise ValueError("model_identifier must not be empty")',
  ),
  fieldMissing: fact(
    { code: 'field_missing', message: '{prompt} {version} is missing {names}' },
    PROMPTS_PY,
    81,
    'raise ValueError(f"{self.prompt_id} {self.version} is missing {\', \'.join(missing)}")',
  ),
  fieldUndeclared: fact(
    { code: 'field_undeclared', message: '{prompt} {version} was given {names}' },
    PROMPTS_PY,
    85,
    'raise ValueError(f"{self.prompt_id} {self.version} was given {\', \'.join(extra)}")',
  ),
};

/**
 * The worked example's four recorded responses, in the page's own order: one brief sent four times,
 * so one input digest, and the four things a single answer can be.
 *
 * The page shows two of the four responses with the specification elided as `{...}`; for those the
 * console sends the smallest object with the property the page states - a specification whose
 * family is `invented_family`, and one whose family the policy declares - and shows exactly what it
 * sent, so the elision is visible rather than filled in silently.
 */
export const RECORDED_REPLIES = [
  fact(
    {
      howRead: 'not a JSON object',
      outcome: 'unstructured',
      response: 'The trend should continue, so buy the strongest names.',
    },
    TUTORIAL,
    137,
    '| `The trend should continue, so buy the strongest names.`        | not a JSON object                           | `unstructured`      |',
  ),
  fact(
    {
      howRead: "the language's validator refused the family",
      outcome: 'schema_violation',
      response: '{"specification": {"identity": {"family": "invented_family"}}}',
    },
    TUTORIAL,
    138,
    '| `{"specification": {...}}` with family `invented_family`        | the language\'s validator refused the family | `schema_violation`  |',
  ),
  fact(
    {
      howRead: 'declared its information was not enough',
      outcome: 'insufficient_data',
      response: '{"outcome": "insufficient_data", "reason": "no bound fits"}',
    },
    TUTORIAL,
    139,
    '| `{"outcome": "insufficient_data", "reason": "no bound fits"}`   | declared its information was not enough     | `insufficient_data` |',
  ),
  fact(
    {
      howRead: "the language's validator accepted it",
      outcome: 'accepted',
      response: '{"specification": {"identity": {"family": "cross_sectional_rank"}}}',
    },
    TUTORIAL,
    140,
    '| `{"specification": {...}}` with a declared family and in-bounds | the language\'s validator accepted it        | `accepted`          |',
  ),
];

/** The five field values the brief carries, which are the whole of what the engineer is given. */
export const BRIEF = fact(
  {
    families: 'cross_sectional_rank, absolute_threshold',
    features: 'ema_trend',
    bounds: 'exit.holding_period_cap in [1, 252]',
    declared_sets: 'selection_modes: rank, threshold',
    information_set: "the policy's bounds and declared sets; no results",
  },
  TUTORIAL,
  131,
  'fields: families `cross_sectional_rank, absolute_threshold`; features `ema_trend`; bounds\n`exit.holding_period_cap in [1, 252]`; declared sets `selection_modes: rank, threshold`; information set\n`the policy\'s bounds and declared sets; no results`.',
);

/**
 * The one refusal the console's stand-in for the language's validator reports.
 *
 * The agent layer never reads a candidate: it hands the proposal to the language's own validator and
 * carries whatever refusals come back. The console's agents view does not own that validator, so it
 * stands in one that applies exactly this rule - the family has to be one the policy declares - and
 * reports it in the schema's own words, which is the message transcribed here.
 */
export const VALIDATOR_REFUSAL = fact(
  { code: 'value_not_declared', message: '{name} must be one of {admitted}, was {value}' },
  SCHEMA_PY,
  1153,
  'f"{name} must be one of {\', \'.join(sorted(declared))}, was {value}",',
);

/** The invocation context the worked example's record carries. */
export const CONTEXT = fact(
  {
    modelIdentifier: 'model.fixture/v1',
    knowledgeCutoff: '2023-06-01',
    periodStart: '2024-03-01',
    contamination: 'post_cutoff',
  },
  TUTORIAL,
  150,
  '  "context": {"model_identifier": "model.fixture/v1", "knowledge_cutoff": "2023-06-01",\n              "period_start": "2024-03-01", "contamination": "post_cutoff"},',
);

/** The five roles, unwrapped, in the code's order. */
export function roles() {
  return ROLES.map((row) => ({
    id: row.id,
    promptId: valueOf(row.prompt).promptId,
    version: valueOf(row.prompt).version,
    fields: valueOf(row.fields),
    produces: valueOf(row.produces),
    mayNot: valueOf(row.mayNot),
    template: valueOf(row.template),
  }));
}

/** One role by id. Throws on an unknown id, because a view asking for a missing role is a mistake. */
export function roleById(id) {
  const row = roles().find((item) => item.id === id);
  if (!row) throw new Error(`unknown role: ${id}`);
  return row;
}

/** The four outcomes, unwrapped, in the order the code tests them. */
export function outcomeNames() {
  return OUTCOMES.map(valueOf);
}

/** The two outcomes that are refusals. */
export function refusalOutcomes() {
  return valueOf(REFUSAL_OUTCOMES);
}

/** The contamination vocabulary, unwrapped. */
export function contaminationNames() {
  return CONTAMINATION.map(valueOf);
}

/** The worked example's four recorded responses, unwrapped, in the page's order. */
export function recordedReplies() {
  return RECORDED_REPLIES.map(valueOf);
}

/** The brief's field values, unwrapped. */
export function brief() {
  return valueOf(BRIEF);
}

/** The invocation context, unwrapped. */
export function context() {
  return valueOf(CONTEXT);
}

/** The refusal the stand-in validator reports, its message still templated. */
export function validatorRefusal() {
  return valueOf(VALIDATOR_REFUSAL);
}

/** The agent layer's messages, as { key: { code, message } }, the messages still templated. */
export function agentRefusals() {
  return Object.fromEntries(
    Object.entries(AGENT_REFUSALS).map(([key, node]) => [key, valueOf(node)]),
  );
}
