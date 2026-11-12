import { useMemo, useState } from 'react'
import {
  Button, Checkbox, LANGUAGE_NAMES, LANGUAGES, Option, Options, Panel, Rail, RailBrand, RailButton, RailDivider, Section, SelectField, Shell,
  WorkingCopyBar, Workspace,
} from '@rail-across/ui'
import { useTexts } from './i18n.js'
import { projectsUrl } from './suite.js'
import PlanView from './PlanView.jsx'
import { netLines, switchPoint } from './model/net.js'
import { STATE_COLORS, stageLabel, timeline, trackState } from './model/stages.js'
import { HomeIcon, LogoIcon, SettingsIcon, StagesIcon } from './icons.jsx'

/**
 * The Construction-Sim (decision 0027; roadmap, phase 5): a variant's net
 * construction stage by construction stage. On the left the stages — the
 * existing state, every day something is put into or taken out of service,
 * the final state — each with what it changes; on the plan the net at the
 * stage chosen, what comes into service with it in red, what leaves service
 * with the next in orange, the stage connectors as points. It only reads.
 */
export default function Editor({ project, variant, revision, record, local }) {
  const { t, language, setLanguage } = useTexts()
  const [section, setSection] = useState('stages')
  const [index, setIndex] = useState(0)
  const [others, setOthers] = useState(true)
  const [fitKey] = useState(0)
  const stages = useMemo(() => timeline(record), [record])
  const lines = useMemo(() => netLines(record?.tracks), [record])
  const at = Math.min(index, Math.max(0, stages.length - 1))
  const stage = stages[at]
  const trackName = (id) => record.tracks.find(tr => tr.id === id)?.name ?? id.slice(0, 8)
  const switchOf = (id) => (record.switches ?? []).find(sw => sw.switchId === id)
  const switchName = (id) => { const sw = switchOf(id); return sw?.name || `${t(`kind_${sw?.kind ?? 'turnout'}`)} ${id.slice(0, 6)}` }

  const states = useMemo(() => {
    const out = new Map()
    for (const id of lines.keys()) {
      const s = trackState(stages, at, id)
      if (s) out.set(id, s)
      else if (others) out.set(id, 'off')
    }
    return out
  }, [lines, stages, at, others])
  const points = useMemo(() => (record.switches ?? [])
    .filter(sw => sw.kind === 'stage_link' && stage?.switches.has(sw.switchId))
    .map(sw => ({ at: switchPoint(sw, record.tracks), color: '#7048e8' }))
    .filter(p => p.at), [record, stage])

  const panel = section === 'settings'
    ? <Panel title={t('section_settings')}>
      <Section title={t('language')}>
        <SelectField compact label={t('language')} value={language} options={LANGUAGES.map(l => ({ value: l, label: LANGUAGE_NAMES[l] }))} onChange={setLanguage} />
      </Section>
    </Panel>
    : <Panel title={t('section_stages')} intro={t('section_stages_intro')}>
      <Options>
        {stages.map((s, i) => (
          <Option key={s.key} icon={<StagesIcon />} active={i === at} onClick={() => setIndex(i)}
            end={i > 0 ? `+${s.came.tracks.length} −${s.went.tracks.length}` : `${s.tracks.size}`}>
            {stageLabel(s, t, language)}
          </Option>
        ))}
      </Options>
      {stages.length <= 2 && <p className="ui-muted">{t('stages_no_days')}</p>}
      {stage && (
        <Section title={stageLabel(stage, t, language)}>
          <p className="ui-muted">{t('stage_in_service', { tracks: stage.tracks.size, switches: stage.switches.size })}</p>
          <Changes title={t('stage_came')} color={STATE_COLORS.came} items={[...stage.came.tracks.map(trackName), ...stage.came.switches.map(switchName)]} />
          <Changes title={t('stage_went')} color={STATE_COLORS.goes} items={[...stage.went.tracks.map(trackName), ...stage.went.switches.map(switchName)]} />
        </Section>
      )}
    </Panel>

  return (
    <Shell>
      <Rail label={t('tool')}
        brand={<RailBrand href={projectsUrl()} label={t('to_projects')} name={t('tool')}><LogoIcon /></RailBrand>}
        top={<RailButton icon={<StagesIcon />} label={t('section_stages')} pressed={section === 'stages'} onClick={() => setSection('stages')} />}
        bottom={<>
          <RailDivider />
          <RailButton icon={<HomeIcon />} label={t('to_projects')} onClick={() => location.assign(projectsUrl())} />
          <RailButton icon={<SettingsIcon />} label={t('section_settings')} pressed={section === 'settings'} onClick={() => setSection('settings')} />
        </>}
      />
      {panel}
      <Workspace>
        <WorkingCopyBar title={project.title} subtitle={variant.name} revision={t('revision_n', { n: revision?.number ?? '—' })}
          note={t(local ? 'note_working_copy' : 'note_head')}
          aside={stage && (
            <span className="stage-step">
              <Button size="small" disabled={at === 0} onClick={() => setIndex(at - 1)} aria-label={t('stage_before')}>‹</Button>
              <select value={at} onChange={e => setIndex(Number(e.target.value))} aria-label={t('stage')}>
                {stages.map((s, i) => <option key={s.key} value={i}>{stageLabel(s, t, language)}</option>)}
              </select>
              <Button size="small" disabled={at >= stages.length - 1} onClick={() => setIndex(at + 1)} aria-label={t('stage_after')}>›</Button>
            </span>
          )} />
        <div className="stage-legend">
          {['stays', 'came', 'goes'].map(k => <span key={k}><i style={{ background: STATE_COLORS[k] }} />{t(`legend_${k}`)}</span>)}
          <Checkbox label={t('legend_others')} checked={others} onChange={setOthers} />
        </div>
        <PlanView lines={lines} states={states} points={points} fitKey={fitKey} />
      </Workspace>
    </Shell>
  )
}

function Changes({ title, color, items }) {
  if (!items.length) return null
  return (
    <div className="stage-changes">
      <p><i style={{ background: color }} /><b>{title}</b></p>
      <ul>{items.map((name, i) => <li key={i}>{name}</li>)}</ul>
    </div>
  )
}
