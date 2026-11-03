import { extend, provide } from '../core/extensions'
import ServiceStatus from './ServiceStatus'
import { dgm1Relief, dgm1Source } from './terrainDgm1'
import { serverClouds } from './cloudsOnServer'
import useCloudServer from './useCloudServer'
import useServerRun from './useServerRun'
import useCloudRegistration from './cloud3d/useCloudRegistration'
import CrossSectionCloudTools from './cloud3d/CrossSectionCloudTools'
import Cloud3dLink from './cloud3d/Cloud3dLink'
import { legacySection, workingCopiesSection } from './localStoreSections'
import { saveProjectMeta } from './projectMeta'
import { MdbDbrefImport, MdbImport } from './panels/tools'

/**
 * The server components dock onto core (Paket L, decision 277): everything
 * core offers an extension point for (core/extensions.js) is filled here,
 * once, when the main build starts — before the first render. The local
 * build never imports this file.
 */

// The two MDB importers (the service converts the database).
extend('importSections', { id: 'mdb', Component: MdbImport })
extend('importSections', { id: 'mdb-dbref', Component: MdbDbrefImport })

// Whether the services are there, at the sidebar's foot (R10.12).
extend('sidebarFoot', ServiceStatus)

// The Länder's DGM1, read by the service — as ground heights, and drawn as a
// basemap from the service's relief tiles.
extend('terrainSources', dgm1Source)
extend('basemaps', dgm1Relief)

// Point clouds on the server: read, listed and uploaded in the panel, long
// runs there, and the 3D window — its link from the map view, its button
// beside the cross section, its re-referencing seen in the section.
extend('cloudProviders', serverClouds)
provide('useCloudServer', useCloudServer)
provide('useServerRun', useServerRun)
provide('useCloudRegistration', useCloudRegistration)
extend('crossSectionTools', CrossSectionCloudTools)
extend('workspaceAddons', Cloud3dLink)

// A title or description saved in the project settings goes to the server's project list too.
provide('saveProjectMeta', saveProjectMeta)

// What the working copies and the stores of version 1 take in this browser.
extend('localStoreSections', workingCopiesSection)
extend('localStoreSections', legacySection)
