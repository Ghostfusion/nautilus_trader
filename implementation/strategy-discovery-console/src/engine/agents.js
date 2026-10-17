/**
 * What the campaign does with an agent's response, and with a recorded one.
 *
 * Models `python/nautilus_trader/discovery/agents.py` and `.../prompts/__init__.py`: the five roles
 * with the field set that limits them, the prompt identity a digest covers, the order a response is
 * read in - JSON object, then an insufficient-data declaration, then the language's own validator -
 * the four outcomes that produces, the contamination label a period carries, and the replay of a
 * recorded response.
 *
 * There is no model here and no network. A live call cannot be reproduced - sampling, version and
 * provider all move - so the only reproducible object is the recorded response, which is why
 * `replay` either returns what was recorded or refuses by name rather than inventing an answer. The
 * language a candidate proposal is validated against is injected, exactly as the repository injects
 * it: this module knows the shape of a validator's answer and nothing about the schema.
 */

import { digestOf } from './digest.js';
import {
  PROTOCOL,
  DIGEST_INPUTS,
  agentRefusals,
  contaminationNames,
  outcomeNames,
  recordedReplies,
  refusalOutcomes,
  roleById,
  roles,
} from '../data/agents.js';

/** The four outcomes a response produces, in the order the code tests them. */
export const OUTCOMES = outcomeNames();

/** The two of them that are refusals rather than proposals. */
export const REFUSING_OUTCOMES = refusalOutcomes();

/** The contamination vocabulary, in the code's order. */
export const CONTAMINATION = contaminationNames();

/** The one role whose response is a candidate proposal, and the only one a language is given for. */
const ENGINEER = roleById('strategy_engineer').id;

/** The five roles, in the code's order, with their fields, their output and their denied authority. */
export function ROLES() {
  return roles();
}

/** The fields each role is given, which is the whole of what it is given. */
export function PROMPT_FIELDS() {
  return Object.fromEntries(roles().map((role) => [role.id, role.fields]));
}

/** Compose a transcribed message, substituting the names it carries. */
function message(key, values = {}) {
  const entry = agentRefusals()[key];
  return entry.message.replace(/\{(\w+)\}/g, (match, name) =>
    name in values ? String(values[name]) : match,
  );
}

/** Refuse a render or an invocation with the code and message the layer reports. */
function refuse(key, values = {}) {
  return { ok: false, code: agentRefusals()[key].code, reason: message(key, values) };
}

/**
 * The digest of a prompt's identity: its identifier, its version, its field names and its template.
 *
 * The template travels in the digest because a change to the wording is a different prompt, not the
 * same prompt edited, so an invocation record can say which instructions produced an answer.
 */
export function promptDigest(prompt) {
  return digestOf({
    prompt_id: prompt.promptId,
    version: prompt.version,
    fields: prompt.fields.join(','),
    template: prompt.template,
  });
}

/**
 * The digest of an invocation: the role, the prompt's identity and the rendered instructions.
 *
 * This is the key a replay is looked up by, so two briefs differing by one sentence are two
 * different invocations rather than one.
 */
export function inputDigest(request) {
  return digestOf({
    role: request.role,
    prompt_id: request.promptId,
    prompt_version: request.promptVersion,
    prompt_digest: request.promptDigest,
    instructions: request.instructions,
  });
}

/**
 * Render a role's prompt from the fields the role is given, and refuse any other field.
 *
 * The template's declared fields are the whole of what a role is handed, so an undeclared field is
 * refused rather than ignored: a brief that leaked a field would defeat the separation the roles
 * exist to keep, which is why this refusal is load-bearing rather than tidy.
 */
export function renderPrompt(roleId, values = {}) {
  const role = roleById(roleId);
  const missing = role.fields.filter((name) => !(name in values)).sort();
  if (missing.length > 0) {
    return refuse('fieldMissing', {
      prompt: role.promptId,
      version: role.version,
      names: missing.join(', '),
    });
  }
  const extra = Object.keys(values)
    .filter((name) => !role.fields.includes(name))
    .sort();
  if (extra.length > 0) {
    return refuse('fieldUndeclared', {
      prompt: role.promptId,
      version: role.version,
      names: extra.join(', '),
    });
  }
  const instructions = role.template.replace(/\{(\w+)\}/g, (match, name) => String(values[name]));
  const digest = promptDigest(role);
  return {
    ok: true,
    role: role.id,
    instructions,
    informationSet: String(values.information_set ?? ''),
    promptId: role.promptId,
    promptVersion: role.version,
    promptDigest: digest,
    inputDigest: inputDigest({
      role: role.id,
      promptId: role.promptId,
      promptVersion: role.version,
      promptDigest: digest,
      instructions,
    }),
  };
}

