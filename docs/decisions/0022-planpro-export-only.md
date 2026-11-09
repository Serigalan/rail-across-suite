# 0022 — PlanPro: export only

Status: **accepted** · 2026-11-09 · changes [0011](0011-signalling-domain.md) as far as PlanPro is exchanged

## Decision

1. **The suite writes PlanPro, it does not read it.** A PlanPro 1.10 plan holds too little of a project to be
   taken in: no heights, no cant, no platforms, no km lines, switch forms only by name, the geometry of the
   signalling plan only. A plan taken in was a project that had to be finished by hand, and its signalling laid on
   another net left orphans. The import of the Signalling-Tool goes, with `placeSignalling` and the reader's face
   in the core (`readPlanPro`, `planproRecord`, `parseXml`).
2. **The export writes what the reader read** (`crates/planpro/src/write.rs`, `writePlanPro` in the core): every
   track an edge (`TOP_Kante`) along its elements (`GEO_Kante`, in the DB_REF Gauss-Krüger strip the project lies
   in), every turnout a node of tip, left and right with its `W_Kr_Anlage`, element and component, buffer stops,
   track names and lines, and the signalling domain — signals with their frames and aspects, train detection,
   switch elements, routes, overlaps, flank protection and PZB. Existing is in both states of the plan, new only
   in the target, what goes only in the start. Ids become GUIDs by name.
3. **The reader stays as the check of the export.** It is no part of the WebAssembly module any more; the crate's
   tests write the reader's test plan and a project in UTM and read them back (`tests/export.rs`), and replay the
   reader's recorded vectors (`tests/replay.rs`).
4. **What PlanPro has no place for here is named, not dropped**: the report of an export lists, per collection,
   what was not written and why — a location in the plane, crossings and slips, logical signals after the first,
   balises of systems other than PZB, station boundaries, stopping places and itineraries.

## Open

- The export is not validated against the PlanPro XSD, which is not in the repository (THIRD-PARTY.md). Mandatory
  parts the reader never read (`Basis_Objekt_Allg`, `Objektreferenzen`, `Bearbeitungsvermerk`, …) are not
  written. A plan should be opened in a PlanPro tool before it is passed on; the Signalling-Tool says so.
- The namespace of the aspects (`nsSignalbegriffe_Ril_301`) and `TOP_Anschluss` `Verbindung` for two tracks
  joining are taken as PlanPro 1.10 names them, without the schema at hand.
