import { wgs84ToUTM } from '@rail-across/core/geometry'

/**
 * The tracks a form of the edit panel offers in its list: not those that are
 * only a switch's own geometry — they go with their switch, not on their own,
 * and the map leaves them out of the pick the same way (noSwitchBranch).
 */
export const pickableTracks = (tracks) =>
  (tracks ?? []).filter(tr => !(tr.elements?.length && tr.elements.every(el => el.switchBranch === true || el.switchBranch === 'true')))

/** A track as such a form names it. */
export const trackTitle = (track) => track.name || track.id.slice(0, 8)

/** How near [px] a click has to come to a track's node to take it. */
export const SNAP_PX = 12

/**
 * The point a click on the map names, in plane `epsg`: the nearest node of an
 * element of `tracks` within SNAP_PX of the click — exact, from the plane data
 * where the track lies in that plane — or else the clicked point itself.
 * `{ easting, northing, wgs, snapped }`; `click` is { lngLat: [lon, lat],
 * point: { x, y } }, `toScreen(wgs)` a WGS84 point's pixel { x, y }.
 */
export function passPoint(tracks, epsg, click, toScreen) {
  let best = null
  for (const track of tracks ?? []) {
    for (const el of track.elements ?? []) {
      const coords = el.geometry?.coordinates
      if (!coords?.length) continue
      for (const [node, wgs] of [[el.startNode, coords[0]], [el.endNode, coords[coords.length - 1]]]) {
        if (!Array.isArray(node) || !wgs) continue
        const px = toScreen(wgs)
        const d = Math.hypot(px.x - click.point.x, px.y - click.point.y)
        if (d > SNAP_PX || (best && d >= best.d)) continue
        best = { d, node, wgs, epsg: track.epsg }
      }
    }
  }
  if (best) {
    const [easting, northing] = String(best.epsg) === String(epsg) ? best.node : planeOf(best.wgs, epsg)
    return { easting, northing, wgs: best.wgs, snapped: true }
  }
  const [easting, northing] = planeOf(click.lngLat, epsg)
  return { easting, northing, wgs: click.lngLat, snapped: false }
}

function planeOf(wgs, epsg) {
  const { easting, northing } = wgs84ToUTM(wgs, epsg)
  return [easting, northing]
}

/** The shift from one pass point to the other [m], to the micrometre the record keeps. */
export const shiftBetween = (from, to) => ({
  dx: Math.round((to.easting - from.easting) * 1e6) / 1e6,
  dy: Math.round((to.northing - from.northing) * 1e6) / 1e6,
})
