import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  Button, ConfirmDialog, FitIcon, FormDialog, IconButton, Message, Rail, RailBrand, RailButton, RailDivider, RailRow,
  RedoIcon, Shell, TextField, UndoIcon, WorkingCopyBar, Workspace,
} from '@rail-across/ui'
import { checkSignalling, trackSectionExtents } from '@rail-across/core/signalling'
import { carryDomain, mergeProject } from '@rail-across/core/merge'
import { projectDocument } from '@rail-across/core/project'
import { localChangeCount } from '@rail-across/working-copy'
import { useTexts } from './i18n.js'
import { api } from './api.js'
import { projectsUrl } from './suite.js'
import { freshCopy, keepCopy } from './workingCopy.js'
import { netDrawing } from './model/net.js'
import { added, cascadeOf, deleted, domainOf, findLogical, labelOf, replaced } from './model/domain.js'
import { logicalSignalWord } from './labels.js'
import { redo, startHistory, step, stepOf, stepText, travel, undo } from './model/history.js'
import PlanView from './plan/PlanView.jsx'
import {
  BaliseIcon, CheckIcon, DetectionIcon, EditIcon, ExchangeIcon, HistoryIcon, HomeIcon, LayersIcon, LogoIcon, RouteIcon, SettingsIcon,
  SignalIcon, SwitchIcon,
} from './icons.jsx'
import SignalsPanel from './panels/SignalsPanel.jsx'
import DetectionPanel from './panels/DetectionPanel.jsx'
import BalisesPanel from './panels/BalisesPanel.jsx'
import EditPanel from './panels/EditPanel.jsx'
import ElementsPanel from './panels/ElementsPanel.jsx'
import RoutesPanel from './panels/RoutesPanel.jsx'
import CheckPanel from './panels/CheckPanel.jsx'
import ExchangePanel from './panels/ExchangePanel.jsx'
import LayersPanel, { DEFAULT_PLAN_BASEMAP } from './panels/LayersPanel.jsx'
import HistoryPanel from './panels/HistoryPanel.jsx'
import SettingsPanel from './panels/SettingsPanel.jsx'

/**
 * The sections of the rail, in groups the gray line parts: what creates
 * signals, train detection and balises (`creates`: not where it only looks),
 * the editing of all three, the switches and routes, the check and the
 * exchange of data (not where it only looks either).
 */
const SECTIONS = [
  [
    { id: 'signals', icon: <SignalIcon />, Panel: SignalsPanel, creates: true },
    { id: 'detection', icon: <DetectionIcon />, Panel: DetectionPanel, creates: true },
    { id: 'balises', icon: <BaliseIcon />, Panel: BalisesPanel, creates: true },
  ],
  [{ id: 'edit', icon: <EditIcon />, Panel: EditPanel }],
  [
    { id: 'elements', icon: <SwitchIcon />, Panel: ElementsPanel },
    { id: 'routes', icon: <RouteIcon />, Panel: RoutesPanel },
  ],
  [
    { id: 'check', icon: <CheckIcon />, Panel: CheckPanel },
    { id: 'exchange', icon: <ExchangeIcon />, Panel: ExchangePanel, creates: true },
  ],
]

/** The section a collection's objects are edited in. */
const SECTION_OF = {
  signals: 'edit', logicalSignals: 'edit', detectionPoints: 'edit', trackSections: 'edit', balises: 'edit',
  switchElements: 'elements', switches: 'elements', flankProtection: 'elements', routes: 'routes', overlaps: 'routes',
}

/** How long the edits rest before the working copy is written [ms]. */
const KEEP_AFTER = 250

/**
 * Working on the signalling of a variant: the rail of sections on the left,
 * as the Layout-Tool orders its own — the map layers first; then creating
 * signals, train detection, balises and magnets; editing all three; the
 * switches with their flank protection and routes with their overlaps; the
 * check and the exchange of data; at its foot undo, the history and redo, the
 * way back to the infrastructure editor and the settings —, the panel of the
 * one chosen, a menu with its pages as the Layout-Tool's panels have, the
 * plan of the net with what the domain holds. What is chosen, on the plan,
 * in the check or after creating it, opens in the panel that edits it.
 *
 * Every edit is a step (undo, redo, the history), and is kept at once in the
 * variant's working copy in this browser — the one the Layout-Tool works on
 * too —, never only in the page: leaving the tool asks nothing, the start
 * page says the variant holds changes. Checking in sends the copy, tracks and
 * signalling, to the project server; updating brings in what others checked
 * in; discarding makes the copy the server's head again. The core's check
 * runs on every edit. The core is the Layout-Tool's: this tool reads it and
 * never changes it (R1).
 */
