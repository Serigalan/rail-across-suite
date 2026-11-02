/**
 * A payload this build will not take, `code` being the key the UI translates
 * (invalid_payload, unsupported_version, stages_unsupported, …) and `details`
 * what was found, as "path: what" lines, where there is more to say.
 */
export class PayloadError extends Error {
  constructor(code, details = []) {
    super(details.length ? `${code}: ${details.slice(0, 5).join('; ')}` : code)
    this.name = 'PayloadError'
    this.code = code
    this.details = details
  }
}
