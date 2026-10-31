import { describe, it, expect } from 'vitest'
import { domainChanges, localChangeCount } from './changes.js'

const track = (id, extra = {}) => ({ id, name: id, epsg: 25832, elements: [{ elementType: 0, length: 100 }], ...extra })
const signalling = (signals) => ({ fields: {}, collections: { signals } })

describe('the changes a working copy holds', () => {
  it('counts the signalling object by object, and the tracks as the project\'s diff does', () => {
    const base = { id: 'p', tracks: [track('t1')], signalling: signalling([{ id: 's1', name: 'A' }]) }
    expect(localChangeCount({ basePayload: base, project: base })).toBe(0)
    const project = {
      ...base, tracks: [track('t1', { name: 'neu' }), track('t2')],
      signalling: signalling([{ id: 's1', name: 'B' }, { id: 's2' }]),
    }
    expect(localChangeCount({ basePayload: base, project })).toBe(4)
    expect(localChangeCount(null)).toBe(0)
  })

  it('counts what was added, removed and changed in a domain, its fields as one', () => {
    expect(domainChanges(undefined, signalling([{ id: 'a' }]))).toBe(1)
    expect(domainChanges(signalling([{ id: 'a' }]), signalling([]))).toBe(1)
    expect(domainChanges({ fields: { x: 1 }, collections: {} }, { fields: { x: 2 }, collections: {} })).toBe(1)
  })
})