export default function Editor({ project, variant, head, copy: opened, readOnly }) {
  const { t, language, setLanguage } = useTexts()
  const [copy, setCopy] = useState(() => opened ?? freshCopy(head))
  const [history, setHistory] = useState(() => startHistory(domainOf(copy.project)))
  const [section, setSection] = useState(readOnly ? 'edit' : 'signals')
  // Each time a section is chosen in the rail its panel starts at its menu again.
  const [visit, setVisit] = useState(0)
  const [selection, setSelection] = useState(null)
  const [mode, setMode] = useState(null)
  const [dialog, setDialog] = useState(null)
  const [notice, setNotice] = useState(null)
  const [busy, setBusy] = useState(false)
  const [fitKey, setFitKey] = useState(0)
  const [basemap, chooseBasemap] = useState(keptBasemap)
  const setBasemap = (id) => { chooseBasemap(id); keepBasemap(id) }

  const domain = history.now
  const record = copy.project
  const current = useMemo(() => ({ ...record, signalling: domain }), [record, domain])
  const net = useMemo(() => ({ tracks: record.tracks ?? [], switches: record.switches ?? [] }), [record])
  const drawing = useMemo(() => netDrawing(net.tracks), [net])
  const coreDomain = useMemo(() => projectDocument({ ...record, signalling: undefined }).domains.core, [record])
  const findings = useMemo(() => checkSignalling({ core: coreDomain, signalling: domain }), [coreDomain, domain])
  const extents = useMemo(() => trackSectionExtents({ core: { collections: net }, signalling: domain }), [net, domain])
  const changes = useMemo(() => (readOnly ? 0 : localChangeCount({ basePayload: copy.basePayload, project: current })), [readOnly, copy, current])

  // Every edit into the working copy, once the edits come to rest; what was
  // opened is not written again.
  const written = useRef(history.now)
  useEffect(() => {
    if (readOnly || domain === written.current) return undefined
    const timer = setTimeout(() => {
      written.current = domain
      keepCopy(variant.id, { ...copy, project: current })
    }, KEEP_AFTER)
    return () => clearTimeout(timer)
  }, [readOnly, domain, copy, current, variant.id])

  /** The copy from now on, kept at once; `nextDomain` the domain worked on, as a step. */
  const adopt = (next, nextDomain = domainOf(next.project)) => {
    const project = { ...next.project, signalling: nextDomain }
    setCopy({ ...next, project })
    written.current = nextDomain
    keepCopy(variant.id, { ...next, project })
    setHistory(h => step(h, nextDomain))
  }

  const edit = useCallback((fn) => setHistory(h => (readOnly ? h : step(h, fn(h.now)))), [readOnly])
  const ops = useMemo(() => ({
    add: (collection, obj) => { edit(d => added(d, collection, obj)); setSelection({ collection, id: obj.id }) },
    update: (collection, obj) => edit(d => replaced(d, collection, obj)),
    /** Change the object of that id as it is now — for a mode that changes it click by click. */
    modify: (collection, id, fn) => edit(d => {
      const obj = d.collections[collection].find(o => o.id === id)
      return obj ? replaced(d, collection, fn(obj)) : d
    }),
    replaceAll: (next) => edit(() => next),
    remove: (collection, id) => setDialog({ kind: 'delete', collection, id }),
  }), [edit])

  const [focus, setFocus] = useState(null)
  const select = useCallback((sel) => {
    setSelection(sel)
    if (sel && SECTION_OF[sel.collection]) setSection(SECTION_OF[sel.collection])
  }, [])
  /** Choose from a list: the plan brings the object into view where it has a place. */
  const chooseFromList = useCallback((sel) => {
    select(sel)
    const at = placeToShow(history.now, sel)
    if (at) setFocus(f => ({ at, n: (f?.n ?? 0) + 1 }))
  }, [select, history])

  // Keys: undo, redo, Esc ends placing, Delete asks to delete the chosen.
  useEffect(() => {
    const onKey = (e) => {
      if (e.target.closest?.('input, select, textarea, [role="dialog"]')) return
      const mod = e.ctrlKey || e.metaKey
      if (mod && e.key.toLowerCase() === 'z' && !e.shiftKey) { e.preventDefault(); setHistory(undo) }
      else if (mod && (e.key.toLowerCase() === 'y' || (e.key.toLowerCase() === 'z' && e.shiftKey))) { e.preventDefault(); setHistory(redo) }
      else if (e.key === 'Escape') setMode(null)
      else if ((e.key === 'Delete' || e.key === 'Backspace') && selection && !readOnly && selection.collection !== 'switches') {
        e.preventDefault(); setDialog({ kind: 'delete', ...selection })
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [selection, readOnly])

  // Picking a section's points ends where its editor is left.
  useEffect(() => {
    if (mode?.kind === 'pick-points' && selection?.collection !== 'trackSections') setMode(null)
  }, [mode, selection])

  const highlight = useMemo(() => highlightOf(domain, selection, extents), [domain, selection, extents])

  // ── the server: checking in, updating, discarding ──

  /**
   * What others checked in, brought into the working copy: merged against the
   * revision the copy rests on, the copy's own splits and joins with it. A
   * conflict in the signalling is decided here (mine laid on the merged net);
   * one in the tracks is the Layout-Tool's to decide. True where it was taken.
   */
  const bringIn = async (theirs) => {
    const remaps = await api.remaps(theirs.revision.id, copy.base.id)
    const merged = mergeProject({ base: copy.basePayload, mine: current, theirs: theirs.record, remapsMine: copy.idLog, remapsTheirs: remaps })
    const next = { base: theirs.revision, basePayload: theirs.record, project: merged.merged, idLog: copy.idLog }
    if (!merged.conflicts.length) { adopt(next); return true }
    if (merged.conflicts.some(c => !(c.collection === 'project' && c.field === 'signalling'))) {
      setNotice({ tone: 'error', text: t('core_conflict') })
      return false
    }
    setDialog({ kind: 'conflict', next })
    return false
  }

  const checkIn = async (message) => {
    const res = await api.checkIn(variant.id, { base: copy.base.id, message, record: current, remaps: copy.idLog })
    if (res.revision) {
      adopt({ base: res.revision, basePayload: current, project: current, idLog: [] }, domain)
      setDialog(null)
      setNotice({ tone: 'ok', text: t('check_in_done', { n: res.revision.number }) })
      return
    }
    if (res.refused) {
      setDialog(null)
      setNotice({ tone: 'error', text: t('check_in_refused', { n: res.refused.length }) })
      setSection('check')
      return
    }
    // Someone checked in first: bring their state in, then the user checks in again.
    setDialog(null)
    if (await bringIn(await api.revision(res.stale.id))) setNotice({ tone: 'info', text: t('check_in_stale') })
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
      const fresh = freshCopy(await api.head(variant.id))
      adopt(fresh)
      setNotice({ tone: 'info', text: t('discarded') })
    } catch (err) {
      setNotice({ tone: 'error', text: t('load_failed', { error: err.code ?? String(err) }) })
    } finally {
      setBusy(false)
    }
  }

  /** Mine laid on the merged net: every location carried by where it stands onto its tracks. */
  const takeMine = ({ next }) => {
    const before = new Map((record.tracks ?? []).map(tr => [tr.id, tr]))
    const now = next.project.tracks ?? []
    const present = new Set(now.map(tr => tr.id))
    const { domain: carried } = carryDomain(domain, {
      present, changed: (id) => before.has(id) && (!present.has(id) || now.find(tr => tr.id === id) !== before.get(id)),
      oldTrack: (id) => before.get(id) ?? null, candidates: () => now, tracks: now, switches: next.project.switches ?? [],
    })
    adopt(next, carried)
    setDialog(null)
  }

  // What the two arrows would do, said on them.
  const undoTitle = history.past.length ? t('undo_step', { step: stepText(stepOf(history.past.at(-1), history.now), t) }) : t('undo')
  const redoTitle = history.future.length ? t('redo_step', { step: stepText(stepOf(history.now, history.future[0]), t) }) : t('redo')

  const counts = { errors: findings.errors.length, warnings: findings.warnings.length }
  const active = SECTIONS.flat().find(s => s.id === section)
  /** A section chosen in the rail: its panel from the start, what was open in it closed. */
  const open = (id) => {
    setSection(id)
    setVisit(v => v + 1)
    setMode(null)
    if (selection && SECTION_OF[selection.collection] === id) setSelection(null)
  }
  const panelProps = {
    domain, net, drawing, findings, extents, selection, select: chooseFromList, ops, mode, setMode, readOnly, t, language,
    record, setNotice, setSection, basemap, setBasemap,
  }

  return (
    <Shell>
      <Rail label={t('tool')}
        brand={<RailBrand href={projectsUrl()} label={t('infrastructure')} name={t('tool')}><LogoIcon /></RailBrand>}
        top={<>
          <RailButton icon={<LayersIcon />} label={t('section_layers')} pressed={section === 'layers'} onClick={() => open('layers')} />
          {SECTIONS.map(group => group.filter(s => !(readOnly && s.creates))).filter(group => group.length).map(group => (
            <Fragment key={group[0].id}>
              <RailDivider />
              {group.map(s => (
                <RailButton key={s.id} icon={s.icon} label={t(`section_${s.id}`)} pressed={section === s.id} onClick={() => open(s.id)}
                  {...(s.id === 'check' ? { count: counts.errors || counts.warnings, countTone: counts.errors ? undefined : 'warn' } : {})} />
              ))}
            </Fragment>
          ))}
        </>}
        bottom={<>
          {!readOnly && <>
            <RailRow>
              <RailButton bare icon={<UndoIcon />} label={undoTitle} disabled={!history.past.length} onClick={() => setHistory(undo)} />
              <RailButton bare icon={<HistoryIcon />} label={t('step_history_title')} pressed={section === 'history'} onClick={() => open('history')} />
              <RailButton bare icon={<RedoIcon />} label={redoTitle} disabled={!history.future.length} onClick={() => setHistory(redo)} />
            </RailRow>
            <RailDivider />
          </>}
          <RailButton icon={<HomeIcon />} label={t('infrastructure')} onClick={() => location.assign(projectsUrl())} />
          <RailButton icon={<SettingsIcon />} label={t('section_settings')} pressed={section === 'settings'} onClick={() => open('settings')} />
        </>}
      />
      {section === 'history' ? <HistoryPanel history={history} travel={(n) => setHistory(h => travel(h, n))} t={t} />
        : section === 'layers' ? <LayersPanel {...panelProps} />
        : section === 'settings' ? <SettingsPanel t={t} language={language} setLanguage={setLanguage} /> : <active.Panel key={`${section}-${visit}`} {...panelProps} />}
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
        {!net.tracks.length && <div className="tool-notice"><Message tone="info">{t('empty_net')}</Message></div>}
        {mode && (
          <div className="tool-mode" role="status">
            <span>{t(mode.kind === 'pick-points' ? 'picking_points' : 'placing')}</span>
            <Button size="small" onClick={() => setMode(null)}>{mode.kind === 'pick-points' ? t('close') : t('cancel')}</Button>
          </div>
        )}
        <PlanView drawing={drawing} net={net} domain={domain} selection={selection} highlight={highlight} mode={mode} fitKey={fitKey} focus={focus} basemap={basemap} t={t}
          onPick={select}
          onPlace={(hit) => { const m = mode; setMode(null); m?.apply?.(m.make(hit)) }}
          onPickPoint={(id) => mode?.toggle?.(id)} />
      </Workspace>

      {dialog?.kind === 'checkin' && <CheckInDialog t={t} onCancel={() => setDialog(null)} onSubmit={checkIn} />}
      {dialog?.kind === 'conflict' && (
        <ConfirmDialog title={t('check_in_title')} confirmLabel={t('check_in_take_mine')} cancelLabel={t('cancel')}
          onCancel={() => setDialog(null)} onConfirm={() => takeMine(dialog)}>
          <p className="ui-muted">{t('check_in_conflict')}</p>
        </ConfirmDialog>
      )}
      {dialog?.kind === 'discard' && (
        <ConfirmDialog title={t('discard')} confirmLabel={t('discard')} cancelLabel={t('cancel')} danger
          onCancel={() => setDialog(null)} onConfirm={discard}>
          <p className="ui-muted">{t('discard_ask', { n: changes })}</p>
        </ConfirmDialog>
      )}
      {dialog?.kind === 'delete' && (
        <DeleteDialog t={t} language={language} domain={domain} what={dialog} onCancel={() => setDialog(null)}
          onConfirm={() => { edit(d => deleted(d, dialog.collection, dialog.id)); setSelection(null); setDialog(null) }} />
      )}
    </Shell>
  )
}

const BASEMAP_KEY = 'rail-across-signalling-basemap'

/** The basemap chosen last in this browser, else the tool's own default. */
function keptBasemap() {
  try { return localStorage.getItem(BASEMAP_KEY) || DEFAULT_PLAN_BASEMAP } catch { return DEFAULT_PLAN_BASEMAP }
}
function keepBasemap(id) {
  try { localStorage.setItem(BASEMAP_KEY, id) } catch { /* chosen again next time */ }
}

/** Where an object is to be shown on the plan: its location, a route's start signal, a section's first point. */
function placeToShow(domain, sel) {
  if (!sel) return null
  const c = domain.collections
  const obj = c[sel.collection]?.find(o => o.id === sel.id)
  const signalAt = (id) => c.signals.find(s => s.id === id)?.at
  switch (sel.collection) {
    case 'signals': case 'detectionPoints': case 'balises': return obj?.at
    case 'logicalSignals': return findLogical(domain, sel.id).signal?.at ?? null
    case 'switchElements': return obj?.derailer?.at ?? null
    case 'routes': return obj && signalAt(obj.startSignalId)
    case 'overlaps': return obj && signalAt(obj.signalId)
    case 'trackSections': return obj && c.detectionPoints.find(d => d.id === obj.boundedBy[0])?.at
    case 'flankProtection': return obj?.protection.signalId ? signalAt(obj.protection.signalId) : null
    default: return null
  }
}

/** What the plan marks for what is chosen: its run, its section's extent, the signals it names. */
function highlightOf(domain, selection, extents) {
  if (!selection) return null
  const c = domain.collections
  const find = (name) => c[name].find(o => o.id === selection.id)
  switch (selection.collection) {
    case 'logicalSignals': {
      const { signal } = findLogical(domain, selection.id)
      return signal ? { ids: [signal.id] } : null
    }
    case 'routes': {
      const r = find('routes')
      if (!r) return null
      const overlapRuns = (r.overlaps ?? []).flatMap(o => c.overlaps.find(x => x.id === o.overlapId)?.path.trackIds ?? [])
      return { runs: r.path.trackIds, stretches: [], ids: [r.startSignalId, r.targetSignalId], overlapRuns }
    }
    case 'overlaps': {
      const o = find('overlaps')
      return o ? { runs: o.path.trackIds, ids: [o.signalId] } : null
    }
    case 'trackSections': {
      const s = find('trackSections')
      if (!s) return null
      const ext = extents.get(s.id)
      return { stretches: (ext?.parts ?? []).map(p => ({ ...p, tone: ext.matches ? null : 'warn' })), ids: s.boundedBy }
    }
    case 'flankProtection': {
      const f = find('flankProtection')
      return f ? { ids: [f.requester.switchElementId, f.protection.signalId, f.protection.switchElementId].filter(Boolean) } : null
    }
    case 'balises': {
      const b = find('balises')
      return b ? { ids: [b.signalId].filter(Boolean) } : null
    }
    default: return null
  }
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

function DeleteDialog({ t, language, domain, what, onCancel, onConfirm }) {
  const logical = what.collection === 'logicalSignals' ? findLogical(domain, what.id) : null
  const obj = domain.collections[what.collection]?.find(o => o.id === what.id)
  const title = logical?.signal ? `${labelOf('signals', logical.signal, t)} · ${logicalSignalWord(logical.logical, language)}` : labelOf(what.collection, obj, t) ?? t('unnamed')
  const gone = cascadeOf(domain, what.collection, what.id)
  const others = Object.entries(gone).filter(([name]) => name !== what.collection || gone[name].length > 1)
    .map(([name, ids]) => `${ids.length} × ${t(`collection_${name}`)}`)
  return (
    <ConfirmDialog title={t('delete_title', { what: title })} confirmLabel={t('delete')}
      cancelLabel={t('cancel')} danger onCancel={onCancel} onConfirm={onConfirm}>
      <p className="ui-muted">{others.length ? t('delete_cascade', { list: others.join(', ') }) : t('delete_alone')}</p>
    </ConfirmDialog>
  )
}
