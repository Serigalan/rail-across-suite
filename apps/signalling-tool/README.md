# Signalling-Tool

The module of Rail-Across-Suite for planning signalling on the net the Layout-Tool draws: signals and boards,
train detection, the switch elements of the interlocking, train and shunting routes with their overlaps, flank
protection and the balises and magnets of train protection (PZB, ETCS) — the domain `signalling` 1.0 ([decision 0011](../../docs/decisions/0011-signalling-domain.md)).
Its look is the suite's library ([`@rail-across/ui`](../../packages/ui/README.md)), shaped like the Layout-Tool:
the rail of sections on the left, the panel of the one chosen, the plan of the net. A panel works as the
Layout-Tool's do: a menu of options under small headings, the groups apart by a gray line; an option opens its
page, which has its way back (*Zurück*). Choosing a section in the rail again starts its panel at its menu.

It reads the core and never writes it (data model, R1): the tracks are the Layout-Tool's. Every location it
stores stands on a track of the core, and the core carries it when the Layout-Tool splits, joins, turns round or
deletes a track (R4, R7).

## What it does

The rail, from the top, in groups the gray line parts:

| Section | Its menu and pages |
|---|---|
| Kartenebenen | the map under the plan, first in the rail as in the Layout-Tool: the suite's basemaps as the Layout-Tool offers them ([`@rail-across/basemaps`](../../packages/basemaps/README.md)) — OpenStreetMap, satellite, relief, basemap.de, DGM5, the states' orthophotos — or the plan alone; the choice is kept in the browser |
| **Signale** | creating signals: *Signalmast erstellen* — the physical signal, its name, mounting, both directions; bare, its logical signals come after —, *Tafel erstellen* — a board of no system on a post of its own (Ne 1, Lf 7, Ra 10 …) —, *Logisches Signal erstellen* — system, kind, function, on a new mast set on the plan or added to a mast there is, its screen showing what the kind asks —, *Fiktives Signal erstellen* |
| **Gleisfreimeldung** | creating train detection: *Freimeldepunkt setzen* of a kind; *Freimeldeabschnitt erstellen*, which opens it in its editor with its points to be clicked on the plan |
| **Balisen/Magnete** | creating balises and magnets: a page per system the catalogue declares (PZB, ETCS) with kind, effect and the signal it belongs to |
| **Bearbeiten** | editing all that the three create: an overview as the Layout-Tool lists its tracks — the signal masts under main, distant and shunting signals, boards, masts without a signal and virtual signals; track sections and detection points; balises and magnets per system — each group folding away with its count, a search over all. A click, here or on the plan, opens the object in its editor: a signal mast — where it stands, its mounting and sighting, the boards it carries, its logical signals, each opened from it —; a logical signal, as OSRD has it — its mast, kind and function, its screen (what its kind may show, what it must marked) and indicators from the catalogue of Ril 301, the systems it announces, the speed it signals for each route from its mast (signallingSpeed; the design speed, designSpeed, is set on the track in the Layout-Tool) —; a detection point; a track section, bounded by its points, its extent derived over the net and drawn; a balise or magnet with the settings the catalogue declares for its kind — frequency, test speed, place in the group … |
| **Weichen** | *Weichenelemente*, listed under switches and derailers; *Weichen ohne Element*, each offered to get one (or all at once); *Gleissperre setzen*. A switch element's editor holds the flank protection it asks for per way it is run over — a signal, a switch or derailer in a position, or waived |
| **Fahrstraßen** | *Fahrstraße erstellen* from a start to a target signal, the run found over the net the way they act; *Durchrutschweg erstellen* ahead of a signal; the routes under train, shunting and middle-switch routes, and the overlaps. A route's editor holds what it shows, the speed its start signal signals for it, the overlaps at its target; *Durchrutschweg ermitteln* measures an overlap from that speed and the gradient over it, which the heights of the tracks give (catalogue `durchrutschweg`, a draft after Ril 819.0202), straight on at every switch |
| **Prüfung** | the core's `checkSignalling` on every edit; a finding opens its object in its editor |
| **Datenaustausch** | *PlanPro-Plan importieren*: a plan's signalling (`.planpro`, `.ppxml`) laid on the project's net — on the plan's own edges where the project was made from it, else carried to the tracks lying there, what nothing lies under orphaned |
| Einstellungen | at the foot of the rail, below undo, the history and redo and the way back to the infrastructure editor, as in the Layout-Tool: the language — the suite's, which the start page and the Layout-Tool share (`@rail-across/ui`) |

A page that creates stays when the thing is set, for the next one, and offers to edit the one set
(*Bearbeiten*). Where it only looks, the tool opens at *Bearbeiten* and offers nothing to create.

Setting an object works as in OSRD's editor: the mouse snaps to the nearest track within reach, and what would
be set is drawn there before the click — a signal two metres off the track on the side the mouse is, acting so
that it stands on the right; a balise or magnet at the rail. Every object is drawn where it stands in the plane:
a dot on its track, a stroke out to its symbol at its offset, turned the way it acts; its editor states the
point (easting, northing), derived from track, station, side and offset and never stored.

Every text is German or English, the catalogue's titles too, in the language of the suite (*Einstellungen*).

Every edit is a step (Ctrl+Z, Ctrl+Y); Delete asks before it deletes, and names what goes with it.

## The working copy

What is changed is kept at once in the variant's working copy in this browser — the one the Layout-Tool works
on too ([`@rail-across/working-copy`](../../packages/working-copy/README.md)) —, never only in the page. Leaving
the tool asks nothing: the changes stay in the copy, the top bar says how many there are, and the suite's start
page says so at the variant. Opening the variant again, in this tool or the Layout-Tool, opens the copy.

- *Einchecken* sends the copy, tracks and signalling, with its splits and joins; where someone checked in first,
  their state is merged in and the user checks in again.
- *Aktualisieren* brings in what others checked in. Where both changed the signalling, mine is laid on the merged
  net; where the tracks conflict, the Layout-Tool decides.
- *Verwerfen* makes the copy the server's head again, the Layout-Tool's changes with it.

## Its address

As every tool of the infrastructure (`apps/README.md`): `?project&variant` works on the variant, `&action=view`
or `history` shows its head read-only. Comparing and merging (`action=compare|merge`) is the Layout-Tool's, for
the whole project with its signalling: the tool hands those addresses on to it. Besides the working copy it keeps the basemap last chosen and the language in the browser.

## Files

| Path | What it is |
|---|---|
| `src/App.jsx`, `src/suite.js`, `src/workingCopy.js` | what the address asks, the session, loading the variant and its working copy |
| `src/Editor.jsx` | the shell: sections, selection, steps, the check, the working copy, checking in, updating, discarding |
| `src/plan/` | the plan: the net in its true position on a MapLibre map (north up), drawn over it in screen pixels; the symbols, picking and placing |
| `src/panels/` | one panel per section, its menu and pages; `editors/` the editors of what *Bearbeiten* edits |
| `src/model/` | the drawing of the net and finding a place on it (`net.js`), runs over it (`runs.js`), editing the domain — places and their logical signals (`domain.js`), measuring overlaps (`overlaps.js`), steps (`history.js`), reading a PlanPro file (`planproFile.js`) |
| `src/i18n.js`, `src/labels.js` | texts in German and English, the words for the domain's values and the check's findings |

## Run

`npm run dev` here (the project server on 127.0.0.1:8787, `/api` proxied), `npm test`, `npm run lint`,
`npm run build`. In the suite it lies under `/signalling/` (`deploy/lib.sh`, `MODULE_APPS`).
