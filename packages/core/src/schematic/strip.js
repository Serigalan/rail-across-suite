import { switchStatus } from '../track/index.js'

/**
 * The strip of a schematic layout as lines, wedges and labels, for one lane
 * spacing: x in metres along the reference, y in whatever unit `laneY` gives
 * — the plan sheet's millimetres, the Signalling-Tool's screen pixels. Every
 * line names the tracks it draws and every switch its x, so a station on a
 * track can be found on the strip (place.js).
 *
 * A connection between two lanes is always drawn at 45°: where the two
 * switches of a crossover lie closer than that allows, they are drawn spread
 * evenly about their middle, and where they lie further apart the diagonal
 * takes the middle of the stretch — a schematic can afford a switch a little
 * off its place, not a steep stroke through the lanes.
 *
 * Moved here from the Layout-Tool's plan output (planSchematicPlan's stripFor);
 * the colours are the caller's: an item states the planning status it is
 * drawn in.
 */

/**
 * What the plan calls a switch: its number alone, without the leading zeros
 * of a generated name — switch.003 is 3, W 12a is 12a. A name without a
 * number stays as it is.
 */
export function switchNumber(name) {
  const str = String(name ?? '').trim()
  const m = /(\d+)\s*([a-z]?)$/i.exec(str)
  return m ? `${Number(m[1])}${m[2]}` : str
}

/**
 * The strip's items: { kind: 'line', pts, status, trackIds, connector },
 * { kind: 'fill', pts, status } (a switch's triangle), { kind: 'dot', x, y,
 * status }, { kind: 'label', x, y, t, track | switch, room }, { kind: 'switch',
 * x, y, sx, sy, t, status } (a number in the angle of its triangle), and
 * { kind: 'platform', x0, x1, y0, y1 }; and `nodeX`, the x each switch is
 * drawn at.
 *
 * @param {object} layout    schematicLayout's
 * @param {object} o
 * @param {Function} o.laneY  lane → y
 * @param {Function} o.dxOf   a rise across in y → the run along x it takes at 45° [m]
 * @param {number} o.leg      legs of a switch's triangle, in y
 * @param {number} o.gap      the spacing of two neighbouring lanes, in y
 * @param {object} o.show     { switches, trackNames, platforms }, all by default
 * @param {object} o.sizes    where labels go from their track, in y ({ trackLabel, switchAbove, switchBelow }),
 *                            and a platform across the gap beside its track ({ platformNear, platformFar }, in gaps)
 */
