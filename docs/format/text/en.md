# Purpose

rail-across-1.0 describes a railway project in **one** JSON file: its infrastructure and its
operation. It is the basis of the data model of the Rail-Across-Suite: what is written here is what the tools
keep, exchange and version.

- **infra-data-1.0** holds the planning state of the infrastructure: the net with its alignment and its heights,
  switches and track ends, reference data such as km lines and operational points, the stopping places — and on
  it the layers of the disciplines: platforms and engineering structures (`civil`), signalling (`signalling`),
  the overhead contact line (`electrification`).
- **operations-1.0** holds the operation: the vehicles (`vehicles`) and the timetables with their trains, kinds
  of trains, formations, connections and turnarounds, delay scenarios and capacity studies (`timetables`).

The infrastructure is planned in variants; the operation belongs to no variant and names the infrastructure by
id only, so that a timetable can be laid on any variant.

The format follows seven principles:

- **Design values, not polylines.** A track is kept as what was designed: straights, circular arcs and
  transitions with length, radius and cant. Every drawn coordinate is derived from them.
- **Plane coordinates, one plane per track.** Alignments are designed and surveyed in projected planes
  (DB_REF/Gauß-Krüger, ETRS89/UTM, historical systems). A track keeps the plane it was designed in. Where the
  plane changes, two tracks meet at a connector.
- **What is derived is not stored.** Stations, polylines, switch symbols, platform areas, the extent of track
  sections, where a contact wire runs, and the construction states follow from the rest. What is not stored
  cannot contradict it.
- **Fixed ids, checked references.** Every object has an id of its own, a UUID. A reference names that id,
  never a name or a position in a list, and every reference must find its object. That makes it possible to
  merge two states object by object and field by field.
- **One model.** What is kept is what is worked on: an operational point is an object with an id, not a name
  repeated at every track; a construction state is a day, not an entry in a register.
- **Versions that count.** Every change of a schema raises the version of its domain. A reader brings an older
  file forward step by step to its own version.
- **What is unknown is kept.** A reader keeps a domain or a member it does not know as it came, and writes it
  back.

# Structure of a file

## The file and its parts

<!-- cols: 20%, 24%, 1fr -->
| Name | Extension | Content |
|---|---|---|
| rail-across-1.0 | `.railacross` (container), `.railacross.json` | the file: the project, the program that wrote it and when, the parts, the blobs |
| infra-data-1.0 | — (a part of the file) | the domain `core` and the layers `civil`, `signalling`, `electrification` |
| operations-1.0 | — (a part of the file) | the domains `vehicles` and `timetables` |

A file holds one part or both. A project server keeps them apart — the infrastructure in revisions per
variant, the operation in revisions of its own — and a file brings them together for exchange.

A file is exchanged in a **container** `.railacross`: a zip archive whose first entry is `mimetype`, stored
uncompressed and without an extra field, holding `application/vnd.rail-across+zip` — so the media type stands at
byte 38 —, and whose second is the file itself, `project.json`, compressed. Unzipped, `project.json` is the file
byte for byte. The JSON alone, ending in `.railacross.json`, is read as well. Whitespace in the JSON has no
meaning; a reader takes an indented file and a compact one alike.

## The envelope

```json
{
  "format": "rail-across",
  "version": "1.0",
  "projectId": "5dbd8fed-f8dc-4995-8934-9d71be69cae0",
  "generator": { "name": "Rail-Across-Suite", "version": "0.9.0" },
  "savedAt": "2026-11-13T09:30:00Z",
  "infraData": {
    "format": "infra-data", "version": "1.0",
    "domains": {
      "core": {
        "version": "6.0", "schema": "sha256:…",
        "fields": { … }, "collections": { … }
      },
      "civil":           { "version": "3.0", "dependsOn": ["core"], … },
      "signalling":      { "version": "4.0", "dependsOn": ["core"], … },
      "electrification": { "version": "3.0", "dependsOn": ["core"], … }
    }
  },
  "operations": {
    "format": "operations", "version": "1.0",
    "domains": {
      "vehicles":   { "version": "1.0", … },
      "timetables": { "version": "1.2", "dependsOn": ["vehicles"], … }
    }
  },
  "blobs": { "sha256:…": … }
}
```

