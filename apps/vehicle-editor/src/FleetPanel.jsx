import { useRef } from 'react'
import { Button, GroupedList, Message, Panel, PlusIcon, UploadIcon } from '@rail-across/ui'
import { CATEGORIES } from '@rail-across/vehicles'
import { vehicleLine } from './fields.js'

/**
 * The catalogue of vehicles beside the editor: the suite's own and OSRD's,
 * each under its category, with a search; a new one, and one or many taken
 * in from OSRD's RailJSON.
 */
export default function FleetPanel({ t, language, catalogue, editor, onImport }) {
  const file = useRef(null)
  const groups = ['suite', 'osrd'].flatMap(source => [...CATEGORIES, null].map(category => ({
    key: `${source}:${category}`,
    heading: t(`fleet_${source}`),
    title: category ? t(`category_${category}`) : t('category_none'),
    items: catalogue.entries
      .filter(e => e.source === source && (category ? e.vehicle.primary_category === category : !CATEGORIES.includes(e.vehicle.primary_category)))
      .map(e => ({ id: e.key, label: e.vehicle.name || t('vehicle_unnamed'), sub: vehicleLine(e.vehicle, language) })),
  })))

  const read = async (e) => {
    const chosen = [...(e.target.files ?? [])]
    e.target.value = ''
    const found = []
    for (const f of chosen) {
      try {
        const json = JSON.parse(await f.text())
        found.push(...(Array.isArray(json) ? json : [json]))
      } catch {
        found.push(null)
      }
    }
    onImport(found)
  }

  return (
    <Panel title={t('section_vehicles')} intro={t('section_vehicles_intro')}
      actions={<>
        <Button size="small" icon={<PlusIcon size={14} />} onClick={() => editor.create()}>{t('vehicle_new')}</Button>
        <Button size="small" icon={<UploadIcon size={14} />} onClick={() => file.current?.click()}>{t('vehicle_import')}</Button>
        <input ref={file} type="file" accept=".json,application/json" multiple hidden onChange={read} />
      </>}>
      {catalogue.error && <Message tone="error">{t('fleet_failed', { error: catalogue.error })}</Message>}
      {catalogue.loading ? <p className="ui-muted">{t('loading')}</p> : (
        <GroupedList label={t('section_vehicles')} groups={groups} selected={editor.draft ? editor.draft.entry?.key ?? null : editor.selected} onSelect={editor.select}
          searchLabel={t('vehicle_search')} noneFound={t('vehicle_none_found')} empty={t('fleet_empty')} />
      )}
    </Panel>
  )
}
