import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  Button, ConfirmDialog, FitIcon, FormDialog, IconButton, Message, Rail, RailBrand, RailButton, RailDivider, RailRow,
  RedoIcon, Shell, Tabs, TextField, UndoIcon, WorkingCopyBar, Workspace,
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
import { added, cascadeOf, deleted, domainOf, findLogical, labelOf, recordOf, replaced, signallingOf, withSignalling, withSpeedsShown } from './model/domain.js'
import { waysBetween } from './model/itineraries.js'
import { routeStretches, runStretches, wayPieces } from './model/routeSearch.js'
import { logicalSignalWord } from './labels.js'
import { changedIds, redo, startHistory, step, stepOf, stepText, travel, undo } from './model/history.js'
import { withClearanceMarkers } from './model/clearanceMarkers.js'
import { DraftContext } from './panels/draftContext.js'
import PlanView from './plan/PlanView.jsx'
import SchematicView from './schema/SchematicView.jsx'
import {
  BaliseIcon, CheckIcon, DetectionIcon, EditIcon, ExchangeIcon, HistoryIcon, HomeIcon, LayersIcon, LogoIcon, RouteIcon, SettingsIcon,
  SignalIcon, StationIcon, SwitchIcon,
} from './icons.jsx'
import SignalsPanel from './panels/SignalsPanel.jsx'
import DetectionPanel from './panels/DetectionPanel.jsx'
import BalisesPanel from './panels/BalisesPanel.jsx'
import EditPanel from './panels/EditPanel.jsx'
import ElementsPanel from './panels/ElementsPanel.jsx'
import StationsPanel from './panels/StationsPanel.jsx'
import RoutesPanel from './panels/RoutesPanel.jsx'
import CheckPanel from './panels/CheckPanel.jsx'
import ExchangePanel from './panels/ExchangePanel.jsx'
import LayersPanel, { DEFAULT_PLAN_BASEMAP } from './panels/LayersPanel.jsx'
import HistoryPanel from './panels/HistoryPanel.jsx'
import SettingsPanel from './panels/SettingsPanel.jsx'

/**
 * The sections of the rail, in groups the gray line parts: what creates
 * signals, train detection and balises (`creates`: not where it only looks),
 * the editing of all three, the stations, switches and routes, the check and
 * the exchange of data (not where it only looks either).
 */
