# Catenary-Tool

The tool of the layer `electrification` ([decision 0008](../../docs/decisions/0008-core-and-layers.md)): which ranges
of the project's tracks lie under contact wire, and of which system — the voltage as OSRD names a traction mode
(`15000V` the 15 kV 16.7 Hz of DB, `25000V`, `3000V`, `1500V`, `750V`, `600V`). The running time takes the traction
mode along a train's way from it (roadmap, phase 4, I3). Later masts, cantilevers, contact wire and sections.

It works as the Signalling-Tool does: on the variant's working copy in this browser, which every tool shares; it reads
the tracks of the core and writes its layer alone; checking in, updating and discarding as there. Its ranges are carried
by the core library over every track split, join and reversal (R4).

```sh
npm run dev      # beside a local project server (OLT_API_TARGET, default http://127.0.0.1:8787)
npm test
npm run build
```

| Path | What it is |
|---|---|
| `src/model/electrification.js` | the layer as pure functions: ranges made, changed, removed; the check; how much is under wire |
| `src/model/net.js` | the net on the map: tracks as lines of WGS84 points with their stations, the station nearest a click |
| `src/Editor.jsx` | the tool: the rail, the panels, the working copy |
| `src/PlanView.jsx` | the map (MapLibre) with the suite's basemaps |
