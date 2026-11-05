import { useRef, useState } from 'react'
import { timetablesFile, timetablesFileName, timetablesOfText } from '@rail-across/timetables/exchange'
import { MODULES, simulationUrl } from '../modules'
import { ModuleIcon } from '../icons'
import { api } from '../api'
import { useTexts } from '../i18n'
import { errorText } from '../useAction'
import { splitProjects, variantTree } from '../projects'
import { useLocalChanges } from '../localChanges'
import { ProjectHead, VariantLine } from './projects/ProjectCard'
import { CopyTemplateDialog } from './projects/dialogs'
import { download } from '../download'

/**
 * The page of a module that works on a variant — the Fahrplan-Sim —: what it
 * does, and the projects as the infrastructure editor lists them — picture,
 * title, description, who works on them, their variants as a tree with their
 * heads —, the templates under them, each variant opening in the module's
 * apps: the Fahrplan-Editor, the timetables and a single train's running time
 * in one interface. It changes no infrastructure: the projects and their
 * variants are made, branched and managed in the infrastructure editor, not
 * here; a template is copied as a project of one's own here too.
 *
 * A project's timetables go out as a file of their own, and come in from
 * one — into a project by whoever works on it, into a template by an admin;
 * all of the file's or none.
 */
export default function Simulation({ route, user, projects, onReload = async () => {} }) {
  const { t, fill } = useTexts()
  const module = MODULES.find(m => m.route === route)
  const { regular, templates } = splitProjects(projects.list)
  const localChanges = useLocalChanges()
  const [copying, setCopying] = useState(null)
  const [notice, setNotice] = useState(null)
  const [error, setError] = useState(null)
  const fileRef = useRef(null)
  const importInto = useRef(null)     // the project the file chosen goes into
  const isAdmin = user?.role === 'admin'
  const say = (text) => { setError(null); setNotice(text) }

  const exportTimetables = async (project) => {
    say(null)
    try {
      const { timetables } = await api.timetables(project.id)
      if (!timetables.length) { say(fill('timetables_none', { title: project.title })); return }
      download(timetablesFile(timetables, { project: project.title }), timetablesFileName(project.title))
    } catch (err) {
      setError(errorText(t, err.code))
    }
  }
  const importTimetables = async (e) => {
    const files = [...(e.target.files ?? [])]
    e.target.value = ''
    const project = importInto.current
    if (!files.length || !project) return
    say(null)
    try {
      const timetables = (await Promise.all(files.map(f => f.text()))).flatMap(timetablesOfText)
      await api.importTimetables(project.id, timetables)
      say(fill('timetables_imported', { n: timetables.length, title: project.title }))
    } catch (err) {
      setError(errorText(t, err.code))
    }
  }

  const card = (p) => (
    <SimulationCard key={p.id} project={p} module={module} localChanges={localChanges} mayImport={!p.template || isAdmin}
      onCopy={() => { say(null); setCopying(p) }} onExport={() => exportTimetables(p)}
      onImport={() => { importInto.current = p; fileRef.current?.click() }} />
  )
  return (
    <main className="home-main">
      <a className="ui-link ra-back" href="#/">← {t('back')}</a>
      <h2 className="ra-page-title">{t(`module_${module.id}`)}</h2>
      <div className="ra-mode">
        <span className="ra-mode-icon"><ModuleIcon id={module.id} /></span>
        <span className="ra-mode-text">{t(`module_${module.id}_open`)}</span>
      </div>
      <input ref={fileRef} type="file" accept=".json,application/json" multiple hidden onChange={importTimetables} />
      {error && <p className="ui-error ra-line" role="alert">{error}</p>}
      {notice && <p className="ui-ok" role="status">{notice}</p>}
      {projects.error && <p className="ui-error" role="alert">{t('projects_error')}</p>}
      {!projects.list && !projects.error && <p className="ui-muted">{t('projects_loading')}</p>}

      <div className="home-section-head"><h2>{t('projects')}</h2></div>
      {projects.list && regular.length === 0 && <p className="ui-muted">{t('projects_empty')}</p>}
      <ul className="home-projects">{regular.map(card)}</ul>

      <section className="ra-section">
        <div className="home-section-head"><h2>{t('templates')}</h2></div>
        {projects.list && templates.length === 0 && <p className="ui-muted">{t('templates_empty')}</p>}
        <ul className="home-projects">{templates.map(card)}</ul>
      </section>
      {copying && (
        <CopyTemplateDialog project={copying} onCancel={() => setCopying(null)}
          onDone={async (made) => { setCopying(null); await onReload(); say(fill(made.timetables ? 'template_copied' : 'template_copied_alone', { title: made.project?.title ?? '', n: made.timetables })) }} />
      )}
    </main>
  )
}

/**
 * A project as the infrastructure editor shows it, without managing it: its
 * variants, each opening in the module's apps; its timetables exported and,
 * where one may, imported; a template copied as a project.
 */
function SimulationCard({ project, module, localChanges, mayImport, onCopy, onExport, onImport }) {
  const { t } = useTexts()
  return (
    <li className="home-project">
      <ProjectHead project={project} />
      <ul className="ra-variant-tree">
        {variantTree(project.variants.filter(v => !v.archived)).map(({ variant, depth }) => (
          <li key={variant.id} className="ra-variant-row" style={{ '--depth': depth }}>
            <div className="ra-variant-main">
              <VariantLine variant={variant} depth={depth} changes={localChanges[variant.id]} />
              <span className="ra-variant-buttons">
                {module.apps.map(app => (
                  <a key={app.id} className="ui-btn ui-btn-primary" href={simulationUrl(module, app, project.id, variant.id)}>{t(`open_${app.id}`)}</a>
                ))}
              </span>
            </div>
          </li>
        ))}
      </ul>
      <div className="ra-project-actions">
        {project.template && <button type="button" className="ui-btn ui-btn-primary" onClick={onCopy}>{t('template_copy')}</button>}
        <button type="button" className="ui-btn" onClick={onExport}>{t('timetables_export')}</button>
        {mayImport && <button type="button" className="ui-btn" onClick={onImport}>{t('timetables_import')}</button>}
      </div>
    </li>
  )
}
