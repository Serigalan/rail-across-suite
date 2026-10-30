/**
 * The tool's edits as steps: the domain now, the ones before it (undo) and
 * after it (redo). A step that changes nothing is none. Pure — the App holds
 * it in its state.
 */

const LIMIT = 200

export const startHistory = (domain) => ({ now: domain, past: [], future: [] })

export function step(h, next) {
  if (next === h.now) return h
  return { now: next, past: [...h.past, h.now].slice(-LIMIT), future: [] }
}

export function undo(h) {
  if (!h.past.length) return h
  return { now: h.past[h.past.length - 1], past: h.past.slice(0, -1), future: [h.now, ...h.future] }
}

export function redo(h) {
  if (!h.future.length) return h
  return { now: h.future[0], past: [...h.past, h.now], future: h.future.slice(1) }
}
