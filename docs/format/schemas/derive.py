#!/usr/bin/env python3
"""The schemas of the specification rail-across-1.0 (docs/format), derived once from the implementation's
schemas (schemas/) with the changes the specification makes. Kept so that what changed is readable; the
schemas it writes are the specification's from now on and are edited as such.

    python3 docs/format/schemas/derive.py
"""
import copy
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, '..', '..', '..', 'schemas')
UUID = '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
VOLTAGES = ['15000V', '25000V', '3000V', '1500V', '750V', '600V']
DRAFT = 'https://json-schema.org/draft/2020-12/schema'


def load(path):
    with open(os.path.join(SRC, path)) as f:
        return json.load(f)


def write(path, doc):
    out = os.path.join(HERE, path)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, 'w') as f:
        f.write(json.dumps(doc, indent=2, ensure_ascii=False) + '\n')


def rename(props, old, new, required=None):
    """A property renamed in place, its place in the order kept."""
    items = [(new if k == old else k, v) for k, v in props.items()]
    props.clear()
    props.update(items)
    if required is not None and old in required:
        required[required.index(old)] = new


ref = lambda target, desc=None: {'$ref': 'core#/$defs/Ref', 'x-target': target, **({'description': desc} if desc else {})}
lifespan = {'$ref': 'core#/$defs/Lifespan'}

# ── core 4.0 ──────────────────────────────────────────────────────────────────
core = load('core/3.1.schema.json')
core['title'] = 'Rail-Data-Core, domain core 4.0'
core['description'] = ('The base every layer depends on: the net (tracks, switches and connectors), what stands on '
                       'it for every discipline (end marks, stopping places) and the reference data (km lines, the '
                       'operational point register). Part of infra-data-1.0 in rail-across-1.0.')
d = core['$defs']
d['Id'] = {'description': 'The id of an object: a UUID in lower case, unique in its collection.', 'type': 'string', 'pattern': UUID}
d['Ref'] = {'description': 'A reference to an object by its id, a UUID in lower case. Where a property is a Ref, its '
            '`x-target` names the collection it points into; the reader checks that the object is there.',
            'type': 'string', 'pattern': UUID}
d['BlobRef'] = {'description': 'Bulk data kept as a blob: the SHA-256 of its canonical JSON.', 'type': 'string', 'pattern': '^sha256:[0-9a-f]{64}$'}
el = d['Element']
el['description'] = ('One design element of a track: straight, arc or transition. No station, no polyline: '
                     'stations are the running sum of the lengths, drawn geometry is derived.')
el['properties']['elementType'] = {'enum': ['straight', 'arc', 'transition']}
del el['properties']['epsg']
tr = d['Track']
tr['properties']['trackType'] = {'enum': ['line', 'station']}
tr['properties']['side'] = {'enum': ['regular', 'opposite']}
rename(tr['properties'], 'operationalPoint', 'operationalPointId', tr.get('required'))
hp = d['HeightPoint']
rename(hp['properties'], 'dep', 'depId', hp.get('required'))
hp['properties']['depId'] = ref('core.heightPoints')
hp['properties']['depId'].pop('x-target')
hp['properties']['depId']['description'] = 'The id of the height point of the same track this one depends on.'
sw = d['Switch']
sw['description'] = ('A switch, crossing, slip, two-end connector (`link`) or construction stage connector '
                     '(`stage_link`) at a node; its ports hold track ends. A switch: A its toe, B1 the main track, '
                     'B2 the branch. A link joins two track ends where the plane changes or where a rebuilt part of '
                     'a track ties in; a stage link joins the track at A to the one at B1 while that is in service '
                     'and to the one at B2 from then on.')
