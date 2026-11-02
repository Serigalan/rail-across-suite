import { LANGUAGE_NAMES, Panel, SelectField } from '@rail-across/ui'

/** The settings: the language, the suite's choice for every module. */
export default function SettingsPanel({ t, language, setLanguage }) {
  return (
    <Panel title={t('section_settings')} intro={t('section_settings_intro')}>
      <div className="ui-fields">
        <SelectField wide label={t('language')} value={language}
          options={Object.entries(LANGUAGE_NAMES).map(([value, label]) => ({ value, label }))}
          onChange={(v) => v && setLanguage?.(v)} />
      </div>
    </Panel>
  )
}
