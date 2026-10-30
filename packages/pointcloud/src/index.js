// @rail-across/pointcloud — point clouds as plain computation: LAS/LAZ and E57
// read as streams, tiled at several voxel levels, cut into sections, rails
// detected and traced, registration, clearance scans.
//
// Where a cloud's tile file lies is the caller's business: every function that
// reads tiles takes the cloud with its `source` ({ read, readMany }) — the
// browser's tile store, the server, a file on disk. Nothing here touches a
// browser API, so the project server runs the same code as the app.
//
// This file is the package's public face; only tests reach into src/.

export { BOTTOM_BAND, checkClearance } from './clearanceCheck.js'
export { scanClearance } from './clearanceScan.js'
export { cloudPlane, cloudToPlane, planeMapper, planeToCloud } from './cloudCrs.js'
export { readCloudHeader, readFirstPoints } from './cloudReader.js'
export { cloudSectionPoints, forgetAllClouds, forgetCloud } from './cloudSection.js'
export { importLevels, importPointCloud } from './importPipeline.js'
export { fileSource, readLasHeader } from './lasReader.js'
export {
  axisPointsCsv, cantMm, detectInClouds, lineGuide, soHeight, TRACE_STEP, traceTrack, trackGuide,
} from './railTrace.js'
export { applyMatrix, IDENTITY, invertMatrix, solveRegistration } from './registration.js'
export { decodeCloudSegment, LEVELS, originalGrid, segmentPlacement } from './tiles.js'
