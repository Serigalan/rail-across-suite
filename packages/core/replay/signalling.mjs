// The vectors of the signalling domain's content (crates/signalling): the
// catalogue's lookups, the check of the Ks station of the tests and of every
// variation the tests make of it, overlaps, the speed a turnout allows a run,
// and the track sections' extent over a small net.
import * as S from '../src/signalling/index.js'
import { straightElement } from '../src/geometry/index.js'
import { ksStation, ksStationCore } from '../test/signallingFixture.js'
import { rec } from './record.mjs'

const byId = (list, id) => list.find(o => o.id === id)
const t3 = (core) => core.collections.tracks.push({ id: 't3', epsg: 25832, elements: [{ elementType: 0, length: 80 }] })
const overBranch = (speed, signalled) => (c, core) => {
  core.collections.switches[0] = { ...core.collections.switches[0], ...(speed ? { speed } : {}),
    ports: { A: { trackId: 't1', endpoint: 'END' }, B1: { trackId: 't2', endpoint: 'BEGIN' }, B2: { trackId: 't3', endpoint: 'BEGIN' } } }
  byId(c.signals, 'sig-n1').logicalSignals[0].signallingSpeeds = signalled == null ? undefined : [{ routeId: 'rt-n1-v', speed: signalled }]
}

// The variations of checkSignalling.test.js.
const CHANGES = [
  () => {},
  (c) => {
    byId(c.signals, 'sig-a').at.trackId = 'gone'
    byId(c.switchElements, 'we-1').switchId = 'w9'
    byId(c.routes, 'rt-a-n1').overlaps[0].overlapId = 'ov-9'
    byId(c.stationBoundaries, 'bd-1').operationalPointId = 'op9'
    byId(c.signals, 'sig-n1').lifespan = { from: 'stage-9' }
    byId(c.signals, 'sig-n1').operationalPointId = 'op8'
  },
  (c) => {
    byId(c.detectionPoints, 'dp-2').at.station = 251
    byId(c.signals, 'sig-ne1').at = { anchor: 'plane', easting: 500000, northing: 5700000, epsg: 25832, bearing: 90 }
  },
  (c) => { byId(c.signals, 'sig-a').logicalSignals[0].frames[0].aspects = [{ aspect: 'Ks_1' }, { aspect: 'Ks_2' }, { aspect: 'Sh_1' }] },
  (c) => {
    byId(c.signals, 'sig-a').logicalSignals[0].frames[0].aspects.push({ aspect: 'Zs_7' }, { aspect: 'Kl' })
    byId(c.signals, 'sig-n1').frames.push({ kind: 'nameplate', aspects: [] })
  },
  (c) => {
    const n1 = byId(c.signals, 'sig-n1').logicalSignals[0]
    n1.frames[0].aspects = [{ aspect: 'Hp_0' }, { aspect: 'Sh_1' }]
    n1.frames[1].aspects = []
  },
  (c) => {
    byId(c.signals, 'sig-a').logicalSignals[0].frames.push(
      { kind: 'indicator', aspects: [{ aspect: 'Zs_3', symbol: '17' }] }, { kind: 'indicator', aspects: [{ aspect: 'Zs_2' }] },
      { kind: 'indicator', aspects: [{ aspect: 'Hp_0' }] }, { kind: 'indicator', aspects: [{ aspect: 'Zs_99' }] },
    )
  },
  (c) => { byId(c.signals, 'sig-n1').logicalSignals[0].frames[1].aspects = [{ aspect: 'Zs_3v', symbol: '4' }] },
  (c) => {
    Object.assign(byId(c.signals, 'sig-n1').logicalSignals[0], { system: 'hv', type: 'main' })
    byId(c.signals, 'sig-a').logicalSignals[0].type = 'trainCover'
    byId(c.signals, 'sig-v').frames = [{ kind: 'plate', aspects: [{ aspect: 'Ne_1' }] }]
  },
  (c) => { byId(c.signals, 'sig-a').logicalSignals.push({ system: 'sh', type: 'shunting', frames: [{ kind: 'screen', aspects: [{ aspect: 'Sh_1' }] }] }) },
  (c) => { byId(c.signals, 'sig-a').logicalSignals.push({ system: 'ks', type: 'main' }) },
  (c) => { delete byId(c.signals, 'sig-a').logicalSignals[0].type },
  (c) => {
    const a = byId(c.signals, 'sig-a')
    Object.assign(a, { frames: a.logicalSignals[0].frames, function: 'entry' })
    delete a.logicalSignals
  },
  (c) => { byId(c.signals, 'sig-n1').frames.push({ kind: 'indicator', aspects: [{ aspect: 'Zs_1' }] }) },
  (c) => { byId(c.signals, 'sig-a').logicalSignals[0].signallingSpeeds.push({ routeId: 'rt-n1-v', speed: 80 }, { routeId: 'rt-a-n1', speed: 40 }) },
  (c) => { byId(c.signals, 'sig-a').logicalSignals[0].settings = { Nf: true } },
  (c) => { byId(c.signals, 'sig-ne1').frames[0].aspects = [{ aspect: 'Ne_99' }] },
  (c) => { byId(c.signals, 'sig-ne1').frames = [{ kind: 'plate', aspects: [{ aspect: 'Zs_3', symbol: '8' }] }] },
  (c) => { byId(c.signals, 'sig-ne1').frames = [{ kind: 'plate', aspects: [{ aspect: 'Ms_ws_rt_ws' }] }] },
  (c) => {
    c.signals.push({
      id: 'sig-gz', kind: 'real', at: { anchor: 'station', trackId: 't2', station: 20, direction: 'down' },
      alsoAt: [{ anchor: 'station', trackId: 't9', station: 20 }], frames: [{ kind: 'body', aspects: [{ aspect: 'Ra_12' }] }],
    })
    byId(c.switchElements, 'we-1').clearanceMarkerId = 'sig-gz'
  },
  (c) => {
    byId(c.routes, 'rt-a-n1').path = { trackIds: ['t2'] }
    byId(c.overlaps, 'ov-n1').signalId = 'sig-a'
  },
  (c) => { byId(c.overlaps, 'ov-n1').path = { trackIds: ['t2'] } },
  (c) => { byId(c.routes, 'rt-a-n1').overlaps = [] },
  (c) => { Object.assign(byId(c.routes, 'rt-a-n1'), { kind: 'shunting', shuntingType: 'RR', trainType: undefined }) },
  (c) => {
    const r = byId(c.routes, 'rt-a-n1')
    r.aspects.push({ signalId: 'sig-a', aspect: 'Zs_3', symbol: '4' })
    r.overlaps.push({ overlapId: 'ov-n1', preferred: true })
  },
  (c, core) => { t3(core); byId(c.overlaps, 'ov-n1').end = { anchor: 'station', trackId: 't3', station: 10 } },
  (c, core) => {
    t3(core)
    core.collections.switches[0].ports = { A: { trackId: 't2', endpoint: 'BEGIN' }, B1: { trackId: 't3', endpoint: 'BEGIN' } }
    byId(c.overlaps, 'ov-n1').end = { anchor: 'station', trackId: 't3', station: 10 }
  },
  overBranch(60, 80), overBranch(60, 60), overBranch(60, 40), overBranch(60, null), overBranch(null, 80),
  (c) => { c.flankProtection.push({ id: 'fla-3', requester: { switchElementId: 'we-1', via: 'left' }, protection: { switchElementId: 'we-1', position: 'right' } }) },
  (c) => {
    delete byId(c.balises, 'pzb-gue').settings.testSpeed
    byId(c.balises, 'pzb-a-1000').settings.testSpeed = 40
    byId(c.balises, 'pzb-n1-2000').settings.frequency = '1500'
    byId(c.balises, 'etcs-n1').settings.position = 9
    delete byId(c.balises, 'pzb-n1-500').signalId
  },
  (c) => { byId(c.balises, 'pzb-a-1000').system = 'lzb'; byId(c.balises, 'etcs-n1').kind = 'magnet' },
  (c) => { byId(c.signals, 'sig-a').logicalSignals[0].signallingSpeeds[0].speed = 80; delete byId(c.overlaps, 'ov-n1').speed },
  (c) => { byId(c.signals, 'sig-a').logicalSignals[0].signallingSpeeds[0].speed = 80 },
  (c) => { byId(c.overlaps, 'ov-n1').gradient = -20 },
  (c) => {
    Object.assign(byId(c.signals, 'sig-a').logicalSignals[0], {
      nextSystems: ['ks', 'bal'], defaultParameters: { jaune_cli: 'false' }, conditionalParameters: [{ routeId: 'rt-a-n1', parameters: {} }],
    })
  },
  (c) => { byId(c.stationBoundaries, 'bd-1').marks = [{ signalId: 'sig-ne1' }, { switchId: 'w1' }, { platformId: 'pl1' }] },
  (c) => { byId(c.stationBoundaries, 'bd-1').marks = [{ signalId: 'sig-n1' }] },
  (c) => { byId(c.stoppingPlaces, 'hp-1').at.startStation = 10; byId(c.stoppingPlaces, 'hp-2').operationalPointId = undefined },
  (c) => { byId(c.stoppingPlaces, 'hp-1').at.endStation = 5 },
  (c) => { byId(c.itineraries, 'fw-1').routeIds = ['rt-n1-v', 'rt-a-n1']; byId(c.itineraries, 'fw-1').targetStoppingPlaceId = 'hp-1' },
  (c) => { delete byId(c.signals, 'sig-a').operationalPointId },
  (c) => {
    c.switchElements.push({ id: 'gs-1', name: 'Gs1', operation: 'electricRemote', derailer: { at: { anchor: 'station', trackId: 't2', station: 200, direction: 'up' }, preferredPosition: 'applied' } })
    c.flankProtection.push({ id: 'fla-3', requester: { switchElementId: 'we-1', via: 'tip' }, protection: { switchElementId: 'gs-1' } })
    byId(c.flankProtection, 'fla-2').protection = { switchElementId: 'we-2' }
    c.switchElements.push({ id: 'we-2', switchId: 'w1', part: 'b', operation: 'electricRemote' })
  },
  (c) => {
    c.detectionPoints.push({ id: 'dp-3', kind: 'axleCounter', at: { anchor: 'station', trackId: 't1', station: 235 } })
    c.trackSections.push({ id: 'ts-2', kind: 'axleCounting', boundedBy: ['dp-3', 'dp-2'] })
  },
  (c) => {
    c.detectionPoints.push({ id: 'dp-3', kind: 'axleCounter', at: { anchor: 'station', trackId: 't2', station: 145 } })
    c.trackSections[0].boundedBy = ['dp-1', 'dp-3']
  },
  (c) => { c.trackSections[0].boundedBy = ['dp-1'] },
  // More than the tests: no signalling, an empty one, a station without its core's switches.
  (c) => { for (const k of Object.keys(c)) c[k] = [] },
  (c, core) => { core.collections.switches = [] },
  (c) => { for (const s of c.signals) delete s.lifespan; c.routes.reverse() },
]

