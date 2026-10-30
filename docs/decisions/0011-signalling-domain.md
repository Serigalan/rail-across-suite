# 0011 — The signalling domain follows PlanPro, its content is a catalogue

Status: **accepted** · domain `signalling` 1.0, catalogue `db-ril-301` 0.2.0 · checked against the PlanPro plan P-Hausen

## Decision

1. **The domain `signalling` 1.0** ([schema](../../schemas/signalling/1.0.schema.json)) depends on `core` only.
   Its collections are those of PlanPro 1.10, named in English and held to the data model's rules: every
   location is a core `TrackLocation`, every object has a `lifespan`, references go into the core or into the
   domain itself (R2, R9).

   | Collection | PlanPro 1.10 |
   |---|---|
   | `signals` — real or virtual, each real one with its frames and their aspects; a clearance marker on both branches also `alsoAt` | `Signal` (`Signal_Real`, `Signal_Fiktiv`), `Signal_Rahmen`, `Signal_Signalbegriff` |
   | `detectionPoints` | `FMA_Komponente` |
   | `trackSections` — name, kind, bounding detection points | `FMA_Anlage` |
   | `switchElements` — the interlocking's switch, referring to the core's, or a derailer at its place; its clearance marker, twofold flank protection | `W_Kr_Gsp_Element`, `Fla_Zwieschutz` |
   | `routes` — train, shunting and middle-switch, start and target signal, a `TrackPath`, rank, overlaps, what they show | `Fstr_Zug_Rangier`, `Fstr_Fahrweg`, `Fstr_Signalisierung` |
   | `overlaps` — run, danger point, design length, gradient, speed, clear sections, locked switches | `Fstr_DWeg`, `Fstr_DWeg_W_Kr` |
   | `flankProtection` — per switch element and position: by a signal, by a switch element in a position, or waived; passed on | `Fla_Schutz`, `Fla_Freimelde_Zuordnung` |
   | `pzbElements` — magnets of 500, 1000, 2000 Hz and speed checks, their signal and routes | `PZB_Element`, `PZB_Element_Zuordnung` |
   | `stationBoundaries` — what marks the boundary of an operational point | (EBO §4; the roadmap's station boundaries) |

2. **Boards are signals.** Ne 1, Ne 2, Lf 6, Lf 7 and the like are signals whose frame is a plate, as in PlanPro.
   There is no collection of signs.

3. **What a signal can show is not in the schema but in a catalogue**,
   [`packages/core/constraints/db-ril-301.json`](../../packages/core/constraints/db-ril-301.json): the aspects by
   PlanPro's names (`Hp_0`, `Ks_1`, `Zs_3` …), the frames each may stand on, the symbol it asks for, and per
   system and kind of signal what its screen must and may show and which indicators it carries. Version 0.2.0
   holds **all 192 aspects of PlanPro's Ril 301 list** with their symbols, and the kinds of **Ks** signal, the
   shunting signal, the boards and the PZB elements. H/V, Sv and Hl come as
   entries of the catalogue, without a new schema. The core checks a domain against it
   (`checkSignalling`, [module page](../../packages/core/docs/signalling.md)).

4. **Derived, not stored**:
   - the switch positions of a route and of an overlap follow from its run of tracks, as in PlanPro;
   - flank protection follows from the switch elements a route runs over;
   - the extent of a track section follows from its own detection points over the net. Its name and kind are
     stored, so a section keeps its identity when a point moves; where its points enclose nothing together,
     that is a finding. Sections overlap where they meet: each is cut by its own points only.

5. **Overlaps and flank protection are in 1.0**, which a simulation model leaves out and DB planning needs.

6. **The Layout-Tool carries the domain** in its record as `signalling: { fields, collections }`, so writing a
   project back never loses it (F1). It does not edit it; the core carries its locations with every write of the
   net and in every merge (R4), and orphans what stood on a deleted track in the plane (R7).

7. **A PlanPro plan reads in** ([planpro](../../packages/core/docs/planpro.md)): its edges as tracks along their
   geometry, its switch nodes as turnouts, its signalling as the domain, its start and target state as the two
   construction stages. Its signalling can be placed on a project's own net by the same remap.

8. **The server checks the domain** at every check-in, on its core; only errors a check-in brings in refuse
   it, as with the record's own.

## Why

**OSRD** (read in its source, `editoast/schemas/src/infra/`, `core/kt-osrd-signaling`) separates a physical
signal from its logical signals, one per signalling system. Each system is a driver in code that declares its
states, its settings (static, such as `Nf`), its parameters (per route), and which signals bound blocks and
routes. Train detection sections are derived from detectors, blocks from signals; routes are stored. This keeps
RailJSON free of any one system: a new system needs no schema change. The price is that RailJSON holds only
string maps, and only the driver knows at load time whether they are valid. OSRD models for simulation, so it
has no stages, no overlaps, no flank protection, no indicators as objects — and no DB system.

**PlanPro** is how DB plans and hands over signalling, and makes the same split: the structure of an aspect
(`Signalbegriffe_Struktur.xsd`) apart from the aspects of Ril 301 (`Signalbegriffe_Ril_301.xsd`).

We take from both what holds for planning:
- the split between a signal and what it shows;
- the system's knowledge outside the generic schema;
- train detection sections derived from points;
- stored routes.

We differ where our model asks it:
- **a catalogue as versioned data, not code**, so the schema stays generic and still every value is checked,
  a form can be built from it, and a merge works field by field;
- **locations as core `TrackLocation`s with lifespans**, so the core carries them through splits and stages
  (R4, R12);
- **references only into the core and into the domain** (R2, R9).

Taking PlanPro's objects, attributes and enumerations nearly one to one keeps a PlanPro export a translation
field by field, and the catalogue's keys need no table. For OSRD the adapter (decision 0006) will derive the logical
signals: one for the system of each screen, `Nf` from the signal's kind; Ks needs a driver of its own there,
which is one of the adaptations to agree upstream.

## Checked against a real plan

The ProSig example plan **P-Hausen** (PlanPro XSD 1.9.0.2) holds a station of 49 edges and 28 switch nodes with
165 signals, 76 detection points, 56 sections, 189 routes and 34 overlaps. Reading it changed the model in
these places, all within 1.0:

- **The Ks screen carries its lights**: Kl, Zl, Zs 1, Zs 7 and Zs 8 are aspects of the `Schirm`, not of an
  indicator. A kind of signal states what its screen must show, at least one of its proceed aspects (`fahrt`),
  and what it may show; an exit signal that shows Hp 0 and Sh 1 only is a warning, not an error.
- **The catalogue holds every aspect PlanPro lists**, mast plates, clearance markers, level crossing and
  orientation signs among them, with the symbols the XSD enumerates.
- **Frames**: a ceramic body (Ra 12) and a name plate that shows no aspect.
- **A clearance marker stands on two edges** at once (`alsoAt`), and a switch element names its marker.
- **Derailers** are switch elements of their own place, not switches of the core.
- **An overlap ends at its danger point**, often the clearance marker of a switch beyond its run.
- **Sections overlap** at their ends; the derivation cuts each by its own points. 44 of 56 come out as
  PlanPro's Gleis_Abschnitt states them, 7 differ by their overlap.
- **Routes have a rank** where several run between the same two signals, and middle-switch routes a kind.

The plan reads without an error of the record or of the domain; three warnings remain, each a property of the
plan. What a station plan holds beyond this — level crossings, block, local control zones, key locks,
platforms of the interlocking, the operating elements — is counted on import and not read yet; each comes as a
collection of its own when a tool plans it.

## What it costs

- **Two places to change.** A new system or a new aspect is a change of the catalogue, which the schema's hash
  does not cover. A project names the catalogue version it was planned against (`fields.catalogue`).
- **A check of our own.** The content check is JavaScript only: the Python core reads the schema through the
  door, but does not check the catalogue.
- **Carrying is by geometry.** A location is carried to where the track lies now, not by a table of offsets:
  a track reshaped so that the place is no longer on it keeps the station, and the object moves with the
  reshaping.
- **No per-domain rights yet.** The server checks the domain, but does not yet refuse a change of it by an
  author without the right to write it (S-R5).
- **Values are missing.** Distances (distant signal spacing, overlap length, where the PZB magnets lie) are not
  checked until the values of Ril 819 and Ril 483 are in the catalogue (`open_points`).
- **PlanPro's XSD is not in the repository.** It is licensed under RailPL V1.1, whose fit with Apache-2.0 and
  CC BY 4.0 has not been checked; the domain names the version it follows (1.10.0.3.3) and copies none of its
  files.
