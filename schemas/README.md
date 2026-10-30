# schemas

The JSON Schema of each domain of the Rail-Data-Core, and the test vectors that hold the JavaScript and Python
implementations to the same results (canonical JSON, the schema check, whole documents). See the
[data model](../docs/data-model.md) for the rules behind them and
[decision 0009](../docs/decisions/0009-core-1-0-in-the-data-model.md) for why core 1.0 has its shape.

| File | What it is |
|---|---|
| `rail-across.schema.json` | A project as one JSON file (`.railacross.json`): the domains, the schemas it carries, its blobs |
| `core/1.0.schema.json` | Domain `core` 1.0: tracks, switches and connectors, platforms, end marks, km lines, the operational point register, the construction stages; the core types `TrackEnd`, `TrackLocation`, `TrackRange`, `TrackPath`, `Lifespan`, `Ref` |
| `layout/1.0.schema.json` | Domain `layout` 1.0: named runs of tracks (`trackPaths`), measured axes, reference axes, the plan header, the project picture |
| `vectors/canonical.json` | Values as JSON text, their canonical JSON (RFC 8785) and its hash |
| `vectors/schema.json` | Values checked against a part of a schema: whether it takes them, and where it does not |
| `vectors/documents.json` | Whole documents: the door's verdict, and for those it takes the manifest and the document's hash |

The schemas are linked into both cores (`packages/core/schemas`, `packages/core-py/schemas`); the types of the
document (`packages/core/types/document.d.ts`) are derived from them by `packages/core/tools/types.mjs`.

## A document

```json
{
  "format": "rail-across",
  "domains": {
    "core":   { "version": "1.0", "schema": "sha256:…", "fields": { … }, "collections": { … } },
    "layout": { "version": "1.0", "schema": "sha256:…", "dependsOn": ["core"], "fields": { … }, "collections": { … } }
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
