import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// The suite's own pages: the start page at the root of the suite's address,
// and the sign-in page, which the server shows in place of every page of the
// suite until one is signed in (deploy/templates/Caddyfile) — so its files
// are named from the root, wherever it is shown. The Layout-Tool lies beside
// them under /layout/, the project server under /api. In development /api
// goes to a local project server, as Caddy sends it in production.
export default defineConfig({
  plugins: [react()],
  base: '/',
  build: {
    rollupOptions: {
      input: { main: 'index.html', signIn: 'sign-in/index.html' },
    },
  },
  server: {
    proxy: {
      '/api': { target: process.env.OLT_API_TARGET ?? 'http://127.0.0.1:8787' },
    },
  },
})
