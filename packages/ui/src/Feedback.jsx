/** A small marker beside a name: `tone` ok, primary, warn, danger, or none; `solid` filled. */
export function Badge({ tone, solid = false, title, children }) {
  const cls = ['ui-badge', tone && `ui-badge-${tone}`, solid && 'ui-badge-solid'].filter(Boolean).join(' ')
  return <span className={cls} title={title}>{children}</span>
}

/** What a page or panel says about its state: `tone` error, warn, ok, info, or none. Errors are announced. */
export function Message({ tone, children }) {
  const cls = ['ui-message', tone && `ui-message-${tone}`].filter(Boolean).join(' ')
  return <p className={cls} role={tone === 'error' ? 'alert' : undefined}>{children}</p>
}
