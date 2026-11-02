import { Button, IconButton, Message, NumberField, Panel, Section, SelectField, TextField, TrashIcon } from '@rail-across/ui'
import { curveFor, modeLabel, powerOf } from '@rail-across/vehicles'
import { vehicleUrl } from '../suite.js'
import { formatDistance } from '@rail-across/running-times'

/**
 * The train to simulate: its vehicle from the catalogue — shown in the
 * Fahrzeug-Editor, in a tab of its own —, with the traction
 * mode it runs in — the net does not say yet where there is which overhead
 * line — and the comfort its curve is for; where it starts, the stops on the way
 * with how long it stands, where it ends — each a signal, a platform or a
 * point picked on the map —, its margin and how it is spread; the path
 * found over the net, what the net lacks along it, and the button that runs it.
 */
export default function TrainPanel(props) {
  const { t, language, places, catalogue, entry, vehicle, chooseVehicle, modes, mode, comforts, comfort, setTraction, start, setStart, end, setEnd, stops, setStops, margin, setMargin, marginText, distribution, setDistribution, picking, setPicking, route, run, stale, busy, go } = props
  const length = route?.path ? route.offsets.at(-1) : null
  const gaps = run?.properties?.gaps
  return (
    <Panel title={t('section_train')} intro={t('section_train_intro')}
      foot={<Button variant="primary" disabled={!route?.path || marginText == null || busy || !vehicle} onClick={go}>{busy ? t('running') : t('run')}</Button>}>
      <Section title={t('vehicle')} actions={entry && <a className="ui-link" href={vehicleUrl(entry.key)} target="_blank" rel="noopener" title={t('vehicle_show_title')}>{t('vehicle_show')}</a>}>
        {catalogue.loading ? <p className="ui-muted">{t('loading')}</p> : !vehicle ? <Message tone="warn">{t('fleet_empty')}</Message> : <>
          <div className="ui-fields">
            <SelectField wide label={t('vehicle')} value={entry.key} onChange={(k) => k && chooseVehicle(k)}
              options={catalogue.entries.map(e => ({ value: e.key, label: e.source === 'osrd' ? `${e.vehicle.name} (OSRD)` : e.vehicle.name }))} />
            {modes.length > 1 && <SelectField compact label={t('f_mode')} value={mode} onChange={(m) => m && setTraction(tr => ({ ...tr, mode: m }))}
              options={modes.map(m => ({ value: m, label: modeLabel(t, m) }))} />}
            {comforts.length > 1 && <SelectField compact label={t('f_comfort')} value={comfort} onChange={(c) => c && setTraction(tr => ({ ...tr, comfort: c }))}
              options={comforts.map(c => ({ value: c, label: t(`comfort_${c}`) }))} />}
          </div>
          <p className="ui-muted sim-spec">{t('vehicle_spec', { mass: Math.round(vehicle.mass / 1000), length: Math.round(vehicle.length), speed: Math.round(vehicle.max_speed * 3.6), power: Math.round(powerOf(curveFor(vehicle, mode, comfort)) / 1000) })}</p>
        </>}
      </Section>

      <Section title={t('journey')}>
        <PlaceField label={t('start')} place={start} onChange={setStart} kind="start" {...{ places, picking, setPicking, t }} />
        {stops.map((s, i) => (
          <div key={s.key} className="sim-stop-row">
            <span className="sim-stop-label" title={s.place.label}>{s.place.label}</span>
            <NumberField compact label={t('dwell')} value={s.duration} min={0} step={5}
              onChange={(v) => setStops(list => list.map((x, j) => (j === i ? { ...x, duration: v ?? 0 } : x)))} />
            <IconButton label={t('remove')} icon={<TrashIcon />} variant="ghost" size="small" onClick={() => setStops(list => list.filter((_, j) => j !== i))} />
          </div>
        ))}
        <AddStop {...{ places, picking, setPicking, setStops, t }} />
        <PlaceField label={t('end')} place={end} onChange={setEnd} kind="end" {...{ places, picking, setPicking, t }} />
        {route?.failedAt != null && <Message tone="warn">{t('no_way', { place: route.failedAt === stops.length + 1 ? t('end') : t('stop_n', { n: route.failedAt }) })}</Message>}
        {length != null && <p className="ui-muted">{t('path_length', { length: formatDistance(length, language), tracks: route.path.length })}</p>}
      </Section>

      <Section title={t('margin')}>
        <div className="ui-fields">
          <TextField compact label={t('margin_value')} value={margin} onChange={setMargin} hint={marginText == null ? t('margin_invalid') : undefined} placeholder="5 %" />
          <SelectField compact label={t('distribution')} value={distribution} onChange={(v) => v && setDistribution(v)}
            options={[{ value: 'mareco', label: t('distribution_mareco') }, { value: 'linear', label: t('distribution_linear') }]} />
        </div>
      </Section>

      {run?.error && !stale && <Message tone="error">{t(`error_${run.error.code}`, { message: run.error.message })}</Message>}
      {gaps && !stale && (gaps.heights.length > 0 || gaps.speed.length > 0) && (
        <Message tone="info">
          {[gaps.heights.length && t('gaps_heights', { length: formatDistance(gaps.heights.reduce((s, g) => s + g.end - g.begin, 0), language) }),
            gaps.speed.length && t('gaps_speed', { length: formatDistance(gaps.speed.reduce((s, g) => s + g.end - g.begin, 0), language) })].filter(Boolean).join(' ')}
        </Message>
      )}
    </Panel>
  )
}

/** A place: chosen from the signals and platforms, or picked on the map. */
function PlaceField({ label, place, onChange, kind, places, picking, setPicking, t }) {
  const options = [
    ...places.signals.map(p => ({ value: p.id, label: p.label })),
    ...places.platforms.map(p => ({ value: p.id, label: p.label })),
    ...(place?.id.startsWith('map:') ? [{ value: place.id, label: place.label }] : []),
  ]
  const find = (id) => [...places.signals, ...places.platforms].find(p => p.id === id) ?? (place?.id === id ? place : null)
  return (
    <div className="sim-place">
      <SelectField wide label={label} value={place?.id ?? null} empty={t('choose')} options={options} onChange={(id) => onChange(id ? find(id) : null)} />
      <Button size="small" active={picking === kind} onClick={() => setPicking(picking === kind ? null : kind)}>{t('on_map')}</Button>
    </div>
  )
}

/** A stop added: a place from the lists, or one picked on the map. */
function AddStop({ places, picking, setPicking, setStops, t }) {
  const all = [...places.signals, ...places.platforms]
  return (
    <div className="sim-place">
      <SelectField wide label={t('add_stop')} value={null} empty={t('choose')} options={all.map(p => ({ value: p.id, label: p.label }))}
        onChange={(id) => { const place = all.find(p => p.id === id); if (place) setStops(list => [...list, { key: `${Date.now()}`, place, duration: 30 }]) }} />
      <Button size="small" active={picking === 'stop'} onClick={() => setPicking(picking === 'stop' ? null : 'stop')}>{t('on_map')}</Button>
    </div>
  )
}
