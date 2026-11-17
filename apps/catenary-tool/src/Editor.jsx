import { useEffect, useMemo, useRef, useState } from 'react'
import {
  Button, ConfirmDialog, FitIcon, FormDialog, IconButton, LANGUAGE_NAMES, LANGUAGES, Message, NumberField, Option, Panel, Rail, RailBrand,
  RailButton, RailDivider, RailRow, RedoIcon, Section, SelectField, Shell, TextField, UndoIcon, WorkingCopyBar, Workspace,
} from '@rail-across/ui'
import { mergeProject } from '@rail-across/core/merge'
import { trackLength } from '@rail-across/core/track'
import { localChangeCount } from '@rail-across/working-copy'
import { useTexts } from './i18n.js'
import { api } from './api.js'
import { projectsUrl } from './suite.js'
import { freshCopy, keepCopy } from './workingCopy.js'
import { netLines } from './model/net.js'
import {
  INSULATOR_KINDS, MAST_KINDS, SPAN, VOLTAGES, VOLTAGE_COLORS, checkElectrification, coverage, domainOf, lengthOf, newId, newWire, put,
  recordOf, statusOf, stretchesOf, withStatus, withStretch, without, withoutMast, withoutWire,
} from './model/electrification.js'
import PlanView, { DEFAULT_BASEMAP, NO_BASEMAP } from './PlanView.jsx'
import { CheckIcon, HomeIcon, InsulatorIcon, LogoIcon, MastIcon, SettingsIcon, WireIcon } from './icons.jsx'

/** How long the edits rest before the working copy is written [ms]. */
const KEEP_AFTER = 250

const BASEMAP_KEY = 'rail-across-catenary-basemap'

/** The basemap chosen last in this browser, else the light one. */
function keptBasemap() {
  try { return localStorage.getItem(BASEMAP_KEY) ?? DEFAULT_BASEMAP } catch { return DEFAULT_BASEMAP }
}
function keepBasemap(id) {
  try { localStorage.setItem(BASEMAP_KEY, id) } catch { /* the light one next time */ }
}

/** A form for a new stretch of wire, empty: onto a new wire of a voltage, or onto one there is (`wireId`). */
const NEW_FORM = { trackId: null, from: null, to: null, wireId: null, voltage: VOLTAGES[0], name: '', span: SPAN, status: 'existing' }
/** A form for a new section insulator, empty. */
const NEW_INSULATOR = { trackId: null, station: null, before: null, after: null, kind: 'insulator' }

/**
 * Working on the overhead contact line of a variant (electrification 3.0,
 * decision 0006): the rail on the left — the contact wires, the masts, the
 * section insulators, the check, at its foot undo and redo, the way back and
 * the settings —, the panel of the one chosen, and the plan of the net with
 * the wires in their voltages' colours where they run, and the masts. A stretch
 * of wire is laid from a track and two stations, typed or picked on the plan:
 * masts along it no farther apart than the span, each with a suspension point
 * that holds the wire. What is chosen on the plan opens in its editor.
 *
 * Every change is a step (undo, redo) and is kept at once in the variant's
 * working copy in this browser — the one the other tools work on too. Checking
 * in sends the copy to the project server; updating brings in what others
 * checked in; discarding makes the copy the server's head again. The core is
 * the Layout-Tool's: this tool reads the tracks and writes its layer alone.
 */
