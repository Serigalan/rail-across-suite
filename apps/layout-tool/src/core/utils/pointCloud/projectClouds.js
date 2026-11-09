import { extensionsOf } from '../../extensions'

/**
 * The clouds of a project as the cross section, the clearance check and the
 * rail trace read them (AP 13.6): the server's, as the providers of the
 * extension point `cloudProviders` list them (server/cloudsOnServer.js). A
 * provider is
 *   { id, list(projectId, { level }) → clouds, sourceKey(cloud), reader(projectId, cloud) }
 * (see cloudSource). One that cannot be reached lists none.
 */
export async function readableClouds(projectId, { level = 1 } = {}) {
  const lists = await Promise.all(extensionsOf('cloudProviders').map(p => p.list(projectId, { level }).catch(() => [])))
  return lists.flat()
}
