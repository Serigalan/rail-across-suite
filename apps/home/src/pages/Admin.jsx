import { useCallback, useEffect, useState } from 'react'
import { api } from '../api'
import { useTexts } from '../i18n'
import { sizeText, startPassword } from '../users'

const KNOWN_ERRORS = new Set(['last_admin', 'login_taken', 'login_invalid', 'password_too_short'])
const errorKey = (code) => `admin_err_${KNOWN_ERRORS.has(code) ? code : 'generic'}`

/**
 * The user administration of the suite, for admins only: who has an account,
 * in which role, with which rights of the modules, whether it is active and
 * when it was last used. One account for every module. Accounts are made here
 * with a start password the user replaces at the first sign-in; a password is
 * reset the same way. Nobody is deleted — a user who leaves is deactivated,
 * which ends their sessions, and their revisions keep their name. The last
 * active admin cannot give that up (the server refuses it).
 */
export default function Admin({ me }) {
  const { t, fill, language } = useTexts()
  const [users, setUsers] = useState(null)
  const [error, setError] = useState(null)
  const [notice, setNotice] = useState(null)   // { text, password? }
  const [form, setForm] = useState(null)       // { user? } — creating, or editing one

  const reload = useCallback(() => api.users().then(setUsers, err => setError(errorKey(err.code))), [])
  useEffect(() => { reload() }, [reload])

  const patch = async (u, body, done) => {
    setError(null)
    setNotice(null)
    try {
      await api.patchUser(u.id, body)
      if (done) setNotice(done)
      await reload()
    } catch (err) {
      setError(errorKey(err.code))
    }
  }
  const resetPassword = (u) => {
    const password = startPassword()
    patch(u, { password }, { text: fill('admin_reset_done', { login: u.login }), password })
  }
  const when = (iso) => new Date(iso).toLocaleString(language === 'de' ? 'de-DE' : 'en-GB', { dateStyle: 'medium', timeStyle: 'short' })

  return (
    <main className="home-main">
      <a className="collab-link ra-back" href="#/">← {t('back')}</a>
      <div className="home-section-head">
        <h2 className="ra-page-title">{t('admin_title')}</h2>
        <div className="home-section-actions">
          <button type="button" className="collab-btn collab-btn-primary" onClick={() => { setNotice(null); setForm({}) }}>{t('admin_create')}</button>
        </div>
      </div>
      <p className="collab-muted ra-admin-intro">{t('admin_intro')}</p>
      {error && <p className="collab-error" role="alert">{t(error)}</p>}
      {notice && (
        <div className="collab-ok ra-admin-notice" role="status">
          {notice.text}
          {notice.password && <> <code className="ra-password">{notice.password}</code> <span className="collab-muted">{t('admin_password_hint')}</span></>}
        </div>
      )}
      {form && (
        <UserForm user={form.user} onCancel={() => setForm(null)}
          onSubmit={async (body) => {
            if (form.user) {
              await api.patchUser(form.user.id, body)
            } else {
              await api.createUser(body)
              setNotice({ text: fill('admin_created', { login: body.login }), password: body.password })
            }
            setForm(null)
            await reload()
          }} />
      )}
      {users === null && !error && <p className="collab-muted">{t('projects_loading')}</p>}
      <ul className="ra-users">
        {(users ?? []).map(u => (
          <li key={u.id} className={`home-project ra-user${u.active ? '' : ' ra-user-off'}`}>
            <div className="ra-user-who">
              <strong>{u.name}</strong>
              <span className="collab-muted">{u.login}{u.id === me.id ? ` · ${t('admin_you')}` : ''}</span>
            </div>
            <div className="ra-user-facts">
              <span className={`ra-badge${u.role === 'admin' ? ' ra-badge-primary' : ''}`}>{t(`admin_role_${u.role}`)}</span>
              <span className={`ra-badge${u.active ? ' ra-badge-ok' : ' ra-badge-off'}`}>{t(u.active ? 'admin_active' : 'admin_inactive')}</span>
              {u.canEditClouds && u.role !== 'admin' && <span className="ra-badge" title={t('admin_clouds')}>{t('admin_badge_clouds')}</span>}
              {u.mustChangePassword && <span className="ra-badge ra-badge-warn">{t('admin_start_password')}</span>}
              <span className="collab-muted">{u.lastLoginAt ? fill('admin_last_login', { date: when(u.lastLoginAt) }) : t('admin_never')}</span>
            </div>
            <div className="ra-user-actions">
              <button type="button" className="collab-btn collab-btn-small" onClick={() => { setNotice(null); setForm({ user: u }) }}>{t('admin_edit')}</button>
              <button type="button" className="collab-btn collab-btn-small" onClick={() => resetPassword(u)}>{t('admin_reset')}</button>
              <button type="button" className={`collab-btn collab-btn-small${u.active ? ' collab-btn-danger' : ''}`}
                onClick={() => patch(u, { active: !u.active }, { text: fill(u.active ? 'admin_deactivated' : 'admin_activated', { login: u.login }) })}>
                {t(u.active ? 'admin_deactivate' : 'admin_activate')}
              </button>
            </div>
          </li>
        ))}
      </ul>
      <CloudDisk />
    </main>
  )
}

