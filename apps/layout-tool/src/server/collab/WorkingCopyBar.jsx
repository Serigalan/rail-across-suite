import { WorkingCopyBar as Bar } from '@rail-across/ui'
import { useI18n } from '../../core/locales/i18nContext'

/**
 * The status of the open working copy, at the top of the app (AP 10.6):
 * which project and variant, the revision it rests on, and whether it is all
 * checked in, has local changes, or the server has moved on — with the ways
 * to act on that: update, throw the local changes away, check in. The bar is
 * the suite's (@rail-across/ui), the Signalling-Tool's too.
 */
export default function WorkingCopyBar({ projectTitle, variantName, base, changes, serverNewer, busy, onCheckIn, onUpdate, onDiscard, onShowChanges }) {
  const { t, fill } = useI18n()
  return (
    <Bar title={projectTitle} subtitle={variantName} revision={base ? fill('wc_rev', { n: base.number }) : null}
      state={serverNewer ? 'newer' : changes > 0 ? 'local' : 'clean'}
      stateLabel={changes > 0 ? fill('wc_changes', { n: changes }) : t('wc_clean')}
      stateNote={serverNewer ? t('wc_server_newer') : null}
      onState={changes ? onShowChanges : undefined}
      actions={[
        { label: t('wc_update'), onClick: onUpdate, disabled: busy },
        { label: t('wc_discard'), onClick: onDiscard, disabled: busy || !changes },
        { label: t('wc_checkin'), onClick: onCheckIn, disabled: busy || !changes, primary: true },
      ]} />
  )
}
