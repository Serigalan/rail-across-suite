# 0005 — A sketch mode in every module

Status: **accepted**

## Decision

1. **Every app can open an infrastructure as a sketch**: a variant marked as sketch, started from a revision, with
   a **scope** — the app's own domain; for the layout tool the core and `layout`. The sketch holds the core and
   its scope domain only. The other domains are neither loaded nor stored in it.
2. **In a sketch the core may be changed freely** — an extra crossover to try a concept — also without the right
   to write the core, since these changes never return. Checks run and show their findings, nothing blocks.
   There are no coordination checks: the other domains are absent.
3. **A sketch is never merged back.** The server refuses to merge a sketch into a regular variant.
4. **The only way back is a takeover**: a merge into a target variant, limited to the sketch's scope domain, of
   all or of selected objects.
   - References to tracks that descend from the same base are carried through the id logs (data model, R6).
   - Objects on tracks that exist only in the sketch arrive orphaned, with their plane position (R7).
   - Changes the sketch made to the core are never taken over; they are listed as proposals for the layout tool.
   - Conflicts with changes made in the target since the sketch started are decided as in a merge.
   - The result is checked in on the target like any change: rights, checks and coordination findings as usual.

   In the layout tool the scope is the core and `layout`, so its takeover is the regular merge: the other domains
   are carried (R4, R6), and what loses its track is orphaned with a finding for its owner.
5. **An orphan is a warning, not an error.** It keeps its place in the plane and waits for a track; it is attached
   to a track again by its plane position — within a tolerance, to exactly one track, confirmed by the user.
   Otherwise a takeover that brings orphans would be blocked by them.
6. **A sketch is visible to its author** until the author shares it with the project's members.

## Why

Planning starts with trying — an arrangement of signals, a catenary concept, a track variant — before others are
involved. In the integrated model every try would carry, check and coordinate with all other disciplines, and
every change to a track would need the right to write the core. What makes the model whole — carrying into every
domain, containment, coordination checks, rights per domain — concerns the other domains; a sketch leaves them
out, and with them the coupling. The takeover needs nothing new: merge, carrying through the id logs, orphans in
the plane.

| Case | In a sketch |
|---|---|
| Two arrangements of signals tried for one station | two sketches with tracks and signals only; nothing blocks; one is taken over |
| An extra crossover drawn to try a concept | its signals arrive orphaned with their plane position; the crossover is a proposal for the layout tool, and once it is built the signals are attached |
| The integrated model moved on: a track was split since the sketch started | the takeover carries the signals onto the pieces through the id log, as a merge does |
| A sketch is merged back by mistake | the server refuses; the sketch's tracks never reach the integrated model |
| The track planner sketches a track variant without regard to signalling and catenary | taken over by the regular merge; signals and masts that lose their track are orphaned with findings for their owners |

## What it costs

- A track change made in a sketch has to be built again in the layout tool; objects on it wait as orphans.
- A takeover is a deliberate step with its own review, not a click.
- As warnings, orphans block nothing and can pile up; they are listed for their owner.
- The server needs a kind of variant, its scope and a refusal.

## Alternatives considered

- **No way back**, results redrawn in the integrated model: simpler, but loses work and invites mistakes.
- **Merging a sketch back like a variant**: would bring the sketch's tracks into the integrated model.
- **Sketches without changes to the core**: safe, but loses the main use — trying a track change for one's own
  concept.
