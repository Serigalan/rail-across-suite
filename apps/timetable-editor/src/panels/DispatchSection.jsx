import { IconButton, NumberField, Section, SelectField } from '@rail-across/ui'
import { dispatchOf } from '@rail-across/timetables'
import { DownIcon, UpIcon } from '../icons.jsx'

/**
 * What the timetable does where trains would hold the same block: the
 * conflicts only said, or the trains waiting for a free block — the one
 * given the net first by its departure, or by its kind in the order set
 * here —; and the times a block is held beyond the train's own running in it.
 */
export default function DispatchSection({ t, timetables, mayEdit }) {
  const d = dispatchOf(timetables.draft?.data)
  const ro = !mayEdit
  const set = (patch) => timetables.changeDispatch({ ...d, ...patch })
  const setTime = (key, value) => value != null && value >= 0 && set({ times: { ...d.times, [key]: value } })
  // the kinds set, then those the trains have that are not among them
  const kinds = [...d.categories, ...new Set(timetables.trains.map(tr => String(tr.category ?? '').trim()).filter(c => c && !d.categories.includes(c)))]
  const move = (i, by) => {
    const next = [...kinds]
    ;[next[i], next[i + by]] = [next[i + by], next[i]]
    set({ categories: next })
  }
  return (
    <Section title={t('dispatch')}>
      <p className="ui-muted">{t('dispatch_intro')}</p>
      <div className="ui-fields">
        <SelectField wide label={t('dispatch_mode')} value={d.mode} disabled={ro} onChange={(mode) => mode && set({ mode })}
          options={[{ value: 'wait', label: t('dispatch_wait') }, { value: 'show', label: t('dispatch_show') }]} />
        {d.mode === 'wait' && (
          <SelectField wide label={t('dispatch_priority')} value={d.priority} disabled={ro} onChange={(priority) => priority && set({ priority })}
            options={[{ value: 'departure', label: t('priority_departure') }, { value: 'category', label: t('priority_category') }]} />
        )}
      </div>
      {d.mode === 'wait' && d.priority === 'category' && (
        <>
          <p className="ui-muted">{t('priority_order')}</p>
          <ol className="tt-kinds">
            {kinds.map((c, i) => (
              <li key={c}>
                <span className="ui-grow">{c}</span>
                <IconButton size="small" variant="ghost" icon={<UpIcon />} label={t('priority_up', { kind: c })} disabled={ro || i === 0} onClick={() => move(i, -1)} />
                <IconButton size="small" variant="ghost" icon={<DownIcon />} label={t('priority_down', { kind: c })} disabled={ro || i === kinds.length - 1} onClick={() => move(i, 1)} />
              </li>
            ))}
          </ol>
        </>
      )}
      <div className="ui-fields tt-fields-3">
        <NumberField compact label={t('blocking_setup')} value={d.times.setup} disabled={ro} onChange={(v) => setTime('setup', v)} />
        <NumberField compact label={t('blocking_approach')} value={d.times.approach} disabled={ro} onChange={(v) => setTime('approach', v)} />
        <NumberField compact label={t('blocking_release')} value={d.times.release} disabled={ro} onChange={(v) => setTime('release', v)} />
      </div>
    </Section>
  )
}
