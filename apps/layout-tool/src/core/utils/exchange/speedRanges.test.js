import { describe, it, expect } from 'vitest'
import { averageSpeed, speedProfile, speedRanges, speedRangesCsv, speedRangesSvg } from './speedRanges'

/** A run of parts, each a track of elements [length, speed], `reversed` where it is run against its stationing. */
const runOf = (...parts) => {
  let offset = 0
  return {
    ok: true,
    parts: parts.map(({ els, reversed = false }) => {
      const length = els.reduce((s, [l]) => s + l, 0)
      const part = { track: { elements: els.map(([l, speed]) => ({ length: l, ...(speed != null ? { speed } : {}) })) }, reversed, offset, length }
      offset += length
      return part
    }),
  }
}
const round = (ranges) => ranges.map(r => [Math.round(r.from), Math.round(r.to), r.speed])

describe('speed ranges along a run', () => {
  it('reads the design speed along the run, a track run backwards from its end, equal speeds joined', () => {
    const run = runOf({ els: [[400, 100], [600, 100]] }, { els: [[300, 80], [700, 120]], reversed: true })
    expect(round(speedProfile(run))).toEqual([[0, 1000, 100], [1000, 1700, 120], [1700, 2000, 80]])
  })

  it('lays ranges of at least 800 m for the highest average speed, each at the lowest design speed in it', () => {
    // A 300 m stretch at 60 between 100 and 120: its range is as short as allowed, and lies where it costs least.
    const profile = speedProfile(runOf({ els: [[1000, 100], [300, 60], [1700, 120]] }))
    const ranges = speedRanges(profile)
    expect(round(ranges)).toEqual([[0, 800, 100], [800, 1600, 60], [1600, 3000, 120]])
    // The least running time: 800/100 + 800/60 + 1400/120, against every other way to lay them.
    expect(averageSpeed(ranges)).toBeCloseTo(3000 / (800 / 100 + 800 / 60 + 1400 / 120), 9)
    expect(averageSpeed(ranges)).toBeGreaterThan(averageSpeed([{ from: 0, to: 1600, speed: 60 }, { from: 1600, to: 3000, speed: 120 }]))
  })

  it('keeps a range as long as it likes, and every one at least the minimum', () => {
    const profile = speedProfile(runOf({ els: [[5000, 160], [200, 100], [3000, 160]] }))
    const ranges = speedRanges(profile)
    expect(ranges.every(r => r.to - r.from >= 800 - 1e-6)).toBe(true)
    expect(ranges.map(r => r.speed)).toEqual([160, 100, 160])
    expect(ranges[1].to - ranges[1].from).toBeCloseTo(800, 6)
    // A shorter minimum gives the slow stretch a shorter range.
    expect(round(speedRanges(profile, { min: 200 }))).toEqual([[0, 5000, 160], [5000, 5200, 100], [5200, 8200, 160]])
  })

  it('makes a run shorter than the minimum one range, and lets a stretch without a design speed limit nothing', () => {
    expect(round(speedRanges(speedProfile(runOf({ els: [[300, 80], [200, 60]] }))))).toEqual([[0, 500, 60]])
    expect(round(speedRanges(speedProfile(runOf({ els: [[1000, 120], [500, null], [1000, 120]] }))))).toEqual([[0, 2500, 120]])
    expect(round(speedRanges(speedProfile(runOf({ els: [[1000, null]] }))))).toEqual([[0, 1000, null]])
  })

  it('finds the least running time every way of laying the ranges on the grid gives', () => {
    // Random profiles on a 100 m grid, against every partition into ranges of at least 800 m.
    let seed = 7
    const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647)
    const time = (profile, a, b) => {
      const v = Math.min(...profile.filter(p => p.to > a && p.from < b).map(p => p.speed))
      return (b - a) / v
    }
    const best = (profile, end, a = 0) => {
      if (end - a < 800 - 1e-9) return Infinity
      let t = time(profile, a, end)
      for (let b = a + 800; b <= end - 800 + 1e-9; b += 100) t = Math.min(t, time(profile, a, b) + best(profile, end, b))
      return t
    }
    for (let k = 0; k < 12; k++) {
      const els = Array.from({ length: 3 + Math.floor(rnd() * 4) }, () => [100 * (1 + Math.floor(rnd() * 8)), [40, 60, 80, 100, 120, 160][Math.floor(rnd() * 6)]])
      const profile = speedProfile(runOf({ els }))
      const end = profile[profile.length - 1].to
      if (end < 800) continue
      const ranges = speedRanges(profile, { step: 100 })
      const got = ranges.reduce((t, r) => t + (r.to - r.from) / r.speed, 0)
      expect(got).toBeCloseTo(best(profile, end), 9)
    }
  })

  it('gives the ranges out as a CSV table and an SVG diagram', () => {
    const ranges = [{ from: 0, to: 800, speed: 100, kmFrom: 29812, kmTo: 30612 }, { from: 800, to: 1650.5, speed: 60, kmFrom: 30612, kmTo: null }]
    const texts = { kmFrom: 'von km', kmTo: 'bis km', from: 'von', to: 'bis', length: 'Länge', speed: 'v' }
    expect(speedRangesCsv(ranges, texts)).toBe('von km;bis km;von;bis;Länge;v\r\n29,812;30,612;0,0;800,0;800,0;100\r\n30,612;;800,0;1650,5;850,5;60\r\n')
    const svg = speedRangesSvg([{ from: 0, to: 1650.5, speed: 60 }], ranges, { title: 'T & <x>', subtitle: 's', speedAxis: 'v', legendDesign: 'd', legendRanges: 'r' })
    expect(svg.startsWith('<svg xmlns="http://www.w3.org/2000/svg"')).toBe(true)
    expect(svg).toContain('T &amp; &lt;x&gt;')
    expect(svg).toContain('km 29,812')
  })
})
