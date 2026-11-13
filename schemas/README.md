# schemas

The one format of a project, **infra-data-2.0** ([decision 0021](../docs/decisions/0021-one-format-one-build-the-project-in-the-module.md),
the layers of [decision 0025](../docs/decisions/0025-layers-on-the-core.md), the lifespans by day of
[decision 0028](../docs/decisions/0028-one-model-lifespans-by-day.md); first called core 1.0): the JSON Schema of its
envelope and of each domain of the Rail-Data-Core, and the test vectors that hold the core's implementations to the same
results (canonical JSON, the schema check, whole documents). See the [data model](../docs/data-model.md) for the rules
behind them and [decision 0009](../docs/decisions/0009-core-1-0-in-the-data-model.md) for why the format has its shape.

A document is the core and the layers planned on it, each written in its tool and each free to be missing: `civil`
(Civil-Tool), `signalling` (Signalling-Tool), `electrification` (Catenary-Tool). The door brings a document of
before forward, version by version: an infra-data 1.0 one to 2.0 (`layout` and the platforms into `civil`, the
stopping places into the core), one with a register of construction stages to lifespans by day (core 3.0 —
a stage's day the day itself, a `to` the day before it, the final stage `"open"`, the existing one no statement),
one of 3.0 whose references may name nothing to references that name something (core 3.1 — what names nothing is
cut as deleting it would have cut it).

The door checks what a schema cannot say: a lifespan's days are days, the first not after the last; every
reference — a member the schema marks with `x-target`, the collection it points into — names an object of that
collection; a switch's branch names its switch.

A lifespan states days (decision 0028): `from` the first day in service, `to` the last, `"open"` where the day is
not known yet; no `from` — it exists today, no `to` — it stays, no lifespan — existing and staying. Every object
may carry one; the states a project passes through are worked out of them, never kept.

| File | What it is |
|---|---|
| `infra-data.schema.json` | infra-data-2.0, a project as one JSON file (`.railacross.json`): the domains, the schemas it carries, its blobs |
| `core/3.1.schema.json` | Domain `core` 3.1: tracks, switches and connectors, end marks, km lines, the operational point register, the stopping places (passenger stops among them), the named runs of tracks (`trackPaths`); the core types `TrackEnd`, `TrackLocation`, `TrackRange`, `TrackPath`, `Lifespan` (by day), `Day`, `Ref` |
| `civil/2.1.schema.json` | Domain `civil` 2.1: platforms, each naming the stopping places at it; bridges, retaining walls and noise barriers; measured axes, reference axes, the plan header, the project picture |
| `signalling/3.1.schema.json` | Domain `signalling` 3.1, after PlanPro 1.10: signals and boards with their logical signals (one per signalling system, as in OSRD), frames and aspects, detection points, track sections, switch elements, train and shunting routes with their setting and release times, overlaps, flank protection, the balises and magnets of train protection, train protection by ranges, station boundaries, itineraries ([decision 0011](../docs/decisions/0011-signalling-domain.md)); which systems, kinds, settings and aspects there are is the catalogue's (`packages/core/constraints/db-ril-301.json`) |
| `electrification/2.1.schema.json` | Domain `electrification` 2.1: the electrified ranges of the tracks and their systems |
| `core/3.0.schema.json`, `civil/2.0.schema.json`, `signalling/3.0.schema.json`, `electrification/2.0.schema.json` | the domains with lifespans by day as they were before references had to name something (decision 0028, phase A), for reference |
| `core/2.0.schema.json`, `civil/1.0.schema.json`, `signalling/2.0.schema.json`, `electrification/1.0.schema.json` | the domains as they were last with a register of construction stages (decision 0027), for reference: the door no longer checks against them, it brings such a document forward |
| `infra-data-1.0.schema.json`, `core/1.0.schema.json`, `layout/1.0.schema.json`, `signalling/1.0.schema.json` | infra-data 1.0 as it was last, for reference: the door no longer checks against them, it brings such a document to 2.0 |
| `operations.schema.json`, `operations/1.0.schema.json` | operations-1.0, a project's operations as one JSON file (`.operations.json`) in the envelope of infra-data: the domain `operations` 1.0 — timetables, trains, train kinds, formations, the vehicles they run with (carried as blobs), connections, turnarounds, delay scenarios, studies (the [roadmap](../ROADMAP.md), phase 4) |
| `vectors/canonical.json` | Values as JSON text, their canonical JSON (RFC 8785) and its hash |
| `vectors/schema.json` | Values checked against a part of a schema: whether it takes them, and where it does not |
| `vectors/documents.json` | Whole documents: the door's verdict, and for those it takes the manifest and the document's hash — infra-data 1.0 ones and ones with stages among them |
| `vectors/operations.json` | Whole operations documents: the door's verdict, and for those it takes the document's hash |
| `vectors/infra-data-1.0.json` | infra-data 1.0 documents as the core wrote them, frozen: the inputs of the 1.0 cases of `documents.json` |
| `vectors/infra-data-2.0-stages.json` | infra-data 2.0 documents as the core wrote them last with a register of stages, frozen: the inputs of the `stages:` cases of `documents.json` |

The schemas are linked into the core (`packages/core/schemas`) and read by its Rust crates (`crates/doc`); the types of the
document (`packages/core/types/document.d.ts`) are derived from them by `packages/core/tools/types.mjs`.

## A document

```json
{
  "format": "infra-data",
  "version": "2.0",
  "domains": {
    "core":  { "version": "3.1", "schema": "sha256:…", "fields": { … }, "collections": { … } },
    "civil": { "version": "2.1", "schema": "sha256:…", "dependsOn": ["core"], "fields": { … }, "collections": { … } }
  },
  "schemas": { "sha256:…": { … } },
  "blobs":   { "sha256:…": { … } }
}
```

- A domain names its schema by the hash of the schema's canonical JSON. A document of a known version must name
  exactly that schema; a file that travels carries the schemas under those hashes (R3).
- Collections are arrays of objects with an `id`, sorted by it (S-R3); within an object, sequences (`elements`,
  `heights`) keep their order.
- Bulk point arrays are blobs: the object names the hash, `blobs` holds the value whose canonical JSON has it
  (S-R2).
- A reference is the bare id of its target; the schema marks it with `x-target`, the collection it points into
  (R3), and the door holds it to name an object there (decision 0028).

## The schema check

Both cores check documents with a schema check of their own that understands the part of JSON Schema these
files use — `type`, `enum`, `const`, `properties`, `required`, `additionalProperties`, `patternProperties`,
`minProperties`, `items`, `minItems`, `maxItems`, `minimum`, `maximum`, `minLength`, `pattern`, `oneOf`, `anyOf`,
`allOf`, `$ref` (also into another schema by its `$id`: `core#/$defs/TrackEnd`), and annotations (`description`,
`x-…`). A schema using another keyword is refused when it is loaded, rather than half understood. Before a schema
uses a new keyword, both checks learn it, with vectors.

## Changing a schema

A schema is a version: its hash is in every document written against it. A change is a new version of the domain
— `1.1` for what older readers may ignore, `2.0` otherwise — with its own file, a step of the migration chain at
the door (S-R10), and the vectors written again (`node packages/core/tools/vectors.mjs`). The owner of
`schemas/` approves it (`.github/CODEOWNERS`).
