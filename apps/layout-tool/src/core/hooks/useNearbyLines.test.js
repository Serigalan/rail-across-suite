import { describe, expect, it } from 'vitest'
import { mergeLineOffers } from './useNearbyLines'
import { localLineDistances } from '../map/localKmLayer'

describe('the line numbers offered', () => {
  it('measures a line of the project from the middle of the view', () => {
    // A line running north along 11° E, the view 0.001° east of it at 51° N.
    const line = { lineNumber: 6399, runs: [[[11, 50.99, 0], [11, 51.01, 2200]]] }
    const [offer] = localLineDistances([line], { lng: 11.001, lat: 51 })
    expect(offer.lineNumber).toBe('6399')
    expect(offer.distance).toBeGreaterThan(65)
    expect(offer.distance).toBeLessThan(75)
  })

  it('offers the project\'s lines however far, the shown ones up to the limit', () => {
    const shown = [1, 2, 3].map(n => ({ lineNumber: String(n), distance: n * 10 }))
    const offers = mergeLineOffers(shown, [{ lineNumber: '9', distance: 5000 }], 2)
    expect(offers.map(o => o.lineNumber)).toEqual(['1', '2', '9'])
    expect(offers[2].local).toBe(true)
  })

  it('takes a line both shown and the project\'s as the project\'s, at the nearer distance', () => {
    const offers = mergeLineOffers([{ lineNumber: '6344', distance: 40 }], [{ lineNumber: '6344', distance: 12 }])
    expect(offers).toEqual([{ lineNumber: '6344', distance: 12, local: true }])
  })
})
