// The survey crate's vectors (crates/survey): a measured axis made from a
// trace, a reference axis read off an alignment with a gradient, lines beside
// it compared with it, a point file in the forms it comes in, and the fit's
// answer of the fixture turned into a request, a report and a track.
import { readFileSync } from 'node:fs'
import * as S from '../src/survey/index.js'
import { arcFrom, straightFrom, transitionElement } from '../src/geometry/index.js'
import { rec, rng } from './record.mjs'

const EPSG = 25832
const chain = (start, bearing, parts) => {
  let at = { ...start, zone: EPSG }
  let b = bearing
  return parts.map(([kind, length, r1, r2]) => {
    const el = kind === 's' ? straightFrom(at, b, length)
      : kind === 'a' ? arcFrom(at, b, length, r1)
        : transitionElement(at, b, length, r1, r2, { transitionType: 'clothoid' }).element
    at = { easting: el.endNode[0], northing: el.endNode[1], zone: EPSG }
    b = el.endBearing ?? el.bearing
    return el
  })
}

const fixture = JSON.parse(readFileSync(new URL('../test/fixtures/align_answer.json', import.meta.url), 'utf8'))

export default function record() {
  const rand = rng(7070)
  const calls = []
  const r = (op, fn, ...args) => calls.push(rec(`survey.${op}`, fn, ...args))
  const thrown = (op, fn, ...args) => {
    try { calls.push(rec(`survey.${op}`, fn, ...args)) } catch (err) {
      calls.push({ op: `survey.${op}`, input: JSON.parse(JSON.stringify(args)), error: { message: err.message } })
    }
  }

  // A measured axis: a trace of 120 points with noise, gaps and doubtful points.
  const tracePoints = Array.from({ length: 120 }, (_, i) => {
    const z = 101.234 + i * 0.0021 + rand() * 0.002
    return {
      station: 12.5 + i * 0.5, easting: 600123.4567 + i * 0.45 + rand() * 0.003, northing: 5600456.789 + i * 0.21 - rand() * 0.003,
      zLeft: z + (i > 60 ? 0.06 : 0), zRight: z + rand() * 0.001, gauge: 1.435 + rand() * 0.004, quality: rand() > 0.15 ? 'good' : 'doubtful',
    }
  })
  const meta = { id: 'sv1', name: 'Achse 1', rail: 'left', guide: { kind: 'track', trackId: 't1' }, cloudNames: ['a.laz'], createdAt: '2026-01-02T03:04:05Z', step: 0.5 }
  r('surveyFromTrace', S.surveyFromTrace, meta, { points: tracePoints, gaps: [{ from: 20, to: 21.5, extra: 1 }], epsg: EPSG })
  r('surveyFromTrace', S.surveyFromTrace, { id: 'empty' }, { points: [], gaps: [], epsg: EPSG })
  const survey = S.surveyFromTrace(meta, { points: tracePoints, gaps: [], epsg: EPSG })
  r('surveyPoints', S.surveyPoints, survey)
  r('surveyPoints', S.surveyPoints, { points: null })
  r('withoutPoints', S.withoutPoints, survey, [0, 5, 6, 119])
  r('surveyStats', S.surveyStats, survey)
  r('surveyStats', S.surveyStats, {})
  for (const bad of [survey, { ...survey, id: '' }, { ...survey, epsg: 'x' }, { ...survey, points: null },
    { ...survey, points: { ...survey.points, zr: [1] } }, { ...survey, points: { ...survey.points, q: null } }]) r('surveyDefect', S.surveyDefect, bad)
  r('surveyAxis', S.surveyAxis, survey)

  // A reference axis along straight, transition, arc, transition, straight.
  const els = chain({ easting: 600000, northing: 5600000 }, 70, [['s', 120], ['t', 40, null, 600], ['a', 90, 600], ['t', 40, 600, null], ['s', 150]])
  const gradient = [{ station: 1000, z: 100 }, { station: 1150, z: 101.5, rv: 5000 }, { station: 1300, z: 100.8, rv: 4000 }, { station: 1440, z: 101.2 }]
  for (const [from, to, max] of [[0, 1500, undefined], [1000, 1440, 200], [0, 6789.5, 2000], [100, 101, 50], [0, 4000, 2000]]) r('referenceStretches', S.referenceStretches, from, to, max)
  const refArgs = { name: 'R', epsg: EPSG, elements: els, startStation: 1000, from: 1000, to: 1440, gradient, heightEpsg: 7837, tra: 'r.tra', gra: 'r.gra', importedAt: '2026-01-01T00:00:00Z', id: 'ref1' }
  r('buildReferenceAxis', S.buildReferenceAxis, refArgs)
  r('buildReferenceAxis', S.buildReferenceAxis, { ...refArgs, from: 1012.345, to: 1301.234, gradient: gradient.slice(1, 3), id: 'ref2' })
  r('buildReferenceAxis', S.buildReferenceAxis, { ...refArgs, gradient: null, gra: 'x', step: 0.05, id: 'ref3' })
  thrown('buildReferenceAxis', S.buildReferenceAxis, { ...refArgs, from: 1300, to: 1200 })
  thrown('buildReferenceAxis', S.buildReferenceAxis, { ...refArgs, from: 2000, to: 2100 })
  const ref = S.buildReferenceAxis(refArgs)
  const refFlat = S.buildReferenceAxis({ ...refArgs, gradient: null, id: 'ref3' })
  r('axisRange', S.axisRange, ref)
  r('axisOutline', S.axisOutline, ref, 5)
  r('axisOutline', S.axisOutline, ref, 0.37)
  for (const bad of [ref, { ...ref, id: null }, { ...ref, epsg: 'n' }, { ...ref, points: { ...ref.points, dn: [0] } },
    { ...ref, points: { ...ref.points, step: 0 } }, { ...ref, points: { ...ref.points, step: 10 } },
    { ...ref, points: { ...ref.points, z: { ...ref.points.z, i0: 44000 } } }]) r('referenceAxisDefect', S.referenceAxisDefect, bad)

  // Lines beside it: a few centimetres off, one leaving it, one with heights.
  const beside = chain({ easting: 600000.03, northing: 5599999.98 }, 70.01, [['s', 120], ['t', 40, null, 602], ['a', 90, 602], ['t', 40, 602, null], ['s', 150]])
  const leaving = chain({ easting: 600010, northing: 5600003.64 }, 70, [['s', 80], ['a', 200, -800]])
  const heights = [{ station: 0, z: 100.05 }, { station: 220, z: 101.9, rv: 3000 }, { station: 440, z: 101.1 }]
  r('comparedLine', S.comparedLine, beside, EPSG, { heights, station0: 0 })
  r('comparedLine', S.comparedLine, leaving, EPSG)
  r('comparedLine', S.comparedLine, [{ ...els[0], length: 0 }, els[1]], EPSG, { heights: [heights[0]], station0: 12 })
  const lineBeside = S.comparedLine(beside, EPSG, { heights })
  const lineLeaving = S.comparedLine(leaving, EPSG)
  for (const [axis, h] of [[ref, 7837], [ref, null], [ref, 5783], [refFlat, 7837], [{ ...ref, heightEpsg: null }, undefined]]) r('heightsComparable', S.heightsComparable, axis, h)
  const rowsBeside = S.shiftValues(ref, lineBeside, { every: 5 })
  r('shiftValues', S.shiftValues, ref, lineBeside, { every: 5 })
  r('shiftValues', S.shiftValues, ref, lineBeside, { every: 3.3, withHeights: false })
  r('shiftValues', S.shiftValues, ref, lineLeaving, { every: 1 })
  r('shiftValues', S.shiftValues, refFlat, lineBeside, {})
  r('shiftValues', S.shiftValues, ref, { e: [1], n: [2], s: [0], heights: null })
  r('referenceProfile', S.referenceProfile, rowsBeside, heights, 5)
  r('referenceProfile', S.referenceProfile, S.shiftValues(ref, lineLeaving, { every: 2 }), null, 2)
  const runs = S.referenceProfile(rowsBeside, heights, 5)
  for (const s of [0, 3.7, 55.55, 120, 239.9, 1000]) r('deviationAt', S.deviationAt, runs, s)
  r('shiftSummary', S.shiftSummary, rowsBeside, { limitQ: 0.03, limitZ: 0.05 })
  r('shiftSummary', S.shiftSummary, S.shiftValues(ref, lineLeaving, { every: 1 }), {})
  r('shiftSummary', S.shiftSummary, [], {})
  r('nearestAxis', S.nearestAxis, [refFlat, { ...ref, epsg: 25833 }, ref], lineBeside, EPSG)
  r('nearestAxis', S.nearestAxis, [ref], lineBeside, 25833)
  r('shiftValuesCsv', S.shiftValuesCsv, rowsBeside, { axisName: 'R', lineName: 'Neu' })
  r('shiftValuesCsv', S.shiftValuesCsv, S.shiftValues(ref, lineBeside, { every: 25, withHeights: false }))

  // Point files.
  const pts = tracePoints.slice(0, 14)
  const stageA = ['Nr;Station [m];Rechtswert [m];Hochwert [m];SO [m];Überhöhung [mm];Güte',
    ...pts.map((p, i) => [i + 1, p.station.toFixed(3), p.easting.toFixed(3).replace('.', ','), p.northing.toFixed(3), p.zRight.toFixed(4), i > 6 ? -40 : '', 'gut'].join(';'))].join('\r\n')
  r('parseAxisPointFile', S.parseAxisPointFile, stageA, 'achse_EPSG25832.csv')
  r('parseAxisPointFile', S.parseAxisPointFile, ['# comment', 'easting,northing,z', ...pts.map(p => [p.easting, p.northing, p.zLeft].join(','))].join('\n'), 'x epsg 31468.txt')
  r('parseAxisPointFile', S.parseAxisPointFile, pts.map((p, i) => [i + 7, p.easting, p.northing, p.zLeft].join('\t')).join('\n'), 'EPSG_99999')
  r('parseAxisPointFile', S.parseAxisPointFile, pts.map(p => `  ${p.easting}   ${p.northing}  `).join('\n'))
  r('parseAxisPointFile', S.parseAxisPointFile, ['"Rechts";"Hoch";"H"', ...pts.map(p => `${p.easting};${p.northing};`)].join('\n'))
  r('parseAxisPointFile', S.parseAxisPointFile, 'Station;Höhe\n1;2', '')
  r('parseAxisPointFile', S.parseAxisPointFile, '\n# nothing\n', '')
  r('parseAxisPointFile', S.parseAxisPointFile, pts.slice(0, 5).map(p => `${p.easting} ${p.northing}`).join('\n'))

  // The fit: request, report, track.
  const { answer, epsg } = fixture
  const axisPts = S.surveyAxis(survey)
  r('alignRequest', S.alignRequest, axisPts, {})
  r('alignRequest', S.alignRequest, [{ station: 3, easting: 1, northing: 2, z: 3, cant: -40 }, { easting: 4, northing: 5, cant: 0 }, { easting: 6, northing: 7, z: 8 }],
    { straights: S.straightRanges([{ from: 0, to: 80 }, { from: 120, to: 160 }]), settings: { ...S.ALIGN_DEFAULTS, chord: '12' } })
  r('alignRequest', S.alignRequest, [], { settings: { chord: 5 } })
  r('fittedCurvature', S.fittedCurvature, answer)
  r('fittedCurvature', S.fittedCurvature, { ...answer, elementStations: null })
  r('elementReport', S.elementReport, answer, 0.01)
  r('elementReport', S.elementReport, { ...answer, elementStats: null, elementCants: null }, 0.001)
  r('gradientReport', S.gradientReport, answer)
  r('gradientReport', S.gradientReport, { gradient: null })
  r('trackFromFit', S.trackFromFit, answer, { name: 'Achse Ist', epsg })
  r('trackFromFit', S.trackFromFit, { ...answer, elementCants: null, gradient: null }, { name: 'x', epsg: String(epsg) })
  const elementCants = [...answer.elementCants.slice(0, 3), { cant: null, measured: null, n: 0, cantEnd: -35 }]
  r('trackFromFit', S.trackFromFit, { ...answer, elements: answer.elements.slice(0, 4), elementCants }, { name: 'x', epsg })
  r('trackFromFit', S.trackFromFit, { ...answer, elements: null }, { name: 'x', epsg })

  // Straights by hand.
  const straights = [{ from: 0, to: 40 }, { from: 60, to: 90 }, { from: 130, to: 200 }]
  r('straightRanges', S.straightRanges, straights)
  r('straightRanges', S.straightRanges, null)
  for (const [i, side, at] of [[0, 'from', -5], [0, 'to', 70], [1, 'from', 30], [1, 'to', 61], [2, 'to', 500], [2, 'from', 199]]) r('moveStraightEnd', S.moveStraightEnd, straights, i, side, at)
  for (const [a, b] of [[100, 120], [120, 100], [35, 65], [95, 96], [-10, 300]]) r('insertStraight', S.insertStraight, straights, a, b)
  r('deleteStraight', S.deleteStraight, straights, 1)
  return { survey: calls }
}
