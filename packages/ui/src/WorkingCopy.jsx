/**
 * The bar of the open working copy over the map or plan, as the Layout-Tool
 * has it: which project (`title`) and variant (`subtitle`), the revision it
 * rests on (`revision`, its words), and its state as a pill — `state` clean,
 * local or newer, `stateLabel` its words, `stateNote` a stronger word after
 * them (the server is newer), `onState` what a click on it does (showing the
 * changes; without it the pill only says) —, or `note` where it has no state
 * (read-only). `actions` [{ label, onClick, disabled, primary }] the ways to
 * act on it: update, discard, check in, or back.
 */
export function WorkingCopyBar({ title, subtitle, revision, state, stateLabel, stateNote, stateTitle, onState, note, actions = [] }) {
  return (
    <div className="ui-wc-bar" role="status">
      <span className="ui-wc-head">
        <span className="ui-wc-where">
          <strong>{title}</strong>
          {subtitle && <><span className="ui-wc-sep">›</span><span>{subtitle}</span></>}
          {revision && <span className="ui-wc-rev">{revision}</span>}
        </span>
        {state && (
          <button type="button" className={`ui-wc-state ui-wc-state-${state}`} title={stateTitle} onClick={onState} disabled={!onState}>
            {stateLabel}
            {stateNote && <span className="ui-wc-newer">{stateNote}</span>}
          </button>
        )}
        {note && <span className="ui-wc-rev">{note}</span>}
      </span>
      {actions.length > 0 && (
        <span className="ui-wc-actions">
          {actions.map(a => (
            <button key={a.label} type="button" className={`ui-wc-btn${a.primary ? ' ui-wc-btn-primary' : ''}`} onClick={a.onClick} disabled={a.disabled}>
              {a.icon}{a.label}
            </button>
          ))}
        </span>
      )}
    </div>
  )
}
