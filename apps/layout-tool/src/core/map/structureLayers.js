import { offsetEdgeCoords } from '@rail-across/core/track'

/**
 * The civil structures on the map (decision 0025): a bridge as a broad band
 * over its track, a retaining wall and a noise barrier as a line beside it at
 * their distance from the axis, on their side — each drawn from where it
 * begins to where it ends along its track, so it follows the track's curves.
 */

const SOURCE = 'civil-structures-source'
/** Bridge band, its outline, retaining walls, noise barriers — under the tracks' labels. */
const LAYERS = ['civil-bridge-band', 'civil-bridge-edge', 'civil-wall', 'civil-barrier']
const EMPTY = { type: 'FeatureCollection', features: [] }

/** The colour each kind of structure is drawn in. */
export const STRUCTURE_COLORS = { bridges: '#6b7280', retainingWalls: '#8a5a2b', noiseBarriers: '#2f8f5b' }
/** How far from the axis a wall or a barrier stands where it says nothing [m]. */
export const DEFAULT_OFFSETS = { retainingWalls: 4.5, noiseBarriers: 3.8 }

/** The line a structure is drawn as, in WGS 84, or null where its track is not there. */
export function structureLine(kind, obj, trackById) {
  const at = obj.at ?? {}
  const track = trackById.get(at.trackId)
  if (!track || !Number.isFinite(at.startStation) || !Number.isFinite(at.endStation)) return null
  const [a, b] = [Math.min(at.startStation, at.endStation), Math.max(at.startStation, at.endStation)]
  const sign = at.side === 'left' ? -1 : 1
  const dist = kind === 'bridges' ? 0 : sign * (obj.offset ?? DEFAULT_OFFSETS[kind])
  const coords = offsetEdgeCoords(track, a, b, dist)
  return coords?.length >= 2 ? coords : null
}

/** The structures of a project as features: [{ kind, id, coords }]. */
export function structureFeatures(project) {
  const trackById = new Map((project?.tracks ?? []).map(t => [t.id, t]))
  const features = []
  for (const kind of Object.keys(STRUCTURE_COLORS)) {
    for (const obj of project?.[kind] ?? []) {
      const coords = structureLine(kind, obj, trackById)
      if (coords) features.push({ type: 'Feature', properties: { kind, id: obj.id }, geometry: { type: 'LineString', coordinates: coords } })
    }
  }
  return { type: 'FeatureCollection', features }
}

/** Draw the project's structures, adding the source and layers the first time (or after another basemap). */
export function renderStructuresOnMap(map, project) {
  // Called with the tracks, once their layers are there: the style is, though its tiles may still be loading.
  if (!map?.getStyle?.()) return
  if (!map.getSource(SOURCE)) map.addSource(SOURCE, { type: 'geojson', data: EMPTY })
  if (!map.getLayer(LAYERS[0])) {
    const of = (kind) => ['==', ['get', 'kind'], kind]
    map.addLayer({ id: 'civil-bridge-band', type: 'line', source: SOURCE, filter: of('bridges'),
      paint: { 'line-color': STRUCTURE_COLORS.bridges, 'line-width': ['interpolate', ['linear'], ['zoom'], 12, 6, 18, 26], 'line-opacity': 0.25 } })
    map.addLayer({ id: 'civil-bridge-edge', type: 'line', source: SOURCE, filter: of('bridges'),
      paint: { 'line-color': STRUCTURE_COLORS.bridges, 'line-width': 1.5, 'line-gap-width': ['interpolate', ['linear'], ['zoom'], 12, 6, 18, 26] } })
    map.addLayer({ id: 'civil-wall', type: 'line', source: SOURCE, filter: of('retainingWalls'),
      paint: { 'line-color': STRUCTURE_COLORS.retainingWalls, 'line-width': 3.5 } })
    map.addLayer({ id: 'civil-barrier', type: 'line', source: SOURCE, filter: of('noiseBarriers'),
      paint: { 'line-color': STRUCTURE_COLORS.noiseBarriers, 'line-width': 3.5, 'line-dasharray': [3, 1] } })
  }
  map.getSource(SOURCE).setData(structureFeatures(project))
}
