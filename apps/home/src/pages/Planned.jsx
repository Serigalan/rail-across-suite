import { MODULES } from '../modules'
import { ModuleIcon } from '../icons'
import { useTexts } from '../i18n'

/** The page of a module that is not there yet: what it will be, and when. */
export default function Planned({ route }) {
  const { t, fill } = useTexts()
  const module = MODULES.find(m => m.route === route)
  return (
    <main className="home-main">
      <a className="ui-link ra-back" href="#/">← {t('back')}</a>
      <section className="ra-planned">
        <span className="ra-tile-icon"><ModuleIcon id={module.id} /></span>
        <div>
          <h2>{t(`module_${module.id}`)}</h2>
          <p>{t(`module_${module.id}_desc`)}</p>
          <p className="ui-muted">{t('planned_text')}</p>
          <span className="ui-badge">{fill('phase', { n: module.phase })}</span>
        </div>
      </section>
    </main>
  )
}
