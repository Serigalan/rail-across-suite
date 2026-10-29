# 0007 — The project's name: Rail-Across-Suite

Status: **accepted**

## Decision

- The project is called **Rail-Across-Suite**, with the subtitle *railway planning across disciplines*. In running
  text and in speech its short form is **Rail-Across**.
- Projects are exchanged in files with the extension **`.railacross`** — the zip container of the data model
  (S-R9). The single-file JSON for small projects ends in `.railacross.json`.
- The container identifies itself by its content, as OpenDocument and EPUB do: its first entry is an uncompressed
  file `mimetype` holding `application/vnd.rail-across+zip`.
- The apps keep their names (`open-layout-tool`, `open-signalling-tool`, …); the core keeps its name,
  Rail-Data-Core.
- The records written before this one (0001–0006) keep the former name, Open-Rail-Designer.

## Why

- The former name sat between two established names of the field: Bentley's OpenRail™ Designer and OSRD, the
  Open Source Railway Designer of the OpenRail Association. Its abbreviation `.ord` already stands for Bentley's
  OpenRoads and OpenRail Designer.
- *Across* says what is particular about the project: one model across all disciplines. *Rail* names the field,
  *Suite* says that it is a set of applications.
- Neither the name nor its short form met a product in the searches, and `.railacross` is unambiguous.

## Names considered

Searched on 2026-10-29 against existing products and file extensions; no trademark register was searched.

| Name | Why not |
|---|---|
| Open-Rail-Designer | Bentley's OpenRail™ Designer; OSRD, the Open Source Railway Designer |
| Open-Track-Designer | OpenTrack, railway simulation from ETH Zurich and OpenTrack Railway Technology; "Open… Designer" is Bentley's product scheme (OpenRoads, OpenRail, OpenBridge, OpenTunnel, OpenSite Designer) |
| Open-Rail-Across | begins as "OpenRail" |
| Open-Joint-, Open-Concord-Designer | "Joint Designer" (Max Frank, concrete joints); Concord (contract management software) |
| Rail-Data-Tool, Rail-Data-Designer | RailData (UIC), Rail Data Marketplace (UK); "Rail … Designer" (Bentley, Weidmüller, ABB) |
| Rail-Across-Planner | its short form "Rail Planner" is the Eurail/Interrail app |
| Rail-Across-Library | "library" means a code library; short form RAL |
| Rail-Across-Hub, -Works, -Atlas | RailHub (Tracsis), RailWorks (Train Simulator), Rail Atlas (books) |
| Rail-Across-Toolkit, -Lab | short forms RAT and RAL |
| Rail-Unity-Suite | Unity (Unity Technologies); short form RUS, extension `.rus` taken |
| Railbench, Railwright, Railforge, Tracklayer | RAIL-BENCH (KIT); game mods; TrainPlayer's TrackLayer |
| Rail-Across-Kit, Open-Across, Open-Junction, Railbed, Permway, Gleiswerk, Trassenwerk | no collision found; *Rail-Across-Suite* names the field, the idea and the kind of software at once |

## Still to do

- *Across* is a brand of Across Systems GmbH for translation software — another market, but software as well.
  The name is checked in the registers of the DPMA and the EUIPO before the first release.
- The repository on GitHub is renamed to `rail-across-suite` by its owner; GitHub forwards the old address.
- The MIME type is registered with the IANA once the format is published.
