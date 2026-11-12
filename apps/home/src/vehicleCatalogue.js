/**
 * The suite's vehicle catalogue as the start page needs it for a project's
 * operations file (decision 0026): every vehicle there is — OSRD's built in,
 * the server's —, to carry those the trains run with on export, and to make one
 * a file brings that the catalogue lacks on import. Loaded only when asked.
 */

const catalogue = () => import('@rail-across/vehicles')

/** The catalogue's entries ({ key, vehicle }); the built-in ones alone where the server's cannot be had. */
export async function catalogueEntries() {
  const { BUILT_IN, suiteEntry, vehicleApi } = await catalogue()
  const own = await vehicleApi.list().then(list => list.map(suiteEntry), () => [])
  return [...BUILT_IN, ...own]
}

/** A vehicle a file brings, made in the server's catalogue: { id }. */
export async function createVehicle(vehicle) {
  const { vehicleApi } = await catalogue()
  return vehicleApi.create(vehicle)
}
