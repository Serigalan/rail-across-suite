import { Button, Panel, Section, SelectField } from '@rail-across/ui'
import { formatClock, sectionsAt, trainAt, trainTitle } from '@rail-across/timetables'

/** How much faster than the clock the timetable can be played. */
const PLAY_SPEEDS = [1, 10, 30, 60, 120, 300]

/**
 * The timetable played on the map (Zugfahrten): the moment shown, played on
 * at a speed or moved by hand; which track sections are shown with each
 * train — none, those it occupies, or those its routes hold for it too —;
 * and every train in its colour with where it is: not yet left, running at
 * a speed, standing, arrived, the sections it occupies. A click on a train
 * brings it into view on the map.
 */
export default function LivePanel({ t, record, sorted, runs, colors, play, setPlay, span, onFocus }) {
  const time = play.time ?? span?.from ?? 0
  const sectionName = (id) => record?.signalling?.collections?.trackSections?.find(x => x.id === id)?.name ?? '?'
  const step = (d) => setPlay(p => ({ ...p, time: Math.min(span.to, Math.max(span.from, time + d)) }))
  const ready = sorted.filter(tr => runs.get(tr.id)?.clock)
  return (
    <Panel title={t('section_live')} intro={t('section_live_intro')}>
      {!span
        ? <p className="ui-muted">{t('live_none')}</p>
        : <>
          <Section title={t('live_clock')}>
            <p className="tt-live-clock" aria-live="off">{formatClock(time, { seconds: true })}</p>
            <input type="range" className="tt-live-range" min={Math.floor(span.from)} max={Math.ceil(span.to)} step={1} value={Math.round(time)}
              aria-label={t('live_clock')} onChange={(e) => setPlay(p => ({ ...p, time: Number(e.target.value) }))} />
            <div className="ui-row">
              <Button size="small" onClick={() => step(-60)}>−1 min</Button>
              <Button size="small" variant="primary" onClick={() => setPlay(p => ({ ...p, playing: !p.playing, time: p.time ?? span.from }))}>
                {play.playing ? t('live_pause') : t('live_play')}
              </Button>
              <Button size="small" onClick={() => step(60)}>+1 min</Button>
            </div>
            <div className="ui-fields">
              <SelectField compact label={t('live_speed')} value={String(play.speed)} onChange={(v) => v && setPlay(p => ({ ...p, speed: Number(v) }))}
                options={PLAY_SPEEDS.map(s => ({ value: String(s), label: `${s}×` }))} />
              <SelectField wide label={t('live_sections')} value={play.sections} onChange={(v) => v && setPlay(p => ({ ...p, sections: v }))}
                options={['none', 'occupied', 'reserved'].map(v => ({ value: v, label: t(`live_sections_${v}`) }))} />
            </div>
          </Section>
          <Section title={t('live_trains')}>
            <ul className="tt-live-trains">
              {ready.map(tr => {
                const run = runs.get(tr.id)
                const at = trainAt(run, time)
                const shown = play.sections !== 'none' && (at.state === 'running' || at.state === 'standing')
                  ? sectionsAt(record, run, time, { reserved: play.sections === 'reserved' }) : null
                return (
                  <li key={tr.id}>
                    <button type="button" className="tt-live-train" onClick={() => onFocus(tr.id)}>
                      <span className="tt-live-swatch" style={{ background: colors.get(tr.id) }} aria-hidden="true" />
                      <span className="tt-live-name" style={{ color: colors.get(tr.id) }}>{trainTitle(tr) || t('train_unnamed')}</span>
                      <span className="tt-live-state">{stateText(t, at, run)}</span>
                    </button>
                    {shown && (shown.occupied.length > 0 || shown.reserved.length > 0) && (
                      <span className="tt-live-sections">
                        {shown.occupied.map(sectionName).join(', ')}
                        {shown.reserved.length > 0 && <span className="ui-muted"> · {t('live_reserved_list', { list: shown.reserved.map(sectionName).join(', ') })}</span>}
                      </span>
                    )}
                  </li>
                )
              })}
            </ul>
            {ready.length < sorted.length && <p className="ui-muted">{t('live_some_missing', { n: sorted.length - ready.length })}</p>}
          </Section>
        </>}
    </Panel>
  )
}

/** Where a train is, in words: not yet left (from when), running at its speed, standing, arrived (when). */
function stateText(t, at, run) {
  if (at.state === 'before') return t('live_before', { time: formatClock(run.clock.at0) })
  if (at.state === 'arrived') return t('live_arrived', { time: formatClock(run.clock.end) })
  if (at.state === 'standing') return t('live_standing')
  return t('live_running', { speed: Math.round(at.speed * 3.6) })
}
