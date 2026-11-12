import { recordAtStage, stagesOf } from '@rail-across/core/document'

/**
 * The construction stages of a variant and what each changes (decision
 * 0027): the existing state, every day something is put into or taken out of
 * service, the final state — for each what stands then, what comes into
 * service with it and what left service with it. The core works the stages
 * out on what they need of the record: ids, statuses, days.
 */

/** The colour a track is drawn in at a stage: standing, come into service with it, leaving service with the next, of another stage. */
export const STATE_COLORS = { stays: '#1f2330', came: '#d6336c', goes: '#e8890c', off: '#9aa0b3' }

const DAYS = ['status', 'inServiceFrom', 'inServiceTo']
const pick = (o, keys) => Object.fromEntries(keys.filter(k => o[k] != null).map(k => [k, o[k]]))

/** What of a record the stages need. */
export function liteRecord(record) {
  return {
    id: record.id,
    ...(record.stages ? { stages: record.stages } : {}),
    tracks: (record.tracks ?? []).map(t => ({ id: t.id, ...pick(t, DAYS) })),
    switches: (record.switches ?? []).map(s => ({ switchId: s.switchId, ...pick(s, DAYS), ports: {} })),
    platforms: (record.platforms ?? []).map(p => ({ id: p.id, ...pick(p, DAYS) })),
    endMarks: (record.endMarks ?? []).map(m => ({ id: m.id, ...pick(m, DAYS) })),
  }
}

const diff = (now, before) => [...now].filter(id => !before.has(id))

/**
 * The stages in their order: [{ key, id, date?, name?, tracks, switches, came: { tracks, switches }, went: { tracks,
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
  return stage.name ? `${stage.name} (${day})` : `${t('stage_from')} ${day}`
}
