import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// The start page is served at the root of the suite's address; the
// Open-Layout-Tool lies beside it under /layout/ and the project server under
// /api — one origin, one sign-in. In development /api goes to a local project
// server, as Caddy sends it in production.
export default defineConfig({
  plugins: [react()],
  base: './',
  server: {
    proxy: {
      '/api': { target: process.env.OLT_API_TARGET ?? 'http://127.0.0.1:8787' },
    },
  },
})
