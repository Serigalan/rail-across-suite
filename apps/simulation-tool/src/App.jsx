import { useEffect, useMemo, useState } from 'react'
import { Message, keepLanguage, keptLanguage } from '@rail-across/ui'
import { getWorkingCopy } from '@rail-across/working-copy'
import { I18n, textsFor, useTexts } from './i18n.js'
import { api } from './api.js'
import Simulator from './Simulator.jsx'
import { linkedRequest, projectsUrl, signInAgain, toProjects } from './suite.js'

export default function App() {
  const [language, setLanguage] = useState(() => keptLanguage())
  const texts = useMemo(() => ({ ...textsFor(language), setLanguage: (l) => { keepLanguage(l); setLanguage(l) } }), [language])
  useEffect(() => { document.documentElement.lang = language }, [language])
  return <I18n.Provider value={texts}><Loader /></I18n.Provider>
}

/**
 * The variant asked for, loaded: the session first (without one the page is
 * loaded again, and the suite's sign-in answers in its place), the project
 * and its variant, the variant's head — and, to simulate what is being
 * planned, this browser's working copy of it where there is one (what the
 * Layout-Tool or the Signalling-Tool changed and did not check in yet). It is
 * only read.
 */
function Loader() {
  const { t } = useTexts()
  const [linked] = useState(() => linkedRequest())
  const [state, setState] = useState(() => ({ status: linked ? 'loading' : 'none' }))

  useEffect(() => {
    if (!linked) return undefined
    let gone = false
    ;(async () => {
      const user = await api.me()
      if (!user) { if (!signInAgain()) setState({ status: 'failed', error: 'unauthorized' }); return }
      const projects = await api.projects()
      const project = projects.find(p => p.id === linked.project)
      if (!project) { toProjects(t('no_project')); return }
      const variant = project.variants?.find(v => v.id === linked.variant) ?? project.variants?.find(v => !v.parentVariantId) ?? project.variants?.[0]
      const head = await api.head(variant.id)
      const copy = linked.action === 'open' ? await getWorkingCopy(variant.id).catch(() => null) : null
      const local = Boolean(copy?.project && copy.base)
      if (!gone) setState({ status: 'ready', project, variant, revision: local ? copy.base : head.revision, record: local ? copy.project : head.record, local })
    })().catch(err => { if (!gone) setState({ status: 'failed', error: err.code ?? String(err) }) })
    return () => { gone = true }
  }, [linked, t])

  if (state.status === 'ready') return <Simulator project={state.project} variant={state.variant} revision={state.revision} record={state.record} local={state.local} />
  return (
    <div className="ui-page tool-center">
      {state.status === 'loading' && <p className="ui-muted">{t('loading')}</p>}
      {state.status === 'none' && <Message>{t('no_project')}</Message>}
      {state.status === 'failed' && <Message tone="error">{t('load_failed', { error: state.error })}</Message>}
      {state.status !== 'loading' && <a className="ui-btn" href={projectsUrl()}>{t('to_projects')}</a>}
    </div>
  )
}
