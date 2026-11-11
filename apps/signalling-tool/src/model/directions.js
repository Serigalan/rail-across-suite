import { kmForTrackPoint, trackLength } from '@rail-across/core/track'
import { planePoint } from './places.js'
import { runStretches } from './routeSearch.js'
import { runAhead } from './runs.js'

/**
 * The direction a signal acts in along its line: with rising or falling
 * kilometrage — read off the project's kilometrage lines (net.kmLines) a few
 * metres ahead of it, or further along its run where the line has not begun
 * there yet —, else, where no line runs near, with or against its
 * track's stationing. A route runs the way its start signal acts, a speed
 * range the way its signal does.
 */

/** The directions to show routes and speed ranges by: both, or one of the two. */
export const DIRECTIONS = ['both', 'rising', 'falling']

/** How far ahead of a signal the kilometrage is read [m]. */
const AHEAD = 20

/**
 * How far along its run the kilometrage is looked for where it reads the same
 * there — a signal before the line's first metre, where every point is
 * stationed at it — or none [m], and in what steps.
 */
const KM_SEARCH = 3000
const KM_STEP = 50

/** How far from its line a point may lie and still be stationed against it [m]. */
const KM_REACH = 500

/** The places along a run from `at` the way it acts: the signal, AHEAD metres on, then every KM_STEP up to KM_SEARCH. */
function placesAhead(net, at) {
  const run = runAhead(net, at, KM_SEARCH)
  const stretches = run ? runStretches(net, at, run.trackIds, run.end) : []
  const total = stretches.reduce((sum, st) => sum + st.hi - st.lo, 0)
  const out = []
  for (let d = 0; stretches.length && d <= total; d = d < AHEAD ? AHEAD : d + KM_STEP) {
    let into = d
    let st = stretches[0]
    for (const x of stretches) {
      st = x
      if (into <= x.hi - x.lo) break
      into -= x.hi - x.lo
    }
    out.push({ trackId: st.trackId, station: st.direction === 'up' ? st.lo + into : st.hi - into })
  }
  return out
}

/**
 * The direction a location acts in: 'rising' or 'falling', or null where it
 * acts in none. Read AHEAD metres on, or — where the kilometrage reads the
 * same there — further along the way it acts, as long as a line is near.
 */
export function kmDirection(net, at) {
  if (at?.anchor !== 'station' || (at.direction !== 'up' && at.direction !== 'down')) return null
  const track = net.tracks.find(t => t.id === at.trackId)
  if (!track) return null
  const along = at.direction === 'up' ? 'rising' : 'falling'
  if (!net.kmLines?.length) return along
  const tracks = new Map(net.tracks.map(t => [t.id, t]))
  const km = ({ trackId, station }) => {
    const p = planePoint(net, { anchor: 'station', trackId, station })
    const hit = p && kmForTrackPoint(net.kmLines, tracks.get(trackId), [p.easting, p.northing])
    return hit && hit.offset <= KM_REACH ? hit : null
  }
  const sign = (a, b) => (a && b && a.lineNumber === b.lineNumber && Math.abs(b.km - a.km) >= 1e-6 ? (b.km > a.km ? 'rising' : 'falling') : null)
  const ahead = at.direction === 'up' ? Math.min(trackLength(track), at.station + AHEAD) : Math.max(0, at.station - AHEAD)
  const from = km(at)
  if (!from) return along
  const near = sign(from, km({ trackId: at.trackId, station: ahead }))
  if (near) return near
  // Stationed alike at both — before the line's first metre or past its last: further along the run.
  for (const place of placesAhead(net, at).slice(2)) {
    const hit = km(place)
    if (!hit) return along
    const far = sign(from, hit)
    if (far) return far
  }
  return along
}

/** Whether a direction chosen (DIRECTIONS) lets a signal's location through. */
export const inDirection = (net, at, chosen) => chosen === 'both' || kmDirection(net, at) === chosen