function stationCases() {
  return CHANGES.map((change) => {
    const signalling = structuredClone(ksStation())
    const core = ksStationCore()
    change(signalling.collections, core)
    return rec('signalling.checkSignalling', S.checkSignalling, { core, signalling })
  })
}

function catalogueCases() {
  const cat = S.SIGNAL_CATALOGUE
  const out = [rec('signalling.CATALOGUE_REF', () => S.CATALOGUE_REF)]
  const aspects = cat.begriffe.map(b => b.id)
  for (const key of [...aspects, 'Zs_99', '', 'constructor']) {
    out.push(rec('signalling.aspectOf', S.aspectOf, key))
    out.push(rec('signalling.boardAspect', S.boardAspect, key))
  }
  for (const b of cat.begriffe) {
    for (const frame of ['screen', 'indicator', 'plate', 'body', 'nameplate', undefined]) out.push(rec('signalling.aspectFitsFrame', S.aspectFitsFrame, b, frame))
    for (const symbol of [undefined, ...(b.symbole ?? []).slice(0, 3), '17', '4,5', 'x']) {
      out.push(rec('signalling.symbolFits', S.symbolFits, b, symbol))
      out.push(rec('signalling.aspectSpeed', S.aspectSpeed, b, symbol))
    }
    for (const lang of ['de', 'en', 'fr']) out.push(rec('signalling.titleIn', S.titleIn, b, lang))
  }
  out.push(rec('signalling.aspectSpeed', S.aspectSpeed, undefined, '4'))
  out.push(rec('signalling.titleIn', S.titleIn, undefined, 'de'))
  const systems = [...Object.keys(cat.systeme), 'hv', 'bal', '']
  for (const system of systems) {
    out.push(rec('signalling.knownSystem', S.knownSystem, system))
    const kinds = Object.keys(cat.systeme[system]?.signalarten ?? {})
    for (const type of [...kinds, 'trainCover', 'shunting', undefined]) {
      out.push(rec('signalling.signalKindOf', S.signalKindOf, system, type))
      out.push(rec('signalling.screenOf', S.screenOf, [{ system, type }]))
    }
  }
  out.push(rec('signalling.screenOf', S.screenOf, [{ system: 'ks', type: 'main' }, { system: 'sh', type: 'shunting' }]))
  out.push(rec('signalling.screenOf', S.screenOf, []))
  const protection = cat.zugbeeinflussung.systeme
  for (const system of [...Object.keys(protection), 'lzb']) {
    out.push(rec('signalling.protectionSystemOf', S.protectionSystemOf, system))
    const sys = protection[system]
    for (const kind of [...Object.keys(sys?.arten ?? {}), 'magnet', 'balise']) {
      out.push(rec('signalling.baliseKindOf', S.baliseKindOf, system, kind))
      const k = sys?.arten?.[kind]
      const declared = sys?.einstellungen
      for (const settings of [undefined, {}, { frequency: '1000' }, { frequency: '1500', testSpeed: 40 }, { position: 9, ina: true }, { name: '' }, { measuringLength: 20, testSpeed: 60, arrangement: 'x' }]) {
        out.push(rec('signalling.settingsProblems', S.settingsProblems, settings, declared, { pflicht: k?.pflicht, erlaubt: k?.erlaubt }))
      }
    }
    for (const [, setting] of Object.entries(sys?.einstellungen ?? {})) {
      for (const v of ['1000', 'x', '', 5, -1, 1e9, NaN, true, false, null]) out.push(rec('signalling.settingFits', S.settingFits, setting, v))
    }
  }
  out.push(rec('signalling.settingsProblems', S.settingsProblems, { a: 1 }, undefined, {}))
  return out
}

