# 0025 — Layers on the core: the alignment in the Layout-Tool, each layer in a tool of its own

Status: **accepted** · 2026-11-11 · changes the domains of [0009](0009-core-1-0-in-the-data-model.md) and
[0021](0021-one-format-one-build-the-project-in-the-module.md) (infra-data-2.0); amends point 3 of
[0017](0017-stations-in-the-signalling-tool.md)

## Decision

1. **The core is the alignment and what every layer places things by**: tracks with their elements, heights, cant
   and superstructure, switches and connectors, end marks, km lines, the operational point register, the
   construction stages, the named runs of tracks (Routen) — and the stopping places. **The Layout-Tool writes it,
   and only that: the alignment** (Trassierung).
2. **Layers build on the core alone**, each written in a tool of its own, each of them free to be missing from a
   document (data model, section 4):

   | Layer (domain) | Tool | Holds |
   |---|---|---|
   | `signalling` | Signalling-Tool | signals, train detection, switch elements, routes, overlaps, flank protection, balises, station boundaries, itineraries, train protection |
   | `electrification` | Catenary-Tool | the electrified ranges of the tracks and their systems; later masts, cantilevers, wires and sections |
   | `civil` | Civil-Tool | platforms; measured axes and reference axes; the plan header; the project picture; what a RailJSON import brought along that the model does not hold |

3. **The `layout` domain goes**: what it held is `civil`'s, with the platforms from the core — but the named runs of
   tracks, which the Layout-Tool's profiles and tables are made of: they are the core's (`trackPaths`; a document of
   `civil` 1.0 that still holds them reads with them in the core). Only the Civil-Tool edits `civil`; the
   Layout-Tool shows what it needs of it — the reference axes to design against, the plan header for its plan — and edits none of it. What an import brings along it
   keeps as it came.
4. **The stopping places are the core's**, one for every train, and mostly the Signalling-Tool writes them there as
   it sets them ([0024](0024-every-module-writes-the-core.md)). **The Civil-Tool details them**: a platform put or
   moved makes the stopping places at it — on its track, overlapping it — passenger stops (`passengerStop`, core) and
   names them (`stoppingPlaceIds`, civil, since the core names nothing above it); where none is at it yet, it writes
   one over the platform into the core. A platform moved away or deleted takes back what it said, and never deletes
   a stopping place. The Signalling-Tool sets no platform and shows whether a stopping place is a passenger stop.
   A station boundary is marked by a stopping place where it was marked by a platform. Which station a platform is
   of is set in the Civil-Tool, no longer in the Signalling-Tool (0017, point 3). A core 2.0 of before
   `passengerStop` reads with every stopping place a platform names a passenger stop (data step
   `0025-passenger-stops-and-runs`).
5. **infra-data-2.0** carries it: `core` 2.0, `signalling` 2.0, `civil` 1.0, `electrification` 1.0. The door reads an
   infra-data 1.0 document as 2.0, nothing lost; the envelope's version is the version of the set (the
   [roadmap](../../ROADMAP.md), phase 4).
6. **The Civil-Tool is the Layout-Tool's app, opened as the Civil-Tool** (under `/civil/`): one build, two tools,
   each editing its own and showing the other's. The project's settings and the topology are the Layout-Tool's
   alone; the Civil-Tool has its own mark, a bridge (`public/civil-favicon.svg`, its railing from 48 px on). The Catenary-Tool is an app of its own (`apps/catenary-tool/`), as
   the Signalling-Tool is.

## Why

- A layer is a discipline's work, and a discipline's tool edits it; the core stays what all of them place things
  by, and small.
- Platforms, surveys and the plan's dress are civil engineering's, not the alignment's; mixing them into the
  Layout-Tool made it the editor of everything that was not signalling.
- Electrification is planned with the catenary, not with the alignment.

## What it costs

- A format version that is not an addition: core 2.0 and signalling 2.0, with a step at the door and on the server.
- The Layout-Tool loses its platform, survey and run editing to the Civil-Tool; both are the same build, so the code
  moves behind a switch of the tool, not into another app.
- One more tool on the start page, and one more app to build and deploy (the Catenary-Tool).

## Alternatives considered

- **Keep `layout` as the Layout-Tool's domain**, beside the core: two domains written by one tool, and the civil
  objects edited with the alignment.
- **A Civil-Tool as an app of its own**: the platform editor, the point clouds and the surveys copied or moved out of
  the Layout-Tool — far more work for the same tool.
- **Electrification in the core** or in the Layout-Tool: the catenary planner's work in the track planner's tool.