export function schematicStrip(layout, { laneY: Y, dxOf, leg, gap, show = {}, sizes = {} }) {
  const shown = { switches: true, trackNames: true, platforms: true, ...show }
  const size = { trackLabel: 0.5, switchAbove: 1.6, switchBelow: 3.2, platformNear: 0.2, platformFar: 0.8, ...sizes }
  const statusOf = (node) => switchStatus(node.sw, layout.trackById)
  const drawable = (n) => n && !n.outside && n.lane != null
  const strip = []
  const at = new Map()
  // Each wedged switch with the way its triangle opens — along x, and across.
  const wedged = new Map()
  const wedge = (node, x, y, sx, sy) => {
    if (!shown.switches || node.link) return
    if (!wedged.has(node)) wedged.set(node, { sx, sy })
    const l = dxOf(leg)
    strip.push({ kind: 'fill', pts: [[x, y], [x + sx * l, y], [x + sx * l, y + sy * leg]], status: statusOf(node) })
  }

  for (const st of layout.strands) {
    if (!st.connector) continue
    const [n1, n2] = st.endNodes
    if (!drawable(n1) || !drawable(n2) || at.has(n1) || at.has(n2)) continue
    const rise = dxOf(Y(n2.lane) - Y(n1.lane))
    const mid = (n1.x + n2.x) / 2
    const s = Math.sign(n2.x - n1.x) || 1
    at.set(n1, mid - s * rise / 2)
    at.set(n2, mid + s * rise / 2)
  }
  const nodeX = (n) => at.get(n) ?? n.x

  for (const st of layout.strands) {
    if (st.lane == null || st.connector) continue
    // Only what lies in the corridor is drawn; a strand's end beyond it has
    // no switch on the sheet to run on to.
    const secs = st.sections.filter(sc => sc.inside && sc.lane != null)
    if (!secs.length) continue
    const last = secs.length - 1
    const whole = st.sections
    const endIn = [secs[0] === whole[0], secs[last] === whole[whole.length - 1]]
    // Which way the strand runs along x, so its sections can be walked in order.
    const d = secs[0].dir
    const startOf = (sc) => (d > 0 ? sc.x0 : sc.x1)
    const endOf = (sc) => (d > 0 ? sc.x1 : sc.x0)
    // Where each section begins and ends along x, in the strand's own order.
    const from = secs.map(startOf)
    const to = secs.map(endOf)

    // The strand's two ends: on to the switch it ends at, or — where it is a
    // branch — a diagonal from that switch onto its own lane.
    st.endNodes.forEach((node, i) => {
      if (!endIn[i] || !drawable(node)) return
      const k = i === 0 ? 0 : last
      const sc = secs[k]
      const nx = nodeX(node)
      if (!st.endAttached[i] || node.lane === sc.lane) {
        if (i === 0) from[0] = d > 0 ? Math.min(from[0], nx) : Math.max(from[0], nx)
        else to[last] = d > 0 ? Math.max(to[last], nx) : Math.min(to[last], nx)
        return
      }
      const y = Y(sc.lane)
      const yn = Y(node.lane)
      const inward = i === 0 ? d : -d
      const land = nx + inward * dxOf(yn - y)
      strip.push({ kind: 'line', pts: [[nx, yn], [land, y]], connector: true, status: sc.status, trackIds: [sc.track.id] })
      wedge(node, nx, yn, inward, Math.sign(y - yn))
      if (i === 0) from[0] = land
      else to[last] = land
    })

    // Between two sections, the switch that joins them; where the lanes
    // differ the track steps across at 45°, in the section beyond the toe.
    secs.slice(0, -1).forEach((a, i) => {
      const b = secs[i + 1]
      const k = whole.indexOf(a)
      // Two sections meet at a switch only where no section left out lies between them.
      const node = whole[k + 1] === b ? st.junctions[k] : null
      const jx = node && drawable(node) ? nodeX(node) : (to[i] + from[i + 1]) / 2
      to[i] = jx
      from[i + 1] = jx
      if (a.lane === b.lane) return
      const stepIn = node?.home === b ? a : b
      const away = stepIn === b ? d : -d
      const [lFrom, lTo] = stepIn === b ? [a.lane, b.lane] : [b.lane, a.lane]
      const land = jx + away * dxOf(Y(lTo) - Y(lFrom))
      strip.push({ kind: 'line', pts: [[jx, Y(lFrom)], [land, Y(lTo)]], status: stepIn.status, trackIds: [stepIn.track.id] })
      if (stepIn === b) from[i + 1] = land
      else to[i] = land
    })

    secs.forEach((sc, i) => {
      const lo = Math.min(from[i], to[i])
      const hi = Math.max(from[i], to[i])
      // A section shorter than its step has no straight of its own.
      if ((to[i] - from[i]) * d <= 0) return
      strip.push({ kind: 'line', pts: [[lo, Y(sc.lane)], [hi, Y(sc.lane)]], status: sc.status, trackIds: [sc.track.id] })
      sc.drawn = [lo, hi]
    })

    if (shown.trackNames) {
      // One label per run of sections that share a name and a lane.
      const runs = []
      for (const sc of secs) {
        if (!sc.drawn) continue
        const lastRun = runs[runs.length - 1]
        if (lastRun && lastRun.name === sc.name && lastRun.lane === sc.lane) {
          lastRun.x0 = Math.min(lastRun.x0, sc.drawn[0])
          lastRun.x1 = Math.max(lastRun.x1, sc.drawn[1])
        } else if (sc.name) {
          runs.push({ name: sc.name, lane: sc.lane, x0: sc.drawn[0], x1: sc.drawn[1] })
        }
      }
      for (const r of runs) {
        strip.push({ kind: 'label', track: true, x: (r.x0 + r.x1) / 2, y: Y(r.lane) - size.trackLabel, t: r.name, room: r.x1 - r.x0 })
      }
    }
  }

  for (const st of layout.strands) {
    if (!st.connector) continue
    const [n1, n2] = st.endNodes
    if (!drawable(n1) || !drawable(n2)) continue
    const [a, b] = [nodeX(n1), nodeX(n2)]
    const [y1, y2] = [Y(n1.lane), Y(n2.lane)]
    const status = st.sections.find(sc => sc.status !== 'existing')?.status ?? 'existing'
    // The connection runs from n1 to n2; its tracks in that order.
    const parts = st.endNodes[0] === n1 ? st.parts : [...st.parts].reverse()
    strip.push({ kind: 'line', pts: [[a, y1], [b, y2]], connector: true, status, trackIds: parts.map(p => p.track.id) })
    const sx = Math.sign(b - a) || 1
    const sy = Math.sign(y2 - y1)
    wedge(n1, a, y1, sx, sy)
    wedge(n2, b, y2, -sx, -sy)
  }

  if (shown.switches) {
    for (const node of layout.nodes) {
      if (node.link || !drawable(node)) continue
      const y = Y(node.lane)
      // A switch whose branch leaves the strip has no diagonal to set its
      // triangle in, so it keeps a point.
      const status = statusOf(node)
      const open = wedged.get(node)
      if (!open) strip.push({ kind: 'dot', x: nodeX(node), y, status, node })
      const number = switchNumber(node.sw.name)
      if (!number) continue
      if (open) {
        strip.push({ kind: 'switch', x: nodeX(node), y, ...open, t: number, status, node })
        continue
      }
      // The label goes to the side the branch does not leave to.
      const others = [...node.attach].map(k => {
        const sc = layout.sectionOfTrack.get(k.split(':')[0])
        if (!sc) return null
        if (!sc.strand.connector) return sc.lane ?? null
        const far = sc.strand.endNodes.find(n => n && n !== node)
        return far?.lane ?? null
      }).filter(l => l != null)
      const up = others.some(l => l > node.lane)
      strip.push({ kind: 'label', switch: true, x: nodeX(node), y: up ? y + size.switchBelow : y - size.switchAbove, t: number, status, node })
    }
  }

  if (shown.platforms) {
    // A platform sits in the gap beside its track, its edges drawn along
    // both long sides, as the DB symbol has it.
    for (const pf of layout.platforms) {
      const y = Y(pf.lane)
      const s = pf.up ? -1 : 1
      const [near, far] = [y + s * size.platformNear * gap, y + s * size.platformFar * gap]
      strip.push({ kind: 'platform', x0: pf.x0, x1: pf.x1, y0: Math.min(near, far), y1: Math.max(near, far), platform: pf.platform })
    }
  }
  return { items: strip, nodeX }
}

