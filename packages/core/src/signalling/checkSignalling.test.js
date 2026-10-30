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
    // dp-2 beyond its track cuts nothing, so section 1 no longer closes there.
    expect(codes(warnings)).toEqual(['orphaned', 'section_extent_differs'])
  })
})

describe('what a signal may show', () => {
  it('holds a screen to its kind: what it must and may show', () => {
    const { errors } = check((c) => {
      const a = byId(c.signals, 'sig-a')
      a.frames[0].aspects = [{ aspect: 'Ks_1' }, { aspect: 'Ks_2' }, { aspect: 'Sh_1' }]   // no Hp 0, and Sh 1 on a signal that is none
    })
    expect(errors.map(e => [e.code, e.params.aspect])).toEqual([['screen_lacks', 'Hp_0'], ['screen_not_allowed', 'Sh_1']])
  })

  it('takes the lights of Zs 1, Zs 7 and Kl on the screen, as PlanPro plans have them', () => {
    const { errors } = check((c) => {
      const a = byId(c.signals, 'sig-a')
      a.frames[0].aspects.push({ aspect: 'Zs_7' }, { aspect: 'Kl' })
      a.frames.push({ kind: 'nameplate', aspects: [] })
    })
    expect(errors).toEqual([])
  })

  it('warns of a screen without a proceed aspect, and of a frame that shows nothing', () => {
    const { errors, warnings } = check((c) => {
      const n1 = byId(c.signals, 'sig-n1')
      n1.frames[0].aspects = [{ aspect: 'Hp_0' }, { aspect: 'Sh_1' }]
      n1.frames[1].aspects = []
    })
    expect(codes(errors)).toEqual(['route_aspect_not_on_signal'])   // the route N1 → line shows Ks 1 there
    expect(codes(warnings)).toEqual(['frame_empty', 'screen_no_proceed'])
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
      byId(c.signals, 'sig-n1').logicalSignals = [{ system: 'hv', type: 'main' }]
      byId(c.signals, 'sig-a').logicalSignals = [{ system: 'ks', type: 'trainCover' }]
      byId(c.signals, 'sig-v').frames = [{ kind: 'plate', aspects: [{ aspect: 'Ne_1' }] }]
    })
    expect(codes(errors)).toEqual(['system_unknown', 'type_not_in_system', 'virtual_with_frames'])
  })

  it('holds a screen to all its logical signals at once, each system once', () => {
    // A Ks signal that is a shunting signal too must show Sh 0 as well, and may show Sh 1.
    const { errors } = check((c) => {
      const a = byId(c.signals, 'sig-a')
      a.logicalSignals.push({ system: 'sh', type: 'shunting' })
      a.frames[0].aspects.push({ aspect: 'Sh_1' })
    })
    expect(errors.map(e => [e.code, e.params.aspect])).toEqual([['screen_lacks', 'Sh_0']])
    const { errors: twice } = check((c) => { byId(c.signals, 'sig-a').logicalSignals.push({ system: 'ks', type: 'main' }) })
    expect(codes(twice)).toEqual(['system_twice'])
  })

  it('wants a kind where a signal has a screen; a screen of no system shows only known aspects', () => {
    const { errors } = check((c) => { byId(c.signals, 'sig-a').logicalSignals = [{ system: 'ks' }] })
    expect(codes(errors)).toEqual(['type_missing'])
    expect(check((c) => { delete byId(c.signals, 'sig-a').logicalSignals })).toEqual({ errors: [], warnings: [] })
  })

  it('takes no settings a system does not declare', () => {
    const { errors } = check((c) => { byId(c.signals, 'sig-a').logicalSignals[0].settings = { Nf: true } })
    expect(errors.map(e => [e.code, e.params.setting])).toEqual([['setting_unknown', 'Nf']])
  })

  it('takes a board on its own only where the catalogue has it as one', () => {
    const { errors } = check((c) => { byId(c.signals, 'sig-ne1').frames[0].aspects = [{ aspect: 'Ne_99' }] })
    expect(codes(errors)).toEqual(['aspect_unknown'])
    const { errors: plate } = check((c) => { byId(c.signals, 'sig-ne1').frames = [{ kind: 'plate', aspects: [{ aspect: 'Zs_3', symbol: '8' }] }] })
    expect(plate).toEqual([])
    const { errors: mast } = check((c) => { byId(c.signals, 'sig-ne1').frames = [{ kind: 'plate', aspects: [{ aspect: 'Ms_ws_rt_ws' }] }] })
    expect(codes(mast)).toEqual(['board_not_allowed'])
  })

  it('checks a clearance marker on both tracks it stands at', () => {
    const { errors } = check((c) => {
      c.signals.push({
        id: 'sig-gz', kind: 'real', at: { anchor: 'station', trackId: 't2', station: 20, direction: 'down' },
        alsoAt: [{ anchor: 'station', trackId: 't9', station: 20 }],
        frames: [{ kind: 'body', aspects: [{ aspect: 'Ra_12' }] }],
      })
      byId(c.switchElements, 'we-1').clearanceMarkerId = 'sig-gz'
    })
    expect(errors.map(e => [e.code, e.params.field])).toEqual([['reference_missing', 'alsoAt.0.trackId']])
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

  it('ends an overlap on its run, or just beside its end', () => {
    const { errors } = check((c, core) => {
      core.collections.tracks.push({ id: 't3', epsg: 25832, elements: [{ elementType: 0, length: 80 }] })
      byId(c.overlaps, 'ov-n1').end = { anchor: 'station', trackId: 't3', station: 10 }
    })
    expect(codes(errors)).toEqual(['overlap_end_off_path'])
    // The clearance marker of a switch at its end, on the other branch: beside it.
    const { errors: beside } = check((c, core) => {
      core.collections.tracks.push({ id: 't3', epsg: 25832, elements: [{ elementType: 0, length: 80 }] })
      core.collections.switches[0].ports = { A: { trackId: 't2', endpoint: 'BEGIN' }, B1: { trackId: 't3', endpoint: 'BEGIN' } }
      byId(c.overlaps, 'ov-n1').end = { anchor: 'station', trackId: 't3', station: 10 }
    })
    expect(beside).toEqual([])
  })
})

