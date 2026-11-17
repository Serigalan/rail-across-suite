# 0014 — Construction scenarios in the Baubetriebs-Sim

Status: **accepted** · as of 2026-11-17 · what it computes: [assumptions of the simulation](../simulation/simulation-assumptions.md)

## Decision

1. **The usable net comes from the Layout-Tool.** Which tracks there are on a day is what their lifespans say
   (0007): the Baubetriebs-Sim reads the variant at the state of the day it studies, and shows the net state by state
   with what each changes.
2. **Temporary closures on top of it.** A stretch of a track can be closed for a while: from a first to a last day,
   all day or in a daily window (a possession, Sperrpause, e.g. 22:00–05:00; a window past midnight runs into the
   next day). A closure is picked on the plan, with a click where it begins and one where it ends.
3. **A construction scenario** (Bauszenario) of a timetable holds its closures, the trains taken out and the trains
   diverted by hand. It is kept in the domain `timetables` (`constructionScenarios`, timetables 1.2) and saved with
   the timetable; nothing new on the project server.
4. **Running the timetable in a scenario**, for one day: every train not taken out runs on the net of the day,
   through the dispatching as the Fahrplan-Editor runs it (0012).
   - A train is **touched** by a closure where its regular run runs over the closed stretch while the closure holds
     — from its head entering the stretch to its tail leaving it.
   - A train not touched runs its way.
   - A train touched is **diverted**: the shortest way around what is closed, its points kept. A diversion set by
     hand — its points, its stops, an itinerary — comes first.
   - A train for which no way is left is **not runnable**, and says at which point.
5. **Beside the timetable.** Each train's state (regular, diverted, by hand, not runnable, taken out) and its times
   are set beside its regular run on the same net: arrival, running time, delay, waits and conflicts; a train chosen
   shows both its ways on the plan.

## Why

Planning construction asks whether the timetable can still be run while a part of the net is closed: which trains
are cancelled, which are diverted, and how long they run then. The lifespans say what is built and when; a
possession closes a track for hours without anything being built.

## What it costs

- A closure is no lifespan: it does not change the variant, only the runs of a scenario.
- Closing part of a track closes the way along it across that part; the rest of the track stays usable — a train may
  still stop in front of a closure or begin behind it.
- A diversion is computed, not stored, until it is set by hand; it changes with the net and the closures. It is the
  shortest way, not the one operations would choose, and the way does not ask whether a track is signalled for its
  direction.

## Open

Not part of a scenario yet: speed restrictions at the site, works trains, and whether a diversion is fit for the
train beyond its way, its traction and its stops.
