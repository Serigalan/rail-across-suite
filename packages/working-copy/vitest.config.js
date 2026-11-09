import { defineConfig } from 'vitest/config'

export default defineConfig({ test: { environment: 'node', setupFiles: ['../../packages/core/test/setupNative.js'] } })
