// document — a project as the Rail-Data-Core keeps it: the infra-data-2.0
// document (decisions 0021, 0025) with its domain `core` and the layers `civil`,
// `signalling` and `electrification`,
// their JSON Schemas (schemas/), the one door that checks them, the conversion
// from and to the record the Layout-Tool works on, canonical JSON and content
// hashes. The door, the
// conversion and the manifest are the core's Rust module (crates/doc;
// decision 0020, step 10).
//
// This file is the module's public face: code outside packages/core/src/document/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/document.md.

export { canonicalJson, contentHash, derivedId } from './canonical.js'
export { DOMAIN_VERSIONS } from './coreFormat.js'
export { defaultStages, renamedStation, stationRegister } from './stations.js'
export { documentHash, manifestOf } from './manifest.js'
export {
  checkDocument, FORMAT, FORMAT_NAME, FORMAT_VERSION, fromDocument, SCHEMA_HASHES, schemaOf, toDocument,
} from './door.js'
export { PayloadError } from './payloadError.js'
export { sha256Hex, sha256OfText } from './sha256.js'
export { viaNative } from './viaNative.js'
