import { unzipSync, strFromU8 } from 'fflate'
import { placeSignalling, planproRecord } from '@rail-across/core/planpro'

/**
 * A PlanPro file as the tool takes it: a `.planpro` container (a zip with
 * its `content.xml`) or the bare XML (`.ppxml`, `.xml`). Its signalling is
 * placed on the project's own net — where the plan's edges are the project's
 * tracks (a project made from that plan) it stays on them, else every
 * object goes to the track lying at its place, and what nothing lies under
 * is orphaned in the plane, to be attached again.
 */

/** The XML text of a PlanPro file's bytes. */
export function planproText(bytes) {
  const zip = bytes[0] === 0x50 && bytes[1] === 0x4b
  if (!zip) return strFromU8(bytes)
  const files = unzipSync(bytes, { filter: (f) => /(^|\/)content\.xml$/i.test(f.name) })
  const name = Object.keys(files)[0]
  if (!name) throw new Error('planpro: the container has no content.xml')
  return strFromU8(files[name])
}

/**
 * The plan's signalling on `record`'s net: { signalling, header, report,
 * counts: { placed, orphaned, total } }.
 */
export function importPlanPro(bytes, record) {
  const plan = planproRecord(planproText(bytes), { id: record.id })
  const own = new Set((record.tracks ?? []).map(t => t.id))
  const sameNet = plan.record.tracks.length > 0 && plan.record.tracks.every(t => own.has(t.id))
  const signalling = sameNet
    ? plan.record.signalling
    : placeSignalling(plan.record.signalling, plan.record.tracks, { tracks: record.tracks ?? [], switches: record.switches ?? [] }).signalling
  let placed = 0, orphaned = 0
  const look = (v) => {
    if (Array.isArray(v)) { v.forEach(look); return }
    if (!v || typeof v !== 'object') return
    if (v.anchor === 'station') { placed++; return }
    if (v.anchor === 'plane') { orphaned++; return }
    Object.values(v).forEach(look)
  }
  look(signalling.collections)
  return { signalling, header: plan.header, report: plan.report, sameNet, counts: { placed, orphaned, total: placed + orphaned } }
}
