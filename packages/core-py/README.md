# rail-across-core (Python)

The Rail-Data-Core for Python: what a Python module — the optimizer, a simulation — needs to read a project of
Rail-Across-Suite with the same results as the JavaScript core (`packages/core`):

- `check_document(doc)` — the one door for a core 1.0 document: the envelope, each domain's version and
  schema, every object against its domain's JSON Schema, ids unique per collection, blobs against their hashes.
  Raises a `PayloadError` (`invalid_payload`, `unsupported_version`) with what it found.
- `canonical_json(value)`, `content_hash(value)` — canonical JSON after RFC 8785 and its SHA-256, the same bytes
  the browser and the server write.
- `manifest_of(doc)`, `document_hash(doc)` — the hash tree of a document: per domain the hash of its fields and of
  each collection.
- `SchemaSet` — the part of JSON Schema the domain schemas use, read exactly as the JavaScript core reads it.

No dependencies beyond the standard library; Python 3.10 or later. The schemas are those at the root of the
repository (`schemas/`, linked into the package).

```python
import json
from rail_across_core import check_document, document_hash

with open('Halle-Trotha.railacross.json', encoding='utf-8') as f:
    doc = check_document(json.load(f))
tracks = doc['domains']['core']['collections']['tracks']
```

## One implementation per language

Both cores are held to the shared vectors in `schemas/vectors/` — canonical JSON, the schema check, whole
documents with their verdicts, manifests and hashes. The JavaScript core writes them
(`node packages/core/tools/vectors.mjs`); a change to one core is a change to both.

```bash
python -m unittest discover -s tests
```
