import { INFRASTRUCTURE_TOOLS, hashOf, listedProjects, openUrl, toolOf } from '../modules'
import { ModuleIcon } from '../icons'
import { blobUrl } from '../api'
import { useTexts } from '../i18n'

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

/** A date as the list shows it: day and month, the year where it is not this one. */
function dateText(iso, language) {
  if (!iso) return ''
  const d = new Date(iso)
  const sameYear = d.getFullYear() === new Date().getFullYear()
  return d.toLocaleDateString(language === 'de' ? 'de-DE' : 'en-GB', { day: 'numeric', month: 'short', ...(sameYear ? {} : { year: 'numeric' }) })
}

/** One project: its picture, title and description, and its variants, each opening in the chosen tool. */
function ProjectRow({ project, tool }) {
  const { t, fill, language } = useTexts()
  const picture = blobUrl(project.imageHash)
  return (
    <li className="home-project ra-project">
      {picture ? <img className="ra-project-image" src={picture} alt="" /> : <span className="ra-project-image" />}
      <div className="ra-project-body">
        <h3>{project.title}</h3>
        {project.description && <p className="collab-muted">{project.description}</p>}
        <ul className="ra-variants">
          {project.variants.map(v => {
            const url = openUrl(tool, project.id, v.id)
            const label = (
              <>
                <span className="ra-variant-name">{v.name}</span>
                {v.head && <span className="ra-variant-meta">{fill('revision', { n: v.head.number })} · {v.head.author?.name} · {dateText(v.head.createdAt, language)}</span>}
              </>
            )
            return (
              <li key={v.id}>
                {url
                  ? <a className="ra-variant" href={url} title={`${t('open')}: ${t(`tool_${tool.id}`)}`}>{label}</a>
                  : <span className="ra-variant ra-variant-off" aria-disabled="true">{label}</span>}
              </li>
            )
          })}
        </ul>
      </div>
    </li>
  )
}

/**
 * The infrastructure editor: its three tools, a line, and the projects of the
 * suite's server — opened in the tool chosen above. Signed in one always is
 * here: the server shows the sign-in page before the suite.
 */
export default function Infrastructure({ toolId, projects }) {
  const { t, fill } = useTexts()
  const tool = toolOf(toolId)
  const toolName = t(`tool_${tool.id}`)
  const listed = projects.list ? listedProjects(projects.list) : null
  return (
    <main className="home-main">
      <a className="collab-link ra-back" href="#/">← {t('back')}</a>
      <h2 className="ra-page-title">{t('module_infrastructure')}</h2>
      <ToolTabs current={tool.id} />
      <hr className="ra-rule" />

      <div className="home-section-head">
        <h2>{t('projects')}</h2>
        {tool.available && (
          <div className="home-section-actions">
            <a className="collab-btn" href={tool.path}>{t('projects_manage')}</a>
          </div>
        )}
      </div>
      <p className={`ra-mode${tool.available ? '' : ' ra-mode-off'}`}>
        <span className="ra-mode-icon"><ModuleIcon id={tool.id} /></span>
        {fill(tool.available ? 'opens_in' : 'opens_not_yet', { tool: toolName })}
      </p>

      {projects.error && <p className="collab-error" role="alert">{t('projects_error')}</p>}
      {!listed && !projects.error && <p className="collab-muted">{t('projects_loading')}</p>}
      {listed?.length === 0 && <p className="collab-muted">{t('projects_empty')}</p>}
      <ul className={`home-projects${tool.available ? '' : ' ra-projects-off'}`}>
        {(listed ?? []).map(p => <ProjectRow key={p.id} project={p} tool={tool} />)}
      </ul>
    </main>
  )
}
