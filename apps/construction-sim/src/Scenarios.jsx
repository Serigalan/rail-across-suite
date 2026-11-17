import { useEffect, useMemo, useState } from 'react'
import {
  Badge, Button, Checkbox, Message, Option, Options, Panel, Section, SelectField, Shell, TextField, Workspace,
} from '@rail-across/ui'
import { recordAtStage } from '@rail-across/core/document'
import { nearestLocation, netLines as routeNet, pathLine, stationsOf, trackLabel } from '@rail-across/running-times'
import { dispatchOf, formatClock, parseClock, scenarioRows, timetableApi, trainTitle, withScenariosOfItsTrains } from '@rail-across/timetables'
import { useRuns, useScenarioRuns } from '@rail-across/timetables/runs'
import { useCatalogue } from '@rail-across/vehicles'
import PlanView from './PlanView.jsx'
import { netLines } from './model/net.js'
import {
  changedClosure, closureLabel, closureWhen, diversionOf, firstDayOf, isCancelled, isDay, newClosure, newScenario, PLAN_COLORS,
  scenariosOf, STATE_TONES, today, toggledCancel, withClosure, withDiversion, withoutClosure, withoutScenario, withScenario,
} from './model/scenario.js'

const NO_TRAINS = []
/** How far from a track a click on the plan still picks it [m]. */
const PICK_REACH = 40

/** The net of a day — the variant at the stage of that day — and its places. */
function useDayNet(record, day) {
  const dayRecord = useMemo(() => recordAtStage(record, day), [record, day])
  const stations = useMemo(() => stationsOf(dayRecord), [dayRecord])
  return { dayRecord, stations }
}

/** A clock field: what is typed kept until it reads as a time. */
function ClockField({ label, value, onChange, disabled }) {
  const [text, setText] = useState(formatClock(value))
  return <TextField compact label={label} value={text} disabled={disabled} onChange={setText} onBlur={() => { const s = parseClock(text); if (s != null) onChange(s % 86400); else setText(formatClock(value)) }} />
}

/** A duration with its sign, in minutes and seconds: "+3:20". */
const signed = (s) => {
  if (s == null || !Number.isFinite(s)) return '—'
  const r = Math.round(Math.abs(s))
  return `${s < 0 ? '−' : s > 0 ? '+' : '±'}${Math.floor(r / 60)}:${String(r % 60).padStart(2, '0')}`
}

/**
 * The construction scenarios of the Baubetriebs-Sim (decision 0014): a
 * project's timetable run on the net of a day with stretches of track closed
 * for a while, trains taken out and diverted by hand. The closures are picked
 * on the plan — a click on a track where the closure begins, one where it
 * ends —; each train says how it fares beside its regular run, its two ways
 * drawn on the plan where it is chosen. Saved with the timetable.
 */
