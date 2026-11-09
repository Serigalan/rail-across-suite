import { FORMAT_NAME, PayloadError, viaNative } from '../document/index.js'
import { reconstructElements, derivedEndBearing, loadedTrackCants } from '../geometry/index.js'
import { rebuildSwitchSymbol } from '../switch/index.js'
import { rebuildPlatformSymbol, rebuildCoords } from '../track/index.js'

/**
 * The record the app works on in memory, and the door to it from the one
 * format of a project, infra-data-1.0 (decision 0021) — what a file holds,
 * what the project server keeps and what the browser keeps of a working copy.
 * The record is the document's content in the shape the app reads, plus the
 * display geometry the load derives (hydrateProjects):
 *
 *   – a track carries one coordinate system (`track.epsg`), and its elements
 *     are stated in it: startNode / endNode, bearing [°], length, radius resp.
 *     r1 / r2 [m]. Everything in WGS84 — element.geometry and renderCoords,
 *     track.coordinates, the switch and platform symbols — is derived and never
 *     kept, so it cannot disagree with the plane data.
 *   – an element states only what does not follow from the rest: no running
 *     station (the stations are the sum of the lengths), and an end bearing
 *     only where it is not the one its bearing, length and radius give (a
 *     kinked straight, an imported tangent). The load puts the derived one back
 *     on arcs and transitions, so code may read `el.endBearing` there.
 *   – a switch element carries the switch's id (`switchId`) and its route; the
 *     switch's name and form label are the switch's own.
 *   – a switch keeps what it joins under `ports`: { A: { trackId, endpoint } … }
 *     — which ports a kind has, switchModel.portsOf says.
 *   – the vertical alignment is the track's own (`track.heights`, stationed
 *     along the track), a platform is stationed along its track too.
 *   – the cant of an element is `cant` where it is the same at both ends and
 *     `cantStart` + `cantEnd` where it changes along it, never both
 *     (Entscheidung 369); a transition always states both its ends, and an
 *     absent cant is 0 (Entscheidung 371). The load gives an arc or a straight
 *     with a ramp its `cant` back, the end that helps the curve least, and
 *     leaves a transition the ends its neighbours give it anyway, so it follows
 *     them (geometry.loadedTrackCants).
 *   – how the element table may change an element (Entscheidung 371): `role`
 *     'follow' or 'intermediate', absent a Festelement; `reasons` { ruleId:
 *     text } the written justification for each Hinweis of the rule catalogue.
 *   – numbers are kept to what they mean: positions, lengths, radii and
 *     stations to 0.1 µm, heights to 0.1 mm, angles to 10⁻⁹ °.
 *   – `kmLines` are foreign reference data, kept in WGS84 as they came, the
 *     one exception to the plane rule (see kmLineUtils).
 *   – project images live outside the records; an export embeds them.
 *
 * parseProjectsPayload is the door: an infra-data-1.0 document is checked and
 * comes out as the record; anything else — an oltjson file among it — is
 * refused, and nothing behind the door has to ask again whether a record is
 * complete. The door and the document are the core's Rust module (crates/doc;
 * decision 0020, step 10); loading a record — its display geometry — is still
 * the page's own.
 */

export { FORMAT_NAME }
export { PayloadError }

/**
 * The records of a payload — an infra-data-1.0 document, one project: { projects: [record] } —
 * and refuse it where it is not one.
 *
 * This is the one place that decides. Everything downstream — hydrateProjects,
 * the switch symbols, the delete rules — may then take the model for granted
 * instead of each guarding for a field that might be missing.
 *
 * Refused with `unsupported_version` where the file is another format (an
 * oltjson file among them) or another version, with `stages_unsupported` where
 * a document has construction stages of its own, and with `invalid_payload`
 * (its `details` saying what) where it states a format it does not hold.
 */
export const parseProjectsPayload = (data) => viaNative('project.parseProjectsPayload', data)

