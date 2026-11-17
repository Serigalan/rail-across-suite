# 0002 — The project's name: Rail-Across-Suite

Status: **accepted** · as of 2026-11-17

## Decision

- The project is called **Rail-Across-Suite**, with the subtitle *railway planning across disciplines*. In running
  text and in speech its short form is **Rail-Across**.
- A project is exchanged as **rail-across-1.0** in one file ending in **`.railacross.json`** (0006).
- The modules are named by their task: Layout-Tool, Signalling-Tool, … (0003). "Open" is no part of a module's
  name; the suite's name says what it is.

## Why

- The name before, Open-Rail-Designer, sat between two established names of the field: Bentley's OpenRail™
  Designer and OSRD, the Open Source Railway Designer of the OpenRail Association. Its abbreviation `.ord` already
  stands for Bentley's OpenRoads and OpenRail Designer.
- *Across* says what is particular about the project: one model across all disciplines. *Rail* names the field,
  *Suite* says that it is a set of applications.
- Neither the name nor its short form met a product in the searches, and `.railacross` is unambiguous.

## Names considered

Searched on 2026-10-29 against existing products and file extensions; no trademark register was searched.

| Name | Why not |
|---|---|
| Open-Rail-Designer | Bentley's OpenRail™ Designer; OSRD, the Open Source Railway Designer |
| Open-Track-Designer | OpenTrack, railway simulation from ETH Zurich and OpenTrack Railway Technology; "Open… Designer" is Bentley's product scheme |
| Open-Rail-Across | begins as "OpenRail" |
| Open-Joint-, Open-Concord-Designer | "Joint Designer" (Max Frank, concrete joints); Concord (contract management software) |
| Rail-Data-Tool, Rail-Data-Designer | RailData (UIC), Rail Data Marketplace (UK); "Rail … Designer" (Bentley, Weidmüller, ABB) |
| Rail-Across-Planner | its short form "Rail Planner" is the Eurail/Interrail app |
| Rail-Across-Library, -Toolkit, -Lab | "library" means a code library; short forms RAL and RAT |
| Rail-Across-Hub, -Works, -Atlas | RailHub (Tracsis), RailWorks (Train Simulator), Rail Atlas (books) |
| Rail-Unity-Suite | Unity (Unity Technologies); short form RUS, extension `.rus` taken |
| Railbench, Railwright, Railforge, Tracklayer | RAIL-BENCH (KIT); game mods; TrainPlayer's TrackLayer |

## Open — before the first public release

- **The trademark.** *Across* is a brand of Across Systems GmbH for translation software — another market, but
  software as well. The name is checked in the registers of the DPMA and the EUIPO.
- **A container and its media type.** Whether a zip container `.railacross` comes beside the JSON file — its first
  entry an uncompressed `mimetype` holding `application/vnd.rail-across+zip`, as OpenDocument and EPUB identify
  themselves —, and if so, its media type registered with the IANA.
