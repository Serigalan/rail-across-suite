# 0011 — The signalling domain follows PlanPro, its content is a catalogue

Status: **accepted** · domain `signalling` 1.0, catalogue `db-ril-301` 0.1.0

## Decision

1. **The domain `signalling` 1.0** ([schema](../../schemas/signalling/1.0.schema.json)) depends on `core` only.
   Its collections are those of PlanPro 1.10, named in English and held to the data model's rules: every
   location is a core `TrackLocation`, every object has a `lifespan`, references go into the core or into the
   domain itself (R2, R9).

   | Collection | PlanPro 1.10 |
   |---|---|
   | `signals` — real or virtual, each real one with its frames and their aspects | `Signal` (`Signal_Real`, `Signal_Fiktiv`), `Signal_Rahmen`, `Signal_Signalbegriff` |
   | `detectionPoints` | `FMA_Komponente` |
   | `trackSections` — name, kind, bounding detection points | `FMA_Anlage` |
   | `switchElements` — the interlocking's switch, referring to the core's | `W_Kr_Gsp_Element` |
   | `routes` — train and shunting, start and target signal, a `TrackPath`, overlaps, what they show | `Fstr_Zug_Rangier`, `Fstr_Fahrweg`, `Fstr_Signalisierung` |
   | `overlaps` — run, end, design length, gradient, speed, clear sections, locked switches | `Fstr_DWeg`, `Fstr_DWeg_W_Kr` |
   | `flankProtection` — per switch element and position: by a signal, by a switch element in a position, or waived; passed on | `Fla_Schutz`, `Fla_Freimelde_Zuordnung` |
   | `pzbElements` — magnets of 500, 1000, 2000 Hz and speed checks, their signal and routes | `PZB_Element`, `PZB_Element_Zuordnung` |
   | `stationBoundaries` — what marks the boundary of an operational point | (EBO §4; the roadmap's station boundaries) |

2. **Boards are signals.** Ne 1, Ne 2, Lf 6, Lf 7 and the like are signals whose frame is a plate, as in PlanPro.
   There is no collection of signs.

3. **What a signal can show is not in the schema but in a catalogue**,
   [`packages/core/constraints/db-ril-301.json`](../../packages/core/constraints/db-ril-301.json): the aspects by
   PlanPro's names (`Hp_0`, `Ks_1`, `Zs_3` …), the frames each may stand on, the symbol it asks for, and per
   system and kind of signal what its screen must and may show and which indicators it carries. Version 0.1.0
   holds **Ks with its indicators, the shunting signal, the boards, and the PZB elements**. H/V, Sv and Hl come as
   entries of the catalogue, without a new schema. The core checks a domain against it
   (`checkSignalling`, [module page](../../packages/core/docs/signalling.md)).

4. **Derived, not stored**:
   - the switch positions of a route and of an overlap follow from its run of tracks, as in PlanPro;
   - flank protection follows from the switch elements a route runs over;
   - the extent of a track section follows from its detection points over the net. Its name and kind are stored,
     so a section keeps its identity when a point moves; where the stored and the derived boundaries differ,
     that is a finding.

5. **Overlaps and flank protection are in 1.0**, which a simulation model leaves out and DB planning needs.

6. **The Layout-Tool carries the domain** in its record as `signalling: { fields, collections }`, untouched, so
   writing a project back never loses it (F1). In a merge it is one value of the project.

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

## What it costs

- **Two places to change.** A new system or a new aspect is a change of the catalogue, which the schema's hash
  does not cover. A project names the catalogue version it was planned against (`fields.catalogue`).
- **A check of our own.** The content check is JavaScript only: the Python core reads the schema through the
  door, but does not check the catalogue.
- **Carrying references is not done yet.** The core does not yet carry the domain's references (R4) when the
  Layout-Tool splits, joins or reverses a track. Until it does, such an edit leaves references that
  `reference_missing` and `off_track` report. This comes with the signalling tool, before it writes real
  projects.
- **Values are missing.** Distances (distant signal spacing, overlap length, where the PZB magnets lie) are not
  checked until the values of Ril 819 and Ril 483 are in the catalogue (`open_points`).
- **PlanPro's XSD is not in the repository.** It is licensed under RailPL V1.1, whose fit with Apache-2.0 and
  CC BY 4.0 has not been checked; the domain names the version it follows (1.10.0.3.3) and copies none of its
  files.
