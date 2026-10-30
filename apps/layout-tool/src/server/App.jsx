import { Suspense, lazy, useEffect, useRef, useState } from 'react'
import '../core/App.css'
import '../core/components/collab/collab.css'
import I18nProvider from '../core/locales/I18nProvider'
import { useI18n } from '../core/locales/i18nContext'
import { useProject } from '../core/hooks/useStore'
import { loadSettings } from '../core/utils/common'
import useSession from './useSession'
import useWorkingCopy from './useWorkingCopy'
import useViewer from './useViewer'
import { PALETTE } from '../core/styles/palette'
import { api } from './api/client'
import { errorText } from './collab/errorText'
import { projectsUrl, signInAgain, suiteBeside, toProjects, toSuite } from './suite'

// What the sign-in does not need is loaded when it is first opened (R9.1) —
// the map above all, with MapLibre and every panel behind it.
const HistoryPage = lazy(() => import('./collab/HistoryPage'))
const MapWorkspace = lazy(() => import('../core/shell/MapWorkspace'))
const ViewerView = lazy(() => import('./ViewerView'))
const WorkingCopyLayer = lazy(() => import('./WorkingCopyLayer'))
const LocalStorageDialog = lazy(() => import('../core/components/home/LocalStorageDialog'))

const ACTIONS = ['open', 'view', 'history', 'compare', 'merge', 'storage']

/**
 * What the suite asks of this tool, by the address (apps/README.md):
 * `?project=<id>&variant=<id>` a variant to work on — the main one without
 * `variant` —, with `&action=view` its head read-only, `&action=history` its
 * revisions; `?project=<id>&action=compare|merge&from=<a>&to=<b>` two
 * variants compared, or a's changes merged into b; `?action=storage` what
 * the tool keeps in this browser. Null without a project or an action.
 */
function linkedRequest() {
  const q = new URLSearchParams(location.search)
  const action = q.get('action') ?? 'open'
  if (!ACTIONS.includes(action)) return null
  if (action !== 'storage' && !q.get('project')) return null
  return { action, project: q.get('project'), variant: q.get('variant'), from: q.get('from'), to: q.get('to') }
}

/** While a page's chunk is on its way. */
function Loading() {
  const { t } = useI18n()
  return <div className="collab-page collab-center"><p className="collab-muted">{t('home_loading')}</p></div>
}

export default function App() {
  return <I18nProvider><Suspense fallback={<Loading />}><Shell /></Suspense></I18nProvider>
}

/**
 * Which page is up (R2.5): the open working copy on the map, a variant's
 * history, a record looked at read-only, or what the tool keeps in this
 * browser — whichever the suite asked for (linkedRequest). The tool has no
 * start page of its own: the projects are the suite's, and every way back
 * leads there (toProjects), with a note where there is one to say. The
 * account is the suite's too: the server lets nobody in without a session,
 * and where one ends meanwhile the page is loaded again for the sign-in; a
 * password to change is on the suite's start page.
 */
function Shell() {
  const { t, fill } = useI18n()
  const session = useSession()
  const project = useProject()
  // 'pending' | 'map' | 'viewer' | 'history' | 'storage' | 'alone'
  const [page, setPage] = useState('pending')
  const [storageProjects, setStorageProjects] = useState(null)
  const wc = useWorkingCopy({ t })
  // The start page is the suite's: going there leaves the tool, and what is to be said goes with it.
  const note = useRef(null)
  const go = (p) => (p === 'start' ? toProjects(note.current) : setPage(p))
  const viewer = useViewer({ t, fill, go, say: (text) => { note.current = text } })
  const [linked] = useState(linkedRequest)

  useEffect(() => {
    document.documentElement.style.setProperty('--color-primary', loadSettings().color ?? PALETTE.primaryDefault)
  }, [])

  const ready = session.status === 'user' && !session.user.mustChangePassword
  const account = session.status === 'anon' ? 'sign-in' : session.user?.mustChangePassword ? 'password' : null
  const [stuck, setStuck] = useState(false)
  useEffect(() => {
    if (!account) return
    if (account === 'sign-in' ? !signInAgain() : !suiteBeside()) setStuck(true)
    else if (account === 'password') toSuite('password')
  }, [account])

  // Once someone is signed in: what the address asks for. The address loses
  // the query, so a reload does not do it again but goes back to the projects.
  useEffect(() => {
    if (!ready) return
    if (!linked) {
      if (suiteBeside()) toProjects(null, { replace: true })
      else setPage('alone')
      return
    }
    history.replaceState(null, '', location.pathname + location.hash)
    const missing = () => Object.assign(new Error('missing'), { missing: true })
    api.projects().then(async ({ projects }) => {
      if (linked.action === 'storage') {
        setStorageProjects(projects)
        setPage('storage')
        return
      }
      const serverProject = projects.find(p => p.id === linked.project)
      const variants = (serverProject?.variants ?? []).filter(v => !v.archived)
      const byId = (id) => variants.find(v => v.id === id)
      if (linked.action === 'compare' || linked.action === 'merge') {
        const [a, b] = [byId(linked.from), byId(linked.to)]
        if (!a || !b) throw missing()
        if (linked.action === 'compare') return viewer.showComparison(serverProject, a, b)
        const outcome = await viewer.startMerge(serverProject, a, b)
        if (outcome === 'up_to_date') toProjects(fill('merge_up_to_date', { source: a.name, target: b.name }))
        return
      }
      const variant = byId(linked.variant) ?? variants.find(v => !v.parentVariantId)
      if (!variant) throw missing()
      if (linked.action === 'view') return viewer.viewVariant(serverProject, variant)
      if (linked.action === 'history') return viewer.showHistory(serverProject, variant)
      await wc.open(serverProject, variant)
      setPage('map')
    }).catch(err => toProjects(err.missing ? t('home_link_missing') : errorText(t, err.code ?? 'generic')))
  // viewer and wc are new on every render; the address is followed once, when ready.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, linked])

  if (session.status === 'loading') return <Loading />
  if (account) {
    return (
      <div className="collab-page collab-center">
        <p className="collab-muted">{stuck ? t('suite_sign_in_elsewhere') : t('home_loading')}</p>
      </div>
    )
  }
  if (page === 'alone') {
    return (
      <div className="collab-page collab-center">
        <p className="collab-muted">{t('suite_projects_elsewhere')} <a href={projectsUrl()}>{projectsUrl()}</a></p>
      </div>
    )
  }
  if (page === 'storage' && storageProjects) {
    return (
      <div className="collab-page">
        <LocalStorageDialog projects={storageProjects} onClose={() => toProjects()} />
      </div>
    )
  }
  if (page === 'history' && viewer.historyFor) {
    return <HistoryPage project={viewer.historyFor.project} variant={viewer.historyFor.variant}
      onBack={() => toProjects()} onView={viewer.viewRevision} onCompareWithHead={viewer.compareWithHead} />
  }
  if (page === 'viewer' && viewer.viewer) {
    return <ViewerView viewer={viewer} />
  }
  if (page === 'map' && project) {
    return (
      <MapWorkspace onHome={() => toProjects()} beforeHome={wc.close} homeKey="tooltip_infrastructure" resetKey={wc.replaced}
        renderExtras={(map) => <WorkingCopyLayer wc={wc} {...map} />} />
    )
  }
  return <Loading />
}