sw['properties']['kind'] = {'enum': ['switch', 'crossing', 'single_slip', 'double_slip', 'link', 'stage_link']}
em = d['EndMark']
em['properties']['type'] = {'$ref': '#/$defs/Text'}
# A km line in design elements, in a plane: far smaller than points, and exact where the source states elements.
d['KmElement'] = {
    'description': 'A design element of a km line: what a track element states of its geometry, without the parts that '
                   'only a track has.',
    'type': 'object', 'required': ['elementType', 'startNode', 'endNode', 'length'], 'additionalProperties': False,
    'properties': {k: copy.deepcopy(el['properties'][k]) for k in
                   ['elementType', 'startNode', 'endNode', 'bearing', 'endBearing', 'length', 'radius', 'r1', 'r2', 'transitionType']},
}
d['KmRun'] = {
    'description': 'A run of a km line without a km jump: its kilometrage at the start and the way it counts along '
                   'the run, and the run itself — design elements, or the points a source gave where it gave no '
                   'elements.',
    'type': 'object', 'required': ['kmStart'], 'additionalProperties': False,
    'properties': {
        'kmStart': {'type': 'number'},
        'counting': {'enum': ['up', 'down']},
        'elements': {'type': 'array', 'items': {'$ref': '#/$defs/KmElement'}, 'minItems': 1},
        'points': {'$ref': '#/$defs/BlobRef'},
    },
    'oneOf': [{'required': ['elements']}, {'required': ['points']}],
}
d['KmLine'] = {
    'description': 'Foreign reference data: the axis of a line around the project, in one plane, as runs from km jump '
                   'to km jump.',
    'type': 'object', 'required': ['id', 'lineNumber', 'epsg', 'runs'], 'additionalProperties': False,
    'properties': {
        'id': {'$ref': '#/$defs/Id'},
        'lineNumber': {'$ref': '#/$defs/Label'},
        'epsg': {'$ref': '#/$defs/Epsg'},
        'runs': {'type': 'array', 'items': {'$ref': '#/$defs/KmRun'}, 'minItems': 1},
        'source': {'$ref': '#/$defs/Text'},
    },
}
write('core/4.0.schema.json', core)

# ── civil 3.0 ─────────────────────────────────────────────────────────────────
civil = load('civil/2.1.schema.json')
civil['title'] = 'Rail-Data-Core, layer civil 3.0'
civil['description'] = ('What stands in the net without being part of it: platforms, engineering structures, '
                        'measured and reference axes, the plan header and the project picture. A layer on core.')
pf = civil['$defs']['Platform']
rename(pf['properties'], 'operationalPoint', 'operationalPointId', pf.get('required'))
write('civil/3.0.schema.json', civil)

# ── signalling 4.0 ────────────────────────────────────────────────────────────
signalling = load('signalling/3.1.schema.json')
signalling['title'] = 'Rail-Data-Core, layer signalling 4.0'
signalling['description'] = ('The signalling: signals and their logical signals, train detection, switch elements, '
                             'routes, overlaps, flank protection, train protection, station boundaries, itineraries. '
                             'Its concepts map onto PlanPro 1.10 so that a plan can be exported; it follows PlanPro '
                             'only so far as that asks. Which systems, kinds, settings and aspects there are is the '
                             'catalogue\'s. A layer on core.')
sd = signalling['$defs']
rename(sd['TrackSection']['properties'], 'boundedBy', 'detectionPointIds', sd['TrackSection'].get('required'))
for k in ['Route', 'Overlap', 'FlankProtection']:
    rename(sd[k]['properties'], 'clearSections', 'clearSectionIds', sd[k].get('required'))
pass_on = sd['FlankProtection']['properties']['passOn']['properties']
rename(pass_on, 'left', 'leftId')
rename(pass_on, 'right', 'rightId')
write('signalling/4.0.schema.json', signalling)

