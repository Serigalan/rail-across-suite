# @rail-across/capacity

The capacity of a stretch from a timetable's blocking times (the roadmap, phase 4, K1; decision 0023), for the
Kapazitäts-Sim (`apps/capacity-sim/`):

- `recordAtStage(record, stage)` — a variant's record as it is at the existing or the final stage;
- `proposedStretches(stations, trains, runs)` — the stretches from one place where trains overtake, cross, begin or
  end to the next, as the trains run them;
- `analyseStretch(stretch, trains, runs, { from, to, additional })` — the timetable compressed after UIC Code 406 over
  the stretch and the period: the occupancy time and rate, with the additional times and without, the trains that
  set it, the minimum headways by kind;
- `stationOccupancy(station, trains, runs, record, { from, to })` — how long each track and track section of a
  station is held;
- `BOUNDS` — the recommended rates of UIC Code 406 by kind of line and period.

What computes is the core's Rust module (`crates/capacity`); this package turns the runs of
`@rail-across/timetables` into its input.
