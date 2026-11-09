// The vectors of the signalling domain's content (crates/signalling): the
// catalogue's lookups, overlaps and the speed a turnout allows a run, recorded
// from the JavaScript the page keeps. The check's and the track sections'
// vectors are frozen: their JavaScript is gone (decision 0020, step 10).
import * as S from '../src/signalling/index.js'
import { ksStation } from '../test/signallingFixture.js'
import { frozen, rec } from './record.mjs'

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

export default function record() {
  // The check's and the track sections' JavaScript is gone (step 10): their vectors stay as recorded.
  return { check: frozen('signalling', 'check'), catalogue: catalogueCases(), overlaps: overlapCases(), sections: frozen('signalling', 'sections') }
}
