# home — the suite's own pages

What is Rail-Across-Suite's rather than a module's: the sign-in, the start page, the account and the users.

**The sign-in page** (`sign-in/`) comes before everything else. The server shows it in place of every page of
the suite until one is signed in — the start page, a module, a project's link — and once one is, it loads the
page again, which now comes ([deploy](../../deploy/README.md)). A start password is changed there before anything
opens. One sign-in for every module.

**The start page** (`https://railacross.open-layout-tool.org/`): the bar with the suite's mark, and below it the
modules — the three editors, a line, the three simulations. Each module that is not there yet has a page saying
when it comes (ROADMAP.md). The **infrastructure editor** shows its three tools as tabs — Layout-Tool,
Signalling-Tool, Catenary-Tool —, the chosen one standing out, the others greyed, and below a line the projects
of the suite's project server, **managed here for every tool**: new projects and templates (with a picture),
importing backup files and exporting a variant's head as one, title and description, editors, deleting;
branching, renaming and archiving variants. Working on a variant, looking at it, its history, comparing and
merging two open in the chosen tool by its address ([apps/README.md](../README.md)). Only the Layout-Tool
exists so far; it lies beside the start page under `/layout/`. For a tool still to come the projects are
managed all the same; only what would open in it cannot be pressed.

**The account and the users**: changing one's password (`#/password`, also where a module sends one, and back),
signing out, and for admins the users of the whole suite (`#/admin`) — accounts with a start password, roles,
the modules' rights (the Layout-Tool's: editing point clouds), resetting, deactivating, and the disk the point
clouds take.

| File | What it does |
|---|---|
| `sign-in/index.html`, `src/signIn.jsx`, `src/SignInScreen.jsx`, `src/SignIn.jsx` | The sign-in page |
| `src/App.jsx` | The start page's pages by the address' hash (`#/`, `#/infrastructure/<tool>`, `#/<module>`, `#/password`, `#/admin`) |
| `src/modules.js` | The modules and tools, what the suite asks a tool by its address (`toolUrl`), the routes, a tool's note |
| `src/projects.js` | The projects and templates, the variant trees, who may change what, backup files in and out |
| `src/pages/projects/` | A project with its variants, its dialogs, its editors, the picture of a new one |
| `src/Dialog.jsx`, `src/useAction.js` | A dialog over the page, a form in one, the server's errors in words |
| `src/projectImages.js`, `public/project-images/` | The default pictures of a project, an own one fitted to the slot |
| `src/Header.jsx` | The bar: mark, language, the user's menu |
| `src/pages/` | The overview, the infrastructure editor with the projects, a module still to come, the password, the users |
| `src/users.js` | Start passwords, sizes as people read them |
| `src/icons.jsx`, `src/logoWords.js` | The suite's mark (its name in Open Sans, as outlines) and the icons of modules and tools |
| `src/i18n.js` | The texts, German and English |
| `src/home.css` | The colours and controls the Layout-Tool's pages have too, and the suite's own |

The bar, buttons and colours copy the Layout-Tool's pages (its `collab.css`) — an app does not import another
app; once a third app needs them, they move into a package.

```bash
npm run dev      # with /api passed to OLT_API_TARGET (default http://127.0.0.1:8787); sign in at /sign-in/
npm test         # vitest: routes, the projects listed, the pages rendered
npm run build    # index.html and sign-in/index.html, named from the root
```

Deployed by the suite's scripts ([`deploy/`](../../deploy/README.md)), which build these pages into the root of
the site and each module's app under its path.
