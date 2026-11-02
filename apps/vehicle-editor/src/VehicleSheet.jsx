import { useState } from 'react'
import { Badge, Button, Checkbox, ConfirmDialog, DownloadIcon, Message, NumberField, SaveIcon, SelectField, Tabs, TextField, TrashIcon } from '@rail-across/ui'
import {
  CATEGORIES, COMFORTS, LOADING_GAUGES, METADATA, MODES, curveFor, curvesOf, isElectric, modeLabel, powerOf, resistanceAt, toOsrd,
  withComfort, withCurve, withMode, withoutCurve, withoutMode,
} from '@rail-across/vehicles'
import { DYNAMICS, ELECTRIC, RESISTANCE, formatNumber, kept, shown, valueAt, withValue } from './fields.js'
import EffortChart from './EffortChart.jsx'
import CurveTable from './CurveTable.jsx'

/** What went wrong in words, the server's code where there are none. */
function errorText(t, code) {
  const key = `vehicle_error_${code}`
  const said = t(key)
  return said === key ? t('vehicle_error_generic', { error: code }) : said
}

/** The vehicle's RailJSON as a file to save. */
function download(vehicle) {
  const blob = new Blob([JSON.stringify(toOsrd(vehicle), null, 2)], { type: 'application/json' })
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = `${(vehicle.name || 'vehicle').replace(/[^\p{L}\p{N}]+/gu, '-').replace(/^-|-$/g, '').toLowerCase() || 'vehicle'}.json`
  a.click()
  setTimeout(() => URL.revokeObjectURL(a.href), 1000)
}

/**
 * One vehicle on a page of its own, as OSRD's rolling stock editor has it:
 * what names it, its category, its physics, its resistance to movement, its
 * equipment, and its tractive effort curves per traction mode and comfort —
 * drawn with the resistance on the level and listed as a table. OSRD's are
 * only read and copied; the suite's are edited by their creator and the
 * admins, saved for everyone, exported as RailJSON for OSRD.
 */
