import { describe, expect, it } from 'vitest'
import { forwardUrl } from './suite.js'

describe('the Simulation-Tool\'s address', () => {
  it('leads on to computing a running time in the Fahrplan-Editor, on the same variant', () => {
    expect(forwardUrl('?project=p&variant=v')).toBe('../timetable/?project=p&section=runtime&variant=v')
    expect(forwardUrl('?project=p&variant=v&action=view')).toBe('../timetable/?project=p&section=runtime&variant=v&action=view')
    expect(forwardUrl('?project=p')).toBe('../timetable/?project=p&section=runtime')
    expect(forwardUrl('')).toBe('../#/timetable')
  })
})
