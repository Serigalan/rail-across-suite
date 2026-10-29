# Contributing

Thank you for helping build Rail-Across-Suite. This page says how a change gets in.

## Before you start

- Read the [roadmap](ROADMAP.md) and, for anything that touches shared data, the [data model](docs/data-model.md).
  Work that no phase of the roadmap covers is welcome, but open an issue first, so it can find its place.
- A change that shapes the repository or the shared data needs a **decision record** in
  [`docs/decisions/`](docs/decisions/README.md): what is decided, why, what it costs, what else was considered.
- Use the words of the [glossary](docs/glossary.md) and the units and signs of the
  [conventions](docs/conventions.md).

## How a change gets in

1. Branch from `dev`. `main` holds what is released; `dev` collects the work towards it.
2. Keep a change to one subject. A refactoring and a feature are two changes.
3. Run the checks before you push: `npm run check` (today: the import boundaries between apps and packages).
4. Open a pull request against `dev`. The owners named in [`.github/CODEOWNERS`](.github/CODEOWNERS) are asked
   for a review; a change to `packages/`, `schemas/`, the data model or the decisions needs theirs.

## Rules for code

- **Apps import packages, never each other.** What two apps need belongs in a package. Packages, the server
  and the services never import an app. `npm run check` enforces it.
- **The core has one implementation per language** (`packages/core`, `packages/core-py`); both are held to the
  shared test vectors in `schemas/`. A change to one is a change to both.
- **Derived data is not stored**, references are ids, locations are one value — see the data model's rules.
- Code, comments, documentation and commit messages are in English; texts the user reads go through
  translation keys (German and English).

## Commit messages

- A subject line in the imperative, up to about 72 characters ("Add the glossary", not "Added …").
- A body that says **why**, wrapped at 72 characters.
- Trailers at the end, each on its own line:

```
Assisted-by: ClaudeCode
Signed-off-by: Your Name <you@example.org>
```

`Assisted-by:` names a tool that helped write the change, if one did. Whoever signs off is responsible for the
change either way.

## Sign-off: Developer Certificate of Origin

Every commit carries a `Signed-off-by:` line with your real name and e-mail (`git commit -s` adds it). With it
you certify the [Developer Certificate of Origin 1.1](https://developercertificate.org/): that you wrote the
change or have the right to submit it under the project's licences. There is no separate contributor
agreement.

## Licences

By contributing you agree that your contribution is licensed like the files it changes: code under the
[Apache License 2.0](LICENSE), documentation under [CC BY 4.0](LICENSE-docs) — which files are which is listed
in the [README](README.md#licence). Material of third parties keeps its own licence and is named where it is
used; do not add any whose licence does not allow it.