function overlapCases() {
  const out = []
  for (const speed of [null, 0, 10, 30, 40, 41, 60, 80, 100, 120, 160, 200, 300]) {
    for (const gradient of [undefined, 0, -5, -10, -15, -20, -25, -40, 25]) out.push(rec('signalling.overlapLength', S.overlapLength, speed, gradient))
  }
  const s = ksStation()
  for (const route of [{ id: 'rt-a-n1', startSignalId: 'sig-a' }, { id: 'rt-x', startSignalId: 'sig-a' }, { id: 'rt-n1-v', startSignalId: 'sig-n1' }, { id: 'rt-a-n1', startSignalId: 'nope' }]) {
    out.push(rec('signalling.signallingSpeedOf', S.signallingSpeedOf, s, route))
  }
  const tracks = [
    { id: 't1', heights: [{ station: 0, z: 100 }, { station: 250, z: 99 }] },
    { id: 't2', heights: [{ station: 0, z: 99 }, { station: 150, z: 98.5 }] },
  ]
  out.push(rec('signalling.gradientAlong', S.gradientAlong, tracks, { trackId: 't1', station: 230 }, { trackId: 't2', station: 100 }, 120))
  out.push(rec('signalling.gradientAlong', S.gradientAlong, tracks, { trackId: 't2', station: 0 }, { trackId: 't1', station: 10 }, 0))
  out.push(rec('signalling.gradientAlong', S.gradientAlong, [{ id: 't1' }], { trackId: 't1', station: 0 }, { trackId: 't1', station: 10 }, 10))
  out.push(rec('signalling.gradientAlong', S.gradientAlong, tracks, { trackId: 't9', station: 0 }, { trackId: 't1', station: 10 }, 10))
  const sw = (id, speed, a, b1, b2) => ({ id, kind: 'turnout', speed, ports: { A: { trackId: a }, B1: { trackId: b1 }, B2: { trackId: b2 } } })
  const switches = [sw('w1', 60, 'a', 'l', 'm'), sw('w2', 40, 'x', 'l', 'y'), { switchId: 'w3', speed: 100, ports: { A: { trackId: 'm' }, B1: { trackId: 'n' } } }]
  for (const run of [['a', 'm'], ['a', 'l'], ['x', 'l', 'a'], ['a', 'm', 'n'], ['l', 'a'], ['n', 'm', 'a'], [], ['a']]) {
    out.push(rec('signalling.branchSpeedOnRun', S.branchSpeedOnRun, switches, run))
  }
  out.push(rec('signalling.branchSpeedOnRun', S.branchSpeedOnRun, [], ['a', 'l']))
  for (const label of ['500 – 1:12', '300-1:9', '190-1:7.5', '1200-1:18.5', undefined]) {
    out.push(rec('signalling.branchSpeedOnRun', S.branchSpeedOnRun, [{ ...switches[0], speed: undefined, label }], ['a', 'l']))
  }
  return out
}

