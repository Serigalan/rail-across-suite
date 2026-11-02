"""The Python core held to the shared vectors the JavaScript core writes
(schemas/vectors/, made by packages/core/tools/vectors.mjs).

    python -m unittest discover -s tests
"""

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from rail_across_core import (  # noqa: E402
    SCHEMA_HASHES, PayloadError, SchemaSet, canonical_json, check_document, content_hash, document_hash,
    manifest_of, unknown_keywords,
)
from rail_across_core.document import DOMAIN_SCHEMAS, ENVELOPE  # noqa: E402

VECTORS = Path(__file__).resolve().parent.parent / 'schemas' / 'vectors'


def vectors(name):
    with open(VECTORS / name, encoding='utf-8') as f:
        return json.load(f)


class Canonical(unittest.TestCase):
    def test_the_same_text_and_hash_as_the_javascript_core(self):
        for v in vectors('canonical.json'):
            value = json.loads(v['json'])
            with self.subTest(json=v['json']):
                self.assertEqual(canonical_json(value), v['canonical'])
                self.assertEqual(content_hash(value), v['hash'])

    def test_refuses_what_json_cannot_hold(self):
        with self.assertRaises(ValueError):
            canonical_json(float('nan'))


class SchemaCheck(unittest.TestCase):
    def test_understands_every_keyword_the_schemas_use(self):
        for s in [ENVELOPE, *DOMAIN_SCHEMAS.values()]:
            self.assertEqual(unknown_keywords(s), [], s['$id'])

    def test_refuses_a_schema_with_a_keyword_it_does_not_know(self):
        with self.assertRaises(ValueError):
            SchemaSet([{'$id': 'x', 'type': 'object', 'if': {'required': ['a']}}])

    def test_the_same_verdict_and_the_same_places_as_the_javascript_core(self):
        schemas = SchemaSet([ENVELOPE, *DOMAIN_SCHEMAS.values()])
        for v in vectors('schema.json'):
            errors = schemas.check(v['ref'], v['value'], limit=1000)
            with self.subTest(ref=v['ref'], value=v['value']):
                self.assertEqual(not errors, v['valid'])
                self.assertEqual(sorted({e['path'] for e in errors}), v['paths'])


class Documents(unittest.TestCase):
    def test_the_same_verdict_manifest_and_hash_as_the_javascript_core(self):
        for v in vectors('documents.json'):
            with self.subTest(name=v['name']):
                if v['valid']:
                    taken = check_document(v['document'])
                    self.assertEqual(manifest_of(taken), v['manifest'])
                    self.assertEqual(document_hash(taken), v['hash'])
                else:
                    with self.assertRaises(PayloadError) as caught:
                        check_document(v['document'])
                    self.assertEqual(caught.exception.code, v['code'])

    def test_names_the_schemas_by_the_hashes_the_javascript_core_names_them_by(self):
        sample = vectors('documents.json')[0]['document']
        for name, domain in sample['domains'].items():
            self.assertEqual(SCHEMA_HASHES[name], domain['schema'])


if __name__ == '__main__':
    unittest.main()
