// The types of the core 1.0 document, derived from the schemas in schemas/ by
// packages/core/tools/types.mjs. Do not edit: change the schema and run the tool.

/** A Rail-Across-Suite project as one JSON file (.railacross.json) */
export namespace Document {
  /** The domain documents of one project state, each with its version and the hash of its schema; the schemas the file carries, by hash; and the blobs its objects name, by hash. The same canonical documents as the container (.railacross) holds, giving the same hashes (S-R9). What each domain holds, its own schema says. */
  export type Root = {
    format: "rail-across"
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

/** Rail-Data-Core, domain core 1.0 */
export namespace Core {
  /** The base every module depends on: the net (tracks, switches and connectors), what stands on it for every discipline (platforms, end marks), the reference data (km lines, the operational point register) and the construction stages. Written by the layout tool, carried by the core library. See docs/data-model.md. */
  export type Root = {
    fields: Fields
    collections: {
      tracks: Array<Track>
      switches: Array<Switch>
      platforms: Array<Platform>
      endMarks: Array<EndMark>
      kmLines: Array<KmLine>
      operationalPoints: Array<OperationalPoint>
    }
  }
  /** The id of an object, unique in its collection; a UUID in lower case by convention. */
  export type Id = string
  /** A reference to an object by its bare id. Where a property is a Ref, its `x-target` names the collection it points into. */
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
  /** The first construction stage an object exists in and the first it no longer exists in; an absent end is open, no lifespan at all means existing and staying (decision 0003). */
  export type Lifespan = {
    from?: Ref
    to?: Ref
  }
  export type Stage = {
    id: Id
    name?: Text
  }
  /** The project's own fields. `stages` is the ordered list of construction stages: the first is the existing state, the last the final state. */
  export type Fields = {
    id: Id
    title?: Text
    description?: Text
    creator?: Text
    createdAt?: Text
    crsSystem?: Text
    lineCategory?: LineCategory
    gaugeProfile?: Text
    stages: Array<Stage>
  }
  /** One design element of a track: straight (0), arc (1) or transition (2). No station, no polyline: stations are the running sum of the lengths, drawn geometry is derived. */
  export type Element = {
    elementType: 0 | 1 | 2
    /** The plane the element is stated in, where it says so: its track's (a finding of the chain checks where not). */
    epsg?: Epsg
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
    speed?: number | null
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
    dep?: Ref
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
    trackType?: 1 | 2
    lineNumber?: Label
    lineName?: Text
    side?: number
    trackNumber?: Label
    operationalPoint?: Ref
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
  /** A turnout, crossing, slip or two-end connector (`link`) at a node; its ports hold track ends. A link joins two track ends where the plane changes or where a rebuilt part of a track ties in. */
  export type Switch = {
    id: Id
    kind: "turnout" | "crossing" | "single_slip" | "double_slip" | "link"
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
  export type Platform = {
    id: Id
    at: TrackRange
    frontOffset?: number
    backOffset?: number
    height?: number
    operationalPoint?: Ref
    lifespan?: Lifespan
  }
  /** A buffer stop or the boundary of the planning area at a free track end. */
  export type EndMark = {
    id: Id
    kind: "buffer_stop" | "boundary"
    at: TrackEnd
    type?: number | string
    brakeLength?: number
    lifespan?: Lifespan
  }
  /** Foreign reference data: the stretch of a DB line around the project, in WGS84, the third value of each point its kilometrage [m]. */
  export type KmLine = {
    id: Id
    lineNumber: Label
    crs?: "EPSG:4326"
    clip?: Array<number>
    runs: Array<Array<Array<number>>>
    /** The import a line was taken from (mdb, vermesn), kept instead of the shipped one. */
    source?: Text
  }
  /** A station, halt or other operational point of the register: its identity. Codes are attributes, stated once they exist; boundaries belong to signalling (R10). */
  export type OperationalPoint = {
    id: Id
    name?: Text
    rl100?: Text
    uic?: Label
  }
}

/** Rail-Data-Core, domain layout 1.0 */
export namespace Layout {
  /** What only the layout tool needs: named runs of tracks, measured axes traced in point clouds, reference axes from surveys, the plan header and the project picture. Depends on core; its references go into the core only. Bulk point arrays are blobs, named by their hash (S-R2). */
  export type Root = {
    fields: Fields
    collections: {
      trackPaths: Array<NamedTrackPath>
      axisSurveys: Array<AxisSurvey>
      referenceAxes: Array<ReferenceAxis>
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
  /** A named run of tracks (a `route` in the Open-Layout-Tool; the bare word is reserved for the signalling route). */
  export type NamedTrackPath = {
    id: Core.Id
    name?: Core.Text
    path: Core.TrackPath
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
}

/** Rail-Data-Core, domain signalling 1.0 */
export namespace Signalling {
  /** The signalling of a project, as the signalling tool plans it: signals and boards, train detection, the switch elements of the interlocking, train and shunting routes with their overlaps, flank protection, the PZB track elements and the station boundaries. Depends on core; its references go into the core or into itself (R2, R9). Its structure follows PlanPro 1.10 (decision 0011); what a signal can show is not in this schema but in the catalogue of signal aspects (packages/core/constraints/db-ril-301.json), which the content check reads. */
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
      pzbElements: Array<PzbElement>
      stationBoundaries: Array<StationBoundary>
    }
  }
  /** The domain's own fields: the catalogue of signal aspects its signals were planned against. */
  export type Fields = {
    catalogue?: {
      id: string
      version: string
    }
  }
  /** A location that states the direction it acts in: `direction` on a track, `bearing` in the plane (an orphan, R7). */
  export type Directed = 
  /** Where a switch element lies, seen from its tip, as the interlocking names it (PlanPro: L, R). */
  export type SwitchPosition = "left" | "right"
  /** A signal or a board at the track (PlanPro: Signal), real or virtual. A real one stands at `at` and acts in its direction — or in both (`bothDirections`); what it can show are its frames, each with its aspects, keyed by the catalogue. A virtual one (PlanPro: Signal_Fiktiv) has no frames: it marks where a route starts or ends without a signal there. */
  export type Signal = {
    id: Core.Id
    name?: Core.Label
    kind: "real" | "virtual"
    at: Directed
    bothDirections?: boolean
    /** PlanPro Signal_Funktion. */
    function?: "entry" | "exit" | "entryExit" | "intermediate" | "exitIntermediate" | "entryBlock" | "block" | "groupExit" | "groupIntermediate" | "groupExitIntermediate" | "followUp" | "cover" | "trainCover" | "trainTarget" | "levelCrossing" | "distant" | "standaloneIndicator" | "other"
    /** PlanPro Fiktives_Signal_Funktion, for a virtual signal. */
    virtualFunction?: "trainStartWithoutSignal" | "trainTargetWithoutSignal" | "trainTargetLine" | "trainStartSiding" | "trainTargetSiding" | "trainStartMiddleSwitch" | "trainStartTargetBlock" | "trainStartTargetBlockZs1" | "trainStartTargetBlockZs7" | "shuntingStartTargetWithoutSignal" | "virtualDistant" | "other"
    /** PlanPro Signal_Art, for a real signal with a screen. */
    type?: "main" | "mainShunting" | "multiSection" | "multiSectionShunting" | "shunting" | "distant" | "distantRepeater" | "trainCover" | "other"
    /** The signalling system of its screen, a key of the catalogue (PlanPro Signalsystem: `ks`). */
    system?: string
    /** PlanPro Signal_Befestigungsart. */
    mounting?: "mast" | "bracket" | "gantry" | "foundation" | "special" | "other"
    /** The distances from which the signal must, at least, and can be seen [m] (PlanPro Signalsicht_Soll, _Mindest, _Erreichbar). */
    sight?: {
      required?: number
      minimum?: number
      achievable?: number
    }
    frames?: Array<Frame>
    lifespan?: Core.Lifespan
  }
  /** A part of a signal that shows aspects (PlanPro Signal_Rahmen): its screen, an indicator, a plate. */
  export type Frame = {
    kind: "screen" | "indicator" | "plate" | "other"
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
    kind: "axleCounting" | "audioFrequencyCircuit" | "lowFrequencyCircuit" | "ftgs" | "other"
    boundedBy: Array<Core.Ref>
    /** PlanPro FMA_Hilffreimeldung. */
    auxiliaryClearing?: boolean
    lifespan?: Core.Lifespan
  }
  /** A switch of the core as the interlocking knows it (PlanPro W_Kr_Gsp_Element): its name, how it is worked, the position it rests in. A slip has two, one per pair of points. */
  export type SwitchElement = {
    id: Core.Id
    name?: Core.Label
    switchId: Core.Ref
    /** Which pair of points of a slip. */
    part?: "a" | "b"
    /** PlanPro W_Kr_Gsp_Stellart. */
    operation: "electricRemote" | "electricLocal" | "mechanicalRemote" | "mechanicalLocal" | "notWorked" | "springBack" | "disabledLeft" | "disabledRight" | "other"
    preferredPosition?: SwitchPosition
    /** PlanPro Auffahrortung. */
    trailable?: boolean
    lifespan?: Core.Lifespan
  }
  /** A train or shunting route (PlanPro Fstr_Zug_Rangier with its Fstr_Fahrweg): from a start signal to a target signal along a run of tracks. Which way each switch element lies follows from the run and is not stored (as in PlanPro). A train route names the overlaps it may end in; flank protection follows from the switch elements it runs over. */
  export type Route = {
    id: Core.Id
    name?: Core.Text
    kind: "train" | "shunting"
    /** PlanPro Fstr_Zug_Art, for a train route. */
    trainType?: "B" | "ZH" | "ZR" | "ZT" | "ZTU" | "ZU" | "ZUH" | "ZZ"
    /** PlanPro Fstr_Rangier_Art, for a shunting route. */
    shuntingType?: "RR" | "RT" | "RTU" | "RU"
    startSignalId: Core.Ref
    targetSignalId: Core.Ref
    path: Core.TrackPath
    /** The speed the route permits [km/h] (PlanPro Fstr_V). */
    speed?: number
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
  }
  /** An overlap beyond a route's target signal (PlanPro Fstr_DWeg): from the signal along a run of tracks to its end, its design length and the gradient that sets it, the speed it allows, the sections that must be clear, and for each switch element on it whether it is locked. */
  export type Overlap = {
    id: Core.Id
    name?: Core.Text
    signalId: Core.Ref
    path: Core.TrackPath
    end: Core.TrackLocation
    /** Its design length [m] (PlanPro Laenge_Soll). */
    length: number
    /** The gradient that sets its length [‰], negative downhill (PlanPro Massgebende_Neigung). */
    gradient?: number
    /** The speed it allows [km/h] (PlanPro DWeg_V). */
    speed?: number
    /** How long it stays set after the train stopped [s] (PlanPro Aufloesung_Verzoegerung). */
    releaseDelay?: number
    clearSections?: Array<Core.Ref>
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
      via: SwitchPosition
    }
    protection: {
      signalId: Core.Ref
      /** PlanPro Fla_Signal_Zielsperrung. */
      targetLock?: boolean
    } | {
      switchElementId: Core.Ref
      position: SwitchPosition
    } | {
      waived: true
    }
    /** The flank protection the protecting switch element asks for in turn, per its position (PlanPro Fla_Schutz_Weitergabe). */
    passOn?: {
      left?: Core.Ref
      right?: Core.Ref
    }
    clearSections?: Array<Core.Ref>
    lifespan?: Core.Lifespan
  }
  /** A PZB track element (PlanPro PZB_Element): a track magnet of 500, 1000 or 2000 Hz (or both 1000 and 2000), or a speed check of two magnets; where it acts, what it belongs to (a signal), and whether it acts always, when its signal shows stop, or with routes. */
  export type PzbElement = {
    id: Core.Id
    at: Directed
    frequency: "500" | "1000" | "2000" | "1000+2000"
    kind: "magnet" | "speedCheck"
    /** The signal it belongs to (PlanPro PZB_Element_Bezugspunkt). */
    signalId?: Core.Ref
    /** PlanPro Wirksamkeit. */
    effect: "permanent" | "switchedBySignal" | "other"
    /** Routes that switch it (PlanPro PZB_Element_Zuordnung_Fstr). */
    routes?: Array<{
      routeId: Core.Ref
      effect: "activeWhenSet" | "inactiveWhenSet" | "other"
    }>
    /** A magnet that is cut out where a train stops at a platform (PlanPro PZB_INA). */
    ina?: boolean
    /** For a speed check (PlanPro PZB_Element_GUE): the speed it tests [km/h], the length it measures over [m], and which magnet comes first. */
    speedCheck?: {
      testSpeed: number
      measuringLength: number
      arrangement?: "startFirst" | "endFirst"
    }
    lifespan?: Core.Lifespan
  }
  /** The boundary of a station of the core's register, where trains enter it: what marks it — an entry signal or a board (Ne 1), or an entry switch where there is neither (EBO §4). Where it is not where the rules put it, a finding says so. */
  export type StationBoundary = {
    id: Core.Id
    operationalPointId: Core.Ref
    marks: Array<{
      signalId: Core.Ref
    } | {
      switchId: Core.Ref
    }>
    lifespan?: Core.Lifespan
  }
}
