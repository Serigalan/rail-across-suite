import { describe, it, expect } from 'vitest'
import { nearestValid, settleStation } from './index.js'
import { reuseFrom, viaRust } from '../native/index.js'

// Steps of the switch module, as the core's module runs them (not part of its face).
const uniqueStations = viaRust('switch.uniqueStations', reuseFrom)

describe('the position slider', () => {
  // Finding the valid stretches asks the dialog back (valid(s)); the module does that inside
  // switchOnTrackSlider and crossingOnTrackSlider, tested with them (commands).
  it('skips what is not valid: the nearest valid station', () => {
    const iv = [[10, 40], [60, 70]]
    expect(nearestValid(45, iv)).toBe(40)
    expect(nearestValid(56, iv)).toBe(60)
    expect(nearestValid(20, iv)).toBe(20)
    expect(nearestValid(5, [])).toBeNull()
  })

  it('snaps within the tolerance onto a valid snap point, never onto an invalid one', () => {
    const iv = [[10, 40], [60, 70]]
    expect(settleStation(24.4, { intervals: iv, snaps: [25], tol: 1 })).toBe(25)
    expect(settleStation(23.5, { intervals: iv, snaps: [25], tol: 1 })).toBe(23.5)
    expect(settleStation(49.5, { intervals: iv, snaps: [50], tol: 1 })).toBe(40)
  })

  it('states snap points once each, ascending — the first of two within 0.1 mm, as it is', () => {
    expect(uniqueStations([3, 1.00001, 1.000012, NaN, 2])).toEqual([1.00001, 2, 3])
    // A node's own station, not one rounded off it: what settles on it leaves no sliver.
    expect(uniqueStations([470.81838944, 470.8184])).toEqual([470.81838944])
  })
})
