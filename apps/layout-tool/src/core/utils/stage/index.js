import { recordAtStage, stagesOf } from '@rail-across/core/document'

/**
 * The map at a construction stage (decision 0027): which stages a project
 * has — the existing state, every day something is put into or taken out of
 * service, the final state — and which of its tracks and switches stand at
 * one. The core works it out on what the stages need of the record alone:
 * the ids, the statuses and the days, not the geometry.
 */

const DAYS = ['status', 'inServiceFrom', 'inServiceTo']
const pick = (o, keys) => Object.fromEntries(keys.filter(k => o[k] != null).map(k => [k, o[k]]))

const lites = new WeakMap()
/** What of the project the stages need, kept as long as the project object is. */
function lite(project) {
  if (!lites.has(project)) {
    lites.set(project, {
      id: project.id,
      ...(project.stages ? { stages: project.stages } : {}),
      tracks: (project.tracks ?? []).map(t => ({ id: t.id, ...pick(t, DAYS) })),
      switches: (project.switches ?? []).map(s => ({ switchId: s.switchId, ...pick(s, DAYS) })),
      platforms: (project.platforms ?? []).map(p => ({ id: p.id, ...pick(p, DAYS) })),
      endMarks: (project.endMarks ?? []).map(m => ({ id: m.id, ...pick(m, DAYS) })),
    })
  }
  return lites.get(project)
}

/** The stages a project offers: [{ key, id, date? }] — 'existing', each day, 'final'. */
export const stageOptions = (project) => (project ? stagesOf(lite(project)) : [])

const members = new WeakMap()
/** The ids of what stands at stage `key`: { tracks, switches, platforms, endMarks } (Sets), kept per project. */
export function stageMembers(project, key) {
  if (!members.has(project)) members.set(project, new Map())
  const byKey = members.get(project)
  if (!byKey.has(key)) {
    const at = recordAtStage(lite(project), key)
    byKey.set(key, {
      tracks: new Set(at.tracks.map(t => t.id)),
      switches: new Set(at.switches.map(s => s.switchId)),
      platforms: new Set(at.platforms.map(p => p.id)),
      endMarks: new Set(at.endMarks.map(m => m.id)),
    })
  }
  return byKey.get(key)
}

/** A stage as a list names it: the existing state, "from <day>", the final state. */
export function stageLabel(stage, t, language = 'de') {
  if (stage.key === 'existing') return t('stage_existing')
  if (stage.key === 'final') return t('stage_final')
  const [y, m, d] = stage.date.split('-').map(Number)
  const day = new Date(Date.UTC(y, m - 1, d)).toLocaleDateString(language, { timeZone: 'UTC', day: '2-digit', month: '2-digit', year: 'numeric' })
  return stage.name ? `${stage.name} (${day})` : `${t('stage_from')} ${day}`
}
