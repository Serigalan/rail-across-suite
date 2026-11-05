import { kmForTrackPoint, trackLength } from '@rail-across/core/track'
import { planePoint } from './places.js'

/**
 * The direction a signal acts in along its line: with rising or falling
 * kilometrage — read off the project's kilometrage lines (net.kmLines) a few
 * metres ahead of it —, else, where no line runs near, with or against its
 * track's stationing. A route runs the way its start signal acts, a speed
 * range the way its signal does.
 */

/** The directions to show routes and speed ranges by: both, or one of the two. */
export const DIRECTIONS = ['both', 'rising', 'falling']

/** How far ahead of a signal the kilometrage is read [m]. */
const AHEAD = 20

/** How far from its line a point may lie and still be stationed against it [m]. */
const KM_REACH = 500

/** The direction a location acts in: 'rising' or 'falling', or null where it acts in none. */
export function kmDirection(net, at) {
  if (at?.anchor !== 'station' || (at.direction !== 'up' && at.direction !== 'down')) return null
  const track = net.tracks.find(t => t.id === at.trackId)
  if (!track) return null
  const along = at.direction === 'up' ? 'rising' : 'falling'
  const ahead = at.direction === 'up' ? Math.min(trackLength(track), at.station + AHEAD) : Math.max(0, at.station - AHEAD)
  if (Math.abs(ahead - at.station) < 1 || !net.kmLines?.length) return along
  const km = (station) => {
    const p = planePoint(net, { anchor: 'station', trackId: track.id, station })
    const hit = p && kmForTrackPoint(net.kmLines, track, [p.easting, p.northing])
    return hit && hit.offset <= KM_REACH ? hit : null
  }
  const [a, b] = [km(at.station), km(ahead)]
  if (!a || !b || a.lineNumber !== b.lineNumber || Math.abs(b.km - a.km) < 1e-6) return along
  return b.km > a.km ? 'rising' : 'falling'
}

/** Whether a direction chosen (DIRECTIONS) lets a signal's location through. */
export const inDirection = (net, at, chosen) => chosen === 'both' || kmDirection(net, at) === chosen
