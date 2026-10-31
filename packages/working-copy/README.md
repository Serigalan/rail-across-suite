# @rail-across/working-copy — the local working copy of a variant

What a user changes in a tool of the suite is kept in the browser first, not on the server: one working copy per
variant, in IndexedDB. Every tool works on the same one — the Layout-Tool on the tracks, the Signalling-Tool on
the signalling — so what one tool changed the next one opens with, and checking in from either sends both. The
suite's start page reads them to say, at each variant, how many changes it holds that are not checked in.

```js
import { getWorkingCopy, putWorkingCopy, localChangeCount } from '@rail-across/working-copy'
```

| Part | What it does |
|---|---|
| `getWorkingCopy(variantId)`, `getAllWorkingCopies()`, `putWorkingCopy(record)`, `deleteWorkingCopy(variantId)` | The store: DB `olt`, version 2, object store `workingCopies`, keyed by `variantId` — `{ variantId, projectId, base, basePayload, project, idLog, updatedAt, format }`: the record being edited (dehydrated), the revision it rests on and that revision's record, the splits and joins since. A copy of another format than the current record format (`oltjson-1.1`) is not read. |
| `openDb`, `request`, `done`, `WORKING` | The database and two promise helpers, for a tool that lists or empties more of it (the Layout-Tool's local storage dialog). |
| `localChanges(wc)` | What the copy changed against its base, as the core's project diff lists it (`diffEntries`). |
| `localChangeCount(wc)`, `domainChanges(before, after)` | How many changes it holds, the signalling counted object by object. |

Nothing is written when a copy is only opened; a tool writes it on its first change. Leaving a tool asks nothing:
the changes stay in the copy until they are checked in or thrown away.

The name and version of the database are the Layout-Tool's from before this package, so the copies kept then are
still read.
