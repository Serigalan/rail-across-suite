import { useState } from 'react'
import { Badge, Button, Checkbox, IconButton, Message, NumberField, PlusIcon, SelectField, TextField, TrashIcon } from '@rail-across/ui'
import { CATEGORIES, categoryColor, formatClock, newPoint, parseClock, trainProblems, trainTitle } from '@rail-across/timetables'
import { formatDistance, lngLatAt, marginValue, stationNamed } from '@rail-across/running-times'
import { comfortsOf, curveFor, modeLabel, powerOf } from '@rail-across/vehicles'
import { CopyIcon, DownIcon, TaktIcon, UpIcon } from './icons.jsx'
import MapView from './map/MapView.jsx'
import { insertionIndex } from './rows.js'
import { keepVehicle, vehicleUrl } from './suite.js'

/**
 * A train on its page: what it is — number, kind, name, vehicle, departure,
 * margin —, its way — the station it begins at, the stations and halts on the
 * way where it stops or passes, the station it ends at —, its times as it
 * runs, a row for every station it passes, and its way on the map. A train
 * begins and ends at a station (Bahnhof) only: the choice for its first and
 * last point offers no other.
 */
export default function TrainSheet(props) {
  const { t, train, timetables, mayEdit, onNewTimetable, onAddTrain } = props
  if (!timetables.draft) {
    return (
      <div className="tt-scroll"><div className="tt-page tt-empty">
        {timetables.loading ? <p className="ui-muted">{t('loading')}</p> : <>
          <p className="ui-muted">{timetables.list.length ? t('timetable_choose') : t('timetables_none_lead')}</p>
          {timetables.mayAdd && !timetables.list.length && <Button variant="primary" icon={<PlusIcon size={14} />} onClick={onNewTimetable}>{t('timetable_new')}</Button>}
        </>}
      </div></div>
    )
  }
  if (!train) {
    return (
      <div className="tt-scroll"><div className="tt-page tt-empty">
        <p className="ui-muted">{t('trains_none_lead')}</p>
        {mayEdit && <Button variant="primary" icon={<PlusIcon size={14} />} onClick={onAddTrain}>{t('train_new')}</Button>}
      </div></div>
    )
  }
  return <Sheet {...props} />
}

