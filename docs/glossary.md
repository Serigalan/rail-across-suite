# Glossary — German ↔ English

Code, documentation and commit messages are in English, but railway planning in Germany is done in German, after
German rules. This list fixes one English word for each German term, so that code, data and texts say the same
thing. Where the data model has an identifier for a term, it is given; `—` means there is none yet.

A German term of art without a good English equivalent stays German, in italics (*Höhenplan*, *Festelement*), as
the Open-Layout-Tool's documentation does.

## Network and alignment

| German | English | In the model | Note |
|---|---|---|---|
| Gleis | track | `tracks` | a chain of elements in one plane |
| Streckengleis | line track | `trackType: 1` | carries `lineNumber`, `lineName`, `side` |
| Bahnhofsgleis | station track | `trackType: 2` | carries `stationName`, `uicStation`, `trackNumber` |
| Hauptgleis / Nebengleis | main track / siding | `trackUse: "siding"` for a siding | |
| Strecke, Streckennummer | line, line number | `lineNumber` | |
| Gleisachse | track axis | — | |
| Gleisanfang / Gleisende | track begin / track end | `BEGIN` / `END` | |
| Stoß, Knoten | joint, node | — | where two track ends meet |
| Trassierung | alignment design | — | |
| Lageplan | plan; horizontal alignment | `elements` | |
| *Höhenplan* | vertical alignment | `heights` | the tangent polygon with vertical curves |
| Element | element | `elements[]` | |
| Gerade | straight | `elementType: 0` | |
| Bogen, Kreisbogen | arc, circular curve | `elementType: 1` | |
| Übergangsbogen | transition (curve) | `elementType: 2` | |
| Klothoide / Bloss-Kurve | clothoid / Bloss curve | `transitionType` | |
| Radius | radius | `radius`, `r1`, `r2` | signed, + = right-hand curve |
| Richtungswinkel | bearing | `bearing` | degrees from grid north, clockwise |
| Überhöhung | cant | `cant`, `cantStart`, `cantEnd` | mm |
| Überhöhungsrampe | cant ramp | — | over a transition |
| *Festelement* / *Folgeelement* / *Zwischenelement* | fixed / following / intermediate element | `role` absent / `follow` / `intermediate` | how the element table may change an element |
| Station, Stationierung | station, stationing | `station` | m from the track's begin |
| Kilometrierung | kilometrage | `kmLines` | the line's km, in metres in the data |
| Kilometersprung | km jump | — | |
| Längsneigung | gradient | — | ‰ |
| Neigungswechsel | change of gradient | `heights[]` | a point of the tangent polygon |
| Ausrundung, Ausrundungshalbmesser | vertical curve, its radius | `rv`, `la` | |
| Entwurfsgeschwindigkeit | design speed | `speed` on an element | km/h |
| zulässige Geschwindigkeit | permitted speed | — | a signalling fact, not the design speed |
| Lagesystem, Meridianstreifen | coordinate system, (meridian) strip | `crsSystem`, `epsg` | the project names the system, a track its strip |
| Höhensystem | vertical datum | `heightEpsg` | default DHHN2016 (EPSG 7837) |
| Streckenkategorie | line category | `lineCategory` | main, secondary, S-Bahn |

## Rules and checks

| German | English | In the model | Note |
|---|---|---|---|
| Regelwerk, Richtlinie (Ril) | rulebook, guideline | `constraints/` | DB Ril 800.0110, 800.0120, 800.0130 |
| Regelwert | standard value | — | |
| Ermessensgrenze | discretionary limit | — | |
| Hinweis / Warnung / Zustimmung / Sonderfall | hint / warning / approval required / special case | rule finding levels | |
| Begründung | justification | `reasons`, `reason` | why a finding is accepted |
| Regellänge / Mindestlänge | standard length / minimum length | — | of transitions |

## Turnouts and track ends

| German | English | In the model | Note |
|---|---|---|---|
| Weiche | turnout | `kind: "turnout"` | ports A, B1, B2 |
| Kreuzung | crossing | `kind: "crossing"` | |
| Einfache / Doppelte Kreuzungsweiche | single / double slip | `single_slip` / `double_slip` | |
| Weichenanfang (WA) | toe of the turnout | port `A` | |
| Weichenende (WE) | end of the turnout | — | |
| letzte durchgehende Schwelle (ldS) | last common sleeper (LCS) | — | the LCS line is derived (`lcsCoords`) |
| gerader Strang / abzweigender Strang | main route / branch | `switchRoute: "main"` / `"branch"` | the turnout's two routes |
| Weichenform, Bauform | switch form | `label`, `formVersion` | from the form tables of Ril 800.0120 |
| Weichenverbindung, Gleisverbindung | switch connection, crossover | — | |
| Verbinder | connector | `kind: "link"` | two track ends joined at a node: where the plane changes, or where a rebuilt part ties in |
| Prellbock | buffer stop | `endMarks`, `kind: "buffer_stop"` | |
| Planungsgrenze | planning-area boundary | `endMarks`, `kind: "boundary"` | |

