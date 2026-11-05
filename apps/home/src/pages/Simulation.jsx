import { MODULES, simulationUrl } from '../modules'
import { ModuleIcon } from '../icons'
import { useTexts } from '../i18n'
import { splitProjects, variantTree } from '../projects'
import { useLocalChanges } from '../localChanges'
import { ProjectHead, VariantLine } from './projects/ProjectCard'

/**
 * The page of a module that works on a variant — the Fahrplan-Sim —: what it
 * does, and the projects as the infrastructure editor lists them — picture,
 * title, description, who works on them, their variants as a tree with their
 * heads —, the templates under them, each variant opening in the module's
 * apps: the Fahrplan-Editor, the timetables and a single train's running time
 * in one interface. It changes no infrastructure: the projects and their
 * variants are made, branched and managed in the infrastructure editor, not
 * here.
 */
export default function Simulation({ route, projects }) {
  const { t } = useTexts()
  const module = MODULES.find(m => m.route === route)
  const { regular, templates } = splitProjects(projects.list)
  const localChanges = useLocalChanges()
  const card = (p) => <SimulationCard key={p.id} project={p} module={module} localChanges={localChanges} />
  return (
    <main className="home-main">
      <a className="ui-link ra-back" href="#/">← {t('back')}</a>
      <h2 className="ra-page-title">{t(`module_${module.id}`)}</h2>
      <div className="ra-mode">
        <span className="ra-mode-icon"><ModuleIcon id={module.id} /></span>
        <span className="ra-mode-text">{t(`module_${module.id}_open`)}</span>
      </div>
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
    </main>
  )
}

/** A project as the infrastructure editor shows it, without managing it: its variants, each opening in the module's apps. */
function SimulationCard({ project, module, localChanges }) {
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
    </li>
  )
}
