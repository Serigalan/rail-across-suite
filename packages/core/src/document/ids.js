import { derivedId } from './canonical.js'

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/

/** Whether `id` is a UUID in lower case, as every id of a document and the record is (decision 0029). */
export const isUuid = (id) => typeof id === 'string' && UUID.test(id)

/**
 * The UUID an id goes by (`ids::uuid_of` of the core's module): a UUID in
 * lower case as it is, one in capitals in lower case, any other id — one a
 * foreign file gave — the UUID drawn from it, the same on every device.
 */
export const uuidOf = (id) => (isUuid(id) ? id : isUuid(id.toLowerCase()) ? id.toLowerCase() : derivedId(id))