## Objects along the track

| German | English | In the model | Note |
|---|---|---|---|
| Lage am Gleis | track location | `TrackLocation` | one value: track, station, side, offset, direction, anchor |
| Abschnitt am Gleis | track range | `TrackRange` | |
| Fahrweg | track path | `TrackPath` | an ordered run of tracks |
| Bahnsteig, Bahnsteigkante, Bahnsteighöhe | platform, platform edge, platform height | `platforms`, `frontOffset`, `height` | |
| Bauwerk: Brücke, Tunnel, Bahnübergang | structure: bridge, tunnel, level crossing | — | planned for the core |
| Betriebsstelle, Bahnhof, Haltepunkt | operational point, station, halt | — | register planned for the core |
| RL100-Kürzel, UIC-Nummer | RL100 code, UIC number | `code`, `uicStation` | attributes, not ids |
| Lichtraum, Lichtraumprofil | clearance, clearance gauge | `gaugeProfile` | |
| Gleisabstand | track spacing | — | |
| Querschnitt | cross section | — | |
| Oberbau, Schiene, Schwelle | superstructure, rail, sleeper | `rails`, `sleepers` | |
| Bestandsachse | reference axis | `referenceAxes` | surveyed axis, points every centimetre |
| gemessene Achse | axis survey | `axisSurveys` | traced in point clouds |
| Punktwolke | point cloud | — | outside the project record |

## Catenary

| German | English | In the model | Note |
|---|---|---|---|
| Oberleitung (OL) | overhead contact line, catenary | — | the catenary domain |
| Mast | mast | — | |
| Ausleger | cantilever | — | |
| Fahrdraht, Tragseil | contact wire, messenger wire | — | |
| Fahrdrahthöhe, Seitenverschiebung | contact wire height, stagger | — | |
| Nachspannabschnitt | tension section | — | |
| Streckentrennung, Schutzstrecke | sectioning, neutral section | — | |
| Speiseabschnitt | feeding section | — | |
| El-Signal | electric traction signal | — | placed by the catenary planner |

## Control-command and signalling

| German | English | In the model | Note |
|---|---|---|---|
| Leit- und Sicherungstechnik (LST) | control-command and signalling (CCS) | — | the signalling domain |
| Hauptsignal, Vorsignal | main signal, distant signal | — | |
| Haltetafel, Geschwindigkeitstafel | stop board, speed board | — | |
| Achszähler | axle counter | — | |
| Gleisfreimeldeabschnitt | track section | — | track vacancy detection |
| Isolierstoß | insulated rail joint | — | |
| Balise | balise | — | |
| Fahrstraße | route | — | reserved for this meaning |
| Durchrutschweg | overlap | — | |
| Flankenschutz | flank protection | — | |
| Blockabschnitt | block section | — | |
| Stellwerk | interlocking | — | |
| Signalsicht | signal sighting | — | |

## Operations

| German | English | In the model | Note |
|---|---|---|---|
| Fahrplan | timetable | — | the operations domain |
| Zugfahrt | train run | — | |
| Halt | stop | — | |
| Fahrzeug, Triebfahrzeug | rolling stock, traction unit | — | |
| Fahrzeit | running time | — | |
| Sperrzeit | blocking time | — | |
| Leistungsfähigkeit, Kapazität | capacity | — | |

## Planning and data

| German | English | In the model | Note |
|---|---|---|---|
| Fachplanung | discipline | — | layout, catenary, signalling, … |
| Domäne, Fachdomäne | domain, discipline domain | — | unit of ownership, schema and version |
| Kern | core | `core` | the shared domain every other depends on |
| Planungsstatus: Bestand / Neubau / Rückbau / Provisorium | planning status: existing / new / removal / temporary | `status` | derived from the lifespan |
| Variante | variant | — | a named state of a project |
| Revision | revision | — | an immutable stored state |
| Arbeitskopie | working copy | — | |
| Einchecken | check in | — | |
| Zusammenführen | merge | — | |
| Bauzustand | construction stage | `stages` | ordered, from the existing to the final state |
| Lebensspanne | lifespan | `lifespan: { from, to }` | the stages an object exists in |
| Ersatz | replacement | `replaces` | a new object taking an old one's place from a stage on |
| Sperrung, Langsamfahrstelle (La) | closure, temporary speed restriction | — | the construction domain |
| Freigabe | approval | — | |
| Befund | finding | — | of a validation or check |
| Koordinationsprüfung | coordination check | — | between disciplines, stores only findings |
| Anker | anchor | `anchor`: `station`, `km`, `plane` | what a location keeps when the track changes |
| Planlage | plane position | `anchor: "plane"` | easting, northing in a plane |
| verwaist | orphaned | — | an object whose track is gone |
| Umhängen | carrying (a reference) | — | the core's remap |
| ID-Protokoll | id log | `idLog` | which tracks became which |
| abgeleitet | derived | — | rebuilt on load, never stored |
| Rückwärtsindex | reverse index | — | what stands on a track |
