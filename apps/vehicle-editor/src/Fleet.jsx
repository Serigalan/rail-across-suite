import { useState } from 'react'
import { ConfirmDialog, Message, Workspace } from '@rail-across/ui'
import { fromOsrd } from '@rail-across/vehicles'
import FleetPanel from './FleetPanel.jsx'
import VehicleSheet from './VehicleSheet.jsx'

/**
 * The vehicles: the catalogue in the panel, the vehicle chosen on the page
 * beside it. Taken in from RailJSON, one vehicle opens as a new one to look
 * over and save; several are saved at once.
 */
export default function Fleet({ t, language, catalogue, editor }) {
  const [imported, setImported] = useState(null)   // { saved, failed } | { error }

  const onImport = async (list) => {
    setImported(null)
    const taken = list.map(raw => { try { return fromOsrd(raw) } catch { return null } })
    const good = taken.filter(Boolean)
    if (!good.length) { setImported({ error: 'not_rolling_stock' }); return }
    if (good.length === 1 && taken.length === 1) {
      const { vehicle, notes } = good[0]
      editor.create(vehicle, notes)
      return
    }
    let saved = 0
    for (const { vehicle } of good) {
      try {
        await catalogue.create(vehicle)
        saved++
      } catch { /* counted below */ }
    }
    setImported({ saved, failed: taken.length - saved })
  }

  return (
    <>
      <FleetPanel t={t} language={language} catalogue={catalogue} editor={editor} onImport={onImport} />
      <Workspace>
        <div className="veh-scroll">
          {imported && (
            <div className="veh-notice">
              <Message tone={imported.error || imported.failed ? 'warn' : 'ok'}>
                {imported.error ? t('import_none') : t('import_done', { saved: imported.saved, failed: imported.failed })}
              </Message>
            </div>
          )}
          <VehicleSheet key={editor.draft ? editor.draft.entry?.key ?? 'new' : editor.selected ?? 'none'}
            t={t} language={language} editor={editor} />
        </div>
      </Workspace>
      {editor.ask?.kind === 'discard' && (
        <ConfirmDialog title={t('discard_title')} confirmLabel={t('discard')} cancelLabel={t('keep_editing')} danger
          onConfirm={editor.confirm} onCancel={editor.dismiss}>
          <p className="ui-muted">{t('discard_text')}</p>
        </ConfirmDialog>
      )}
    </>
  )
}
