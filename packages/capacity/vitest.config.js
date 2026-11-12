import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    environment: 'node',
    // The core's Rust module, which the capacity crate runs in (decision 0020).
    setupFiles: ['../../packages/core/test/setupNative.js'],
    include: ['src/**/*.test.js'],
  },
})
