import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import basicSsl from '@vitejs/plugin-basic-ssl'

// The computing modules under src/core/utils and those of the packages
// (packages/core/src and packages/pointcloud/src, imported as @rail-across/core
// and @rail-across/pointcloud) do nothing when they are loaded
// but define functions and constants — the workers' own entries aside, which
// are entries anyway (docs/architecture.md). Saying so lets the bundler drop
// whatever a module's index re-exports and nobody uses, even where a constant
// is computed on load (the catalogue parsed, a table built).
const treeshake = {
  moduleSideEffects: (id) => !/\/(src\/core\/utils|packages\/(core|pointcloud)\/src)\/(?!.*Worker\.js$)/.test(id),
}

// https://vite.dev/config/
export default defineConfig({
  // The dev server runs over https because two of the Landesvermessung WMS
  // (Sachsen-Anhalt, Schleswig-Holstein) only answer CORS for https origins —
  // over plain http://localhost the browser drops their tiles. Production is
  // served over https anyway, so this only affects local development.
  plugins: [react(), basicSsl()],
  base: './',
  // The main build has a second entry, share.html: the same app, for read-only
  // share links, which the suite's server lets through without a session.
  build: { rollupOptions: { treeshake, input: { main: 'index.html', share: 'share.html' } } },
  worker: { rollupOptions: { treeshake } },
  // The project server (server/ in the repository) answers under /api on the
  // app's own origin, as it does behind Caddy in production
  // (deploy/templates/Caddyfile).
  server: {
    proxy: {
      '/api': { target: process.env.OLT_API_TARGET ?? 'http://127.0.0.1:8787' },
    },
  },
})
