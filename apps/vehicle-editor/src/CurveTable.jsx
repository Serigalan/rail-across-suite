import { useState } from 'react'
import { Button, FormDialog, IconButton, NumberField, PlusIcon, TrashIcon } from '@rail-across/ui'
import { curveFromPower } from '@rail-across/vehicles'
import { formatNumber, parseRows, shown } from './fields.js'

/**
 * A tractive effort curve as a table: speed [km/h], effort [kN] and the power
 * at the rim they make [kW], a row per point. Edited, a row is added after
 * the last, taken away, the whole curve made from the starting effort and the
 * power, or pasted from a spreadsheet.
 */
export default function CurveTable({ t, language, curve, editing, maxSpeed, onChange }) {
  const [dialog, setDialog] = useState(null)   // 'power' | 'paste'
  const speeds = curve?.speeds ?? [], efforts = curve?.max_efforts ?? []
  const set = (i, speed, effort) => onChange({
    speeds: speeds.map((s, j) => (j === i ? speed : s)),
    max_efforts: efforts.map((e, j) => (j === i ? effort : e)),
  })
  const add = () => {
    const last = speeds.length - 1
    onChange({ speeds: [...speeds, (speeds[last] ?? -10 / 3.6) + 10 / 3.6], max_efforts: [...efforts, efforts[last] ?? 0] })
  }
  const remove = (i) => onChange({ speeds: speeds.filter((_, j) => j !== i), max_efforts: efforts.filter((_, j) => j !== i) })
  const num = (x, digits) => formatNumber(x, language, digits)

  return (
    <div className="veh-curve">
      <table className="veh-table-base veh-table">
        <thead>
          <tr><th>{t('curve_speed')}</th><th>{t('curve_effort')}</th><th>{t('curve_power')}</th>{editing && <th aria-label={t('remove')} />}</tr>
        </thead>
        <tbody>
          {speeds.map((s, i) => (
            <tr key={i}>
              {editing ? <>
                <td><input className="ui-input veh-cell" type="number" step="any" aria-label={`${t('curve_speed')} ${i + 1}`} value={shown(s, 3.6) ?? ''}
                  onChange={(e) => set(i, e.target.value === '' ? null : Number(e.target.value) / 3.6, efforts[i])} /></td>
                <td><input className="ui-input veh-cell" type="number" step="any" aria-label={`${t('curve_effort')} ${i + 1}`} value={shown(efforts[i], 1 / 1000) ?? ''}
                  onChange={(e) => set(i, s, e.target.value === '' ? null : Number(e.target.value) * 1000)} /></td>
              </> : <>
                <td>{num(s * 3.6, 4)}</td>
                <td>{num(efforts[i] / 1000, 4)}</td>
              </>}
              <td className="ui-muted">{Number.isFinite(s * efforts[i]) && s != null && efforts[i] != null ? Math.round((s * efforts[i]) / 1000).toLocaleString(language) : '–'}</td>
              {editing && <td><IconButton label={t('remove')} icon={<TrashIcon size={14} />} variant="ghost" size="small" onClick={() => remove(i)} /></td>}
            </tr>
          ))}
        </tbody>
      </table>
      {editing && (
        <div className="ui-row veh-curve-actions">
          <Button size="small" icon={<PlusIcon size={14} />} onClick={add}>{t('curve_add_row')}</Button>
          <Button size="small" onClick={() => setDialog('power')}>{t('curve_from_power')}</Button>
          <Button size="small" onClick={() => setDialog('paste')}>{t('curve_paste')}</Button>
        </div>
      )}
      {dialog === 'power' && <PowerDialog t={t} curve={curve} maxSpeed={maxSpeed} onCancel={() => setDialog(null)} onDone={(c) => { onChange(c); setDialog(null) }} />}
      {dialog === 'paste' && <PasteDialog t={t} onCancel={() => setDialog(null)} onDone={(c) => { onChange(c); setDialog(null) }} />}
    </div>
  )
}

/** The curve made from the starting effort and the power at the rim, up to the top speed. */
function PowerDialog({ t, curve, maxSpeed, onCancel, onDone }) {
  const [effort, setEffort] = useState(() => Math.round((curve?.max_efforts?.[0] ?? 160000) / 1000))
  const [power, setPower] = useState(() => Math.round(Math.max(0, ...(curve?.speeds ?? []).map((s, i) => s * (curve.max_efforts[i] ?? 0))) / 1000) || 2000)
  const [step, setStep] = useState(10)
  const ok = effort > 0 && power > 0 && step > 0 && maxSpeed > 0
  return (
    <FormDialog title={t('curve_from_power')} submitLabel={t('curve_make')} cancelLabel={t('cancel')} canSubmit={ok} onCancel={onCancel}
      onSubmit={() => onDone(curveFromPower({ effort: effort * 1000, power: power * 1000, maxSpeed, step }))}>
      <p className="ui-muted">{t('curve_from_power_intro', { speed: Math.round(maxSpeed * 3.6) })}</p>
      <div className="ui-fields">
        <NumberField label={t('curve_start_effort')} value={effort} min={0} onChange={setEffort} />
        <NumberField label={t('curve_rim_power')} value={power} min={0} onChange={setPower} />
        <NumberField label={t('curve_step')} value={step} min={1} onChange={setStep} />
      </div>
    </FormDialog>
  )
}

/** The curve pasted as rows of speed and effort. */
function PasteDialog({ t, onCancel, onDone }) {
  const [text, setText] = useState('')
  const curve = parseRows(text)
  return (
    <FormDialog title={t('curve_paste')} submitLabel={t('curve_take', { n: curve.speeds.length })} cancelLabel={t('cancel')} canSubmit={curve.speeds.length >= 2}
      onCancel={onCancel} onSubmit={() => onDone(curve)}>
      <p className="ui-muted">{t('curve_paste_intro')}</p>
      <label className="ui-field">
        <span>{t('curve_rows')}</span>
        <textarea rows={10} value={text} onChange={(e) => setText(e.target.value)} placeholder={'0\t158\n10\t158\n30\t108'} />
      </label>
    </FormDialog>
  )
}
