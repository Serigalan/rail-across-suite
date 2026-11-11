import { defineConfig } from 'vitest/config'

// Components are rendered to static markup — no DOM. JSX with the automatic
// runtime, as the build has it.
export default defineConfig({
  esbuild: { jsx: 'automatic' },
  test: {
    environment: 'node',
    // The core's Rust module, which its functions run on (decision 0020).
    setupFiles: ['../../packages/core/test/setupNative.js'],
    include: ['src/**/*.test.{js,jsx}'],
  },
})
