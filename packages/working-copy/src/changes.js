import { diffEntries, diffProject } from '@rail-across/core/merge'

/**
 * What a working copy changed against the revision it rests on, as the
 * project's diff lists it (diffEntries): its tracks, switches and the rest
 * object by object; a domain the core carries (signalling) as one entry.
 */
export function localChanges(wc) {
  if (!wc?.basePayload || !wc.project) return []
  return diffEntries(diffProject(wc.basePayload, wc.project))
}

const sameJson = (a, b) => JSON.stringify(a) === JSON.stringify(b)

/**
 * How many objects of a carried domain differ between two states of it
 * ({ fields, collections }): added, removed or changed, collection by
 * collection, its fields counting one.
 */
export function domainChanges(before, after) {
  let n = sameJson(before?.fields ?? {}, after?.fields ?? {}) ? 0 : 1
  const names = new Set([...Object.keys(before?.collections ?? {}), ...Object.keys(after?.collections ?? {})])
  for (const name of names) {
    const a = new Map((before?.collections?.[name] ?? []).map(o => [o.id, o]))
    const b = new Map((after?.collections?.[name] ?? []).map(o => [o.id, o]))
    for (const [id, obj] of b) if (!a.has(id) || !sameJson(a.get(id), obj)) n++
    for (const id of a.keys()) if (!b.has(id)) n++
  }
  return n
}

/**
 * How many changes a working copy holds, the signalling counted object by
 * object rather than as the one entry the project's diff makes of it.
 */
export function localChangeCount(wc) {
  if (!wc?.basePayload || !wc.project) return 0
  const core = localChanges(wc).filter(e => !(e.collection === 'project' && e.id === 'signalling')).length
  return core + domainChanges(wc.basePayload.signalling, wc.project.signalling)
}
