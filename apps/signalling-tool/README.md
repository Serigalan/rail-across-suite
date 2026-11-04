# Signalling-Tool

The module of Rail-Across-Suite for planning signalling on the net the Layout-Tool draws: signals and boards,
train detection, the switch elements of the interlocking, train and shunting routes with their overlaps, flank
protection and the balises and magnets of train protection (PZB, ETCS) — the domain `signalling` 1.0 ([decision 0011](../../docs/decisions/0011-signalling-domain.md)).
Its look is the suite's library ([`@rail-across/ui`](../../packages/ui/README.md)), shaped like the Layout-Tool:
the rail of sections on the left, the panel of the one chosen, the plan of the net. A panel works as the
Layout-Tool's do: a menu of options under small headings, the groups apart by a gray line; an option opens its
page, which has its way back (*Zurück*). Choosing a section in the rail again starts its panel at its menu.

It reads the core and writes of it only the stations — the points of the operational point register and which
station a platform is of ([decision 0017](../../docs/decisions/0017-stations-in-the-signalling-tool.md)); the
tracks are the Layout-Tool's. Every location it
stores stands on a track of the core, and the core carries it when the Layout-Tool splits, joins, turns round or
deletes a track (R4, R7).

## What it does

The rail, from the top, in groups the gray line parts:

| Section | Its menu and pages |
|---|---|
| Kartenebenen | the map under the plan, first in the rail as in the Layout-Tool: the suite's basemaps as the Layout-Tool offers them ([`@rail-across/basemaps`](../../packages/basemaps/README.md)) — OpenStreetMap, satellite, relief, basemap.de, DGM5, the states' orthophotos — or the plan alone; the choice is kept in the browser |
| **Signale** | creating signals: *Signalmast erstellen* — the physical signal, its name, mounting, both directions; on a gantry or a bracket where its screen and its foundation stand (*Lage Signalschirm*, *Lage Signalfundament*: a side seen the way it acts, an offset from the axis); bare, its logical signals come after —, *Tafel erstellen* — a board of no system on a post of its own (Ne 1, Lf 7, Ra 10 …) —, *Logisches Signal erstellen* — system, kind, function, on a new mast set on the plan or added to a mast there is, its screen showing what the kind asks —, *Fiktives Signal erstellen* |
| **Gleisfreimeldung** | creating train detection: *Freimeldepunkt setzen* of a kind; *Freimeldeabschnitt erstellen*, which opens it in its editor with its points to be clicked on the plan |
| **Balisen/Magnete** | creating balises and magnets: a page per system the catalogue declares (PZB, ETCS) with kind, effect and the signal it belongs to — chosen in the list, or clicked on the plan (*In der Karte wählen*), as in its editor |
| **Bearbeiten** | editing all that the three create: an overview as the Layout-Tool lists its tracks — the signal masts under main, distant and shunting signals, boards, masts without a signal and virtual signals; track sections and detection points; balises and magnets per system — each group folding away with its count, a search over all. A click, here or on the plan, opens the object in its editor: a signal mast — where it stands, its mounting and sighting, the boards it carries, its logical signals, each opened from it —; a logical signal, as OSRD has it — its mast, kind and function, its screen (what its kind may show, what it must marked) and indicators from the catalogue of Ril 301, the systems it announces, the speed it signals for each route from its mast (signallingSpeed; the design speed, designSpeed, is set on the track in the Layout-Tool) —; a detection point; a track section, bounded by its points, its extent derived over the net and drawn; a balise or magnet with the settings the catalogue declares for its kind — frequency, test speed, place in the group … |
| **Bahnhöfe** | *Bahnhof anlegen* — a station of the register, its name and RL100 code —; *Bahnhöfe*, the register's stations, those the Layout-Tool's station tracks and platforms name among them. A station's editor holds its signals (every other one is of the open line; a signal mast's editor names its station too), its platforms from the Layout-Tool, its boundary towards the open line: its entry signals or boards Ne 1, which it proposes, else its entry switches (EBO §4) —, and its stopping places: on one of its tracks or at one of its platforms, a stretch of it the signals and axle counters bound, chosen among the stretches they cut it into or picked on the plan, as the Layout-Tool picks a platform's track — a click takes the stretch around it; its two ends can be picked anew there. A stopping place is a band under its track on the plan and in the schematic view, chosen by a click. *Halteplätze an der freien Strecke*: those at the platforms of no station. A station the tracks name is renamed and deleted in the Layout-Tool |
| **Weichen** | *Weichenelemente*, listed under switches and derailers; *Weichen ohne Element*, each offered to get one (or all at once); *Gleissperre setzen*. A switch element's editor holds the flank protection it asks for per way it is run over — a signal, a switch or derailer in a position, or waived |
| **Fahrstraßen** | *Fahrwege suchen* from one stopping place to another: the ways the net has between them, each with the train routes it takes — one of each station from signal to signal, the station of its start signal; none on the open line —, those there are and those it would make; the two stopping places can be picked on the plan, and the plan shows the ways found, the one pointed at in the list with arrows the way it runs, as an itinerary's editor shows its way; *Festlegen* makes the way an itinerary with the routes it lacks. *Fahrstraße erstellen* from a start to a target signal, the run found over the net the way they act; *Durchrutschweg erstellen* ahead of a signal; the itineraries, the routes under the stations they belong to, and the overlaps. A route's editor holds what it shows, the speed its start signal signals for it, the overlaps at its target; *Durchrutschweg ermitteln* measures an overlap from that speed and the gradient over it, which the heights of the tracks give (catalogue `durchrutschweg`, a draft after Ril 819.0202), straight on at every switch |
| **Prüfung** | the core's `checkSignalling` on every edit; a finding opens its object in its editor |
| **Datenaustausch** | *PlanPro-Plan importieren*: a plan's signalling (`.planpro`, `.ppxml`) laid on the project's net — on the plan's own edges where the project was made from it, else carried to the tracks lying there, what nothing lies under orphaned |
| Einstellungen | at the foot of the rail, below undo, the history and redo and the way back to the infrastructure editor, as in the Layout-Tool: the language — the suite's, which the start page and the Layout-Tool share (`@rail-across/ui`) |

