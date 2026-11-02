# Vehicles from OSRD

`osrd/` holds the eleven example vehicles of OSRD (`tests/data/rolling_stocks/realistic/` at commit
`fda69ee1f8027175096371d258c563fc98f9295e`), unchanged, in OSRD's RailJSON rolling stock format. They are not
types of real vehicles but typical ones of their kind: a commuter train, regional trains (diesel and electric),
tram-trains, an intercity, a high-speed and a night train, freight trains and a touristic train. They are OSRD's
work, © 2022 The OSRD Contributors, under LGPL-3.0-only, the suite's licence.

They are the suite's built-in vehicles: `src/builtIn.js` takes them into the build, the Fahrzeug-Editor shows and
copies them, the Fahrplan-Sim runs them.

## What to know about them

- **Rolling resistance per kilogram.** The Davis coefficients of these files (`A` 0.0077, `B` 0.00033, `C`
  0.000022 for the commuter train) are newtons per kilogram of the train — the unit OSRD gives a towed rolling
  stock's —, while OSRD reads a rolling stock's as newtons for the whole train. Taken as newtons they make the
  resistance to movement vanish. Times the mass they give what such trains have (the high-speed train: 66 kN at
  320 km/h, about what a TGV meets). `fromOsrd` multiplies them by the mass when it takes them in, and the editor says so.
- The heavy freight train's `A` (0.000012 N/kg, 25 N for 2090 t) is likely meant to be the light freight
  train's 0.0012; it is left as OSRD has it.
- The touristic train has no rolling resistance at all.
- Each has a curve for comfort `STANDARD` and one for `AIR_CONDITIONING` per traction mode (`thermal`, `750V`,
  `1500V`, `25000V`).