<!-- cols: 26%, 1fr -->
| Member | Content |
|---|---|
| `format`, `version` | always `rail-across` and `1.0` |
| `projectId` | the id of the project, a UUID |
| `generator`, `savedAt` | the program and version that wrote the file, and when (ISO 8601, UTC) |
| `infraData` | the part infra-data-1.0: format, version and its domains; `core` is required |
| `operations` | the part operations-1.0: format, version and its domains |
| `blobs` | bulk data (measured points, points of km lines), each under the hash of its content, shared by both parts |

The file carries no schemas: a domain names its schema by hash, and a reader knows the versions it reads.

## Domains and layers

A **domain** is a field of its own, with its own schema and its own version. It has fields of its own (`fields`)
and collections (`collections`). A **layer** is a domain of infra-data on top of the core: it names `core` under
`dependsOn` and refers only into itself and into the core. Each layer is edited in its own tool.

<!-- cols: 16%, 22%, 1fr -->
| Part | Domain | Content |
|---|---|---|
| infra-data-1.0 | `core` 6.0 | the net: tracks, switches and connectors, track ends (buffer stops, boundaries of the planning area), km lines, operational points, stopping places, named runs of tracks. The base of all others |
| | `civil` 3.0 | platforms and engineering structures (railway bridges, retaining walls, noise barriers), axes measured in point clouds, reference axes, title block, project picture |
| | `signalling` 4.0 | signals with their logical signals, train detection, switch elements, routes, overlaps, flank protection, train protection, station boundaries, itineraries — exportable to PlanPro |
| | `electrification` 3.0 | masts, suspension points, section insulators and the contact wires with their voltage |
| operations-1.0 | `vehicles` 1.0 | the vehicles with their rolling stock as RailJSON |
| | `timetables` 1.2 | timetables, trains, kinds of trains, formations, connections, turnarounds, delay scenarios, studies, construction scenarios |

The domains of operations name the infrastructure by id: operational points, track ends, tracks, itineraries.
These references are checked where the operation is laid on the infrastructure — the infra-data part of the
same file, or a variant on a server.

## Collections and objects

- A collection is a list of objects with `id`, **sorted by `id`**. The same state thus gives the same bytes and
  the same hash.
