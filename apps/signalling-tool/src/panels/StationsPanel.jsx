import { useMemo, useState } from 'react'
import { Button, ConfirmDialog, GroupedList, Option, Options, OptionsHeading, Panel, PlusIcon, Section, SelectField, TextField } from '@rail-across/ui'
import { designationOf } from '../model/domain.js'
import {
  boundaryOf, entryMarks, nameTaken, newStation, stationName, stationsOf, withBoundary, withPlatforms, withSignals, withStation, withoutStation,
} from '../model/stations.js'
import { signalOptions as signalChoices, trackName } from '../model/places.js'
import { FUNCTION, word } from '../labels.js'
import { StationIcon, StopIcon } from '../icons.jsx'
import { CheckList, EditorHead, ObjectFindings } from './common.jsx'
import { StoppingPlaceEditor, StoppingPlacesSection } from './StoppingPlaces.jsx'

const byLabel = (a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true })

const designation = designationOf

/**
 * The stations, with a menu as the other panels: creating one — a point of
 * the core's operational point register, its name and RL100 code —, and the
 * list of the register's stations, each opened in its editor: its name and
 * code, its signals (the others are of the open line), its platforms from
 * the Layout-Tool, its boundary towards the open line — its entry signals
 * or boards Ne 1, else its entry switches (EBO §4) —, and its stopping
 * places, on its tracks or at its platforms. A station the tracks name is
 * named and deleted in the Layout-Tool, where its tracks are. The open line
 * has stopping places too, at the platforms of no station.
 */
export default function StationsPanel({ domain, net, core, findings, selection, select, ops, mode, setMode, readOnly, t, language }) {
  const [page, setPage] = useState('menu')
  const stations = useMemo(() => stationsOf(core), [core])
  const register = useMemo(() => core.collections.operationalPoints ?? [], [core])
  const back = { label: t('back'), onClick: () => setPage('menu') }
  const chosenId = selection?.collection === 'operationalPoints' ? selection.id
    : selection?.collection === 'stationBoundaries' ? boundaryById(domain, selection.id)?.operationalPointId : null
  const chosen = chosenId ? stations.get(chosenId) : null
  const place = selection?.collection === 'stoppingPlaces' ? domain.collections.stoppingPlaces.find(h => h.id === selection.id) : null

  if (place) {
    const toOwner = place.operationalPointId && stations.has(place.operationalPointId)
      ? { label: stationName(stations.get(place.operationalPointId).point), onClick: () => select({ collection: 'operationalPoints', id: place.operationalPointId }) }
      : { label: t('back'), onClick: () => { select(null); setPage('openLine') } }
    return (
      <Panel title={t('object_stoppingPlaces')} back={toOwner}>
        <StoppingPlaceEditor key={place.id} place={place} domain={domain} net={net} core={core} findings={findings} ops={ops} mode={mode} setMode={setMode}
          readOnly={readOnly} t={t} />
      </Panel>
    )
  }

  if (chosen) {
    return (
      <Panel title={t('object_operationalPoints')} back={{ label: t('back'), onClick: () => select(null) }}>
        <StationEditor key={chosen.point.id} entry={chosen} stations={stations} register={register} domain={domain} net={net} core={core}
          findings={findings} ops={ops} select={select} mode={mode} setMode={setMode} readOnly={readOnly} t={t} language={language} />
      </Panel>
    )
  }

  if (page === 'create') {
    return <Panel title={t('create_station')} intro={t('create_station_intro')} back={back}><CreatePage register={register} ops={ops} t={t} /></Panel>
  }

  if (page === 'openLine') {
    return (
      <Panel title={t('stopping_places_open')} intro={t('stopping_places_open_intro')} back={back}>
        <StoppingPlacesSection entry={null} domain={domain} net={net} core={core} ops={ops} select={select} mode={mode} setMode={setMode} readOnly={readOnly} t={t} />
      </Panel>
    )
  }

  if (page === 'list') {
    const signalsOf = (id) => domain.collections.signals.filter(s => s.operationalPointId === id).length
    const items = [...stations.values()].map(({ point, platforms }) => ({
      id: point.id, label: stationName(point),
      sub: [point.name && point.rl100, t('station_counts', { signals: signalsOf(point.id), platforms: platforms.length })].filter(Boolean).join(' · '),
    })).sort(byLabel)
    return (
      <Panel title={t('stations')} intro={t('stations_intro')} back={back}>
        <GroupedList label={t('stations')} open searchLabel={t('search_objects')} noneFound={t('search_none')} empty={t('stations_none')}
          groups={[{ key: 'stations', title: t('stations'), items }]}
          onSelect={(id) => select({ collection: 'operationalPoints', id })} />
      </Panel>
    )
  }

  return (
    <Panel title={t('section_stations')} intro={t('section_stations_intro')}>
      <Options>
        <OptionsHeading>{t('stations')}</OptionsHeading>
        <Option icon={<StationIcon />} end={stations.size} onClick={() => setPage('list')}>{t('stations')}</Option>
        {!readOnly && <Option icon={<PlusIcon size={22} />} onClick={() => setPage('create')}>{t('create_station')}</Option>}
      </Options>
      <Options>
        <OptionsHeading>{t('open_line')}</OptionsHeading>
        <Option icon={<StopIcon />} end={domain.collections.stoppingPlaces.filter(h => !h.operationalPointId).length} onClick={() => setPage('openLine')}>{t('stopping_places_open')}</Option>
      </Options>
    </Panel>
  )
}

