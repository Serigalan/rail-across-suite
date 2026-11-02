import { useCallback, useEffect, useState } from 'react'
import { newTimetable, timetableApi } from '@rail-across/timetables'

/**
 * The timetables of a project and the one being edited: the list from the
 * server, the one chosen, and its draft — what is changed and not saved yet.
 * Choosing another, or leaving, with changes not saved asks first (`guard`).
 * A save that finds the timetable saved by someone else meanwhile keeps the
 * changes and says so (`conflict`); saving again then puts them over the
 * other's. `initial` the id of the timetable chosen first.
 */
export function useTimetables(projectId, initial = null) {
  const [list, setList] = useState(null)   // [timetable] | null while loading
  const [mayAdd, setMayAdd] = useState(false)
  const [error, setError] = useState(null)
  const [selected, setSelected] = useState(initial)
  const [draft, setDraft] = useState(null)   // { id, name, data, version } — the timetable chosen, as edited
  const [busy, setBusy] = useState(false)
  const [conflict, setConflict] = useState(null)   // the timetable as someone else saved it
  const [ask, setAsk] = useState(null)   // { then } — changes not saved, to leave?

  const current = list?.find(tt => tt.id === selected) ?? null
  const dirty = Boolean(draft && current && (draft.name !== current.name || JSON.stringify(draft.data) !== JSON.stringify(current.data) || draft.version !== current.version))

  const open = useCallback((tt) => {
    setSelected(tt?.id ?? null)
    setDraft(tt ? { id: tt.id, name: tt.name, data: structuredClone(tt.data), version: tt.version } : null)
    setConflict(null)
  }, [])

  useEffect(() => {
    let gone = false
    timetableApi.list(projectId).then(
      ({ timetables, mayEdit }) => {
        if (gone) return
        setList(timetables)
        setMayAdd(mayEdit)
        open(timetables.find(tt => tt.id === initial) ?? timetables[0] ?? null)
      },
      (err) => { if (!gone) { setList([]); setError(err.code ?? 'generic') } },
    )
    return () => { gone = true }
    // the list is loaded once; `initial` only says which to show first
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, open])

  // Leaving the page with changes not saved: the browser asks.
  useEffect(() => {
    if (!dirty) return undefined
    const stay = (e) => { e.preventDefault(); e.returnValue = '' }
    addEventListener('beforeunload', stay)
    return () => removeEventListener('beforeunload', stay)
  }, [dirty])

  const replaced = (tt) => setList(l => (l.some(x => x.id === tt.id) ? l.map(x => (x.id === tt.id ? tt : x)) : [...l, tt]).sort((a, b) => a.name.localeCompare(b.name, 'de', { numeric: true })))
  const act = async (work) => {
    setBusy(true)
    setError(null)
    try {
      return await work()
    } catch (err) {
      setError(err.code ?? 'generic')
      return null
    } finally {
      setBusy(false)
    }
  }
  const guard = (then) => (dirty ? setAsk({ then }) : then())

  return {
    list: list ?? [], loading: list == null, mayAdd, error, selected, current, draft, dirty, busy, conflict, ask,
    mayEdit: Boolean(current?.mayEdit),
    trains: draft?.data.trains ?? [],
    guard,
    confirm: () => { const then = ask?.then; setAsk(null); open(current); then?.() },
    dismiss: () => setAsk(null),
    select: (id) => guard(() => open(list.find(tt => tt.id === id) ?? null)),
    /** The trains changed: `fn(trains)` gives them as they are to be. */
    changeTrains: (fn) => setDraft(d => (d ? { ...d, data: { ...d.data, trains: fn(d.data.trains) } } : d)),
    rename: (name) => setDraft(d => (d ? { ...d, name } : d)),
    /** The changes dropped — for the timetable as someone else saved it, where that is what a save found. */
    discard: () => { if (conflict) { replaced(conflict); open(conflict) } else open(current) },
    async create(name) {
      return act(async () => {
        const tt = await timetableApi.create(projectId, name, newTimetable())
        replaced(tt)
        open(tt)
        return tt
      })
    },
    async save() {
      if (!draft) return null
      return act(async () => {
        try {
          const tt = await timetableApi.save(draft.id, draft.version, draft.name.trim() || current.name, draft.data)
          replaced(tt)
          open(tt)
          return tt
        } catch (err) {
          // someone else saved meanwhile: the changes stay, on top of theirs once saved again
          if (err.status === 409 && err.data?.timetable) {
            const theirs = err.data.timetable
            setConflict(theirs)
            setDraft(d => ({ ...d, version: theirs.version }))
          }
          throw err
        }
      })
    },
    async remove() {
      if (!current) return null
      return act(async () => {
        await timetableApi.remove(current.id)
        const rest = list.filter(x => x.id !== current.id)
        setList(rest)
        open(rest[0] ?? null)
        return true
      })
    },
  }
}