- Sequences within an object (the columns of a track's plan, `heights`, the points of a train) keep their order; it
  means something.
- Every object allows only the properties its schema names (`additionalProperties: false`).
- **Ids are always UUIDs in lower case**, unique in their collection — also where an import draws them from
  something it read (a km line from its line number, a station from its name): then as a UUID drawn from that.

# Ground rules

## Hashes

A domain names its schema by `schema`: the SHA-256 of its **canonical JSON** after RFC 8785 (JSON
Canonicalization Scheme), written as `sha256:` and 64 hex digits. The hash serves integrity only — which
version a domain is, its `version` says. Every entry of `blobs` must carry exactly the hash of its content.

## References

A reference is the id of its target. In the schema the property carries the annotation `x-target`, naming the
collection it points into, e.g. `core.tracks` or `signalling.routes`, and its name ends in `Id` (`Ids` for a
list). Every such reference must find an object of that collection; a file whose reference points at nothing is
not accepted (see *Checks on reading*). Track ends are named as a pair of track and end (`TrackEnd`: `trackId`
and `endpoint` `BEGIN` or `END`).

## Locations on the track

Where an object stands is **one** value (`TrackLocation`) with an anchor:

- `anchor: "station"` — the object follows the track: `trackId`, `station` (m from the beginning of the track),
  `side` (`left`/`right`, seen in the direction of stationing), `offset` (m from the axis), `direction` (`up`
  with the stationing, `down` against it). Where a track is split, joined or turned round, these locations are
  carried along.
- `anchor: "plane"` — the object keeps its place in the plane: `easting`, `northing`, `epsg`, optionally
  `bearing`. So an object whose track was removed is kept.

A stretch of a track is a `TrackRange` (`trackId`, `startStation`, `endStation`, `side`), a run of tracks a
`TrackPath` (`trackIds` in running order; the direction of each follows from where they meet).

## Lifespan and construction states

Every object may carry a **lifespan**: the days it is in service.

<!-- cols: 46%, 1fr -->
| Lifespan | Meaning |
|---|---|
| `{ "from": "2027-05-01" }` | new from 1 May 2027 |
| `{ "from": "2027-05-01", "to": "2031-12-12" }` | temporary, from 1 May 2027 to 12 December 2031 |
| `{ "from": "open" }` | new, the day not known yet |
| `{ "to": "open" }` | to be removed, the day not known yet |
| none | exists and stays |

`from` is the first day in service, `to` the last. Without `from` the object exists today, without `to` it
stays. An object that is neither newly built nor removed carries **no** lifespan.

The **construction states** are not stored; they follow from the lifespans: the existing state, a state for
every day something is put into or taken out of service, and the final state.

<!-- cols: 24%, 22%, 30%, 1fr -->
| Lifespan | existing state | on a day T | final state |
|---|---|---|---|
| none | yes | yes | yes |
| `from`: day F | no | if F ≤ T | yes |
| `from`: `open` | no | no | yes |
| `to`: day L | yes | if T ≤ L | no |
| `to`: `open` | yes | yes | no |

Where both are stated, both conditions must hold. The status shown follows likewise: with `from` *new*, with
`to` alone *removal*, else *existing*. A switch without a lifespan of its own shows what its tracks say.

## Blobs

Bulk data is not kept in the object but in `blobs`: the points of measured and reference axes (`civil`), the
points of a km line where a source gave no design elements (`core`). The object names the blob by the hash of its canonical JSON. Equal content thus stands in the file
once.

## What is unknown is kept

A domain a reader does not know it keeps whole and writes back unchanged. A domain of a later minor version than
the reader knows it reads with the schema of the version it knows; members it does not know it keeps as they
came and writes back. A domain of a later major version it cannot read; it says so, and refuses the file.

# The net (domain `core`)

## Tracks and design elements

A track (`tracks`) has a plane (`epsg`) in which its plan is stated, a vertical reference (`heightEpsg`), its plan
by column (`horizontalAlignment`) and its heights (`heights`). A station track (`trackType: "station"`) names its
operational point (`operationalPointId`) and its track number, a line track (`trackType: "line"`) its line and which
track of the line it is (`side`: `regular` or `opposite`).

**The plan by column.** A track's design elements are not a list of objects but columns, as the OSRD export states
them. Over N elements `stationing`, `coordinates` and `bearingGon` hold N+1 values — each element's start, the last
the track's end —, the other columns N.

<!-- cols: 26%, 1fr -->
| Column | Content |
|---|---|
| `stationing` | the station of each element's start and of the track's end, m from 0; an element's length is the difference |
| `coordinates` | the point of each element's start and of the track's end, `[E, N]` in m |
| `bearingGon` | the bearing at each element's start and at the track's end, gon, clockwise from grid north |
| `elementType` | `straight`, `arc`, `clothoid` or `bloss` |
| `beginRadius`, `endRadius` | the radius at each element's start and end, m, signed: + curving right; `null` for a straight end; an arc's two are the same |
| `beginCant`, `endCant` | the cant at each element's start and end, mm, signed by the raised rail seen in the direction of stationing |
| `designSpeed` | the design speed of each element, km/h, `null` where it states none; missing where no element states one |
| `elementDetails` | what few elements carry, by row with the element's index (`element`, from 0): the marking of a switch's route (`switchId`, `switchRoute`, `switchBranch`), the role in changes (`role`), reasons for rule findings (`reasons`) |

**No gap, no kink after a curve.** Each element begins where the one before ends. Only a straight may leave in
another direction than it runs: its kink is the step from its bearing to the next one. An arc or a transition ends
on the bearing the next element begins on. A reader checks each element from its own start point: its values lead
to the next point within 1 mm, and an arc or a transition ends on the next bearing within what a millimetre of
length makes of it on its sharpest curvature. What lies further apart is refused; an import that brings such data
fits it first.

## Heights

The heights are a vertical tangent polygon, independent of the design elements: points with `station` (m from
the beginning of the track) and `z` (m, top of the rail that is not raised), optionally with a vertical curve
radius `rv` and the length `la` that governs it. A point may depend on another of the same track (`depId`,
`depMode`), e.g. in a switch. Between two points the gradient is constant.

## Switches and connectors

An object of the collection `switches` sits at a node. Its **ports** hold track ends (`TrackEnd`). Which ports
it has, `kind` fixes:

<!-- cols: 26%, 18%, 1fr -->
| `kind` | Ports | Meaning |
|---|---|---|
| `switch` | `A`, `B1`, `B2` | switch: `A` the toe, `B1` the branch, `B2` the main track — as OSRD's point switch is written, so that an export passes the ports on as they are |
| `crossing` | `A`, `B`, `C`, `D` | diamond crossing |
| `single_slip`, `double_slip` | `A`, `B`, `C`, `D` | single and double slip |
| `link` | `A`, `B` | connector: two track ends, where the plane changes or a rebuilt part ties in |
| `stage_link` | `A`, `B1`, `B2` | construction stage connector: the track at `A` is joined to the one at `B1` while that is in service, and to the one at `B2` from then on |

The routes of a switch are the elements of the tracks at it marked in their `elementDetails` with `switchId` and
`switchRoute` (`main`, `branch`). The form is stated in `label` (e.g. `300 – 1:9`) and refers to a catalogue of forms, `formVersion`
naming its version. The symbol of the switch is not stored.

## Operational points and stopping places

An **operational point** (`operationalPoints`) is an object with a fixed id: a station, a halt or another
operational point, with its name, RL100 code and UIC number. It is **found by its id**: tracks, platforms,
signals, stopping places and the points of trains refer to it by its UUID; its name is stated with it alone.
Renaming it changes exactly one field. Its boundaries are the signalling's (`stationBoundaries`).

A **stopping place** (`stoppingPlaces`) is where trains stop: a stretch of one track, in a station or on the open
line, for every train. It belongs to the core because signalling, platforms and timetables need it alike. It is a
**passenger stop** (`passengerStop`) where a platform lies at it and names it (layer `civil`, `stoppingPlaceIds`).

## Km lines

A **km line** (`kmLines`) is foreign reference data: the axis of a line around the project with its
kilometrage. It is kept **in design elements**, as a track is: in one plane (`epsg`), as runs from km jump to km
jump (`runs`). A run states its kilometrage at the start (`kmStart`, m) and the way it counts along the run
(`counting`: `up` or `down`), and its plan by column (`horizontalAlignment`) as a track does — without cant,
design speed and element details, which only a track has; its stations begin with 0 at the run's start. The
kilometrage of a point is the start value plus or minus its station.

That is far smaller than a polyline with a point every few metres, and exact where the source states elements —
the DB's line axes are designed in elements as the tracks are. The same calculation serves tracks and km lines.
For a track in another plane, the axis is transformed into it. Where a source delivers points only, a run holds
them as a blob (`points`: `[easting, northing, km]` per point) instead, until it is fitted with elements.

## Further collections

- `endMarks`: a buffer stop (`buffer_stop`) or a boundary of the planning area (`boundary`) at a free track end.
  A boundary states the speed trains cross it at and the least time between two trains.
- `trackPaths`: named runs of tracks, for the profiles and tables of the alignment.

# Platforms and structures (layer `civil`)

The layer holds what stands in the net without being part of it:

- **Platforms** (`platforms`) as a `TrackRange` with edge distance, height and operational point. A platform names
  the stopping places at it (`stoppingPlaceIds`), which are passenger stops by that.
- **Engineering structures**: railway bridges (`bridges`) with their structure, spans and clear height, retaining
  walls (`retainingWalls`) and noise barriers (`noiseBarriers`) with type, distance and height — each as a stretch
  of a track. Where the track is split or turned round, they are carried along.
- **Measured axes** (`axisSurveys`, points as whole millimetres in parallel lists, as a blob) and **reference
  axes** from surveys (`referenceAxes`, one point per centimetre, as a blob).
- The title block of the plan and the project picture (`fields`).

# Signalling (layer `signalling`)

The layer models signalling for railway planning. Its concepts map onto PlanPro 1.10 so that a plan can be
**exported to PlanPro**; it follows PlanPro only so far as that compatibility asks, not in its structure. Every
location stands on a track of the core.

- **Signals** (`signals`) are physical locations, real or virtual. A real signal carries a **logical signal**
  per signalling system (e.g. Ks main signal, Sh signal) with screen, indicators and the signalled speed per
  route. Boards without a system (Ne, Lf, Ra, Zs …) are frames of the location itself. A virtual signal marks
  where a route begins or ends without a signal, e.g. at a boundary of the planning area.
- **Aspects** name their key in the catalogue, the PlanPro name (`Hp_0`, `Ks_1`, `Zs_3`, `Lf_7`), with a symbol
  where the catalogue asks for one: for Zs 3 and Lf 7 the speed in tens of km/h (`7` = 70 km/h).
- **Train detection**: detection points (`detectionPoints`) bound track sections (`trackSections`). The extent of
  a section is not stored; it follows from its points over the net.
- **Switch elements** (`switchElements`) are the switches of the core and derailers, as the interlocking knows them.
- **Routes** (`routes`) lead from a start to a target signal over a `TrackPath`, with route setting and release
  times where they differ from the layer's. The switch positions follow from the run. Besides, **overlaps**
  (`overlaps`) and **flank protection** (`flankProtection`).
- **Train protection**: at points (`balises`: PZB magnets, speed checks, ETCS balises) and over stretches of
  track (`trainProtection`: PZB, LZB, ETCS with its level).
- **Station boundaries** (`stationBoundaries`) and **itineraries** (`itineraries`) from stopping place to
  stopping place.

Which signalling systems, kinds of signals, aspects and settings there are, the schema does not fix, but a
**catalogue** after DB Ril 301. The layer names it in `fields.catalogue` (`id`, `version`). A new signalling
system is a new entry of the catalogue, not a new schema.

# The overhead contact line (layer `electrification`)

The layer holds the overhead contact line. **Everything is referred to a track** of the core.

- **Masts** (`masts`) stand at a location on a track (station, side, distance from the axis): a single mast
  with a cantilever, a portal or a headspan across several tracks.
- **Suspension points** (`suspensionPoints`) belong to a mast and stand over a track: each holds a contact wire
  there, at its height and stagger. A portal holds one over each of its tracks.
- **Section insulators** (`sectionInsulators`) are parts of the contact line that separate two circuits on a
  track: they name the wire before them and the wire after them, in the direction of stationing. A neutral
  section is one of them.
- **Contact wires** (`contactWires`) are the circuits. A wire carries a **voltage**: `15000V` (15 kV 16.7 Hz),
  `25000V` (25 kV 50 Hz), `3000V`, `1500V`, `750V` or `600V` (DC), named as OSRD names the traction mode.

Where a wire runs is not stored: on a track, it runs from its first to its last carrying point there — the
suspension points and section insulators that name it. The voltage over a stretch of track is that of the wire
over it. **Operations read it**: a vehicle runs electrically over a stretch whose voltage is among its traction
modes; elsewhere it needs a mode of its own (`thermal`), or cannot run there.

# The operation (part operations-1.0)

## Vehicles (domain `vehicles`)

The vehicles are a domain of their own. A vehicle (`vehicles`) has the key it came under from a catalogue
(`key`) and its **rolling stock** as RailJSON (`rollingStock`), the format of OSRD — kept in the object, not as a
blob: a vehicle is a few kilobytes, and kept in place two states of it can be compared and merged field by field.
The schema checks what the suite reads from it — the RailJSON version, the name and the tractive effort curves —
and takes every further RailJSON property as it comes, in the units of RailJSON (m, kg, m/s, N).

The **traction modes** of a vehicle are the modes of its effort curves (`effort_curves.modes`): the voltages it can
draw, named as the voltages of the contact wires, and `thermal` for a drive of its own. They are not stored a second
time. The timetables name vehicles by their id. A file thus runs with the vehicles it holds, not with a catalogue
that may have changed since; a changed vehicle is a new version of the domain, as any other change. A vehicle without
its rolling stock is one of the suite's catalogue, named by its key — what a project server keeps where its trains
run with the catalogue's vehicles; a file that travels carries the rolling stock.

## Timetables (domain `timetables`)

- A **timetable** (`timetables`) gathers trains and says what happens where they meet (`dispatching`: only show
  the conflicts, or give the trains the net one after the other, by departure or by kind), and which parts of the
  blocking time the operation gives (`blockingTimes`).
- A **train** (`trains`) has a number, a kind, a formation, a departure in seconds from midnight of the
  timetable's day (past 24:00 the next day), a running time margin and its **points**: each an operational point
  (`operationalPointId`) or a track end — buffer stop or boundary (`endMarkId`) — **by its UUID**, with the track
  (`trackId`), whether it stops, its dwell, a departure, and the itinerary it prefers to the next point
  (`itineraryId`).
- **Kinds of trains** (`trainKinds`) with rank and colour; **formations** (`formations`) of vehicles front to back
  (`vehicleId`), hauled load and brake.
- **Connections** (`connections`) and **turnarounds** (`turnarounds`) between trains.
- **Delay scenarios** (`delayScenarios`) and **capacity studies** (`studies`) after UIC 406.
- **Construction scenarios** (`constructionScenarios`) of a timetable (`timetableId`): stretches of track closed for a
  while (`closures`: a track range, a first and a last day, a daily window), the trains taken out
  (`cancelledTrainIds`) and the trains diverted by hand with their points (`diversions`).

## How the operation reads the infrastructure

A train finds its operational points, track ends, tracks and itineraries by their ids in the infrastructure it is
laid on; where a variant lacks one, the train cannot be laid on it there, and a tool says which. Where it runs
electrically, the voltages of the contact wires along its way and the traction modes of its vehicles decide.

# Units and precision

<!-- cols: 36%, 30%, 1fr -->
| Quantity | Unit | Precision |
|---|---|---|
| coordinates, lengths, radii, stations | m | 0.1 µm (7 decimals) |
| heights `z` | m | 0.1 mm (4 decimals) |
| bearings | °, clockwise from grid north | 10⁻⁹ ° |
| cant | mm | 0.001 mm |
| speeds | km/h; of vehicles (RailJSON) m/s | as stated |
| times of the operation | s | as stated |
| voltage | V, as named | — |
| kilometrage | m | as delivered |

Positions are kept this fine because values are chained when read again: a rounded point would make a kink at a
switch, and a rounded bearing makes a long track drift sideways.

# Checks and versions

## Checks on reading

A reader accepts a file only after a check:

1. Format and version of the envelope and of each part; anything else is refused as `unsupported_version`.
2. The hashes of the blobs.
3. The version of each domain. An earlier version is brought forward (below); a later minor version is read
   with what is unknown kept; a later major version is refused, saying so.
4. Each domain against its schema, every id a UUID and only once in its collection.
5. What a schema cannot express: the days of a lifespan are days, the first not after the last; an element marked
   as a route of a switch names its switch; a traction mode of a vehicle is electric exactly when it is a voltage;
   **every reference finds its object** — within the part it is in;
   those of the operation into the infrastructure where the operation is laid on it.

Faults of steps 2, 4 and 5 are reported as `invalid_payload` with the places found, e.g.
`/infraData/domains/signalling/collections/switchElements/0/switchId: names …, which core.switches lacks`. What a
reader writes it checks the same way before it leaves.

The check uses the part of JSON Schema these schemas need: `type`, `enum`, `const`, `properties`, `required`,
`additionalProperties`, `patternProperties`, `minProperties`, `items`, `minItems`, `maxItems`, `minimum`,
`maximum`, `minLength`, `pattern`, `oneOf`, `anyOf`, `allOf` and `$ref`, also into another schema by its `$id`
(e.g. `core#/$defs/TrackEnd`).

Besides, the tools check the content as engineering asks: whether locations lie on their track, whether signals
show only aspects the catalogue allows, whether routes fit their signals. The format does not prescribe such
findings; a project server checks in no state that brings new findings of this kind.

## Versions

**A schema is a version.** Every change of a schema — also one that only changes a rule the schema names —
raises the version of its domain: the minor version for additions an older reader may skip, else the major
version. A reader brings an older file forward step by step, number by number; older states are brought forward
once, not read leniently for ever.

The versions of the domains continue the suite's counting: core 6.0, civil 3.0, signalling 4.0, electrification
3.0; vehicles 1.0 and timetables 1.0 are the domain operations 1.0 of before, split in two. Core 4.0 had a switch's
`B1` its main track; 5.0 has it the branch again, and a reader brings a 4.0 file forward by swapping the two. Core
5.0 had a track's elements as a list of objects; 6.0 states its plan by column, and a reader brings a 5.0 file
forward element by element, closing a gap within a millimetre and refusing a track whose elements lie further
apart.
Timetables 1.1 adds a train's reversal to its point (`reverse`, `turnaround`); a reader brings a 1.0 file forward
as it is. Timetables 1.2 adds the construction scenarios (`constructionScenarios`); a reader brings a 1.1 file forward
with none.

# An example

A file with a track of a straight and an arc, new from 1 May 2027, a boundary at its end, its km line, a mast
with a suspension point holding a contact wire at 15 kV, a vehicle, and a train from the boundary to
Halle-Trotha, where it stops on that track:

```json
{
  "format": "rail-across", "version": "1.0",
  "projectId": "5dbd8fed-f8dc-4995-8934-9d71be69cae0",
  "generator": { "name": "Rail-Across-Suite", "version": "0.9.0" },
  "savedAt": "2026-11-13T09:30:00Z",
  "infraData": {
    "format": "infra-data", "version": "1.0",
    "domains": {
      "core": {
        "version": "6.0", "schema": "sha256:1111…",
        "fields": { "id": "5dbd8fed-f8dc-4995-8934-9d71be69cae0", "title": "Halle-Trotha" },
        "collections": {
          "tracks": [ {
            "id": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b", "name": "6050.001",
            "trackType": "line", "lineNumber": 6050, "epsg": 5684, "heightEpsg": 7837,
            "lifespan": { "from": "2027-05-01" },
            "horizontalAlignment": {
              "stationing": [0, 100, 200],
              "coordinates": [[4497120.512, 5706210.004], [4497220.512, 5706210.004], [4497320.05, 5706201.69]],
              "bearingGon": [100, 100, 110.610329539],
              "elementType": ["straight", "arc"],
              "beginRadius": [null, 600], "endRadius": [null, 600],
              "beginCant": [0, 60], "endCant": [0, 60],
              "designSpeed": [100, 100]
            },
            "heights": [ { "station": 0, "z": 108.55 }, { "station": 200, "z": 109.15 } ]
          } ],
          "switches": [],
          "endMarks": [ {
            "id": "b2c3d4e5-f6a7-4b8c-9d0e-1f2a3b4c5d6e", "kind": "boundary",
            "name": "Grenze Ost", "speed": 100,
            "at": { "trackId": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b", "endpoint": "END" }
          } ],
          "kmLines": [ {
            "id": "0f9e8d7c-6b5a-4f3e-8d2c-1b0a9f8e7d6c", "lineNumber": 6050, "epsg": 5684,
            "runs": [ { "kmStart": 12400.0, "counting": "up", "horizontalAlignment": {
              "stationing": [0, 250], "coordinates": [[4497050.0, 5706220.0], [4497300.0, 5706220.0]],
              "bearingGon": [100, 100], "elementType": ["straight"],
              "beginRadius": [null], "endRadius": [null] } } ]
          } ],
          "operationalPoints": [ {
            "id": "b64cfd92-f881-43ad-82d7-f0d78ef78747", "name": "Halle-Trotha",
            "rl100": "LHT", "uic": "8023111"
          } ],
          "stoppingPlaces": []
        }
      },
      "electrification": {
        "version": "3.0", "schema": "sha256:2222…", "dependsOn": ["core"], "fields": {},
        "collections": {
          "contactWires": [ {
            "id": "c1d2e3f4-a5b6-4c7d-8e9f-0a1b2c3d4e5f", "name": "Kettenwerk 1",
            "voltage": "15000V"
          } ],
          "masts": [ {
            "id": "d1e2f3a4-b5c6-4d7e-8f9a-0b1c2d3e4f5a", "name": "12/4",
            "kind": "cantilever",
            "at": { "anchor": "station", "trackId": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b",
                    "station": 40, "side": "right", "offset": 3.2 }
          } ],
          "suspensionPoints": [ {
            "id": "e1f2a3b4-c5d6-4e7f-8a9b-0c1d2e3f4a5b",
            "mastId": "d1e2f3a4-b5c6-4d7e-8f9a-0b1c2d3e4f5a",
            "at": { "anchor": "station", "trackId": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b",
                    "station": 40 },
            "contactWireId": "c1d2e3f4-a5b6-4c7d-8e9f-0a1b2c3d4e5f",
            "height": 5.5, "stagger": 0.3
          } ],
          "sectionInsulators": []
        }
      }
    }
  },
  "operations": {
    "format": "operations", "version": "1.0",
    "domains": {
      "vehicles": {
        "version": "1.0", "schema": "sha256:3333…", "fields": {},
        "collections": { "vehicles": [ {
          "id": "a7b8c9d0-e1f2-4a3b-8c4d-5e6f7a8b9c0d", "key": "osrd:br1440",
          "rollingStock": {
            "railjson_version": "3.3", "name": "BR 1440", "mass": 84000, "max_speed": 44.44,
            "effort_curves": {
              "default_mode": "15000V",
              "modes": { "15000V": { "is_electric": true, "curves": [ {
                "cond": { "comfort": "STANDARD" },
                "curve": { "speeds": [0, 20, 44.44],
                           "max_efforts": [300000, 300000, 135000] }
              } ] } }
            }
          }
        } ] }
      },
      "timetables": {
        "version": "1.2", "schema": "sha256:5555…", "dependsOn": ["vehicles"], "fields": {},
        "collections": {
          "timetables": [ {
            "id": "f0e1d2c3-b4a5-4968-8776-655443322110", "name": "Grundtakt"
          } ],
          "trains": [ {
            "id": "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
            "timetableId": "f0e1d2c3-b4a5-4968-8776-655443322110", "number": "16001",
            "formationId": "8b7c6d5e-4f3a-4b2c-9d1e-0f9a8b7c6d5e",
            "departure": 28800, "margin": "5 %", "distribution": "mareco",
            "points": [
              { "id": "7c6d5e4f-3a2b-4c1d-8e0f-9a8b7c6d5e4f",
                "endMarkId": "b2c3d4e5-f6a7-4b8c-9d0e-1f2a3b4c5d6e", "stop": false },
              { "id": "6d5e4f3a-2b1c-4d0e-9f8a-7b6c5d4e3f2a",
                "operationalPointId": "b64cfd92-f881-43ad-82d7-f0d78ef78747",
                "trackId": "6e1f2a3b-0c4d-4e5f-8a6b-7c8d9e0f1a2b",
                "stop": true, "dwell": 60 }
            ]
          } ],
          "trainKinds": [],
          "formations": [ {
            "id": "8b7c6d5e-4f3a-4b2c-9d1e-0f9a8b7c6d5e", "name": "Lok mit 5 Wagen",
            "tractionUnits": [ { "vehicleId": "a7b8c9d0-e1f2-4a3b-8c4d-5e6f7a8b9c0d" } ]
          } ],
          "connections": [], "turnarounds": [], "delayScenarios": [], "studies": [],
          "constructionScenarios": []
        }
      }
    }
  },
  "blobs": {}
}
```

The hashes are shortened here; in a file each has its 64 hex digits.

# Relation to the current implementation

This specification is the target of the data model. As of 13 November 2026 the suite's implementation differs
from it in these points, each to be brought in line by its own step (decision 0006 and those after it):

- The project server keeps a variant's infrastructure as a document of the envelope of before (infra-data 2.0:
  its domains and its blobs, core 6.0, civil 3.0, signalling 4.0, electrification 3.0) and the operation as an
  operations document (vehicles 1.0, timetables 1.2); the file rail-across-1.0 brings them together, and a file of
  before is still read.
- A km line shipped with the app is fitted into design elements when it is fetched only where the fit stays
  within 1 cm of every point; its files give points a centimetre apart at best, so its runs mostly keep their
  points.
- A domain the reader does not know is kept; one of a later version of a domain it knows is refused, not read
  with the version it knows.
