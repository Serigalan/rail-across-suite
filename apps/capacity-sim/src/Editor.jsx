import { useEffect, useMemo, useState } from 'react'
import {
  Button, LANGUAGE_NAMES, LANGUAGES, Message, NumberField, Option, Options, Panel, Rail, RailBrand, RailButton, RailDivider, Section, SelectField,
  Shell, TextField, Workspace,
} from '@rail-across/ui'
import { stationsOf } from '@rail-across/running-times'
import { dispatchOf, formatClock, parseClock, timetableApi, trainTitle } from '@rail-across/timetables'
import { useRuns } from '@rail-across/timetables/runs'
import { useCatalogue } from '@rail-across/vehicles'
import { analyseStretch, BOUNDS, proposedStretches, recordAtStage, stagesOf, stationOccupancy } from '@rail-across/capacity'
import { useTexts } from './i18n.js'
import { api } from './api.js'
import { projectsUrl } from './suite.js'
import Diagram from './Diagram.jsx'
import ConflictPlan from './ConflictPlan.jsx'
import { planMarks } from './plan.js'
import { busiestHour, minutes, percent, periodText, resourceLabel, stageName } from './study.js'
import { HomeIcon, LogoIcon, SettingsIcon, StationIcon, StretchIcon, StudyIcon } from './icons.jsx'

const NO_TRAINS = []
const NO_RUNS = new Map()

/** The heads of the variants asked for, loaded as they are: Map id → record. */
function useHeads(ids) {
  const [heads, setHeads] = useState(() => new Map())
  const [asked] = useState(() => new Set())
  const key = ids.filter(Boolean).join('|')
  useEffect(() => {
    for (const id of key.split('|').filter(Boolean)) {
      if (asked.has(id)) continue
      asked.add(id)
      api.head(id).then(h => setHeads(m => new Map(m).set(id, h.record)), () => asked.delete(id))
    }
  }, [key, asked])
  return heads
}

/** One side of a study: a variant at a stage, its trains run over it, each stretch analysed. */
function useSide(head, stage, trains, catalogue, dispatch, options) {
  const record = useMemo(() => (head ? recordAtStage(head, stage) : null), [head, stage])
  const stations = useMemo(() => (record ? stationsOf(record) : []), [record])
  const runs = useRuns(record, stations, record ? trains : NO_TRAINS, catalogue, dispatch)
  return { record, stations, runs: record ? runs : NO_RUNS, options }
}

/**
 * The Kapazitäts-Sim (roadmap, phase 4, K1): a project's timetable run over a
 * variant at a stage, as the Fahrplan-Editor runs it, and from the trains'
 * blocking times its stretches compressed after UIC Code 406 — occupancy time
 * and rate against the recommended bound, the trains that set it, the minimum
 * headways by kind — and its stations' tracks as long as they are held; a
 * second variant or stage beside it. It only reads: nothing here is saved,
 * the results are derived (decision 0023, point 4).
 */
