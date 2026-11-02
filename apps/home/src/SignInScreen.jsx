import { useEffect, useMemo, useState } from 'react'
import { I18n, LANGUAGES, initialLanguage, keepLanguage, textsFor, useTexts } from './i18n'
import { Logo } from './icons'
import SignIn from './SignIn'
import { PasswordPage } from './pages/Account'

/**
 * Once signed in, on to what was asked for: the server showed this page in
 * place of it, so the same address now gives it (Caddy, deploy/templates/
 * Caddyfile). Asked for the sign-in page itself, on to the start page.
 */
function proceed() {
  if (location.pathname.startsWith('/sign-in')) location.assign('/')
  else location.reload()
}

/**
 * The sign-in page of the suite. Nothing of Rail-Across-Suite opens without a
 * session: the server shows this page wherever one is asked for without one —
 * the start page, a module, a project's link. One sign-in for all of them; a
 * start password is changed here before anything opens.
 */
function SignInScreen({ onLanguage }) {
  const { t, language } = useTexts()
  const [user, setUser] = useState(null)
  return (
    <div className="ui-page ra-gate">
      <div className="ra-gate-language">
        {LANGUAGES.map(l => (
          <button key={l} type="button" className={`ui-link${l === language ? ' ra-current' : ''}`}
            onClick={() => onLanguage(l)}>
            {l.toUpperCase()}
          </button>
        ))}
      </div>
      <Logo className="ra-gate-logo" />
      {user?.mustChangePassword
        ? <PasswordPage forced onDone={proceed} />
        : (
          <main className="ra-account">
            <section className="ra-card">
              <h1 className="ra-gate-title">{t('sign_in')}</h1>
              <SignIn onSignedIn={(u) => { if (u.mustChangePassword) setUser(u); else proceed() }} />
            </section>
          </main>
        )}
    </div>
  )
}

/** The sign-in page in the language chosen before (the entry is signIn.jsx). */
export default function SignInPage() {
  const [language, setLanguage] = useState(initialLanguage)
  const texts = useMemo(() => textsFor(language), [language])
  useEffect(() => { document.documentElement.lang = language }, [language])
  const choose = (l) => { keepLanguage(l); setLanguage(l) }
  return <I18n.Provider value={texts}><SignInScreen onLanguage={choose} /></I18n.Provider>
}
