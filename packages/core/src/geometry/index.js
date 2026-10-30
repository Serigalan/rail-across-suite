// geometry — the projected plane and the element: coordinate systems and NTv2
// grids, straights, arcs and transitions (clothoid, Bloss), the display geometry
// derived from the design values, parallels and plane changes.
//
// This file is the module's public face: code outside packages/core/src/geometry/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/geometry.md.

export { epsgMismatches, nodeGaps, untrueLengths } from './chainChecks.js'
export { clamp } from './numbers.js'
export {
  clothoidRadiusAt, computeClothoidUtm, curvatureOf, projectOnTransitionUtm, radiusOfCurvature,
  cantEnds, sampleTransitionUtm, transitionBearingAtUtm, transitionCantEnds, transitionPointAtUtm, withCantRamp,
  storedTrackCants, loadedTrackCants, hasCantEnds, withCantAtEnd, withCantEdit, withCantsOver,
} from './clothoidUtils.js'
export {
  crsCentre, crsDatum, crsLabel, crsName, crsSystem, crsSystemName, EPSG_OPTIONS, epsgForLngLat,
  FILE_CRS_OPTIONS, gkZone, isCrsSystem, planeCoordsToWgs84, projectCrsOptions, systemCodeAt,
  systemZones, toWgs, transformGridBearing, transformPlanePoint, utmToWgs84, wgs84ToUTM,
} from './coordinateUtils.js'
export { arcElement, arcFrom, straightElement, straightFrom, transitionElement } from './elementFactory.js'
export { reconstructElements } from './elementReconstruct.js'
export {
  arcCenter, arcCoordsFromRadiusUtm, arcPlanePoints, bearingAfterUtm, computeCurvedValuesUtm,
  computeStraightValuesUtm, derivedEndBearing, displayCoords, endPointCurvedUtm,
  endPointStraightUtm, nodeUtm, projectOnArcUtm, projectOnBearingUtm, resolveEndBearing,
  reverseElement, signedRadiusFrom3PointsUtm,
} from './elementUtils.js'
export {
  GEOJSON_MAXZOOM, SAGITTA_ELEMENT, SAGITTA_TRACK, STRAIGHT_VERTEX_SPACING, SWITCH_ROUTE_STEP,
} from './geometryPrecision.js'
export { loadedRegionalGrids, loadGridsFor, loadNtv2Grid, loadRegionalGrids } from './ntv2Grid.js'
export { offsetTrackElements } from './parallelUtils.js'
export { transformTrackToPlane } from './planeTransform.js'
