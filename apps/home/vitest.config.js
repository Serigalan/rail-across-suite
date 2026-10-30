import { defineConfig } from 'vitest/config'

// Components are rendered to static markup — no DOM. JSX with the automatic
// runtime, as the build has it.
export default defineConfig({
  esbuild: { jsx: 'automatic' },
  test: {
    environment: 'node',
    include: ['src/**/*.test.{js,jsx}'],
  },
})
