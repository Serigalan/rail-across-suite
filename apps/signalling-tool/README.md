# Signalling-Tool

The module of Rail-Across-Suite for planning signalling on the net the Layout-Tool draws: signals and boards,
train detection, the switch elements of the interlocking, train and shunting routes with their overlaps, flank
protection and the balises and magnets of train protection (PZB, ETCS) — the domain `signalling` 1.0 ([decision 0011](../../docs/decisions/0011-signalling-domain.md)).
Its look is the suite's library ([`@rail-across/ui`](../../packages/ui/README.md)), shaped like the Layout-Tool:
the rail of sections on the left, the panel of the one chosen, the plan of the net.

It reads the core and never writes it (data model, R1): the tracks are the Layout-Tool's. Every location it
stores stands on a track of the core, and the core carries it when the Layout-Tool splits, joins, turns round or
deletes a track (R4, R7).

## What it does

| Section | What it holds | How a new one is made |
|---|---|---|
| Kartenebenen | the map under the plan, first in the rail as in the Layout-Tool: the suite's basemaps as the Layout-Tool offers them ([`@rail-across/basemaps`](../../packages/basemaps/README.md)) — OpenStreetMap, satellite, relief, basemap.de, DGM5, the states' orthophotos — or the plan alone; the choice is kept in the browser | — |
| Signale | signals and boards; function, mounting, sight; its logical signals, one per signalling system (Ks, shunting signal) with its kind there, as OSRD has them; frames with their aspects from the catalogue of Ril 301 — a screen offers what its kinds may show and marks what they must | *Neues Signal* or *Neue Tafel*, then a click on a track: the side is the side clicked |
| Gleisfreimeldung | detection points; sections bounded by them, their extent derived over the net and drawn | a point by a click; a section, then its points clicked on the plan |
| Weichenelemente | an element per switch of the core (a switch without one is offered); derailers at their place | from the list of switches without one; a derailer by a click |
| Fahrstraßen | train, shunting and middle-switch routes: the run found over the net from start to target signal the way they act, what they show, the overlaps at the target; overlaps run their length ahead, straight on at every switch | *Neue Fahrstraße*, *Neuer Durchrutschweg* |
| Flankenschutz | per switch element and the way it is run over: a signal, a switch or derailer in a position, or waived | *Neuer Flankenschutz* |
| Geschwindigkeiten | the speed profile after OSRD's SpeedSection: a stretch from a start over the run to an end, its speed, per kind of train, the direction and the routes it holds on | *Neuer Abschnitt*, then its start and its end on the plan; only where a run leads from the start |
| Balisen/Magnete | the track elements of train protection: of a system (PZB, ETCS) and a kind (track magnet, speed check; fixed or controlled Eurobalise), with the settings the catalogue declares for them — frequency, test speed, place in the group …; the signal they belong to, how they act | a click on a track |
| Prüfung | the core's `checkSignalling` on every edit; a finding leads to its object | — |
| PlanPro | a plan's signalling (`.planpro`, `.ppxml`) laid on the project's net: on the plan's own edges where the project was made from it, else carried to the tracks lying there, what nothing lies under orphaned | a file |
| Einstellungen | at the foot of the rail, as in the Layout-Tool: the language — the suite's, which the start page and the Layout-Tool share (`@rail-across/ui`) | — |

Setting an object works as in OSRD's editor: the mouse snaps to the nearest track within reach, and what would
be set is drawn there before the click — a signal two metres off the track on the side the mouse is, acting so
that it stands on the right; a balise or magnet at the rail. Every object is drawn where it stands in the plane:
a dot on its track, a stroke out to its symbol at its offset, turned the way it acts; its editor states the
point (easting, northing), derived from track, station, side and offset and never stored.

Every text is German or English, the catalogue's titles too, in the language of the suite (*Einstellungen*).

Every edit is a step (Ctrl+Z, Ctrl+Y); Delete asks before it deletes, and names what goes with it. Checking in
sends the record with its signalling; where someone checked in first, their state is merged in (the signalling
whole, the core carried) and the user checks in again — or, where both changed the signalling, lays theirs on
the new net.

## Its address

As every tool of the infrastructure (`apps/README.md`): `?project&variant` works on the variant, `&action=view`
or `history` shows its head read-only. Comparing and merging (`action=compare|merge`) is the Layout-Tool's, for
the whole project with its signalling: the tool hands those addresses on to it. It keeps nothing in the browser but the basemap last chosen and the language.

## Files

| Path | What it is |
|---|---|
| `src/App.jsx`, `src/suite.js` | what the address asks, the session, loading the variant |
| `src/Editor.jsx` | the shell: sections, selection, steps, the check, checking in |
| `src/plan/` | the plan: the net in its true position on a MapLibre map (north up), drawn over it in screen pixels; the symbols, picking and placing |
| `src/panels/` | one panel per section |
| `src/model/` | the drawing of the net and finding a place on it (`net.js`), runs over it (`runs.js`), editing the domain (`domain.js`), steps (`history.js`), reading a PlanPro file (`planproFile.js`) |
| `src/i18n.js`, `src/labels.js` | texts in German and English, the words for the domain's values and the check's findings |

## Run

`npm run dev` here (the project server on 127.0.0.1:8787, `/api` proxied), `npm test`, `npm run lint`,
`npm run build`. In the suite it lies under `/signalling/` (`deploy/lib.sh`, `MODULE_APPS`).
