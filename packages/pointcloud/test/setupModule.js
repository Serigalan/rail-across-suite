import { loadPointCloudModuleFile } from '../src/nativeNode.js'

// The import module (crates/pointcloud-wasm), built by `npm run wasm`.
if (!loadPointCloudModuleFile()) throw new Error('the point cloud module is not built (npm run wasm in packages/pointcloud)')
