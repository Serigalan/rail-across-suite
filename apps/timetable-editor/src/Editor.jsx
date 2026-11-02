import { useEffect, useMemo, useState } from 'react'
import { ConfirmDialog, FormDialog, NumberField, Rail, RailBrand, RailButton, RailDivider, Shell, TextField, Workspace, WorkingCopyBar } from '@rail-across/ui'
import { netLines, stationsOf } from '@rail-across/running-times'
import { copyTrain, newTrain, numberAfter, taktCopies } from '@rail-across/timetables'
import { useCatalogue } from '@rail-across/vehicles'
import { useTexts } from './i18n.js'
import { GraphIcon, HomeIcon, LogoIcon, SettingsIcon, TrainIcon } from './icons.jsx'
import TrainsPanel from './panels/TrainsPanel.jsx'
import GraphPanel from './panels/GraphPanel.jsx'
import SettingsPanel from './panels/SettingsPanel.jsx'
import TrainSheet from './TrainSheet.jsx'
import Graph from './Graph.jsx'
import { useTimetables } from './useTimetables.js'
import { useRuns } from './useRuns.js'
import { byDeparture } from './rows.js'
import { keepInAddress, keptVehicle, projectsUrl } from './suite.js'

/** The vehicle a new train gets: the one chosen last in this browser, else OSRD's regional multiple unit. */
const FIRST_VEHICLE = 'osrd:emu-regional-train'

/** The number a new train gets: two on from the highest of the timetable's that ends in digits, else none. */
function nextNumber(trains) {
  const numbered = trains.map(tr => tr.number).filter(n => /\d+$/.test(n ?? ''))
  if (!numbered.length) return ''
  const highest = numbered.reduce((a, b) => (Number(b.match(/\d+$/)[0]) > Number(a.match(/\d+$/)[0]) ? b : a))
  return numberAfter(highest, 2)
}

/**
 * The Fahrplan-Editor: a project's timetables laid on the net of a variant,
 * which it only reads. In the panel the timetable chosen and its trains; on
 * the page a train — its number, kind, vehicle, departure and margin, where
 * it begins (a station), the stations and halts it stops at or passes, where
 * it ends (a station) — with its times as the simulation module runs it, the
 * stations it passes among them, and its way on the map; or the graphical
 * timetable of all trains along the way of one. Changes are saved for the
 * project on the server; leaving with changes not saved asks first.
 */
