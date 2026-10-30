# home — the start page of Rail-Across-Suite

The page at the root of the suite's address (`https://railacross.open-layout-tool.org/`): the bar of the
Open-Layout-Tool with the suite's mark, and below it the modules — the three editors, a line, the three
simulations. Each module that is not there yet has a page saying when it comes (ROADMAP.md).

The **infrastructure editor** shows its three tools as tabs — Layout-Tool, Signalling-Tool, Catenary-Tool —,
the chosen one standing out, the others greyed, and below a line the projects of the suite's project server:
each variant opens in the chosen tool. Only the Layout-Tool exists so far; it lies beside the start page under
`/layout/` and opens a variant named in the address (`layout/?project=<id>&variant=<id>`). One origin, so one
sign-in: the start page signs in at the project server (`/api`) like the tool does.

| File | What it does |
|---|---|
| `src/App.jsx` | The pages by the address' hash (`#/`, `#/infrastructure/<tool>`, `#/<module>`), who is signed in, the projects |
| `src/modules.js` | The modules and tools, where a variant opens, which projects are listed |
| `src/Header.jsx` | The bar: mark, name, language, the user's menu |
| `src/pages/` | The overview, the infrastructure editor, the page of a module still to come |
| `src/icons.jsx` | The suite's mark and the icons of modules and tools |
| `src/i18n.js` | The texts, German and English |
| `src/home.css` | The Open-Layout-Tool's start page styles, and the tiles and tabs |
| `public/logo.svg` | The mark: R and A written as tracks branching off one track running across |

The bar, buttons and colours copy the Open-Layout-Tool's start page (its `collab.css`) — an app does not
import another app; once a third app needs them, they move into a package.

```bash
npm run dev      # with /api passed to OLT_API_TARGET (default http://127.0.0.1:8787)
npm test         # vitest: routes, the projects listed, the pages rendered
npm run build
```

Deployed by the suite's scripts ([`deploy/`](../../deploy/README.md)), which build this page into
the root of the site and each module's app under its path.
