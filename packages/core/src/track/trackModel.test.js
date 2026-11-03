import { describe, it, expect } from 'vitest'
import { inheritedSpeed } from './trackModel.js'

describe('inheritedSpeed', () => {
  const track = (...speeds) => ({ elements: speeds.map(speed => ({ elementType: 0, speed })) })

  it('takes the last element\'s speed', () => {
    expect(inheritedSpeed(track(60, 100))).toBe(100)
  })

  it('looks back past elements that state none', () => {
    expect(inheritedSpeed(track(60, 120, 0, undefined))).toBe(120)
  })

  it('is a turnout\'s on its branch, or the element\'s own where that is slower', () => {
    const switches = [{ id: 'w1', speed: 60 }, { id: 'w2' }]
    const branch = (id, speed) => ({ elementType: 1, switchBranch: true, switchRoute: 'branch', switchId: id, speed })
    expect(inheritedSpeed({ elements: [{ elementType: 0, speed: 100 }, branch('w1')] }, switches)).toBe(60)
    expect(inheritedSpeed({ elements: [branch('w1', 120)] }, switches)).toBe(60)
    expect(inheritedSpeed({ elements: [branch('w1', 40)] }, switches)).toBe(40)
    // Past the turnout what comes after it counts, and a turnout without a speed tells nothing.
    expect(inheritedSpeed({ elements: [branch('w1'), { elementType: 0, speed: 80 }] }, switches)).toBe(80)
    expect(inheritedSpeed({ elements: [{ elementType: 0, speed: 100 }, branch('w2')] }, switches)).toBe(100)
    // Its through route runs at the line's.
    expect(inheritedSpeed({ elements: [{ elementType: 0, speed: 100 }, { ...branch('w1'), switchRoute: 'main' }] }, switches)).toBe(100)
    expect(inheritedSpeed({ elements: [branch('w1')] })).toBeNull()
  })

  it('is null when no element states one', () => {
    expect(inheritedSpeed(track(0, undefined))).toBeNull()
    expect(inheritedSpeed({ elements: [] })).toBeNull()
    expect(inheritedSpeed(null)).toBeNull()
  })
})
