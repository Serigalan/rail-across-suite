# Fahrplan-Editor

The timetables of a project, laid on the net of one of its variants
([decision 0016](../../docs/decisions/0016-timetables-of-a-project.md)). The start page lists the projects under
*Fahrplan-Sim* and opens a variant here, `timetable/?project=<id>&variant=<id>` (`&action=view` its head alone,
`&timetable=<id>&train=<id>` the timetable and train shown). The net is this browser's working copy of the variant
where there is one, else its head, and only read; the timetables are the project's, on the server.

## What it does

- **Timetables**: a project has any number, by name; whoever sees the project changes them, a template's the
  admins alone. Changes are saved with *Speichern*; one saved by someone else meanwhile keeps the changes and says
  so.
- **A train**: its number, kind and name, its vehicle from the catalogue (the Fahrzeug-Editor's, *Ansehen* in a tab
  of its own) in a traction mode and comfort, its departure, its margin and how it is spread.
- **Its way**: it **begins and ends at a station** — the layout's station tracks of one name, or a station of the
  Signalling-Tool on the tracks of its stopping places and platforms — on a track chosen or the nearest, or at a
  buffer stop or a boundary of the planning area the Layout-Tool set, running in or out over a boundary at its
  speed; on the way it stops or passes at stations and halts (a name only platforms carry), with its dwell and a
  departure it waits for. The choice of its first and last point offers stations, buffer stops and boundaries
  only. From each point to the next it may prefer an itinerary of the Signalling-Tool, between stopping places of the two stations; where the
  run cannot take it, it runs the shortest way and says so.
- **Its times**: run with OSRD's simulation core (`@rail-across/running-times`) whenever what it is run with changes
  — a row for every station along its way, the stations it passes among them (*Hier halten* makes it stop there),
  its running time, length, speed and energy, its way on the map.
- **Blockbelegung**: a train runs only into a free block — from main signal to main signal of the variant's
  signalling, the overlap beyond with it; without main signals the whole way is one block. Where the variant has
  track sections (the Signalling-Tool's Freimeldeabschnitte), a block is held section by section, and each section is
  free again as soon as the train has left it: a route over a section another train holds waits for that section
  alone, and a conflict names it. The timetable says what
  happens where two trains need one block: *Konflikte nur anzeigen*, each train run for itself; or *Züge warten
  automatisch*, the trains given the net by departure or by kind in an order set (*Vorrang*), each waiting —
  later from its start, longer at a stop, else before the signal — until the block is free. A train's page says
  where it waits and for whom, or where it conflicts and when; the list marks both. The times a block is held
  beyond the running (route setting, distant signal, release) are set there too.
- **Takt**: copies of a train at an interval, their numbers counted on.
- **Speed and height**: under a train's page, its speed over its way — allowed, fastest, with its margin — and
  under it the height of the tracks along it, the stations it passes marked.
- **Fahrzeit rechnen** (`src/runtime/`, what the Simulation-Tool was): in the same rail, a single train computed on
  its own — a vehicle, a start, stops and an end among signals, platforms and points picked on the map, a margin —,
  its result (running times, the times at its stops, energy, the run played on the map), the speed and the height
  over its way under the map. It keeps its state while the timetables are shown; `&section=runtime` opens it, and the
  Simulation-Tool's old address (`simulation/`) leads on to it.
- **Bildfahrplan**: every train along the way of one, the stations across at their distances, the time downwards;
  under each, the blocks it holds over time (Sperrzeitentreppe), red where two trains hold one at once.

## Development

```sh
npm run sim && npm run dev   # with a project server on 127.0.0.1:8787 (OLT_API_TARGET)
npm test
npm run lint
```
