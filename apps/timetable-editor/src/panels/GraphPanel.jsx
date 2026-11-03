import { useState } from 'react'
import { Checkbox, Panel, Section, SelectField, TextField } from '@rail-across/ui'
import { formatClock, parseClock, trainTitle } from '@rail-across/timetables'

/**
 * The graphical timetable's choices: the train whose way is the axis — the
 * stations along it, across —, the hours shown and how tall a minute is.
 */
export default function GraphPanel({ t, sorted, train, runs, axisId, setAxisId, view, setView }) {
  const runnable = sorted.filter(tr => runs.get(tr.id)?.rows)
  const axis = runnable.find(tr => tr.id === axisId) ?? (runs.get(train?.id)?.rows ? train : null)
  return (
    <Panel title={t('section_graph')} intro={t('section_graph_intro')}>
      <Section title={t('graph_axis')}>
        <div className="ui-fields">
          <SelectField wide label={t('graph_axis_train')} value={axis?.id ?? null} empty={t('choose')} onChange={(id) => setAxisId(id)}
            options={runnable.map(tr => ({ value: tr.id, label: `${trainTitle(tr) || t('train_unnamed')} · ${runs.get(tr.id).rows[0].name} → ${runs.get(tr.id).rows.at(-1).name}` }))} />
        </div>
        <p className="ui-muted">{t('graph_axis_note')}</p>
      </Section>
      <Section title={t('graph_window')}>
        <div className="ui-fields tt-fields-2">
          <HourField t={t} label={t('graph_from')} value={view.from} onChange={(from) => setView(v => ({ ...v, from }))} />
          <HourField t={t} label={t('graph_to')} value={view.to} onChange={(to) => setView(v => ({ ...v, to }))} />
        </div>
        <div className="ui-fields">
          <SelectField compact label={t('graph_scale')} value={String(view.scale)} onChange={(s) => s && setView(v => ({ ...v, scale: Number(s) }))}
            options={[2, 4, 8, 12].map(s => ({ value: String(s), label: t('graph_scale_n', { n: s }) }))} />
        </div>
        <Checkbox label={t('graph_blocks')} checked={view.blocks} onChange={(blocks) => setView(v => ({ ...v, blocks }))} />
      </Section>
    </Panel>
  )
}

/** An hour to show from or to — empty for the trains' own. */
function HourField({ t, label, value, onChange }) {
  const [text, setText] = useState(value == null ? '' : formatClock(value))
  const bad = text.trim() && parseClock(text) == null
  return (
    <TextField compact label={label} value={text} placeholder={t('graph_auto')} hint={bad ? t('clock_invalid') : undefined}
      onChange={(v) => {
        setText(v)
        if (!v.trim()) onChange(null)
        else if (parseClock(v) != null) onChange(parseClock(v))
      }} />
  )
}
