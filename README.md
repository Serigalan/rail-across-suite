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
