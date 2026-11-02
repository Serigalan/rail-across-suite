import { defineConfig } from 'vitest/config'

// The core is plain computation on plain objects: Node is enough.
export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.js', 'test/**/*.test.js'],
  },
})
