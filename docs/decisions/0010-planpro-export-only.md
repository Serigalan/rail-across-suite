# 0010 — PlanPro: export only

Status: **accepted** · as of 2026-11-17

## Decision

1. **The suite writes PlanPro, it does not read it.** A PlanPro 1.10 plan holds too little of a project to be taken
   in: no heights, no cant, no platforms, no km lines, switch forms only by name, the geometry of the signalling
   plan only. A plan taken in was a project to be finished by hand, and its signalling laid on another net left
   orphans.
2. **The export reads the document** (`crates/planpro/src/write.rs`, `writePlanPro` in the core), so that the
   model decides what is written, not PlanPro: every track an edge (`TOP_Kante`) along its elements (`GEO_Kante`,
   in the DB_REF Gauss-Krüger strip the project lies in), every switch a node of tip and branches with its
   `W_Kr_Anlage`, element and component, buffer stops, track names, lines and kinds (`Gleis_Art`), heights
   (`Hoehenpunkt`, `Hoehenlinie`), cant (`Ueberhoehung`, `Ueberhoehungslinie`), platforms (`Bahnsteig_Anlage`,
   `Bahnsteig_Kante`), operational points (`Oertlichkeit`), and the signalling layer — signals with their frames
   and aspects, train detection with each section's extent over the net, switch elements, routes, overlaps, flank
   protection and PZB. Existing is in both states of the plan, new only in the target, what goes only in the start.
   Ids become GUIDs by name.
3. **A plan valid against the XSD 1.10.0.3**: every object's base (`Basis_Objekt_Allg` with `Datum_Regelwerk`), the
   Container in the schema's order, the interlocking (ESTW) and the geodata (Geo) as two parts of the plan, and
   `Objektmanagement` with the planning data (project number, planner, organisation, leading operational point,
   construction stage — given by the user, else filled in). What PlanPro requires and the model does not say is
   written with a stated value and named in the report (`defaulted`); one outside element control
   (`Gleisfreimelde_Innenanlage`) stands in for the interlocking's equipment the axle counters refer to. The file
   is the container PlanPro tools exchange, `.planpro`: a ZIP of `content.xml` and `manifest.xml`.
4. **What PlanPro has no place for is named, not dropped**: the report lists, per collection, what was not written
   and why — a location in the plane, logical signals after the first, balises of systems other than PZB, station
   boundaries, stopping places and itineraries.
5. **The reader stays as the check of the export.** It is no part of the WebAssembly module; the crate's tests write
   a plan and a project in UTM and read them back (`tests/export.rs`), and replay the reader's recorded vectors.

## Decided, not built yet

The export writes next, in this order of need:

- **crossings and slips** (crossings, single and double slip switches);
- **the drawing of a plan** (`layout.xml`);
- **kilometres along lines** (`Punkt_Objekt_Strecke`, `Strecke_Punkt`) from the km lines.

## Open — before the first public release

- **PlanPro's licence.** PlanPro's XSD is licensed under RailPL V1.1, whose fit with the LGPL has not been checked.
  So the XSD is not in the repository and the crate's tests cannot validate against it; the check against the XSD
  was made outside it. The catalogue takes the names, titles and symbol lists of the 192 aspects
  (`THIRD-PARTY.md`); whether that may stand is part of the same question.
- **The stated values in a PlanPro tool.** The defaults and the placeholder for the interlocking's equipment are to
  be looked at in a PlanPro tool before a plan is passed on; the Signalling-Tool says so.
