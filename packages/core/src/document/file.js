import { viaNative } from './viaNative.js'

// The file rail-across-1.0 (`.railacross.json`; decision 0029, phase 5): one
// project in one file — its infrastructure (the part infra-data-1.0) and its
// operation (the part operations-1.0), the blobs of both together, the
// envelope naming the project, the program that wrote it and when; no schemas
// carried. The parts go in and come out each through its door (the core's
// module, crates/doc, file).

/** The file format's name and version, as the envelope states them. */
export const FILE_FORMAT = 'rail-across'
export const FILE_VERSION = '1.0'

/** Whether a parsed file is one of rail-across-1.0 — its parts beside its envelope. */
export const isProjectFile = (data) => data?.format === FILE_FORMAT && (data.infraData != null || data.operations != null)

/**
 * The file of a project: `infra` an infra-data document of it (projectDocument),
 * `operations` an operations document (toOperations) — either may be missing —,
 * `meta` `{ projectId?, generator?: { name, version }, savedAt? }`. Throws a
 * PayloadError where a part is not as its door takes it.
 */
export const projectFile = ({ infra = null, operations = null }, meta = {}) => viaNative('file.toFile', { infra, operations, meta })

/**
 * The parts of a file, each checked by its door: `{ projectId, generator?,
 * savedAt?, infra?, operations? }` — `infra` an infra-data document,
 * `operations` an operations document. Throws a PayloadError: `unsupported_version`
 * for another version, `invalid_payload` for one that is not as it says.
 */
export const fileParts = (data) => viaNative('file.fromFile', data)