/** Create a user (login, name, role, rights, start password) or edit one (name, role, rights). */
function UserForm({ user = null, onCancel, onSubmit }) {
  const { t } = useTexts()
  const [login, setLogin] = useState(user?.login ?? '')
  const [name, setName] = useState(user?.name ?? '')
  const [role, setRole] = useState(user?.role ?? 'user')
  const [clouds, setClouds] = useState(user?.canEditClouds ?? false)
  const [password, setPassword] = useState(() => (user ? '' : startPassword()))
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(null)
  const submit = async (e) => {
    e.preventDefault()
    setBusy(true)
    setError(null)
    try {
      await onSubmit(user
        ? { name: name.trim(), role, canEditClouds: clouds }
        : { login: login.trim(), name: name.trim(), role, password, canEditClouds: clouds })
    } catch (err) {
      setError(errorKey(err.code))
      setBusy(false)
    }
  }
  return (
    <form className="home-project ra-user-form" onSubmit={submit}>
      <h3>{t(user ? 'admin_edit_title' : 'admin_create')}</h3>
      {!user && (
        <label className="collab-field">
          <span>{t('login')} *</span>
          <input value={login} onChange={e => setLogin(e.target.value)} autoFocus required autoComplete="off" />
        </label>
      )}
      <label className="collab-field">
        <span>{t('admin_name')}</span>
        <input value={name} onChange={e => setName(e.target.value)} autoFocus={Boolean(user)} />
      </label>
      <label className="collab-field">
        <span>{t('admin_role')}</span>
        <select value={role} onChange={e => setRole(e.target.value)}>
          <option value="user">{t('admin_role_user')}</option>
          <option value="admin">{t('admin_role_admin')}</option>
        </select>
      </label>
      <fieldset className="ra-rights">
        <legend>{t('admin_rights')}</legend>
        <label className="ra-check">
          <input type="checkbox" checked={role === 'admin' || clouds} disabled={role === 'admin'} onChange={e => setClouds(e.target.checked)} />
          <span><strong>{t('tool_layout')}:</strong> {t('admin_clouds')}</span>
        </label>
        <p className="collab-muted">{t('admin_clouds_hint')}</p>
      </fieldset>
      {!user && (
        <label className="collab-field">
          <span>{t('admin_start_password')}</span>
          <span className="ra-password-row">
            <input value={password} onChange={e => setPassword(e.target.value)} minLength={12} required autoComplete="off" />
            <button type="button" className="collab-btn collab-btn-small" onClick={() => setPassword(startPassword())}>{t('admin_generate')}</button>
          </span>
        </label>
      )}
      {error && <p className="collab-error" role="alert">{t(error)}</p>}
      <div className="ra-actions">
        <button type="button" className="collab-btn" onClick={onCancel} disabled={busy}>{t('cancel')}</button>
        <button type="submit" className="collab-btn collab-btn-primary" disabled={busy}>{t(user ? 'admin_save' : 'admin_create_submit')}</button>
      </div>
    </form>
  )
}

/**
 * The disk the point clouds lie on: how much is free — with a warning under
 * 20 GB — and what each project's clouds take of their quota.
 */
function CloudDisk() {
  const { t, fill } = useTexts()
  const [info, setInfo] = useState(null)
  useEffect(() => {
    let alive = true
    api.cloudAdmin().then(r => { if (alive) setInfo(r) }, () => { if (alive) setInfo(false) })
    return () => { alive = false }
  }, [])
  if (!info) return null
  return (
    <section className="ra-disk">
      <h3>{t('admin_disk_title')}</h3>
      <p className={info.disk.warn ? 'collab-error' : 'collab-muted'}>
        {fill('admin_disk_free', { free: sizeText(info.disk.free), total: sizeText(info.disk.total) })}
        {info.disk.warn && ` ${fill('admin_disk_warn', { limit: sizeText(info.disk.warnBelow) })}`}
      </p>
      {info.projects.length === 0
        ? <p className="collab-muted">{t('admin_disk_none')}</p>
        : (
          <ul className="ra-disk-list">
            {info.projects.map(p => (
              <li key={p.project_id}>
                <strong>{p.title}</strong>{' '}
                <span className="collab-muted">{fill('admin_disk_project', { n: p.clouds, size: sizeText(p.bytes + p.raw), quota: sizeText(info.quota) })}</span>
              </li>
            ))}
          </ul>
        )}
    </section>
  )
}
