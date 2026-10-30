import { useEffect, useRef, useState } from 'react'
import { Logo } from './icons'
import { LANGUAGES, useTexts } from './i18n'

/**
 * The bar across the top: the suite's mark on the left, the language and the
 * user's menu on the right — the password, for admins the users of the suite,
 * signing out. The account is the suite's: signed in here, signed in in every
 * module.
 */
export default function Header({ user, onLanguage, onSignOut }) {
  const { t, language } = useTexts()
  const [menu, setMenu] = useState(null)   // 'language' | 'user' | null
  const root = useRef(null)
  useEffect(() => {
    if (!menu) return undefined
    const close = (e) => { if (!root.current?.contains(e.target)) setMenu(null) }
    const escape = (e) => { if (e.key === 'Escape') setMenu(null) }
    document.addEventListener('pointerdown', close)
    document.addEventListener('keydown', escape)
    return () => { document.removeEventListener('pointerdown', close); document.removeEventListener('keydown', escape) }
  }, [menu])
  const toggle = (name) => setMenu(m => (m === name ? null : name))

  return (
    <header className="home-top">
      <a className="ra-brand" href="#/">
        <Logo className="ra-logo" />
        <h1 className="ui-visually-hidden">{t('suite')}</h1>
      </a>
      <div className="home-user" ref={root}>
        <div className="ui-menu">
          <button type="button" className="ui-btn" aria-haspopup="menu" aria-expanded={menu === 'language'}
            aria-label={t('language')} onClick={() => toggle('language')}>
            {language.toUpperCase()}
          </button>
          {menu === 'language' && (
            <div className="ui-menu-list home-language-list" role="menu">
              {LANGUAGES.map(l => (
                <button key={l} type="button" role="menuitem" className={l === language ? 'current' : ''}
                  onClick={() => { setMenu(null); onLanguage(l) }}>
                  <span className="home-language-check">{l === language ? '✓' : ''}</span>
                  {l === 'de' ? 'Deutsch' : 'English'}
                </button>
              ))}
            </div>
          )}
        </div>
        {user && (
          <div className="ui-menu">
            {/* The user's menu behind their symbol; the name is its label. */}
            <button type="button" className="ui-btn ui-btn-icon" aria-haspopup="menu" aria-expanded={menu === 'user'}
              aria-label={user.name} title={user.name} onClick={() => toggle('user')}>
              <UserIcon />
            </button>
            {menu === 'user' && (
              <div className="ui-menu-list" role="menu">
                <p className="ui-menu-user">{user.name}</p>
                <a role="menuitem" href="#/password" onClick={() => setMenu(null)}>{t('change_password')}</a>
                {user.role === 'admin' && <a role="menuitem" href="#/admin" onClick={() => setMenu(null)}>{t('admin_title')}</a>}
                <button type="button" role="menuitem" onClick={() => { setMenu(null); onSignOut() }}>{t('sign_out')}</button>
              </div>
            )}
          </div>
        )}
      </div>
    </header>
  )
}

const UserIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
    <circle cx="12" cy="8" r="4" />
    <path d="M4 21 c0 -4 4 -6 8 -6 s8 2 8 6" />
  </svg>
)
