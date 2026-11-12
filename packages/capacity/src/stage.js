import { defaultStages } from '@rail-across/core/document'

/**
 * A variant's record as it stands at one of the project's two stages: at
 * `existing` without what is new, at `final` without what is removed — the
 * tracks, switches, platforms and end marks by their planning status, the
 * layers' objects by their lifespan over the final stage. So one variant's
 * two stages are laid beside each other (K1).
 */
export const STAGES = ['existing', 'final']

export function recordAtStage(record, stage) {
  if (!record || stage !== 'existing' && stage !== 'final') return record
  const final = defaultStages(record.id)[1].id
  const gone = stage === 'existing' ? 'new' : 'removal'
  const byStatus = (list) => (list ?? []).filter(o => o.status !== gone)
  const byLife = (o) => (stage === 'existing' ? o.lifespan?.from !== final : o.lifespan?.to !== final)
  const layer = (d) => (d ? { ...d, collections: Object.fromEntries(Object.entries(d.collections ?? {}).map(([k, v]) => [k, Array.isArray(v) ? v.filter(byLife) : v])) } : d)
  return {
    ...record,
    tracks: byStatus(record.tracks),
    switches: byStatus(record.switches),
    platforms: byStatus(record.platforms),
    ...(record.endMarks ? { endMarks: byStatus(record.endMarks) } : {}),
    ...(record.signalling ? { signalling: layer(record.signalling) } : {}),
    ...(record.electrification ? { electrification: layer(record.electrification) } : {}),
  }
}
