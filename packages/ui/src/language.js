// The language the suite is set to: one choice for the start page and every
// module, kept in this browser under one key, so a language chosen in one
// tool is the language of the next one opened.

const KEY = 'rail-across-language'

/** The suite's languages, each named in itself — the same in every language, so no text to translate. */
export const LANGUAGE_NAMES = { de: 'Deutsch', en: 'English' }
export const LANGUAGES = Object.keys(LANGUAGE_NAMES)

/**
 * The language chosen last, else `before` (what a module kept for itself
 * before the suite had one choice), else the browser's (German where it asks
 * for German, English otherwise).
 */
export function keptLanguage(before = null) {
  try {
    const kept = globalThis.localStorage?.getItem(KEY)
    if (LANGUAGES.includes(kept)) return kept
  } catch { /* no storage: the browser's language */ }
  if (LANGUAGES.includes(before)) return before
  const asked = globalThis.navigator?.language ?? 'de'
  return asked.toLowerCase().startsWith('de') ? 'de' : 'en'
}

/** Keep the language for the whole suite; where the browser keeps nothing, it is chosen again next time. */
export function keepLanguage(language) {
  try { globalThis.localStorage?.setItem(KEY, language) } catch { /* not kept */ }
}
