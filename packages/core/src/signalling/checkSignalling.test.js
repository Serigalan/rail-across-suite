import { describe, it, expect } from 'vitest'
import { checkSignalling } from './index.js'
import { ksStation, ksStationCore } from '../../test/signallingFixture.js'

const check = (change = () => {}) => {
  const signalling = structuredClone(ksStation())
  const core = ksStationCore()
  change(signalling.collections, core)
  return checkSignalling({ core, signalling })
}
const codes = (findings) => findings.map(f => f.code).sort()
const byId = (list, id) => list.find(o => o.id === id)

describe('a station signalled with Ks and PZB', () => {
  it('holds together', () => {
    expect(check()).toEqual({ errors: [], warnings: [] })
  })

  it('names what is missing in the domain and in the core', () => {
    const { errors } = check((c) => {
      byId(c.signals, 'sig-a').at.trackId = 'gone'
      byId(c.switchElements, 'we-1').switchId = 'w9'
      byId(c.routes, 'rt-a-n1').overlaps[0].overlapId = 'ov-9'
      byId(c.stationBoundaries, 'bd-1').operationalPointId = 'op9'
      byId(c.signals, 'sig-n1').lifespan = { from: 'stage-9' }
    })
    const missing = errors.filter(e => e.code === 'reference_missing').map(e => [e.id, e.params.target, e.params.ref])
    expect(missing).toEqual(expect.arrayContaining([
      ['sig-a', 'core.tracks', 'gone'], ['we-1', 'core.switches', 'w9'], ['rt-a-n1', 'signalling.overlaps', 'ov-9'],
      ['bd-1', 'core.operationalPoints', 'op9'], ['sig-n1', 'core.fields.stages', 'stage-9'],
    ]))
  })

  it('finds a location beyond its track, and an orphan in the plane', () => {
    const { errors, warnings } = check((c) => {
      byId(c.detectionPoints, 'dp-2').at.station = 251
      byId(c.signals, 'sig-ne1').at = { anchor: 'plane', easting: 500000, northing: 5700000, epsg: 25832, bearing: 90 }
    })
    expect(codes(errors)).toEqual(['off_track'])
    expect(codes(warnings)).toEqual(['orphaned'])
  })
})

describe('what a signal may show', () => {
  it('holds a screen to its kind: what it must and may show', () => {
    const { errors } = check((c) => {
      const a = byId(c.signals, 'sig-a')
      a.frames[0].aspects = [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }, { aspect: 'Sh_1' }]   // no Ks 2, and Sh 1 on a signal that is none
    })
    expect(errors.map(e => [e.code, e.params.aspect])).toEqual([['screen_lacks', 'Ks_2'], ['screen_not_allowed', 'Sh_1']])
  })

  it('knows its aspects, their frames and their symbols', () => {
    const { errors } = check((c) => {
      byId(c.signals, 'sig-a').frames.push(
        { kind: 'indicator', aspects: [{ aspect: 'Zs_3', symbol: '17' }] },
        { kind: 'indicator', aspects: [{ aspect: 'Zs_2' }] },
        { kind: 'indicator', aspects: [{ aspect: 'Hp_0' }] },
        { kind: 'indicator', aspects: [{ aspect: 'Zs_99' }] },
      )
    })
    expect(errors.map(e => [e.code, e.params.aspect])).toEqual([
      ['symbol_invalid', 'Zs_3'], ['symbol_invalid', 'Zs_2'], ['aspect_not_on_frame', 'Hp_0'], ['aspect_unknown', 'Zs_99'],
    ])
  })

  it('lets a main signal carry no Zs 3v, which only announces', () => {
    const { errors } = check((c) => { byId(c.signals, 'sig-n1').frames[1].aspects = [{ aspect: 'Zs_3v', symbol: '4' }] })
    expect(codes(errors)).toEqual(['indicator_not_allowed'])
  })

  it('wants a known system and a kind it has; a virtual signal shows nothing', () => {
    const { errors } = check((c) => {
      byId(c.signals, 'sig-n1').system = 'hv'
      byId(c.signals, 'sig-a').type = 'shunting'
      byId(c.signals, 'sig-v').frames = [{ kind: 'plate', aspects: [{ aspect: 'Ne_1' }] }]
    })
    expect(codes(errors)).toEqual(['system_unknown', 'type_not_in_system', 'virtual_with_frames'])
  })

  it('takes a board on its own only where the catalogue has it as one', () => {
    const { errors } = check((c) => { byId(c.signals, 'sig-ne1').frames[0].aspects = [{ aspect: 'Ne_14' }] })
    expect(codes(errors)).toEqual(['aspect_unknown'])
    const { errors: plate } = check((c) => { byId(c.signals, 'sig-ne1').frames = [{ kind: 'plate', aspects: [{ aspect: 'Zs_3', symbol: '8' }] }] })
    expect(plate).toEqual([])
  })
})

