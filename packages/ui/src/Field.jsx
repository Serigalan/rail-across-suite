import { useId } from 'react'

/**
 * A labelled field: `label` above whatever control is its child, a `hint`
 * below. `compact` is the panel's smaller kind, its label in capitals.
 */
export function Field({ label, hint, compact = false, wide = false, className = '', children }) {
  const cls = ['ui-field', compact && 'ui-field-compact', wide && 'ui-wide', className].filter(Boolean).join(' ')
  return (
    <label className={cls}>
      <span>{label}</span>
      {children}
      {hint && <small className="ui-field-hint">{hint}</small>}
    </label>
  )
}

/** A text field with its label. `onChange` gets the text. */
export function TextField({ label, value, onChange, hint, compact, wide, ...rest }) {
  return (
    <Field label={label} hint={hint} compact={compact} wide={wide}>
      <input type="text" value={value ?? ''} onChange={(e) => onChange?.(e.target.value)} {...rest} />
    </Field>
  )
}

/**
 * A number field with its label. `onChange` gets a number, or null for an
 * empty field — never NaN.
 */
export function NumberField({ label, value, onChange, hint, compact, wide, step = 'any', ...rest }) {
  return (
    <Field label={label} hint={hint} compact={compact} wide={wide}>
      <input type="number" step={step} value={value ?? ''} {...rest}
        onChange={(e) => {
          const v = e.target.value
          if (v === '') return onChange?.(null)
          const n = Number(v)
          if (Number.isFinite(n)) onChange?.(n)
        }} />
    </Field>
  )
}

/**
 * A choice with its label: `options` [{ value, label }]; `empty` a label for
 * no choice at all, which gives null.
 */
export function SelectField({ label, value, onChange, options, empty, hint, compact, wide, ...rest }) {
  return (
    <Field label={label} hint={hint} compact={compact} wide={wide}>
      <select value={value ?? ''} onChange={(e) => onChange?.(e.target.value === '' ? null : e.target.value)} {...rest}>
        {empty != null && <option value="">{empty}</option>}
        {options.map(o => <option key={o.value} value={o.value} disabled={o.disabled}>{o.label}</option>)}
      </select>
    </Field>
  )
}

/** A checkbox with its words beside it. */
export function Checkbox({ label, checked, onChange, ...rest }) {
  const id = useId()
  return (
    <label className="ui-check" htmlFor={id}>
      <input id={id} type="checkbox" checked={!!checked} onChange={(e) => onChange?.(e.target.checked)} {...rest} />
      <span>{label}</span>
    </label>
  )
}