export default function VehicleSheet({ t, language, editor }) {
  const { entry, draft } = editor
  const vehicle = draft?.vehicle ?? entry?.vehicle
  const editing = Boolean(draft)
  const [modeChoice, setModeChoice] = useState(null)
  const [curveChoice, setCurveChoice] = useState(null)
  if (!vehicle) {
    return <div className="veh-page veh-empty"><p className="ui-muted">{t('vehicle_choose')}</p></div>
  }

  const modes = Object.keys(vehicle.effort_curves?.modes ?? {})
  const mode = modes.includes(modeChoice) ? modeChoice : (vehicle.effort_curves?.default_mode ?? modes[0])
  const all = curvesOf(vehicle, mode)
  const plain = all.filter(c => c.profile == null && c.restriction == null)
  // a mode without a curve for a comfort has its default curve alone
  const shownCurves = plain.length ? plain : (vehicle.effort_curves?.modes?.[mode] ? [{ comfort: 'STANDARD', curve: vehicle.effort_curves.modes[mode].default_curve, index: 'default' }] : [])
  const current = shownCurves.find(c => c.index === curveChoice) ?? shownCurves.find(c => c.comfort === 'STANDARD') ?? shownCurves[0]
  const others = all.length - plain.length
  const problemAt = (field) => editor.problems.find(p => p.field === field)
  const hint = (field) => { const p = problemAt(field); return p ? t(`problem_${p.key}`) : undefined }
  const change = (next) => editor.change(next)
  const set = (path, value) => change(withValue(vehicle, path, value))

  const field = ({ path, unit, scale = 1 }) => {
    const label = `${t(`f_${path}`)}${unit ? ` [${unit}]` : ''}`
    const value = valueAt(vehicle, path)
    return editing
      ? <NumberField key={path} compact label={label} value={shown(value, scale)} min={0} hint={hint(path)} onChange={(x) => set(path, kept(x, scale))} />
      : <Fact key={path} label={label}>{formatNumber(value == null ? null : value * scale, language, 5)}</Fact>
  }
  const text = (path, label = t(`f_${path}`)) => (editing
    ? <TextField key={path} compact label={label} value={valueAt(vehicle, path) ?? ''} hint={hint(path)} onChange={(x) => set(path, x)} />
    : <Fact key={path} label={label}>{valueAt(vehicle, path) || '–'}</Fact>)

  const top = vehicle.max_speed
  const curve = current?.curve
  const power = powerOf(curveFor(vehicle, mode))
  const source = entry?.source ?? 'suite'

  return (
    <div className="veh-page">
      <header className="veh-head">
        <div className="veh-title">
          <h1>{vehicle.name || t('vehicle_unnamed')}</h1>
          <div className="veh-badges">
            {CATEGORIES.includes(vehicle.primary_category) && <Badge>{t(`category_${vehicle.primary_category}`)}</Badge>}
            {editing && !draft.entry ? <Badge tone="warn">{t('vehicle_unsaved')}</Badge>
              : source === 'osrd' ? <Badge tone="primary">{t('fleet_osrd_badge')}</Badge>
              : <span className="ui-muted">{t('vehicle_by', { name: entry?.createdBy?.name ?? '' })}</span>}
            {editing && draft.entry && editor.dirty && <Badge tone="warn">{t('vehicle_changed')}</Badge>}
          </div>
        </div>
        <div className="ui-row veh-actions">
          {editing ? <>
            <Button onClick={editor.cancel} disabled={editor.busy}>{t('cancel')}</Button>
            <Button variant="primary" icon={<SaveIcon size={16} />} onClick={editor.save} disabled={editor.busy || (draft.entry && !editor.dirty)}>{editor.busy ? t('saving') : t('save')}</Button>
          </> : <>
            <Button icon={<DownloadIcon size={16} />} onClick={() => download(vehicle)}>{t('vehicle_export')}</Button>
            <Button onClick={() => editor.copy(t('vehicle_copy_name', { name: vehicle.name }))}>{t('vehicle_copy')}</Button>
            {entry?.mayEdit && <Button variant="danger" icon={<TrashIcon size={16} />} onClick={editor.askDelete}>{t('delete')}</Button>}
            {entry?.mayEdit && <Button variant="primary" onClick={editor.edit}>{t('edit')}</Button>}
          </>}
        </div>
      </header>

      {editor.error && <Message tone="error">{errorText(t, editor.error)}</Message>}
      {editor.problems.length > 0 && <Message tone="warn">{t('vehicle_problems', { n: editor.problems.length })}</Message>}
      {(draft?.notes ?? entry?.notes ?? []).includes('resistance_per_kg') && <Message tone="info">{t('note_resistance_per_kg')}</Message>}
      {source === 'osrd' && !editing && <p className="ui-muted veh-lead">{t('fleet_osrd_lead')}</p>}

      <div className="veh-grid">
        <div className="veh-col">
          <Card title={t('vehicle_identity')}>
            <div className="ui-fields">
              {editing
                ? <TextField compact wide label={t('f_name')} value={vehicle.name} hint={hint('name')} onChange={(x) => set('name', x)} />
                : null}
              {METADATA.map(k => text(`metadata.${k}`, t(`f_metadata_${k}`)))}
            </div>
          </Card>

          <Card title={t('vehicle_categories')}>
            {editing ? <>
              <div className="ui-fields">
                <SelectField compact wide label={t('f_primary_category')} value={vehicle.primary_category ?? null} empty={t('choose')} hint={hint('primary_category')}
                  options={CATEGORIES.map(c => ({ value: c, label: t(`category_${c}`) }))}
                  onChange={(c) => change({ ...vehicle, primary_category: c, other_categories: (vehicle.other_categories ?? []).filter(o => o !== c) })} />
              </div>
              <div className="veh-checks">
                {CATEGORIES.filter(c => c !== vehicle.primary_category).map(c => (
                  <Checkbox key={c} label={t(`category_${c}`)} checked={(vehicle.other_categories ?? []).includes(c)}
                    onChange={(on) => set('other_categories', on ? [...(vehicle.other_categories ?? []), c] : vehicle.other_categories.filter(o => o !== c))} />
                ))}
              </div>
            </> : (
              <div className="ui-fields">
                <Fact label={t('f_primary_category')}>{CATEGORIES.includes(vehicle.primary_category) ? t(`category_${vehicle.primary_category}`) : '–'}</Fact>
                <Fact label={t('f_other_categories')}>{(vehicle.other_categories ?? []).map(c => t(`category_${c}`)).join(', ') || '–'}</Fact>
              </div>
            )}
          </Card>

          <Card title={t('vehicle_dynamics')}>
            <div className="ui-fields">{DYNAMICS.map(field)}</div>
          </Card>

          <Card title={t('vehicle_resistance')}>
            <p className="ui-muted veh-lead">{t('vehicle_resistance_intro')}</p>
            <div className="ui-fields veh-fields-3">{RESISTANCE.map(field)}</div>
            <p className="ui-muted veh-lead">{t('resistance_at', {
              speed: Math.round(top * 3.6), force: formatNumber(resistanceAt(vehicle, top) / 1000, language, 3),
              share: formatNumber((resistanceAt(vehicle, top) / (vehicle.mass * 9.81)) * 1000, language, 2),
            })}</p>
          </Card>

          <Card title={t('vehicle_equipment')}>
            <div className="ui-fields">
              {editing
                ? <SelectField compact label={t('f_loading_gauge')} value={vehicle.loading_gauge ?? 'G1'} options={LOADING_GAUGES.map(g => ({ value: g, label: g }))} onChange={(g) => set('loading_gauge', g)} />
                : <Fact label={t('f_loading_gauge')}>{vehicle.loading_gauge ?? '–'}</Fact>}
              {text('base_power_class')}
              {isElectric(vehicle) && ELECTRIC.map(field)}
            </div>
          </Card>
        </div>

        <div className="veh-col">
          <Card title={t('vehicle_effort')} intro={t('vehicle_effort_intro', { power: formatNumber(power / 1000, language, 4) })}>
            {modes.length > 0 && (
              <div className="veh-modes">
                <Tabs label={t('f_mode')} value={mode} onChange={(m) => { setModeChoice(m); setCurveChoice(null) }}
                  options={modes.map(m => ({ value: m, label: `${modeLabel(t, m)}${m === vehicle.effort_curves.default_mode && modes.length > 1 ? ' ★' : ''}` }))} />
              </div>
            )}
            {editing && <ModeTools t={t} vehicle={vehicle} mode={mode} modes={modes} change={change} onMode={setModeChoice} />}
            {shownCurves.length > 1 || editing ? (
              <div className="veh-comforts">
                {shownCurves.length > 1 && <Tabs label={t('f_comfort')} value={current?.index} onChange={setCurveChoice}
                  options={shownCurves.map(c => ({ value: c.index, label: t(`comfort_${c.comfort ?? 'STANDARD'}`) }))} />}
                {editing && current && <ComfortTools t={t} vehicle={vehicle} mode={mode} shownCurves={shownCurves} current={current} change={change} onCurve={setCurveChoice} />}
              </div>
            ) : null}
            {hint(`effort_curves.${mode}.${current?.index}`) && <Message tone="warn">{hint(`effort_curves.${mode}.${current.index}`)}</Message>}
            {hint('effort_curves') && <Message tone="warn">{hint('effort_curves')}</Message>}
            {current && <EffortChart t={t} vehicle={vehicle} curves={shownCurves} chosen={current.index} />}
            {current && <CurveTable t={t} language={language} curve={curve} editing={editing} maxSpeed={top}
              onChange={(c) => change(withCurve(vehicle, mode, current.index, c))} />}
            {others > 0 && <p className="ui-muted veh-lead">{t('curves_kept', { n: others })}</p>}
          </Card>
        </div>
      </div>

      {editor.ask?.kind === 'delete' && (
        <ConfirmDialog title={t('vehicle_delete_title')} confirmLabel={t('delete')} cancelLabel={t('cancel')} danger busy={editor.busy}
          onConfirm={editor.remove} onCancel={editor.dismiss}>
          <p className="ui-muted">{t('vehicle_delete_text', { name: vehicle.name })}</p>
        </ConfirmDialog>
      )}
    </div>
  )
}

