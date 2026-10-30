import { useState } from 'react'
import { Checkbox, Panel, Section } from '@rail-across/ui'
import {
  BASEMAP_GROUPS, LANDESVERMESSUNG_STATES, LANDESVERMESSUNG_THUMBNAIL, allBasemaps, lvBasemapId, lvStatesOf,
} from '@rail-across/basemaps'
import { NO_BASEMAP } from '../plan/PlanView.jsx'

/** The basemap the tool opens with, until one is chosen: light, so the signals read on it. */
export const DEFAULT_PLAN_BASEMAP = 'positron'

/**
 * The map layers, as the Layout-Tool has them: every basemap a row with its
 * preview, grouped by where it covers; the states' orthophotos as tiles to
 * tick, several at once; and the plan without a map.
 */
export default function LayersPanel({ basemap, setBasemap, t, language }) {
  const [lvOpen, setLvOpen] = useState(lvStatesOf(basemap).length > 0)
  const lvIds = lvStatesOf(basemap).map(s => s.id)
  const setLvState = (id, on) => {
    const next = on ? [...lvIds, id] : lvIds.filter(x => x !== id)
    setBasemap(next.length ? lvBasemapId(next) : DEFAULT_PLAN_BASEMAP)
  }
  const row = (id, thumbnail, label, sublabel, extra = {}) => (
    <button key={id} type="button" className={`layer-row${basemap === id || extra.active ? ' is-active' : ''}`}
      aria-pressed={extra.toggle ? undefined : basemap === id} aria-expanded={extra.toggle ? extra.open : undefined}
      onClick={extra.toggle ?? (() => setBasemap(id))}>
      {thumbnail ? <img src={thumbnail} alt="" className="layer-thumbnail" /> : <span className="layer-thumbnail layer-thumbnail-none" aria-hidden="true" />}
      <span className="layer-text"><span>{label}</span>{sublabel && <span className="ui-muted layer-sub">{sublabel}</span>}</span>
      {extra.toggle && <span className={`layer-caret${extra.open ? ' open' : ''}`} aria-hidden="true">▸</span>}
    </button>
  )
  const lvSub = !lvIds.length ? t('basemap_landesvermessung_pick')
    : lvIds.length <= 2 ? lvStatesOf(basemap).map(s => s.name).join(', ') : t('basemap_landesvermessung_count', { n: lvIds.length })

  return (
    <Panel title={t('section_layers')} intro={t('section_layers_intro')}>
      {BASEMAP_GROUPS.map(group => (
        <Section key={group} title={t(`layers_group_${group}`)}>
          <div className="layer-list">
            {allBasemaps().filter(b => b.group === group).map(b => row(b.id, b.thumbnail, t(b.labelKey)))}
            {group === 'germany' && row('lv', LANDESVERMESSUNG_THUMBNAIL, t('basemap_landesvermessung'), lvSub,
              { toggle: () => setLvOpen(v => !v), open: lvOpen, active: lvIds.length > 0 })}
            {group === 'germany' && lvOpen && (
              <div className="layer-states" lang={language}>
                {LANDESVERMESSUNG_STATES.map(s => (
                  <Checkbox key={s.id} label={s.name} checked={lvIds.includes(s.id)} disabled={!s.wmsUrl} onChange={(on) => setLvState(s.id, on)} />
                ))}
              </div>
            )}
          </div>
        </Section>
      ))}
      <Section title={t('layers_none_group')}>
        <div className="layer-list">{row(NO_BASEMAP, null, t('basemap_none'), t('basemap_none_hint'))}</div>
      </Section>
    </Panel>
  )
}
