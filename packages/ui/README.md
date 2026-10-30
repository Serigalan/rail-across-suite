# @rail-across/ui — the look of the suite as a library

Every module of Rail-Across-Suite looks the same because its pages are made of the same parts. This package
holds them: the colours and sizes once (`ui.css`, light and dark), and the React components every module's
pages use. Its look is the Layout-Tool's — the icon rail in the primary colour, the white panel beside it, the
buttons, fields and dialogs of its pages ([decision 0012](../../docs/decisions/0012-ui-library.md)).

```js
import '@rail-across/ui/ui.css'            // once, at the app's entry, before the app's own styles
import { Button, Panel, Rail, RailButton, Shell, Workspace } from '@rail-across/ui'
```

## What it holds

| Part | Components | Classes |
|---|---|---|
| Colours, sizes | — | `--color-primary`, `--ui-bg`, `--ui-surface`, `--ui-fg`, `--ui-muted`, `--ui-border`, `--ui-danger` … on `:root`; dark where the system asks or `data-theme="dark"` says so |
| Buttons | `Button` (primary, danger, ghost; small; block; active; as a link with `href`), `IconButton` | `ui-btn`, `ui-btn-primary` … |
| Fields | `Field`, `TextField`, `NumberField` (a number or null, never NaN), `SelectField` (null for no choice), `Checkbox` | `ui-field`, `ui-field-compact`, `ui-fields`, `ui-check` |
| Feedback | `Badge`, `Message` (errors announced) | `ui-badge`, `ui-message`, `ui-muted`, `ui-error` |
| Dialogs | `Dialog` (focus kept inside, Escape and a click beside close it), `FormDialog`, `ConfirmDialog` | `ui-overlay`, `ui-dialog`, `ui-actions` |
| Choosing | `Tabs` (arrow keys), `SelectList` (rows with a mark, title, line below, badges; scrolling within `maxHeight`) | `ui-tabs`, `ui-list` |
| Menus | — | `ui-menu`, `ui-menu-list` |
| The shell of a tool | `Shell`, `Rail`, `RailBrand`, `RailButton` (named beside its symbol, with a count), `RailRow`, `RailDivider`, `Panel`, `Section`, `Workspace` | `ui-shell`, `ui-rail`, `ui-panel`, `ui-workspace`, `ui-topbar` |
| Symbols | `PlusIcon`, `TrashIcon`, `CloseIcon`, `CheckIcon`, `BackIcon`, `UndoIcon`, `RedoIcon`, `UploadIcon`, `DownloadIcon`, `SaveIcon`, `WarningIcon`, `SearchIcon`, `FitIcon`, `HelpIcon` | — |

Texts are the app's: a component takes its words as props (`cancelLabel`, `label`), so the library has no
language of its own.

## Who uses it

| Module | Since |
|---|---|
| the suite's start page (`apps/home`) | its buttons, fields, dialogs, badges, menus and colours |
| the Signalling-Tool (`apps/signalling-tool`) | all of it, the shell first |
| the Layout-Tool (`apps/layout-tool`) | not yet: its look is the library's model; its pages move to the components one by one |

## Rules

- **A module adds, it does not override.** What a module draws for itself (a plan, a map, a tile) has its own
  classes; a `ui-` class is changed here, for every module.
- **One primary colour.** `--color-primary` is the only colour a module may set (the Layout-Tool sets the
  project colour the user picks).
- **Accessible by default**: buttons are `type="button"` unless they submit, dialogs are modal with their
  focus kept inside, lists are listboxes, a count in the rail is hidden from screen readers and said by the
  button's label.

## Tests

`src/ui.test.jsx` renders the components to static markup (no DOM).