export default function Editor({ project, variant, revision, record, local, initial = {} }) {
  const { t, language, setLanguage } = useTexts()
  const [section, setSection] = useState('trains')
  const timetables = useTimetables(project.id, initial.timetable ?? null)
  // a vehicle changed in the Fahrzeug-Editor's tab is there when this one is shown again
  const catalogue = useCatalogue({ refreshOnShow: true })
  const stations = useMemo(() => stationsOf(record), [record])
  const net = useMemo(() => netLines(record), [record])
  const runs = useRuns(record, stations, timetables.trains, catalogue)
  const [trainId, setTrainId] = useState(initial.train ?? null)
  const [dialog, setDialog] = useState(null)   // { kind: 'new' | 'rename' | 'delete' | 'deleteTrain' | 'takt', ... }
  const [axisId, setAxisId] = useState(null)
  const [view, setView] = useState({ from: null, to: null, scale: 4 })

  const sorted = useMemo(() => byDeparture(timetables.trains), [timetables.trains])
  const train = timetables.trains.find(tr => tr.id === trainId) ?? sorted[0] ?? null
  useEffect(() => { keepInAddress({ timetable: timetables.selected, train: train?.id ?? null }) }, [timetables.selected, train?.id])
  const { mayEdit, dirty } = timetables

  const update = (id, fn) => timetables.changeTrains(list => list.map(tr => (tr.id === id ? fn(tr) : tr)))
  const addTrain = () => {
    const tr = newTrain({ vehicle: keptVehicle() ?? FIRST_VEHICLE, number: nextNumber(timetables.trains) })
    timetables.changeTrains(list => [...list, tr])
    setTrainId(tr.id)
    setSection('trains')
  }
  const copy = (tr) => {
    const c = copyTrain(tr, { number: numberAfter(tr.number, 2) })
    timetables.changeTrains(list => [...list, c])
    setTrainId(c.id)
  }
  const takt = (tr, every, count, numberStep) => timetables.changeTrains(list => [...list, ...taktCopies(tr, { every, count, numberStep })])
  const removeTrain = (tr) => {
    timetables.changeTrains(list => list.filter(x => x.id !== tr.id))
    setTrainId(null)
  }
  const show = (next) => setSection(next)
  const leave = () => timetables.guard(() => location.assign(projectsUrl()))

  const bar = (
    <WorkingCopyBar title={project.title} subtitle={timetables.draft ? `${variant.name} · ${timetables.draft.name}` : variant.name}
      revision={revision?.number != null ? t('revision_n', { n: revision.number }) : undefined}
      state={timetables.draft && mayEdit ? (dirty ? 'local' : 'clean') : undefined}
      stateLabel={dirty ? t('unsaved') : t('saved')}
      note={timetables.draft && !mayEdit ? t('read_only') : t(local ? 'reads_copy' : 'reads_head')}
      actions={dirty
        ? [{ label: t('discard'), onClick: () => timetables.discard(), disabled: timetables.busy },
            { label: timetables.busy ? t('saving') : t('save'), primary: true, onClick: () => timetables.save().catch(() => {}), disabled: timetables.busy }]
        : [{ label: t('back'), onClick: leave }]} />
  )

  const panelProps = { t, language, timetables, sorted, train, setTrainId, runs, stations, mayEdit, addTrain, setDialog }
  return (
    <Shell>
      <Rail label={t('tool')}
        brand={<RailBrand href={projectsUrl()} label={t('suite_timetables')} name={t('tool')}><LogoIcon /></RailBrand>}
        top={<>
          <RailButton icon={<TrainIcon />} label={t('section_trains')} pressed={section === 'trains'} onClick={() => show('trains')} />
          <RailButton icon={<GraphIcon />} label={t('section_graph')} pressed={section === 'graph'} onClick={() => show('graph')} />
        </>}
        bottom={<>
          <RailDivider />
          <RailButton icon={<HomeIcon />} label={t('suite_timetables')} onClick={leave} />
          <RailButton icon={<SettingsIcon />} label={t('section_settings')} pressed={section === 'settings'} onClick={() => show('settings')} />
        </>}
      />
      {section === 'trains' && <TrainsPanel {...panelProps} />}
      {section === 'graph' && <GraphPanel {...panelProps} axisId={axisId} setAxisId={setAxisId} view={view} setView={setView} />}
      {section === 'settings' && <SettingsPanel t={t} language={language} setLanguage={setLanguage} />}
      <Workspace>
        {bar}
        {section === 'graph'
          ? <Graph t={t} trains={sorted} runs={runs} axisTrain={timetables.trains.find(tr => tr.id === axisId) ?? train} view={view}
              selected={train?.id} onSelect={(id) => { setTrainId(id); setSection('trains') }} />
          : <TrainSheet key={train?.id ?? 'none'} t={t} language={language} train={train} run={train ? runs.get(train.id) : null}
              timetables={timetables} stations={stations} catalogue={catalogue} record={record} net={net} mayEdit={mayEdit}
              update={(fn) => train && update(train.id, fn)} onCopy={() => copy(train)} onTakt={() => setDialog({ kind: 'takt' })}
              onDelete={() => setDialog({ kind: 'deleteTrain' })} onNewTimetable={() => setDialog({ kind: 'new' })} onAddTrain={addTrain} />}
      </Workspace>
      <Dialogs t={t} dialog={dialog} setDialog={setDialog} timetables={timetables} train={train} takt={takt} removeTrain={removeTrain} />
    </Shell>
  )
}

