/**
 * The shell of a tool, as the Layout-Tool has it: the icon rail in the
 * primary colour on the left, the panel of what is chosen beside it, the
 * workspace — a map, a plan — filling the rest. On a narrow screen the rail
 * runs across the top and the panel lies above the workspace.
 */
export function Shell({ children }) {
  return <div className="ui-shell">{children}</div>
}

/** The icon rail: `top` and `bottom` its two groups; `brand` the suite's mark above them. */
export function Rail({ label, brand, top, bottom }) {
  return (
    <nav className="ui-rail" aria-label={label}>
      <div className="ui-rail-group">{brand}{top}</div>
      <div className="ui-rail-group">{bottom}</div>
    </nav>
  )
}

/** The suite's mark at the top of the rail, leading back to the suite. */
export function RailBrand({ href, label, src }) {
  return <a className="ui-rail-brand" href={href} title={label} aria-label={label}><img src={src} alt="" /></a>
}

/**
 * One button of the rail: its symbol, `label` its name (title and for screen
 * readers), `pressed` while its panel shows; `count` a number in a dot —
 * `countTone` warn for a yellow one, else red.
 */
export function RailButton({ icon, label, pressed, onClick, count = 0, countTone, disabled }) {
  return (
    <button type="button" className="ui-rail-btn" aria-label={label} title={label} aria-pressed={pressed ?? undefined}
      onClick={onClick} disabled={disabled}>
      {icon}
      {count > 0 && <span className={`ui-rail-count${countTone === 'warn' ? ' ui-rail-count-warn' : ''}`} aria-hidden="true">{count > 99 ? '99+' : count}</span>}
    </button>
  )
}

export const RailDivider = () => <hr className="ui-rail-divider" />

/** The panel beside the rail: a heading with its `intro`, the body that scrolls, a `foot` that stays. */
export function Panel({ title, intro, actions, foot, label, children }) {
  return (
    <aside className="ui-panel" aria-label={label ?? (typeof title === 'string' ? title : undefined)}>
      <div className="ui-panel-head">
        <h2>{title}</h2>
        {intro && <p>{intro}</p>}
        {actions && <div className="ui-row">{actions}</div>}
      </div>
      <div className="ui-panel-body">{children}</div>
      {foot && <div className="ui-panel-foot">{foot}</div>}
    </aside>
  )
}

/** A part of a panel under a small heading; `actions` beside the heading. */
export function Section({ title, actions, children }) {
  return (
    <section className="ui-section">
      {title && <h3 className="ui-section-title"><span className="ui-grow">{title}</span>{actions}</h3>}
      {children}
    </section>
  )
}

/** The workspace: a `bar` across its top, then what it shows. */
export function Workspace({ bar, children }) {
  return (
    <main className="ui-workspace">
      {bar && <div className="ui-topbar">{bar}</div>}
      {children}
    </main>
  )
}
