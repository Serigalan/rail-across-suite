# planpro — reading a PlanPro plan

`packages/core/src/planpro/` · needs `geometry`, `merge`, `document`, `signalling`

Reading a PlanPro 1.10 plan (the `content.xml` of a `.planpro` container, or a `.ppxml`): its net as the
core's tracks, switches and end marks, its signalling as the domain `signalling` 1.0
([decision 0011](../../../docs/decisions/0011-signalling-domain.md)); and placing a plan's signalling on
another net. Pure, without a DOM: it reads its XML itself, so it runs in the browser and under plain Node.

## Files

| File | What it does |
|---|---|
| `xml.js` | A small XML reader, enough for PlanPro: elements, attributes, text, entities |
| `mapping.js` | PlanPro's enumerations and the domain's values they are read as |
| `planpro.js` | `readPlanPro` (the plan's objects), `planproRecord` (a record of the suite), `placeSignalling` (onto another net) |

## The net

| PlanPro | Record |
|---|---|
| `TOP_Kante` | a track, id the edge's; its elements the `GEO_Kante`n along it, in the edge's direction (A to B), turned where the geometry runs the other way |
| `GEO_Kante` Gerade, Bogen, Klothoide, Blosskurve | a straight, an arc, a clothoid, a Bloss curve; which way it bends is read off its bearing (arc) or its end point (transition) |
| `GEO_Punkt` in `CR0` … `FR0` | the plane DB_REF in its Gauss-Krüger strip (EPSG 5682–5685) |
| `TOP_Knoten` of tip, left and right | a turnout: A the tip, B2 the trunk (`Stammgleis`) that goes on straight, B1 the branch; named after its `W_Kr_Gsp_Element`, labelled with its `W_Kr_Grundform` |
| a node with one edge | an end mark: a buffer stop where a `Gleis_Abschluss` (Prellbock) stands, else the boundary of the planning area |
| `Gleis_Bezeichnung`, `Strecke` | the track's name and line number |

A point object (`Punkt_Objekt_TOP_Kante`) at `Abstand` on an edge stands at that station of its track;
`Wirkrichtung` in is up, gegen down, beide both directions; `Seitliche_Lage` is the side, `Seitlicher_Abstand`
the offset. A double slip is four nodes in PlanPro and four turnouts here (reported).

## The signalling

| PlanPro | Domain |
|---|---|
| `Signal` with `Signal_Rahmen` and `Signal_Signalbegriff` | `signals` — places — with their logical signal from the `Signalsystem` and `Signal_Art` of the screen (a `Sperrsignal` of no system one of `sh`), which takes the screen, the indicators, the `Signal_Funktion` and, from each route starting there, its `Fstr_V_Hg` as signallingSpeed; boards and the name plate stay the place's; a second `Punkt_Objekt_TOP_Kante` (a clearance marker on both branches) is `alsoAt` |
| `FMA_Komponente`, `FMA_Anlage` | `detectionPoints`, `trackSections` — bounded by the points whose `ID_FMAgrenze` names them |
| `W_Kr_Gsp_Element` (Weiche_Element, Gleissperre_Element), `Fla_Zwieschutz` | `switchElements`, on the turnout of their component's node, or a derailer at its place |
| `Fstr_Zug_Rangier`, `Fstr_Fahrweg`, `Fstr_Signalisierung` | `routes`: the run's edges in running order from the start signal's edge, what they show |
| `Fstr_DWeg`, `Fstr_DWeg_W_Kr` | `overlaps`, ending at their danger point (`ID_PZB_Gefahrpunkt`) — of a clearance marker on two edges, the place on or beside the run |
| `Fla_Schutz`, `Fla_Freimelde_Zuordnung` | `flankProtection` |
| `PZB_Element`, `PZB_Element_Zuordnung` | `balises` of the system `pzb`: a magnet or, with `PZB_Element_GUE`, a speed check; frequency, INA, test speed, measuring length and arrangement as its settings |

What is in both the start and the target state (`LST_Zustand_Start`, `_Ziel`) is existing; what only the target
has is new (`lifespan.from` the final stage), what only the start has is removed (`lifespan.to`).

The report names what could not be placed (`not_placed`: a flank protection asked for by a local control zone,
one whose protection is no element of the plan) and counts, by tag, the objects of kinds the record does not
hold: level crossings, blocks, local control zones, keys, platforms, heights and cant, operating elements,
attachments.

## On another net

A plan's signalling stands on the plan's edges. `placeSignalling(signalling, planTracks, project)` carries it
onto a project's own tracks with the core's one remap (`merge/carryDomain`): every location onto the track lying
at its place, every run onto the tracks lying along it, an object that nothing lies under orphaned in the plane
(R7), to be attached by the user.

## P-Hausen

The ProSig example plan P-Hausen (PlanPro XSD 1.9.0.2, 49 edges, 28 switch nodes, 165 signals, 189 routes) is
read in under a second: the record has no error of `validateProject`, its document passes the door, and
`checkSignalling` finds no error — three warnings, each a property of the plan. The plan is not in the
repository; `PLANPRO_SAMPLE=…/content.xml` makes the test read it where it is at hand.

## Tests

`planpro.test.js`: a plan built in the test — a turnout, two signals, a route with its overlap, a section,
flank protection, PZB, a signal removed — read, checked, and placed on another net.
