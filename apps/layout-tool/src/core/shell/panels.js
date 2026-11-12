import { lazy } from 'react'
import { HistoryIcon, ProjectSettingsIcon, LayerIcon, TopologyIcon, PlaceIcon, SettingsIcon, DataExchangeIcon, EditElementIcon, ConnectSwitchIcon, SpliceElementIcon, CrossSectionIcon, PlanExportIcon, ElevationIcon, CheckIcon, NewPlatformIcon } from '../components/icons'
import { TOOL } from '../tool'

// Each panel is its own chunk (R9.1), loaded the first time it is opened.
const ProjectSettingsPanel = lazy(() => import('../components/panels/ProjectSettingsPanel'))
const LayersPanel = lazy(() => import('../components/panels/LayersPanel'))
const TopologyPanel = lazy(() => import('../components/panels/TopologyPanel'))
const CreatePanel = lazy(() => import('../components/panels/CreatePanel'))
const ConnectSwitchPanel = lazy(() => import('../components/panels/ConnectSwitchPanel'))
const SpliceConnectPanel = lazy(() => import('../components/panels/SpliceConnectPanel'))
const ElevationPanel = lazy(() => import('../components/panels/ElevationPanel'))
const CrossSectionsPanel = lazy(() => import('../components/panels/CrossSectionsPanel'))
const EditElementPanel = lazy(() => import('../components/panels/EditElementPanel'))
const SettingsPanel = lazy(() => import('../components/panels/SettingsPanel'))
const DataExchangePanel = lazy(() => import('../components/panels/DataExchangePanel'))
const HistoryPanel = lazy(() => import('../components/panels/HistoryPanel'))
const PlanExportPanel = lazy(() => import('../components/panels/PlanExportPanel'))
const CheckPanel = lazy(() => import('../components/panels/CheckPanel'))
const CivilPlatformsPanel = lazy(() => import('../components/panels/CivilPlatformsPanel'))

/**
 * The panels of the map view (R2.3), in the order the sidebar shows them.
 * The sidebar and the panel pane are both made from this list.
 *
 * In the order of the work (R10.7): the project's settings first (decision
 * 317); the view on the map; laying out the alignment — create (platforms
 * too), splice, the switches; editing; the heights; cross sections; checking
 * against the rules; exchange; the plan. The sidebar draws a line where the
 * `group` changes.
 *
 *   id          what the shell calls the panel
 *   icon        its sidebar symbol, titleKey its name
 *   group       the work step it belongs to
 *   place       'top' or 'bottom' of the sidebar, 'none' when it is opened
 *               from elsewhere (the history, from the button between the arrows)
 *   Component   the panel itself
 *   props(s)    what it takes from the shell `s` (state and handlers in App)
 *   onEnter(s)  runs when the panel is opened, onLeave(s) when it is left
 *   overlay     the map overlay that belongs to the panel and closes with it
 *   tools       the tools that show it (decision 0025): 'layout', the alignment,
 *               and 'civil', the civil layer; without, both
 */
