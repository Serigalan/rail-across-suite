import { describe, it, expect } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import {
  Badge, Button, Checkbox, IconButton, Message, NumberField, Panel, Rail, RailButton, SelectField, SelectList, Tabs,
} from './index.js'
import { PlusIcon } from './icons.jsx'

const html = (el) => renderToStaticMarkup(el)

describe('the suite\'s components', () => {
  it('make a button that never submits by accident, in its variants', () => {
    expect(html(<Button variant="primary">Go</Button>)).toBe('<button type="button" class="ui-btn ui-btn-primary">Go</button>')
    expect(html(<Button size="small" active>On</Button>)).toContain('class="ui-btn ui-btn-small ui-btn-active" aria-pressed="true"')
    expect(html(<Button href="#/x">Link</Button>)).toBe('<a class="ui-btn" href="#/x">Link</a>')
    expect(html(<IconButton label="Add" icon={<PlusIcon />} />)).toContain('aria-label="Add" title="Add"')
  })

  it('label their fields, and give null for an empty choice', () => {
    expect(html(<NumberField label="Station" value={12.5} />)).toMatch(/<label class="ui-field"><span>Station<\/span><input type="number" step="any" value="12.5"\/><\/label>/)
    const select = html(<SelectField label="Art" value={null} empty="—" options={[{ value: 'a', label: 'A' }]} />)
    expect(select).toContain('<option value="" selected="">—</option>')
    expect(html(<Checkbox label="Beide Richtungen" checked />)).toContain('checked=""')
  })

  it('mark the chosen row and tab, and say when a list is empty', () => {
    const list = html(<SelectList label="Signale" selected="b" items={[{ id: 'a', title: 'A' }, { id: 'b', title: 'B', sub: 'Ks' }]} />)
    expect(list).toContain('aria-selected="false"')
    expect(list).toContain('aria-selected="true"><span class="ui-list-text"><span class="ui-list-title">B</span><span class="ui-list-sub">Ks</span>')
    expect(html(<SelectList items={[]} empty="Keine" />)).toBe('<p class="ui-list-empty">Keine</p>')
    expect(html(<Tabs options={[{ value: 1, label: 'x' }, { value: 2, label: 'y' }]} value={2} onChange={() => {}} />))
      .toContain('aria-selected="true" tabindex="0">y</button>')
  })

  it('build the shell of a tool: rail buttons with their counts, the panel', () => {
    const rail = html(<Rail label="Werkzeuge" top={<RailButton label="Prüfung" count={120} pressed />} />)
    expect(rail).toContain('aria-pressed="true"')
    expect(rail).toContain('<span class="ui-rail-count" aria-hidden="true">99+</span>')
    expect(html(<Panel title="Signale" intro="Alle Signale">x</Panel>)).toContain('<aside class="ui-panel" aria-label="Signale">')
    expect(html(<Badge tone="danger" solid>3</Badge>)).toBe('<span class="ui-badge ui-badge-danger ui-badge-solid">3</span>')
    expect(html(<Message tone="error">Nein</Message>)).toBe('<p class="ui-message ui-message-error" role="alert">Nein</p>')
  })
})