const boundaryById = (domain, id) => domain.collections.stationBoundaries.find(b => b.id === id)

/** A new station: its name, which no other has, and its RL100 code where there is one. It opens in its editor. */
function CreatePage({ register, ops, t }) {
  const [name, setName] = useState('')
  const [rl100, setRl100] = useState('')
  const taken = Boolean(name.trim()) && nameTaken(register, name)
  return <>
    <div className="ui-fields">
      <TextField compact wide label={`${t('name')} *`} value={name} onChange={setName} autoFocus />
      <TextField compact label={t('rl100')} value={rl100} onChange={setRl100} />
    </div>
    {taken && <p className="ui-message ui-message-warn">{t('station_exists')}</p>}
    <div className="ui-row">
      <Button variant="primary" icon={<PlusIcon size={14} />} disabled={!name.trim() || taken}
        onClick={() => ops.add('operationalPoints', newStation({ name, rl100 }))}>{t('create_station')}</Button>
    </div>
  </>
}

/**
 * A text that is taken once it is left (or Enter is pressed), not with every
 * key: a name half typed would be another station's for a moment. Keyed by
 * its value, it starts anew where an undo changes it.
 */
function CommitField({ label, value, onCommit, check = () => null, disabled, wide }) {
  const [text, setText] = useState(value ?? '')
  const problem = text.trim() !== (value ?? '') ? check(text.trim()) : null
  const commit = () => { if (!problem && text.trim() !== (value ?? '')) onCommit(text.trim()) }
  return <>
    <TextField compact wide={wide} label={label} value={text} disabled={disabled} onChange={setText} onBlur={commit}
      onKeyDown={(e) => { if (e.key === 'Enter') commit() }} />
    {problem && <p className="ui-wide ui-message ui-message-warn">{problem}</p>}
  </>
}

