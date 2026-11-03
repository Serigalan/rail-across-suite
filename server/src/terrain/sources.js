// The Länder whose 1 m terrain model (DGM1) can be had simply: a fixed URL
// per tile, no portal, no login, no index to look a file name up in.
//
//     Thüringen            1 km XYZ in a ZIP     EPSG:25832, two vintages
//     Sachsen              2 km GeoTIFF in a ZIP EPSG:25833
//     Berlin               2 km XYZ in a ZIP     EPSG:25833
//     Brandenburg          1 km GeoTIFF in a ZIP EPSG:25833
//     Bayern               1 km GeoTIFF          EPSG:25832
//     Baden-Württemberg    2 km ZIP of four 1 km XYZ, EPSG:25832
//     Nordrhein-Westfalen  WCS, read as 1 km GeoTIFF cells, EPSG:25832
//
// The others need more than that (Schleswig-Holstein names its files after the
// flight year, so an index would have to be read first; Sachsen-Anhalt,
// Niedersachsen and Hessen hand theirs out through portal applications only;
// Mecklenburg-Vorpommern's fixed URL returns a coloured picture, not heights)
// and are left to DGM5.
//
// Heights are NHN — DHHN2016 mostly, DHHN92 in older Thüringen tiles. The
// difference is a few centimetres, below what a terrain line in a cross
// section shows.

const TH = 'https://geoportal.geoportal-th.de/hoehendaten/DGM'
const SN = 'https://geocloud.landesvermessung.sachsen.de/public.php/dav/files/JCcXyifaNdLDnxZ'
const NW = 'https://www.wcs.nrw.de/geobasis/wcs_nw_dgm'

const nwCell = (ek, nk) => `${NW}?SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCoverage&COVERAGEID=nw_dgm`
  + `&FORMAT=image/tiff&SUBSET=x(${ek * 1000},${ek * 1000 + 1000})&SUBSET=y(${nk * 1000},${nk * 1000 + 1000})`

/**
 * One Land's DGM1: `size` the tile edge in metres, `offset` where the tile
 * grid starts (Baden-Württemberg's 2 km tiles begin on odd kilometres east),
 * `vintages` [label, url(eKm, nKm)] with the kilometre of the tile's
 * lower-left corner, newest first, `box` a coarse box around the Land
 * [w, s, e, n], so a point far away is not even transformed, `fmt` 'xyz-zip',
 * 'tif-zip' or 'tif'.
 */
function source(id, name, crs, box, size, fmt, vintages, offset = [0, 0]) {
  return {
    id, name, crs, box, size, fmt, vintages, offset,
    covers: (lng, lat) => box[0] <= lng && lng <= box[2] && box[1] <= lat && lat <= box[3],
    /** Lower-left corner of the tile holding (e, n), in whole kilometres. */
    tileOf: (e, n) => [
      Math.floor((offset[0] + size * Math.floor((e - offset[0]) / size)) / 1000),
      Math.floor((offset[1] + size * Math.floor((n - offset[1]) / size)) / 1000),
    ],
  }
}

// Order matters only where boxes overlap: a Land's tiles end at its border
// (the cells beyond hold "no data"), so the next source answers there. Berlin
// comes before Brandenburg, which surrounds it.
export const SOURCES = [
  source('dgm1-th', 'Thüringen', 25832, [9.85, 50.17, 12.68, 51.68], 1000, 'xyz-zip', [
    ['2020-2025', (ek, nk) => `${TH}/dgm_2020-2025/dgm1_32_${ek}_${nk}_1_th_2020-2025.zip`],
    ['2014-2019', (ek, nk) => `${TH}/dgm_2014-2019/dgm1_${ek}_${nk}_1_th_2014-2019.zip`],
  ]),
  source('dgm1-sn', 'Sachsen', 25833, [11.85, 50.15, 15.05, 51.70], 2000, 'tif-zip', [
    [null, (ek, nk) => `${SN}/dgm1_33${ek}_${nk}_2_sn_tiff.zip`],
  ]),
  source('dgm1-be', 'Berlin', 25833, [13.07, 52.33, 13.78, 52.68], 2000, 'xyz-zip', [
    [null, (ek, nk) => `https://gdi.berlin.de/data/dgm1/atom/DGM1_${ek}_${nk}.zip`],
  ]),
  source('dgm1-bb', 'Brandenburg', 25833, [11.25, 51.35, 14.78, 53.57], 1000, 'tif-zip', [
    [null, (ek, nk) => `https://data.geobasis-bb.de/geobasis/daten/dgm/tif/dgm_33${ek}-${nk}.zip`],
  ]),
  source('dgm1-by', 'Bayern', 25832, [8.95, 47.26, 13.85, 50.57], 1000, 'tif', [
    [null, (ek, nk) => `https://download1.bayernwolke.de/a/dgm/dgm1/${ek}_${nk}.tif`],
  ]),
  source('dgm1-bw', 'Baden-Württemberg', 25832, [7.50, 47.53, 10.50, 49.80], 2000, 'xyz-zip', [
    [null, (ek, nk) => `https://opengeodata.lgl-bw.de/data/dgm/dgm1_32_${ek}_${nk}_2_bw.zip`],
  ], [1000, 0]),
  source('dgm1-nw', 'Nordrhein-Westfalen', 25832, [5.85, 50.32, 9.47, 52.54], 1000, 'tif', [
    [null, nwCell],
  ]),
]
