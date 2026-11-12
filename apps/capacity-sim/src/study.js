import { formatClock } from '@rail-across/timetables'

/**
 * What the Kapazitäts-Sim shows, as text and numbers: pure functions of a
 * study's figures.
 */

/** A rate as a percentage, one decimal: "39,9 %". */
export const percent = (rate, language = 'de') =>
  (rate == null || !Number.isFinite(rate) ? '—' : `${(rate * 100).toLocaleString(language, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} %`)

/** A duration as minutes and seconds: "23:56". */
export const minutes = (s) => {
  if (s == null || !Number.isFinite(s)) return '—'
  const r = Math.round(s)
  return `${Math.floor(r / 60)}:${String(Math.abs(r % 60)).padStart(2, '0')}`
}

/** The hour a timetable's trains are thickest in, as the period a study starts with: { from, to } [s]. */
export function busiestHour(trains) {
  const departures = (trains ?? []).map(t => t.departure).filter(Number.isFinite)
  if (!departures.length) return { from: 6 * 3600, to: 7 * 3600 }
  let best = null
  for (const d of departures) {
    const from = Math.floor(d / 3600) * 3600
    const n = departures.filter(x => x >= from && x < from + 3600).length
    if (!best || n > best.n) best = { from, n }
  }
  return { from: best.from, to: best.from + 3600 }
}

/** A period as it is read: "08:00–09:00". */
export const periodText = ({ from, to }) => `${formatClock(from)}–${formatClock(to)}`

/** What a resource of the capacity crate is called: a track section, or a stretch of a track. */
export function resourceLabel(key, trackName, t) {
  const [kind, ...rest] = key.split(':')
  if (kind === 'section') return t('resource_section', { id: rest.join(':').slice(0, 8) })
  const [track, range] = [rest.slice(0, -1).join(':'), rest.at(-1)]
  const [from, to] = (range ?? '').split('-')
  return t('resource_track', { track: trackName(track), from, to })
}

/** A stage as a list names it: the existing state, "from <day>", the final state. */
export function stageName(key, t, language = 'de') {
  if (key === 'existing' || key === 'final') return t(`stage_${key}`)
  const [y, m, d] = String(key).split('-').map(Number)
  if (!y || !m || !d) return String(key)
  return `${t('stage_from')} ${new Date(Date.UTC(y, m - 1, d)).toLocaleDateString(language, { timeZone: 'UTC', day: '2-digit', month: '2-digit', year: 'numeric' })}`
}
