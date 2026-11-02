import { useEffect, useState } from 'react'
import { Rail, RailBrand, RailButton, RailDivider, Shell, Workspace } from '@rail-across/ui'
import { useCatalogue } from '@rail-across/vehicles'
import { useTexts } from './i18n.js'
import { HomeIcon, LogoIcon, SettingsIcon, TrainIcon } from './icons.jsx'
import SettingsPanel from './panels/SettingsPanel.jsx'
import Fleet from './Fleet.jsx'
import { useVehicleEditor } from './useVehicleEditor.js'
import { keepVehicleInAddress, requestedVehicle, suiteUrl } from './suite.js'

/**
 * The Fahrzeug-Editor: the catalogue of the suite's vehicles and OSRD's
 * examples in the panel, the vehicle chosen on the page beside it, as OSRD's
 * rolling stock editor has them. Leaving with changes not saved asks first.
 */
export default function Editor() {
  const { t, language, setLanguage } = useTexts()
  const [section, setSection] = useState('vehicles')
  const catalogue = useCatalogue()
  const editor = useVehicleEditor(catalogue, requestedVehicle())
  useEffect(() => { keepVehicleInAddress(editor.selected) }, [editor.selected])
  const show = (next) => (section === 'vehicles' && next !== 'vehicles' ? editor.guard(() => setSection(next)) : setSection(next))
  return (
    <Shell>
      <Rail label={t('tool')}
        brand={<RailBrand href={suiteUrl()} label={t('suite')} name={t('tool')}><LogoIcon /></RailBrand>}
        top={<RailButton icon={<TrainIcon />} label={t('section_vehicles')} pressed={section === 'vehicles'} onClick={() => show('vehicles')} />}
        bottom={<>
          <RailDivider />
          <RailButton icon={<HomeIcon />} label={t('to_suite')} onClick={() => editor.guard(() => location.assign(suiteUrl()))} />
          <RailButton icon={<SettingsIcon />} label={t('section_settings')} pressed={section === 'settings'} onClick={() => show('settings')} />
        </>}
      />
      {section === 'vehicles'
        ? <Fleet t={t} language={language} catalogue={catalogue} editor={editor} />
        : <><SettingsPanel t={t} language={language} setLanguage={setLanguage} /><Workspace /></>}
    </Shell>
  )
}
