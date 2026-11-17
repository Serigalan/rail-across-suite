# 0009 — Signalling after PlanPro and OSRD, its content a catalogue; the stations

Status: **accepted** · as of 2026-11-17 · layer `signalling` 4.0, catalogue `db-ril-301` 0.4.0 · checked against the
PlanPro plan P-Hausen

## Decision

1. **The layer `signalling`** depends on the core alone (0008). Its collections are those of PlanPro 1.10, named in
   English and held to the format's rules: every location is a location on a track of the core, every object may
   carry a lifespan, references go into the core or into the layer itself.

   | Collection | PlanPro 1.10 | OSRD (RailJSON) |
   |---|---|---|
   | `signals` — real or virtual, of an operational point or of the open line; a real one with its logical signals (one per signalling system), each with its function, frames, aspects and the speed it signals per route; a clearance marker on both branches also `alsoAt` | `Signal` (`Signal_Real`, `Signal_Fiktiv`), `Signalsystem`, `Signal_Art`, `Signal_Funktion`, `Signal_Rahmen`, `Signal_Signalbegriff`; `Fstr_V_Hg` | `Signal` with `logical_signals` |
   | `detectionPoints` | `FMA_Komponente` | `Detector` |
   | `trackSections` — name, kind, bounding detection points | `FMA_Anlage` | derived from detectors |
   | `switchElements` — the interlocking's switch or a derailer, its clearance marker, twofold flank protection | `W_Kr_Gsp_Element`, `Fla_Zwieschutz` | `Switch` |
   | `routes` — train, shunting and middle-switch; start and target signal, a run of tracks, rank, overlaps | `Fstr_Zug_Rangier`, `Fstr_Fahrweg`, `Fstr_Signalisierung` | `Route` |
   | `overlaps` — run, danger point, design length, gradient, speed, clear sections, locked switches | `Fstr_DWeg`, `Fstr_DWeg_W_Kr` | — |
   | `flankProtection` — per switch element and position | `Fla_Schutz`, `Fla_Freimelde_Zuordnung` | — |
   | `balises` — the balises and magnets of train protection, of a system and kind, with their settings | `PZB_Element`, `Balise` | — |
   | `stationBoundaries` — what marks the boundary of an operational point: an entry signal or board, an entry switch, a stopping place | (EBO §4) | `OperationalPoint` parts |
   | `itineraries` — a way from one stopping place to another, its run of tracks and the train routes it takes; a train may prefer it | (Fahrweg) | a train schedule's path |
   | `trainProtection` — the train protection systems of the net | — | — |

2. **Boards are signals.** Ne 1, Ne 2, Lf 6, Lf 7 and the like are signals whose frame is a plate, as in PlanPro.
3. **Systems are data, as in OSRD.** A signal carries a **logical signal** per signalling system it shows: the
   system, its kind there, the system's settings. A Ks main signal that is a shunting signal as well has one of
   `ks` and one of `sh`. A **balise** names its train protection system, its kind and its settings. **Two speeds,
   two owners**: the design speed of a track element is the core's, set in the Layout-Tool; the speed a route
   permits is signalled — its start signal's logical signal states it for the route (`signallingSpeeds`).
4. **What there is lies not in the schema but in a catalogue**,
   [`packages/core/constraints/db-ril-301.json`](../../packages/core/constraints/db-ril-301.json): the aspects by
   PlanPro's names with their frames and symbols — all 192 of PlanPro's Ril 301 list —, per system and kind of
   signal what its screen must and may show, per train protection system its kinds of balise and magnet and their
   settings, the length an overlap needs, every entry titled in German and in English. H/V, Sv, Hl, ZUB or LZB come
   as entries of the catalogue, without a new schema. A project names the catalogue version it was planned against
   (`fields.catalogue`); the core checks the layer against it.
5. **Derived, not stored**: the switch positions of a route and an overlap follow from its run of tracks; flank
   protection from the switch elements a route runs over; a track section's extent from its own detection points
   over the net (its name and kind are stored, so it keeps its identity when a point moves). Where a signal is drawn
   follows from its track, station, side and offset.
6. **Overlaps and flank protection are in it**, which a simulation model leaves out and DB planning needs.
7. **The stations are made in the Signalling-Tool**, under *Bahnhöfe*: a station (EBO §4) is a railway facility
   with at least one switch where trains may start, end, pass or turn back. It is an operational point of the core,
   which the Signalling-Tool writes there (0008); the Layout-Tool's station tracks name it by `operationalPointId`.
8. **A signal belongs to an operational point or to the open line** (`operationalPointId` on the signal); its
   editor names the station, the station's editor lists its signals.
9. **The boundary towards the open line** is the signalling's (`stationBoundaries`): in general the station's entry
   signals or trapezoid boards (Ne 1), otherwise its entry switches or a stopping place. The station's editor
   proposes the entry signals and boards that belong to it; a boundary marked by a signal of the open line or of
   another station is a warning.
10. **The server checks the layer** at every check-in, on its core; only errors a check-in brings in refuse it.

## Why

**OSRD** separates a physical signal from its logical signals, one per signalling system, each system a driver in
code; train detection sections are derived from detectors, blocks from signals, routes are stored. That keeps
RailJSON free of any one system — but only the driver knows whether its string maps are valid, and OSRD models for
simulation: no overlaps, no flank protection, no DB system.

**PlanPro** is how DB plans and hands over signalling, and makes the same split: the structure of an aspect apart
from the aspects of Ril 301. Taking PlanPro's objects, attributes and enumerations nearly one to one keeps the
PlanPro export a translation field by field (0010).

We differ where the model asks it: **a catalogue as versioned data, not code**, so the schema stays generic and
still every value is checked, a form can be built from it in either language, and a merge works field by field.

## Checked against a real plan

The ProSig example plan **P-Hausen** (PlanPro XSD 1.9.0.2) — 49 edges, 28 switch nodes, 165 signals, 76 detection
points, 56 sections, 189 routes, 34 overlaps — changed the model: the Ks screen carries its lights (Kl, Zl, Zs 1,
Zs 7, Zs 8); the catalogue holds every aspect PlanPro lists; frames include a ceramic body and a name plate; a
clearance marker stands on two edges at once; derailers are switch elements of their own place; an overlap ends at
its danger point; sections overlap at their ends (44 of 56 come out as PlanPro states them, 7 differ by their
overlap); routes have a rank. What a station plan holds beyond this — level crossings, block, local control zones,
key locks — comes as a collection of its own when a tool plans it.

## What it costs

- **Two places to change.** A new system or aspect is a change of the catalogue, which the schema's hash does not
  cover.
- **Carrying is by geometry.** A location is carried to where the track lies now, not by a table of offsets.
- **Two names for a station can stand side by side** until one is renamed to the other.
- **The inner switches are not checked**: that a station has at least one switch within its boundary is its
  definition, not yet derived.

## Open

- **The values of the guidelines.** The overlap lengths in the catalogue are a draft: the base lengths per speed
  step are the known values (50 m up to 40 km/h, 100 m up to 60 km/h, 200 m above), their lengthening on a fall is
  an approximation of our own, not checked against Ril 819.0202. The distances of Ril 819 and Ril 483 — distant
  signal spacing, where balises and magnets lie — are not in the catalogue, so they are not checked
  (`open_points`). The catalogue and the tools mark these values as a draft until the guidelines' values are in.
