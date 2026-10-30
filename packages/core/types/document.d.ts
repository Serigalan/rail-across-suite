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
