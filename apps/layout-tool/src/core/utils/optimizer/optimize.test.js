import { describe, it, expect } from 'vitest'
import { reconstructElements } from '@rail-across/core/src/geometry/elementReconstruct.js'
import { expectValidTrack } from '@rail-across/core/test/chainInvariants.js'
import korbbogen from '../../test/fixtures/optimizer_service_korbbogen.json'
import { built, optimizer } from '../../test/optimizerModule'

// The optimizer as the panel runs it: the module of optimizer/ (Rust, decision
// 0019), as cargo built it — skipped where it has not been. The optimization
// itself is held to the service's test tracks in
// optimizer/crates/optimizer/tests/optimize.rs; here the answer has to come
// back as a track this app will take.
describe.skipIf(!built)('an optimized compound curve', () => {
  const { track } = korbbogen.request
  const optimized = built ? optimizer('optimize', korbbogen.request) : null

  it('rebuilds into a valid element chain', () => {
    const elements = reconstructElements(optimized.elements, track.epsg)
    expectValidTrack({ ...track, elements })
  })

  it('stays one group of two arcs turning the same way', () => {
    const arcs = optimized.elements.filter(el => el.radius != null)
    expect(arcs).toHaveLength(2)
    expect(new Set(optimized.report.map(r => r.group)).size).toBe(1)
    expect(optimized.report.map(r => r.arc)).toEqual([1, 2])
    expect(Math.sign(arcs[0].radius)).toBe(Math.sign(arcs[1].radius))
  })

  it('is faster than what it replaces, and keeps the track ends where they were', () => {
    const before = track.elements
    const after = optimized.elements
    expect(optimized.vNeu).toBeGreaterThan(optimized.vBestand)
    expect(after[0].startNode[0]).toBeCloseTo(before[0].startNode[0], 6)
    expect(after[0].startNode[1]).toBeCloseTo(before[0].startNode[1], 6)
    expect(after.at(-1).endNode[0]).toBeCloseTo(before.at(-1).endNode[0], 6)
    expect(after.at(-1).endNode[1]).toBeCloseTo(before.at(-1).endNode[1], 6)
  })

  // The rule a compound curve adds over a simple one: the cant step between two
  // arcs is carried by the ramp between them, measured against the step itself
  // rather than against zero.
  it('carries the cant step between the arcs on a long enough ramp', () => {
    const els = optimized.elements
    const first = els.findIndex(el => el.radius != null)
    const second = els.findIndex((el, i) => el.radius != null && i > first)
    const between = els.slice(first + 1, second)
    expect(between).toHaveLength(1)
    expect(between[0].elementType).toBe(2)

    const deltaU = Math.abs(Math.abs(els[first].cant) - Math.abs(els[second].cant))
    const k = between[0].transitionType === 'bloss' ? 6 : 8
    const v = Math.min(els[first].speed, els[second].speed)
    expect(between[0].length).toBeGreaterThanOrEqual(k * v * deltaU / 1000 - 1e-9)
  })
})
