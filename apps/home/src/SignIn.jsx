import { useState } from 'react'
import { api } from './api'
import { useTexts } from './i18n'

const KNOWN = new Set(['invalid_credentials', 'too_many_attempts'])

/** Signing in to the suite's project server — one sign-in for the start page and every tool beside it. */
export default function SignIn({ onSignedIn }) {
  const { t } = useTexts()
  const [login, setLogin] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(null)
  const submit = async (e) => {
    e.preventDefault()
    setBusy(true)
    setError(null)
    try {
      onSignedIn(await api.login(login.trim(), password))
    } catch (err) {
      setError(KNOWN.has(err.code) ? err.code : 'sign_in_failed')
      setBusy(false)
    }
  }
  return (
    <form className="ra-sign-in" onSubmit={submit}>
      <p className="collab-muted">{t('sign_in_hint')}</p>
      <label className="collab-field">
        <span>{t('login')}</span>
        <input autoFocus autoComplete="username" value={login} onChange={e => setLogin(e.target.value)} required />
      </label>
      <label className="collab-field">
        <span>{t('password')}</span>
        <input type="password" autoComplete="current-password" value={password} onChange={e => setPassword(e.target.value)} required />
      </label>
      {error && <p className="collab-error" role="alert">{t(error)}</p>}
      <button type="submit" className="collab-btn collab-btn-primary collab-btn-block" disabled={busy}>
        {t(busy ? 'signing_in' : 'sign_in')}
      </button>
    </form>
  )
}
