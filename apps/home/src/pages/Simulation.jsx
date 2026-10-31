import { MODULES, simulationUrl } from '../modules'
import { ModuleIcon } from '../icons'
import { useTexts } from '../i18n'
import { splitProjects, variantTree } from '../projects'

/**
 * The page of a simulation that is there: what it does, and the projects
 * with their variants, each opening in it. A simulation reads the
 * infrastructure and writes nothing: the projects are managed in the
 * infrastructure editor, not here.
 */
export default function Simulation({ route, projects }) {
  const { t, fill } = useTexts()
  const module = MODULES.find(m => m.route === route)
  const { regular, templates } = splitProjects(projects.list)
  const list = [...regular, ...templates]
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
      {projects.list && list.length === 0 && <p className="ui-muted">{t('projects_empty')}</p>}
      <ul className="home-projects">
        {list.map(p => (
          <li key={p.id} className="ra-sim-project">
            <h3>{p.title}</h3>
            <ul className="ra-sim-variants">
              {variantTree(p.variants.filter(v => !v.archived)).map(({ variant, depth }) => (
                <li key={variant.id} style={{ paddingLeft: depth * 18 }}>
                  <span className="ra-grow">{variant.name}</span>
                  {variant.head && <span className="ui-muted">{fill('revision', { n: variant.head.number })}</span>}
                  <a className="ui-btn ui-btn-small" href={simulationUrl(module, p.id, variant.id)}>{t('simulate')}</a>
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
    </main>
  )
}
