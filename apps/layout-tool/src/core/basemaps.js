import {
  allBasemaps as basemapsWith, findBasemap as findWith,
} from '@rail-across/basemaps'
import { extensionsOf } from './extensions'

// The basemaps are the suite's (@rail-across/basemaps, shared with the
// Signalling-Tool); the Layout-Tool adds those its server registers
// (extension point `basemaps`, the Länder's DGM1).

export {
  DEFAULT_BASEMAP, LANDESVERMESSUNG_STATES, LANDESVERMESSUNG_THUMBNAIL, RELIEF_RAMP, fineReliefStyle, lvBasemapId,
  lvStatesOf, onElevationRange, updateElevationRange,
} from '@rail-across/basemaps'

/** The fixed basemaps and, after them, those the server adds. */
export const allBasemaps = () => basemapsWith(extensionsOf('basemaps'))

/** The basemap with this id, the state orthophotos in any combination and the server's included. */
export const findBasemap = (basemapId) => findWith(basemapId, extensionsOf('basemaps'))

/** Whether the basemap with this id is coloured by elevation. */
export const isElevationBasemap = (basemapId) => findBasemap(basemapId)?.elevation === true
