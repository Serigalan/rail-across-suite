import { recordAtStage, stagesOf } from '@rail-across/core/document'

/**
 * The construction stages of a variant and what each changes (decisions
 * 0027, 0028): the existing state, every day something is put into or taken
 * out of service, the final state — for each what stands then, what comes
 * into service with it and what left service with it. The core works the
 * stages out of the lifespans; nothing else of the record is needed.
 */

/** The colour a track is drawn in at a stage: standing, come into service with it, leaving service with the next, of another stage. */
export const STATE_COLORS = { stays: '#1f2330', came: '#d6336c', goes: '#e8890c', off: '#9aa0b3' }

const life = (o) => (o.lifespan ? { lifespan: o.lifespan } : {})

/** What of a record the stages need: the ids and the lifespans. */
export function liteRecord(record) {
  return {
    id: record.id,
    tracks: (record.tracks ?? []).map(t => ({ id: t.id, ...life(t) })),
    switches: (record.switches ?? []).map(s => ({ switchId: s.switchId, ...life(s), ports: {} })),
    platforms: (record.platforms ?? []).map(p => ({ id: p.id, ...life(p) })),
    endMarks: (record.endMarks ?? []).map(m => ({ id: m.id, ...life(m) })),
  }
}

const diff = (now, before) => [...now].filter(id => !before.has(id))

/**
 * The stages in their order: [{ key, date?, tracks, switches, came: { tracks, switches }, went: { tracks,
 * switches } }] — `tracks`, `switches` the ids standing then (Sets), `came` what stands from this stage on and not
 * before, `went` what stood before and not any more (arrays of ids).
 */
export function timeline(record) {
  if (!record) return []
  const lite = liteRecord(record)
  let before = null
  return stagesOf(lite).map(stage => {
    const at = recordAtStage(lite, stage.key)
    const now = { tracks: new Set(at.tracks.map(t => t.id)), switches: new Set(at.switches.map(s => s.switchId)) }
    const step = {
      ...stage, ...now,
      came: { tracks: before ? diff(now.tracks, before.tracks) : [], switches: before ? diff(now.switches, before.switches) : [] },
      went: { tracks: before ? diff(before.tracks, now.tracks) : [], switches: before ? diff(before.switches, now.switches) : [] },
    }
    before = now
    return step
  })
}

/** How each track stands at a stage: 'came' (into service with it), 'goes' (out of service with the next), 'stays', or null (not in service). */
export function trackState(stages, i, id) {
  const s = stages[i]
  if (!s?.tracks.has(id)) return null
  if (s.came.tracks.includes(id)) return 'came'
  if (stages[i + 1] && !stages[i + 1].tracks.has(id)) return 'goes'
  return 'stays'
}

/** A stage as a list names it: the existing state, "from <day>", the final state. */
export function stageLabel(stage, t, language = 'de') {
  if (stage.key === 'existing') return t('stage_existing')
  if (stage.key === 'final') return t('stage_final')
  const [y, m, d] = stage.date.split('-').map(Number)
  const day = new Date(Date.UTC(y, m - 1, d)).toLocaleDateString(language, { timeZone: 'UTC', day: '2-digit', month: '2-digit', year: 'numeric' })
  return `${t('stage_from')} ${day}`
}
