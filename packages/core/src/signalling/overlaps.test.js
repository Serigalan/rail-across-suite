import { describe, it, expect } from 'vitest'
import { gradientAlong, overlapLength, signallingSpeedOf } from './overlaps.js'
import { ksStation } from '../../test/signallingFixture.js'

describe('what an overlap needs', () => {
  it('asks the length of the speed\'s step, longer on a steep fall, never shorter uphill', () => {
    expect(overlapLength(30)).toBe(50)
    expect(overlapLength(40)).toBe(50)
    expect(overlapLength(60)).toBe(100)
    expect(overlapLength(160)).toBe(200)
    expect(overlapLength(160, -5)).toBe(200)
    expect(overlapLength(160, -25)).toBe(300)
    expect(overlapLength(160, 25)).toBe(200)
    expect(overlapLength(null)).toBeNull()
  })

  it('takes the speed a route\'s start signal signals for it', () => {
    const s = ksStation()
    expect(signallingSpeedOf(s, { id: 'rt-a-n1', startSignalId: 'sig-a' })).toBe(60)
    expect(signallingSpeedOf(s, { id: 'rt-x', startSignalId: 'sig-a' })).toBeNull()
  })

  it('takes the gradient over a run from the heights at its two ends, rising positive', () => {
    const tracks = [
      { id: 't1', heights: [{ station: 0, z: 100 }, { station: 250, z: 99 }] },
      { id: 't2', heights: [{ station: 0, z: 99 }, { station: 150, z: 98.5 }] },
    ]
    expect(gradientAlong(tracks, { trackId: 't1', station: 230 }, { trackId: 't2', station: 100 }, 120)).toBeCloseTo(-3.4, 1)
    expect(gradientAlong([{ id: 't1' }], { trackId: 't1', station: 0 }, { trackId: 't1', station: 10 }, 10)).toBeNull()
  })
})
