// The document crate's vectors (crates/doc): the door, the record ↔ document
// conversion and the record as it is kept, recorded from the JavaScript on the
// shared document vectors and the core's own fixtures — never on data that is
// not the repository's.
import { readFileSync } from 'node:fs'
import { checkDocument, fromDocument, manifestOf, PayloadError, toDocument } from '../src/document/index.js'
import { dehydrateProjects, hydrateProjects, parseProjectsPayload, parseTracksPayload, projectDocument, projectsPayload } from '../src/project/index.js'
import { branchTrack, mainTrack, turnout } from '../test/turnoutFixture.js'

const documents = JSON.parse(readFileSync(new URL('../../../schemas/vectors/documents.json', import.meta.url), 'utf8'))
const clone = (v) => JSON.parse(JSON.stringify(v))

/** One call recorded: what went in, and what came out or what was refused. */
function call(op, input, fn) {
  try {
    return { op, input: clone(input), output: clone(fn()) }
  } catch (err) {
    if (!(err instanceof PayloadError)) throw err
    return { op, input: clone(input), error: { code: err.code, details: err.details } }
  }
}

export default function record() {
  const records = []
  const door = []
  for (const v of documents) {
    door.push(call('document.fromDocument', v.document, () => fromDocument(v.document)))
    const checked = v.valid ? checkDocument(v.document) : v.document
    door.push(call('document.manifestOf', checked, () => manifestOf(checked)))
    if (v.valid) records.push(fromDocument(v.document))
  }
  records.push({ id: 'turnout', title: 'Weiche', tracks: [mainTrack(40), branchTrack()], switches: [turnout()], platforms: [] })

  const kept = []
  for (const r of records) {
    kept.push(call('document.toDocument', { record: r, carrySchemas: true, check: true }, () => toDocument(dehydrateProjects([r])[0], { carrySchemas: true })))
    kept.push(call('project.projectDocument', { record: r, carrySchemas: false, check: true }, () => projectDocument(r)))
    kept.push(call('project.parseProjectsPayload', projectsPayload([r]), () => parseProjectsPayload(projectsPayload([r]))))
    const [hydrated] = hydrateProjects([clone(r)])
    kept.push(call('project.dehydrateProjects', [hydrated], () => dehydrateProjects([hydrated])))
  }
  kept.push(call('project.parseProjectsPayload', { format: 'oltjson', version: '1.0', projects: [] }, () => parseProjectsPayload({ format: 'oltjson', version: '1.0', projects: [] })))
  kept.push(call('project.parseTracksPayload', { data: [mainTrack()], switchIds: ['s1'] }, () => parseTracksPayload([mainTrack()], { switchIds: new Set(['s1']) })))
  kept.push(call('project.parseTracksPayload', { data: [mainTrack()], switchIds: [] }, () => parseTracksPayload([mainTrack()])))
  return { door, kept }
}
