# planpro — writing a PlanPro plan

`packages/core/src/planpro/` · the core's module (`crates/planpro`)

Writing a project as a PlanPro 1.10 plan (a `.ppxml`): its tracks, switches and end marks as PlanPro's net, its
signalling domain `signalling` 1.0 ([decision 0011](../../../docs/decisions/0011-signalling-domain.md)) as
PlanPro's objects. There is no import ([decision 0022](../../../docs/decisions/0022-planpro-export-only.md)): a
plan holds too little of a project to be taken in.

    writePlanPro(record, { signalling, now, version }) → { text, report: { notes }, counts }

`record` holds `id`, `title`, `tracks`, `switches` and `endMarks`; `signalling` is the domain (else the
record's). `counts` says how many objects of each PlanPro kind were written, `report.notes` what was not and why
(`not_written`: collection, id, why).

## Files

| In `crates/planpro/src/` | What it does |
|---|---|
| `write.rs` | `write_planpro`: the project as a plan |
| `mapping.rs` | PlanPro's enumerations, read both ways: a key as the domain's value, a value as the key written |
| `xml.rs`, `planpro.rs` | A small XML reader and the conversion of a plan back into a record — no longer in the WebAssembly module, the check of the export in the crate's tests |

The export writes exactly what the reader reads, so what is written reads back as the project: the tests write
the reader's test plan (every object it takes) and a project in UTM and read them back (`tests/export.rs`), and
replay the vectors recorded from the reader's JavaScript (`tests/replay.rs`).

## The net

| PlanPro | Project |
|---|---|
| `TOP_Kante` | a track, its edge; `TOP_Laenge` its length in the plan's plane |
| `GEO_Kante` Gerade, Bogen, Klothoide, Blosskurve | an element: a straight, an arc, a clothoid, a Bloss curve, between `GEO_Knoten` at its ends; `GEO_Richtungswinkel` its bearing at the start in gon, radii and length scaled into the plane |
| `GEO_Punkt` in `CR0` … `FR0` | the coordinates in DB_REF, in the Gauss-Krüger strip (EPSG 5682–5685) the first track lies in or nearest to; a project in UTM is converted (Helmert, or the grid where it is loaded) |
| `TOP_Knoten` of tip, left and right (`TOP_Anschluss` Spitze, Links, Rechts) | a turnout: A the tip, B2 the straight route (`Stammgleis`), B1 the branch, its side read off the first elements; with its `W_Kr_Anlage` (`W_Kr_Grundform` the form's label), `W_Kr_Gsp_Element` (`Bezeichnung_Tabelle` the switch's name) and component at the tip |
| a node with one edge (`Ende`) | an end mark; a buffer stop has a `Gleis_Abschluss` (Festprellbock) |
| a node of two edges (`Verbindung`) | two tracks joining; ends within a millimetre are one node |
| `Gleis_Bezeichnung`, `Strecke` | the track's number (else its name) and line number |

A point object (`Punkt_Objekt_TOP_Kante`) at a station of its track stands at `Abstand` on the edge, scaled as
the track; `Wirkrichtung` in is up, gegen down, beide both directions; `Seitliche_Lage` is the side,
`Seitlicher_Abstand` the offset. Crossings and slips are not written (reported).

## The signalling

| PlanPro | Domain |
|---|---|
| `Signal` with `Signal_Rahmen` and `Signal_Signalbegriff` | `signals`: the place with its first logical signal as `Signalsystem` and `Signal_Art` of the screen (`sh` a `Sperrsignal` of no system) and its `Signal_Funktion`; the logical signal's frames (screen, indicators) and the place's own (boards, name plate), each aspect `nsSignalbegriffe_Ril_301:<aspect>`; `alsoAt` further `Punkt_Objekt_TOP_Kante`n; a virtual signal `Signal_Fiktiv` |
| `FMA_Komponente`, `FMA_Anlage` with `Gleis_Abschnitt` | `detectionPoints`, `trackSections` — each point names the sections it bounds (`ID_FMAgrenze`) |
| `W_Kr_Gsp_Element` (Weiche_Element, Gleissperre_Element), `Fla_Zwieschutz` | `switchElements`: the element of its turnout, or a derailer with its component at its place |
| `Fstr_Zug_Rangier`, `Fstr_Fahrweg`, `Fstr_Signalisierung` | `routes`: the run over the path's tracks from the start to the target signal, `Fstr_V_Hg` the speed the start signal signals for it, what it shows |
| `Fstr_DWeg`, `Fstr_DWeg_W_Kr`, `Markanter_Punkt`, `Sonstiger_Punkt` | `overlaps`: a run from the route's target signal to the danger point (`ID_PZB_Gefahrpunkt`) marking the overlap's end |
| `Fla_Schutz`, `Fla_Freimelde_Zuordnung` | `flankProtection` |
| `PZB_Element`, `PZB_Element_Zuordnung` | `balises` of the system `pzb`: a magnet (`PZB_Element_GM`) or a speed check (`PZB_Element_GUE`), assigned to its signal and routes |

What is existing is in both the start and the target state (`LST_Zustand_Start`, `_Ziel`), what is new
(`lifespan.from`, a track's or switch's `status` `new`) only in the target, what goes (`lifespan.to`, `removal`)
only in the start. Ids become GUIDs: one that is one stays, any other is made one by name (`planpro_id`), the
same on every export.

## Not written

The report names, by collection, what was not written: a location in the plane (`no_track`), crossings and slips
(`kind_not_written`), logical signals after the first (`only_the_first`), balises of systems other than PZB,
station boundaries, stopping places and itineraries.

The file is not validated against the PlanPro XSD, which is not in the repository: the mandatory parts the reader
never read (`Basis_Objekt_Allg`, `Objektreferenzen`, `Bearbeitungsvermerk`, …) are not written. A plan should be
opened in a PlanPro tool before it is passed on.

## Tests

`planpro.test.js`: the Ks station of the signalling tests on a net in UTM, written — the net in DB_REF, the turnout
with its element, the signalling, what is not written. In the crate: the round trips and the reader's vectors.
