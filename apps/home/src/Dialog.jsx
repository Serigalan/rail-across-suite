import { FormDialog as UiFormDialog } from '@rail-across/ui'
import { useTexts } from './i18n'

export { Dialog } from '@rail-across/ui'

/** The suite's form dialog (@rail-across/ui), its cancel button in the page's language. */
export function FormDialog(props) {
  const { t } = useTexts()
  return <UiFormDialog cancelLabel={t('cancel')} {...props} />
}
