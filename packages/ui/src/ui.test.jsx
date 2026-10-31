import { describe, it, expect } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import {
  Badge, Button, Checkbox, GroupedList, IconButton, Message, NumberField, Option, Options, OptionsDivider, OptionsHeading, Panel, Rail, RailBrand,
  RailButton, SelectField, SelectList, Tabs, WorkingCopyBar,
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
    expect(rail).toContain('<span class="ui-rail-label">Prüfung</span><span class="ui-rail-count" aria-hidden="true">99+</span>')
    expect(html(<RailButton label="Verlauf" bare />)).not.toContain('ui-rail-label')
    expect(html(<RailBrand href="../" label="Zur Suite" name="Signalling-Tool"><svg /></RailBrand>))
      .toBe('<a class="ui-rail-brand" href="../" title="Zur Suite" aria-label="Zur Suite"><svg></svg><span class="ui-rail-brand-name">Signalling-Tool</span></a>')
    expect(html(<Panel title="Signale" intro="Alle Signale">x</Panel>)).toContain('<aside class="ui-panel" aria-label="Signale">')
    expect(html(<Badge tone="danger" solid>3</Badge>)).toBe('<span class="ui-badge ui-badge-danger ui-badge-solid">3</span>')
    expect(html(<Message tone="error">Nein</Message>)).toBe('<p class="ui-message ui-message-error" role="alert">Nein</p>')
  })

  it('make a panel\'s menu as the Layout-Tool\'s: options under headings, a line between, a page with its way back', () => {
    const menu = html(<><Options><OptionsHeading>Weichen</OptionsHeading><Option end={3}>Weichenelemente</Option></Options><OptionsDivider /></>)
    expect(menu).toContain('<span class="ui-options-heading">Weichen</span>')
    expect(menu).toContain('<button type="button" class="ui-btn ui-btn-option"><span class="ui-grow">Weichenelemente</span><span class="ui-option-end">3</span></button>')
    expect(menu).toContain('<hr class="ui-options-divider"/>')
    const page = html(<Panel title="Tafel erstellen" back={{ label: 'Zurück', onClick: () => {} }}>x</Panel>)
    expect(page).toMatch(/<button type="button" class="ui-back"><svg[^]*<\/svg>Zurück<\/button><h2>Tafel erstellen<\/h2>/)
  })

  it('make the working copy\'s bar as the Layout-Tool\'s: where, the revision, the state, the ways to act', () => {
    const bar = html(<WorkingCopyBar title="P" subtitle="Bestand" revision="rev 3" state="local" stateLabel="2 lokale Änderung(en)" onState={() => {}}
      actions={[{ label: 'Aktualisieren' }, { label: 'Einchecken', primary: true, disabled: true }]} />)
    expect(bar).toContain('<span class="ui-wc-where"><strong>P</strong><span class="ui-wc-sep">›</span><span>Bestand</span><span class="ui-wc-rev">rev 3</span></span>')
    expect(bar).toContain('<button type="button" class="ui-wc-state ui-wc-state-local">2 lokale Änderung(en)</button>')
    expect(bar).toContain('<button type="button" class="ui-wc-btn ui-wc-btn-primary" disabled="">Einchecken</button>')
    expect(html(<WorkingCopyBar title="P" note="schreibgeschützt" />)).toContain('<span class="ui-wc-rev">schreibgeschützt</span>')
  })

  it('list objects under their groups, folded but the one holding the chosen, with a search and the headings over them', () => {
    const groups = [
      { key: 'a', heading: 'Signale', title: 'Hauptsignale', items: [{ id: 'A', label: 'A', sub: 'Ks' }, { id: 'B', label: 'B' }] },
      { key: 'b', heading: 'Signale', title: 'Vorsignale', items: [{ id: 'a', label: 'a' }] },
      { key: 'c', heading: 'Gleisfreimeldung', title: 'Abschnitte', items: [] },
    ]
    const list = html(<GroupedList label="Objekte" groups={groups} selected="B" searchLabel="Suchen" empty="Nichts" />)
    expect(list).toContain('aria-label="Suchen"')
    expect(list.match(/ui-options-heading/g)).toHaveLength(1)
    expect(list).toContain('aria-expanded="true"><span class="ui-group-caret open" aria-hidden="true">▸</span><span class="ui-group-title">Hauptsignale</span><span class="ui-group-count">2</span>')
    expect(list).toContain('aria-pressed="true" class="ui-btn ui-btn-option ui-btn-small ui-group-item ui-btn-active">')
    expect(list).toContain('<span class="ui-group-item-sub">Ks</span>')
    expect(list).toContain('aria-expanded="false"><span class="ui-group-caret" aria-hidden="true">▸</span><span class="ui-group-title">Vorsignale</span>')
    expect(list).not.toContain('Abschnitte')
    expect(html(<GroupedList groups={groups} open />)).not.toContain('aria-expanded="false"')
    expect(html(<GroupedList groups={[]} empty="Nichts" />)).toBe('<p class="ui-list-empty">Nichts</p>')
  })
})
