/** A time in seconds as m:ss, or h:mm:ss from an hour. */
export function formatTime(seconds) {
  if (!Number.isFinite(seconds)) return '–'
  const s = Math.round(seconds)
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), r = s % 60
  const pad = (n) => String(n).padStart(2, '0')
  return h ? `${h}:${pad(m)}:${pad(r)}` : `${m}:${pad(r)}`
}

/** A distance in metres, as km from 2 km on. */
export function formatDistance(metres, language = 'de') {
  if (!Number.isFinite(metres)) return '–'
  if (metres >= 2000) return `${(metres / 1000).toLocaleString(language, { maximumFractionDigits: 2 })} km`
  return `${Math.round(metres).toLocaleString(language)} m`
}

/** The point of a run at a moment: { position, speed, time }, between its points. */
export function pointAt(report, time) {
  const { times, positions, speeds } = report
  if (!times.length) return null
  if (time <= times[0]) return { position: positions[0], speed: speeds[0], time: times[0] }
  for (let i = 1; i < times.length; i++) {
    if (time <= times[i]) {
      const dt = times[i] - times[i - 1]
      const f = dt > 0 ? (time - times[i - 1]) / dt : 1
      // constant acceleration over the step: the distance grows with the mean speed so far
      const v0 = speeds[i - 1], v1 = speeds[i]
      const ds = positions[i] - positions[i - 1]
      const share = v0 + v1 > 0 ? (f * (2 * v0 + (v1 - v0) * f)) / (v0 + v1) : f
      return { position: positions[i - 1] + ds * share, speed: v0 + (v1 - v0) * f, time }
    }
  }
  const n = times.length - 1
  return { position: positions[n], speed: speeds[n], time: times[n] }
}
