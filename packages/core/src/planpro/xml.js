/**
 * A small XML reader, enough for PlanPro: elements, attributes, text, the
 * five named entities and character references; declarations, comments,
 * processing instructions and doctype skipped, CDATA taken as text. No
 * namespaces are resolved — a name keeps its prefix (`xsi:type`), which is
 * how PlanPro is read anyway. Runs under plain Node and in the browser alike,
 * which DOMParser does not.
 *
 * An element is { name, attrs, children, text }: `text` is the text directly
 * inside it, trimmed; `children` its elements in document order.
 */

const ENTITIES = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'" }

const decode = (s) => (s.indexOf('&') < 0 ? s : s.replace(/&(#x[0-9a-fA-F]+|#[0-9]+|[a-z]+);/g, (m, e) => {
  if (e[0] === '#') return String.fromCodePoint(e[1] === 'x' ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10))
  return ENTITIES[e] ?? m
}))

const ATTR = /([^\s=/>]+)\s*=\s*("([^"]*)"|'([^']*)')/g

/** The document element of `text`. Throws an Error naming the offset where the text is no XML. */
export function parseXml(text) {
  const root = { name: '#document', attrs: {}, children: [], text: '' }
  const stack = [{ el: root, parts: [] }]
  const top = () => stack[stack.length - 1]
  let i = 0
  const n = text.length
  const fail = (what) => { throw new Error(`xml: ${what} at ${i}`) }
  while (i < n) {
    const lt = text.indexOf('<', i)
    if (lt < 0) { top().parts.push(text.slice(i)); break }
    if (lt > i) top().parts.push(text.slice(i, lt))
    i = lt
    if (text.startsWith('<!--', i)) {
      const end = text.indexOf('-->', i + 4)
      if (end < 0) fail('comment without end')
      i = end + 3
    } else if (text.startsWith('<![CDATA[', i)) {
      const end = text.indexOf(']]>', i + 9)
      if (end < 0) fail('CDATA without end')
      top().parts.push({ raw: text.slice(i + 9, end) })
      i = end + 3
    } else if (text[i + 1] === '?' || text[i + 1] === '!') {
      const end = text.indexOf('>', i)
      if (end < 0) fail('declaration without end')
      i = end + 1
    } else if (text[i + 1] === '/') {
      const end = text.indexOf('>', i)
      if (end < 0) fail('end tag without end')
      const name = text.slice(i + 2, end).trim()
      const frame = stack.pop()
      if (stack.length === 0 || frame.el.name !== name) fail(`</${name}> closes <${frame.el.name}>`)
      frame.el.text = textOf(frame.parts)
      i = end + 1
    } else {
      const end = text.indexOf('>', i)
      if (end < 0) fail('tag without end')
      const selfClosing = text[end - 1] === '/'
      const inner = text.slice(i + 1, selfClosing ? end - 1 : end)
      const space = inner.search(/\s/)
      const name = space < 0 ? inner : inner.slice(0, space)
      const attrs = {}
      if (space >= 0) {
        for (const m of inner.slice(space).matchAll(ATTR)) attrs[m[1]] = decode(m[3] ?? m[4])
      }
      const el = { name, attrs, children: [], text: '' }
      top().el.children.push(el)
      if (!selfClosing) stack.push({ el, parts: [] })
      i = end + 1
    }
  }
  if (stack.length > 1) throw new Error(`xml: <${top().el.name}> without end`)
  const doc = root.children[0]
  if (!doc) throw new Error('xml: no element')
  return doc
}

const textOf = (parts) => parts.map(p => (typeof p === 'string' ? decode(p) : p.raw)).join('').trim()

/** The first child element named `name`, or undefined. */
export const child = (el, name) => el?.children.find(c => c.name === name)

/** The child elements named `name`. */
export const childrenNamed = (el, name) => el?.children.filter(c => c.name === name) ?? []

/** The element at a path of child names (`'A/B/C'`), or undefined. */
export function at(el, path) {
  let cur = el
  for (const name of path.split('/')) {
    cur = child(cur, name)
    if (!cur) return undefined
  }
  return cur
}

/**
 * PlanPro's values sit in a `Wert` child: the text of `path`'s `Wert`, or
 * undefined where it is missing or nil.
 */
export function value(el, path) {
  const w = child(at(el, path), 'Wert')
  if (!w || w.attrs['xsi:nil'] === 'true') return undefined
  return w.text
}
