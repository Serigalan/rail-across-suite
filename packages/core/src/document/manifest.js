import { contentHash } from './canonical.js'

/**
 * The manifest of a document: per domain its version, its schema and the hash
 * of its fields and of each collection — the hash tree of the data model
 * (section 5.2). The unit of storage is the collection (S-R1): a revision that
 * leaves a collection as it was names the same hash for it.
 */
export function manifestOf(doc) {
  const domains = {}
  for (const name of Object.keys(doc.domains).sort()) {
    const d = doc.domains[name]
    domains[name] = {
      version: d.version,
      schema: d.schema,
      ...(d.dependsOn ? { dependsOn: d.dependsOn } : {}),
      fields: contentHash(d.fields),
      collections: Object.fromEntries(Object.keys(d.collections).sort().map(c => [c, contentHash(d.collections[c])])),
    }
  }
  return { domains }
}

/** The hash of a document as a whole: its manifest's. */
export const documentHash = (doc) => contentHash(manifestOf(doc))
