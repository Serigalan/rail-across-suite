import { elementAtStation, kmForTrackPoint, pointAtStationUtm, resolveRoute } from '@rail-across/core/track'

/**
 * Speed ranges along a run of tracks: the design speed of its elements
 * (`speed`) joined into ranges no shorter than MIN_RANGE, each run at the
 * highest speed it allows — the lowest design speed in it —, laid so that a
 * train running them all at those speeds has the highest average speed: the
 * least time for the run's length. Stated by station along the run and by the
 * kilometrage of the line where the project has one, as a CSV table or an
 * SVG diagram.
 */

/** The shortest a range may be [m]. */
export const MIN_RANGE = 800

/** The grid a range may begin and end on between two changes of the design speed [m]. */
const STEP = 10

/**
 * A run of tracks laid out end to end: { parts, length, ok } as resolveRoute
 * gives it — a named route's tracks, or one track alone.
 */
export const runOf = (trackIds, tracks, switches) => resolveRoute({ trackIds }, tracks, switches)

/**
 * The design speed along a run: [{ from, to, speed }] [m along the run,
 * km/h], in running order; `speed` null where an element states none.
 */
export function speedProfile(run) {
  const out = []
  for (const part of run.parts ?? []) {
    let s = 0
    const pieces = (part.track.elements ?? []).map(el => {
      const piece = { from: s, to: s + (el.length ?? 0), speed: el.speed > 0 ? el.speed : null }
      s = piece.to
      return piece
    })
    for (const p of part.reversed ? pieces.reverse() : pieces) {
      const [a, b] = part.reversed ? [part.length - p.to, part.length - p.from] : [p.from, p.to]
      if (b - a <= 0) continue
      const last = out[out.length - 1]
      if (last && last.speed === p.speed && Math.abs(last.to - (part.offset + a)) < 1e-6) last.to = part.offset + b
      else out.push({ from: part.offset + a, to: part.offset + b, speed: p.speed })
    }
  }
  return out
}

/**
 * The ranges of a profile: [{ from, to, speed }] covering the run from 0 to
 * its end, each at least `min` long (the whole run one range where it is
 * shorter), at the lowest design speed in it; laid out — on the profile's
 * changes and a grid of STEP between them — for the least running time
 * Σ length / speed, which is the highest average speed; neighbours of one
 * speed are joined. A stretch without a
 * design speed limits nothing; a range with none at all has `speed` null and
 * counts at the highest speed of the run.
 */
export function speedRanges(profile, { min = MIN_RANGE, step = STEP } = {}) {
  if (!profile.length) return []
  const end = profile[profile.length - 1].to
  const top = Math.max(0, ...profile.map(p => p.speed ?? 0))
  if (!(top > 0)) return [{ from: 0, to: end, speed: null }]
  // Where a range may begin or end: every change of speed, and the grid between.
  const points = [0]
  for (const p of profile) {
    for (let s = Math.ceil((p.from + 1e-9) / step) * step; s < p.to - 1e-9; s += step) points.push(s)
    points.push(p.to)
  }
  const at = [...new Set(points.map(s => Math.round(s * 1e6) / 1e6))].sort((a, b) => a - b)
  // The speed of each interval between two neighbouring points; Infinity where none is stated.
  let k = 0
  const speedOf = at.slice(1).map((s, i) => {
    const mid = (at[i] + s) / 2
    while (profile[k].to < mid) k++
    return profile[k].speed ?? Infinity
  })
  const n = at.length
  const shortest = Math.min(min, end)
  const time = new Array(n).fill(Infinity)
  const from = new Array(n).fill(-1)
  time[0] = 0
  for (let j = 1; j < n; j++) {
    let lowest = Infinity
    for (let i = j - 1; i >= 0; i--) {
      lowest = Math.min(lowest, speedOf[i])
      if (at[j] - at[i] < shortest - 1e-6 || time[i] === Infinity) continue
      const t = time[i] + (at[j] - at[i]) / (lowest === Infinity ? top : lowest)
      if (t < time[j] - 1e-12) {
        time[j] = t
        from[j] = i
      }
    }
  }
  const out = []
  for (let j = n - 1; j > 0; j = from[j]) {
    const i = from[j]
    let lowest = Infinity
    for (let x = i; x < j; x++) lowest = Math.min(lowest, speedOf[x])
    out.unshift({ from: at[i], to: at[j], speed: lowest === Infinity ? null : lowest })
  }
  // Neighbours of one speed are one range: it takes the same time, and is no shorter than either.
  return out.reduce((joined, r) => {
    const last = joined[joined.length - 1]
    if (last && last.speed === r.speed) last.to = r.to
    else joined.push({ ...r })
    return joined
  }, [])
}

