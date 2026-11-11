# schemas

The one format of a project, **infra-data-2.0** ([decision 0021](../docs/decisions/0021-one-format-one-build-the-project-in-the-module.md),
the layers of [decision 0025](../docs/decisions/0025-layers-on-the-core.md); first called core 1.0): the JSON Schema of its
envelope and of each domain of the Rail-Data-Core, and the test vectors that hold the core's implementations to the same
results (canonical JSON, the schema check, whole documents). See the [data model](../docs/data-model.md) for the rules
behind them and [decision 0009](../docs/decisions/0009-core-1-0-in-the-data-model.md) for why the format has its shape.

A document is the core and the layers planned on it, each written in its tool and each free to be missing: `civil`
(Civil-Tool), `signalling` (Signalling-Tool), `electrification` (Catenary-Tool). The door reads an infra-data 1.0
document and brings it to 2.0: `layout` and the platforms into `civil`, the stopping places into the core.

| File | What it is |
|---|---|
| `infra-data.schema.json` | infra-data-2.0, a project as one JSON file (`.railacross.json`): the domains, the schemas it carries, its blobs |
| `core/2.0.schema.json` | Domain `core` 2.0: tracks, switches and connectors, end marks, km lines, the operational point register, the stopping places, the construction stages; the core types `TrackEnd`, `TrackLocation`, `TrackRange`, `TrackPath`, `Lifespan`, `Ref` |
| `civil/1.0.schema.json` | Domain `civil` 1.0: platforms, each naming the stopping places at it; named runs of tracks (`trackPaths`), measured axes, reference axes, the plan header, the project picture |
| `signalling/2.0.schema.json` | Domain `signalling` 2.0, after PlanPro 1.10: signals and boards with their logical signals (one per signalling system, as in OSRD), frames and aspects, detection points, track sections, switch elements, train and shunting routes with their setting and release times, overlaps, flank protection, the balises and magnets of train protection, train protection by ranges, station boundaries, itineraries ([decision 0011](../docs/decisions/0011-signalling-domain.md)); which systems, kinds, settings and aspects there are is the catalogue's (`packages/core/constraints/db-ril-301.json`) |
| `electrification/1.0.schema.json` | Domain `electrification` 1.0: the electrified ranges of the tracks and their systems |
| `infra-data-1.0.schema.json`, `core/1.0.schema.json`, `layout/1.0.schema.json`, `signalling/1.0.schema.json` | infra-data 1.0 as it was last, for reference: the door no longer checks against them, it brings such a document to 2.0 |
| `vectors/canonical.json` | Values as JSON text, their canonical JSON (RFC 8785) and its hash |
| `vectors/schema.json` | Values checked against a part of a schema: whether it takes them, and where it does not |
| `vectors/documents.json` | Whole documents: the door's verdict, and for those it takes the manifest and the document's hash — infra-data 1.0 ones among them |
| `vectors/infra-data-1.0.json` | infra-data 1.0 documents as the core wrote them, frozen: the inputs of the 1.0 cases of `documents.json` |

The schemas are linked into the core (`packages/core/schemas`) and read by its Rust crates (`crates/doc`); the types of the
document (`packages/core/types/document.d.ts`) are derived from them by `packages/core/tools/types.mjs`.

## A document

```json
{
  "format": "infra-data",
  "version": "2.0",
  "domains": {
    "core":  { "version": "2.0", "schema": "sha256:…", "fields": { … }, "collections": { … } },
    "civil": { "version": "1.0", "schema": "sha256:…", "dependsOn": ["core"], "fields": { … }, "collections": { … } }
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
  (R3).

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
