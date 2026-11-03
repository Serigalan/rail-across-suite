import { optimizeErrorText } from '@rail-across/core/commands'

/** What a run the optimizer module could not answer says: the module missing, or a failure. */
export function moduleErrorText(t, err) {
  if (err?.code === 'optimizer_missing') return t('optimizer_missing')
  return optimizeErrorText(t, err?.code === 'invalid_request' ? 'invalid_payload' : 'internal')
}
