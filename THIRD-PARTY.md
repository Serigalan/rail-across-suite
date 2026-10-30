# Third-party material

Material in this repository that is not the project's own work keeps its own licence. This list names each such
item, where it comes from and under which terms it is redistributed. Add an entry with every new item.

The entries below arrived with the move of the Layout-Tool (roadmap, phase 0).

## NTv2 transformation grids

The grids transform historic German datums to ETRS89. They are the GeoTIFF conversions of the
[PROJ-data](https://github.com/OSGeo/PROJ-data) package, identical to its files (SHA-256 checked for all four
against cdn.proj.org). They are not in the repository's history: `apps/layout-tool/tools/data.mjs` fetches them
by the hashes in `apps/layout-tool/data.json`, from this repository's release `data-1` or from cdn.proj.org.
Licence and source as PROJ-data states them:

| File | Area | Source | Licence |
|---|---|---|---|
| `de_adv_BETA2007.tif` | Germany | Arbeitsgemeinschaft der Vermessungsverwaltungen der Länder (AdV), via [BKG](http://crs.bkg.bund.de/crseu/crs/descrtrans/BeTA/de_dhdn2etrs_beta.php) | "Free redistribution is allowed and welcome" (AdV) |
| `de_hvbg_hessen_HeTA2010.tif` | Hessen | Hessische Verwaltung für Bodenmanagement und Geoinformation (HVBG) | [Datenlizenz Deutschland – Namensnennung – 2.0](https://www.govdata.de/dl-de/by-2-0) |
| `de_lgvl_saarland_SeTa2016.tif` | Saarland | Landesamt für Vermessung, Geoinformation und Landentwicklung Saarland (LVGL) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |
| `de_tlbg_thueringen_NTv2gridTH.tif` | Thüringen | Thüringer Landesamt für Bodenmanagement und Geoinformation (TLBG), [Geoportal Thüringen](https://geoportal.thueringen.de/gdi-th/download-offene-geodaten/ntv2-gittertransformation) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |

Attribution: "NTv2 grids © AdV, HVBG Hessen (dl-de/by-2-0), LVGL Saarland (CC BY 4.0), TLBG Thüringen
(CC BY 4.0); GeoTIFF conversion by the PROJ project".

## Kilometrage data

The kilometrage lines the Layout-Tool shows (`tiles/km/`, `tiles/km_linie.pmtiles`) are **for tests
only**. They are neither in this repository nor in its history, and no build ships them: whoever has them puts them
into `apps/layout-tool/tiles/`, from where `tools/data.mjs` links them into the app. A test fixture taken from
them names its source where it lies.

## Used, not included

| Software | Used for | Licence |
|---|---|---|
| [OSRD](https://github.com/OpenRailAssociation/osrd), OpenRail Association | the base of Open-Capacity-Sim, run as a service at a pinned version ([decision 0006](docs/decisions/0006-capacity-on-osrd.md)) | LGPL-3.0 |

## Project pictures

The default project pictures (`apps/home/public/project-images/`, and for the Layout-Tool's local build
`apps/layout-tool/public/project-images/`) use country outlines from [Natural Earth](https://www.naturalearthdata.com) 1:50m
(public domain) and pictograms from [Font Awesome Free](https://fontawesome.com) 7.3.1
([CC BY 4.0](https://fontawesome.com/license/free)).

## Not third-party

- `Leitfaden_Trassierung.pdf` and its HTML versions are the project's own work, published under CC BY 4.0 like
  the rest of the documentation.
- The values from the DB guidelines (Ril 800.0110, 800.0120, 800.0130) in the rule catalogues may be taken over.