# ── electrification 3.0: masts, suspension points, section insulators, contact wires ───────────────────────
electrification = {
    '$schema': DRAFT, '$id': 'electrification', 'title': 'Rail-Data-Core, layer electrification 3.0',
    'description': ('The overhead contact line: masts, the suspension points on them that hold a contact wire over a '
                    'track, the section insulators that separate two circuits, and the contact wires with their '
                    'voltage. Everything stands on a track of the core. Where a wire runs follows from the points that '
                    'carry it; the voltage over a track is what operations read to know where a vehicle runs electrically.'),
    'type': 'object', 'required': ['fields', 'collections'], 'additionalProperties': False,
    'properties': {
        'fields': {'$ref': '#/$defs/Fields'},
        'collections': {
            'type': 'object', 'required': ['contactWires', 'masts', 'suspensionPoints', 'sectionInsulators'],
            'additionalProperties': False,
            'properties': {
                'contactWires': {'type': 'array', 'items': {'$ref': '#/$defs/ContactWire'}},
                'masts': {'type': 'array', 'items': {'$ref': '#/$defs/Mast'}},
                'suspensionPoints': {'type': 'array', 'items': {'$ref': '#/$defs/SuspensionPoint'}},
                'sectionInsulators': {'type': 'array', 'items': {'$ref': '#/$defs/SectionInsulator'}},
            },
        },
    },
    '$defs': {
        'Fields': {'description': 'The layer\'s own fields.', 'type': 'object', 'additionalProperties': False, 'properties': {}},
        'Voltage': {'description': 'A traction voltage, named as OSRD names the traction mode.', 'enum': VOLTAGES},
        'ContactWire': {
            'description': 'A contact wire: one circuit, with its voltage. It runs where the suspension points and '
                           'section insulators that name it carry it.',
            'type': 'object', 'required': ['id', 'voltage'], 'additionalProperties': False,
            'properties': {'id': {'$ref': 'core#/$defs/Id'}, 'name': {'$ref': 'core#/$defs/Text'},
                           'voltage': {'$ref': '#/$defs/Voltage'}, 'lifespan': lifespan},
        },
        'Mast': {
            'description': 'A mast beside a track, or a portal or headspan across several: what holds the suspension points.',
            'type': 'object', 'required': ['id', 'at'], 'additionalProperties': False,
            'properties': {'id': {'$ref': 'core#/$defs/Id'}, 'name': {'$ref': 'core#/$defs/Label'},
                           'kind': {'enum': ['cantilever', 'portal', 'headspan', 'other']},
                           'at': {'$ref': 'core#/$defs/TrackLocation'}, 'lifespan': lifespan},
        },
        'SuspensionPoint': {
            'description': 'Where a mast holds a contact wire over a track: the wire, its height and its stagger there.',
            'type': 'object', 'required': ['id', 'mastId', 'at', 'contactWireId'], 'additionalProperties': False,
            'properties': {'id': {'$ref': 'core#/$defs/Id'}, 'mastId': ref('electrification.masts'),
                           'at': {'$ref': 'core#/$defs/TrackLocation'}, 'contactWireId': ref('electrification.contactWires'),
                           'height': {'type': 'number', 'minimum': 0}, 'stagger': {'type': 'number'}, 'lifespan': lifespan},
        },
        'SectionInsulator': {
            'description': 'A section insulator (Streckentrennung): a part of the contact line that separates two '
                           'circuits on a track — the wire before it and the wire after it, in the direction of stationing.',
            'type': 'object', 'required': ['id', 'at', 'contactWireBeforeId', 'contactWireAfterId'], 'additionalProperties': False,
            'properties': {'id': {'$ref': 'core#/$defs/Id'}, 'name': {'$ref': 'core#/$defs/Label'},
                           'kind': {'enum': ['insulator', 'neutralSection']},
                           'at': {'$ref': 'core#/$defs/TrackLocation'},
                           'contactWireBeforeId': ref('electrification.contactWires'),
                           'contactWireAfterId': ref('electrification.contactWires'), 'lifespan': lifespan},
        },
    },
}
write('electrification/3.0.schema.json', electrification)

