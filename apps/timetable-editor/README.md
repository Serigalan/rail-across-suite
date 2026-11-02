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
- **Its way**: it **begins and ends at a station** — the layout's station tracks of one name — on a track chosen or
  the nearest; on the way it stops or passes at stations and halts (a name only platforms carry), with its dwell and
  a departure it waits for. The choice of its first and last point offers stations only.
- **Its times**: run with OSRD's simulation core (`@rail-across/running-times`) whenever what it is run with changes
  — a row for every station along its way, the stations it passes among them (*Hier halten* makes it stop there),
  its running time, length, speed and energy, its way on the map.
- **Takt**: copies of a train at an interval, their numbers counted on.
- **Bildfahrplan**: every train along the way of one, the stations across at their distances, the time downwards.

## Development

```sh
npm run sim && npm run dev   # with a project server on 127.0.0.1:8787 (OLT_API_TARGET)
npm test
npm run lint
```