/** A part of the vehicle's page: a heading, a line under it, its fields. */
function Card({ title, intro, children }) {
  return (
    <section className="veh-card">
      <h2>{title}</h2>
      {intro && <p className="ui-muted veh-lead">{intro}</p>}
      {children}
    </section>
  )
}

/** A value read: its label above it. */
function Fact({ label, children }) {
  return <div className="veh-fact"><span>{label}</span><b>{children}</b></div>
}

/** Traction modes added, taken away, the default one chosen. */
function ModeTools({ t, vehicle, mode, modes, change, onMode }) {
  const [adding, setAdding] = useState('')
  const free = MODES.filter(m => !modes.includes(m))
  return (
    <div className="veh-tools">
      {modes.length > 1 && (
        <SelectField compact label={t('f_default_mode')} value={vehicle.effort_curves.default_mode}
          options={modes.map(m => ({ value: m, label: modeLabel(t, m) }))}
          onChange={(m) => change({ ...vehicle, effort_curves: { ...vehicle.effort_curves, default_mode: m } })} />
      )}
      <div className="veh-tool-row">
        <SelectField compact label={t('mode_add')} value={adding || null} empty={t('choose')}
          options={free.map(m => ({ value: m, label: modeLabel(t, m) }))} onChange={(m) => setAdding(m ?? '')} />
        <Button size="small" disabled={!adding} onClick={() => { change(withMode(vehicle, adding, mode || null)); onMode(adding); setAdding('') }}>{t('add')}</Button>
      </div>
      {mode && modes.length > 1 && (
        <Button size="small" variant="ghost" icon={<TrashIcon size={14} />} onClick={() => { change(withoutMode(vehicle, mode)); onMode(null) }}>
          {t('mode_remove', { mode: modeLabel(t, mode) })}
        </Button>
      )}
    </div>
  )
}

/** Curves for another comfort added to the mode, one taken away. */
function ComfortTools({ t, vehicle, mode, shownCurves, current, change, onCurve }) {
  const free = COMFORTS.filter(c => !shownCurves.some(s => s.comfort === c))
  const fromDefault = current.index === 'default'
  return (
    <div className="veh-tool-row">
      {!fromDefault && free.map(c => (
        <Button key={c} size="small" onClick={() => { change(withComfort(vehicle, mode, c)); onCurve(vehicle.effort_curves.modes[mode].curves.length) }}>
          {t('comfort_add', { comfort: t(`comfort_${c}`) })}
        </Button>
      ))}
      {!fromDefault && current.comfort !== 'STANDARD' && (
        <Button size="small" variant="ghost" icon={<TrashIcon size={14} />} onClick={() => { change(withoutCurve(vehicle, mode, current.index)); onCurve(null) }}>
          {t('comfort_remove', { comfort: t(`comfort_${current.comfort}`) })}
        </Button>
      )}
    </div>
  )
}
