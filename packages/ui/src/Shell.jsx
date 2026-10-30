/**
 * The shell of a tool, as the Layout-Tool has it: the rail in the primary
 * colour on the left, its buttons named beside their symbols, the panel of
 * what is chosen beside it, the workspace — a map, a plan — filling the rest.
 * On a narrow screen the rail runs across the top, symbols only, and the
 * panel lies above the workspace.
 */
export function Shell({ children }) {
  return <div className="ui-shell">{children}</div>
}

/** The rail: `top` and `bottom` its two groups; `brand` the tool's mark above them. */
export function Rail({ label, brand, top, bottom }) {
  return (
    <nav className="ui-rail" aria-label={label}>
      <div className="ui-rail-group">{brand}{top}</div>
      <div className="ui-rail-group">{bottom}</div>
    </nav>
  )
}

/**
 * The tool's mark on white at the top of the rail, its `name` beside it, a
 * way back to the suite: `children` the mark drawn inline, else the image at `src`.
 */
export function RailBrand({ href, label, name, src, children }) {
  return (
    <a className="ui-rail-brand" href={href} title={label} aria-label={label}>
      {children ?? <img src={src} alt="" />}
      {name && <span className="ui-rail-brand-name">{name}</span>}
    </a>
  )
}

/**
 * One button of the rail: its symbol, `label` its name beside it (and title
 * and for screen readers) — `bare` a symbol only, as in a `RailRow` —,
 * `pressed` while its panel shows; `count` a number in a dot — `countTone`
 * warn for a yellow one, else red.
 */
export function RailButton({ icon, label, pressed, onClick, count = 0, countTone, disabled, bare = false }) {
  return (
    <button type="button" className="ui-rail-btn" aria-label={label} title={label} aria-pressed={pressed ?? undefined}
      onClick={onClick} disabled={disabled}>
      {icon}
      {!bare && <span className="ui-rail-label">{label}</span>}
      {count > 0 && <span className={`ui-rail-count${countTone === 'warn' ? ' ui-rail-count-warn' : ''}`} aria-hidden="true">{count > 99 ? '99+' : count}</span>}
    </button>
  )
}

/** Buttons side by side in the rail, symbols only — undo, the history, redo. */
export const RailRow = ({ children }) => <div className="ui-rail-row">{children}</div>

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
