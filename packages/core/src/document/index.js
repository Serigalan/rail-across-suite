// document — a project as the Rail-Data-Core keeps it: the core 1.0 document
// with its domains `core` and `layout`, their JSON Schemas (schemas/), the one
// door that checks them, the conversion from and to the record the
// Layout-Tool works on (the first step of the migration chain, from
// oltjson-1.1), canonical JSON and content hashes.
//
// This file is the module's public face: code outside packages/core/src/document/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/document.md.

export { canonicalJson, contentHash, derivedId } from './canonical.js'
export { DOMAIN_VERSIONS, defaultStages } from './coreFormat.js'
export { documentHash, manifestOf } from './manifest.js'
export { checkDocument, FORMAT, fromDocument, SCHEMA_HASHES, schemaOf, toDocument } from './door.js'
export { PayloadError } from './payloadError.js'
export { KEYWORDS, SchemaSet, unknownKeywords } from './schemaCheck.js'
export { sha256Hex, sha256OfText } from './sha256.js'
