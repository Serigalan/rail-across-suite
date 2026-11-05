import { useEffect, useMemo } from 'react'
import { labelOf } from '../model/domain.js'
import { speedWhereText, whereStretch } from '../model/places.js'
import { speedSignalReach } from '../model/routeSpeeds.js'
import { wayPieces } from '../model/routeSearch.js'

/** The places at either end of a stretch, to bring it into view. */
const endsOf = (s) => [s.from, s.to].map(station => ({ anchor: 'station', trackId: s.trackId, station }))

/**
 * What the elements allow and the element that sets it, which a click shows
 * on the plan: marked over what `base` draws (the way it lies on), brought
 * into view.
 */
export function ElementLink({ net, elements, base, setSketch, showOnPlan, t }) {
  const show = () => {
    const s = whereStretch(net, elements.where)
    if (!s) return
    setSketch?.({ ...base, stretches: [...(base?.stretches ?? []), { ...s, tone: 'warn' }] })
    showOnPlan?.(endsOf(s))
  }
  return (
    <span className="ui-grow ui-muted">
      {t('element_speed_is', { speed: elements.speed })}{' '}
      <button type="button" className="ui-link" title={t('element_show_hint')} onClick={show}>{speedWhereText(net, elements.where)}</button>
    </span>
  )
}

/**
 * How far an Lf 7's speed holds — up to the next Zs 3 or Lf 7 on every way
 * ahead, or the end of the track — and what the elements there allow. The
 * plan draws those ways from it while this is shown, the element that sets
 * the speed marked; `children` beside the element (taking its speed).
 */
export function SpeedReach({ signal, domain, net, setSketch, showOnPlan, t, children }) {
  const signals = domain.collections.signals
  const reach = useMemo(() => speedSignalReach(net, signals, signal), [net, signals, signal])
  const sketch = useMemo(() => {
    const element = reach.elements && whereStretch(net, reach.elements.where)
    return {
      ways: reach.ways.map(w => ({ pieces: wayPieces(w.stretches), strong: true })),
      ids: [signal.id, ...reach.ways.map(w => w.target?.id).filter(Boolean)],
      stretches: element ? [{ ...element, tone: 'warn' }] : [],
    }
  }, [reach, net, signal.id])
  useEffect(() => {
    setSketch?.(sketch)
    return () => setSketch?.(null)
  }, [sketch, setSketch])
  const ends = [...new Map(reach.ways.map(w => [w.target?.id ?? 'end', w.target ? labelOf('signals', w.target, t) : t('speed_reach_end')])).values()]
  const base = { ways: sketch.ways, ids: sketch.ids }
  return <>
    <p>{t('speed_reach')}: {ends.length ? ends.join(' · ') : t('speed_reach_none')}</p>
    {reach.elements && (
      <div className="ui-row">
        <ElementLink net={net} elements={reach.elements} base={base} setSketch={setSketch} showOnPlan={showOnPlan} t={t} />
        {children}
      </div>
    )}
  </>
}
