import { useEffect, useRef, useState } from 'react'
import { placing } from '../model/places.js'

/**
 * Setting things on the plan while a page that creates them is open, without
 * a button: the mouse snaps to a track and the thing is drawn there as the
 * page's fields say (`offer`, on every render); a click sets it as a preview,
 * a further click moves that preview and the page's fields go on changing it
 * (`remake`), till it is committed or dropped at the panel's foot; then a
 * click sets the next one. `active` while the page open sets; `hints` the
 * mode's words, setting and moving.
 *
 * Returns { made, setMade, offer, remake }: `made` what was set last —
 * { collection, id, what } for its editor, and while it is a preview the
 * object's id, the place clicked and how to say it (`madeOf`). `offer(make,
 * madeOf)` takes what a click makes of a place — `make(hit, id)`, `id` the
 * preview's where it is made anew — and how to say it.
 */
export function useSetting({ collection, active, mode, setMode, ops, pending, hints: [setHint, moveHint], t }) {
  const owner = `setting-${collection}`
  const [made, setMade] = useState(null)
  const setting = useRef(null)
  const offer = (make, madeOf) => { setting.current = { make, madeOf } }
  const moving = !!made?.hit && pending.has(made.objectId)
  // Read when the plan is clicked, which the mode was made before.
  const latest = useRef(null)
  latest.current = { made, moving }

  /** The preview made anew — by `make`, where the plan was hit last or at `hit`. */
  const renew = (last, make, hit = last.hit) => {
    const obj = { ...make(hit, last.objectId), id: last.objectId }
    ops.update(collection, obj)
    setMade({ ...last, ...last.madeOf(obj), hit })
  }
  const settingMode = () => ({
    ...placing(collection, (hit) => setting.current?.make(hit) ?? null, (obj, hit) => {
      const { made: last, moving: move } = latest.current
      if (move) { renew(last, setting.current.make, hit); return }
      const { madeOf } = setting.current
      ops.add(collection, obj)
      setMade({ ...madeOf(obj), objectId: obj.id, hit, madeOf })
    }),
    owner, sticky: true, hint: t(moving ? moveHint : setHint),
  })

  // The plan sets while the page does — anew after every click, commit or cancel, which end the mode.
  useEffect(() => {
    if (active && !mode) setMode(settingMode())
    else if (!active && mode?.owner === owner) setMode(null)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, mode])
  useEffect(() => () => setMode(m => (m?.owner === owner ? null : m)), [setMode, owner])

  /** The preview made anew of the page's fields, where there is one not committed yet. */
  const remake = (make) => { if (moving) renew(made, make) }
  return { made, setMade, offer, remake }
}

/**
 * A page's fields, one object: a change sets them, and makes the one the
 * page set last anew of them (`remake`), while it is a preview.
 */
export function useFields(initial, remake, makeOf) {
  const [fields, setFields] = useState(initial)
  const set = (patch) => {
    const next = { ...fields, ...patch }
    setFields(next)
    remake(makeOf(next))
  }
  return [fields, set]
}
