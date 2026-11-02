# merge — comparing and merging states of a project

`packages/core/src/merge/` · needs `project`, `track`

Three-way merge of two states of a project against the state both came from, the diff behind it, carrying
references across splits and joins of tracks. Drawing a comparison on the map is the app's (the Layout-Tool's
`compare` module). Pure, without a DOM: the
browser merges when checking in over a stale base or merging variants; the module also loads and runs under
plain Node (a test holds it to that), as everything the project server might share has to.

## Files

| File | What it does |
|---|---|
| `collections.js` | What a record is made of for merging: the collections and their keys, fields, canonical comparison, labels |
| `diff.js` | `diffProject`, `diffEntries`: what changed between two dehydrated states |
| `merge.js` | `mergeProject(base, mine, theirs)` → merged record, conflicts, warnings, what was applied; `resolve` |
| `remap.js` | Carrying a reference across a change of track ids, by geometry |
| `carryLocations.js` | Carrying the locations of a domain the core does not write (signalling …) across a change of the net: after every write of the Layout-Tool (`carryAfterWrite`) and in a merge (R4, R7) |
| `index.js` | The module's public interface |

## The unit of merging

A record is the **dehydrated** project — only primary data; derived geometry neither counts as a change nor
rides into the result. Collections are compared by id:

| Collection | Key | Merged as |
|---|---|---|
| `tracks` | `id` | field by field; `elements` and `heights` are one field each (elements have no ids) |
| `switches` | `switchId` | field by field; `ports` is one field |
| `platforms` | `id` | field by field; its location — `trackId`, `startStation`, `endStation`, `side` — is one field, `at` |
| `kmLines` | `lineNumber` | whole object |
| `endMarks`, `axisSurveys`, `referenceAxes`, `routes` | `id` | whole object |

The project's own fields (title, settings, plan header, image hash …) are merged field by field too.

## The rules

1. Carry first, then compare (data model, R6). The tracks are merged first; then base, mine and theirs are
   each carried onto the tracks the result has, through both id logs — splitting and joining give pieces new
   ids, and the working copy logs them as `{ from, to: [ids] }`. Only then are the other collections compared,
   so a change that is only carrying is no change. What still names a track the result does not have
   afterwards (a choice taking an object from one side) is carried once more.
2. Compare collections by id: added, removed, changed.
3. Merge field by field: different fields changed on the two sides → both taken; the same field changed
   differently → conflict; changed alike → no conflict. A location is one field (R5): a platform's track,
   stations and side are never taken from different sides.
4. Deleted against changed is always a conflict.
5. Validate the result (`project/validateProject`); an error the merge brings in is a conflict on the object,
   decided by taking the whole object from one side (`object:<collection>:<id>`), after which it is validated
   again. Warnings it brings in (e.g. a track drawn on both sides) are listed.

**Case F2 of the data model.** Alice splits track `T` at 100 m, which re-stations platform `P` (120–260 m) to
`T2` at 20–160 m; Bob, on the old state, moves `P` to 135–275 m on `T`. Compared field by field and carried
afterwards, the merge took `trackId = T2` from Alice and a conflict on the stations — and either side of it was
wrong: Bob's put `P` 100 m off, Alice's lost his move. Carried first, base and Alice say `T2` at 20–160, Bob
`T2` at 35–175: only Bob changed it, and the result is `T2` at 35–175 without a conflict, in both directions
(`merge.test.js`).

**Nothing repairs.** Where both sides changed the same thing differently it is a conflict, decided by choosing
a side, never by mixing. The merged record is dehydrated; whoever adopts it hydrates it.

## Remapping by geometry

The id log says which pieces an old track became, not which piece a given point went to — and it does not
have to: the pieces lie exactly where the old track lay. A switch port or end mark on `a.END` goes to the piece
one of whose ends is where `a`'s end was; a platform goes to the piece on which both its stations lie (at the
stations there, with the side swapped if that piece runs the other way); a route's track is replaced by the
pieces in order along it. That holds over any number of splits and joins, which a table of offsets would have
to replay one by one.

## Carrying the other domains' locations (R4)

A domain the core does not write says where its objects stand with the core's location types only: a
`TrackLocation` (`{ anchor: 'station', trackId, station, side, offset, direction }`) and a `TrackPath`
(`{ trackIds }`). `carryDomain` looks for those shapes and nothing else, so it carries a domain it has never
heard of. A location on a track that changed is looked for, at the place it stood, on the tracks lying there
now:

| What the net did | What happens to the location |
|---|---|
| split, joined | onto the piece the place lies on, at its station there |
| turned round | the station from the other end; side and direction swap |
| a piece put in front, one taken off | the station moves with the track's stationing |
| reshaped away from the place, same track | the station stays: it follows the track |
| deleted, nothing lies there | orphaned in the same step: the plane point and bearing it had (R7), id and references kept |

A run of tracks takes, for each track that went, the pieces lying where it lay, in running order (as a
route does, decision 249). The Layout-Tool calls `carryAfterWrite` in its write pipeline, so every edit
carries the record's `signalling` before it is stored; the merge carries it through the id logs with the
platforms (`carryReferences`).

## Decisions

- **The track is the unit**, elements and heights one field each.
- **Only dehydrated records are versioned and merged**; the result is validated, a violation is a conflict.
- **Remaps by geometry**, not by stored offsets.
- **The browser merges** (the server only takes revisions on the current head and checks them), so no result
  is stored that no user has seen.

## Tests

`merge.test.js` (incl. the diff, the plain-Node load and the PEK data set) and `remap.test.js`.
