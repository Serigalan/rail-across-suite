import { useState } from 'react'
import { Button, Checkbox, PlusIcon, Section } from '@rail-across/ui'
import { SIGNAL_CATALOGUE, aspectOf, titleIn } from '@rail-across/core/signalling'

/** The aspects that may stand on a frame of that kind, for a signal of that kind (indicators: those it carries). */
function choicesFor(frameKind, kind) {
  const all = SIGNAL_CATALOGUE.begriffe.filter(b => !b.nur_dv && (!b.rahmen || b.rahmen.includes(frameKind)))
  if (frameKind === 'indicator' && kind) return all.filter(b => kind.zusatz.includes(b.id))
  return all
}

/**
 * Frames and what they show — a logical signal's (its screen as the set of
 * what its kind may show, what it must marked, and its indicators) or a
 * place's (boards, a clearance marker's body, the name plate, and a screen
 * where it has no logical signal). `kind` is the catalogue's kind of signal
 * whose screen it is, or null; `adds` the kinds of frame that can be added.
 */
export default function FramesEditor({ frames = [], kind, adds, onChange, readOnly, t, language, title }) {
  const screenAt = frames.findIndex(f => f.kind === 'screen')
  const setFrame = (i, frame) => onChange(frames.map((f, j) => (j === i ? frame : f)))
  const removeFrame = (i) => onChange(frames.filter((_, j) => j !== i))
  return (
    <Section title={title ?? t('frames')}>
      {kind && (
        <div className="tool-frame">
          <div className="tool-frame-head">{t('frame_screen')}</div>
          <div className="tool-checklist">
            {kind.schirm.erlaubt.map(id => {
              const on = screenAt >= 0 && frames[screenAt].aspects.some(a => a.aspect === id)
              const must = kind.schirm.pflicht.includes(id)
              return (
                <Checkbox key={id} checked={on} disabled={readOnly || (must && on)}
                  label={<>{aspectOf(id)?.name ?? id}{must && <span className="tool-must"> · {t('must')}</span>}<span className="ui-muted tool-aspect-title"> {titleIn(aspectOf(id), language)}</span></>}
                  onChange={(v) => {
                    const screen = screenAt >= 0 ? frames[screenAt] : { kind: 'screen', aspects: [] }
                    const aspects = v ? [...screen.aspects, { aspect: id }] : screen.aspects.filter(a => a.aspect !== id)
                    if (screenAt >= 0) setFrame(screenAt, { ...screen, aspects })
                    else onChange([{ ...screen, aspects }, ...frames])
                  }} />
              )
            })}
          </div>
        </div>
      )}
      {frames.map((f, i) => ((kind && i === screenAt) ? null : (
        <FrameEditor key={i} frame={f} kind={kind} readOnly={readOnly} t={t} language={language} onChange={(fr) => setFrame(i, fr)} onRemove={() => removeFrame(i)} />
      )))}
      {!readOnly && adds.length > 0 && (
        <div className="ui-row">
          {adds.filter(a => a !== 'nameplate' || !frames.some(f => f.kind === 'nameplate')).filter(a => a !== 'screen' || screenAt < 0).map(a => (
            <Button key={a} size="small" icon={<PlusIcon size={14} />} onClick={() => onChange([...frames, { kind: a, aspects: [] }])}>{t(`frame_${a}`)}</Button>
          ))}
        </div>
      )}
    </Section>
  )
}

function FrameEditor({ frame, kind, onChange, onRemove, readOnly, t, language }) {
  const [adding, setAdding] = useState('')
  const choices = choicesFor(frame.kind, kind)
  const setAspect = (i, a) => onChange({ ...frame, aspects: frame.aspects.map((x, j) => (j === i ? a : x)) })
  return (
    <div className="tool-frame">
      <div className="tool-frame-head">
        <span className="ui-grow">{t(`frame_${frame.kind}`)}</span>
        {!readOnly && <button type="button" className="ui-link" onClick={onRemove}>{t('remove')}</button>}
      </div>
      {frame.aspects.map((a, i) => {
        const def = aspectOf(a.aspect)
        return (
          <div key={i} className="tool-aspect-row">
            <span className="ui-grow" title={titleIn(def, language)}>{def?.name ?? a.aspect}<span className="ui-muted tool-aspect-title"> {titleIn(def, language)}</span></span>
            {(def?.symbole || def?.symbol_frei) && (def?.symbole ? (
              <select className="ui-input tool-symbol" aria-label={t('symbol')} value={a.symbol ?? ''} disabled={readOnly}
                onChange={(e) => setAspect(i, { ...a, symbol: e.target.value || undefined })}>
                <option value="">{t('symbol')}</option>
                {def.symbole.map(s => <option key={s} value={s}>{def.symbol_bedeutung === 'speed' ? `${s} (${Number(s) * 10} km/h)` : s}</option>)}
              </select>
            ) : (
              <input className="ui-input tool-symbol" aria-label={t('symbol')} value={a.symbol ?? ''} disabled={readOnly}
                onChange={(e) => setAspect(i, { ...a, symbol: e.target.value || undefined })} />
            ))}
            {!readOnly && <button type="button" className="ui-link" aria-label={t('remove')} onClick={() => onChange({ ...frame, aspects: frame.aspects.filter((_, j) => j !== i) })}>×</button>}
          </div>
        )
      })}
      {!readOnly && frame.kind !== 'nameplate' && (
        <select className="ui-input" aria-label={t('aspect')} value={adding}
          onChange={(e) => { const v = e.target.value; setAdding(''); if (v) onChange({ ...frame, aspects: [...frame.aspects, { aspect: v }] }) }}>
          <option value="">+ {t('aspect')}</option>
          {choices.map(b => <option key={b.id} value={b.id}>{b.name}{b.title ? ` — ${titleIn(b, language)}` : ''}</option>)}
        </select>
      )}
    </div>
  )
}
