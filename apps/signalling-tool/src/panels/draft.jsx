import { Button, Panel as Frame } from '@rail-across/ui'
import { useDraft } from './draftContext.js'

/**
 * A panel of the tool: the suite's, with Cancel and Commit at its foot where
 * it changes the domain — as the Layout-Tool's dialogs have them. What is
 * changed is drawn as a preview and holds once committed.
 */
export function Panel({ foot, ...props }) {
  const draft = useDraft()
  return <Frame {...props} foot={draft ? <CommitBar draft={draft} /> : foot} />
}

/** Cancel and Commit side by side, Cancel on the left; above them what would be committed. */
function CommitBar({ draft: { open, what, commit, cancel, t } }) {
  return (
    <div className="tool-commit">
      <p className={`tool-commit-what${open ? '' : ' ui-muted'}`} role="status">{open ? t('draft_open', { what }) : t('draft_none')}</p>
      <div className="tool-commit-row">
        <Button disabled={!open} onClick={cancel}>{t('cancel')}</Button>
        <Button variant="primary" disabled={!open} title={open ? undefined : t('draft_none')} onClick={commit}>{t('commit')}</Button>
      </div>
    </div>
  )
}
