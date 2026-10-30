"""Canonical JSON after RFC 8785 (JCS) and content hashes — the bytes the
JavaScript core writes (packages/core/src/document/canonical.js), so both
hash the same data alike (data model, S-R3).

Numbers as ECMAScript writes them (``100``, ``1e-7``, ``-0`` as ``0``),
strings escaped as JSON.stringify escapes them, object members sorted by
the UTF-16 code units of their names, no whitespace.
"""

import hashlib
import math

_ESCAPES = {'"': '\\"', '\\': '\\\\', '\b': '\\b', '\f': '\\f', '\n': '\\n', '\r': '\\r', '\t': '\\t'}


def _number(x):
    """A number as ECMAScript's Number::toString writes it."""
    if isinstance(x, int):
        if abs(x) < 2 ** 53:
            return str(x)
        x = float(x)
    if math.isnan(x) or math.isinf(x):
        raise ValueError(f'no JSON for {x}')
    if x == 0:
        return '0'
    text = repr(x)  # the shortest digits that read back as x, as ECMAScript finds them
    sign = ''
    if text[0] == '-':
        sign, text = '-', text[1:]
    mantissa, _, exp = text.partition('e')
    whole, _, frac = mantissa.partition('.')
    raw = whole + frac
    digits = raw.lstrip('0')
    # The value is 0.d1d2…dk × 10^n.
    n = len(whole) + (int(exp) if exp else 0) - (len(raw) - len(digits))
    digits = digits.rstrip('0')
    k = len(digits)
    if k <= n <= 21:
        return sign + digits + '0' * (n - k)
    if 0 < n <= 21:
        return sign + digits[:n] + '.' + digits[n:]
    if -6 < n <= 0:
        return sign + '0.' + '0' * -n + digits
    e = n - 1
    return sign + digits[0] + ('.' + digits[1:] if k > 1 else '') + 'e' + ('+' if e >= 0 else '-') + str(abs(e))


def _string(s):
    out = ['"']
    for ch in s:
        c = ord(ch)
        if ch in _ESCAPES:
            out.append(_ESCAPES[ch])
        elif c < 0x20 or 0xD800 <= c <= 0xDFFF:
            # Control characters, and a surrogate standing alone, as JSON.stringify writes them.
            out.append(f'\\u{c:04x}')
        else:
            out.append(ch)
    out.append('"')
    return ''.join(out)


def _utf16(name):
    return name.encode('utf-16-be', 'surrogatepass')


def canonical_json(value):
    """The canonical JSON text of a value read from JSON."""
    if value is None:
        return 'null'
    if value is True:
        return 'true'
    if value is False:
        return 'false'
    if isinstance(value, (int, float)):
        return _number(value)
    if isinstance(value, str):
        return _string(value)
    if isinstance(value, (list, tuple)):
        return '[' + ','.join(canonical_json(v) for v in value) + ']'
    if isinstance(value, dict):
        names = sorted(value, key=_utf16)
        return '{' + ','.join(_string(k) + ':' + canonical_json(value[k]) for k in names) + '}'
    raise TypeError(f'no JSON for a {type(value).__name__}')


def content_hash(value):
    """"sha256:" and the SHA-256 of the canonical JSON of a value, uncompressed."""
    data = canonical_json(value).encode('utf-8', 'surrogatepass')
    return 'sha256:' + hashlib.sha256(data).hexdigest()
