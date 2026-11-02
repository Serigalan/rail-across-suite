# @rail-across/timetables

A project's timetables ([decision 0016](../../docs/decisions/0016-timetables-of-a-project.md)), for the
Fahrplan-Editor:

- **the model** (`timetable.js`): a train — number, kind, name, a vehicle of the catalogue (`@rail-across/vehicles`)
  with its traction mode and comfort, its departure, its margin — and its points, each a place by its id (`place`, the name only shown; `withPlaces` gives a point named by its name only its id),
  its track or any, a stop or a pass, the dwell, a departure waited for, the itinerary it prefers to the next
  (`itineraryOptions`, `preferredLegs`: between the signalling's stopping places of the two). A train begins and ends at a station, a buffer stop or a boundary only
  (`trainProblems` says where it does not), over a boundary at its speed (`crossingSpeed`); copies at an interval (`taktCopies`); clock times read and written;
- **its run** (`trainRequest`, `trainTimes`, `runTrain`): its way over a variant's net through the tracks its
  points allow — along the itinerary a leg prefers where it leads there, else the shortest (`route.preferred` says
  which, leg by leg) —, the simulation's request, and the times as the timetable shows them — a row for every station along
  the way, stops with their arrival and departure, passes with the time they are passed; `holds` where it waits
  for a block, and its run in time (`runClock`);
- **the occupancy of the blocks** (`occupancy.js`): a train's blocks from main signal to main signal along its way,
  the overlap beyond with each (`blocksAlong`), the time it holds each (`blockingTimes`, after Ril 405 and OSRD's
  spacing requirements), where two trains hold a stretch of track at once (`conflictsOf`); and the timetable's
  trains run as its `dispatch` says (`occupancy`): each for itself with the conflicts said, or one after the other —
  by departure or by kind — each waiting for a free block, longer at a stop or before a signal;
- **the server** (`api.js`): `/api/projects/<id>/timetables`, `/api/timetables/<id>`; a save names the version it
  changed, and one on an older version fails with 409 and the timetable as it is now.

```sh
npm test   # the runs with the simulation module where cargo built it (sim/target/…)
```
