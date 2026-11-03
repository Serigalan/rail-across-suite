# src/server — server components

What needs the project server (`/api`, server): sign-in, projects with
variants and revisions, the working copy, the MDB import, DGM1 heights and
relief, point clouds on the server and their 3D view.

`server` may import from `src/core`, never the other way round. It docks onto
the app through the extension points of `core` — see `register.js`. The main
build (`npm run build`) ships `core` and `server`, the local build only `core`.

Described in `docs/server.md`, with the project server (`server`).
