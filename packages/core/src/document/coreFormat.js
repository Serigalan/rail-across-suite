// What of the infra-data document the page reads as a value: the domains this
// build reads and writes. Turning a record into a document and back is the
// core's module (crates/doc; decisions 0005 and 0006).

/** The domains this build reads and writes, with their versions (crates/doc, DOMAIN_VERSIONS). */
export const DOMAIN_VERSIONS = { core: '6.0', civil: '3.0', signalling: '4.0', electrification: '3.0' }
