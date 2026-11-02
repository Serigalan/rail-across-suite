import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// The Signalling-Tool lies beside the suite's start page, under /signalling/
// (deploy/lib.sh, MODULE_APPS); its files are named relative to the page. In
// development /api goes to a local project server, as Caddy sends it in
// production.
export default defineConfig({
  plugins: [react()],
  base: './',
  server: {
    proxy: {
      '/api': { target: process.env.OLT_API_TARGET ?? 'http://127.0.0.1:8787' },
    },
  },
})
