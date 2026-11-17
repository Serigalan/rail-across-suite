import { recordAtStage, stagesOf } from '@rail-across/core/document'

/**
 * The map at a construction stage (decision 0007): which stages a
 * project has — the existing state, every day something is put into or taken
 * out of service, the final state — and which of its tracks and switches
 * stand at one. The core works it out on what the stages need of the record
 * alone: the ids and the lifespans, not the geometry.
 */

const life = (o) => (o.lifespan ? { lifespan: o.lifespan } : {})

const lites = new WeakMap()
/** What of the project the stages need, kept as long as the project object is. */
function lite(project) {
  if (!lites.has(project)) {
    lites.set(project, {
      id: project.id,
      tracks: (project.tracks ?? []).map(t => ({ id: t.id, ...life(t) })),
      switches: (project.switches ?? []).map(s => ({ switchId: s.switchId, ...life(s) })),
      platforms: (project.platforms ?? []).map(p => ({ id: p.id, ...life(p) })),
      endMarks: (project.endMarks ?? []).map(m => ({ id: m.id, ...life(m) })),
    })
  }
  return lites.get(project)
}

/** The stages a project offers: [{ key, date? }] — 'existing', each day, 'final'. */
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

/** Whether `key` is a day (YYYY-MM-DD) rather than 'existing' or 'final'. */
export const isDayKey = (key) => typeof key === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(key)

/** A day (YYYY-MM-DD) as the language writes it. */
export function dayLabel(day, language = 'de') {
  const [y, m, d] = day.split('-').map(Number)
  return new Date(Date.UTC(y, m - 1, d)).toLocaleDateString(language, { timeZone: 'UTC', day: '2-digit', month: '2-digit', year: 'numeric' })
}

/** A stage as a list names it: the existing state, "from <day>", the final state. */
export function stageLabel(stage, t, language = 'de') {
  if (stage.key === 'existing') return t('stage_existing')
  if (stage.key === 'final') return t('stage_final')
  return `${t('stage_from')} ${dayLabel(stage.date, language)}`
}

/** A day (YYYY-MM-DD) of year `y`, month `m` (1–12), day `d`. */
export const dayOf = (y, m, d) => `${String(y).padStart(4, '0')}-${String(m).padStart(2, '0')}-${String(d).padStart(2, '0')}`

/**
 * The weeks of a month as a calendar shows them, Monday first: rows of seven
 * days (YYYY-MM-DD), null where a day belongs to the month before or after.
 */
export function monthWeeks(y, m) {
  const first = (new Date(Date.UTC(y, m - 1, 1)).getUTCDay() + 6) % 7
  const length = new Date(Date.UTC(y, m, 0)).getUTCDate()
  const cells = [...Array(first).fill(null), ...Array.from({ length }, (_, i) => dayOf(y, m, i + 1))]
  while (cells.length % 7) cells.push(null)
  return Array.from({ length: cells.length / 7 }, (_, i) => cells.slice(i * 7, i * 7 + 7))
}

/** The month `n` months after year `y`, month `m`: [year, month]. */
export function monthAfter(y, m, n) {
  const i = y * 12 + (m - 1) + n
  return [Math.floor(i / 12), (i % 12) + 1]
}
