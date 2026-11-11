# 0024 — Every module writes the core where its work needs it

Status: **accepted** · 2026-11-11 · changes R1, case S5, S-R5 and section 4 of the [data model](../data-model.md);
the exception of [0017](0017-stations-in-the-signalling-tool.md), point 1, becomes the rule

## Decision

1. **Every module writes the core where its work needs it**, not the Layout-Tool alone: the Signalling-Tool the
   stations of the register (0017) and, from infra-data-2.0 on, the stopping places
   ([roadmap](../../ROADMAP.md), phase 4); later modules what they need of it in the same way.
2. **Always through the core library**: every change of the core carries the references of every domain (R4),
   moves the orphans to their plane position (R7), keeps the lifespans over the stages (R12) and is validated at
   check-in as any other. No module writes the core's documents by hand.
3. **The disciplines' domains stay their owners'**: a module writes its own domain and the core, never another
   discipline's domain — apart from the core library's own changes (R4, R7). There are still no stored references
   between sibling domains (R9).
4. **One fact, one place** (R10): a module that needs a fact of the core edits it there and keeps no copy.
5. **Rights**: the right to write a module covers the changes of the core it makes. The server's check per domain
   (S-R5) counts the core as writable by every module's right.

## Why

- What several disciplines place things by is the core's, and more than one discipline makes it: the stopping
  places come into the core in infra-data-2.0, and the Signalling-Tool sets them; the stations already came from
  the Signalling-Tool (0017).
- One writer forced either a detour through the Layout-Tool for what another discipline plans, or an exception per
  collection, as 0017 made one.

## What it costs

- More modules can change the net and the references on it; a change made in the Signalling-Tool carries the
  layout's and the catenary's objects as one made in the Layout-Tool does.
- Two tools change the core at the same time more often; they meet in the merge as two variants do.
- Case S5 no longer holds that only track planners change the core: the server checks what a change of the core
  carries into the domains the author may not write (S-R5), whoever makes it.

## Alternatives considered

- **The Layout-Tool alone** (the rule before): the stopping places would be set there, away from the itineraries
  and boundaries they are planned with.
- **An exception per collection**, as 0017: a list that grows with every module.
