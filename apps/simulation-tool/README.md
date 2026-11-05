# Simulation-Tool (forwarding)

Computing the running time of one train on its own — what this tool was — is a section of the Fahrplan-Editor
now ([`apps/timetable-editor`](../timetable-editor/README.md), *Fahrzeit rechnen*), beside the timetables, in one
interface. This app stays at `simulation/` for the addresses that name it: `simulation/?project=<id>&variant=<id>`
(`&action=view`) leads on to `timetable/?project=<id>&variant=<id>&section=runtime`, without a project to the
Fahrplan-Sim's page on the start page (`src/suite.js`).

```sh
npm test
npm run lint
```
