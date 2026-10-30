import { useRef, useState } from 'react'
import { INFRASTRUCTURE_TOOLS, hashOf, toolOf, toolUrl } from '../modules'
import { ModuleIcon } from '../icons'
import { api } from '../api'
import { useTexts } from '../i18n'
import { errorText } from '../useAction'
import { importedProjects, splitProjects } from '../projects'
import { splitDataUrl } from '../projectImages'
import ProjectCard from './projects/ProjectCard'
import MembersDialog from './projects/MembersDialog'
import {
  BranchDialog, DeleteProjectDialog, ExportDialog, NewProjectDialog, PairDialog, RenameProjectDialog, VariantDialog,
} from './projects/dialogs'

/**
 * The tools as tabs: large, each with its icon. The chosen one stands out and
 * the others are greyed — the projects below open in the chosen tool. A tool
 * that is not there yet can be chosen all the same, and says so below.
 */
function ToolTabs({ current }) {
  const { t, fill } = useTexts()
  return (
    <div className="ra-tabs" role="tablist">
      {INFRASTRUCTURE_TOOLS.map(tool => {
        const chosen = tool.id === current
        return (
          <a key={tool.id} role="tab" aria-selected={chosen} href={hashOf({ page: 'infrastructure', tool: tool.id })}
            className={`ra-tab${chosen ? ' ra-tab-chosen' : ''}`}>
            <span className="ra-tile-icon"><ModuleIcon id={tool.id} /></span>
            <span className="ra-tile-name">{t(`tool_${tool.id}`)}</span>
            <span className="ra-tile-desc">{t(`tool_${tool.id}_desc`)}</span>
            {!tool.available && <span className="ra-badge">{fill('tool_soon', { n: tool.phase })}</span>}
          </a>
        )
      })}
    </div>
  )
}

/**
 * The infrastructure editor: its three tools, a line, and the projects of the
 * suite's server — managed here for every tool, opened in the one chosen
 * above. A project is made here, or imported from a backup file (the only
 * place whole projects come in), exported, renamed, given its editors and
 * deleted; its variants are branched, renamed and archived here. Working on
 * a variant, looking at it, its history, comparing and merging two are the
 * tool's: they open in it (toolUrl), and the tool sends one back here.
 *
 * Each user sees the projects they created or were added to, an admin sees
 * all (decision 127). Below them the templates: everyone branches a variant
 * of their own off one; the template itself only an admin edits, everyone
 * else looks at it read-only. Signed in one always is here: the server shows
 * the sign-in page before the suite.
 */
