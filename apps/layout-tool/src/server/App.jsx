import { Suspense, lazy, useEffect, useRef, useState } from 'react'
import '../core/App.css'
import I18nProvider from '../core/locales/I18nProvider'
import { useI18n } from '../core/locales/i18nContext'
import { useProject } from '../core/hooks/useStore'
import { loadSettings } from '../core/utils/common'
import StartPage from './home/StartPage'
import useSession from './useSession'
import useWorkingCopy from './useWorkingCopy'
import useViewer from './useViewer'
import { PALETTE } from '../core/styles/palette'
import { api } from './api/client'
import { errorText } from './collab/errorText'
import { signInAgain, suiteBeside, suiteUrl, toSuite } from './suite'

// What the sign-in and the start page do not need is loaded when it is first
// opened (R9.1) — the map above all, with MapLibre and every panel behind it.
const HistoryPage = lazy(() => import('./collab/HistoryPage'))
const MapWorkspace = lazy(() => import('../core/shell/MapWorkspace'))
const ViewerView = lazy(() => import('./ViewerView'))
const WorkingCopyLayer = lazy(() => import('./WorkingCopyLayer'))

/**
 * The variant the address names, `?project=<id>&variant=<id>` — how the
 * suite's start page opens a project in this tool. Null without one.
 */
function linkedVariant() {
  const q = new URLSearchParams(location.search)
  return q.get('project') ? { project: q.get('project'), variant: q.get('variant') } : null
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
 * Which page is up (R2.5): the start page with the projects, a variant's
 * history, a record looked at read-only, or the open working copy on the
 * map. The pages and their state live in shell/; this only routes between
 * them. The account is the suite's: the server lets nobody in without a
 * session, and where one ends meanwhile the page is loaded again for the
 * sign-in; a password to change, the users and signing out are on the
 * suite's start page.
 */
function Shell() {
  const { t, fill } = useI18n()
  const session = useSession()
  const project = useProject()
  // 'start' | 'map' | 'viewer' | 'history'
  const [page, setPage] = useState('start')
  const [homeNote, setHomeNote] = useState(null)
  const wc = useWorkingCopy({ t })
  const viewer = useViewer({ t, fill, go: setPage, say: setHomeNote })
  const [linked, setLinked] = useState(linkedVariant)

  useEffect(() => {
    document.documentElement.style.setProperty('--color-primary', loadSettings().color ?? PALETTE.primaryDefault)
  }, [])

  const openVariant = async (serverProject, variant) => {
    setHomeNote(null)
    await wc.open(serverProject, variant)
    setPage('map')
  }

  // Opened from a link that names a variant: once someone is signed in, that
  // variant on the map — the main one where the link names the project only.
  // The address loses the query, so a reload lands on the start page.
  const ready = session.status === 'user' && !session.user.mustChangePassword
  const leaving = useRef(false)
  const account = session.status === 'anon' ? 'sign-in' : session.user?.mustChangePassword ? 'password' : null
  const [stuck, setStuck] = useState(false)
  useEffect(() => {
    if (!account || leaving.current) return
    if (account === 'sign-in' ? !signInAgain() : !suiteBeside()) setStuck(true)
    else if (account === 'password') toSuite('password')
  }, [account])
  useEffect(() => {
    if (!ready || !linked) return
    setLinked(null)
    history.replaceState(null, '', location.pathname + location.hash)
    api.projects().then(({ projects }) => {
      const project = projects.find(p => p.id === linked.project)
      const variants = (project?.variants ?? []).filter(v => !v.archived)
      const variant = variants.find(v => v.id === linked.variant) ?? variants.find(v => !v.parentVariantId)
      if (!variant) throw Object.assign(new Error('missing'), { missing: true })
      return openVariant(project, variant)
    }).catch(err => setHomeNote(err.missing ? t('home_link_missing') : errorText(t, err.code ?? 'generic')))
  // openVariant is new on every render; the link is followed once, when ready.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, linked])

  if (session.status === 'loading') {
    return <div className="collab-page collab-center"><p className="collab-muted">{t('home_loading')}</p></div>
  }
  if (account) {
    return (
      <div className="collab-page collab-center">
        <p className="collab-muted">{stuck ? t('suite_sign_in_elsewhere') : t('home_loading')}</p>
      </div>
    )
  }
  if (page === 'history' && viewer.historyFor) {
    return <HistoryPage project={viewer.historyFor.project} variant={viewer.historyFor.variant}
      onBack={() => setPage('start')} onView={viewer.viewRevision} onCompareWithHead={viewer.compareWithHead} />
  }
  if (page === 'viewer' && viewer.viewer) {
    return <ViewerView viewer={viewer} />
  }
  if (page === 'map' && project) {
    return (
      <MapWorkspace onHome={() => setPage('start')} beforeHome={wc.close} resetKey={wc.replaced}
        renderExtras={(map) => <WorkingCopyLayer wc={wc} {...map} />} />
    )
  }

  return (
    <StartPage user={session.user} note={homeNote}
      onOpenVariant={openVariant} onViewVariant={viewer.viewVariant}
      onCompare={viewer.showComparison} onMerge={viewer.startMerge} onHistory={viewer.showHistory}
      adminUrl={suiteUrl('admin')}
      onSignOut={async () => {
        leaving.current = true
        await session.signOut()
        location.assign(suiteUrl())
      }} />
  )
}
