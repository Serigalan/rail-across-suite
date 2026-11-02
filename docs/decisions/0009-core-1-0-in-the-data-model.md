# 0009 — core 1.0 has the data model's shape from the start

Status: **accepted** · changes the roadmap: the format changes listed for phase 2 come with core 1.0 in phase 1

## Decision

The first version of the Rail-Data-Core's format, **core 1.0**, is written in the shape of the
[data model](../data-model.md), not as oltjson-1.1 under a new name:

1. **Domains.** A project is the domains `core` and `layout`, each with its version, its JSON Schema (named by
   hash), its fields and its collections (section 4).
2. **One value per location.** A platform stands at a `TrackRange`, an end mark and a switch port hold a
   `TrackEnd`, a named run of tracks is a `TrackPath`; locations state their anchor (R5, R11).
3. **Lifespans over stages.** The planning status is a `Lifespan` over the core's stage list — for now the two
   stages every project has, the existing and the final state (R12, [decision 0003](0003-construction-stages.md)).
4. **An operational point register.** Station names and codes are points of a register the tracks and
   platforms refer to (R10).
5. **Storage by collection.** Collections sorted by id, canonical JSON after RFC 8785, bulk point arrays as
   blobs named by their hash, a manifest of hashes per domain and collection (S-R1 to S-R3). A single file holds
   it as `.railacross.json` and carries the schemas (S-R9).
6. **One door with a migration chain.** oltjson-1.1 opens by being migrated into core 1.0 — files, and the
   project server's older revisions; what cannot be migrated is refused (S-R10).

In phase 1 the Open-Layout-Tool keeps its record (oltjson-1.1) as the model it works on, in memory and in the
browser's own storage; what crosses a boundary — a file, the project server — is the core 1.0 document. The
conversion between the two is lossless both ways. What the record does not hold yet — stages of a project's
own, a register edited as such — is drawn from what it holds, and a document that has more is refused until the
app holds it (phase 2).

## Why

- **The first published version is the one everybody migrates from.** A core 1.0 that is oltjson-1.1 renamed
  would be followed by a core 2.0 in phase 2 — a second migration for every file and every revision, and two
  versions for the Python core and every module to read.
- **The other modules read the format, not the app's record.** Signalling, catenary and the simulations are
  written against the schemas; they should start from the shape they keep.
- **The shape is decided.** The data model was reviewed and accepted with exactly these rules; carrying them out
  in the format costs little next to the remap and the stage views, which stay in phase 2.

## What it costs

- A conversion layer between the app's record and the document, with tests that it loses nothing — the
  Open-Layout-Tool's own "one format everywhere" holds again only once the app works on the document (phase 2).
- Until then, the stage list and the register are drawn from the record (ids drawn from the project's id and
  the station names); a station renamed in a track's fields is a new point.
- Collections sorted by id: the order tracks were drawn in is not kept.
- The project server's API speaks documents; its older revisions are migrated each time they are read.

## Alternatives considered

- **core 1.0 = oltjson-1.1 under the new name**, the data model's changes in a core 2.0 with phase 2: one
  migration more, and modules written against a format about to change.
- **The app on the document right away**: everything the app reads and writes would change at once — phase 2's
  work, without the remap and the stage views that make it worthwhile.
