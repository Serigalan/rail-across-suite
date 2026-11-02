import { useEffect, useMemo, useState } from 'react'
import { Message, keepLanguage, keptLanguage } from '@rail-across/ui'
import { I18n, textsFor, useTexts } from './i18n.js'
import Editor from './Editor.jsx'
import { signInAgain, suiteUrl } from './suite.js'

export default function App() {
  const [language, setLanguage] = useState(() => keptLanguage())
  const texts = useMemo(() => ({ ...textsFor(language), setLanguage: (l) => { keepLanguage(l); setLanguage(l) } }), [language])
  useEffect(() => { document.documentElement.lang = language }, [language])
  return <I18n.Provider value={texts}><Loader /></I18n.Provider>
}

/** The session first — without one the page is loaded again, and the suite's sign-in answers in its place —, then the editor. */
function Loader() {
  const { t } = useTexts()
  const [state, setState] = useState({ status: 'loading' })
  useEffect(() => {
    let gone = false
    fetch('/api/me', { credentials: 'same-origin', headers: { accept: 'application/json' } })
      .then(async res => {
        if (gone) return
        if (res.status === 401) { if (!signInAgain()) setState({ status: 'failed', error: 'unauthorized' }); return }
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        setState({ status: 'ready' })
      })
      .catch(err => { if (!gone) setState({ status: 'failed', error: err.message === 'Failed to fetch' ? 'offline' : err.message }) })
    return () => { gone = true }
  }, [])
  if (state.status === 'ready') return <Editor />
  return (
    <div className="ui-page tool-center">
      {state.status === 'loading' && <p className="ui-muted">{t('loading')}</p>}
      {state.status === 'failed' && <>
        <Message tone="error">{t('load_failed', { error: state.error })}</Message>
        <a className="ui-btn" href={suiteUrl()}>{t('to_suite')}</a>
      </>}
    </div>
  )
}