export default function Infrastructure({ toolId, user, projects, onReload = async () => {} }) {
  const { t, fill } = useTexts()
  const tool = toolOf(toolId)
  const toolName = t(`tool_${tool.id}`)
  const [dialog, setDialog] = useState(null)      // { kind, project?, variant?, template? }
  const [error, setError] = useState(null)
  const [notice, setNotice] = useState(null)
  const [showArchived, setShowArchived] = useState(false)
  const importRef = useRef(null)
  const { regular, templates } = splitProjects(projects.list)
  const isAdmin = user?.role === 'admin'
  const storage = toolUrl(tool, { action: 'storage' })

  const done = async () => { setDialog(null); await onReload() }
  const say = (text) => { setError(null); setNotice(text) }

  // A backup file: every project in it becomes a server project of its own,
  // its picture uploaded on the way.
  const handleImport = async (e) => {
    const file = e.target.files?.[0]
    e.target.value = ''
    if (!file) return
    say(null)
    let incoming
    try {
      incoming = importedProjects(await file.text(), file.name)
    } catch (err) {
      setError(errorText(t, err.code))
      return
    }
    try {
      for (const p of incoming) {
        const record = { ...p.record }
        const picture = splitDataUrl(p.image)
        if (picture) record.imageHash = (await api.uploadImage(picture.mime, picture.data)).hash
        await api.createProject({ title: p.title, description: p.description, payload: record })
      }
      say(fill('projects_imported', { n: incoming.length }))
    } catch (err) {
      setError(errorText(t, err.code))
    }
    await onReload()
  }

  const card = (p) => (
    <ProjectCard key={p.id} project={p} user={user} tool={tool} showArchived={showArchived}
      onDialog={(d) => { say(null); setDialog(d) }} />
  )

  return (
    <main className="home-main">
      <a className="collab-link ra-back" href="#/">← {t('back')}</a>
      <h2 className="ra-page-title">{t('module_infrastructure')}</h2>
      <ToolTabs current={tool.id} />
      <hr className="ra-rule" />

      <div className="home-section-head">
        <h2>{t('projects')}</h2>
        <div className="home-section-actions">
          <input ref={importRef} type="file" accept=".json,application/json" hidden onChange={handleImport} />
          <button type="button" className="collab-btn collab-btn-primary" onClick={() => { say(null); setDialog({ kind: 'new' }) }}>{t('project_new')}</button>
          <button type="button" className="collab-btn" onClick={() => importRef.current?.click()}>{t('project_import')}</button>
        </div>
      </div>
      <div className={`ra-mode${tool.available ? '' : ' ra-mode-off'}`}>
        <span className="ra-mode-icon"><ModuleIcon id={tool.id} /></span>
        <span className="ra-mode-text">{fill(tool.available ? 'opens_in' : 'opens_not_yet', { tool: toolName })}</span>
        {storage && <a className="collab-link" href={storage}>{fill('tool_storage', { tool: toolName })}</a>}
      </div>

      {error && <p className="collab-error ra-line" role="alert">{error}</p>}
      {notice && <p className="collab-ok" role="status">{notice}</p>}
      {projects.error && <p className="collab-error" role="alert">{t('projects_error')}</p>}
      {!projects.list && !projects.error && <p className="collab-muted">{t('projects_loading')}</p>}
      {projects.list && regular.length === 0 && <p className="collab-muted">{t('projects_empty')}</p>}
      <ul className="home-projects">
        {regular.map(card)}
      </ul>
      {projects.list?.some(p => p.variants?.some(v => v.archived)) && (
        <label className="ra-check ra-archived-toggle">
          <input type="checkbox" checked={showArchived} onChange={e => setShowArchived(e.target.checked)} />
          {t('show_archived')}
        </label>
      )}

      <section className="ra-section">
        <div className="home-section-head">
          <h2>{t('templates')}</h2>
          {isAdmin && (
            <div className="home-section-actions">
              <button type="button" className="collab-btn collab-btn-primary" onClick={() => { say(null); setDialog({ kind: 'new', template: true }) }}>{t('project_new')}</button>
            </div>
          )}
        </div>
        {projects.list && templates.length === 0 && <p className="collab-muted">{t('templates_empty')}</p>}
        <ul className="home-projects">
          {templates.map(card)}
        </ul>
      </section>

      {dialog?.kind === 'new' && <NewProjectDialog template={Boolean(dialog.template)} onCancel={() => setDialog(null)} onDone={done} />}
      {dialog?.kind === 'rename' && <RenameProjectDialog project={dialog.project} onCancel={() => setDialog(null)} onDone={done} />}
      {dialog?.kind === 'export' && <ExportDialog project={dialog.project} onCancel={() => setDialog(null)} onDone={() => setDialog(null)} />}
      {dialog?.kind === 'delete' && <DeleteProjectDialog project={dialog.project} onCancel={() => setDialog(null)} onDone={done} />}
      {dialog?.kind === 'members' && (
        <MembersDialog project={dialog.project} onClose={async (changed) => { setDialog(null); if (changed) await onReload() }} />
      )}
      {dialog?.kind === 'branch' && <BranchDialog project={dialog.project} onCancel={() => setDialog(null)} onDone={done} />}
      {dialog?.kind === 'variant' && <VariantDialog variant={dialog.variant} onCancel={() => setDialog(null)} onDone={done} />}
      {(dialog?.kind === 'compare' || dialog?.kind === 'merge') && (
        <PairDialog kind={dialog.kind} project={dialog.project} toolName={toolName} onCancel={() => setDialog(null)}
          onChoose={(from, to) => location.assign(toolUrl(tool, { project: dialog.project.id, action: dialog.kind, from, to }))} />
      )}
    </main>
  )
}
