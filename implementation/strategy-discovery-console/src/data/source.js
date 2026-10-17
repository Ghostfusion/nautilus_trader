/**
 * Provenance for every transcribed fact.
 *
 * The console states rules that live in this repository: a bound, a refusal code, a gate question, a
 * paragraph of a tutorial. Each one is stored inside a `fact(...)` wrapper that carries the file it
 * came from, the line it was read at, and the verbatim text that supports it. tools/check-facts.mjs
 * walks every data module, finds these wrappers, and fails when the supporting text is no longer in
 * the file - which is what keeps a demonstration from drifting away from the code and the documents
 * it claims to implement.
 *
 * A fact is data. Nothing in this module computes anything, and a data module must never derive a
 * transcribed value at runtime: if the value is not written down with its source, it cannot be
 * checked.
 */

/** Wrap a transcribed value with the file, line and verbatim text it was read from. */
export function fact(value, path, line, text) {
  return { value, source: { path, line, text } };
}

/** True when a node is a transcription wrapper rather than a plain object. */
export function isFact(node) {
  return (
    node !== null &&
    typeof node === 'object' &&
    !Array.isArray(node) &&
    'value' in node &&
    node.source !== null &&
    typeof node.source === 'object' &&
    typeof node.source.path === 'string' &&
    typeof node.source.text === 'string'
  );
}

/** The transcribed value, whether or not it is wrapped. */
export function valueOf(node) {
  return isFact(node) ? node.value : node;
}

/** The provenance of a node, or null when it carries none. */
export function sourceOf(node) {
  return isFact(node) ? node.source : null;
}

/** A one-line citation for display: the file and the line. */
export function cite(node) {
  const source = sourceOf(node);
  if (!source) return '';
  return `${source.path}:${source.line}`;
}

/** Collapse whitespace, so a quotation may be stored wrapped and compared to prose. */
export function normalise(text) {
  return String(text).replace(/\s+/g, ' ').trim();
}