export default function Editor({ project, variants, initialVariant }) {
  const { t, language, setLanguage } = useTexts()
  const [section, setSection] = useState('stretches')
  const [chosen, setChosen] = useState(null)
  const [compressed, setCompressed] = useState(true)
  const [timetables, setTimetables] = useState(null)
  const [timetableId, setTimetableId] = useState(null)
  const [study, setStudy] = useState({ variantId: initialVariant?.id, stage: 'final', compareId: null, compareStage: 'existing', line: 'mixed', perTrain: 0, fixed: 0, period: null })
  const set = (patch) => setStudy(s => ({ ...s, ...patch }))
  const catalogue = useCatalogue()

  useEffect(() => {
    let gone = false
    timetableApi.list(project.id).then(({ timetables: list }) => {
      if (gone) return
      setTimetables(list)
      setTimetableId(list[0]?.id ?? null)
    }, () => { if (!gone) setTimetables([]) })
    return () => { gone = true }
  }, [project.id])

  const timetable = timetables?.find(tt => tt.id === timetableId) ?? null
  const trains = timetable?.data?.trains ?? NO_TRAINS
  const dispatch = useMemo(() => dispatchOf(timetable?.data), [timetable])
  const period = study.period ?? busiestHour(trains)
  const options = useMemo(() => ({ from: period.from, to: period.to, line: study.line, additional: { perTrain: study.perTrain || 0, fixed: study.fixed || 0 } }),
    [period.from, period.to, study.line, study.perTrain, study.fixed])

  const heads = useHeads([study.variantId, study.compareId])
  const a = useSide(heads.get(study.variantId), study.stage, trains, catalogue, dispatch, options)
  const b = useSide(study.compareId ? heads.get(study.compareId) : null, study.compareStage, trains, catalogue, dispatch, options)

  const stretches = useMemo(() => proposedStretches(a.stations, trains, a.runs), [a.stations, trains, a.runs])
  const analysed = (side) => new Map(stretches.map(s => [s.id, side.record ? analyseStretch(s, trains, side.runs, side.options) : null]))
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const analysesA = useMemo(() => analysed(a), [stretches, trains, a.runs, a.options])
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const analysesB = useMemo(() => analysed(b), [stretches, trains, b.runs, b.options, b.record])
  const titles = useMemo(() => new Map(trains.map(tr => [tr.id, trainTitle(tr) || t('unnamed')])), [trains, t])
  const stations = useMemo(() => {
    const seen = new Set([...a.runs.values()].flatMap(r => (r?.rows ?? []).map(row => row.key)))
    return a.stations.filter(s => s.kind === 'station' && s.tracks.length >= 2 && seen.has(s.key))
  }, [a.stations, a.runs])

  const run = [...a.runs.values()]
  const done = run.filter(r => r?.blocks).length
  const failed = run.filter(r => r && (r.problems || r.error || r.failedAt != null)).length
  const sideName = (id, stage) => `${variants.find(v => v.id === id)?.name ?? '—'} · ${stageName(stage, t, language)}`
  const trackName = (id) => a.record?.tracks?.find(tr => tr.id === id)?.name ?? id?.slice(0, 8) ?? '—'

  const panel = section === 'study'
    ? <StudyPanel {...{ t, language, study, set, period, timetables, timetableId, setTimetableId, variants, heads }} />
    : section === 'stations'
      ? <ListPanel title={t('section_stations')} intro={t('section_stations_intro')} empty={t('stations_none')}
        items={stations.map(s => ({ id: s.key, label: s.name, icon: <StationIcon />, end: `${s.tracks.length}` }))} chosen={chosen} choose={setChosen} />
      : section === 'settings'
        ? <Panel title={t('section_settings')}>
          <Section title={t('language')}>
            <SelectField compact label={t('language')} value={language} options={LANGUAGES.map(l => ({ value: l, label: LANGUAGE_NAMES[l] }))} onChange={setLanguage} />
          </Section>
        </Panel>
        : <ListPanel title={t('section_stretches')} intro={t('section_stretches_intro')} empty={t('stretches_none')}
          items={stretches.map(s => {
            const ra = analysesA.get(s.id), rb = analysesB.get(s.id)
            const rate = (r) => <span className={`cap-rate${r && r.rateWithAdditional > r.bound ? ' over' : ''}`}>{percent(r?.rateWithAdditional, language)}</span>
            return { id: s.id, label: `${s.fromName} – ${s.toName}`, icon: <StretchIcon />, end: <>{rate(ra)}{study.compareId && <> · {rate(rb)}</>}</> }
          })} chosen={chosen} choose={setChosen} />

  const stretch = section === 'stretches' ? stretches.find(s => s.id === chosen) ?? null : null
  const station = section === 'stations' ? stations.find(s => s.key === chosen) ?? null : null

  return (
    <Shell>
      <Rail label={t('tool')}
        brand={<RailBrand href={projectsUrl()} label={t('to_projects')} name={t('tool')}><LogoIcon /></RailBrand>}
        top={<>
          <RailButton icon={<StretchIcon />} label={t('section_stretches')} pressed={section === 'stretches'} onClick={() => { setSection('stretches'); setChosen(null) }} />
          <RailButton icon={<StationIcon />} label={t('section_stations')} pressed={section === 'stations'} onClick={() => { setSection('stations'); setChosen(null) }} />
          <RailButton icon={<StudyIcon />} label={t('section_study')} pressed={section === 'study'} onClick={() => setSection('study')} />
        </>}
        bottom={<>
          <RailDivider />
          <RailButton icon={<HomeIcon />} label={t('to_projects')} onClick={() => location.assign(projectsUrl())} />
          <RailButton icon={<SettingsIcon />} label={t('section_settings')} pressed={section === 'settings'} onClick={() => setSection('settings')} />
        </>}
      />
      {panel}
      <Workspace>
        <div className="cap-page">
          <div className="cap-head">
            <h1>{project.title}</h1>
            <span className="ui-muted">{timetable?.name ?? '—'} · {periodText(period)} · {sideName(study.variantId, study.stage)}
              {study.compareId && <> · {t('compare')}: {sideName(study.compareId, study.compareStage)}</>}</span>
          </div>
          {timetables && !timetables.length && <Message tone="info">{t('timetables_none')}</Message>}
          {trains.length > 0 && done < trains.length - failed && <Message tone="info">{t('running', { n: done, m: trains.length })}</Message>}
          {failed > 0 && <Message tone="warn">{t('run_problems', { n: failed })}</Message>}
          {stretch && (
            <StretchView {...{ t, language, stretch, titles, compressed, setCompressed, trackName }} side={a} a={analysesA.get(stretch.id)}
              b={study.compareId ? analysesB.get(stretch.id) : null} names={[sideName(study.variantId, study.stage), sideName(study.compareId, study.compareStage)]} />
          )}
          {station && (
            <div className="cap-sides">
              <StationView t={t} language={language} name={sideName(study.variantId, study.stage)} trackName={trackName}
                held={stationOccupancy(station, trains, a.runs, a.record, period)} />
              {study.compareId && b.record && (
                <StationView t={t} language={language} name={sideName(study.compareId, study.compareStage)} trackName={trackName}
                  held={stationOccupancy(station, trains, b.runs, b.record, period)} />
              )}
            </div>
          )}
          {!stretch && !station && section === 'stretches' && stretches.length > 0 && (
            <Overview {...{ t, language, stretches, analysesA, analysesB, compare: Boolean(study.compareId), choose: setChosen }} />
          )}
        </div>
      </Workspace>
    </Shell>
  )
}

