import { withStationTexts } from '@rail-across/core/document'
import { wireStretches, DEFAULT_HEIGHT_EPSG } from '@rail-across/core/track'
import {
  exact, round3, buildCoords, totalLength, curveEntries, slopeEntries,
  switchesToPorts,
  platformsToOperationalPoints, endMarksToBufferStops,
} from './alignmentCodec'

// The OSRD instance this tool's exports are meant for — used by the Data
// Exchange and Info panels.
export const OSRD_URL = 'https://osrd.open-layout-tool.org/'

/** RailJSON schema this exporter writes. */
const RAILJSON_VERSION = '3.5.4'

// Infra-level objects the app does not model. An imported file's originals are
// kept on the project (see osrdImport) and handed back here unchanged.
const EMPTY_INFRA = {
  signals: [], speed_sections: [], detectors: [], extended_switch_types: [],
  buffer_stops: [], routes: [], operational_points: [], level_crossings: [],
  electrifications: [], neutral_sections: [],
}

/**
 * RailJSON for OSRD: what it models in its own fields, the design data of
 * the alignment in its `curves` and `slopes` (see curveEntries), the rest in
 * the extensions. `project` is the project record (hydrated).
 */
export function exportToOsrd(record) {
  // RailJSON names a station by its name and number: the points' (decision 0006).
  const project = record && withStationTexts(record)
  const tracks    = project?.tracks ?? []
  const switches  = project?.switches ?? []
  const platforms = project?.platforms ?? []
  const trackMap = Object.fromEntries(tracks.map(t => [t.id, t]))

  // ── track_sections ──────────────────────────────────────────────────────
  const trackSections = tracks.map(track => {
    // Anything carried over from an import (loading gauge, foreign extensions)
    // sits underneath; the regenerated fields win.
    const kept = track.osrd ?? {}
    return {
      loading_gauge_limits: [],
      ...kept,
      id: track.id,
      geo: { type: 'LineString', coordinates: buildCoords(track) },
      length: round3(totalLength(track)),
      curves: curveEntries(track),
      slopes: slopeEntries(track, DEFAULT_HEIGHT_EPSG),
      extensions: {
        source: null,
        ...(kept.extensions ?? {}),
        sncf: {
          ...(kept.extensions?.sncf ?? {}),
          line_code:    Number(track.lineNumber) || 9900,
          line_name:    String(track.lineName  ?? '').trim() || 'default_name',
          track_name:   String(track.name      ?? '').trim() || 'default_track',
          track_number: Number(track.trackNumber) || 1,
        },
      },
    }
  })

  return {
    ...EMPTY_INFRA,
    ...(project?.osrd ?? {}),
    // Stated after what an import brought along, not before it: the file being
    // written is this exporter's, whatever schema the one it came from was.
    version: RAILJSON_VERSION,
    track_sections: trackSections,
    switches:       switchesToPorts(switches, trackMap, project?.osrd?.switches ?? []),
    // Platforms become the stations' operational points; ones an imported file
    // brought along stay unless a generated point takes their id.
    operational_points: platformsToOperationalPoints(
      platforms, trackMap, project?.osrd?.operational_points ?? []),
    buffer_stops: endMarksToBufferStops(
      project?.endMarks ?? [], trackMap, project?.osrd?.buffer_stops ?? []),
    electrifications: electrificationsOf(project, trackMap),
  }
}

/**
 * The layer `electrification` (decisions 0008 and 0006) as RailJSON's
 * electrifications: one per voltage, the stretches its contact wires run on
 * (from the first to the last point carrying them on a track) in both
 * directions — OSRD names a range by its voltage, as the layer does. Ranges on a track the export does not hold
 * are left out; one an imported file brought along stays where the layer has
 * none of its system.
 */
export function electrificationsOf(project, trackMap) {
  const ranges = wireStretches(project?.electrification).filter(r => r.endStation > r.startStation)
  const kept = project?.osrd?.electrifications ?? []
  const bySystem = new Map()
  for (const r of ranges) {
    if (!trackMap[r.trackId]) continue
    const list = bySystem.get(r.voltage) ?? []
    list.push({ track: r.trackId, begin: exact(r.startStation), end: exact(r.endStation), applicable_directions: 'BOTH' })
    bySystem.set(r.voltage, list)
  }
  const made = [...bySystem.entries()].map(([voltage, track_ranges]) => ({ id: `electrification-${voltage}`, voltage, track_ranges }))
  return [...made, ...kept.filter(e => !bySystem.has(e.voltage))]
}
