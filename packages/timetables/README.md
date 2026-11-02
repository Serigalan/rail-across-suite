# @rail-across/timetables

A project's timetables ([decision 0016](../../docs/decisions/0016-timetables-of-a-project.md)), for the
Fahrplan-Editor:

- **the model** (`timetable.js`): a train — number, kind, name, a vehicle of the catalogue (`@rail-across/vehicles`)
  with its traction mode and comfort, its departure, its margin — and its points, each a station or halt by name,
  its track or any, a stop or a pass, the dwell, a departure waited for. A train begins and ends at a station only
  (`trainProblems` says where it does not); copies at an interval (`taktCopies`); clock times read and written;
- **its run** (`trainRequest`, `trainTimes`, `runTrain`): its way over a variant's net through the tracks its
  points allow, the simulation's request, and the times as the timetable shows them — a row for every station along
  the way, stops with their arrival and departure, passes with the time they are passed;
- **the server** (`api.js`): `/api/projects/<id>/timetables`, `/api/timetables/<id>`; a save names the version it
  changed, and one on an older version fails with 409 and the timetable as it is now.

```sh
npm test   # the runs with the simulation module where cargo built it (sim/target/…)
```