const ALL_PANELS = [
  {
    id: 'project', tools: ['layout'], group: 'project', icon: ProjectSettingsIcon, titleKey: 'project_settings', place: 'top', Component: ProjectSettingsPanel,
    overlay: 'projectTracks',
    props: (s) => ({
      tracksOpen: s.overlay?.kind === 'projectTracks',
      onShowTracks: (on) => (on ? s.openOverlay({ kind: 'projectTracks' }) : s.closeOverlay('projectTracks')),
      kmLocalOn: s.kmOverlays.local, onKmOverlayChange: s.setKmOverlay,
    }),
    // The tracks come up with the panel, over the map (decision 324).
    onEnter: (s) => s.openOverlay({ kind: 'projectTracks' }),
  },
  {
    id: 'layers', group: 'view', icon: LayerIcon, titleKey: 'tooltip_layers', place: 'top', Component: LayersPanel,
    props: (s) => ({
      activeBasemap: s.activeBasemap, onBasemapChange: s.setBasemap,
      kmOverlays: s.kmOverlays, onKmOverlayChange: s.setKmOverlay, kmLinesError: s.kmLinesError,
    }),
  },
  {
    id: 'topology', tools: ['layout'], group: 'view', icon: TopologyIcon, titleKey: 'topology_title', place: 'top', Component: TopologyPanel,
    overlay: 'topologyGraph',
    props: (s) => ({
      selection: s.topologySelection, onSelect: s.setTopologySelection,
      graphOpen: s.overlay?.kind === 'topologyGraph',
      onShowGraph: (on) => (on ? s.openOverlay({ kind: 'topologyGraph' }) : s.closeOverlay('topologyGraph')),
    }),
    // The view is drawn over a pale Liberty, and its diagram comes up with it
    // every time; whatever basemap was up comes back when it is left, and the
    // selection belongs to the view and goes with it.
    onEnter: (s) => {
      if (s.activeBasemap !== 'liberty') {
        s.setBasemapBeforeTopology(s.activeBasemap)
        s.setBasemap('liberty')
      }
      s.openOverlay({ kind: 'topologyGraph' })
    },
    onLeave: (s) => {
      s.setTopologySelection(null)
      if (s.basemapBeforeTopology) {
        s.setBasemap(s.basemapBeforeTopology)
        s.setBasemapBeforeTopology(null)
      }
    },
  },
  { id: 'places', tools: ['layout'], group: 'design', icon: PlaceIcon, titleKey: 'create_element', place: 'top', Component: CreatePanel },
  { id: 'splice', tools: ['layout'], group: 'design', icon: SpliceElementIcon, titleKey: 'splice_element', place: 'top', Component: SpliceConnectPanel },
  {
    id: 'connect_switch', tools: ['layout'], group: 'design', icon: ConnectSwitchIcon, titleKey: 'connect_switch', place: 'top', Component: ConnectSwitchPanel,
    overlay: 'switchDetail',
    props: (s) => ({
      detailTarget: s.overlay?.kind === 'switchDetail' ? s.overlay : null,
      onShowDetail: (target) => (target ? s.openOverlay({ kind: 'switchDetail', ...target }) : s.closeOverlay('switchDetail')),
    }),
  },
  // The civil layer's own (decision 0025): the platforms, and the named runs of tracks.
  { id: 'platforms', tools: ['civil'], group: 'design', icon: NewPlatformIcon, titleKey: 'platform_title', place: 'top', Component: CivilPlatformsPanel },
  {
    id: 'edit', tools: ['layout'], group: 'edit', icon: EditElementIcon, titleKey: 'edit', place: 'top', Component: EditElementPanel,
    overlay: 'trackTable',
    props: (s) => ({
      trackTableId: s.overlay?.kind === 'trackTable' ? s.overlay.track?.id : undefined,
      trackTableRouteId: s.overlay?.kind === 'trackTable' ? s.overlay.routeId : undefined,
      onShowTrackTable: s.showTrackTable,
      onShowRouteTable: s.showRouteTable,
      onCloseConstraints: s.closePopup,
    }),
  },
  {
    id: 'elevation', tools: ['layout'], group: 'heights', icon: ElevationIcon, titleKey: 'tooltip_elevation', place: 'top', Component: ElevationPanel,
    overlay: 'profile',
    props: (s) => ({
      profileTrackId: s.overlay?.kind === 'profile' && !s.overlay.routeId ? s.overlay.trackId : null,
      profileRouteId: s.overlay?.kind === 'profile' ? s.overlay.routeId ?? null : null,
      onShowProfile: (trackId) => (trackId ? s.openOverlay({ kind: 'profile', trackId }) : s.closeOverlay('profile')),
      onShowRouteProfile: (routeId) => (routeId ? s.openOverlay({ kind: 'profile', routeId }) : s.closeOverlay('profile')),
    }),
  },
  {
    id: 'section', tools: ['civil'], group: 'section', icon: CrossSectionIcon, titleKey: 'platform_cross_section', place: 'top', Component: CrossSectionsPanel,
    overlay: 'crossSection',
    props: (s) => ({
      // Over the map or in its own window — a station picked goes to where it is shown.
      crossSectionAt: [s.overlay, s.detached].find(o => o?.kind === 'crossSection')?.at ?? null,
      onShowCrossSection: (at) => (at ? s.openOverlay({ kind: 'crossSection', at }) : s.closeOverlay('crossSection')),
    }),
  },
  {
    id: 'check', group: 'check', icon: CheckIcon, titleKey: 'check_title', place: 'top', Component: CheckPanel,
    overlay: 'bands',
    props: (s) => ({
      onShowRulebook: s.showRulebook,
      bandsTrackId: s.overlay?.kind === 'bands' ? s.overlay.trackId : null,
      onShowBands: (trackId) => (trackId ? s.openOverlay({ kind: 'bands', trackId }) : s.closeOverlay('bands')),
    }),
  },
  {
    id: 'data', group: 'exchange', icon: DataExchangeIcon, titleKey: 'data_exchange', place: 'top', Component: DataExchangePanel,
    props: (s) => ({ onShowCompare: s.setCompare }),
  },
  {
    id: 'plan', group: 'plan', icon: PlanExportIcon, titleKey: 'plan_title', place: 'top', Component: PlanExportPanel,
    overlay: 'planPreview',
    props: (s) => ({
      onShowPlanPreview: (preview) => (preview ? s.openOverlay({ kind: 'planPreview', ...preview }) : s.closeOverlay('planPreview')),
    }),
  },
  {
    id: 'settings', group: 'app', icon: SettingsIcon, titleKey: 'settings', place: 'bottom', Component: SettingsPanel,
    props: (s) => ({ color: s.color, onColorChange: s.setColor }),
  },
  // The steps that can be undone and redone (decision 328), opened by the
  // button between the two arrows.
  { id: 'history', group: 'app', icon: HistoryIcon, titleKey: 'step_history_title', place: 'none', Component: HistoryPanel },
]

/** The panels of this tool, in the sidebar's order. */
export const PANELS = ALL_PANELS.filter(p => !p.tools || p.tools.includes(TOOL))

/** The panel the tool opens with. */
export const FIRST_PANEL = TOOL === 'civil' ? 'platforms' : 'places'

export const panelById = (id) => PANELS.find(p => p.id === id) ?? null