function Sheet({ t, language, train, run, timetables, stations, catalogue, record, net, mayEdit, update, onCopy, onTakt, onDelete }) {
  const entry = catalogue.entries.find(e => e.key === train.vehicle) ?? null
  const problems = trainProblems(train, stations, { vehicle: catalogue.loading || Boolean(entry) })
  const set = (patch) => update(tr => ({ ...tr, ...patch }))
  const ro = !mayEdit
  return (
    <div className="tt-scroll">
      <div className="tt-page">
        {timetables.conflict && (
          <Message tone="warn">{t('conflict', { name: timetables.conflict.updatedBy?.name ?? '?', time: new Date(timetables.conflict.updatedAt).toLocaleString(language) })}</Message>
        )}
        <header className="tt-head">
          <div className="tt-title">
            <h1><span className="tt-dot tt-dot-big" style={{ background: categoryColor(train.category) }} />{trainTitle(train) || t('train_unnamed')}</h1>
            <div className="tt-badges">
              {run?.rows && <Badge tone="ok">{t('runs_ok')}</Badge>}
              {problems.length > 0 && <Badge tone="warn">{t('problems_n', { n: problems.length })}</Badge>}
              {!problems.length && run && !run.rows && <Badge tone="warn">{t('train_cannot_run')}</Badge>}
            </div>
          </div>
          {mayEdit && (
            <div className="ui-row tt-actions">
              <Button size="small" icon={<CopyIcon />} onClick={onCopy}>{t('train_copy')}</Button>
              <Button size="small" icon={<TaktIcon />} onClick={onTakt}>{t('takt')}</Button>
              <Button size="small" variant="danger" icon={<TrashIcon size={14} />} onClick={onDelete}>{t('delete')}</Button>
            </div>
          )}
        </header>

        <div className="tt-grid">
          <div className="tt-col">
            <section className="tt-card">
              <h2>{t('card_train')}</h2>
              <div className="ui-fields tt-fields-3">
                <TextField compact label={t('f_category')} value={train.category} list="tt-categories" maxLength={20} disabled={ro} onChange={(v) => set({ category: v })} />
                <TextField compact label={t('f_number')} value={train.number} maxLength={40} disabled={ro} onChange={(v) => set({ number: v })} />
                <ClockField t={t} label={t('f_departure')} value={train.departure} disabled={ro} onChange={(v) => v != null && set({ departure: v })} />
              </div>
              <datalist id="tt-categories">{CATEGORIES.map(c => <option key={c} value={c} />)}</datalist>
              <div className="ui-fields">
                <TextField wide label={t('f_name')} value={train.name} maxLength={200} disabled={ro} placeholder={t('f_name_hint')} onChange={(v) => set({ name: v })} />
              </div>
              <VehicleFields t={t} train={train} entry={entry} catalogue={catalogue} ro={ro} set={set} />
              <div className="ui-fields tt-fields-2">
                <TextField compact label={t('f_margin')} value={train.margin} disabled={ro} placeholder="5 %" hint={marginValue(train.margin) == null ? t('margin_invalid') : undefined} onChange={(v) => set({ margin: v })} />
                <SelectField compact label={t('f_distribution')} value={train.distribution ?? 'mareco'} disabled={ro} onChange={(v) => v && set({ distribution: v })}
                  options={[{ value: 'mareco', label: t('distribution_mareco') }, { value: 'linear', label: t('distribution_linear') }]} />
              </div>
            </section>

            <RouteCard t={t} train={train} stations={stations} problems={problems} ro={ro} update={update} />
          </div>

          <div className="tt-col">
            <TimesCard t={t} language={language} train={train} run={run} stations={stations} problems={problems} ro={ro} update={update} />
            <section className="tt-card tt-map-card">
              <h2>{t('card_map')}</h2>
              <div className="tt-map-box">
                <MapView t={t} record={record} net={net} stations={stations} path={run?.route?.path ?? null}
                  points={run?.route ? run.route.chosen.map((loc, i) => ({ kind: i === 0 ? 'start' : i === run.route.chosen.length - 1 ? 'end' : train.points[i]?.stop ? 'stop' : 'pass', lngLat: lngLatAt(record, loc) })) : []} />
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>
  )
}

/** A clock time to type — 7:05, 07:05:30 —, taken once it reads; `empty` allows none. */
function ClockField({ t, label, value, onChange, disabled, empty = false, compact = true }) {
  const show = (v) => (v == null ? '' : formatClock(v, { seconds: v % 60 !== 0 }))
  const [text, setText] = useState(() => show(value))
  const [seen, setSeen] = useState(value)
  if (value !== seen) { setSeen(value); setText(show(value)) }
  const blank = !text.trim()
  const bad = blank ? !empty : parseClock(text) == null
  return (
    <TextField compact={compact} label={label} value={text} disabled={disabled} placeholder={empty ? '–' : 'hh:mm'} inputMode="numeric"
      hint={bad ? t('clock_invalid') : undefined}
      onChange={(v) => {
        setText(v)
        const parsed = v.trim() ? parseClock(v) : null
        if (parsed != null || (empty && !v.trim())) { setSeen(parsed); onChange(parsed) }
      }} />
  )
}

