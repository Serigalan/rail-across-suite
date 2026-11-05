import { useEffect, useMemo, useRef, useState } from 'react'
import { simulate } from '@rail-across/running-times'
import { occupancy, runKey, runTrain } from '@rail-across/timetables'

/** Runs kept for a net: beyond them the oldest are made again when asked for. */
const KEPT = 2000

/**
 * The runs of a timetable's trains over the net, made in the background
 * with the simulation module: first each train for itself, as they come;
 * then the occupancy of the blocks (occupancy.js) as the timetable's
 * `dispatch` says — the conflicts said, or the routes set as the trains
 * ask for them, a train run again where it waits or takes another route. A
 * run is made again only where what it is run with changed — its points,
 * departure, vehicle, margin, where it waits, the routes it is sent over —
 * or when the net is another. Map train id → { key, rows, route,
 * request, properties, result, blocks, conflicts, waits, delay } | { key,
 * problems } | { key, failedAt } | { key, error }; a train not run yet is
 * not in it, one whose blocks are not known yet has none.
 */
export function useRuns(record, stations, trains, catalogue, dispatch) {
  const [runs, setRuns] = useState(() => new Map())
  const made = useRef({ record: null, byKey: new Map() })
  // the catalogue is new on every load; its vehicles, as they are, decide
  const vehicles = catalogue.loading ? null : catalogue.entries.map(e => `${e.key}@${e.version ?? 0}`).join('|')
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const entries = useMemo(() => catalogue.entries, [vehicles])
  const rules = JSON.stringify(dispatch)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const settled = useMemo(() => dispatch, [rules])

  useEffect(() => {
    if (vehicles == null) return undefined
    if (made.current.record !== record) made.current = { record, byKey: new Map() }
    const byKey = made.current.byKey
    const entryOf = (train) => entries.find(e => e.key === train.vehicle) ?? null
    let gone = false
    const runOf = async (train, holds = [], routes = []) => {
      if (gone) throw new Error('gone')
      const key = holds.length || routes.length ? `${runKey(train, entryOf(train))}|${JSON.stringify(holds)}|${JSON.stringify(routes)}` : runKey(train, entryOf(train))
      if (byKey.has(key)) return byKey.get(key)
      let run
      try {
        run = { key, ...await runTrain(record, stations, train, entryOf(train)?.vehicle ?? null, simulate, holds, routes) }
      } catch (err) {
        run = { key, error: { code: err.code ?? 'internal', message: String(err.message ?? err) } }
      }
      if (byKey.size >= KEPT) byKey.delete(byKey.keys().next().value)
      byKey.set(key, run)
      return run
    }
    ;(async () => {
      const next = new Map()
      for (const train of trains) {
        const key = runKey(train, entryOf(train))
        if (byKey.has(key)) next.set(train.id, byKey.get(key))
      }
      setRuns(new Map(next))
      try {
        for (const train of trains) {
          if (next.has(train.id)) continue
          next.set(train.id, await runOf(train))
          if (gone) return
          setRuns(new Map(next))
        }
        const placed = await occupancy(record, trains, runOf, settled)
        if (!gone) setRuns(placed)
      } catch (err) {
        if (!gone) throw err
      }
    })()
    return () => { gone = true }
  }, [record, stations, trains, entries, vehicles, settled])

  return runs
}
