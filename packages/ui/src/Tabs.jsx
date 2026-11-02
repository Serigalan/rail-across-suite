/**
 * A row of choices, one of them chosen (a segmented control): `options`
 * [{ value, label }], `value` the chosen one; arrow keys move between them.
 */
export function Tabs({ label, options, value, onChange }) {
  const move = (e, i) => {
    const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0
    if (!step) return
    e.preventDefault()
    const next = options[(i + step + options.length) % options.length]
    onChange(next.value)
    e.currentTarget.parentElement?.children[(i + step + options.length) % options.length]?.focus()
  }
  return (
    <div className="ui-tabs" role="tablist" aria-label={label}>
      {options.map((o, i) => (
        <button key={o.value} type="button" role="tab" className="ui-tab" aria-selected={o.value === value}
          tabIndex={o.value === value ? 0 : -1} onClick={() => onChange(o.value)} onKeyDown={(e) => move(e, i)}>
          {o.label}
        </button>
      ))}
    </div>
  )
}
