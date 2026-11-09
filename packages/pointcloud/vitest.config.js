import { defineConfig } from 'vitest/config'

// Point clouds as plain computation on bytes and typed arrays: Node is enough.
// The import is the Rust module's (crates/pointcloud-wasm), loaded from its file.
export default defineConfig({
  test: {
    environment: 'node',
    setupFiles: ['./test/setupModule.js'],
    include: ['src/**/*.test.js', 'test/**/*.test.js'],
  },
})