/** The questions the editor asks: a new timetable's name, another name, deleting, a Takt. */
function Dialogs({ t, dialog, setDialog, timetables, train, takt, removeTrain }) {
  const [name, setName] = useState('')
  const [form, setForm] = useState({ every: 60, count: 4, step: 2 })
  const close = () => setDialog(null)
  const kind = dialog?.kind
  useEffect(() => {
    if (kind === 'new') setName(t('timetable_default'))
    if (kind === 'rename') setName(timetables.draft?.name ?? '')
    // the name is set when the dialog opens
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kind])
  if (timetables.ask) {
    return (
      <ConfirmDialog title={t('discard_title')} confirmLabel={t('discard')} cancelLabel={t('keep_editing')} danger
        onConfirm={timetables.confirm} onCancel={timetables.dismiss}>
        <p className="ui-muted">{t('discard_text')}</p>
      </ConfirmDialog>
    )
  }
  if (kind === 'new') {
    return (
      <FormDialog title={t('timetable_new')} submitLabel={t('create')} cancelLabel={t('cancel')} busy={timetables.busy} canSubmit={Boolean(name.trim())}
        error={timetables.error ? t('error_save', { error: timetables.error }) : null} onCancel={close}
        onSubmit={() => timetables.guard(async () => { if (await timetables.create(name.trim())) close() })}>
        <div className="ui-fields"><TextField wide label={t('timetable_name')} value={name} onChange={setName} autoFocus /></div>
      </FormDialog>
    )
  }
  if (kind === 'rename') {
    return (
      <FormDialog title={t('timetable_rename')} submitLabel={t('apply')} cancelLabel={t('cancel')} canSubmit={Boolean(name.trim())}
        onCancel={close} onSubmit={() => { timetables.rename(name.trim()); close() }}>
        <div className="ui-fields"><TextField wide label={t('timetable_name')} value={name} onChange={setName} autoFocus /></div>
        <p className="ui-muted">{t('timetable_rename_note')}</p>
      </FormDialog>
    )
  }
  if (kind === 'delete') {
    return (
      <ConfirmDialog title={t('timetable_delete_title', { name: timetables.current?.name })} confirmLabel={t('delete')} cancelLabel={t('cancel')} danger busy={timetables.busy}
        onConfirm={async () => { if (await timetables.remove()) close() }} onCancel={close}>
        <p className="ui-muted">{t('timetable_delete_text', { n: timetables.current?.data.trains.length ?? 0 })}</p>
      </ConfirmDialog>
    )
  }
  if (kind === 'deleteTrain' && train) {
    return (
      <ConfirmDialog title={t('train_delete_title')} confirmLabel={t('delete')} cancelLabel={t('cancel')} danger
        onConfirm={() => { removeTrain(train); close() }} onCancel={close}>
        <p className="ui-muted">{t('train_delete_text')}</p>
      </ConfirmDialog>
    )
  }
  if (kind === 'takt' && train) {
    const ok = form.every > 0 && Number.isInteger(form.count) && form.count >= 1 && form.count <= 100
    return (
      <FormDialog title={t('takt_title')} submitLabel={t('takt_make', { n: form.count ?? 0 })} cancelLabel={t('cancel')} canSubmit={ok}
        onCancel={close} onSubmit={() => { takt(train, Math.round(form.every * 60), form.count, form.step ?? 0); close() }}>
        <p className="ui-muted">{t('takt_text')}</p>
        <div className="ui-fields">
          <NumberField compact label={t('takt_every')} value={form.every} min={1} step={1} onChange={(v) => setForm(f => ({ ...f, every: v }))} />
          <NumberField compact label={t('takt_count')} value={form.count} min={1} max={100} step={1} onChange={(v) => setForm(f => ({ ...f, count: v }))} />
          <NumberField compact label={t('takt_step')} value={form.step} step={1} onChange={(v) => setForm(f => ({ ...f, step: v }))} />
        </div>
      </FormDialog>
    )
  }
  return null
}
