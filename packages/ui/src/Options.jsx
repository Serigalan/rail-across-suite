import { useState } from 'react'
import { SearchIcon } from './icons.jsx'

/**
 * A panel's menu, as the Layout-Tool's panels have it: one option per line —
 * a symbol and its words — under small headings, the groups apart by a gray
 * line; a click on an option opens its page, which has a way back.
 */
export const Options = ({ children }) => <div className="ui-options">{children}</div>

/** A small heading over the options that belong together. */
export const OptionsHeading = ({ children }) => <span className="ui-options-heading">{children}</span>

/** The gray line between two groups of options. */
export const OptionsDivider = () => <hr className="ui-options-divider" />

/** One option: its symbol, its words, `end` what follows them (a count, a badge). */
export function Option({ icon, end, active = false, children, ...rest }) {
  return (
    <button type="button" className={`ui-btn ui-btn-option${active ? ' ui-btn-active' : ''}`} aria-pressed={active || undefined} {...rest}>
      {icon}
      <span className="ui-grow">{children}</span>
      {end != null && <span className="ui-option-end">{end}</span>}
    </button>
  )
}

/**
 * Objects under the groups they belong to, each group folding away under its
 * header with how many it holds — as the Layout-Tool lists a project's
 * tracks under their lines and stations. `groups` [{ key, title, heading,
 * items: [{ id, label, sub, end }] }] — `heading` a small heading over the
 * groups that share it —; a group without items is left out. `selected` is the
 * id chosen, `onSelect(id)` picks one. The groups start folded but the one
 * holding the chosen; `open` starts them open. With `searchLabel` a search
 * field above finds by any part of the words, and opens every group it finds
 * something in; `noneFound` says when it finds nothing, `empty` when there is
 * nothing at all.
 */
export function GroupedList({ label, groups, selected, onSelect, empty, searchLabel, noneFound, open: allOpen = false }) {
  const [query, setQuery] = useState('')
  const [toggled, setToggled] = useState(() => new Set())
  const words = query.trim().toLowerCase()
  const found = groups
    .map(g => ({ ...g, items: words ? g.items.filter(i => `${i.label} ${i.sub ?? ''}`.toLowerCase().includes(words)) : g.items }))
    .filter(g => g.items.length)
  const total = groups.reduce((n, g) => n + g.items.length, 0)
  if (!total) return <p className="ui-list-empty">{empty}</p>
  const startsOpen = (g) => allOpen || g.items.some(i => i.id === selected)
  const toggle = (key) => setToggled(prev => {
    const next = new Set(prev)
    if (next.has(key)) next.delete(key)
    else next.add(key)
    return next
  })
  return (
    <div className="ui-grouped">
      {searchLabel && (
        <label className="ui-search">
          <SearchIcon size={16} />
          <input type="search" value={query} placeholder={searchLabel} aria-label={searchLabel}
            onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => { if (e.key === 'Escape' && query) { e.stopPropagation(); setQuery('') } }} />
        </label>
      )}
      {!found.length && <p className="ui-list-empty">{noneFound ?? empty}</p>}
      <div className="ui-groups" role="list" aria-label={label}>
        {found.map((g, n) => {
          const open = !!words || startsOpen(g) !== toggled.has(g.key)
          const heading = g.heading && g.heading !== found[n - 1]?.heading ? g.heading : null
          return (
            <div key={g.key} className="ui-group" role="listitem">
              {heading && <span className="ui-options-heading">{heading}</span>}
              <button type="button" className="ui-group-toggle" aria-expanded={open} onClick={() => toggle(g.key)}>
                <span className={`ui-group-caret${open ? ' open' : ''}`} aria-hidden="true">▸</span>
                <span className="ui-group-title">{g.title}</span>
                <span className="ui-group-count">{g.items.length}</span>
              </button>
              {open && (
                <div className="ui-group-list">
                  {g.items.map(i => (
                    <button key={i.id} type="button" aria-pressed={i.id === selected}
                      className={`ui-btn ui-btn-option ui-btn-small ui-group-item${i.id === selected ? ' ui-btn-active' : ''}`}
                      onClick={() => onSelect?.(i.id)}>
                      <span className="ui-group-item-text">
                        <span className="ui-group-item-label">{i.label}</span>
                        {i.sub && <span className="ui-group-item-sub">{i.sub}</span>}
                      </span>
                      {i.end}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
