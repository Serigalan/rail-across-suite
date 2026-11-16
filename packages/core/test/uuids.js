// What the door makes of a fixture's short ids (decision 0029, phase 2): every
// id a UUID, the one drawn from it (derivedId), every reference to it the same.
// For tests that state a record with ids like 't1' and expect it back.
import { uuidOf } from '../src/document/ids.js'

/** The UUID an id goes by in a document. */
export const u = uuidOf

// The record names some references as before (decision 0029, phase 1): not ending in Id.
const BEFORE = new Set(['operationalPoint', 'dep', 'boundedBy', 'clearSections'])
const isReference = (key, parent) => key === 'id' || key.endsWith('Id') || key.endsWith('Ids') || BEFORE.has(key) ||
  (parent === 'passOn' && (key === 'left' || key === 'right'))
// What is kept under `id` there is a catalogue's key or a foreign program's, no object's.
const FOREIGN = new Set(['catalogue', 'osrd'])

const mapped = (v) => (typeof v === 'string' ? u(v) : Array.isArray(v) ? v.map(x => (typeof x === 'string' ? u(x) : x)) : undefined)

/** A record or document with its ids as the door writes them. */
export function withUuids(value, parent) {
  if (Array.isArray(value)) return value.map(v => withUuids(v, parent))
  if (!value || typeof value !== 'object') return value
  return Object.fromEntries(Object.entries(value).map(([k, v]) => {
    if (FOREIGN.has(k)) return [k, v]
    if (isReference(k, parent)) return [k, mapped(v) ?? withUuids(v, k)]
    return [k, withUuids(v, k)]
  }))
}
