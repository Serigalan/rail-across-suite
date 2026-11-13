// The types of the infra-data-1.0 document, derived from the schemas in schemas/ by
// packages/core/tools/types.mjs. Do not edit: change the schema and run the tool.

/** infra-data-2.0: a Rail-Across-Suite project as one JSON file (.railacross.json) */
export namespace Document {
  /** The domain documents of one project state, each with its version and the hash of its schema; the schemas the file carries, by hash; and the blobs its objects name, by hash. The same canonical documents as the container (.railacross) holds, giving the same hashes (S-R9). What each domain holds, its own schema says. */
  export type Root = {
    format: "infra-data"
    version: "2.0"
    domains: {
      /** Names matching ^[a-z][a-zA-Z0-9]*$. */
      [name: string]: Domain
    }
    /** The schemas of the domains, keyed by the hash the domains name them by. */
    schemas?: {
      /** Names matching ^sha256:[0-9a-f]{64}$. */
      [name: string]: {
        [name: string]: unknown
      }
    }
    /** Bulk arrays the objects name by hash, each the value whose canonical JSON has that hash. */
    blobs: {
      /** Names matching ^sha256:[0-9a-f]{64}$. */
      [name: string]: unknown
    }
  }
  export type Domain = {
    version: string
    schema: string
    dependsOn?: Array<string>
    fields: {
      [name: string]: unknown
    }
    collections: {
      [name: string]: Array<{
        [name: string]: unknown
      }>
    }
  }
}

