// pointCloud — point clouds in the browser: imported into OPFS by a worker, kept
// and cached there or read from the server, painted, registered against
// tracks. The computation — readers, tiles, sections, rail detection and
// tracing, registration, clearance scans — is @rail-across/pointcloud's; what
// of it the app uses is re-exported here, the reading functions by project
// (cloudReading).
//
// This file is the module's public face: code outside src/core/utils/pointCloud/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/pointCloud.md.

export {
  applyMatrix, BOTTOM_BAND, checkClearance, cloudPlane, cloudToPlane, decodeCloudSegment, fileSource, IDENTITY,
  invertMatrix, LEVELS, originalGrid, planeMapper, readCloudHeader, segmentPlacement, solveRegistration,
  axisPointsCsv, cantMm, lineGuide, soHeight, TRACE_STEP, trackGuide,
} from '@rail-across/pointcloud'
export { cacheGet, cacheLimit, cachePut, cacheUsage, clearCache, forgetCache, setCacheLimit } from './cloudCache'
export { CLOUD_CRS, count, crsHint, duration, mb, sizeText, stepText } from './cloudFormat'
export { cloudSectionPoints, detectInClouds, forgetAllClouds, forgetCloud, scanClearance, traceTrack } from './cloudReading'
export { cloudSize, outlineFeature } from './cloudOutline'
export { coloringsFor, INTRUSION_COLOR, paintCloudCanvas } from './cloudPaint'
export { probeExtent, projectPlane } from './cloudProbe'
export { registrationHook } from './cloudRegistration'
export { cloudServerHook } from './cloudServer'
export { sourceOf } from './cloudSource'
export {
  deleteCloud, estimateCloudBytes, INDEX_FILE, listClouds, opfsAvailable, persistStorage,
  storageEstimate, storagePersisted,
} from './cloudStore'
export { formatName, pointsAsText, PREVIEW_POINTS } from './lasText'
export { previewPoints, startImport } from './pointCloudImport'
export { readableClouds } from './projectClouds'
export { runIsActive, runsOnServer, serverRunHook } from './serverRuns'
export { cloudRefsOf, shiftSurvey, surveyChanges } from './surveyShift'