A page that creates stays when the thing is set, for the next one, and offers to edit the one set
(*Bearbeiten*). Where it only looks, the tool opens at *Bearbeiten* and offers nothing to create.

Setting an object works as in OSRD's editor: the mouse snaps to the nearest track within reach, and what would
be set is drawn there before the click — a signal two metres off the track on the side the mouse is, acting so
that it stands on the right; a balise or magnet at the rail. Every object is drawn where it stands in the plane:
a dot on its track, a stroke out to its symbol at its offset, turned the way it acts; a signal on a gantry or a
bracket with the bridge from its foundation out to its screen, to scale. Its editor states the point (easting,
northing), derived from track, station, side and offset and never stored.

A signal is named with its area's number (*21-B*), a track as the Layout-Tool names it (*koennern.1*); a list to
choose a signal from names the track it stands at with it (*21-B · koennern.1*).

## Site plan and schematic view

At the plan's bottom right the view is chosen, and kept in the browser: the **site plan** — the net in its true
position on the map — or the **schematic view**: the net as the core's schematic strip
([`@rail-across/core/schematic`](../../packages/core/docs/schematic.md), the one the Layout-Tool's schematic
overview cuts into sheets) — its lines lanes along the longest of them, every switch a 45° connection, the length
along the lanes to scale — and on it the signalling in the symbols of a signalling plan, after the principles of
DB Ril 819.9002 rather than its every measure: a signal folded down along its track in the direction it acts, its
foot where it stands, the screen at the end of its mast — a main signal's rounded at its end, a multi-section
signal's at both, a distant signal's towards the mast, a shunting signal a disc with its bar —, filled (the
plans draw a signal the train works open; the domain does not say who works one yet); boards by their shape
(Ne 1, Ne 2, Ne 4, Ne 5, Ne 14, Lf 1 – 7, Zs 3), every text on a symbol and a signal's name turned the way it
acts, its top towards the mast; axle counters and joints on the track,
PZB magnets beside the rail with their frequency, ETCS data points on it, filled where they are switched.

The schematic view shows, picks, highlights and is edited from as the site plan is; setting anew is the site
plan's, which it switches to. It has one scale both ways — a metre along the lanes a pixel at 100 %, the lanes
52 apart, every symbol and text its size — and the wheel zooms it as a whole: zoomed out, all of it is smaller
together, the lanes, the symbols and the names with the length; only a track keeps a hairline, so the net stays in
sight. A drag moves.

Every text is German or English, the catalogue's titles too, in the language of the suite (*Einstellungen*).

Every edit is a step (Ctrl+Z, Ctrl+Y); Delete asks before it deletes, and names what goes with it.

## The working copy

What is changed is kept at once in the variant's working copy in this browser — the one the Layout-Tool works
on too ([`@rail-across/working-copy`](../../packages/working-copy/README.md)) —, never only in the page. Leaving
the tool asks nothing: the changes stay in the copy, the top bar says how many there are, and the suite's start
page says so at the variant. Opening the variant again, in this tool or the Layout-Tool, opens the copy. Its bar
over the plan is the Layout-Tool's (`WorkingCopyBar` of `@rail-across/ui`): project › variant, the revision, the
state, and the three ways to act on it.

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
| `src/plan/` | the site plan: the net in its true position on a MapLibre map (north up), drawn as its layers; the symbols, picking and placing |
| `src/schema/` | the schematic view: the core's strip with the symbols of a signalling plan (`planzeichen.jsx`, which symbol an object takes in `symbols.js`), picking |
| `src/panels/` | one panel per section, its menu and pages; `editors/` the editors of what *Bearbeiten* edits |
| `src/model/` | the drawing of the net and finding a place on it (`net.js`), runs over it (`runs.js`), editing the domain — places and their logical signals (`domain.js`), measuring overlaps (`overlaps.js`), steps (`history.js`), reading a PlanPro file (`planproFile.js`) |
| `src/i18n.js`, `src/labels.js` | texts in German and English, the words for the domain's values and the check's findings |

## Run

`npm run dev` here (the project server on 127.0.0.1:8787, `/api` proxied), `npm test`, `npm run lint`,
`npm run build`. In the suite it lies under `/signalling/` (`deploy/lib.sh`, `MODULE_APPS`).