function ListPanel({ title, intro, empty, items, chosen, choose }) {
  return (
    <Panel title={title} intro={intro}>
      {items.length
        ? <Options>{items.map(i => <Option key={i.id} icon={i.icon} end={i.end} active={i.id === chosen} onClick={() => choose(i.id === chosen ? null : i.id)}>{i.label}</Option>)}</Options>
        : <p className="ui-list-empty">{empty}</p>}
    </Panel>
  )
}

/** A clock field: what is typed kept until it reads as a time (keyed by its value, so a new one starts anew). */
function ClockField({ label, value, onChange }) {
  const [text, setText] = useState(formatClock(value))
  return <TextField compact label={label} value={text} onChange={setText} onBlur={() => { const s = parseClock(text); if (s != null) onChange(s); else setText(formatClock(value)) }} />
}

function StudyPanel({ t, language, study, set, period, timetables, timetableId, setTimetableId, variants, heads }) {
  const variantOptions = variants.map(v => ({ value: v.id, label: v.name }))
  // The stages each variant has: the existing state, every day something is put into or taken out of service, the final.
  const stageOptions = (id) => {
    const listed = stagesOf(heads.get(id) ?? { id: 'none' })
    return (listed.length ? listed : [{ key: 'existing' }, { key: 'final' }]).map(s => ({ value: s.key, label: stageName(s.key, t, language) }))
  }
  return (
    <Panel title={t('section_study')} intro={t('section_study_intro')}>
      <Section title={t('timetable')}>
        <div className="ui-fields">
          <SelectField wide compact label={t('timetable')} value={timetableId} options={(timetables ?? []).map(tt => ({ value: tt.id, label: tt.name }))}
            onChange={(id) => { setTimetableId(id); set({ period: null }) }} />
          <ClockField key={`from-${period.from}`} label={t('period_from')} value={period.from} onChange={(from) => set({ period: { from, to: Math.max(period.to, from + 60) } })} />
          <ClockField key={`to-${period.to}`} label={t('period_to')} value={period.to} onChange={(to) => set({ period: { from: period.from, to: Math.max(to, period.from + 60) } })} />
          <SelectField wide compact label={t('line_kind')} value={study.line} options={BOUNDS.map(b => ({ value: b.line, label: t(`line_${b.line}`) }))} onChange={(line) => set({ line })} />
          <NumberField compact label={t('additional_per_train')} value={study.perTrain} onChange={(v) => set({ perTrain: v ?? 0 })} />
          <NumberField compact label={t('additional_fixed')} value={study.fixed} onChange={(v) => set({ fixed: v ?? 0 })} />
        </div>
        <p className="ui-muted">{t('additional_hint')}</p>
      </Section>
      <Section title={t('side_a')}>
        <div className="ui-fields">
          <SelectField wide compact label={t('variant')} value={study.variantId} options={variantOptions} onChange={(variantId) => set({ variantId })} />
          <SelectField compact label={t('stage')} value={study.stage} options={stageOptions(study.variantId)} onChange={(stage) => set({ stage })} />
        </div>
      </Section>
      <Section title={t('side_b')}>
        <div className="ui-fields">
          <SelectField wide compact label={t('variant')} value={study.compareId} empty={t('compare_none')} options={variantOptions} onChange={(compareId) => set({ compareId })} />
          <SelectField compact label={t('stage')} value={study.compareStage} options={stageOptions(study.compareId)} onChange={(compareStage) => set({ compareStage })} />
        </div>
      </Section>
    </Panel>
  )
}

