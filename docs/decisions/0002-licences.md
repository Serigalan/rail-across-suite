# 0002 — Code under Apache-2.0, documentation under CC BY 4.0, contributions signed off

Status: **accepted**

## Decision

- **Code** — everything that is not documentation, including the schemas — under the Apache License 2.0
  (`LICENSE`).
- **Documentation** — the README files, the roadmap, `docs/` and the architecture diagram — under Creative
  Commons Attribution 4.0 International (`LICENSE-docs`).
- **Third-party material** keeps its own licence and is named where it is used.
- **Contributions** are signed off after the Developer Certificate of Origin 1.1 (`Signed-off-by:`); there is no
  contributor licence agreement.

## Why

- **Wide use.** The toolkit should be usable by railway operators, planning offices, authorities and software
  vendors, also inside their own products. A permissive licence puts no condition on that beyond keeping the
  notice.
- **Apache-2.0 rather than MIT**: it grants a licence to the contributors' patents and ends it for whoever sues
  over them, and it is the permissive licence large organisations accept most readily.
- **CC BY 4.0 for documentation**: a licence made for texts and pictures, permissive like the code's, asking only
  for attribution.
- **DCO rather than a CLA**: one line per commit instead of a signed agreement, the usual practice in open
  projects. The project already signs off its commits.

## What it costs

- Others may build closed products or services on the toolkit without giving improvements back. That was
  weighed against copyleft licences (AGPL-3.0, EUPL-1.2), which would require it, and accepted for wider use.
- Without a CLA the licence can later only be changed with the consent of every contributor — another reason it
  was decided before the first outside contribution.

## Alternatives considered

| Licence | Why not |
|---|---|
| MIT | no patent grant |
| AGPL-3.0, EUPL-1.2 | would keep improvements open also for hosted services, but deters adoption by vendors and offices |
| MPL-2.0 | file-level copyleft; a middle way not needed for the goal of wide use |
