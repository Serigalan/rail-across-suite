# src/core — browser components

The browser's part of the app: the project store, the map, the panels, the
point clouds read from the server, the plan export. Where a server part has to appear,
`core` offers an extension point (`core/extensions.js`) and
`src/server/register.js` fills it. There is one build (decision 0021).

The computing modules live in `utils/`, one directory per subject (geometry,
track, switch, rules, elevation, …), each with an `index.js` as its public
interface — code outside a module imports only from there (`tools/modules.mjs`).
What each module does, how it computes and why: `docs/modules/`, and the
whole in `docs/architecture.md`.