export default function Scenarios({ t, rail, project, record }) {
  const catalogue = useCatalogue()
  const [timetables, setTimetables] = useState(null)
  const [timetableId, setTimetableId] = useState(null)
  const [draft, setDraft] = useState(null)       // the timetable chosen, as changed here
  const [saved, setSaved] = useState(null)       // as it was saved
  const [state, setState] = useState({ busy: false, error: null, conflict: false })
  const [scenarioId, setScenarioId] = useState(null)
  const [dayText, setDayText] = useState(null)
  const [picking, setPicking] = useState(null)   // null, { }, or { first: { trackId, station } }
  const [trainId, setTrainId] = useState(null)

  useEffect(() => {
    let gone = false
    timetableApi.list(project.id).then(({ timetables: list }) => {
      if (gone) return
      setTimetables(list)
      const first = list[0] ?? null
      setTimetableId(first?.id ?? null)
      setDraft(first)
      setSaved(first)
    }, () => { if (!gone) setTimetables([]) })
    return () => { gone = true }
  }, [project.id])

  const choose = (id) => {
    const tt = timetables.find(x => x.id === id) ?? null
    setTimetableId(id); setDraft(tt); setSaved(tt); setScenarioId(null); setTrainId(null)
  }
  const scenarios = scenariosOf(draft?.data)
  const scenario = scenarios.find(s => s.id === scenarioId) ?? scenarios[0] ?? null
  const trains = draft?.data?.trains ?? NO_TRAINS
  const dirty = draft !== saved
  const mayEdit = Boolean(draft?.mayEdit)
  const change = (fn) => setDraft(d => (d ? { ...d, data: withScenariosOfItsTrains(fn(d.data)) } : d))
  const changeScenario = (fn) => scenario && change(data => withScenario(data, fn(scenario)))

  const [fallback] = useState(today)
  const day = isDay(dayText) ? dayText : firstDayOf(scenario, fallback)
  const { dayRecord, stations } = useDayNet(record, day)
  const dispatch = useMemo(() => dispatchOf(draft?.data), [draft?.data])
  const regular = useRuns(dayRecord, stations, trains, catalogue, dispatch)
  const { runs, busy } = useScenarioRuns(dayRecord, stations, trains, scenario, day, catalogue, dispatch, regular)
  const rows = useMemo(() => (runs ? scenarioRows(trains, regular, runs) : null), [runs, trains, regular])
  const rowOf = new Map((rows ?? []).map(r => [r.id, r]))

  const lines = useMemo(() => netLines(dayRecord?.tracks), [dayRecord])
  const net = useMemo(() => routeNet(dayRecord), [dayRecord])
  const states = useMemo(() => new Map([...lines.keys()].map(id => [id, 'stays'])), [lines])
  const trackName = (id) => {
    const tr = record.tracks.find(x => x.id === id)
    return tr ? trackLabel(tr) : id.slice(0, 8)
  }
  const train = trains.find(x => x.id === trainId) ?? null
  const overlays = [
    ...(scenario?.closures ?? []).map(c => ({ coordinates: pathLine(net, dayRecord, [{ trackId: c.at.trackId, start: c.at.startStation, end: c.at.endStation }]), color: PLAN_COLORS.closed, width: 8 })),
    ...(train && regular.get(train.id)?.route ? [{ coordinates: pathLine(net, dayRecord, regular.get(train.id).route.path), color: PLAN_COLORS.regular, width: 5, dashed: true }] : []),
    ...(train && runs?.get(train.id)?.run?.route ? [{ coordinates: pathLine(net, dayRecord, runs.get(train.id).run.route.path), color: PLAN_COLORS.scenario, width: 4 }] : []),
  ]
  const points = picking?.first ? [{ at: pathLine(net, dayRecord, [{ trackId: picking.first.trackId, start: picking.first.station, end: picking.first.station + 0.1 }])[0], color: PLAN_COLORS.pick }].filter(p => p.at) : []
  const pick = (lngLat) => {
    const at = nearestLocation(net, lngLat, PICK_REACH)
    if (!at) return
    if (!picking.first) { setPicking({ first: at }); return }
    if (at.trackId !== picking.first.trackId || Math.abs(at.station - picking.first.station) < 1) { setPicking({ first: at }); return }
    changeScenario(sc => withClosure(sc, newClosure({ trackId: at.trackId, from: picking.first.station, to: at.station }, day)))
    setPicking(null)
  }

  const save = async () => {
    setState({ busy: true, error: null, conflict: false })
    try {
      const next = await timetableApi.save(draft.id, draft.version, draft.name, draft.data)
      setTimetables(list => list.map(x => (x.id === next.id ? next : x)))
      setDraft(next); setSaved(next)
      setState({ busy: false, error: null, conflict: false })
    } catch (err) {
      setState({ busy: false, error: err.code ?? String(err), conflict: err.code === 'stale_version' })
    }
  }
  const addScenario = () => {
    const sc = newScenario(t('scenario_new_name', { n: scenarios.length + 1 }))
    change(data => withScenario(data, sc))
    setScenarioId(sc.id)
  }

  const panel = (
    <Panel title={t('section_scenarios')} intro={t('section_scenarios_intro')}>
      {timetables && !timetables.length && <Message tone="info">{t('timetables_none')}</Message>}
      <Section title={t('timetable')}>
        <div className="ui-fields">
          <SelectField wide compact label={t('timetable')} value={timetableId} options={(timetables ?? []).map(x => ({ value: x.id, label: x.name }))} onChange={choose} disabled={dirty} />
          <SelectField wide compact label={t('scenario')} value={scenario?.id ?? null} empty={t('scenario_none')} options={scenarios.map(s => ({ value: s.id, label: s.name }))} onChange={setScenarioId} />
        </div>
        <div className="ui-row">
          <Button size="small" disabled={!draft || !mayEdit} onClick={addScenario}>{t('scenario_add')}</Button>
          {scenario && <Button size="small" variant="danger" disabled={!mayEdit} onClick={() => { change(data => withoutScenario(data, scenario.id)); setScenarioId(null) }}>{t('scenario_delete')}</Button>}
          <Button size="small" variant="primary" disabled={!dirty || !mayEdit || state.busy} onClick={save}>{t('save')}</Button>
        </div>
        {draft && !mayEdit && <p className="ui-muted">{t('read_only')}</p>}
        {state.conflict && <Message tone="warn">{t('save_conflict')}</Message>}
        {state.error && !state.conflict && <Message tone="error">{t('save_failed', { error: state.error })}</Message>}
      </Section>
      {scenario && (
        <>
          <Section title={t('scenario')}>
            <div className="ui-fields">
              <TextField wide compact label={t('scenario_name')} value={scenario.name} disabled={!mayEdit} onChange={(name) => changeScenario(sc => ({ ...sc, name: name || sc.name }))} />
              <TextField compact type="date" label={t('day')} value={day} onChange={setDayText} />
            </div>
            <p className="ui-muted">{t('day_hint')}</p>
          </Section>
          <Section title={t('closures')}>
            {(scenario.closures ?? []).map(c => (
              <div key={c.id} className="closure">
                <p><b>{closureLabel(c, trackName)}</b> <span className="ui-muted">{closureWhen(c, t)}</span></p>
                <div className="ui-fields">
                  <TextField compact label={t('closure_name')} value={c.name ?? ''} disabled={!mayEdit} onChange={(name) => changeScenario(sc => changedClosure(sc, c.id, { name: name || undefined }))} />
                  <TextField compact type="date" label={t('first_day')} value={c.firstDay} disabled={!mayEdit}
                    onChange={(v) => isDay(v) && changeScenario(sc => changedClosure(sc, c.id, { firstDay: v, lastDay: v > c.lastDay ? v : c.lastDay }))} />
                  <TextField compact type="date" label={t('last_day')} value={c.lastDay} disabled={!mayEdit}
                    onChange={(v) => isDay(v) && changeScenario(sc => changedClosure(sc, c.id, { lastDay: v, firstDay: v < c.firstDay ? v : c.firstDay }))} />
                </div>
                <div className="ui-fields">
                  <Checkbox label={t('possession')} checked={Boolean(c.window)} disabled={!mayEdit}
                    onChange={(on) => changeScenario(sc => changedClosure(sc, c.id, { window: on ? { from: 22 * 3600, to: 5 * 3600 } : null }))} />
                  {c.window && <ClockField key={`f-${c.window.from}`} label={t('window_from')} value={c.window.from} disabled={!mayEdit} onChange={(from) => changeScenario(sc => changedClosure(sc, c.id, { window: { ...c.window, from } }))} />}
                  {c.window && <ClockField key={`t-${c.window.to}`} label={t('window_to')} value={c.window.to} disabled={!mayEdit} onChange={(to) => changeScenario(sc => changedClosure(sc, c.id, { window: { ...c.window, to } }))} />}
                  <Button size="small" variant="danger" disabled={!mayEdit} onClick={() => changeScenario(sc => withoutClosure(sc, c.id))}>{t('remove')}</Button>
                </div>
              </div>
            ))}
            {!(scenario.closures ?? []).length && <p className="ui-muted">{t('closures_none')}</p>}
            <Button size="small" active={Boolean(picking)} disabled={!mayEdit} onClick={() => setPicking(p => (p ? null : {}))}>{t(picking ? 'closure_pick_cancel' : 'closure_pick')}</Button>
            {picking && <p className="ui-muted">{t(picking.first ? 'closure_pick_second' : 'closure_pick_first')}</p>}
          </Section>
          <Section title={t('trains')}>
            {busy && <p className="ui-muted">{t('running')}</p>}
            <Options>
              {trains.map(tr => {
                const row = rowOf.get(tr.id)
                return (
                  <Option key={tr.id} active={tr.id === trainId} onClick={() => setTrainId(tr.id === trainId ? null : tr.id)}
                    end={row && <Badge tone={STATE_TONES[row.state]}>{t(`state_${row.state}`)}{row.delay ? ` ${signed(row.delay)}` : ''}</Badge>}>
                    {trainTitle(tr) || t('unnamed')}
                  </Option>
                )
              })}
            </Options>
          </Section>
          {train && <TrainSection {...{ t, train, scenario, stations, mayEdit, changeScenario, row: rowOf.get(train.id), trackName }} />}
        </>
      )}
    </Panel>
  )

  return (
    <Shell>
      {rail}
      {panel}
      <Workspace>
        <div className="scenario-plan">
          <div className="stage-legend">
            <span><i style={{ background: PLAN_COLORS.closed }} />{t('legend_closed')}</span>
            <span><i style={{ background: PLAN_COLORS.regular }} />{t('legend_regular')}</span>
            <span><i style={{ background: PLAN_COLORS.scenario }} />{t('legend_scenario')}</span>
            <span className="ui-muted">{t('stage_of_day', { day })}</span>
          </div>
          <PlanView lines={lines} states={states} points={points} fitKey={0} overlays={overlays} onPick={picking ? pick : null} />
        </div>
        {scenario && rows && <Results t={t} trains={trains} rows={rows} chosen={trainId} choose={setTrainId} />}
      </Workspace>
    </Shell>
  )
}

