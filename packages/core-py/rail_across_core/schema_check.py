"""The domain schemas checked as JSON Schema — the part of it they use, no
more, exactly as the JavaScript core checks them
(packages/core/src/document/schemaCheck.js), held to the same vectors
(schemas/vectors/schema.json). A schema that uses another keyword is refused
when it is registered rather than half understood.
"""

import re

#: The keywords understood; annotations (``x-…``) are ignored.
KEYWORDS = frozenset([
    '$schema', '$id', '$ref', '$defs', '$comment', 'title', 'description',
    'type', 'enum', 'const',
    'properties', 'required', 'additionalProperties', 'patternProperties', 'minProperties',
    'items', 'minItems', 'maxItems',
    'minimum', 'maximum', 'minLength', 'pattern',
    'oneOf', 'anyOf', 'allOf',
])


def unknown_keywords(schema, path=''):
    """Every keyword of a schema this check does not understand, as "path: keyword"."""
    out = []
    if not isinstance(schema, dict):
        return out
    for k, v in schema.items():
        if k not in KEYWORDS and not k.startswith('x-'):
            out.append(f'{path or "/"}: {k}')
        if k in ('properties', 'patternProperties', '$defs'):
            for name, sub in v.items():
                out.extend(unknown_keywords(sub, f'{path}/{k}/{name}'))
        elif k in ('items', 'additionalProperties'):
            out.extend(unknown_keywords(v, f'{path}/{k}'))
        elif k in ('oneOf', 'anyOf', 'allOf'):
            for i, sub in enumerate(v):
                out.extend(unknown_keywords(sub, f'{path}/{k}/{i}'))
    return out


