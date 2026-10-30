import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  Badge, Button, ConfirmDialog, FitIcon, FormDialog, IconButton, Message, Rail, RailBrand, RailButton, RailDivider,
  RedoIcon, SaveIcon, Shell, TextField, UndoIcon, Workspace,
} from '@rail-across/ui'
import { checkSignalling, trackSectionExtents } from '@rail-across/core/signalling'
import { carryDomain, mergeProject } from '@rail-across/core/merge'
import { projectDocument } from '@rail-across/core/project'
import { useTexts } from './i18n.js'
import { api } from './api.js'
import { projectsUrl } from './suite.js'
import { netDrawing } from './model/net.js'
import { added, cascadeOf, deleted, domainOf, labelOf, replaced } from './model/domain.js'
import { redo, startHistory, step, undo } from './model/history.js'
import PlanView from './plan/PlanView.jsx'
import { CheckIcon, DetectionIcon, FlankIcon, ImportIcon, LayersIcon, PzbIcon, RouteIcon, SignalIcon, SwitchIcon } from './icons.jsx'
import SignalsPanel from './panels/SignalsPanel.jsx'
import DetectionPanel from './panels/DetectionPanel.jsx'
import ElementsPanel from './panels/ElementsPanel.jsx'
import RoutesPanel from './panels/RoutesPanel.jsx'
import FlankPanel from './panels/FlankPanel.jsx'
import PzbPanel from './panels/PzbPanel.jsx'
import CheckPanel from './panels/CheckPanel.jsx'
import ImportPanel from './panels/ImportPanel.jsx'
import LayersPanel, { DEFAULT_PLAN_BASEMAP } from './panels/LayersPanel.jsx'

const SECTIONS = [
  { id: 'signals', icon: <SignalIcon />, Panel: SignalsPanel },
  { id: 'detection', icon: <DetectionIcon />, Panel: DetectionPanel },
  { id: 'elements', icon: <SwitchIcon />, Panel: ElementsPanel },
  { id: 'routes', icon: <RouteIcon />, Panel: RoutesPanel },
  { id: 'flank', icon: <FlankIcon />, Panel: FlankPanel },
  { id: 'pzb', icon: <PzbIcon />, Panel: PzbPanel },
]

/** The section a collection's objects are edited in. */
const SECTION_OF = {
  signals: 'signals', detectionPoints: 'detection', trackSections: 'detection', switchElements: 'elements', switches: 'elements',
  routes: 'routes', overlaps: 'routes', flankProtection: 'flank', pzbElements: 'pzb', stationBoundaries: 'signals',
}

/**
 * Working on the signalling of a variant: the rail of sections on the left —
 * signals, train detection, switch elements, routes, flank protection, PZB,
 * the check and the PlanPro import —, the panel of the one chosen, the plan
 * of the net with what the domain holds. Every edit is a step (undo, redo);
 * the core's check runs on every one; checking in sends the record with its
 * signalling to the project server, merging what others checked in meanwhile.
 * The core is the Layout-Tool's: this tool reads it and never changes it (R1).
 */
