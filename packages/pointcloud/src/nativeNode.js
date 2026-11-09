import { existsSync, readFileSync } from 'node:fs'
import { usePointCloudModule } from './native.js'

/** Where scripts/wasm.mjs puts the module. */
export const POINTCLOUD_MODULE_FILE = new URL('../wasm/rail_across_pointcloud.wasm', import.meta.url)

/** Load the module from its file, synchronously (Node: the tests). Returns whether it is there. */
export function loadPointCloudModuleFile(file = POINTCLOUD_MODULE_FILE) {
  if (!existsSync(file)) return false
  return usePointCloudModule(new WebAssembly.Module(readFileSync(file)))
}
