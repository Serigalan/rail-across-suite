import { provided } from '../../extensions'

/**
 * The point cloud panel's clouds on the server (AP 13.6) — the server's
 * (`useCloudServer`, provided by server/useCloudServer.js, Paket L):
 *
 *   useCloudServer(projectId, { onMessage }) → {
 *     rows, uploads, readable, outlines, refresh,
 *     upload, resume, remove, retry, open3d     (null: not offered)
 *   }
 *
 * Without it (tests): no clouds, nothing to load.
 */
const NO_SERVER = Object.freeze({
  rows: [], uploads: [], readable: [], outlines: [], refresh: async () => {},
  upload: null, resume: null, remove: null, retry: null, open3d: null,
})
const useNoCloudServer = () => NO_SERVER

export const cloudServerHook = () => provided('useCloudServer', useNoCloudServer)