function Figures({ t, language, r, name }) {
  if (!r) return null
  const over = r.rateWithAdditional > r.bound
  return (
    <div>
      {name && <p className="ui-muted">{name}</p>}
      <div className="cap-figures">
        <div className="cap-figure">{t('occupancy')}<b>{minutes(r.occupancy)} {t('min')}</b></div>
        <div className="cap-figure">{t('rate')}<b>{percent(r.rate, language)}</b></div>
        <div className="cap-figure">{t('rate_with_additional')}<b className={over ? 'over' : ''}>{percent(r.rateWithAdditional, language)}</b>{over && t('over_bound')}</div>
        <div className="cap-figure">{t('bound')}<b>{percent(r.bound, language)}</b></div>
      </div>
    </div>
  )
}

function StretchView({ t, language, stretch, a, b, names, titles, compressed, setCompressed, side, trackName }) {
  const marks = useMemo(() => planMarks(a, side.record), [a, side.record])
  if (!a) return null
  const signals = new Map((side.record?.signalling?.collections?.signals ?? []).map(s => [s.id, s]))
  const signalName = (id) => (id ? signals.get(id)?.name || id.slice(0, 8) : t('block_open'))
  const blockText = (b) => (b ? `${signalName(b.signalId)} → ${signalName(b.targetId)}` : '—')
  return (
    <>
      <h2>{stretch.fromName} – {stretch.toName} <span className="ui-muted">{t('stretch_trains', { n: a.trains.length })}</span></h2>
      <div className="cap-sides">
        <Figures t={t} language={language} r={a} name={b ? names[0] : null} />
        {b && <Figures t={t} language={language} r={b} name={names[1]} />}
      </div>
      {a.critical.length > 0 && <p><b>{t('critical')}:</b> {a.critical.map(id => titles.get(id) ?? id).join(' → ')}</p>}
      <div className="cap-diagram">
        <div className="ui-row">
          <b>{t('diagram')}</b>
          <Button size="small" active={compressed} onClick={() => setCompressed(true)}>{t('diagram_compressed')}</Button>
          <Button size="small" active={!compressed} onClick={() => setCompressed(false)}>{t('diagram_as_timetabled')}</Button>
        </div>
        <Diagram analysis={a} compressed={compressed} fromName={stretch.fromName} toName={stretch.toName} />
      </div>
      <h3>{t('plan')}</h3>
      <ConflictPlan record={side.record} stations={side.stations} marks={marks} t={t} />
      <h3>{t('binding')}</h3>
      {marks.binding.length ? (
        <table className="cap-table">
          <thead><tr><th className="num">#</th><th>{t('binding_trains')}</th><th>{t('resource')}</th><th>{t('binding_block')}</th><th className="num">{t('binding_at')}</th></tr></thead>
          <tbody>{marks.binding.map(b => (
            <tr key={b.after}>
              <td className="num">{b.n}</td><td>{titles.get(b.before) ?? b.before} → {titles.get(b.after) ?? b.after}</td>
              <td>{resourceLabel(b.key, trackName, t)}</td><td>{blockText(b.beforeBlock)} / {blockText(b.block)}</td>
              <td className="num">{formatClock(b.at, { seconds: true })}</td>
            </tr>
          ))}</tbody>
        </table>
      ) : <p className="ui-muted">{t('binding_none')}</p>}
      <h3>{t('areas')}</h3>
      {marks.areas.length ? (
        <table className="cap-table">
          <thead><tr><th>{t('resource')}</th><th className="num">{t('stretch_trains', { n: '' }).trim()}</th><th className="num">{t('held')}</th><th className="num">{t('rate')}</th></tr></thead>
          <tbody>{(a.areas ?? []).map(r => (
            <tr key={r.resource}><td>{resourceLabel(r.resource, trackName, t)}</td><td className="num">{r.trains.length}</td><td className="num">{minutes(r.held)}</td><td className="num">{percent(r.rate, language)}</td></tr>
          ))}</tbody>
        </table>
      ) : <p className="ui-muted">{t('areas_none')}</p>}
      <h3>{t('headways')}</h3>
      {a.headways.length ? (
        <table className="cap-table">
          <thead><tr><th>{t('headway_before')}</th><th>{t('headway_after')}</th><th className="num">{t('headway_least')}</th><th className="num">{t('headway_most')}</th><th className="num">{t('headway_pairs')}</th></tr></thead>
          <tbody>{a.headways.map(h => (
            <tr key={`${h.before}-${h.after}`}><td>{h.before || '—'}</td><td>{h.after || '—'}</td><td className="num">{minutes(h.least)}</td><td className="num">{minutes(h.most)}</td><td className="num">{h.pairs}</td></tr>
          ))}</tbody>
        </table>
      ) : <p className="ui-muted">{t('headways_none')}</p>}
    </>
  )
}

