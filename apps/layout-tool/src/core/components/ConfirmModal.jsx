import { useI18n } from '../locales/i18nContext'
import Modal from './Modal'

/** A question with two answers — the one that does something, and Cancel. */
export default function ConfirmModal({ message, onConfirm, onCancel, confirmLabel, cancelLabel, busy = false, danger = true }) {
  const { t } = useI18n()
  return (
    <Modal onClose={onCancel} busy={busy} ariaLabel={message} initialFocus=".ui-btn-danger, .ui-btn-primary"
      actions={<>
        <button type="button" className="ui-btn" onClick={onCancel} disabled={busy}>
          {cancelLabel ?? t('btn_cancel')}
        </button>
        <button type="button" className={`ui-btn ${danger ? 'ui-btn-danger' : 'ui-btn-primary'}`} onClick={onConfirm} disabled={busy}>
          {confirmLabel ?? t('modal_delete')}
        </button>
      </>}>
      <p className="modal-message">{message}</p>
    </Modal>
  )
}