/** What a scenario does to the train chosen: taken out or not, diverted by hand on its points. */
function TrainSection({ t, train, scenario, stations, mayEdit, changeScenario, row, trackName }) {
  const cancelled = isCancelled(scenario, train.id)
  const diversion = diversionOf(scenario, train.id)
  const points = diversion?.points ?? null
  const setPoints = (fn) => changeScenario(sc => withDiversion(sc, train.id, fn(points)))
  const placeOptions = stations.filter(s => s.kind !== 'end').map(s => ({ value: s.id, label: s.name }))
  const placeName = (p) => stations.find(s => s.id === (p.operationalPointId ?? p.endMarkId))?.name ?? '?'
  const trackOptions = (p) => {
    const s = stations.find(x => x.id === (p.operationalPointId ?? p.endMarkId))
    return [...(s?.tracks ?? []).map(tr => tr.id), ...(s?.platforms ?? []).map(pf => pf.trackId)]
      .filter((id, i, all) => all.indexOf(id) === i).map(id => ({ value: id, label: trackName(id) }))
  }
  return (
    <Section title={trainTitle(train) || t('unnamed')}>
      {row && <p className="ui-muted">{t(`state_${row.state}_long`, { point: row.failedAt != null ? placeName((points ?? train.points)[row.failedAt] ?? {}) : '' })}</p>}
      <div className="ui-row">
        <Button size="small" disabled={!mayEdit} onClick={() => changeScenario(sc => toggledCancel(sc, train.id))}>{t(cancelled ? 'train_put_back' : 'train_take_out')}</Button>
        {!cancelled && !points && <Button size="small" disabled={!mayEdit} onClick={() => setPoints(() => structuredClone(train.points))}>{t('train_divert')}</Button>}
        {points && <Button size="small" disabled={!mayEdit} onClick={() => changeScenario(sc => withDiversion(sc, train.id, null))}>{t('train_divert_drop')}</Button>}
      </div>
      {points && !cancelled && (
        <div className="diversion">
          <p className="ui-muted">{t('diversion_hint')}</p>
          {points.map((p, i) => {
            const end = i === 0 || i === points.length - 1
            return (
              <div key={p.id ?? i} className="ui-fields">
                {end
                  ? <span className="diversion-end">{placeName(p)}</span>
                  : <SelectField compact label={t('via')} value={p.operationalPointId ?? null} options={placeOptions} disabled={!mayEdit}
                    onChange={(id) => setPoints(ps => ps.map((x, j) => (j === i ? { ...x, operationalPointId: id, endMarkId: undefined, trackId: null } : x)))} />}
                <SelectField compact label={t('track')} value={p.trackId ?? null} empty={t('track_any')} options={trackOptions(p)} disabled={!mayEdit}
                  onChange={(trackId) => setPoints(ps => ps.map((x, j) => (j === i ? { ...x, trackId } : x)))} />
                {!end && <Checkbox label={t('stops')} checked={Boolean(p.stop)} disabled={!mayEdit} onChange={(stop) => setPoints(ps => ps.map((x, j) => (j === i ? { ...x, stop } : x)))} />}
                {!end && <Button size="small" variant="ghost" disabled={!mayEdit} onClick={() => setPoints(ps => ps.filter((_, j) => j !== i))}>{t('remove')}</Button>}
              </div>
            )
          })}
          <SelectField compact label={t('via_add')} value={null} empty={t('via_choose')} options={placeOptions} disabled={!mayEdit}
            onChange={(id) => id && setPoints(ps => [...ps.slice(0, -1), { id: globalThis.crypto?.randomUUID?.() ?? `${Date.now()}`, operationalPointId: id, trackId: null, stop: false, dwell: 30, departure: null, itineraryId: null }, ps[ps.length - 1]])} />
        </div>
      )}
    </Section>
  )
}

