# 0010 — One suite on one server; modules named by their task

Status: **accepted** · replaces the app names of [decision 0001](0001-one-repository.md) and
[decision 0007](0007-project-name.md) (`open-layout-tool`, `open-signalling-tool`, …)

## Decision

1. **Rail-Across-Suite is one application on one server.** Its start page, every module's app (each under its
   own path: the Layout-Tool under `/layout/`), the project server and the modules' services run behind one
   address. The project server and the deployment are the suite's (`server/`, `deploy/`), not a module's.
2. **Signing in comes before the suite.** Nothing of it opens without a session: the server asks the project
   server for every page and shows the suite's sign-in page in its place until one is signed in. A start
   password is changed there before anything opens. One account for every module.
3. **The suite manages what is the suite's:** the account (password, signing out) and the users with their
   roles and the modules' rights (`apps/home`, `#/admin`). A module has no sign-in or user administration of its
   own; it sends to the suite's pages and gets the user back.
4. **Modules are named by their task.** The Open-Layout-Tool is the module **Layout-Tool** (`apps/layout-tool`);
   later modules follow: `signalling-tool`, `catenary-tool`, `timetable-editor`, `capacity-sim`,
   `construction-sim`. "Open" is no part of a module's name; the suite's name says what it is.

What stays open without a session: the sign-in page and the built files it and the apps load, and the entry of
the Layout-Tool's read-only share links, whose holders have no account.

## Why

- **One account, one place.** Users who plan track layout, signalling and timetables in the same project should
  sign in once and be managed once; a sign-in per module would mean as many accounts and administrations.
- **Nothing to see before signing in.** A start page that showed itself and asked for a sign-in only further in
  told every visitor what the suite holds; asking at the door shows nothing.
- **A module is a part, not a product.** The names say what each part does inside the suite.

## What it costs

- The server's site configuration asks the project server for every page that is not open — one small request
  each, answered from the session table.
- The Layout-Tool's own mark and the wordmark in its plan title block still show its old name and need a new
  design.
- Data the Layout-Tool keeps in browsers keeps its internal names (`olt-…` storage keys), so nothing kept is lost.
