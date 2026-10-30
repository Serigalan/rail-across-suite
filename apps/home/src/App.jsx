import { useCallback, useEffect, useMemo, useState } from 'react'
import { api } from './api'
import { I18n, initialLanguage, keepLanguage, textsFor } from './i18n'
import { hashOf, routeOf } from './modules'
import Header from './Header'
import Home from './pages/Home'
import Infrastructure from './pages/Infrastructure'
import Planned from './pages/Planned'
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

/**
 * The start page of the suite. Pages by the address' hash: the overview, the
 * infrastructure editor with its tools and the projects, and a page for every
 * module still to come. Who is signed in is asked once (`undefined` while it
 * is asked, `null` signed out); the projects are loaded once somebody is.
 */
export default function App() {
  const [language, setLanguage] = useState(initialLanguage)
  const texts = useMemo(() => textsFor(language), [language])
  const route = useRoute()
  const [user, setUser] = useState(undefined)
  const [projects, setProjects] = useState({ list: null, error: false })

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

  const chooseLanguage = (l) => { keepLanguage(l); setLanguage(l) }
  const signOut = useCallback(async () => {
    await api.logout().catch(() => {})
    setUser(null)
    setProjects({ list: null, error: false })
  }, [])
  // Signing in happens where the projects are listed.
  const signIn = () => { location.hash = hashOf({ page: 'infrastructure', tool: route.tool }) }

  return (
    <I18n.Provider value={texts}>
      <div className="collab-page home">
        <Header user={user} onLanguage={chooseLanguage} onSignIn={signIn} onSignOut={signOut} />
        {route.page === '' && <Home />}
        {route.page === 'infrastructure' && (
          <Infrastructure toolId={route.tool} user={user} projects={projects} onSignedIn={setUser} />
        )}
        {route.page !== '' && route.page !== 'infrastructure' && <Planned route={route.page} />}
      </div>
    </I18n.Provider>
  )
}
