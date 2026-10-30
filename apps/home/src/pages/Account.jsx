import { useState } from 'react'
import { api } from '../api'
import { useTexts } from '../i18n'
import SignIn from '../SignIn'

/** At least as long as the server asks (server/src/auth.js). */
const MIN = 12

/**
 * Signing in, for the whole suite: every module sends whoever is not signed
 * in here, and gets them back (`next`) once they are.
 */
export function SignInPage({ onSignedIn }) {
  const { t } = useTexts()
  return (
    <main className="home-main ra-account">
      <section className="ra-card">
        <h2>{t('sign_in')}</h2>
        <SignIn onSignedIn={onSignedIn} />
      </section>
    </main>
  )
}

/**
 * Changing one's own password — forced after an admin set a start password or
 * reset it, voluntarily from the user's menu. For the whole suite, as the
 * sign-in is.
 */
export function PasswordPage({ forced, onDone, onCancel }) {
  const { t } = useTexts()
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const [repeat, setRepeat] = useState('')
  const [error, setError] = useState(null)
  const [busy, setBusy] = useState(false)
  const submit = async (e) => {
    e.preventDefault()
    if ([...next].length < MIN) { setError('password_err_short'); return }
    if (next !== repeat) { setError('password_err_repeat'); return }
    setBusy(true)
    setError(null)
    try {
      await api.changePassword(current, next)
      onDone(await api.me())
    } catch (err) {
      setError({
        wrong_password: 'password_err_wrong', password_too_short: 'password_err_short', password_unchanged: 'password_err_unchanged',
      }[err.code] ?? 'password_err_generic')
      setBusy(false)
    }
  }
  return (
    <main className="home-main ra-account">
      <form className="ra-card ra-sign-in" onSubmit={submit}>
        <h2>{t('change_password')}</h2>
        {forced && <p className="collab-muted">{t('must_change_password')}</p>}
        <label className="collab-field">
          <span>{t('password_current')}</span>
          <input type="password" autoComplete="current-password" value={current} onChange={e => setCurrent(e.target.value)} required autoFocus />
        </label>
        <label className="collab-field">
          <span>{t('password_new')}</span>
          <input type="password" autoComplete="new-password" minLength={MIN} value={next} onChange={e => setNext(e.target.value)} required />
        </label>
        <label className="collab-field">
          <span>{t('password_repeat')}</span>
          <input type="password" autoComplete="new-password" value={repeat} onChange={e => setRepeat(e.target.value)} required />
        </label>
        <p className="collab-muted">{t('password_rule')}</p>
        {error && <p className="collab-error" role="alert">{t(error)}</p>}
        <div className="ra-actions">
          {!forced && <button type="button" className="collab-btn" onClick={onCancel} disabled={busy}>{t('cancel')}</button>}
          <button type="submit" className="collab-btn collab-btn-primary" disabled={busy}>{t('password_submit')}</button>
        </div>
      </form>
    </main>
  )
}
