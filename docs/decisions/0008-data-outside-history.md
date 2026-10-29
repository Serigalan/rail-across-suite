# 0008 — Large data outside the history, fetched by version and hash

Status: **accepted**

## Decision

Large binary data stays out of the repository's history. A module that needs such data names each file in a
manifest (`data.json` in the module's directory) with its size, its SHA-256 and the addresses it can be fetched
from; a script (`tools/data.mjs`) fetches what is missing and accepts a file only with that hash. The first
address is a release asset of this repository (`data-<n>`, one release per version of the data); a mirror of the
original publisher may follow.

For the Open-Layout-Tool this holds:

- **NTv2 grids** (about 2 MB): release `data-1`, and cdn.proj.org, whose files are byte for byte the same.
- **Kilometrage lines** (about 190 MB): not published at all — their source does not allow it. Whoever has them
  puts them into `apps/open-layout-tool/tiles/`; the script links them into the app only where they are, so no
  build ships them otherwise.
- **Small pictures** (basemap previews, project pictures; a few kB each) stay in the repository.

When the Open-Layout-Tool moved here, its history was rewritten without `tiles/` and `grids/`: the clone went
from 100 MB to under 6 MB, every commit kept.

## Why

- **The history never shrinks.** A file once committed is carried by every clone for good; the kilometrage tiles
  alone were 195 MB over the Open-Layout-Tool's history, with every rebuild adding more.
- **Data that may not be published must not be in a public history** — not even as an old blob.
- **A hash makes the version exact.** Tests and servers get the very bytes the manifest names, from wherever
  they come; a changed file is a changed manifest, reviewed like code.

## What it costs

- One more step before tests and builds (`npm run data`), and in CI and the deployment scripts. A file that is in
  place with the right hash costs nothing.
- Someone has to upload a new data version as a release.
- The kilometrage lines are not there for whoever clones the repository; tests that need them are skipped.

## Alternatives considered

- **Git LFS.** `git clone` would bring everything, but GitHub's free LFS quota (1 GB storage and bandwidth a
  month) would be used up by a few clones of the tiles, and the data that may not be published would still sit
  in a public store.
- **A data repository**, referenced by tag. A second repository to keep and to grant access to; for the few files
  that may be published, a release does the same.
