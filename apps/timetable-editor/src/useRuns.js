import { useEffect, useMemo, useRef, useState } from 'react'
import { simulate } from '@rail-across/running-times'
import { runKey, runTrain } from '@rail-across/timetables'

/**
 * The runs of a timetable's trains over the net, made in the background one
 * after the other with the simulation module, and made again only for a
 * train whose run would change — its points, departure, vehicle, margin —
 * or when the net is another. Map train id → { key, rows, route, request,
 * properties, result } | { key, problems } | { key, failedAt } | { key,
 * error }; a train not run yet is not in it.
 */
export function useRuns(record, stations, trains, catalogue) {
  const [runs, setRuns] = useState(() => new Map())
  const made = useRef({ record: null, byKey: new Map() })
  // the catalogue is new on every load; its vehicles, as they are, decide
  const vehicles = catalogue.loading ? null : catalogue.entries.map(e => `${e.key}@${e.version ?? 0}`).join('|')
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const entries = useMemo(() => catalogue.entries, [vehicles])

  useEffect(() => {
    if (vehicles == null) return undefined
    if (made.current.record !== record) made.current = { record, byKey: new Map() }
    const byKey = made.current.byKey
    const entryOf = (train) => entries.find(e => e.key === train.vehicle) ?? null
    let gone = false
    ;(async () => {
      const next = new Map()
      const todo = []
      for (const train of trains) {
        const key = runKey(train, entryOf(train))
        if (byKey.has(key)) next.set(train.id, byKey.get(key))
        else todo.push({ train, key })
      }
      setRuns(new Map(next))
      for (const { train, key } of todo) {
        let run
        try {
          run = { key, ...await runTrain(record, stations, train, entryOf(train)?.vehicle ?? null, simulate) }
        } catch (err) {
          run = { key, error: { code: err.code ?? 'internal', message: String(err.message ?? err) } }
        }
        if (gone) return
        byKey.set(key, run)
        next.set(train.id, run)
        setRuns(new Map(next))
      }
    })()
    return () => { gone = true }
  }, [record, stations, trains, entries, vehicles])

  return runs
}
