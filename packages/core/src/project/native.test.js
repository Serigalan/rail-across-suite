import { describe, expect, it } from 'vitest'
import { contentHash, documentHash } from '../document/index.js'
import { dehydrateProjects, hydrateProjects, parseProjectsPayload, projectDocument, projectsPayload } from './persistenceUtils.js'
import { bothWays } from '../../test/native.js'
import { hasPekBestand, loadPekBestand } from '../../test/pekFixture.js'
import { loadNativeFile } from '../native/node.js'

// The record as it is kept and its door, in Rust (decision 0020, step 1), held
// to the JavaScript on real planning data: the same records, the same bytes.
describe.skipIf(!hasPekBestand || !loadNativeFile())('the door in Rust, on the PEK Bestand', () => {
  it('reads it to the same record', () => {
    const payload = projectsPayload([loadPekBestand()])
    const { rust, js } = bothWays(() => parseProjectsPayload(payload))
    expect(rust).toEqual(js)
  })

  it('keeps it as the same document, to the byte', () => {
    const [hydrated] = hydrateProjects([loadPekBestand()])
    const { rust, js } = bothWays(() => projectDocument(hydrated))
    expect(contentHash(rust)).toBe(contentHash(js))
    expect(documentHash(rust)).toBe(documentHash(js))
  })

  it('strips and rounds it alike', () => {
    const [hydrated] = hydrateProjects([loadPekBestand()])
    const { rust, js } = bothWays(() => dehydrateProjects([hydrated]))
    expect(rust).toEqual(js)
  })
})
