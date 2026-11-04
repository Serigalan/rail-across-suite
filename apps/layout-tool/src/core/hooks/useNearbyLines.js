import { useEffect, useState } from 'react'
import { nearbyLineNumbers } from '../map/kmLineLayer'
import { localLineDistances } from '../map/localKmLayer'
import { mapIsLive } from '../map/pick'
import { useProject } from './useStore'

/** How many lines of the overlays are offered; the project's own come on top. */
const LIMIT = 6

/**
 * The offers of both kinds in one list, nearest first: `shown` from the
 * overlays, `local` the project's own lines — each of those offered however
 * far, and marked `local`, where a line is both it is the project's.
 */
export function mergeLineOffers(shown, local, limit = LIMIT) {
  const byNumber = new Map(shown.map(o => [String(o.lineNumber), { ...o }]))
  for (const o of local) {
    const prev = byNumber.get(String(o.lineNumber))
    byNumber.set(String(o.lineNumber), {
      lineNumber: prev?.lineNumber ?? o.lineNumber,
      distance: Math.min(prev?.distance ?? Infinity, o.distance),
      local: true,
    })
  }
  return [...byNumber.values()]
    .sort((a, b) => a.distance - b.distance)
    .filter((o, i) => o.local || i < limit)
}

/**
 * Hook: the line numbers around the current view, for a form to offer instead
 * of asking for the number to be typed — those the kilometrage overlay shows,
 * and the project's own kilometrage lines (`project.kmLines`, an import's
 * among them), which are offered always.
 *
 * It follows the view, because the answer depends on where the user is about
 * to draw. With the overlay off and no line of its own the list is empty,
 * which is what makes the field itself keep working as a plain input.
 */
export default function useNearbyLines(map) {
  const [lines, setLines] = useState([])
  const kmLines = useProject()?.kmLines

  useEffect(() => {
    if (!map?.current) return
    const m = map.current
    const read = () => {
      if (!mapIsLive(map, m) || !m.isStyleLoaded()) return
      setLines(mergeLineOffers(nearbyLineNumbers(m, LIMIT), localLineDistances(kmLines, m.getCenter())))
    }
    read()
    m.on('idle', read)
    return () => { m.off('idle', read) }
  }, [map, kmLines])

  return lines
}
