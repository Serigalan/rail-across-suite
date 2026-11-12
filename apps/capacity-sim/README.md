# Kapazitäts-Sim

The capacity simulation of Rail-Across-Suite (roadmap, phase 4, K1; [decision 0023](../../docs/decisions/0023-capacity-simulation-of-our-own.md)):
a project's timetable run over a variant's net at a stage, as the Fahrplan-Editor runs it, and from the trains'
blocking times

- the **stretches** from one place where trains overtake, cross, begin or end to the next, each compressed after
  UIC Code 406 over a period: the occupancy time, the occupancy rate with its additional times and without, held
  against the recommended bound for its kind of line, and the trains that set it;
- the **minimum headways** of the trains that follow each other there, by kind;
- how long each track and track section of a **station** is held;
- the same for **another variant or stage** beside it.

What computes is the core's Rust module (`crates/capacity`, through `@rail-across/capacity`); the trains run with
the simulation module (`npm run sim` puts it in `public/sim/`). Served under `/capacity/`; the start page's
Kapazitäts-Sim lists the projects and opens a variant here: `?project=<id>&variant=<id>`.