const SECTIONS = [
  [
    { id: 'signals', icon: <SignalIcon />, Panel: SignalsPanel, creates: true },
    { id: 'detection', icon: <DetectionIcon />, Panel: DetectionPanel, creates: true },
    { id: 'balises', icon: <BaliseIcon />, Panel: BalisesPanel, creates: true },
  ],
  [{ id: 'edit', icon: <EditIcon />, Panel: EditPanel }],
  [
    { id: 'stations', icon: <StationIcon />, Panel: StationsPanel },
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
  operationalPoints: 'stations', stationBoundaries: 'stations', stoppingPlaces: 'stations', itineraries: 'routes',
}

/** How long the edits rest before the working copy is written [ms]. */
const KEEP_AFTER = 250

/**
 * Working on the signalling of a variant: the rail of sections on the left,
 * as the Layout-Tool orders its own — the map layers first; then creating
 * signals, train detection, balises and magnets; editing all three; the
 * stations with their signals, platforms and boundary; the switches with
 * their flank protection and routes with their overlaps; the
 * check and the exchange of data; at its foot undo, the history and redo, the
 * way back to the infrastructure editor and the settings —, the panel of the
 * one chosen, a menu with its pages as the Layout-Tool's panels have, the
 * plan of the net with what the domain holds. What is chosen, on the plan,
 * in the check or after creating it, opens in the panel that edits it.
 *
 * Every edit is first a draft: the plan and the schematic view draw what it
 * changes as a preview, the check runs on it, and Cancel and Commit at the
 * foot of every panel that changes something drop it or make it hold. What
 * is committed is a step (undo, redo, the history), and is kept at once in the
 * variant's working copy in this browser — the one the Layout-Tool works on
 * too —, never only in the page: leaving the tool asks nothing, the start
 * page says the variant holds changes. Checking in sends the copy, tracks and
 * signalling, to the project server; updating brings in what others checked
 * in; discarding makes the copy the server's head again. The core's check
 * runs on every edit. The core is the Layout-Tool's: this tool reads it, and
 * writes of it only the stations — the points of the operational point
 * register and which station a platform is of (data model, R1) —, which its
 * state holds beside the domain (domainOf).
 */
export default function Editor({ project, variant, head, copy: opened, readOnly }) {
  const { t, language, setLanguage } = useTexts()
  const [copy, setCopy] = useState(() => opened ?? freshCopy(head))
  const [history, setHistory] = useState(() => startHistory(domainOf(copy.project)))
  // What is edited and not committed yet: the domain as it would be, or null.
  const [draft, setDraft] = useState(null)
  const [section, setSection] = useState(readOnly ? 'edit' : 'signals')
  // Each time a section is chosen in the rail its panel starts at its menu again.
  const [visit, setVisit] = useState(0)
  const [selection, setSelection] = useState(null)
  const [mode, setMode] = useState(null)
  // What a panel draws on the plan beyond what is chosen — the ways it found.
  const [sketch, setSketch] = useState(null)
  const [dialog, setDialog] = useState(null)
  const [notice, setNotice] = useState(null)
  const [busy, setBusy] = useState(false)
  const [fitKey, setFitKey] = useState(0)
  const [basemap, chooseBasemap] = useState(keptBasemap)
  const setBasemap = (id) => { chooseBasemap(id); keepBasemap(id) }
  // The site plan or the schematic view; setting anew is the site plan's.
  const [view, chooseView] = useState(keptView)
  const setView = (v) => { chooseView(v); keepView(v) }

  // The state: the domain, and the parts of the core the tool writes (the stations).
  // What is shown, checked and edited is the draft where there is one; what is kept and checked in, what is committed.
  const committed = history.now
  const domain = draft ?? committed
  const record = copy.project
  const signalling = useMemo(() => signallingOf(domain), [domain])
  const current = useMemo(() => recordOf(record, committed), [record, committed])
  const shown = useMemo(() => (draft ? recordOf(record, draft) : current), [record, draft, current])
  const pending = useMemo(() => (draft ? changedIds(committed, draft) : NONE), [committed, draft])
  const net = useMemo(() => ({ tracks: record.tracks ?? [], switches: record.switches ?? [] }), [record])
  const drawing = useMemo(() => netDrawing(net.tracks), [net])
  const coreDomain = useMemo(() => projectDocument({ ...shown, signalling: undefined }).domains.core, [shown])
  const findings = useMemo(() => checkSignalling({ core: coreDomain, signalling }), [coreDomain, signalling])
  const extents = useMemo(() => trackSectionExtents({ core: { collections: net }, signalling }), [net, signalling])
  const changes = useMemo(() => (readOnly ? 0 : localChangeCount({ basePayload: copy.basePayload, project: current })), [readOnly, copy, current])

  // Every step into the working copy, once the steps come to rest; what was
  // opened is not written again, a draft never.
  const written = useRef(history.now)
  useEffect(() => {
    if (readOnly || committed === written.current) return undefined
    const timer = setTimeout(() => {
      written.current = committed
      keepCopy(variant.id, { ...copy, project: current })
    }, KEEP_AFTER)
    return () => clearTimeout(timer)
  }, [readOnly, committed, copy, current, variant.id])

  // Leaving the page with a draft asks first: it is in no working copy yet.
  useEffect(() => {
    if (!draft) return undefined
    const onLeave = (e) => { e.preventDefault(); e.returnValue = '' }
    window.addEventListener('beforeunload', onLeave)
    return () => window.removeEventListener('beforeunload', onLeave)
  }, [draft])

  /** The copy from now on, kept at once; `nextDomain` the domain worked on, as a step. */
  const adopt = (next, nextDomain = domainOf(next.project)) => {
    const project = recordOf(next.project, nextDomain)
    setCopy({ ...next, project })
    written.current = nextDomain
    keepCopy(variant.id, { ...next, project })
    setDraft(null)
    setHistory(h => step(h, nextDomain))
  }

  // An edit changes the draft — begun from what is committed; one that comes back to it leaves none.
  const now = useRef(committed)
  now.current = committed
  const edit = useCallback((fn) => {
    if (readOnly) return
    setDraft(d => { const next = fn(d ?? now.current); return next === now.current ? null : next })
  }, [readOnly])
  const ops = useMemo(() => ({
    add: (collection, obj) => { edit(d => added(d, collection, obj)); setSelection({ collection, id: obj.id }) },
    update: (collection, obj) => edit(d => replaced(d, collection, obj)),
    /** Change the object of that id as it is now — for a mode that changes it click by click. */
    modify: (collection, id, fn) => edit(d => {
      const obj = d.collections[collection].find(o => o.id === id)
      return obj ? replaced(d, collection, fn(obj)) : d
    }),
    /** The state anew; a collection `next` does not hold — the core's, where a domain is read in — stays as it is. */
    replaceAll: (next) => edit(d => ({ fields: next.fields, collections: { ...d.collections, ...next.collections } })),
    remove: (collection, id) => setDialog({ kind: 'delete', collection, id }),
  }), [edit])

  // Every switch has its clearance markers Ra 12, every route's speed its Zs 3 at the start signal: what is lacking
  // is set as the tool opens, or the net brings new switches — a step of its own.
  useEffect(() => {
    if (readOnly) return
    const { domain: marked, added: n } = withClearanceMarkers(now.current, net)
    const { domain: next, changed: m } = withSpeedsShown(marked)
    if (!n && !m) return
    setHistory(h => step(h, next))
    setNotice({ tone: 'info', text: [n ? t('clearance_markers_set', { n }) : null, m ? t('speeds_shown_set', { n: m }) : null].filter(Boolean).join(' ') })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [net, readOnly])

  const [focus, setFocus] = useState(null)
  const select = useCallback((sel) => {
    setSelection(sel)
    if (sel && SECTION_OF[sel.collection]) setSection(SECTION_OF[sel.collection])
  }, [])
  /** Choose from a list: the plan brings the object into view where it has a place. */
  const chooseFromList = useCallback((sel) => {
    select(sel)
    const at = placeToShow(domain, sel)
    if (at) setFocus(f => ({ at, n: (f?.n ?? 0) + 1 }))
  }, [select, domain])

  /** The draft made to hold: one step. */
  const commit = () => {
    if (draft) setHistory(h => step(h, draft))
    setDraft(null)
    setMode(null)
  }
  /** The draft dropped: what it set is gone, and with it the choice of it. */
  const cancel = () => {
    setDraft(null)
    setMode(null)
    setSelection(sel => (sel && !isIn(committed, sel) ? null : sel))
  }
  /** Undo: the draft first, where there is one; else the last step. */
  const undoNow = () => (draft ? cancel() : setHistory(undo))
  const redoNow = () => { if (!draft) setHistory(redo) }
  const draftApi = readOnly ? null : { open: !!draft, what: draft ? stepText(stepOf(committed, draft), t) : null, commit, cancel, t }

  // Keys: undo, redo, Esc ends placing — else drops the draft —, Delete asks to delete the chosen.
  useEffect(() => {
    const onKey = (e) => {
      if (e.target.closest?.('input, select, textarea, [role="dialog"]')) return
      const mod = e.ctrlKey || e.metaKey
      if (mod && e.key.toLowerCase() === 'z' && !e.shiftKey) { e.preventDefault(); keys.current.undo() }
      else if (mod && (e.key.toLowerCase() === 'y' || (e.key.toLowerCase() === 'z' && e.shiftKey))) { e.preventDefault(); keys.current.redo() }
      else if (e.key === 'Escape') keys.current.escape()
      else if ((e.key === 'Delete' || e.key === 'Backspace') && selection && !readOnly && !['switches', 'operationalPoints'].includes(selection.collection)) {
        e.preventDefault(); setDialog({ kind: 'delete', ...selection })
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [selection, readOnly])
  // What the keys do, as it is now.
  const keys = useRef(null)
  // A mode a page holds while it is open (sticky) is not ended by Esc: Esc drops the draft there.
  keys.current = { undo: undoNow, redo: redoNow, escape: () => (mode && !mode.sticky ? setMode(null) : draft && cancel()) }

  // Placing or picking points is done on the site plan.
  useEffect(() => { if (mode) chooseView('plan') }, [mode])

  // Picking a section's points ends where its editor is left.
  useEffect(() => {
    if (mode?.kind === 'pick-points' && selection?.collection !== 'trackSections') setMode(null)
  }, [mode, selection])

  const chosen = useMemo(() => highlightOf(domain, net, selection, extents), [domain, net, selection, extents])
  const highlight = sketch ?? chosen
  /** Bring locations into view on the plan, all of them at once. */
  const showOnPlan = useCallback((fit) => setFocus(f => ({ fit, n: (f?.n ?? 0) + 1 })), [])

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
      adopt({ base: res.revision, basePayload: current, project: current, idLog: [] }, committed)
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
    const { domain: carried } = carryDomain(signallingOf(committed), {
      present, changed: (id) => before.has(id) && (!present.has(id) || now.find(tr => tr.id === id) !== before.get(id)),
      oldTrack: (id) => before.get(id) ?? null, candidates: () => now, tracks: now, switches: next.project.switches ?? [],
    })
    adopt(next, withSignalling(domainOf(next.project), carried))
    setDialog(null)
  }

  // What the two arrows would do, said on them.
  const undoTitle = draft ? t('undo_draft')
    : history.past.length ? t('undo_step', { step: stepText(stepOf(history.past.at(-1), history.now), t) }) : t('undo')
  const redoTitle = !draft && history.future.length ? t('redo_step', { step: stepText(stepOf(history.now, history.future[0]), t) }) : t('redo')

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
    domain, net, drawing, findings, extents, selection, select: chooseFromList, ops, mode, setMode, readOnly, t, language, pending,
    record, core: coreDomain, setNotice, setSection, basemap, setBasemap, setSketch, showOnPlan,
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
              <RailButton bare icon={<UndoIcon />} label={undoTitle} disabled={!draft && !history.past.length} onClick={undoNow} />
              <RailButton bare icon={<HistoryIcon />} label={t('step_history_title')} pressed={section === 'history'} onClick={() => open('history')} />
              <RailButton bare icon={<RedoIcon />} label={redoTitle} disabled={!!draft || !history.future.length} onClick={redoNow} />
            </RailRow>
            <RailDivider />
          </>}
          <RailButton icon={<HomeIcon />} label={t('infrastructure')} onClick={() => location.assign(projectsUrl())} />
          <RailButton icon={<SettingsIcon />} label={t('section_settings')} pressed={section === 'settings'} onClick={() => open('settings')} />
        </>}
      />
      {section === 'history' ? <HistoryPanel history={history} travel={(n) => { setDraft(null); setHistory(h => travel(h, n)) }} t={t} />
        : section === 'layers' ? <LayersPanel {...panelProps} />
        : section === 'settings' ? <SettingsPanel t={t} language={language} setLanguage={setLanguage} />
          : <DraftContext.Provider value={draftApi}><active.Panel key={`${section}-${visit}`} {...panelProps} /></DraftContext.Provider>}
      <Workspace>
        <WorkingCopyBar title={project.title} subtitle={variant.name} revision={t('revision_n', { n: copy.base.number })}
          {...(readOnly
            ? { note: t('read_only_note'), actions: [{ label: t('back'), onClick: () => location.assign(projectsUrl()) }] }
            : {
              state: changes ? 'local' : 'clean', stateLabel: changes ? t('local_changes', { n: changes }) : t('saved'),
              stateTitle: changes ? t('local_changes_hint') : undefined,
              actions: [
                // Updating and checking in wait for the draft: committed or dropped first.
                { label: t('update'), onClick: update, disabled: busy || !!draft },
                { label: t('discard'), onClick: () => setDialog({ kind: 'discard' }), disabled: busy || !changes },
                { label: t('check_in'), onClick: () => setDialog({ kind: 'checkin' }), disabled: busy || !changes || !!draft, primary: true },
              ],
            })} />
        <div className="tool-view">
          <Tabs label={t('view_label')} value={view} onChange={setView}
            options={[{ value: 'plan', label: t('view_plan') }, { value: 'schema', label: t('view_schema') }]} />
        </div>
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
            <span>{mode.hint ?? t(mode.kind === 'pick-points' ? 'picking_points' : 'placing')}</span>
            {!mode.sticky && <Button size="small" onClick={() => setMode(null)}>{mode.kind === 'pick-points' ? t('close') : t('cancel')}</Button>}
          </div>
        )}
        {view === 'schema' && (
          <SchematicView net={net} platforms={domain.collections.platforms} domain={domain} selection={selection} highlight={highlight} pending={pending}
            fitKey={fitKey} focus={focus} t={t} onPick={select} />
        )}
        <PlanView hidden={view === 'schema'} drawing={drawing} net={net} domain={domain} selection={selection} highlight={highlight} pending={pending} mode={mode} fitKey={fitKey} focus={focus} basemap={basemap} t={t}
          onPick={select}
          onPlace={(hit) => { const m = mode; setMode(null); m?.apply?.(m.make(hit), hit) }}
          onPickPoint={(id) => mode?.toggle?.(id)} onPickStretch={(s) => mode?.apply?.(s)} onPickPlace={(id) => mode?.apply?.(id)} onPickSignal={(id) => mode?.apply?.(id)} />
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

/** No ids: nothing drafted. */
const NONE = new Set()

/** Whether the domain holds what is chosen: the object, or a logical signal's place with it. */
const isIn = (domain, sel) => (sel.collection === 'logicalSignals'
  ? !!findLogical(domain, sel.id).logical
  : !!domain.collections[sel.collection]?.some(o => o.id === sel.id))

const BASEMAP_KEY = 'rail-across-signalling-basemap'
const VIEW_KEY = 'rail-across-signalling-view'

/** The view chosen last in this browser, else the site plan. */
function keptView() {
  try { return localStorage.getItem(VIEW_KEY) === 'schema' ? 'schema' : 'plan' } catch { return 'plan' }
}
function keepView(v) {
  try { localStorage.setItem(VIEW_KEY, v) } catch { /* the site plan next time */ }
}

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
    case 'operationalPoints': return c.signals.find(s => s.operationalPointId === sel.id)?.at ?? null
    case 'stoppingPlaces': return obj ? middleOf(obj.at) : null
    case 'itineraries': {
      const h = obj && c.stoppingPlaces.find(x => x.id === obj.startStoppingPlaceId)
      return h ? middleOf(h.at) : null
    }
    case 'stationBoundaries': return obj ? c.signals.find(s => obj.marks.some(m => m.signalId === s.id))?.at ?? null : null
    default: return null
  }
}

