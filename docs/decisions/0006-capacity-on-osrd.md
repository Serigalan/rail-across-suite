# 0006 — Open-Capacity-Sim builds on OSRD

Status: **accepted**

## Decision

1. **Open-Capacity-Sim takes OSRD as its base** — the Open Source Railway Designer of the OpenRail Association:
   its simulation core (`core`: pathfinding, running times with the signalling nearby, conflict detection,
   last-minute train paths) and the services around it, as far as they are needed. A simulation of our own is
   not planned.
2. **OSRD stays a project of its own.** It is used at a pinned version and run as a service, not copied into this
   repository. `apps/open-capacity-sim/` holds what is ours: the adapter.
3. **The adapter** turns one construction stage of a project into OSRD's input — the infrastructure from `core`
   and `signalling` as RailJSON (which the Open-Layout-Tool already writes, with its alignment extension), the
   timetable from `operations` — and turns OSRD's results back into derived results, cached by the hashes of
   their inputs and the stage (data model, S-R6 and case S7). Nothing of OSRD's model enters the domains.
4. **Adaptations OSRD needs are contributed upstream** to the OpenRail Association. Where upstream does not take
   one, it lives in a fork in a repository of its own, kept close to upstream.
5. **Licences**: OSRD is under LGPL-3.0, the adapter under Apache-2.0. Changes to OSRD stay under LGPL-3.0 and
   are published.

Adaptations to be agreed with OSRD's maintainers, among them what DB planning needs and OSRD does not cover
yet — e.g. DB's signalling systems and rules (H/V, Ks, PZB, LZB), the alignment as designed (transitions, cant)
beyond RailJSON's derived curves and slopes, and one infrastructure per construction stage. The list is settled
in phase 3, while the signalling domain takes shape.

## Why

- OSRD exists, is open source and is developed by European railways and the UIC in the OpenRail Association; it
  covers what Open-Capacity-Sim needs. A simulation of our own would repeat years of work.
- The Open-Layout-Tool already writes RailJSON for OSRD.
- Working with the OpenRail Association instead of next to it.

## What it costs

- Open-Capacity-Sim depends on OSRD's pace and decisions; an adaptation needs upstream agreement or a fork to
  maintain.
- Two models meet: design values, signalling and stages on our side, RailJSON on OSRD's. The adapter translates
  and has to follow OSRD's versions.
- OSRD brings its own services and databases, which the toolkit's server has to run.

## Alternatives considered

- **A capacity simulation of our own**: repeats what OSRD has.
- **Copying OSRD into this repository**: its size and its several languages, drift from upstream, and LGPL-3.0
  code mixed with Apache-2.0 code.
- **Only exporting to OSRD**, as the Open-Layout-Tool does today: leaves stages, signalling and timetable to a
  hand-over by hand.