# ── operations: the domains vehicles 1.0 and timetables 1.0 ────────────────────────────────────────────────
ops = load('operations/1.0.schema.json')
vehicles = {
    '$schema': DRAFT, '$id': 'vehicles', 'title': 'Rail-Data-Core, domain vehicles 1.0',
    'description': ('The vehicles a project\'s trains run with, each with its rolling stock as RailJSON. Part of '
                    'operations-1.0 in rail-across-1.0; the timetables name them by id.'),
    'type': 'object', 'required': ['fields', 'collections'], 'additionalProperties': False,
    'properties': {
        'fields': {'$ref': '#/$defs/Fields'},
        'collections': {'type': 'object', 'required': ['vehicles'], 'additionalProperties': False,
                        'properties': {'vehicles': {'type': 'array', 'items': {'$ref': '#/$defs/Vehicle'}}}},
    },
    '$defs': {
        'Fields': {'description': 'The domain\'s own fields.', 'type': 'object', 'additionalProperties': False, 'properties': {}},
        'Vehicle': {
            'description': 'A vehicle (a traction unit or a unit of coaches): where it came from and its rolling stock.',
            'type': 'object', 'required': ['id', 'rollingStock'], 'additionalProperties': False,
            'properties': {
                'id': {'$ref': 'core#/$defs/Id'},
                'key': {'$ref': 'core#/$defs/Text'},
                'rollingStock': {'$ref': '#/$defs/RollingStock'},
            },
        },
        # RailJSON belongs to OSRD: the schema checks what the suite reads from it and takes the rest as it comes.
        'RollingStock': {
            'description': 'The vehicle as RailJSON rolling stock, kept in the object.',
            'type': 'object', 'required': ['railjson_version', 'name', 'effort_curves'],
            'properties': {
                'railjson_version': {'$ref': 'core#/$defs/Text'},
                'name': {'$ref': 'core#/$defs/Text'},
                'effort_curves': {
                    'type': 'object', 'required': ['modes'],
                    'properties': {
                        'modes': {'type': 'object', 'minProperties': 1, 'additionalProperties': False,
                                  'properties': {m: {'$ref': '#/$defs/EffortMode'} for m in VOLTAGES + ['thermal']}},
                        'default_mode': {'enum': VOLTAGES + ['thermal']},
                    },
                },
            },
        },
        'EffortMode': {
            'description': 'The tractive effort of one traction mode.',
            'type': 'object', 'required': ['is_electric', 'curves'],
            'properties': {
                'is_electric': {'type': 'boolean'},
                'curves': {'type': 'array', 'minItems': 1, 'items': {'type': 'object'}},
                'default_curve': {'type': 'object'},
            },
        },
    },
}
write('vehicles/1.0.schema.json', vehicles)

tt = copy.deepcopy(ops)
tt['$id'] = 'timetables'
tt['title'] = 'Rail-Data-Core, domain timetables 1.0'
tt['description'] = ('A project\'s timetables and their trains, the kinds of trains, the formations, connections and '
                     'turnarounds, delay scenarios and studies. Part of operations-1.0 in rail-across-1.0; it belongs '
                     'to no variant of the infrastructure and names it by id — operational points, end marks, tracks, '
                     'itineraries —, checked where it is laid on a variant.')
cols = tt['properties']['collections']
cols['required'].remove('vehicles')
del cols['properties']['vehicles']
td = tt['$defs']
del td['Vehicle']
td['TractionUnit'] = {'description': 'A vehicle of a formation, by its id in the domain vehicles.', 'type': 'object',
                      'required': ['vehicleId'], 'additionalProperties': False,
                      'properties': {'vehicleId': ref('vehicles.vehicles')}}
td['Formation']['description'] = 'A formation (Zugbildung): its vehicles front to back, the hauled load, the brake.'
tp = td['TrainPoint']
props = tp['properties']
tp['description'] = 'A point of a train: an operational point or an end mark by its id, the track, whether it stops there, a departure it waits for.'
new = {}
for k, v in props.items():
    if k == 'place':
        new['operationalPointId'] = ref('core.operationalPoints')
        new['endMarkId'] = ref('core.endMarks')
    elif k == 'station':
        continue
    elif k == 'track':
        new['trackId'] = ref('core.tracks')
    elif k == 'itinerary':
        new['itineraryId'] = ref('signalling.itineraries')
    else:
        new[k] = v