/** The vehicle of the catalogue, its traction mode and comfort; shown in the Fahrzeug-Editor in a tab of its own. */
function VehicleFields({ t, train, entry, catalogue, ro, set }) {
  const vehicle = entry?.vehicle ?? null
  const modes = Object.keys(vehicle?.effort_curves?.modes ?? {})
  const mode = modes.includes(train.mode) ? train.mode : vehicle?.effort_curves?.default_mode
  const comforts = vehicle ? comfortsOf(vehicle, mode) : []
  const comfort = comforts.includes(train.comfort) ? train.comfort : 'STANDARD'
  if (catalogue.loading) return <p className="ui-muted">{t('loading')}</p>
  return (
    <>
      <div className="ui-fields tt-vehicle">
        <SelectField wide label={t('f_vehicle')} value={entry?.key ?? null} empty={t('choose')} disabled={ro}
          onChange={(k) => { if (k) { keepVehicle(k); set({ vehicle: k, mode: null }) } }}
          options={catalogue.entries.map(e => ({ value: e.key, label: e.source === 'osrd' ? `${e.vehicle.name} (OSRD)` : e.vehicle.name }))} />
        {modes.length > 1 && <SelectField compact label={t('f_mode')} value={mode} disabled={ro} onChange={(m) => m && set({ mode: m })}
          options={modes.map(m => ({ value: m, label: modeLabel(t, m) }))} />}
        {comforts.length > 1 && <SelectField compact label={t('f_comfort')} value={comfort} disabled={ro} onChange={(c) => c && set({ comfort: c })}
          options={comforts.map(c => ({ value: c, label: t(`comfort_${c}`) }))} />}
      </div>
      {vehicle && (
        <p className="ui-muted tt-spec">
          {t('vehicle_spec', { mass: Math.round(vehicle.mass / 1000), length: Math.round(vehicle.length), speed: Math.round(vehicle.max_speed * 3.6), power: Math.round(powerOf(curveFor(vehicle, mode, comfort)) / 1000) })}
          {' · '}<a className="ui-link" href={vehicleUrl(entry.key)} target="_blank" rel="noopener" title={t('vehicle_show_title')}>{t('vehicle_show')}</a>
        </p>
      )}
      {train.vehicle && !entry && <Message tone="warn">{t('vehicle_missing')}</Message>}
    </>
  )
}

/** The choice of a station's track: any, or one of its tracks — for a halt, the tracks of its platforms. */
function trackOptions(t, station) {
  if (!station) return []
  if (station.kind === 'halt') return [...new Set(station.platforms.map(p => p.trackId))].map(id => ({ value: id, label: station.platforms.find(p => p.trackId === id).code ? t('track_of_platform', { code: station.platforms.find(p => p.trackId === id).code }) : t('track_platform') }))
  return station.tracks.map(tr => ({ value: tr.id, label: tr.platforms.length ? t('track_n_platform', { n: tr.label }) : t('track_n', { n: tr.label }) }))
}

/**
 * The train's way: where it begins — a station —, the points on the way —
 * stations and halts, a stop or a pass, how long it stands, a departure it
 * waits for —, where it ends — a station. Each with its track, or any.
 */
