# 0004 — One look for every module, as a library

Status: **accepted** · as of 2026-11-17

## Decision

1. **The suite's look is a package**, [`@rail-across/ui`](../../packages/ui/README.md): the colours and sizes
   once, as CSS custom properties (light and dark), and the React components every module's pages are made of —
   buttons, fields, dialogs, badges, messages, tabs, lists, menus, and the shell of a tool.
2. **Its model is the Layout-Tool**, the first and largest module: the icon rail in the primary colour with the
   panel of the chosen section beside it and the workspace filling the rest; its buttons, fields and dialogs as its
   collaboration pages have them.
3. **The start page and every module use it.** The Layout-Tool moves to it page by page, as its pages are touched;
   its map view keeps the light look (`data-theme="light"`) while the project pages follow the system.
4. **One language for the suite**, kept by the library (`keptLanguage`, `keepLanguage`): every module sets and
   reads the same choice.
5. **Classes begin with `ui-`**; a module styles what is its own (a plan, a map) with its own classes and never
   overrides a `ui-` class. `--color-primary` is the one colour a module may set.
6. **No texts in the library**: a component takes its words as props, so every module keeps its own translations.

## Why

- A suite whose modules look alike is learnt once. Copying styles into each app makes every change a change in
  every copy, and the copies drift.
- A library of components, not only of styles, keeps the behaviour alike too: the focus kept in a dialog, Escape
  closing it, a number field never giving NaN, a list chosen from with the keyboard.
- Taking the Layout-Tool's look unchanged means no module looks new and none looks old.

## What it costs

- **Two looks for a while**: the Layout-Tool keeps its own styles until its pages have moved; its tokens and the
  library's are the same values, kept so by hand until then.
- **A shared dependency**: a change of the library shows in every module at once, and is checked in each.