/** The average speed over ranges [km/h]: their length over the time they take at their speeds. */
export function averageSpeed(ranges) {
  const top = Math.max(0, ...ranges.map(r => r.speed ?? 0))
  const length = ranges.reduce((sum, r) => sum + r.to - r.from, 0)
  const time = ranges.reduce((sum, r) => sum + (r.to - r.from) / (r.speed ?? top), 0)
  return time > 0 ? length / time : null
}

/**
 * The kilometrage at a station along the run [m], or null: of the line the
 * track there lies on, where the project keeps it.
 */
export function kmAlong(run, s, kmLines) {
  if (!kmLines?.length) return null
  const parts = run.parts ?? []
  const part = parts.find(p => s <= p.offset + p.length + 1e-6) ?? parts[parts.length - 1]
  if (!part) return null
  const own = Math.min(part.length, Math.max(0, s - part.offset))
  const there = elementAtStation(part.track.elements, part.reversed ? part.length - own : own)
  if (!there) return null
  const p = pointAtStationUtm(there.el, there.s, part.track.epsg)
  const hit = p && kmForTrackPoint(kmLines, part.track, [p.easting, p.northing])
  return hit ? hit.km : null
}

/** Ranges with the kilometrage at their ends: { …range, kmFrom, kmTo }. */
export const withKm = (run, ranges, kmLines) =>
  ranges.map(r => ({ ...r, kmFrom: kmAlong(run, r.from, kmLines), kmTo: kmAlong(run, r.to, kmLines) }))

const decimal = (v, digits) => (v == null ? '' : v.toFixed(digits).replace('.', ','))

/**
 * The ranges as a CSV table, `;`-separated with decimal commas as a German
 * spreadsheet reads it: kilometre from and to (km, where the line is known),
 * station from and to along the run, length, highest speed.
 */
export function speedRangesCsv(ranges, texts) {
  const head = [texts.kmFrom, texts.kmTo, texts.from, texts.to, texts.length, texts.speed]
  const rows = ranges.map(r => [
    decimal(r.kmFrom == null ? null : r.kmFrom / 1000, 3), decimal(r.kmTo == null ? null : r.kmTo / 1000, 3),
    decimal(r.from, 1), decimal(r.to, 1), decimal(r.to - r.from, 1), r.speed ?? '',
  ])
  return [head, ...rows].map(row => row.join(';')).join('\r\n') + '\r\n'
}

const xml = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

/**
 * The ranges as an SVG diagram: speed over the run, the design speed of the
 * elements thin and grey, the ranges bold with their speed, the kilometrage
 * (or the station) along the bottom, the average speed in the title.
 */