tp['properties'] = new
tp['required'] = ['id', 'stop']
tp['oneOf'] = [{'required': ['operationalPointId']}, {'required': ['endMarkId']}]
td['Train']['properties']['mode'] = {'type': 'string', 'description': 'The traction mode it runs with; without one, what the contact wire along its way and its vehicles allow.'}
for name in ['Connection', 'DelayRule']:
    p = td[name]['properties']
    rename(p, 'place', 'operationalPointId', td[name].get('required'))
    p['operationalPointId'] = ref('core.operationalPoints')
st = td['Study']['properties']['stretches']['items']
rename(st['properties'], 'from', 'fromOperationalPointId', st.get('required'))
rename(st['properties'], 'to', 'toOperationalPointId', st.get('required'))
write('timetables/1.0.schema.json', tt)

# ── rail-across-1.0: the file ─────────────────────────────────────────────────────────────────────────────
domain = {
    'description': 'A domain in the file: its version, the hash of its schema, the domains it refers into, its fields and collections.',
    'type': 'object', 'required': ['version', 'schema', 'fields', 'collections'], 'additionalProperties': False,
    'properties': {
        'version': {'type': 'string', 'pattern': '^[0-9]+\\.[0-9]+$'},
        'schema': {'type': 'string', 'pattern': '^sha256:[0-9a-f]{64}$'},
        'dependsOn': {'type': 'array', 'items': {'type': 'string'}},
        'fields': {'type': 'object'},
        'collections': {'type': 'object', 'additionalProperties': {'type': 'array', 'items': {'type': 'object'}}},
    },
}
def part(fmt, required):
    return {
        'description': f'The part {fmt}-1.0: its format, its version and its domains.',
        'type': 'object', 'required': ['format', 'version', 'domains'], 'additionalProperties': False,
        'properties': {
            'format': {'const': fmt}, 'version': {'const': '1.0'},
            'domains': {'type': 'object', 'required': required,
                        'patternProperties': {'^[a-z][a-zA-Z0-9]*$': {'$ref': '#/$defs/Domain'}}, 'additionalProperties': False},
        },
    }
rail_across = {
    '$schema': DRAFT, '$id': 'rail-across', 'title': 'rail-across-1.0: a railway project as one JSON file (.railacross.json)',
    'description': ('A project\'s infrastructure (infra-data-1.0) and its operations (operations-1.0), each a set of '
                    'domains with their versions and the hashes of their schemas, and the blobs their objects name.'),
    'type': 'object', 'required': ['format', 'version', 'projectId', 'blobs'], 'additionalProperties': False,
    'anyOf': [{'required': ['infraData']}, {'required': ['operations']}],
    'properties': {
        'format': {'const': 'rail-across'},
        'version': {'const': '1.0'},
        'projectId': {'type': 'string', 'pattern': UUID},
        'generator': {'type': 'object', 'required': ['name', 'version'], 'additionalProperties': False,
                      'properties': {'name': {'type': 'string'}, 'version': {'type': 'string'}}},
        'savedAt': {'type': 'string', 'pattern': '^[0-9]{4}-[0-9]{2}-[0-9]{2}T'},
        'infraData': {'$ref': '#/$defs/InfraData'},
        'operations': {'$ref': '#/$defs/Operations'},
        'blobs': {'type': 'object', 'patternProperties': {'^sha256:[0-9a-f]{64}$': {}}, 'additionalProperties': False},
    },
    '$defs': {'InfraData': part('infra-data', ['core']), 'Operations': part('operations', []), 'Domain': domain},
}
write('rail-across.schema.json', rail_across)
print('written')
