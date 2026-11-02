import { Badge, Button, Message, Panel, PlusIcon, Section, SelectField, SelectList } from '@rail-across/ui'
import { categoryColor, trainTitle } from '@rail-across/timetables'
import { trainLine } from '../rows.js'

/**
 * The timetables of the project — the one shown, a new one, another name,
 * deleting it — and its trains by their departure, each with where it begins
 * and ends and when, marked where it cannot be run.
 */
export default function TrainsPanel({ t, timetables, sorted, train, setTrainId, runs, stations, mayEdit, addTrain, setDialog }) {
  const { list, loading, mayAdd, draft, error } = timetables
  const items = sorted.map(tr => {
    const run = runs.get(tr.id)
    const bad = run && !run.rows
    return {
      id: tr.id,
      mark: <span className="tt-dot" style={{ background: categoryColor(tr.category) }} />,
      title: trainTitle(tr) || t('train_unnamed'),
      sub: trainLine(tr, run),
      end: bad ? <Badge tone="warn" title={t('train_cannot_run')}>!</Badge> : null,
    }
  })
  return (
    <Panel title={t('section_trains')} intro={t('section_trains_intro')}
      actions={mayEdit && draft && <Button size="small" icon={<PlusIcon size={14} />} onClick={addTrain}>{t('train_new')}</Button>}>
      {error && <Message tone="error">{t('error_save', { error })}</Message>}
      {!stations.some(s => s.kind === 'station') && <Message tone="warn">{t('no_stations')}</Message>}
      <Section title={t('timetable')}
        actions={mayEdit && draft && <span className="ui-row">
          <Button size="small" variant="ghost" onClick={() => setDialog({ kind: 'rename' })}>{t('rename')}</Button>
          <Button size="small" variant="ghost" onClick={() => setDialog({ kind: 'delete' })}>{t('delete')}</Button>
        </span>}>
        {loading ? <p className="ui-muted">{t('loading')}</p> : list.length > 0 && (
          <div className="ui-fields">
            <SelectField wide label={t('timetable')} value={timetables.selected} onChange={(id) => id && timetables.select(id)}
              options={list.map(tt => ({ value: tt.id, label: tt.name }))} />
          </div>
        )}
        {!loading && !list.length && <p className="ui-muted">{t('timetables_none')}</p>}
        {mayAdd && !loading && <Button size="small" icon={<PlusIcon size={14} />} onClick={() => setDialog({ kind: 'new' })}>{t('timetable_new')}</Button>}
      </Section>
      {draft && (
        <Section title={t('trains_n', { n: sorted.length })}>
          <SelectList label={t('section_trains')} items={items} selected={train?.id} onSelect={setTrainId} empty={t('trains_none')} />
        </Section>
      )}
    </Panel>
  )
}
