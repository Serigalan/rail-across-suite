/**
 * Rows to pick one from: `items` [{ id, title, sub, mark, end }] — `mark` a
 * symbol before the words, `end` badges after them. `selected` is the id
 * chosen; `onSelect(id)` picks one. `empty` says what there is when there is
 * nothing. With `maxHeight` the list scrolls within it, so what follows it
 * stays in view.
 */
export function SelectList({ label, items, selected, onSelect, empty, maxHeight }) {
  if (!items.length) return <p className="ui-list-empty">{empty}</p>
  return (
    <ul className={`ui-list${maxHeight ? ' ui-list-scroll' : ''}`} role="listbox" aria-label={label} style={maxHeight ? { maxHeight } : undefined}>
      {items.map(item => (
        <li key={item.id} role="none">
          <button type="button" role="option" className="ui-list-row" aria-selected={item.id === selected}
            onClick={() => onSelect?.(item.id)}>
            {item.mark != null && <span className="ui-list-mark" aria-hidden="true">{item.mark}</span>}
            <span className="ui-list-text">
              <span className="ui-list-title">{item.title}</span>
              {item.sub && <span className="ui-list-sub">{item.sub}</span>}
            </span>
            {item.end && <span className="ui-list-end">{item.end}</span>}
          </button>
        </li>
      ))}
    </ul>
  )
}
