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
  SYSTEMS, SYSTEM_COLORS, added, checkElectrification, coverage, domainOf, lengthOf, newElectrification, recordOf, removed, replaced, statusOf, withStatus,
} from './model/electrification.js'
import PlanView, { DEFAULT_BASEMAP, NO_BASEMAP } from './PlanView.jsx'
import { CheckIcon, HomeIcon, LogoIcon, RangeIcon, SettingsIcon } from './icons.jsx'

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

/** A form for a new range, empty. */
const NEW_FORM = { trackId: null, from: null, to: null, system: SYSTEMS[0], name: '', status: 'existing' }

/**
 * Working on the electrification of a variant (decision 0025): the rail on the
 * left — the ranges, the check, at its foot undo and redo, the way back and the
 * settings —, the panel of the one chosen, and the plan of the net with the
 * ranges under contact wire in their systems' colours. A range is made from a
 * track and two stations, typed or picked on the plan, and a system; what is
 * chosen on the plan opens in its editor.
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
  const [section, setSection] = useState('ranges')
  const [selection, setSelection] = useState(null)
  const [form, setForm] = useState(NEW_FORM)
  // Picking a range on the plan: { first } once its start is picked.
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

  const select = (id) => { setSelection(id); if (id) setSection('ranges') }

  // A pick on the plan: the start, then the end on the same track — into the form.
  const pick = (hit) => {
    if (!picking?.first) { setPicking({ first: hit }); return }
    if (hit.trackId !== picking.first.trackId) { setNotice({ tone: 'warn', text: t('range_pick_other_track') }); return }
    const [a, b] = [picking.first.station, hit.station].sort((x, y) => x - y)
    setForm(f => ({ ...f, trackId: hit.trackId, from: Math.round(a), to: Math.round(b) }))
    setPicking(null)
  }

  const addRange = () => {
    const obj = withStatus(newElectrification({
      trackId: form.trackId, from: form.from, to: form.to, system: form.system, name: form.name.trim() || undefined,
    }), form.status, record.id)
    change(d => added(d, obj))
    setForm(f => ({ ...NEW_FORM, system: f.system, status: f.status }))
    setSelection(obj.id)
  }

  // Keys: undo and redo; Esc ends picking; Delete asks to delete the chosen.
  const keys = useRef(null)
  keys.current = { undo, redo, escape: () => setPicking(null), remove: () => selection && !readOnly && setDialog({ kind: 'delete', id: selection }) }
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
  const chosen = domain.collections.electrifications.find(e => e.id === selection) ?? null
  const trackName = (id) => tracks.find(tr => tr.id === id)?.name ?? id?.slice(0, 8) ?? '—'
  const props = { domain, tracks, lines, trackName, record, readOnly, t }

  return (
    <Shell>
      <Rail label={t('tool')}
        brand={<RailBrand href={projectsUrl()} label={t('infrastructure')} name={t('tool')}><LogoIcon /></RailBrand>}
        top={<>
          <RailButton icon={<RangeIcon />} label={t('section_ranges')} pressed={section === 'ranges'} onClick={() => { setSection('ranges'); setSelection(null) }} />
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
      {section === 'check' ? <CheckPanel findings={findings} select={select} {...props} />
        : section === 'settings' ? <SettingsPanel t={t} language={language} setLanguage={setLanguage} basemap={basemap} setBasemap={setBasemap} />
          : chosen ? <RangeEditor key={chosen.id} range={chosen} findings={findings} back={() => setSelection(null)}
            onChange={(obj) => change(d => replaced(d, obj))} onDelete={() => setDialog({ kind: 'delete', id: chosen.id })} {...props} />
            : <RangesPanel form={form} setForm={setForm} onAdd={addRange} select={select} picking={picking}
              onPickStart={() => setPicking({})} onPickEnd={() => setPicking(null)} {...props} />}
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
            <span>{t(picking.first ? 'range_pick_second' : 'range_pick_first')}</span>
            <Button size="small" onClick={() => setPicking(null)}>{t('cancel')}</Button>
          </div>
        )}
        <PlanView lines={lines} domain={domain} selection={selection} picking={!!picking} picked={picking?.first ?? null}
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
          onCancel={() => setDialog(null)} onConfirm={() => { change(d => removed(d, dialog.id)); setSelection(null); setDialog(null) }}>
          <p className="ui-muted">{rangeLabel(domain.collections.electrifications.find(e => e.id === dialog.id), trackName, t)}</p>
        </ConfirmDialog>
      )}
    </Shell>
  )
}

/** A range as a list names it: its name, else its track and stations. */
function rangeLabel(e, trackName, t) {
  if (!e) return ''
  return e.name ?? `${t('track')} ${trackName(e.at.trackId)} · ${Math.round(e.at.startStation)}–${Math.round(e.at.endStation)} m`
}

const km = (m) => (m / 1000).toFixed(m < 10000 ? 2 : 1)
const swatch = (system) => <span className="tool-swatch" style={{ background: SYSTEM_COLORS[system] }} aria-hidden="true" />
const systemOptions = (t) => SYSTEMS.map(s => ({ value: s, label: t(`system_${s}`) }))
const statusOptions = (t) => ['existing', 'new', 'removal'].map(s => ({ value: s, label: t(`status_${s}`) }))

