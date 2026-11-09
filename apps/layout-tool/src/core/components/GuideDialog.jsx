import { createPortal } from 'react-dom'
import Modal from './Modal'
import { useI18n } from '../locales/i18nContext'

/**
 * The guideline is a static page of its own per language, served from `public`.
 * A language without one falls back to the German original.
 */
const GUIDE_PAGES = { de: './guideline.html', en: './guideline_en.html' }

const guidePage = (language) => GUIDE_PAGES[language] ?? GUIDE_PAGES.de

/**
 * The whole guideline in a window of its own over the page: opened from an
 * empty project's first steps. Put on the
 * body, or the panel it is opened from would keep it under the sidebar.
 */
export default function GuideDialog({ onClose }) {
  const { t, language } = useI18n()
  return createPortal(
    <Modal title={t('start_guideline')} onClose={onClose} className="modal-guide"
      actions={(
        <>
          <a href="./Leitfaden_Trassierung.pdf" download="Leitfaden_Trassierung.pdf" className="ui-btn link-as-btn">
            {t('start_guideline_download')}
          </a>
          <button type="button" className="ui-btn ui-btn-primary" onClick={onClose}>{t('btn_close')}</button>
        </>
      )}>
      <iframe className="guide-frame" src={guidePage(language)} title={t('start_guideline')} />
    </Modal>,
    document.body,
  )
}
