"""The core 1.0 document: its one door and its manifest, as the JavaScript
core has them (packages/core/src/document/door.js, manifest.js) — the same
verdicts, the same hashes, held to schemas/vectors/documents.json.

A module reading a project (the optimizer, a simulation) takes it through
``check_document`` and may then take the model for granted.
"""

import json
from pathlib import Path

from .canonical import content_hash
from .schema_check import SchemaSet

#: The schemas, at the root of the repository (linked into this package).
SCHEMAS_DIR = Path(__file__).resolve().parent.parent / 'schemas'

#: The file format's name, as the envelope states it.
FORMAT = 'rail-across'

#: The domains this build reads, with their versions.
DOMAIN_VERSIONS = {'core': '1.0', 'layout': '1.0', 'signalling': '1.0'}


def _load(rel):
    with open(SCHEMAS_DIR / rel, encoding='utf-8') as f:
        return json.load(f)


ENVELOPE = _load('rail-across.schema.json')
DOMAIN_SCHEMAS = {
    'core': _load('core/1.0.schema.json'),
    'layout': _load('layout/1.0.schema.json'),
    'signalling': _load('signalling/1.0.schema.json'),
}
#: The hash each domain's schema goes by — what a document names it with.
SCHEMA_HASHES = {name: content_hash(s) for name, s in DOMAIN_SCHEMAS.items()}
_SCHEMAS = SchemaSet([ENVELOPE, *DOMAIN_SCHEMAS.values()])


class PayloadError(ValueError):
    """A document this build will not take: ``code`` (invalid_payload,
    unsupported_version) and ``details``, the breaches as "path: what" lines."""

    def __init__(self, code, details=()):
        super().__init__(f"{code}: {'; '.join(list(details)[:5])}" if details else code)
        self.code = code
        self.details = list(details)


def _breaches(errors, prefix):
    return [f"{prefix}{'' if e['path'] == '/' else e['path']}: {e['message']}" for e in errors]


#: Core 1.0 as it was written before, by the hash its schema went by, and the
#: step that brings a domain of it one schema on — the steps the JavaScript
#: core's door takes (S-R10). First: an element's ``speed`` is its
#: ``designSpeed`` now. Second: an end mark may have a name, and a boundary
#: the speed a train crosses it at; none has yet.
CORE_FIRST = 'sha256:c237af09e3ac3915398d9c7795088518df4023bfd0b954053dc9f04112fbbaf6'
CORE_SECOND = 'sha256:16ae19c7adbb08fedabbb0ad778fc765c7faa8a8b2c56cbdbc9adfa91f622a19'


def _core_with_design_speed(domain):
    def element(el):
        if 'speed' not in el:
            return el
        out = {k: v for k, v in el.items() if k != 'speed'}
        out['designSpeed'] = el['speed']
        return out
    tracks = [dict(t, elements=[element(el) for el in t['elements']]) if 'elements' in t else t
              for t in domain.get('collections', {}).get('tracks', [])]
    return dict(domain, schema=CORE_SECOND, collections=dict(domain['collections'], tracks=tracks))


_CORE_STEPS = {
    CORE_FIRST: _core_with_design_speed,
    CORE_SECOND: lambda domain: dict(domain, schema=SCHEMA_HASHES['core']),
}


def _migrated(doc):
    core = doc['domains'].get('core')
    if not core or core.get('version') != '1.0' or core.get('schema') not in _CORE_STEPS:
        return doc
    while core.get('schema') in _CORE_STEPS:
        core = _CORE_STEPS[core['schema']](core)
    return dict(doc, domains=dict(doc['domains'], core=core))


def check_document(doc):
    """Check a core 1.0 document; return it — a core of an earlier schema of
    1.0 brought to this one —, or raise a PayloadError."""
    if not isinstance(doc, dict) or doc.get('format') != FORMAT:
        raise PayloadError('invalid_payload', ['/format: is not rail-across'])
    shape = _SCHEMAS.check('rail-across', doc)
    if shape:
        raise PayloadError('invalid_payload', _breaches(shape, ''))
    for h, schema in doc.get('schemas', {}).items():
        if content_hash(schema) != h:
            raise PayloadError('invalid_payload', [f'/schemas/{h}: holds another schema'])
    for h, value in doc['blobs'].items():
        if content_hash(value) != h:
            raise PayloadError('invalid_payload', [f'/blobs/{h}: holds other content'])
    doc = _migrated(doc)
    found = []
    for name, domain in doc['domains'].items():
        if DOMAIN_VERSIONS.get(name) != domain['version']:
            raise PayloadError('unsupported_version', [f"/domains/{name}: {domain['version']}"])
        if domain['schema'] != SCHEMA_HASHES[name]:
            found.append(f"/domains/{name}/schema: is not the schema of {name} {domain['version']}")
            continue
        deps = domain.get('dependsOn', [])
        if name != 'core' and deps != ['core']:
            found.append(f'/domains/{name}/dependsOn: is not ["core"]')
        part = {'fields': domain['fields'], 'collections': domain['collections']}
        found.extend(_breaches(_SCHEMAS.check(name, part), f'/domains/{name}'))
        for cname, objects in domain['collections'].items():
            seen = set()
            for obj in objects:
                if obj['id'] in seen:
                    found.append(f"/domains/{name}/collections/{cname}: holds {obj['id']} twice")
                seen.add(obj['id'])
    if found:
        raise PayloadError('invalid_payload', found)
    return doc


def manifest_of(doc):
    """Per domain its version, its schema and the hash of its fields and of each collection."""
    domains = {}
    for name in sorted(doc['domains']):
        d = doc['domains'][name]
        entry = {'version': d['version'], 'schema': d['schema']}
        if 'dependsOn' in d:
            entry['dependsOn'] = d['dependsOn']
        entry['fields'] = content_hash(d['fields'])
        entry['collections'] = {c: content_hash(d['collections'][c]) for c in sorted(d['collections'])}
        domains[name] = entry
    return {'domains': domains}


def document_hash(doc):
    """The hash of a document as a whole: its manifest's."""
    return content_hash(manifest_of(doc))