export default function Editor({ project, variant, head, readOnly }) {
  const { t, language } = useTexts()
  const [current, setCurrent] = useState({ revision: head.revision, record: head.record })
  const [history, setHistory] = useState(() => startHistory(domainOf(head.record)))
  const [savedDomain, setSavedDomain] = useState(history.now)
  const [section, setSection] = useState('signals')
  const [selection, setSelection] = useState(null)
  const [mode, setMode] = useState(null)
  const [dialog, setDialog] = useState(null)
  const [notice, setNotice] = useState(null)
  const [fitKey, setFitKey] = useState(0)
  const [basemap, chooseBasemap] = useState(keptBasemap)
  const setBasemap = (id) => { chooseBasemap(id); keepBasemap(id) }

  const domain = history.now
  const record = current.record
  const net = useMemo(() => ({ tracks: record.tracks ?? [], switches: record.switches ?? [] }), [record])
  const drawing = useMemo(() => netDrawing(net.tracks), [net])
  const coreDomain = useMemo(() => projectDocument({ ...record, signalling: undefined }).domains.core, [record])
  const findings = useMemo(() => checkSignalling({ core: coreDomain, signalling: domain }), [coreDomain, domain])
  const extents = useMemo(() => trackSectionExtents({ core: { collections: net }, signalling: domain }), [net, domain])
  const dirty = domain !== savedDomain

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

  useEffect(() => {
    if (!dirty) return undefined
    const warn = (e) => { e.preventDefault() }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [dirty])

  const highlight = useMemo(() => highlightOf(domain, selection, extents), [domain, selection, extents])

  // ── checking in ──
  const checkIn = async (message) => {
    const mine = { ...record, signalling: domain }
    const res = await api.checkIn(variant.id, { base: current.revision.id, message, record: mine })
    if (res.revision) {
      setCurrent({ revision: res.revision, record: mine })
      setSavedDomain(domain)
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
    const theirs = await api.revision(res.stale.id)
    const remaps = await api.remaps(res.stale.id, current.revision.id)
    const merged = mergeProject({ base: { ...record, signalling: savedDomain }, mine, theirs: theirs.record, remapsTheirs: remaps })
    if (!merged.conflicts.length) {
      adopt(theirs, domainOf(merged.merged))
      setDialog(null)
      setNotice({ tone: 'info', text: t('check_in_stale') })
      return
    }
    setDialog({ kind: 'conflict', theirs })
  }

  /** The head of the variant as the base, with `nextDomain` the domain worked on. */
  const adopt = (theirs, nextDomain) => {
    setCurrent({ revision: theirs.revision, record: theirs.record })
    setSavedDomain(domainOf(theirs.record))
    setHistory(h => step(h, nextDomain))
  }

  /** Mine laid on their net: every location carried by where it stands onto their tracks. */
  const takeMine = (theirs) => {
    const before = new Map((record.tracks ?? []).map(tr => [tr.id, tr]))
    const now = theirs.record.tracks ?? []
    const present = new Set(now.map(tr => tr.id))
    const { domain: carried } = carryDomain(domain, {
      present, changed: (id) => before.has(id) && (!present.has(id) || now.find(tr => tr.id === id) !== before.get(id)),
      oldTrack: (id) => before.get(id) ?? null, candidates: () => now, tracks: now, switches: theirs.record.switches ?? [],
    })
    adopt(theirs, carried)
    setDialog(null)
  }

  const counts = { errors: findings.errors.length, warnings: findings.warnings.length }
  const active = SECTIONS.find(s => s.id === section)
  const panelProps = {
    domain, net, drawing, findings, extents, selection, select: chooseFromList, ops, mode, setMode, readOnly, t, language,
    record, setNotice, setSection, basemap, setBasemap,
  }

  return (
    <Shell>
      <Rail label={t('tool')}
        brand={<RailBrand href={projectsUrl()} label={`${t('to_projects')} — ${t('suite')}`} src="../logo.svg" />}
        top={<>
          {SECTIONS.map(s => (
            <RailButton key={s.id} icon={s.icon} label={t(`section_${s.id}`)} pressed={section === s.id} onClick={() => setSection(s.id)} />
          ))}
          <RailDivider />
          <RailButton icon={<CheckIcon />} label={t('section_check')} pressed={section === 'check'} onClick={() => setSection('check')}
            count={counts.errors || counts.warnings} countTone={counts.errors ? undefined : 'warn'} />
          {!readOnly && <RailButton icon={<ImportIcon />} label={t('section_import')} pressed={section === 'import'} onClick={() => setSection('import')} />}
        </>}
        bottom={<RailButton icon={<LayersIcon />} label={t('section_layers')} pressed={section === 'layers'} onClick={() => setSection('layers')} />}
      />
      {section === 'check' ? <CheckPanel {...panelProps} /> : section === 'import' ? <ImportPanel {...panelProps} />
        : section === 'layers' ? <LayersPanel {...panelProps} /> : <active.Panel {...panelProps} />}
      <Workspace bar={<>
        <span className="ui-topbar-title">{project.title}</span>
        <span className="ui-muted">{variant.name}</span>
        {readOnly ? <Badge tone="primary">{t('read_only')}</Badge> : <Badge tone={dirty ? 'warn' : 'ok'}>{dirty ? t('unsaved') : t('saved')}</Badge>}
        <span className="ui-grow" />
        <IconButton label={t('fit')} icon={<FitIcon />} variant="ghost" onClick={() => setFitKey(k => k + 1)} />
        {!readOnly && <>
          <IconButton label={t('undo')} icon={<UndoIcon />} variant="ghost" disabled={!history.past.length} onClick={() => setHistory(undo)} />
          <IconButton label={t('redo')} icon={<RedoIcon />} variant="ghost" disabled={!history.future.length} onClick={() => setHistory(redo)} />
          <Button variant="primary" icon={<SaveIcon />} disabled={!dirty} onClick={() => setDialog({ kind: 'checkin' })}>{t('check_in')}</Button>
        </>}
        <a className="ui-link" href={projectsUrl()}>{t('to_projects')}</a>
      </>}>
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
          onPlace={(hit) => { const m = mode; setMode(null); m?.apply?.(hit) }}
          onPickPoint={(id) => mode?.toggle?.(id)} />
      </Workspace>

      {dialog?.kind === 'checkin' && <CheckInDialog t={t} onCancel={() => setDialog(null)} onSubmit={checkIn} />}
      {dialog?.kind === 'conflict' && (
        <ConfirmDialog title={t('check_in_title')} confirmLabel={t('check_in_take_mine')} cancelLabel={t('cancel')}
          onCancel={() => setDialog(null)} onConfirm={() => takeMine(dialog.theirs)}>
          <p className="ui-muted">{t('check_in_conflict')}</p>
        </ConfirmDialog>
      )}
      {dialog?.kind === 'delete' && (
        <DeleteDialog t={t} domain={domain} what={dialog} onCancel={() => setDialog(null)}
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
    case 'signals': case 'detectionPoints': case 'pzbElements': return obj?.at
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
      return f ? { ids: [f.protection.signalId, f.protection.switchElementId].filter(Boolean) } : null
    }
    case 'pzbElements': {
      const p = find('pzbElements')
      return p ? { ids: [p.signalId].filter(Boolean) } : null
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

function DeleteDialog({ t, domain, what, onCancel, onConfirm }) {
  const obj = domain.collections[what.collection]?.find(o => o.id === what.id)
  const gone = cascadeOf(domain, what.collection, what.id)
  const others = Object.entries(gone).filter(([name]) => name !== what.collection || gone[name].length > 1)
    .map(([name, ids]) => `${ids.length} × ${t(`collection_${name}`)}`)
  return (
    <ConfirmDialog title={t('delete_title', { what: labelOf(what.collection, obj, t) ?? t('unnamed') })} confirmLabel={t('delete')}
      cancelLabel={t('cancel')} danger onCancel={onCancel} onConfirm={onConfirm}>
      <p className="ui-muted">{others.length ? t('delete_cascade', { list: others.join(', ') }) : t('delete_alone')}</p>
    </ConfirmDialog>
  )
}