export default function Editor({ project, variant, head, copy: opened, readOnly }) {
  const { t, language, setLanguage } = useTexts()
  const [copy, setCopy] = useState(() => opened ?? freshCopy(head))
  const [history, setHistory] = useState(() => ({ past: [], now: domainOf(copy.project), future: [] }))
  const [section, setSection] = useState('wires')
  // What is chosen: { collection, id }.
  const [selection, setSelection] = useState(null)
  const [form, setForm] = useState(NEW_FORM)
  const [insulatorForm, setInsulatorForm] = useState(NEW_INSULATOR)
  // Picking on the plan: { target: 'stretch' | 'insulator', first } once a stretch's start is picked.
  const [picking, setPicking] = useState(null)
  const [notice, setNotice] = useState(null)
  const [dialog, setDialog] = useState(null)
  const [busy, setBusy] = useState(false)
  const [fitKey, setFitKey] = useState(0)
  const [basemap, chooseBasemap] = useState(keptBasemap)
  const setBasemap = (id) => { chooseBasemap(id); keepBasemap(id) }

  const domain = history.now
  const record = copy.project
  const current = useMemo(() => recordOf(record, domain), [record, domain])
  const tracks = useMemo(() => record.tracks ?? [], [record])
  const lines = useMemo(() => netLines(tracks), [tracks])
  const stretches = useMemo(() => stretchesOf(domain), [domain])
  const findings = useMemo(() => checkElectrification(domain, tracks), [domain, tracks])
  const changes = useMemo(() => (readOnly ? 0 : localChangeCount({ basePayload: copy.basePayload, project: current })), [readOnly, copy, current])

  // Every step into the working copy, once the steps come to rest.
  const written = useRef(history.now)
  useEffect(() => {
    if (readOnly || domain === written.current) return undefined
    const timer = setTimeout(() => {
      written.current = domain
      keepCopy(variant.id, { ...copy, project: current })
    }, KEEP_AFTER)
    return () => clearTimeout(timer)
  }, [readOnly, domain, copy, current, variant.id])

  /** A change of the domain: one step. */
  const change = (fn) => {
    if (readOnly) return
    setHistory(h => ({ past: [...h.past, h.now], now: fn(h.now), future: [] }))
  }
  const undo = () => setHistory(h => (h.past.length ? { past: h.past.slice(0, -1), now: h.past.at(-1), future: [h.now, ...h.future] } : h))
  const redo = () => setHistory(h => (h.future.length ? { past: [...h.past, h.now], now: h.future[0], future: h.future.slice(1) } : h))

  /** The copy from now on, kept at once, its domain the one worked on. */
  const adopt = (next) => {
    setCopy(next)
    const d = domainOf(next.project)
    written.current = d
    keepCopy(variant.id, next)
    setHistory(h => ({ past: [...h.past, h.now], now: d, future: [] }))
  }

  const SECTION_OF = { contactWires: 'wires', masts: 'masts', suspensionPoints: 'masts', sectionInsulators: 'insulators' }
  const select = (sel) => {
    // A suspension point opens on its mast.
    const point = sel?.collection === 'suspensionPoints' && domain.collections.suspensionPoints.find(p => p.id === sel.id)
    const next = point ? { collection: 'masts', id: point.mastId } : sel
    setSelection(next)
    if (next) setSection(SECTION_OF[next.collection])
  }

  // A pick on the plan: a stretch's start, then its end on the same track; an insulator's place.
  const pick = (hit) => {
    if (picking?.target === 'insulator') {
      setInsulatorForm(f => ({ ...f, trackId: hit.trackId, station: Math.round(hit.station) }))
      setPicking(null)
      return
    }
    if (!picking?.first) { setPicking({ target: 'stretch', first: hit }); return }
    if (hit.trackId !== picking.first.trackId) { setNotice({ tone: 'warn', text: t('pick_other_track') }); return }
    const [a, b] = [picking.first.station, hit.station].sort((x, y) => x - y)
    setForm(f => ({ ...f, trackId: hit.trackId, from: Math.round(a), to: Math.round(b) }))
    setPicking(null)
  }

  const addStretch = () => {
    const wire = domain.collections.contactWires.find(w => w.id === form.wireId)
      ?? withStatus(newWire({ voltage: form.voltage, name: form.name.trim() || undefined }), form.status)
    change(d => withStretch(d, wire, { trackId: form.trackId, from: form.from, to: form.to, span: form.span || SPAN }))
    setForm(f => ({ ...NEW_FORM, voltage: f.voltage, status: f.status, span: f.span }))
    setSelection({ collection: 'contactWires', id: wire.id })
  }

  const addInsulator = () => {
    const f = insulatorForm
    const obj = { id: newId(), kind: f.kind, at: { anchor: 'station', trackId: f.trackId, station: f.station }, contactWireBeforeId: f.before, contactWireAfterId: f.after }
    change(d => put(d, 'sectionInsulators', obj))
    setInsulatorForm(NEW_INSULATOR)
    setSelection({ collection: 'sectionInsulators', id: obj.id })
  }

  const remove = (sel) => change(d => (sel.collection === 'contactWires' ? withoutWire(d, sel.id)
    : sel.collection === 'masts' ? withoutMast(d, sel.id) : without(d, sel.collection, sel.id)))

  // Keys: undo and redo; Esc ends picking; Delete asks to delete the chosen.
  const keys = useRef(null)
  keys.current = { undo, redo, escape: () => setPicking(null), remove: () => selection && !readOnly && setDialog({ kind: 'delete', ...selection }) }
  useEffect(() => {
    const onKey = (e) => {
      if (e.target.closest?.('input, select, textarea, [role="dialog"]')) return
      const mod = e.ctrlKey || e.metaKey
      if (mod && e.key.toLowerCase() === 'z' && !e.shiftKey) { e.preventDefault(); keys.current.undo() }
      else if (mod && (e.key.toLowerCase() === 'y' || (e.key.toLowerCase() === 'z' && e.shiftKey))) { e.preventDefault(); keys.current.redo() }
      else if (e.key === 'Escape') keys.current.escape()
      else if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); keys.current.remove() }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  // ── the server: checking in, updating, discarding ──

  /** What others checked in, merged into the working copy; true where it was taken. */
  const bringIn = async (theirs) => {
    const remaps = await api.remaps(theirs.revision.id, copy.base.id)
    const merged = mergeProject({ base: copy.basePayload, mine: current, theirs: theirs.record, remapsMine: copy.idLog, remapsTheirs: remaps })
    if (merged.conflicts.length) { setNotice({ tone: 'error', text: t('conflict') }); return false }
    adopt({ base: theirs.revision, basePayload: theirs.record, project: merged.merged, idLog: copy.idLog })
    return true
  }

  const checkIn = async (message) => {
    const res = await api.checkIn(variant.id, { base: copy.base.id, message, record: current, remaps: copy.idLog })
    setDialog(null)
    if (res.revision) {
      adopt({ base: res.revision, basePayload: current, project: current, idLog: [] })
      setNotice({ tone: 'ok', text: t('check_in_done', { n: res.revision.number }) })
    } else if (res.refused) {
      setNotice({ tone: 'error', text: t('check_in_refused', { n: res.refused.length }) })
    } else if (await bringIn(await api.revision(res.stale.id))) {
      setNotice({ tone: 'info', text: t('check_in_stale') })
    }
  }

  const update = async () => {
    setBusy(true)
    try {
      const theirs = await api.head(variant.id)
      if (theirs.revision.id === copy.base.id) setNotice({ tone: 'ok', text: t('up_to_date') })
      else if (await bringIn(theirs)) setNotice({ tone: 'info', text: t('updated', { n: theirs.revision.number }) })
    } catch (err) {
      setNotice({ tone: 'error', text: t('load_failed', { error: err.code ?? String(err) }) })
    } finally {
      setBusy(false)
    }
  }

  const discard = async () => {
    setDialog(null)
    setBusy(true)
    try {
      adopt(freshCopy(await api.head(variant.id)))
      setNotice({ tone: 'info', text: t('discarded') })
    } catch (err) {
      setNotice({ tone: 'error', text: t('load_failed', { error: err.code ?? String(err) }) })
    } finally {
      setBusy(false)
    }
  }

  const counts = { errors: findings.errors.length, warnings: findings.warnings.length }
  const chosen = selection && domain.collections[selection.collection]?.find(o => o.id === selection.id)
  const trackName = (id) => tracks.find(tr => tr.id === id)?.name ?? id?.slice(0, 8) ?? '—'
  const props = { domain, tracks, lines, stretches, trackName, record, readOnly, t }
  const edit = (collection) => (obj) => change(d => put(d, collection, obj))
  const ask = () => setDialog({ kind: 'delete', ...selection })
  const back = () => setSelection(null)
  const open = (name) => () => { setSection(name); setSelection(null) }

  const panel = section === 'check' ? <CheckPanel findings={findings} select={select} {...props} />
    : section === 'settings' ? <SettingsPanel t={t} language={language} setLanguage={setLanguage} basemap={basemap} setBasemap={setBasemap} />
      : chosen && selection.collection === 'contactWires' ? <WireEditor key={chosen.id} wire={chosen} findings={findings} back={back}
        onChange={edit('contactWires')} onDelete={ask} select={select} {...props} />
        : chosen && selection.collection === 'masts' ? <MastEditor key={chosen.id} mast={chosen} findings={findings} back={back}
          onChange={edit('masts')} onPoint={edit('suspensionPoints')} onPointDelete={(id) => change(d => without(d, 'suspensionPoints', id))}
          onDelete={ask} {...props} />
          : chosen && selection.collection === 'sectionInsulators' ? <InsulatorEditor key={chosen.id} insulator={chosen} findings={findings} back={back}
            onChange={edit('sectionInsulators')} onDelete={ask} {...props} />
            : section === 'masts' ? <MastsPanel select={select} {...props} />
              : section === 'insulators' ? <InsulatorsPanel form={insulatorForm} setForm={setInsulatorForm} onAdd={addInsulator} select={select}
                picking={picking?.target === 'insulator'} onPickStart={() => setPicking({ target: 'insulator' })} onPickEnd={() => setPicking(null)} {...props} />
                : <WiresPanel form={form} setForm={setForm} onAdd={addStretch} select={select} picking={picking?.target === 'stretch'}
                  onPickStart={() => setPicking({ target: 'stretch' })} onPickEnd={() => setPicking(null)} {...props} />

  return (
    <Shell>
      <Rail label={t('tool')}
        brand={<RailBrand href={projectsUrl()} label={t('infrastructure')} name={t('tool')}><LogoIcon /></RailBrand>}
        top={<>
          <RailButton icon={<WireIcon />} label={t('section_wires')} pressed={section === 'wires'} onClick={open('wires')} />
          <RailButton icon={<MastIcon />} label={t('section_masts')} pressed={section === 'masts'} onClick={open('masts')} />
          <RailButton icon={<InsulatorIcon />} label={t('section_insulators')} pressed={section === 'insulators'} onClick={open('insulators')} />
          <RailButton icon={<CheckIcon />} label={t('section_check')} pressed={section === 'check'} onClick={() => setSection('check')}
            count={counts.errors || counts.warnings} countTone={counts.errors ? undefined : 'warn'} />
        </>}
        bottom={<>
          {!readOnly && <>
            <RailRow>
              <RailButton bare icon={<UndoIcon />} label={t('undo')} disabled={!history.past.length} onClick={undo} />
              <RailButton bare icon={<RedoIcon />} label={t('redo')} disabled={!history.future.length} onClick={redo} />
            </RailRow>
            <RailDivider />
          </>}
          <RailButton icon={<HomeIcon />} label={t('infrastructure')} onClick={() => location.assign(projectsUrl())} />
          <RailButton icon={<SettingsIcon />} label={t('section_settings')} pressed={section === 'settings'} onClick={() => setSection('settings')} />
        </>}
      />
      {panel}
      <Workspace>
        <WorkingCopyBar title={project.title} subtitle={variant.name} revision={t('revision_n', { n: copy.base.number })}
          {...(readOnly
            ? { note: t('read_only_note'), actions: [{ label: t('back'), onClick: () => location.assign(projectsUrl()) }] }
            : {
              state: changes ? 'local' : 'clean', stateLabel: changes ? t('local_changes', { n: changes }) : t('saved'),
              stateTitle: changes ? t('local_changes_hint') : undefined,
              actions: [
                { label: t('update'), onClick: update, disabled: busy },
                { label: t('discard'), onClick: () => setDialog({ kind: 'discard' }), disabled: busy || !changes },
                { label: t('check_in'), onClick: () => setDialog({ kind: 'checkin' }), disabled: busy || !changes, primary: true },
              ],
            })} />
        <div className="tool-fit"><IconButton label={t('fit')} icon={<FitIcon />} onClick={() => setFitKey(k => k + 1)} /></div>
        {notice && (
          <div className="tool-notice" role="status">
            <Message tone={notice.tone}>{notice.text}</Message>
            <IconButton label={t('close')} icon={<span aria-hidden="true">×</span>} variant="ghost" size="small" onClick={() => setNotice(null)} />
          </div>
        )}
        {!tracks.length && <div className="tool-notice"><Message tone="info">{t('empty_net')}</Message></div>}
        {picking && (
          <div className="tool-mode" role="status">
            <span>{t(picking.target === 'insulator' ? 'pick_insulator' : picking.first ? 'pick_second' : 'pick_first')}</span>
            <Button size="small" onClick={() => setPicking(null)}>{t('cancel')}</Button>
          </div>
        )}
        <PlanView lines={lines} domain={domain} stretches={stretches} selection={selection} picking={!!picking} picked={picking?.first ?? null}
          basemap={basemap} fitKey={fitKey} onSelect={select} onPick={pick} />
      </Workspace>

      {dialog?.kind === 'checkin' && <CheckInDialog t={t} onCancel={() => setDialog(null)} onSubmit={checkIn} />}
      {dialog?.kind === 'discard' && (
        <ConfirmDialog title={t('discard')} confirmLabel={t('discard')} cancelLabel={t('cancel')} danger
          onCancel={() => setDialog(null)} onConfirm={discard}>
          <p className="ui-muted">{t('discard_ask', { n: changes })}</p>
        </ConfirmDialog>
      )}
      {dialog?.kind === 'delete' && (
        <ConfirmDialog title={t('delete')} confirmLabel={t('delete')} cancelLabel={t('cancel')} danger
          onCancel={() => setDialog(null)} onConfirm={() => { remove(dialog); setSelection(null); setDialog(null) }}>
          <p className="ui-muted">{labelOf(domain, dialog, trackName, t)}</p>
          {dialog.collection === 'contactWires' && <p className="ui-muted">{t('delete_wire_hint')}</p>}
        </ConfirmDialog>
      )}
    </Shell>
  )
}

const km = (m) => (m / 1000).toFixed(m < 10000 ? 2 : 1)
const swatch = (voltage) => <span className="tool-swatch" style={{ background: VOLTAGE_COLORS[voltage] ?? '#888' }} aria-hidden="true" />
const voltageOptions = (t) => VOLTAGES.map(v => ({ value: v, label: t(`voltage_${v}`) }))
const statusOptions = (t) => ['existing', 'new', 'removal'].map(s => ({ value: s, label: t(`status_${s}`) }))
const where = (at, trackName, t) => (at?.anchor === 'station' ? `${t('track')} ${trackName(at.trackId)} · ${Math.round(at.station)} m` : '—')

/** A wire as a list names it: its name, else its voltage. */
const wireLabel = (w, t) => (w ? w.name ?? t(`voltage_${w.voltage}`) : '—')

/** What a list or a question names an object by. */
function labelOf(domain, sel, trackName, t) {
  const o = domain.collections[sel.collection]?.find(x => x.id === sel.id)
  if (!o) return ''
  if (sel.collection === 'contactWires') return wireLabel(o, t)
  const kind = sel.collection === 'masts' ? t(`mast_${o.kind ?? 'other'}`)
    : sel.collection === 'suspensionPoints' ? t('point') : t(`insulator_${o.kind ?? 'insulator'}`)
  return `${o.name ? `${o.name} · ` : ''}${kind} · ${where(o.at, trackName, t)}`
}

const mine = (findings, collection, id) => [...findings.errors.map(f => ({ ...f, tone: 'error' })), ...findings.warnings.map(f => ({ ...f, tone: 'warn' }))]
  .filter(f => f.collection === collection && f.id === id)

/** The wires: every one to choose with the stretches it runs on, a new stretch of a wire, how much of the net is under wire. */
function WiresPanel({ domain, tracks, stretches, trackName, form, setForm, onAdd, select, picking, onPickStart, onPickEnd, readOnly, t }) {
  const set = (patch) => setForm(f => ({ ...f, ...patch }))
  const track = tracks.find(tr => tr.id === form.trackId)
  const valid = !!track && form.from != null && form.to != null && Math.abs(form.to - form.from) > 0.01
  const cov = coverage(domain, tracks)
  const wires = [...domain.collections.contactWires].sort((a, b) => wireLabel(a, t).localeCompare(wireLabel(b, t), 'de'))
  const runOf = (id) => stretches.filter(s => s.contactWireId === id).reduce((m, s) => m + lengthOf(s), 0)
  return (
    <Panel title={t('section_wires')} intro={t('section_wires_intro')}>
      <Section title={t('section_wires')}>
        {wires.length
          ? <div className="ui-options">{wires.map(w => (
            <Option key={w.id} icon={swatch(w.voltage)} end={`${Math.round(runOf(w.id))} m`} onClick={() => select({ collection: 'contactWires', id: w.id })}>
              {wireLabel(w, t)}{w.name ? ` · ${t(`voltage_${w.voltage}`)}` : ''}
            </Option>
          ))}</div>
          : <p className="ui-list-empty">{t('wires_none')}</p>}
      </Section>
      {!readOnly && (
        <Section title={t('stretch_new')}>
          <div className="ui-fields">
            <SelectField wide compact label={t('track')} value={form.trackId} empty={t('track_choose')}
              options={tracks.map(tr => ({ value: tr.id, label: trackName(tr.id) })).sort((a, b) => a.label.localeCompare(b.label, 'de'))}
              onChange={(trackId) => set({ trackId })} />
            <NumberField compact label={t('from')} value={form.from} onChange={(v) => set({ from: v })} />
            <NumberField compact label={t('to')} value={form.to} onChange={(v) => set({ to: v })} />
            <SelectField wide compact label={t('wire')} value={form.wireId} empty={t('wire_new')}
              options={wires.map(w => ({ value: w.id, label: wireLabel(w, t) }))} onChange={(wireId) => set({ wireId })} />
            {!form.wireId && <>
              <SelectField compact label={t('voltage')} value={form.voltage} options={voltageOptions(t)} onChange={(voltage) => set({ voltage })} />
              <SelectField compact label={t('status')} value={form.status} options={statusOptions(t)} onChange={(status) => set({ status })} />
              <TextField wide compact label={t('name')} value={form.name} onChange={(name) => set({ name })} />
            </>}
            <NumberField compact label={t('span')} value={form.span} onChange={(span) => set({ span })} />
          </div>
          <div className="ui-row">
            <Button size="small" disabled={!track} onClick={() => set({ from: 0, to: Math.round(trackLength(track) * 100) / 100 })}>{t('whole_track')}</Button>
            <Button size="small" active={!!picking} onClick={picking ? onPickEnd : onPickStart}>{t('pick')}</Button>
            <Button size="small" variant="primary" disabled={!valid} onClick={onAdd}>{t('stretch_add')}</Button>
          </div>
        </Section>
      )}
      <Section title={t('coverage')}>
        <p className="ui-muted">{t('coverage_line', { electrified: km(cov.electrified), total: km(cov.tracks) })}</p>
        {VOLTAGES.filter(v => cov.byVoltage[v]).map(v => (
          <p key={v} className="ui-row">{swatch(v)}<span>{t('coverage_voltage', { voltage: t(`voltage_${v}`), km: km(cov.byVoltage[v]) })}</span></p>
        ))}
      </Section>
    </Panel>
  )
}

/** One wire: its voltage, name and planning status, the stretches it runs on; deleting it with what carries it. */
function WireEditor({ wire, findings, stretches, trackName, back, onChange, onDelete, readOnly, t }) {
  const runs = stretches.filter(s => s.contactWireId === wire.id)
  return (
    <Panel title={t('wire_title')} back={{ label: t('back'), onClick: back }}>
      <div className="tool-editor">
        <p className="ui-row">{swatch(wire.voltage)}<strong>{wireLabel(wire, t)}</strong></p>
        {mine(findings, 'contactWires', wire.id).map((f, i) => <Message key={i} tone={f.tone}>{t(`finding_${f.code}`, f.params)}</Message>)}
        <div className="ui-fields">
          <SelectField compact label={t('voltage')} value={wire.voltage} disabled={readOnly} options={voltageOptions(t)}
            onChange={(voltage) => voltage && onChange({ ...wire, voltage })} />
          <SelectField compact label={t('status')} value={statusOf(wire)} disabled={readOnly} options={statusOptions(t)}
            onChange={(status) => status && onChange(withStatus(wire, status))} />
          <TextField wide compact label={t('name')} value={wire.name ?? ''} disabled={readOnly}
            onChange={(name) => { const { name: _n, ...rest } = wire; onChange(name.trim() ? { ...rest, name } : rest) }} />
        </div>
        <Section title={t('wire_runs')}>
          {runs.length
            ? runs.map((s, i) => <p key={i} className="ui-muted">{t('track')} {trackName(s.trackId)} · {Math.round(s.startStation)}–{Math.round(s.endStation)} m</p>)
            : <p className="ui-list-empty">{t('wire_runs_none')}</p>}
        </Section>
        {!readOnly && <div className="ui-row"><Button size="small" variant="danger" onClick={onDelete}>{t('delete')}</Button></div>}
      </div>
    </Panel>
  )
}

/** The masts: every one to choose, by track and station. */
function MastsPanel({ domain, trackName, select, t }) {
  const masts = [...domain.collections.masts].sort((a, b) => trackName(a.at?.trackId).localeCompare(trackName(b.at?.trackId), 'de') || (a.at?.station ?? 0) - (b.at?.station ?? 0))
  const held = (id) => domain.collections.suspensionPoints.filter(p => p.mastId === id).length
  return (
    <Panel title={t('section_masts')} intro={t('section_masts_intro')}>
      {masts.length
        ? <div className="ui-options">{masts.map(m => (
          <Option key={m.id} end={t('points_n', { n: held(m.id) })} onClick={() => select({ collection: 'masts', id: m.id })}>
            {labelOf(domain, { collection: 'masts', id: m.id }, trackName, t)}
          </Option>
        ))}</div>
        : <p className="ui-list-empty">{t('masts_none')}</p>}
    </Panel>
  )
}

/** One mast: where it stands, its kind and name, and the suspension points on it, each holding a wire. */
function MastEditor({ mast, domain, findings, trackName, back, onChange, onPoint, onPointDelete, onDelete, readOnly, t }) {
  const [station, setStation] = useState(mast.at?.station ?? null)
  const points = domain.collections.suspensionPoints.filter(p => p.mastId === mast.id)
  const wires = domain.collections.contactWires
  const at = mast.at ?? {}
  const commitStation = () => {
    if (station == null || station === at.station) return
    onChange({ ...mast, at: { ...at, station } })
    for (const p of points) if (p.at?.trackId === at.trackId) onPoint({ ...p, at: { ...p.at, station } })
  }
  const addPoint = () => wires.length && onPoint({
    id: newId(), mastId: mast.id, at: { anchor: 'station', trackId: at.trackId, station: at.station }, contactWireId: wires[0].id, height: 5.5,
  })
  return (
    <Panel title={t('mast_title')} back={{ label: t('back'), onClick: back }}>
      <div className="tool-editor">
        <p className="ui-row"><strong>{labelOf(domain, { collection: 'masts', id: mast.id }, trackName, t)}</strong></p>
        {[...mine(findings, 'masts', mast.id), ...points.flatMap(p => mine(findings, 'suspensionPoints', p.id))]
          .map((f, i) => <Message key={i} tone={f.tone}>{t(`finding_${f.code}`, f.params)}</Message>)}
        <div className="ui-fields">
          <NumberField compact label={t('station')} value={station} disabled={readOnly} onChange={setStation} onBlur={commitStation} />
          <SelectField compact label={t('side')} value={at.side ?? null} disabled={readOnly} empty="—"
            options={['left', 'right'].map(s => ({ value: s, label: t(`side_${s}`) }))}
            onChange={(side) => onChange({ ...mast, at: { ...at, ...(side ? { side } : {}) } })} />
          <NumberField compact label={t('offset')} value={at.offset ?? null} disabled={readOnly}
            onChange={(offset) => onChange({ ...mast, at: { ...at, ...(offset != null && offset >= 0 ? { offset } : {}) } })} />
          <SelectField compact label={t('mast_kind')} value={mast.kind ?? 'other'} disabled={readOnly}
            options={MAST_KINDS.map(k => ({ value: k, label: t(`mast_${k}`) }))} onChange={(kind) => kind && onChange({ ...mast, kind })} />
          <SelectField compact label={t('status')} value={statusOf(mast)} disabled={readOnly} options={statusOptions(t)}
            onChange={(status) => status && onChange(withStatus(mast, status))} />
          <TextField compact label={t('mast_name')} value={mast.name ?? ''} disabled={readOnly}
            onChange={(name) => { const { name: _n, ...rest } = mast; onChange(name.trim() ? { ...rest, name } : rest) }} />
        </div>
        <Section title={t('points')}>
          {points.map(p => (
            <div key={p.id} className="ui-fields">
              <SelectField wide compact label={t('wire')} value={p.contactWireId} disabled={readOnly}
                options={wires.map(w => ({ value: w.id, label: wireLabel(w, t) }))} onChange={(id) => id && onPoint({ ...p, contactWireId: id })} />
              <NumberField compact label={t('height')} value={p.height ?? null} disabled={readOnly}
                onChange={(height) => onPoint({ ...p, ...(height != null && height >= 0 ? { height } : {}) })} />
              <NumberField compact label={t('stagger')} value={p.stagger ?? null} disabled={readOnly}
                onChange={(stagger) => onPoint({ ...p, ...(stagger != null ? { stagger } : {}) })} />
              {!readOnly && <Button size="small" variant="danger" onClick={() => onPointDelete(p.id)}>{t('delete')}</Button>}
            </div>
          ))}
          {!points.length && <p className="ui-list-empty">{t('points_none')}</p>}
          {!readOnly && <div className="ui-row"><Button size="small" disabled={!wires.length} onClick={addPoint}>{t('point_add')}</Button></div>}
        </Section>
        {!readOnly && <div className="ui-row"><Button size="small" variant="danger" onClick={onDelete}>{t('delete')}</Button></div>}
      </div>
    </Panel>
  )
}

/** The section insulators: every one to choose, a new one from a track, a station and the two wires it parts. */
function InsulatorsPanel({ domain, tracks, trackName, form, setForm, onAdd, select, picking, onPickStart, onPickEnd, readOnly, t }) {
  const set = (patch) => setForm(f => ({ ...f, ...patch }))
  const wires = domain.collections.contactWires.map(w => ({ value: w.id, label: wireLabel(w, t) }))
  const valid = !!tracks.find(tr => tr.id === form.trackId) && form.station != null && form.before && form.after && form.before !== form.after
  return (
    <Panel title={t('section_insulators')} intro={t('section_insulators_intro')}>
      {domain.collections.sectionInsulators.length
        ? <div className="ui-options">{domain.collections.sectionInsulators.map(i => (
          <Option key={i.id} onClick={() => select({ collection: 'sectionInsulators', id: i.id })}>
            {labelOf(domain, { collection: 'sectionInsulators', id: i.id }, trackName, t)}
          </Option>
        ))}</div>
        : <p className="ui-list-empty">{t('insulators_none')}</p>}
      {!readOnly && (
        <Section title={t('insulator_new')}>
          <div className="ui-fields">
            <SelectField wide compact label={t('track')} value={form.trackId} empty={t('track_choose')}
              options={tracks.map(tr => ({ value: tr.id, label: trackName(tr.id) })).sort((a, b) => a.label.localeCompare(b.label, 'de'))}
              onChange={(trackId) => set({ trackId })} />
            <NumberField compact label={t('station')} value={form.station} onChange={(station) => set({ station })} />
            <SelectField compact label={t('insulator_kind')} value={form.kind}
              options={INSULATOR_KINDS.map(k => ({ value: k, label: t(`insulator_${k}`) }))} onChange={(kind) => kind && set({ kind })} />
            <SelectField wide compact label={t('wire_before')} value={form.before} empty={t('wire_choose')} options={wires} onChange={(before) => set({ before })} />
            <SelectField wide compact label={t('wire_after')} value={form.after} empty={t('wire_choose')} options={wires} onChange={(after) => set({ after })} />
          </div>
          <div className="ui-row">
            <Button size="small" active={!!picking} onClick={picking ? onPickEnd : onPickStart}>{t('pick')}</Button>
            <Button size="small" variant="primary" disabled={!valid} onClick={onAdd}>{t('insulator_add')}</Button>
          </div>
        </Section>
      )}
    </Panel>
  )
}

/** One section insulator: where it is, its kind, the wire before and after it; deleting it. */
function InsulatorEditor({ insulator, domain, findings, trackName, back, onChange, onDelete, readOnly, t }) {
  const [station, setStation] = useState(insulator.at?.station ?? null)
  const wires = domain.collections.contactWires.map(w => ({ value: w.id, label: wireLabel(w, t) }))
  return (
    <Panel title={t('insulator_title')} back={{ label: t('back'), onClick: back }}>
      <div className="tool-editor">
        <p className="ui-row"><strong>{labelOf(domain, { collection: 'sectionInsulators', id: insulator.id }, trackName, t)}</strong></p>
        {mine(findings, 'sectionInsulators', insulator.id).map((f, i) => <Message key={i} tone={f.tone}>{t(`finding_${f.code}`, f.params)}</Message>)}
        <div className="ui-fields">
          <NumberField compact label={t('station')} value={station} disabled={readOnly} onChange={setStation}
            onBlur={() => station != null && station !== insulator.at?.station && onChange({ ...insulator, at: { ...insulator.at, station } })} />
          <SelectField compact label={t('insulator_kind')} value={insulator.kind ?? 'insulator'} disabled={readOnly}
            options={INSULATOR_KINDS.map(k => ({ value: k, label: t(`insulator_${k}`) }))} onChange={(kind) => kind && onChange({ ...insulator, kind })} />
          <SelectField wide compact label={t('wire_before')} value={insulator.contactWireBeforeId} disabled={readOnly} options={wires}
            onChange={(id) => id && onChange({ ...insulator, contactWireBeforeId: id })} />
          <SelectField wide compact label={t('wire_after')} value={insulator.contactWireAfterId} disabled={readOnly} options={wires}
            onChange={(id) => id && onChange({ ...insulator, contactWireAfterId: id })} />
        </div>
        {!readOnly && <div className="ui-row"><Button size="small" variant="danger" onClick={onDelete}>{t('delete')}</Button></div>}
      </div>
    </Panel>
  )
}

/** What the check says: the errors, then the warnings, each opening what it is about. */
function CheckPanel({ findings, domain, trackName, select, t }) {
  const group = (title, list, tone) => list.length > 0 && (
    <Section title={`${title} (${list.length})`}>
      <ul className="tool-findings">
        {list.map((f, i) => (
          <li key={i}>
            <Option onClick={() => select({ collection: f.collection, id: f.id })}>
              <Message tone={tone}>{t(`finding_${f.code}`, f.params)}</Message>
              <span className="ui-muted">{labelOf(domain, f, trackName, t)}</span>
            </Option>
          </li>
        ))}
      </ul>
    </Section>
  )
  return (
    <Panel title={t('section_check')}>
      {!findings.errors.length && !findings.warnings.length && <p className="ui-list-empty">{t('check_none')}</p>}
      {group(t('check_errors'), findings.errors, 'error')}
      {group(t('check_warnings'), findings.warnings, 'warn')}
    </Panel>
  )
}

function SettingsPanel({ t, language, setLanguage, basemap, setBasemap }) {
  return (
    <Panel title={t('section_settings')}>
      <div className="ui-fields">
        <SelectField wide compact label={t('language')} value={language} options={LANGUAGES.map(l => ({ value: l, label: LANGUAGE_NAMES[l] }))}
          onChange={(l) => l && setLanguage(l)} />
        <SelectField wide compact label={t('basemap')} value={basemap === NO_BASEMAP ? NO_BASEMAP : DEFAULT_BASEMAP}
          options={[{ value: DEFAULT_BASEMAP, label: t('basemap_map') }, { value: NO_BASEMAP, label: t('basemap_none') }]}
          onChange={(id) => id && setBasemap(id)} />
      </div>
    </Panel>
  )
}

function CheckInDialog({ t, onCancel, onSubmit }) {
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(null)
  const submit = async () => {
    setBusy(true); setError(null)
    try { await onSubmit(message) } catch (err) { setError(err.code ?? String(err)); setBusy(false) }
  }
  return (
    <FormDialog title={t('check_in_title')} submitLabel={t('check_in')} cancelLabel={t('cancel')} busy={busy} error={error}
      onCancel={onCancel} onSubmit={submit}>
      <TextField label={t('check_in_message')} value={message} onChange={setMessage} autoFocus />
    </FormDialog>
  )
}
