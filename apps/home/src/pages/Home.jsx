import { EDITORS, SIMULATIONS, hashOf } from '../modules'
import { ModuleIcon } from '../icons'
import { useTexts } from '../i18n'

/** One module as a tile: its icon, name and what it is for, and whether it is there yet. */
function ModuleTile({ module }) {
  const { t, fill } = useTexts()
  return (
    <li>
      <a className={`ra-tile${module.available ? '' : ' ra-tile-planned'}`} href={hashOf({ page: module.route })}>
        <span className="ra-tile-icon"><ModuleIcon id={module.id} /></span>
        <span className="ra-tile-name">{t(`module_${module.id}`)}</span>
        <span className="ra-tile-desc">{t(`module_${module.id}_desc`)}</span>
        <span className={`ra-badge${module.available ? ' ra-badge-ok' : ''}`}
          title={module.available ? undefined : fill('phase', { n: module.phase })}>
          {t(module.available ? 'available' : 'planned')}
        </span>
      </a>
    </li>
  )
}

/**
 * The start page: the three editors, which write the description of the
 * railway, a line, and the three simulations, which read it.
 */
export default function Home() {
  const { t } = useTexts()
  return (
    <main className="home-main">
      <p className="ra-tagline">{t('tagline')}</p>
      <h2 className="ra-group">{t('editors')}</h2>
      <ul className="ra-tiles">{EDITORS.map(m => <ModuleTile key={m.id} module={m} />)}</ul>
      <hr className="ra-rule" />
      <h2 className="ra-group">{t('simulations')}</h2>
      <ul className="ra-tiles">{SIMULATIONS.map(m => <ModuleTile key={m.id} module={m} />)}</ul>
    </main>
  )
}
