import { extensionsOf } from '../../extensions'

/**
 * One way to read a cloud's tile file (AP 13.6): `read(offset, length)` and
 * `readMany(ranges)`, each giving bytes. cloudSection, cloudSlice and
 * clearanceCheck read through it and do not care where from. The clouds are
 * the server's: the provider of the extension point `cloudProviders` that
 * says a cloud is its own — `sourceKey(cloud)` a key, not null — reads it
 * through its `reader(projectId, cloud)` (server/cloudsOnServer.js).
 */

const sources = new Map()

/**
 * The source of a cloud as cloudSection gets it — `cloud.source` where one is
 * given (tests), else its provider's. One per cloud (and level), kept. A
 * provider's key starts with the cloud's id.
 */
export function sourceOf(projectId, cloud) {
  if (cloud.source) return cloud.source
  const provider = extensionsOf('cloudProviders').find(p => p.sourceKey(cloud) != null)
  if (!provider) throw new Error(`pointcloud: no source for cloud ${cloud.id}`)
  const key = provider.sourceKey(cloud)
  let s = sources.get(key)
  if (!s) {
    s = provider.reader(projectId, cloud)
    sources.set(key, s)
  }
  return s
}

/** Forget the source of a cloud — after it was deleted. */
export function forgetSource(cloudId) {
  for (const key of [...sources.keys()]) if (key.startsWith(`${cloudId}|`)) sources.delete(key)
}

/** Forget every source. */
export const forgetSources = () => sources.clear()
