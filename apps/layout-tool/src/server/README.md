# src/server — server components

What needs the project server (`/api`, server): sign-in, projects with
variants and revisions, the working copy, the MDB import, DGM1 heights and
relief, point clouds on the server and their 3D view.

It docks onto the app through the extension points of `core` — see
`register.js`. There is one build (`npm run build`, decision 0021), on the
server or in the desktop app.

Described in `docs/server.md`, with the project server (`server`).