/** The ranges: every one to choose, a new one from a track and two stations, how much of the net is under wire. */
function RangesPanel({ domain, tracks, trackName, form, setForm, onAdd, select, picking, onPickStart, onPickEnd, readOnly, t }) {
  const set = (patch) => setForm(f => ({ ...f, ...patch }))
  const track = tracks.find(tr => tr.id === form.trackId)
  const valid = !!track && form.from != null && form.to != null && Math.abs(form.to - form.from) > 0.01
  const cov = coverage(domain, tracks)
  const list = [...domain.collections.electrifications].sort((a, b) => trackName(a.at.trackId).localeCompare(trackName(b.at.trackId), 'de') || a.at.startStation - b.at.startStation)
  return (
    <Panel title={t('section_ranges')} intro={t('section_ranges_intro')}>
      <Section title={t('section_ranges')}>
        {list.length
          ? <div className="ui-options">{list.map(e => (
            <Option key={e.id} icon={swatch(e.system)} end={`${Math.round(lengthOf(e))} m`} onClick={() => select(e.id)}>
              {rangeLabel(e, trackName, t)} · {t(`system_${e.system}`)}
            </Option>
          ))}</div>
          : <p className="ui-list-empty">{t('ranges_none')}</p>}
      </Section>
      {!readOnly && (
        <Section title={t('range_new')}>
          <div className="ui-fields">
            <SelectField wide compact label={t('track')} value={form.trackId} empty={t('track_choose')}
              options={tracks.map(tr => ({ value: tr.id, label: trackName(tr.id) })).sort((a, b) => a.label.localeCompare(b.label, 'de'))}
              onChange={(trackId) => set({ trackId })} />
            <NumberField compact label={t('from')} value={form.from} onChange={(v) => set({ from: v })} />
            <NumberField compact label={t('to')} value={form.to} onChange={(v) => set({ to: v })} />
            <SelectField compact label={t('system')} value={form.system} options={systemOptions(t)} onChange={(system) => set({ system })} />
            <SelectField compact label={t('status')} value={form.status} options={statusOptions(t)} onChange={(status) => set({ status })} />
            <TextField wide compact label={t('name')} value={form.name} onChange={(name) => set({ name })} />
          </div>
          <div className="ui-row">
            <Button size="small" disabled={!track} onClick={() => set({ from: 0, to: Math.round(trackLength(track) * 100) / 100 })}>{t('range_whole_track')}</Button>
            <Button size="small" active={!!picking} onClick={picking ? onPickEnd : onPickStart}>{t('range_pick')}</Button>
            <Button size="small" variant="primary" disabled={!valid} onClick={onAdd}>{t('range_add')}</Button>
          </div>
        </Section>
      )}
      <Section title={t('coverage')}>
        <p className="ui-muted">{t('coverage_line', { electrified: km(cov.electrified), total: km(cov.tracks) })}</p>
        {SYSTEMS.filter(s => cov.bySystem[s]).map(s => (
          <p key={s} className="ui-row">{swatch(s)}<span>{t('coverage_system', { system: t(`system_${s}`), km: km(cov.bySystem[s]) })}</span></p>
        ))}
      </Section>
    </Panel>
  )
}

/** One range: its stations, system, name and planning status; deleting it. */
function RangeEditor({ range, findings, trackName, record, back, onChange, onDelete, readOnly, t }) {
  const [from, setFrom] = useState(range.at.startStation)
  const [to, setTo] = useState(range.at.endStation)
  const commitStations = () => {
    const [a, b] = [from, to]
    if (a == null || b == null) return
    if (a === range.at.startStation && b === range.at.endStation) return
    onChange({ ...range, at: { ...range.at, startStation: Math.min(a, b), endStation: Math.max(a, b) } })
  }
  const mine = [...findings.errors.map(f => ({ ...f, tone: 'error' })), ...findings.warnings.map(f => ({ ...f, tone: 'warn' }))].filter(f => f.id === range.id)
  return (
    <Panel title={t('range_title')} back={{ label: t('back'), onClick: back }}>
      <div className="tool-editor">
        <p className="ui-row">{swatch(range.system)}<strong>{rangeLabel(range, trackName, t)}</strong></p>
        {mine.map((f, i) => <Message key={i} tone={f.tone}>{t(`finding_${f.code}`, f.params)}</Message>)}
        <div className="ui-fields">
          <NumberField compact label={t('from')} value={from} disabled={readOnly} onChange={setFrom} onBlur={commitStations} />
          <NumberField compact label={t('to')} value={to} disabled={readOnly} onChange={setTo} onBlur={commitStations} />
          <SelectField compact label={t('system')} value={range.system} disabled={readOnly} options={systemOptions(t)}
            onChange={(system) => system && onChange({ ...range, system })} />
          <SelectField compact label={t('status')} value={statusOf(range, record.id)} disabled={readOnly} options={statusOptions(t)}
            onChange={(status) => status && onChange(withStatus(range, status, record.id))} />
          <TextField wide compact label={t('name')} value={range.name ?? ''} disabled={readOnly}
            onChange={(name) => { const { name: _n, ...rest } = range; onChange(name.trim() ? { ...rest, name } : rest) }} />
        </div>
        <p className="ui-muted">{t('track')} {trackName(range.at.trackId)} · {t('length')} {Math.round(lengthOf(range))} m</p>
        {!readOnly && <div className="ui-row"><Button size="small" variant="danger" onClick={onDelete}>{t('delete')}</Button></div>}
      </div>
    </Panel>
  )
}

/** What the check says: the errors, then the warnings, each opening its range. */
function CheckPanel({ findings, domain, trackName, select, t }) {
  const label = (id) => rangeLabel(domain.collections.electrifications.find(e => e.id === id), trackName, t)
  const group = (title, list, tone) => list.length > 0 && (
    <Section title={`${title} (${list.length})`}>
      <ul className="tool-findings">
        {list.map((f, i) => (
          <li key={i}>
            <Option onClick={() => select(f.id)}>
              <Message tone={tone}>{t(`finding_${f.code}`, f.params)}</Message>
              <span className="ui-muted">{label(f.id)}</span>
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