describe('routes and overlaps', () => {
  it('run from the start signal\'s track to the target\'s, and end in an overlap behind the target', () => {
    const { errors } = check((c) => {
      byId(c.routes, 'rt-a-n1').path = { trackIds: ['t2'] }
      byId(c.overlaps, 'ov-n1').signalId = 'sig-a'
    })
    expect(codes(errors)).toEqual(['overlap_not_at_target', 'route_start_off_path', 'route_target_off_path'])
    const { errors: overlap } = check((c) => { byId(c.overlaps, 'ov-n1').path = { trackIds: ['t2'] } })
    expect(codes(overlap)).toEqual(['overlap_start_off_path'])
  })

  it('warns of a train route to a real signal without an overlap; a shunting route has none', () => {
    const { warnings } = check((c) => { byId(c.routes, 'rt-a-n1').overlaps = [] })
    expect(codes(warnings)).toEqual(['route_without_overlap'])
    const { errors } = check((c) => { Object.assign(byId(c.routes, 'rt-a-n1'), { kind: 'shunting', shuntingType: 'RR', trainType: undefined }) })
    expect(codes(errors)).toEqual(['shunting_with_overlap'])
  })

  it('shows only what its signals have, and prefers one overlap at most', () => {
    const { errors } = check((c) => {
      const r = byId(c.routes, 'rt-a-n1')
      r.aspects.push({ signalId: 'sig-a', aspect: 'Zs_3', symbol: '4' })
      r.overlaps.push({ overlapId: 'ov-n1', preferred: true })
    })
    expect(codes(errors)).toEqual(['overlap_preferred_many', 'route_aspect_not_on_signal'])
  })

  it('ends an overlap on the last track of its path', () => {
    const { errors } = check((c) => { byId(c.overlaps, 'ov-n1').end.trackId = 't1' })
    expect(codes(errors)).toEqual(['overlap_end_off_path'])
  })
})

describe('flank protection, PZB, station boundaries', () => {
  it('asks once per switch element and position, never of the element itself', () => {
    const { errors } = check((c) => {
      c.flankProtection.push({ id: 'fla-3', requester: { switchElementId: 'we-1', via: 'left' }, protection: { switchElementId: 'we-1', position: 'right' } })
    })
    expect(codes(errors)).toEqual(['flank_asked_twice', 'flank_by_itself'])
  })

  it('wants a speed check\'s values, none on a magnet, and the signal that switches an element', () => {
    const { errors } = check((c) => {
      delete byId(c.pzbElements, 'pzb-gue').speedCheck
      byId(c.pzbElements, 'pzb-a-1000').speedCheck = { testSpeed: 40, measuringLength: 20 }
      delete byId(c.pzbElements, 'pzb-n1-500').signalId
    })
    expect(codes(errors)).toEqual(['magnet_with_speed_check', 'pzb_switched_without_signal', 'speed_check_values_missing'])
  })

  it('takes an entry signal, a board Ne 1 or a switch as a boundary, and warns of anything else', () => {
    const ok = check((c) => { byId(c.stationBoundaries, 'bd-1').marks = [{ signalId: 'sig-ne1' }, { switchId: 'w1' }] })
    expect(ok).toEqual({ errors: [], warnings: [] })
    const { warnings } = check((c) => { byId(c.stationBoundaries, 'bd-1').marks = [{ signalId: 'sig-n1' }] })
    expect(codes(warnings)).toEqual(['boundary_mark_special'])
  })

  it('warns of a detection point that bounds no section', () => {
    const { warnings } = check((c) => { c.trackSections[0].boundedBy = ['dp-1'] })
    expect(warnings.map(w => [w.code, w.id])).toEqual([['detection_point_unused', 'dp-2']])
  })
})
