import { describe, expect, it } from 'vitest'
import { dayOf, isDayKey, monthAfter, monthWeeks } from './index'

describe('the calendar of the point in time', () => {
  it('lays a month out Monday first, null outside it', () => {
    // June 2028 begins on a Thursday and has 30 days.
    const weeks = monthWeeks(2028, 6)
    expect(weeks[0]).toEqual([null, null, null, '2028-06-01', '2028-06-02', '2028-06-03', '2028-06-04'])
    expect(weeks.at(-1)).toEqual(['2028-06-26', '2028-06-27', '2028-06-28', '2028-06-29', '2028-06-30', null, null])
    expect(weeks.flat().filter(Boolean)).toHaveLength(30)
    expect(monthWeeks(2028, 2).flat().filter(Boolean)).toHaveLength(29)
  })

  it('steps months across years', () => {
    expect(monthAfter(2028, 12, 1)).toEqual([2029, 1])
    expect(monthAfter(2028, 1, -1)).toEqual([2027, 12])
    expect(monthAfter(2028, 6, -12)).toEqual([2027, 6])
  })

  it('tells a day from the existing and the final state', () => {
    expect(dayOf(2028, 6, 2)).toBe('2028-06-02')
    expect(isDayKey('2028-06-02')).toBe(true)
    expect(isDayKey('existing')).toBe(false)
    expect(isDayKey(null)).toBe(false)
  })
})