/** A range's middle, as a location to show. */
const middleOf = (at) => ({ anchor: 'station', trackId: at.trackId, station: (at.startStation + at.endStation) / 2 })

/** What the plan marks for what is chosen: its run, its section's extent, the signals it names; an itinerary's way, the way it runs. */
function highlightOf(domain, net, selection, extents) {
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
      // From signal to signal, the way it runs; its overlaps beyond, fainter.
      const overlapWays = (r.overlaps ?? []).map(o => c.overlaps.find(x => x.id === o.overlapId)).filter(Boolean)
        .map(o => ({ pieces: wayPieces(runStretches(net, c.signals.find(s => s.id === o.signalId)?.at, o.path.trackIds, o.end)) }))
      return { ways: [{ pieces: wayPieces(routeStretches(net, r, c.signals)), strong: true }, ...overlapWays], ids: [r.startSignalId, r.targetSignalId] }
    }
    case 'overlaps': {
      const o = find('overlaps')
      return o ? { ways: [{ pieces: wayPieces(runStretches(net, c.signals.find(s => s.id === o.signalId)?.at, o.path.trackIds, o.end)), strong: true }], ids: [o.signalId] } : null
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
    case 'operationalPoints': return {
      ids: c.signals.filter(s => s.operationalPointId === selection.id).map(s => s.id),
      stretches: c.stoppingPlaces.filter(h => h.operationalPointId === selection.id).map(h => ({ trackId: h.at.trackId, from: h.at.startStation, to: h.at.endStation })),
    }
    case 'itineraries': {
      const it = find('itineraries')
      if (!it) return null
      const [from, to] = [it.startStoppingPlaceId, it.targetStoppingPlaceId].map(id => c.stoppingPlaces.find(h => h.id === id))
      const ids = it.routeIds.flatMap(id => { const r = c.routes.find(x => x.id === id); return r ? [r.startSignalId, r.targetSignalId] : [] })
      const stretches = [from, to].filter(Boolean).map(h => ({ trackId: h.at.trackId, from: h.at.startStation, to: h.at.endStation }))
      // Its way as the search found it, where the net still has it; else its tracks.
      const way = from && to ? waysBetween(net, from, to).find(w => w.trackIds.join() === it.path.trackIds.join()) : null
      return way
        ? { ways: [{ pieces: way.pieces.map(p => ({ trackId: p.trackId, from: p.start, to: p.end })), strong: true }], ids, stretches }
        : { runs: it.path.trackIds, ids, stretches }
    }
    case 'stationBoundaries': {
      const b = find('stationBoundaries')
      return b ? { ids: c.signals.filter(s => s.operationalPointId === b.operationalPointId).map(s => s.id) } : null
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
