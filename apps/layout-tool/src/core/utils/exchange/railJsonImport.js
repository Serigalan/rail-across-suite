import { generateId, nextTrackName, rebuildCoords } from '@rail-across/core/track'
import { uuidOf } from '@rail-across/core/document'

/**
 * What a parsed RailJSON or alignment exchange file (osrdImport) adds to a
 * project: its tracks — under the UUID drawn from the file's id, as every id
 * of the record is one (decision 0006), or a fresh one where the
 * project already has it; renamed where the name is taken — the switches and
 * buffer stops on them, repointed to those ids, and the passthrough of
 * everything the app does not model (`osrd`), its references to those tracks
 * and stops repointed too, so an export hands it back.
 *
 * Returns { addTracks, addSwitches, addEndMarks, osrd }.
 */
export function planRailJsonImport({ tracks, switches = [], endMarks = [], infra = {} }, {
  existingTracks = [], existingMarks = [], newId = generateId,
} = {}) {
  const existingIds = new Set(existingTracks.map(tr => tr.id))
  const names = new Set(existingTracks.map(tr => tr.name).filter(Boolean))
  const idMap = {}
  const addTracks = tracks.map(tr => {
    const elements = tr.elements
    const name = tr.name && !names.has(tr.name)
      ? tr.name
      : nextTrackName(tr.name?.split('.')[0] || 'track', names)
    names.add(name)
    const own = tr.id ? uuidOf(tr.id) : newId()
    const id = existingIds.has(own) ? newId() : own
    if (tr.id && id !== tr.id) idMap[tr.id] = id
    return { ...tr, id, name, elements, coordinates: rebuildCoords(elements) }
  })
  const remapId = (id) => idMap[id] ?? id

  // Switches the import could rebuild into the app's own model — they are
  // drawn as switch bodies and regenerated on the next export, which is why
  // the passthrough copy is dropped there by id.
  const addSwitches = switches.map(sw => ({
    ...sw,
    ports: Object.fromEntries(Object.entries(sw.ports ?? {})
      .map(([port, p]) => [port, p?.trackId ? { ...p, trackId: remapId(p.trackId) } : p])),
  }))

  // Buffer stops stand on the imported track ends; one whose id the project
  // already uses (the same file imported twice) gets a new one, or it would
  // take the place of the first.
  const markIds = new Set(existingMarks.map(m => m.id))
  const markMap = {}
  const addEndMarks = endMarks.map(mark => {
    const own = mark.id ? uuidOf(mark.id) : newId()
    const id = markIds.has(own) ? newId() : own
    if (mark.id && id !== mark.id) markMap[mark.id] = id
    return { ...mark, id, trackId: remapId(mark.trackId) }
  })

  // What is carried over names the tracks by the ids they have now, and a
  // buffer stop that became a mark by the mark's, so the export's own takes
  // its place.
  const repointed = (value) => {
    if (Array.isArray(value)) return value.map(repointed)
    if (!value || typeof value !== 'object') return value
    return Object.fromEntries(Object.entries(value).map(([k, v]) =>
      [k, k === 'track' && idMap[v] ? idMap[v] : repointed(v)]))
  }
  const osrd = repointed({
    ...infra,
    ...(infra.buffer_stops ? { buffer_stops: infra.buffer_stops.map(b => (markMap[b.id] ? { ...b, id: markMap[b.id] } : b)) } : {}),
  })

  return { addTracks, addSwitches, addEndMarks, osrd }
}
