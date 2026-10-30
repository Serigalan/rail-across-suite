# apps

One directory per module with its own build, named after the module: `home/` (the suite's start page: the
sign-in, the user administration, the modules and the projects), `layout-tool/` (the Layout-Tool, which was
the Layout-Tool before it moved here in phase 0), later `signalling-tool/`, `catenary-tool/`,
`timetable-editor/`, `capacity-sim/` (an adapter to OSRD), `construction-sim/`
([decision 0010](../docs/decisions/0010-one-suite-modules-by-task.md)). An app imports packages, never another app
([decision 0001](../docs/decisions/0001-one-repository.md)).

## The projects of the infrastructure and its tools

The projects of the infrastructure — creating, importing, exporting, who works on them, branching, renaming and
archiving variants, the templates — are managed once, on the suite's start page (`home/`, `#/infrastructure/<tool>`),
for every tool: the Layout-Tool now, the Signalling-Tool and the Catenary-Tool later. A tool has no start page of
its own. The suite asks it by its address what to do (`toolUrl` in `home/src/modules.js`):

| Address of the tool | What it does |
|---|---|
| `?project=<id>&variant=<id>` | works on the variant (the main one without `variant`) |
| `?project=<id>&variant=<id>&action=view` | shows its head, read-only |
| `?project=<id>&variant=<id>&action=history` | shows its revisions |
| `?project=<id>&action=compare&from=<a>&to=<b>` | compares two variants |
| `?project=<id>&action=merge&from=<a>&to=<b>` | merges a's changes into b |
| `?action=storage` | shows what the tool keeps in this browser |

Every way back leads to `../#/infrastructure/<tool>`. A line to show there — a merge done, a link to a project
that is not there — goes in `sessionStorage` under `rail-across-notice`; the start page shows it once.
