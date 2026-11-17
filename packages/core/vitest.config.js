import { defineConfig } from 'vitest/config'

// The core is plain computation on plain objects: Node is enough. Its
// WebAssembly module is loaded first where it is built (decision 0005).
export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.js', 'test/**/*.test.js'],
    setupFiles: ['./test/setupNative.js'],
  },
})