export function speedRangesSvg(profile, ranges, texts) {
  const W = 1200, H = 520, left = 70, right = 30, top = 60, bottom = 70
  const end = ranges.length ? ranges[ranges.length - 1].to : 0
  const vmax = Math.max(10, ...profile.map(p => p.speed ?? 0), ...ranges.map(r => r.speed ?? 0))
  const vtop = Math.ceil(vmax / 20) * 20 + 20
  const x = (s) => left + (end > 0 ? (s / end) * (W - left - right) : 0)
  const y = (v) => top + (1 - v / vtop) * (H - top - bottom)
  const out = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" font-family="sans-serif" font-size="12">`,
    `<rect width="${W}" height="${H}" fill="#fff"/>`,
    `<text x="${left}" y="28" font-size="16" font-weight="bold">${xml(texts.title)}</text>`,
    `<text x="${left}" y="46" fill="#444">${xml(texts.subtitle)}</text>`,
  ]
  for (let v = 0; v <= vtop; v += 20) {
    out.push(`<line x1="${left}" y1="${y(v)}" x2="${W - right}" y2="${y(v)}" stroke="#e5e5e5"/>`,
      `<text x="${left - 8}" y="${y(v) + 4}" text-anchor="end" fill="#444">${v}</text>`)
  }
  out.push(`<text x="18" y="${(top + H - bottom) / 2}" transform="rotate(-90 18 ${(top + H - bottom) / 2})" text-anchor="middle" fill="#444">${xml(texts.speedAxis)}</text>`)
  // The design speed of the elements, a stretch without one left out.
  for (const p of profile) {
    if (p.speed == null) continue
    out.push(`<line x1="${x(p.from)}" y1="${y(p.speed)}" x2="${x(p.to)}" y2="${y(p.speed)}" stroke="#999" stroke-width="1.5"/>`)
  }
  // The ranges, joined at their ends.
  let path = ''
  ranges.forEach((r, i) => {
    if (r.speed == null) return
    path += `${i && ranges[i - 1].speed != null ? 'L' : 'M'}${x(r.from)},${y(r.speed)}L${x(r.to)},${y(r.speed)}`
  })
  out.push(`<path d="${path}" fill="none" stroke="#c0392b" stroke-width="3"/>`)
  for (const r of ranges) {
    out.push(`<line x1="${x(r.from)}" y1="${top}" x2="${x(r.from)}" y2="${H - bottom}" stroke="#c0392b" stroke-width="0.6" stroke-dasharray="3 3"/>`)
    if (r.speed != null && x(r.to) - x(r.from) > 26) {
      out.push(`<text x="${(x(r.from) + x(r.to)) / 2}" y="${y(r.speed) - 7}" text-anchor="middle" fill="#c0392b" font-weight="bold">${r.speed}</text>`)
    }
  }
  // Along the bottom: each range's start, by kilometre where it is known, else by station.
  const mark = (s, km) => (km != null ? `km ${decimal(km / 1000, 3)}` : `${Math.round(s)} m`)
  const ends = [...ranges.map(r => [r.from, r.kmFrom]), ...(ranges.length ? [[end, ranges[ranges.length - 1].kmTo]] : [])]
  let lastX = -Infinity
  for (const [s, km] of ends) {
    if (x(s) - lastX < 70 && s !== end) continue
    lastX = x(s)
    out.push(`<line x1="${x(s)}" y1="${H - bottom}" x2="${x(s)}" y2="${H - bottom + 5}" stroke="#333"/>`,
      `<text x="${x(s)}" y="${H - bottom + 18}" text-anchor="middle" fill="#333">${xml(mark(s, km))}</text>`)
  }
  out.push(`<line x1="${left}" y1="${H - bottom}" x2="${W - right}" y2="${H - bottom}" stroke="#333"/>`,
    `<line x1="${left}" y1="${top}" x2="${left}" y2="${H - bottom}" stroke="#333"/>`,
    `<line x1="${left}" y1="${H - 22}" x2="${left + 30}" y2="${H - 22}" stroke="#999" stroke-width="1.5"/>`,
    `<text x="${left + 36}" y="${H - 18}" fill="#444">${xml(texts.legendDesign)}</text>`,
    `<line x1="${left + 260}" y1="${H - 22}" x2="${left + 290}" y2="${H - 22}" stroke="#c0392b" stroke-width="3"/>`,
    `<text x="${left + 296}" y="${H - 18}" fill="#444">${xml(texts.legendRanges)}</text>`,
    '</svg>')
  return out.join('\n') + '\n'
}