/** Rail-Data-Core, domain core 4.0 */
export namespace Core {
  /** The base every module depends on: the net (tracks, switches and connectors), what stands on it for every discipline (platforms, end marks) and the reference data (km lines, the operational point register). Written by the layout tool, carried by the core library. See docs/data-model.md. — 2.0 (decision 0025): the alignment and what every layer places things by; the platforms are civil's, the stopping places came in from signalling, the named runs of tracks from layout. — 3.0 (decision 0028): no construction stages; a lifespan states days. — 3.1 (decision 0028, phase C): every reference names an object of the collection its `x-target` names (the door's rule). — 4.0 (decision 0029, phase 1): the kinds as words, every reference ending in `Id`, no plane of an element's own, a switch with B1 its main track and B2 its branch; (phase 2) every id a UUID in lower case, a km line in one plane as runs from km jump to km jump. */
  export type Root = {
    fields: Fields
    collections: {
      tracks: Array<Track>
      switches: Array<Switch>
      endMarks: Array<EndMark>
      kmLines: Array<KmLine>
      operationalPoints: Array<OperationalPoint>
      stoppingPlaces: Array<StoppingPlace>
      trackPaths?: Array<NamedTrackPath>
    }
  }
  /** The id of an object: a UUID in lower case, unique in its collection. */
  export type Id = string
  /** A reference to an object by its id, a UUID in lower case. Where a property is a Ref, its `x-target` names the collection it points into; the reader checks that the object is there. */
  export type Ref = string
  export type Text = string
  /** A number as people write it: a line or track number, sometimes with letters. */
  export type Label = string | number
  export type Epsg = number
  /** [easting, northing] in m, in the plane of the object that states it. */
  export type Point = Array<number>
  export type Side = "left" | "right"
  export type Endpoint = "BEGIN" | "END"
  export type LineCategory = "main" | "secondary" | "s_bahn"
  /** One end of a track: what a switch port, a connector or an end mark holds. */
  export type TrackEnd = {
    trackId: Ref
    endpoint: Endpoint
  }
  /** Where an object stands, as one value (R5), stating its anchor (R11): `station` follows the track, `plane` keeps its place. Side and offset are seen in the direction of stationing; `direction` up = towards END. */
  export type TrackLocation = {
    anchor: "station"
    trackId: Ref
    station: number
    side?: Side
    offset?: number
    direction?: "up" | "down"
  } | {
    anchor: "plane"
    easting: number
    northing: number
    epsg: Epsg
    bearing?: number
  }
  /** A stretch of one track, as one value (R5): from startStation to endStation, on a side. */
  export type TrackRange = {
    anchor: "station"
    trackId: Ref
    startStation: number
    endStation: number
    side?: Side
  }
  /** A run of tracks in running order; the direction each one is run is derived from where they meet. */
  export type TrackPath = {
    trackIds: Array<Ref>
  }
  /** A day (ISO 8601). */
  export type Day = string
  /** A day, or the word "open" where the day is not known yet. */
  export type LifespanDay = Day | "open"
  /** The days an object is in service (decision 0028): `from` the first, `to` the last, "open" where the day is not known yet — newly built, removed. No `from`: it exists today; no `to`: it stays; no lifespan at all: existing and staying. A `from` that is a day is not after a `to` that is one (the door's rule). */
  export type Lifespan = {
    from?: LifespanDay
    to?: LifespanDay
  }
  /** The project's own fields. */
  export type Fields = {
    id: Id
    title?: Text
    description?: Text
    creator?: Text
    createdAt?: Text
    crsSystem?: Text
    lineCategory?: LineCategory
    gaugeProfile?: Text
  }
  /** One design element of a track: straight, arc or transition. No station, no polyline: stations are the running sum of the lengths, drawn geometry is derived. */
  export type Element = {
    elementType: "straight" | "arc" | "transition"
    startNode: Point
    endNode: Point
    bearing?: number
    endBearing?: number
    length: number
    radius?: number | null
    r1?: number | null
    r2?: number | null
    transitionType?: "clothoid" | "bloss"
    cant?: number
    cantStart?: number
    cantEnd?: number
    cantException?: Text
    role?: "follow" | "intermediate"
    reasons?: {
      [name: string]: string
    }
    /** The design speed of the element [km/h], which the layout tool lays it out for and checks it against (the signalling domain states the speeds it signals, signallingSpeed, on its signals). */
    designSpeed?: number | null
    switchBranch?: boolean
    switchRoute?: "main" | "branch" | "cross" | "slip1" | "slip2"
    switchId?: Ref
    /** Why an import could not build the switch this element belongs to: form, number, reason. */
    switchHint?: Text
  }
  /** A point of a track's vertical tangent polygon: station [m], height of the rail that is not raised [m], vertical curve radius and what governs it, and how it depends on another point. */
  export type HeightPoint = {
    station: number
    z: number
    rv?: number | null
    la?: number | null
    reason?: Text
    locked?: boolean
    id?: Id
    /** The id of the height point of the same track this one depends on. */
    depId?: Ref
    depMode?: "lds" | "continueBefore" | "continueAfter" | "wa" | "continue"
    free?: boolean
  }
  export type Superstructure = {
    type: Text
    from: number
    to: number
  }
  export type Track = {
    id: Id
    name?: Text
    trackType?: "line" | "station"
    lineNumber?: Label
    lineName?: Text
    side?: "regular" | "opposite"
    trackNumber?: Label
    operationalPointId?: Ref
    owner?: Text
    trackUse?: "siding"
    lineCategory?: LineCategory
    lifespan?: Lifespan
    epsg: Epsg
    heightEpsg?: Epsg
    elements: Array<Element>
    heights?: Array<HeightPoint>
    rails?: Array<Superstructure>
    sleepers?: Array<Superstructure>
    /** The DB Lagesystem an MDB import took the track from (EA0, DB0, …). */
    lagesystem?: Text
    /** The ProVI axis an import took the track from. */
    proviAxis?: Text
    /** What a RailJSON import brought along for the track section that the model does not hold, handed back on export. */
    osrd?: {
      [name: string]: unknown
    }
  }
  /** A switch, crossing, slip, two-end connector (`link`) or construction stage connector (`stage_link`) at a node; its ports hold track ends. A switch: A its toe, B1 the main track, B2 the branch. A link joins two track ends where the plane changes or where a rebuilt part of a track ties in; a stage link joins the track at A to the one at B1 while that exists and to the one at B2 from then on — never both: one track ends, the other begins (decision 0027). */
  export type Switch = {
    id: Id
    kind: "switch" | "crossing" | "single_slip" | "double_slip" | "link" | "stage_link"
    formVersion: number
    name?: Text
    number?: number
    label?: Text
    speed?: number
    trailing?: boolean
    swapped?: boolean
    toeLead?: "branch"
    lifespan?: Lifespan
    mainRadius?: number | null
    mainRadiusEnd?: number | null
    pad?: unknown
    ports: {
      /** Names matching ^(A|B|B1|B2|C|D)$. */
      [name: string]: TrackEnd | null
    }
  }
  /** A buffer stop or the boundary of the planning area at a free track end; a train may begin and end at either. */
  export type EndMark = {
    id: Id
    kind: "buffer_stop" | "boundary"
    at: TrackEnd
    /** What a timetable names the place by. */
    name?: Text
    type?: number | string
    brakeLength?: number
    /** At a boundary: the speed [km/h] a train crosses it at in a simulation, entering or leaving; without one, what the line allows there. */
    speed?: number
    lifespan?: Lifespan
    /** At a boundary: the least time [s] between two trains crossing it the same way — how close trains follow from the net beyond. */
    headway?: number
  }
  /** Bulk data kept as a blob: the SHA-256 of its canonical JSON. */
  export type BlobRef = string
  /** A design element of a km line: what a track element states of its geometry, without the parts that only a track has. */
  export type KmElement = {
    elementType: "straight" | "arc" | "transition"
    startNode: Point
    endNode: Point
    bearing?: number
    endBearing?: number
    length: number
    radius?: number | null
    r1?: number | null
    r2?: number | null
    transitionType?: "clothoid" | "bloss"
  }
  /** A run of a km line without a km jump: its kilometrage at the start and the way it counts along the run, and the run itself — design elements, or the points a source gave where it gave no elements. */
  export type KmRun =  | 
  /** Foreign reference data: the axis of a line around the project, in one plane, as runs from km jump to km jump. */
  export type KmLine = {
    id: Id
    lineNumber: Label
    epsg: Epsg
    runs: Array<KmRun>
    source?: Text
  }
  /** A station, halt or other operational point of the register: its identity. Codes are attributes, stated once they exist; boundaries belong to signalling (R10). */
  export type OperationalPoint = {
    id: Id
    name?: Text
    rl100?: Text
    uic?: Label
  }
  /** Where trains stop (Halteplatz): a stretch of one track, in a station on one of its tracks or at a platform, on the open line at a platform — for every train. The Signalling-Tool sets it (decision 0024); the Civil-Tool makes or details one where it puts a platform: a passenger stop (passengerStop), named by the platform (civil). Signals and detection points bound it; where one stands within it, a finding says so. */
  export type StoppingPlace = {
    id: Id
    name?: Text
    /** The station it is of; without one it lies on the open line. */
    operationalPointId?: Ref
    at: TrackRange
    /** A passenger stop (Personenhalt): passengers board and alight here, at a platform. The Civil-Tool sets it where it puts a platform at the stopping place, which then names it (civil); without it nothing says that passengers board there. */
    passengerStop?: boolean
    lifespan?: Lifespan
  }
  /** A named run of tracks (a `route` in the Open-Layout-Tool; the bare word is reserved for the signalling route): the Layout-Tool's, for its profiles and tables. */
  export type NamedTrackPath = {
    id: Id
    name?: Text
    path: TrackPath
  }
}