/** Read a response as JSON, accepting only a JSON object as the repository's loader does. */
function load(response) {
  if (typeof response !== 'string') return null;
  let loaded;
  try {
    loaded = JSON.parse(response);
  } catch {
    return null;
  }
  if (loaded === null || typeof loaded !== 'object' || Array.isArray(loaded)) return null;
  return loaded;
}

/**
 * Read a response into one of the four outcomes.
 *
 * The order is the code's and it matters: a response that is not a JSON object is unstructured
 * before anything else is read; an insufficient-data declaration is a complete result rather than a
 * malformed one; the four roles that are not candidate producers are accepted on a JSON object
 * alone, because their limit is the field set they were given; and only a candidate proposal is put
 * through the language's validator. A proposal offered without a language throws, because accepting
 * an unvalidated proposal is the thing the contract exists to prevent.
 */
export function classifyReply(reply, { role, language } = {}) {
  const payload = load(reply);
  if (payload === null) {
    return {
      outcome: 'unstructured',
      reason: message('unstructured'),
      payload: null,
      refusals: [],
    };
  }

  const declared = payload[PROTOCOL.insufficientDataKey.value];
  if (typeof declared === 'string' && declared === PROTOCOL.insufficientDataValue.value) {
    return {
      outcome: 'insufficient_data',
      reason: String(payload.reason ?? ''),
      payload,
      refusals: [],
    };
  }

  if (role !== ENGINEER) {
    return { outcome: 'accepted', reason: '', payload, refusals: [] };
  }

  const candidate = payload[PROTOCOL.specificationKey.value];
  if (candidate === null || typeof candidate !== 'object' || Array.isArray(candidate)) {
    const refusal = {
      code: 'block_missing',
      path: PROTOCOL.specificationKey.value,
      message: message('blockMissing', { key: PROTOCOL.specificationKey.value }),
    };
    return {
      outcome: 'schema_violation',
      reason: refusal.message,
      payload,
      refusals: [refusal],
    };
  }

  if (language === undefined || language === null) throw new Error(message('languageMissing'));

  const validation = language.validate(candidate);
  if (validation.accepted) {
    return { outcome: 'accepted', reason: '', payload, refusals: [], validation };
  }
  const first = (validation.errors ?? [])[0];
  return {
    outcome: 'schema_violation',
    reason: first === undefined ? '' : first.message,
    payload,
    refusals: validation.errors ?? [],
    validation,
  };
}

/**
 * Replay a recorded response, or refuse the invocation that was never recorded.
 *
 * A replay that invented a response would not be a replay, so an invocation with nothing recorded
 * against it is refused by name - the repository raises for the same reason - and the response that
 * is replayed is read again through the same classifier, which is what makes the stored outcome
 * checkable rather than trusted.
 */
export function replay(record, options = {}) {
  if (record === null || record === undefined || typeof record.response !== 'string') {
    return refuse('unrecorded', { digest: record?.inputDigest ?? '' });
  }
  const read = classifyReply(record.response, { role: record.role, language: options.language });
  return {
    ok: true,
    inputDigest: record.inputDigest ?? '',
    role: record.role ?? '',
    response: record.response,
    responseDigest: digestOf({ response: record.response }),
    ...read,
  };
}

/** Whether an outcome is a refusal rather than an accepted proposal. */
export function isRefusal(outcome) {
  return REFUSING_OUTCOMES.includes(outcome);
}

/** The messages the layer reports, with the code each carries. */
export function refusals() {
  return agentRefusals();
}

/** The four responses the worked example records, in the page's own order. */
export function recorded() {
  return recordedReplies();
}

/** Whether a value has the shape of an ISO calendar date, which is what the classifier checks. */
function isIsoDate(value) {
  if (typeof value !== 'string' || value.length !== PROTOCOL.isoDateLength.value) return false;
  const parts = value.split('-');
  const widths = PROTOCOL.isoDatePartWidths.value;
  if (parts.length !== widths.length) return false;
  return parts.every((part, index) => /^\d+$/.test(part) && part.length === widths[index]);
}

/**
 * Label a period against the model's declared knowledge cutoff.
 *
 * The comparison is made on the text of two dates, which is only sound because both are checked to
 * be zero-padded and the same length; a value that is not is labelled unknown rather than compared.
 * The label is standing: nothing converts a vulnerable period into a clean one.
 */
export function contamination(knowledgeCutoff, periodStart) {
  if (!isIsoDate(knowledgeCutoff) || !isIsoDate(periodStart)) return CONTAMINATION[2];
  if (periodStart > knowledgeCutoff) return CONTAMINATION[0];
  return CONTAMINATION[1];
}

/** The keys each digest covers, for a reader who wants to see what an identity is made of. */
export function digestInputs() {
  return {
    prompt: DIGEST_INPUTS.prompt.value,
    invocation: DIGEST_INPUTS.invocation.value,
    response: DIGEST_INPUTS.response.value,
  };
}