function RouteCard({ t, train, stations, problems, ro, update }) {
  const points = train.points
  const last = points.length - 1
  const ends = stations.filter(s => s.kind === 'station')
  const endOptions = ends.map(s => ({ value: s.name, label: s.name }))
  const allOptions = stations.map(s => ({ value: s.name, label: s.kind === 'halt' ? t('halt_name', { name: s.name }) : s.name }))
  const setPoints = (fn) => update(tr => ({ ...tr, points: fn(tr.points) }))
  const setPoint = (i, patch) => setPoints(ps => ps.map((p, j) => (j === i ? { ...p, ...patch } : p)))
  const problemAt = (i) => problems.find(p => p.point === i)
  // a station kept by name that the variant does not have stays in the choice
  const withMissing = (options, name) => (name && !options.some(o => o.value === name) ? [{ value: name, label: t('station_not_in_net', { name }) }, ...options] : options)

  const start = points[0] ?? null
  const end = points.length >= 2 ? points[last] : null
  const setStart = (name) => setPoints(ps => (ps.length ? [{ ...ps[0], station: name, track: null }, ...ps.slice(1)] : [newPoint(name)]))
  const setEnd = (name) => setPoints(ps => (ps.length >= 2 ? [...ps.slice(0, -1), { ...ps[ps.length - 1], station: name, track: null }] : [...ps, newPoint(name)]))
  const addVia = (name) => setPoints(ps => [...ps.slice(0, -1), newPoint(name, { dwell: 30 }), ps[ps.length - 1]])
  const move = (i, by) => setPoints(ps => { const out = [...ps]; [out[i], out[i + by]] = [out[i + by], out[i]]; return out })
  const remove = (i) => setPoints(ps => ps.filter((_, j) => j !== i))

  const endRow = (kind, point, i, onStation) => {
    const station = point ? stationNamed(stations, point.station) : null
    return (
      <div className={`tt-point tt-point-${kind}`}>
        <span className="tt-point-mark" aria-hidden="true" />
        <div className="ui-fields tt-point-fields">
          <SelectField compact label={t(kind === 'start' ? 'start' : 'end')} value={point?.station ?? null} empty={t('choose_station')} disabled={ro || (kind === 'end' && !start)}
            options={withMissing(endOptions, point?.station)} onChange={(v) => v && onStation(v)} />
          {point && <SelectField compact label={t('track')} value={point.track} empty={t('track_any')} disabled={ro || !station} options={trackOptions(t, station)}
            onChange={(v) => setPoint(i, { track: v })} />}
        </div>
        {point && problemAt(i) && <p className="ui-error tt-point-problem">{t(`problem_${problemAt(i).key}`)}</p>}
      </div>
    )
  }

  return (
    <section className="tt-card">
      <h2>{t('card_route')}</h2>
      <p className="ui-muted tt-lead">{t('route_lead')}</p>
      {!ends.length && <Message tone="warn">{t('no_stations')}</Message>}
      {endRow('start', start, 0, setStart)}
      {points.slice(1, -1).map((p, k) => {
        const i = k + 1
        const station = stationNamed(stations, p.station)
        return (
          <div key={p.id} className={`tt-point tt-point-${p.stop ? 'stop' : 'pass'}`}>
            <span className="tt-point-mark" aria-hidden="true" />
            <div className="ui-fields tt-point-fields">
              <SelectField compact label={t('via')} value={p.station} disabled={ro} options={withMissing(allOptions, p.station)}
                onChange={(v) => v && setPoint(i, { station: v, track: null })} />
              <SelectField compact label={t('track')} value={p.track} empty={t('track_any')} disabled={ro || !station} options={trackOptions(t, station)}
                onChange={(v) => setPoint(i, { track: v })} />
            </div>
            <div className="tt-point-more">
              <Checkbox label={t('stops_here')} checked={p.stop} disabled={ro} onChange={(v) => setPoint(i, { stop: v })} />
              {p.stop && <NumberField compact label={t('dwell_s')} value={p.dwell} min={0} step={15} disabled={ro} onChange={(v) => setPoint(i, { dwell: v ?? 0 })} />}
              {p.stop && <ClockField t={t} label={t('waits_for')} value={p.departure} empty disabled={ro} onChange={(v) => setPoint(i, { departure: v })} />}
              {!ro && (
                <span className="ui-row tt-point-tools">
                  <IconButton label={t('move_up')} icon={<UpIcon />} variant="ghost" size="small" disabled={i <= 1} onClick={() => move(i, -1)} />
                  <IconButton label={t('move_down')} icon={<DownIcon />} variant="ghost" size="small" disabled={i >= last - 1} onClick={() => move(i, 1)} />
                  <IconButton label={t('remove')} icon={<TrashIcon />} variant="ghost" size="small" onClick={() => remove(i)} />
                </span>
              )}
            </div>
            {problemAt(i) && <p className="ui-error tt-point-problem">{t(`problem_${problemAt(i).key}`)}</p>}
          </div>
        )
      })}
      {!ro && end && (
        <div className="tt-point tt-point-add">
          <span className="tt-point-mark" aria-hidden="true" />
          <div className="ui-fields tt-point-fields">
            <SelectField compact label={t('via_add')} value={null} empty={t('choose_station')} options={allOptions} onChange={(v) => v && addVia(v)} />
          </div>
        </div>
      )}
      {endRow('end', end, last, setEnd)}
      {problems.some(p => p.key === 'too_few_points') && start && <p className="ui-muted">{t('problem_too_few_points')}</p>}
    </section>
  )
}

/** A span of time as one says it: 30 s, 4 min 10 s, 1 h 05 min. */
function duration(t, seconds) {
  const s = Math.round(seconds)
  if (s < 60) return t('span_s', { s })
  if (s < 3600) return s % 60 ? t('span_min_s', { m: Math.floor(s / 60), s: s % 60 }) : t('span_min', { m: s / 60 })
  return t('span_h_min', { h: Math.floor(s / 3600), m: String(Math.round((s % 3600) / 60)).padStart(2, '0') })
}

/** A time: hours and minutes, the seconds small after them. */
function Clock({ value }) {
  if (value == null) return null
  const s = Math.round(value)
  return <span className="tt-clock">{formatClock(s)}<small>{String(s % 60).padStart(2, '0')}</small></span>
}

/**
 * The train's times as it runs: a row for every station along its way —
 * where it stops, its arrival and departure; where it passes, the time it
 * passes, and the way to make it stop there —, what keeps it from running,
 * and its running time, length, speed and energy.
 */
