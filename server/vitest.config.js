import { defineConfig } from 'vitest/config'

// The project server's tests run on the core's Rust module where it is built
// (decision 0005), as the server does; RA_NATIVE=0 holds them to its JavaScript.
export default defineConfig({
  test: { setupFiles: ['./test/setupNative.js'] },
})