describe('flank protection, balises and magnets, station boundaries', () => {
  it('asks once per switch element and position, never of the element itself', () => {
    const { errors } = check((c) => {
      c.flankProtection.push({ id: 'fla-3', requester: { switchElementId: 'we-1', via: 'left' }, protection: { switchElementId: 'we-1', position: 'right' } })
    })
    expect(codes(errors)).toEqual(['flank_asked_twice', 'flank_by_itself'])
  })

  it('holds a balise or magnet to its system and kind: the settings it must have, may have, and their values', () => {
    const { errors } = check((c) => {
      delete byId(c.balises, 'pzb-gue').settings.testSpeed
      byId(c.balises, 'pzb-a-1000').settings.testSpeed = 40
      byId(c.balises, 'pzb-n1-2000').settings.frequency = '1500'
      byId(c.balises, 'etcs-n1').settings.position = 9
      delete byId(c.balises, 'pzb-n1-500').signalId
    })
    expect(errors.map(e => [e.code, e.id, e.params.setting])).toEqual([
      ['setting_unknown', 'pzb-a-1000', 'testSpeed'],
      ['balise_switched_without_signal', 'pzb-n1-500', undefined],
      ['setting_invalid', 'pzb-n1-2000', 'frequency'],
      ['setting_missing', 'pzb-gue', 'testSpeed'],
      ['setting_invalid', 'etcs-n1', 'position'],
    ])
  })

  it('wants a train protection system and a kind of it the catalogue knows', () => {
    const { errors } = check((c) => {
      byId(c.balises, 'pzb-a-1000').system = 'lzb'
      byId(c.balises, 'etcs-n1').kind = 'magnet'
    })
    expect(codes(errors)).toEqual(['kind_not_in_system', 'system_unknown'])
  })

  it('takes an entry signal, a board Ne 1 or a switch as a boundary, and warns of anything else', () => {
    const ok = check((c) => { byId(c.stationBoundaries, 'bd-1').marks = [{ signalId: 'sig-ne1' }, { switchId: 'w1' }] })
    expect(ok).toEqual({ errors: [], warnings: [] })
    const { warnings } = check((c) => { byId(c.stationBoundaries, 'bd-1').marks = [{ signalId: 'sig-n1' }] })
    expect(codes(warnings)).toEqual(['boundary_mark_special'])
  })

  it('holds a derailer at its place, and a switch giving flank protection to its position', () => {
    const { errors } = check((c) => {
      c.switchElements.push({ id: 'gs-1', name: 'Gs1', operation: 'electricRemote', derailer: { at: { anchor: 'station', trackId: 't2', station: 200, direction: 'up' }, preferredPosition: 'applied' } })
      c.flankProtection.push({ id: 'fla-3', requester: { switchElementId: 'we-1', via: 'tip' }, protection: { switchElementId: 'gs-1' } })
      byId(c.flankProtection, 'fla-2').protection = { switchElementId: 'we-2' }
      c.switchElements.push({ id: 'we-2', switchId: 'w1', part: 'b', operation: 'electricRemote' })
    })
    expect(errors.map(e => e.code)).toEqual(['off_track', 'flank_position_missing'])
  })

  it('derives a section from its own points over the net, and says where they enclose nothing', () => {
    // A point of another section inside it is no boundary of its: sections overlap where they meet.
    const inside = check((c) => {
      c.detectionPoints.push({ id: 'dp-3', kind: 'axleCounter', at: { anchor: 'station', trackId: 't1', station: 100 } })
      c.trackSections.push({ id: 'ts-2', kind: 'axleCounting', boundedBy: ['dp-3', 'dp-2'] })
    })
    expect(inside).toEqual({ errors: [], warnings: [] })
    // t2 is not connected to t1: no part of the net lies between a point on each.
    const { warnings } = check((c) => {
      c.detectionPoints.push({ id: 'dp-3', kind: 'axleCounter', at: { anchor: 'station', trackId: 't2', station: 100 } })
      c.trackSections[0].boundedBy = ['dp-1', 'dp-3']
    })
    expect(warnings.map(w => [w.code, w.id])).toEqual([['detection_point_unused', 'dp-2'], ['section_extent_differs', 'ts-1']])
    expect(warnings[1].params.missing).toHaveLength(1)
  })

  it('warns of a detection point that bounds no section', () => {
    const { warnings } = check((c) => { c.trackSections[0].boundedBy = ['dp-1'] })
    expect(warnings.map(w => [w.code, w.id])).toEqual([['detection_point_unused', 'dp-2']])
  })
})
