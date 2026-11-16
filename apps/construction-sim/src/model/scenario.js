import { formatClock } from '@rail-across/timetables'

/**
 * A timetable's construction scenarios as the Baubetriebs-Sim changes them
 * (decision 0030): kept in the timetable's data (`data.scenarios`), each with
 * its closures, the trains taken out, the trains diverted by hand. Every
 * change gives a new object; nothing here runs a train.
 */

// A UUID, as every id of the record (decision 0029, phase 6) — drawn by hand where the browser draws none.
const newId = () => globalThis.crypto?.randomUUID?.() ?? '10000000-1000-4000-8000-100000000000'.replace(/[018]/g, c => (c ^ (Math.random() * 16) >> (c / 4)).toString(16))

/** Today, as a day (`YYYY-MM-DD`). */
export const today = () => new Date().toISOString().slice(0, 10)

/** Whether a text is a day (`YYYY-MM-DD`). */
export const isDay = (s) => /^\d{4}-\d{2}-\d{2}$/.test(String(s ?? ''))

/** A new scenario, empty. */
export const newScenario = (name) => ({ id: newId(), name, closures: [], cancelledTrainIds: [], diversions: [] })

/** A new closure of a track from one station to another, all day on `day`. */
export const newClosure = ({ trackId, from, to }, day) => ({
  id: newId(), at: { anchor: 'station', trackId, startStation: Math.round(Math.min(from, to) * 10) / 10, endStation: Math.round(Math.max(from, to) * 10) / 10 },
  firstDay: day, lastDay: day,
})

/** The scenarios of a timetable's data. */
export const scenariosOf = (data) => data?.scenarios ?? []

/** The data with `scenario` in it: in its place where it is there, else last. */
export function withScenario(data, scenario) {
  const list = scenariosOf(data)
  const there = list.some(s => s.id === scenario.id)
  return { ...data, scenarios: there ? list.map(s => (s.id === scenario.id ? scenario : s)) : [...list, scenario] }
}

/** The data without the scenario of that id. */
export const withoutScenario = (data, id) => ({ ...data, scenarios: scenariosOf(data).filter(s => s.id !== id) })

/** The scenario with the closure of that id changed by `patch` — a window null: all day. */
export function changedClosure(scenario, id, patch) {
  const closures = (scenario.closures ?? []).map(c => {
    if (c.id !== id) return c
    const next = { ...c, ...patch }
    if (next.window == null) delete next.window
    return next
  })
  return { ...scenario, closures }
}

/** The scenario with a closure added, or without the one of that id. */
export const withClosure = (scenario, closure) => ({ ...scenario, closures: [...(scenario.closures ?? []), closure] })
export const withoutClosure = (scenario, id) => ({ ...scenario, closures: (scenario.closures ?? []).filter(c => c.id !== id) })

/** Whether a train is taken out in the scenario. */
export const isCancelled = (scenario, trainId) => (scenario?.cancelledTrainIds ?? []).includes(trainId)

/** The scenario with the train taken out, or put back where it was. */
export function toggledCancel(scenario, trainId) {
  const list = scenario.cancelledTrainIds ?? []
  return { ...scenario, cancelledTrainIds: list.includes(trainId) ? list.filter(id => id !== trainId) : [...list, trainId] }
}

/** A train's diversion by hand in the scenario, or null. */
export const diversionOf = (scenario, trainId) => (scenario?.diversions ?? []).find(d => d.trainId === trainId) ?? null

/** The scenario with the train diverted by hand on `points`, or — points null — not any more. */
export function withDiversion(scenario, trainId, points) {
  const rest = (scenario.diversions ?? []).filter(d => d.trainId !== trainId)
  return { ...scenario, diversions: points ? [...rest, { trainId, points }] : rest }
}

/**
 * The day a scenario is looked at first: the first day of its closures, the
 * earliest — else `fallback`.
 */
export function firstDayOf(scenario, fallback) {
  const days = (scenario?.closures ?? []).map(c => c.firstDay).filter(isDay).sort()
  return days[0] ?? fallback
}

/** How a closure is named in a list: its name, else its track and stretch. */
export const closureLabel = (c, trackName) => c.name || `${trackName(c.at.trackId)} · ${Math.round(c.at.startStation)}–${Math.round(c.at.endStation)} m`

/** When a closure holds, as it reads: its days, all day or in its window. */
export function closureWhen(c, t) {
  const days = c.firstDay === c.lastDay ? c.firstDay : `${c.firstDay} – ${c.lastDay}`
  return c.window ? `${days}, ${formatClock(c.window.from)}–${formatClock(c.window.to)}` : `${days}, ${t('all_day')}`
}

/** The colour of a train's state in a scenario. */
export const STATE_TONES = { regular: 'ok', diverted: 'warn', by_hand: 'primary', not_runnable: 'danger', cancelled: null }

/** The colours the plan draws a scenario in: what is closed, a train's regular way, its way in the scenario. */
export const PLAN_COLORS = { closed: '#e03131', regular: '#4c6ef5', scenario: '#2f9e44', pick: '#e03131' }
