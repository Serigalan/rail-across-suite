# 0008 — The core and its layers: who writes what

Status: **accepted** · as of 2026-11-17

## Decision

1. **The core is the alignment and what every layer places things by**: tracks with their elements, heights, cant
   and superstructure, switches and connectors, end marks, km lines, the operational points, the named runs of
   tracks (`trackPaths`) — and the stopping places. **The Layout-Tool writes the alignment.**
2. **Layers build on the core alone**, each written in a tool of its own, each free to be missing from a document:

   | Layer (domain) | Tool | Holds |
   |---|---|---|
   | `signalling` | Signalling-Tool | signals, train detection, switch elements, routes, overlaps, flank protection, balises, station boundaries, itineraries (0009) |
   | `electrification` | Catenary-Tool | the electrified ranges of the tracks and their systems, masts, suspension points, section insulators, contact wires with their voltage |
   | `civil` | Civil-Tool | platforms; bridges, retaining walls, noise barriers along their tracks; measured and reference axes; the plan header; the project picture; what an import brought along that the model does not hold |

3. **Every module writes the core where its work needs it**, not the Layout-Tool alone — the Signalling-Tool the
   operational points and the stopping places, the Civil-Tool a stopping place over a platform. **Always through
   the core library**: every change of the core carries the references of every layer, moves what lost its track
   to its plane position, keeps the lifespans and is checked at check-in as any other.
4. **The layers stay their owners'.** A module writes its own layer and the core, never another discipline's layer
   — apart from the core library's own changes. There are no stored references between layers.
5. **One fact, one place**: a module that needs a fact of the core edits it there and keeps no copy.
6. **The stopping places are the core's**, one for every train, and mostly the Signalling-Tool writes them as it
   sets them. **The Civil-Tool details them**: a platform put or moved makes the stopping places at it — on its
   track, overlapping it — passenger stops (`passengerStop`) and names them (`stoppingPlaceIds`); where none is at
   it yet, it writes one over the platform into the core. A platform moved away or deleted takes back what it said,
   and never deletes a stopping place. Which operational point a platform is of is set in the Civil-Tool.
7. **The Civil-Tool is the Layout-Tool's app, opened as the Civil-Tool** (under `/civil/`): one build, two tools,
   each editing its own and showing the other's. The project's settings and the topology are the Layout-Tool's.
   The Catenary-Tool and the Signalling-Tool are apps of their own.

## Why

- A layer is a discipline's work, and a discipline's tool edits it; the core stays what all of them place things
  by, and small.
- What several disciplines place things by is the core's, and more than one discipline makes it. One writer of the
  core forced either a detour through the Layout-Tool or an exception per collection.
- Platforms, surveys and the plan's dress are civil engineering's, electrification the catenary planner's — not the
  alignment's.

## What it costs

- More tools change the net and the references on it; a change made in the Signalling-Tool carries the other
  layers' objects as one made in the Layout-Tool does, and two tools meet in the merge as two variants do.
- The Layout-Tool and the Civil-Tool share one build, behind a switch of the tool.

## Open — to be taken up once several disciplines work apart on one project

- **Rights per layer.** The server is to check at check-in that the author may write every layer the change
  touches, the core writable by every module's right, and that a layer the author may not write is unchanged but
  for what the core library carried into it. Not built: today whoever may edit a project may check in any layer of
  it.
- **Sketches.** A module opens an infrastructure as a sketch: the core and its own layer only, the core changed
  freely, nothing blocking; a sketch is never merged back, the way back is a takeover of its own layer into a
  variant, what lost its track arriving orphaned in the plane, the sketch's changes of the core listed as
  proposals. Not built.
