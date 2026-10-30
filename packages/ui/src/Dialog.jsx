import { useEffect, useId, useRef } from 'react'
import { Button } from './Button.jsx'

const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

/**
 * A dialog over the page: `role="dialog"` and `aria-modal`, named by its
 * title; Escape and a click beside it close it (not while `busy`); Tab stays
 * inside it; the focus goes to its first field when it opens and back to
 * where it was when it closes. With `onSubmit` it is a form: Enter in a field
 * submits it. `actions` are its buttons, at its foot.
 */
export function Dialog({ title, onClose, onSubmit, busy = false, actions = null, wide = false, children }) {
  const titleId = useId()
  const box = useRef(null)
  const closeRef = useRef(onClose)
  const busyRef = useRef(busy)
  useEffect(() => { closeRef.current = onClose; busyRef.current = busy })

  useEffect(() => {
    const before = document.activeElement
    const el = box.current
    const first = el?.querySelector('input:not([disabled]), select:not([disabled]), textarea:not([disabled])') ?? el?.querySelector(FOCUSABLE)
    ;(first ?? el)?.focus()
    const onKey = (e) => {
      if (e.key === 'Escape') {
        e.stopPropagation()
        if (!busyRef.current) closeRef.current?.()
        return
      }
      if (e.key !== 'Tab' || !el) return
      const items = [...el.querySelectorAll(FOCUSABLE)]
      if (!items.length) { e.preventDefault(); return }
      const i = items.indexOf(document.activeElement)
      if (e.shiftKey && i <= 0) { e.preventDefault(); items[items.length - 1].focus() }
      else if (!e.shiftKey && i === items.length - 1) { e.preventDefault(); items[0].focus() }
    }
    document.addEventListener('keydown', onKey, true)
    return () => {
      document.removeEventListener('keydown', onKey, true)
      if (before && typeof before.focus === 'function' && document.contains(before)) before.focus()
    }
  }, [])

  const Box = onSubmit ? 'form' : 'div'
  const submit = onSubmit ? (e) => { e.preventDefault(); if (!busy) onSubmit(e) } : undefined
  return (
    <div className="ui-overlay" onMouseDown={(e) => { if (e.target === e.currentTarget && !busy) onClose?.() }}>
      <Box ref={box} className={`ui-dialog${wide ? ' ui-dialog-wide' : ''}`} role="dialog" aria-modal="true"
        aria-labelledby={titleId} tabIndex={-1} onSubmit={submit}>
        <h3 id={titleId} className="ui-dialog-title">{title}</h3>
        {children}
        {actions && <div className="ui-actions">{actions}</div>}
      </Box>
    </div>
  )
}

/**
 * A small form in a dialog: title, the fields, cancel and submit, an error
 * line. `danger` for a submit that deletes; `canSubmit` false locks it.
 */
export function FormDialog({
  title, submitLabel, cancelLabel, busy, error, onCancel, onSubmit, children, danger = false, canSubmit = true, wide = false,
}) {
  return (
    <Dialog title={title} onClose={onCancel} busy={busy} onSubmit={onSubmit} wide={wide}
      actions={<>
        <Button onClick={onCancel} disabled={busy}>{cancelLabel}</Button>
        <Button type="submit" variant={danger ? 'danger' : 'primary'} disabled={busy || !canSubmit}>{submitLabel}</Button>
      </>}>
      {children}
      {error && <p className="ui-error" role="alert">{error}</p>}
    </Dialog>
  )
}

/** A question to confirm: what will happen, and the two buttons. */
export function ConfirmDialog({ title, children, confirmLabel, cancelLabel, danger = false, busy = false, onConfirm, onCancel }) {
  return (
    <Dialog title={title} onClose={onCancel} busy={busy}
      actions={<>
        <Button onClick={onCancel} disabled={busy}>{cancelLabel}</Button>
        <Button variant={danger ? 'danger' : 'primary'} onClick={onConfirm} disabled={busy}>{confirmLabel}</Button>
      </>}>
      {children}
    </Dialog>
  )
}
