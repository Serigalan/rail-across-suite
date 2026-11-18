import { reconstructElements } from '@rail-across/core/geometry'
import { generateId, rebuildCoords } from '@rail-across/core/track'
import { runOptimizer } from '../optimizer'

/** A track moved further than this by the fit is named in the import report [m]. */
const NAMED_MOVE = 0.001

const plain = ({ geometry: _g, renderCoords: _r, ...el }) => el

/**
 * Imported tracks made to hold together before they are kept, as a document of
 * core 6.0 has them (decision 0006): the optimizer's `fit` — the coordinates
 * fixed, straights through their points, curves between them with their radii
 * and transitions, a switch's routes as they are (optimizer/…/fit.rs). Returns
 * { tracks, notes }: the tracks with their elements fitted and drawn again, and
 * per track how far it moved and what stayed as it lay. Where the module cannot
 * answer, the tracks come back as they are and a note says so — saving then
 * refuses what does not hold together, naming it.
 */
export async function fitImportedTracks(tracks, { run = runOptimizer } = {}) {
  if (!tracks.length) return { tracks, notes: [] }
  const withIds = tracks.map(t => (t.id ? t : { ...t, id: generateId() }))
  let answer
  try {
    answer = await run('fit', { tracks: withIds.map(t => ({ id: t.id, elements: (t.elements ?? []).map(plain) })) })
  } catch (err) {
    return { tracks: withIds, notes: [`Einpassen nicht möglich (${err.message}) – die Gleise sind übernommen, wie sie in der Datei stehen.`] }
  }
  const byId = new Map((answer?.tracks ?? []).map(t => [t.id, t]))
  const notes = []
  const out = withIds.map(t => {
    const fitted = byId.get(t.id)
    if (!fitted) return t
    const label = t.name ?? t.id
    if (fitted.moved > NAMED_MOVE) notes.push(`${label}: um bis zu ${(fitted.moved * 1000).toFixed(1)} mm eingepasst`)
    for (const note of fitted.notes ?? []) notes.push(`${label}: ${note}`)
    const elements = reconstructElements(fitted.elements, t.epsg)
    return { ...t, elements, coordinates: rebuildCoords(elements) }
  })
  return { tracks: out, notes }
}

/**
 * Imported kilometrage lines whose design runs (`design`, mdbKmLine and
 * kmLineEntries) are fitted as tracks are, each run on its own: a km line's
 * plan by column holds together as a track's does (decision 0006). Returns
 * { lines, notes }; a line without a design comes back as it is.
 */
export async function fitImportedKmLines(lines, { run = runOptimizer } = {}) {
  const runs = lines.flatMap((line, i) => (line.design?.runs ?? [])
    .map((r, k) => ({ id: `${i}:${k}`, name: `Km-Linie ${line.lineNumber}, Lauf ${k + 1}`, epsg: line.design.epsg, elements: r.elements }))
    .filter(r => r.elements?.length))
  if (!runs.length) return { lines, notes: [] }
  const fitted = await fitImportedTracks(runs, { run })
  const byId = new Map(fitted.tracks.map(t => [t.id, t.elements.map(plain)]))
  const out = lines.map((line, i) => (line.design
    ? { ...line, design: { ...line.design, runs: line.design.runs.map((r, k) => (byId.has(`${i}:${k}`) ? { ...r, elements: byId.get(`${i}:${k}`) } : r)) } }
    : line))
  return { lines: out, notes: fitted.notes }
}
