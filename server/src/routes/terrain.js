import { ApiError } from '../errors.js'
import { sample } from '../terrain/index.js'
import { MAX_ZOOM, MIN_ZOOM, render } from '../terrain/dem.js'
import { MdbError, convertMdb } from '../mdb.js'

// What the Layout-Tool cannot do in the browser because the data is not to be
// had there (decision 0019): the Länder's DGM1 — heights for points, and relief
// tiles for the map — and converting an Access file for the MDB import.

const MAX_TERRAIN_POINTS = Number(process.env.RA_TERRAIN_MAX_POINTS ?? 20000)
const MAX_MDB_BYTES = Number(process.env.RA_MDB_MAX_BODY ?? 128 * 1024 * 1024)
/** How long the browser keeps a relief tile drawn with every DGM1 tile it needed [s]. */
const DEM_MAX_AGE = 7 * 24 * 3600
const MDB_STATUS = { not_a_database: 400, no_records: 422, timeout: 504 }

export default async function terrainRoutes(api) {
  const opts = { preHandler: api.requireUser }

  // Ground heights for WGS84 points: { heights, sources }, null where no Land has one.
  api.post('/terrain', opts, async (req) => {
    const lnglat = req.body?.lnglat
    if (!Array.isArray(lnglat)) throw new ApiError(400, 'invalid_payload')
    if (lnglat.length > MAX_TERRAIN_POINTS) throw new ApiError(413, 'too_large')
    return sample(lnglat)
  })

  // A relief tile in terrain-RGB to the centimetre. Drawn without a DGM1 tile
  // the portal did not hand out, it is sent with `no-store`, so the browser
  // asks again rather than keeping the gap for a week.
  api.get('/dem/:z/:x/:file', opts, async (req, reply) => {
    const z = Number(req.params.z)
    const x = Number(req.params.x)
    const y = Number(/^(\d{1,7})\.png$/.exec(req.params.file)?.[1])
    if (!Number.isInteger(z) || !Number.isInteger(x) || !Number.isInteger(y) || z < MIN_ZOOM || z > MAX_ZOOM || x >= 2 ** z || y >= 2 ** z) {
      throw new ApiError(400, 'invalid_payload')
    }
    const { png, keep } = await render(z, x, y)
    reply.header('Cache-Control', keep ? `private, max-age=${DEM_MAX_AGE}` : 'no-store')
    if (!png) return reply.code(404).send({ error: 'not_found' })
    return reply.type('image/png').send(png)
  })

  // An Access file, as bytes, converted and removed again.
  api.register(async (raw) => {
    raw.addContentTypeParser('application/octet-stream', { parseAs: 'buffer', bodyLimit: MAX_MDB_BYTES },
      (req, body, done) => done(null, body))
    raw.post('/mdb', { ...opts, bodyLimit: MAX_MDB_BYTES }, async (req) => {
      if (!Buffer.isBuffer(req.body) || !req.body.length) throw new ApiError(400, 'invalid_payload')
      try {
        return await convertMdb(req.body)
      } catch (err) {
        if (err instanceof MdbError) {
          if (err.code === 'internal') req.log.error(err.message)
          throw new ApiError(MDB_STATUS[err.code] ?? 500, err.code)
        }
        throw err
      }
    })
  })
}
