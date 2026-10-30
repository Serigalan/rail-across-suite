import { useEffect, useMemo, useRef, useState } from 'react'
import { blobUrl } from '../../api'
import { useTexts } from '../../i18n'
import { toolUrl } from '../../modules'
import { readOnlyVariant, rightsOf, variantTree } from '../../projects'

/** A date as the list shows it: day and month, the year where it is not this one. */
function dateText(iso, language) {
  if (!iso) return ''
  const d = new Date(iso)
  const sameYear = d.getFullYear() === new Date().getFullYear()
  return d.toLocaleDateString(language === 'de' ? 'de-DE' : 'en-GB', { day: 'numeric', month: 'short', ...(sameYear ? {} : { year: 'numeric' }) })
}

/** Open while chosen; closed by a click beside it or Escape. */
function useMenu() {
  const [open, setOpen] = useState(false)
  const root = useRef(null)
  useEffect(() => {
    if (!open) return undefined
    const close = (e) => { if (!root.current?.contains(e.target)) setOpen(false) }
    const escape = (e) => { if (e.key === 'Escape') setOpen(false) }
    document.addEventListener('pointerdown', close)
    document.addEventListener('keydown', escape)
    return () => { document.removeEventListener('pointerdown', close); document.removeEventListener('keydown', escape) }
  }, [open])
  return { open, rootRef: root, toggle: () => setOpen(o => !o), close: () => setOpen(false) }
}

/** A way into the tool: a link where the tool is there, a button that cannot be pressed where it is not. */
function ToolLink({ href, className, title, children }) {
  return href
    ? <a className={className} href={href} title={title}>{children}</a>
    : <button type="button" className={className} disabled title={title}>{children}</button>
}

/**
 * One project: its picture, title, description and who works on it, its
 * menu (title and description, export, editors, delete), and its variants as
 * a tree — each indented under the variant it was branched off, with its
 * head (revision, author, date). What the suite does with a project happens
 * here; what a tool does with a variant — working on it, looking at it, its
 * history, comparing and merging two — opens in the tool chosen above.
 */
export default function ProjectCard({ project, user, tool, showArchived, onDialog }) {
  const { t, fill, language } = useTexts()
  const rights = rightsOf(project, user)
  const rows = useMemo(() => variantTree(project.variants.filter(v => showArchived || !v.archived)), [project, showArchived])
  const { open: menuOpen, rootRef: menuRootRef, toggle: toggleMenu, close: closeMenu } = useMenu()
  const picture = blobUrl(project.imageHash)
  const byId = new Map(project.variants.map(v => [v.id, v]))
  const many = project.variants.filter(v => !v.archived).length > 1
  const toolName = t(`tool_${tool.id}`)
  const at = (params) => toolUrl(tool, { project: project.id, ...params })
  const item = (label, kind, danger = false) => (
    <button type="button" role="menuitem" className={danger ? 'danger' : ''}
      onClick={() => { closeMenu(); onDialog({ kind, project }) }}>{label}</button>
  )

  return (
    <li className="home-project">
      <div className="ra-project">
        {picture ? <img className="ra-project-image" src={picture} alt="" /> : <span className="ra-project-image" />}
        <div className="ra-project-body">
          <h3>{project.title}</h3>
          {project.description && <p className="collab-muted">{project.description}</p>}
          {project.members?.length > 0 && (
            <p className="collab-muted">{fill('members_line', { names: project.members.map(m => m.name).join(', ') })}</p>
          )}
        </div>
        <div className="home-menu" ref={menuRootRef}>
          <button type="button" className="collab-btn collab-btn-small" aria-label={t('project_menu')} title={t('project_menu')}
            aria-haspopup="menu" aria-expanded={menuOpen} onClick={toggleMenu}>⋯</button>
          {menuOpen && (
            <div className="home-menu-list" role="menu">
              {rights.canEdit && item(t('project_rename'), 'rename')}
              {item(t('project_export'), 'export')}
              {!project.template && item(t('members'), 'members')}
              {rights.canDelete && item(t('project_delete'), 'delete', true)}
            </div>
          )}
        </div>
      </div>

      <ul className="ra-variant-tree">
        {rows.map(({ variant: v, depth }) => {
          const parent = v.parentVariantId ? byId.get(v.parentVariantId) : null
          const readOnly = readOnlyVariant(v, rights)
          return (
            <li key={v.id} className={`ra-variant-row${v.archived ? ' ra-variant-archived' : ''}`} style={{ '--depth': depth }}>
              <div className="ra-variant-main">
                <span className="ra-variant-name">{depth > 0 && <span className="ra-branch">└</span>}{v.name}</span>
                {v.head && (
                  <span className="ra-variant-meta">
                    {fill('revision', { n: v.head.number })} · {v.head.author?.name} · {dateText(v.head.createdAt, language)}
                  </span>
                )}
                <span className="ra-variant-buttons">
                  <ToolLink className="collab-btn collab-btn-small" href={at({ variant: v.id, action: 'history' })}
                    title={fill('in_tool', { action: t('history'), tool: toolName })}>{t('history')}</ToolLink>
                  {!readOnly && (
                    <button type="button" className="collab-btn collab-btn-small" title={t('variant_edit')} aria-label={t('variant_edit')}
                      onClick={() => onDialog({ kind: 'variant', project, variant: v })}>✎</button>
                  )}
                  {readOnly
                    ? <ToolLink className="collab-btn collab-btn-primary" href={at({ variant: v.id, action: 'view' })}
                        title={fill('in_tool', { action: t('view'), tool: toolName })}>{t('view')}</ToolLink>
                    : <ToolLink className="collab-btn collab-btn-primary" href={at({ variant: v.id })}
                        title={fill('in_tool', { action: t('open'), tool: toolName })}>{t('open')}</ToolLink>}
                </span>
              </div>
              {parent && v.parentAhead > 0 && (
                <div className="ra-variant-ahead">{fill('parent_ahead', { parent: parent.name, n: v.parentAhead })}</div>
              )}
            </li>
          )
        })}
      </ul>

      <div className="ra-project-actions">
        <button type="button" className="collab-btn" onClick={() => onDialog({ kind: 'branch', project })}>{t('branch')}</button>
        <button type="button" className="collab-btn" disabled={!many || !tool.available}
          onClick={() => onDialog({ kind: 'compare', project })}>{t('compare')}</button>
        <button type="button" className="collab-btn" disabled={!many || !tool.available}
          onClick={() => onDialog({ kind: 'merge', project })}>{t('merge')}</button>
      </div>
    </li>
  )
}
