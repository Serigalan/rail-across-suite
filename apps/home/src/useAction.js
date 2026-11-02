import { useState } from 'react'
import { useTexts } from './i18n'

/** What the server's error `code` means, in words: its own text where there is one, the generic one otherwise. */
export function errorText(t, code) {
  const key = `err_${code}`
  const text = t(key)
  return text === key ? t('err_generic') : text
}

/** Run an async action with busy and error state, for a dialog. */
export function useAction() {
  const { t } = useTexts()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(null)
  const run = async (fn) => {
    setBusy(true)
    setError(null)
    try { await fn() } catch (err) { setError(errorText(t, err.code)); setBusy(false) }
  }
  return { busy, error, run }
}