/**
 * Kilometrage marks every `step` metres between two x, out of the table the
 * layout read along the reference.
 */
export function kmMarks(kmTable, xa, xb, step = 100) {
  const marks = []
  for (let i = 1; i < kmTable.length; i++) {
    const a = kmTable[i - 1]
    const b = kmTable[i]
    if (b.x < xa || a.x > xb || b.x - a.x < 1e-6) continue
    const lo = Math.min(a.km, b.km)
    const hi = Math.max(a.km, b.km)
    // A jump in the kilometrage is no stretch of line to mark.
    if (hi - lo > 3 * (b.x - a.x) + 50) continue
    for (let h = Math.ceil(lo / step) * step; h <= hi; h += step) {
      if (h === lo && i > 1) continue
      const t = (h - a.km) / (b.km - a.km || 1)
      const x = a.x + (b.x - a.x) * t
      if (x >= xa && x <= xb) marks.push({ x, km: h })
    }
  }
  return marks
}

/** Kilometrage at an x of the strip, read off the table the layout made. */
export function kmAt(kmTable, x) {
  if (x <= kmTable[0].x) return kmTable[0].km
  for (let i = 1; i < kmTable.length; i++) {
    const a = kmTable[i - 1]
    const b = kmTable[i]
    if (x <= b.x) return a.km + (b.km - a.km) * (x - a.x) / (b.x - a.x || 1)
  }
  return kmTable[kmTable.length - 1].km
}