def _is_number(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def _is_integer(v):
    # As ECMAScript's Number.isInteger: 25832.0 is an integer.
    return _is_number(v) and (isinstance(v, int) or float(v).is_integer())


_TYPES = {
    'null': lambda v: v is None,
    'boolean': lambda v: isinstance(v, bool),
    'number': _is_number,
    'integer': _is_integer,
    'string': lambda v: isinstance(v, str),
    'array': lambda v: isinstance(v, list),
    'object': lambda v: isinstance(v, dict),
}


def _same_json(a, b):
    """Equality of two JSON values: true is no 1, 1 is 1.0."""
    if isinstance(a, bool) or isinstance(b, bool):
        return isinstance(a, bool) and isinstance(b, bool) and a == b
    if _is_number(a) or _is_number(b):
        return _is_number(a) and _is_number(b) and a == b
    if isinstance(a, list):
        return isinstance(b, list) and len(a) == len(b) and all(_same_json(x, y) for x, y in zip(a, b))
    if isinstance(a, dict):
        return isinstance(b, dict) and a.keys() == b.keys() and all(_same_json(a[k], b[k]) for k in a)
    return type(a) is type(b) and a == b


def _json(v):
    import json
    return json.dumps(v, ensure_ascii=False, separators=(',', ':'))


class SchemaSet:
    """Schemas that refer to each other by ``$id`` ("core#/$defs/Track").

    ``check(ref, value)`` gives the breaches, each ``{'path', 'message'}``, at
    most ``limit`` of them; an empty list is a value the schema takes.
    """

    def __init__(self, schemas=()):
        self.by_id = {}
        self._patterns = {}
        for s in schemas:
            self.add(s)

    def add(self, schema):
        if not isinstance(schema, dict) or not isinstance(schema.get('$id'), str):
            raise ValueError('a schema needs its $id')
        unknown = unknown_keywords(schema)
        if unknown:
            raise ValueError(f"schema {schema['$id']} uses what this check does not know: {', '.join(unknown)}")
        self.by_id[schema['$id']] = schema

    def check(self, ref, value, limit=20):
        if '#' in ref:
            root, node = self._resolve(None, ref)
        else:
            root = node = self.by_id.get(ref)
        if root is None:
            raise KeyError(f'no schema {ref}')
        errors = []
        self._walk(root, node, value, '', errors, limit)
        return errors

    def _resolve(self, root, ref):
        hash_at = ref.find('#')
        target = self.by_id.get(ref[:hash_at]) if hash_at > 0 else root
        if target is None:
            raise KeyError(f'no schema {ref[:hash_at]}')
        node = target
        for part in [p for p in ref[hash_at + 1:].split('/') if p]:
            node = node.get(part) if isinstance(node, dict) else None
        if not isinstance(node, dict):
            raise KeyError(f'cannot resolve {ref}')
        return target, node

    def _regex(self, pattern):
        re_ = self._patterns.get(pattern)
        if re_ is None:
            # ECMAScript's $ ends the text; Python's would also match before a final newline.
            py = pattern[:-1] + r'\Z' if pattern.endswith('$') and not pattern.endswith('\\$') else pattern
            re_ = self._patterns[pattern] = re.compile(py)
        return re_

    def _passes(self, root, schema, value, path):
        errors = []
        self._walk(root, schema, value, path, errors, 1)
        return not errors

    def _walk(self, root, schema, value, path, errors, limit):
        if len(errors) >= limit:
            return

        def fail(message):
            if len(errors) < limit:
                errors.append({'path': path or '/', 'message': message})

        if '$ref' in schema:
            r, node = self._resolve(root, schema['$ref'])
            self._walk(r, node, value, path, errors, limit)
        if 'type' in schema:
            types = schema['type'] if isinstance(schema['type'], list) else [schema['type']]
            if not any(_TYPES[t](value) for t in types):
                fail(f"is not {' or '.join(types)}")
                return
        if 'enum' in schema and not any(_same_json(e, value) for e in schema['enum']):
            fail(f"is not one of {_json(schema['enum'])}")
        if 'const' in schema and not _same_json(schema['const'], value):
            fail(f"is not {_json(schema['const'])}")

        if _is_number(value):
            if 'minimum' in schema and value < schema['minimum']:
                fail(f"is less than {schema['minimum']}")
            if 'maximum' in schema and value > schema['maximum']:
                fail(f"is more than {schema['maximum']}")
        if isinstance(value, str):
            if 'minLength' in schema and len(value) < schema['minLength']:
                fail(f"is shorter than {schema['minLength']}")
            if 'pattern' in schema and not self._regex(schema['pattern']).search(value):
                fail(f"does not match {schema['pattern']}")
        if isinstance(value, list):
            if 'minItems' in schema and len(value) < schema['minItems']:
                fail(f"has fewer than {schema['minItems']} items")
            if 'maxItems' in schema and len(value) > schema['maxItems']:
                fail(f"has more than {schema['maxItems']} items")
            if 'items' in schema:
                for i, v in enumerate(value):
                    self._walk(root, schema['items'], v, f'{path}/{i}', errors, limit)
        if isinstance(value, dict):
            for k in schema.get('required', []):
                if k not in value:
                    fail(f'lacks {k}')
            if 'minProperties' in schema and len(value) < schema['minProperties']:
                fail(f"has fewer than {schema['minProperties']} members")
            props = schema.get('properties', {})
            patterns = list(schema.get('patternProperties', {}).items())
            for k in value:
                at = f'{path}/{k}'
                known = False
                if k in props:
                    known = True
                    self._walk(root, props[k], value[k], at, errors, limit)
                for p, sub in patterns:
                    if self._regex(p).search(k):
                        known = True
                        self._walk(root, sub, value[k], at, errors, limit)
                if not known and 'additionalProperties' in schema:
                    extra = schema['additionalProperties']
                    if extra is False:
                        fail(f'has {k}, which it may not')
                    elif isinstance(extra, dict):
                        self._walk(root, extra, value[k], at, errors, limit)

        for sub in schema.get('allOf', []):
            self._walk(root, sub, value, path, errors, limit)
        if 'anyOf' in schema and not any(self._passes(root, sub, value, path) for sub in schema['anyOf']):
            fail('matches none of anyOf')
        if 'oneOf' in schema:
            n = sum(1 for sub in schema['oneOf'] if self._passes(root, sub, value, path))
            if n != 1:
                # Where exactly one alternative has the type, its own breaches say more.
                def typed(sub):
                    if 'type' not in sub:
                        return True
                    ts = sub['type'] if isinstance(sub['type'], list) else [sub['type']]
                    return any(_TYPES[t](value) for t in ts)
                candidates = [sub for sub in schema['oneOf'] if typed(sub)]
                if n == 0 and len(candidates) == 1:
                    self._walk(root, candidates[0], value, path, errors, limit)
                else:
                    fail('matches none of oneOf' if n == 0 else 'matches more than one of oneOf')