/**
 * The infra-data-1.0 document of a project record — what a file, the project
 * server and a working copy hold. Derived data is stripped first; `carrySchemas` puts the
 * domains' schemas in, as a file that travels carries them; `check: false`
 * writes it although the schemas refuse it (toDocument).
 */
export const projectDocument = (record, { carrySchemas = false, check = true } = {}) =>
  viaNative('project.projectDocument', { record: withoutDerived(record), carrySchemas, check })

/**
 * The infra-data-1.0 document of some tracks, as the data exchange panel
 * exports them: a project of these tracks alone, its schemas carried.
 */
export const tracksDocument = (tracks) =>
  projectDocument({ id: 'tracks', tracks }, { carrySchemas: true })

/**
 * The tracks of a file the data exchange panel exported (tracksDocument) — and
 * refuse it where it is not one: the door checks it as any document. A track
 * that belongs to a switch may only come back into a project that has that
 * switch (`switchIds`), or its branch would point at nothing.
 *
 * Returns the tracks; throws a PayloadError ('unsupported_version',
 * 'invalid_payload', 'tracks_empty', 'switch_missing') otherwise.
 */
export const parseTracksPayload = (data, { switchIds = new Set() } = {}) =>
  viaNative('project.parseTracksPayload', { data, switchIds: [...switchIds] })

// ── Hydrate (load) / dehydrate (persist) ─────────────────────────────────────

/**
 * Rebuild all derived data in place: element geometry and renderCoords, the
 * end bearing of arcs and transitions where the record leaves it to them,
 * track coordinates, the switch symbols and the platform polygons — from the
 * plane data only. Nothing here converts or repairs; what reaches this point
 * has been through parseProjectsPayload or was written by this tool.
 */
export function hydrateProjects(projects) {
  for (const p of projects ?? []) {
    for (const track of p.tracks ?? []) {
      const elements = loadedTrackCants(track.elements ?? []).map(el => {
        if (el.endBearing != null) return el
        const derived = derivedEndBearing(el)
        return derived == null ? el : { ...el, endBearing: derived }
      })
      track.elements    = reconstructElements(elements, track.epsg)
      track.coordinates = rebuildCoords(track.elements)
    }
    if (p.switches || p.platforms) {
      const byId = Object.fromEntries((p.tracks ?? []).map(t => [t.id, t]))
      if (p.switches)  p.switches  = p.switches.map(sw => rebuildSwitchSymbol(sw, byId))
      if (p.platforms) p.platforms = p.platforms.map(pf => rebuildPlatformSymbol(pf, byId))
    }
  }
  return projects ?? []
}

/**
 * A record without the display geometry the load derived — what keeping it
 * strips anyway, left behind before it is handed to the core's module rather
 * than written out to it and back.
 */
function withoutDerived(p) {
  if (!p || typeof p !== 'object') return p
  const out = { ...p }
  if (Array.isArray(p.tracks)) {
    out.tracks = p.tracks.map(({ coordinates: _c, ...t }) => (Array.isArray(t.elements)
      ? { ...t, elements: t.elements.map(el => (el && typeof el === 'object' ? (({ renderCoords: _r, geometry: _g, ...rest }) => rest)(el) : el)) }
      : t))
  }
  if (Array.isArray(p.switches)) {
    out.switches = p.switches.map(sw => (sw && typeof sw === 'object'
      ? (({ fillCoords: _f, lcsCoords: _l, labelCoords: _lc, bodyCentre: _bc, ...rest }) => rest)(sw) : sw))
  }
  if (Array.isArray(p.platforms)) out.platforms = p.platforms.map(pf => (pf && typeof pf === 'object' ? (({ coords: _c, ...rest }) => rest)(pf) : pf))
  return out
}

/** The records as their documents keep them: derived data stripped, numbers rounded. */
export const dehydrateProjects = (projects) => viaNative('project.dehydrateProjects', (projects ?? []).map(withoutDerived))