function sectionCases() {
  const EPSG = 25832
  const P = (x, y = 0) => ({ easting: 500000 + x, northing: 5700000 + y, zone: EPSG })
  const track = (id, a, b) => ({ id, epsg: EPSG, elements: [straightElement(P(...a), P(...b))] })
  const core = {
    fields: {},
    collections: {
      tracks: [track('t1', [0, 0], [200, 0]), track('t2', [200, 0], [400, 0]), track('t3', [200, 0], [380, 40])],
      switches: [{ id: 'w1', switchId: 'w1', kind: 'turnout', formVersion: 1, ports: {
        A: { trackId: 't1', endpoint: 'END' }, B1: { trackId: 't3', endpoint: 'BEGIN' }, B2: { trackId: 't2', endpoint: 'BEGIN' },
      } }],
    },
  }
  const point = (id, trackId, station) => ({ id, kind: 'axleCounter', at: { anchor: 'station', trackId, station } })
  const signalling = (sections, extra = []) => ({
    fields: {},
    collections: { detectionPoints: [point('a', 't1', 150), point('b', 't2', 50), point('c', 't3', 60), point('d', 't2', 180), ...extra], trackSections: sections },
  })
  const apart = { ...core, collections: { ...core.collections, tracks: [...core.collections.tracks, track('t4', [0, 500], [100, 500])] } }
  const out = []
  const cases = [
    [core, signalling([])],
    [core, signalling([{ id: 'W1', kind: 'axleCounting', boundedBy: ['a', 'b', 'c'] }])],
    [core, signalling([{ id: 'W1', kind: 'axleCounting', boundedBy: ['a', 'b'] }, { id: 'G2', kind: 'axleCounting', boundedBy: ['b', 'd'] }])],
    [apart, signalling([{ id: 'X', kind: 'axleCounting', boundedBy: ['a', 'e'] }], [point('e', 't4', 50)])],
    [apart, signalling([{ id: 'Y', kind: 'axleCounting', boundedBy: ['zz'] }, { id: 'Z', kind: 'axleCounting', boundedBy: [] }])],
    [ksStationCore(), ksStation()],
  ]
  for (const [c, s] of cases) {
    out.push(rec('signalling.netParts', S.netParts, { core: c, signalling: s }))
    out.push(rec('signalling.netParts', S.netParts, { core: c, signalling: s }, new Set(['a', 'b'])))
    out.push(rec('signalling.trackSectionExtents', S.trackSectionExtents, { core: c, signalling: s }))
  }
  return out
}

export default function record() {
  return { check: stationCases(), catalogue: catalogueCases(), overlaps: overlapCases(), sections: sectionCases() }
}
