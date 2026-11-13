import { useState } from 'react'
import { loadSwitches, loadTracks, updateSwitch } from '../../../storage'
import { lifespanWithStatus, switchStatus, trackStatus } from '@rail-across/core/track'
import StatusField from '../StatusField'
import { SWITCH_PICK_LAYERS, hasSwitch } from '../../../map/pick'
import { useI18n } from '../../../locales/i18nContext'
import CommitBar from '../../form/CommitBar'
import CancelButton from '../../form/CancelButton'
import useMapPick, { useSelectedOnMap } from '../../../map/useMapPick'

/**
 * The planning status of a switch, picked on the map like a switch to delete.
 * Left open, the status follows the switch's tracks (switchStatus), which is
 * what it is for most switches — only one renewed in a standing track, or
 * taken out where its tracks stay, needs a lifespan of its own (decision
 * 0028: a switch with none shows what its tracks say).
 */
export default function SwitchStatusForm({ onCommitted }) {
  const { t, fill } = useI18n()
  const [selected, setSelected] = useState(null)   // switch record
  const [status, setStatus] = useState(null)


  useMapPick({
    active: selected === null, layers: SWITCH_PICK_LAYERS, accept: hasSwitch,
    onPick: ({ switchId }) => {
      const sw = loadSwitches().find(s => s.switchId === switchId)
      if (!sw) return
      setSelected(sw)
      setStatus(sw.lifespan ? trackStatus(sw) : null)
    },
  })
  useSelectedOnMap(selected ? { switchId: selected.switchId } : null)

  const derived = selected
    ? switchStatus({ ...selected, lifespan: undefined }, new Map(loadTracks().map(tr => [tr.id, tr])))
    : null

  const handleCommit = () => {
    updateSwitch(selected.switchId, { lifespan: lifespanWithStatus(selected.lifespan, status) })
    onCommitted?.()
  }

  return (
    <>
      {!selected ? (
        <p>{t('switch_status_hint')}</p>
      ) : (
        <div className="element-form">
          <p>{fill('switch_status_selected', { name: selected.name || selected.switchId.slice(0, 8) })}</p>
          <StatusField value={status} onChange={setStatus} auto={derived} statuses={['new', 'removal']} />
        </div>
      )}
      {selected
        ? <CommitBar onCommit={handleCommit} onCancel={onCommitted} />
        : <CancelButton onClick={onCommitted} />}
    </>
  )
}
