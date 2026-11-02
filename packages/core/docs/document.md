# document — the core 1.0 document and its door

`packages/core/src/document/` · needs nothing

A project as the Rail-Data-Core keeps it and hands it on: the **core 1.0 document**, in the shape of the
[data model](../../../docs/data-model.md) — the domains `core` and `layout`, and `signalling` where a project has it, each with its version, its JSON
Schema, its fields and its collections; locations as one value each; the planning status as a lifespan over
the construction stages; station names and codes as an operational point register; named runs of tracks as
`TrackPath`s; bulk point arrays as blobs named by their hash. The schemas are in [`schemas/`](../../../schemas/README.md)
at the root of the repository (linked into the package), with the test vectors that hold the Python core to the
same results.

The module checks documents (the door), turns the record the Layout-Tool works on into one and back, and
writes canonical JSON and content hashes. The `project` module opens the door for the app and the server
(`parseProjectsPayload`, `projectDocument`).

## Files

| File | What it does |
|---|---|
| `door.js` | The one door: `checkDocument` (envelope, domain versions and schema hashes — a domain of an earlier schema of its version brought to this one first: core before `designSpeed`, signalling's three earlier schemas —, every object against its domain's schema, ids unique per collection, blobs against their hashes), `toDocument` (a record as a checked document, the schemas carried where it travels), `fromDocument` (a checked document as the record) |
| `coreFormat.js` | The record ↔ document conversion, both ways and lossless; the first step of the migration chain (oltjson-1.1 → core 1.0) |
| `schemaCheck.js` | The part of JSON Schema the domain schemas use — no more, so Python reads them alike; a schema with another keyword is refused when it is registered |
| `canonical.js` | Canonical JSON after RFC 8785, content hashes (`sha256:` + hex), ids of UUID form drawn from a text |
| `sha256.js` | SHA-256, synchronous, in plain JavaScript (the browser's `crypto.subtle` only answers asynchronously) |
| `manifest.js` | The manifest of a document — per domain the hash of its fields and of each collection, the hash tree of the data model — and the document's hash |
| `payloadError.js` | `PayloadError`: the code the UI translates, and the breaches found |

## The document

```json
{
  "format": "rail-across",
  "domains": {
    "core":   { "version": "1.0", "schema": "sha256:…", "fields": { "id": "…", "stages": [ … ] },
                "collections": { "tracks": [ … ], "switches": [ … ], "platforms": [ … ], "endMarks": [ … ],
                                 "kmLines": [ … ], "operationalPoints": [ … ] } },
    "layout": { "version": "1.0", "schema": "sha256:…", "dependsOn": ["core"], "fields": { … },
                "collections": { "trackPaths": [ … ], "axisSurveys": [ … ], "referenceAxes": [ … ] } }
  },
  "schemas": { "sha256:…": { … } },
  "blobs":   { "sha256:…": { … } }
}
```

What each object holds, the schemas say (`schemas/core/1.0.schema.json`, `schemas/layout/1.0.schema.json`,
`schemas/signalling/1.0.schema.json`); how
it differs from oltjson-1.1:

| oltjson-1.1 (the record) | core 1.0 (the document) |
|---|---|
| one object with everything | `core`: the net, what stands on it, reference data, stages; `layout`: what only the layout tool needs |
| `status`: `existing`, `new`, `removal` | `lifespan`: none, `{ from: final }`, `{ to: final }` over `stages` (existing state, final state) |
| `stationName`, `uicStation` (tracks), `stationName`, `code` (platforms) | `operationalPoint`, a reference into `operationalPoints` `{ id, name, rl100, uic }` |
| a platform's `trackId`, `startStation`, `endStation`, `side` | `at`: a `TrackRange` `{ anchor: 'station', trackId, startStation, endStation, side }` |
| an end mark's `trackId`, `endpoint` | `at`: a `TrackEnd` `{ trackId, endpoint }` |
| a switch's `switchId` | `id`, like every object of every collection |
| an element's `speed` | `designSpeed`, as the data format names the design speed beside the speeds the signalling signals |
| `routes` `{ id, name, trackIds }` | `layout.trackPaths` `{ id, name, path: { trackIds } }` |
| `signalling` `{ fields, collections }`, carried as it came, a member that is `undefined` left out as JSON leaves it | the domain `signalling`, `dependsOn: ["core"]` ([signalling](signalling.md)) |
| a survey's or reference axis' `points` inline | `points: "sha256:…"`, the array a blob |
| collections in the order they were written | sorted by id, so the same state is the same bytes |

## How it behaves

**One door, both ways.** What comes in is checked (`checkDocument`), and so is what goes out: `toDocument`
checks the document it made, so a record with a field the format does not have is refused when it is saved, not
when someone else opens it.

**Lossless.** A record turned into a document and back is the record; a document turned into a record and back
is the same bytes. Where the record says one thing two ways, the document says it one way: an explicit status
`existing` is no lifespan, an empty station text no station, `null` in a field of a track, switch, platform or end
mark an absent field (in an element, `null` keeps its meaning — a straight end).

**What the app does not hold yet is drawn from what it holds.** The record has no stages and no register; the
document's are drawn from it: two stages with ids drawn from the project's id, one operational point per station
name with its id drawn from the name and the codes stated for it (two codes under one name: two points). So the
same record gives the same ids on every device. A document with stages of its own is refused
(`stages_unsupported`) until the app holds stages (roadmap, phase 2).

**Schemas by hash.** A domain names its schema by the hash of its canonical JSON; a document of a known version
must name exactly that schema. A file that travels carries the schemas (`schemas`), each checked against the
hash it is filed under.

## Decisions

- **core 1.0 has the data model's shape from the start** ([decision 0009](../../../docs/decisions/0009-core-1-0-in-the-data-model.md)).
- **The record stays the app's working model in phase 1**; files and the project server hold documents.
- **A schema check of our own**, the part of JSON Schema the schemas use, the same in JavaScript and Python.
- **Canonical JSON and SHA-256 written out in plain code**, so both languages make the same bytes and hashes.

## Tests

`canonical.test.js` (SHA-256 against node:crypto, the RFC's examples), `schemaCheck.test.js`, `door.test.js`
(the conversion both ways, the door's verdicts, the manifest, the PEK data set losslessly, the shared vectors
current). `node tools/vectors.mjs` writes the shared vectors to `schemas/vectors/`; the Python core is held to
them.
