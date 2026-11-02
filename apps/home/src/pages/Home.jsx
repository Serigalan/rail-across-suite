import { EDITORS, INFRASTRUCTURE_TOOLS, SIMULATIONS, hashOf, tileUrl } from '../modules'
import { ModuleArt, ModuleIcon } from '../icons'
import { useTexts } from '../i18n'

/**
 * What a tile shows of a module: its drawing, framed on its own ground, and
 * its name — grey for one still to come, the phase that brings it as its title.
 */
function TileBody({ module }) {
  const { t } = useTexts()
  return (
    <>
      <span className="ra-tile-art"><ModuleArt id={module.id} /></span>
      <span className="ra-tile-name">{t(`module_${module.id}`)}</span>
    </>
  )
}

/** One module as a tile. */
function ModuleTile({ module }) {
  const { fill } = useTexts()
  return (
    <li>
      <a className={`ra-tile${module.available ? '' : ' ra-tile-planned'}`} href={tileUrl(module)}
        title={module.available ? undefined : fill('phase', { n: module.phase })}>
        <TileBody module={module} />
      </a>
    </li>
  )
}

/**
 * The infrastructure editor's tile, twice as wide: the module on the left,
 * its tools on the right as a list of names — each opening the editor in that tool,
 * grey for one still to come.
 */
function InfrastructureTile({ module }) {
  const { t, fill } = useTexts()
  return (
    <li className="ra-tile-wide">
      <div className="ra-tile ra-tile-split">
        <a className="ra-tile-main" href={tileUrl(module)}><TileBody module={module} /></a>
        <ul className="ra-subtools">
          {INFRASTRUCTURE_TOOLS.map(tool => (
            <li key={tool.id}>
              <a className={`ra-subtool${tool.available ? '' : ' ra-subtool-planned'}`} href={hashOf({ page: 'infrastructure', tool: tool.id })}
                title={tool.available ? undefined : fill('tool_soon', { n: tool.phase })}>
                <span className="ra-subtool-icon"><ModuleIcon id={tool.id} /></span>
                <span className="ra-subtool-name">{t(`tool_${tool.id}`)}</span>
              </a>
            </li>
          ))}
        </ul>
      </div>
    </li>
  )
}

/**
 * The start page: the editors, which write the description of the railway —
 * the infrastructure editor with its tools, the Fahrzeug-Editor —, and the
 * simulations, which read it.
 */
export default function Home() {
  const { t } = useTexts()
  return (
    <main className="home-main">
      <h2 className="ra-group">{t('editors')}</h2>
      <ul className="ra-tiles">
        {EDITORS.map(m => (m.id === 'infrastructure' ? <InfrastructureTile key={m.id} module={m} /> : <ModuleTile key={m.id} module={m} />))}
      </ul>
      <h2 className="ra-group ra-group-next">{t('simulations')}</h2>
      <ul className="ra-tiles">{SIMULATIONS.map(m => <ModuleTile key={m.id} module={m} />)}</ul>
    </main>
  )
}
