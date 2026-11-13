import { describe, expect, it } from 'vitest'
import { proposedRoles, stageLinkNote } from './stageLink'

const end = (id, lifespan) => ({ track: { id, ...(lifespan ? { lifespan } : {}) }, endpoint: 'END' })

describe('a construction stage connector (decision 0027)', () => {
  it('proposes the roles by the tracks\' days: the one going B1, the one coming B2, the one staying A', () => {
    const ends = [end('new', { from: '2028-06-02' }), end('stays'), end('old', { to: '2028-06-01' })]
    const roles = proposedRoles(ends)
    expect([roles.A.track.id, roles.B1.track.id, roles.B2.track.id]).toEqual(['stays', 'old', 'new'])
    expect(stageLinkNote(roles)).toBeNull()
  })

  it('says where the days do not meet: missing, overlapping, apart', () => {
    const roles = (until, from) => ({ A: end('a'), B1: end('b', { to: until }), B2: end('c', { from }) })
    expect(stageLinkNote(roles('open', '2028-06-02'))).toBe('stage_link_note_days')
    expect(stageLinkNote(roles('2028-06-01', '2028-06-01'))).toBe('stage_link_note_overlap')
    expect(stageLinkNote(roles('2028-06-01', '2028-07-01'))).toBe('stage_link_note_gap')
    expect(stageLinkNote(roles('2028-12-31', '2029-01-01'))).toBeNull()
    // removed and new with the days open: the roles still, the days to be stated
    const open = proposedRoles([end('old', { to: 'open' }), end('stays'), end('new', { from: 'open' })])
    expect([open.A.track.id, open.B1.track.id, open.B2.track.id]).toEqual(['stays', 'old', 'new'])
    expect(stageLinkNote(open)).toBe('stage_link_note_days')
  })
})