/** Rail-Data-Core, domain civil 3.0 */
export namespace Civil {
  /** Civil engineering's layer, edited in the Civil-Tool (decision 0025): platforms, bridges, retaining walls and noise barriers, measured axes traced in point clouds, reference axes from surveys, the plan header and the project picture — what layout 1.0 held but its named runs of tracks, and the platforms from core 1.0. Depends on core; its references go into the core only. Bulk point arrays are blobs, named by their hash (S-R2). — 2.0 (decision 0028): on core 3.0, a lifespan states days. — 2.1 (decision 0028, phase C): every reference names an object of the collection its `x-target` names (the door's rule). — 3.0 (decision 0029, phase 1): a platform names its operational point by `operationalPointId`. */
  export type Root = {
    fields: Fields
    collections: {
      platforms: Array<Platform>
      axisSurveys: Array<AxisSurvey>
      referenceAxes: Array<ReferenceAxis>
      bridges?: Array<Bridge>
      retainingWalls?: Array<RetainingWall>
      noiseBarriers?: Array<NoiseBarrier>
    }
  }
  /** A bulk array kept as a blob of its own: the SHA-256 of its canonical JSON. */
  export type BlobRef = string
  export type Fields = {
    /** The editable title block of the plan export. */
    planHeader?: {
      [name: string]: unknown
    }
    /** The project picture as a data URL, where a file or a device keeps it. */
    image?: string
    /** The project picture as a blob on the server, by its SHA-256. */
    imageHash?: string
    /** RailJSON objects the layout tool does not model, handed back on export. */
    osrd?: {
      [name: string]: unknown
    }
  }
  /** A measured track axis traced in point clouds, in a plane of its own; its points are whole millimetres in parallel arrays, a blob. */
  export type AxisSurvey = {
    id: Core.Id
    name?: Core.Text
    epsg: Core.Epsg
    rail?: string
    guide?: {
      kind: "track" | "line"
      trackId?: Core.Ref
    }
    cloudNames?: Array<string>
    cloudRefs?: Array<unknown>
    createdAt?: Core.Text
    step?: number
    gaps?: Array<{
      from: number
      to: number
    }>
    points: BlobRef
  }
  /** The axis of a surveyed track as points every centimetre (Bestandsachse); its points are a blob. */
  export type ReferenceAxis = {
    id: Core.Id
    name?: Core.Text
    epsg: Core.Epsg
    heightEpsg?: Core.Epsg | null
    source?: {
      [name: string]: unknown
    }
    points: BlobRef
  }
  /** It names the stopping places at it (stoppingPlaceIds). */
  export type Platform = {
    id: Core.Id
    at: Core.TrackRange
    frontOffset?: number
    backOffset?: number
    height?: number
    operationalPointId?: Core.Ref
    lifespan?: Core.Lifespan
    /** The stopping places at the platform: the core names nothing above it, so the platform names them. */
    stoppingPlaceIds?: Array<Core.Ref>
  }
  /** A railway bridge (Eisenbahnüberführung) the track crosses over on: from where it begins to where it ends along the track. */
  export type Bridge = {
    id: Core.Id
    name?: Core.Text
    at: Core.TrackRange
    /** Its structure: beam, arch, frame, truss. */
    kind?: "beam" | "arch" | "frame" | "truss" | "other"
    /** How many spans. */
    spans?: number
    crosses?: Core.Text
    /** The clear height under it [m]. */
    clearance?: number
    lifespan?: Core.Lifespan
  }
  /** A retaining wall (Stützmauer) beside the track: from where it begins to where it ends along it, on its side. */
  export type RetainingWall = {
    id: Core.Id
    name?: Core.Text
    at: Core.TrackRange
    /** Its build: cantilever (Winkelstützmauer), gravity (Schwergewichtsmauer), sheet pile (Spundwand), gabion. */
    kind?: "cantilever" | "gravity" | "sheetPile" | "gabion" | "other"
    /** How far from the track's axis [m]. */
    offset?: number
    /** How high over the ground in front of it [m]. */
    height?: number
    lifespan?: Core.Lifespan
  }
  /** A noise barrier (Lärmschutzwand) beside the track: from where it begins to where it ends along it, on its side. */
  export type NoiseBarrier = {
    id: Core.Id
    name?: Core.Text
    at: Core.TrackRange
    /** Its face: highly absorbing, reflecting, transparent. */
    kind?: "absorbing" | "reflecting" | "transparent"
    /** How far from the track's axis [m]. */
    offset?: number
    /** How high over top of rail [m]. */
    height?: number
    lifespan?: Core.Lifespan
  }
}

