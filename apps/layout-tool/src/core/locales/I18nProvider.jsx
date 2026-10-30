import { useCallback, useMemo, useState } from 'react'
import { I18nContext, i18nValue } from './i18nContext'
import { translations } from './i18n'
import { keepLanguage, keptLanguage } from '@rail-across/ui'
import { loadSettings } from '../utils/common'

/**
 * Holds the interface language, starting from `initial` where it is one (a
 * link made in a language — not remembered unless chosen here), else from the
 * suite's, which every module and the start page share — the one this tool
 * kept for itself before standing in for it until one is chosen.
 */
export default function I18nProvider({ children, initial = null }) {
  const [language, setLanguageState] = useState(() => (translations[initial] ? initial : keptLanguage(loadSettings().language)))
  const setLanguage = useCallback((lang) => {
    setLanguageState(lang)
    keepLanguage(lang)
  }, [])
  const ctx = useMemo(() => i18nValue(language, setLanguage), [language, setLanguage])
  return <I18nContext.Provider value={ctx}>{children}</I18nContext.Provider>
}
