import { useEffect, useMemo, useState } from 'react'
import { Message, keepLanguage, keptLanguage } from '@rail-across/ui'
import { I18n, textsFor, useTexts } from './i18n.js'
import { api } from './api.js'
import Editor from './Editor.jsx'
import { openCopy } from './workingCopy.js'
import { SUITE, linkedRequest, projectsUrl, signInAgain, toProjects } from './suite.js'

export default function App() {
  const [language, setLanguage] = useState(() => keptLanguage())
  const texts = useMemo(() => ({ ...textsFor(language), setLanguage: (l) => { keepLanguage(l); setLanguage(l) } }), [language])
  useEffect(() => { document.documentElement.lang = language }, [language])
  return <I18n.Provider value={texts}><Loader /></I18n.Provider>
}

/**
 * The variant asked for, loaded: the session first (without one the page is
 * loaded again, and the suite's sign-in answers in its place), then the
 * project and its variant, then the variant's head — and to work on it, its
 * working copy in this browser where there is one (what the Layout-Tool or
 * this tool changed and did not check in yet).
 */
function Loader() {
  const { t } = useTexts()
  const [linked] = useState(() => linkedRequest())
  const [state, setState] = useState(() => ({ status: linked ? 'loading' : 'none' }))

  useEffect(() => {
    if (!linked) return undefined
    if (linked.forward) {
      location.replace(`${SUITE}layout/${location.search}`)
      return undefined
    }
    let gone = false
    ;(async () => {
      const user = await api.me()
      if (!user) { if (!signInAgain()) setState({ status: 'failed', error: 'unauthorized' }); return }
      const projects = await api.projects()
      const project = projects.find(p => p.id === linked.project)
      if (!project) { toProjects(t('no_project')); return }
      const variant = project.variants?.find(v => v.id === linked.variant) ?? project.variants?.find(v => !v.parentVariantId) ?? project.variants?.[0]
      const head = await api.head(variant.id)
      const copy = linked.action === 'open' ? await openCopy(variant.id, head) : null
      if (!gone) setState({ status: 'ready', user, project, variant, head, copy })
    })().catch(err => { if (!gone) setState({ status: 'failed', error: err.code ?? String(err) }) })
    return () => { gone = true }
  }, [linked, t])

  if (state.status === 'ready') {
    return <Editor project={state.project} variant={state.variant} head={state.head} copy={state.copy} readOnly={linked.action !== 'open'} />
  }
  return (
    <div className="ui-page tool-center">
      {state.status === 'loading' && <p className="ui-muted">{t('loading')}</p>}
      {state.status === 'none' && <Message>{t('no_project')}</Message>}
      {state.status === 'failed' && <Message tone="error">{t('load_failed', { error: state.error })}</Message>}
      {state.status !== 'loading' && <a className="ui-btn" href={projectsUrl()}>{t('to_projects')}</a>}
    </div>
  )
}