/** Rail-Data-Core, domain signalling 4.0 */
export namespace Signalling {
  /** The signalling of a project, as the signalling tool plans it: signals and boards, train detection, the switch elements of the interlocking, train and shunting routes with their overlaps, flank protection, the balises and magnets of train protection, the station boundaries and the itineraries from one to another. Depends on core; its references go into the core or into itself (R2, R9). Its objects follow PlanPro 1.10, and like OSRD it keeps every signalling and train protection system out of the schema (decision 0011): a signal carries a logical signal per system, a balise names its system; which systems, kinds, aspects and settings there are is not in this schema but in the catalogue (packages/core/constraints/db-ril-301.json), which the content check reads. — 2.0 (decision 0025): the stopping places are the core's; routes carry their setting and release times; train protection by ranges. — 3.0 (decision 0028): on core 3.0, a lifespan states days. — 3.1 (decision 0028, phase C): every reference names an object of the collection its `x-target` names (the door's rule). — 4.0 (decision 0029, phase 1): every reference ends in `Id` (`detectionPointIds`, `clearSectionIds`, `passOn.leftId`/`rightId`). */
  export type Root = {
    fields: Fields
    collections: {
      signals: Array<Signal>
      detectionPoints: Array<DetectionPoint>
      trackSections: Array<TrackSection>
      switchElements: Array<SwitchElement>
      routes: Array<Route>
      overlaps: Array<Overlap>
      flankProtection: Array<FlankProtection>
      balises: Array<Balise>
      stationBoundaries: Array<StationBoundary>
      itineraries: Array<Itinerary>
      trainProtection?: Array<TrainProtection>
    }
  }
  /** The domain's own fields: the catalogue of signal aspects its signals were planned against. `routeTimes`: the interlocking's setting and release times [s], for every route that states none. */
  export type Fields = {
    catalogue?: {
      id: string
      version: string
    }
    routeTimes?: {
      setting?: number
      release?: number
    }
  }
  /** A location that states the direction it acts in: `direction` on a track, `bearing` in the plane (an orphan, R7). */
  export type Directed = 
  /** Where a switch element lies, seen from its tip, as the interlocking names it (PlanPro: L, R). */
  export type SwitchPosition = "left" | "right"
  /** A signal's place at the track, the physical signal (PlanPro: Signal; OSRD: Signal), real or virtual. A real one stands at `at` and acts in its direction — or in both (`bothDirections`), on its mounting — a gantry or a bracket on its `foundation` beside the track —, seen from its sighting distances; what it is in each signalling system it shows are its logical signals, each with the frames it shows; the boards it carries of no system (Ne 1, Ra 12, mast plates, its name plate) are its own frames. A virtual one (PlanPro: Signal_Fiktiv) has neither: it marks where a route starts or ends without a signal there. */
  export type Signal = {
    id: Core.Id
    name?: Core.Label
    /** The number of the interlocking area it belongs to, which leads its full designation (PlanPro Kennzahl: 60 in 60A). */
    areaNumber?: Core.Label
    /** The station it belongs to; without one it is a signal of the open line. */
    operationalPointId?: Core.Ref
    kind: "real" | "virtual"
    at: Directed
    /** Further tracks it stands at, at the same place: a clearance marker (Ra 12) between the two tracks beyond a switch, which PlanPro places on both edges. */
    alsoAt?: Array<Core.TrackLocation>
    bothDirections?: boolean
    /** PlanPro Signal_Funktion, for a signal of no system (a level crossing's monitoring signal, a standalone indicator); one with logical signals states it on each. */
    function?: "entry" | "exit" | "entryExit" | "intermediate" | "exitIntermediate" | "entryBlock" | "block" | "groupExit" | "groupIntermediate" | "groupExitIntermediate" | "followUp" | "cover" | "trainCover" | "trainTarget" | "levelCrossing" | "distant" | "standaloneIndicator" | "other"
    /** PlanPro Fiktives_Signal_Funktion, for a virtual signal. */
    virtualFunction?: "trainStartWithoutSignal" | "trainTargetWithoutSignal" | "trainTargetLine" | "trainStartSiding" | "trainTargetSiding" | "trainStartMiddleSwitch" | "trainStartTargetBlock" | "trainStartTargetBlockZs1" | "trainStartTargetBlockZs7" | "shuntingStartTargetWithoutSignal" | "virtualDistant" | "other"
    /** What it is in each signalling system it shows (OSRD: logical signals; PlanPro Signalsystem and Signal_Art of its screen): a Ks main signal, a shunting signal … A real signal with a screen has at least one. */
    logicalSignals?: Array<LogicalSignal>
    /** PlanPro Signal_Befestigungsart. */
    mounting?: "mast" | "bracket" | "gantry" | "foundation" | "special" | "other"
    /** Where the foundation of a signal on a gantry or a bracket stands, at the signal's station: its offset from the axis of the track the signal stands at and its side, seen in the direction the signal acts — so a track turned round, which turns the signal's direction, leaves it as it is (PlanPro Signal_Befestigung, its Fundament). The signal's own side and offset are its screen's, which the gantry or the bracket carries out over the track. */
    foundation?: {
      side?: Core.Side
      offset: number
    }
    /** The distances from which the signal must, at least, and can be seen [m] (PlanPro Signalsicht_Soll, _Mindest, _Erreichbar). */
    sight?: {
      required?: number
      minimum?: number
      achievable?: number
    }
    /** What it shows of no system: boards, the ceramic body of a clearance marker, its name plate — and the screen of a signal of no system. A logical signal's screen and indicators are the logical signal's. */
    frames?: Array<Frame>
    lifespan?: Core.Lifespan
  }
  /** A signal in one signalling system at a place (OSRD: LogicalSignal): the system, a key of the catalogue's `systeme`; the kind of signal it is there, a key of that system's `signalarten` or `other`; its function (PlanPro Signal_Funktion); the frames it shows — its screen, its indicators; the speed it signals for each route that starts at its place (signallingSpeed; PlanPro Fstr_V_Hg), where the design speed of the track (core `designSpeed`) is the layout's; its settings, as the catalogue declares them for the system; the systems the signals it announces may be of (OSRD next_signaling_systems); and the parameters it shows with, by default and per route (OSRD default_parameters, conditional_parameters), as the system declares them. */
  export type LogicalSignal = {
    system: string
    type?: string
    /** PlanPro Signal_Funktion. */
    function?: "entry" | "exit" | "entryExit" | "intermediate" | "exitIntermediate" | "entryBlock" | "block" | "groupExit" | "groupIntermediate" | "groupExitIntermediate" | "followUp" | "cover" | "trainCover" | "trainTarget" | "levelCrossing" | "distant" | "standaloneIndicator" | "other"
    frames?: Array<Frame>
    /** Per route that starts at its place, the speed it signals for it [km/h] (signallingSpeed; PlanPro Fstr_V_Hg). */
    signallingSpeeds?: Array<{
      routeId: Core.Ref
      speed: number
    }>
    settings?: Settings
    nextSystems?: Array<string>
    defaultParameters?: Settings
    conditionalParameters?: Array<{
      routeId: Core.Ref
      parameters: Settings
    }>
  }
  /** Values a system's driver reads (OSRD: settings), by the names the catalogue declares for that system and kind: a choice, a number, a flag or a text. The content check holds them to the catalogue. */
  export type Settings = {
    [name: string]: string | number | boolean
  }
  /** A part of a signal that shows aspects (PlanPro Signal_Rahmen): its screen with every light it shows, an indicator, a plate, the ceramic body of a clearance marker, the name plate. Only a name plate shows no aspect. */
  export type Frame = {
    /** PlanPro Rahmen_Art: Schirm, Zusatzanzeiger, Blechtafel, Keramikkoerper, Bezeichnungsschild, sonstige. */
    kind: "screen" | "indicator" | "plate" | "body" | "nameplate" | "other"
    aspects: Array<Aspect>
  }
  /** One aspect a frame can show (PlanPro Signal_Signalbegriff): its key in the catalogue, and where the catalogue asks for one, its symbol — the speed a Zs 3 shows in tens of km/h, the letter of a Zs 2. */
  export type Aspect = {
    aspect: string
    symbol?: string
    lit?: boolean
    switched?: boolean
  }
  /** A point that bounds track sections (PlanPro FMA_Komponente): an axle counting point or a joint. */
  export type DetectionPoint = {
    id: Core.Id
    name?: Core.Label
    kind: "axleCounter" | "insulatedJointOneRail" | "insulatedJointTwoRail" | "electricalJoint"
    at: Core.TrackLocation
    lifespan?: Core.Lifespan
  }
  /** A track detection section (PlanPro FMA_Anlage): its name and kind, and the detection points it is bounded by. Its extent is not stored: it is the part of the net between those points (and track ends), derived; where the derived and the stored boundaries differ, that is a finding. */
  export type TrackSection = {
    id: Core.Id
    name?: Core.Label
    /** The number of the interlocking area it belongs to, which leads its full designation (PlanPro Kennzahl: 60 in 60A). */
    areaNumber?: Core.Label
    kind: "axleCounting" | "audioFrequencyCircuit" | "lowFrequencyCircuit" | "ftgs" | "other"
    detectionPointIds: Array<Core.Ref>
    /** PlanPro FMA_Hilffreimeldung. */
    auxiliaryClearing?: boolean
    lifespan?: Core.Lifespan
  }
  /** A switch of the core or a derailer as the interlocking knows it (PlanPro W_Kr_Gsp_Element): its name, how it is worked, the position it rests in. A slip has two, one per pair of points. A derailer is no switch of the core: it stands `at` a place of its own and is applied or removed. */
  export type SwitchElement =  | 
  /** A train, shunting or middle-switch route (PlanPro Fstr_Zug_Rangier with its Fstr_Fahrweg): from a start signal to a target signal along a run of tracks. Which way each switch element lies follows from the run and is not stored (as in PlanPro). A train route names the overlaps it may end in; flank protection follows from the switch elements it runs over. Where several routes run between the same two signals, `rank` orders them. The speed it permits is not its own: its start signal signals it (a logical signal's signallingSpeeds). */
  export type Route = {
    id: Core.Id
    name?: Core.Text
    kind: "train" | "shunting" | "middle"
    /** PlanPro Fstr_Zug_Art, for a train route. */
    trainType?: "B" | "ZH" | "ZR" | "ZT" | "ZTU" | "ZU" | "ZUH" | "ZZ"
    /** PlanPro Fstr_Rangier_Art, for a shunting route. */
    shuntingType?: "RR" | "RT" | "RTU" | "RU"
    /** PlanPro Fstr_Mittel_Art, for a route from a middle switch. */
    middleType?: "ZM" | "ZUM"
    /** Its place among the routes between the same two signals, 0 the regular one (PlanPro Fstr_Reihenfolge). */
    rank?: number
    startSignalId: Core.Ref
    targetSignalId: Core.Ref
    path: Core.TrackPath
    /** Set by the automatic route setting (PlanPro Automatische_Einstellung). */
    autoSetting?: boolean
    /** Its distant signal stands at a shortened braking distance (PlanPro Fstr_Vsigabstand_Verkuerzt). */
    shortDistantSpacing?: boolean
    /** For a shunting route, the sections that must be clear (PlanPro ID_FMA_Anlage_Rangier_Frei); a train route's follow from its run. */
    clearSectionIds?: Array<Core.Ref>
    overlaps?: Array<{
      overlapId: Core.Ref
      preferred?: boolean
    }>
    /** What the route shows (PlanPro Fstr_Signalisierung): at which signal which aspect, and its symbol. */
    aspects?: Array<{
      signalId: Core.Ref
      aspect: string
      symbol?: string
    }>
    lifespan?: Core.Lifespan
    /** How long setting the route takes [s] (Fahrstraßenbildezeit); without one, the domain's. */
    settingTime?: number
    /** How long releasing it takes once the train has left it [s] (Auflösezeit); without one, the domain's. */
    releaseTime?: number
  }
  /** An overlap beyond a route's target signal (PlanPro Fstr_DWeg): from the signal along a run of tracks to its danger point, its design length and the gradient that sets it, the speed it allows, the sections that must be clear, and for each switch element on it whether it is locked. */
  export type Overlap = {
    id: Core.Id
    name?: Core.Text
    signalId: Core.Ref
    path: Core.TrackPath
    /** Its danger point (PlanPro ID_PZB_Gefahrpunkt): on its last track, or the clearance marker of a switch at its end, on a track beyond it. */
    end: Core.TrackLocation
    /** Its design length [m] (PlanPro Laenge_Soll). */
    length: number
    /** The gradient that sets its length [‰], negative downhill (PlanPro Massgebende_Neigung). */
    gradient?: number
    /** The speed it allows [km/h] (PlanPro DWeg_V). */
    speed?: number
    /** How long it stays set after the train stopped [s] (PlanPro Aufloesung_Verzoegerung). */
    releaseDelay?: number
    clearSectionIds?: Array<Core.Ref>
    switches?: Array<{
      switchElementId: Core.Ref
      locked: boolean
    }>
    lifespan?: Core.Lifespan
  }
  /** The flank protection a switch element asks for when run over in one position (PlanPro Fla_Schutz): given by a signal kept at stop or by a switch element held in a position, passed on to the next where the protecting one asks for its own, or waived; with the sections that must be clear for it. */
  export type FlankProtection = {
    id: Core.Id
    requester: {
      switchElementId: Core.Ref
      /** Run over in which position, or from its tip (PlanPro Fahrt_Ueber). */
      via: "left" | "right" | "tip"
    }
    protection: {
      signalId: Core.Ref
      /** PlanPro Fla_Signal_Zielsperrung. */
      targetLock?: boolean
    } | {
      switchElementId: Core.Ref
      /** The position a switch is held in; a derailer gives it applied, and states none. */
      position?: SwitchPosition
    } | {
      waived: true
    }
    /** The flank protection the protecting switch element asks for in turn, per its position (PlanPro Fla_Schutz_Weitergabe). */
    passOn?: {
      leftId?: Core.Ref
      rightId?: Core.Ref
    }
    clearSectionIds?: Array<Core.Ref>
    lifespan?: Core.Lifespan
  }
  /** A track element of a train protection system that acts at a point — a PZB track magnet or speed check, an ETCS Eurobalise (PlanPro PZB_Element, Balise): its system and kind, keys of the catalogue's `zugbeeinflussung`, with the settings the catalogue declares for them (a PZB magnet's frequency, an ETCS balise's place in its group); where it acts, the signal it belongs to, and whether it acts always, with its signal or with routes. */
  export type Balise = {
    id: Core.Id
    name?: Core.Label
    at: Directed
    system: string
    kind: string
    settings?: Settings
    /** The signal it belongs to (PlanPro PZB_Element_Bezugspunkt). */
    signalId?: Core.Ref
    /** PlanPro Wirksamkeit. */
    effect: "permanent" | "switchedBySignal" | "other"
    /** Routes that switch it (PlanPro PZB_Element_Zuordnung_Fstr). */
    routes?: Array<{
      routeId: Core.Ref
      effect: "activeWhenSet" | "inactiveWhenSet" | "other"
    }>
    lifespan?: Core.Lifespan
  }
  /** The boundary of a station of the core's register, where trains enter it: what marks it — an entry signal or a board (Ne 1), an entry switch where there is neither (EBO §4), or for a halt its platform. Where it is not where the rules put it, a finding says so. */
  export type StationBoundary = {
    id: Core.Id
    operationalPointId: Core.Ref
    marks: Array<{
      signalId: Core.Ref
    } | {
      switchId: Core.Ref
    } | {
      /** A halt's boundary: the stopping place at its platform. */
      stoppingPlaceId: Core.Ref
    }>
    lifespan?: Core.Lifespan
  }
  /** A way from one stopping place to another (Fahrweg): its run of tracks, and the train routes it takes, in running order — one route of each station it runs through from signal to signal, none on the open line. A timetable names it as the way a train prefers between two of its points. */
  export type Itinerary = {
    id: Core.Id
    name?: Core.Text
    startStoppingPlaceId: Core.Ref
    targetStoppingPlaceId: Core.Ref
    routeIds: Array<Core.Ref>
    path: Core.TrackPath
    lifespan?: Core.Lifespan
  }
  /** The train protection over a range of a track: its system and, for ETCS, its level. */
  export type TrainProtection = {
    id: Core.Id
    name?: Core.Text
    at: Core.TrackRange
    system: "pzb" | "lzb" | "etcs"
    /** The ETCS level. */
    level?: "1" | "2" | "3"
    lifespan?: Core.Lifespan
  }
}

/** Rail-Data-Core, domain electrification 2.1 */
export namespace Electrification {
  /** The electrification of the tracks, planned in the Catenary-Tool (decision 0025): which ranges carry a contact wire, and its system. Depends on core; its references go into the core only. Later the masts, cantilevers, wires and sections. — 2.0 (decision 0028): on core 3.0, a lifespan states days. — 2.1 (decision 0028, phase C): every reference names an object of the collection its `x-target` names (the door's rule). */
  export type Root = {
    fields: Fields
    collections: {
      electrifications: Array<Electrification>
    }
  }
  /** The domain's own fields. */
  export type Fields = Record<string, never>
  /** A range of a track under a contact wire (or a conductor rail) of one system. `system` is the voltage as OSRD names a traction mode: 15000V the 15 kV 16.7 Hz of DB, ÖBB and SBB, 25000V 25 kV 50 Hz, the others direct current. A train runs electrically where its vehicle has the mode. */
  export type Electrification = {
    id: Core.Id
    name?: Core.Text
    at: Core.TrackRange
    system: "15000V" | "25000V" | "3000V" | "1500V" | "750V" | "600V"
    lifespan?: Core.Lifespan
  }
}
