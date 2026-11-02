/**
 * The tool's edits as steps: the domain now, the ones before it (undo) and
 * after it (redo). A step that changes nothing is none. Pure — the App holds
 * it in its state.
 */

import { labelOf } from './domain.js'

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

/** `n` steps at once: forward (redo) where it is positive, back (undo) where it is negative. */
export function travel(h, n) {
  let at = h
  for (let i = 0; i < Math.abs(n); i++) at = n > 0 ? redo(at) : undo(at)
  return at
}

/**
 * What a step did, from the domain before it and after it: { kind, collection,
 * obj, more } — `kind` added, removed or changed for one object (`more` the
 * others a deletion took along), `many` for several, `fields` where only the
 * domain's own fields changed. The domain is immutable: an object not the
 * same is one changed.
 */
export function stepOf(before, after) {
  const added = [], removed = [], changed = []
  for (const [collection, list] of Object.entries(after.collections)) {
    const was = new Map((before.collections[collection] ?? []).map(o => [o.id, o]))
    const now = new Set(list.map(o => o.id))
    for (const obj of list) {
      if (!was.has(obj.id)) added.push({ collection, obj })
      else if (was.get(obj.id) !== obj) changed.push({ collection, obj })
    }
    for (const obj of was.values()) if (!now.has(obj.id)) removed.push({ collection, obj })
  }
  const n = added.length + removed.length + changed.length
  if (!n) return { kind: 'fields', n }
  if (removed.length && removed.length + changed.length === n) return { kind: 'removed', ...removed[0], more: n - 1 }
  if (n > 1) return { kind: 'many', n }
  return { kind: added.length ? 'added' : 'changed', ...(added[0] ?? changed[0]), more: 0 }
}

/** The steps that can be undone (`past`, oldest first) and redone (`future`, nearest first), each with what it did. */
export function stepsOf(h) {
  const states = [...h.past, h.now, ...h.future]
  const steps = states.slice(1).map((after, i) => stepOf(states[i], after))
  return { past: steps.slice(0, h.past.length), future: steps.slice(h.past.length) }
}

/** What a step did, in words: "Signal „A“ angelegt", "3 Objekte geändert". */
export function stepText(step, t) {
  if (step.kind === 'fields') return t('step_fields')
  if (step.kind === 'many') return t('step_many', { n: step.n })
  const name = labelOf(step.collection, step.obj, t)
  const object = t(`object_${step.collection}`)
  const what = name ? t('step_what', { object, name }) : object
  if (step.kind === 'removed' && step.more) return t('step_removed_more', { what, n: step.more })
  return t(`step_${step.kind}`, { what })
}
