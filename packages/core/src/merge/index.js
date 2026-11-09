// merge — comparing and merging two states of a project (three-way merge,
// id remapping), and carrying a domain's locations across a change of the net.
// Computed by the core's Rust module (decision 0020, step 10: crates/commands'
// merge), called with the arguments as the JavaScript took them. Drawing the
// comparison on the map is the app's (compare).
//
// This file is the module's public face: code outside packages/core/src/merge/
// imports from here and nowhere else (tools/modules.mjs). See docs/modules/merge.md.

import { keepSame, viaRust } from '../native/index.js'

/** A project as the comparison reads it: its dehydrated record. */
export const primary = viaRust('merge.primary')
/** What changed from `base` to `other`, per collection and object, field by field. */
export const diffProject = viaRust('merge.diffProject')
/** How many objects a diff names. */
export const diffSize = viaRust('merge.diffSize')
/** A diff as a flat list: { collection, id, kind, fields }. */
export const diffEntries = viaRust('merge.diffEntries')

/**
 * The three-way merge `{ base, mine, theirs, remapsMine, remapsTheirs }`:
 * { merged, conflicts, warnings, applied, context } — nothing repaired, a
 * conflict decided by choosing a side (resolve).
 */
export const mergeProject = viaRust('merge.mergeProject')
/** The conflicts and warnings a merged record adds to what mine already fails. */
export const validationConflicts = viaRust('merge.validationConflicts')
/** A merge with its conflicts decided, `choices` { conflictId → 'mine'|'theirs' }: the record. */
export const resolve = viaRust('merge.resolve')

/** The domains a record carries for others, by the name it holds them under. */
export const CARRIED_DOMAINS = ['signalling']

const carryAfterWriteRust = viaRust('merge.carryAfterWrite')
/** A track as the carrying reads it: without its display line, which it does not need and which is the bulk. */
const plainTrack = ({ coordinates: _c, ...track }) => track

/**
 * A record after one write of the net (the Layout-Tool's storage.js: every
 * write comes by here): the domains it carries for others with their
 * locations carried onto the tracks `after` has — the record itself where
 * nothing moved. Only a write that changed the tracks of a record carrying a
 * domain goes to the module, with just what the carrying reads.
 */
export function carryAfterWrite(before, after) {
  if (!after || before?.tracks === after.tracks) return after
  const domains = CARRIED_DOMAINS.filter(name => after[name]?.collections)
  if (!domains.length) return after
  const out = carryAfterWriteRust(
    { tracks: (before?.tracks ?? []).map(plainTrack) },
    { tracks: (after.tracks ?? []).map(plainTrack), switches: after.switches ?? [], ...Object.fromEntries(domains.map(n => [n, after[n]])) },
  )
  let next = after
  for (const name of domains) next = keepSame({ ...next, [name]: out[name] }, after)
  return next
}
/**
 * A domain's locations carried from the tracks `before` onto `project`'s
 * tracks and switches — a track changed where it is gone or no longer the
 * same: { domain, carried: [{ collection, id, field, from, to }] }.
 */
export const carryDomainOnto = (domain, before, project) =>
  viaRust('merge.carryDomainOnto')(domain, before ?? [], project?.tracks ?? [], project?.switches ?? [])