/** The scenario's trains beside their regular runs. */
function Results({ t, trains, rows, chosen, choose }) {
  const title = new Map(trains.map(tr => [tr.id, trainTitle(tr) || t('unnamed')]))
  const clock = (s) => (s == null ? '—' : formatClock(s))
  const counts = rows.reduce((m, r) => ({ ...m, [r.state]: (m[r.state] ?? 0) + 1 }), {})
  return (
    <div className="results">
      <p className="results-sum">{['regular', 'diverted', 'by_hand', 'not_runnable', 'cancelled'].filter(s => counts[s]).map(s => `${t(`state_${s}`)}: ${counts[s]}`).join(' · ')}</p>
      <table className="results-table">
        <thead><tr><th>{t('train')}</th><th>{t('state')}</th><th className="num">{t('arrival_regular')}</th><th className="num">{t('arrival_scenario')}</th><th className="num">{t('delay')}</th><th className="num">{t('running_time_change')}</th><th className="num">{t('conflicts')}</th></tr></thead>
        <tbody>{rows.map(r => (
          <tr key={r.id} className={r.id === chosen ? 'chosen' : ''} onClick={() => choose(r.id === chosen ? null : r.id)}>
            <td>{title.get(r.id)}</td><td><Badge tone={STATE_TONES[r.state]}>{t(`state_${r.state}`)}</Badge></td>
            <td className="num">{clock(r.regularArrival)}</td><td className="num">{clock(r.arrival)}</td><td className="num">{signed(r.delay)}</td>
            <td className="num">{r.runningTime != null && r.regularRunningTime != null ? signed(r.runningTime - r.regularRunningTime) : '—'}</td>
            <td className="num">{r.conflicts || (r.waits ? t('waits_n', { n: r.waits }) : '—')}</td>
          </tr>
        ))}</tbody>
      </table>
    </div>
  )
}
