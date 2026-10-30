import * as pc from '@rail-across/pointcloud'
import { forgetSource, forgetSources, sourceOf } from './cloudSource'

// The reading functions of @rail-across/pointcloud as the app calls them: by
// project, each cloud given the source it is read from on this device — its
// tile file in OPFS, or the server's (cloudSource). The package itself only
// reads `cloud.source`.

/** The cloud with its source, as the package reads it. */
const withSource = (projectId, cloud) => (cloud.source ? cloud : { ...cloud, source: sourceOf(projectId, cloud) })

/** The points of `cloud` in a cross section (see the package's cloudSection). */
export const cloudSectionPoints = (projectId, cloud, frame) => pc.cloudSectionPoints(withSource(projectId, cloud), frame)

/** Rails found in the section through the clouds (see the package's railTrace). */
export const detectInClouds = (projectId, clouds, options) =>
  pc.detectInClouds(clouds.map(c => withSource(projectId, c)), options)

/** A clearance scan along a track over the project's clouds. */
export const scanClearance = ({ projectId, clouds, ...rest }) =>
  pc.scanClearance({ clouds: clouds.map(c => withSource(projectId, c)), ...rest })

/** A rail traced along a guide through the project's clouds. */
export const traceTrack = ({ projectId, clouds, ...rest }) =>
  pc.traceTrack({ clouds: clouds.map(c => withSource(projectId, c)), ...rest })

/** Forget what was read of a cloud, and how — after it was deleted. */
export function forgetCloud(cloudId) {
  pc.forgetCloud(cloudId)
  forgetSource(cloudId)
}

/** Forget what was read of every cloud — after all of them were deleted. */
export function forgetAllClouds() {
  pc.forgetAllClouds()
  forgetSources()
}
