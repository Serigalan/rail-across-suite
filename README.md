# Open-Rail-Designer
Modular open-source toolkit for railway infrastructure planning, built around a shared data core.

![Open-Rail-Designer architecture](architecture.svg)

## Repository

One repository holds the core, every module and the services
([decision 0001](docs/decisions/0001-one-repository.md)):

| Directory | Content |
|---|---|
| `packages/` | the Rail-Data-Core (JavaScript and Python) |
| `schemas/` | JSON Schema per domain, shared test vectors |
| `apps/` | the modules, starting with the Open-Layout-Tool |
| `server/` | the project server for all modules |
| `services/` | the Python services |
| `docs/` | roadmap material, data model, decisions |

## Roadmap

The [roadmap](ROADMAP.md) starts with the Rail-Data-Core; how its data is structured and stored is proposed in
[docs/data-model.md](docs/data-model.md).

## Licence

Copyright 2026 Serigalan and the Open-Rail-Designer contributors.

- **Code** — everything that is not documentation, including the schemas — under the
  [Apache License 2.0](LICENSE) (`Apache-2.0`).
- **Documentation** — `README.md`, `ROADMAP.md`, the `README.md` files in the directories, everything in `docs/`
  and `architecture.svg` — under [Creative Commons Attribution 4.0 International](LICENSE-docs) (`CC-BY-4.0`).
- **Third-party material** keeps its own licence, named where it is used.

Contributions are made under these licences and signed off (`Signed-off-by:`) after the
[Developer Certificate of Origin](https://developercertificate.org/).
