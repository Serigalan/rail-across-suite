import { describe, expect, it } from 'vitest'
import { electrificationsOf, exportToOsrd } from './osrdExport'
import { parseOsrdRailJson } from './osrdImport'
import { arcFrom, straightFrom, transitionElement } from '@rail-across/core/geometry'

const range = (id, trackId, startStation, endStation, voltage) => ({ id, trackId, startStation, endStation, voltage })
// Contact wires over ranges of tracks, each carried at its ends (electrification 3.0, decision 0006).
const wired = (ranges) => ({ fields: {}, collections: {
  contactWires: ranges.map(r => ({ id: r.id, voltage: r.voltage })),
  masts: [],
  suspensionPoints: ranges.flatMap(r => [r.startStation, r.endStation].map((station, i) => ({
    id: `${r.id}-${i}`, mastId: `m-${r.id}-${i}`, at: { anchor: 'station', trackId: r.trackId, station }, contactWireId: r.id,
  }))),
  sectionInsulators: [],
} })

describe('the electrification as RailJSON (decisions 0008 and 0006)', () => {
  it('writes one electrification per system, its ranges both ways, on the tracks the export holds', () => {
    const project = {
      electrification: wired([
        range('a', 't1', 0, 400, '15000V'), range('b', 't2', 100, 200.123456789012, '15000V'), range('c', 't1', 400, 900, '25000V'), range('x', 'gone', 0, 10, '15000V'),
      ]),
      osrd: { electrifications: [{ id: 'kept', voltage: '1500V', track_ranges: [] }, { id: 'old', voltage: '15000V', track_ranges: [] }] },
    }
    const out = electrificationsOf(project, { t1: {}, t2: {} })
    expect(out).toEqual([
      { id: 'electrification-15000V', voltage: '15000V', track_ranges: [
        { track: 't1', begin: 0, end: 400, applicable_directions: 'BOTH' }, { track: 't2', begin: 100, end: 200.123456789, applicable_directions: 'BOTH' },
      ] },
      { id: 'electrification-25000V', voltage: '25000V', track_ranges: [{ track: 't1', begin: 400, end: 900, applicable_directions: 'BOTH' }] },
      // What an imported file brought along stays where the layer has none of its system.
      { id: 'kept', voltage: '1500V', track_ranges: [] },
    ])
    expect(electrificationsOf({}, { t1: {} })).toEqual([])
  })
})

describe('the alignment in curves and slopes', () => {
  const EPSG = 5683
  const start = { easting: 3450606.79067, northing: 5651498.82691, zone: EPSG }
  const zone = (u) => ({ ...u, zone: EPSG })
  // line — transition — arc R 249 (cant 80) — transition — line with a kink at its end — arc, left
  const line = straightFrom(start, 120.236235195, 100)
  const into = transitionElement(zone({ easting: line.endNode[0], northing: line.endNode[1] }), line.bearing, 40, null, 249, { transitionType: 'clothoid' })
  const arc = arcFrom(zone(into.endUtm), into.endBearing, 80, 249, { cant: 80 })
  const out = transitionElement(zone({ easting: arc.endNode[0], northing: arc.endNode[1] }), arc.endBearing, 40, 249, null, { transitionType: 'clothoid' })
  const tail = straightFrom(zone(out.endUtm), out.endBearing, 50)
  const kinked = { ...tail, endBearing: (tail.bearing + 3) % 360 }
  const left = arcFrom(zone({ easting: kinked.endNode[0], northing: kinked.endNode[1] }), kinked.endBearing, 60, -500)
  const elements = [line, { ...into.element, cantEnd: 80 }, arc, { ...out.element, cantStart: 80 }, kinked, left]
  const track = {
    id: 't1', epsg: EPSG, elements,
    heights: [{ station: 0, z: 116.12 }, { station: 200, z: 118.3, rv: 4000 }, { station: 370, z: 117.5 }],
  }
  const file = exportToOsrd({ tracks: [track] })
  const section = file.track_sections[0]

  it('writes one curve per arc as OSRD reads it, the first carrying the alignment of the whole track', () => {
    expect(section.curves.map(c => [c.radius, c.begin, c.end])).toEqual([[249, 140, 220], [500, 310, 370]])
    expect(section.curves[1].horizontal_alignment).toBeUndefined()
    const a = section.curves[0].horizontal_alignment
    expect(a.epsg).toBe(EPSG)
    expect(a.element_type).toEqual(['straight', 'clothoid', 'arc', 'clothoid', 'straight', 'arc'])
    expect(a.stationing).toEqual([0, 100, 140, 220, 260, 310, 370])
    expect(a.coordinates).toHaveLength(7)
    expect(a.bearing_gon).toHaveLength(7)
    expect(a.begin_radius).toEqual([null, null, 249, 249, null, -500])
    expect(a.end_cant).toEqual([0, 80, 80, 0, 0, 0])
    expect(section.extensions.db).toBeUndefined()
  })

  it('writes one slope per stretch of the heights, the first carrying the heights by column', () => {
    expect(section.slopes).toEqual([
      { gradient: 10.9, begin: 0, end: 200, vertical_alignment: {
        epsg: 7837, stationing: [0, 200, 370], elevation: [116.12, 118.3, 117.5], vertical_curve_radius: [null, 4000, null],
      } },
      { gradient: -4.706, begin: 200, end: 370 },
    ])
  })

  it('comes back as the same track, the kink and the heights included', () => {
    const { tracks, errors } = parseOsrdRailJson(file)
    expect(errors).toEqual([])
    const back = tracks[0]
    expect(back.elements.map(e => e.elementType)).toEqual(elements.map(e => e.elementType))
    back.elements.forEach((e, i) => {
      expect(e.length).toBeCloseTo(elements[i].length, 2)
      expect(e.endNode[0]).toBeCloseTo(elements[i].endNode[0], 2)
      expect(e.endNode[1]).toBeCloseTo(elements[i].endNode[1], 2)
    })
    expect(back.elements[4].endBearing).toBeCloseTo(kinked.endBearing, 5)
    expect(back.heights).toEqual(track.heights)
    expect(back.heightEpsg).toBe(7837)
  })
})
