import { defineConfig } from 'vitest/config'

// Point clouds as plain computation on bytes and typed arrays: Node is enough.
export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.js', 'test/**/*.test.js'],
  },
})
