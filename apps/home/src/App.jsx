import { useEffect, useMemo, useState } from 'react'
import { api } from './api'
import { I18n, initialLanguage, keepLanguage, textsFor } from './i18n'
import { routeOf } from './modules'
import Header from './Header'
import Home from './pages/Home'
import Infrastructure from './pages/Infrastructure'
import Planned from './pages/Planned'
import Admin from './pages/Admin'
import { PasswordPage } from './pages/Account'
import './home.css'

/** The page the address names, following it as it changes (back and forward included). */
function useRoute() {
  const [route, setRoute] = useState(() => routeOf(location.hash))
  useEffect(() => {
    const follow = () => setRoute(routeOf(location.hash))
    addEventListener('hashchange', follow)
    return () => removeEventListener('hashchange', follow)
  }, [])
  return route
}

/** On to a module's address after changing the password (`next`), or to the overview. */
function goOn(next) {
  if (next) location.assign(next)
  else location.hash = '#/'
}

/**
 * The start page of the suite. Nothing of it opens without a session: the
 * server shows the sign-in page (sign-in/) in its place until one is signed
 * in, so here somebody always is — and where the session ends meanwhile, the
 * page is loaded again and the server asks for the sign-in. Pages by the
 * address' hash: the overview, the infrastructure editor with its tools and
 * the projects, a page for every module still to come, the password, and for
 * admins the users of the whole suite.
 */
export default function App() {
  const [language, setLanguage] = useState(initialLanguage)
  const texts = useMemo(() => textsFor(language), [language])
  const route = useRoute()
  const [user, setUser] = useState(undefined)
  const [projects, setProjects] = useState({ list: null, error: false })
  const [notice, setNotice] = useState(null)

  useEffect(() => { document.documentElement.lang = language }, [language])
  useEffect(() => {
    api.me().then(u => (u ? setUser(u) : location.reload()), () => setUser(null))
  }, [])
  useEffect(() => {
    if (!user || user.mustChangePassword) return undefined
    let alive = true
    api.projects().then(
      list => { if (alive) setProjects({ list, error: false }) },
      () => { if (alive) setProjects({ list: null, error: true }) },
    )
    return () => { alive = false }
  }, [user])

  const chooseLanguage = (l) => { keepLanguage(l); setLanguage(l) }
  const passwordChanged = (u) => {
    setUser(u)
    setNotice('password_done')
    goOn(route.next)
  }
  // Signed out, the start page is the sign-in page again.
  const signOut = async () => {
    await api.logout().catch(() => {})
    location.assign('/')
  }
  const forced = Boolean(user?.mustChangePassword)
  const page = forced ? 'password' : route.page

  return (
    <I18n.Provider value={texts}>
      <div className="collab-page home">
        <Header user={user} onLanguage={chooseLanguage} onSignOut={signOut} />
        {notice && <p className="collab-ok ra-notice" role="status">{texts.t(notice)}</p>}
        {user === null && <p className="collab-error ra-notice" role="alert">{texts.t('server_unreachable')}</p>}
        {user && page === '' && <Home />}
        {user && page === 'infrastructure' && <Infrastructure toolId={route.tool} projects={projects} />}
        {user && page === 'password' && (
          <PasswordPage forced={forced} onDone={passwordChanged} onCancel={() => goOn(route.next)} />
        )}
        {user && page === 'admin' && (user.role === 'admin' ? <Admin me={user} /> : <Home />)}
        {user && !['', 'infrastructure', 'password', 'admin'].includes(page) && <Planned route={page} />}
      </div>
    </I18n.Provider>
  )
}
