/**
 * The suite's button. `variant` primary (the one thing to do), danger (what
 * cannot be undone lightly), ghost (in a toolbar), or none (everything else);
 * `size` small; `block` across its container; `active` for a toggle that is on.
 * With `href` it is a link that looks like one. Every other prop goes to the
 * element; `type` is button unless said otherwise, so it never submits a form
 * by accident.
 */
export function Button({ variant, size, block = false, active = false, icon = null, href, className = '', children, ...rest }) {
  const cls = [
    'ui-btn', variant && `ui-btn-${variant}`, size === 'small' && 'ui-btn-small', block && 'ui-btn-block',
    active && 'ui-btn-active', !children && icon && 'ui-btn-icon', className,
  ].filter(Boolean).join(' ')
  if (href) return <a className={cls} href={href} {...rest}>{icon}{children}</a>
  return <button type="button" className={cls} aria-pressed={active || undefined} {...rest}>{icon}{children}</button>
}

/** A button that shows only its symbol: `label` names it (its title and for screen readers). */
export function IconButton({ label, icon, ...rest }) {
  return <Button icon={icon} aria-label={label} title={label} {...rest} />
}
