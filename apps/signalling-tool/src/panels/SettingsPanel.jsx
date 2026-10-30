import { LANGUAGE_NAMES, Panel, SelectField } from '@rail-across/ui'

/**
 * The tool's own settings, as the Layout-Tool has them: the language — the
 * suite's, so the start page and every other module follow it.
 */
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
