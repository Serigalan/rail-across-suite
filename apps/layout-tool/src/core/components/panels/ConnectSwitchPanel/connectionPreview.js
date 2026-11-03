import { ZOOM_LINE_WIDTH } from '../../../map/style'
import { PALETTE } from '../../../styles/palette'

// The preview of a switch connection on the map (usePreview): both branches
// green or red by the verdict, the elements between in the preview colour,
// and the four points TP1, B1E, B2A, TP2 — for the switch connection and the
// extended one alike.

export const PREVIEW_SOURCE = 'scurve-preview-source'
export const POINTS_SOURCE  = 'scurve-points-source'

export const PREVIEW_LAYERS = [
  {
    sourceId: PREVIEW_SOURCE,
    layer: {
      id: 'scurve-preview-layer', type: 'line',
      layout: { 'line-cap': 'round', 'line-join': 'round' },
      paint: { 'line-color': ['get', 'colour'], 'line-width': ZOOM_LINE_WIDTH },
    },
  },
  {
    sourceId: POINTS_SOURCE,
    layer: {
      id: 'scurve-points-layer', type: 'circle', source: POINTS_SOURCE,
      paint: {
        'circle-radius': 4, 'circle-color': PALETTE.previewPoint,
        'circle-stroke-color': PALETTE.white, 'circle-stroke-width': 1.5,
      },
    },
  },
]

/** Branch, middle element(s), branch: green or red by the verdict. */
export function previewGeoJSON(result) {
  const colour = result.valid ? PALETTE.valid : PALETTE.invalid
  return {
    type: 'FeatureCollection',
    features: [
      { type: 'Feature', properties: { colour },
        geometry: { type: 'LineString', coordinates: result.arc1Coords ?? [] } },
      { type: 'Feature', properties: { colour: PALETTE.previewLine },
        geometry: { type: 'LineString', coordinates: result.midCoords } },
      { type: 'Feature', properties: { colour },
        geometry: { type: 'LineString', coordinates: result.arc2Coords ?? [] } },
    ],
  }
}

/** The toes and the branch ends. */
export function pointsGeoJSON(result) {
  return {
    type: 'FeatureCollection',
    features: [result.tp1Wgs, result.b1eWgs, result.b2aWgs, result.tp2Wgs].map(c => ({
      type: 'Feature', properties: {}, geometry: { type: 'Point', coordinates: c },
    })),
  }
}

/**
 * The two cants at the ldS, for the message that they differ: as magnitudes,
 * signed only where they lean to different sides.
 */
export function ldsCants({ cantLds1: a = 0, cantLds2: b = 0 }) {
  const opposite = a * b < 0
  const show = (u) => String(opposite ? Math.round(u) : Math.abs(Math.round(u)))
  return { u1: show(a), u2: show(b) }
}
