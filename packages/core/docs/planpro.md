# planpro — writing a PlanPro plan

`packages/core/src/planpro/` · the core's module (`crates/planpro`)

Writing a project as a PlanPro 1.10 plan, valid against PlanPro's XSD 1.10.0.3, in the container file PlanPro
tools exchange (`.planpro`). The infra-data-1.0 document is the model: the plan says of it what PlanPro has a
place for — the net, its geometry, heights and cant, lines, operational points, platforms, turnouts and end
marks, and the signalling domain `signalling` 1.0 ([decision 0009](../../../docs/decisions/0009-signalling.md)).
There is no import ([decision 0010](../../../docs/decisions/0010-planpro-export-only.md)): a plan holds too little
of a project to be taken in.

    writePlanPro(project, { signalling, now, version, rulesDate, planning })
      → { text, manifest, archive, report: { notes }, counts }

`project` is an infra-data-1.0 document, or a record of the suite, which is made one first. `signalling` is the
domain as the editor holds it (else the document's), `now` the time stamp of the file — its date is the planning's
and, unless `rulesDate` says otherwise, the date of the rules every object is planned by —, `version` the suite's.
`planning` files the plan: `projectNumber`, `planner`, `organisation`, `location` (the leading operational point,
its Ril 100 code), `stage` (the construction stage), `index`, `number`, `informative`, `kind`.

`text` is the plan (`content.xml`), `manifest` its manifest, `archive` the `.planpro` file of both in Base64 (a ZIP,
stored). `counts` says how many objects of each PlanPro kind were written; `report.notes` what was not and why
(`not_written`: collection, id, why), what PlanPro requires that the project does not say and what was written
for it (`defaulted`: collection, id, field, value), and the placeholder standing for the interlocking's equipment
(`placeholder`).

## Files

| In `crates/planpro/src/` | What it does |
|---|---|
| `write.rs` | `write_planpro`: the project as a plan |
| `archive.rs` | The `.planpro` container: a stored ZIP, its manifest, Base64 |
| `mapping.rs` | PlanPro's enumerations, read both ways: a key as the domain's value, a value as the key written |
| `xml.rs`, `planpro.rs` | A small XML reader and the conversion of a plan back into a record — no longer in the WebAssembly module, the check of the export in the crate's tests |

`cargo run -p rail-across-planpro --example write_plan -- <document.json> <plan.planpro>` writes a project at hand
(or reads a PlanPro plan and writes it again); its `content.xml` lands beside the file.

## The plan

Two parts, as PlanPro tools write them, each an `Ausgabe_Fachdaten` with a start and a target state and a
planning group of its own in `Objektmanagement`: the geodata (`Geo`) — edges, nodes, geometry, lines, operational
points, heights, cant — and the interlocking (`ESTW`) — everything else. What is existing is in both states, what
is new (its lifespan begins in the second stage) only in the target, what goes (ends there) only in the start.
Every object carries its base (`Basis_Objekt_Allg`, `Datum_Regelwerk`); the objects of a state stand in the order
the schema's Container lists them. Ids become GUIDs: one that is one stays, any other is made one by name
(`planpro_id`), the same on every export.

## The net

| PlanPro | Project |
|---|---|
| `TOP_Kante` | a track, its edge; `TOP_Laenge` its length in the plan's plane |
| `GEO_Kante` Gerade, Bogen, Klothoide, Blosskurve | an element: a straight, an arc, a clothoid, a Bloss curve, between `GEO_Knoten` at its ends; `GEO_Richtungswinkel` its bearing at the start in gon, radii signed (+ a right-hand curve) and lengths scaled into the plane; a straight over 10 km is cut, as PlanPro takes no longer edge |
| `GEO_Punkt` in `CR0` … `FR0` | the coordinates in DB_REF, in the Gauss-Krüger strip (EPSG 5682–5685) the first track lies in or nearest to; a project in UTM is converted |
| `TOP_Knoten` of tip, left and right (`TOP_Anschluss` Spitze, Links, Rechts) | a turnout: A the tip, B2 the straight route (`Stammgleis`), B1 the branch; with its `W_Kr_Anlage` (`W_Kr_Grundform` the label, `EW` before it), `W_Kr_Gsp_Element` (`Bezeichnung_Tabelle` the switch's name) and the component at the tip with its tongues (`Zungenpaar`: the speeds over both routes from the switch and the design speeds next to it) |
| a node with one edge (`Ende`) | an end mark; a buffer stop has a `Gleis_Abschluss` (Festprellbock) |
| a node of two edges (`Verbindung`) | two tracks joining, or a link; ends within a millimetre are one node |
| `Gleis_Bezeichnung`, `Gleis_Art`, `Strecke` | the track's number (else its name), its kind (line, station, siding track), its line |
| `Oertlichkeit` | an operational point with its Ril 100 code: a station where station tracks are of it, a halt where only platforms are |
| `Bahnsteig_Anlage`, `Bahnsteig_Kante` | the platforms of an operational point, each with its side and system height |
| `Hoehenpunkt`, `Hoehenlinie` | the heights of a track, its vertical tangent polygon (vertical curves not written) |
| `Ueberhoehung`, `Ueberhoehungslinie` | the cant at the elements' ends, constant or a ramp between |

A point object (`Punkt_Objekt_TOP_Kante`) at a station of its track stands at `Abstand` on the edge, scaled as the
track; `Wirkrichtung` in is up, gegen down, beide both directions; with an offset in the model it has a
`Seitlicher_Abstand` (right of the edge's direction positive), else its side (`Seitliche_Lage`). Crossings and
slips are not written (reported).

## The signalling

| PlanPro | Domain |
|---|---|
| `Signal` with `Signal_Rahmen` and `Signal_Signalbegriff` | `signals`: the place with its first logical signal as `Signalsystem` and `Signal_Art` of the screen and its `Signal_Funktion`; the logical signal's frames and the place's own, each aspect `nsSignalbegriffe_Ril_301:<aspect>` with its symbol where the catalogue asks for one; `alsoAt` further places; a virtual signal `Signal_Fiktiv` |
| `FMA_Komponente`, `FMA_Anlage` with `Gleis_Abschnitt` | `detectionPoints`, `trackSections` — each point names the sections it bounds (`ID_FMAgrenze`), an axle counter as `FMA_Komponente_Achszaehlpunkt`; a section's extent (`Gleis_Abschnitt`) is the stretch of the net its points bound, its letter (`Bezeichnung_Kennbuchstabe`) the first of its name |
| `Aussenelementansteuerung` | a placeholder (`Gleisfreimelde_Innenanlage`) the axle counters and sections refer to |
| `W_Kr_Gsp_Element` (Weiche_Element, Gleissperre_Element), `Fla_Zwieschutz` | `switchElements`: the element of its turnout, or a derailer with its component (`Entgleisungsschuh`) at its place |
| `Fstr_Zug_Rangier`, `Fstr_Fahrweg`, `Fstr_Signalisierung` | `routes`: the run over the path's tracks from the start to the target signal, `Fstr_V_Hg` the speed the start signal signals for it, what it shows, one overlap |
| `Fstr_DWeg`, `Fstr_DWeg_W_Kr`, `Markanter_Punkt`, `Sonstiger_Punkt` | `overlaps`: a run from the route's target signal to the danger point marking the overlap's end; its target track (`ID_FMA_Anlage_Zielgleis`) the section before the signal |
| `Fla_Schutz`, `Fla_Freimelde_Zuordnung` | `flankProtection` |
| `PZB_Element`, `PZB_Element_Zuordnung` | `balises` of the system `pzb`: a magnet (`PZB_Element_GM`) or a speed check (`PZB_Element_GUE`), assigned to its signal and routes |

A reference is written only to an object that is: a route between signals on tracks, an aspect a signal shows.

## Not written, filled in

The report names, by collection, what was not written: a location in the plane (`no_track`), crossings and slips
(`kind_not_written`), logical signals after the first (`only_the_first`), aspects or symbols PlanPro does not know,
a section whose extent its points do not give, balises of systems other than PZB or without a signal, station
boundaries, stopping places and itineraries, vertical curves.

PlanPro requires some values the model does not hold — a signal's mounting and aim distance
(`Richtpunktentfernung`), how a switch is worked, the speeds over a turnout where no design speed is next to it,
the build and power of a speed check, the leading operational point. Each is written with a stated value and named
(`defaulted`).

## Tests

`planpro.test.js`: the Ks station of the signalling tests on a net in UTM, written — the net in DB_REF, the turnout
with its element, the signalling, the two parts, the container file, the planning data, what is not written. In the
crate: the round trips (the reader's test plan and a project in UTM read back), the reader's vectors and the
container. The XSD is not in the repository; the export of these projects and of the PEK Halle-Könnern was checked
against it outside.