function TimesCard({ t, language, train, run, stations, problems, ro, update }) {
  const stopAt = (row) => update(tr => {
    // a point it passes stops; a station it only runs through becomes one
    if (row.point != null) return { ...tr, points: tr.points.map((p, i) => (i === row.point ? { ...p, stop: true, dwell: p.dwell || 30 } : p)) }
    const at = insertionIndex(run, row.offset)
    const points = [...tr.points]
    points.splice(at, 0, newPoint(row.name, { dwell: 30, track: null }))
    return { ...tr, points }
  })
  let body
  if (problems.length) {
    body = <Message tone="warn">{problems.filter(p => p.point == null).map(p => t(`problem_${p.key}`)).join(' ') || t('problems_in_route')}</Message>
  } else if (!run) {
    body = <p className="ui-muted">{t('running')}</p>
  } else if (run.failedAt != null) {
    body = <Message tone="warn">{t('no_way', { station: train.points[run.failedAt]?.station ?? '?' })}</Message>
  } else if (run.error) {
    body = <Message tone="error">{t(`error_${run.error.code}`, { message: run.error.message })}</Message>
  } else if (run.problems) {
    body = <Message tone="warn">{t('problems_in_route')}</Message>
  }
  const rows = run?.rows
  const final = run?.result?.final
  const length = run?.request?.path.length
  const time = rows ? rows[rows.length - 1].arrival - rows[0].departure : null
  const dwell = rows ? rows.reduce((s, r) => s + (r.arrival != null && r.departure != null ? r.departure - r.arrival : 0), 0) : 0
  const gaps = run?.properties?.gaps
  const trackLabel = (row) => {
    const station = stationNamed(stations, row.name)
    return station?.tracks.find(tr => tr.id === row.track)?.label ?? station?.platforms.find(p => p.trackId === row.track)?.code ?? ''
  }
  return (
    <section className="tt-card">
      <h2>{t('card_times')}</h2>
      {body}
      {rows && !problems.length && (
        <>
          <table className="tt-table">
            <thead><tr><th>{t('station')}</th><th>{t('track_short')}</th><th>{t('arrival')}</th><th>{t('departure')}</th><th aria-label={t('stop')} /></tr></thead>
            <tbody>
              {rows.map((r, i) => (
                <tr key={`${r.key}-${i}`} className={r.stop ? 'tt-row-stop' : 'tt-row-pass'}>
                  <td title={r.name}>{r.name}</td>
                  <td>{r.stop ? trackLabel(r) : ''}</td>
                  <td>{r.stop ? <Clock value={r.arrival} /> : null}</td>
                  <td>{r.stop ? <Clock value={r.departure} /> : <span className="tt-pass"><Clock value={r.departure} /></span>}</td>
                  <td className="tt-row-end">
                    {r.stop
                      ? (r.arrival != null && r.departure != null && <span className="ui-muted">{duration(t, r.departure - r.arrival)}</span>)
                      : (!ro && <Button size="small" variant="ghost" onClick={() => stopAt(r)}>{t('stop_here')}</Button>)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <dl className="tt-facts">
            <dt>{t('running_time')}</dt><dd><strong>{duration(t, time)}</strong></dd>
            <dt>{t('distance')}</dt><dd>{formatDistance(length, language)}</dd>
            <dt>{t('mean_speed')}</dt><dd>{Math.round(length / Math.max(time - dwell, 1) * 3.6)} km/h</dd>
            <dt>{t('energy')}</dt><dd>{(final.energy_consumption / 3.6e6).toLocaleString(language, { maximumFractionDigits: 1 })} kWh</dd>
          </dl>
          {gaps && (gaps.heights.length > 0 || gaps.speed.length > 0) && (
            <Message tone="info">
              {[gaps.heights.length && t('gaps_heights', { length: formatDistance(gaps.heights.reduce((s, g) => s + g.end - g.begin, 0), language) }),
                gaps.speed.length && t('gaps_speed', { length: formatDistance(gaps.speed.reduce((s, g) => s + g.end - g.begin, 0), language) })].filter(Boolean).join(' ')}
            </Message>
          )}
        </>
      )}
    </section>
  )
}
