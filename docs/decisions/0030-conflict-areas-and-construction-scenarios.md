# 0030 — Conflict areas in the Kapazitäts-Sim, construction scenarios in the Baubetriebs-Sim

Status: **accepted** · 2026-11-16 · builds on [0023](0023-capacity-simulation-of-our-own.md) and
[0027](0027-construction-stages-by-their-days.md)

The Kapazitäts-Sim says how full a stretch is and which trains set it, not where on the net they meet. The
Baubetriebs-Sim shows the net stage by stage as the Layout-Tool's lifespans make it, but no train runs on it, and
nothing can be closed there for a while. Planning construction asks both: where do the trains get in each other's
way, and can the timetable still be run while a part of the net is closed — which trains are cancelled, which
are diverted, how long do they run then.

## Decision

1. **Conflict areas and the elements that bind.** For a stretch compressed after UIC 406, the Kapazitäts-Sim shows
   on a schematic plan of the variant (the core's schematic strip, as the Signalling-Tool draws it):
   - the *conflict areas*: what two trains of the period both hold — a track section, else a stretch of track
     no section covers —, coloured by how many trains hold it;
   - the *binding elements* (Belegungselemente): for each two trains of the critical chain, the element where the
     second's blocking time touches the first's — the one that sets the minimum headway —, with the two trains,
     the block (signal to signal) and the time it binds. They are listed beside the plan and highlighted on it.
   The capacity crate states the binding element of every train it compresses (`binding`); nothing is saved.
2. **The usable net comes from the Layout-Tool.** Which tracks there are on a day is what their lifespans say
   (0027, 0028): the Baubetriebs-Sim reads the variant at the stage of the day it studies.
3. **Temporary closures in the Baubetriebs-Sim.** On top of that, a stretch of a track can be closed for a while:
   from a first to a last day, all day or in a daily window (a possession, Sperrpause, e.g. 22:00–05:00; a window
   past midnight runs into the next day). A train is touched by a closure on a day where its way runs over the
   closed stretch while the closure holds — in a window, where it is on the stretch within the window.
4. **Construction scenarios, kept with the operation.** Closures, the trains taken out and the trains diverted by
   hand form a *construction scenario* (Bauszenario) of a timetable. It is kept in the domain timetables, as a new
   collection `constructionScenarios` (**timetables 1.2**, which only adds to 1.1; a 1.1 domain is brought forward
   as it is). The record keeps a timetable's scenarios in its data (`data.scenarios`), so that the timetable API
   saves them as it saves the trains; nothing new on the project server.
5. **Running the timetable in a scenario.** For a day, every train not taken out is run on the net of the day with
   the scenario's closures, through the dispatching as the Fahrplan-Editor runs it:
   - a train not touched runs its way;
   - a train touched is *diverted*: the shortest way around what is closed, its points kept; a diversion set by
     hand (its points, its stops, an itinerary) comes first;
   - a train for which no way is left is *not runnable*, and says at which point.
   Each train's times are set beside its regular run: arrival, running time, delay, conflicts.
6. **The phases**, each committed by itself with the tests green:
   1. *Capacity:* the binding element in the crate, the conflict areas, the schematic plan in the Kapazitäts-Sim.
   2. *Format:* timetables 1.2 with `constructionScenarios`; door, record, vectors, the format description.
   3. *Running in a scenario:* the way around closed stretches (`findPath` with what is closed), which trains a
      closure touches, the scenario's runs beside the regular ones (`@rail-across/timetables`).
   4. *The Baubetriebs-Sim:* scenarios of a timetable, closures picked on the plan, the day, trains taken out and
      diverted, the results; saved through the timetable API.
   The assumptions of the simulation (`docs/simulation`) say what each phase computes.

## Consequences

- The Kapazitäts-Sim and the Baubetriebs-Sim read the core's schematic strip and the timetables; neither writes
  the infrastructure. A closure is no lifespan: it does not change the variant, only the runs of a scenario.
- Closing part of a track closes the way along it across that part; the rest of the track stays usable — a train
  may still stop in front of a closure or begin behind it.
- A diversion is computed, not stored, until it is set by hand; it changes with the net and the closures.
- The project server is not deployed with timetables 1.2 before phase 2 is committed.
