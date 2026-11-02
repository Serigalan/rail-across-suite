# Fahrzeug-Editor

The suite's vehicles: viewed, added, edited, imported from OSRD and exported for it, as OSRD's rolling stock editor
has them, in the suite's look. Its tile on the start page opens it (`vehicles/`); it needs no project.
`vehicles/?vehicle=<key>` opens one vehicle — the Fahrplan-Sim links a train's vehicle so.

## What it does

The suite's vehicles are OSRD's RailJSON rolling stock, kept on the server beside the projects
([decision 0014](../../docs/decisions/0014-vehicles-in-osrds-form.md), `/api/vehicles`); OSRD's eleven example
vehicles are built in and copied, not changed (`@rail-across/vehicles`).

- **The catalogue** (`src/FleetPanel.jsx`): the suite's and OSRD's under their categories, with a search;
  a new vehicle, and one or many imported from RailJSON files.
- **A vehicle's page** (`VehicleSheet.jsx`), as OSRD's rolling stock editor has it: what names it (OSRD's
  metadata), its categories, its dynamics (length, mass, top speed, inertia coefficient, braking deceleration,
  startup values), its rolling resistance A + B·v + C·v² in kN, kN/(km/h), kN/(km/h)² with what it comes to at top
  speed, its equipment (loading gauge, power class, the times of an electric one), and its tractive effort curves
  per traction mode and comfort — drawn with the resistance on the level (`EffortChart.jsx`) and listed as a
  table to edit, made from the starting effort and the power, or pasted from a spreadsheet (`CurveTable.jsx`).
  Exported as RailJSON for OSRD.
- **Who changes what**: everyone adds; the creator and the admins change and delete, the others copy. Leaving a
  vehicle with changes not saved asks first; a save over someone else's newer one says so and keeps the changes
  (`useVehicleEditor.js`).
- **Taken in** (`@rail-across/vehicles`, `fromOsrd`): OSRD's defaults where values are missing; Davis
  coefficients all below one are taken as N per kg, as OSRD's example files have them, and multiplied by the mass.

## Development

```sh
npm run dev   # with a project server on 127.0.0.1:8787 (OLT_API_TARGET)
npm test
npm run lint
```