/** A station: its name and code, its signals, its platforms and its boundary. */
function StationEditor({ entry, stations, register, domain, net, core, findings, ops, select, mode, setMode, readOnly, t, language }) {
  const [asking, setAsking] = useState(false)
  const { point, tracks, platforms } = entry
  const byTracks = tracks.length > 0
  const { signals } = domain.collections
  const stationOf = (id) => (id && id !== point.id ? stations.get(id)?.point : null)
  const boundary = boundaryOf(domain, point.id)
  const marks = boundary?.marks ?? []
  const save = (patch) => ops.replaceAll(withStation(domain, { ...point, ...patch }, platforms))
  const setMarks = (next) => ops.replaceAll(withBoundary(domain, point.id, next))

  // Every real signal by its designation and function, those of another station saying whose.
  const signalOptions = signals.filter(s => s.kind === 'real').map(s => {
    const other = stationOf(s.operationalPointId)
    const fn = s.logicalSignals?.[0]?.function ?? s.function
    return { value: s.id, label: [designation(s, t), s.at?.anchor === 'station' ? trackName(net, s.at.trackId) : null, fn ? word(FUNCTION, fn, language) : null, other ? `(${stationName(other)})` : null].filter(Boolean).join(' · ') }
  }).sort(byLabel)
  const mine = signals.filter(s => s.operationalPointId === point.id)

  // Every platform, where it lies, those of another station saying whose.
  const platformOf = new Map([...stations.values()].flatMap(st => st.platforms.map(id => [id, st.point])))
  const platformOptions = domain.collections.platforms.map(pf => {
    const other = platformOf.get(pf.id)
    const where = t('platform_at', { track: trackName(net, pf.trackId), from: Math.round(pf.startStation), to: Math.round(pf.endStation) })
    return { value: pf.id, label: `${where}${other && other.id !== point.id ? ` (${stationName(other)})` : ''}` }
  }).sort(byLabel)

  const markText = (m) => {
    if (m.signalId) return `${t('signal')} ${designation(signals.find(s => s.id === m.signalId), t)}`
    if (m.switchId) {
      const sw = net.switches.find(x => x.switchId === m.switchId)
      return `${t('switch')} ${sw?.name ?? sw?.label ?? m.switchId.slice(0, 8)}`
    }
    return t('object_platforms')
  }
  const proposal = entryMarks(domain, point.id)
  const unmarked = (key) => (o) => !marks.some(m => m[key] === o.value)

  return (
    <div className="tool-editor">
      <EditorHead title={stationName(point)} readOnly={readOnly} t={t} onDelete={byTracks ? null : () => setAsking(true)} />
      {boundary && <ObjectFindings findings={findings} collection="stationBoundaries" id={boundary.id} t={t} />}
      <div className="ui-fields">
        <CommitField key={`name-${point.name}`} wide label={`${t('name')} *`} value={point.name} disabled={readOnly || byTracks} onCommit={(name) => save({ name })}
          check={(v) => (!v ? t('station_name_missing') : nameTaken(register, v, point.id) ? t('station_exists') : null)} />
        <CommitField key={`rl100-${point.rl100}`} label={t('rl100')} value={point.rl100} disabled={readOnly} onCommit={(rl100) => save({ rl100: rl100 || undefined })} />
      </div>
      {byTracks && <p className="ui-muted">{t('station_named_by_tracks', { n: tracks.length })}</p>}

      <Section title={t('station_signals')}>
        <p className="ui-muted">{t('station_signals_hint')}</p>
        {signalOptions.length
          ? <CheckList options={signalOptions} value={mine.map(s => s.id)} readOnly={readOnly} onChange={(ids) => ops.replaceAll(withSignals(domain, point.id, ids))} />
          : <p className="ui-list-empty">{t('station_signals_none')}</p>}
      </Section>

      <Section title={t('station_platforms')}>
        <p className="ui-muted">{t('station_platforms_hint')}</p>
        {platformOptions.length
          ? <CheckList options={platformOptions} value={platforms} readOnly={readOnly || !point.name}
            onChange={(ids) => ops.replaceAll(withPlatforms(domain, point, ids, platforms))} />
          : <p className="ui-list-empty">{t('station_platforms_none')}</p>}
      </Section>

      <StoppingPlacesSection entry={entry} domain={domain} net={net} core={core} ops={ops} select={select} mode={mode} setMode={setMode} readOnly={readOnly} t={t} />

      <Section title={t('station_boundary')}>
        <p className="ui-muted">{t('station_boundary_hint')}</p>
        {marks.length ? (
          <ul className="tool-marks">
            {marks.map(m => (
              <li key={m.signalId ?? m.switchId ?? m.platformId} className="ui-row">
                <span className="ui-grow">{markText(m)}</span>
                {!readOnly && <button type="button" className="ui-link" onClick={() => setMarks(marks.filter(x => x !== m))}>{t('remove')}</button>}
              </li>
            ))}
          </ul>
        ) : <p className="ui-list-empty">{t('station_boundary_none')}</p>}
        {!readOnly && <>
          <div className="ui-row">
            <Button size="small" disabled={!proposal.length}
              onClick={() => setMarks([...marks.filter(m => !m.signalId), ...proposal])}>{t('boundary_from_entries', { n: proposal.length })}</Button>
          </div>
          <div className="ui-fields">
            <SelectField compact wide label={t('boundary_add_signal')} value={null} empty={t('none')}
              options={signalChoices(net, mine, t).filter(unmarked('signalId'))}
              onChange={(v) => v && setMarks([...marks, { signalId: v }])} />
            <SelectField compact wide label={t('boundary_add_switch')} value={null} empty={t('none')}
              options={net.switches.filter(sw => sw.kind !== 'link').map(sw => ({ value: sw.switchId, label: sw.name ?? sw.label ?? sw.switchId.slice(0, 8) })).filter(unmarked('switchId')).sort(byLabel)}
              onChange={(v) => v && setMarks([...marks, { switchId: v }])} />
          </div>
        </>}
      </Section>

      {asking && (
        <ConfirmDialog title={t('delete_title', { what: stationName(point) })} confirmLabel={t('delete')} cancelLabel={t('cancel')} danger
          onCancel={() => setAsking(false)}
          onConfirm={() => { ops.replaceAll(withoutStation(domain, point.id, platforms)); setAsking(false); select(null) }}>
          <p className="ui-muted">{t('station_delete_ask')}</p>
        </ConfirmDialog>
      )}
    </div>
  )
}
