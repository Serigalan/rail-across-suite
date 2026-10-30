import { useCallback, useEffect, useMemo, useState } from 'react'
import { api } from './api'
import { I18n, initialLanguage, keepLanguage, textsFor } from './i18n'
import { hashOf, routeOf } from './modules'
import Header from './Header'
import Home from './pages/Home'
import Infrastructure from './pages/Infrastructure'
import Planned from './pages/Planned'
import { PasswordPage, SignInPage } from './pages/Account'
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

/** On to a module's address after signing in (`next`), or to the overview. */
function goOn(next) {
  if (next) location.assign(next)
  else location.hash = '#/'
}

/**
 * The start page of the suite. Pages by the address' hash: the overview, the
 * infrastructure editor with its tools and the projects, a page for every
 * module still to come, and the account — signing in and the password, for
 * every module: they send whoever is not signed in here and get them back.
 * Who is signed in is asked once (`undefined` while it is asked, `null`
 * signed out); the projects are loaded once somebody is.
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
    api.me().then(setUser, () => setUser(null))
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

  // Signed in already, the sign-in page has nothing to do but send one on.
  useEffect(() => {
    if (user && !user.mustChangePassword && route.page === 'sign-in') goOn(route.next)
  }, [user, route])

  const chooseLanguage = (l) => { keepLanguage(l); setLanguage(l) }
  const signedIn = (u) => {
    setUser(u)
    if (u.mustChangePassword) location.hash = hashOf({ page: 'password', next: route.next })
    else if (route.page === 'sign-in') goOn(route.next)
  }
  const passwordChanged = (u) => {
    setUser(u)
    setNotice('password_done')
    goOn(route.next)
  }
  const signOut = useCallback(async () => {
    await api.logout().catch(() => {})
    setUser(null)
    setProjects({ list: null, error: false })
  }, [])
  const signIn = () => { location.hash = hashOf({ page: 'sign-in' }) }
  const forced = Boolean(user?.mustChangePassword)
  const page = forced ? 'password' : route.page

  return (
    <I18n.Provider value={texts}>
      <div className="collab-page home">
        <Header user={user} onLanguage={chooseLanguage} onSignIn={signIn} onSignOut={signOut} />
        {notice && <p className="collab-ok ra-notice" role="status">{texts.t(notice)}</p>}
        {page === '' && <Home />}
        {page === 'infrastructure' && (
          <Infrastructure toolId={route.tool} user={user} projects={projects} onSignedIn={signedIn} />
        )}
        {page === 'sign-in' && user === null && <SignInPage onSignedIn={signedIn} />}
        {page === 'password' && user === null && <SignInPage onSignedIn={signedIn} />}
        {page === 'password' && user && (
          <PasswordPage forced={forced} onDone={passwordChanged} onCancel={() => goOn(route.next)} />
        )}
        {!['', 'infrastructure', 'sign-in', 'password'].includes(page) && <Planned route={page} />}
      </div>
    </I18n.Provider>
  )
}