function Overview({ t, language, stretches, analysesA, analysesB, compare, choose }) {
  const cell = (r) => <td className={`num cap-rate${r && r.rateWithAdditional > r.bound ? ' over' : ''}`}>{percent(r?.rateWithAdditional, language)}</td>
  return (
    <table className="cap-table">
      <thead><tr><th>{t('section_stretches')}</th><th className="num">{t('stretch_trains', { n: '' }).trim()}</th><th className="num">{t('occupancy')}</th><th className="num">{t('rate_with_additional')}</th>{compare && <th className="num">{t('side_b')}</th>}<th className="num">{t('bound')}</th></tr></thead>
      <tbody>{stretches.map(s => {
        const r = analysesA.get(s.id)
        return (
          <tr key={s.id} onClick={() => choose(s.id)} style={{ cursor: 'pointer' }}>
            <td>{s.fromName} – {s.toName}</td><td className="num">{r?.trains.length ?? 0}</td><td className="num">{minutes(r?.occupancy)}</td>
            {cell(r)}{compare && cell(analysesB.get(s.id))}<td className="num">{percent(r?.bound, language)}</td>
          </tr>
        )
      })}</tbody>
    </table>
  )
}

function StationView({ t, language, name, held, trackName }) {
  return (
    <div>
      <p className="ui-muted">{name}</p>
      <table className="cap-table">
        <thead><tr><th>{t('resource')}</th><th className="num">{t('held')}</th><th className="num">{t('rate')}</th><th className="num">{t('stretch_trains', { n: '' }).trim()}</th></tr></thead>
        <tbody>{held.map(r => (
          <tr key={r.resource}><td>{resourceLabel(r.resource, trackName, t)}</td><td className="num">{minutes(r.held)}</td><td className="num">{percent(r.rate, language)}</td><td className="num">{r.trains.length}</td></tr>
        ))}</tbody>
      </table>
    </div>
  )
}
