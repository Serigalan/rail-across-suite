import { describe, expect, it } from 'vitest'
import { proposedRoles, stageLinkNote } from './stageLink'

const end = (id, days) => ({ track: { id, ...days }, endpoint: 'END' })

describe('a construction stage connector (decision 0027)', () => {
  it('proposes the roles by the tracks\' days: the one going B1, the one coming B2, the one staying A', () => {
    const ends = [end('new', { status: 'new', inServiceFrom: '2028-06-02' }), end('stays', {}), end('old', { status: 'removal', inServiceTo: '2028-06-01' })]
    const roles = proposedRoles(ends)
    expect([roles.A.track.id, roles.B1.track.id, roles.B2.track.id]).toEqual(['stays', 'old', 'new'])
    expect(stageLinkNote(roles)).toBeNull()
  })

  it('says where the days do not meet: missing, overlapping, apart', () => {
    const roles = (until, from) => ({ A: end('a', {}), B1: end('b', { inServiceTo: until }), B2: end('c', { inServiceFrom: from }) })
    expect(stageLinkNote(roles(undefined, '2028-06-02'))).toBe('stage_link_note_days')
    expect(stageLinkNote(roles('2028-06-01', '2028-06-01'))).toBe('stage_link_note_overlap')
    expect(stageLinkNote(roles('2028-06-01', '2028-07-01'))).toBe('stage_link_note_gap')
    expect(stageLinkNote(roles('2028-12-31', '2029-01-01'))).toBeNull()
  })
})
