import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// The Simulation-Tool lies beside the suite's start page, under /simulation/
// (deploy/lib.sh, MODULE_APPS); its files are named relative to the page, the
// simulation module among them (public/sim/, npm run sim). In development
// /api goes to a local project server, as Caddy sends it in production.
export default defineConfig({
  plugins: [react()],
  base: './',
  server: {
    proxy: {
      '/api': { target: process.env.OLT_API_TARGET ?? 'http://127.0.0.1:8787' },
    },
  },
})
