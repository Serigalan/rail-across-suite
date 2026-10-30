import { pointAtStation } from '../src/track/platformUtils.js'

/**
 * Where a platform's front edge stands at station `s` of its track, in the
 * plane: [easting, northing]. A platform carried to another track, or turned
 * round with its track, stands where it stood when this is the same point.
 */
export function edgeAt(track, platform, s) {
  const { utm, bearing } = pointAtStation(track, s)
  const [e, n] = Array.isArray(utm) ? utm : [utm.easting, utm.northing]
  const rad = bearing * Math.PI / 180
  // Square to the running direction, to the side the platform stands on.
  const sign = platform.side === 'left' ? -1 : 1
  return [e + sign * platform.frontOffset * Math.cos(rad), n - sign * platform.frontOffset * Math.sin(rad)]
}
