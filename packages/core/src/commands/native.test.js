import { describe, expect, it } from 'vitest'
import { hydrateProjects } from '../project/persistenceUtils.js'
import { planElementChange } from './index.js'
import { mergeProject } from '../merge/index.js'
import { callNative } from '../native/index.js'
import { loadNativeFile } from '../native/node.js'
import { hasPekBestand, loadPekBestand } from '../../test/pekFixture.js'

// The commands and the merge in Rust (decision 0020, step 5), held to the
// JavaScript on real planning data at run time — the edit as a drag makes
// it, on the project the module holds (session.*), so only what changed
// crosses the boundary.

const strip = (tracks) => tracks.map(t => ({ id: t.id, elements: t.elements.map(e => [e.length, e.bearing, e.startNode, e.endNode]) }))
const close = (a, b) => JSON.stringify(a, (k, v) => (typeof v === 'number' ? Math.round(v * 1e6) / 1e6 : v))
  === JSON.stringify(b, (k, v) => (typeof v === 'number' ? Math.round(v * 1e6) / 1e6 : v))

describe.skipIf(!hasPekBestand || !loadNativeFile())('the commands in Rust, on the PEK Bestand', { timeout: 120000 }, () => {
  it('edits an element on the held project as the JavaScript edits the whole one', () => {
    const [p] = hydrateProjects([loadPekBestand()])
    callNative('session.open', [{ tracks: p.tracks, switches: p.switches }])
    try {
      for (const track of p.tracks.filter(t => t.elements.length > 3).slice(0, 12)) {
        for (const i of [0, 1, track.elements.length - 1]) {
          const change = { length: track.elements[i].length + 0.25 }
          const js = planElementChange(p.tracks, p.switches, track.id, i, change)
          const rs = callNative('session.planElementChange', [track.id, i, change])
          expect(rs.touchedTrackIds).toEqual(js.touchedTrackIds)
          expect(rs.touchedSwitchIds).toEqual(js.touchedSwitchIds)
          expect(rs.error).toBe(js.error)
          expect(close(strip(rs.tracks), strip(js.tracks.filter(t => js.touchedTrackIds.includes(t.id))))).toBe(true)
        }
      }
    } finally {
      callNative('session.close')
    }
  })

  it('merges a state with itself to nothing', () => {
    const [p] = hydrateProjects([loadPekBestand()])
    const rs = callNative('merge.mergeProject', [{ base: p, mine: p, theirs: p }])
    const js = mergeProject({ base: p, mine: p, theirs: p })
    expect(rs.conflicts.length).toBe(js.conflicts.length)
    expect(rs.applied).toEqual(JSON.parse(JSON.stringify(js.applied)))
    expect(rs.merged.tracks.length).toBe(js.merged.tracks.length)
  })
})
