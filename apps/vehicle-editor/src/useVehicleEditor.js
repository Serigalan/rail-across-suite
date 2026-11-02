import { useState } from 'react'
import { newVehicle, problemsOf, suiteEntry } from '@rail-across/vehicles'

/**
 * The vehicle chosen in the catalogue and the one being edited: a vehicle
 * of the suite's changed, or a new one — empty, copied, imported — until it
 * is saved. Choosing another, or leaving, with changes not saved asks first.
 * A save that finds the vehicle saved by someone else meanwhile keeps the
 * changes and says so; saving again then puts them over the other's.
 * `initial` the key of the vehicle chosen first.
 */
export function useVehicleEditor(catalogue, initial = null) {
  const [selected, setSelected] = useState(initial)
  const [draft, setDraft] = useState(null)   // { vehicle, entry (null for a new one), notes }
  const [ask, setAsk] = useState(null)   // { kind: 'discard', then } | { kind: 'delete' }
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(null)
  const [showProblems, setShowProblems] = useState(false)

  const entry = catalogue.entries.find(e => e.key === selected) ?? null
  const dirty = Boolean(draft && (!draft.entry || JSON.stringify(draft.vehicle) !== JSON.stringify(draft.entry.vehicle)))
  const problems = draft ? problemsOf(draft.vehicle) : []

  const guard = (then) => (dirty ? setAsk({ kind: 'discard', then }) : then())
  const open = (next) => { setDraft(next); setError(null); setShowProblems(false) }

  return {
    selected, entry, draft, dirty, ask, busy, error, problems: showProblems ? problems : [],
    /** Changes not saved, the way to leave them asked about first. */
    guard,
    select: (key) => guard(() => { open(null); setSelected(key) }),
    edit: () => { if (entry?.mayEdit) open({ vehicle: structuredClone(entry.vehicle), entry, notes: [] }) },
    create: (vehicle = newVehicle(), notes = []) => guard(() => { open({ vehicle, entry: null, notes }); setSelected(null) }),
    copy: (name) => guard(() => { if (entry) open({ vehicle: { ...structuredClone(entry.vehicle), name }, entry: null, notes: entry.notes }) }),
    change: (vehicle) => setDraft(d => ({ ...d, vehicle })),
    cancel: () => guard(() => open(null)),
    confirm: () => { const then = ask?.then; setAsk(null); if (ask?.kind === 'discard') { setDraft(null); then?.() } },
    dismiss: () => setAsk(null),
    askDelete: () => setAsk({ kind: 'delete' }),
    async save() {
      if (!draft) return
      if (problems.length) { setShowProblems(true); return }
      setBusy(true)
      setError(null)
      try {
        const saved = draft.entry ? await catalogue.save(draft.entry, draft.vehicle) : await catalogue.create(draft.vehicle)
        setSelected(saved.key)
        open(null)
      } catch (err) {
        if (err.status === 409 && err.data?.vehicle) setDraft(d => ({ ...d, entry: suiteEntry(err.data.vehicle) }))
        setError(err.code ?? 'generic')
      } finally {
        setBusy(false)
      }
    },
    async remove() {
      if (!entry?.mayEdit) return
      setBusy(true)
      setError(null)
      try {
        await catalogue.remove(entry)
        setAsk(null)
        open(null)
        setSelected(null)
      } catch (err) {
        setAsk(null)
        setError(err.code ?? 'generic')
      } finally {
        setBusy(false)
      }
    },
  }
}
