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

## Amended 2026-11-10: a plan valid against the XSD

The export reads the infra-data-1.0 document — a record is made one first —, so that the model decides what is
written, not PlanPro: platforms (`Bahnsteig_Anlage`, `Bahnsteig_Kante`), heights (`Hoehenpunkt`, `Hoehenlinie`),
cant (`Ueberhoehung`, `Ueberhoehungslinie`), operational points (`Oertlichkeit`) and track kinds (`Gleis_Art`)
are written too. Checked against the PlanPro XSD 1.10.0.3 and a plan of ProSig, it writes what the schema
requires: every object's base (`Basis_Objekt_Allg` with `Datum_Regelwerk`), the Container in the schema's order,
the interlocking (ESTW) and the geodata (Geo) as two parts of the plan as PlanPro tools have them, and
`Objektmanagement` with the planning data (project number, planner, organisation, leading operational point,
construction stage — given by the user, else filled in). A track section gets its extent over the net from the
detection points bounding it. What PlanPro requires and the model does not say is written with a stated value and
named in the report (`defaulted`); one outside element control (`Gleisfreimelde_Innenanlage`) stands in for the
interlocking's equipment the axle counters and sections refer to. The file is the container PlanPro tools exchange,
`.planpro`: a ZIP of `content.xml` and `manifest.xml`.

## Open

- The XSD is not in the repository (THIRD-PARTY.md), so the crate's tests cannot validate against it; the
  check was made outside it. The defaults and the placeholder should be looked at in a PlanPro tool before a plan
  is passed on; the Signalling-Tool says so.
- Crossings and slips, `layout.xml` (the drawing of a plan) and kilometres along lines
  (`Punkt_Objekt_Strecke`, `Strecke_Punkt`) are not written yet.
